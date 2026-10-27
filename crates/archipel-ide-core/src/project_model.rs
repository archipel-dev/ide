//! The project model and the external system import (IntelliJ's `ModuleManager` /
//! `ExternalSystemManager` roles) as seen by the rest of the IDE.
//!
//! `IdeState::project_model()` (a project service) holds the last imported [`ProjectModel`], the running import and the
//! auto-import schedule. The import itself runs on a worker thread
//! (`archipel-ide-externalsystem`); the UI loop calls [`IdeState::poll_project_model`].
//! Nothing else reads the model yet: index roots, the runner classpath, Android, toolchain
//! requirements and jdtls still use their own discovery (see
//! `docs/architecture/project-model.md` for the call sites to migrate).

use crate::services::ProjectModelService;
use crate::{IdeState, spec};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_externalsystem::{
    CancelToken, ExternalSystemRegistry, ImportResult, ImportSettings, ProgressListener,
    ProjectDataManager, ProjectSystemId, ResolveRequest, import_project, load_cached_model,
    save_cached_model,
};
use archipel_ide_projectmodel::{OrderEntry, ProjectFileIndex, ProjectModel};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

/// A build file change waits this long for more changes before the import starts.
const AUTO_IMPORT_DEBOUNCE: Duration = Duration::from_millis(1500);
/// Changes made by the import itself (`Cargo.lock`) are ignored for this long after it.
const SELF_CHANGE_GRACE: Duration = Duration::from_secs(3);

/// Why an import runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportReason {
    /// The project was opened.
    Open,
    /// The user ran Sync (`externalSystem.refresh`).
    Refresh,
    /// A build file changed.
    AutoImport,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ImportStatus {
    /// Nothing imported yet (or no build system found).
    #[default]
    Idle,
    Importing,
    Failed(String),
}

enum Outcome {
    Done(Box<ImportResult>, Arc<ProjectFileIndex>, Duration),
    Failed(String),
    Cancelled,
}

struct ImportJob {
    receiver: Mutex<Receiver<Outcome>>,
    cancel: CancelToken,
    progress: Arc<Mutex<String>>,
}

struct Progress {
    text: Arc<Mutex<String>>,
    waker: crate::Waker,
}

impl ProgressListener for Progress {
    fn text(&self, text: &str) {
        if let Ok(mut slot) = self.text.lock() {
            *slot = text.to_owned();
        }
        self.waker.wake();
    }
}

pub struct ProjectModelState {
    /// The model of the last successful import (or the cached one from the previous session).
    pub model: Option<Arc<ProjectModel>>,
    index: Option<Arc<ProjectFileIndex>>,
    pub status: ImportStatus,
    /// Build systems of the last successful import.
    pub systems: Vec<ProjectSystemId>,
    /// Warnings and errors of the last import (unresolved dependencies, failed resolvers).
    pub diagnostics: Vec<archipel_ide_externalsystem::DiagnosticData>,
    /// Import when build files change.
    pub auto_import: bool,
    /// Number of finished imports of this project, for scenarios that wait for one.
    pub imports_finished: u64,
    /// How long the last import took.
    pub last_duration: Option<Duration>,
    registry: ExternalSystemRegistry,
    job: Option<ImportJob>,
    /// An import was requested while one was running.
    rerun: bool,
    /// A build file changed; start the import at this time.
    scheduled: Option<Instant>,
    ignore_changes_until: Option<Instant>,
}

impl Default for ProjectModelState {
    fn default() -> Self {
        Self {
            model: None,
            index: None,
            status: ImportStatus::Idle,
            systems: Vec::new(),
            diagnostics: Vec::new(),
            // `ARCHIPEL_EXTERNAL_IMPORT=off` keeps headless runs from starting build tools.
            // This crate's own tests open projects with stub build scripts: tests that want
            // the import switch it on.
            auto_import: !cfg!(test)
                && archipel_ide_compat::env_var("ARCHIPEL_EXTERNAL_IMPORT")
                    .map(|v| !matches!(v.as_str(), "off" | "0" | "false"))
                    .unwrap_or(true),
            imports_finished: 0,
            last_duration: None,
            registry: ExternalSystemRegistry::with_defaults(),
            job: None,
            rerun: false,
            scheduled: None,
            ignore_changes_until: None,
        }
    }
}

impl ProjectModelState {
    pub fn is_importing(&self) -> bool {
        self.job.is_some()
    }

    /// Importing, or a build file change is waiting for its debounce.
    pub fn is_busy(&self) -> bool {
        self.job.is_some() || self.scheduled.is_some()
    }

    /// The progress line of the running import ("Gradle: resolving dependencies").
    pub fn progress_text(&self) -> Option<String> {
        let job = self.job.as_ref()?;
        let text = job.progress.lock().ok()?.clone();
        Some(if text.is_empty() {
            "Importing project…".into()
        } else {
            text
        })
    }

    pub fn file_index(&self) -> Option<&ProjectFileIndex> {
        self.index.as_deref()
    }

    pub fn cancel(&mut self) {
        if let Some(job) = &self.job {
            job.cancel.cancel();
        }
        self.scheduled = None;
        self.rerun = false;
    }

    /// Forgets everything (project closed or switched).
    pub fn reset(&mut self) {
        self.cancel();
        // The worker finishes on its own; its result has no receiver any more.
        self.job = None;
        self.model = None;
        self.index = None;
        self.status = ImportStatus::Idle;
        self.systems.clear();
        self.diagnostics.clear();
        self.imports_finished = 0;
        self.last_duration = None;
        self.ignore_changes_until = None;
    }

    pub fn state_id(&self) -> &'static str {
        match (&self.status, self.job.is_some()) {
            (_, true) => "importing",
            (ImportStatus::Failed(_), false) => "failed",
            (ImportStatus::Idle, false) if self.model.is_some() => "ready",
            (ImportStatus::Idle, false) => "idle",
            (ImportStatus::Importing, false) => "idle",
        }
    }

    /// The control socket / `externalSystem.model` view: modules with roots, library and SDK
    /// counts. Paths are relative to `root` where they are inside it.
    pub fn snapshot(&self, root: &Path) -> Value {
        let relative = |path: &Path| -> String {
            path.strip_prefix(root)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| path.to_string_lossy().into_owned())
        };
        let modules: Vec<Value> = self
            .model
            .iter()
            .flat_map(|model| model.modules.iter().map(move |module| (model, module)))
            .map(|(model, module)| {
                let dependencies = |library: bool| -> Vec<Value> {
                    module
                        .order_entries
                        .iter()
                        .filter_map(|entry| match entry {
                            OrderEntry::Library {
                                library: name,
                                scope,
                                ..
                            } if library => Some(json!({"name": name, "scope": scope.id()})),
                            OrderEntry::Module {
                                module: name,
                                scope,
                                ..
                            } if !library => Some(json!({"name": name, "scope": scope.id()})),
                            _ => None,
                        })
                        .collect()
                };
                json!({
                    "name": module.name,
                    "type": format!("{:?}", module.module_type).to_lowercase(),
                    "system": module.external_project.as_ref().map(|e| e.system.clone()),
                    "languageLevel": module.language_level,
                    "sdk": model.module_sdk(module).map(|sdk| sdk.name.clone()),
                    "contentRoots": module.content_roots().map(|p| relative(p)).collect::<Vec<_>>(),
                    "sourceFolders": module.source_folders().map(|folder| json!({
                        "path": relative(&folder.path),
                        "type": folder.root_type.id(),
                    })).collect::<Vec<_>>(),
                    "excluded": module.excluded_folders().map(|p| relative(p)).collect::<Vec<_>>(),
                    "moduleDependencies": dependencies(false),
                    "libraryDependencies": dependencies(true).len(),
                    "libraries": dependencies(true),
                })
            })
            .collect();
        json!({
            "state": self.state_id(),
            "systems": self.systems.iter().map(|s| s.id()).collect::<Vec<_>>(),
            "progress": self.progress_text(),
            "error": match &self.status { ImportStatus::Failed(error) => Some(error.clone()), _ => None },
            "autoImport": self.auto_import,
            "imports": self.imports_finished,
            "modules": modules,
            "libraries": self.model.as_ref().map_or(0, |m| m.library_count()),
            "sdks": self.model.iter().flat_map(|m| m.sdks.iter().map(|s| s.name.clone())).collect::<Vec<_>>(),
            "diagnostics": self.diagnostics.iter().map(|d| json!({
                "severity": format!("{:?}", d.severity).to_lowercase(),
                "message": d.message,
            })).collect::<Vec<_>>(),
        })
    }
}

impl IdeState {
    /// Called when a project is installed: shows the cached model and imports in the background.
    pub(crate) fn open_project_model(&mut self) {
        let Some(service) = self.project_model() else {
            return;
        };
        service.write().reset();
        let Some(root) = self.project_root() else {
            return;
        };
        if let Some(model) = load_cached_model(&root) {
            let model = Arc::new(model);
            let mut state = service.write();
            state.index = Some(Arc::new(model.file_index()));
            state.model = Some(model);
        }
        // Missing JDK/Gradle/Maven are installed first; the import starts when that finishes.
        let requirements = self.toolchain_requirements();
        let waits_for_tools = self.toolchains().read().auto_setup
            && requirements.is_some_and(|r| !r.read().missing_components().is_empty());
        if service.read().auto_import && !waits_for_tools {
            // No build system, no import: the status stays idle and nothing is shown.
            let _ = self.start_external_import(ImportReason::Open);
        }
    }

    /// Starts (or queues) an import of the open project's build systems.
    pub fn start_external_import(&mut self, reason: ImportReason) -> Result<(), String> {
        let root = self.project_root().ok_or("no project is open")?;
        if reason == ImportReason::Refresh {
            // PLAT-SAVE-034: the import reads the build files from disk, so the edit of
            // `build.gradle` must be there ("Reload" buttons, "Refresh all").
            self.save_before(crate::SavePoint::ExternalSystem);
        }
        let service = self.project_model().ok_or("no project is open")?;
        let registry = {
            let mut state = service.write();
            if state.job.is_some() {
                state.rerun = true;
                return Ok(());
            }
            state.registry.clone()
        };
        let resolvers = registry.detect(&root);
        if resolvers.is_empty() {
            service.write().status = ImportStatus::Idle;
            return Err(
                "No supported build system (Gradle, Maven, Cargo, Bazel) found in the project root"
                    .into(),
            );
        }
        let (sender, receiver) = mpsc::channel();
        let cancel = CancelToken::default();
        let text = Arc::new(Mutex::new(String::new()));
        let waker = self.waker.clone();
        let mut request = ResolveRequest::new(&root);
        request.settings = ImportSettings {
            offline_only: archipel_ide_gradle::options().offline,
        };
        request.cancel = cancel.clone();
        request.listener = Arc::new(Progress {
            text: Arc::clone(&text),
            waker: waker.clone(),
        });
        let worker_root = root.clone();
        let spawned = std::thread::Builder::new()
            .name("archipel-ide-import".into())
            .spawn(move || {
                let started = Instant::now();
                let outcome = match import_project(&registry, &ProjectDataManager::new(), &request)
                {
                    Ok(result) => {
                        if !result.model.is_empty() {
                            let _ = save_cached_model(&worker_root, &result.model);
                        }
                        let index = Arc::new(Arc::new(result.model.clone()).file_index());
                        Outcome::Done(Box::new(result), index, started.elapsed())
                    }
                    Err(archipel_ide_externalsystem::ImportError::Cancelled) => Outcome::Cancelled,
                    Err(error) => Outcome::Failed(error.to_string()),
                };
                let _ = sender.send(outcome);
                waker.wake();
            });
        if let Err(error) = spawned {
            return Err(format!("could not start the import: {error}"));
        }
        {
            let mut state = service.write();
            state.job = Some(ImportJob {
                receiver: Mutex::new(receiver),
                cancel,
                progress: text,
            });
            state.status = ImportStatus::Importing;
            state.scheduled = None;
        }
        let names: Vec<&str> = resolvers
            .iter()
            .map(|r| r.system_id().display_name())
            .collect();
        self.status = match reason {
            ImportReason::Open | ImportReason::AutoImport => {
                format!("Importing {} project…", names.join(" and "))
            }
            ImportReason::Refresh => format!("Syncing {} project…", names.join(" and ")),
        };
        Ok(())
    }

    /// Applies a finished import and starts a scheduled one. Call once per frame; returns whether
    /// the status bar changed.
    pub fn poll_project_model(&mut self) -> bool {
        let Some(service) = self.project_model() else {
            return false;
        };
        let mut changed = false;
        let received = {
            let mut state = service.write();
            let received = state.job.as_ref().map(|job| {
                job.receiver
                    .lock()
                    .ok()
                    .map(|r| r.try_recv())
                    .unwrap_or(Err(TryRecvError::Disconnected))
            });
            match received {
                Some(Ok(outcome)) => {
                    state.job = None;
                    Some(outcome)
                }
                Some(Err(TryRecvError::Disconnected)) => {
                    state.job = None;
                    state.status = ImportStatus::Failed("the import stopped unexpectedly".into());
                    changed = true;
                    None
                }
                _ => None,
            }
        };
        if let Some(outcome) = received {
            changed = true;
            self.apply_import_outcome(&service, outcome);
        }
        let start = {
            let mut state = service.write();
            let due = state.scheduled.is_some_and(|at| Instant::now() >= at);
            let start = state.job.is_none() && (due || state.rerun);
            if start {
                state.rerun = false;
                state.scheduled = None;
            }
            start
        };
        if start && self.start_external_import(ImportReason::AutoImport).is_ok() {
            changed = true;
        }
        changed
    }

    fn apply_import_outcome(&mut self, service: &ProjectModelService, outcome: Outcome) {
        service.write().ignore_changes_until = Some(Instant::now() + SELF_CHANGE_GRACE);
        match outcome {
            Outcome::Done(result, index, took) => {
                let result = *result;
                let modules = result.model.modules.len();
                let libraries = result.model.library_count();
                let model = Arc::new(result.model);
                let warnings = result
                    .diagnostics
                    .iter()
                    .filter(|d| d.severity != archipel_ide_externalsystem::Severity::Info)
                    .count();
                {
                    let mut state = service.write();
                    state.systems = result.systems;
                    state.diagnostics = result.diagnostics;
                    state.model = Some(Arc::clone(&model));
                    state.index = Some(index);
                    state.status = ImportStatus::Idle;
                    state.last_duration = Some(took);
                    state.imports_finished += 1;
                }
                let plural =
                    |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
                self.status = format!(
                    "Project imported: {}, {}{}",
                    plural(modules, "module"),
                    plural(libraries, "library"),
                    if warnings > 0 {
                        format!(" ({})", plural(warnings, "warning"))
                    } else {
                        String::new()
                    }
                );
                service.publish_imported(&model);
            }
            Outcome::Failed(error) => {
                self.status = format!("Project import failed: {error}");
                {
                    let mut state = service.write();
                    state.status = ImportStatus::Failed(error.clone());
                    state.imports_finished += 1;
                }
                service.publish_failed(&error);
            }
            Outcome::Cancelled => {
                service.write().status = ImportStatus::Idle;
                self.status = "Project import cancelled".into();
            }
        }
    }

    /// Called with the paths of outside file changes: a build file schedules an auto-import.
    pub(crate) fn project_model_files_changed<'a>(
        &mut self,
        paths: impl IntoIterator<Item = &'a PathBuf>,
    ) {
        let Some(service) = self.project_model() else {
            return;
        };
        let Some(root) = self.project_root() else {
            return;
        };
        let mut state = service.write();
        if !state.auto_import
            || state.job.is_some()
            || state
                .ignore_changes_until
                .is_some_and(|until| Instant::now() < until)
        {
            return;
        }
        if paths
            .into_iter()
            .any(|path| state.registry.is_build_input(&root, path))
        {
            state.scheduled = Some(Instant::now() + AUTO_IMPORT_DEBOUNCE);
        }
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "externalSystem.refresh",
                "Sync Project with Build Files",
                "External System",
                None,
                "{}",
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, _| {
                state
                    .start_external_import(ImportReason::Refresh)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "externalSystem.cancel",
                "Cancel Project Import",
                "External System",
                None,
                "{}",
            ),
            |state: &IdeState| state.project_model().is_some_and(|m| m.is_busy()),
            |state: &mut IdeState, _| {
                if let Some(model) = state.project_model() {
                    model.cancel();
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "externalSystem.model",
                "Show Project Model",
                "External System",
                None,
                "{}",
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, _| {
                let root = state.project_root().unwrap_or_default();
                Ok(state
                    .project_model()
                    .map_or(Value::Null, |m| m.snapshot(&root)))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;
    use archipel_ide_platform::ComponentManager;
    use archipel_ide_project::Project;
    use serde_json::json;

    fn write(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn cargo_project(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("archipel-core-pm-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write(
            &root,
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        );
        write(&root, "src/main.rs", "fn main() {}\n");
        write(&root, "tests/it.rs", "#[test]\nfn t() {}\n");
        root
    }

    fn pm(state: &IdeState) -> Arc<ProjectModelService> {
        state.project_model().expect("a project is open")
    }

    fn wait_idle(state: &mut IdeState) {
        let deadline = Instant::now() + Duration::from_secs(60);
        while pm(state).is_busy() && Instant::now() < deadline {
            state.poll_project_model();
            std::thread::sleep(Duration::from_millis(25));
        }
        state.poll_project_model();
        assert!(!pm(state).is_busy(), "import did not finish");
    }

    fn open(root: &Path) -> IdeState {
        let mut state = IdeState::default();
        state.install_project(Project::open(root).unwrap()).unwrap();
        pm(&state).write().auto_import = true;
        state.open_project_model();
        state
    }

    #[test]
    fn opening_a_project_imports_it_in_the_background() {
        if archipel_ide_runner::find_program("cargo").is_none() {
            return;
        }
        let root = cargo_project("open");
        let mut state = open(&root);
        assert!(pm(&state).is_importing(), "the import starts on open");
        assert_eq!(pm(&state).snapshot(&root)["state"], "importing");
        wait_idle(&mut state);
        let snapshot = pm(&state).snapshot(&root);
        assert_eq!(snapshot["state"], "ready", "{snapshot}");
        assert_eq!(snapshot["systems"], json!(["cargo"]));
        let module = &snapshot["modules"][0];
        assert_eq!(module["name"], "demo");
        assert_eq!(module["contentRoots"], json!([""]));
        assert!(
            module["sourceFolders"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["path"] == "src" && f["type"] == "source")
        );
        assert!(
            module["sourceFolders"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["path"] == "tests" && f["type"] == "testSource")
        );
        assert!(
            module["excluded"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p == "target")
        );
        let service = pm(&state);
        let guard = service.read();
        let index = guard.file_index().unwrap();
        assert!(index.is_in_test_source(&root.join("tests/it.rs")));
        assert!(
            state.status.starts_with("Project imported: 1 module"),
            "{}",
            state.status
        );
        // The model is cached for the next session.
        assert!(load_cached_model(&root).is_some());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_cached_model_shows_before_the_import_finishes() {
        if archipel_ide_runner::find_program("cargo").is_none() {
            return;
        }
        let root = cargo_project("cache");
        let mut first = open(&root);
        wait_idle(&mut first);
        drop(first);
        let state = open(&root);
        assert!(
            pm(&state).read().model.is_some(),
            "cached model is available immediately"
        );
        assert!(pm(&state).is_importing());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sync_command_reimports_and_reports_missing_build_systems() {
        let empty =
            std::env::temp_dir().join(format!("archipel-core-pm-empty-{}", std::process::id()));
        std::fs::create_dir_all(&empty).unwrap();
        let commands = built_in_commands();
        let mut state = open(&empty);
        assert_eq!(pm(&state).snapshot(&empty)["state"], "idle");
        let error = commands
            .execute("externalSystem.refresh", &mut state, json!({}))
            .unwrap_err();
        assert!(
            error.to_string().contains("No supported build system"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(&empty);

        if archipel_ide_runner::find_program("cargo").is_none() {
            return;
        }
        let root = cargo_project("sync");
        let mut state = open(&root);
        wait_idle(&mut state);
        let before = pm(&state).read().imports_finished;
        commands
            .execute("externalSystem.refresh", &mut state, json!({}))
            .unwrap();
        assert!(
            state.status.starts_with("Syncing Cargo"),
            "{}",
            state.status
        );
        wait_idle(&mut state);
        assert_eq!(pm(&state).read().imports_finished, before + 1);
        let model = commands
            .execute("externalSystem.model", &mut state, json!({}))
            .unwrap();
        assert_eq!(model["modules"][0]["name"], "demo");
        // Requests while an import runs are queued, not refused.
        commands
            .execute("externalSystem.refresh", &mut state, json!({}))
            .unwrap();
        commands
            .execute("externalSystem.refresh", &mut state, json!({}))
            .unwrap();
        assert!(pm(&state).read().rerun);
        commands
            .execute("externalSystem.cancel", &mut state, json!({}))
            .unwrap();
        assert!(!pm(&state).read().rerun);
        wait_idle(&mut state);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn build_file_changes_schedule_a_debounced_import() {
        let root = cargo_project("auto");
        let mut state = open(&root);
        // Pretend the first import is done and quiet.
        pm(&state).write().reset();
        state.project_model_files_changed([&root.join("src/main.rs")]);
        assert!(!pm(&state).is_busy(), "sources are not build inputs");
        state.project_model_files_changed([&root.join("Cargo.toml")]);
        assert!(pm(&state).read().scheduled.is_some());
        // Not due yet: polling does not start it.
        assert!(!state.poll_project_model());
        assert!(!pm(&state).is_importing());
        pm(&state).write().scheduled = Some(Instant::now());
        state.poll_project_model();
        assert!(pm(&state).is_importing() || pm(&state).read().scheduled.is_none());
        pm(&state).cancel();
        wait_idle(&mut state);
        // Changes made by the import itself are ignored.
        state.project_model_files_changed([&root.join("Cargo.lock")]);
        assert!(pm(&state).read().scheduled.is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn auto_import_can_be_turned_off() {
        let root = cargo_project("off");
        let mut state = IdeState::default();
        state
            .install_project(Project::open(&root).unwrap())
            .unwrap();
        assert!(!pm(&state).read().auto_import);
        assert!(!pm(&state).is_busy());
        state.project_model_files_changed([&root.join("Cargo.toml")]);
        assert!(!pm(&state).is_busy());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[derive(Default)]
    struct Heard(parking_lot::Mutex<Vec<String>>);

    impl crate::services::ProjectModelListener for Heard {
        fn model_imported(&self, model: &ProjectModel) {
            self.0
                .lock()
                .push(format!("imported {}", model.modules.len()));
        }
        fn import_failed(&self, error: &str) {
            self.0.lock().push(format!("failed {error}"));
        }
    }

    #[test]
    fn a_finished_import_is_announced_on_the_project_bus() {
        if archipel_ide_runner::find_program("cargo").is_none() {
            return;
        }
        let root = cargo_project("topic");
        let mut state = open(&root);
        let heard = Arc::new(Heard::default());
        let project = state.platform_project().unwrap();
        project
            .message_bus()
            .subscribe(
                project.disposable(),
                crate::services::PROJECT_MODEL_TOPIC,
                heard.clone(),
            )
            .unwrap();
        wait_idle(&mut state);
        assert_eq!(*heard.0.lock(), ["imported 1"]);
        let _ = std::fs::remove_dir_all(&root);
    }
}

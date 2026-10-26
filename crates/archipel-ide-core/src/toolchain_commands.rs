//! Toolchain provisioning commands (`toolchain.*`): list, locate, install with
//! progress, cancel, uninstall, "use existing path" and project pins.
//!
//! Installs only start from `toolchain.install`, i.e. from an explicit user
//! action (Set up / Install buttons, palette, CLI); nothing downloads silently.

use crate::{IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_toolchains::providers::ProjectRequirement;
use archipel_ide_toolchains::service::{JobId, ToolchainEvent, ToolchainService};
use archipel_ide_toolchains::{
    ToolVersion, ToolchainSettings, ToolchainStore, Toolchains, VersionRequest,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Provisioning state owned by the IDE.
pub struct ToolchainState {
    pub toolchains: Toolchains,
    pub service: ToolchainService,
    /// `toolchains.toml` that `toolchain.usePath` writes (user level).
    pub user_settings: Option<PathBuf>,
    /// Problem loading `toolchains.toml`, shown instead of failing project open.
    pub settings_error: Option<String>,
    /// What the open project needs, by language (from marker files).
    pub requirements: Vec<ProjectRequirement>,
    /// Requirements with components that cannot be found, cached for the UI.
    missing: Vec<(ProjectRequirement, Vec<String>)>,
    /// Languages whose "missing toolchain" bar was dismissed this session.
    dismissed: BTreeSet<String>,
    /// Install what an opened project is missing without asking. The app turns this on;
    /// it stays off in tests and headless runs so they never download.
    pub auto_setup: bool,
}

/// The "missing toolchain" editor notification for one language.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ToolchainBanner {
    pub language: String,
    /// Missing component ids (requirements included), in setup order.
    pub components: Vec<String>,
    /// "Java project: JDK, Gradle and the Java language server are missing".
    pub text: String,
    /// Whether a setup of these components is running.
    pub busy: bool,
}

impl Default for ToolchainState {
    fn default() -> Self {
        let store = ToolchainStore::new(archipel_ide_toolchains::default_root());
        store.clean();
        Self::new(
            Toolchains::new(archipel_ide_toolchains::providers::builtin_catalog(), store),
            ToolchainSettings::user_file(),
        )
    }
}

impl ToolchainState {
    pub fn new(toolchains: Toolchains, user_settings: Option<PathBuf>) -> Self {
        Self {
            toolchains,
            service: ToolchainService::default(),
            user_settings,
            settings_error: None,
            requirements: Vec::new(),
            missing: Vec::new(),
            dismissed: BTreeSet::new(),
            auto_setup: false,
        }
    }

    /// Recomputes which required components cannot be found.
    pub fn refresh_missing(&mut self) {
        self.missing = self
            .requirements
            .iter()
            .filter_map(|requirement| {
                let missing = self.toolchains.missing(requirement.components);
                // Setup order: dependencies (JDK) before what needs them (jdtls).
                let mut ordered: Vec<String> = requirement
                    .components
                    .iter()
                    .filter(|id| missing.iter().any(|m| m == *id))
                    .map(|id| (*id).to_owned())
                    .collect();
                let extra: Vec<String> = missing
                    .into_iter()
                    .filter(|id| !ordered.contains(id))
                    .collect();
                ordered.extend(extra);
                (!ordered.is_empty()).then(|| (requirement.clone(), ordered))
            })
            .collect();
    }

    /// The bar to show above an editor for `path`, if its language lacks tools.
    pub fn banner_for(&self, path: &Path) -> Option<ToolchainBanner> {
        let found = self.missing.iter().find(|(requirement, _)| {
            requirement.applies_to(path) && !self.dismissed.contains(requirement.language)
        })?;
        self.banner(found)
    }

    /// The bar shown while no editor is open: the first language of the project
    /// that lacks tools, so setup is offered as soon as the project opens.
    pub fn project_banner(&self) -> Option<ToolchainBanner> {
        let found = self
            .missing
            .iter()
            .find(|(requirement, _)| !self.dismissed.contains(requirement.language))?;
        self.banner(found)
    }

    fn banner(
        &self,
        (requirement, components): &(ProjectRequirement, Vec<String>),
    ) -> Option<ToolchainBanner> {
        let names: Vec<String> = components
            .iter()
            .map(|id| self.banner_name(id, requirement.language))
            .collect();
        let list = match names.as_slice() {
            [one] => one.clone(),
            [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
            [] => return None,
        };
        let verb = if names.len() == 1 { "is" } else { "are" };
        Some(ToolchainBanner {
            language: requirement.language.to_owned(),
            text: format!("{} project: {list} {verb} missing", requirement.language),
            busy: self
                .service
                .jobs()
                .iter()
                .any(|job| components.contains(&job.component)),
            components: components.clone(),
        })
    }

    /// Reloads user and project `toolchains.toml` and the project's pins.
    pub fn reload(&mut self, project_root: Option<&Path>) {
        let project_file = project_root.map(ToolchainSettings::project_file);
        let settings =
            ToolchainSettings::load_from(self.user_settings.as_deref(), project_file.as_deref());
        self.settings_error = settings.as_ref().err().map(ToString::to_string);
        self.toolchains
            .apply_project(project_root, settings.unwrap_or_default());
        self.requirements = project_root
            .map(archipel_ide_toolchains::providers::project_requirements)
            .unwrap_or_default();
        self.refresh_missing();
    }

    /// Status-bar text and fraction for the first running install.
    pub fn progress(&self) -> Option<(String, Option<f32>)> {
        let job = self.service.jobs().first()?;
        // Component ids keep the status bar entry short (`jdtls`, `jdk`).
        let mut text = job.describe(&job.component);
        let more = self.service.jobs().len() - 1;
        if more > 0 {
            text.push_str(&format!(" (+{more})"));
        }
        Some((text, job.fraction()))
    }

    /// Component name in the bar: language servers read "the Java language server".
    fn banner_name(&self, id: &str, language: &str) -> String {
        let server = self.toolchains.catalog().get(id).is_some_and(|provider| {
            provider.spec().kind == archipel_ide_toolchains::ComponentKind::LanguageServer
        });
        if server {
            format!("the {language} language server")
        } else {
            self.component_name(id)
        }
    }

    fn component_name(&self, id: &str) -> String {
        self.toolchains
            .catalog()
            .get(id)
            .map_or(id.to_owned(), |p| p.spec().name.clone())
    }
}

impl IdeState {
    pub(crate) fn project_root(&self) -> Option<PathBuf> {
        self.project.as_ref().map(|p| p.root().to_owned())
    }

    /// Points the language servers at the current toolchain settings and pins.
    pub(crate) fn refresh_toolchain_resolver(&mut self) {
        let resolver =
            crate::toolchain_lsp::ToolchainServerResolver::new(self.toolchains.toolchains.clone());
        self.lsp.set_resolver(Some(std::sync::Arc::new(resolver)));
        self.sync_tool_environment();
    }

    /// Makes JDKs and build tools that are not on `PATH` (Archipel IDE-managed or
    /// configured in `toolchains.toml`) available to builds, runs and tests.
    pub(crate) fn sync_tool_environment(&self) {
        archipel_ide_runner::set_tool_environment(tool_environment(&self.toolchains.toolchains));
    }

    /// Installs `components` and whatever they require that cannot be found.
    /// Returns the started jobs (empty when everything is present).
    pub fn setup_toolchains(
        &mut self,
        components: &[&str],
    ) -> Result<Vec<(String, JobId)>, String> {
        for component in components {
            if self
                .toolchains
                .toolchains
                .catalog()
                .get(component)
                .is_none()
            {
                return Err(format!("unknown toolchain component `{component}`"));
            }
        }
        let missing = self.toolchains.toolchains.missing(components);
        let mut jobs = Vec::new();
        for component in missing {
            let job = self.install_toolchain(&component, None)?;
            jobs.push((component, job));
        }
        if jobs.is_empty() {
            self.status = "All toolchain components are already set up".into();
        } else {
            let ids: Vec<&str> = jobs
                .iter()
                .map(|(component, _)| component.as_str())
                .collect();
            self.status = format!("Setting up {}…", ids.join(", "));
        }
        Ok(jobs)
    }

    /// Starts installing everything the open project is missing when automatic
    /// setup is on. Components that are already being installed are skipped.
    pub fn auto_setup_toolchains(&mut self) {
        if !self.toolchains.auto_setup {
            return;
        }
        let mut components: Vec<String> = Vec::new();
        for (_, missing) in &self.toolchains.missing {
            for id in missing {
                let running = self
                    .toolchains
                    .service
                    .jobs()
                    .iter()
                    .any(|job| &job.component == id);
                if !running && !components.contains(id) {
                    components.push(id.clone());
                }
            }
        }
        if components.is_empty() {
            return;
        }
        let ids: Vec<&str> = components.iter().map(String::as_str).collect();
        if let Err(error) = self.setup_toolchains(&ids) {
            self.status = error;
        }
    }

    /// Applies finished installs to the status line. Call from the UI loop.
    pub fn poll_toolchains(&mut self) {
        let events = self.toolchains.service.poll();
        self.apply_toolchain_events(&events);
    }

    fn apply_toolchain_events(&mut self, events: &[ToolchainEvent]) {
        let mut installed_any = false;
        if events
            .iter()
            .any(|event| matches!(event, ToolchainEvent::Finished { .. }))
        {
            self.toolchains.refresh_missing();
        }
        for event in events {
            if let ToolchainEvent::Finished {
                component, result, ..
            } = event
            {
                let name = self.toolchains.component_name(component);
                self.status = match result {
                    Ok(installed) => {
                        installed_any = true;
                        format!("{name} {} installed", installed.version)
                    }
                    Err(error) => format!("{name} setup failed: {error}"),
                };
            }
        }
        if installed_any {
            self.sync_tool_environment();
        }
        // Language servers that were missing can start now; wait until the
        // whole setup (e.g. jdtls and its JDK) has finished.
        if installed_any
            && !self.toolchains.service.is_busy()
            && self.lsp.missing_servers().next().is_some()
        {
            let status = std::mem::take(&mut self.status);
            self.restart_language_servers();
            self.status = status;
        }
        // A Gradle project imports itself once its JDK (and Gradle) are there.
        if installed_any
            && !self.toolchains.service.is_busy()
            && self.is_gradle_project()
            && !self.runner.is_running()
        {
            let status = std::mem::take(&mut self.status);
            match self.start_gradle_sync() {
                Ok(()) => self.status = format!("{status}; syncing Gradle project…"),
                Err(error) => self.status = format!("{status}; Gradle sync: {error}"),
            }
        }
    }

    /// Starts installing `component`; returns the job id.
    pub fn install_toolchain(
        &mut self,
        component: &str,
        request: Option<VersionRequest>,
    ) -> Result<JobId, String> {
        if self
            .toolchains
            .toolchains
            .catalog()
            .get(component)
            .is_none()
        {
            return Err(format!("unknown toolchain component `{component}`"));
        }
        let job = self
            .toolchains
            .service
            .install(&self.toolchains.toolchains, component, request);
        self.status = format!("Setting up {component}…");
        Ok(job)
    }
}

/// Build-side environment for the located JDK and Gradle: their `bin`
/// directories and `JAVA_HOME`, for tools that were not found on `PATH`.
pub(crate) fn tool_environment(toolchains: &Toolchains) -> archipel_ide_runner::ToolEnvironment {
    use archipel_ide_toolchains::LocateSource;
    let mut environment = archipel_ide_runner::ToolEnvironment::default();
    for id in ["jdk", "gradle", "maven", "bazelisk"] {
        let Some(located) = toolchains.locate(id) else {
            continue;
        };
        if matches!(
            located.source,
            LocateSource::Path | LocateSource::HomeEnv(_)
        ) {
            continue;
        }
        if let Some(bin) = located.executable.parent() {
            environment.path_dirs.push(bin.to_owned());
        }
        if id == "jdk" && std::env::var_os("JAVA_HOME").is_none() {
            environment
                .vars
                .push(("JAVA_HOME".into(), located.home.display().to_string()));
        }
    }
    environment
}

fn located_json(located: &archipel_ide_toolchains::Located) -> Value {
    json!({
        "path": located.executable,
        "home": located.home,
        "version": located.version,
        "source": located.source,
    })
}

fn component_param(state: &IdeState, params: &Value) -> Result<String, CommandError> {
    let id = string_param(params, "component")?;
    if state.toolchains.toolchains.catalog().get(&id).is_none() {
        return Err(CommandError::InvalidParams(format!(
            "unknown toolchain component `{id}`"
        )));
    }
    Ok(id)
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec("toolchain.list", "Toolchains", "Toolchains", None, "{}"),
            |_| true,
            |state: &mut IdeState, _| {
                let toolchains = &state.toolchains.toolchains;
                let components: Vec<Value> = toolchains
                    .catalog()
                    .specs()
                    .filter_map(|spec| toolchains.status(&spec.id))
                    .map(|status| {
                        json!({
                            "id": status.spec.id,
                            "name": status.spec.name,
                            "kind": status.spec.kind,
                            "requires": status.spec.requires,
                            "located": status.located.as_ref().map(located_json),
                            "installed": status.installed.iter().map(|i| &i.version).collect::<Vec<_>>(),
                            "pin": status.pin,
                        })
                    })
                    .collect();
                Ok(json!({
                    "root": toolchains.store().root(),
                    "platform": toolchains.platform().key(),
                    "components": components,
                    "settingsError": state.toolchains.settings_error,
                }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "toolchain.locate",
                "Locate Toolchain Component",
                "Toolchains",
                None,
                r#"{"component":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = component_param(state, &params)?;
                Ok(state
                    .toolchains
                    .toolchains
                    .locate(&id)
                    .as_ref()
                    .map_or(Value::Null, located_json))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "toolchain.missing",
                "Missing Toolchain Components",
                "Toolchains",
                None,
                r#"{"components":["string"]}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let ids = crate::string_array_param(&params, "components")?;
                let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
                Ok(json!(state.toolchains.toolchains.missing(&ids)))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "toolchain.install",
                "Set Up Toolchain Component",
                "Toolchains",
                None,
                r#"{"component":"string","version":"string?","wait":"bool?"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = component_param(state, &params)?;
                let request = params
                    .get("version")
                    .and_then(Value::as_str)
                    .map(VersionRequest::parse);
                let job = state
                    .install_toolchain(&id, request)
                    .map_err(CommandError::Failed)?;
                if params.get("wait").and_then(Value::as_bool) != Some(true) {
                    return Ok(json!({"job": job}));
                }
                // Headless use (CLI): block until the install finishes.
                let (events, result) = state
                    .toolchains
                    .service
                    .wait(job, Duration::from_secs(60 * 60));
                state.apply_toolchain_events(&events);
                match result {
                    Some(Ok(installed)) => Ok(json!({
                        "job": job,
                        "component": installed.component,
                        "version": installed.version,
                        "path": installed.path,
                    })),
                    Some(Err(error)) => Err(CommandError::Failed(error)),
                    None => Err(CommandError::Failed("install timed out".into())),
                }
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "toolchain.setup",
                "Set Up Toolchains",
                "Toolchains",
                None,
                r#"{"components":["string"],"wait":"bool?"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let ids = crate::string_array_param(&params, "components")?;
                let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
                let jobs = state
                    .setup_toolchains(&ids)
                    .map_err(CommandError::InvalidParams)?;
                let mut result: Vec<Value> = jobs
                    .iter()
                    .map(|(component, job)| json!({"component": component, "job": job}))
                    .collect();
                if params.get("wait").and_then(Value::as_bool) == Some(true) {
                    let mut seen: Vec<ToolchainEvent> = Vec::new();
                    for (entry, (_, job)) in result.iter_mut().zip(&jobs) {
                        // A job may have finished while waiting on an earlier one.
                        let earlier = seen.iter().find_map(|event| match event {
                            ToolchainEvent::Finished {
                                job: done, result, ..
                            } if done == job => Some(result.clone()),
                            _ => None,
                        });
                        let outcome = match earlier {
                            Some(result) => Some(result),
                            None => {
                                let (events, outcome) = state
                                    .toolchains
                                    .service
                                    .wait(*job, Duration::from_secs(60 * 60));
                                state.apply_toolchain_events(&events);
                                seen.extend(events);
                                outcome
                            }
                        };
                        match outcome {
                            Some(Ok(installed)) => {
                                entry["version"] = json!(installed.version);
                                entry["path"] = json!(installed.path);
                            }
                            Some(Err(error)) => return Err(CommandError::Failed(error)),
                            None => return Err(CommandError::Failed("setup timed out".into())),
                        }
                    }
                }
                Ok(json!(result))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "toolchain.cancel",
                "Cancel Toolchain Setup",
                "Toolchains",
                None,
                r#"{"job":"number?"}"#,
            ),
            |state: &IdeState| state.toolchains.service.is_busy(),
            |state: &mut IdeState, params| {
                match params.get("job").and_then(Value::as_u64) {
                    Some(job) => {
                        if !state.toolchains.service.cancel(job) {
                            return Err(CommandError::InvalidParams(format!(
                                "no running setup job {job}"
                            )));
                        }
                    }
                    None => state.toolchains.service.cancel_all(),
                }
                state.status = "Cancelling toolchain setup…".into();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "toolchain.jobs",
                "Toolchain Setup Jobs",
                "Toolchains",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                let jobs: Vec<Value> = state
                    .toolchains
                    .service
                    .jobs()
                    .iter()
                    .map(|job| {
                        json!({
                            "job": job.id,
                            "component": job.component,
                            "version": job.version,
                            "text": job.describe(&state.toolchains.component_name(&job.component)),
                            "fraction": job.fraction(),
                        })
                    })
                    .collect();
                Ok(json!(jobs))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "toolchain.uninstall",
                "Uninstall Toolchain Component",
                "Toolchains",
                None,
                r#"{"component":"string","version":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = string_param(&params, "component")?;
                let version = ToolVersion::new(string_param(&params, "version")?);
                if state
                    .toolchains
                    .service
                    .jobs()
                    .iter()
                    .any(|job| job.component == id)
                {
                    return Err(CommandError::Failed(format!(
                        "{id} is being set up; cancel the setup first"
                    )));
                }
                state
                    .toolchains
                    .toolchains
                    .store()
                    .uninstall(&id, &version)
                    .map_err(|error| CommandError::Failed(error.to_string()))?;
                state.status = format!(
                    "Uninstalled {} {version}",
                    state.toolchains.component_name(&id)
                );
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "toolchain.usePath",
                "Use Existing Toolchain Installation",
                "Toolchains",
                None,
                r#"{"component":"string","path":"string?"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = component_param(state, &params)?;
                let path = params
                    .get("path")
                    .and_then(Value::as_str)
                    .map(PathBuf::from);
                if let Some(path) = &path {
                    let toolchains = &state.toolchains.toolchains;
                    let spec = toolchains
                        .catalog()
                        .get(&id)
                        .map(|p| p.spec().clone())
                        .ok_or_else(|| CommandError::InvalidParams(id.clone()))?;
                    let usable = path.is_file()
                        || archipel_ide_toolchains::store::find_executable(
                            path,
                            &spec,
                            &toolchains.platform(),
                        )
                        .is_some();
                    if !usable {
                        return Err(CommandError::InvalidParams(format!(
                            "no {} executable ({}) found at {}",
                            spec.name,
                            spec.primary_executable(),
                            path.display()
                        )));
                    }
                }
                let file = state.toolchains.user_settings.clone().ok_or_else(|| {
                    CommandError::Failed("no user configuration directory".into())
                })?;
                ToolchainSettings::update_file(&file, &id, |component| {
                    component.path = path.clone();
                })
                .map_err(|error| CommandError::Failed(error.to_string()))?;
                let root = state.project_root();
                state.toolchains.reload(root.as_deref());
                state.refresh_toolchain_resolver();
                let located = state.toolchains.toolchains.locate(&id);
                Ok(located.as_ref().map_or(Value::Null, located_json))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "toolchain.banner",
                "Missing Toolchain Notification",
                "Toolchains",
                None,
                r#"{"path":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let path = PathBuf::from(string_param(&params, "path")?);
                Ok(json!(state.toolchains.banner_for(&path)))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "toolchain.dismissBanner",
                "Dismiss Missing Toolchain Notification",
                "Toolchains",
                None,
                r#"{"language":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let language = string_param(&params, "language")?;
                state.toolchains.dismissed.insert(language);
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "toolchain.pins",
                "Project Toolchain Pins",
                "Toolchains",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| Ok(json!(state.toolchains.toolchains.pins().pins)),
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;
    use archipel_ide_toolchains::catalog::{IndexedRelease, ReleaseIndex};
    use archipel_ide_toolchains::download::sha256_hex;
    use archipel_ide_toolchains::{
        ArchiveKind, Artifact, Catalog, Checksum, ComponentKind, ComponentSpec, Platform,
        StaticProvider,
    };
    use std::collections::BTreeMap;
    use std::io::{Read, Write};
    use std::sync::Arc;

    fn archive(payload: &[u8]) -> Vec<u8> {
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        let mut header = tar::Header::new_gnu();
        header.set_size(payload.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder
            .append_data(&mut header, "demo-1.0/bin/demo", payload)
            .unwrap();
        builder.into_inner().unwrap().finish().unwrap()
    }

    /// Serves `body` to every connection on a local port.
    fn serve(body: Vec<u8>) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/demo-1.0.tar.gz", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut request = [0u8; 4096];
                let _ = stream.read(&mut request);
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        url
    }

    fn state_with_demo(dir: &Path, url: String, sha256: String) -> IdeState {
        let platform = Platform::parse("linux", "x86_64").unwrap();
        let mut catalog = Catalog::new();
        catalog.register(Arc::new(StaticProvider {
            spec: ComponentSpec::new("demo", "Demo Tool", ComponentKind::BuildTool, "demo"),
            index: ReleaseIndex {
                releases: vec![IndexedRelease {
                    version: ToolVersion::new("1.0"),
                    channels: vec![],
                    artifacts: BTreeMap::from([(
                        platform.key(),
                        Artifact {
                            url,
                            checksum: Checksum::Sha256(sha256),
                            archive: ArchiveKind::TarGz,
                            size: None,
                        },
                    )]),
                    license: None,
                }],
            },
        }));
        let toolchains = Toolchains::new(catalog, ToolchainStore::new(dir.join("toolchains")))
            .with_platform(platform)
            .with_environment(Default::default(), BTreeMap::new());
        IdeState {
            toolchains: ToolchainState::new(toolchains, Some(dir.join("user/toolchains.toml"))),
            ..IdeState::default()
        }
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("archipel-ide-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn install_locate_uninstall_round_trip_through_commands() {
        let dir = temp("toolchain-commands");
        let body = archive(b"#!/bin/sh\n");
        let sha = sha256_hex(&body);
        let mut state = state_with_demo(&dir, serve(body), sha);
        let commands = built_in_commands();

        let missing = commands
            .execute(
                "toolchain.missing",
                &mut state,
                json!({"components": ["demo"]}),
            )
            .unwrap();
        assert_eq!(missing, json!(["demo"]));
        assert_eq!(
            commands
                .execute("toolchain.locate", &mut state, json!({"component": "demo"}))
                .unwrap(),
            Value::Null
        );
        assert!(!commands.is_enabled("toolchain.cancel", &state));

        let result = commands
            .execute(
                "toolchain.install",
                &mut state,
                json!({"component": "demo", "wait": true}),
            )
            .unwrap();
        assert_eq!(result["version"], "1.0");
        assert_eq!(state.status, "Demo Tool 1.0 installed");

        let located = commands
            .execute("toolchain.locate", &mut state, json!({"component": "demo"}))
            .unwrap();
        assert_eq!(located["source"]["kind"], "managed");
        let list = commands
            .execute("toolchain.list", &mut state, json!({}))
            .unwrap();
        assert_eq!(list["components"][0]["installed"], json!(["1.0"]));

        commands
            .execute(
                "toolchain.uninstall",
                &mut state,
                json!({"component": "demo", "version": "1.0"}),
            )
            .unwrap();
        assert!(!dir.join("toolchains/demo/1.0").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_installs_and_bad_input_return_structured_errors() {
        let dir = temp("toolchain-errors");
        let body = archive(b"x");
        let mut state = state_with_demo(&dir, serve(body), "00".repeat(32));
        let commands = built_in_commands();
        let error = commands
            .execute(
                "toolchain.install",
                &mut state,
                json!({"component": "demo", "wait": true}),
            )
            .unwrap_err();
        assert!(error.to_string().contains("checksum mismatch"), "{error}");
        assert!(state.status.contains("setup failed"));
        let error = commands
            .execute(
                "toolchain.install",
                &mut state,
                json!({"component": "nope"}),
            )
            .unwrap_err();
        assert!(matches!(error, CommandError::InvalidParams(_)));
        let error = commands
            .execute(
                "toolchain.usePath",
                &mut state,
                json!({"component": "demo", "path": dir.join("missing")}),
            )
            .unwrap_err();
        assert!(matches!(error, CommandError::InvalidParams(_)), "{error}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn use_path_persists_and_project_pins_are_reported() {
        let dir = temp("toolchain-usepath");
        let mut state = state_with_demo(&dir, "http://127.0.0.1:9/x".into(), "00".repeat(32));
        let commands = built_in_commands();
        let home = dir.join("opt/demo");
        std::fs::create_dir_all(home.join("bin")).unwrap();
        std::fs::write(home.join("bin/demo"), "").unwrap();
        let located = commands
            .execute(
                "toolchain.usePath",
                &mut state,
                json!({"component": "demo", "path": home}),
            )
            .unwrap();
        assert_eq!(located["source"]["kind"], "configured");
        assert_eq!(located["home"], json!(home));
        let saved = std::fs::read_to_string(dir.join("user/toolchains.toml")).unwrap();
        assert!(saved.contains("[components.demo]"), "{saved}");
        commands
            .execute(
                "toolchain.usePath",
                &mut state,
                json!({"component": "demo"}),
            )
            .unwrap();
        assert!(state.toolchains.toolchains.locate("demo").is_none());

        let project = dir.join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(project.join(".bazelversion"), "7.3.1\n").unwrap();
        commands
            .execute("project.open", &mut state, json!({"path": project}))
            .unwrap();
        let pins = commands
            .execute("toolchain.pins", &mut state, json!({}))
            .unwrap();
        assert_eq!(pins[0]["component"], "bazel");
        assert_eq!(pins[0]["request"]["value"], "7.3.1");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn setup_installs_missing_requirements_too() {
        let dir = temp("toolchain-setup");
        let body = archive(b"#!/bin/sh\n");
        let sha = sha256_hex(&body);
        let url = serve(body);
        let mut state = state_with_demo(&dir, url.clone(), sha.clone());
        // `demo-ls` requires `demo`; both come from the same test archive.
        let mut spec =
            ComponentSpec::new("demo-ls", "Demo LS", ComponentKind::LanguageServer, "demo");
        spec.requires = vec!["demo".into()];
        let platform = Platform::parse("linux", "x86_64").unwrap();
        let mut catalog = state.toolchains.toolchains.catalog().clone();
        catalog.register(Arc::new(StaticProvider {
            spec,
            index: ReleaseIndex {
                releases: vec![IndexedRelease {
                    version: ToolVersion::new("2.0"),
                    channels: vec![],
                    artifacts: BTreeMap::from([(
                        platform.key(),
                        Artifact {
                            url,
                            checksum: Checksum::Sha256(sha),
                            archive: ArchiveKind::TarGz,
                            size: None,
                        },
                    )]),
                    license: None,
                }],
            },
        }));
        state.toolchains.toolchains =
            Toolchains::new(catalog, ToolchainStore::new(dir.join("toolchains")))
                .with_platform(platform)
                .with_environment(Default::default(), BTreeMap::new());
        let commands = built_in_commands();
        let result = commands
            .execute(
                "toolchain.setup",
                &mut state,
                json!({"components": ["demo-ls"], "wait": true}),
            )
            .unwrap();
        let mut installed: Vec<&str> = result
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["component"].as_str().unwrap())
            .collect();
        installed.sort();
        assert_eq!(installed, ["demo", "demo-ls"]);
        assert!(
            result
                .as_array()
                .unwrap()
                .iter()
                .all(|e| e["version"].is_string())
        );
        // Everything present: nothing to do.
        let again = commands
            .execute(
                "toolchain.setup",
                &mut state,
                json!({"components": ["demo-ls"]}),
            )
            .unwrap();
        assert_eq!(again, json!([]));
        assert!(
            commands
                .execute(
                    "toolchain.setup",
                    &mut state,
                    json!({"components": ["nope"]})
                )
                .is_err()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn banner_lists_missing_components_for_the_project_language() {
        let dir = temp("toolchain-banner");
        let mut catalog = Catalog::new();
        for spec in [
            archipel_ide_toolchains::providers::jdtls::spec(),
            archipel_ide_toolchains::providers::java_debug::spec(),
            archipel_ide_toolchains::providers::jdk::spec(),
            archipel_ide_toolchains::providers::gradle::spec(),
        ] {
            catalog.register(Arc::new(StaticProvider {
                spec,
                index: ReleaseIndex::default(),
            }));
        }
        let toolchains = Toolchains::new(catalog, ToolchainStore::new(dir.join("toolchains")))
            .with_platform(Platform::parse("linux", "x86_64").unwrap())
            .with_environment(Default::default(), BTreeMap::new());
        let mut state = IdeState {
            toolchains: ToolchainState::new(toolchains, Some(dir.join("user/toolchains.toml"))),
            ..IdeState::default()
        };
        let project = dir.join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(project.join("settings.gradle"), "").unwrap();
        let commands = built_in_commands();
        commands
            .execute("project.open", &mut state, json!({"path": project}))
            .unwrap();

        let banner = commands
            .execute(
                "toolchain.banner",
                &mut state,
                json!({"path": "src/App.java"}),
            )
            .unwrap();
        // No wrapper: the project also needs Gradle.
        assert_eq!(
            banner["text"],
            "Java project: JDK, Gradle, the Java language server and Java debugger (java-debug) are missing"
        );
        assert_eq!(
            banner["components"],
            json!(["jdk", "gradle", "jdtls", "java-debug"])
        );
        // The same bar is offered while no editor is open.
        assert_eq!(
            state.toolchains.project_banner().map(|banner| banner.text),
            Some(banner["text"].as_str().unwrap().to_owned())
        );
        assert_eq!(banner["busy"], false);
        let other = commands
            .execute("toolchain.banner", &mut state, json!({"path": "README.md"}))
            .unwrap();
        assert_eq!(other, Value::Null);

        // An existing JDK configured by path leaves only jdtls.
        let jdk = dir.join("jdk");
        std::fs::create_dir_all(jdk.join("bin")).unwrap();
        std::fs::write(jdk.join("bin/java"), "").unwrap();
        commands
            .execute(
                "toolchain.usePath",
                &mut state,
                json!({"component": "jdk", "path": jdk}),
            )
            .unwrap();
        let banner = commands
            .execute(
                "toolchain.banner",
                &mut state,
                json!({"path": "src/App.java"}),
            )
            .unwrap();
        assert_eq!(
            banner["text"],
            "Java project: Gradle, the Java language server and Java debugger (java-debug) are missing"
        );
        // A configured JDK is handed to builds (bin on PATH, JAVA_HOME).
        let environment = tool_environment(&state.toolchains.toolchains);
        assert_eq!(environment.path_dirs, [jdk.join("bin")]);
        if std::env::var_os("JAVA_HOME").is_none() {
            assert_eq!(
                environment.vars,
                [("JAVA_HOME".to_owned(), jdk.display().to_string())]
            );
        }

        commands
            .execute(
                "toolchain.dismissBanner",
                &mut state,
                json!({"language": "Java"}),
            )
            .unwrap();
        assert_eq!(
            commands
                .execute(
                    "toolchain.banner",
                    &mut state,
                    json!({"path": "src/App.java"})
                )
                .unwrap(),
            Value::Null
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}

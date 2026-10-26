//! The daemon as a registry service: `archipel-ide-core` registers it once
//! (through `analysis_modules`) and reaches it from the host's registry, so
//! no package needs a field in `IdeState`.

use crate::clock::{Clock, SystemClock};
use crate::daemon::Daemon;
use crate::inspection::batch::{BatchEvent, BatchProblem, BatchRun, BatchSummary, start_batch};
use crate::inspection::profile::{
    DEFAULT_PROFILE, EffectiveProfile, Origin, PROJECT_DEFAULT_PROFILE, ProfileError, ProfileFile,
    ProfileStore,
};
use crate::inspection::tier_a;
use crate::passes::{Shared, register_passes};
use archipel_ide_analysis::AnalysisHost;
use archipel_ide_analysis::inspect::AnalysisScope;
use archipel_ide_language::{Registration, ServiceRegistry};
use parking_lot::{Mutex, MutexGuard};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Project-relative directory of inspection profiles.
pub const PROJECT_PROFILE_DIR: &str = ".archipel/inspection-profiles";
/// Directory (below the config dir) of application-level profiles.
pub const APP_PROFILE_DIR: &str = "inspection-profiles";

/// The profiles on disk and which one is active.
#[derive(Debug, Default)]
pub struct ProfileState {
    pub store: ProfileStore,
    pub selected: String,
    /// Where the store was loaded from (reload only when this changes).
    pub source: Option<(Option<PathBuf>, Option<PathBuf>, String)>,
}

/// Results of the last "Inspect Code" run.
#[derive(Default)]
pub struct BatchState {
    run: Option<BatchRun>,
    /// What the run covers (shown in the results tab title).
    pub label: String,
    pub problems: Vec<BatchProblem>,
    pub summary: Option<BatchSummary>,
    pub running: bool,
    /// Bumped whenever results arrive (UI cache key).
    pub version: u64,
}

impl std::fmt::Debug for BatchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchState")
            .field("problems", &self.problems.len())
            .field("running", &self.running)
            .finish()
    }
}

/// A [`Daemon`] behind a mutex plus the state its passes share.
#[derive(Debug)]
pub struct DaemonService {
    daemon: Mutex<Daemon>,
    shared: Arc<Shared>,
    profiles: Mutex<ProfileState>,
    batch: Mutex<BatchState>,
}

impl DaemonService {
    pub fn new(clock: Arc<dyn Clock>) -> Arc<Self> {
        let shared = Arc::new(Shared::default());
        Arc::new(Self {
            daemon: Mutex::new(Daemon::new(clock, shared.clone())),
            shared,
            profiles: Mutex::new(ProfileState {
                store: ProfileStore::new(),
                selected: DEFAULT_PROFILE.to_owned(),
                source: None,
            }),
            batch: Mutex::new(BatchState::default()),
        })
    }

    /// With the system clock.
    pub fn with_system_clock() -> Arc<Self> {
        Self::new(Arc::new(SystemClock::new()))
    }

    /// Registers the built-in passes, the Tier-A inspections and the
    /// service itself.
    pub fn register(self: &Arc<Self>, registry: &mut ServiceRegistry) {
        register_passes(registry, &self.shared);
        tier_a::register(registry);
        registry
            .register::<DaemonService>(Registration::any("daemon").dumb_aware(true), self.clone());
    }

    pub fn lock(&self) -> MutexGuard<'_, Daemon> {
        self.daemon.lock()
    }

    pub fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }

    /// Replaces the active inspection profile.
    pub fn set_profile(&self, profile: EffectiveProfile) {
        *self.shared.profile.write() = Arc::new(profile);
    }

    pub fn profile(&self) -> Arc<EffectiveProfile> {
        self.shared.profile.read().clone()
    }

    pub fn profiles(&self) -> MutexGuard<'_, ProfileState> {
        self.profiles.lock()
    }

    /// Loads the profiles of the application and the project and activates
    /// `selected` (falling back to `Default` with an error when it does not
    /// resolve). Returns `Ok(true)` when something was (re)loaded, `Ok(false)`
    /// when the sources are unchanged; errors are the unreadable files.
    pub fn reload_profiles(
        &self,
        project_root: Option<&Path>,
        config_dir: Option<&Path>,
        selected: &str,
        force: bool,
    ) -> (bool, Vec<ProfileError>) {
        let source = (
            project_root.map(Path::to_path_buf),
            config_dir.map(Path::to_path_buf),
            selected.to_owned(),
        );
        let mut state = self.profiles.lock();
        if !force && state.source.as_ref() == Some(&source) {
            return (false, Vec::new());
        }
        let mut store = ProfileStore::new();
        let mut errors = Vec::new();
        if let Some(dir) = config_dir {
            errors.extend(store.load_dir(&dir.join(APP_PROFILE_DIR), Origin::App));
        }
        if let Some(root) = project_root {
            errors.extend(store.load_dir(&root.join(PROJECT_PROFILE_DIR), Origin::Project));
        }
        let effective = match store.resolve(selected) {
            Ok(profile) => profile,
            Err(error) => {
                errors.push(error);
                store.resolve(DEFAULT_PROFILE).unwrap_or_default()
            }
        };
        state.store = store;
        state.selected = selected.to_owned();
        state.source = Some(source);
        drop(state);
        self.set_profile(effective);
        (true, errors)
    }

    /// Applies `edit` to the active profile's file (creating `Project Default`
    /// when the active profile is built in), saves it and reloads. Returns the
    /// name of the edited profile.
    pub fn edit_profile(
        &self,
        project_root: Option<&Path>,
        config_dir: Option<&Path>,
        edit: impl FnOnce(&mut ProfileFile),
    ) -> Result<String, ProfileError> {
        let (name, origin, dir, mut file) = {
            let state = self.profiles.lock();
            let selected = state.selected.clone();
            let (name, origin) = match state.store.origin(&selected) {
                Some(Origin::BuiltIn) | None => (PROJECT_DEFAULT_PROFILE.to_owned(), None),
                Some(origin) => (selected.clone(), Some(origin)),
            };
            let origin = origin.unwrap_or(if project_root.is_some() {
                Origin::Project
            } else {
                Origin::App
            });
            let dir = match origin {
                Origin::App => config_dir.map(|d| d.join(APP_PROFILE_DIR)),
                _ => project_root.map(|r| r.join(PROJECT_PROFILE_DIR)),
            };
            let file = state.store.get(&name).cloned().unwrap_or_default();
            (name, origin, dir, file)
        };
        let dir =
            dir.ok_or_else(|| ProfileError::Unknown("no project or config directory".into()))?;
        edit(&mut file);
        {
            let mut state = self.profiles.lock();
            state.store.insert(&name, origin, file);
            state.store.save(&name, &dir)?;
        }
        self.reload_profiles(project_root, config_dir, &name, true);
        Ok(name)
    }

    pub fn batch(&self) -> MutexGuard<'_, BatchState> {
        self.batch.lock()
    }

    /// Starts an "Inspect Code" run with the active profile (a running one is
    /// cancelled).
    pub fn start_batch(&self, host: &mut AnalysisHost, scope: AnalysisScope, label: &str) {
        let profile = self.profile();
        let mut batch = self.batch.lock();
        if let Some(run) = batch.run.take() {
            run.cancel();
        }
        batch.problems.clear();
        batch.summary = None;
        batch.label = label.to_owned();
        batch.running = true;
        batch.version += 1;
        batch.run = Some(start_batch(host, scope, profile));
    }

    /// Collects results of the running batch; true when anything arrived.
    pub fn poll_batch(&self) -> bool {
        let mut batch = self.batch.lock();
        let Some(run) = &batch.run else { return false };
        let events = run.drain();
        if events.is_empty() {
            return false;
        }
        for event in events {
            match event {
                BatchEvent::Problem(problem) => batch.problems.push(*problem),
                BatchEvent::Finished(summary) => {
                    batch.summary = Some(summary);
                    batch.running = false;
                    batch.run = None;
                }
                BatchEvent::Cancelled => {
                    batch.running = false;
                    batch.run = None;
                }
            }
        }
        batch.version += 1;
        true
    }

    /// The service registered in `registry`, if any.
    pub fn from_registry(registry: &ServiceRegistry) -> Option<Arc<DaemonService>> {
        registry.get::<DaemonService>(archipel_ide_language::LanguageId::ANY)
    }
}

//! The services that left `IdeState`: the template for moving the remaining fields (see
//! `docs/architecture/platform-kernel.md`).
//!
//! | Service | Scope | Replaces |
//! |---|---|---|
//! | [`ToolchainManager`] | application | `IdeState::toolchains` (discovery, pins, installs) |
//! | [`ToolchainRequirementsService`] | project | `ToolchainState::{requirements, missing, dismissed}` |
//! | [`SettingsService`] | application | `IdeState::settings` |
//! | [`Notifications`] | application | `IdeState::update_notification`, `PluginHostState::notifications` |
//! | [`ProjectModelService`] | project | `IdeState::project_model` (imported modules, roots, libraries) |
//! | [`GitRepositories`] | project | `IdeState::{git, git_watcher, git_pending}` |
//! | [`ExecutionManager`] | project | `IdeState::{runner, build_window, run_configurations, edit_run_configurations, runnable_cache}` |
//! | [`XDebuggerManager`] | project | `IdeState::debugger` (breakpoints, watches, the session) |
//! | [`TerminalService`] | project | `IdeState::{terminal, terminal_tabs}` |
//! | [`SpringStarters`] | application | `IdeState::spring` (starter catalog of the New Project wizard) |
//! | [`FindInProjectService`] | project | `IdeState::{search, search_results, search_query, ...}` (Find in Files) |
//! | [`SearchEverywhereManager`] | application | `IdeState::quick_nav` |
//! | [`ActionManager`] | application | `IdeState::include_disabled_actions` |
//! | [`GitLogService`], [`BlameAnnotations`], [`LineStatusTracker`] | project | `IdeState::{git_log, annotations, line_changes}` |
//! | [`GitRemoteOperations`] | project | `IdeState::{remote_job, remote_progress, remote_error}` |
//! | [`CloneManager`] | application | `IdeState::{clone_job, clone_progress, clone_error}` |
//! | [`KeymapManager`] | application | `IdeState::keymap` |
//! | [`UpdateChecker`] | application | `IdeState::{updates, update_state, update_silent, update_executable, update_installer}` |
//! | [`RecentProjectsManager`] | application | `IdeState::recent_projects` |
//! | [`AgentsManager`] | application | `IdeState::agents` (sessions, thread tabs, review, balloons) |
//! | [`AndroidTooling`] | application | `IdeState::android` (SDK, devices, Logcat, deploy, ...) |

use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, MutexGuard, RwLock, RwLockReadGuard,
    RwLockWriteGuard,
};

/// The state of a run, debug or terminal service. Reads nest: a guard may be held while the same
/// thread reads again (the UI paints from a guard and calls commands that read), so only a write
/// can deadlock, and only against a guard of the same cell that is still alive. In debug builds
/// a write that waits ten seconds panics and names the cell instead of hanging.
pub(crate) struct Cell<T> {
    value: RwLock<T>,
    what: &'static str,
}

impl<T: Default> Default for Cell<T> {
    fn default() -> Self {
        Self::new(T::default(), "state")
    }
}

impl<T> Cell<T> {
    pub(crate) fn new(value: T, what: &'static str) -> Self {
        Self {
            value: RwLock::new(value),
            what,
        }
    }

    pub(crate) fn read(&self) -> RwLockReadGuard<'_, T> {
        self.value.read_recursive()
    }

    pub(crate) fn write(&self) -> RwLockWriteGuard<'_, T> {
        #[cfg(debug_assertions)]
        {
            self.value
                .try_write_for(std::time::Duration::from_secs(10))
                .unwrap_or_else(|| {
                    panic!(
                        "{} is written while a guard of it is alive on this thread",
                        self.what
                    )
                })
        }
        #[cfg(not(debug_assertions))]
        {
            self.value.write()
        }
    }

    pub(crate) fn read_map<U: ?Sized>(
        &self,
        f: impl FnOnce(&T) -> &U,
    ) -> MappedRwLockReadGuard<'_, U> {
        RwLockReadGuard::map(self.read(), f)
    }

    pub(crate) fn write_map<U: ?Sized>(
        &self,
        f: impl FnOnce(&mut T) -> &mut U,
    ) -> MappedRwLockWriteGuard<'_, U> {
        RwLockWriteGuard::map(self.write(), f)
    }
}

/// Locks `mutex`; see [`Cell`] for the debug-build deadlock check.
pub(crate) fn lock<'a, T>(mutex: &'a Mutex<T>, what: &str) -> MutexGuard<'a, T> {
    #[cfg(debug_assertions)]
    {
        mutex
            .try_lock_for(std::time::Duration::from_secs(10))
            .unwrap_or_else(|| panic!("{what} is locked again while a guard of it is alive"))
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = what;
        mutex.lock()
    }
}

mod actions;
mod agents;
mod android;
mod debugger;
mod execution;
mod find;
mod git;
mod keymap;
mod notifications;
mod project_model;
mod recent_projects;
mod remote;
mod search_everywhere;
mod settings;
mod spring;
mod terminal;
mod toolchains;
mod update;
mod vcs;

pub use actions::{ActionManager, ActionOptions};
pub use agents::{AGENTS_TOPIC, AgentsListener, AgentsManager};
pub use android::{ANDROID_TOPIC, AndroidListener, AndroidTooling};
pub(crate) use debugger::Announced;
pub use debugger::{DEBUG_SESSION_TOPIC, DebugSessionListener, XDebuggerManager};
pub use execution::{EXECUTION_TOPIC, ExecutionListener, ExecutionManager};
pub use find::{FIND_TOPIC, FindInProjectService, FindListener, FindState, SearchSummary};
pub use git::{GIT_REPO_CHANGE, GitRepositories, GitRepositoryChangeListener};
pub use keymap::{KeymapManager, KeymapState};
pub use notifications::{
    NOTIFICATION_TOPIC, NotificationListener, NotificationSource, Notifications,
};
pub use project_model::{PROJECT_MODEL_TOPIC, ProjectModelListener, ProjectModelService};
pub use recent_projects::RecentProjectsManager;
pub use remote::{
    CLONE_TOPIC, CloneJob, CloneListener, CloneManager, CloneOutcome, GitRemoteOperations,
    REMOTE_TOPIC, RemoteOperationListener, RemoteOutcome,
};
pub use search_everywhere::SearchEverywhereManager;
pub use settings::{SETTINGS_TOPIC, Settings, SettingsListener, SettingsService};
pub use spring::SpringStarters;
pub use terminal::{TERMINAL_TOPIC, TerminalListener, TerminalService, TerminalState};
pub use toolchains::{
    TOOLCHAIN_TOPIC, ToolchainBanner, ToolchainListener, ToolchainManager, ToolchainRequirements,
    ToolchainRequirementsService, ToolchainState,
};

/// The states live in shared services, so they must be `Sync`.
#[allow(dead_code)]
fn assert_sync() {
    fn sync<T: Sync>() {}
    sync::<archipel_ide_runner::TaskService>();
    sync::<archipel_ide_runner::RunManager>();
    sync::<crate::build_window::BuildWindowState>();
    sync::<crate::gutter_commands::RunnableCache>();
    sync::<crate::debug_commands::DebuggerState>();
    sync::<archipel_ide_terminal::TerminalSession>();
    sync::<crate::AndroidState>();
    sync::<crate::agents::AgentsState>();
}
pub use update::{UPDATE_TOPIC, UpdateChecker, UpdateListener};
pub(crate) use vcs::REBLAME_INTERVAL;
pub use vcs::{
    BlameAnnotations, GitLogService, LINE_STATUS_TOPIC, LineStatusListener, LineStatusTracker,
};

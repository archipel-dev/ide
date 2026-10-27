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
//! | [`FindInProjectService`] | project | `IdeState::{search, search_results, search_query, ...}` (Find in Files) |
//! | [`SearchEverywhereManager`] | application | `IdeState::quick_nav` |
//! | [`ActionManager`] | application | `IdeState::include_disabled_actions` |
//! | [`GitLogService`], [`BlameAnnotations`], [`LineStatusTracker`] | project | `IdeState::{git_log, annotations, line_changes}` |
//! | [`GitRemoteOperations`] | project | `IdeState::{remote_job, remote_progress, remote_error}` |
//! | [`CloneManager`] | application | `IdeState::{clone_job, clone_progress, clone_error}` |
//! | [`KeymapManager`] | application | `IdeState::keymap` |
//! | [`UpdateChecker`] | application | `IdeState::{updates, update_state, update_silent, update_executable, update_installer}` |
//! | [`RecentProjectsManager`] | application | `IdeState::recent_projects` |

mod actions;
mod find;
mod git;
mod keymap;
mod notifications;
mod project_model;
mod recent_projects;
mod remote;
mod search_everywhere;
mod settings;
mod toolchains;
mod update;
mod vcs;

pub use actions::{ActionManager, ActionOptions};
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
pub use toolchains::{
    TOOLCHAIN_TOPIC, ToolchainBanner, ToolchainListener, ToolchainManager, ToolchainRequirements,
    ToolchainRequirementsService, ToolchainState,
};
pub use update::{UPDATE_TOPIC, UpdateChecker, UpdateListener};
pub(crate) use vcs::REBLAME_INTERVAL;
pub use vcs::{
    BlameAnnotations, GitLogService, LINE_STATUS_TOPIC, LineStatusListener, LineStatusTracker,
};

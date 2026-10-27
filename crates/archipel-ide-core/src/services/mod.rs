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

mod git;
mod notifications;
mod project_model;
mod settings;
mod toolchains;

pub use git::{GIT_REPO_CHANGE, GitRepositories, GitRepositoryChangeListener};
pub use notifications::{
    NOTIFICATION_TOPIC, NotificationListener, NotificationSource, Notifications,
};
pub use project_model::{PROJECT_MODEL_TOPIC, ProjectModelListener, ProjectModelService};
pub use settings::{Settings, SettingsService};
pub use toolchains::{
    TOOLCHAIN_TOPIC, ToolchainBanner, ToolchainListener, ToolchainManager, ToolchainRequirements,
    ToolchainRequirementsService, ToolchainState,
};

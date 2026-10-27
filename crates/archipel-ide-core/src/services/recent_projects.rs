//! The welcome screen's list of recently opened projects (IntelliJ: `RecentProjectsManager`).

use archipel_ide_platform::ServiceState;
use archipel_ide_project::recent::RecentProjects;

/// The application service: `application.service::<RecentProjectsManager>()`. It survives
/// projects; the list is loaded from the per-user config directory at startup.
pub type RecentProjectsManager = ServiceState<RecentProjects>;

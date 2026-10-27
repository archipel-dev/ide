//! Search Everywhere, Recent Locations and the Switcher as a project service (IntelliJ:
//! `SearchEverywhereManager`).
//!
//! The service owns what the popups list or cycle (the symbol index, recent caret locations,
//! the Switcher, the language-server symbol answer, the Text tab's background search) and the
//! popup the UI should open next. It dies with the project, so every project starts with an
//! empty index and no locations.

use crate::quick_nav::QuickNavState;
use archipel_ide_platform::ServiceState;

/// The project service: `project.service::<SearchEverywhereManager>()`.
pub type SearchEverywhereManager = ServiceState<QuickNavState>;

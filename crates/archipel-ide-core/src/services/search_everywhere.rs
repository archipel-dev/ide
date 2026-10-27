//! Search Everywhere, Recent Locations and the Switcher as a service (IntelliJ:
//! `SearchEverywhereManager`).
//!
//! The service owns what the popups list or cycle (the symbol index, recent caret locations,
//! the Switcher, the language-server symbol answer, the Text tab's background search) and the
//! popup the UI should open next. It is an application service because the popup also works on
//! the welcome screen; opening a project resets it, so every project starts with an empty index
//! and no locations.

use crate::quick_nav::QuickNavState;
use archipel_ide_platform::ServiceState;

/// The application service: `application.service::<SearchEverywhereManager>()`.
pub type SearchEverywhereManager = ServiceState<QuickNavState>;

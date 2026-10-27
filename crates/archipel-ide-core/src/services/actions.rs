//! Options of Find Action (IntelliJ: `ActionManager` and the "include disabled actions" check
//! box of Search Everywhere's Actions tab).

use archipel_ide_platform::ServiceState;

/// What the Actions tab lists.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActionOptions {
    /// Find Action: list disabled actions too (persisted in the workspace file).
    pub include_disabled: bool,
}

/// The application service: `application.service::<ActionManager>()`.
pub type ActionManager = ServiceState<ActionOptions>;

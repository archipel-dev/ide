//! The Spring Boot starter catalog as an application service.
//!
//! IntelliJ offers Spring Initializr as a project generator of the New Project wizard. Archipel
//! has no generator extension point yet (the first extension points arrive with the language
//! packs), and the wizard runs on the welcome screen where no project exists, so the catalog is
//! a small application service rather than a project service. When the generator extension
//! point lands, this service becomes the Spring generator's backing store and the wizard page
//! becomes its contribution.

use super::lock;
use crate::spring_commands::SpringState;
use parking_lot::{Mutex, MutexGuard};

/// Built-in catalog plus the refresh from the service, shared by the New Project wizard.
#[derive(Default)]
pub struct SpringStarters {
    state: Mutex<SpringState>,
}

impl SpringStarters {
    /// The catalog and the refresh state. Hold the guard briefly.
    pub fn state(&self) -> MutexGuard<'_, SpringState> {
        lock(&self.state, "the Spring catalog")
    }
}

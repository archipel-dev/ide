//! The project model as a project service (IntelliJ: `ModuleManager` and the external system
//! import's `ExternalProjectsManager`).
//!
//! [`ProjectModelService`] owns the imported model, the running import and the auto-import
//! schedule (see [`crate::project_model`] for what drives it). When an import finishes it
//! announces the result on [`PROJECT_MODEL_TOPIC`] of the project bus.

use crate::project_model::ProjectModelState;
use archipel_ide_platform::{Disposable, MessageBus, ServiceState, Topic};
use archipel_ide_projectmodel::ProjectModel;
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};

/// Reports the outcome of an import (IntelliJ `ExternalSystemTaskNotificationListener`).
pub trait ProjectModelListener: Send + Sync {
    /// The model was replaced by a finished import.
    fn model_imported(&self, _model: &ProjectModel) {}

    /// The import failed; the previous model (if any) stays.
    fn import_failed(&self, _error: &str) {}
}

/// On the project's message bus.
pub const PROJECT_MODEL_TOPIC: Topic<dyn ProjectModelListener> = Topic::new("ProjectModelListener");

pub struct ProjectModelService {
    bus: MessageBus,
    state: ServiceState<ProjectModelState>,
}

impl ProjectModelService {
    pub fn new(bus: MessageBus) -> Self {
        Self {
            bus,
            state: ServiceState::default(),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<'_, ProjectModelState> {
        self.state.read()
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, ProjectModelState> {
        self.state.write()
    }

    /// Importing, or a build file change is waiting for its debounce.
    pub fn is_busy(&self) -> bool {
        self.read().is_busy()
    }

    pub fn is_importing(&self) -> bool {
        self.read().is_importing()
    }

    pub fn cancel(&self) {
        self.write().cancel();
    }

    pub fn snapshot(&self, root: &std::path::Path) -> serde_json::Value {
        self.read().snapshot(root)
    }

    pub(crate) fn publish_imported(&self, model: &ProjectModel) {
        self.bus
            .publish(PROJECT_MODEL_TOPIC, |l| l.model_imported(model));
    }

    pub(crate) fn publish_failed(&self, error: &str) {
        self.bus
            .publish(PROJECT_MODEL_TOPIC, |l| l.import_failed(error));
    }
}

impl Disposable for ProjectModelService {
    /// Cancels a running import and forgets the model.
    fn dispose(&self) {
        self.write().reset();
    }
}

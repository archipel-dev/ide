//! `ComponentManager`: what `Application` and `Project` have in common (IntelliJ:
//! `ComponentManager`): lazily created typed services, a message bus and a disposable.

use crate::bus::MessageBus;
use crate::disposer::{DisposableId, Disposer};
use crate::service::{ServiceContainer, ServiceDescriptor, ServiceError};
use std::sync::Arc;

/// An owner of services, a message bus and a place in the disposer tree.
///
/// Services must not store a handle to their owner (that would be a reference cycle); keep a
/// clone of the [`MessageBus`] or the `Arc`s of the services you depend on instead.
pub trait ComponentManager: Sized + 'static {
    #[doc(hidden)]
    fn service_container(&self) -> &Arc<ServiceContainer<Self>>;

    /// This owner's bus. Application topics live on the application's bus, project topics on
    /// the project's.
    fn message_bus(&self) -> &MessageBus;

    /// The node of this owner in the [`Disposer`] tree: register anything that must not
    /// outlive it as a child.
    fn disposable(&self) -> DisposableId;

    fn disposer(&self) -> &Disposer;

    fn is_disposed(&self) -> bool {
        self.disposer().is_disposed(self.disposable())
    }

    /// Declares a service; it is created on first access. Works at any time, so plugins and
    /// language packs add their services when they load.
    fn register_service(&self, descriptor: ServiceDescriptor<Self>) -> Result<(), ServiceError> {
        self.service_container().register(descriptor)
    }

    /// The service `T`, created on first access.
    fn try_service<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, ServiceError> {
        self.service_container().get::<T>(self)
    }

    /// The service `T`, or `None` if it is not registered, depends on itself, or this owner
    /// was disposed (IntelliJ `getService`).
    fn get_service<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.try_service().ok()
    }

    /// The service `T` (IntelliJ `service<T>()`). A missing registration is a programming
    /// error, so this panics with the reason; use [`ComponentManager::get_service`] where the
    /// service is optional.
    fn service<T: Send + Sync + 'static>(&self) -> Arc<T> {
        match self.try_service() {
            Ok(service) => service,
            Err(error) => panic!("{error}"),
        }
    }
}

//! The platform kernel of Archipel IDE: the structure that features plug into, shaped after the
//! IntelliJ Platform.
//!
//! | Here | IntelliJ |
//! |---|---|
//! | [`Application`], [`Project`], [`ProjectManager`] | `Application`, `Project`, `ProjectManager` |
//! | [`ComponentManager::service`], [`ServiceDescriptor`] | `ComponentManager.getService`, `<applicationService>`/`<projectService>` |
//! | [`MessageBus`], [`Topic`], [`MessageBusConnection`] | `MessageBus`, `Topic`, `MessageBusConnection` |
//! | [`ExtensionPointName`], [`ExtensionRegistry`], [`LoadingOrder`] | `ExtensionPointName`, `ExtensionsArea`, `LoadingOrder` |
//! | [`Disposable`], [`Disposer`] | `Disposable`, `Disposer` |
//!
//! No UI toolkit appears here, and there is no global mutable state: everything hangs off an
//! [`Application`] value.

mod application;
mod bus;
mod component;
mod disposer;
mod extension;
mod service;

pub use application::{
    Application, Project, ProjectManager, ProjectManagerListener, WeakApplication,
};
pub use bus::{BusSender, MessageBus, MessageBusConnection, Topic};
pub use component::ComponentManager;
pub use disposer::{Disposable, DisposableId, Disposed, Disposer};
pub use extension::{ExtensionError, ExtensionPointName, ExtensionRegistry, LoadingOrder};
pub use service::{ServiceContainer, ServiceDescriptor, ServiceError, ServiceState};

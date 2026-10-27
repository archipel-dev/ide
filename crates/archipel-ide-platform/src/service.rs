//! The typed service container (IntelliJ: the service half of `ComponentManager`).
//!
//! A service is any `Send + Sync` type, keyed by its Rust type. It is declared with a
//! [`ServiceDescriptor`] (a factory) and created on first access, never earlier. Descriptors
//! can be added at any time, so plugins and language packs register their services when they
//! load. A service that implements [`Disposable`] and was declared with
//! [`ServiceDescriptor::disposable`] is disposed with its owner, newest first.
//!
//! Services share state through `Arc`, so mutable state needs interior mutability;
//! [`ServiceState`] is the usual wrapper for it.

use crate::disposer::{Disposable, DisposableId, Disposer};
use parking_lot::{Mutex, ReentrantMutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::any::{Any, TypeId, type_name};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

type Instance = Arc<dyn Any + Send + Sync>;
type Factory<O> = Arc<dyn Fn(&O) -> Instance + Send + Sync>;
type AsDisposable = fn(Instance) -> Arc<dyn Disposable>;

/// How to create one service of an owner of type `O` ([`Application`](crate::Application) or
/// [`Project`](crate::Project)). The factory receives the owner, so it can fetch the services
/// it depends on; it runs at most once, on first access.
pub struct ServiceDescriptor<O> {
    type_id: TypeId,
    type_name: &'static str,
    factory: Factory<O>,
    as_disposable: Option<AsDisposable>,
}

impl<O> Clone for ServiceDescriptor<O> {
    fn clone(&self) -> Self {
        Self {
            type_id: self.type_id,
            type_name: self.type_name,
            factory: self.factory.clone(),
            as_disposable: self.as_disposable,
        }
    }
}

impl<O: 'static> ServiceDescriptor<O> {
    /// A service created by `factory`.
    pub fn new<T, F>(factory: F) -> Self
    where
        T: Send + Sync + 'static,
        F: Fn(&O) -> T + Send + Sync + 'static,
    {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: type_name::<T>(),
            factory: Arc::new(move |owner| Arc::new(factory(owner))),
            as_disposable: None,
        }
    }

    /// A service created with `T::default()`.
    pub fn of_default<T: Default + Send + Sync + 'static>() -> Self {
        Self::new(|_: &O| T::default())
    }

    /// Like [`ServiceDescriptor::new`], and the service is disposed with its owner.
    pub fn disposable<T, F>(factory: F) -> Self
    where
        T: Disposable + 'static,
        F: Fn(&O) -> T + Send + Sync + 'static,
    {
        let mut descriptor = Self::new(factory);
        descriptor.as_disposable = Some(|instance| {
            instance
                .downcast::<T>()
                .unwrap_or_else(|_| unreachable!("the descriptor created a `T`"))
        });
        descriptor
    }

    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
}

/// Why a service could not be obtained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    /// No descriptor for the type was registered.
    NotRegistered(&'static str),
    /// A descriptor for the type is already registered.
    AlreadyRegistered(&'static str),
    /// The factory of the service (transitively) asked for the service itself.
    Cycle(&'static str),
    /// The owner was disposed; no service is created or handed out any more.
    Disposed(&'static str),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRegistered(name) => write!(f, "service `{name}` is not registered"),
            Self::AlreadyRegistered(name) => write!(f, "service `{name}` is already registered"),
            Self::Cycle(name) => {
                write!(f, "service `{name}` depends on itself while being created")
            }
            Self::Disposed(name) => {
                write!(f, "service `{name}` requested after its owner was disposed")
            }
        }
    }
}

impl std::error::Error for ServiceError {}

struct Slot<O> {
    descriptor: ServiceDescriptor<O>,
    instance: Option<Instance>,
}

/// The services of one owner.
///
/// Creation is serialized per container (a reentrant lock, so a factory may fetch other
/// services of the same owner). Services of an application must not depend on services of a
/// project, which keeps the two creation locks free of lock-order cycles.
pub struct ServiceContainer<O> {
    slots: Mutex<HashMap<TypeId, Slot<O>>>,
    creating: ReentrantMutex<RefCell<Vec<TypeId>>>,
    disposed: AtomicBool,
    disposer: Disposer,
    owner: DisposableId,
}

impl<O: 'static> ServiceContainer<O> {
    /// A container whose disposable services hang under `owner`; disposing `owner` also stops
    /// the container from creating or handing out services.
    pub fn new(disposer: Disposer, owner: DisposableId) -> Arc<Self> {
        let container = Arc::new(Self {
            slots: Mutex::default(),
            creating: ReentrantMutex::new(RefCell::new(Vec::new())),
            disposed: AtomicBool::new(false),
            disposer: disposer.clone(),
            owner,
        });
        let weak = Arc::downgrade(&container);
        // Registered first, so it runs last: services are disposed before the container closes.
        let _ = disposer.register_fn(owner, "services", move || {
            if let Some(container) = weak.upgrade() {
                container.disposed.store(true, Ordering::SeqCst);
                container.slots.lock().clear();
            }
        });
        container
    }

    /// Declares a service. Fails if the type already has a descriptor.
    pub fn register(&self, descriptor: ServiceDescriptor<O>) -> Result<(), ServiceError> {
        let mut slots = self.slots.lock();
        if slots.contains_key(&descriptor.type_id) {
            return Err(ServiceError::AlreadyRegistered(descriptor.type_name));
        }
        slots.insert(
            descriptor.type_id,
            Slot {
                descriptor,
                instance: None,
            },
        );
        Ok(())
    }

    /// Whether a descriptor for `T` exists (the service may not have been created yet).
    pub fn is_registered<T: 'static>(&self) -> bool {
        self.slots.lock().contains_key(&TypeId::of::<T>())
    }

    /// Whether `T` was already created.
    pub fn is_created<T: 'static>(&self) -> bool {
        self.slots
            .lock()
            .get(&TypeId::of::<T>())
            .is_some_and(|slot| slot.instance.is_some())
    }

    /// The service `T`, created on first access. `owner` is the handle passed to the factory.
    pub fn get<T: Send + Sync + 'static>(&self, owner: &O) -> Result<Arc<T>, ServiceError> {
        let id = TypeId::of::<T>();
        let name = type_name::<T>();
        if self.disposed.load(Ordering::SeqCst) {
            return Err(ServiceError::Disposed(name));
        }
        if let Some(found) = self.lookup(id, name)? {
            return downcast(found, name);
        }
        let guard = self.creating.lock();
        // Another thread may have created it while this one waited for the lock.
        if let Some(found) = self.lookup(id, name)? {
            return downcast(found, name);
        }
        if guard.borrow().contains(&id) {
            return Err(ServiceError::Cycle(name));
        }
        let (factory, as_disposable) = {
            let slots = self.slots.lock();
            let slot = slots.get(&id).ok_or(ServiceError::NotRegistered(name))?;
            (
                slot.descriptor.factory.clone(),
                slot.descriptor.as_disposable,
            )
        };
        guard.borrow_mut().push(id);
        let created = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| factory(owner)));
        guard.borrow_mut().pop();
        let instance = match created {
            Ok(instance) => instance,
            Err(panic) => std::panic::resume_unwind(panic),
        };
        {
            let mut slots = self.slots.lock();
            match slots.get_mut(&id) {
                Some(slot) => slot.instance = Some(instance.clone()),
                None => return Err(ServiceError::Disposed(name)),
            }
        }
        if let Some(as_disposable) = as_disposable {
            let _ = self
                .disposer
                .register(self.owner, name, as_disposable(instance.clone()));
        }
        downcast(instance, name)
    }

    fn lookup(&self, id: TypeId, name: &'static str) -> Result<Option<Instance>, ServiceError> {
        match self.slots.lock().get(&id) {
            Some(slot) => Ok(slot.instance.clone()),
            None => Err(ServiceError::NotRegistered(name)),
        }
    }
}

fn downcast<T: Send + Sync + 'static>(
    instance: Instance,
    name: &'static str,
) -> Result<Arc<T>, ServiceError> {
    instance
        .downcast::<T>()
        .map_err(|_| ServiceError::NotRegistered(name))
}

/// Interior-mutable state for a service: many readers or one writer.
///
/// Hold a guard briefly and never across a call that may lock the same state for writing.
/// Reads are reentrant, so nested read access on one thread cannot deadlock.
#[derive(Default)]
pub struct ServiceState<T> {
    value: RwLock<T>,
}

impl<T> ServiceState<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: RwLock::new(value),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.value.read_recursive()
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.value.write()
    }

    /// Runs `f` with shared access.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.read())
    }

    /// Runs `f` with exclusive access.
    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        f(&mut self.write())
    }

    /// Replaces the value, returning the old one (used by tests and by startup code that
    /// configures a service).
    pub fn replace(&self, value: T) -> T {
        std::mem::replace(&mut *self.write(), value)
    }
}

impl<T: fmt::Debug> fmt::Debug for ServiceState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ServiceState").field(&*self.read()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    /// A stand-in owner: the factories only need something to receive.
    struct Owner {
        container: Arc<ServiceContainer<Owner>>,
    }

    impl Owner {
        fn new() -> (Disposer, DisposableId, Arc<Owner>) {
            let disposer = Disposer::new();
            let node = disposer.new_disposable("owner");
            let owner = Arc::new(Owner {
                container: ServiceContainer::new(disposer.clone(), node),
            });
            (disposer, node, owner)
        }

        fn get<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, ServiceError> {
            self.container.get::<T>(self)
        }
    }

    struct Counter(usize);

    #[test]
    fn services_are_created_lazily_and_once() {
        let (_, _, owner) = Owner::new();
        let created = Arc::new(AtomicUsize::new(0));
        let probe = created.clone();
        owner
            .container
            .register(ServiceDescriptor::new(move |_: &Owner| {
                Counter(probe.fetch_add(1, Ordering::SeqCst))
            }))
            .unwrap();
        assert_eq!(
            created.load(Ordering::SeqCst),
            0,
            "registering does not create"
        );
        assert!(owner.container.is_registered::<Counter>());
        assert!(!owner.container.is_created::<Counter>());
        let first = owner.get::<Counter>().unwrap();
        let second = owner.get::<Counter>().unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(first.0, 0);
        assert_eq!(created.load(Ordering::SeqCst), 1);
        assert!(owner.container.is_created::<Counter>());
    }

    #[test]
    fn unknown_and_duplicate_services_are_errors_not_panics() {
        let (_, _, owner) = Owner::new();
        assert!(matches!(
            owner.get::<Counter>(),
            Err(ServiceError::NotRegistered(_))
        ));
        owner
            .container
            .register(ServiceDescriptor::of_default::<ServiceState<u8>>())
            .unwrap();
        assert!(matches!(
            owner
                .container
                .register(ServiceDescriptor::of_default::<ServiceState<u8>>()),
            Err(ServiceError::AlreadyRegistered(_))
        ));
    }

    #[test]
    fn a_factory_can_fetch_other_services() {
        struct Base(u32);
        struct Derived(u32);
        let (_, _, owner) = Owner::new();
        owner
            .container
            .register(ServiceDescriptor::new(|_: &Owner| Base(20)))
            .unwrap();
        owner
            .container
            .register(ServiceDescriptor::new(|owner: &Owner| {
                Derived(owner.get::<Base>().unwrap().0 + 1)
            }))
            .unwrap();
        assert_eq!(owner.get::<Derived>().unwrap().0, 21);
    }

    #[test]
    fn a_dependency_cycle_is_reported() {
        struct A;
        struct B;
        let (_, _, owner) = Owner::new();
        owner
            .container
            .register(ServiceDescriptor::new(|owner: &Owner| {
                owner.get::<B>().unwrap();
                A
            }))
            .unwrap();
        owner
            .container
            .register(ServiceDescriptor::new(|owner: &Owner| {
                let a = owner.get::<A>();
                assert!(
                    matches!(a, Err(ServiceError::Cycle(_))),
                    "A is still being created"
                );
                B
            }))
            .unwrap();
        assert!(owner.get::<A>().is_ok());
    }

    #[test]
    fn services_can_be_registered_after_others_were_created() {
        let (_, _, owner) = Owner::new();
        owner
            .container
            .register(ServiceDescriptor::new(|_: &Owner| Counter(1)))
            .unwrap();
        owner.get::<Counter>().unwrap();
        struct FromPlugin;
        owner
            .container
            .register(ServiceDescriptor::new(|_: &Owner| FromPlugin))
            .unwrap();
        assert!(owner.get::<FromPlugin>().is_ok());
    }

    #[test]
    fn disposable_services_are_disposed_with_the_owner_newest_first() {
        struct Closing(&'static str, Arc<Mutex<Vec<&'static str>>>);
        impl Disposable for Closing {
            fn dispose(&self) {
                self.1.lock().push(self.0);
            }
        }
        struct First(Closing);
        struct Second(Closing);
        impl Disposable for First {
            fn dispose(&self) {
                self.0.dispose();
            }
        }
        impl Disposable for Second {
            fn dispose(&self) {
                self.0.dispose();
            }
        }
        let (disposer, node, owner) = Owner::new();
        let log = Arc::new(Mutex::new(Vec::new()));
        let (l1, l2) = (log.clone(), log.clone());
        owner
            .container
            .register(ServiceDescriptor::disposable(move |_: &Owner| {
                First(Closing("first", l1.clone()))
            }))
            .unwrap();
        owner
            .container
            .register(ServiceDescriptor::disposable(move |_: &Owner| {
                Second(Closing("second", l2.clone()))
            }))
            .unwrap();
        owner.get::<First>().unwrap();
        owner.get::<Second>().unwrap();
        disposer.dispose(node);
        assert_eq!(*log.lock(), ["second", "first"]);
    }

    #[test]
    fn services_never_created_are_not_disposed_and_disposal_stops_access() {
        struct Never;
        impl Disposable for Never {
            fn dispose(&self) {
                panic!("never created, must not be disposed");
            }
        }
        let (disposer, node, owner) = Owner::new();
        owner
            .container
            .register(ServiceDescriptor::disposable(|_: &Owner| Never))
            .unwrap();
        owner
            .container
            .register(ServiceDescriptor::new(|_: &Owner| Counter(0)))
            .unwrap();
        owner.get::<Counter>().unwrap();
        disposer.dispose(node);
        assert!(matches!(
            owner.get::<Counter>(),
            Err(ServiceError::Disposed(_))
        ));
        assert!(matches!(
            owner.get::<Never>(),
            Err(ServiceError::Disposed(_))
        ));
    }

    #[test]
    fn service_state_reads_and_writes() {
        let state = ServiceState::new(vec![1]);
        state.with_mut(|v| v.push(2));
        assert_eq!(state.with(|v| v.len()), 2);
        // A nested read on the same thread is fine.
        let outer = state.read();
        assert_eq!(state.read().len(), outer.len());
        drop(outer);
        assert_eq!(state.replace(vec![9]), vec![1, 2]);
        assert_eq!(*state.read(), vec![9]);
    }

    #[test]
    fn concurrent_first_access_creates_one_instance() {
        let (_, _, owner) = Owner::new();
        let created = Arc::new(AtomicUsize::new(0));
        let probe = created.clone();
        owner
            .container
            .register(ServiceDescriptor::new(move |_: &Owner| {
                probe.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(20));
                Counter(0)
            }))
            .unwrap();
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let owner = owner.clone();
                std::thread::spawn(move || owner.get::<Counter>().unwrap())
            })
            .collect();
        let instances: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert!(
            instances
                .windows(2)
                .all(|pair| Arc::ptr_eq(&pair[0], &pair[1]))
        );
        assert_eq!(created.load(Ordering::SeqCst), 1);
    }
}

//! `Application` and `Project` (IntelliJ: `Application`, `Project`, `ProjectManager`).
//!
//! The [`Application`] lives as long as the process: application services, the application
//! message bus, the extension registry and the [`ProjectManager`]. A [`Project`] is one open
//! project: project services and its own bus. Project services are declared once on the
//! application (`register_project_service`) and instantiated lazily per project.

use crate::bus::{MessageBus, Topic};
use crate::component::ComponentManager;
use crate::disposer::{DisposableId, Disposer};
use crate::extension::ExtensionRegistry;
use crate::service::{ServiceContainer, ServiceDescriptor, ServiceError};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};

struct ApplicationInner {
    disposer: Disposer,
    node: DisposableId,
    services: Arc<ServiceContainer<Application>>,
    bus: MessageBus,
    extensions: ExtensionRegistry,
    project_services: Mutex<Vec<ServiceDescriptor<Project>>>,
    /// Weak: a project is owned by whoever opened it, so dropping it cannot leak through here.
    projects: Mutex<Vec<Weak<ProjectInner>>>,
}

/// The process-wide container. Cloning gives another handle to the same application.
#[derive(Clone)]
pub struct Application {
    inner: Arc<ApplicationInner>,
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Application {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Application")
    }
}

impl Application {
    pub fn new() -> Self {
        let disposer = Disposer::new();
        let node = disposer.new_disposable("Application");
        Self {
            inner: Arc::new(ApplicationInner {
                services: ServiceContainer::new(disposer.clone(), node),
                bus: MessageBus::new(disposer.clone(), "application"),
                extensions: ExtensionRegistry::new(disposer.clone()),
                disposer,
                node,
                project_services: Mutex::default(),
                projects: Mutex::default(),
            }),
        }
    }

    /// A handle that does not keep the application alive. Listeners and services that need
    /// the application later hold this instead of an [`Application`] (which would be a cycle).
    pub fn downgrade(&self) -> WeakApplication {
        WeakApplication(Arc::downgrade(&self.inner))
    }

    pub fn extensions(&self) -> &ExtensionRegistry {
        &self.inner.extensions
    }

    pub fn project_manager(&self) -> ProjectManager {
        ProjectManager { app: self.clone() }
    }

    /// Declares a service of every project, present and future (IntelliJ `<projectService>`).
    /// Fails if the type is already declared.
    pub fn register_project_service(
        &self,
        descriptor: ServiceDescriptor<Project>,
    ) -> Result<(), ServiceError> {
        let mut declared = self.inner.project_services.lock();
        if declared
            .iter()
            .any(|existing| existing.type_name() == descriptor.type_name())
        {
            return Err(ServiceError::AlreadyRegistered(descriptor.type_name()));
        }
        for project in self.project_manager().open_projects() {
            project.register_service(descriptor.clone())?;
        }
        declared.push(descriptor);
        Ok(())
    }

    /// Closes all projects, then disposes the application. Idempotent.
    pub fn dispose(&self) {
        let manager = self.project_manager();
        for project in manager.open_projects() {
            manager.close(&project);
        }
        self.inner.disposer.dispose(self.inner.node);
    }
}

impl ComponentManager for Application {
    fn service_container(&self) -> &Arc<ServiceContainer<Self>> {
        &self.inner.services
    }

    fn message_bus(&self) -> &MessageBus {
        &self.inner.bus
    }

    fn disposable(&self) -> DisposableId {
        self.inner.node
    }

    fn disposer(&self) -> &Disposer {
        &self.inner.disposer
    }
}

/// See [`Application::downgrade`].
#[derive(Clone)]
pub struct WeakApplication(Weak<ApplicationInner>);

impl WeakApplication {
    /// The application, unless every strong handle is gone.
    pub fn upgrade(&self) -> Option<Application> {
        self.0.upgrade().map(|inner| Application { inner })
    }
}

struct ProjectInner {
    application: Application,
    root: PathBuf,
    name: String,
    node: DisposableId,
    services: Arc<ServiceContainer<Project>>,
    bus: MessageBus,
}

impl Drop for ProjectInner {
    fn drop(&mut self) {
        // A project that was dropped without `ProjectManager::close` still releases its
        // resources (no listeners run).
        self.application.inner.disposer.dispose(self.node);
    }
}

/// One open project. Cloning gives another handle to the same project.
#[derive(Clone)]
pub struct Project {
    inner: Arc<ProjectInner>,
}

impl PartialEq for Project {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for Project {}

impl std::fmt::Debug for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Project({})", self.inner.root.display())
    }
}

impl Project {
    fn new(application: &Application, root: PathBuf) -> Self {
        let disposer = application.inner.disposer.clone();
        let name = root.file_name().map_or_else(
            || root.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let node = disposer
            .new_child(application.inner.node, &format!("Project {name}"))
            .unwrap_or_else(|_| disposer.new_disposable(&format!("Project {name} (orphan)")));
        let project = Self {
            inner: Arc::new(ProjectInner {
                application: application.clone(),
                services: ServiceContainer::new(disposer.clone(), node),
                bus: MessageBus::new(disposer, &format!("project {name}")),
                root,
                name,
                node,
            }),
        };
        for descriptor in application.inner.project_services.lock().iter() {
            let _ = project.register_service(descriptor.clone());
        }
        project
    }

    pub fn application(&self) -> &Application {
        &self.inner.application
    }

    pub fn root(&self) -> &Path {
        &self.inner.root
    }

    /// The directory name of the root.
    pub fn name(&self) -> &str {
        &self.inner.name
    }
}

impl ComponentManager for Project {
    fn service_container(&self) -> &Arc<ServiceContainer<Self>> {
        &self.inner.services
    }

    fn message_bus(&self) -> &MessageBus {
        &self.inner.bus
    }

    fn disposable(&self) -> DisposableId {
        self.inner.node
    }

    fn disposer(&self) -> &Disposer {
        &self.inner.application.inner.disposer
    }
}

/// Listens to projects opening and closing (IntelliJ `ProjectManagerListener`), on
/// [`ProjectManager::TOPIC`] of the application bus.
pub trait ProjectManagerListener: Send + Sync {
    /// The project exists and its services can be used.
    fn project_opened(&self, _project: &Project) {}
    /// The project is about to be disposed; its services are still alive.
    fn project_closing(&self, _project: &Project) {}
    /// The project was disposed.
    fn project_closed(&self, _project: &Project) {}
}

/// Opens and closes projects (IntelliJ `ProjectManager`).
#[derive(Clone)]
pub struct ProjectManager {
    app: Application,
}

impl ProjectManager {
    pub const TOPIC: Topic<dyn ProjectManagerListener> = Topic::new("ProjectManagerListener");

    /// Creates the project for `root` and announces it with `project_opened`. Nothing in the
    /// project (services, indexes, language servers) exists before it is first used.
    pub fn open(&self, root: impl Into<PathBuf>) -> Project {
        let project = Project::new(&self.app, root.into());
        self.app
            .inner
            .projects
            .lock()
            .push(Arc::downgrade(&project.inner));
        self.app
            .message_bus()
            .publish(Self::TOPIC, |l| l.project_opened(&project));
        project
    }

    /// Announces `project_closing`, disposes the project (its services and subscriptions), and
    /// announces `project_closed`. Returns false if the project was closed before.
    pub fn close(&self, project: &Project) -> bool {
        if project.is_disposed() {
            return false;
        }
        self.app
            .message_bus()
            .publish(Self::TOPIC, |l| l.project_closing(project));
        let closed = project.disposer().dispose(project.disposable());
        self.app.inner.projects.lock().retain(|open| {
            open.upgrade()
                .is_some_and(|open| !Arc::ptr_eq(&open, &project.inner))
        });
        if closed {
            self.app
                .message_bus()
                .publish(Self::TOPIC, |l| l.project_closed(project));
        }
        closed
    }

    /// The projects that were opened and not closed or dropped.
    pub fn open_projects(&self) -> Vec<Project> {
        self.app
            .inner
            .projects
            .lock()
            .iter()
            .filter_map(Weak::upgrade)
            .map(|inner| Project { inner })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disposer::Disposable;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct Events(Mutex<Vec<String>>);

    impl ProjectManagerListener for Events {
        fn project_opened(&self, project: &Project) {
            self.0.lock().push(format!("opened {}", project.name()));
        }
        fn project_closing(&self, project: &Project) {
            assert!(
                !project.is_disposed(),
                "services are still alive while closing"
            );
            self.0.lock().push(format!("closing {}", project.name()));
        }
        fn project_closed(&self, project: &Project) {
            assert!(project.is_disposed());
            self.0.lock().push(format!("closed {}", project.name()));
        }
    }

    fn listen(app: &Application) -> Arc<Events> {
        let events = Arc::new(Events::default());
        app.message_bus()
            .subscribe(app.disposable(), ProjectManager::TOPIC, events.clone())
            .unwrap();
        events
    }

    #[test]
    fn open_and_close_fire_the_project_manager_topics_in_order() {
        let app = Application::new();
        let events = listen(&app);
        let manager = app.project_manager();
        let project = manager.open("/work/demo");
        assert_eq!(project.name(), "demo");
        assert_eq!(project.root(), Path::new("/work/demo"));
        assert_eq!(manager.open_projects(), std::slice::from_ref(&project));
        assert!(manager.close(&project));
        assert!(!manager.close(&project), "closing twice does nothing");
        assert!(manager.open_projects().is_empty());
        assert_eq!(
            *events.0.lock(),
            ["opened demo", "closing demo", "closed demo"]
        );
    }

    #[test]
    fn application_services_are_lazy_and_shared_by_projects() {
        struct Settings(usize);
        let app = Application::new();
        let created = Arc::new(AtomicUsize::new(0));
        let probe = created.clone();
        app.register_service(ServiceDescriptor::new(move |_: &Application| {
            Settings(probe.fetch_add(1, Ordering::SeqCst))
        }))
        .unwrap();
        app.register_project_service(ServiceDescriptor::new(|project: &Project| {
            // A project service reaches application services through its application.
            project.application().service::<Settings>().0 + 100
        }))
        .unwrap();
        let project = app.project_manager().open("/p");
        assert_eq!(
            created.load(Ordering::SeqCst),
            0,
            "opening creates no service"
        );
        assert_eq!(*project.service::<usize>(), 100);
        assert_eq!(created.load(Ordering::SeqCst), 1);
        assert_eq!(
            app.service::<Settings>().0,
            0,
            "same instance as the project saw"
        );
    }

    #[test]
    fn project_services_are_per_project_and_disposed_with_it() {
        struct Index(Arc<AtomicUsize>);
        impl Disposable for Index {
            fn dispose(&self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let app = Application::new();
        let disposed = Arc::new(AtomicUsize::new(0));
        let shared = disposed.clone();
        app.register_project_service(ServiceDescriptor::disposable(move |_: &Project| {
            Index(shared.clone())
        }))
        .unwrap();
        let manager = app.project_manager();
        let (a, b) = (manager.open("/a"), manager.open("/b"));
        assert!(!Arc::ptr_eq(&a.service::<Index>(), &b.service::<Index>()));
        manager.close(&a);
        assert_eq!(disposed.load(Ordering::SeqCst), 1);
        assert!(
            a.get_service::<Index>().is_none(),
            "no services after close"
        );
        assert!(b.get_service::<Index>().is_some());
    }

    #[test]
    fn a_project_service_declared_later_reaches_open_projects() {
        struct Late;
        let app = Application::new();
        let project = app.project_manager().open("/p");
        assert!(project.get_service::<Late>().is_none());
        app.register_project_service(ServiceDescriptor::new(|_: &Project| Late))
            .unwrap();
        assert!(project.get_service::<Late>().is_some());
        assert!(
            app.project_manager()
                .open("/q")
                .get_service::<Late>()
                .is_some()
        );
        assert!(
            app.register_project_service(ServiceDescriptor::new(|_: &Project| Late))
                .is_err()
        );
    }

    #[test]
    fn project_subscriptions_end_with_the_project() {
        trait Saved: Send + Sync {
            fn saved(&self);
        }
        const SAVED: Topic<dyn Saved> = Topic::new("saved");
        struct Count(Arc<AtomicUsize>);
        impl Saved for Count {
            fn saved(&self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let app = Application::new();
        let manager = app.project_manager();
        let project = manager.open("/p");
        let count = Arc::new(AtomicUsize::new(0));
        project
            .message_bus()
            .subscribe(project.disposable(), SAVED, Arc::new(Count(count.clone())))
            .unwrap();
        project.message_bus().publish(SAVED, |l| l.saved());
        manager.close(&project);
        project.message_bus().publish(SAVED, |l| l.saved());
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(
            app.message_bus().subscriber_count(SAVED),
            0,
            "buses are separate"
        );
    }

    #[test]
    fn a_dropped_project_releases_its_resources() {
        struct Watcher(Arc<AtomicUsize>);
        impl Disposable for Watcher {
            fn dispose(&self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let app = Application::new();
        let disposed = Arc::new(AtomicUsize::new(0));
        let shared = disposed.clone();
        app.register_project_service(ServiceDescriptor::disposable(move |_: &Project| {
            Watcher(shared.clone())
        }))
        .unwrap();
        {
            let project = app.project_manager().open("/p");
            project.service::<Watcher>();
        }
        assert_eq!(disposed.load(Ordering::SeqCst), 1);
        assert!(app.project_manager().open_projects().is_empty());
    }

    #[test]
    fn disposing_the_application_closes_projects_first() {
        let app = Application::new();
        let events = listen(&app);
        let project = app.project_manager().open("/p");
        app.dispose();
        assert!(app.is_disposed());
        assert!(project.is_disposed());
        assert_eq!(*events.0.lock(), ["opened p", "closing p", "closed p"]);
        assert!(app.get_service::<usize>().is_none());
    }

    #[test]
    fn a_weak_handle_does_not_keep_the_application_alive() {
        let app = Application::new();
        let weak = app.downgrade();
        assert!(weak.upgrade().is_some());
        drop(app);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    #[should_panic(expected = "not registered")]
    fn service_panics_for_an_unregistered_type() {
        Application::new().service::<String>();
    }

    #[test]
    fn extension_points_live_on_the_application() {
        use crate::extension::{ExtensionPointName, LoadingOrder};
        trait Language: Send + Sync {}
        struct Java;
        impl Language for Java {}
        const LANGUAGES: ExtensionPointName<dyn Language> = ExtensionPointName::new("language");
        let app = Application::new();
        app.extensions()
            .register(
                LANGUAGES,
                "java",
                LoadingOrder::Anywhere,
                Arc::new(Java),
                Some(app.disposable()),
            )
            .unwrap();
        assert_eq!(app.extensions().ids(LANGUAGES), ["java"]);
        app.dispose();
        assert!(app.extensions().ids(LANGUAGES).is_empty());
    }
}

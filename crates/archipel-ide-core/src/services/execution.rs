//! Running things as a project service (IntelliJ: `ExecutionManager` and `RunManager`).
//!
//! [`ExecutionManager`] owns what belongs to running and building the open project: the task
//! service that runs one child process at a time, the run/debug configurations and the run
//! widget selection, the Build tool window's tabs, the gutter run targets of the active
//! document and the pending request to show the Run/Debug Configurations dialog. Closing the
//! project disposes the manager, which stops the running process.
//!
//! Starting, ending and failing a run are events: they are announced on [`EXECUTION_TOPIC`] of
//! the project bus. The Build tool window and the status bar follow the topic instead of being
//! called by whoever starts a task.

use super::Cell;
use crate::build_window::BuildWindowState;
use crate::gutter_commands::RunnableCache;
use archipel_ide_platform::{Disposable, MessageBus, Topic};
use archipel_ide_runner::{RunManager, RunnerError, TaskPlan, TaskService, TaskState};
use parking_lot::{Mutex, RwLockReadGuard, RwLockWriteGuard};

/// Reports the life of the runner's tasks (IntelliJ `ExecutionListener`).
pub trait ExecutionListener: Send + Sync {
    /// A task was started, or recorded as failed before it could start. `run_id` identifies the
    /// run (`TaskService::run_id`).
    fn process_started(&self, _plan: &TaskPlan, _run_id: u64) {}

    /// The task of `run_id` ended: it exited (`TaskState::Exited`), was cancelled, or never
    /// started (`TaskState::Failed`). Published after the manager released its locks, so
    /// listeners may read the runner.
    fn process_terminated(&self, _plan: &TaskPlan, _state: &TaskState, _run_id: u64) {}
}

/// On the project's message bus.
pub const EXECUTION_TOPIC: Topic<dyn ExecutionListener> = Topic::new("ExecutionListener");

/// The project service. Each part has its own cell, so reading the run configurations never
/// waits for the runner. Reads nest; hold a write guard briefly (see [`Cell`]).
pub struct ExecutionManager {
    bus: MessageBus,
    runner: Cell<TaskService>,
    build_window: Cell<BuildWindowState>,
    configurations: Cell<RunManager>,
    edit_request: Mutex<Option<String>>,
    runnables: Cell<RunnableCache>,
}

impl ExecutionManager {
    pub fn new(bus: MessageBus) -> Self {
        Self {
            bus,
            runner: Cell::new(TaskService::default(), "the task service"),
            build_window: Cell::new(BuildWindowState::default(), "the Build tool window state"),
            configurations: Cell::new(RunManager::default(), "the run configurations"),
            edit_request: Mutex::default(),
            runnables: Cell::new(RunnableCache::default(), "the gutter run targets"),
        }
    }

    /// The task service: the running or last task, its console, events and test results.
    /// Start, cancel and poll through the manager so the events are published.
    pub fn runner(&self) -> RwLockReadGuard<'_, TaskService> {
        self.runner.read()
    }

    /// Tabs and console toolbar of the Build tool window.
    pub fn build_window(&self) -> RwLockReadGuard<'_, BuildWindowState> {
        self.build_window.read()
    }

    pub fn build_window_mut(&self) -> RwLockWriteGuard<'_, BuildWindowState> {
        self.build_window.write()
    }

    /// Run/Debug configurations of the project and the run widget selection.
    pub fn configurations(&self) -> RwLockReadGuard<'_, RunManager> {
        self.configurations.read()
    }

    pub fn configurations_mut(&self) -> RwLockWriteGuard<'_, RunManager> {
        self.configurations.write()
    }

    /// Gutter run targets of the active document, cached per text revision.
    pub fn runnables(&self) -> RwLockReadGuard<'_, RunnableCache> {
        self.runnables.read()
    }

    pub fn runnables_mut(&self) -> RwLockWriteGuard<'_, RunnableCache> {
        self.runnables.write()
    }

    pub fn is_running(&self) -> bool {
        self.runner.read().is_running()
    }

    /// Asks the UI to show the Run/Debug Configurations dialog, focusing `name`.
    pub fn request_edit_configurations(&self, name: impl Into<String>) {
        *self.edit_request.lock() = Some(name.into());
    }

    /// Whether the dialog was requested and not shown yet.
    pub fn edit_configurations_requested(&self) -> bool {
        self.edit_request.lock().is_some()
    }

    /// Takes the pending dialog request (the focused configuration name).
    pub fn take_edit_configurations_request(&self) -> Option<String> {
        self.edit_request.lock().take()
    }

    /// Starts `plan`, replacing a running task. A task that cannot start is recorded as
    /// failed, like in the task service, and announced as started and terminated.
    pub fn start(&self, plan: TaskPlan) -> Result<(), RunnerError> {
        let result = self.runner.write().start(plan);
        self.announce_new_run();
        result
    }

    /// Records a task that could not even be planned, so the failure is visible in the Build
    /// tool window.
    pub fn fail(&self, plan: Option<TaskPlan>, message: impl Into<String>) {
        let before = self.runner.read().run_id();
        self.runner.write().fail(plan, message);
        if self.runner.read().run_id() != before {
            self.announce_new_run();
        }
    }

    /// Stops the running task, if any.
    pub fn cancel(&self) -> Result<(), RunnerError> {
        let was_running = self.is_running();
        let result = self.runner.write().cancel();
        if was_running {
            self.announce_end();
        }
        result
    }

    /// Pumps the running task's output; announces its end when it exited.
    pub fn poll(&self) -> Result<(), RunnerError> {
        let (result, ended) = {
            let mut runner = self.runner.write();
            let was_running = runner.is_running();
            let result = runner.poll();
            (result, was_running && !runner.is_running())
        };
        if ended {
            self.announce_end();
        }
        result
    }

    fn announce_new_run(&self) {
        let Some((plan, run_id, running)) = self.snapshot() else {
            return;
        };
        self.bus
            .publish(EXECUTION_TOPIC, |l| l.process_started(&plan, run_id));
        if !running {
            self.announce_end();
        }
    }

    fn announce_end(&self) {
        let (plan, state, run_id) = {
            let runner = self.runner.read();
            let Some(plan) = runner.plan().cloned() else {
                return;
            };
            (plan, runner.state().clone(), runner.run_id())
        };
        self.bus.publish(EXECUTION_TOPIC, |l| {
            l.process_terminated(&plan, &state, run_id)
        });
    }

    fn snapshot(&self) -> Option<(TaskPlan, u64, bool)> {
        let runner = self.runner.read();
        Some((runner.plan()?.clone(), runner.run_id(), runner.is_running()))
    }
}

impl Disposable for ExecutionManager {
    /// Stops the running process and forgets the project's runs and configurations.
    fn dispose(&self) {
        *self.runner.write() = TaskService::default();
        *self.build_window.write() = BuildWindowState::default();
        *self.configurations.write() = RunManager::default();
        *self.edit_request.lock() = None;
        *self.runnables.write() = RunnableCache::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_platform::{Application, ComponentManager, ServiceDescriptor};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    #[derive(Default)]
    struct Heard(Mutex<Vec<String>>);

    impl ExecutionListener for Heard {
        fn process_started(&self, plan: &TaskPlan, run_id: u64) {
            self.0
                .lock()
                .push(format!("started {} #{run_id}", plan.name));
        }
        fn process_terminated(&self, plan: &TaskPlan, state: &TaskState, run_id: u64) {
            self.0
                .lock()
                .push(format!("terminated {} #{run_id} {state:?}", plan.name));
        }
    }

    fn manager() -> (Application, Arc<ExecutionManager>, Arc<Heard>) {
        let application = Application::new();
        application
            .register_service(ServiceDescriptor::disposable(|app: &Application| {
                ExecutionManager::new(app.message_bus().clone())
            }))
            .unwrap();
        let heard = Arc::new(Heard::default());
        application
            .message_bus()
            .subscribe(application.disposable(), EXECUTION_TOPIC, heard.clone())
            .unwrap();
        let manager = application.service::<ExecutionManager>();
        (application, manager, heard)
    }

    fn wait(manager: &ExecutionManager) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while manager.is_running() && Instant::now() < deadline {
            manager.poll().unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
        manager.poll().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_run_is_announced_when_it_starts_and_when_it_exits() {
        let (_application, manager, heard) = manager();
        manager
            .start(TaskPlan::new("job", "sh", ["-c", "exit 3"], "."))
            .unwrap();
        assert_eq!(*heard.0.lock(), ["started job #1"]);
        wait(&manager);
        assert_eq!(
            *heard.0.lock(),
            ["started job #1", "terminated job #1 Exited(3)"]
        );
        manager.poll().unwrap();
        assert_eq!(heard.0.lock().len(), 2, "an exit is announced once");
    }

    #[cfg(unix)]
    #[test]
    fn cancelling_and_failing_to_start_are_announced() {
        let (_application, manager, heard) = manager();
        manager
            .start(TaskPlan::new("long", "sleep", ["30"], "."))
            .unwrap();
        manager.cancel().unwrap();
        manager.cancel().unwrap();
        assert!(
            manager
                .start(TaskPlan::new("nope", "no-such-program-here", ["x"], "."))
                .is_err()
        );
        manager.fail(None, "nothing to build");
        let heard = heard.0.lock();
        assert_eq!(heard.len(), 6, "{heard:?}");
        assert_eq!(heard[0], "started long #1");
        assert_eq!(heard[1], "terminated long #1 Cancelled");
        assert_eq!(heard[2], "started nope #2");
        assert!(heard[3].starts_with("terminated nope #2 Failed("));
        assert_eq!(heard[4], "started Build #3");
        assert_eq!(heard[5], "terminated Build #3 Failed(\"nothing to build\")");
    }

    #[cfg(unix)]
    #[test]
    fn disposing_the_manager_stops_the_running_process() {
        let (application, manager, _) = manager();
        manager
            .start(TaskPlan::new("long", "sleep", ["30"], "."))
            .unwrap();
        assert!(manager.is_running());
        application.dispose();
        assert!(!manager.is_running());
        assert!(manager.runner().plan().is_none());
    }

    #[test]
    fn the_dialog_request_is_taken_once() {
        let (_application, manager, _) = manager();
        assert!(!manager.edit_configurations_requested());
        manager.request_edit_configurations("App");
        assert!(manager.edit_configurations_requested());
        assert_eq!(
            manager.take_edit_configurations_request().as_deref(),
            Some("App")
        );
        assert_eq!(manager.take_edit_configurations_request(), None);
    }
}

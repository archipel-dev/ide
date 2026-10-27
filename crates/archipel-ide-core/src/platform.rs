//! The platform kernel inside the IDE: [`Platform`] owns the [`Application`] and the current
//! [`Project`], registers the core services, and routes events between features.
//!
//! `IdeState` reaches migrated features only through the accessors below
//! (`state.toolchains()`, `state.settings()`, `state.git()`, ...), which are thin wrappers over
//! `application().service::<T>()` and `platform_project().service::<T>()`.
//!
//! Features that still live on `IdeState` need `&mut IdeState` to react to an event, which a
//! bus listener cannot have. Their listeners therefore only record a [`Reaction`];
//! [`IdeState::dispatch_reactions`] runs them right after the event was published. When a
//! feature becomes a service, its listener does the work itself and the reaction disappears.

use crate::build_window::BuildWindowFollower;
use crate::services::{
    DEBUG_SESSION_TOPIC, DebugSessionListener, EXECUTION_TOPIC, ExecutionListener,
    ExecutionManager, GIT_REPO_CHANGE, GitRepositories, GitRepositoryChangeListener, Notifications,
    ProjectModelService, SettingsService, SpringStarters, TOOLCHAIN_TOPIC, TerminalService,
    ToolchainBanner, ToolchainListener, ToolchainManager, ToolchainRequirementsService,
    ToolchainState, XDebuggerManager,
};
use crate::{IdeState, toolchain_commands};
use archipel_ide_dap::StackFrame;
use archipel_ide_git::RepositoryStatus;
use archipel_ide_platform::{
    Application, ComponentManager, Disposable, DisposableId, MessageBus, Project, ProjectManager,
    ProjectManagerListener, ServiceDescriptor, WeakApplication,
};
use archipel_ide_runner::{TaskPlan, TaskState};
use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;

/// Work a listener asks `IdeState` to do (see the module documentation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reaction {
    /// A toolchain component finished installing.
    ToolchainInstalled,
    /// The Git status was replaced.
    VcsChanged { structure_changed: bool },
    /// A task exited on its own (not cancelled, not failed to start).
    RunExited,
    /// The debug session stopped at a new execution point.
    DebugPaused(StackFrame),
}

#[derive(Clone, Default)]
pub(crate) struct Reactions(Arc<Mutex<Vec<Reaction>>>);

impl Reactions {
    fn push(&self, reaction: Reaction) {
        self.0.lock().push(reaction);
    }

    fn take(&self) -> Vec<Reaction> {
        std::mem::take(&mut *self.0.lock())
    }
}

/// Keeps JDKs and build tools that were just installed visible to builds, and asks `IdeState`
/// to restart what waited for them.
struct ToolchainReactions {
    application: WeakApplication,
    reactions: Reactions,
}

impl ToolchainListener for ToolchainReactions {
    fn component_installed(&self, _component: &str, _version: &str) {
        if let Some(application) = self.application.upgrade() {
            toolchain_commands::sync_tool_environment(
                &application.service::<ToolchainManager>().read(),
            );
        }
        self.reactions.push(Reaction::ToolchainInstalled);
    }
}

struct VcsReactions(Reactions);

impl GitRepositoryChangeListener for VcsReactions {
    fn repository_changed(&self, _status: &RepositoryStatus, structure_changed: bool) {
        self.0.push(Reaction::VcsChanged { structure_changed });
    }
}

/// Asks `IdeState` for what follows a finished task: the pending debug session, the end of a
/// sync and the status bar text.
struct RunReactions(Reactions);

impl ExecutionListener for RunReactions {
    fn process_terminated(&self, _plan: &TaskPlan, state: &TaskState, _run_id: u64) {
        if matches!(state, TaskState::Exited(_)) {
            self.0.push(Reaction::RunExited);
        }
    }
}

/// Asks `IdeState` to reveal the execution point in the editor.
struct DebugReactions(Reactions);

impl DebugSessionListener for DebugReactions {
    fn session_paused(&self, _name: &str, frame: &StackFrame) {
        self.0.push(Reaction::DebugPaused(frame.clone()));
    }
}

/// Subscribes what follows runs and debug sessions on `bus` until `parent` is disposed: the
/// Build tool window follows the runner, `IdeState` reacts to finished tasks and stops.
fn connect_run_listeners(
    bus: &MessageBus,
    parent: DisposableId,
    execution: &Arc<ExecutionManager>,
    reactions: &Reactions,
) {
    let _ = bus.subscribe(
        parent,
        EXECUTION_TOPIC,
        Arc::new(BuildWindowFollower(Arc::downgrade(execution))),
    );
    let _ = bus.subscribe(
        parent,
        EXECUTION_TOPIC,
        Arc::new(RunReactions(reactions.clone())),
    );
    let _ = bus.subscribe(
        parent,
        DEBUG_SESSION_TOPIC,
        Arc::new(DebugReactions(reactions.clone())),
    );
}

/// Connects the per-project listeners when a project opens.
struct CoreProjectListener(Reactions);

impl ProjectManagerListener for CoreProjectListener {
    fn project_opened(&self, project: &Project) {
        let _ = project.message_bus().subscribe(
            project.disposable(),
            GIT_REPO_CHANGE,
            Arc::new(VcsReactions(self.0.clone())),
        );
        connect_run_listeners(
            project.message_bus(),
            project.disposable(),
            &project.service::<ExecutionManager>(),
            &self.0,
        );
    }
}

fn register_core_services(application: &Application) {
    let registered = [
        application.register_service(ServiceDescriptor::new(|_: &Application| {
            ToolchainManager::new(ToolchainState::default())
        })),
        application.register_service(ServiceDescriptor::of_default::<SettingsService>()),
        application.register_service(ServiceDescriptor::new(|application: &Application| {
            Notifications::new(application.message_bus().clone())
        })),
        application.register_project_service(ServiceDescriptor::of_default::<
            ToolchainRequirementsService,
        >()),
        application.register_project_service(ServiceDescriptor::disposable(|project: &Project| {
            GitRepositories::new(project.message_bus().clone())
        })),
        application.register_project_service(ServiceDescriptor::disposable(|project: &Project| {
            ProjectModelService::new(project.message_bus().clone())
        })),
        application.register_project_service(ServiceDescriptor::disposable(|project: &Project| {
            ExecutionManager::new(project.message_bus().clone())
        })),
        application.register_project_service(ServiceDescriptor::disposable(|project: &Project| {
            XDebuggerManager::new(project.message_bus().clone())
        })),
        application.register_project_service(ServiceDescriptor::disposable(|project: &Project| {
            TerminalService::new(project.message_bus().clone())
        })),
        application.register_service(ServiceDescriptor::of_default::<SpringStarters>()),
    ];
    debug_assert!(
        registered.iter().all(Result::is_ok),
        "core services are unique"
    );
}

/// The run, debug and terminal services of a window without a project (the welcome screen).
///
/// Those are project services, but their accessors are used all over the UI, which draws before
/// any project is open. Instead of an `Option` at every call site the accessors fall back to
/// this set. It never belongs to a project: a project gets fresh services when it opens, and
/// nothing started here (there is no project to run in) outlives the window.
struct Detached {
    execution: Arc<ExecutionManager>,
    debugger: Arc<XDebuggerManager>,
    terminal: Arc<TerminalService>,
}

impl Detached {
    fn new(application: &Application, reactions: &Reactions) -> Self {
        let bus = application.message_bus();
        let detached = Self {
            execution: Arc::new(ExecutionManager::new(bus.clone())),
            debugger: Arc::new(XDebuggerManager::new(bus.clone())),
            terminal: Arc::new(TerminalService::new(bus.clone())),
        };
        connect_run_listeners(
            bus,
            application.disposable(),
            &detached.execution,
            reactions,
        );
        detached
    }
}

impl Detached {
    /// Stops what ran without a project (a shell opened on the welcome screen).
    fn reset(&self) {
        self.execution.dispose();
        self.debugger.dispose();
        self.terminal.dispose();
    }
}

impl Drop for Detached {
    fn drop(&mut self) {
        self.reset();
    }
}

/// The application and the open project of this IDE window.
pub struct Platform {
    application: Application,
    project: Option<Project>,
    reactions: Reactions,
    detached: Detached,
}

impl Default for Platform {
    fn default() -> Self {
        let application = Application::new();
        register_core_services(&application);
        let reactions = Reactions::default();
        let bus = application.message_bus();
        let owner = application.disposable();
        let _ = bus.subscribe(
            owner,
            TOOLCHAIN_TOPIC,
            Arc::new(ToolchainReactions {
                application: application.downgrade(),
                reactions: reactions.clone(),
            }),
        );
        let _ = bus.subscribe(
            owner,
            ProjectManager::TOPIC,
            Arc::new(CoreProjectListener(reactions.clone())),
        );
        let detached = Detached::new(&application, &reactions);
        Self {
            application,
            project: None,
            reactions,
            detached,
        }
    }
}

impl Drop for Platform {
    fn drop(&mut self) {
        self.application.dispose();
    }
}

impl Platform {
    pub fn application(&self) -> &Application {
        &self.application
    }

    pub fn project(&self) -> Option<&Project> {
        self.project.as_ref()
    }

    /// Closes the current project (`project_closing`, dispose, `project_closed`) and opens
    /// `root` (`project_opened`).
    pub(crate) fn open_project(&mut self, root: &Path) -> &Project {
        self.close_project();
        self.detached.reset();
        self.project
            .insert(self.application.project_manager().open(root))
    }

    pub(crate) fn close_project(&mut self) {
        if let Some(project) = self.project.take() {
            self.application.project_manager().close(&project);
        }
    }
}

impl IdeState {
    /// The application: its services, message bus and extension points.
    pub fn application(&self) -> &Application {
        self.platform.application()
    }

    /// The open project of the platform kernel (`None` on the welcome screen). Not to be
    /// confused with `IdeState::project`, the file model of the project root.
    pub fn platform_project(&self) -> Option<&Project> {
        self.platform.project()
    }

    /// Settings (application service).
    pub fn settings(&self) -> Arc<SettingsService> {
        self.application().service()
    }

    /// Toolchain discovery, pins and installs (application service).
    pub fn toolchains(&self) -> Arc<ToolchainManager> {
        self.application().service()
    }

    /// Notification balloons (application service).
    pub fn notifications(&self) -> Arc<Notifications> {
        self.application().service()
    }

    /// What the open project needs from the toolchains (project service).
    pub fn toolchain_requirements(&self) -> Option<Arc<ToolchainRequirementsService>> {
        self.platform_project()?.get_service()
    }

    /// Imported modules, roots and libraries of the open project (project service).
    pub fn project_model(&self) -> Option<Arc<ProjectModelService>> {
        self.platform_project()?.get_service()
    }

    /// Runs, builds, run configurations and the Build tool window (project service; a detached
    /// instance while no project is open).
    pub fn execution(&self) -> Arc<ExecutionManager> {
        self.platform_project()
            .and_then(|project| project.get_service())
            .unwrap_or_else(|| self.platform.detached.execution.clone())
    }

    /// Breakpoints and the debug session (project service; detached while no project is open).
    pub fn debugger(&self) -> Arc<XDebuggerManager> {
        self.platform_project()
            .and_then(|project| project.get_service())
            .unwrap_or_else(|| self.platform.detached.debugger.clone())
    }

    /// The Terminal tool window's shells (project service; detached while no project is open).
    pub fn terminal(&self) -> Arc<TerminalService> {
        self.platform_project()
            .and_then(|project| project.get_service())
            .unwrap_or_else(|| self.platform.detached.terminal.clone())
    }

    /// The Spring Boot starter catalog of the New Project wizard (application service).
    pub fn spring(&self) -> Arc<SpringStarters> {
        self.application().service()
    }

    /// Git repository state of the open project (project service).
    pub fn git(&self) -> Option<Arc<GitRepositories>> {
        self.platform_project()?.get_service()
    }

    /// The Git status of the open project, if it is in a repository.
    pub fn git_status(&self) -> Option<Arc<RepositoryStatus>> {
        self.git()?.status()
    }

    /// The interrupted merge/rebase/cherry-pick of the open project, if any.
    pub fn git_pending(&self) -> Option<archipel_ide_git::PendingOperation> {
        self.git()?.pending()
    }

    /// Whether the open project is in a Git repository.
    pub fn has_git(&self) -> bool {
        self.git_status().is_some()
    }

    /// Records that files or `.git` changed outside the IDE (see [`GitRepositories::is_stale`]).
    pub(crate) fn mark_vcs_stale(&self) {
        if let Some(git) = self.git() {
            git.mark_stale();
        }
    }

    /// Reloads `toolchains.toml` and the project's requirements.
    pub(crate) fn reload_toolchains(&self, project_root: Option<&Path>) {
        let tools = self.toolchains();
        let mut tools = tools.write();
        tools.reload(project_root);
        if let (Some(root), Some(requirements)) = (project_root, self.toolchain_requirements()) {
            requirements.write().reload(root, &tools);
        }
    }

    /// The "missing toolchain" bar above the editor of `path`.
    pub fn toolchain_banner_for(&self, path: &Path) -> Option<ToolchainBanner> {
        let requirements = self.toolchain_requirements()?;
        let tools = self.toolchains();
        requirements.read().banner_for(&tools.read(), path)
    }

    /// The "missing toolchain" bar shown while no editor is open.
    pub fn toolchain_project_banner(&self) -> Option<ToolchainBanner> {
        let requirements = self.toolchain_requirements()?;
        let tools = self.toolchains();
        requirements.read().project_banner(&tools.read())
    }

    /// Runs what listeners asked for since the last call. Called right after the publishing
    /// code, so an event has its full effect before the publisher returns.
    pub(crate) fn dispatch_reactions(&mut self) {
        for reaction in self.platform.reactions.take() {
            match reaction {
                Reaction::VcsChanged { structure_changed } => {
                    self.invalidate_line_changes();
                    self.invalidate_git_log();
                    self.invalidate_annotations();
                    if structure_changed && let Some(project) = &self.project {
                        // Files created or deleted outside the IDE change the set of
                        // untracked/added/deleted paths; rescan so the project tree shows them.
                        self.set_files(project.entries(20_000));
                    }
                    self.refresh_diff();
                }
                Reaction::ToolchainInstalled => self.toolchain_installed(),
                Reaction::RunExited => self.run_exited(),
                Reaction::DebugPaused(frame) => self.debug_paused(&frame),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    #[derive(Default)]
    struct Events(Mutex<Vec<String>>);

    impl ProjectManagerListener for Events {
        fn project_opened(&self, project: &Project) {
            self.0.lock().push(format!("opened {}", project.name()));
        }
        fn project_closing(&self, project: &Project) {
            self.0.lock().push(format!("closing {}", project.name()));
        }
        fn project_closed(&self, project: &Project) {
            self.0.lock().push(format!("closed {}", project.name()));
        }
    }

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("archipel-platform-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn git(dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args(args)
            .output()
            .expect("git is installed");
        assert!(output.status.success(), "git {args:?} failed");
    }

    fn listen(state: &IdeState) -> Arc<Events> {
        let events = Arc::new(Events::default());
        state
            .application()
            .message_bus()
            .subscribe(
                state.application().disposable(),
                ProjectManager::TOPIC,
                events.clone(),
            )
            .unwrap();
        events
    }

    #[test]
    fn opening_and_closing_a_project_fires_the_project_manager_topics() {
        let dir = temp("lifecycle");
        let mut state = IdeState::default();
        let events = listen(&state);
        assert!(state.platform_project().is_none());
        assert!(state.git().is_none(), "no project, no project services");

        state.open_project(&dir).unwrap();
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(state.platform_project().unwrap().root(), dir);
        assert!(state.git().is_some());
        assert_eq!(*events.0.lock(), [format!("opened {name}")]);

        let project = state.platform_project().unwrap().clone();
        state.close_project().unwrap();
        assert!(state.platform_project().is_none());
        assert!(project.is_disposed(), "closing disposes the project");
        assert!(state.git().is_none());
        assert_eq!(
            *events.0.lock(),
            [
                format!("opened {name}"),
                format!("closing {name}"),
                format!("closed {name}")
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn opening_another_project_closes_the_previous_one_first() {
        let (a, b) = (temp("switch-a"), temp("switch-b"));
        let mut state = IdeState::default();
        let events = listen(&state);
        state.open_project(&a).unwrap();
        let first = state.platform_project().unwrap().clone();
        state.open_project(&b).unwrap();
        assert!(first.is_disposed());
        assert_eq!(state.platform_project().unwrap().root(), b);
        let events = events.0.lock();
        let names: Vec<_> = events
            .iter()
            .map(|e| e.split(' ').next().unwrap())
            .collect();
        assert_eq!(names, ["opened", "closing", "closed", "opened"]);
        drop(events);
        assert_eq!(
            state.application().project_manager().open_projects().len(),
            1
        );
        let _ = std::fs::remove_dir_all(&a);
        let _ = std::fs::remove_dir_all(&b);
    }

    #[test]
    fn migrated_services_are_created_lazily_and_shared() {
        let state = IdeState::default();
        let app = state.application();
        assert!(!app.service_container().is_created::<SettingsService>());
        let settings = state.settings();
        assert!(Arc::ptr_eq(&settings, &state.settings()));
        assert!(app.service_container().is_created::<SettingsService>());
        assert!(!app.service_container().is_created::<ToolchainManager>());
    }

    #[test]
    fn a_git_status_change_is_announced_and_rescans_new_files() {
        #[derive(Default)]
        struct Heard(Mutex<Vec<bool>>);
        impl GitRepositoryChangeListener for Heard {
            fn repository_changed(&self, _: &RepositoryStatus, structure_changed: bool) {
                self.0.lock().push(structure_changed);
            }
        }
        let dir = temp("vcs-event");
        git(&dir, &["init", "-q", "-b", "main"]);
        std::fs::write(dir.join("a.txt"), "a\n").unwrap();
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "-q", "-m", "init"]);
        let mut state = IdeState::default();
        state.open_project(&dir).unwrap();
        let heard = Arc::new(Heard::default());
        let project = state.platform_project().unwrap().clone();
        project
            .message_bus()
            .subscribe(project.disposable(), GIT_REPO_CHANGE, heard.clone())
            .unwrap();

        std::fs::write(dir.join("new.txt"), "n\n").unwrap();
        assert!(
            !state
                .files
                .iter()
                .any(|f| f.relative_path == Path::new("new.txt"))
        );
        state.refresh_git().unwrap();
        assert_eq!(
            *heard.0.lock(),
            [true],
            "an untracked file is a structural change"
        );
        assert!(
            state
                .files
                .iter()
                .any(|f| f.relative_path == Path::new("new.txt")),
            "the VCS listener rescanned the project tree"
        );
        assert_eq!(state.git_status().unwrap().changes.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_installed_toolchain_component_is_announced_and_asks_for_follow_ups() {
        let state = IdeState::default();
        state
            .application()
            .message_bus()
            .publish(TOOLCHAIN_TOPIC, |l| l.component_installed("jdk", "21"));
        assert_eq!(
            state.platform.reactions.take(),
            [Reaction::ToolchainInstalled]
        );
        state
            .application()
            .message_bus()
            .publish(TOOLCHAIN_TOPIC, |l| l.component_failed("jdk", "offline"));
        assert!(
            state.platform.reactions.take().is_empty(),
            "failures need no follow-up"
        );
    }

    #[cfg(unix)]
    #[test]
    fn closing_the_project_stops_its_task_and_shells() {
        let dir = temp("dispose-run");
        let mut state = IdeState::default();
        state.open_project(&dir).unwrap();
        let execution = state.execution();
        let terminal = state.terminal();
        assert!(!Arc::ptr_eq(&execution, &state.platform.detached.execution));
        execution
            .start(TaskPlan::new("long", "sleep", ["30"], &dir))
            .unwrap();
        terminal.state_mut().set_session(
            archipel_ide_terminal::TerminalSession::spawn("/bin/sh", &["-c", "sleep 30"], &dir)
                .unwrap(),
        );
        assert!(execution.is_running() && terminal.has_session());

        state.platform.close_project();
        assert!(
            !execution.is_running(),
            "closing the project stops its task"
        );
        assert!(!terminal.has_session(), "and kills its shells");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn without_a_project_the_services_are_detached_and_reset_when_one_opens() {
        let dir = temp("detached");
        let mut state = IdeState::default();
        let welcome = state.terminal();
        welcome.state_mut().set_session(
            archipel_ide_terminal::TerminalSession::spawn(
                "/bin/sh",
                &["-c", "sleep 30"],
                std::path::Path::new("/tmp"),
            )
            .unwrap(),
        );
        assert!(state.terminal().has_session());
        state.open_project(&dir).unwrap();
        assert!(!welcome.has_session(), "a shell of the welcome screen ends");
        assert!(!Arc::ptr_eq(&welcome, &state.terminal()));
        assert!(!state.terminal().has_session(), "the project starts empty");
        state.close_project().unwrap();
        assert!(Arc::ptr_eq(&welcome, &state.terminal()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_task_that_exits_drives_the_build_window_and_the_status_bar() {
        let mut state = IdeState::default();
        state
            .execution()
            .start(
                TaskPlan::new("job", "sh", ["-c", "echo hello; exit 2"], ".")
                    .with_kind(archipel_ide_runner::TaskKind::Build),
            )
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !state.status.contains("failed with exit code 2")
            && std::time::Instant::now() < deadline
        {
            state.poll_runner();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(state.status, "job failed with exit code 2");
        let execution = state.execution();
        let window = execution.build_window();
        let snapshot = window
            .snapshot(crate::BuildTab::Output)
            .expect("the Build tool window kept the finished run");
        assert_eq!(snapshot.state, TaskState::Exited(2));
        assert!(snapshot.console.lines().iter().any(|l| l.text == "hello"));
    }

    #[test]
    fn the_notification_service_is_shared_by_the_ui_and_plugins() {
        let state = IdeState::default();
        assert!(!state.notifications().update_balloon());
        state.notifications().show_update_balloon();
        assert!(state.notifications().update_balloon());
    }
}

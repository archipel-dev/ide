//! Agents mode as an application service (IntelliJ: the AI Assistant's agent sessions).
//!
//! [`AgentsManager`] owns the sessions the backend runs, the thread tabs, the Review screen's
//! selection and the balloons about agents. It is an application service: agent tasks belong to
//! the window and survive switching projects (the backend only re-roots its worktrees through
//! [`AgentsManager::set_project`]). What happened is announced on [`AGENTS_TOPIC`]: the
//! backend's news (needs input, done, failed, spawned) and the agent the user selected.
//!
//! The UI asks narrow questions (`counts()`, `sessions()`, `selected()`, `notices()`, ...) that
//! return copies, so no guard outlives a call; commands write through [`AgentsManager::write`].

use super::Cell;
use crate::agent_events::AgentEvent;
use crate::agents::{
    AgentCounts, AgentNews, AgentNotice, AgentProfile, AgentsScreen, AgentsState, ChangedFile,
    SessionId, SessionInfo, TaskId, TaskInfo, WorkbenchMode,
};
use archipel_ide_git::FileDiff;
use archipel_ide_platform::{MessageBus, Topic};
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Reports what the agents did (IntelliJ: the agent session listeners).
pub trait AgentsListener: Send + Sync {
    /// The backend reported news about an agent.
    fn agent_news(&self, _news: &AgentNews) {}
    /// The user selected an agent (its thread tab is the active one).
    fn agent_selected(&self, _session: SessionId) {}
}

/// On the application's message bus.
pub const AGENTS_TOPIC: Topic<dyn AgentsListener> = Topic::new("AgentsListener");

/// The application service: `state.agents_manager()`.
pub struct AgentsManager {
    state: Cell<AgentsState>,
    bus: MessageBus,
}

impl AgentsManager {
    pub fn new(bus: MessageBus) -> Self {
        Self::with_state(bus, AgentsState::default())
    }

    /// A manager over `state` (a different backend, or tests).
    pub fn with_state(bus: MessageBus, state: AgentsState) -> Self {
        Self {
            state: Cell::new(state, "the agents"),
            bus,
        }
    }

    pub(crate) fn read(&self) -> RwLockReadGuard<'_, AgentsState> {
        self.state.read()
    }

    pub(crate) fn write(&self) -> RwLockWriteGuard<'_, AgentsState> {
        self.state.write()
    }

    // Queries (copies: no guard outlives the call).

    pub fn sessions(&self) -> Vec<SessionInfo> {
        self.read().sessions()
    }

    pub fn session(&self, id: SessionId) -> Option<SessionInfo> {
        self.read().session(id)
    }

    pub fn counts(&self) -> AgentCounts {
        self.read().counts()
    }

    pub fn tasks(&self) -> Vec<TaskInfo> {
        self.read().backend.tasks()
    }

    /// The files the agent changed so far.
    pub fn changes(&self, session: SessionId) -> Vec<ChangedFile> {
        self.read().backend.changes(session)
    }

    /// The structured transcript of the session.
    pub fn events(&self, session: SessionId) -> Vec<AgentEvent> {
        self.read().backend.events(session).to_vec()
    }

    /// Base against agent version of `path`, for the diff viewer.
    pub fn diff(&self, session: SessionId, path: &Path) -> Option<FileDiff> {
        self.read().backend.diff(session, path)
    }

    pub fn screen(&self) -> AgentsScreen {
        self.read().screen
    }

    pub fn selected(&self) -> Option<SessionId> {
        self.read().selected
    }

    /// The agent the "Back to agent" banner returns to.
    pub fn return_to(&self) -> Option<SessionId> {
        self.read().return_to
    }

    /// Open thread tabs, in tab order.
    pub fn threads(&self) -> Vec<SessionId> {
        self.read().threads.clone()
    }

    /// Whether the empty "new thread" tab is open.
    pub fn new_thread(&self) -> bool {
        self.read().new_thread
    }

    /// Whether the "new thread" tab is the active one.
    pub fn draft_active(&self) -> bool {
        let state = self.read();
        state.new_thread && state.draft_active
    }

    pub fn notices(&self) -> Vec<AgentNotice> {
        self.read().notices.clone()
    }

    pub fn has_notices(&self) -> bool {
        !self.read().notices.is_empty()
    }

    /// The file shown on the Review screen.
    pub fn review_file(&self) -> Option<PathBuf> {
        self.read().review_file.clone()
    }

    pub fn set_review_file(&self, path: Option<PathBuf>) {
        self.write().review_file = path;
    }

    pub fn set_screen(&self, screen: AgentsScreen) {
        self.write().screen = screen;
    }

    // Changes that are announced.

    /// Re-roots the backend's worktrees at the open project.
    pub fn set_project(&self, root: Option<&Path>) {
        self.write().backend.set_project(root);
    }

    /// Selects an agent: opens its tab, marks it read and drops its balloons.
    pub fn select(&self, id: SessionId) {
        self.write().select(id);
        self.bus.publish(AGENTS_TOPIC, |l| l.agent_selected(id));
    }

    /// Starts a task and opens a thread tab for every top-level agent of it.
    pub fn launch(&self, prompt: &str, profiles: &[AgentProfile]) -> TaskId {
        let (task, selected) = {
            let mut state = self.write();
            let task = state.launch(prompt, profiles);
            (task, state.selected)
        };
        if let Some(id) = selected {
            self.bus.publish(AGENTS_TOPIC, |l| l.agent_selected(id));
        }
        task
    }

    /// Lets the wall-clock time since the last poll pass. Returns whether an agent is still
    /// working. The news is published after the state is released, so a listener may read it.
    pub fn poll(&self, now: Instant, mode: WorkbenchMode) -> bool {
        let (busy, news) = {
            let mut state = self.write();
            let busy = state.poll(now, mode);
            (busy, state.take_news())
        };
        self.announce(&news);
        busy
    }

    /// [`Self::poll`] with an explicit time step (tests, demos).
    pub fn advance(&self, dt: Duration, mode: WorkbenchMode) -> bool {
        let (busy, news) = {
            let mut state = self.write();
            let busy = state.advance(dt, mode);
            (busy, state.take_news())
        };
        self.announce(&news);
        busy
    }

    fn announce(&self, news: &[AgentNews]) {
        for item in news {
            self.bus.publish(AGENTS_TOPIC, |l| l.agent_news(item));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_platform::{Application, ComponentManager, ServiceDescriptor};
    use parking_lot::Mutex;
    use std::sync::Arc;

    #[derive(Default)]
    struct Recorder(Mutex<Vec<String>>);

    impl AgentsListener for Recorder {
        fn agent_news(&self, news: &AgentNews) {
            self.0.lock().push(format!("{news:?}"));
        }
        fn agent_selected(&self, session: SessionId) {
            self.0.lock().push(format!("selected {session}"));
        }
    }

    /// A listener that reads the service while it is being told: the news is published after
    /// the state was released, so this must not deadlock.
    struct Reader(Arc<AgentsManager>, Mutex<usize>);

    impl AgentsListener for Reader {
        fn agent_news(&self, _news: &AgentNews) {
            *self.1.lock() = self.0.sessions().len();
        }
    }

    fn setup() -> (Application, Arc<AgentsManager>, Arc<Recorder>) {
        let application = Application::new();
        application
            .register_service(ServiceDescriptor::new(|app: &Application| {
                AgentsManager::new(app.message_bus().clone())
            }))
            .unwrap();
        let recorder = Arc::new(Recorder::default());
        application
            .message_bus()
            .subscribe(application.disposable(), AGENTS_TOPIC, recorder.clone())
            .unwrap();
        let manager = application.service::<AgentsManager>();
        (application, manager, recorder)
    }

    #[test]
    fn launching_opens_a_tab_and_announces_the_selection() {
        let (_application, agents, recorder) = setup();
        let task = agents.launch("Fix the tab order", &[AgentProfile::Claude]);
        let sessions = agents.sessions();
        assert!(sessions.iter().any(|s| s.task == task));
        assert_eq!(agents.threads().len(), 1);
        assert_eq!(agents.selected(), agents.threads().first().copied());
        assert!(
            recorder
                .0
                .lock()
                .iter()
                .any(|line| line.starts_with("selected "))
        );
    }

    #[test]
    fn news_reaches_listeners_that_read_the_service() {
        let (application, agents, recorder) = setup();
        let reader = Arc::new(Reader(agents.clone(), Mutex::new(usize::MAX)));
        application
            .message_bus()
            .subscribe(application.disposable(), AGENTS_TOPIC, reader.clone())
            .unwrap();
        agents.launch("Add a test", &[AgentProfile::Claude]);
        let mut busy = true;
        for _ in 0..600 {
            busy = agents.advance(Duration::from_millis(500), WorkbenchMode::Code);
            if !busy {
                break;
            }
        }
        assert!(!busy, "the scripted agent finishes");
        let news = recorder
            .0
            .lock()
            .iter()
            .filter(|l| !l.starts_with("selected"))
            .count();
        assert!(news > 0, "finishing is news: {:?}", recorder.0.lock());
        assert_eq!(*reader.1.lock(), agents.sessions().len());
    }

    #[test]
    fn selecting_marks_read_and_clears_the_balloons() {
        let (_application, agents, _) = setup();
        agents.launch("Refactor", &[AgentProfile::Claude]);
        for _ in 0..600 {
            if !agents.advance(Duration::from_millis(500), WorkbenchMode::Code) {
                break;
            }
        }
        let id = agents.sessions()[0].id;
        agents.select(id);
        assert!(agents.notices().iter().all(|n| n.session != id));
        assert_eq!(agents.counts().done_unread, 0);
    }

    #[test]
    fn queries_return_copies_so_a_command_may_write_between_them() {
        let (_application, agents, _) = setup();
        let sessions = agents.sessions();
        agents.set_screen(AgentsScreen::Review);
        agents.set_review_file(Some("a.rs".into()));
        assert_eq!(agents.screen(), AgentsScreen::Review);
        assert_eq!(agents.review_file(), Some(PathBuf::from("a.rs")));
        assert_eq!(sessions.len(), agents.sessions().len());
    }
}

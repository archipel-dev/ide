//! Agents mode: the workbench mode switch, the backend contract and the state the UI renders.
//!
//! Agents mode is a second arrangement of the same window (editor, tabs and project stay
//! as they are). Everything about the agents themselves sits behind [`AgentsBackend`]; the
//! prototype ships [`crate::agents_mock::MockAgentsBackend`], a scripted stand-in, so the
//! real engine can replace it later without touching the UI. No toolkit types here.

use crate::agent_events::AgentEvent;
use crate::agents_mock::MockAgentsBackend;
use crate::{CommandError, IdeState, spec, string_array_param, string_param, usize_param};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_git::FileDiff;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Action id of the mode toggle (see `keymap_commands::ARCHIPEL_ACTIONS`).
pub const TOGGLE_MODE_ACTION: &str = "Archipel.ToggleAgentsMode";
/// Action id of "New Agent Thread".
pub const NEW_THREAD_ACTION: &str = "Archipel.NewAgentThread";

pub type SessionId = u32;
pub type TaskId = u32;

/// The two arrangements of the workbench.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WorkbenchMode {
    #[default]
    Code,
    Agents,
}

impl WorkbenchMode {
    pub fn id(self) -> &'static str {
        match self {
            WorkbenchMode::Code => "code",
            WorkbenchMode::Agents => "agents",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            WorkbenchMode::Code => "Code",
            WorkbenchMode::Agents => "Agents",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        match id {
            "code" => Some(WorkbenchMode::Code),
            "agents" => Some(WorkbenchMode::Agents),
            _ => None,
        }
    }

    pub fn other(self) -> Self {
        match self {
            WorkbenchMode::Code => WorkbenchMode::Agents,
            WorkbenchMode::Agents => WorkbenchMode::Code,
        }
    }
}

/// The screens of Agents mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AgentsScreen {
    /// Thread tabs, the inspector and the prompt bar.
    #[default]
    Main,
    /// The selected agent's diff.
    Review,
}

/// A CLI agent the user can launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentProfile {
    Claude,
    Codex,
    Gemini,
    Opencode,
}

impl AgentProfile {
    pub const ALL: [AgentProfile; 4] = [
        AgentProfile::Claude,
        AgentProfile::Codex,
        AgentProfile::Gemini,
        AgentProfile::Opencode,
    ];

    pub fn id(self) -> &'static str {
        match self {
            AgentProfile::Claude => "claude",
            AgentProfile::Codex => "codex",
            AgentProfile::Gemini => "gemini",
            AgentProfile::Opencode => "opencode",
        }
    }

    /// Product name, e.g. `Claude Code`.
    pub fn product_name(self) -> &'static str {
        match self {
            AgentProfile::Claude => "Claude Code",
            AgentProfile::Codex => "Codex",
            AgentProfile::Gemini => "Gemini CLI",
            AgentProfile::Opencode => "opencode",
        }
    }

    /// Product name and the model it runs (shown in the inspector).
    pub fn product(self) -> &'static str {
        match self {
            AgentProfile::Claude => "Claude Code \u{b7} sonnet",
            AgentProfile::Codex => "Codex \u{b7} gpt-5-codex",
            AgentProfile::Gemini => "Gemini CLI \u{b7} gemini-2.5-pro",
            AgentProfile::Opencode => "opencode \u{b7} default model",
        }
    }

    /// How the agent reports structured events. Agents without such a protocol are not
    /// offered at all: Agents mode has no terminal view.
    pub fn protocol(self) -> &'static str {
        match self {
            AgentProfile::Claude => "stream-json",
            AgentProfile::Codex => "exec --json / app-server",
            AgentProfile::Gemini | AgentProfile::Opencode => "structured output",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|profile| profile.id() == id)
    }
}

/// What an agent is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Working,
    /// Blocked on a question or a permission prompt.
    NeedsInput,
    Done,
    Error,
    /// Interrupted by the user; sending input resumes it.
    Stopped,
}

impl AgentState {
    pub fn label(self) -> &'static str {
        match self {
            AgentState::Working => "working",
            AgentState::NeedsInput => "needs input",
            AgentState::Done => "done",
            AgentState::Error => "error",
            AgentState::Stopped => "stopped",
        }
    }

    /// Working or waiting: the session still has a live process.
    pub fn is_active(self) -> bool {
        matches!(self, AgentState::Working | AgentState::NeedsInput)
    }
}

/// How a finished agent's work was handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    Squash,
    Merge,
    Discard,
}

impl MergeStrategy {
    pub fn id(self) -> &'static str {
        match self {
            MergeStrategy::Squash => "squash",
            MergeStrategy::Merge => "merge",
            MergeStrategy::Discard => "discard",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        [
            MergeStrategy::Squash,
            MergeStrategy::Merge,
            MergeStrategy::Discard,
        ]
        .into_iter()
        .find(|strategy| strategy.id() == id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionInfo {
    pub id: SessionId,
    pub task: TaskId,
    /// Set for an agent started by another agent.
    pub parent: Option<SessionId>,
    pub profile: AgentProfile,
    /// Worktree / branch name, e.g. `fix-tabs`.
    pub name: String,
    pub state: AgentState,
    /// News the user has not looked at yet (finished, asked a question, failed).
    pub unread: bool,
    pub elapsed: Duration,
    /// Where the agent works (a git worktree) and the branch it commits to.
    pub worktree: String,
    pub branch: String,
    /// Tokens used so far.
    pub tokens: u32,
    /// The open question while the state is [`AgentState::NeedsInput`].
    pub question: Option<String>,
    pub files: usize,
    pub added: usize,
    pub removed: usize,
    /// Set once the work was merged or discarded.
    pub outcome: Option<MergeStrategy>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskInfo {
    pub id: TaskId,
    pub title: String,
    pub prompt: String,
    /// Every session of the task, parents before their children.
    pub sessions: Vec<SessionId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Modified,
    Added,
}

/// One file an agent changed.
#[derive(Debug, Clone, PartialEq)]
pub struct ChangedFile {
    /// Project-relative path.
    pub path: PathBuf,
    pub kind: ChangeKind,
    pub added: usize,
    pub removed: usize,
    /// First changed line in the new text, 1-based.
    pub first_line: usize,
}

/// Something worth telling the user about, produced while time advances.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentNews {
    NeedsInput(SessionId),
    Done(SessionId),
    Failed(SessionId),
    Spawned { parent: SessionId, child: SessionId },
}

/// The contract between the Agents mode UI and whatever runs the agents.
///
/// The prototype implements it with scripted sessions
/// ([`crate::agents_mock::MockAgentsBackend`]); a real engine (worktrees, PTYs, hooks)
/// implements the same trait. The UI never assumes more than this.
pub trait AgentsBackend {
    /// The project agents work on (worktrees are created from it).
    fn set_project(&mut self, root: Option<&Path>);
    fn tasks(&self) -> Vec<TaskInfo>;
    /// All sessions, parents before their children, oldest task first.
    fn sessions(&self) -> Vec<SessionInfo>;
    /// Starts one agent per profile on a new task; returns the task.
    fn launch(&mut self, prompt: &str, profiles: &[AgentProfile]) -> TaskId;
    /// Types `input` into the agent's terminal (an answer, a follow-up prompt).
    fn send(&mut self, session: SessionId, input: &str);
    fn stop(&mut self, session: SessionId);
    /// The files the agent changed so far.
    fn changes(&self, session: SessionId) -> Vec<ChangedFile>;
    /// Base against agent version of `path`, for the diff viewer.
    fn diff(&self, session: SessionId, path: &Path) -> Option<FileDiff>;
    fn merge(&mut self, session: SessionId, strategy: MergeStrategy) -> Result<String, String>;
    /// The structured transcript of the session (empty for agents without a protocol).
    fn events(&self, session: SessionId) -> &[AgentEvent];
    fn mark_read(&mut self, session: SessionId);
    /// Lets `dt` of wall-clock time pass and reports what happened.
    fn advance(&mut self, dt: Duration) -> Vec<AgentNews>;
}

/// How many agents are in each state (status bar widget, toolbar badge).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AgentCounts {
    pub working: usize,
    pub needs_input: usize,
    /// Finished with news the user has not seen.
    pub done_unread: usize,
    pub failed: usize,
}

impl AgentCounts {
    pub fn of(sessions: &[SessionInfo]) -> Self {
        let mut counts = Self::default();
        for session in sessions {
            match session.state {
                AgentState::Working => counts.working += 1,
                AgentState::NeedsInput => counts.needs_input += 1,
                AgentState::Done if session.unread => counts.done_unread += 1,
                AgentState::Error => counts.failed += 1,
                _ => {}
            }
        }
        counts
    }

    /// Agents that want the user's attention now.
    pub fn attention(&self) -> usize {
        self.needs_input + self.done_unread + self.failed
    }

    pub fn is_idle(&self) -> bool {
        self.working == 0 && self.attention() == 0
    }

    /// Status bar text, e.g. `2 running · ? 1 input`; empty when there is nothing to say.
    pub fn status_text(&self) -> String {
        let mut parts = Vec::new();
        if self.working > 0 {
            parts.push(format!("{} running", self.working));
        }
        if self.needs_input > 0 {
            parts.push(format!("? {} input", self.needs_input));
        }
        if self.done_unread > 0 {
            parts.push(format!("{} done", self.done_unread));
        }
        if self.failed > 0 {
            parts.push(format!("{} failed", self.failed));
        }
        parts.join(" · ")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    NeedsInput,
    Done,
    Failed,
    Spawned,
}

/// A balloon notification about one agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentNotice {
    pub id: u64,
    pub session: SessionId,
    pub kind: NoticeKind,
    pub title: String,
    pub body: String,
    /// Clock reading after which the balloon goes away by itself; sticky when `None`.
    pub expires_ms: Option<u64>,
}

/// Longest a non-sticky balloon stays.
const NOTICE_LIFETIME_MS: u64 = 12_000;
/// Same agent, same kind of news: at most one balloon per this long.
const NOTICE_COOLDOWN_MS: u64 = 5_000;

pub struct AgentsState {
    pub backend: Box<dyn AgentsBackend>,
    pub screen: AgentsScreen,
    pub selected: Option<SessionId>,
    /// The file shown on the Review screen.
    pub review_file: Option<PathBuf>,
    /// Open thread tabs, in tab order. `selected` is the active one.
    pub threads: Vec<SessionId>,
    /// An empty "new thread" tab is open (after the agent tabs) ...
    pub new_thread: bool,
    /// ... and it is the active tab.
    pub draft_active: bool,
    pub notices: Vec<AgentNotice>,
    /// Set by "Open in Code mode": the agent the "Back to agent" banner returns to.
    pub return_to: Option<SessionId>,
    clock_ms: u64,
    last_poll: Option<Instant>,
    next_notice: u64,
    last_notice: HashMap<(SessionId, u8), u64>,
}

impl Default for AgentsState {
    fn default() -> Self {
        Self::new(Box::new(MockAgentsBackend::default()))
    }
}

impl AgentsState {
    pub fn new(backend: Box<dyn AgentsBackend>) -> Self {
        Self {
            backend,
            screen: AgentsScreen::Main,
            selected: None,
            review_file: None,
            threads: Vec::new(),
            new_thread: false,
            draft_active: false,
            notices: Vec::new(),
            return_to: None,
            clock_ms: 0,
            last_poll: None,
            next_notice: 1,
            last_notice: HashMap::new(),
        }
    }

    pub fn sessions(&self) -> Vec<SessionInfo> {
        self.backend.sessions()
    }

    pub fn session(&self, id: SessionId) -> Option<SessionInfo> {
        self.backend.sessions().into_iter().find(|s| s.id == id)
    }

    pub fn counts(&self) -> AgentCounts {
        AgentCounts::of(&self.backend.sessions())
    }

    /// Opens a thread tab for `id` (or focuses it) and makes it the active one.
    pub fn open_thread(&mut self, id: SessionId) {
        if !self.threads.contains(&id) {
            self.threads.push(id);
        }
        self.selected = Some(id);
        self.draft_active = false;
    }

    /// Opens the empty "new thread" tab and activates it.
    pub fn open_draft(&mut self) {
        self.new_thread = true;
        self.draft_active = true;
    }

    /// Closes the "new thread" tab; the last agent tab becomes active again.
    pub fn close_draft(&mut self) {
        self.new_thread = false;
        if self.draft_active {
            self.draft_active = false;
            self.selected = self.selected.or_else(|| self.threads.last().copied());
        }
    }

    /// Closes the tab; the neighbour becomes active. The agent keeps running.
    pub fn close_thread(&mut self, id: SessionId) {
        let Some(at) = self.threads.iter().position(|t| *t == id) else {
            return;
        };
        self.threads.remove(at);
        if self.selected == Some(id) {
            self.selected = self
                .threads
                .get(at)
                .or_else(|| at.checked_sub(1).and_then(|i| self.threads.get(i)))
                .copied();
        }
    }

    /// Activates the next (or previous) thread tab, wrapping around.
    pub fn cycle_thread(&mut self, forward: bool) {
        let n = self.threads.len();
        if n == 0 {
            return;
        }
        let at = self
            .selected
            .and_then(|id| self.threads.iter().position(|t| *t == id))
            .unwrap_or(0);
        let next = if forward {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        };
        self.select(self.threads[next]);
    }

    /// Selects an agent: opens its tab, marks it read and drops its balloons.
    pub fn select(&mut self, id: SessionId) {
        self.open_thread(id);
        self.backend.mark_read(id);
        self.notices.retain(|notice| notice.session != id);
    }

    /// Starts a task and opens a thread tab for every top-level agent of it.
    pub fn launch(&mut self, prompt: &str, profiles: &[AgentProfile]) -> TaskId {
        let task = self.backend.launch(prompt, profiles);
        self.new_thread = false;
        self.draft_active = false;
        let sessions = self.backend.sessions();
        let mut first = None;
        for session in sessions
            .iter()
            .filter(|s| s.task == task && s.parent.is_none())
        {
            self.open_thread(session.id);
            first.get_or_insert(session.id);
        }
        if let Some(first) = first {
            self.selected = Some(first);
        }
        task
    }

    /// Advances the backend by the wall-clock time since the last poll (capped) and turns
    /// its events into balloons. Returns whether anything is still moving.
    pub fn poll(&mut self, now: Instant, mode: WorkbenchMode) -> bool {
        let dt = self
            .last_poll
            .map(|last| now.saturating_duration_since(last))
            .unwrap_or_default()
            .min(Duration::from_secs(2));
        self.last_poll = Some(now);
        self.advance(dt, mode)
    }

    /// [`Self::poll`] with an explicit time step (tests, demos).
    pub fn advance(&mut self, dt: Duration, mode: WorkbenchMode) -> bool {
        self.clock_ms += dt.as_millis() as u64;
        let events = self.backend.advance(dt);
        let sessions = self.backend.sessions();
        for event in events {
            self.on_event(&event, &sessions, mode);
        }
        // News about the agent the user is looking at is not news.
        if mode == WorkbenchMode::Agents
            && let Some(selected) = self.selected
        {
            self.backend.mark_read(selected);
        }
        let now = self.clock_ms;
        self.notices
            .retain(|notice| notice.expires_ms.is_none_or(|end| end > now));
        sessions.iter().any(|s| s.state == AgentState::Working)
    }

    fn on_event(&mut self, event: &AgentNews, sessions: &[SessionInfo], mode: WorkbenchMode) {
        let find = |id: SessionId| sessions.iter().find(|s| s.id == id);
        let (session, kind, title, sticky) = match event {
            AgentNews::NeedsInput(id) => {
                let Some(info) = find(*id) else { return };
                (
                    *id,
                    NoticeKind::NeedsInput,
                    format!("{} in {} needs input", info.profile.id(), info.name),
                    true,
                )
            }
            AgentNews::Done(id) => {
                let Some(info) = find(*id) else { return };
                (
                    *id,
                    NoticeKind::Done,
                    format!("{} in {} finished", info.profile.id(), info.name),
                    false,
                )
            }
            AgentNews::Failed(id) => {
                let Some(info) = find(*id) else { return };
                (
                    *id,
                    NoticeKind::Failed,
                    format!("{} in {} failed", info.profile.id(), info.name),
                    true,
                )
            }
            AgentNews::Spawned { parent, child } => {
                let (Some(parent), Some(child)) = (find(*parent), find(*child)) else {
                    return;
                };
                // The pane of a child is opened by the user; the tree shows it at once.
                (
                    child.id,
                    NoticeKind::Spawned,
                    format!("{} started {}", parent.name, child.name),
                    false,
                )
            }
        };
        // In Agents mode the agents are in view (state icons, unread markers): no balloons.
        if mode == WorkbenchMode::Agents {
            return;
        }
        let key = (session, kind as u8);
        if self
            .last_notice
            .get(&key)
            .is_some_and(|at| self.clock_ms.saturating_sub(*at) < NOTICE_COOLDOWN_MS)
        {
            return;
        }
        self.last_notice.insert(key, self.clock_ms);
        let body = match (kind, find(session)) {
            (NoticeKind::NeedsInput, Some(info)) => {
                info.question.clone().unwrap_or_default().trim().to_owned()
            }
            (NoticeKind::Done, Some(info)) => format!(
                "{} changed, +{} -{}",
                crate::agents_mock::plural(info.files, "file", "files"),
                info.added,
                info.removed
            ),
            (NoticeKind::Spawned, Some(info)) => format!("Agent task: {}", info.name),
            _ => String::new(),
        };
        let id = self.next_notice;
        self.next_notice += 1;
        self.notices.push(AgentNotice {
            id,
            session,
            kind,
            title,
            body,
            expires_ms: (!sticky).then_some(self.clock_ms + NOTICE_LIFETIME_MS),
        });
    }
}

impl IdeState {
    /// Switches the arrangement of the window. Documents, tabs, carets and the project are
    /// untouched; balloons about agents are dropped when the agents themselves are in view.
    pub fn set_workbench_mode(&mut self, mode: WorkbenchMode) {
        if self.workbench_mode == mode {
            return;
        }
        self.workbench_mode = mode;
        if mode == WorkbenchMode::Agents {
            self.agents.notices.clear();
            self.agents.return_to = None;
        }
    }

    pub fn poll_agents(&mut self) -> bool {
        let mode = self.workbench_mode;
        self.agents.poll(Instant::now(), mode)
    }

    /// Project changes must reach the backend (worktrees start from the project).
    pub fn sync_agents_project(&mut self) {
        let root = self.project.as_ref().map(|p| p.root().to_path_buf());
        self.agents.backend.set_project(root.as_deref());
    }

    /// Opens an agent's changed file in the editor at the changed line and switches to Code
    /// mode, remembering where to come back to.
    pub fn agents_open_in_code(
        &mut self,
        session: SessionId,
        path: Option<&Path>,
        line: Option<usize>,
    ) -> Result<(), String> {
        let changes = self.agents.backend.changes(session);
        let change = match path {
            Some(path) => changes.iter().find(|c| c.path == path),
            None => changes
                .iter()
                .find(|c| c.kind == ChangeKind::Modified)
                .or(changes.first()),
        }
        .ok_or("this agent has not changed any file yet")?;
        if change.kind == ChangeKind::Added {
            return Err(format!(
                "{} is a new file in the agent's worktree; it is not in your project yet",
                change.path.display()
            ));
        }
        let line = line.unwrap_or(change.first_line).max(1);
        let target = change.path.clone();
        self.open_file(&target)?;
        crate::editor_commands::select_in_active(self, line, 1, 0)?;
        self.agents.selected = Some(session);
        self.agents.return_to = Some(session);
        self.workbench_mode = WorkbenchMode::Code;
        Ok(())
    }

    /// The "Back to agent" banner: returns to the agent's review.
    pub fn agents_back(&mut self) {
        if let Some(session) = self.agents.return_to.take() {
            self.agents.selected = Some(session);
            self.agents.screen = AgentsScreen::Review;
            self.set_workbench_mode(WorkbenchMode::Agents);
        }
    }

    /// Seeds three realistic tasks and a failed spike, as if the user had started them a
    /// few seconds ago (`ARCHIPEL_AGENTS_DEMO=1`).
    pub fn seed_agents_demo(&mut self) {
        self.sync_agents_project();
        self.agents.seed_demo();
    }
}

impl AgentsState {
    pub fn seed_demo(&mut self) {
        if !self.backend.sessions().is_empty() {
            return;
        }
        let seeds: [(&str, AgentProfile, bool); 4] = [
            (
                "Fix tab ordering when the active tab is closed",
                AgentProfile::Claude,
                true,
            ),
            (
                "Add OAuth login with a device-code flow",
                AgentProfile::Codex,
                true,
            ),
            (
                "Refactor the storage layer behind one interface",
                AgentProfile::Gemini,
                true,
            ),
            (
                "Spike a Redis cache for the session store",
                AgentProfile::Claude,
                false,
            ),
        ];
        for (prompt, profile, in_grid) in seeds {
            let task = self.launch(prompt, &[profile]);
            if !in_grid {
                for session in self.backend.sessions().iter().filter(|s| s.task == task) {
                    self.close_thread(session.id);
                }
            }
        }
        // Start a few seconds in: the first agent already asks its question.
        // Only the question is news; the rest of the pre-roll is not.
        self.advance(Duration::from_millis(4_200), WorkbenchMode::Code);
        self.notices
            .retain(|notice| notice.kind == NoticeKind::NeedsInput);
        if let Some(first) = self.backend.sessions().first() {
            let id = first.id;
            self.selected = Some(id);
        }
    }
}

fn session_param(params: &Value) -> Result<SessionId, CommandError> {
    usize_param(params, "session").map(|id| id as SessionId)
}

fn known_session(state: &IdeState, params: &Value) -> Result<SessionId, CommandError> {
    let id = session_param(params)?;
    state
        .agents
        .session(id)
        .map(|s| s.id)
        .ok_or_else(|| CommandError::Failed(format!("no agent session {id}")))
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "workbench.setMode",
                "Switch Workbench Mode",
                "Agents",
                None,
                r#"{"mode":"code|agents"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = string_param(&params, "mode")?;
                let mode = WorkbenchMode::parse(&id)
                    .ok_or_else(|| CommandError::InvalidParams(format!("unknown mode `{id}`")))?;
                state.set_workbench_mode(mode);
                Ok(json!({"mode": mode.id()}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "workbench.toggleMode",
                "Toggle Agents Mode",
                "Agents",
                Some("⌥⇧⌘X"),
                "{}",
            ),
            |s: &IdeState| s.project.is_some(),
            |state: &mut IdeState, _| {
                let mode = state.workbench_mode.other();
                state.set_workbench_mode(mode);
                Ok(json!({"mode": mode.id()}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "agents.launch",
                "Launch Agents",
                "Agents",
                None,
                r#"{"prompt":"string","profiles":"string[]?"}"#,
            ),
            |s: &IdeState| s.project.is_some(),
            |state: &mut IdeState, params| {
                let prompt = string_param(&params, "prompt")?;
                if prompt.trim().is_empty() {
                    return Err(CommandError::InvalidParams("the prompt is empty".into()));
                }
                let names = if params.get("profiles").is_some() {
                    string_array_param(&params, "profiles")?
                } else {
                    vec!["claude".into()]
                };
                let mut profiles = Vec::new();
                for name in &names {
                    let profile = AgentProfile::parse(name).ok_or_else(|| {
                        CommandError::InvalidParams(format!("unknown agent `{name}`"))
                    })?;
                    if !profiles.contains(&profile) {
                        profiles.push(profile);
                    }
                }
                if profiles.is_empty() {
                    return Err(CommandError::InvalidParams("choose an agent".into()));
                }
                state.agents.screen = AgentsScreen::Main;
                let task = state.agents.launch(prompt.trim(), &profiles);
                Ok(json!({"task": task}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "agents.select",
                "Select Agent",
                "Agents",
                None,
                r#"{"session":1}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = known_session(state, &params)?;
                state.agents.select(id);
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "agents.show",
                "Show Agent",
                "Agents",
                None,
                r#"{"session":1,"review":"bool?"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = known_session(state, &params)?;
                state.set_workbench_mode(WorkbenchMode::Agents);
                state.agents.select(id);
                let review = params
                    .get("review")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                state.agents.screen = if review {
                    AgentsScreen::Review
                } else {
                    AgentsScreen::Main
                };
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "agents.review",
                "Review Agent Changes",
                "Agents",
                None,
                r#"{"session":"number?","path":"string?"}"#,
            ),
            |s: &IdeState| s.project.is_some(),
            |state: &mut IdeState, params| {
                let id = if params.get("session").is_some() {
                    known_session(state, &params)?
                } else {
                    state
                        .agents
                        .selected
                        .ok_or_else(|| CommandError::Failed("no agent is selected".into()))?
                };
                state.set_workbench_mode(WorkbenchMode::Agents);
                state.agents.select(id);
                state.agents.review_file = params
                    .get("path")
                    .and_then(Value::as_str)
                    .map(PathBuf::from);
                state.agents.screen = AgentsScreen::Review;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("agents.closeReview", "Back to Agents", "Agents", None, "{}"),
            |s: &IdeState| s.agents.screen == AgentsScreen::Review,
            |state: &mut IdeState, _| {
                state.agents.screen = AgentsScreen::Main;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "agents.send",
                "Send Input to Agent",
                "Agents",
                None,
                r#"{"session":1,"input":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = known_session(state, &params)?;
                let input = string_param(&params, "input")?;
                state.agents.backend.send(id, &input);
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "agents.stop",
                "Stop Agent",
                "Agents",
                None,
                r#"{"session":1}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = known_session(state, &params)?;
                state.agents.backend.stop(id);
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "agents.merge",
                "Merge Agent Work",
                "Agents",
                None,
                r#"{"session":1,"strategy":"squash|merge|discard"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = known_session(state, &params)?;
                let name = string_param(&params, "strategy")?;
                let strategy = MergeStrategy::parse(&name).ok_or_else(|| {
                    CommandError::InvalidParams(format!("unknown strategy `{name}`"))
                })?;
                let message = state
                    .agents
                    .backend
                    .merge(id, strategy)
                    .map_err(CommandError::Failed)?;
                state.status = message.clone();
                state.agents.screen = AgentsScreen::Main;
                Ok(json!({"message": message}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "agents.openInCode",
                "Open Agent Changes in Code Mode",
                "Agents",
                None,
                r#"{"session":"number?","path":"string?","line":"number?"}"#,
            ),
            |s: &IdeState| s.project.is_some(),
            |state: &mut IdeState, params| {
                let id = if params.get("session").is_some() {
                    known_session(state, &params)?
                } else {
                    state
                        .agents
                        .selected
                        .ok_or_else(|| CommandError::Failed("no agent is selected".into()))?
                };
                let path = params
                    .get("path")
                    .and_then(Value::as_str)
                    .map(PathBuf::from);
                let line = params
                    .get("line")
                    .and_then(Value::as_u64)
                    .map(|line| line as usize);
                state
                    .agents_open_in_code(id, path.as_deref(), line)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("agents.backToAgent", "Back to Agent", "Agents", None, "{}"),
            |s: &IdeState| s.agents.return_to.is_some(),
            |state: &mut IdeState, _| {
                state.agents_back();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "agents.newThread",
                "New Agent Thread",
                "Agents",
                Some("\u{2325}\u{21e7}\u{2318}Y"),
                "{}",
            ),
            |s: &IdeState| s.project.is_some(),
            |state: &mut IdeState, _| {
                state.set_workbench_mode(WorkbenchMode::Agents);
                state.agents.screen = AgentsScreen::Main;
                state.agents.open_draft();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "agents.closeDraft",
                "Close New Agent Thread",
                "Agents",
                None,
                "{}",
            ),
            |s: &IdeState| s.agents.new_thread,
            |state: &mut IdeState, _| {
                state.agents.close_draft();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "agents.closeThread",
                "Close Agent Thread",
                "Agents",
                None,
                r#"{"session":1}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = known_session(state, &params)?;
                state.agents.close_thread(id);
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    for (id, label, forward) in [
        ("agents.nextThread", "Next Agent Thread", true),
        ("agents.previousThread", "Previous Agent Thread", false),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Agents", None, "{}"),
                |s: &IdeState| s.agents.threads.len() > 1,
                move |state: &mut IdeState, _| {
                    state.agents.cycle_thread(forward);
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "agents.dismissNotice",
                "Dismiss Agent Notification",
                "Agents",
                None,
                r#"{"id":1}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = usize_param(&params, "id")? as u64;
                state.agents.notices.retain(|notice| notice.id != id);
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("agents.demo", "Start Agents Demo", "Agents", None, "{}"),
            |s: &IdeState| s.project.is_some(),
            |state: &mut IdeState, _| {
                state.seed_agents_demo();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(id: SessionId, state: AgentState, unread: bool) -> SessionInfo {
        SessionInfo {
            id,
            task: 1,
            parent: None,
            profile: AgentProfile::Claude,
            name: format!("a{id}"),
            state,
            unread,
            elapsed: Duration::ZERO,
            worktree: String::new(),
            branch: String::new(),
            tokens: 0,
            question: None,
            files: 0,
            added: 0,
            removed: 0,
            outcome: None,
        }
    }

    #[test]
    fn counts_and_status_text_follow_the_states() {
        let sessions = [
            info(1, AgentState::Working, false),
            info(2, AgentState::Working, false),
            info(3, AgentState::NeedsInput, true),
            info(4, AgentState::Done, true),
            info(5, AgentState::Done, false),
            info(6, AgentState::Error, true),
        ];
        let counts = AgentCounts::of(&sessions);
        assert_eq!(
            counts,
            AgentCounts {
                working: 2,
                needs_input: 1,
                done_unread: 1,
                failed: 1
            }
        );
        assert_eq!(counts.attention(), 3);
        assert_eq!(
            counts.status_text(),
            "2 running · ? 1 input · 1 done · 1 failed"
        );
        assert_eq!(AgentCounts::default().status_text(), "");
        assert!(AgentCounts::default().is_idle());
    }

    #[test]
    fn mode_round_trips_and_clears_balloons_when_agents_are_in_view() {
        let mut state = IdeState::default();
        assert_eq!(state.workbench_mode, WorkbenchMode::Code);
        state.agents.notices.push(AgentNotice {
            id: 1,
            session: 1,
            kind: NoticeKind::Done,
            title: "x".into(),
            body: String::new(),
            expires_ms: None,
        });
        state.set_workbench_mode(WorkbenchMode::Agents);
        assert!(state.agents.notices.is_empty());
        assert_eq!(WorkbenchMode::parse("agents"), Some(WorkbenchMode::Agents));
        assert_eq!(WorkbenchMode::parse("x"), None);
        assert_eq!(WorkbenchMode::Code.other(), WorkbenchMode::Agents);
    }

    #[test]
    fn the_draft_tab_opens_once_and_launching_replaces_it_with_agent_tabs() {
        let mut agents = AgentsState::default();
        agents.open_draft();
        agents.open_draft();
        assert!(agents.new_thread && agents.draft_active);
        agents.open_thread(1);
        assert!(
            agents.new_thread && !agents.draft_active,
            "the tab stays, inactive"
        );
        agents.open_draft();
        let task = agents.launch("Add retries", &[AgentProfile::Claude, AgentProfile::Codex]);
        assert!(task > 0);
        assert!(!agents.new_thread && !agents.draft_active);
        assert_eq!(agents.threads, [1, 2], "one tab per launched agent");
        agents.open_draft();
        agents.close_draft();
        assert!(!agents.new_thread && agents.selected.is_some());
    }

    #[test]
    fn threads_open_once_close_to_a_neighbour_and_cycle() {
        let mut agents = AgentsState::default();
        for id in [1, 2, 3, 2] {
            agents.open_thread(id);
        }
        assert_eq!(agents.threads, [1, 2, 3]);
        assert_eq!(agents.selected, Some(2));
        agents.close_thread(2);
        assert_eq!(agents.threads, [1, 3]);
        assert_eq!(agents.selected, Some(3), "the right neighbour takes over");
        agents.close_thread(3);
        assert_eq!(agents.selected, Some(1));
        agents.open_thread(3);
        agents.cycle_thread(true);
        assert_eq!(agents.selected, Some(1));
        agents.cycle_thread(false);
        assert_eq!(agents.selected, Some(3));
    }
}

//! The session engine: launches agents in git worktrees, follows their output, answers
//! permission requests, tracks changes, merges or discards the work and persists everything.
//!
//! Nothing here blocks the caller on an agent: worktree creation, the process itself and
//! change computation run on worker threads that report through one channel; the UI calls
//! [`Engine::poll`] once per frame to apply what arrived.

use crate::adapter::{Choice, Parsed};
use crate::claude::ClaudeAdapter;
use crate::codex::CodexAdapter;
use crate::events::{AgentEvent, ToolKind};
use crate::process::{self, Body, Proc, SpawnSpec};
use crate::profile::{
    self, AgentKind, PermissionMode, SessionArg, agent_env, build_command, find_cli, new_uuid,
};
use crate::redact::Redactor;
use crate::store::{SessionMeta, Store, repo_root_dir};
use crate::update::{Transcript, Update, first_line, truncate};
use crate::workspace::{self, Change, ChangeSet, Integration};
use archipel_ide_git::FileDiff;
use archipel_ide_git::worktrees;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

pub type SessionId = u32;
pub type TaskId = u32;

/// Text of the transcript row added when a running agent is cut off by the IDE closing.
pub const STOPPED_ON_CLOSE: &str = "Stopped when the IDE closed. Send a message to resume.";
pub const INTERRUPTED: &str = "Interrupted from the IDE. Send a message to resume.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Working,
    /// Blocked on a permission request.
    NeedsInput,
    Done,
    Failed,
    /// Interrupted (by the user or by the IDE closing); sending input resumes it.
    Stopped,
}

impl State {
    pub fn id(self) -> &'static str {
        match self {
            State::Working => "working",
            State::NeedsInput => "needs_input",
            State::Done => "done",
            State::Failed => "failed",
            State::Stopped => "stopped",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        [
            State::Working,
            State::NeedsInput,
            State::Done,
            State::Failed,
            State::Stopped,
        ]
        .into_iter()
        .find(|state| state.id() == id)
    }

    pub fn is_active(self) -> bool {
        matches!(self, State::Working | State::NeedsInput)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Squash,
    Merge,
    Discard,
}

impl Outcome {
    pub fn id(self) -> &'static str {
        match self {
            Outcome::Squash => "squash",
            Outcome::Merge => "merge",
            Outcome::Discard => "discard",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        [Outcome::Squash, Outcome::Merge, Outcome::Discard]
            .into_iter()
            .find(|o| o.id() == id)
    }
}

/// Something worth telling the user about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum News {
    NeedsInput(SessionId),
    Done(SessionId),
    Failed(SessionId),
    Spawned { parent: SessionId, child: SessionId },
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Root of persisted sessions and (by default) worktrees.
    pub data_dir: PathBuf,
    /// Worktrees go to `<dir>/<repo>-<hash>/<name>` instead of the data dir.
    pub worktree_base: Option<PathBuf>,
    /// Agents that may work at the same time (spawned agents included).
    pub max_parallel: usize,
    /// How deep agents may start agents (a top-level agent has depth 0).
    pub max_spawn_depth: u32,
    pub default_mode: PermissionMode,
    pub claude_path: Option<PathBuf>,
    pub codex_path: Option<PathBuf>,
    /// Time an agent gets to end after the graceful signal before it is killed.
    pub stop_grace: Duration,
    /// An idle (finished) agent process is stopped after this long; it resumes on demand.
    pub idle_reap: Duration,
    /// Extra environment for agents (explicit, never read from disk).
    pub extra_env: Vec<(String, String)>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            worktree_base: None,
            max_parallel: 4,
            max_spawn_depth: 2,
            default_mode: PermissionMode::Ask,
            claude_path: None,
            codex_path: None,
            stop_grace: Duration::from_secs(5),
            idle_reap: Duration::from_secs(10 * 60),
            extra_env: Vec::new(),
        }
    }
}

/// `$XDG_DATA_HOME` or `~/.local/share` (Linux), `~/Library/Application Support` (macOS),
/// `%LOCALAPPDATA%` (Windows); the temp dir when none is known.
pub fn default_data_dir() -> PathBuf {
    let env = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    let base = if cfg!(windows) {
        env("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        env("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| env("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    };
    base.unwrap_or_else(std::env::temp_dir)
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionSnapshot {
    pub id: SessionId,
    pub task: TaskId,
    pub parent: Option<SessionId>,
    pub kind: AgentKind,
    pub name: String,
    pub state: State,
    pub unread: bool,
    pub elapsed: Duration,
    pub worktree: PathBuf,
    pub branch: String,
    pub base_branch: String,
    pub tokens: u32,
    pub cost_usd: f64,
    pub model: Option<String>,
    pub question: Option<String>,
    pub files: usize,
    pub added: usize,
    pub removed: usize,
    pub outcome: Option<Outcome>,
    pub depth: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskSnapshot {
    pub id: TaskId,
    pub title: String,
    pub prompt: String,
    pub sessions: Vec<SessionId>,
}

struct Task {
    id: TaskId,
    title: String,
    prompt: String,
    sessions: Vec<SessionId>,
}

enum Adapter {
    Claude(ClaudeAdapter),
    Codex(CodexAdapter),
}

impl Adapter {
    fn new(kind: AgentKind, cwd: &Path) -> Self {
        match kind {
            AgentKind::Claude => Adapter::Claude(ClaudeAdapter::new(cwd)),
            AgentKind::Codex => Adapter::Codex(CodexAdapter::new()),
        }
    }

    fn parse_line(&mut self, line: &str) -> Parsed {
        match self {
            Adapter::Claude(a) => a.parse_line(line),
            Adapter::Codex(a) => a.parse_line(line),
        }
    }
}

struct Session {
    id: SessionId,
    task: TaskId,
    parent: Option<SessionId>,
    depth: u32,
    kind: AgentKind,
    name: String,
    branch: String,
    worktree: PathBuf,
    base_branch: String,
    mode: PermissionMode,
    state: State,
    unread: bool,
    outcome: Option<Outcome>,
    transcript: Transcript,
    adapter: Adapter,
    proc: Option<Proc>,
    /// A previous process that is still being stopped.
    dying: Option<Proc>,
    /// Bumped whenever the process is replaced or abandoned; stale messages are dropped.
    generation: u64,
    elapsed: Duration,
    working_since: Option<Instant>,
    turn_started: Option<Instant>,
    idle_since: Option<Instant>,
    /// Inputs waiting for the next turn (agents that take one prompt per process).
    queued: Vec<String>,
    stderr_tail: VecDeque<String>,
    worktree_ready: bool,
    changes: Vec<Change>,
    diffs: HashMap<PathBuf, FileDiff>,
    stats_in_flight: bool,
    stats_dirty: bool,
    patch_turn: bool,
}

impl Session {
    fn freeze_clock(&mut self) {
        if let Some(since) = self.working_since.take() {
            self.elapsed += since.elapsed();
        }
    }

    fn elapsed(&self) -> Duration {
        self.elapsed + self.working_since.map(|s| s.elapsed()).unwrap_or_default()
    }

    fn question(&self) -> Option<String> {
        self.transcript.events.iter().rev().find_map(|e| match e {
            AgentEvent::PermissionRequest {
                question,
                answer: None,
                ..
            } => Some(question.clone()),
            _ => None,
        })
    }
}

struct Msg {
    epoch: u64,
    session: SessionId,
    generation: u64,
    body: MsgBody,
}

enum MsgBody {
    Proc(Body),
    Started {
        worktree_created: bool,
        result: Result<Proc, String>,
    },
    Stats(Result<ChangeSet, String>),
}

pub struct Engine {
    config: EngineConfig,
    root: Option<PathBuf>,
    store: Option<Store>,
    redactor: Arc<Redactor>,
    epoch: u64,
    sessions: Vec<Session>,
    tasks: Vec<Task>,
    next_session: u32,
    next_task: u32,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    news: Vec<News>,
    /// Why the last user action did not happen (shown by the caller as a status).
    last_error: Option<String>,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            redactor: Arc::new(Redactor::new(profile::secret_values(
                std::env::vars().collect::<Vec<_>>(),
            ))),
            config,
            root: None,
            store: None,
            epoch: 0,
            sessions: Vec::new(),
            tasks: Vec::new(),
            next_session: 1,
            next_task: 1,
            tx,
            rx,
            news: Vec::new(),
            last_error: None,
        }
    }

    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: EngineConfig) {
        self.config = config;
    }

    /// The CLI that would run `kind`, when it is installed.
    pub fn cli_path(&self, kind: AgentKind) -> Option<PathBuf> {
        let override_path = match kind {
            AgentKind::Claude => self.config.claude_path.as_deref(),
            AgentKind::Codex => self.config.codex_path.as_deref(),
        };
        let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .map(PathBuf::from);
        find_cli(
            kind,
            override_path,
            std::env::var_os("PATH").as_deref(),
            home.as_deref(),
        )
    }

    pub fn available(&self) -> Vec<AgentKind> {
        AgentKind::ALL
            .into_iter()
            .filter(|kind| self.cli_path(*kind).is_some())
            .collect()
    }

    /// Takes the reason the last action failed, if it did.
    pub fn take_error(&mut self) -> Option<String> {
        self.last_error.take()
    }

    // -----------------------------------------------------------------------------------------
    // project and restore

    /// Switches the project. Agents of the previous one are stopped; the sessions stored for
    /// the new one are loaded, and those that were running become `Stopped` (resumable).
    pub fn set_project(&mut self, root: Option<&Path>) {
        if self.root.as_deref() == root {
            return;
        }
        self.stop_all(Duration::ZERO);
        self.epoch += 1;
        self.sessions.clear();
        self.tasks.clear();
        self.news.clear();
        self.next_session = 1;
        self.next_task = 1;
        self.root = root.map(Path::to_path_buf);
        self.store = None;
        let Some(root) = root else { return };
        let repo_dir = repo_root_dir(&self.config.data_dir, &worktrees::repo_dir_name(root));
        let store = Store::new(&repo_dir);
        for (meta, updates) in store.load_all() {
            self.restore(&store, meta, updates);
        }
        self.store = Some(store);
    }

    fn restore(&mut self, store: &Store, meta: SessionMeta, updates: Vec<Update>) {
        let Some(kind) = AgentKind::parse(&meta.kind) else {
            return;
        };
        let mut transcript = Transcript::default();
        for update in &updates {
            transcript.apply(update);
        }
        if transcript.provider_session.is_none() {
            transcript.provider_session = meta.provider_session.clone();
        }
        let mut state = State::parse(&meta.state).unwrap_or(State::Stopped);
        let was_active = state.is_active();
        if was_active {
            state = State::Stopped;
        }
        let mut session = Session {
            id: meta.id,
            task: meta.task,
            parent: meta.parent,
            depth: meta.depth,
            kind,
            name: meta.name.clone(),
            branch: meta.branch.clone(),
            worktree: meta.worktree.clone(),
            base_branch: meta.base_branch.clone(),
            mode: PermissionMode::parse(&meta.mode).unwrap_or_default(),
            state,
            unread: meta.unread,
            outcome: meta.outcome.as_deref().and_then(Outcome::parse),
            transcript,
            adapter: Adapter::new(kind, &meta.worktree),
            proc: None,
            dying: None,
            generation: 0,
            elapsed: Duration::from_millis(meta.elapsed_ms),
            working_since: None,
            turn_started: None,
            idle_since: None,
            queued: Vec::new(),
            stderr_tail: VecDeque::new(),
            worktree_ready: meta.worktree_ready,
            changes: Vec::new(),
            diffs: HashMap::new(),
            stats_in_flight: false,
            stats_dirty: false,
            patch_turn: false,
        };
        session.transcript.tokens = meta.tokens;
        session.transcript.cost_usd = meta.cost_usd;
        if session.transcript.model.is_none() {
            session.transcript.model = meta.model.clone();
        }
        if was_active {
            let update = Update::Push {
                event: AgentEvent::Error(STOPPED_ON_CLOSE.into()),
            };
            let _ = store.append(session.id, &update, &self.redactor);
            session.transcript.apply(&update);
        }
        self.next_session = self.next_session.max(meta.id + 1);
        self.next_task = self.next_task.max(meta.task + 1);
        match self.tasks.iter_mut().find(|t| t.id == meta.task) {
            Some(task) => task.sessions.push(meta.id),
            None => self.tasks.push(Task {
                id: meta.task,
                title: meta.task_title.clone(),
                prompt: meta.task_prompt.clone(),
                sessions: vec![meta.id],
            }),
        }
        let idx = self.sessions.len();
        self.sessions.push(session);
        if was_active {
            self.save(idx);
        }
        if self.sessions[idx].worktree_ready && self.sessions[idx].outcome.is_none() {
            self.request_stats(idx);
        }
    }

    // -----------------------------------------------------------------------------------------
    // reading

    pub fn tasks(&self) -> Vec<TaskSnapshot> {
        self.tasks
            .iter()
            .map(|t| TaskSnapshot {
                id: t.id,
                title: t.title.clone(),
                prompt: t.prompt.clone(),
                sessions: t.sessions.clone(),
            })
            .collect()
    }

    /// All sessions, parents before their children, oldest task first.
    pub fn sessions(&self) -> Vec<SessionSnapshot> {
        let mut out: Vec<SessionSnapshot> = Vec::new();
        for task in &self.tasks {
            let mut ids = task.sessions.clone();
            // Parents first, each followed by its children.
            let mut ordered = Vec::new();
            for id in ids.iter().copied().filter(|id| {
                self.session(*id)
                    .is_none_or(|s| s.parent.is_none_or(|p| !task.sessions.contains(&p)))
            }) {
                self.push_tree(id, &mut ordered);
            }
            for id in ids.drain(..) {
                if !ordered.contains(&id) {
                    ordered.push(id);
                }
            }
            out.extend(ordered.into_iter().filter_map(|id| self.snapshot(id)));
        }
        out
    }

    fn push_tree(&self, id: SessionId, out: &mut Vec<SessionId>) {
        if out.contains(&id) {
            return;
        }
        out.push(id);
        for child in self.sessions.iter().filter(|s| s.parent == Some(id)) {
            self.push_tree(child.id, out);
        }
    }

    fn session(&self, id: SessionId) -> Option<&Session> {
        self.sessions.iter().find(|s| s.id == id)
    }

    fn index(&self, id: SessionId) -> Option<usize> {
        self.sessions.iter().position(|s| s.id == id)
    }

    pub fn snapshot(&self, id: SessionId) -> Option<SessionSnapshot> {
        let s = self.session(id)?;
        Some(SessionSnapshot {
            id: s.id,
            task: s.task,
            parent: s.parent,
            kind: s.kind,
            name: s.name.clone(),
            state: s.state,
            unread: s.unread,
            elapsed: s.elapsed(),
            worktree: s.worktree.clone(),
            branch: s.branch.clone(),
            base_branch: s.base_branch.clone(),
            tokens: s.transcript.tokens,
            cost_usd: s.transcript.cost_usd,
            model: s.transcript.model.clone(),
            question: if s.state == State::NeedsInput {
                s.question()
            } else {
                None
            },
            files: s.changes.len(),
            added: s.changes.iter().map(|c| c.added).sum(),
            removed: s.changes.iter().map(|c| c.removed).sum(),
            outcome: s.outcome,
            depth: s.depth,
        })
    }

    pub fn events(&self, id: SessionId) -> &[AgentEvent] {
        self.session(id)
            .map_or(&[][..], |s| &s.transcript.events)
    }

    pub fn changes(&self, id: SessionId) -> &[Change] {
        self.session(id).map_or(&[][..], |s| &s.changes)
    }

    pub fn diff(&self, id: SessionId, path: &Path) -> Option<FileDiff> {
        self.session(id)?.diffs.get(path).cloned()
    }

    /// The agent's own session id (what `--resume` takes).
    pub fn provider_session(&self, id: SessionId) -> Option<String> {
        self.session(id)?.transcript.provider_session.clone()
    }

    /// Process ids of the agent (and of one being stopped), for diagnostics and tests.
    pub fn process_ids(&self, id: SessionId) -> Vec<u32> {
        self.session(id)
            .map(|s| {
                s.proc
                    .iter()
                    .chain(s.dying.iter())
                    .map(Proc::pid)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn mark_read(&mut self, id: SessionId) {
        if let Some(idx) = self.index(id)
            && self.sessions[idx].unread
        {
            self.sessions[idx].unread = false;
            self.save(idx);
        }
    }

    fn active_count(&self, except: Option<usize>) -> usize {
        self.sessions
            .iter()
            .enumerate()
            .filter(|(i, s)| Some(*i) != except && s.state.is_active())
            .count()
    }

    // -----------------------------------------------------------------------------------------
    // launching

    /// Starts one agent per kind on a new task. Failures (no project, no CLI, too many
    /// agents) show up as failed sessions with the reason in their transcript.
    pub fn launch(
        &mut self,
        prompt: &str,
        kinds: &[AgentKind],
        mode: Option<PermissionMode>,
    ) -> TaskId {
        let task = self.next_task;
        self.next_task += 1;
        self.tasks.push(Task {
            id: task,
            title: truncate(first_line(prompt, 200).as_str(), 48),
            prompt: prompt.to_owned(),
            sessions: Vec::new(),
        });
        let slug = slug_of(prompt);
        for kind in kinds {
            let base = if kinds.len() > 1 {
                format!("{slug}-{}", kind.id())
            } else {
                slug.clone()
            };
            self.create_session(task, None, 0, *kind, &base, prompt, mode);
        }
        task
    }

    /// Starts an agent on behalf of `parent` (the control socket's "spawn"). Refused beyond
    /// the spawn depth and the parallel limit.
    pub fn launch_child(
        &mut self,
        parent: SessionId,
        prompt: &str,
        kind: AgentKind,
    ) -> Result<SessionId, String> {
        let p = self
            .session(parent)
            .ok_or_else(|| format!("no agent session {parent}"))?;
        let (task, depth, mode, name) = (p.task, p.depth + 1, p.mode, p.name.clone());
        if depth > self.config.max_spawn_depth {
            return Err(format!(
                "{name} may not start agents: the spawn depth limit is {}",
                self.config.max_spawn_depth
            ));
        }
        if self.active_count(None) >= self.config.max_parallel {
            return Err(format!(
                "{} agents are already running (the limit is {})",
                self.active_count(None),
                self.config.max_parallel
            ));
        }
        let child = self.create_session(
            task,
            Some(parent),
            depth,
            kind,
            &slug_of(prompt),
            prompt,
            Some(mode),
        );
        if let Some(p) = self.index(parent) {
            let child_name = self
                .session(child)
                .map(|s| s.name.clone())
                .unwrap_or_default();
            self.apply(
                p,
                Update::Push {
                    event: AgentEvent::SubagentSpawned {
                        child,
                        name: child_name,
                    },
                },
            );
        }
        self.news.push(News::Spawned { parent, child });
        Ok(child)
    }

    #[allow(clippy::too_many_arguments)]
    fn create_session(
        &mut self,
        task: TaskId,
        parent: Option<SessionId>,
        depth: u32,
        kind: AgentKind,
        base_name: &str,
        prompt: &str,
        mode: Option<PermissionMode>,
    ) -> SessionId {
        let id = self.next_session;
        self.next_session += 1;
        let mode = mode.unwrap_or(self.config.default_mode);
        let (name, branch, worktree, base_branch, problem) = self.plan_session(base_name);
        let mut session = Session {
            id,
            task,
            parent,
            depth,
            kind,
            name,
            branch,
            worktree: worktree.clone(),
            base_branch,
            mode,
            state: State::Working,
            unread: false,
            outcome: None,
            transcript: Transcript::default(),
            adapter: Adapter::new(kind, &worktree),
            proc: None,
            dying: None,
            generation: 0,
            elapsed: Duration::ZERO,
            working_since: None,
            turn_started: None,
            idle_since: None,
            queued: Vec::new(),
            stderr_tail: VecDeque::new(),
            worktree_ready: false,
            changes: Vec::new(),
            diffs: HashMap::new(),
            stats_in_flight: false,
            stats_dirty: false,
            patch_turn: false,
        };
        session.transcript.apply(&Update::Push {
            event: AgentEvent::UserPrompt(prompt.to_owned()),
        });
        let idx = self.sessions.len();
        self.sessions.push(session);
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task) {
            task.sessions.push(id);
        }
        // The prompt is the first line of the log.
        let first = Update::Push {
            event: AgentEvent::UserPrompt(prompt.to_owned()),
        };
        self.persist_update(idx, &first);
        match problem {
            Some(problem) => self.fail(idx, &problem),
            None => self.start_turn(idx, prompt.to_owned()),
        }
        self.save(idx);
        id
    }

    /// Name, branch, worktree path and base branch of a new session, or a problem.
    fn plan_session(&self, base_name: &str) -> (String, String, PathBuf, String, Option<String>) {
        let fallback = |problem: &str| {
            (
                base_name.to_owned(),
                format!("archipel/{base_name}"),
                PathBuf::new(),
                String::new(),
                Some(problem.to_owned()),
            )
        };
        let Some(root) = self.root.as_deref() else {
            return fallback("Open a project before starting an agent.");
        };
        let Some(base_branch) = workspace::current_branch(root).or_else(|| {
            // Detached HEAD: branch from the commit, but merging back will need a branch.
            None
        }) else {
            return fallback(
                "The project has no branch checked out (detached HEAD). Check out a branch first.",
            );
        };
        let base_dir = match &self.config.worktree_base {
            Some(dir) => dir.join(worktrees::repo_dir_name(root)),
            None => worktrees::managed_base(&self.config.data_dir, root),
        };
        for n in 1..200u32 {
            let candidate = if n == 1 {
                base_name.to_owned()
            } else {
                format!("{base_name}-{n}")
            };
            let branch = format!("archipel/{candidate}");
            if self.sessions.iter().any(|s| s.name == candidate)
                || worktrees::branch_exists(root, &branch)
            {
                continue;
            }
            match worktrees::plan_path_in(&base_dir, root, &candidate) {
                Ok(path) if path.file_name().is_some_and(|n| n == candidate.as_str()) => {
                    return (candidate, branch, path, base_branch, None);
                }
                Ok(_) => continue,
                Err(error) => return fallback(&format!("Cannot place the worktree: {error}")),
            }
        }
        fallback("Could not find a free worktree name.")
    }

    // -----------------------------------------------------------------------------------------
    // turns

    /// Starts (or resumes) the agent process for one turn.
    fn start_turn(&mut self, idx: usize, prompt: String) {
        let limit = self.config.max_parallel.max(1);
        if self.active_count(Some(idx)) >= limit {
            let reason = format!(
                "{limit} agents are already working (the limit is a setting under Tools > \
                 Agents). Stop one or wait, then send a message to start this one."
            );
            self.fail(idx, &reason);
            return;
        }
        let kind = self.sessions[idx].kind;
        let Some(program) = self.cli_path(kind) else {
            let name = kind.binary();
            self.fail(
                idx,
                &format!("The `{name}` CLI was not found. Install it or set its path under Tools > Agents."),
            );
            return;
        };
        let Some(root) = self.root.clone() else {
            self.fail(idx, "No project is open.");
            return;
        };
        let session_arg = {
            let s = &mut self.sessions[idx];
            match (s.kind, s.transcript.provider_session.clone()) {
                (_, Some(id)) => SessionArg::Resume(id),
                (AgentKind::Claude, None) => {
                    let id = new_uuid();
                    s.transcript.provider_session = Some(id.clone());
                    SessionArg::New(id)
                }
                (AgentKind::Codex, None) => SessionArg::Fresh,
            }
        };
        let plan = {
            let s = &self.sessions[idx];
            build_command(s.kind, s.mode, &session_arg, &prompt, s.transcript.model.as_deref())
        };
        let env = agent_env(kind, std::env::vars_os(), &self.config.extra_env);
        let s = &mut self.sessions[idx];
        s.generation += 1;
        s.adapter = Adapter::new(kind, &s.worktree);
        s.state = State::Working;
        s.unread = false;
        s.idle_since = None;
        s.turn_started = Some(Instant::now());
        if s.working_since.is_none() {
            s.working_since = Some(Instant::now());
        }
        let spec = SpawnSpec {
            program,
            args: plan.args,
            cwd: s.worktree.clone(),
            env,
        };
        let prep = (!s.worktree_ready).then(|| {
            (
                root,
                s.worktree.clone(),
                s.branch.clone(),
                s.base_branch.clone(),
            )
        });
        let previous = s.dying.take();
        let grace = self.config.stop_grace;
        let (tx, epoch, session, generation) = (self.tx.clone(), self.epoch, s.id, s.generation);
        thread::spawn(move || {
            if let Some(previous) = previous {
                previous.wait_exit(grace + Duration::from_secs(2));
            }
            let mut created = false;
            if let Some((root, path, branch, base)) = prep {
                if let Err(error) = workspace::create_worktree(&root, &path, &branch, &base) {
                    let _ = tx.send(Msg {
                        epoch,
                        session,
                        generation,
                        body: MsgBody::Started {
                            worktree_created: false,
                            result: Err(error),
                        },
                    });
                    return;
                }
                created = true;
            }
            let sink: Arc<dyn Fn(Body) + Send + Sync> = {
                let tx = std::sync::Mutex::new(tx.clone());
                Arc::new(move |body| {
                    if let Ok(tx) = tx.lock() {
                        let _ = tx.send(Msg {
                            epoch,
                            session,
                            generation,
                            body: MsgBody::Proc(body),
                        });
                    }
                })
            };
            let result = process::spawn(&spec, sink)
                .map_err(|e| format!("could not start {}: {e}", spec.program.display()))
                .inspect(|proc| {
                    if let Some(first) = &plan.first_input {
                        let _ = proc.write_line(first);
                    }
                });
            let _ = tx.send(Msg {
                epoch,
                session,
                generation,
                body: MsgBody::Started {
                    worktree_created: created,
                    result,
                },
            });
        });
        self.save(idx);
    }

    fn fail(&mut self, idx: usize, message: &str) {
        self.apply(
            idx,
            Update::Push {
                event: AgentEvent::Error(message.to_owned()),
            },
        );
        let s = &mut self.sessions[idx];
        s.generation += 1;
        if let Some(proc) = s.proc.take() {
            proc.kill();
            s.dying = Some(proc);
        }
        s.freeze_clock();
        s.state = State::Failed;
        s.unread = true;
        let id = s.id;
        self.close_open_permissions(idx, "Cancelled");
        self.news.push(News::Failed(id));
        self.save(idx);
    }

    fn close_open_permissions(&mut self, idx: usize, answer: &str) {
        let mut ids = Vec::new();
        for event in &self.sessions[idx].transcript.events {
            if let AgentEvent::PermissionRequest {
                id, answer: None, ..
            } = event
            {
                ids.push(*id);
            }
        }
        if let Adapter::Claude(a) = &mut self.sessions[idx].adapter {
            a.drop_pending();
        }
        for id in ids {
            self.apply(
                idx,
                Update::Answered {
                    id,
                    answer: answer.to_owned(),
                },
            );
        }
    }

    fn finish_turn(&mut self, idx: usize, ok: bool) {
        let s = &mut self.sessions[idx];
        s.freeze_clock();
        s.turn_started = None;
        s.state = if ok { State::Done } else { State::Failed };
        s.unread = true;
        s.idle_since = Some(Instant::now());
        s.patch_turn = ok;
        let id = s.id;
        self.news.push(if ok {
            News::Done(id)
        } else {
            News::Failed(id)
        });
        self.request_stats(idx);
        self.save(idx);
    }

    // -----------------------------------------------------------------------------------------
    // input

    /// Answers a permission card (`y`, `a`, `n`) or sends `input` as the next prompt.
    pub fn send(&mut self, id: SessionId, input: &str) {
        let Some(idx) = self.index(id) else {
            return;
        };
        if self.sessions[idx].outcome.is_some() {
            self.last_error = Some("This agent's work was already merged or discarded.".into());
            return;
        }
        let text = input.trim_matches(|c| c == '\r' || c == '\n').trim();
        let open = self.sessions[idx].transcript.open_permission();
        if let (Some(permission), Some(choice)) = (open, Choice::from_input(text)) {
            self.answer(idx, permission, choice);
            return;
        }
        if text.is_empty() {
            return;
        }
        if let Some(permission) = open {
            // Free text while a card is open: the card is declined, the text goes on.
            self.answer(idx, permission, Choice::Deny);
        }
        self.apply(
            idx,
            Update::Push {
                event: AgentEvent::UserPrompt(text.to_owned()),
            },
        );
        let s = &mut self.sessions[idx];
        match (s.kind, s.state.is_active(), s.proc.is_some()) {
            (AgentKind::Claude, _, true) => {
                let line = ClaudeAdapter::user_message(text);
                let written = s.proc.as_ref().is_some_and(|p| p.write_line(&line).is_ok());
                if written {
                    if !s.state.is_active() {
                        s.state = State::Working;
                        s.working_since = Some(Instant::now());
                        s.turn_started = Some(Instant::now());
                    }
                    s.idle_since = None;
                    s.unread = false;
                    self.save(idx);
                    return;
                }
                s.queued.push(text.to_owned());
            }
            (_, true, _) => s.queued.push(text.to_owned()),
            _ => {
                self.start_turn(idx, text.to_owned());
                return;
            }
        }
        // Queued: it runs when the current process ends.
        let s = &self.sessions[idx];
        if s.proc.is_none() && !s.state.is_active() {
            let queued = std::mem::take(&mut self.sessions[idx].queued).join("\n\n");
            self.start_turn(idx, queued);
        }
    }

    fn answer(&mut self, idx: usize, permission: u32, choice: Choice) {
        let reply = match &mut self.sessions[idx].adapter {
            Adapter::Claude(a) => a.answer(permission, choice),
            Adapter::Codex(_) => None,
        };
        match reply {
            Some((line, update)) => {
                let written = self.sessions[idx]
                    .proc
                    .as_ref()
                    .is_some_and(|p| p.write_line(&line).is_ok());
                if !written {
                    self.last_error =
                        Some("The agent is no longer running; the request was dropped.".into());
                }
                self.apply(idx, update);
            }
            None => {
                // The request belongs to a process that is gone.
                self.apply(
                    idx,
                    Update::Answered {
                        id: permission,
                        answer: "Cancelled".into(),
                    },
                );
            }
        }
        let s = &mut self.sessions[idx];
        if s.state == State::NeedsInput && s.transcript.open_permission().is_none() {
            s.state = State::Working;
        }
        self.save(idx);
    }

    /// Interrupts the agent; sending input later resumes the same agent session.
    pub fn stop(&mut self, id: SessionId) {
        let Some(idx) = self.index(id) else {
            return;
        };
        let grace = self.config.stop_grace;
        let s = &mut self.sessions[idx];
        if !s.state.is_active() {
            // An idle process is just released.
            if let Some(proc) = s.proc.take() {
                s.generation += 1;
                proc.stop(grace);
                s.dying = Some(proc);
            }
            return;
        }
        s.generation += 1;
        if let Some(proc) = s.proc.take() {
            proc.stop(grace);
            s.dying = Some(proc);
        }
        s.freeze_clock();
        s.state = State::Stopped;
        s.unread = false;
        s.queued.clear();
        self.close_open_permissions(idx, "Cancelled");
        self.apply(
            idx,
            Update::Push {
                event: AgentEvent::Error(INTERRUPTED.into()),
            },
        );
        self.save(idx);
    }

    /// Stops every agent. Waits up to `grace` for a graceful end, then kills.
    fn stop_all(&mut self, grace: Duration) {
        let mut procs = Vec::new();
        for idx in 0..self.sessions.len() {
            let s = &mut self.sessions[idx];
            s.generation += 1;
            let was_active = s.state.is_active();
            let proc = s.proc.take();
            let dying = s.dying.take();
            procs.extend(proc);
            procs.extend(dying);
            if was_active {
                s.freeze_clock();
                s.state = State::Stopped;
                s.queued.clear();
                let update = Update::Push {
                    event: AgentEvent::Error(STOPPED_ON_CLOSE.into()),
                };
                self.close_open_permissions(idx, "Cancelled");
                self.apply(idx, update);
            }
            self.save(idx);
        }
        for proc in &procs {
            proc.stop(grace);
        }
        for proc in &procs {
            if !proc.wait_exit(grace + Duration::from_millis(500)) {
                proc.kill();
                proc.wait_exit(Duration::from_secs(1));
            }
        }
    }

    /// Called when the IDE closes: agents stop, their sessions stay resumable.
    pub fn shutdown(&mut self) {
        let grace = self.config.stop_grace.min(Duration::from_secs(2));
        self.stop_all(grace);
    }

    // -----------------------------------------------------------------------------------------
    // polling

    /// Applies everything the workers reported since the last call.
    pub fn poll(&mut self) -> Vec<News> {
        while let Ok(msg) = self.rx.try_recv() {
            if msg.epoch != self.epoch {
                if let MsgBody::Started { result: Ok(p), .. } = msg.body {
                    p.kill();
                }
                continue;
            }
            self.handle(msg);
        }
        self.reap_idle();
        std::mem::take(&mut self.news)
    }

    fn reap_idle(&mut self) {
        let limit = self.config.idle_reap;
        let grace = self.config.stop_grace;
        for s in &mut self.sessions {
            if !s.state.is_active()
                && s.proc.is_some()
                && s.idle_since.is_some_and(|since| since.elapsed() > limit)
            {
                if let Some(proc) = s.proc.take() {
                    s.generation += 1;
                    proc.stop(grace);
                    s.dying = Some(proc);
                }
            }
        }
    }

    fn handle(&mut self, msg: Msg) {
        let Some(idx) = self.index(msg.session) else {
            if let MsgBody::Started { result: Ok(p), .. } = msg.body {
                p.kill();
            }
            return;
        };
        let current = self.sessions[idx].generation == msg.generation;
        match msg.body {
            MsgBody::Started {
                worktree_created,
                result,
            } => {
                if worktree_created {
                    self.sessions[idx].worktree_ready = true;
                }
                if !current {
                    if let Ok(proc) = result {
                        proc.kill();
                    }
                    self.save(idx);
                    return;
                }
                match result {
                    Ok(proc) => self.sessions[idx].proc = Some(proc),
                    Err(error) => self.fail(idx, &error),
                }
                self.save(idx);
            }
            MsgBody::Proc(body) if current => match body {
                Body::Line(line) => self.on_line(idx, &line),
                Body::Stderr(line) => {
                    let tail = &mut self.sessions[idx].stderr_tail;
                    tail.push_back(truncate(line.trim(), 300));
                    while tail.len() > 20 {
                        tail.pop_front();
                    }
                }
                Body::Exited(code) => self.on_exit(idx, code),
            },
            MsgBody::Proc(_) => {}
            MsgBody::Stats(result) => self.on_stats(idx, result),
        }
    }

    fn on_line(&mut self, idx: usize, line: &str) {
        let parsed = self.sessions[idx].adapter.parse_line(line);
        for reply in &parsed.replies {
            if let Some(proc) = &self.sessions[idx].proc {
                let _ = proc.write_line(reply);
            }
        }
        let mut turn: Option<bool> = None;
        let mut files_touched = false;
        for mut update in parsed.updates {
            match &mut update {
                Update::Turn {
                    ok, duration_ms, ..
                } => {
                    if duration_ms.is_none() {
                        *duration_ms = self.sessions[idx]
                            .turn_started
                            .map(|t| t.elapsed().as_millis() as u64);
                    }
                    turn = Some(*ok);
                }
                Update::ToolResult { call, .. } => {
                    let call = *call;
                    files_touched |= self.sessions[idx].transcript.events.iter().any(|e| {
                        matches!(e, AgentEvent::ToolCall { id, kind, .. }
                            if *id == call && matches!(kind, ToolKind::Edit | ToolKind::Write | ToolKind::Bash))
                    });
                }
                _ => {}
            }
            self.apply(idx, update);
        }
        if let Some(ok) = turn {
            self.close_open_permissions(idx, "Cancelled");
            self.finish_turn(idx, ok);
        } else if self.sessions[idx].state == State::Working
            && self.sessions[idx].transcript.open_permission().is_some()
        {
            let s = &mut self.sessions[idx];
            s.state = State::NeedsInput;
            s.unread = true;
            let id = s.id;
            self.news.push(News::NeedsInput(id));
            self.save(idx);
        } else if files_touched {
            self.request_stats(idx);
        }
    }

    fn on_exit(&mut self, idx: usize, code: Option<i32>) {
        let s = &mut self.sessions[idx];
        s.proc = None;
        let (kind, active) = (s.kind, s.state.is_active());
        let tail: Vec<String> = s.stderr_tail.iter().rev().take(4).rev().cloned().collect();
        if active {
            match (kind, code) {
                (AgentKind::Codex, Some(0)) => {
                    // A turn that ended without a completion event.
                    self.finish_turn(idx, true);
                }
                _ => {
                    let how = match code {
                        Some(code) => format!("exited with code {code}"),
                        None => "was terminated".to_owned(),
                    };
                    let mut message = format!("{} {how} before finishing.", kind.binary());
                    if !tail.is_empty() {
                        message.push('\n');
                        message.push_str(&tail.join("\n"));
                    }
                    self.fail(idx, &message);
                }
            }
        } else {
            self.close_open_permissions(idx, "Cancelled");
        }
        let s = &mut self.sessions[idx];
        if !s.queued.is_empty() && s.state != State::Failed && s.state != State::Stopped {
            let next = std::mem::take(&mut s.queued).join("\n\n");
            self.start_turn(idx, next);
        } else if !s.queued.is_empty() {
            s.queued.clear();
        }
    }

    // -----------------------------------------------------------------------------------------
    // changes

    fn request_stats(&mut self, idx: usize) {
        let s = &mut self.sessions[idx];
        if !s.worktree_ready || s.outcome.is_some() || s.worktree.as_os_str().is_empty() {
            return;
        }
        if s.stats_in_flight {
            s.stats_dirty = true;
            return;
        }
        s.stats_in_flight = true;
        let (tx, epoch, session, generation) = (self.tx.clone(), self.epoch, s.id, s.generation);
        let (worktree, base) = (s.worktree.clone(), s.base_branch.clone());
        thread::spawn(move || {
            let result = if worktree.exists() {
                workspace::compute_changes(&worktree, &base)
            } else {
                Err("the worktree is gone".into())
            };
            let _ = tx.send(Msg {
                epoch,
                session,
                generation,
                body: MsgBody::Stats(result),
            });
        });
    }

    fn on_stats(&mut self, idx: usize, result: Result<ChangeSet, String>) {
        let s = &mut self.sessions[idx];
        s.stats_in_flight = false;
        if let Ok(set) = result {
            s.changes = set.changes;
            s.diffs = set.diffs;
            if s.patch_turn {
                s.patch_turn = false;
                let update = Update::TurnStats {
                    files: s.changes.len(),
                    added: s.changes.iter().map(|c| c.added).sum(),
                    removed: s.changes.iter().map(|c| c.removed).sum(),
                };
                self.apply(idx, update);
            }
            self.save(idx);
        }
        let s = &mut self.sessions[idx];
        if s.stats_dirty {
            s.stats_dirty = false;
            self.request_stats(idx);
        }
    }

    // -----------------------------------------------------------------------------------------
    // merge

    /// Squash-merges or merges the agent's work into its base branch and removes the
    /// worktree, or discards it.
    pub fn merge(&mut self, id: SessionId, outcome: Outcome) -> Result<String, String> {
        let idx = self
            .index(id)
            .ok_or_else(|| format!("no agent session {id}"))?;
        let s = &self.sessions[idx];
        if s.state.is_active() {
            return Err(format!(
                "{} is still running; stop it or wait until it is done",
                s.name
            ));
        }
        if s.outcome.is_some() {
            return Err(format!("{} was already handled", s.name));
        }
        let root = self.root.clone().ok_or("no project is open")?;
        let (name, branch, worktree, base) = (
            s.name.clone(),
            s.branch.clone(),
            s.worktree.clone(),
            s.base_branch.clone(),
        );
        let title = self
            .tasks
            .iter()
            .find(|t| t.id == s.task)
            .map(|t| (t.title.clone(), t.prompt.clone()))
            .unwrap_or_default();
        // The agent's process must be gone before its directory is.
        {
            let s = &mut self.sessions[idx];
            s.generation += 1;
            for proc in [s.proc.take(), s.dying.take()].into_iter().flatten() {
                proc.kill();
                proc.wait_exit(Duration::from_secs(3));
            }
        }
        if !self.sessions[idx].worktree_ready {
            if outcome != Outcome::Discard {
                return Err(format!("{name} never started; there is nothing to merge"));
            }
        } else {
            match outcome {
                Outcome::Discard => {
                    workspace::remove_worktree(&root, &worktree, &branch, true)?;
                }
                Outcome::Squash | Outcome::Merge => {
                    if let Ok(set) = workspace::compute_changes(&worktree, &base) {
                        self.sessions[idx].changes = set.changes;
                        self.sessions[idx].diffs = set.diffs;
                    }
                    let message = format!("{}\n\n{}", title.0, truncate(title.1.trim(), 1_000));
                    workspace::commit_all(&worktree, &format!("agent: {}", title.0))?;
                    if !workspace::has_new_commits(&root, &base, &branch) {
                        return Err(format!("{name} has no changes to merge"));
                    }
                    let how = if outcome == Outcome::Squash {
                        Integration::Squash
                    } else {
                        Integration::Merge
                    };
                    workspace::integrate(&root, &branch, &base, how, message.trim_end())?;
                }
            }
        }
        let (added, removed) = {
            let s = &self.sessions[idx];
            (
                s.changes.iter().map(|c| c.added).sum::<usize>(),
                s.changes.iter().map(|c| c.removed).sum::<usize>(),
            )
        };
        let mut warning = String::new();
        if matches!(outcome, Outcome::Squash | Outcome::Merge)
            && self.sessions[idx].worktree_ready
            && let Err(error) = workspace::remove_worktree(&root, &worktree, &branch, true)
        {
            warning = format!(" ({error})");
        }
        let s = &mut self.sessions[idx];
        s.outcome = Some(outcome);
        s.state = if s.state == State::Failed {
            State::Failed
        } else {
            State::Done
        };
        s.unread = false;
        if outcome == Outcome::Discard {
            s.changes.clear();
            s.diffs.clear();
        }
        self.save(idx);
        Ok(match outcome {
            Outcome::Discard => format!("Discarded the changes of {name}"),
            Outcome::Squash => {
                format!("Squash-merged {name} into {base} (+{added} -{removed}){warning}")
            }
            Outcome::Merge => format!("Merged {name} into {base} (+{added} -{removed}){warning}"),
        })
    }

    // -----------------------------------------------------------------------------------------
    // persistence

    fn apply(&mut self, idx: usize, update: Update) {
        self.sessions[idx].transcript.apply(&update);
        self.persist_update(idx, &update);
    }

    fn persist_update(&self, idx: usize, update: &Update) {
        if let Some(store) = &self.store {
            let _ = store.append(self.sessions[idx].id, update, &self.redactor);
        }
    }

    fn save(&self, idx: usize) {
        let Some(store) = &self.store else { return };
        let s = &self.sessions[idx];
        let Some(task) = self.tasks.iter().find(|t| t.id == s.task) else {
            return;
        };
        let meta = SessionMeta {
            id: s.id,
            task: s.task,
            task_title: task.title.clone(),
            task_prompt: task.prompt.clone(),
            parent: s.parent,
            depth: s.depth,
            kind: s.kind.id().to_owned(),
            name: s.name.clone(),
            branch: s.branch.clone(),
            worktree: s.worktree.clone(),
            base_branch: s.base_branch.clone(),
            provider_session: s.transcript.provider_session.clone(),
            model: s.transcript.model.clone(),
            mode: s.mode.id().to_owned(),
            state: s.state.id().to_owned(),
            outcome: s.outcome.map(|o| o.id().to_owned()),
            tokens: s.transcript.tokens,
            cost_usd: s.transcript.cost_usd,
            elapsed_ms: s.elapsed().as_millis() as u64,
            unread: s.unread,
            worktree_ready: s.worktree_ready,
        };
        let _ = store.save_meta(&meta);
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// A short lowercase slug for a worktree and branch name from the prompt.
pub fn slug_of(prompt: &str) -> String {
    let words: Vec<String> = prompt
        .split_whitespace()
        .take(6)
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect();
    let joined = truncate_chars(&words.join("-"), 40);
    worktrees::sanitize_name(&joined).unwrap_or_else(|_| "agent".into())
}

fn truncate_chars(text: &str, max: usize) -> String {
    text.chars().take(max).collect::<String>().trim_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_short_and_safe() {
        assert_eq!(slug_of("Fix tab ordering when the active tab is closed"), "fix-tab-ordering-when-the-active");
        assert_eq!(slug_of("  !!! "), "agent");
        assert_eq!(slug_of("../../etc/passwd now"), "etcpasswd-now");
        assert!(slug_of(&"word ".repeat(40)).len() <= 40);
    }

    #[test]
    fn states_round_trip() {
        for s in [State::Working, State::NeedsInput, State::Done, State::Failed, State::Stopped] {
            assert_eq!(State::parse(s.id()), Some(s));
        }
        assert!(State::Working.is_active() && !State::Stopped.is_active());
    }
}

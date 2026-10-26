//! A scripted stand-in for the agents backend (prototype only).
//!
//! No agent CLI is started. Each session plays a script on a clock and emits the same
//! structured [`AgentEvent`]s a stream-json agent would. Sessions go working, then (one of
//! them) needs input, then done; one agent starts a child agent; each produces a small fake
//! diff against real files of the open project (or a bundled sample). All mock behaviour
//! lives in this module; the rest of the IDE only sees [`AgentsBackend`].

use crate::agent_events::{
    AgentEvent, DiffLine, EditSummary, PERMISSION_OPTIONS, ToolKind, ToolStatus,
};
use crate::agents::{
    AgentNews, AgentProfile, AgentState, AgentsBackend, ChangeKind, ChangedFile, MergeStrategy,
    SessionId, SessionInfo, TaskId, TaskInfo,
};
use archipel_ide_git::FileDiff;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

const FAKE_BASE: &str = "a1b2c3d";

pub(crate) fn plural(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

/// Files used when the project offers nothing to edit.
const SAMPLE: [(&str, &str); 2] = [
    (
        "src/tabs.rs",
        "use std::collections::VecDeque;\n\n/// Open editor tabs in the order the user opened them.\npub struct Tabs {\n    order: VecDeque<String>,\n    active: Option<usize>,\n}\n\nimpl Tabs {\n    pub fn new() -> Self {\n        Self { order: VecDeque::new(), active: None }\n    }\n\n    pub fn open(&mut self, name: &str) {\n        self.order.push_back(name.to_owned());\n        self.active = Some(self.order.len() - 1);\n    }\n\n    pub fn close_active(&mut self) {\n        if let Some(index) = self.active.take() {\n            self.order.remove(index);\n            self.active = self.order.len().checked_sub(1);\n        }\n    }\n}\n",
    ),
    (
        "src/session.rs",
        "pub struct Session {\n    pub user: String,\n    pub token: String,\n}\n\npub fn login(user: &str) -> Session {\n    Session { user: user.to_owned(), token: String::from(\"anonymous\") }\n}\n\npub fn logout(session: Session) -> String {\n    format!(\"bye {}\", session.user)\n}\n",
    ),
];

/// One fake edit, prepared when the session starts and revealed by the script.
#[derive(Debug, Clone)]
struct PlannedEdit {
    path: PathBuf,
    kind: ChangeKind,
    old: String,
    new: String,
    added: usize,
    removed: usize,
    first_line: usize,
    snippet: Vec<DiffLine>,
}

enum Act {
    Pause,
    Ask(String),
    Edit(usize),
    Spawn(SpawnPlan),
    WaitChildren,
    Finish,
    Fail,
}

/// A change to the structured transcript made when a beat fires.
enum EvOp {
    Push(AgentEvent),
    /// Finishes the tool call `id`.
    EndTool {
        id: u32,
        status: ToolStatus,
        result: Option<String>,
        output: Option<String>,
    },
}

struct Beat {
    delay: Duration,
    act: Act,
    ev: Vec<EvOp>,
}

impl Beat {
    fn ev(mut self, op: EvOp) -> Self {
        self.ev.push(op);
        self
    }
}

struct SpawnPlan {
    profile: AgentProfile,
    prompt: String,
}

struct MockSession {
    info: SessionInfo,
    script: VecDeque<Beat>,
    /// Time left until the front beat fires.
    wait_left: Duration,
    asking: bool,
    waiting_children: bool,
    planned: Vec<PlannedEdit>,
    applied: Vec<usize>,
    /// What the user is typing into the terminal right now.
    typed: String,
    test_cmd: String,
    /// The structured transcript (what a stream-json agent would report).
    events: Vec<AgentEvent>,
}

impl MockSession {
    fn apply(&mut self, op: EvOp) {
        match op {
            EvOp::Push(event) => self.events.push(event),
            EvOp::EndTool {
                id,
                status,
                result,
                output,
            } => {
                for event in &mut self.events {
                    if let AgentEvent::ToolCall {
                        id: call,
                        status: current,
                        result: shown,
                        output: out,
                        ..
                    } = event
                        && *call == id
                    {
                        *current = status;
                        *shown = result.clone();
                        *out = output.clone();
                    }
                }
            }
        }
    }

    /// Records the user's answer on the open permission card.
    fn record_answer(&mut self, choice: &str) {
        for event in self.events.iter_mut().rev() {
            if let AgentEvent::PermissionRequest { answer, .. } = event
                && answer.is_none()
            {
                *answer = Some(choice.to_owned());
                break;
            }
        }
    }

    fn load(&mut self, beats: Vec<Beat>) {
        self.script = beats.into();
        self.wait_left = self.script.front().map(|b| b.delay).unwrap_or_default();
    }

    fn refresh_stats(&mut self) {
        let edits: Vec<&PlannedEdit> = self.applied.iter().map(|&i| &self.planned[i]).collect();
        self.info.files = edits.len();
        self.info.added = edits.iter().map(|e| e.added).sum();
        self.info.removed = edits.iter().map(|e| e.removed).sum();
    }
}

struct MockTask {
    id: TaskId,
    title: String,
    prompt: String,
    sessions: Vec<SessionId>,
}

#[derive(Default)]
pub struct MockAgentsBackend {
    root: Option<PathBuf>,
    /// Project files the fake edits are made against: (project-relative path, text).
    candidates: Vec<(PathBuf, String)>,
    test_cmd: String,
    tasks: Vec<MockTask>,
    sessions: Vec<MockSession>,
    next_task: TaskId,
    next_session: SessionId,
}

impl MockAgentsBackend {
    pub fn new(root: Option<&Path>) -> Self {
        let mut backend = Self::default();
        backend.set_project(root);
        backend
    }

    fn files(&self) -> Vec<(PathBuf, String)> {
        if self.candidates.is_empty() {
            return SAMPLE
                .iter()
                .map(|(path, text)| (PathBuf::from(path), (*text).to_owned()))
                .collect();
        }
        self.candidates.clone()
    }

    fn test_cmd(&self) -> String {
        if self.test_cmd.is_empty() {
            "make test".into()
        } else {
            self.test_cmd.clone()
        }
    }

    fn index(&self, id: SessionId) -> Option<usize> {
        self.sessions.iter().position(|s| s.info.id == id)
    }

    fn project_name(&self) -> String {
        self.root
            .as_deref()
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "project".into())
    }

    fn unique_name(&self, base: &str) -> String {
        let taken = |name: &str| self.sessions.iter().any(|s| s.info.name == name);
        if !taken(base) {
            return base.to_owned();
        }
        (2..)
            .map(|n| format!("{base}-{n}"))
            .find(|name| !taken(name))
            .expect("an unused suffix exists")
    }

    /// Creates a session with its script, plans its edits and attaches it to `task`.
    fn create_session(
        &mut self,
        task: TaskId,
        parent: Option<SessionId>,
        profile: AgentProfile,
        name: String,
        prompt: &str,
    ) -> SessionId {
        let id = self.next_session + 1;
        self.next_session = id;
        let seed = self.sessions.len();
        let planned = self.plan_edits(&name, profile, prompt, seed, parent.is_some());
        let test_cmd = self.test_cmd();
        let failing = ["fail", "flaky", "spike"]
            .iter()
            .any(|word| prompt.to_lowercase().contains(word));
        let beats = script(&ScriptInput {
            profile,
            prompt,
            test_cmd: &test_cmd,
            planned: &planned,
            failing,
            child: parent.is_some(),
            child_name: format!("{name}-tests"),
        });
        let worktree = format!("~/wt/{}/{name}", self.project_name());
        let branch = format!("agent/{name}");
        let mut session = MockSession {
            info: SessionInfo {
                id,
                task,
                parent,
                profile,
                name,
                state: AgentState::Working,
                unread: false,
                elapsed: Duration::ZERO,
                worktree,
                branch,
                tokens: 0,
                question: None,
                files: 0,
                added: 0,
                removed: 0,
                outcome: None,
            },
            script: VecDeque::new(),
            wait_left: Duration::ZERO,
            asking: false,
            waiting_children: false,
            planned,
            applied: Vec::new(),
            typed: String::new(),
            test_cmd: test_cmd.clone(),
            events: vec![AgentEvent::UserPrompt(prompt.to_owned())],
        };
        session.load(beats);
        self.sessions.push(session);
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task) {
            task.sessions.push(id);
        }
        id
    }

    fn plan_edits(
        &self,
        name: &str,
        profile: AgentProfile,
        prompt: &str,
        seed: usize,
        child: bool,
    ) -> Vec<PlannedEdit> {
        let files = self.files();
        let note = short(prompt, 46);
        let mut edits = Vec::new();
        if !child {
            for offset in 0..2 {
                let (path, old) = &files[(seed + offset) % files.len()];
                if edits.iter().any(|e: &PlannedEdit| &e.path == path) {
                    continue;
                }
                let new = fake_edit(path, old, &note, seed);
                edits.push(planned(
                    path.clone(),
                    ChangeKind::Modified,
                    old.clone(),
                    new,
                ));
            }
        }
        let notes = PathBuf::from(format!("docs/agents/{name}.md"));
        let text = format!(
            "# {name}\n\nNotes from the {} agent.\n\n- Task: {prompt}\n- Verified with `{}`\n",
            profile.id(),
            self.test_cmd()
        );
        edits.push(planned(notes, ChangeKind::Added, String::new(), text));
        edits
    }

    fn step(&mut self, index: usize, dt: Duration, events: &mut Vec<AgentNews>) {
        if !self.sessions[index].info.state.is_active() {
            return;
        }
        // The clock stops while the agent waits for the user.
        if self.sessions[index].asking {
            return;
        }
        let info = &mut self.sessions[index].info;
        info.elapsed += dt;
        info.tokens += (dt.as_millis() as u32) * 3 / 2;
        if self.sessions[index].waiting_children {
            let id = self.sessions[index].info.id;
            let busy = self
                .sessions
                .iter()
                .any(|c| c.info.parent == Some(id) && c.info.state.is_active());
            if busy {
                return;
            }
            self.sessions[index].waiting_children = false;
        }
        let mut budget = dt;
        loop {
            let session = &mut self.sessions[index];
            if session.script.is_empty() {
                return;
            }
            if session.wait_left > budget {
                session.wait_left -= budget;
                return;
            }
            budget -= session.wait_left;
            let beat = session.script.pop_front().expect("checked above");
            session.wait_left = session.script.front().map(|b| b.delay).unwrap_or_default();
            for op in beat.ev {
                self.sessions[index].apply(op);
            }
            if self.run(index, beat.act, events) {
                return;
            }
        }
    }

    /// Runs one beat; true when the session cannot continue this tick.
    fn run(&mut self, index: usize, act: Act, events: &mut Vec<AgentNews>) -> bool {
        let id = self.sessions[index].info.id;
        match act {
            Act::Pause => {}
            Act::Ask(question) => {
                let session = &mut self.sessions[index];
                session.info.state = AgentState::NeedsInput;
                session.info.question = Some(question);
                session.info.unread = true;
                session.asking = true;
                events.push(AgentNews::NeedsInput(id));
                return true;
            }
            Act::Edit(slot) => {
                let session = &mut self.sessions[index];
                if slot < session.planned.len() && !session.applied.contains(&slot) {
                    session.applied.push(slot);
                    session.refresh_stats();
                }
            }
            Act::Spawn(plan) => {
                let (task, parent_name) = {
                    let s = &self.sessions[index];
                    (s.info.task, s.info.name.clone())
                };
                let name = self.unique_name(&format!("{parent_name}-tests"));
                let child = self.create_session(task, Some(id), plan.profile, name, &plan.prompt);
                let child_name = self
                    .sessions
                    .iter()
                    .find(|s| s.info.id == child)
                    .map(|s| s.info.name.clone())
                    .unwrap_or_default();
                self.sessions[index]
                    .events
                    .push(AgentEvent::SubagentSpawned {
                        child,
                        name: child_name,
                    });
                events.push(AgentNews::Spawned { parent: id, child });
            }
            Act::WaitChildren => {
                let has_children = self.sessions.iter().any(|c| c.info.parent == Some(id));
                self.sessions[index].waiting_children = has_children;
                return has_children;
            }
            Act::Finish => {
                let session = &mut self.sessions[index];
                let info = &session.info;
                let result = AgentEvent::TurnResult {
                    files: info.files,
                    added: info.added,
                    removed: info.removed,
                    duration: info.elapsed,
                    tokens: 3_200 + 900 * info.files as u32 + info.elapsed.as_secs() as u32 * 120,
                };
                session.events.push(result);
                session.info.state = AgentState::Done;
                session.info.unread = true;
                session.info.question = None;
                events.push(AgentNews::Done(id));
                return true;
            }
            Act::Fail => {
                let session = &mut self.sessions[index];
                session.events.push(AgentEvent::Error(
                    "The task failed; see the log above.".into(),
                ));
                session.info.state = AgentState::Error;
                session.info.unread = true;
                session.info.question = None;
                events.push(AgentNews::Failed(id));
                return true;
            }
        }
        false
    }

    /// The user typed a full line into an agent that is not asking anything.
    fn submit_line(&mut self, index: usize, line: &str) {
        let session = &mut self.sessions[index];
        session.events.push(AgentEvent::UserPrompt(line.to_owned()));
        if session.info.state == AgentState::Working {
            return;
        }
        if session.info.outcome.is_some() {
            return;
        }
        let resumed = session.info.state != AgentState::Done;
        session.info.state = AgentState::Working;
        session.info.unread = false;
        let cmd = session.test_cmd.clone();
        let said = if resumed {
            "Resuming where I stopped."
        } else {
            "On it."
        };
        let mut beats = vec![
            beat(400).ev(EvOp::Push(AgentEvent::AssistantText(said.into()))),
            beat(900).ev(start(
                90,
                ToolKind::Read,
                "Read notes".into(),
                "docs/agents/notes.md".into(),
            )),
            beat(800).ev(finish(90, true, Some("ok"), None)).ev(start(
                91,
                ToolKind::Bash,
                format!("Run {cmd}"),
                cmd.clone(),
            )),
            beat(900).ev(finish(
                91,
                true,
                Some("passed"),
                Some("all checks passed".into()),
            )),
            beat(300),
        ];
        beats.push(act(100, Act::Finish));
        session.load(beats);
    }

    fn answer(&mut self, index: usize, choice: char) {
        let session = &mut self.sessions[index];
        session.asking = false;
        session.info.state = AgentState::Working;
        session.info.question = None;
        if choice != 'n' {
            let label = if choice == 'a' {
                PERMISSION_OPTIONS[1]
            } else {
                PERMISSION_OPTIONS[0]
            };
            session.record_answer(label);
            return;
        }
        session.record_answer(PERMISSION_OPTIONS[2]);
        session.applied.clear();
        session.refresh_stats();
        session.load(vec![
            beat(400).ev(EvOp::Push(AgentEvent::AssistantText(
                "Understood, I will not edit that file.".into(),
            ))),
            beat(600).ev(EvOp::Push(AgentEvent::AssistantText(
                "Stopped without changes. Ask again with more context.".into(),
            ))),
            act(100, Act::Finish),
        ]);
    }
}

impl AgentsBackend for MockAgentsBackend {
    fn set_project(&mut self, root: Option<&Path>) {
        self.root = root.map(Path::to_path_buf);
        self.candidates = root.map(collect_candidates).unwrap_or_default();
        self.test_cmd = root.map(test_command).unwrap_or("make test").to_owned();
    }

    fn tasks(&self) -> Vec<TaskInfo> {
        self.tasks
            .iter()
            .map(|task| TaskInfo {
                id: task.id,
                title: task.title.clone(),
                prompt: task.prompt.clone(),
                sessions: task.sessions.clone(),
            })
            .collect()
    }

    fn sessions(&self) -> Vec<SessionInfo> {
        self.tasks
            .iter()
            .flat_map(|task| task.sessions.iter())
            .filter_map(|id| self.index(*id))
            .map(|index| self.sessions[index].info.clone())
            .collect()
    }

    fn launch(&mut self, prompt: &str, profiles: &[AgentProfile]) -> TaskId {
        let id = self.next_task + 1;
        self.next_task = id;
        self.tasks.push(MockTask {
            id,
            title: short(prompt, 48),
            prompt: prompt.to_owned(),
            sessions: Vec::new(),
        });
        let slug = slug(prompt);
        for profile in profiles {
            let base = if profiles.len() > 1 {
                format!("{slug}-{}", profile.id())
            } else {
                slug.clone()
            };
            let name = self.unique_name(&base);
            self.create_session(id, None, *profile, name, prompt);
        }
        id
    }

    fn send(&mut self, session: SessionId, input: &str) {
        let Some(index) = self.index(session) else {
            return;
        };
        for ch in input.chars() {
            if self.sessions[index].asking {
                if let c @ ('y' | 'a' | 'n') = ch.to_ascii_lowercase() {
                    self.answer(index, c);
                }
                continue;
            }
            match ch {
                '\r' | '\n' => {
                    let line = std::mem::take(&mut self.sessions[index].typed);
                    if !line.trim().is_empty() {
                        self.submit_line(index, line.trim());
                    }
                }
                '\u{7f}' | '\u{8}' => {
                    let session = &mut self.sessions[index];
                    session.typed.pop();
                }
                c if !c.is_control() => {
                    let session = &mut self.sessions[index];
                    session.typed.push(c);
                }
                _ => {}
            }
        }
    }

    fn stop(&mut self, session: SessionId) {
        let Some(index) = self.index(session) else {
            return;
        };
        let session = &mut self.sessions[index];
        if !session.info.state.is_active() {
            return;
        }
        session.events.push(AgentEvent::Error(
            "Interrupted from the IDE. Send a message to resume.".into(),
        ));
        session.info.state = AgentState::Stopped;
        session.info.question = None;
        session.info.unread = false;
        session.asking = false;
        session.waiting_children = false;
        session.script.clear();
    }

    fn changes(&self, session: SessionId) -> Vec<ChangedFile> {
        let Some(index) = self.index(session) else {
            return Vec::new();
        };
        let session = &self.sessions[index];
        session
            .applied
            .iter()
            .map(|&i| {
                let edit = &session.planned[i];
                ChangedFile {
                    path: edit.path.clone(),
                    kind: edit.kind,
                    added: edit.added,
                    removed: edit.removed,
                    first_line: edit.first_line,
                }
            })
            .collect()
    }

    fn diff(&self, session: SessionId, path: &Path) -> Option<FileDiff> {
        let session = &self.sessions[self.index(session)?];
        let edit = session
            .applied
            .iter()
            .map(|&i| &session.planned[i])
            .find(|e| e.path == path)?;
        Some(FileDiff::compute(
            edit.path.clone(),
            (edit.kind == ChangeKind::Modified).then(|| FAKE_BASE.to_owned()),
            edit.old.clone(),
            edit.new.clone(),
        ))
    }

    fn merge(&mut self, session: SessionId, strategy: MergeStrategy) -> Result<String, String> {
        let index = self
            .index(session)
            .ok_or_else(|| format!("no agent session {session}"))?;
        let session = &mut self.sessions[index];
        if session.info.state.is_active() {
            return Err(format!(
                "{} is still running; stop it or wait until it is done",
                session.info.name
            ));
        }
        if session.info.outcome.is_some() {
            return Err(format!("{} was already handled", session.info.name));
        }
        let name = session.info.name.clone();
        match strategy {
            MergeStrategy::Discard => {
                session.applied.clear();
                session.refresh_stats();
                session.info.outcome = Some(strategy);
                Ok(format!("Discarded the changes of {name}"))
            }
            MergeStrategy::Squash | MergeStrategy::Merge => {
                if session.applied.is_empty() {
                    return Err(format!("{name} has no changes to merge"));
                }
                session.info.outcome = Some(strategy);
                let how = if strategy == MergeStrategy::Squash {
                    "Squash-merged"
                } else {
                    "Merged"
                };
                Ok(format!(
                    "{how} {name} into main (+{} -{})",
                    session.info.added, session.info.removed
                ))
            }
        }
    }

    fn events(&self, session: SessionId) -> &[AgentEvent] {
        self.index(session)
            .map_or(&[][..], |i| &self.sessions[i].events)
    }

    fn mark_read(&mut self, session: SessionId) {
        if let Some(index) = self.index(session) {
            self.sessions[index].info.unread = false;
        }
    }

    fn advance(&mut self, dt: Duration) -> Vec<AgentNews> {
        let mut events = Vec::new();
        for index in 0..self.sessions.len() {
            self.step(index, dt, &mut events);
        }
        events
    }
}

/// A beat that only changes the transcript.
fn beat(delay_ms: u64) -> Beat {
    act(delay_ms, Act::Pause)
}

fn act(delay_ms: u64, act: Act) -> Beat {
    Beat {
        delay: Duration::from_millis(delay_ms),
        act,
        ev: Vec::new(),
    }
}

struct ScriptInput<'a> {
    profile: AgentProfile,
    prompt: &'a str,
    test_cmd: &'a str,
    planned: &'a [PlannedEdit],
    failing: bool,
    child: bool,
    child_name: String,
}

/// The scripted session of one agent. Timings add up to a few seconds. Beats change the
/// structured transcript through `ev`.
fn script(input: &ScriptInput) -> Vec<Beat> {
    if input.child {
        return child_script(input);
    }
    let p = input.prompt;
    let cmd = input.test_cmd;
    let kw = keyword(p);
    let first = &input.planned[0];
    let f0 = first.path.display().to_string();
    let n0 = file_name(&first.path);
    let second = input
        .planned
        .get(1)
        .filter(|e| e.kind == ChangeKind::Modified);
    let notes_slot = input.planned.len() - 1;
    let test_output = String::from(
        "running 12 tests\ntest keeps_order ... ok\ntest selects_neighbour ... ok\ntest closes_last_tab ... ok\n\ntest result: ok. 12 passed; 0 failed",
    );
    let search_output = format!(
        "{f0}:12: ...{kw}...\n{f0}:31: ...{kw}...\n{}:7: ...{kw}...",
        second.map_or("README.md".to_owned(), |e| e.path.display().to_string())
    );
    let mut beats = Vec::new();
    match input.profile {
        AgentProfile::Claude => {
            beats.push(beat(0));
            beats.push(beat(250).ev(EvOp::Push(
                AgentEvent::Thinking(format!(
                    "The task is about {kw}. I should find where it is handled before changing anything, then run {cmd}."
                )),
            )));
            beats.push(beat(500).ev(say(format!("Let me find where **{kw}** is handled."))));
            beats.push(beat(700).ev(start(
                1,
                ToolKind::Search,
                format!("Search \"{kw}\""),
                "in the project".into(),
            )));
            beats.push(
                beat(600)
                    .ev(finish(1, true, Some("7 matches"), Some(search_output)))
                    .ev(start(2, ToolKind::Read, format!("Read {n0}"), f0.clone())),
            );
            beats.push(
                beat(500)
                    .ev(finish(2, true, None, Some(head(&first.old))))
                    .ev(say(format!("The change belongs in `{n0}`."))),
            );
            beats.push(beat(400).ev(EvOp::Push(AgentEvent::PermissionRequest {
                id: 10,
                question: format!("Claude wants to edit {f0}"),
                options: PERMISSION_OPTIONS.map(str::to_owned).to_vec(),
                answer: None,
            })));
            if input.failing {
                beats.push(act(300, Act::Edit(0)).ev(edit_done(3, "Edit", first)));
                beats.push(beat(500).ev(say("Wiring up the cache client.".into())));
                beats.push(beat(700).ev(start(
                    4,
                    ToolKind::Bash,
                    format!("Run {cmd}"),
                    cmd.to_owned(),
                )));
                beats.push(
                    beat(700)
                    .ev(finish(
                        4,
                        false,
                        Some("connection refused"),
                        Some("error: could not connect to redis://127.0.0.1:6379\nconnection refused (os error 111)".into()),
                    )),
                );
                beats.push(beat(500).ev(say("Retrying with a local fallback.".into())));
                beats.push(beat(600));
                beats.push(act(100, Act::Fail));
                return beats;
            }
            beats.push(act(100, Act::Ask(format!("  Allow edit to {f0}? (y/n) "))));
            beats.push(act(200, Act::Edit(0)).ev(edit_done(3, "Edit", first)));
            beats.push(beat(100));
            if let Some(second) = second {
                beats.push(beat(800));
                beats.push(act(200, Act::Edit(1)).ev(edit_done(5, "Edit", second)));
                beats.push(beat(100));
            }
            beats.push(beat(600).ev(start(
                6,
                ToolKind::Bash,
                format!("Run {cmd}"),
                cmd.to_owned(),
            )));
            beats.push(beat(500));
            for _ in 0..3 {
                beats.push(beat(350));
            }
            beats.push(beat(600).ev(finish(6, true, Some("12 passed"), Some(test_output))));
            beats.push(act(150, Act::Edit(notes_slot)));
            beats.push(beat(400).ev(say(format!(
                "Done. The tests pass and **{}** changed.",
                plural(input.planned.len(), "file", "files")
            ))));
            beats.push(act(100, Act::Finish));
        }
        AgentProfile::Codex => {
            beats.push(beat(0));
            beats.push(
                beat(250).ev(say(format!(
                    "Planning in three steps:\n\n1. read `{n0}`\n2. implement the change\n3. cover it with tests and run `{cmd}`"
                ))),
            );
            beats.push(beat(500));
            beats.push(beat(700).ev(start(
                1,
                ToolKind::Search,
                format!("Search \"{kw}\""),
                format!("rg -n \"{kw}\" ."),
            )));
            beats.push(
                beat(600)
                    .ev(finish(1, true, Some("5 matches"), Some(search_output)))
                    .ev(start(2, ToolKind::Read, format!("Read {n0}"), f0.clone())),
            );
            beats.push(
                beat(700)
                    .ev(finish(2, true, None, Some(head(&first.old))))
                    .ev(say(
                        "I will edit the code here and hand the tests to a second agent.".into(),
                    )),
            );
            beats.push(act(
                500,
                Act::Spawn(SpawnPlan {
                    profile: AgentProfile::Codex,
                    prompt: format!("Write tests for: {p}"),
                }),
            ));
            beats.push(beat(50));
            beats.push(act(600, Act::Edit(0)).ev(edit_done(3, "Edit", first)));
            beats.push(beat(50));
            if let Some(second) = second {
                beats.push(act(800, Act::Edit(1)).ev(edit_done(4, "Edit", second)));
                beats.push(beat(50));
            }
            beats.push(beat(700).ev(say(format!(
                "Waiting for `{}` to finish the tests.",
                input.child_name
            ))));
            beats.push(act(100, Act::WaitChildren));
            beats.push(beat(300));
            beats.push(beat(500).ev(start(
                5,
                ToolKind::Bash,
                format!("Run {cmd}"),
                cmd.to_owned(),
            )));
            beats.push(beat(700).ev(finish(5, true, Some("all passed"), Some(test_output))));
            beats.push(act(150, Act::Edit(notes_slot)));
            beats.push(beat(400).ev(say("All checks pass.".into())));
            beats.push(act(100, Act::Finish));
        }
        AgentProfile::Gemini | AgentProfile::Opencode => {
            beats.push(beat(0));
            beats.push(beat(250));
            beats.push(beat(600).ev(say("Reading the project files first.".into())));
            beats.push(beat(700).ev(start(1, ToolKind::Read, format!("Read {n0}"), f0.clone())));
            if let Some(second) = second {
                beats.push(
                    beat(600)
                        .ev(finish(1, true, None, Some(head(&first.old))))
                        .ev(start(
                            2,
                            ToolKind::Read,
                            format!("Read {}", file_name(&second.path)),
                            second.path.display().to_string(),
                        )),
                );
            }
            beats.push(
                beat(800)
                    .ev(finish(2, true, None, second.map(|e| head(&e.old))))
                    .ev(say("Applying the change to both files.".into())),
            );
            beats.push(act(400, Act::Edit(0)).ev(edit_done(3, "Edit", first)));
            beats.push(beat(50));
            if let Some(second) = second {
                beats.push(act(700, Act::Edit(1)).ev(edit_done(4, "Edit", second)));
                beats.push(beat(50));
            }
            beats.push(beat(600).ev(start(
                5,
                ToolKind::Bash,
                format!("Run {cmd}"),
                cmd.to_owned(),
            )));
            beats.push(beat(900).ev(finish(5, true, Some("ok"), Some(test_output))));
            beats.push(act(150, Act::Edit(notes_slot)));
            beats.push(beat(400).ev(say("Task complete.".into())));
            beats.push(act(100, Act::Finish));
        }
    }
    beats
}

fn child_script(input: &ScriptInput) -> Vec<Beat> {
    let cmd = input.test_cmd;
    vec![
        beat(0).ev(EvOp::Push(AgentEvent::UserPrompt(input.prompt.to_owned()))),
        beat(300).ev(say("Writing the tests next to the change.".into())),
        beat(700),
        act(800, Act::Edit(0)).ev(edit_done(1, "Create", &input.planned[0])),
        beat(50),
        beat(900).ev(start(
            2,
            ToolKind::Bash,
            format!("Run {cmd}"),
            cmd.to_owned(),
        )),
        beat(900).ev(finish(
            2,
            true,
            Some("3 passed"),
            Some("test a ... ok\ntest b ... ok\ntest c ... ok".into()),
        )),
        beat(300),
        act(100, Act::Finish),
    ]
}

fn planned(path: PathBuf, kind: ChangeKind, old: String, new: String) -> PlannedEdit {
    let diff = FileDiff::compute(path.clone(), None, old.clone(), new.clone());
    let added = diff.hunks.iter().map(|h| h.new.len()).sum();
    let removed = diff.hunks.iter().map(|h| h.old.len()).sum();
    let first_line = diff.hunks.first().map_or(1, |h| h.new.start + 1);
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let mut snippet = Vec::new();
    for hunk in diff.hunks.iter().take(2) {
        if let Some(before) = hunk.new.start.checked_sub(1).and_then(|i| new_lines.get(i)) {
            snippet.push(DiffLine {
                sign: ' ',
                text: (*before).to_owned(),
            });
        }
        for line in old_lines.iter().take(hunk.old.end).skip(hunk.old.start) {
            snippet.push(DiffLine {
                sign: '-',
                text: (*line).to_owned(),
            });
        }
        for line in new_lines.iter().take(hunk.new.end).skip(hunk.new.start) {
            snippet.push(DiffLine {
                sign: '+',
                text: (*line).to_owned(),
            });
        }
    }
    snippet.truncate(12);
    PlannedEdit {
        path,
        kind,
        old,
        new,
        added,
        removed,
        first_line,
        snippet,
    }
}

fn start(id: u32, kind: ToolKind, title: String, args: String) -> EvOp {
    EvOp::Push(AgentEvent::ToolCall {
        id,
        kind,
        title,
        args,
        status: ToolStatus::Running,
        result: None,
        output: None,
        edit: None,
    })
}

fn finish(id: u32, ok: bool, result: Option<&str>, output: Option<String>) -> EvOp {
    EvOp::EndTool {
        id,
        status: if ok {
            ToolStatus::Ok
        } else {
            ToolStatus::Error
        },
        result: result.map(str::to_owned),
        output,
    }
}

/// A finished edit tool call with its mini diff.
fn edit_done(id: u32, verb: &str, edit: &PlannedEdit) -> EvOp {
    let name = edit
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    EvOp::Push(AgentEvent::ToolCall {
        id,
        kind: if edit.kind == ChangeKind::Added {
            ToolKind::Write
        } else {
            ToolKind::Edit
        },
        title: format!("{verb} {name}"),
        args: edit.path.display().to_string(),
        status: ToolStatus::Ok,
        result: Some(format!("+{} -{}", edit.added, edit.removed)),
        output: None,
        edit: Some(EditSummary {
            path: edit.path.clone(),
            added: edit.added,
            removed: edit.removed,
            snippet: edit.snippet.clone(),
        }),
    })
}

fn say(text: String) -> EvOp {
    EvOp::Push(AgentEvent::AssistantText(text))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The first lines of a file, as a read tool's output.
fn head(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().take(8).collect();
    if text.lines().count() > 8 {
        lines.push("...");
    }
    lines.join("\n")
}

/// `text` cut to `max` characters on a word boundary.
fn short(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let cut: String = text.chars().take(max).collect();
    match cut.rfind(' ') {
        Some(at) if at > max / 2 => cut[..at].to_owned(),
        _ => cut,
    }
}

/// A branch-style name from the first two meaningful words of `prompt`.
fn slug(prompt: &str) -> String {
    const SKIP: [&str; 14] = [
        "the", "a", "an", "when", "to", "with", "for", "of", "behind", "one", "is", "in", "on",
        "and",
    ];
    let words: Vec<String> = prompt
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .filter(|w| !SKIP.contains(&w.as_str()))
        .take(2)
        .collect();
    if words.is_empty() {
        "task".into()
    } else {
        words.join("-")
    }
}

/// The longest word of the prompt, as a search keyword for the fake transcript.
fn keyword(prompt: &str) -> String {
    prompt
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 4)
        .max_by_key(|w| w.len())
        .unwrap_or("handler")
        .to_lowercase()
}

fn comment_marker(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "rs" | "java" | "kt" | "kts" | "js" | "ts" | "tsx" | "go" | "c" | "h" | "cpp" | "swift"
        | "gradle" | "scala" | "cs" => "//",
        "py" | "sh" | "toml" | "yaml" | "yml" | "properties" | "rb" => "#",
        _ => return None,
    })
}

fn indent_of(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

fn looks_like_code(line: &str, marker: Option<&str>) -> bool {
    let t = line.trim();
    if t.is_empty() || t == "{" || t == "}" || t.starts_with("import ") || t.starts_with("use ") {
        return false;
    }
    if t.starts_with("package ") || t.starts_with("mod ") || t.starts_with('@') {
        return false;
    }
    marker.is_none_or(|m| !t.starts_with(m))
}

fn pick_line(lines: &[String], from: usize, marker: Option<&str>) -> Option<usize> {
    (from..lines.len()).find(|&i| looks_like_code(&lines[i], marker))
}

/// A plausible small edit of `old`. Known shapes first (a guard clause in the sample
/// `greeting` method, one more assertion in its test, a dependency version bump), then a
/// constant tweak, and last a one-line comment for files without anything to change.
fn fake_edit(path: &Path, old: &str, note: &str, salt: usize) -> String {
    let marker = comment_marker(path);
    let mut lines: Vec<String> = old.split_inclusive('\n').map(str::to_owned).collect();
    if lines.is_empty() {
        return format!("{note}\n");
    }
    let who = ["World", "stranger", "friend", "there"][salt % 4];
    let find = |needle: &str, from: usize| (from..lines.len()).find(|&i| lines[i].contains(needle));
    // 1. A guard clause before the return of `greeting(String name)`.
    if let Some(declaration) = find("greeting(String name)", 0)
        && let Some(ret) = find("return", declaration)
    {
        let indent = indent_of(&lines[ret]).to_owned();
        let guard = format!(
            "{indent}if (name == null || name.isBlank()) {{\n{indent}    return \"Hello, {who}!\";\n{indent}}}\n"
        );
        lines.insert(ret, guard);
        return lines.concat();
    }
    // 2. One more assertion next to the existing one.
    if let Some(assertion) = find("assertEquals(", 0)
        && lines[assertion].contains("greeting(")
    {
        let indent = indent_of(&lines[assertion]).to_owned();
        let call = lines[assertion]
            .split("App.")
            .nth(1)
            .map_or("App.greeting", |_| "App.greeting");
        lines.insert(
            assertion + 1,
            format!("{indent}assertEquals(\"Hello, {who}!\", {call}(\" \"));\n"),
        );
        return lines.concat();
    }
    // 3. A version bump (build files).
    for index in 0..lines.len() {
        if let Some(bumped) = bump_version(&lines[index]) {
            lines[index] = bumped;
            return lines.concat();
        }
    }
    // 4. A tweaked constant in code.
    for index in 0..lines.len() {
        if looks_like_code(&lines[index], marker)
            && let Some(tweaked) = bump_number(&lines[index])
        {
            lines[index] = tweaked;
            let at = pick_line(&lines, 0, marker).unwrap_or(0).min(index);
            let indent = indent_of(&lines[at]).to_owned();
            let comment = marker.map_or(format!("{indent}{note}\n"), |m| {
                format!("{indent}{m} {note}\n")
            });
            lines.insert(at, comment);
            return lines.concat();
        }
    }
    // 5. Nothing to change: a note.
    let at = pick_line(&lines, 0, marker).unwrap_or(0);
    let indent = indent_of(&lines[at]).to_owned();
    lines.insert(
        at,
        marker.map_or(format!("{indent}{note}\n"), |m| {
            format!("{indent}{m} {note}\n")
        }),
    );
    lines.concat()
}

/// `1.2.3` to `1.2.4` in the first version-like number of `line`.
fn bump_version(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric()) {
            let end = line[i..]
                .find(|c: char| !(c.is_ascii_digit() || c == '.'))
                .map_or(line.len(), |e| i + e);
            let parts: Vec<&str> = line[i..end].split('.').collect();
            if parts.len() == 3
                && let Ok(patch) = parts[2].parse::<u32>()
            {
                return Some(format!(
                    "{}{}.{}.{}{}",
                    &line[..i],
                    parts[0],
                    parts[1],
                    patch + 1,
                    &line[end..]
                ));
            }
            i = end;
        } else {
            i += 1;
        }
    }
    None
}

/// The first standalone integer in `line`, plus one.
fn bump_number(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if b.is_ascii_digit()
            && (i == 0
                || !(bytes[i - 1].is_ascii_alphanumeric()
                    || bytes[i - 1] == b'_'
                    || bytes[i - 1] == b'.'))
        {
            let end = line[i..]
                .find(|c: char| !c.is_ascii_digit())
                .map_or(line.len(), |e| i + e);
            if line[end..].starts_with(|c: char| c.is_alphanumeric() || c == '.') {
                continue;
            }
            let value: u64 = line[i..end].parse().ok()?;
            return Some(format!("{}{}{}", &line[..i], value + 1, &line[end..]));
        }
    }
    None
}

fn collect_candidates(root: &Path) -> Vec<(PathBuf, String)> {
    fn walk(dir: &Path, root: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            if name.starts_with('.')
                || matches!(
                    name.as_str(),
                    "target" | "build" | "node_modules" | "out" | "dist" | "__pycache__"
                )
            {
                continue;
            }
            match entry.file_type() {
                Ok(kind) if kind.is_dir() && depth < 6 => walk(&path, root, depth + 1, out),
                Ok(kind) if kind.is_file() => {
                    if let Ok(relative) = path.strip_prefix(root) {
                        out.push(relative.to_path_buf());
                    }
                }
                _ => {}
            }
        }
    }
    let mut paths = Vec::new();
    walk(root, root, 0, &mut paths);
    let rank = |path: &PathBuf| -> u8 {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs" | "java" | "kt" | "ts" | "tsx" | "js" | "py" | "go" | "c" | "cpp") => 0,
            Some("kts" | "gradle" | "toml" | "yaml" | "yml") => 1,
            Some("md" | "txt" | "properties") => 2,
            _ => 3,
        }
    };
    paths.retain(|p| rank(p) < 3);
    // Source first; main sources before tests, shorter paths before deeper ones.
    paths.sort_by_key(|p| {
        let text = p.to_string_lossy().to_lowercase();
        (rank(p), text.contains("test"), p.components().count())
    });
    paths
        .into_iter()
        .filter_map(|relative| {
            let absolute = root.join(&relative);
            let meta = std::fs::metadata(&absolute).ok()?;
            if meta.len() == 0 || meta.len() > 64 * 1024 {
                return None;
            }
            let text = std::fs::read_to_string(&absolute).ok()?;
            Some((relative, text))
        })
        .take(24)
        .collect()
}

fn test_command(root: &Path) -> &'static str {
    let has = |name: &str| root.join(name).exists();
    if has("Cargo.toml") {
        "cargo test"
    } else if has("gradlew") || has("build.gradle") || has("build.gradle.kts") {
        "./gradlew test"
    } else if has("package.json") {
        "npm test"
    } else if has("go.mod") {
        "go test ./..."
    } else if has("pyproject.toml") || has("requirements.txt") {
        "pytest"
    } else {
        "make test"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_events::permission_input;

    const SECOND: Duration = Duration::from_secs(1);

    fn run(backend: &mut MockAgentsBackend, seconds: u64) -> Vec<AgentNews> {
        let mut events = Vec::new();
        for _ in 0..seconds * 10 {
            events.extend(backend.advance(Duration::from_millis(100)));
        }
        events
    }

    fn state_of(backend: &MockAgentsBackend, id: SessionId) -> AgentState {
        backend
            .sessions()
            .into_iter()
            .find(|s| s.id == id)
            .unwrap()
            .state
    }

    fn texts(backend: &MockAgentsBackend, id: SessionId) -> String {
        backend
            .events(id)
            .iter()
            .map(|event| format!("{event:?}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn claude_works_then_asks_then_finishes_after_the_answer() {
        let mut backend = MockAgentsBackend::default();
        let task = backend.launch(
            "Fix tab ordering when the active tab is closed",
            &[AgentProfile::Claude],
        );
        let id = backend.sessions()[0].id;
        assert_eq!(backend.tasks()[0].id, task);
        assert_eq!(state_of(&backend, id), AgentState::Working);
        assert!(backend.changes(id).is_empty());
        assert!(matches!(backend.events(id)[0], AgentEvent::UserPrompt(_)));

        let events = run(&mut backend, 4);
        assert_eq!(events, vec![AgentNews::NeedsInput(id)]);
        assert_eq!(state_of(&backend, id), AgentState::NeedsInput);
        let info = backend.sessions().remove(0);
        assert!(info.unread);
        assert!(info.question.unwrap().contains("(y/n)"));
        let open = backend
            .events(id)
            .iter()
            .filter(|e| e.is_open_question())
            .count();
        assert_eq!(open, 1, "the permission card waits for an answer");

        // Nothing moves, and the clock stops, while it waits.
        let before = backend.sessions()[0].elapsed;
        assert!(run(&mut backend, 5).is_empty());
        assert_eq!(backend.sessions()[0].elapsed, before);
        assert_eq!(state_of(&backend, id), AgentState::NeedsInput);

        backend.send(id, permission_input(0));
        assert_eq!(state_of(&backend, id), AgentState::Working);
        assert!(!backend.events(id).iter().any(|e| e.is_open_question()));
        let events = run(&mut backend, 10);
        assert_eq!(events, vec![AgentNews::Done(id)]);
        assert_eq!(state_of(&backend, id), AgentState::Done);
        assert!(backend.changes(id).len() >= 2);
        let log = backend.events(id);
        assert!(matches!(log.last(), Some(AgentEvent::TurnResult { files, .. }) if *files >= 2));
        assert!(texts(&backend, id).contains("Allow once"));
    }

    #[test]
    fn tool_calls_start_running_and_finish_with_output() {
        let mut backend = MockAgentsBackend::default();
        backend.launch("Add retries", &[AgentProfile::Gemini]);
        let id = backend.sessions()[0].id;
        run(&mut backend, 2);
        assert!(backend.events(id).iter().any(|e| matches!(
            e,
            AgentEvent::ToolCall {
                status: ToolStatus::Running,
                ..
            }
        )));
        run(&mut backend, 8);
        let calls: Vec<_> = backend
            .events(id)
            .iter()
            .filter_map(|e| match e {
                AgentEvent::ToolCall {
                    kind,
                    status,
                    edit,
                    output,
                    ..
                } => Some((*kind, *status, edit.is_some(), output.is_some())),
                _ => None,
            })
            .collect();
        assert!(calls.iter().all(|c| c.1 == ToolStatus::Ok), "{calls:?}");
        assert!(calls.iter().any(|c| c.0 == ToolKind::Edit && c.2));
        assert!(calls.iter().any(|c| c.0 == ToolKind::Bash && c.3));
    }

    #[test]
    fn declining_the_edit_ends_without_changes() {
        let mut backend = MockAgentsBackend::default();
        backend.launch("Rename the helper", &[AgentProfile::Claude]);
        let id = backend.sessions()[0].id;
        run(&mut backend, 4);
        backend.send(id, permission_input(2));
        run(&mut backend, 4);
        assert_eq!(state_of(&backend, id), AgentState::Done);
        assert!(backend.changes(id).is_empty());
        assert!(texts(&backend, id).contains("Deny"));
    }

    #[test]
    fn always_allow_is_recorded() {
        let mut backend = MockAgentsBackend::default();
        backend.launch("Rename the helper", &[AgentProfile::Claude]);
        let id = backend.sessions()[0].id;
        run(&mut backend, 4);
        backend.send(id, permission_input(1));
        assert!(texts(&backend, id).contains("Always allow"));
    }

    #[test]
    fn codex_spawns_a_child_and_waits_for_it() {
        let mut backend = MockAgentsBackend::default();
        backend.launch(
            "Add OAuth login with a device code flow",
            &[AgentProfile::Codex],
        );
        let parent = backend.sessions()[0].id;
        let events = run(&mut backend, 4);
        let child = backend
            .sessions()
            .into_iter()
            .find(|s| s.parent == Some(parent))
            .expect("a child agent was started")
            .id;
        assert!(events.contains(&AgentNews::Spawned { parent, child }));
        assert!(
            backend
                .events(parent)
                .iter()
                .any(|e| matches!(e, AgentEvent::SubagentSpawned { child: c, .. } if *c == child))
        );
        let sessions = backend.sessions();
        assert_eq!(sessions[0].id, parent);
        assert_eq!(sessions[1].id, child);
        assert_eq!(sessions[0].task, sessions[1].task);

        let mut done = Vec::new();
        for event in run(&mut backend, 12) {
            if let AgentNews::Done(id) = event {
                done.push(id);
            }
        }
        assert_eq!(
            done,
            vec![child, parent],
            "the parent finishes after its child"
        );
    }

    #[test]
    fn fan_out_starts_one_agent_per_profile_with_distinct_names() {
        let mut backend = MockAgentsBackend::default();
        backend.launch("Add retries", &AgentProfile::ALL);
        let sessions = backend.sessions();
        assert_eq!(sessions.len(), 4);
        let names: Vec<&str> = sessions.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "add-retries-claude",
                "add-retries-codex",
                "add-retries-gemini",
                "add-retries-opencode"
            ]
        );
        assert_eq!(sessions[0].branch, "agent/add-retries-claude");
        backend.launch("Add retries", &[AgentProfile::Gemini]);
        assert_eq!(backend.sessions().last().unwrap().name, "add-retries");
    }

    #[test]
    fn a_failing_prompt_ends_in_error() {
        let mut backend = MockAgentsBackend::default();
        backend.launch("Spike a Redis cache", &[AgentProfile::Claude]);
        let id = backend.sessions()[0].id;
        let events = run(&mut backend, 8);
        assert_eq!(events, vec![AgentNews::Failed(id)]);
        assert_eq!(state_of(&backend, id), AgentState::Error);
        assert!(matches!(
            backend.events(id).last(),
            Some(AgentEvent::Error(_))
        ));
    }

    #[test]
    fn stop_interrupts_and_a_message_resumes() {
        let mut backend = MockAgentsBackend::default();
        backend.launch("Add retries", &[AgentProfile::Gemini]);
        let id = backend.sessions()[0].id;
        run(&mut backend, 1);
        backend.stop(id);
        assert_eq!(state_of(&backend, id), AgentState::Stopped);
        assert!(run(&mut backend, 10).is_empty());
        backend.send(id, "continue please\r");
        assert_eq!(state_of(&backend, id), AgentState::Working);
        assert!(texts(&backend, id).contains("continue please"));
        assert_eq!(run(&mut backend, 6), vec![AgentNews::Done(id)]);
    }

    #[test]
    fn fake_diffs_use_real_project_files_and_merge_marks_the_outcome() {
        let root =
            std::env::temp_dir().join(format!("archipel-agents-mock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/App.java"),
            "package x;\n\npublic class App {\n    int a = 1;\n    int b = 2;\n    int c = 3;\n    int d = 4;\n    int e = 5;\n}\n",
        )
        .unwrap();
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let mut backend = MockAgentsBackend::new(Some(&root));
        backend.launch("Add retries", &[AgentProfile::Gemini]);
        let id = backend.sessions()[0].id;
        assert!(
            backend.merge(id, MergeStrategy::Squash).is_err(),
            "still running"
        );
        run(&mut backend, 8);
        let changes = backend.changes(id);
        assert_eq!(changes[0].path, Path::new("src/App.java"));
        assert!(changes[0].first_line > 1);
        let diff = backend.diff(id, Path::new("src/App.java")).unwrap();
        assert!(diff.old_text.contains("int a = 1;"));
        assert!(diff.new_text.contains("// Add retries"));
        // The inline mini diff of the transcript matches the real diff.
        let edit = backend
            .events(id)
            .iter()
            .find_map(|e| match e {
                AgentEvent::ToolCall {
                    edit: Some(edit), ..
                } if edit.path == changes[0].path => Some(edit.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(edit.added, changes[0].added);
        assert_eq!(edit.removed, changes[0].removed);
        assert_eq!(
            edit.snippet.iter().filter(|l| l.sign == '+').count(),
            changes[0].added
        );
        assert!(texts(&backend, id).contains("cargo test"));
        let message = backend.merge(id, MergeStrategy::Squash).unwrap();
        assert!(message.starts_with("Squash-merged"), "{message}");
        assert_eq!(backend.sessions()[0].outcome, Some(MergeStrategy::Squash));
        assert!(backend.merge(id, MergeStrategy::Discard).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn edits_are_plausible_code_changes() {
        let app = "public class App {\n    public static String greeting(String name) {\n        return \"Hello, \" + name + \"!\";\n    }\n}\n";
        let new = fake_edit(Path::new("App.java"), app, "x", 0);
        assert!(
            new.contains("if (name == null || name.isBlank()) {"),
            "{new}"
        );
        assert!(new.contains("return \"Hello, World!\";"));
        let test = "    @Test\n    void greets() {\n        assertEquals(\"Hello, Java!\", App.greeting(\"Java\"));\n    }\n";
        let new = fake_edit(Path::new("AppTest.java"), test, "x", 1);
        assert!(
            new.contains("assertEquals(\"Hello, stranger!\", App.greeting(\" \"));"),
            "{new}"
        );
        let gradle = "testImplementation(\"org.junit.jupiter:junit-jupiter:5.11.0\")\n";
        assert!(fake_edit(Path::new("build.gradle.kts"), gradle, "x", 0).contains("5.11.1"));
        assert_eq!(bump_number("let a = 41;").as_deref(), Some("let a = 42;"));
        assert_eq!(bump_number("x2 = y"), None);
    }

    #[test]
    fn names_come_from_the_prompt() {
        assert_eq!(
            slug("Fix tab ordering when the active tab is closed"),
            "fix-tab"
        );
        assert_eq!(slug("Add OAuth login"), "add-oauth");
        assert_eq!(slug("   "), "task");
        assert_eq!(keyword("Fix the ordering"), "ordering");
        assert_eq!(short("one two three four", 9), "one two");
        let _ = SECOND;
    }
}

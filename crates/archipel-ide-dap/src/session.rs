//! Debug session state machine.
//!
//! [`Session`] owns no I/O: it consumes adapter messages with [`Session::handle`] and
//! produces outgoing requests through an internal queue that the caller drains with
//! [`Session::take_outgoing`]. This keeps the protocol logic deterministic and testable
//! with scripted adapter messages.

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// `initialize` sent, waiting for capabilities.
    Initializing,
    /// `launch` sent; breakpoints are configured once `initialized` arrives.
    Launching,
    Running,
    Stopped {
        thread_id: i64,
        reason: String,
        description: Option<String>,
    },
    Terminated {
        exit_code: Option<i64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackFrame {
    pub id: i64,
    pub name: String,
    pub path: Option<PathBuf>,
    /// One-based line as reported by the adapter.
    pub line: i64,
    pub column: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub type_name: Option<String>,
    /// Non-zero when the variable has children.
    pub reference: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub name: String,
    pub reference: i64,
    pub expensive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadInfo {
    pub id: i64,
    pub name: String,
}

/// A breakpoint as the adapter reports it (verified or not).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakpointStatus {
    pub path: PathBuf,
    /// Zero-based line requested by the user.
    pub line: usize,
    pub verified: bool,
    pub message: Option<String>,
}

/// Per-breakpoint options from the breakpoint properties popup.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BreakpointOptions {
    /// Only suspend when this expression is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Logpoint: print this message (with `{expr}` interpolation) instead of suspending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_message: Option<String>,
    /// Only suspend once the hit count satisfies this (`3`, `>= 2`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit_condition: Option<String>,
}

impl BreakpointOptions {
    pub fn is_empty(&self) -> bool {
        self.condition.is_none() && self.log_message.is_none() && self.hit_condition.is_none()
    }

    /// Trims the fields and turns blank ones into `None`.
    pub fn normalized(self) -> Self {
        let clean = |value: Option<String>| {
            value
                .map(|text| text.trim().to_owned())
                .filter(|text| !text.is_empty())
        };
        Self {
            condition: clean(self.condition),
            log_message: clean(self.log_message),
            hit_condition: clean(self.hit_condition),
        }
    }
}

/// A watch expression and its latest value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Watch {
    pub expression: String,
    /// `None` until evaluated at a stop.
    pub result: Option<Result<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputCategory {
    Console,
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputLine {
    pub category: OutputCategory,
    pub text: String,
}

/// What to launch: the adapter-specific `launch` arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchRequest {
    pub adapter_id: String,
    /// Send `attach` instead of `launch` (the debuggee already runs and listens).
    pub attach: bool,
    pub arguments: Value,
}

/// Commands that change the run state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Continue,
    Next,
    StepIn,
    StepOut,
    Pause,
    /// Step into the first call of the line even when the adapter would skip it.
    ForceStepIn,
}

pub struct Session {
    seq: i64,
    state: SessionState,
    launch: LaunchRequest,
    /// Requested breakpoints: path → zero-based lines.
    breakpoints: BTreeMap<PathBuf, BTreeSet<usize>>,
    statuses: Vec<BreakpointStatus>,
    pending: HashMap<i64, Pending>,
    outgoing: Vec<Value>,
    capabilities: Value,
    threads: Vec<ThreadInfo>,
    frames: Vec<StackFrame>,
    selected_frame: Option<i64>,
    scopes: Vec<Scope>,
    variables: HashMap<i64, Vec<Variable>>,
    output: Vec<OutputLine>,
    evaluations: Vec<(String, Result<String, String>)>,
    error: Option<String>,
    options: BTreeMap<(PathBuf, usize), BreakpointOptions>,
    watches: Vec<Watch>,
    /// Temporary "run to cursor" breakpoint, removed at the next stop.
    run_to: Option<(PathBuf, usize)>,
}

#[derive(Debug, Clone)]
enum Pending {
    Initialize,
    Launch,
    SetBreakpoints { path: PathBuf, lines: Vec<usize> },
    ConfigurationDone,
    Threads,
    StackTrace { thread_id: i64 },
    Scopes { frame_id: i64 },
    Variables { reference: i64 },
    Evaluate { expression: String },
    EvaluateWatch { expression: String, frame_id: i64 },
    StepInTargets { thread_id: i64 },
    Step,
    Disconnect,
}

/// Maximum console lines kept per session.
const MAX_OUTPUT: usize = 20_000;

impl Session {
    /// Starts a session; the `initialize` request is queued immediately.
    pub fn new(launch: LaunchRequest, breakpoints: BTreeMap<PathBuf, BTreeSet<usize>>) -> Self {
        let mut session = Self {
            seq: 0,
            state: SessionState::Initializing,
            launch,
            breakpoints,
            statuses: Vec::new(),
            pending: HashMap::new(),
            outgoing: Vec::new(),
            capabilities: Value::Null,
            threads: Vec::new(),
            frames: Vec::new(),
            selected_frame: None,
            scopes: Vec::new(),
            variables: HashMap::new(),
            output: Vec::new(),
            evaluations: Vec::new(),
            error: None,
            options: BTreeMap::new(),
            watches: Vec::new(),
            run_to: None,
        };
        let adapter_id = session.launch.adapter_id.clone();
        session.request(
            "initialize",
            json!({
                "clientID": "archipel-ide",
                "clientName": "Archipel IDE",
                "adapterID": adapter_id,
                "pathFormat": "path",
                "linesStartAt1": true,
                "columnsStartAt1": true,
                "supportsVariableType": true,
                "supportsRunInTerminalRequest": false,
            }),
            Pending::Initialize,
        );
        session
    }

    fn request(&mut self, command: &str, arguments: Value, pending: Pending) -> i64 {
        self.seq += 1;
        let seq = self.seq;
        self.outgoing.push(json!({
            "seq": seq,
            "type": "request",
            "command": command,
            "arguments": arguments,
        }));
        self.pending.insert(seq, pending);
        seq
    }

    /// Requests queued for the adapter since the last call.
    pub fn take_outgoing(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.outgoing)
    }

    /// Processes one message from the adapter.
    pub fn handle(&mut self, message: &Value) {
        match message["type"].as_str() {
            Some("response") => self.on_response(message),
            Some("event") => self.on_event(message),
            Some("request") => {
                // Reverse requests (runInTerminal, startDebugging) are not supported;
                // answer so the adapter does not wait forever.
                self.seq += 1;
                self.outgoing.push(json!({
                    "seq": self.seq,
                    "type": "response",
                    "request_seq": message["seq"],
                    "command": message["command"],
                    "success": false,
                    "message": "not supported by Archipel IDE",
                }));
            }
            _ => {}
        }
    }

    fn on_response(&mut self, message: &Value) {
        let Some(pending) = message["request_seq"]
            .as_i64()
            .and_then(|seq| self.pending.remove(&seq))
        else {
            return;
        };
        let body = &message["body"];
        if message["success"].as_bool() != Some(true) {
            let reason = message["message"]
                .as_str()
                .or_else(|| body["error"]["format"].as_str())
                .unwrap_or("request failed")
                .to_owned();
            match pending {
                Pending::Evaluate { expression } => {
                    self.evaluations.push((expression, Err(reason)))
                }
                Pending::EvaluateWatch {
                    expression,
                    frame_id,
                } => self.set_watch_result(&expression, frame_id, Err(reason)),
                Pending::StepInTargets { thread_id } => self.plain_step_in(thread_id),
                Pending::Launch | Pending::Initialize => {
                    self.error = Some(format!("debugger failed to start: {reason}"));
                    self.state = SessionState::Terminated { exit_code: None };
                }
                Pending::SetBreakpoints { path, lines } => {
                    for line in lines {
                        self.set_status(&path, line, false, Some(reason.clone()));
                    }
                }
                _ => self.push_output(OutputCategory::Console, format!("{reason}\n")),
            }
            return;
        }
        match pending {
            Pending::Initialize => {
                self.capabilities = body.clone();
                let arguments = self.launch.arguments.clone();
                self.state = SessionState::Launching;
                let command = if self.launch.attach {
                    "attach"
                } else {
                    "launch"
                };
                self.request(command, arguments, Pending::Launch);
            }
            Pending::Launch => {
                if self.state == SessionState::Launching {
                    self.state = SessionState::Running;
                }
            }
            Pending::SetBreakpoints { path, lines } => {
                let reported = body["breakpoints"].as_array().cloned().unwrap_or_default();
                for (index, line) in lines.into_iter().enumerate() {
                    let entry = reported.get(index);
                    let verified = entry
                        .and_then(|bp| bp["verified"].as_bool())
                        .unwrap_or(false);
                    let message = entry
                        .and_then(|bp| bp["message"].as_str())
                        .map(str::to_owned);
                    self.set_status(&path, line, verified, message);
                }
            }
            Pending::ConfigurationDone | Pending::Step | Pending::Disconnect => {}
            Pending::Threads => {
                self.threads = body["threads"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|thread| {
                        Some(ThreadInfo {
                            id: thread["id"].as_i64()?,
                            name: thread["name"].as_str().unwrap_or("thread").to_owned(),
                        })
                    })
                    .collect();
            }
            Pending::StackTrace { thread_id } => {
                if !matches!(self.state, SessionState::Stopped { thread_id: stopped, .. } if stopped == thread_id)
                {
                    return;
                }
                self.frames = body["stackFrames"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|frame| {
                        Some(StackFrame {
                            id: frame["id"].as_i64()?,
                            name: frame["name"].as_str().unwrap_or("?").to_owned(),
                            path: frame["source"]["path"].as_str().map(PathBuf::from),
                            line: frame["line"].as_i64().unwrap_or(0),
                            column: frame["column"].as_i64().unwrap_or(0),
                        })
                    })
                    .collect();
                // Select the topmost frame that has source, like the reference debugger.
                let first = self
                    .frames
                    .iter()
                    .find(|frame| frame.path.is_some())
                    .or(self.frames.first())
                    .map(|frame| frame.id);
                if let Some(frame_id) = first {
                    self.select_frame(frame_id);
                }
            }
            Pending::Scopes { frame_id } => {
                if self.selected_frame != Some(frame_id) {
                    return;
                }
                self.scopes = body["scopes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|scope| {
                        Some(Scope {
                            name: scope["name"].as_str().unwrap_or("Scope").to_owned(),
                            reference: scope["variablesReference"].as_i64()?,
                            expensive: scope["expensive"].as_bool().unwrap_or(false),
                        })
                    })
                    .collect();
                let cheap: Vec<i64> = self
                    .scopes
                    .iter()
                    .filter(|scope| !scope.expensive && scope.reference > 0)
                    .map(|scope| scope.reference)
                    .collect();
                for reference in cheap {
                    self.request_variables(reference);
                }
            }
            Pending::Variables { reference } => {
                let variables = body["variables"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|variable| Variable {
                        name: variable["name"].as_str().unwrap_or("?").to_owned(),
                        value: variable["value"].as_str().unwrap_or_default().to_owned(),
                        type_name: variable["type"].as_str().map(str::to_owned),
                        reference: variable["variablesReference"].as_i64().unwrap_or(0),
                    })
                    .collect();
                self.variables.insert(reference, variables);
            }
            Pending::Evaluate { expression } => {
                let value = body["result"].as_str().unwrap_or_default().to_owned();
                self.evaluations.push((expression, Ok(value)));
            }
            Pending::EvaluateWatch {
                expression,
                frame_id,
            } => {
                let value = body["result"].as_str().unwrap_or_default().to_owned();
                self.set_watch_result(&expression, frame_id, Ok(value));
            }
            Pending::StepInTargets { thread_id } => {
                if !matches!(self.state, SessionState::Stopped { thread_id: t, .. } if t == thread_id)
                {
                    return;
                }
                let target = body["targets"]
                    .as_array()
                    .and_then(|targets| targets.first())
                    .and_then(|target| target["id"].as_i64());
                match target {
                    Some(id) => {
                        self.request(
                            "stepIn",
                            json!({"threadId": thread_id, "targetId": id}),
                            Pending::Step,
                        );
                        self.mark_running();
                    }
                    None => self.plain_step_in(thread_id),
                }
            }
        }
    }

    fn on_event(&mut self, message: &Value) {
        let body = &message["body"];
        match message["event"].as_str() {
            Some("initialized") => {
                let files: Vec<PathBuf> = self.breakpoints.keys().cloned().collect();
                for path in files {
                    self.send_breakpoints(&path);
                }
                if self.capabilities["supportsConfigurationDoneRequest"].as_bool() != Some(false) {
                    self.request("configurationDone", json!({}), Pending::ConfigurationDone);
                }
            }
            Some("stopped") => {
                let thread_id = body["threadId"]
                    .as_i64()
                    .or_else(|| self.threads.first().map(|thread| thread.id))
                    .unwrap_or(1);
                self.state = SessionState::Stopped {
                    thread_id,
                    reason: body["reason"].as_str().unwrap_or("pause").to_owned(),
                    description: body["description"]
                        .as_str()
                        .or_else(|| body["text"].as_str())
                        .map(str::to_owned),
                };
                self.frames.clear();
                self.scopes.clear();
                self.variables.clear();
                self.selected_frame = None;
                if let Some((path, _)) = self.run_to.take() {
                    self.send_breakpoints(&path);
                }
                self.request("threads", json!({}), Pending::Threads);
                self.request(
                    "stackTrace",
                    json!({"threadId": thread_id, "startFrame": 0, "levels": 100}),
                    Pending::StackTrace { thread_id },
                );
            }
            Some("continued") => {
                if !matches!(self.state, SessionState::Terminated { .. }) {
                    self.state = SessionState::Running;
                    self.frames.clear();
                    self.scopes.clear();
                    self.variables.clear();
                }
            }
            Some("output") => {
                let category = match body["category"].as_str() {
                    Some("stdout") => OutputCategory::Stdout,
                    Some("stderr") => OutputCategory::Stderr,
                    Some("telemetry") => return,
                    _ => OutputCategory::Console,
                };
                let text = body["output"].as_str().unwrap_or_default().to_owned();
                self.push_output(category, text);
            }
            Some("exited") => {
                let code = body["exitCode"].as_i64();
                self.push_output(
                    OutputCategory::Console,
                    format!(
                        "\nProcess finished with exit code {}\n",
                        code.map_or("?".into(), |c| c.to_string())
                    ),
                );
                self.state = SessionState::Terminated { exit_code: code };
            }
            Some("terminated") => {
                if !matches!(self.state, SessionState::Terminated { .. }) {
                    self.state = SessionState::Terminated { exit_code: None };
                }
            }
            Some("breakpoint") => {
                // Verification updates for already requested breakpoints.
                let breakpoint = &body["breakpoint"];
                if let (Some(path), Some(line)) = (
                    breakpoint["source"]["path"].as_str(),
                    breakpoint["line"].as_i64(),
                ) {
                    let verified = breakpoint["verified"].as_bool().unwrap_or(false);
                    let line = usize::try_from(line - 1).unwrap_or(0);
                    if let Some(status) = self.statuses.iter_mut().find(|status| {
                        status.path == std::path::Path::new(path) && status.line == line
                    }) {
                        status.verified = verified;
                    }
                }
            }
            _ => {}
        }
    }

    fn push_output(&mut self, category: OutputCategory, text: String) {
        self.output.push(OutputLine { category, text });
        if self.output.len() > MAX_OUTPUT {
            let excess = self.output.len() - MAX_OUTPUT;
            self.output.drain(..excess);
        }
    }

    fn set_status(&mut self, path: &PathBuf, line: usize, verified: bool, message: Option<String>) {
        let user_owned = self
            .breakpoints
            .get(path)
            .is_some_and(|lines| lines.contains(&line));
        if !user_owned {
            return;
        }
        self.statuses
            .retain(|status| !(status.path == *path && status.line == line));
        self.statuses.push(BreakpointStatus {
            path: path.clone(),
            line,
            verified,
            message,
        });
    }

    fn send_breakpoints(&mut self, path: &PathBuf) {
        let lines: Vec<usize> = self
            .breakpoints
            .get(path)
            .map(|lines| lines.iter().copied().collect())
            .unwrap_or_default();
        self.statuses.retain(|status| status.path != *path);
        let mut breakpoints: Vec<Value> = lines
            .iter()
            .map(|line| {
                let mut breakpoint = json!({"line": line + 1});
                if let Some(options) = self.options.get(&(path.clone(), *line)) {
                    if let Some(condition) = &options.condition {
                        breakpoint["condition"] = json!(condition);
                    }
                    if let Some(message) = &options.log_message {
                        breakpoint["logMessage"] = json!(message);
                    }
                    if let Some(hits) = &options.hit_condition {
                        breakpoint["hitCondition"] = json!(hits);
                    }
                }
                breakpoint
            })
            .collect();
        let mut sent_lines: Vec<usize> = lines.clone();
        // The run-to-cursor target is sent last so statuses of user breakpoints keep
        // their index; it is not reported as a user breakpoint.
        let temporary = self
            .run_to
            .as_ref()
            .filter(|(target, line)| target == path && !lines.contains(line))
            .map(|(_, line)| *line);
        if let Some(line) = temporary {
            breakpoints.push(json!({"line": line + 1}));
            sent_lines.push(line);
        }
        self.request(
            "setBreakpoints",
            json!({
                "source": {"path": path, "name": path.file_name().map(|n| n.to_string_lossy().into_owned())},
                "breakpoints": breakpoints,
                "lines": sent_lines.iter().map(|line| line + 1).collect::<Vec<_>>(),
            }),
            Pending::SetBreakpoints {
                path: path.clone(),
                lines: sent_lines,
            },
        );
    }

    /// Replaces the breakpoints of `path` (sent immediately when the adapter is configured).
    pub fn update_breakpoints(&mut self, path: PathBuf, lines: BTreeSet<usize>) {
        self.options
            .retain(|(file, line), _| *file != path || lines.contains(line));
        if lines.is_empty() {
            self.breakpoints.remove(&path);
        } else {
            self.breakpoints.insert(path.clone(), lines);
        }
        if matches!(
            self.state,
            SessionState::Running | SessionState::Stopped { .. }
        ) {
            self.send_breakpoints(&path);
        }
    }

    /// Loads the stored breakpoint options (used when a session starts).
    pub fn load_options(&mut self, options: BTreeMap<(PathBuf, usize), BreakpointOptions>) {
        self.options = options;
    }

    /// Sets the condition / logpoint / hit-count options of one breakpoint line and
    /// resends the file's breakpoints. Empty options clear them.
    pub fn set_breakpoint_options(
        &mut self,
        path: PathBuf,
        line: usize,
        options: BreakpointOptions,
    ) {
        let options = options.normalized();
        if options.is_empty() {
            self.options.remove(&(path.clone(), line));
        } else {
            self.options.insert((path.clone(), line), options);
        }
        if matches!(
            self.state,
            SessionState::Running | SessionState::Stopped { .. }
        ) {
            self.send_breakpoints(&path);
        }
    }

    pub fn breakpoint_options(
        &self,
        path: &std::path::Path,
        line: usize,
    ) -> Option<&BreakpointOptions> {
        self.options.get(&(path.to_owned(), line))
    }

    /// Resumes until `line` (zero-based) of `path` is reached, or another breakpoint stops
    /// first. DAP has no such request, so a temporary breakpoint is set and removed at
    /// the next stop.
    pub fn run_to_line(&mut self, path: PathBuf, line: usize) {
        let SessionState::Stopped { .. } = self.state else {
            return;
        };
        let previous = self.run_to.replace((path.clone(), line));
        if let Some((old, _)) = previous
            && old != path
        {
            self.send_breakpoints(&old);
        }
        self.send_breakpoints(&path);
        self.step(Step::Continue);
    }

    pub fn step(&mut self, step: Step) {
        if step == Step::ForceStepIn {
            let SessionState::Stopped { thread_id, .. } = self.state else {
                return;
            };
            match self.selected_frame {
                Some(frame_id) => {
                    self.request(
                        "stepInTargets",
                        json!({"frameId": frame_id}),
                        Pending::StepInTargets { thread_id },
                    );
                }
                None => self.plain_step_in(thread_id),
            }
            return;
        }
        let thread_id = match &self.state {
            SessionState::Stopped { thread_id, .. } => *thread_id,
            SessionState::Running if step == Step::Pause => {
                self.threads.first().map(|thread| thread.id).unwrap_or(1)
            }
            _ => return,
        };
        let command = match step {
            Step::Continue => "continue",
            Step::Next => "next",
            Step::StepIn => "stepIn",
            Step::StepOut => "stepOut",
            Step::Pause => "pause",
            Step::ForceStepIn => unreachable!("handled above"),
        };
        self.request(command, json!({"threadId": thread_id}), Pending::Step);
        if step != Step::Pause {
            // Optimistically running; a `stopped` event follows for steps.
            self.mark_running();
        }
    }

    fn mark_running(&mut self) {
        self.state = SessionState::Running;
        self.frames.clear();
        self.scopes.clear();
        self.variables.clear();
    }

    fn plain_step_in(&mut self, thread_id: i64) {
        if matches!(self.state, SessionState::Stopped { thread_id: t, .. } if t == thread_id) {
            self.request("stepIn", json!({"threadId": thread_id}), Pending::Step);
            self.mark_running();
        }
    }

    pub fn watches(&self) -> &[Watch] {
        &self.watches
    }

    /// Adds a watch (duplicates ignored) and evaluates it when stopped.
    pub fn add_watch(&mut self, expression: &str) {
        let expression = expression.trim();
        if expression.is_empty() || self.watches.iter().any(|w| w.expression == expression) {
            return;
        }
        self.watches.push(Watch {
            expression: expression.to_owned(),
            result: None,
        });
        self.evaluate_watches();
    }

    pub fn remove_watch(&mut self, index: usize) {
        if index < self.watches.len() {
            self.watches.remove(index);
        }
    }

    /// Replaces the expression of a watch; an empty expression removes it.
    pub fn edit_watch(&mut self, index: usize, expression: &str) {
        let expression = expression.trim();
        if index >= self.watches.len() {
            return;
        }
        if expression.is_empty() {
            self.remove_watch(index);
            return;
        }
        if self
            .watches
            .iter()
            .enumerate()
            .any(|(i, w)| i != index && w.expression == expression)
        {
            return;
        }
        self.watches[index] = Watch {
            expression: expression.to_owned(),
            result: None,
        };
        self.evaluate_watches();
    }

    /// Re-evaluates the watches that have no value in the selected frame.
    fn evaluate_watches(&mut self) {
        let (SessionState::Stopped { .. }, Some(frame_id)) = (&self.state, self.selected_frame)
        else {
            return;
        };
        let pending: Vec<String> = self
            .watches
            .iter()
            .filter(|watch| watch.result.is_none())
            .map(|watch| watch.expression.clone())
            .collect();
        for expression in pending {
            self.request(
                "evaluate",
                json!({"expression": expression, "context": "watch", "frameId": frame_id}),
                Pending::EvaluateWatch {
                    expression,
                    frame_id,
                },
            );
        }
    }

    fn set_watch_result(
        &mut self,
        expression: &str,
        frame_id: i64,
        result: Result<String, String>,
    ) {
        if self.selected_frame != Some(frame_id) {
            return;
        }
        for watch in self
            .watches
            .iter_mut()
            .filter(|w| w.expression == expression)
        {
            watch.result = Some(result.clone());
        }
    }

    pub fn select_frame(&mut self, frame_id: i64) {
        if !self.frames.iter().any(|frame| frame.id == frame_id) {
            return;
        }
        self.selected_frame = Some(frame_id);
        self.scopes.clear();
        self.variables.clear();
        for watch in &mut self.watches {
            watch.result = None;
        }
        self.request(
            "scopes",
            json!({"frameId": frame_id}),
            Pending::Scopes { frame_id },
        );
        self.evaluate_watches();
    }

    /// Loads the children of a structured variable or scope.
    pub fn request_variables(&mut self, reference: i64) {
        if reference <= 0 || self.variables.contains_key(&reference) {
            return;
        }
        self.request(
            "variables",
            json!({"variablesReference": reference}),
            Pending::Variables { reference },
        );
    }

    pub fn evaluate(&mut self, expression: &str) {
        let expression = expression.trim().to_owned();
        if expression.is_empty() {
            return;
        }
        let mut arguments = json!({"expression": expression, "context": "repl"});
        if let Some(frame) = self.selected_frame {
            arguments["frameId"] = json!(frame);
        }
        self.request("evaluate", arguments, Pending::Evaluate { expression });
    }

    /// Asks the adapter to end the debuggee.
    pub fn disconnect(&mut self) {
        if matches!(self.state, SessionState::Terminated { .. }) {
            return;
        }
        self.request(
            "disconnect",
            json!({"terminateDebuggee": true}),
            Pending::Disconnect,
        );
    }

    /// Records that the adapter process ended without a `terminated` event.
    pub fn adapter_exited(&mut self, detail: &str) {
        if !matches!(self.state, SessionState::Terminated { .. }) {
            if !detail.trim().is_empty() {
                self.push_output(OutputCategory::Stderr, detail.to_owned());
            }
            self.state = SessionState::Terminated { exit_code: None };
        }
    }

    pub fn state(&self) -> &SessionState {
        &self.state
    }
    pub fn frames(&self) -> &[StackFrame] {
        &self.frames
    }
    pub fn selected_frame(&self) -> Option<&StackFrame> {
        self.selected_frame
            .and_then(|id| self.frames.iter().find(|frame| frame.id == id))
    }
    pub fn threads(&self) -> &[ThreadInfo] {
        &self.threads
    }
    pub fn scopes(&self) -> &[Scope] {
        &self.scopes
    }
    pub fn variables(&self, reference: i64) -> Option<&[Variable]> {
        self.variables.get(&reference).map(Vec::as_slice)
    }
    pub fn output(&self) -> &[OutputLine] {
        &self.output
    }
    pub fn breakpoint_statuses(&self) -> &[BreakpointStatus] {
        &self.statuses
    }
    pub fn evaluations(&self) -> &[(String, Result<String, String>)] {
        &self.evaluations
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub fn is_terminated(&self) -> bool {
        matches!(self.state, SessionState::Terminated { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(request: &Value, body: Value) -> Value {
        json!({
            "type": "response",
            "request_seq": request["seq"],
            "command": request["command"],
            "success": true,
            "body": body,
        })
    }

    fn take(session: &mut Session, command: &str) -> Value {
        let outgoing = session.take_outgoing();
        outgoing
            .into_iter()
            .find(|message| message["command"] == command)
            .unwrap_or_else(|| panic!("expected `{command}` request"))
    }

    fn started() -> Session {
        let mut breakpoints = BTreeMap::new();
        breakpoints.insert(PathBuf::from("/w/src/main.rs"), BTreeSet::from([4, 9]));
        let mut session = Session::new(
            LaunchRequest {
                adapter_id: "lldb-dap".into(),
                attach: false,
                arguments: json!({"program": "/w/target/debug/app"}),
            },
            breakpoints,
        );
        let initialize = take(&mut session, "initialize");
        session.handle(&response(
            &initialize,
            json!({"supportsConfigurationDoneRequest": true}),
        ));
        let launch = take(&mut session, "launch");
        assert_eq!(launch["arguments"]["program"], "/w/target/debug/app");
        session.handle(&json!({"type": "event", "event": "initialized"}));
        let outgoing = session.take_outgoing();
        let set = outgoing
            .iter()
            .find(|m| m["command"] == "setBreakpoints")
            .unwrap();
        assert_eq!(
            set["arguments"]["breakpoints"],
            json!([{"line": 5}, {"line": 10}])
        );
        assert!(outgoing.iter().any(|m| m["command"] == "configurationDone"));
        session.handle(&response(
            set,
            json!({"breakpoints": [{"verified": true, "line": 5}, {"verified": false, "message": "no code"}]}),
        ));
        session.handle(&response(&launch, json!({})));
        session
    }

    #[test]
    fn startup_sequence_configures_breakpoints() {
        let session = started();
        assert_eq!(session.state(), &SessionState::Running);
        let statuses = session.breakpoint_statuses();
        assert_eq!(statuses.len(), 2);
        assert!(statuses[0].verified);
        assert_eq!(statuses[1].message.as_deref(), Some("no code"));
    }

    #[test]
    fn stop_loads_frames_scopes_and_variables() {
        let mut session = started();
        session.handle(&json!({"type": "event", "event": "stopped", "body": {"reason": "breakpoint", "threadId": 7}}));
        let outgoing = session.take_outgoing();
        let threads = outgoing
            .iter()
            .find(|m| m["command"] == "threads")
            .unwrap()
            .clone();
        let trace = outgoing
            .iter()
            .find(|m| m["command"] == "stackTrace")
            .unwrap()
            .clone();
        session.handle(&response(
            &threads,
            json!({"threads": [{"id": 7, "name": "main"}]}),
        ));
        session.handle(&response(
            &trace,
            json!({"stackFrames": [
                {"id": 100, "name": "std::rt::lang_start", "line": 0, "column": 0},
                {"id": 101, "name": "app::main", "line": 5, "column": 9, "source": {"path": "/w/src/main.rs"}}
            ]}),
        ));
        assert_eq!(session.selected_frame().unwrap().name, "app::main");
        let scopes = take(&mut session, "scopes");
        assert_eq!(scopes["arguments"]["frameId"], 101);
        session.handle(&response(
            &scopes,
            json!({"scopes": [{"name": "Locals", "variablesReference": 1000}, {"name": "Registers", "variablesReference": 2000, "expensive": true}]}),
        ));
        let variables = take(&mut session, "variables");
        assert_eq!(variables["arguments"]["variablesReference"], 1000);
        session.handle(&response(
            &variables,
            json!({"variables": [{"name": "x", "value": "42", "type": "i32", "variablesReference": 0}, {"name": "v", "value": "size=2", "variablesReference": 1001}]}),
        ));
        assert_eq!(session.variables(1000).unwrap()[0].value, "42");
        session.request_variables(1001);
        assert_eq!(
            take(&mut session, "variables")["arguments"]["variablesReference"],
            1001
        );
        assert!(matches!(
            session.state(),
            SessionState::Stopped { thread_id: 7, .. }
        ));
        session.step(Step::Next);
        let next = take(&mut session, "next");
        assert_eq!(next["arguments"]["threadId"], 7);
        assert_eq!(session.state(), &SessionState::Running);
        assert!(session.frames().is_empty());
    }

    #[test]
    fn output_exit_and_failures() {
        let mut session = started();
        session.handle(&json!({"type": "event", "event": "output", "body": {"category": "stdout", "output": "Hello\n"}}));
        session.handle(&json!({"type": "event", "event": "output", "body": {"category": "telemetry", "output": "x"}}));
        session.handle(&json!({"type": "event", "event": "exited", "body": {"exitCode": 3}}));
        assert_eq!(session.output()[0].text, "Hello\n");
        assert_eq!(session.output().len(), 2);
        assert_eq!(
            session.state(),
            &SessionState::Terminated { exit_code: Some(3) }
        );

        let mut failing = Session::new(
            LaunchRequest {
                adapter_id: "gdb".into(),
                attach: false,
                arguments: json!({}),
            },
            BTreeMap::new(),
        );
        let initialize = take(&mut failing, "initialize");
        failing.handle(&json!({"type": "response", "request_seq": initialize["seq"], "success": false, "message": "unsupported"}));
        assert!(failing.is_terminated());
        assert!(failing.error().unwrap().contains("unsupported"));
        // Reverse requests are answered, never left hanging.
        failing.handle(&json!({"type": "request", "seq": 9, "command": "runInTerminal"}));
        let answer = failing.take_outgoing();
        assert_eq!(answer[0]["request_seq"], 9);
        assert_eq!(answer[0]["success"], false);
    }

    #[test]
    fn breakpoint_changes_during_a_session_are_sent() {
        let mut session = started();
        session.update_breakpoints(PathBuf::from("/w/src/lib.rs"), BTreeSet::from([0]));
        let set = take(&mut session, "setBreakpoints");
        assert_eq!(set["arguments"]["source"]["path"], "/w/src/lib.rs");
        session.update_breakpoints(PathBuf::from("/w/src/lib.rs"), BTreeSet::new());
        let cleared = take(&mut session, "setBreakpoints");
        assert_eq!(cleared["arguments"]["breakpoints"], json!([]));
    }

    #[test]
    fn evaluate_uses_selected_frame() {
        let mut session = started();
        session.evaluate("  1 + 1 ");
        let evaluate = take(&mut session, "evaluate");
        assert_eq!(evaluate["arguments"]["expression"], "1 + 1");
        session.handle(&response(&evaluate, json!({"result": "2"})));
        assert_eq!(
            session.evaluations()[0],
            ("1 + 1".to_owned(), Ok("2".to_owned()))
        );
    }

    /// Stops at /w/src/main.rs:5 with frame 101 selected; returns the session.
    fn stopped_at_main() -> Session {
        let mut session = started();
        session.handle(&json!({"type": "event", "event": "stopped", "body": {"reason": "breakpoint", "threadId": 7}}));
        let outgoing = session.take_outgoing();
        let trace = outgoing
            .iter()
            .find(|m| m["command"] == "stackTrace")
            .unwrap()
            .clone();
        session.handle(&response(
            &trace,
            json!({"stackFrames": [{"id": 101, "name": "main", "line": 5, "column": 1, "source": {"path": "/w/src/main.rs"}}]}),
        ));
        session
    }

    #[test]
    fn attach_sessions_send_attach_instead_of_launch() {
        let mut session = Session::new(
            LaunchRequest {
                adapter_id: "java-debug".into(),
                attach: true,
                arguments: json!({"hostName": "localhost", "port": 5005}),
            },
            BTreeMap::new(),
        );
        let initialize = take(&mut session, "initialize");
        session.handle(&response(&initialize, json!({})));
        let attach = take(&mut session, "attach");
        assert_eq!(attach["arguments"]["port"], 5005);
    }

    #[test]
    fn conditions_and_logpoints_are_sent_with_the_breakpoint() {
        let mut session = started();
        let path = PathBuf::from("/w/src/main.rs");
        session.set_breakpoint_options(
            path.clone(),
            4,
            BreakpointOptions {
                condition: Some(" n > 3 ".into()),
                log_message: None,
                hit_condition: Some("2".into()),
            },
        );
        session.set_breakpoint_options(
            path.clone(),
            9,
            BreakpointOptions {
                log_message: Some("n={n}".into()),
                ..Default::default()
            },
        );
        let sent = take(&mut session, "setBreakpoints");
        // Both calls resend the file; the first one carries only the first option.
        assert_eq!(
            sent["arguments"]["breakpoints"][0],
            json!({"line": 5, "condition": "n > 3", "hitCondition": "2"})
        );
        let all = session.breakpoint_options(&path, 9).unwrap();
        assert_eq!(all.log_message.as_deref(), Some("n={n}"));
        session.set_breakpoint_options(path.clone(), 4, BreakpointOptions::default());
        assert!(session.breakpoint_options(&path, 4).is_none());
        let last = session
            .take_outgoing()
            .into_iter()
            .rfind(|m| m["command"] == "setBreakpoints")
            .unwrap();
        assert_eq!(
            last["arguments"]["breakpoints"],
            json!([{"line": 5}, {"line": 10, "logMessage": "n={n}"}])
        );
    }

    #[test]
    fn options_are_dropped_when_the_breakpoint_is_removed() {
        let mut session = started();
        let path = PathBuf::from("/w/src/main.rs");
        session.set_breakpoint_options(
            path.clone(),
            4,
            BreakpointOptions {
                condition: Some("x".into()),
                ..Default::default()
            },
        );
        session.update_breakpoints(path.clone(), BTreeSet::from([9]));
        assert!(session.breakpoint_options(&path, 4).is_none());
    }

    #[test]
    fn watches_are_evaluated_on_each_stop() {
        let mut session = started();
        session.add_watch("n * 2");
        // Not stopped: nothing is sent yet.
        assert!(
            session
                .take_outgoing()
                .iter()
                .all(|m| m["command"] != "evaluate")
        );
        session.handle(&json!({"type": "event", "event": "stopped", "body": {"reason": "breakpoint", "threadId": 7}}));
        let outgoing = session.take_outgoing();
        let trace = outgoing
            .iter()
            .find(|m| m["command"] == "stackTrace")
            .unwrap()
            .clone();
        session.handle(&response(
            &trace,
            json!({"stackFrames": [{"id": 101, "name": "main", "line": 5, "column": 1, "source": {"path": "/w/src/main.rs"}}]}),
        ));
        let evaluate = take(&mut session, "evaluate");
        assert_eq!(evaluate["arguments"]["context"], "watch");
        assert_eq!(evaluate["arguments"]["frameId"], 101);
        session.handle(&response(&evaluate, json!({"result": "84"})));
        assert_eq!(session.watches()[0].result, Some(Ok("84".into())));

        // Step: the next stop clears and re-evaluates.
        session.step(Step::Next);
        session.handle(&json!({"type": "event", "event": "stopped", "body": {"reason": "step", "threadId": 7}}));
        let outgoing = session.take_outgoing();
        let trace = outgoing
            .iter()
            .find(|m| m["command"] == "stackTrace")
            .unwrap()
            .clone();
        session.handle(&response(
            &trace,
            json!({"stackFrames": [{"id": 102, "name": "main", "line": 6, "column": 1, "source": {"path": "/w/src/main.rs"}}]}),
        ));
        assert_eq!(session.watches()[0].result, None);
        let evaluate = take(&mut session, "evaluate");
        assert_eq!(evaluate["arguments"]["frameId"], 102);
        session.handle(&json!({
            "type": "response", "request_seq": evaluate["seq"], "command": "evaluate",
            "success": false, "message": "n is not defined"
        }));
        assert_eq!(
            session.watches()[0].result,
            Some(Err("n is not defined".into()))
        );
    }

    #[test]
    fn watches_can_be_added_edited_and_removed() {
        let mut session = stopped_at_main();
        session.add_watch("a");
        session.add_watch(" a ");
        session.add_watch("");
        assert_eq!(session.watches().len(), 1);
        let evaluate = take(&mut session, "evaluate");
        assert_eq!(evaluate["arguments"]["expression"], "a");
        session.add_watch("b");
        session.edit_watch(0, "c");
        assert_eq!(session.watches()[0].expression, "c");
        // A stale response for the edited expression does not fill the new one.
        session.handle(&response(&evaluate, json!({"result": "1"})));
        assert_eq!(session.watches()[0].result, None);
        session.edit_watch(0, "b"); // duplicate: ignored
        assert_eq!(session.watches()[0].expression, "c");
        session.edit_watch(1, "");
        assert_eq!(session.watches().len(), 1);
        session.remove_watch(0);
        assert!(session.watches().is_empty());
    }

    #[test]
    fn run_to_line_sets_a_temporary_breakpoint_and_removes_it_at_the_next_stop() {
        let mut session = stopped_at_main();
        let path = PathBuf::from("/w/src/main.rs");
        session.run_to_line(path.clone(), 20);
        let outgoing = session.take_outgoing();
        let set = outgoing
            .iter()
            .find(|m| m["command"] == "setBreakpoints")
            .unwrap();
        assert_eq!(
            set["arguments"]["breakpoints"],
            json!([{"line": 5}, {"line": 10}, {"line": 21}])
        );
        assert!(outgoing.iter().any(|m| m["command"] == "continue"));
        // The temporary line never shows up as a user breakpoint.
        session.handle(&response(
            set,
            json!({"breakpoints": [{"verified": true}, {"verified": true}, {"verified": true}]}),
        ));
        assert!(session.breakpoint_statuses().iter().all(|s| s.line != 20));
        session.handle(&json!({"type": "event", "event": "stopped", "body": {"reason": "breakpoint", "threadId": 7}}));
        let set = take(&mut session, "setBreakpoints");
        assert_eq!(
            set["arguments"]["breakpoints"],
            json!([{"line": 5}, {"line": 10}])
        );
    }

    #[test]
    fn run_to_line_in_another_file_cleans_up_both() {
        let mut session = stopped_at_main();
        session.run_to_line(PathBuf::from("/w/src/lib.rs"), 2);
        let outgoing = session.take_outgoing();
        let set = outgoing
            .iter()
            .find(|m| m["arguments"]["source"]["path"] == "/w/src/lib.rs")
            .unwrap();
        assert_eq!(set["arguments"]["breakpoints"], json!([{"line": 3}]));
        session.handle(&json!({"type": "event", "event": "stopped", "body": {"reason": "breakpoint", "threadId": 7}}));
        let set = take(&mut session, "setBreakpoints");
        assert_eq!(set["arguments"]["breakpoints"], json!([]));
    }

    #[test]
    fn run_to_line_requires_a_stop() {
        let mut session = started();
        session.run_to_line(PathBuf::from("/w/src/main.rs"), 20);
        assert!(session.take_outgoing().is_empty());
    }

    #[test]
    fn force_step_into_uses_the_first_step_in_target() {
        let mut session = stopped_at_main();
        session.step(Step::ForceStepIn);
        let targets = take(&mut session, "stepInTargets");
        assert_eq!(targets["arguments"]["frameId"], 101);
        session.handle(&response(
            &targets,
            json!({"targets": [{"id": 3, "label": "helper()"}, {"id": 4, "label": "other()"}]}),
        ));
        let step_in = take(&mut session, "stepIn");
        assert_eq!(step_in["arguments"], json!({"threadId": 7, "targetId": 3}));
        assert_eq!(session.state(), &SessionState::Running);
    }

    #[test]
    fn force_step_into_falls_back_to_a_plain_step_in() {
        let mut session = stopped_at_main();
        session.step(Step::ForceStepIn);
        let targets = take(&mut session, "stepInTargets");
        session.handle(&response(&targets, json!({"targets": []})));
        let step_in = take(&mut session, "stepIn");
        assert!(step_in["arguments"].get("targetId").is_none());

        let mut session = stopped_at_main();
        session.step(Step::ForceStepIn);
        let targets = take(&mut session, "stepInTargets");
        session.handle(&json!({
            "type": "response", "request_seq": targets["seq"], "command": "stepInTargets",
            "success": false, "message": "unsupported"
        }));
        take(&mut session, "stepIn");
    }
}

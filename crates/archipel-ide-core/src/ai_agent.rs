//! Agent mode: runs a multi-step task with project tools. Reading, listing and
//! searching run automatically; every edit, new file and command waits for the
//! user's approval. Edits go through editor documents, so they are undoable.

use crate::{IdeState, ToolWindow, spec, string_param, usize_param};
use archipel_ide_ai::agent::{AgentConversation, AgentTool, TurnOutcome};
use archipel_ide_ai::diff::{DiffLine, line_diff};
use archipel_ide_ai::{AiError, AiUpdate, ChatResponse, StreamEvent};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_runner::{TaskPlan, TaskState};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const LIST_LIMIT: usize = 1_000;
const SEARCH_LIMIT: usize = 100;
const OUTPUT_LINES: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    Running,
    AwaitingApproval,
    Done,
    Rejected,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepKind {
    /// Assistant text (plan, explanations, summary).
    Message(String),
    Tool {
        summary: String,
        /// Diff of an edit or new file, shown for approval.
        diff: Vec<DiffLine>,
        /// Command line of `run_command`.
        command: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStep {
    pub kind: StepKind,
    pub status: StepStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentPhase {
    Thinking,
    AwaitingApproval,
    RunningCommand,
    Done,
    Stopped,
    Failed(String),
}

impl AgentPhase {
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            AgentPhase::Thinking | AgentPhase::AwaitingApproval | AgentPhase::RunningCommand
        )
    }
}

#[derive(Debug, Clone)]
struct PendingCall {
    id: String,
    tool: Option<AgentTool>,
    step: usize,
    result: Option<Result<String, String>>,
}

/// One agent task.
#[derive(Debug, Clone)]
pub struct AgentRun {
    pub goal: String,
    pub steps: Vec<AgentStep>,
    pub phase: AgentPhase,
    conversation: AgentConversation,
    request: Option<u64>,
    pending: Vec<PendingCall>,
    /// Step receiving streamed text of the current turn.
    streaming_step: Option<usize>,
    /// When the running command was seen to exit (its last output may trail).
    command_exited: Option<std::time::Instant>,
}

impl AgentRun {
    /// Index of the step awaiting approval, if any.
    pub fn awaiting(&self) -> Option<usize> {
        self.steps
            .iter()
            .position(|s| s.status == StepStatus::AwaitingApproval)
    }
}

fn numbered(text: &str) -> String {
    text.lines()
        .enumerate()
        .map(|(index, line)| format!("{:>5}| {line}", index + 1))
        .collect::<Vec<_>>()
        .join("\n")
}

impl IdeState {
    fn agent_shell() -> (String, Vec<String>) {
        if cfg!(windows) {
            (
                "powershell".into(),
                vec!["-NoProfile".into(), "-Command".into()],
            )
        } else {
            ("sh".into(), vec!["-c".into()])
        }
    }

    /// Current text of a project file: the open document, else the file on disk.
    fn agent_text(&self, path: &Path) -> Result<String, String> {
        if let Some(document) = self.documents.iter().find(|d| d.path == path) {
            return Ok(document.text.text());
        }
        self.project
            .as_ref()
            .ok_or("no project is open")?
            .read_text(path)
            .map_err(|e| e.to_string())
    }

    pub fn ai_agent_start(&mut self, goal: &str) -> Result<(), String> {
        if !self.ai.is_enabled() {
            return Err("AI is off: configure a provider first".into());
        }
        if !self.ai.policy.share_file_contents {
            return Err("agent mode reads project files; this project does not share them".into());
        }
        if goal.trim().is_empty() {
            return Err("describe the task first".into());
        }
        if self.ai.agent.as_ref().is_some_and(|a| a.phase.is_active()) {
            return Err("stop the running task first".into());
        }
        let conversation = AgentConversation::new(goal);
        let id = self.ai.start(conversation.request())?;
        self.ai.agent = Some(AgentRun {
            goal: goal.trim().to_owned(),
            steps: Vec::new(),
            phase: AgentPhase::Thinking,
            conversation,
            request: Some(id),
            pending: Vec::new(),
            streaming_step: None,
            command_exited: None,
        });
        self.tool_windows.show(ToolWindow::AiChat);
        Ok(())
    }

    pub fn ai_agent_stop(&mut self) {
        let execution = self.execution();
        let Some(agent) = self.ai.agent.as_mut() else {
            return;
        };
        if let Some(id) = agent.request.take() {
            self.ai.service.cancel(id);
        }
        if agent.phase == AgentPhase::RunningCommand {
            let _ = execution.cancel();
        }
        for step in &mut agent.steps {
            if matches!(
                step.status,
                StepStatus::Running | StepStatus::AwaitingApproval
            ) {
                step.status = StepStatus::Rejected;
            }
        }
        if agent.phase.is_active() {
            agent.phase = AgentPhase::Stopped;
        }
    }

    /// Handles an update of the agent's request; returns it back otherwise.
    pub(crate) fn ai_agent_update(&mut self, id: u64, update: AiUpdate) -> Option<AiUpdate> {
        let Some(agent) = self.ai.agent.as_mut().filter(|a| a.request == Some(id)) else {
            return Some(update);
        };
        match update {
            AiUpdate::Event(StreamEvent::TextDelta(text)) => {
                let step = *agent.streaming_step.get_or_insert_with(|| {
                    agent.steps.push(AgentStep {
                        kind: StepKind::Message(String::new()),
                        status: StepStatus::Running,
                    });
                    agent.steps.len() - 1
                });
                if let StepKind::Message(message) = &mut agent.steps[step].kind {
                    message.push_str(&text);
                }
            }
            AiUpdate::Event(_) => {}
            AiUpdate::Finished(result) => self.ai_agent_turn(result),
        }
        None
    }

    fn ai_agent_turn(&mut self, result: Result<ChatResponse, AiError>) {
        let Some(agent) = self.ai.agent.as_mut() else {
            return;
        };
        agent.request = None;
        let streamed = agent.streaming_step.take();
        let response = match result {
            Ok(response) => response,
            Err(AiError::Cancelled) => {
                agent.phase = AgentPhase::Stopped;
                return;
            }
            Err(error) => {
                agent.phase = AgentPhase::Failed(error.to_string());
                return;
            }
        };
        // The assembled text is authoritative.
        let text = response.text();
        match streamed {
            Some(step) => {
                agent.steps[step].kind = StepKind::Message(text);
                agent.steps[step].status = StepStatus::Done;
            }
            None if !text.trim().is_empty() => agent.steps.push(AgentStep {
                kind: StepKind::Message(text),
                status: StepStatus::Done,
            }),
            None => {}
        }
        match agent.conversation.assistant_turn(&response) {
            TurnOutcome::Done => agent.phase = AgentPhase::Done,
            TurnOutcome::Failed(message) => agent.phase = AgentPhase::Failed(message),
            TurnOutcome::Tools(calls) => {
                agent.pending.clear();
                for call in calls {
                    let (summary, tool, status, result) = match call.tool {
                        Ok(tool) => (tool.summary(), Some(tool), StepStatus::Running, None),
                        Err(error) => (
                            "Invalid tool call".to_owned(),
                            None,
                            StepStatus::Failed(error.clone()),
                            Some(Err(error)),
                        ),
                    };
                    agent.steps.push(AgentStep {
                        kind: StepKind::Tool {
                            summary,
                            diff: Vec::new(),
                            command: None,
                        },
                        status,
                    });
                    agent.pending.push(PendingCall {
                        id: call.id,
                        tool,
                        step: agent.steps.len() - 1,
                        result,
                    });
                }
                self.ai_agent_advance();
            }
        }
    }

    /// Runs automatic tools, prepares approvals in order, and sends the results
    /// once every call of the turn has one.
    fn ai_agent_advance(&mut self) {
        loop {
            let Some(agent) = self.ai.agent.as_ref() else {
                return;
            };
            let Some(index) = agent.pending.iter().position(|c| c.result.is_none()) else {
                break;
            };
            let call = agent.pending[index].clone();
            let Some(tool) = call.tool.clone() else {
                break;
            };
            if tool.needs_approval() {
                if agent.steps[call.step].status == StepStatus::Running {
                    match self.ai_agent_prepare(&tool) {
                        Ok((diff, command)) => {
                            let agent = self.ai.agent.as_mut().expect("agent exists");
                            let step = &mut agent.steps[call.step];
                            if let StepKind::Tool {
                                diff: d,
                                command: c,
                                ..
                            } = &mut step.kind
                            {
                                *d = diff;
                                *c = command;
                            }
                            step.status = StepStatus::AwaitingApproval;
                            agent.phase = AgentPhase::AwaitingApproval;
                        }
                        Err(error) => {
                            self.ai_agent_record(index, Err(error));
                            continue;
                        }
                    }
                }
                return;
            }
            let result = self.ai_agent_execute(&tool);
            self.ai_agent_record(index, result);
        }
        self.ai_agent_send_results();
    }

    fn ai_agent_record(&mut self, index: usize, result: Result<String, String>) {
        let Some(agent) = self.ai.agent.as_mut() else {
            return;
        };
        let call = &mut agent.pending[index];
        agent.steps[call.step].status = match &result {
            Ok(_) => StepStatus::Done,
            Err(error) => StepStatus::Failed(error.lines().next().unwrap_or_default().to_owned()),
        };
        call.result = Some(result);
    }

    fn ai_agent_send_results(&mut self) {
        let Some(agent) = self.ai.agent.as_mut() else {
            return;
        };
        if !agent.phase.is_active() || agent.pending.iter().any(|c| c.result.is_none()) {
            return;
        }
        let results = agent
            .pending
            .drain(..)
            .map(|call| (call.id, call.result.expect("all results present")))
            .collect();
        agent.conversation.tool_results(results);
        let request = agent.conversation.request();
        agent.phase = AgentPhase::Thinking;
        match self.ai.start(request) {
            Ok(id) => {
                if let Some(agent) = self.ai.agent.as_mut() {
                    agent.request = Some(id);
                }
            }
            Err(error) => {
                if let Some(agent) = self.ai.agent.as_mut() {
                    agent.phase = AgentPhase::Failed(error);
                }
            }
        }
    }

    /// Diff or command shown for approval; errors (e.g. text not found) go straight
    /// back to the model.
    fn ai_agent_prepare(
        &self,
        tool: &AgentTool,
    ) -> Result<(Vec<DiffLine>, Option<String>), String> {
        match tool {
            AgentTool::EditFile {
                path,
                old_text,
                new_text,
            } => {
                let before = self.agent_text(Path::new(path))?;
                let after = replace_once(&before, old_text, new_text)?;
                Ok((line_diff(&before, &after, 3), None))
            }
            AgentTool::CreateFile { path, content } => {
                let root = self.project.as_ref().ok_or("no project is open")?.root();
                if root.join(path).exists() {
                    return Err(format!("{path} already exists; use edit_file"));
                }
                Ok((line_diff("", content, 3), None))
            }
            AgentTool::RunCommand { command } => Ok((Vec::new(), Some(command.clone()))),
            _ => Ok((Vec::new(), None)),
        }
    }

    /// Runs a tool that needs no approval (or an approved one that completes now).
    fn ai_agent_execute(&mut self, tool: &AgentTool) -> Result<String, String> {
        match tool {
            AgentTool::ListFiles { directory } => {
                let prefix = Path::new(directory.trim_matches('/'));
                let files: Vec<String> = self
                    .files
                    .iter()
                    .filter(|e| !e.is_dir && e.relative_path.starts_with(prefix))
                    .map(|e| e.relative_path.display().to_string())
                    .collect();
                let total = files.len();
                let mut listing = files
                    .into_iter()
                    .take(LIST_LIMIT)
                    .collect::<Vec<_>>()
                    .join("\n");
                if total > LIST_LIMIT {
                    listing.push_str(&format!("\n[{} more files not listed]", total - LIST_LIMIT));
                }
                Ok(if listing.is_empty() {
                    "(no files)".into()
                } else {
                    listing
                })
            }
            AgentTool::ReadFile { path } => Ok(numbered(&self.agent_text(Path::new(path))?)),
            AgentTool::Search { query } => {
                let needle = query.to_lowercase();
                let mut matches = Vec::new();
                for entry in self.files.iter().filter(|e| !e.is_dir) {
                    let Ok(text) = self.agent_text(&entry.relative_path) else {
                        continue;
                    };
                    for (line, content) in text.lines().enumerate() {
                        if content.to_lowercase().contains(&needle) {
                            matches.push(format!(
                                "{}:{}: {}",
                                entry.relative_path.display(),
                                line + 1,
                                content.trim()
                            ));
                        }
                    }
                    if matches.len() >= SEARCH_LIMIT {
                        matches.truncate(SEARCH_LIMIT);
                        matches.push(format!("[stopped after {SEARCH_LIMIT} matches]"));
                        break;
                    }
                }
                Ok(if matches.is_empty() {
                    "no matches".into()
                } else {
                    matches.join("\n")
                })
            }
            AgentTool::EditFile {
                path,
                old_text,
                new_text,
            } => {
                let path = PathBuf::from(path);
                self.open_file(&path)?;
                let index = self.active_document.ok_or("file did not open")?;
                let before = self.documents[index].text.text();
                let start_byte = unique_match(&before, old_text)?;
                let start = before[..start_byte].chars().count();
                let end = start + old_text.chars().count();
                self.edit_active(|document| {
                    document
                        .text
                        .edit(start, end, new_text)
                        .map_err(|e| e.to_string())?;
                    document
                        .selections
                        .set_single(archipel_ide_text::Selection::caret(
                            start + new_text.chars().count(),
                        ));
                    Ok(())
                })?;
                // Saved so builds and tests see the change; Undo in the editor reverts it.
                self.save(index)?;
                Ok(format!("Edited {}", path.display()))
            }
            AgentTool::CreateFile { path, content } => {
                let project = self.project.as_ref().ok_or("no project is open")?;
                let relative = PathBuf::from(path);
                if let Some(parent) = relative.parent().filter(|p| !p.as_os_str().is_empty()) {
                    project
                        .create_dir(parent)
                        .or_else(|error| {
                            if project.root().join(parent).is_dir() {
                                Ok(())
                            } else {
                                Err(error)
                            }
                        })
                        .map_err(|e| e.to_string())?;
                }
                project.create_file(&relative).map_err(|e| e.to_string())?;
                project
                    .write_text(&relative, content)
                    .map_err(|e| e.to_string())?;
                self.set_files(project.entries(20_000));
                self.open_file(&relative)?;
                Ok(format!("Created {path}"))
            }
            AgentTool::RunCommand { .. } => Err("commands run only after approval".into()),
        }
    }

    /// Starts an approved command; its result is recorded when it exits.
    fn ai_agent_run_command(&mut self, command: &str) -> Result<(), String> {
        let root = self
            .project
            .as_ref()
            .ok_or("no project is open")?
            .root()
            .to_owned();
        if self.execution().is_running() {
            return Err("another task is running; try again when it finishes".into());
        }
        let (program, mut args) = Self::agent_shell();
        args.push(command.to_owned());
        self.execution()
            .start(TaskPlan {
                name: "Agent".into(),
                program,
                args,
                cwd: root,
                env: Vec::new(),
                kind: archipel_ide_runner::TaskKind::Custom,
            })
            .map_err(|e| e.to_string())?;
        self.tool_windows.show(ToolWindow::Build);
        if let Some(agent) = self.ai.agent.as_mut() {
            agent.phase = AgentPhase::RunningCommand;
            agent.command_exited = None;
        }
        Ok(())
    }

    /// Approves the step awaiting approval and continues the task.
    pub fn ai_agent_approve(&mut self) -> Result<(), String> {
        let agent = self.ai.agent.as_ref().ok_or("no agent task")?;
        let step = agent.awaiting().ok_or("nothing awaits approval")?;
        let index = agent
            .pending
            .iter()
            .position(|c| c.step == step)
            .ok_or("the step is not pending")?;
        let tool = agent.pending[index]
            .tool
            .clone()
            .ok_or("invalid tool call")?;
        if let Some(agent) = self.ai.agent.as_mut() {
            agent.steps[step].status = StepStatus::Running;
            agent.phase = AgentPhase::Thinking;
        }
        let result = match &tool {
            AgentTool::RunCommand { command } => match self.ai_agent_run_command(command) {
                Ok(()) => return Ok(()),
                Err(error) => Err(error),
            },
            other => self.ai_agent_execute(other),
        };
        self.ai_agent_record(index, result);
        self.ai_agent_advance();
        Ok(())
    }

    /// Rejects the step awaiting approval; the model is told and can adapt.
    pub fn ai_agent_reject(&mut self, reason: &str) -> Result<(), String> {
        let agent = self.ai.agent.as_mut().ok_or("no agent task")?;
        let step = agent.awaiting().ok_or("nothing awaits approval")?;
        let index = agent
            .pending
            .iter()
            .position(|c| c.step == step)
            .ok_or("the step is not pending")?;
        agent.steps[step].status = StepStatus::Rejected;
        agent.phase = AgentPhase::Thinking;
        let message = if reason.trim().is_empty() {
            "The user rejected this action.".to_owned()
        } else {
            format!("The user rejected this action: {}", reason.trim())
        };
        agent.pending[index].result = Some(Err(message));
        self.ai_agent_advance();
        Ok(())
    }

    /// Completes a running `run_command` step when its process has exited.
    pub(crate) fn ai_agent_poll(&mut self) {
        let Some(agent) = self.ai.agent.as_ref() else {
            return;
        };
        if agent.phase != AgentPhase::RunningCommand || self.execution().is_running() {
            return;
        }
        // Output can arrive shortly after the exit status; give it a moment.
        let exited = *self
            .ai
            .agent
            .as_mut()
            .expect("agent exists")
            .command_exited
            .get_or_insert_with(std::time::Instant::now);
        if exited.elapsed() < std::time::Duration::from_millis(200) {
            return;
        }
        let Some(agent) = self.ai.agent.as_ref() else {
            return;
        };
        let Some(index) = agent.pending.iter().position(|c| c.result.is_none()) else {
            return;
        };
        let output: String = self
            .execution()
            .runner()
            .output()
            .iter()
            .map(|c| c.text.as_str())
            .collect();
        let tail = archipel_ide_ai::actions::output_tail(&output, OUTPUT_LINES);
        let result = match self.execution().runner().state() {
            TaskState::Exited(code) => Ok(format!("exit code {code}\n{tail}")),
            TaskState::Cancelled => Err(format!("cancelled\n{tail}")),
            other => Err(format!("command did not complete ({other:?})\n{tail}")),
        };
        if let Some(agent) = self.ai.agent.as_mut() {
            agent.phase = AgentPhase::Thinking;
        }
        self.ai_agent_record(index, result);
        self.ai_agent_advance();
    }
}

/// Byte offset of the only occurrence of `needle`.
fn unique_match(haystack: &str, needle: &str) -> Result<usize, String> {
    let mut found = haystack.match_indices(needle);
    match (found.next(), found.next()) {
        (Some((index, _)), None) => Ok(index),
        (None, _) => Err(
            "old_text was not found in the file; read it again and copy the text exactly".into(),
        ),
        (Some(_), Some(_)) => Err(format!(
            "old_text occurs {} times; include more surrounding lines so it is unique",
            haystack.matches(needle).count()
        )),
    }
}

fn replace_once(haystack: &str, needle: &str, replacement: &str) -> Result<String, String> {
    let index = unique_match(haystack, needle)?;
    Ok(format!(
        "{}{}{}",
        &haystack[..index],
        replacement,
        &haystack[index + needle.len()..]
    ))
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let awaiting = |state: &IdeState| {
        state
            .ai
            .agent
            .as_ref()
            .and_then(AgentRun::awaiting)
            .is_some()
    };
    let commands = vec![
        Command::new(
            spec(
                "ai.agent.start",
                "AI Agent: Start Task",
                "AI",
                None,
                r#"{"goal":"string"}"#,
            ),
            |state: &IdeState| {
                state.project.is_some()
                    && state.ai.is_enabled()
                    && !state.ai.agent.as_ref().is_some_and(|a| a.phase.is_active())
            },
            |state: &mut IdeState, params| {
                let goal = string_param(&params, "goal")?;
                state.ai_agent_start(&goal).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec(
                "ai.agent.approve",
                "AI Agent: Approve Step",
                "AI",
                None,
                "{}",
            ),
            awaiting,
            |state: &mut IdeState, _| {
                state.ai_agent_approve().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec(
                "ai.agent.reject",
                "AI Agent: Reject Step",
                "AI",
                None,
                r#"{"reason":"string?"}"#,
            ),
            awaiting,
            |state: &mut IdeState, params| {
                let reason = params
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                state
                    .ai_agent_reject(reason)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec("ai.agent.stop", "AI Agent: Stop", "AI", None, "{}"),
            |state: &IdeState| state.ai.agent.as_ref().is_some_and(|a| a.phase.is_active()),
            |state: &mut IdeState, _| {
                state.ai_agent_stop();
                state.poll_ai();
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec("ai.agent.clear", "AI Agent: New Task", "AI", None, "{}"),
            |state: &IdeState| {
                state
                    .ai
                    .agent
                    .as_ref()
                    .is_some_and(|a| !a.phase.is_active())
            },
            |state: &mut IdeState, _| {
                state.ai.agent = None;
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec(
                "ai.agent.step",
                "AI Agent: Step Details",
                "AI",
                None,
                r#"{"index":0}"#,
            ),
            |state: &IdeState| state.ai.agent.is_some(),
            |state: &mut IdeState, params| {
                let index = usize_param(&params, "index")?;
                let agent = state
                    .ai
                    .agent
                    .as_ref()
                    .ok_or_else(|| CommandError::Disabled("ai.agent.step".into()))?;
                let step = agent
                    .steps
                    .get(index)
                    .ok_or_else(|| CommandError::InvalidParams("no such step".into()))?;
                Ok(
                    json!({"step": format!("{:?}", step.kind), "status": format!("{:?}", step.status)}),
                )
            },
        ),
    ];
    for command in commands {
        registry.register(command).expect("unique built-in id");
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentPhase, StepKind, StepStatus};
    use crate::{IdeState, built_in_commands};
    use archipel_ide_ai::{ContentBlock, MockProvider, MockReply};
    use archipel_ide_command::CommandRegistry;
    use serde_json::{Value, json};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn open(
        name: &str,
        replies: Vec<MockReply>,
    ) -> (
        IdeState,
        CommandRegistry<IdeState>,
        Arc<MockProvider>,
        PathBuf,
    ) {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-agent-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/lib.rs"),
            "pub fn add(a: u8, b: u8) -> u8 {\n    a - b\n}\n",
        )
        .unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        state.init_ai(Some(root.join(".config")));
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        let provider = Arc::new(MockProvider::new(replies));
        state.ai.use_provider(provider.clone());
        (state, commands, provider, root)
    }

    /// Polls until the agent needs the user or finishes.
    fn settle(state: &mut IdeState) -> AgentPhase {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            state.poll_runner();
            state.poll_ai();
            let phase = state.ai.agent.as_ref().unwrap().phase.clone();
            if !matches!(phase, AgentPhase::Thinking | AgentPhase::RunningCommand) {
                return phase;
            }
            assert!(Instant::now() < deadline, "agent did not settle: {phase:?}");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn tool(name: &str, input: Value) -> MockReply {
        MockReply::tool(name, input)
    }

    fn results(provider: &MockProvider, request: usize) -> Vec<(String, bool)> {
        provider.requests()[request]
            .messages
            .last()
            .unwrap()
            .content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::ToolResult {
                    content, is_error, ..
                } => Some((content.clone(), *is_error)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn reads_automatically_and_edits_only_after_approval() {
        let replies = vec![
            MockReply {
                chunks: vec!["1. Read lib.rs\n2. Fix add".into()],
                tool_calls: vec![("read_file".into(), json!({"path": "src/lib.rs"}))],
                ..MockReply::default()
            },
            tool(
                "edit_file",
                json!({"path": "src/lib.rs", "old_text": "a - b", "new_text": "a + b"}),
            ),
            MockReply::text("Fixed `add`."),
        ];
        let (mut state, commands, provider, root) = open("edit", replies);
        commands
            .execute("ai.agent.start", &mut state, json!({"goal": "fix add"}))
            .unwrap();
        assert_eq!(settle(&mut state), AgentPhase::AwaitingApproval);
        assert!(results(&provider, 1)[0].0.contains("    2|     a - b"));
        let agent = state.ai.agent.as_ref().unwrap();
        assert!(matches!(&agent.steps[0].kind, StepKind::Message(m) if m.starts_with("1. Read")));
        let StepKind::Tool { summary, diff, .. } = &agent.steps[2].kind else {
            panic!("{:?}", agent.steps)
        };
        assert_eq!(summary, "Edit src/lib.rs");
        assert_eq!(diff.iter().filter(|l| l.text.contains("a + b")).count(), 1);
        // Nothing is written before approval.
        assert!(
            std::fs::read_to_string(root.join("src/lib.rs"))
                .unwrap()
                .contains("a - b")
        );
        commands
            .execute("ai.agent.approve", &mut state, Value::Null)
            .unwrap();
        assert_eq!(settle(&mut state), AgentPhase::Done);
        assert!(
            std::fs::read_to_string(root.join("src/lib.rs"))
                .unwrap()
                .contains("a + b")
        );
        assert_eq!(
            results(&provider, 2),
            vec![("Edited src/lib.rs".into(), false)]
        );
        // The edit is one undoable step in the editor.
        commands
            .execute("editor.undo", &mut state, Value::Null)
            .unwrap();
        assert!(state.documents[0].text.text().contains("a - b"));
        assert!(commands.is_enabled("ai.agent.clear", &state));
    }

    #[test]
    fn rejection_and_bad_edits_are_reported_to_the_model() {
        let replies = vec![
            tool(
                "edit_file",
                json!({"path": "src/lib.rs", "old_text": "missing", "new_text": "x"}),
            ),
            tool(
                "create_file",
                json!({"path": "src/new.rs", "content": "// new\n"}),
            ),
            MockReply::text("Understood, stopping."),
        ];
        let (mut state, commands, provider, root) = open("reject", replies);
        commands
            .execute("ai.agent.start", &mut state, json!({"goal": "do things"}))
            .unwrap();
        assert_eq!(settle(&mut state), AgentPhase::AwaitingApproval);
        let first = results(&provider, 1);
        assert!(first[0].1 && first[0].0.contains("not found"), "{first:?}");
        commands
            .execute(
                "ai.agent.reject",
                &mut state,
                json!({"reason": "no new files"}),
            )
            .unwrap();
        assert_eq!(settle(&mut state), AgentPhase::Done);
        assert_eq!(
            results(&provider, 2),
            vec![("The user rejected this action: no new files".into(), true)]
        );
        assert!(!root.join("src/new.rs").exists());
        let statuses: Vec<_> = state
            .ai
            .agent
            .as_ref()
            .unwrap()
            .steps
            .iter()
            .map(|s| s.status.clone())
            .collect();
        assert!(statuses.contains(&StepStatus::Rejected));
    }

    #[cfg(unix)]
    #[test]
    fn commands_run_after_approval_and_return_exit_code_and_output() {
        let replies = vec![
            tool("run_command", json!({"command": "echo agent-ran; exit 3"})),
            MockReply::text("The command failed with 3."),
        ];
        let (mut state, commands, provider, _) = open("command", replies);
        commands
            .execute("ai.agent.start", &mut state, json!({"goal": "run it"}))
            .unwrap();
        assert_eq!(settle(&mut state), AgentPhase::AwaitingApproval);
        assert!(!state.execution().is_running());
        commands
            .execute("ai.agent.approve", &mut state, Value::Null)
            .unwrap();
        assert_eq!(settle(&mut state), AgentPhase::Done);
        let result = &results(&provider, 1)[0];
        assert!(result.0.starts_with("exit code 3\n"), "{result:?}");
        assert!(result.0.contains("agent-ran"), "{result:?}");
    }

    #[test]
    fn stop_cancels_and_create_file_writes_after_approval() {
        let replies = vec![
            tool(
                "create_file",
                json!({"path": "src/extra/mod.rs", "content": "pub fn x() {}\n"}),
            ),
            MockReply::text("Created."),
        ];
        let (mut state, commands, _, root) = open("create", replies);
        commands
            .execute("ai.agent.start", &mut state, json!({"goal": "add module"}))
            .unwrap();
        assert_eq!(settle(&mut state), AgentPhase::AwaitingApproval);
        commands
            .execute("ai.agent.approve", &mut state, Value::Null)
            .unwrap();
        assert_eq!(settle(&mut state), AgentPhase::Done);
        assert_eq!(
            std::fs::read_to_string(root.join("src/extra/mod.rs")).unwrap(),
            "pub fn x() {}\n"
        );
        commands
            .execute("ai.agent.clear", &mut state, Value::Null)
            .unwrap();

        let mut slow = MockReply::text("thinking for a long time about this task");
        slow.delay = Duration::from_millis(50);
        let (mut state, commands, _, _) = open("stop", vec![slow]);
        commands
            .execute("ai.agent.start", &mut state, json!({"goal": "x"}))
            .unwrap();
        commands
            .execute("ai.agent.stop", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.ai.agent.as_ref().unwrap().phase, AgentPhase::Stopped);
        assert!(commands.is_enabled("ai.agent.start", &state));
    }
}

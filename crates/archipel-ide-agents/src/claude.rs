//! Claude Code adapter: `claude -p --output-format stream-json --input-format stream-json
//! --verbose`.
//!
//! Sources. [DOC] `docs/agents/agent-integrations.md` section 1 (and the CLI reference it cites)
//! documents the flags used here: `-p`, `--output-format`, `--input-format`, `--verbose`,
//! `--permission-mode <default|acceptEdits|plan|...>`, `--session-id <uuid>` and
//! `--resume <id>`. That document does **not** describe the message shapes of the stream, so
//! every shape below is [INFER] from Anthropic's public Agent SDK documentation and must be
//! confirmed against a real install (see the verification list in the report): top-level
//! messages `system` (subtype `init`), `assistant` and `user` (with `message.content` blocks
//! `text`, `thinking`, `tool_use`, `tool_result`), `result`, and, in `--permission-prompt-tool
//! stdio` mode, `control_request` (`can_use_tool`) answered by a `control_response` line.
//!
//! Everything is parsed defensively: unknown fields are ignored, wrong types skip the field,
//! unknown tools become a generic tool row and unknown message types a generic `Other` row.

use crate::adapter::{Choice, Parsed};
use crate::events::{
    AgentEvent, DiffLine, EditSummary, PERMISSION_OPTIONS, ToolKind, ToolStatus,
};
use crate::update::{Update, first_line, truncate};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Message types that carry no transcript content.
const IGNORED_TYPES: [&str; 5] = [
    "stream_event",
    "rate_limit_event",
    "keep_alive",
    "control_response",
    "auth_status",
];
/// Longest tool output kept in a transcript row.
const MAX_OUTPUT: usize = 20_000;

struct PendingPermission {
    request_id: String,
    tool: String,
    input: Value,
    suggestions: Option<Value>,
}

pub struct ClaudeAdapter {
    cwd: PathBuf,
    next_call: u32,
    next_permission: u32,
    calls: HashMap<String, u32>,
    pending: HashMap<u32, PendingPermission>,
    /// Output tokens already counted for the turn in progress.
    running_tokens: u32,
}

impl ClaudeAdapter {
    /// `cwd` is the session's worktree; paths inside it are shown relative.
    pub fn new(cwd: &Path) -> Self {
        Self {
            cwd: cwd.to_path_buf(),
            next_call: 1,
            next_permission: 1,
            calls: HashMap::new(),
            pending: HashMap::new(),
            running_tokens: 0,
        }
    }

    /// The line to write to stdin to send `text` as a user message.
    pub fn user_message(text: &str) -> String {
        json!({
            "type": "user",
            "message": {"role": "user", "content": [{"type": "text", "text": text}]}
        })
        .to_string()
    }

    pub fn parse_line(&mut self, line: &str) -> Parsed {
        let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
            return Parsed::none();
        };
        let Some(kind) = value.get("type").and_then(Value::as_str) else {
            return Parsed::none();
        };
        match kind {
            "system" => self.system(&value),
            "assistant" => self.assistant(&value),
            "user" => self.user(&value),
            "result" => self.result(&value),
            "control_request" => self.control_request(&value),
            "control_cancel_request" => self.control_cancel(&value),
            other if IGNORED_TYPES.contains(&other) => Parsed::none(),
            other => Parsed::one(Update::Push {
                event: self.generic(&format!("Event {other}"), &value),
            }),
        }
    }

    /// Answers the open permission request `id`. Returns the stdin line and the update.
    pub fn answer(&mut self, id: u32, choice: Choice) -> Option<(String, Update)> {
        let pending = self.pending.remove(&id)?;
        let response = match choice {
            Choice::Deny => json!({
                "behavior": "deny",
                "message": "The user denied this request in Archipel IDE."
            }),
            Choice::AllowOnce => json!({
                "behavior": "allow",
                "updatedInput": pending.input
            }),
            Choice::AlwaysAllow => {
                let permissions = pending.suggestions.clone().unwrap_or_else(|| {
                    json!([{
                        "type": "addRules",
                        "rules": [{"toolName": pending.tool}],
                        "behavior": "allow",
                        "destination": "session"
                    }])
                });
                json!({
                    "behavior": "allow",
                    "updatedInput": pending.input,
                    "updatedPermissions": permissions
                })
            }
        };
        let line = json!({
            "type": "control_response",
            "response": {
                "subtype": "success",
                "request_id": pending.request_id,
                "response": response
            }
        })
        .to_string();
        Some((
            line,
            Update::Answered {
                id,
                answer: choice.label().to_owned(),
            },
        ))
    }

    /// Open requests were lost (the process ended): their cards are closed by the caller.
    pub fn drop_pending(&mut self) -> Vec<u32> {
        let ids: Vec<u32> = self.pending.keys().copied().collect();
        self.pending.clear();
        ids
    }

    fn system(&mut self, value: &Value) -> Parsed {
        match value.get("subtype").and_then(Value::as_str) {
            Some("init") => Parsed::one(Update::Session {
                id: str_field(value, "session_id").unwrap_or_default(),
                model: str_field(value, "model"),
            }),
            // Hooks, compaction boundaries and the like carry no transcript content.
            _ => Parsed::none(),
        }
    }

    fn assistant(&mut self, value: &Value) -> Parsed {
        let mut out = Parsed::none();
        let Some(message) = value.get("message") else {
            return out;
        };
        if let Some(blocks) = message.get("content").and_then(Value::as_array) {
            for block in blocks {
                match block.get("type").and_then(Value::as_str) {
                    Some("text") => {
                        if let Some(text) = str_field(block, "text").filter(|t| !t.trim().is_empty())
                        {
                            out.updates.push(Update::Push {
                                event: AgentEvent::AssistantText(text),
                            });
                        }
                    }
                    Some("thinking") => {
                        if let Some(text) =
                            str_field(block, "thinking").filter(|t| !t.trim().is_empty())
                        {
                            out.updates.push(Update::Push {
                                event: AgentEvent::Thinking(text),
                            });
                        }
                    }
                    Some("tool_use") => self.tool_use(block, &mut out),
                    _ => {}
                }
            }
        } else if let Some(text) = message.get("content").and_then(Value::as_str)
            && !text.trim().is_empty()
        {
            out.updates.push(Update::Push {
                event: AgentEvent::AssistantText(text.to_owned()),
            });
        }
        let output = message
            .pointer("/usage/output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if output > 0 {
            let output = output.min(u64::from(u32::MAX)) as u32;
            self.running_tokens = self.running_tokens.saturating_add(output);
            out.updates.push(Update::Tokens { add: output });
        }
        out
    }

    fn tool_use(&mut self, block: &Value, out: &mut Parsed) {
        let name = str_field(block, "name").unwrap_or_else(|| "tool".into());
        let empty = Value::Null;
        let input = block.get("input").unwrap_or(&empty);
        let id = self.next_call;
        self.next_call += 1;
        if let Some(tool_use_id) = str_field(block, "id") {
            self.calls.insert(tool_use_id, id);
        }
        let described = self.describe(&name, input);
        out.updates.push(Update::Push {
            event: AgentEvent::ToolCall {
                id,
                kind: described.kind,
                title: described.title,
                args: described.args,
                status: ToolStatus::Running,
                result: None,
                output: None,
                edit: described.edit,
            },
        });
        if described.kind == ToolKind::Spawn {
            // The subagent runs inside this session: there is no thread of its own to open.
            out.updates.push(Update::Push {
                event: AgentEvent::SubagentSpawned {
                    child: 0,
                    name: described.spawn_name.unwrap_or_else(|| "subagent".into()),
                },
            });
        }
    }

    fn user(&mut self, value: &Value) -> Parsed {
        let mut out = Parsed::none();
        let blocks = value
            .pointer("/message/content")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for block in &blocks {
            if block.get("type").and_then(Value::as_str) != Some("tool_result") {
                continue;
            }
            let Some(call) = str_field(block, "tool_use_id").and_then(|id| self.calls.get(&id).copied())
            else {
                continue;
            };
            let is_error = block
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let text = content_text(block.get("content"));
            out.updates.push(Update::ToolResult {
                call,
                status: if is_error {
                    ToolStatus::Error
                } else {
                    ToolStatus::Ok
                },
                result: (!text.trim().is_empty()).then(|| first_line(&text, 80)),
                output: (!text.trim().is_empty()).then(|| truncate(&text, MAX_OUTPUT)),
            });
        }
        out
    }

    fn result(&mut self, value: &Value) -> Parsed {
        let is_error = value
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || value
                .get("subtype")
                .and_then(Value::as_str)
                .is_some_and(|s| s.starts_with("error"));
        let usage = value.get("usage");
        let count = |key: &str| {
            usage
                .and_then(|u| u.get(key))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        };
        let total = count("input_tokens") + count("output_tokens") + count("cache_creation_input_tokens");
        let total = total.min(u64::from(u32::MAX)) as u32;
        let mut out = Parsed::none();
        let extra = total.saturating_sub(self.running_tokens);
        self.running_tokens = 0;
        if extra > 0 {
            out.updates.push(Update::Tokens { add: extra });
        }
        let message = if is_error {
            let text = str_field(value, "result")
                .filter(|t| !t.trim().is_empty())
                .or_else(|| {
                    value
                        .get("errors")
                        .and_then(Value::as_array)
                        .map(|errors| {
                            errors
                                .iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join("; ")
                        })
                        .filter(|t| !t.is_empty())
                })
                .or_else(|| {
                    str_field(value, "subtype").map(|s| format!("Claude Code stopped: {s}"))
                })
                .unwrap_or_else(|| "Claude Code reported an error".into());
            Some(text)
        } else {
            None
        };
        out.updates.push(Update::Turn {
            ok: !is_error,
            duration_ms: value.get("duration_ms").and_then(Value::as_u64),
            cost_usd: value.get("total_cost_usd").and_then(Value::as_f64),
            message,
        });
        out
    }

    fn control_request(&mut self, value: &Value) -> Parsed {
        let Some(request_id) = str_field(value, "request_id") else {
            return Parsed::none();
        };
        let request = value.get("request").unwrap_or(&Value::Null);
        let subtype = request.get("subtype").and_then(Value::as_str);
        if subtype != Some("can_use_tool") {
            // Never leave the agent waiting for a request we do not implement.
            let reply = json!({
                "type": "control_response",
                "response": {
                    "subtype": "error",
                    "request_id": request_id,
                    "error": "not supported by this client"
                }
            })
            .to_string();
            return Parsed {
                updates: Vec::new(),
                replies: vec![reply],
            };
        }
        let tool = str_field(request, "tool_name").unwrap_or_else(|| "tool".into());
        let input = request.get("input").cloned().unwrap_or(Value::Null);
        let described = self.describe(&tool, &input);
        let id = self.next_permission;
        self.next_permission += 1;
        let question = match described.kind {
            ToolKind::Bash => format!("Run {}?", first_line(&described.args, 120)),
            ToolKind::Edit | ToolKind::Write => format!("Allow {}?", described.title),
            _ => format!("Allow {}?", described.title),
        };
        self.pending.insert(
            id,
            PendingPermission {
                request_id,
                tool,
                input,
                suggestions: request.get("permission_suggestions").cloned().filter(Value::is_array),
            },
        );
        Parsed::one(Update::Push {
            event: AgentEvent::PermissionRequest {
                id,
                question,
                options: PERMISSION_OPTIONS.iter().map(|o| (*o).to_owned()).collect(),
                answer: None,
            },
        })
    }

    fn control_cancel(&mut self, value: &Value) -> Parsed {
        let Some(request_id) = str_field(value, "request_id") else {
            return Parsed::none();
        };
        let id = self
            .pending
            .iter()
            .find(|(_, p)| p.request_id == request_id)
            .map(|(id, _)| *id);
        match id {
            Some(id) => {
                self.pending.remove(&id);
                Parsed::one(Update::Answered {
                    id,
                    answer: "Cancelled".into(),
                })
            }
            None => Parsed::none(),
        }
    }

    fn generic(&self, title: &str, value: &Value) -> AgentEvent {
        let id_value = value.to_string();
        AgentEvent::ToolCall {
            id: 0,
            kind: ToolKind::Other,
            title: truncate(title, 80),
            args: String::new(),
            status: ToolStatus::Ok,
            result: None,
            output: Some(truncate(&id_value, 2_000)),
            edit: None,
        }
    }

    fn rel(&self, path: &str) -> PathBuf {
        let path = Path::new(path);
        path.strip_prefix(&self.cwd).unwrap_or(path).to_path_buf()
    }

    fn describe(&self, name: &str, input: &Value) -> Described {
        let file = || {
            ["file_path", "path", "notebook_path"]
                .iter()
                .find_map(|key| str_field(input, key))
                .map(|p| self.rel(&p))
        };
        let file_name = |p: &Path| {
            p.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string())
        };
        match name {
            "Read" => {
                let path = file().unwrap_or_default();
                Described::new(ToolKind::Read, format!("Read {}", file_name(&path)), path.display().to_string())
            }
            "Edit" | "MultiEdit" | "NotebookEdit" => {
                let path = file().unwrap_or_default();
                let pairs: Vec<(String, String)> = if name == "MultiEdit" {
                    input
                        .get("edits")
                        .and_then(Value::as_array)
                        .map(|edits| {
                            edits
                                .iter()
                                .map(|e| {
                                    (
                                        str_field(e, "old_string").unwrap_or_default(),
                                        str_field(e, "new_string").unwrap_or_default(),
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                } else {
                    vec![(
                        str_field(input, "old_string").unwrap_or_default(),
                        str_field(input, "new_string")
                            .or_else(|| str_field(input, "new_source"))
                            .unwrap_or_default(),
                    )]
                };
                let mut snippet = Vec::new();
                let (mut added, mut removed) = (0, 0);
                for (old, new) in &pairs {
                    removed += count_lines(old);
                    added += count_lines(new);
                    snippet.extend(old.lines().take(3).map(|l| DiffLine {
                        sign: '-',
                        text: truncate(l, 160),
                    }));
                    snippet.extend(new.lines().take(3).map(|l| DiffLine {
                        sign: '+',
                        text: truncate(l, 160),
                    }));
                }
                snippet.truncate(8);
                let mut d = Described::new(
                    ToolKind::Edit,
                    format!("Edit {}", file_name(&path)),
                    path.display().to_string(),
                );
                d.edit = Some(EditSummary {
                    path,
                    added,
                    removed,
                    snippet,
                });
                d
            }
            "Write" => {
                let path = file().unwrap_or_default();
                let content = str_field(input, "content").unwrap_or_default();
                let mut d = Described::new(
                    ToolKind::Write,
                    format!("Write {}", file_name(&path)),
                    path.display().to_string(),
                );
                d.edit = Some(EditSummary {
                    path,
                    added: count_lines(&content),
                    removed: 0,
                    snippet: content
                        .lines()
                        .take(6)
                        .map(|l| DiffLine {
                            sign: '+',
                            text: truncate(l, 160),
                        })
                        .collect(),
                });
                d
            }
            "Bash" | "BashOutput" => {
                let command = str_field(input, "command").unwrap_or_default();
                Described::new(
                    ToolKind::Bash,
                    format!("Bash {}", first_line(&command, 80)),
                    command,
                )
            }
            "Grep" | "Glob" => {
                let query = ["pattern", "query"]
                    .iter()
                    .find_map(|key| str_field(input, key))
                    .unwrap_or_default();
                Described::new(ToolKind::Search, format!("{name} {}", truncate(&query, 80)), query)
            }
            "WebFetch" | "WebSearch" => {
                let target = ["url", "query"]
                    .iter()
                    .find_map(|key| str_field(input, key))
                    .unwrap_or_default();
                let verb = if name == "WebFetch" { "Fetch" } else { "Search the web:" };
                Described::new(ToolKind::Other, format!("{verb} {}", truncate(&target, 80)), target)
            }
            "Task" | "Agent" => {
                let description = str_field(input, "description")
                    .or_else(|| str_field(input, "subagent_type"))
                    .unwrap_or_else(|| "subagent".into());
                let prompt = str_field(input, "prompt").unwrap_or_default();
                let mut d = Described::new(
                    ToolKind::Spawn,
                    format!("Task {}", truncate(&description, 80)),
                    prompt,
                );
                d.spawn_name = Some(truncate(&description, 60));
                d
            }
            other => {
                let target = file()
                    .map(|p| p.display().to_string())
                    .or_else(|| str_field(input, "command"))
                    .or_else(|| str_field(input, "description"))
                    .unwrap_or_default();
                let title = if target.is_empty() {
                    other.to_owned()
                } else {
                    format!("{other} {}", truncate(&target, 80))
                };
                Described::new(ToolKind::Other, title, target)
            }
        }
    }
}

struct Described {
    kind: ToolKind,
    title: String,
    args: String,
    edit: Option<EditSummary>,
    spawn_name: Option<String>,
}

impl Described {
    fn new(kind: ToolKind, title: String, args: String) -> Self {
        Self {
            kind,
            title,
            args,
            edit: None,
            spawn_name: None,
        }
    }
}

fn str_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn count_lines(text: &str) -> usize {
    if text.is_empty() { 0 } else { text.lines().count() }
}

/// Tool result content: a string or an array of `{type: "text", text}` blocks.
fn content_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .filter_map(|b| b.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter() -> ClaudeAdapter {
        ClaudeAdapter::new(Path::new("/wt"))
    }

    fn pushed(parsed: &Parsed) -> Vec<&AgentEvent> {
        parsed
            .updates
            .iter()
            .filter_map(|u| match u {
                Update::Push { event } => Some(event),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn init_gives_session_and_model() {
        let p = adapter().parse_line(
            r#"{"type":"system","subtype":"init","session_id":"abc","model":"sonnet","tools":["Read"],"extra":{"x":1}}"#,
        );
        assert_eq!(
            p.updates,
            [Update::Session {
                id: "abc".into(),
                model: Some("sonnet".into())
            }]
        );
    }

    #[test]
    fn tools_map_to_kinds_and_results_find_their_call() {
        let mut a = adapter();
        let p = a.parse_line(
            r#"{"type":"assistant","message":{"content":[
              {"type":"text","text":"Looking."},
              {"type":"thinking","thinking":"hmm"},
              {"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/wt/src/a.rs"}},
              {"type":"tool_use","id":"t2","name":"Edit","input":{"file_path":"/wt/src/a.rs","old_string":"a\nb","new_string":"c"}},
              {"type":"tool_use","id":"t3","name":"Task","input":{"description":"Find uses","prompt":"go"}},
              {"type":"tool_use","id":"t4","name":"Grep","input":{"pattern":"foo"}},
              {"type":"tool_use","id":"t5","name":"Mystery","input":{"x":1}}
            ],"usage":{"output_tokens":12}}}"#,
        );
        let events = pushed(&p);
        assert!(matches!(events[0], AgentEvent::AssistantText(t) if t == "Looking."));
        assert!(matches!(events[1], AgentEvent::Thinking(_)));
        let kinds: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                AgentEvent::ToolCall { kind, .. } => Some(*kind),
                _ => None,
            })
            .collect();
        assert_eq!(
            kinds,
            [
                ToolKind::Read,
                ToolKind::Edit,
                ToolKind::Spawn,
                ToolKind::Search,
                ToolKind::Other
            ]
        );
        assert!(events.iter().any(|e| matches!(e, AgentEvent::SubagentSpawned { name, .. } if name == "Find uses")));
        let AgentEvent::ToolCall { title, edit, .. } = events[3] else {
            panic!("edit row");
        };
        assert_eq!(title, "Edit a.rs");
        let edit = edit.as_ref().unwrap();
        assert_eq!((edit.added, edit.removed), (1, 2));
        assert_eq!(edit.path, PathBuf::from("src/a.rs"));
        assert!(p.updates.contains(&Update::Tokens { add: 12 }));

        let done = a.parse_line(
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":[{"type":"text","text":"fn main() {}\nmore"}]},
               {"type":"tool_result","tool_use_id":"nope","content":"x"},
               {"type":"tool_result","tool_use_id":"t4","is_error":true,"content":"bad regex"}]}}"#,
        );
        assert_eq!(done.updates.len(), 2);
        assert!(matches!(&done.updates[0], Update::ToolResult { call: 1, status: ToolStatus::Ok, result: Some(r), .. } if r == "fn main() {}"));
        assert!(matches!(&done.updates[1], Update::ToolResult { status: ToolStatus::Error, .. }));
    }

    #[test]
    fn result_carries_cost_usage_duration_and_errors() {
        let mut a = adapter();
        a.parse_line(r#"{"type":"assistant","message":{"content":[],"usage":{"output_tokens":10}}}"#);
        let ok = a.parse_line(
            r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":4200,"total_cost_usd":0.05,"result":"done","usage":{"input_tokens":100,"output_tokens":30}}"#,
        );
        assert!(ok.updates.contains(&Update::Tokens { add: 120 }));
        assert!(ok.updates.contains(&Update::Turn {
            ok: true,
            duration_ms: Some(4200),
            cost_usd: Some(0.05),
            message: None
        }));
        let bad = a.parse_line(
            r#"{"type":"result","subtype":"error_during_execution","is_error":true,"errors":["no key"]}"#,
        );
        assert!(bad.updates.iter().any(|u| matches!(u, Update::Turn { ok: false, message: Some(m), .. } if m == "no key")));
    }

    #[test]
    fn permission_round_trip_builds_the_control_response() {
        let mut a = adapter();
        let p = a.parse_line(
            r#"{"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"cargo test"}}}"#,
        );
        let AgentEvent::PermissionRequest { id, question, options, answer } = pushed(&p)[0].clone() else {
            panic!("permission row");
        };
        assert_eq!(question, "Run cargo test?");
        assert_eq!(options.len(), 3);
        assert!(answer.is_none());
        let (line, update) = a.answer(id, Choice::AllowOnce).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["type"], "control_response");
        assert_eq!(v["response"]["request_id"], "r1");
        assert_eq!(v["response"]["response"]["behavior"], "allow");
        assert_eq!(v["response"]["response"]["updatedInput"]["command"], "cargo test");
        assert_eq!(update, Update::Answered { id, answer: "Allow once".into() });
        assert!(a.answer(id, Choice::Deny).is_none(), "answered once");

        let p = a.parse_line(
            r#"{"type":"control_request","request_id":"r2","request":{"subtype":"can_use_tool","tool_name":"Edit","input":{"file_path":"/wt/x"}}}"#,
        );
        let AgentEvent::PermissionRequest { id, .. } = pushed(&p)[0].clone() else {
            panic!();
        };
        let (line, _) = a.answer(id, Choice::AlwaysAllow).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["response"]["response"]["updatedPermissions"][0]["rules"][0]["toolName"], "Edit");
        let p = a.parse_line(r#"{"type":"control_request","request_id":"r3","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{}}}"#);
        let AgentEvent::PermissionRequest { id, .. } = pushed(&p)[0].clone() else {
            panic!();
        };
        let (line, _) = a.answer(id, Choice::Deny).unwrap();
        assert!(line.contains("\"deny\""));
    }

    #[test]
    fn unsupported_control_requests_get_an_error_reply_and_cancels_close_cards() {
        let mut a = adapter();
        let p = a.parse_line(r#"{"type":"control_request","request_id":"h1","request":{"subtype":"hook_callback"}}"#);
        assert_eq!(p.replies.len(), 1);
        assert!(p.replies[0].contains("\"error\""));
        a.parse_line(r#"{"type":"control_request","request_id":"r9","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{}}}"#);
        let c = a.parse_line(r#"{"type":"control_cancel_request","request_id":"r9"}"#);
        assert!(matches!(&c.updates[0], Update::Answered { answer, .. } if answer == "Cancelled"));
    }

    #[test]
    fn unknown_and_malformed_lines_never_panic() {
        let mut a = adapter();
        for line in [
            "",
            "not json",
            "[]",
            "null",
            r#"{"type":5}"#,
            r#"{"type":"assistant"}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use"}]}}"#,
            r#"{"type":"assistant","message":{"content":[7,null,{"type":"text","text":5}]}}"#,
            r#"{"type":"user","message":{"content":"plain"}}"#,
            r#"{"type":"result"}"#,
            r#"{"type":"control_request"}"#,
            r#"{"type":"system","subtype":"hook_started"}"#,
        ] {
            let _ = a.parse_line(line);
        }
        let p = a.parse_line(r#"{"type":"brand_new_thing","x":1}"#);
        assert!(matches!(
            pushed(&p)[0],
            AgentEvent::ToolCall { kind: ToolKind::Other, title, .. } if title == "Event brand_new_thing"
        ));
        assert!(a.parse_line(r#"{"type":"rate_limit_event"}"#).updates.is_empty());
    }

    #[test]
    fn user_messages_are_stream_json() {
        let v: Value = serde_json::from_str(&ClaudeAdapter::user_message("hi \"there\"")).unwrap();
        assert_eq!(v["type"], "user");
        assert_eq!(v["message"]["content"][0]["text"], "hi \"there\"");
    }
}

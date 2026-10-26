//! Codex adapter: `codex exec --json` (and `codex exec resume <thread> --json`).
//!
//! Sources. [DOC] `docs/agents/agent-integrations.md` section 2 documents `codex exec` as the
//! non-interactive form, `codex exec resume`, `-c key=value` overrides, `--sandbox` values and
//! the `approval_policy` / `sandbox_mode` config keys. It does **not** describe the JSONL event
//! shapes, so all of the following is [INFER] from the public Codex documentation of
//! `codex exec --json` and must be confirmed on a real install: events `thread.started`
//! (`thread_id`), `turn.started`, `item.started` / `item.updated` / `item.completed` with an
//! `item` of type `agent_message`, `reasoning`, `command_execution`, `file_change`,
//! `mcp_tool_call`, `web_search`, `todo_list` or `error`, then `turn.completed` (`usage`),
//! `turn.failed` (`error.message`) and a top-level `error`.
//!
//! `codex exec` never asks for approvals, so there is no permission round-trip for Codex:
//! the sandbox decides. Parsing is defensive: unknown fields are ignored, unknown item types
//! become a generic tool row and unknown event types a generic `Other` row.

use crate::adapter::Parsed;
use crate::events::{AgentEvent, EditSummary, ToolKind, ToolStatus};
use crate::update::{Update, first_line, truncate};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

const MAX_OUTPUT: usize = 20_000;

pub struct CodexAdapter {
    next_call: u32,
    /// Item id -> tool call id of rows started but not completed.
    open: HashMap<String, u32>,
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexAdapter {
    pub fn new() -> Self {
        Self {
            next_call: 1,
            open: HashMap::new(),
        }
    }


    pub fn parse_line(&mut self, line: &str) -> Parsed {
        let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
            return Parsed::none();
        };
        let Some(kind) = value.get("type").and_then(Value::as_str) else {
            return Parsed::none();
        };
        match kind {
            "thread.started" => Parsed::one(Update::Session {
                id: str_field(&value, "thread_id").unwrap_or_default(),
                model: str_field(&value, "model"),
            }),
            "turn.started" => Parsed::none(),
            "item.started" | "item.updated" | "item.completed" => {
                let Some(item) = value.get("item") else {
                    return Parsed::none();
                };
                self.item(kind == "item.completed", kind == "item.started", item)
            }
            "turn.completed" => {
                let usage = value.get("usage");
                let count = |key: &str| {
                    usage
                        .and_then(|u| u.get(key))
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                };
                let total = (count("input_tokens") + count("output_tokens")).min(u64::from(u32::MAX))
                    as u32;
                let mut out = Parsed::none();
                if total > 0 {
                    out.updates.push(Update::Tokens { add: total });
                }
                out.updates.push(Update::Turn {
                    ok: true,
                    duration_ms: None,
                    cost_usd: None,
                    message: None,
                });
                out
            }
            "turn.failed" => Parsed::one(Update::Turn {
                ok: false,
                duration_ms: None,
                cost_usd: None,
                message: Some(
                    value
                        .pointer("/error/message")
                        .and_then(Value::as_str)
                        .or_else(|| value.get("message").and_then(Value::as_str))
                        .unwrap_or("Codex reported a failed turn")
                        .to_owned(),
                ),
            }),
            "error" => Parsed::one(Update::Push {
                event: AgentEvent::Error(
                    str_field(&value, "message").unwrap_or_else(|| "Codex reported an error".into()),
                ),
            }),
            other => Parsed::one(Update::Push {
                event: self.generic(&format!("Event {other}"), &value),
            }),
        }
    }

    fn item(&mut self, completed: bool, started: bool, item: &Value) -> Parsed {
        let kind = item.get("type").and_then(Value::as_str).unwrap_or("");
        let item_id = str_field(item, "id").unwrap_or_default();
        if kind.is_empty() {
            return Parsed::none();
        }
        match kind {
            "agent_message" if completed => text_event(item, AgentEvent::AssistantText),
            "reasoning" if completed => text_event(item, AgentEvent::Thinking),
            "agent_message" | "reasoning" | "todo_list" => Parsed::none(),
            "error" => Parsed::one(Update::Push {
                event: AgentEvent::Error(
                    str_field(item, "message").unwrap_or_else(|| "Codex reported an error".into()),
                ),
            }),
            "command_execution" => {
                let command = str_field(item, "command").unwrap_or_default();
                let output = str_field(item, "aggregated_output").unwrap_or_default();
                let exit = item.get("exit_code").and_then(Value::as_i64);
                let status = item.get("status").and_then(Value::as_str).unwrap_or("");
                let failed = status == "failed" || exit.is_some_and(|code| code != 0);
                self.tool(
                    &item_id,
                    started || !completed,
                    completed,
                    ToolKind::Bash,
                    format!("Bash {}", first_line(&command, 80)),
                    command,
                    None,
                    failed,
                    if completed {
                        Some(match exit {
                            Some(code) if code != 0 => format!("exit {code}"),
                            _ if !output.trim().is_empty() => first_line(&output, 80),
                            _ => "ok".into(),
                        })
                    } else {
                        None
                    },
                    (!output.is_empty()).then(|| truncate(&output, MAX_OUTPUT)),
                )
            }
            "file_change" => {
                let changes: Vec<(String, String)> = item
                    .get("changes")
                    .and_then(Value::as_array)
                    .map(|changes| {
                        changes
                            .iter()
                            .map(|c| {
                                (
                                    str_field(c, "path").unwrap_or_default(),
                                    str_field(c, "kind").unwrap_or_else(|| "update".into()),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let first = changes.first().cloned().unwrap_or_default();
                let all_new = !changes.is_empty() && changes.iter().all(|(_, k)| k == "add");
                let name = std::path::Path::new(&first.0)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| first.0.clone());
                let more = changes.len().saturating_sub(1);
                let title = format!(
                    "{} {}{}",
                    if all_new { "Write" } else { "Edit" },
                    name,
                    if more > 0 {
                        format!(" (+{more} more)")
                    } else {
                        String::new()
                    }
                );
                let status = item.get("status").and_then(Value::as_str).unwrap_or("");
                let edit = (!first.0.is_empty()).then(|| EditSummary {
                    path: PathBuf::from(&first.0),
                    added: 0,
                    removed: 0,
                    snippet: Vec::new(),
                });
                self.tool(
                    &item_id,
                    started || !completed,
                    completed,
                    if all_new { ToolKind::Write } else { ToolKind::Edit },
                    title,
                    changes
                        .iter()
                        .map(|(p, _)| p.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    edit,
                    status == "failed",
                    completed.then(|| plural_files(changes.len())),
                    None,
                )
            }
            "mcp_tool_call" => {
                let server = str_field(item, "server").unwrap_or_default();
                let tool = str_field(item, "tool").unwrap_or_default();
                let status = item.get("status").and_then(Value::as_str).unwrap_or("");
                let args = item
                    .get("arguments")
                    .map(|a| truncate(&a.to_string(), 500))
                    .unwrap_or_default();
                self.tool(
                    &item_id,
                    started || !completed,
                    completed,
                    ToolKind::Other,
                    format!("{server}.{tool}"),
                    args,
                    None,
                    status == "failed" || item.get("error").is_some_and(|e| !e.is_null()),
                    None,
                    item.get("result").filter(|r| !r.is_null()).map(|r| truncate(&r.to_string(), MAX_OUTPUT)),
                )
            }
            "web_search" => {
                let query = str_field(item, "query").unwrap_or_default();
                self.tool(
                    &item_id,
                    started || !completed,
                    completed,
                    ToolKind::Search,
                    format!("Search the web: {}", truncate(&query, 80)),
                    query,
                    None,
                    false,
                    None,
                    None,
                )
            }
            other => {
                if completed {
                    Parsed::one(Update::Push {
                        event: self.generic(&format!("Item {other}"), item),
                    })
                } else {
                    Parsed::none()
                }
            }
        }
    }

    /// Starts a row, completes it, or both when only the completion is seen.
    #[allow(clippy::too_many_arguments)]
    fn tool(
        &mut self,
        item_id: &str,
        may_start: bool,
        completed: bool,
        kind: ToolKind,
        title: String,
        args: String,
        edit: Option<EditSummary>,
        failed: bool,
        result: Option<String>,
        output: Option<String>,
    ) -> Parsed {
        let mut out = Parsed::none();
        let known = self.open.get(item_id).copied();
        let call = match known {
            Some(call) => call,
            None => {
                if !may_start && !completed {
                    return out;
                }
                let call = self.next_call;
                self.next_call += 1;
                out.updates.push(Update::Push {
                    event: AgentEvent::ToolCall {
                        id: call,
                        kind,
                        title,
                        args,
                        status: ToolStatus::Running,
                        result: None,
                        output: None,
                        edit,
                    },
                });
                if !completed {
                    self.open.insert(item_id.to_owned(), call);
                }
                call
            }
        };
        if completed {
            self.open.remove(item_id);
            out.updates.push(Update::ToolResult {
                call,
                status: if failed {
                    ToolStatus::Error
                } else {
                    ToolStatus::Ok
                },
                result,
                output,
            });
        }
        out
    }

    fn generic(&self, title: &str, value: &Value) -> AgentEvent {
        AgentEvent::ToolCall {
            id: 0,
            kind: ToolKind::Other,
            title: truncate(title, 80),
            args: String::new(),
            status: ToolStatus::Ok,
            result: None,
            output: Some(truncate(&value.to_string(), 2_000)),
            edit: None,
        }
    }

}

fn plural_files(n: usize) -> String {
    if n == 1 {
        "1 file".into()
    } else {
        format!("{n} files")
    }
}

fn text_event(item: &Value, make: fn(String) -> AgentEvent) -> Parsed {
    match str_field(item, "text").filter(|t| !t.trim().is_empty()) {
        Some(text) => Parsed::one(Update::Push { event: make(text) }),
        None => Parsed::none(),
    }
}

fn str_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(lines: &[&str]) -> Vec<Update> {
        let mut a = CodexAdapter::new();
        lines.iter().flat_map(|l| a.parse_line(l).updates).collect()
    }

    #[test]
    fn a_full_turn_maps_items_to_events() {
        let updates = run(&[
            r#"{"type":"thread.started","thread_id":"th-1"}"#,
            r#"{"type":"turn.started"}"#,
            r#"{"type":"item.completed","item":{"id":"i0","type":"reasoning","text":"plan"}}"#,
            r#"{"type":"item.started","item":{"id":"i1","type":"command_execution","command":"cargo test","aggregated_output":"","status":"in_progress"}}"#,
            r#"{"type":"item.completed","item":{"id":"i1","type":"command_execution","command":"cargo test","aggregated_output":"ok\n3 passed","exit_code":0,"status":"completed"}}"#,
            r#"{"type":"item.completed","item":{"id":"i2","type":"file_change","changes":[{"path":"src/a.rs","kind":"update"},{"path":"b.rs","kind":"add"}],"status":"completed"}}"#,
            r#"{"type":"item.completed","item":{"id":"i3","type":"agent_message","text":"Done."}}"#,
            r#"{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":50,"output_tokens":20}}"#,
        ]);
        assert_eq!(updates[0], Update::Session { id: "th-1".into(), model: None });
        assert!(matches!(&updates[1], Update::Push { event: AgentEvent::Thinking(t) } if t == "plan"));
        assert!(matches!(
            &updates[2],
            Update::Push { event: AgentEvent::ToolCall { id: 1, kind: ToolKind::Bash, status: ToolStatus::Running, .. } }
        ));
        assert!(matches!(
            &updates[3],
            Update::ToolResult { call: 1, status: ToolStatus::Ok, result: Some(r), .. } if r == "ok"
        ));
        assert!(matches!(
            &updates[4],
            Update::Push { event: AgentEvent::ToolCall { id: 2, kind: ToolKind::Edit, title, .. } } if title == "Edit a.rs (+1 more)"
        ));
        assert!(matches!(&updates[5], Update::ToolResult { call: 2, status: ToolStatus::Ok, .. }));
        assert!(matches!(&updates[6], Update::Push { event: AgentEvent::AssistantText(t) } if t == "Done."));
        assert_eq!(updates[7], Update::Tokens { add: 120 });
        assert!(matches!(&updates[8], Update::Turn { ok: true, .. }));
    }

    #[test]
    fn failures_map_to_errors() {
        let updates = run(&[
            r#"{"type":"item.completed","item":{"id":"c","type":"command_execution","command":"false","aggregated_output":"","exit_code":1,"status":"failed"}}"#,
            r#"{"type":"error","message":"stream disconnected"}"#,
            r#"{"type":"turn.failed","error":{"message":"stream disconnected"}}"#,
        ]);
        assert!(matches!(&updates[1], Update::ToolResult { status: ToolStatus::Error, result: Some(r), .. } if r == "exit 1"));
        assert!(matches!(&updates[2], Update::Push { event: AgentEvent::Error(_) }));
        assert!(matches!(&updates[3], Update::Turn { ok: false, message: Some(m), .. } if m == "stream disconnected"));
    }

    #[test]
    fn unknown_items_and_events_are_generic_rows_and_garbage_is_ignored() {
        let updates = run(&[
            "",
            "garbage",
            "42",
            r#"{"type":"item.completed"}"#,
            r#"{"type":"item.completed","item":{"type":"hologram","id":"h"}}"#,
            r#"{"type":"item.completed","item":5}"#,
            r#"{"type":"surprise"}"#,
            r#"{"type":"item.started","item":{"type":"hologram"}}"#,
        ]);
        assert_eq!(updates.len(), 2);
        for u in updates {
            assert!(matches!(u, Update::Push { event: AgentEvent::ToolCall { kind: ToolKind::Other, .. } }));
        }
    }

    #[test]
    fn web_search_and_mcp_rows() {
        let updates = run(&[
            r#"{"type":"item.completed","item":{"id":"w","type":"web_search","query":"egui docs"}}"#,
            r#"{"type":"item.completed","item":{"id":"m","type":"mcp_tool_call","server":"git","tool":"log","arguments":{"n":3},"status":"completed","result":{"ok":true}}}"#,
        ]);
        assert!(matches!(&updates[0], Update::Push { event: AgentEvent::ToolCall { kind: ToolKind::Search, .. } }));
        assert!(matches!(&updates[2], Update::Push { event: AgentEvent::ToolCall { title, .. } } if title == "git.log"));
    }
}

//! Protocol-neutral transcript updates and the reducer that folds them into a transcript.
//!
//! Adapters ([`crate::claude`], [`crate::codex`]) turn one line of agent output into
//! [`Update`]s. The same updates are appended to the session's JSONL log
//! ([`crate::store`]), so a transcript restored after a restart is built by the same
//! reducer as a live one.

use crate::events::{AgentEvent, ToolStatus};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Update {
    /// The agent's own session (thread) id, needed to resume, and its model when known.
    Session { id: String, model: Option<String> },
    /// Appends a transcript row.
    Push { event: AgentEvent },
    /// Completes the tool call `call`.
    ToolResult {
        call: u32,
        status: ToolStatus,
        result: Option<String>,
        output: Option<String>,
    },
    /// A permission request was answered (or withdrawn by the agent).
    Answered { id: u32, answer: String },
    /// Tokens used since the last update.
    Tokens { add: u32 },
    /// The agent finished a turn.
    Turn {
        ok: bool,
        /// Wall time of the turn; the engine fills it in when the agent does not report it.
        duration_ms: Option<u64>,
        cost_usd: Option<f64>,
        /// The failure text for `ok == false`.
        message: Option<String>,
    },
    /// Counts of the changes made so far, patched into the latest turn result.
    TurnStats {
        files: usize,
        added: usize,
        removed: usize,
    },
}

/// The state a transcript accumulates from its updates.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Transcript {
    pub events: Vec<AgentEvent>,
    pub provider_session: Option<String>,
    pub model: Option<String>,
    pub tokens: u32,
    pub cost_usd: f64,
    /// Tokens of the turn in progress (shown in the turn result).
    turn_tokens: u32,
}

impl Transcript {
    pub fn apply(&mut self, update: &Update) {
        match update {
            Update::Session { id, model } => {
                if !id.is_empty() {
                    self.provider_session = Some(id.clone());
                }
                if model.as_ref().is_some_and(|m| !m.is_empty()) {
                    self.model = model.clone();
                }
            }
            Update::Push { event } => self.events.push(event.clone()),
            Update::ToolResult {
                call,
                status,
                result,
                output,
            } => {
                for event in self.events.iter_mut().rev() {
                    if let AgentEvent::ToolCall {
                        id,
                        status: s,
                        result: r,
                        output: o,
                        ..
                    } = event
                        && id == call
                    {
                        *s = *status;
                        *r = result.clone();
                        *o = output.clone();
                        break;
                    }
                }
            }
            Update::Answered { id, answer } => {
                for event in self.events.iter_mut().rev() {
                    if let AgentEvent::PermissionRequest {
                        id: i, answer: a, ..
                    } = event
                        && i == id
                        && a.is_none()
                    {
                        *a = Some(answer.clone());
                        break;
                    }
                }
            }
            Update::Tokens { add } => {
                self.tokens = self.tokens.saturating_add(*add);
                self.turn_tokens = self.turn_tokens.saturating_add(*add);
            }
            Update::Turn {
                ok,
                duration_ms,
                cost_usd,
                message,
            } => {
                self.cost_usd += cost_usd.unwrap_or(0.0);
                if !ok
                    && let Some(text) = message.as_ref().filter(|m| !m.trim().is_empty())
                {
                    let same = matches!(self.events.last(), Some(AgentEvent::Error(last)) if last == text);
                    if !same {
                        self.events.push(AgentEvent::Error(text.clone()));
                    }
                }
                if *ok {
                    self.events.push(AgentEvent::TurnResult {
                        files: 0,
                        added: 0,
                        removed: 0,
                        duration: Duration::from_millis(duration_ms.unwrap_or(0)),
                        tokens: self.turn_tokens,
                    });
                }
                self.turn_tokens = 0;
            }
            Update::TurnStats {
                files,
                added,
                removed,
            } => {
                if let Some(AgentEvent::TurnResult {
                    files: f,
                    added: a,
                    removed: r,
                    ..
                }) = self
                    .events
                    .iter_mut()
                    .rev()
                    .find(|e| matches!(e, AgentEvent::TurnResult { .. }))
                {
                    *f = *files;
                    *a = *added;
                    *r = *removed;
                }
            }
        }
    }

    /// The open permission request, if any.
    pub fn open_permission(&self) -> Option<u32> {
        self.events.iter().rev().find_map(|event| match event {
            AgentEvent::PermissionRequest {
                id, answer: None, ..
            } => Some(*id),
            _ => None,
        })
    }
}

/// `text` cut to `max` characters (on a character boundary) with an ellipsis.
pub fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

/// The first non-empty line of `text`, truncated.
pub fn first_line(text: &str, max: usize) -> String {
    truncate(
        text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or(""),
        max,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::ToolKind;

    fn call(id: u32) -> Update {
        Update::Push {
            event: AgentEvent::ToolCall {
                id,
                kind: ToolKind::Bash,
                title: "Bash ls".into(),
                args: "ls".into(),
                status: ToolStatus::Running,
                result: None,
                output: None,
                edit: None,
            },
        }
    }

    #[test]
    fn tool_results_find_their_call_and_unknown_ids_are_ignored() {
        let mut t = Transcript::default();
        t.apply(&call(1));
        t.apply(&call(2));
        t.apply(&Update::ToolResult {
            call: 1,
            status: ToolStatus::Ok,
            result: Some("done".into()),
            output: None,
        });
        t.apply(&Update::ToolResult {
            call: 99,
            status: ToolStatus::Error,
            result: None,
            output: None,
        });
        let statuses: Vec<_> = t
            .events
            .iter()
            .filter_map(|e| match e {
                AgentEvent::ToolCall { status, .. } => Some(*status),
                _ => None,
            })
            .collect();
        assert_eq!(statuses, [ToolStatus::Ok, ToolStatus::Running]);
    }

    #[test]
    fn a_turn_collects_tokens_and_takes_stats_afterwards() {
        let mut t = Transcript::default();
        t.apply(&Update::Tokens { add: 40 });
        t.apply(&Update::Turn {
            ok: true,
            duration_ms: Some(1500),
            cost_usd: Some(0.25),
            message: None,
        });
        t.apply(&Update::TurnStats {
            files: 2,
            added: 7,
            removed: 1,
        });
        assert_eq!(
            t.events.last(),
            Some(&AgentEvent::TurnResult {
                files: 2,
                added: 7,
                removed: 1,
                duration: Duration::from_millis(1500),
                tokens: 40
            })
        );
        assert_eq!(t.tokens, 40);
        assert!((t.cost_usd - 0.25).abs() < 1e-9);
        t.apply(&Update::Tokens { add: 5 });
        t.apply(&Update::Turn {
            ok: true,
            duration_ms: None,
            cost_usd: None,
            message: None,
        });
        assert!(matches!(
            t.events.last(),
            Some(AgentEvent::TurnResult { tokens: 5, .. })
        ));
        assert_eq!(t.tokens, 45);
    }

    #[test]
    fn failed_turns_report_the_error_once() {
        let mut t = Transcript::default();
        t.apply(&Update::Push {
            event: AgentEvent::Error("boom".into()),
        });
        t.apply(&Update::Turn {
            ok: false,
            duration_ms: None,
            cost_usd: None,
            message: Some("boom".into()),
        });
        assert_eq!(t.events.len(), 1);
    }

    #[test]
    fn answers_close_the_open_permission() {
        let mut t = Transcript::default();
        t.apply(&Update::Push {
            event: AgentEvent::PermissionRequest {
                id: 3,
                question: "Run ls?".into(),
                options: vec![],
                answer: None,
            },
        });
        assert_eq!(t.open_permission(), Some(3));
        t.apply(&Update::Answered {
            id: 3,
            answer: "Allow once".into(),
        });
        assert_eq!(t.open_permission(), None);
    }

    #[test]
    fn truncation_respects_characters() {
        assert_eq!(truncate("héllo wörld", 6), "héllo\u{2026}");
        assert_eq!(truncate("ab", 6), "ab");
        assert_eq!(first_line("\n  \n hi there \nx", 20), "hi there");
    }
}

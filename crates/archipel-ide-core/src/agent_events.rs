//! The structured transcript of an agent session.
//!
//! Real agents report typed events: Claude Code with `--output-format stream-json`
//! (and `--input-format stream-json` for follow-ups), Codex with `codex exec --json` or
//! its app-server protocol. A session stores them as a `Vec<AgentEvent>` and the Agents
//! mode UI renders each one as a native row. Agents without such a protocol keep only the
//! terminal. No toolkit types here.

use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Read,
    Edit,
    Write,
    Bash,
    Search,
    Spawn,
    Other,
}

impl ToolKind {
    pub fn label(self) -> &'static str {
        match self {
            ToolKind::Read => "Read",
            ToolKind::Edit => "Edit",
            ToolKind::Write => "Write",
            ToolKind::Bash => "Bash",
            ToolKind::Search => "Search",
            ToolKind::Spawn => "Spawn",
            ToolKind::Other => "Tool",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Ok,
    Error,
}

/// One line of a mini diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    /// `'+'`, `'-'` or `' '`.
    pub sign: char,
    pub text: String,
}

/// What an edit tool call changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditSummary {
    pub path: PathBuf,
    pub added: usize,
    pub removed: usize,
    pub snippet: Vec<DiffLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentEvent {
    UserPrompt(String),
    /// Markdown.
    AssistantText(String),
    /// Collapsed by default.
    Thinking(String),
    ToolCall {
        /// Stable within the session; a later update finds the call by it.
        id: u32,
        kind: ToolKind,
        /// What the row says, e.g. `Read App.java`.
        title: String,
        /// The full target: a path, a command, a query.
        args: String,
        status: ToolStatus,
        /// Short outcome shown on the row, e.g. `212 passed`.
        result: Option<String>,
        /// Collapsed output.
        output: Option<String>,
        edit: Option<EditSummary>,
    },
    PermissionRequest {
        id: u32,
        question: String,
        options: Vec<String>,
        /// The option chosen, once answered.
        answer: Option<String>,
    },
    SubagentSpawned {
        child: u32,
        name: String,
    },
    TurnResult {
        files: usize,
        added: usize,
        removed: usize,
        duration: Duration,
        tokens: u32,
    },
    Error(String),
}

/// The options every permission card offers, in order.
pub const PERMISSION_OPTIONS: [&str; 3] = ["Allow once", "Always allow", "Deny"];

/// The input an agent's prompt understands for the permission option at `index`
/// (`y`, `a` for always, `n`).
pub fn permission_input(index: usize) -> &'static str {
    match index {
        0 => "y",
        1 => "a",
        _ => "n",
    }
}

impl AgentEvent {
    /// A permission request that still waits for an answer.
    pub fn is_open_question(&self) -> bool {
        matches!(self, AgentEvent::PermissionRequest { answer: None, .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_options_map_to_prompt_keys() {
        assert_eq!(permission_input(0), "y");
        assert_eq!(permission_input(1), "a");
        assert_eq!(permission_input(2), "n");
        assert_eq!(PERMISSION_OPTIONS.len(), 3);
    }

    #[test]
    fn only_unanswered_requests_are_open() {
        let open = AgentEvent::PermissionRequest {
            id: 1,
            question: "?".into(),
            options: vec![],
            answer: None,
        };
        assert!(open.is_open_question());
        let closed = AgentEvent::PermissionRequest {
            id: 1,
            question: "?".into(),
            options: vec![],
            answer: Some("Deny".into()),
        };
        assert!(!closed.is_open_question());
        assert!(!AgentEvent::Error("x".into()).is_open_question());
    }
}

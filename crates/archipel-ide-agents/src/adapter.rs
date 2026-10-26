//! What every protocol adapter returns for one line of agent output.

use crate::update::Update;

/// The result of parsing one stdout line.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Parsed {
    /// Transcript updates, in order.
    pub updates: Vec<Update>,
    /// Lines the agent expects on its stdin in response (control replies).
    pub replies: Vec<String>,
}

impl Parsed {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn one(update: Update) -> Self {
        Self {
            updates: vec![update],
            replies: Vec::new(),
        }
    }

    pub fn is_turn_end(&self) -> bool {
        self.updates
            .iter()
            .any(|update| matches!(update, Update::Turn { .. }))
    }
}

/// How the user answered a permission card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    AllowOnce,
    AlwaysAllow,
    Deny,
}

impl Choice {
    /// From the keys the UI sends (`y`, `a`, `n`; see `events::permission_input`).
    pub fn from_input(input: &str) -> Option<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "y" => Some(Choice::AllowOnce),
            "a" => Some(Choice::AlwaysAllow),
            "n" => Some(Choice::Deny),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Choice::AllowOnce => crate::events::PERMISSION_OPTIONS[0],
            Choice::AlwaysAllow => crate::events::PERMISSION_OPTIONS[1],
            Choice::Deny => crate::events::PERMISSION_OPTIONS[2],
        }
    }
}

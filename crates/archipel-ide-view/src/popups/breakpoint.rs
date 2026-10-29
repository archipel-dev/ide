//! The breakpoint properties form (M4): its title, its three fields (condition, log message
//! and hit count) and what Done, Remove and closing do. Both frontends keep the edit buffers
//! in a [`BreakpointForm`] and draw the fields; the commands are the view crate's.
//! Spec: TUI-GUI-140 (breakpoint form), UI-DBG breakpoint properties.

use crate::dispatch::Effect;
use archipel_ide_core::IdeState;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// One field of the form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormField {
    /// Stable id (`condition`, `logMessage`, `hitCondition`: the parameters of
    /// `debug.setBreakpointOptions`).
    pub id: &'static str,
    /// The label.
    pub label: &'static str,
    /// The hint shown while the field is empty.
    pub hint: &'static str,
}

/// The fields, top to bottom.
pub const FIELDS: [FormField; 3] = [
    FormField {
        id: "condition",
        label: "Condition",
        hint: "e.g. i == 3",
    },
    FormField {
        id: "logMessage",
        label: "Log message",
        hint: "Logpoint: print, {expr} interpolates",
    },
    FormField {
        id: "hitCondition",
        label: "Hit count",
        hint: "e.g. 3 or >= 2",
    },
];

/// The label of the default button.
pub const DONE: &str = "Done";
/// The label of the button that removes the breakpoint.
pub const REMOVE: &str = "Remove Breakpoint";

/// The breakpoint whose properties are open, if any.
pub fn editing(state: &IdeState) -> Option<(PathBuf, usize)> {
    state.debugger().state().editing_breakpoint.clone()
}

/// The edit buffers of the form for one breakpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BreakpointForm {
    /// The project-relative file.
    pub path: PathBuf,
    /// The zero-based line.
    pub line: usize,
    /// The condition.
    pub condition: String,
    /// The log message of a logpoint.
    pub log_message: String,
    /// The hit count condition.
    pub hit_condition: String,
}

impl BreakpointForm {
    /// The form of the breakpoint at `line` of `path`, filled with its options.
    pub fn load(state: &IdeState, path: &Path, line: usize) -> Self {
        let options = state.breakpoint_options(path, line).unwrap_or_default();
        Self {
            path: path.to_owned(),
            line,
            condition: options.condition.unwrap_or_default(),
            log_message: options.log_message.unwrap_or_default(),
            hit_condition: options.hit_condition.unwrap_or_default(),
        }
    }

    /// The title: `Line Breakpoint — Main.java:12`.
    pub fn title(&self) -> String {
        format!(
            "Line Breakpoint — {}:{}",
            self.path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            self.line + 1
        )
    }

    /// The buffer of field `id`.
    pub fn value(&self, id: &str) -> &str {
        match id {
            "condition" => &self.condition,
            "logMessage" => &self.log_message,
            _ => &self.hit_condition,
        }
    }

    /// Sets the buffer of field `id`.
    pub fn set(&mut self, id: &str, value: String) {
        match id {
            "condition" => self.condition = value,
            "logMessage" => self.log_message = value,
            _ => self.hit_condition = value,
        }
    }

    /// Done: store the options (empty ones clear them), then close the form.
    pub fn done(&self) -> Vec<Effect> {
        vec![
            command(
                "debug.setBreakpointOptions",
                json!({
                    "path": self.path,
                    "line": self.line,
                    "condition": self.condition,
                    "logMessage": self.log_message,
                    "hitCondition": self.hit_condition,
                }),
            ),
            close(),
        ]
    }

    /// Remove Breakpoint: toggle it off (its options go with it), then close the form.
    pub fn remove(&self) -> Vec<Effect> {
        vec![
            command(
                "debug.toggleBreakpoint",
                json!({"path": self.path, "line": self.line}),
            ),
            close(),
        ]
    }
}

/// Closing the form without a change (Escape).
pub fn close() -> Effect {
    command("debug.closeBreakpointEditor", Value::Null)
}

/// The effects of opening the form for `line` of `path` (the breakpoint is created when the
/// line has none, like the properties popup of the reference IDE).
pub fn open(path: &Path, line: usize) -> Vec<Effect> {
    vec![command(
        "debug.editBreakpoint",
        json!({"path": path, "line": line}),
    )]
}

/// The effects of a click on a line number: toggle the breakpoint there.
pub fn toggle(path: &Path, line: usize) -> Vec<Effect> {
    vec![command(
        "debug.toggleBreakpoint",
        json!({"path": path, "line": line}),
    )]
}

fn command(id: &str, params: Value) -> Effect {
    Effect::Command {
        id: id.to_owned(),
        params,
    }
}

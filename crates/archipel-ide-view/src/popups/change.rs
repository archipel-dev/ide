//! The popup of a gutter change bar (M2): a toolbar row (previous and next change, Rollback,
//! Show Diff, Close) and the `HEAD` version of the changed lines. Both frontends draw this
//! model and apply [`ChangePopup::step`].
//! Spec: TUI-GUI-091, TUI-GUI-140 (change popup).

use crate::ViewCx;
use crate::dispatch::Effect;
use archipel_ide_git::Hunk;
use archipel_kit_model::Glyph;
use serde_json::json;

/// The most old lines the popup shows.
pub const MAX_OLD_LINES: usize = 20;

/// A button of the toolbar row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// Select the previous change (moves the caret there).
    Previous,
    /// Select the next change.
    Next,
    /// Restore the `HEAD` text of the change.
    Rollback,
    /// Open the diff of the change.
    ShowDiff,
    /// Close the popup.
    Close,
}

/// One button.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeTool {
    /// What it does.
    pub tool: Tool,
    /// The text of a text button (`Rollback`, `Show Diff`); `None` for an icon button.
    pub label: Option<&'static str>,
    /// The icon of an icon button.
    pub glyph: Option<Glyph>,
    /// The tooltip, with the shortcut.
    pub tooltip: String,
    /// Whether it can be used.
    pub enabled: bool,
    /// Buttons of one group sit together; groups have a gap between them.
    pub group: u8,
}

/// The model of an open change popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangePopup {
    /// Index of the hunk in the document's changes.
    pub hunk: usize,
    /// The new-side first line of the hunk (zero-based).
    pub line: usize,
    /// The toolbar row.
    pub tools: Vec<ChangeTool>,
    /// The old lines (at most [`MAX_OLD_LINES`], tabs as four blanks); empty for a pure
    /// insertion.
    pub old_lines: Vec<String>,
    /// How many old lines were left out.
    pub more: usize,
}

/// What a tool did.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    /// Commands to run.
    pub effects: Vec<Effect>,
    /// The hunk the popup shows next, when the selection moved.
    pub select: Option<usize>,
    /// The popup is done.
    pub close: bool,
}

/// The model of the popup of hunk `hunk` of the active document, `None` when there is no
/// such hunk (the document or its changes went away).
pub fn change_popup(cx: &ViewCx, hunk: usize) -> Option<ChangePopup> {
    let index = cx
        .state
        .active_index()
        .filter(|_| !cx.state.diff_active())?;
    let hunks = cx.state.document_changes(index);
    let current = hunks.get(hunk)?;
    let old_text = cx.state.hunk_base_text(index, current).unwrap_or_default();
    let all: Vec<String> = archipel_ide_git::split_lines(&old_text)
        .into_iter()
        .map(|line| line.replace('\t', "    "))
        .collect();
    let more = all.len().saturating_sub(MAX_OLD_LINES);
    let old_lines = if current.old.is_empty() {
        Vec::new()
    } else {
        all.into_iter().take(MAX_OLD_LINES).collect()
    };
    let notation = cx.caps.notation();
    let tip = |title: &str, action: &str| match cx.shortcut(action) {
        Some(shortcut) => format!("{title} ({})", shortcut.label(notation)),
        None => title.to_owned(),
    };
    let tools = vec![
        ChangeTool {
            tool: Tool::Previous,
            label: None,
            glyph: Some(Glyph::ChevronUp),
            tooltip: tip("Previous Change", "VcsShowPrevChangeMarker"),
            enabled: hunk > 0,
            group: 0,
        },
        ChangeTool {
            tool: Tool::Next,
            label: None,
            glyph: Some(Glyph::ChevronDown),
            tooltip: tip("Next Change", "VcsShowNextChangeMarker"),
            enabled: hunk + 1 < hunks.len(),
            group: 0,
        },
        ChangeTool {
            tool: Tool::Rollback,
            label: Some("Rollback"),
            glyph: None,
            tooltip: tip("Rollback Lines", "Vcs.RollbackChangedLines"),
            enabled: true,
            group: 1,
        },
        ChangeTool {
            tool: Tool::ShowDiff,
            label: Some("Show Diff"),
            glyph: None,
            // The terminal has no diff viewer yet (Wave 4 of the terminal plan).
            tooltip: if cx.caps.terminal {
                "Show Diff (not available in the terminal yet)".into()
            } else {
                "Show Diff".into()
            },
            enabled: !cx.caps.terminal,
            group: 1,
        },
        ChangeTool {
            tool: Tool::Close,
            label: None,
            glyph: Some(Glyph::Close),
            tooltip: "Close (Esc)".into(),
            enabled: true,
            group: 2,
        },
    ];
    Some(ChangePopup {
        hunk,
        line: current.new.start,
        tools,
        old_lines,
        more,
    })
}

/// The hunk that owns `line` (zero-based) of the active document: the one covering it, or a
/// deletion marker directly above it.
pub fn hunk_at(cx: &ViewCx, line: usize) -> Option<usize> {
    let index = cx.state.active_index()?;
    archipel_ide_core::hunk_at_line(&cx.state.document_changes(index), line)
}

impl ChangePopup {
    /// The button `tool`, if the row has it.
    pub fn tool(&self, tool: Tool) -> Option<&ChangeTool> {
        self.tools.iter().find(|candidate| candidate.tool == tool)
    }

    /// Uses `tool`: Previous and Next move the caret to that change and show it, Rollback and
    /// Show Diff act on this change and close, Close closes. A disabled tool does nothing.
    pub fn step(&self, cx: &ViewCx, tool: Tool) -> Step {
        let nothing = Step {
            effects: Vec::new(),
            select: None,
            close: false,
        };
        if !self.tool(tool).is_some_and(|tool| tool.enabled) {
            return nothing;
        }
        let line = json!({ "line": self.line });
        let hunks: Vec<Hunk> = cx
            .state
            .active_index()
            .map(|index| cx.state.document_changes(index))
            .unwrap_or_default();
        let command = |id: &str, params: serde_json::Value| Effect::Command {
            id: id.to_owned(),
            params,
        };
        match tool {
            Tool::Previous | Tool::Next => {
                let target = if tool == Tool::Previous {
                    self.hunk.checked_sub(1)
                } else {
                    Some(self.hunk + 1)
                };
                match target.and_then(|at| hunks.get(at).map(|hunk| (at, hunk))) {
                    Some((at, hunk)) => Step {
                        // Follow the selected change in the editor too.
                        effects: vec![command(
                            "editor.gotoLine",
                            json!({ "line": hunk.new.start + 1 }),
                        )],
                        select: Some(at),
                        close: false,
                    },
                    None => nothing,
                }
            }
            Tool::Rollback => Step {
                effects: vec![command("vcs.rollbackChange", line)],
                select: None,
                close: true,
            },
            Tool::ShowDiff => Step {
                effects: vec![command("vcs.showChangeDiff", line)],
                select: None,
                close: true,
            },
            Tool::Close => Step {
                effects: Vec::new(),
                select: None,
                close: true,
            },
        }
    }
}

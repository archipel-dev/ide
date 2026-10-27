//! The editor's input layer: what typing, Enter, Backspace, Tab, the caret keys and the editing
//! actions do to a document and its carets. It is the one place both frontends (the egui window
//! and the terminal UI) reach through typed commands (`editor.type`, `editor.action`, ...), so a
//! behavior fixed here is fixed everywhere.
//!
//! The functions work on a [`Document`] and the [`EditorOptions`] read from the settings; they
//! never touch the window. Behavior is specified by `docs/specs/editor/` (requirement ids
//! `ED-*`, executable cases in `docs/specs/editor/cases/`).

mod commands;
mod enter;
mod indent;
mod lang;
mod lines;
mod movement;
mod options;
mod paste;
mod typing;
mod words;

pub use commands::register;
pub use lang::{Family, Lang, Region, RegionKind, View};
pub(crate) use movement::shrink_selection;
pub use options::{EditorOptions, PasteMode, UnindentMode};

use crate::Document;
use archipel_ide_text::{Selection, TextChange, UndoKind};

/// Per-document input state: modes and the memory some actions need between key presses.
#[derive(Debug, Clone, Default)]
pub struct InputState {
    /// Overwrite mode (Insert key, ED-CARET-020).
    pub overwrite: bool,
    /// Sticky selection: plain moves extend the selection (ED-CARET-052).
    pub sticky: bool,
    /// The column Up and Down try to keep, valid while the carets are where the last vertical
    /// move left them (ED-CARET-003).
    goal: Option<Goal>,
    /// Closing brackets and quotes Tab may jump over (ED-TYPE-011, ED-TYPE-030).
    scopes: Vec<TabOutScope>,
    /// Lines the viewport shows (Page Up/Down, ED-CARET-017); the window keeps it current.
    pub page_lines: usize,
    /// First visible line (Move Caret to Page Top/Bottom); the window keeps it current.
    pub top_line: usize,
}

impl InputState {
    /// Forgets the remembered column (the carets were moved by something else).
    pub(crate) fn goal_reset(&mut self) {
        self.goal = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Goal {
    /// Where each caret was left by the last vertical move.
    heads: Vec<usize>,
    columns: Vec<usize>,
}

/// A bracket or quote pair inserted by typing: Tab at the caret just before `close` moves the
/// caret behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TabOutScope {
    /// Offset of the closing character.
    pub(crate) close: usize,
    pub(crate) closer: char,
}

/// The effect of one action on one caret: replace `start..end` by `text` and select
/// `select` (anchor, head), counted in characters from the start of the replacement.
/// The offsets may point past the end of `text` (the caret moves over text that stays).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaretEdit {
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub select: (usize, usize),
    /// A pair was inserted: its closing character is at this offset of `text`.
    pub scope: Option<(usize, char)>,
}

impl CaretEdit {
    pub fn new(start: usize, end: usize, text: impl Into<String>, caret: usize) -> Self {
        Self {
            start,
            end,
            text: text.into(),
            select: (caret, caret),
            scope: None,
        }
    }

    /// Moves the caret to `offset` (absolute, in the text before the edit) without changing text.
    pub fn move_to(from: usize, offset: usize) -> Self {
        // Expressed as an empty replacement at `from`; the selection is relative to it.
        Self {
            start: from,
            end: from,
            text: String::new(),
            select: (offset.wrapping_sub(from), offset.wrapping_sub(from)),
            scope: None,
        }
    }
}

/// Applies one [`CaretEdit`] per selection as one undo step. The edits are in the order of
/// `doc.selections` (ascending). Returns whether the text changed.
pub(crate) fn apply_caret_edits(
    doc: &mut Document,
    kind: UndoKind,
    edits: Vec<CaretEdit>,
) -> Result<bool, String> {
    let before = doc.selections.selections().to_vec();
    let ages = doc.selections.ages().to_vec();
    let revision = doc.text.revision();
    let changes: Vec<TextChange> = edits
        .iter()
        .map(|e| TextChange {
            start_char: e.start,
            end_char: e.end,
            inserted: e.text.clone(),
        })
        .collect();
    doc.text.begin_undo_step(kind, &before);
    if let Err(error) = doc.text.edit_many(changes) {
        doc.text.cancel_undo_step();
        return Err(error.to_string());
    }
    let mut shift: isize = 0;
    let mut next = Vec::with_capacity(edits.len());
    let mut scopes: Vec<(usize, char)> = Vec::new();
    for edit in &edits {
        let start = edit.start.saturating_add_signed(shift);
        let at = |relative: usize| start.wrapping_add(relative);
        next.push(Selection {
            anchor: at(edit.select.0),
            head: at(edit.select.1),
        });
        if let Some((offset, closer)) = edit.scope {
            scopes.push((start + offset, closer));
        }
        shift += edit.text.chars().count() as isize - (edit.end - edit.start) as isize;
    }
    // Existing tab-out scopes follow the edits; a scope whose closer was deleted is dropped when
    // it is used (the character check).
    let map = |position: usize| {
        let mut position = position;
        let mut delta: isize = 0;
        for edit in &edits {
            if edit.end <= position {
                delta += edit.text.chars().count() as isize - (edit.end - edit.start) as isize;
            } else if edit.start < position {
                position = edit.start;
                break;
            }
        }
        position.saturating_add_signed(delta)
    };
    for scope in &mut doc.input.scopes {
        scope.close = map(scope.close);
    }
    for (close, closer) in scopes {
        doc.input.scopes.push(TabOutScope { close, closer });
    }
    let len = doc.text.len_chars();
    for selection in &mut next {
        selection.anchor = selection.anchor.min(len);
        selection.head = selection.head.min(len);
    }
    doc.selections.set_all_aged(next, ages);
    let changed = doc.text.revision() != revision;
    if changed {
        doc.text.finish_undo_step(doc.selections.selections());
        let source = doc.text.text();
        doc.refresh_analysis(&source);
    } else {
        doc.text.cancel_undo_step();
    }
    Ok(changed)
}

/// Which side of an insertion a position at the insertion point moves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Bias {
    /// Stays before the inserted text (the start of a selection that keeps its indent).
    Left,
    /// Moves behind it (a caret moves with the text it is in front of).
    Right,
}

/// Where `position` of the old text is after `changes` (offsets in the old text).
pub(crate) fn map_through(changes: &[TextChange], position: usize, bias: Bias) -> usize {
    let mut delta: isize = 0;
    for change in changes {
        let inserted = change.inserted.chars().count() as isize;
        let removed = (change.end_char - change.start_char) as isize;
        if change.end_char < position
            || (change.end_char == position
                && (change.start_char < change.end_char || bias == Bias::Right))
        {
            delta += inserted - removed;
        } else if change.start_char < position && position < change.end_char {
            // Inside removed text: the start of the replacement.
            return (change.start_char as isize + delta + inserted) as usize;
        } else if change.start_char == position && change.end_char > position {
            break;
        }
    }
    position.saturating_add_signed(delta)
}

/// Applies `changes` as one undo step and sets the selections `select` computes from them.
pub(crate) fn apply_changes(
    doc: &mut Document,
    kind: UndoKind,
    changes: Vec<TextChange>,
    select: impl FnOnce(&[TextChange], &Document) -> Vec<Selection>,
) -> Result<bool, String> {
    let before = doc.selections.selections().to_vec();
    let revision = doc.text.revision();
    let next = select(&changes, doc);
    doc.text.begin_undo_step(kind, &before);
    if let Err(error) = doc.text.edit_many(changes) {
        doc.text.cancel_undo_step();
        return Err(error.to_string());
    }
    let len = doc.text.len_chars();
    let next: Vec<Selection> = next
        .into_iter()
        .map(|s| Selection {
            anchor: s.anchor.min(len),
            head: s.head.min(len),
        })
        .collect();
    if !next.is_empty() {
        doc.selections.set_all(next);
    }
    let changed = doc.text.revision() != revision;
    if changed {
        doc.text.finish_undo_step(doc.selections.selections());
        let source = doc.text.text();
        doc.refresh_analysis(&source);
    } else {
        doc.text.cancel_undo_step();
    }
    Ok(changed)
}

/// Runs an editing function as one recorded undo step: the selections before and after are
/// kept for Undo and Redo (ED-UNDO-012).
pub fn recorded<T>(doc: &mut Document, kind: UndoKind, run: impl FnOnce(&mut Document) -> T) -> T {
    let before = doc.selections.selections().to_vec();
    let revision = doc.text.revision();
    doc.text.begin_undo_step(kind, &before);
    let result = run(doc);
    if doc.text.revision() != revision {
        doc.text.finish_undo_step(doc.selections.selections());
    } else {
        doc.text.cancel_undo_step();
    }
    result
}

/// The view of `doc` the handlers read.
pub(crate) fn view_of(doc: &Document) -> View {
    View::new(&doc.text, Lang::of(&doc.path))
}

#[cfg(test)]
mod tests;

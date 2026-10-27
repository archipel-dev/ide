//! Inline AI completion in the editor: debounced requests while typing, ghost
//! text painting at the caret, and Tab / word / Esc handling.
//!
//! The LSP completion popup always wins: no request is made and no ghost text is
//! shown while it is open, and its Tab/Enter/Esc handling is untouched.

use crate::{IdeApp, theme};
use eframe::egui::{self, vec2};
use serde_json::Value;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Typing pause before a suggestion is requested.
const DEBOUNCE: Duration = Duration::from_millis(350);

#[derive(Default)]
pub struct InlineUi {
    /// Last seen (path, revision) of the active document.
    seen: Option<(PathBuf, u64)>,
    /// When the document last changed, if a request is still due.
    due: Option<Instant>,
}

/// Runs once per frame before shortcuts: keeps the suggestion in sync with the
/// document and starts a request after a typing pause.
pub fn tick(app: &mut IdeApp, ctx: &egui::Context) {
    app.state.ai_inline_sync();
    let Some(document) = app.state.active_doc() else {
        app.ai_ui.inline = InlineUi::default();
        return;
    };
    let key = (document.path.clone(), document.text.revision());
    let inline = &mut app.ai_ui.inline;
    if inline.seen.as_ref() != Some(&key) {
        // Only edits trigger suggestions (not opening a file or moving the caret).
        let edited = inline.seen.as_ref().is_some_and(|(path, _)| *path == key.0);
        inline.seen = Some(key);
        inline.due = edited.then(Instant::now);
    }
    let Some(changed) = inline.due else {
        return;
    };
    if !app.state.ai_inline_available() || !app.editor_focused(ctx) {
        app.ai_ui.inline.due = None;
        return;
    }
    let elapsed = changed.elapsed();
    if elapsed < DEBOUNCE {
        ctx.request_repaint_after(DEBOUNCE - elapsed);
        return;
    }
    app.ai_ui.inline.due = None;
    if app.state.ai().inline.is_none() {
        app.dispatch("ai.inline.request", Value::Null);
    }
}

/// Tab accepts, ⌥→ (Ctrl+→ off macOS) accepts a word, Esc dismisses. Must run
/// before the general shortcut handler so Tab does not indent.
pub fn handle_keys(app: &mut IdeApp, ctx: &egui::Context) {
    if app.state.ai().inline.is_none()
        || !app.editor_focused(ctx)
        || app
            .state
            .completion
            .as_ref()
            .is_some_and(|session| session.visible())
    {
        return;
    }
    let word = if cfg!(target_os = "macos") {
        egui::Modifiers::ALT
    } else {
        egui::Modifiers::COMMAND
    };
    let consume = |modifiers, key| ctx.input_mut(|input| input.consume_key(modifiers, key));
    if consume(egui::Modifiers::NONE, egui::Key::Tab) {
        app.dispatch("ai.inline.accept", Value::Null);
    } else if consume(word, egui::Key::ArrowRight) {
        app.dispatch("ai.inline.acceptWord", Value::Null);
    } else if consume(egui::Modifiers::NONE, egui::Key::Escape) {
        app.dispatch("ai.inline.dismiss", Value::Null);
        return;
    } else {
        return;
    }
    // Accepting is an edit, but not typing: it must not trigger a new request.
    if let Some(document) = app.state.active_doc() {
        app.ai_ui.inline.seen = Some((document.path.clone(), document.text.revision()));
        app.ai_ui.inline.due = None;
    }
}

/// Paints the suggestion as ghost text; called from the editor panel so it is
/// clipped to the editor island. Text after the caret on its line is redrawn after
/// the suggestion, as it will be once the suggestion is inserted.
pub fn paint(app: &mut IdeApp, ui: &egui::Ui) {
    app.state.ai_inline_sync();
    let (Some(suggestion), Some(caret_bottom)) = (&app.state.ai().inline, app.caret_screen_pos)
    else {
        return;
    };
    let Some(document) = app.state.active_doc() else {
        return;
    };
    if document.path != suggestion.path {
        return;
    }
    let row = crate::editor::ROW_HEIGHT;
    let font = egui::FontId::monospace(theme::EDITOR_FONT_SIZE);
    let char_width = ui.fonts_mut(|fonts| fonts.glyph_width(&font, 'm'));
    let line = document.text.char_to_line(suggestion.position);
    let line_start = document
        .text
        .line_to_char(line)
        .unwrap_or(suggestion.position);
    let column = suggestion.position.saturating_sub(line_start);
    let rest_of_line: String = document
        .text
        .line(line)
        .unwrap_or_default()
        .chars()
        .skip(column)
        .take_while(|c| *c != '\n' && *c != '\r')
        .collect();
    let width = |text: &str| text.chars().count() as f32 * char_width;
    let line_left = caret_bottom.x - column as f32 * char_width;
    let top = caret_bottom.y - row;
    let text_y =
        |index: usize| top + index as f32 * row + (row - theme::EDITOR_FONT_SIZE) / 2.0 - 1.0;
    let painter = ui.painter();
    let ghost = |pos: egui::Pos2, text: &str, color| {
        painter.text(pos, egui::Align2::LEFT_TOP, text, font.clone(), color);
    };
    let lines: Vec<&str> = suggestion.text.split('\n').collect();
    // Hide the displaced rest of the caret line; it is redrawn after the suggestion.
    if !rest_of_line.is_empty() {
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(caret_bottom.x + 1.5, top),
                vec2(width(&rest_of_line), row),
            ),
            0.0,
            theme::editor::CARET_ROW,
        );
    }
    ghost(
        egui::pos2(caret_bottom.x, text_y(0)),
        lines[0],
        *theme::MUTED,
    );
    let mut caret_line_end = caret_bottom.x + width(lines[0]);
    if lines.len() > 1 {
        // Following lines cover the code below while the suggestion is shown.
        let longest =
            lines[1..].iter().map(|l| width(l)).fold(0.0, f32::max) + width(&rest_of_line);
        let block = egui::Rect::from_min_size(
            egui::pos2(line_left, caret_bottom.y),
            vec2(
                (longest + 2.0 * char_width).max(ui.clip_rect().right() - line_left),
                (lines.len() - 1) as f32 * row,
            ),
        );
        painter.rect_filled(block, 0.0, theme::ISLAND);
        for (index, text) in lines.iter().enumerate().skip(1) {
            ghost(egui::pos2(line_left, text_y(index)), text, *theme::MUTED);
        }
        let last = lines.len() - 1;
        ghost(
            egui::pos2(line_left + width(lines[last]), text_y(last)),
            &rest_of_line,
            *theme::editor::TEXT,
        );
    } else {
        ghost(
            egui::pos2(caret_line_end, text_y(0)),
            &rest_of_line,
            *theme::editor::TEXT,
        );
        caret_line_end += width(&rest_of_line);
    }
    // Discoverability hint after the caret line.
    let hint = if cfg!(target_os = "macos") {
        "⇥ accept  ⌥→ word  ⎋ dismiss"
    } else {
        "Tab accept  Ctrl+→ word  Esc dismiss"
    };
    let hint_font = egui::FontId::proportional(theme::UI_FONT_SIZE - 2.0);
    let galley = painter.layout_no_wrap(hint.to_owned(), hint_font, *theme::TEXT_INFO);
    let pill = egui::Rect::from_min_size(
        egui::pos2(caret_line_end + 16.0, top + 3.0),
        galley.size() + vec2(10.0, 2.0),
    );
    painter.rect_filled(pill, 4.0, theme::FRAME);
    painter.galley(pill.min + vec2(5.0, 1.0), galley, *theme::TEXT_INFO);
}

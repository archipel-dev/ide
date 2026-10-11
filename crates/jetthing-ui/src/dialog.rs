//! The one shell every modal dialog uses (`docs/ux/interaction-spec.md` §2): dimmed inert
//! backdrop, centred placement, a size that fits the window with scrolling content and a
//! pinned button row, a title bar with a close button, one default button bound to Enter,
//! Escape = Cancel and the button order `[secondary…] … [Cancel] [OK]`.
//!
//! It is built from kit parts only (`Surface::Dialog`, `Button`, `IconButton`), because the
//! kit's own `Dialog` has no close button, scrolling body or several buttons yet (see "Kit
//! gaps" in the pull request that added this file).

use archipel_kit::Glyph;
use archipel_kit::Theme;
use archipel_kit::components::{Button, IconButton};
use archipel_kit::parts::Surface;
use archipel_kit::tokens::{size, typography};
use eframe::egui::{self, Align, Key, Layout, RichText};

/// Margin kept between a dialog and the window edge.
const SCREEN_MARGIN: f32 = 20.0;

/// Height of the title bar, the button row, the frame padding and the gaps around the body.
const CHROME_HEIGHT: f32 = 128.0;

/// What the user did with a dialog this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The default button or Enter.
    Ok,
    /// The Cancel button, the title-bar close button or Escape.
    Cancel,
    /// The n-th secondary button (left of the row).
    Secondary(usize),
}

#[must_use = "show it with `shell.show(ctx, body)`"]
pub struct Shell {
    id: &'static str,
    title: String,
    width: f32,
    ok: Option<String>,
    ok_enabled: bool,
    cancel: Option<String>,
    cancel_enabled: bool,
    secondary: Vec<(String, bool)>,
    keys_held: bool,
}

impl Shell {
    /// A dialog with a unique `id`, titled `title`, with Cancel and no default button yet.
    pub fn new(id: &'static str, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            width: 420.0,
            ok: None,
            ok_enabled: true,
            cancel: Some("Cancel".into()),
            cancel_enabled: true,
            secondary: Vec::new(),
            keys_held: false,
        }
    }

    /// Preferred width; the dialog never exceeds the window minus a margin.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// The default (accent) button; Enter activates it.
    pub fn ok(mut self, label: impl Into<String>) -> Self {
        self.ok = Some(label.into());
        self
    }

    /// Disables the default button and Enter (the form is invalid).
    pub fn ok_enabled(mut self, enabled: bool) -> Self {
        self.ok_enabled = enabled;
        self
    }

    /// Renames or (with `None`) removes the Cancel button; Escape and the close button
    /// still report [`Outcome::Cancel`].
    pub fn cancel(mut self, label: Option<&str>) -> Self {
        self.cancel = label.map(str::to_owned);
        self
    }

    /// Disables the Cancel button (work that cannot be stopped is running).
    pub fn cancel_enabled(mut self, enabled: bool) -> Self {
        self.cancel_enabled = enabled;
        self
    }

    /// Adds a secondary button at the left of the row (`Browse…`, `Define remote…`).
    pub fn secondary(mut self, label: impl Into<String>, enabled: bool) -> Self {
        self.secondary.push((label.into(), enabled));
        self
    }

    /// Leaves Enter and Escape to a popup of the dialog that is not an egui popup (a kit
    /// `Popover`), so they close it instead of pressing a button.
    pub fn keys_held(mut self, held: bool) -> Self {
        self.keys_held = held;
        self
    }

    /// Shows the dialog and reports what the user did.
    pub fn show(self, ctx: &egui::Context, body: impl FnOnce(&mut egui::Ui)) -> Option<Outcome> {
        let screen = ctx.content_rect();
        let width = self
            .width
            .min(screen.width() - 2.0 * SCREEN_MARGIN)
            .max(280.0);
        let max_body = (screen.height() - 2.0 * SCREEN_MARGIN - CHROME_HEIGHT).max(120.0);
        let enabled_ok = self.ok.is_some() && self.ok_enabled;
        // A popup that is open now owns Enter and Escape for this frame (a closed combo
        // list must not also press the default button).
        let popup_open = self.keys_held || egui::Popup::is_any_open(ctx);
        let focused_before = ctx.memory(|m| m.focused());
        let enter = !popup_open && ctx.input(|i| i.key_pressed(Key::Enter));
        let escape =
            !popup_open && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape));
        let mut outcome = None;
        let mut button_focused = false;
        let colors = Theme::colors(ctx);
        let frame = Surface::Dialog.frame(ctx);
        egui::Modal::new(egui::Id::new(("dialog", self.id)))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(
                if Theme::current(ctx).dark { 120 } else { 60 },
            ))
            .show(ctx, |ui| {
                ui.set_width(width - 2.0 * size::DIALOG_PADDING);
                ui.spacing_mut().item_spacing.y = size::SECTION_SPACING;
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&self.title)
                            .font(typography::semibold(ui.ctx(), typography::TITLE + 1.0))
                            .color(colors.text_strong),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.add(IconButton::new(Glyph::Close, "Close")).clicked() {
                            outcome = Some(Outcome::Cancel);
                        }
                    });
                });
                scrolling_body(ui, self.id, max_body, body);
                ui.add_space(size::SECTION_SPACING);
                button_row(ui, &self, &mut outcome, &mut button_focused);
            });
        if outcome.is_none() {
            if escape {
                outcome = Some(Outcome::Cancel);
            } else if enter
                && enabled_ok
                && !button_focused
                && focused_before.is_none_or(|id| is_text_edit(ctx, id))
            {
                outcome = Some(Outcome::Ok);
            }
        }
        outcome
    }
}

/// The body of the dialog: drawn in place while it fits, in a scroll area of exactly
/// `max_height` when it does not (the size is decided from the previous pass, so the
/// dialog never resizes itself gradually).
fn scrolling_body(
    ui: &mut egui::Ui,
    id: &'static str,
    max_height: f32,
    body: impl FnOnce(&mut egui::Ui),
) {
    let key = egui::Id::new(("dialog-body-height", id));
    let natural: f32 = ui.ctx().data(|d| d.get_temp(key)).unwrap_or(0.0);
    let pad = size::FOCUS_RING;
    let measured = if natural > max_height {
        egui::ScrollArea::vertical()
            .id_salt(("dialog-body", id))
            .max_height(max_height)
            .min_scrolled_height(max_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Scroll bars must not cover the controls' focus rings.
                ui.add_space(pad);
                body(ui);
                ui.add_space(pad);
            })
            .content_size
            .y
    } else {
        ui.scope(|ui| {
            ui.add_space(pad);
            body(ui);
            ui.add_space(pad);
        })
        .response
        .rect
        .height()
    };
    if (measured - natural).abs() > 0.5 {
        ui.ctx().data_mut(|d| d.insert_temp(key, measured));
        ui.ctx().request_discard("dialog body height changed");
    }
}

fn is_text_edit(ctx: &egui::Context, id: egui::Id) -> bool {
    egui::TextEdit::load_state(ctx, id).is_some()
}

/// `[secondary…]` at the left, `[Cancel] [OK]` at the right, created in reading order so
/// Tab visits them the way they read. The right group's width is remembered from the
/// previous pass.
fn button_row(
    ui: &mut egui::Ui,
    shell: &Shell,
    outcome: &mut Option<Outcome>,
    button_focused: &mut bool,
) {
    let key = egui::Id::new(("dialog-buttons", shell.id));
    let remembered: f32 = ui.ctx().data(|d| d.get_temp(key)).unwrap_or(0.0);
    let mut right = 0.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = size::SECTION_SPACING;
        for (index, (label, enabled)) in shell.secondary.iter().enumerate() {
            let response = ui.add(Button::secondary(label.as_str()).enabled(*enabled));
            *button_focused |= response.has_focus();
            if response.clicked() {
                *outcome = Some(Outcome::Secondary(index));
            }
        }
        ui.add_space((ui.available_width() - remembered).max(0.0));
        let start = ui.cursor().left();
        if let Some(cancel) = &shell.cancel {
            let response = ui.add(Button::secondary(cancel.as_str()).enabled(shell.cancel_enabled));
            *button_focused |= response.has_focus();
            if response.clicked() {
                *outcome = Some(Outcome::Cancel);
            }
        }
        if let Some(ok) = &shell.ok {
            let response = ui.add(Button::primary(ok.as_str()).enabled(shell.ok_enabled));
            *button_focused |= response.has_focus();
            if response.clicked() {
                *outcome = Some(Outcome::Ok);
            }
        }
        right = ui.cursor().left() - start - ui.spacing().item_spacing.x;
    });
    if (right - remembered).abs() > 0.5 {
        ui.ctx().data_mut(|d| d.insert_temp(key, right));
        ui.ctx().request_discard("dialog button row width changed");
    }
}

/// What to select in a dialog's first field when it opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Select {
    /// Everything (a default value such as `untitled`).
    All,
    /// The first `n` characters (a file name without its extension).
    Chars(usize),
}

/// Focuses `response` (the dialog just opened) and selects part of its text, as the
/// reference IDE does for a default value.
pub fn focus_first(ctx: &egui::Context, response: &egui::Response, select: Select) {
    response.request_focus();
    let end = match select {
        Select::All => usize::MAX,
        Select::Chars(n) => n,
    };
    if let Some(mut state) = egui::TextEdit::load_state(ctx, response.id) {
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(end),
            )));
        state.store(ctx, response.id);
    }
}

/// The error shown under a dialog's field, if any.
pub fn error(ctx: &egui::Context, id: &'static str) -> Option<String> {
    ctx.data(|d| d.get_temp::<Option<String>>(egui::Id::new(("dialog-error", id))))
        .flatten()
}

/// Sets or clears the error under a dialog's field.
pub fn set_error(ctx: &egui::Context, id: &'static str, error: Option<String>) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(("dialog-error", id)), error));
}

/// Turns a command failure into a sentence for the user that names the field: no
/// `command failed:` prefix, no OS error codes, no internal error chain.
pub fn friendly(field: &str, message: &str) -> String {
    let message = message.strip_prefix("command failed: ").unwrap_or(message);
    let message = message
        .strip_prefix("filesystem error: ")
        .unwrap_or(message);
    let message = strip_os_error(message);
    let message = message.trim();
    if let Some(path) = message.strip_prefix("already exists: ") {
        return format!("{field}: “{path}” already exists.");
    }
    if message.starts_with("invalid name") {
        return format!(
            "{field} must be a path relative to the folder, without “..” and without a leading “/”."
        );
    }
    if let Some(path) = message.strip_prefix("path is outside the project: ") {
        return format!("{field}: {path} is outside the project.");
    }
    let mut chars = message.chars();
    let sentence = match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
        None => "The operation failed".into(),
    };
    let sentence = sentence.trim_end_matches('.');
    format!("{field}: {sentence}.")
}

/// Removes a trailing ` (os error N)`.
fn strip_os_error(message: &str) -> String {
    match message.rfind(" (os error ") {
        Some(start) if message.ends_with(')') => message[..start].to_owned(),
        _ => message.to_owned(),
    }
}

/// A wrapped paragraph of body text.
pub fn message(ui: &mut egui::Ui, text: impl Into<String>) {
    let colors = Theme::colors(ui.ctx());
    ui.add(egui::Label::new(RichText::new(text.into()).color(colors.text)).wrap());
}

/// Muted explanatory text under a field.
pub fn hint(ui: &mut egui::Ui, text: impl Into<String>) {
    let colors = Theme::colors(ui.ctx());
    ui.add(egui::Label::new(RichText::new(text.into()).color(colors.text_secondary)).wrap());
}

/// Where a list selection that landed on a non-selectable row (a section title) ends up:
/// the nearest selectable row in the direction of travel, else the other direction, else
/// where it came from.
pub fn skip_unselectable(
    len: usize,
    from: usize,
    to: usize,
    selectable: impl Fn(usize) -> bool,
) -> usize {
    if selectable(to) {
        return to;
    }
    let after = (to..len).find(|index| selectable(*index));
    let before = (0..to).rev().find(|index| selectable(*index));
    if to >= from {
        after.or(before)
    } else {
        before.or(after)
    }
    .unwrap_or(from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_name_the_field_and_drop_the_chain() {
        assert_eq!(
            friendly("New name", "command failed: already exists: src/Main.java"),
            "New name: “src/Main.java” already exists."
        );
        assert_eq!(
            friendly(
                "Name",
                "command failed: filesystem error: No such file or directory (os error 2)"
            ),
            "Name: No such file or directory."
        );
        assert!(
            friendly(
                "Name",
                "invalid name `../x`: use a relative path without `..`"
            )
            .starts_with("Name must be a path relative")
        );
    }
}

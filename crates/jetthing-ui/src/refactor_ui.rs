//! Rename UI: the in-place name field next to the symbol (the editor paints
//! the live preview of its occurrences) and the dialog used when the language
//! server has no `prepareRename`. Enter applies, Escape cancels; the policy
//! lives in `jetthing_core::refactor`, these controls only dispatch commands.

use crate::{IdeApp, popup_stack};
use archipel_kit::Theme;
use archipel_kit::components::Button;
use archipel_kit::parts::Surface;
use archipel_kit::tokens::typography;
use eframe::egui::{self, RichText, text_selection::CCursorRange, vec2};
use serde_json::{Value, json};

fn field_id() -> egui::Id {
    egui::Id::new("rename_field")
}

/// The rename field or dialog, while a rename is in progress.
pub fn popup(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(session) = app.state.refactor.rename.clone() else {
        ctx.data_mut(|data| data.remove_temp::<(usize, u64)>(egui::Id::new("rename_focus")));
        return;
    };
    // Focus and select the whole name once per session.
    let focus_key = (session.start, session.revision);
    let first = ctx.data_mut(|data| {
        let id = egui::Id::new("rename_focus");
        let first = data.get_temp::<(usize, u64)>(id) != Some(focus_key);
        data.insert_temp(id, focus_key);
        first
    });
    let has_focus = ctx.memory(|memory| memory.has_focus(field_id()));
    let submit = has_focus && ctx.input(|input| input.key_pressed(egui::Key::Enter));
    let mut name = session.name.clone();
    let mut ok = false;
    let mut close = false;
    let title = format!("Rename '{}'", session.original);
    let mut body = |ui: &mut egui::Ui| {
        ui.set_min_width(280.0);
        let colors = Theme::colors(ui.ctx());
        ui.label(
            RichText::new(&title)
                .font(typography::semibold(ui.ctx(), typography::BODY))
                .color(colors.text_strong),
        );
        ui.add_space(4.0);
        let mut output = egui::TextEdit::singleline(&mut name)
            .id(field_id())
            .desired_width(f32::INFINITY)
            .margin(egui::Margin::symmetric(6, 5))
            .show(ui);
        if first {
            output.response.request_focus();
            output.state.cursor.set_char_range(Some(CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(name.chars().count()),
            )));
            output.state.store(ui.ctx(), field_id());
        }
        let count = session.occurrences.len();
        let hint = if session.dialog {
            String::new()
        } else {
            format!(
                "{count} occurrence{} in this file · Enter to rename, Esc to cancel",
                if count == 1 { "" } else { "s" }
            )
        };
        if let Some(error) = &session.error {
            ui.label(RichText::new(error).color(colors.error));
        } else if !hint.is_empty() {
            ui.label(RichText::new(hint).color(colors.text_secondary));
        }
        if session.dialog {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ok = ui.add(Button::primary("Refactor")).clicked();
                close = ui.add(Button::secondary("Cancel")).clicked();
            });
        }
    };
    let response = if session.dialog {
        egui::Area::new(egui::Id::new("rename_dialog"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, -80.0))
            .show(ctx, |ui| {
                Surface::Dialog.frame(ctx).show(ui, &mut body);
            })
            .response
    } else {
        let anchor = app
            .caret_screen_pos
            .unwrap_or_else(|| ctx.content_rect().center());
        egui::Area::new(egui::Id::new("rename_popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor + vec2(-12.0, 6.0))
            .show(ctx, |ui| {
                Surface::Popup
                    .frame(ctx)
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, &mut body);
            })
            .response
    };
    if name != session.name {
        app.dispatch("refactor.rename.setName", json!({"name": name}));
    }
    if submit || ok {
        app.dispatch("refactor.rename.apply", Value::Null);
    } else if close {
        app.dispatch("refactor.rename.cancel", Value::Null);
        ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
    } else if popup_stack::pressed_outside(ctx, response.rect, None, None) {
        app.dispatch("refactor.rename.cancel", Value::Null);
    }
}

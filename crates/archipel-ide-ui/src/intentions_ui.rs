//! Alt+Enter intention popup and the lightbulb at the caret line.
//!
//! When the caret rests, the actions at the caret are requested in the
//! background; a lightbulb appears left of the line when there are any (red
//! when one of them fixes a problem). Alt+Enter or a click on the bulb opens
//! the popup. Requests and application live in `archipel_ide_core::code_actions`.

use crate::{IdeApp, theme};
use eframe::egui::{self, Color32, Key, Modifiers, Pos2, Rect, Sense, vec2};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// How long the caret must rest before actions are requested.
const REST: Duration = Duration::from_millis(350);
const BULB_YELLOW: Color32 = Color32::from_rgb(0xF2, 0xC5, 0x5C);
const BULB_RED: Color32 = Color32::from_rgb(0xE5, 0x57, 0x65);

fn selected_id() -> egui::Id {
    egui::Id::new("intention_selected")
}

#[derive(Clone, PartialEq)]
struct CaretKey {
    path: PathBuf,
    offset: usize,
    revision: u64,
    diagnostics: u64,
}

/// Alt+Enter, popup keys and the resting-caret request.
pub fn handle(app: &mut IdeApp, ctx: &egui::Context) {
    let consume = |modifiers: Modifiers, key: Key| {
        ctx.input_mut(|input| {
            let before = input.events.len();
            input.events.retain(|event| {
                !matches!(
                    event,
                    egui::Event::Key { key: k, modifiers: m, pressed: true, .. }
                        if *k == key && m.matches_exact(modifiers)
                )
            });
            input.events.len() != before
        })
    };
    if app.state.code_actions().popup {
        if app
            .state
            .current_alerts()
            .is_none_or(|set| set.actions.is_empty())
        {
            app.state.code_actions_mut().popup = false;
        } else {
            popup_keys(app, ctx, &consume);
        }
    }
    // Alt+Enter itself is the keymap's `ShowIntentionActions` (`ACTION_COMMANDS`).
    request_when_resting(app, ctx);
}

/// Selects the first row of the next intentions popup.
pub fn reset_selection(ctx: &egui::Context) {
    ctx.data_mut(|data| data.insert_temp(selected_id(), 0usize));
}

fn request_when_resting(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(document) = app.state.active_doc() else {
        return;
    };
    let key = CaretKey {
        path: document.path.clone(),
        offset: document.selections.primary().head,
        revision: document.text.revision(),
        diagnostics: app.state.lsp().diagnostics_generation(),
    };
    let id = egui::Id::new("intention_caret");
    let (last, since, requested): (Option<CaretKey>, Instant, bool) =
        ctx.data(|data| data.get_temp(id).unwrap_or((None, Instant::now(), false)));
    if last.as_ref() != Some(&key) {
        ctx.data_mut(|data| data.insert_temp(id, (Some(key), Instant::now(), false)));
        ctx.request_repaint_after(REST);
        return;
    }
    if requested || since.elapsed() < REST {
        if !requested {
            ctx.request_repaint_after(REST.saturating_sub(since.elapsed()));
        }
        return;
    }
    ctx.data_mut(|data| data.insert_temp(id, (Some(key), since, true)));
    if app.state.current_code_actions().is_none() {
        // Background refresh (native actions are computed at once): failures
        // (server busy, …) are not worth a status message.
        let _ = app
            .commands
            .execute("code.requestActions", &mut app.state, Value::Null);
    }
}

use archipel_ide_view::popups::intentions::{self as model, Choice, IntentionRow};

fn expanded_id() -> egui::Id {
    egui::Id::new("intention_expanded_submenu")
}

fn current_rows(app: &mut IdeApp, ctx: &egui::Context) -> Vec<IntentionRow> {
    let Some(set) = app.state.current_alerts() else {
        return Vec::new();
    };
    let expanded: Option<String> = ctx.data(|data| data.get_temp(expanded_id()));
    model::rows(&set, expanded.as_deref())
}

fn popup_keys(app: &mut IdeApp, ctx: &egui::Context, consume: &dyn Fn(Modifiers, Key) -> bool) {
    let rows = current_rows(app, ctx);
    let count = rows.len();
    let mut selected: usize = ctx.data(|data| data.get_temp(selected_id()).unwrap_or(0));
    if consume(Modifiers::NONE, Key::ArrowDown) && count > 0 {
        selected = (selected + 1) % count;
    }
    if consume(Modifiers::NONE, Key::ArrowUp) && count > 0 {
        selected = (selected + count - 1) % count;
    }
    ctx.data_mut(|data| data.insert_temp(selected_id(), selected));
    if consume(Modifiers::NONE, Key::Escape) {
        app.state.code_actions_mut().popup = false;
        return;
    }
    if consume(Modifiers::NONE, Key::Enter)
        && let Some(row) = rows.get(selected)
    {
        match model::choose(row) {
            Choice::Apply(index) => {
                app.dispatch("code.applyAlert", json!({"index": index}));
            }
            Choice::Expand(next) => ctx.data_mut(|data| match next {
                Some(title) => data.insert_temp(expanded_id(), title),
                None => data.remove::<String>(expanded_id()),
            }),
        }
    }
}

/// Paints a lightbulb centred on `center`.
fn paint_bulb(painter: &egui::Painter, center: Pos2, color: Color32) {
    painter.circle_filled(center + vec2(0.0, -1.5), 4.5, color);
    painter.rect_filled(
        Rect::from_center_size(center + vec2(0.0, 4.5), vec2(4.0, 2.5)),
        0.5,
        theme::TEXT_SECONDARY,
    );
}

/// The lightbulb at the caret line; a click opens the popup.
pub fn lightbulb(app: &mut IdeApp, ui: &mut egui::Ui, anchor: Option<Pos2>) {
    let Some(anchor) = anchor else {
        return;
    };
    let Some(set) = app.state.current_alerts() else {
        return;
    };
    let color = match set.bulb() {
        archipel_ide_core::intentions::Bulb::None => return,
        archipel_ide_core::intentions::Bulb::ErrorFix => BULB_RED,
        archipel_ide_core::intentions::Bulb::Intention => BULB_YELLOW,
    };
    let rect = Rect::from_center_size(anchor, vec2(18.0, 18.0));
    let response = ui.interact(rect, egui::Id::new("intention_bulb"), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme::HOVER);
    }
    paint_bulb(ui.painter(), anchor, color);
    let tooltip = format!(
        "Show Context Actions ({})",
        crate::keys::display("⌥↩").replace('↩', "Enter")
    );
    if response.on_hover_text(tooltip).clicked() {
        ui.ctx()
            .data_mut(|data| data.insert_temp(selected_id(), 0usize));
        app.state.code_actions_mut().popup = true;
    }
}

/// The intention popup below the caret.
pub fn popup(app: &mut IdeApp, ctx: &egui::Context) {
    if !app.state.code_actions().popup {
        return;
    }
    if app.state.current_alerts().is_none() {
        return;
    }
    let rows = current_rows(app, ctx);
    let selected: usize = ctx.data(|data| data.get_temp(selected_id()).unwrap_or(0));
    let anchor = app
        .caret_screen_pos
        .unwrap_or_else(|| ctx.content_rect().center());
    let mut apply = None;
    let mut toggle: Option<Option<String>> = None;
    let response = egui::Area::new(egui::Id::new("intention_popup"))
        .order(egui::Order::Foreground)
        .fixed_pos(anchor + vec2(-24.0, 4.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    ui.set_min_width(320.0);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
                    let label = |row: &IntentionRow| row.text().to_owned();
                    let width = rows
                        .iter()
                        .map(|row| {
                            ui.painter()
                                .layout_no_wrap(label(row), font.clone(), *theme::TEXT)
                                .size()
                                .x
                        })
                        .fold(320.0_f32, f32::max)
                        .min(720.0)
                        + 64.0;
                    for (position, row) in rows.iter().enumerate() {
                        let (rect, response) =
                            ui.allocate_exact_size(vec2(width, 24.0), Sense::click());
                        if position == selected {
                            ui.painter()
                                .rect_filled(rect, 4.0, theme::COMPLETION_SELECTION);
                        } else if response.hovered() {
                            ui.painter().rect_filled(rect, 4.0, theme::HOVER);
                        }
                        let (indent, text, color) = match row {
                            IntentionRow::Action {
                                text, fix, nested, ..
                            } => (
                                if *nested { 30.0 } else { 0.0 },
                                text.clone(),
                                if *fix { BULB_RED } else { BULB_YELLOW },
                            ),
                            IntentionRow::Submenu { .. } => (0.0, row.label(), BULB_YELLOW),
                        };
                        if matches!(row, IntentionRow::Action { .. }) {
                            paint_bulb(
                                ui.painter(),
                                egui::pos2(rect.left() + 14.0 + indent, rect.center().y),
                                color,
                            );
                        }
                        crate::widgets::row_text(
                            ui,
                            rect,
                            rect.left() + 30.0 + indent,
                            &text,
                            font.clone(),
                            theme::TEXT_BRIGHT,
                        );
                        if response.clicked() {
                            match model::choose(row) {
                                Choice::Apply(index) => apply = Some(index),
                                Choice::Expand(next) => toggle = Some(next),
                            }
                        }
                    }
                });
        })
        .response;
    if let Some(next) = toggle {
        ctx.data_mut(|data| match next {
            Some(title) => data.insert_temp(expanded_id(), title),
            None => data.remove::<String>(expanded_id()),
        });
    }
    if let Some(index) = apply {
        app.dispatch("code.applyAlert", json!({"index": index}));
        ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
    } else if ctx.input(|input| input.pointer.any_pressed()) && !response.contains_pointer() {
        app.state.code_actions_mut().popup = false;
    }
}

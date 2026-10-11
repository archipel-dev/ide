//! Alt+Enter intention popup and the lightbulb at the caret line.
//!
//! When the caret rests, the actions at the caret are requested in the
//! background; a lightbulb appears left of the line when there are any (red
//! when one of them fixes a problem). Alt+Enter or a click on the bulb opens
//! the popup. Requests and application live in `jetthing_core::code_actions`.

use crate::IdeApp;
use archipel_kit::components::{IconButton, ListRow};
use archipel_kit::icons::Glyph;
use archipel_kit::parts::{Surface, nav};
use archipel_kit::{Theme, Tone};
use eframe::egui::{self, Key, Modifiers, Pos2, Rect, vec2};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// How long the caret must rest before actions are requested.
const REST: Duration = Duration::from_millis(350);
/// Rows of an intention list a PgUp/PgDn moves by.
const PAGE: usize = 8;

/// The kit has no lightbulb glyph yet: a red error mark stands for a quick fix and the
/// yellow weak-warning mark for an intention.
fn bulb(quick_fix: bool) -> Glyph {
    if quick_fix {
        Glyph::Error
    } else {
        Glyph::WeakWarning
    }
}

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
    if app.state.code_actions.popup {
        if app.state.current_code_actions().is_none() {
            app.state.code_actions.popup = false;
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
    let Some(document) = app
        .state
        .active_document
        .and_then(|index| app.state.documents.get(index))
    else {
        return;
    };
    let key = CaretKey {
        path: document.path.clone(),
        offset: document.selections.primary().head,
        revision: document.text.revision(),
        diagnostics: app.state.lsp.diagnostics_generation(),
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
    if app.state.current_code_actions().is_none() && app.state.active_supports_code_actions() {
        // Background refresh: failures (server busy, …) are not worth a status message.
        let _ = app
            .commands
            .execute("code.requestActions", &mut app.state, Value::Null);
    }
}

/// Display order: quick fixes first, then intentions, keeping server order.
fn display_order(app: &IdeApp) -> Vec<usize> {
    let Some(list) = app.state.current_code_actions() else {
        return Vec::new();
    };
    let mut order: Vec<usize> = (0..list.items.len()).collect();
    order.sort_by_key(|&index| {
        let item = &list.items[index];
        (!item.quick_fix, !item.preferred, item.disabled.is_some())
    });
    order
}

fn popup_keys(app: &mut IdeApp, ctx: &egui::Context, consume: &dyn Fn(Modifiers, Key) -> bool) {
    let order = display_order(app);
    let count = order.len();
    let mut selected: usize = ctx.data(|data| data.get_temp(selected_id()).unwrap_or(0));
    for key in nav::KEYS {
        if consume(Modifiers::NONE, key)
            && let Some(next) = nav::step(key, Some(selected), count, PAGE)
        {
            selected = next;
        }
    }
    ctx.data_mut(|data| data.insert_temp(selected_id(), selected));
    if consume(Modifiers::NONE, Key::Escape) {
        app.state.code_actions.popup = false;
        return;
    }
    if consume(Modifiers::NONE, Key::Enter)
        && let Some(&index) = order.get(selected)
    {
        app.dispatch("code.applyAction", json!({"index": index}));
    }
}

/// The lightbulb at the caret line; a click opens the popup.
pub fn lightbulb(app: &mut IdeApp, ui: &mut egui::Ui, anchor: Option<Pos2>) {
    let (Some(anchor), Some(list)) = (anchor, app.state.current_code_actions()) else {
        return;
    };
    if list.items.is_empty() {
        return;
    }
    let tooltip = format!(
        "Show Context Actions ({})",
        crate::keys::display("⌥↩").replace('↩', "Enter")
    );
    let rect = Rect::from_center_size(anchor, vec2(20.0, 20.0));
    let response = ui.put(
        rect,
        IconButton::new(bulb(list.has_quick_fix()), tooltip).small(true),
    );
    if response.clicked() {
        ui.ctx()
            .data_mut(|data| data.insert_temp(selected_id(), 0usize));
        app.state.code_actions.popup = true;
    }
}

/// The intention popup below the caret.
pub fn popup(app: &mut IdeApp, ctx: &egui::Context) {
    if !app.state.code_actions.popup {
        return;
    }
    let Some(list) = app.state.current_code_actions().cloned() else {
        return;
    };
    let order = display_order(app);
    let selected: usize = ctx.data(|data| data.get_temp(selected_id()).unwrap_or(0));
    let anchor = app
        .caret_screen_pos
        .unwrap_or_else(|| ctx.content_rect().center());
    let mut apply = None;
    let colors = Theme::colors(ctx);
    let response = egui::Area::new(egui::Id::new("intention_popup"))
        .order(egui::Order::Foreground)
        .sense(egui::Sense::hover())
        .fixed_pos(anchor + vec2(-24.0, 4.0))
        .show(ctx, |ui| {
            Surface::Popup.frame(ctx).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let font = egui::TextStyle::Body.resolve(ui.style());
                let width = order
                    .iter()
                    .map(|&index| {
                        ui.painter()
                            .layout_no_wrap(
                                list.items[index].title.clone(),
                                font.clone(),
                                colors.text,
                            )
                            .size()
                            .x
                    })
                    .fold(320.0_f32, f32::max)
                    .min(720.0)
                    + 56.0;
                ui.set_width(width);
                for (row, &index) in order.iter().enumerate() {
                    let item = &list.items[index];
                    let shown = ListRow::new(item.title.as_str())
                        .flat(true)
                        .icon(bulb(item.quick_fix))
                        .selected(row == selected)
                        .enabled(item.disabled.is_none())
                        .label_tone(if item.disabled.is_some() {
                            Tone::Disabled
                        } else {
                            Tone::Strong
                        })
                        .show(ui);
                    let response = match &item.disabled {
                        Some(reason) => shown.response.on_hover_text(reason),
                        None => shown.response,
                    };
                    if response.clicked() {
                        apply = Some(index);
                    }
                }
                if order.is_empty() {
                    ui.label(egui::RichText::new("No actions").color(colors.text_secondary));
                }
            });
        })
        .response;
    if let Some(index) = apply {
        app.dispatch("code.applyAction", json!({"index": index}));
        ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
    } else if ctx.input(|input| input.pointer.any_pressed()) && !response.contains_pointer() {
        app.state.code_actions.popup = false;
    }
}

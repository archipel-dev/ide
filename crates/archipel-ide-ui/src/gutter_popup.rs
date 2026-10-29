//! Gutter run icons and their popup (Run / Debug / Modify Run Configuration…).

use crate::icons::{self, Icon};
use crate::{IdeApp, theme};
use archipel_ide_core::RunnableKind;
use archipel_ide_view::popups::gutter;
use archipel_kit_model::MenuEntry;
use eframe::egui::{self, vec2};

#[derive(Clone, Copy)]
struct Open {
    line: usize,
    position: egui::Pos2,
    /// Pass in which the popup opened; that pass's click must not close it.
    opened: u64,
}

fn state_id() -> egui::Id {
    egui::Id::new("gutter_run_popup")
}

/// Paints a run icon centred at `center` (single test, test class/module or main).
pub fn paint_icon(painter: &egui::Painter, center: egui::Pos2, kind: RunnableKind) {
    let rect = egui::Rect::from_center_size(center, vec2(16.0, 16.0));
    match kind {
        RunnableKind::Main | RunnableKind::Test => {
            icons::paint(painter, rect, Icon::Run, theme::RUN_GREEN)
        }
        RunnableKind::TestContainer => icons::paint(painter, rect, Icon::Rerun, theme::RUN_GREEN),
    }
}

pub fn open(ctx: &egui::Context, line: usize, position: egui::Pos2) {
    let opened = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| {
        data.insert_temp(
            state_id(),
            Some(Open {
                line,
                position,
                opened,
            }),
        )
    });
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(Some(open)) = ctx.data(|data| data.get_temp::<Option<Open>>(state_id())) else {
        return;
    };
    // The rows are the view crate's: the terminal draws the same ones.
    let entries = app.with_view(|cx| gutter::run_popup(cx, open.line));
    if entries.is_empty() {
        close(ctx);
        return;
    }
    let notation = archipel_ide_view::Caps::gui().notation();
    let mut chosen: Option<String> = None;
    let area = egui::Area::new(egui::Id::new("gutter_run_popup_area"))
        .order(egui::Order::Foreground)
        .fixed_pos(open.position + vec2(8.0, 4.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(280.0);
                for entry in &entries {
                    match entry {
                        MenuEntry::Item(item) => {
                            let mut row =
                                crate::widgets::menu_item(item.label.clone()).enabled(item.enabled);
                            match item.icon.as_ref().and_then(|icon| icon.glyph()) {
                                Some(archipel_kit_model::Glyph::Run) => row = row.icon(Icon::Run),
                                Some(archipel_kit_model::Glyph::Debug) => {
                                    row = row.icon(Icon::Debug)
                                }
                                _ => {}
                            }
                            if let Some(shortcut) = &item.shortcut {
                                row = row.shortcut(shortcut.label(notation));
                            }
                            let mut response = ui.add(row);
                            if let Some(reason) = &item.disabled_reason {
                                response = response.on_disabled_hover_text(reason);
                            }
                            if response.clicked() {
                                chosen = Some(item.id.clone());
                            }
                        }
                        MenuEntry::Separator { .. } => {
                            ui.separator();
                        }
                    }
                }
            });
        });
    // Escape is handled by the workbench's Escape chain (see `is_open`). A press
    // outside closes the popup, as in the reference, so dragging a selection
    // in the editor does not leave it open.
    let pressed_elsewhere = ctx.cumulative_pass_nr() > open.opened
        && ctx.input(|input| {
            input.pointer.any_pressed()
                && input
                    .pointer
                    .press_origin()
                    .is_some_and(|origin| !area.response.rect.contains(origin))
        })
        && chosen.is_none();
    if let Some(id) = chosen {
        close(ctx);
        let effects = app.with_view(|cx| archipel_ide_view::dispatch::menu_row(cx, &id));
        app.apply_view_effects(effects, ctx);
    } else if pressed_elsewhere {
        close(ctx);
    }
}

/// Whether the run popup is showing (Escape closes it before anything else).
pub fn is_open(ctx: &egui::Context) -> bool {
    matches!(
        ctx.data(|data| data.get_temp::<Option<Open>>(state_id())),
        Some(Some(_))
    )
}

pub fn close(ctx: &egui::Context) {
    ctx.data_mut(|data| data.insert_temp::<Option<Open>>(state_id(), None));
}

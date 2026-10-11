//! Gutter run icons and their popup (Run / Debug / Modify Run Configuration…).

use crate::icons::{self, Icon};
use crate::{IdeApp, keys, theme};
use eframe::egui::{self, vec2};
use jetthing_core::RunnableKind;
use serde_json::json;

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
    let rect = egui::Rect::from_center_size(center, vec2(13.0, 13.0));
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
    let marks = app.state.gutter_marks();
    let Some(mark) = marks.iter().find(|mark| mark.line == open.line).cloned() else {
        close(ctx);
        return;
    };
    let target = mark.label.trim_start_matches("Run ").to_owned();
    let mut action = None;
    let mut debug_action = false;
    let area = egui::Area::new(egui::Id::new("gutter_run_popup_area"))
        .order(egui::Order::Foreground)
        .fixed_pos(open.position + vec2(8.0, 4.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(280.0);
                let running = app.state.runner.is_running();
                let run = ui.add(
                    crate::widgets::menu_item(mark.label.clone())
                        .icon(Icon::Run)
                        .shortcut(app.action_shortcut("RunClass").unwrap_or_default())
                        .enabled(!running),
                );
                if run.clicked() {
                    action = Some(false);
                }
                let debug = ui.add(
                    crate::widgets::menu_item(format!("Debug {target}"))
                        .icon(Icon::Debug)
                        .shortcut(keys::display("⌃D"))
                        .enabled(!app.state.is_debugging() && !running),
                );
                if debug.clicked() {
                    debug_action = true;
                }
                ui.add(
                    crate::widgets::menu_item(format!("Run {target} with Coverage")).enabled(false),
                )
                .on_disabled_hover_text("Code coverage is not available yet (IJ-RUN-069)");
                if ui
                    .add(crate::widgets::menu_item("Modify Run Configuration…"))
                    .clicked()
                {
                    action = Some(true);
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
        && action.is_none();
    if debug_action {
        close(ctx);
        app.dispatch(
            "run.gutter",
            json!({"line": open.line, "edit": false, "debug": true}),
        );
    } else if let Some(edit) = action {
        close(ctx);
        app.dispatch("run.gutter", json!({"line": open.line, "edit": edit}));
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

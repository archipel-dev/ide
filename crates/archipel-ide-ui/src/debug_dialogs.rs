//! Debugger popups: breakpoint properties (condition, logpoint, hit count) and the
//! Evaluate Expression dialog (Alt+F8). Both only dispatch commands.

use crate::popups::{popup_frame, title};
use crate::{IdeApp, theme};
use archipel_ide_dap::SessionState;
use eframe::egui::{self, Color32, RichText};
use serde_json::json;

/// Edit buffers of the breakpoint properties popup.
#[derive(Clone, Default)]
struct BreakpointForm {
    /// `(path, line)` the buffers were loaded for.
    key: Option<(std::path::PathBuf, usize)>,
    /// The condition field still has to take keyboard focus (first frame).
    focus_pending: bool,
    condition: String,
    log_message: String,
    hit_condition: String,
}

fn anchor_id() -> egui::Id {
    egui::Id::new("breakpoint_properties_anchor")
}

/// Remembers where the breakpoint popup should open (the right-click position).
pub fn anchor_breakpoint_popup(ctx: &egui::Context, position: egui::Pos2) {
    ctx.data_mut(|data| data.insert_temp(anchor_id(), position));
}

fn form_id() -> egui::Id {
    egui::Id::new("breakpoint_properties_form")
}

/// Paints an end-of-line inline value hint starting at `origin` (the editor passes the
/// column after the line text) and returns nothing; the caller owns placement.
pub fn paint_inline_value(
    painter: &egui::Painter,
    origin: egui::Pos2,
    row_height: f32,
    text: &str,
) {
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        egui::FontId::proportional(theme::UI_FONT_SIZE - 1.5),
        *theme::editor::INLINE_VALUE,
    );
    let rect = egui::Rect::from_min_size(
        egui::pos2(origin.x, origin.y + (row_height - galley.size().y) / 2.0),
        galley.size(),
    );
    painter.galley(rect.min, galley, Color32::WHITE);
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    breakpoint_properties(app, ctx);
    evaluate_dialog(app, ctx);
}

/// The properties popup, next to where it was requested (like the run popup), kept inside
/// the window; centered when opened from the keyboard.
fn breakpoint_frame(ctx: &egui::Context, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    let screen = ctx.content_rect();
    let anchor = ctx.data(|data| data.get_temp::<egui::Pos2>(anchor_id()));
    let height = 190.0;
    let position = match anchor {
        Some(anchor) => egui::pos2(
            (anchor.x + 12.0)
                .min(screen.right() - width - 8.0)
                .max(screen.left() + 8.0),
            (anchor.y - 16.0)
                .min(screen.bottom() - height - 8.0)
                .max(screen.top() + 8.0),
        ),
        None => egui::pos2(
            screen.center().x - width / 2.0,
            screen.top() + screen.height() * 0.2,
        ),
    };
    egui::Area::new(egui::Id::new("breakpoint_properties"))
        .order(egui::Order::Foreground)
        .fixed_pos(position)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .corner_radius(10)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(width - 24.0);
                    add(ui);
                });
        });
}

fn breakpoint_properties(app: &mut IdeApp, ctx: &egui::Context) {
    let Some((path, line)) = app.state.debugger().state().editing_breakpoint.clone() else {
        // Closed (maybe by Escape in the workbench): the next open starts fresh.
        ctx.data_mut(|data| {
            data.remove::<BreakpointForm>(form_id());
            data.remove::<egui::Pos2>(anchor_id());
        });
        return;
    };
    let mut form: BreakpointForm = ctx
        .data(|data| data.get_temp(form_id()))
        .unwrap_or_default();
    let key = Some((path.clone(), line));
    if form.key != key {
        let options = app.state.breakpoint_options(&path, line);
        let options = options.unwrap_or_default();
        form = BreakpointForm {
            key,
            focus_pending: true,
            condition: options.condition.unwrap_or_default(),
            log_message: options.log_message.unwrap_or_default(),
            hit_condition: options.hit_condition.unwrap_or_default(),
        };
    }
    let mut done = false;
    let mut remove = false;
    breakpoint_frame(ctx, 460.0, |ui| {
        title(
            ui,
            &format!(
                "Line Breakpoint — {}:{}",
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                line + 1
            ),
        );
        egui::Grid::new("breakpoint_fields")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Condition").color(theme::TEXT_SECONDARY));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut form.condition)
                        .id(egui::Id::new("breakpoint_condition"))
                        .hint_text("e.g. i == 3")
                        .desired_width(320.0),
                );
                if std::mem::take(&mut form.focus_pending) {
                    response.request_focus();
                }
                ui.end_row();
                ui.label(RichText::new("Log message").color(theme::TEXT_SECONDARY));
                ui.add(
                    egui::TextEdit::singleline(&mut form.log_message)
                        .id(egui::Id::new("breakpoint_log_message"))
                        .hint_text("Logpoint: print, {expr} interpolates")
                        .desired_width(320.0),
                );
                ui.end_row();
                ui.label(RichText::new("Hit count").color(theme::TEXT_SECONDARY));
                ui.add(
                    egui::TextEdit::singleline(&mut form.hit_condition)
                        .id(egui::Id::new("breakpoint_hit_condition"))
                        .hint_text("e.g. 3 or >= 2")
                        .desired_width(320.0),
                );
                ui.end_row();
            });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let done_button =
                egui::Button::new(RichText::new("Done").color(Color32::WHITE)).fill(theme::ACCENT);
            if ui.add(done_button).clicked() {
                done = true;
            }
            if ui.button("Remove Breakpoint").clicked() {
                remove = true;
            }
        });
    });
    let (enter, escape) = ctx.input(|input| {
        (
            input.key_pressed(egui::Key::Enter),
            input.key_pressed(egui::Key::Escape),
        )
    });
    done |= enter;
    if done {
        let text = |value: &str| Some(value.to_owned());
        app.dispatch(
            "debug.setBreakpointOptions",
            json!({
                "path": path,
                "line": line,
                "condition": text(&form.condition),
                "logMessage": text(&form.log_message),
                "hitCondition": text(&form.hit_condition),
            }),
        );
    } else if remove {
        app.dispatch(
            "debug.toggleBreakpoint",
            json!({"path": path, "line": line}),
        );
    }
    if done || remove || escape {
        ctx.data_mut(|data| data.remove::<BreakpointForm>(form_id()));
        app.dispatch("debug.closeBreakpointEditor", serde_json::Value::Null);
    } else {
        ctx.data_mut(|data| data.insert_temp(form_id(), form));
    }
}

fn evaluate_dialog(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(mut expression) = app.state.debugger().state().evaluate_dialog.clone() else {
        ctx.data_mut(|data| data.remove::<bool>(egui::Id::new("evaluate_dialog_opened")));
        return;
    };
    let (result, stopped) = match app.state.debugger().state().client.as_ref() {
        Some(client) => (
            client.session().evaluations().last().cloned(),
            matches!(client.session().state(), SessionState::Stopped { .. }),
        ),
        None => (None, false),
    };
    let mut evaluate = false;
    let mut watch = false;
    let mut close = false;
    popup_frame(ctx, "evaluate_expression_dialog", 560.0, |ui| {
        title(ui, "Evaluate Expression");
        let response = ui.add(
            egui::TextEdit::multiline(&mut expression)
                .id(egui::Id::new("evaluate_expression_input"))
                .hint_text("Expression (Enter evaluates, Shift+Enter adds a line)")
                .desired_rows(2)
                .desired_width(f32::INFINITY)
                .font(egui::FontId::monospace(theme::UI_FONT_SIZE - 0.5)),
        );
        // Take keyboard focus when the dialog opens (an editor or watch field may hold it).
        let opened_id = egui::Id::new("evaluate_dialog_opened");
        if !ui.data(|data| data.get_temp::<bool>(opened_id).unwrap_or(false)) {
            response.request_focus();
            ui.data_mut(|data| data.insert_temp(opened_id, true));
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            // The default button (Enter) is filled with the accent color.
            let evaluate_button =
                egui::Button::new(RichText::new("Evaluate").color(Color32::WHITE))
                    .fill(theme::ACCENT);
            if ui.add_enabled(stopped, evaluate_button).clicked() {
                evaluate = true;
            }
            if ui
                .add_enabled(
                    !expression.trim().is_empty(),
                    egui::Button::new("Add to Watches"),
                )
                .clicked()
            {
                watch = true;
            }
            if ui.button("Close").clicked() {
                close = true;
            }
        });
        ui.add_space(6.0);
        match &result {
            Some((expression, Ok(value))) => {
                ui.label(
                    RichText::new(format!("{expression} = {value}"))
                        .monospace()
                        .color(*theme::editor::TEXT),
                );
            }
            Some((expression, Err(error))) => {
                ui.label(
                    RichText::new(format!("{expression}: {error}"))
                        .monospace()
                        .color(*theme::ERROR),
                );
            }
            None => {
                ui.label(RichText::new("Result").color(*theme::TEXT_DISABLED));
            }
        }
        if !stopped {
            ui.label(
                RichText::new("The program must be suspended to evaluate").color(*theme::TEXT_INFO),
            );
        }
    });
    let (enter, shift, escape) = ctx.input(|input| {
        (
            input.key_pressed(egui::Key::Enter),
            input.modifiers.shift,
            input.key_pressed(egui::Key::Escape),
        )
    });
    if enter && shift {
        // Shift+Enter keeps the newline the multi-line field inserted.
    } else if enter {
        expression = expression.trim_end_matches('\n').to_owned();
        evaluate = true;
    }
    if escape {
        close = true;
    }
    if evaluate && stopped && !expression.trim().is_empty() {
        app.dispatch("debug.evaluate", json!({"expression": expression.trim()}));
    }
    if watch {
        app.dispatch("debug.addWatch", json!({"expression": expression.trim()}));
    }
    if close {
        ctx.data_mut(|data| data.remove::<bool>(egui::Id::new("evaluate_dialog_opened")));
        app.dispatch("debug.closeEvaluateDialog", serde_json::Value::Null);
    } else {
        app.state.debugger().state_mut().evaluate_dialog = Some(expression);
    }
}

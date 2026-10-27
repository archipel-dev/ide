//! Debug tool window: toolbar, frames, variables (with evaluate) and console.

use crate::icons::{self, Icon};
use crate::widgets::{list_row, row_text, tool_window_header};
use crate::{IdeApp, keys, theme};
use archipel_ide_core::ToolWindow;
use archipel_ide_dap::{OutputCategory, Session, SessionState};
use eframe::egui::{self, Color32, RichText, vec2};
use serde_json::{Value, json};
use std::collections::HashSet;

pub fn paint_execution_arrow(painter: &egui::Painter, center: egui::Pos2) {
    let points = vec![
        center + vec2(-6.0, -4.0),
        center + vec2(1.0, -4.0),
        center + vec2(6.0, 0.0),
        center + vec2(1.0, 4.0),
        center + vec2(-6.0, 4.0),
    ];
    painter.add(egui::Shape::convex_polygon(
        points,
        *theme::ACCENT,
        egui::Stroke::NONE,
    ));
}

#[derive(Clone, Default)]
struct View {
    console: bool,
    expanded: HashSet<i64>,
    expression: String,
    /// Text of the "add watch" field.
    watch_input: String,
    /// Watch being edited: index and text.
    editing_watch: Option<(usize, String)>,
}

fn view_id() -> egui::Id {
    egui::Id::new("debug_window_view")
}

/// `"main"@1: RUNNING` / `Suspended at breakpoint` status text.
pub fn status_text(session: &Session) -> (String, Color32) {
    match session.state() {
        SessionState::Initializing | SessionState::Launching => {
            ("Starting debugger…".into(), *theme::TEXT_SECONDARY)
        }
        SessionState::Running => ("Running".into(), *theme::SUCCESS),
        SessionState::Stopped {
            thread_id,
            reason,
            description,
        } => {
            let thread = session
                .threads()
                .iter()
                .find(|thread| thread.id == *thread_id)
                .map_or_else(
                    || format!("thread {thread_id}"),
                    |t| format!("\"{}\"@{}", t.name, t.id),
                );
            let why = description.clone().unwrap_or_else(|| reason.clone());
            (format!("{thread}: suspended ({why})"), *theme::WARNING)
        }
        SessionState::Terminated { exit_code } => (
            match exit_code {
                Some(code) => format!("Terminated (exit code {code})"),
                None => "Terminated".into(),
            },
            *theme::TEXT_INFO,
        ),
    }
}

pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    let name = app.state.debugger().state().name.clone();
    if tool_window_header(ui, "Debug", |ui| {
        ui.label(RichText::new(&name).color(*theme::TEXT_SECONDARY));
    }) {
        app.toggle_tool_window(ToolWindow::Debug);
    }
    let mut view: View = ui.data(|data| data.get_temp(view_id())).unwrap_or_default();
    // Toolbar row, as in the reference: session actions, stepping, then view tabs.
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.spacing_mut().item_spacing.x = 2.0;
        let buttons: [(&str, Icon, Color32, String); 7] = [
            (
                "debug.start",
                Icon::Rerun,
                *theme::RUN_GREEN,
                format!("Debug ({})", keys::display("⌃D")),
            ),
            ("debug.stop", Icon::Stop, *theme::STOP_RED, "Stop".into()),
            (
                "debug.resume",
                Icon::Run,
                *theme::RUN_GREEN,
                format!("Resume Program ({})", keys::display("⌥⌘R")),
            ),
            (
                "debug.pause",
                Icon::Pause,
                *icons::ICON,
                "Pause Program".into(),
            ),
            (
                "debug.stepOver",
                Icon::StepOver,
                *icons::ICON,
                format!("Step Over ({})", keys::display("F8")),
            ),
            (
                "debug.stepInto",
                Icon::StepInto,
                *icons::ICON,
                format!("Step Into ({})", keys::display("F7")),
            ),
            (
                "debug.stepOut",
                Icon::StepOut,
                *icons::ICON,
                format!("Step Out ({})", keys::display("⇧F8")),
            ),
        ];
        for (index, (id, icon, color, tooltip)) in buttons.into_iter().enumerate() {
            if index == 2 {
                ui.add_space(8.0);
            }
            let enabled = app.commands.is_enabled(id, &app.state);
            let color = if enabled {
                color
            } else {
                *theme::TEXT_DISABLED
            };
            if ui
                .add_enabled_ui(enabled, |ui| {
                    icons::button_colored(ui, icon, color, &tooltip, false)
                })
                .inner
                .clicked()
            {
                app.dispatch(id, Value::Null);
            }
        }
        ui.add_space(16.0);
        if ui
            .selectable_label(!view.console, "Threads & Variables")
            .clicked()
        {
            view.console = false;
        }
        if ui.selectable_label(view.console, "Console").clicked() {
            view.console = true;
        }
    });
    ui.separator();
    let debugger = app.state.debugger();
    let debugger_state = debugger.state();
    let Some(client) = debugger_state.client.as_ref() else {
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            if debugger_state.pending.is_some() {
                ui.label(RichText::new("Building before debugging…").color(*theme::TEXT_SECONDARY));
            } else {
                ui.label(RichText::new("No debug session").color(*theme::TEXT_SECONDARY));
                ui.label(
                    RichText::new(format!(
                        "Set breakpoints by clicking line numbers, then Debug ({})",
                        keys::display("⌃D")
                    ))
                    .color(*theme::TEXT_INFO),
                );
            }
            if let Some(error) = &debugger_state.error {
                ui.label(RichText::new(error).color(*theme::ERROR));
            }
        });
        ui.data_mut(|data| data.insert_temp(view_id(), view));
        return;
    };
    let session = client.session();
    let mut select_frame = None;
    let mut load = None;
    let mut evaluate = None;
    let mut watch_actions: Vec<(&'static str, Value)> = Vec::new();
    if view.console {
        console(ui, session);
    } else {
        let available = ui.available_rect_before_wrap();
        let left_width = (available.width() * 0.3).clamp(220.0, 420.0);
        let left_rect =
            egui::Rect::from_min_size(available.min, vec2(left_width, available.height()));
        let right_rect = egui::Rect::from_min_max(
            egui::pos2(left_rect.right() + 1.0, available.top()),
            available.max,
        );
        ui.allocate_rect(available, egui::Sense::hover());
        ui.painter().line_segment(
            [left_rect.right_top(), left_rect.right_bottom()],
            egui::Stroke::new(1.0_f32, *theme::TOOLBAR_SEPARATOR),
        );
        let mut left = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(left_rect.shrink2(vec2(6.0, 4.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let (status, color) = status_text(session);
        left.label(RichText::new(status).color(color));
        if let Some(error) = session.error() {
            left.label(RichText::new(error).color(*theme::ERROR));
        }
        left.add_space(4.0);
        egui::ScrollArea::vertical()
            .id_salt("debug_frames")
            .auto_shrink([false, false])
            .show(&mut left, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let selected = session.selected_frame().map(|frame| frame.id);
                for frame in session.frames() {
                    let (rect, response) = list_row(ui, selected == Some(frame.id));
                    let location = frame
                        .path
                        .as_ref()
                        .and_then(|path| path.file_name())
                        .map(|file| format!("{}:{}", file.to_string_lossy(), frame.line))
                        .unwrap_or_else(|| "<no source>".into());
                    let color = if frame.path.is_some() {
                        *theme::TEXT
                    } else {
                        *theme::TEXT_INFO
                    };
                    let end = row_text(
                        ui,
                        rect,
                        rect.left() + 10.0,
                        &frame.name,
                        egui::FontId::proportional(theme::UI_FONT_SIZE),
                        color,
                    );
                    row_text(
                        ui,
                        rect,
                        end + 8.0,
                        &location,
                        egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0),
                        *theme::TEXT_INFO,
                    );
                    if response.clicked() {
                        select_frame = Some(frame.id);
                    }
                }
            });
        let mut right = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(right_rect.shrink2(vec2(8.0, 4.0)))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let stopped = matches!(session.state(), SessionState::Stopped { .. });
        right.add_enabled_ui(stopped, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut view.expression)
                    .hint_text(
                        RichText::new("Evaluate expression (Enter)").color(*theme::TEXT_DISABLED),
                    )
                    .desired_width(f32::INFINITY)
                    .font(egui::FontId::monospace(theme::UI_FONT_SIZE - 0.5)),
            );
            if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                evaluate = Some(std::mem::take(&mut view.expression));
                response.request_focus();
            }
        });
        right.add_space(4.0);
        egui::ScrollArea::both()
            .id_salt("debug_variables")
            .auto_shrink([false, false])
            .show(&mut right, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                watches_panel(ui, session, &mut view, &mut watch_actions);
                for (expression, result) in session.evaluations().iter().rev().take(5) {
                    let (text, color) = match result {
                        Ok(value) => (format!("{expression} = {value}"), *theme::editor::TEXT),
                        Err(error) => (format!("{expression}: {error}"), *theme::ERROR),
                    };
                    ui.label(RichText::new(text).monospace().color(color));
                }
                for scope in session.scopes() {
                    variable_rows(
                        ui,
                        session,
                        scope.reference,
                        &scope.name,
                        None,
                        0,
                        &mut view,
                        &mut load,
                        true,
                    );
                }
                if stopped && session.scopes().is_empty() {
                    ui.label(RichText::new("Loading variables…").color(*theme::TEXT_INFO));
                }
                if !stopped && !session.is_terminated() {
                    ui.label(RichText::new("The program is running").color(*theme::TEXT_INFO));
                }
            });
    }
    ui.data_mut(|data| data.insert_temp(view_id(), view));
    // Commands change the session: release the read guard first.
    drop(debugger_state);
    if let Some(frame) = select_frame {
        app.dispatch("debug.selectFrame", json!({"frame": frame}));
    }
    if let Some(reference) = load {
        app.dispatch("debug.loadVariables", json!({"reference": reference}));
    }
    if let Some(expression) = evaluate {
        app.dispatch("debug.evaluate", json!({"expression": expression}));
    }
    for (command, params) in watch_actions {
        app.dispatch(command, params);
    }
}

/// The Watches section: one row per watch with its value, edit/remove buttons and an
/// "add watch" field. Changes are returned as commands for the caller to dispatch.
fn watches_panel(
    ui: &mut egui::Ui,
    session: &Session,
    view: &mut View,
    actions: &mut Vec<(&'static str, Value)>,
) {
    ui.label(RichText::new("Watches").color(*theme::TEXT_SECONDARY));
    for (index, watch) in session.watches().iter().enumerate() {
        if let Some((editing, text)) = view.editing_watch.as_mut()
            && *editing == index
        {
            let response = ui.add(
                egui::TextEdit::singleline(text)
                    .id(egui::Id::new("debug_watch_edit"))
                    .desired_width(f32::INFINITY)
                    .font(egui::FontId::monospace(theme::UI_FONT_SIZE - 0.5)),
            );
            // Keep the field focused while editing, but let Enter/Escape end the edit.
            if !response.lost_focus() {
                response.request_focus();
            }
            let escape = ui.input(|input| input.key_pressed(egui::Key::Escape));
            if response.lost_focus() && !escape {
                actions.push((
                    "debug.editWatch",
                    json!({"index": index, "expression": text.clone()}),
                ));
            }
            if response.lost_focus() || escape {
                view.editing_watch = None;
            }
            continue;
        }
        // Hover highlight is painted behind the row once its size is known.
        let background = ui.painter().add(egui::Shape::Noop);
        let row = ui.horizontal(|ui| {
            let (text, color) = match &watch.result {
                Some(Ok(value)) => (
                    format!("{} = {value}", watch.expression),
                    *theme::editor::TEXT,
                ),
                Some(Err(error)) => (format!("{}: {error}", watch.expression), *theme::ERROR),
                None => (format!("{} = …", watch.expression), *theme::TEXT_INFO),
            };
            ui.label(RichText::new(text).monospace().color(color));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("×").on_hover_text("Remove Watch").clicked() {
                    actions.push(("debug.removeWatch", json!({"index": index})));
                }
                if ui
                    .small_button("Edit")
                    .on_hover_text("Edit Watch (or double-click the row)")
                    .clicked()
                {
                    view.editing_watch = Some((index, watch.expression.clone()));
                }
            });
        });
        let rect = row.response.rect;
        let area = ui.interact(
            rect,
            ui.id().with(("watch_row", index)),
            egui::Sense::click(),
        );
        if area.hovered() {
            ui.painter().set(
                background,
                egui::Shape::rect_filled(rect.shrink2(vec2(0.0, -1.0)), 3.0, *theme::HOVER),
            );
        }
        if area.double_clicked() {
            view.editing_watch = Some((index, watch.expression.clone()));
        }
    }
    let response = ui.add(
        egui::TextEdit::singleline(&mut view.watch_input)
            .id(egui::Id::new("debug_watch_input"))
            .hint_text(RichText::new("Add watch (Enter)").color(*theme::TEXT_DISABLED))
            .desired_width(f32::INFINITY)
            .font(egui::FontId::monospace(theme::UI_FONT_SIZE - 0.5)),
    );
    if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
        let expression = std::mem::take(&mut view.watch_input);
        if !expression.trim().is_empty() {
            actions.push(("debug.addWatch", json!({"expression": expression})));
        }
        response.request_focus();
    }
    ui.add_space(4.0);
}

#[allow(clippy::too_many_arguments)]
fn variable_rows(
    ui: &mut egui::Ui,
    session: &Session,
    reference: i64,
    name: &str,
    value: Option<(&str, Option<&str>)>,
    depth: usize,
    view: &mut View,
    load: &mut Option<i64>,
    scope: bool,
) {
    let expandable = reference > 0;
    // Scopes start expanded (like "Locals" in the reference); variables start collapsed.
    let open = expandable && (view.expanded.contains(&reference) != scope);
    let (rect, response) = list_row(ui, false);
    let mut x = rect.left() + 8.0 + depth as f32 * 16.0;
    if expandable {
        icons::paint(
            ui.painter(),
            egui::Rect::from_min_size(egui::pos2(x, rect.center().y - 7.0), vec2(14.0, 14.0)),
            if open {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            },
            *theme::TEXT_SECONDARY,
        );
    }
    x += 18.0;
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    match value {
        None => {
            row_text(
                ui,
                rect,
                x,
                name,
                theme::semibold(theme::UI_FONT_SIZE),
                *theme::TEXT_SECONDARY,
            );
        }
        Some((value, type_name)) => {
            x = row_text(
                ui,
                rect,
                x,
                name,
                font.clone(),
                Color32::from_rgb(0xC7, 0x7D, 0xBB),
            );
            x = row_text(ui, rect, x, " = ", font.clone(), *theme::TEXT_INFO);
            if let Some(type_name) = type_name.filter(|t| !t.is_empty()) {
                x = row_text(
                    ui,
                    rect,
                    x,
                    &format!("{{{type_name}}} "),
                    font.clone(),
                    *theme::TEXT_INFO,
                );
            }
            row_text(
                ui,
                rect,
                x,
                value,
                egui::FontId::monospace(theme::UI_FONT_SIZE - 0.5),
                *theme::editor::TEXT,
            );
        }
    }
    if expandable && response.clicked() {
        if !view.expanded.remove(&reference) {
            view.expanded.insert(reference);
        }
        if !open && session.variables(reference).is_none() {
            *load = Some(reference);
        }
    }
    if open {
        match session.variables(reference) {
            Some(children) => {
                for child in children.iter().take(500) {
                    variable_rows(
                        ui,
                        session,
                        child.reference,
                        &child.name,
                        Some((&child.value, child.type_name.as_deref())),
                        depth + 1,
                        view,
                        load,
                        false,
                    );
                }
            }
            None => {
                if load.is_none() {
                    *load = Some(reference);
                }
            }
        }
    }
}

fn console(ui: &mut egui::Ui, session: &Session) {
    egui::ScrollArea::both()
        .id_salt("debug_console")
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let text: String = session
                .output()
                .iter()
                .map(|line| line.text.as_str())
                .collect();
            let mut job = egui::text::LayoutJob::default();
            for line in session.output() {
                let color = match line.category {
                    OutputCategory::Stdout => *theme::editor::TEXT,
                    OutputCategory::Stderr => *theme::ERROR,
                    OutputCategory::Console => *theme::TEXT_INFO,
                };
                job.append(
                    &line.text,
                    0.0,
                    egui::TextFormat::simple(
                        egui::FontId::monospace(theme::UI_FONT_SIZE - 0.5),
                        color,
                    ),
                );
            }
            if text.is_empty() {
                ui.label(RichText::new("No output yet").color(*theme::TEXT_INFO));
            } else {
                ui.label(job);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_dap::LaunchRequest;
    use std::collections::BTreeMap;

    #[test]
    fn status_text_follows_session_state() {
        let session = Session::new(
            LaunchRequest {
                adapter_id: "lldb-dap".into(),
                attach: false,
                arguments: json!({}),
            },
            BTreeMap::new(),
        );
        assert_eq!(status_text(&session).0, "Starting debugger…");
    }
}

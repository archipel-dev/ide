//! Build / Run tool window: execution tree with timings on the left, console on the right.

use crate::console_links::{self, LinkTarget};
use crate::icons::{self, Icon};
use crate::widgets::{self, tool_window_header};
use crate::{IdeApp, theme};
use archipel_ide_core::{BuildSnapshot, BuildTab, ToolWindow};
use archipel_ide_runner::{
    BuildEvents, ConsoleBuffer, MessageKind, OutputStream, StepOutcome, TaskKind, TaskPlan,
    TaskState,
};
use eframe::egui::{self, Color32, RichText, vec2};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

const CONSOLE_ROW: f32 = 18.0;

/// Navigation requested from the tree (applied after painting).
enum Action {
    Select(usize, Option<usize>),
    Open {
        node: usize,
        path: String,
        line: u32,
        column: u32,
    },
}

#[derive(Clone, Copy, Default)]
struct View {
    /// Selected tree node id (0 = root, 1 + step, 1_000_000 + message).
    selected: Option<usize>,
    /// Console line to reveal.
    reveal: Option<usize>,
}

fn view_id() -> egui::Id {
    egui::Id::new("build_window_view")
}

/// `3 sec, 520 ms` / `818 ms` / `2 min, 5 sec`, as the reference shows durations.
pub fn format_duration(duration: Duration) -> String {
    let millis = duration.as_millis();
    if millis < 1000 {
        return format!("{millis} ms");
    }
    let secs = duration.as_secs();
    if secs < 60 {
        return format!("{secs} sec, {} ms", millis % 1000);
    }
    let minutes = secs / 60;
    if minutes < 60 {
        return format!("{minutes} min, {} sec", secs % 60);
    }
    format!("{} h, {} min", minutes / 60, minutes % 60)
}

/// Header subject of a run/test task: `<project> [<task>]`. Plans that already
/// carry the project prefix (Gradle) are not wrapped a second time.
pub(crate) fn run_subject(project: &str, plan_name: &str) -> String {
    if plan_name.starts_with(&format!("{project} [")) {
        plan_name.to_owned()
    } else {
        format!("{project} [{plan_name}]")
    }
}

/// What the window shows: the live runner, or the last finished run of a tab.
struct Data<'a> {
    plan: &'a TaskPlan,
    state: &'a TaskState,
    events: &'a BuildEvents,
    console: &'a ConsoleBuffer,
    elapsed: Option<Duration>,
}

fn data<'a>(
    runner: &'a archipel_ide_runner::TaskService,
    snapshot: Option<&'a BuildSnapshot>,
) -> Option<Data<'a>> {
    match snapshot {
        Some(snapshot) => Some(Data {
            plan: &snapshot.plan,
            state: &snapshot.state,
            events: &snapshot.events,
            console: &snapshot.console,
            elapsed: snapshot.elapsed,
        }),
        None => Some(Data {
            plan: runner.plan()?,
            state: runner.state(),
            events: runner.events(),
            console: runner.console(),
            elapsed: runner.elapsed(),
        }),
    }
}

pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    // Build and Sync results live in tabs of their own; Run and Test replace
    // the content like before.
    let live_tab = app.state.live_build_tab();
    let tab = app.state.execution().build_window().tab;
    let tabbed = live_tab.is_some()
        || (app.state.execution().runner().plan().is_none()
            && BuildTab::ALL.iter().any(|tab| {
                app.state
                    .execution()
                    .build_window()
                    .snapshot(*tab)
                    .is_some()
            }));
    let snapshot: Option<Arc<BuildSnapshot>> =
        if tabbed && (live_tab != Some(tab) || app.state.execution().runner().plan().is_none()) {
            app.state.execution().build_window().snapshot(tab)
        } else {
            None
        };
    let showing_other_tab = tabbed && live_tab != Some(tab);
    let plan = if showing_other_tab && snapshot.is_none() {
        None
    } else {
        let execution = app.state.execution();
        let runner = execution.runner();
        data(&runner, snapshot.as_deref()).map(|data| data.plan.clone())
    };
    let kind = plan.as_ref().map_or(TaskKind::Build, |plan| plan.kind);
    let title = match kind {
        TaskKind::Run => "Run",
        TaskKind::Test => "Test",
        TaskKind::Sync | TaskKind::Build | TaskKind::Custom => "Build",
    };
    let running = snapshot.is_none() && !showing_other_tab && app.state.execution().is_running();
    let hide = tool_window_header(ui, title, |ui| {
        // "Fix with AI" next to a failed build/run (AI Assistant).
        crate::ai_actions::build_link(app, ui);
        if tabbed {
            for candidate in BuildTab::ALL {
                let label = RichText::new(candidate.title()).size(theme::UI_FONT_SIZE);
                if ui.selectable_label(candidate == tab, label).clicked() {
                    let name = if candidate == BuildTab::Sync {
                        "sync"
                    } else {
                        "output"
                    };
                    app.dispatch("build.showTab", json!({ "tab": name }));
                }
            }
        } else if let Some(plan) = &plan {
            ui.label(
                RichText::new(&plan.name)
                    .color(*theme::TEXT_SECONDARY)
                    .size(theme::UI_FONT_SIZE),
            );
        }
    });
    if hide {
        app.toggle_tool_window(ToolWindow::Build);
    }
    let Some(plan) = plan else {
        ui.add_space(16.0);
        ui.vertical_centered(|ui| {
            let empty = if tabbed {
                format!("No {} result yet", tab.title())
            } else {
                "Nothing has run yet".to_owned()
            };
            ui.label(RichText::new(empty).color(*theme::TEXT_SECONDARY));
            let hint = match &app.state.detected_build {
                Some(build) => format!(
                    "{} project · Build Project or Run from the toolbar",
                    build.kind.display_name()
                ),
                None if app.state.project.is_some() => {
                    "No supported build system (Cargo, Gradle, Maven, Bazel, npm, Go) was detected"
                        .into()
                }
                None => "Open a project to build it".into(),
            };
            ui.label(RichText::new(hint).color(*theme::TEXT_INFO));
        });
        return;
    };
    let mut view: View = ui.data(|data| data.get_temp(view_id())).unwrap_or_default();
    let mut action = None;
    let available = ui.available_rect_before_wrap();
    let toolbar_width = 34.0;
    let tree_width = (available.width() * 0.36).clamp(220.0, 520.0);
    let toolbar_rect =
        egui::Rect::from_min_size(available.min, vec2(toolbar_width, available.height()));
    let tree_rect = egui::Rect::from_min_size(
        available.min + vec2(toolbar_width, 0.0),
        vec2(tree_width, available.height()),
    );
    let console_rect = egui::Rect::from_min_max(
        egui::pos2(tree_rect.right() + 1.0, available.top()),
        available.max,
    );
    ui.allocate_rect(available, egui::Sense::hover());
    ui.painter().line_segment(
        [tree_rect.right_top(), tree_rect.right_bottom()],
        egui::Stroke::new(1.0_f32, *theme::TOOLBAR_SEPARATOR),
    );

    let mut toolbar = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(toolbar_rect.shrink2(vec2(4.0, 4.0)))
            .layout(egui::Layout::top_down(egui::Align::Center)),
    );
    toolbar.spacing_mut().item_spacing.y = 2.0;
    let rerun = app.commands.is_enabled("run.rerun", &app.state);
    if toolbar
        .add_enabled_ui(rerun, |ui| icons::button(ui, Icon::Rerun, "Rerun"))
        .inner
        .clicked()
    {
        app.dispatch("run.rerun", Value::Null);
    }
    let stop_color = if running {
        *theme::STOP_RED
    } else {
        *theme::TEXT_DISABLED
    };
    if toolbar
        .add_enabled_ui(running, |ui| {
            icons::button_colored(ui, Icon::Stop, stop_color, "Stop", false)
        })
        .inner
        .clicked()
    {
        app.dispatch("run.cancel", Value::Null);
    }

    let test_mode = snapshot.is_none()
        && kind == TaskKind::Test
        && (running || !app.state.execution().runner().tests().is_empty());
    if test_mode {
        let failed = app.commands.is_enabled("test.rerunFailed", &app.state);
        if toolbar
            .add_enabled_ui(failed, |ui| {
                icons::button_colored(ui, Icon::Rerun, *theme::ERROR, "Rerun Failed Tests", false)
            })
            .inner
            .clicked()
        {
            app.dispatch("test.rerunFailed", Value::Null);
        }
    }

    toolbar.add_space(4.0);
    let (wrap, scroll_end, clear) = (
        app.state.execution().build_window().soft_wrap,
        "build.console.scrollToEnd",
        "build.console.clear",
    );
    if icons::button_colored(
        &mut toolbar,
        Icon::SoftWrap,
        *theme::ICON,
        "Soft-Wrap",
        wrap,
    )
    .clicked()
    {
        app.dispatch("build.console.softWrap", Value::Null);
    }
    if icons::button(&mut toolbar, Icon::ScrollEnd, "Scroll to the End").clicked() {
        app.dispatch(scroll_end, Value::Null);
    }
    let can_clear = app.commands.is_enabled(clear, &app.state);
    if toolbar
        .add_enabled_ui(can_clear, |ui| icons::button(ui, Icon::Close, "Clear All"))
        .inner
        .clicked()
    {
        app.dispatch(clear, Value::Null);
    }

    let project_name = app.project_name();
    let project_root = app
        .state
        .project
        .as_ref()
        .map(|project| project.root().to_owned());
    let mut tree = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(tree_rect.shrink2(vec2(0.0, 4.0)))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    if test_mode {
        crate::test_view::tree(app, &mut tree, running);
    } else {
        egui::ScrollArea::both()
            .id_salt("build_tree")
            .auto_shrink([false, false])
            .show(&mut tree, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let execution = app.state.execution();
                let runner = execution.runner();
                let Some(shown) = data(&runner, snapshot.as_deref()) else {
                    return;
                };
                let state = shown.state.clone();
                let events = shown.events;
                let (icon, verb) = match &state {
                    TaskState::Running | TaskState::Idle => (None, "running…".to_owned()),
                    TaskState::Exited(0) => (
                        Some((Icon::Success, *theme::SUCCESS)),
                        match kind {
                            TaskKind::Build => "build finished".into(),
                            TaskKind::Sync => "sync finished".into(),
                            _ => "successful".into(),
                        },
                    ),
                    TaskState::Exited(code) => (
                        Some((Icon::Error, *theme::ERROR)),
                        match kind {
                            TaskKind::Build => format!("build failed (exit code {code})"),
                            TaskKind::Sync => format!("sync failed (exit code {code})"),
                            _ => format!("failed (exit code {code})"),
                        },
                    ),
                    TaskState::Cancelled => {
                        (Some((Icon::Stop, *theme::WARNING)), "cancelled".into())
                    }
                    TaskState::Failed(_) => {
                        (Some((Icon::Error, *theme::ERROR)), "could not start".into())
                    }
                };
                let subject = match kind {
                    TaskKind::Build | TaskKind::Sync => project_name.clone(),
                    _ => run_subject(&project_name, &plan.name),
                };
                let title = format!("{subject}:");
                let detail = match shown.elapsed {
                    Some(elapsed) => format!("{verb}  {}", format_duration(elapsed)),
                    None => verb.clone(),
                };
                let out = widgets::tree_row(
                    ui,
                    widgets::TreeRow {
                        selected: view.selected == Some(0),
                        icon,
                        reserve_icon: icon.is_none(),
                        font: theme::semibold(theme::UI_FONT_SIZE),
                        color: *theme::TEXT_BRIGHT,
                        secondary: Some((&detail, *theme::TEXT)),
                        ..widgets::TreeRow::new(&title)
                    },
                );
                if icon.is_none() {
                    spinner(ui, out.icon);
                }
                let response = out.response;
                if response.clicked() {
                    action = Some(Action::Select(0, Some(0)));
                }
                if let TaskState::Failed(reason) = &state {
                    message_row(ui, 1, Icon::Error, *theme::ERROR, reason, None, false);
                }
                let messages = events.messages();
                let paths = Paths {
                    root: project_root.as_deref(),
                    cwd: &plan.cwd,
                };
                for &id in events.orphan_messages() {
                    if let Some(clicked) = message_node(ui, &view, id, &messages[id], 1, &paths) {
                        action = Some(clicked);
                    }
                }
                for (index, step) in events.steps().iter().enumerate() {
                    let node = 1 + index;
                    let (icon, color) = match step.outcome {
                        StepOutcome::Failed => (Some(Icon::Error), *theme::ERROR),
                        StepOutcome::Running => (None, *theme::TEXT),
                        _ if step
                            .messages
                            .iter()
                            .any(|&id| messages[id].kind == MessageKind::Warning) =>
                        {
                            (Some(Icon::Warning), *theme::WARNING)
                        }
                        _ => (Some(Icon::Success), *theme::SUCCESS),
                    };
                    let mut detail = step.outcome.label().unwrap_or_default().to_owned();
                    if let Some(duration) = step.duration().filter(|d| !d.is_zero()) {
                        if !detail.is_empty() {
                            detail.push_str("  ");
                        }
                        detail.push_str(&format_duration(duration));
                    }
                    let out = widgets::tree_row(
                        ui,
                        widgets::TreeRow {
                            depth: 1,
                            selected: view.selected == Some(node),
                            icon: icon.map(|icon| (icon, color)),
                            reserve_icon: icon.is_none(),
                            secondary: (!detail.is_empty()).then_some((&detail, *theme::TEXT_INFO)),
                            ..widgets::TreeRow::new(&step.name)
                        },
                    );
                    if icon.is_none() {
                        spinner(ui, out.icon);
                    }
                    let response = out.response;
                    if response.clicked() {
                        action = Some(Action::Select(node, Some(step.console_line)));
                    }
                    for &id in &step.messages {
                        if let Some(clicked) = message_node(ui, &view, id, &messages[id], 2, &paths)
                        {
                            action = Some(clicked);
                        }
                    }
                }
                if let Some(summary) = events.summary() {
                    widgets::tree_row(
                        ui,
                        widgets::TreeRow {
                            depth: 1,
                            color: *theme::TEXT_INFO,
                            ..widgets::TreeRow::new(summary)
                        },
                    );
                }
            });
    }

    let mut console = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(console_rect.shrink2(vec2(10.0, 6.0)))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let command_line = if plan.program.is_empty() {
        String::new()
    } else {
        plan.command_line()
    };
    if test_mode {
        crate::test_view::header(app, &mut console);
        match crate::test_view::selected_case(app, ui.ctx()) {
            Some(case) => {
                if let Some((path, line)) =
                    crate::test_view::details(&mut console, &case, project_root.as_deref())
                {
                    action = Some(Action::Open {
                        node: 0,
                        path: path.display().to_string(),
                        line: line as u32,
                        column: 1,
                    });
                }
            }
            None => {
                let options = console_options(app);
                let execution = app.state.execution();
                let runner = execution.runner();
                if let Some(shown) = data(&runner, snapshot.as_deref()) {
                    action = console_view(
                        &mut console,
                        &shown,
                        &command_line,
                        view.reveal.take(),
                        options,
                    )
                    .or(action);
                }
            }
        }
    } else {
        let options = console_options(app);
        let execution = app.state.execution();
        let runner = execution.runner();
        if let Some(shown) = data(&runner, snapshot.as_deref()) {
            action = console_view(
                &mut console,
                &shown,
                &command_line,
                view.reveal.take(),
                options,
            )
            .or(action);
        }
    }

    match action {
        Some(Action::Select(node, reveal)) => {
            view.selected = Some(node);
            view.reveal = reveal;
        }
        Some(Action::Open {
            node,
            path,
            line,
            column,
        }) => {
            view.selected = Some(node);
            // Console links are relative to the task's working directory.
            let path = project_root
                .as_deref()
                .and_then(|root| project_relative(root, &plan.cwd, Path::new(&path)))
                .unwrap_or(path);
            app.dispatch("editor.openFile", json!({"path": path}));
            app.dispatch("editor.gotoLine", json!({"line": line, "column": column}));
        }
        None => {}
    }
    ui.data_mut(|data| data.insert_temp(view_id(), view));
}

const MESSAGE_NODE: usize = 1_000_000;

struct Paths<'a> {
    root: Option<&'a Path>,
    cwd: &'a Path,
}

fn message_node(
    ui: &mut egui::Ui,
    view: &View,
    id: usize,
    message: &archipel_ide_runner::BuildMessage,
    depth: usize,
    paths: &Paths,
) -> Option<Action> {
    let (icon, color) = match message.kind {
        MessageKind::Error => (Icon::Error, *theme::ERROR),
        MessageKind::Warning => (Icon::Warning, *theme::WARNING),
    };
    let location = message.file.as_ref().map(|file| {
        let name = file
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        match message.line {
            Some(line) => format!("{name}:{line}"),
            None => name,
        }
    });
    let node = MESSAGE_NODE + id;
    let response = message_row(
        ui,
        depth,
        icon,
        color,
        &message.text,
        location.as_deref(),
        view.selected == Some(node),
    );
    let target = message
        .file
        .as_ref()
        .zip(paths.root)
        .and_then(|(file, root)| project_relative(root, paths.cwd, file));
    if response.clicked()
        && let Some(path) = target
    {
        return Some(Action::Open {
            node,
            path,
            line: message.line.unwrap_or(1),
            column: message.column.unwrap_or(1),
        });
    }
    response
        .clicked()
        .then_some(Action::Select(node, Some(message.console_line)))
}

fn message_row(
    ui: &mut egui::Ui,
    depth: usize,
    icon: Icon,
    color: Color32,
    text: &str,
    location: Option<&str>,
    selected: bool,
) -> egui::Response {
    let out = widgets::tree_row(
        ui,
        widgets::TreeRow {
            depth,
            selected,
            icon: Some((icon, color)),
            secondary: location.map(|location| (location, *theme::TEXT_INFO)),
            ..widgets::TreeRow::new(text)
        },
    );
    out.response.on_hover_text(text)
}

/// Spinner for a running step, centred on the row's icon slot.
fn spinner(ui: &mut egui::Ui, slot: egui::Rect) {
    let mut child = ui.new_child(
        egui::UiBuilder::new().max_rect(egui::Rect::from_center_size(
            slot.center(),
            vec2(12.0, 12.0),
        )),
    );
    widgets::busy_spinner(&mut child, 12.0, *theme::ACCENT);
}

/// Height for the console viewport that holds whole rows only, so a view
/// stuck to the bottom never shows a half-cut first line under the header.
pub fn whole_rows_height(available: f32, row_height: f32) -> f32 {
    if row_height <= 0.0 {
        return available.max(0.0);
    }
    (available / row_height).floor().max(1.0) * row_height
}

/// Console toolbar state taken from the IDE state before painting.
#[derive(Clone, Copy)]
struct ConsoleOptions {
    soft_wrap: bool,
    scroll_to_end: bool,
    /// Console lines hidden by Clear.
    cleared: usize,
}

fn console_options(app: &mut IdeApp) -> ConsoleOptions {
    let execution = app.state.execution();
    let mut window = execution.build_window_mut();
    ConsoleOptions {
        soft_wrap: window.soft_wrap,
        scroll_to_end: window.take_scroll_to_end(),
        cleared: window.cleared(window.tab),
    }
}

/// Soft-wrapped consoles lay out at most this many trailing lines.
const WRAPPED_LINES: usize = 2000;

fn console_view(
    ui: &mut egui::Ui,
    shown: &Data<'_>,
    command_line: &str,
    reveal: Option<usize>,
    options: ConsoleOptions,
) -> Option<Action> {
    let console = shown.console;
    let all = console.lines();
    let dropped = console.dropped();
    // Clear hides the lines present when it was pressed.
    let hidden = options.cleared.saturating_sub(dropped).min(all.len());
    let lines = &all[hidden..];
    let partial = [OutputStream::Stdout, OutputStream::Stderr]
        .into_iter()
        .map(|stream| (stream, console.partial(stream)))
        .filter(|(_, text)| !text.is_empty())
        .collect::<Vec<_>>();
    let font = egui::FontId::monospace(theme::UI_FONT_SIZE - 0.5);
    if let TaskState::Failed(reason) = shown.state {
        // Nothing ran; show the complete, wrapped reason instead of an empty console.
        ui.label(RichText::new(reason).font(font).color(*theme::ERROR));
        return None;
    }
    // Line 0 echoes the command, like a terminal prompt (until cleared).
    let prompt = usize::from(options.cleared == 0 && !command_line.is_empty());
    let rows: Vec<(String, Color32)> = {
        let mut rows = Vec::with_capacity(lines.len() + partial.len() + 1);
        if prompt == 1 {
            rows.push((format!("$ {command_line}"), *theme::TEXT_INFO));
        }
        let skip = if options.soft_wrap {
            lines.len().saturating_sub(WRAPPED_LINES)
        } else {
            0
        };
        for line in &lines[skip..] {
            let color = match line.stream {
                OutputStream::Stdout => stdout_color(&line.text),
                OutputStream::Stderr => stderr_color(&line.text),
            };
            rows.push((line.text.clone(), color));
        }
        for (stream, text) in &partial {
            let color = match stream {
                OutputStream::Stdout => *theme::editor::TEXT,
                OutputStream::Stderr => stderr_color(text),
            };
            rows.push((text.to_string(), color));
        }
        rows
    };
    let mut area = egui::ScrollArea::both()
        .id_salt("build_console")
        .auto_shrink([false, false])
        .max_height(whole_rows_height(ui.available_height(), CONSOLE_ROW));
    if options.soft_wrap {
        area = egui::ScrollArea::vertical()
            .id_salt("build_console_wrapped")
            .auto_shrink([false, false]);
    }
    if options.scroll_to_end {
        area = area.vertical_scroll_offset(f32::MAX);
    } else if let Some(line) = reveal {
        let row = line.saturating_sub(dropped + hidden) + prompt;
        area = area.vertical_scroll_offset((row as f32 * CONSOLE_ROW - 40.0).max(0.0));
    } else {
        area = area.stick_to_bottom(true);
    }
    let mut clicked = None;
    let mut paint_row = |ui: &mut egui::Ui, text: &str, color: Color32| {
        if let Some(target) = console_row(ui, text, color, &font, options.soft_wrap) {
            clicked = Some(target);
        }
    };
    if options.soft_wrap {
        area.show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (text, color) in &rows {
                paint_row(ui, text, *color);
            }
        });
    } else {
        area.show_rows(ui, CONSOLE_ROW, rows.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for row in range {
                if let Some((text, color)) = rows.get(row) {
                    paint_row(ui, text, *color);
                }
            }
        });
    }
    clicked.map(|target| match target {
        LinkTarget::Url(url) => {
            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
            None
        }
        LinkTarget::File { path, line, column } => Some(Action::Open {
            node: 0,
            path,
            line,
            column,
        }),
    })?
}

/// Paints one console line; links are underlined and clickable. Returns the
/// clicked link.
fn console_row(
    ui: &mut egui::Ui,
    text: &str,
    color: Color32,
    font: &egui::FontId,
    wrap: bool,
) -> Option<LinkTarget> {
    let links = console_links::find(text);
    let width = ui.available_width().max(200.0);
    let mut job = egui::text::LayoutJob::default();
    let plain = egui::TextFormat::simple(font.clone(), color);
    let mut cursor = 0;
    for link in &links {
        job.append(&text[cursor..link.range.start], 0.0, plain.clone());
        let mut format = egui::TextFormat::simple(font.clone(), crate::navigation_ui::LINK);
        format.underline = egui::Stroke::new(1.0_f32, crate::navigation_ui::LINK);
        job.append(&text[link.range.clone()], 0.0, format);
        cursor = link.range.end;
    }
    job.append(&text[cursor..], 0.0, plain);
    if wrap {
        job.wrap.max_width = width;
    }
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let height = galley.size().y.max(CONSOLE_ROW);
    let sense = if links.is_empty() {
        egui::Sense::hover()
    } else {
        egui::Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), sense);
    let width_of_text = galley.size().x;
    let origin = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
    let hovered = response.hover_pos().and_then(|pointer| {
        let index = galley.cursor_from_pos(pointer - origin).index;
        let byte = text
            .char_indices()
            .nth(index)
            .map_or(text.len(), |(b, _)| b);
        links.iter().find(|link| link.range.contains(&byte))
    });
    ui.painter().galley(origin, galley, color);
    if !wrap && width_of_text > rect.width() {
        // Extend the content so horizontal scrolling reaches long lines.
        ui.allocate_exact_size(
            vec2(width_of_text - rect.width(), 0.0),
            egui::Sense::hover(),
        );
    }
    let link = hovered?;
    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    response.clicked().then(|| link.target.clone())
}

fn stdout_color(text: &str) -> Color32 {
    if text.starts_with("BUILD SUCCESSFUL") {
        *theme::SUCCESS
    } else if text.starts_with("BUILD FAILED") || text.starts_with("FAILURE:") {
        *theme::ERROR
    } else {
        *theme::editor::TEXT
    }
}

/// Tools write progress to stderr; only highlight real errors/warnings.
fn stderr_color(text: &str) -> Color32 {
    let trimmed = text.trim_start();
    if trimmed.starts_with("error") || trimmed.starts_with("ERROR") || trimmed.starts_with("e: ") {
        *theme::ERROR
    } else if trimmed.starts_with("warning")
        || trimmed.starts_with("WARNING")
        || trimmed.starts_with("w: ")
    {
        *theme::WARNING
    } else {
        *theme::editor::TEXT
    }
}

/// Converts a tool-reported path to a project-relative one the editor can open.
pub fn project_relative(root: &Path, cwd: &Path, file: &Path) -> Option<String> {
    let absolute = if file.is_absolute() {
        file.to_owned()
    } else {
        cwd.join(file)
    };
    absolute
        .strip_prefix(root)
        .ok()
        .map(|relative| relative.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stderr_lines_are_only_red_for_errors() {
        assert_eq!(stderr_color("error[E0308]: mismatched"), *theme::ERROR);
        assert_eq!(stderr_color("   Compiling foo"), *theme::editor::TEXT);
        assert_eq!(stdout_color("BUILD SUCCESSFUL in 4s"), *theme::SUCCESS);
    }

    #[test]
    fn viewport_holds_whole_rows() {
        assert_eq!(whole_rows_height(250.0, 18.0), 234.0);
        assert_eq!(whole_rows_height(10.0, 18.0), 18.0);
        assert_eq!(whole_rows_height(100.0, 0.0), 100.0);
    }

    #[test]
    fn durations_read_like_the_reference() {
        assert_eq!(format_duration(Duration::from_millis(818)), "818 ms");
        assert_eq!(
            format_duration(Duration::from_millis(3520)),
            "3 sec, 520 ms"
        );
        assert_eq!(format_duration(Duration::from_secs(125)), "2 min, 5 sec");
        assert_eq!(format_duration(Duration::from_secs(3720)), "1 h, 2 min");
    }

    #[test]
    fn tool_paths_become_project_relative() {
        let root = Path::new("/w/demo");
        assert_eq!(
            project_relative(root, root, Path::new("src/main.rs")).as_deref(),
            Some("src/main.rs")
        );
        assert_eq!(
            project_relative(root, root, Path::new("/w/demo/app/A.java")).as_deref(),
            Some("app/A.java")
        );
        assert_eq!(
            project_relative(root, root, Path::new("/usr/lib/x.h")),
            None
        );
    }
}

#[cfg(test)]
mod run_subject_tests {
    use super::run_subject;

    #[test]
    fn header_is_not_nested_twice() {
        assert_eq!(
            run_subject("java-gradle", "java-gradle [run]"),
            "java-gradle [run]"
        );
        assert_eq!(run_subject("demo", "cargo run"), "demo [cargo run]");
    }
}

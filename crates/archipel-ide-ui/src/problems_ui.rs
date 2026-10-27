//! Problem highlighting around the editor and the Problems tool window:
//! squiggles, hover tooltips, the error stripe next to the scrollbar, the
//! inspection widget (`❗1 ⚠2 ˄ ˅`) at the editor's top right, and the File /
//! Project Errors tabs. The data comes from `archipel_ide_core::problems`.

use crate::icons::{self, Icon};
use crate::widgets::{self, row_text};
use crate::{IdeApp, theme};
use archipel_ide_core::{Problem, ProblemCounts, Severity, ToolWindow};
use eframe::egui::{self, Color32, Rect, Sense, Stroke, vec2};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Width of the error stripe right of the editor text.
pub const STRIPE_WIDTH: f32 = 12.0;

pub fn icon(severity: Severity) -> Icon {
    match severity {
        Severity::Error => Icon::Error,
        Severity::Warning => Icon::Warning,
        Severity::WeakWarning | Severity::Hint => Icon::WeakWarning,
    }
}

fn effect_color(severity: Severity) -> Option<Color32> {
    match severity {
        Severity::Error => Some(*theme::editor::ERROR_EFFECT),
        Severity::Warning => Some(*theme::editor::WARNING_EFFECT),
        Severity::WeakWarning => Some(*theme::editor::WEAK_WARNING_EFFECT),
        Severity::Hint => None,
    }
}

fn stripe_color(severity: Severity) -> Color32 {
    match severity {
        Severity::Error => *theme::editor::ERROR_STRIPE,
        Severity::Warning => *theme::editor::WARNING_STRIPE,
        _ => *theme::editor::WEAK_WARNING_EFFECT,
    }
}

/// Character range of `problem` on `line`; empty ranges cover one character.
fn columns_on_line(problem: &Problem, line: usize, line_len: usize) -> (usize, usize) {
    let start = if problem.line == line {
        problem.column
    } else {
        0
    };
    let end = if problem.end_line == line {
        problem.end_column
    } else {
        line_len
    };
    let start = start.min(line_len);
    (start, end.min(line_len).max(start + 1))
}

/// Paints the effects of every problem touching `line`: squiggles by
/// severity, unused code faded out and deprecated code struck through.
pub fn paint_line(
    painter: &egui::Painter,
    problems: &[Problem],
    line: usize,
    line_len: usize,
    column_x: impl Fn(usize) -> f32,
    row: Rect,
) {
    for problem in problems.iter().filter(|p| p.on_line(line)) {
        let (start, end) = columns_on_line(problem, line, line_len);
        let (x1, x2) = (column_x(start), column_x(end));
        if problem.unnecessary {
            let fade = Rect::from_min_max(egui::pos2(x1, row.top()), egui::pos2(x2, row.bottom()));
            painter.rect_filled(fade, 0.0, theme::ISLAND.gamma_multiply(0.55));
        }
        if problem.deprecated {
            let y = row.center().y;
            painter.line_segment(
                [egui::pos2(x1, y), egui::pos2(x2, y)],
                Stroke::new(1.0_f32, theme::editor::TEXT),
            );
        }
        // Unused code is only greyed out, like the reference IDE.
        if problem.unnecessary && problem.severity != Severity::Error {
            continue;
        }
        if let Some(color) = effect_color(problem.severity) {
            squiggle(painter, x1, x2, row.bottom() - 2.0, color);
        }
    }
}

/// Wavy underline used for problems and for error propagation in the tree
/// and tabs.
pub fn squiggle(painter: &egui::Painter, x1: f32, x2: f32, y: f32, color: Color32) {
    let step = 2.0;
    let mut points = Vec::new();
    let mut x = x1;
    let mut up = true;
    while x <= x2.max(x1 + 6.0) {
        points.push(egui::pos2(x, if up { y - 1.5 } else { y + 0.5 }));
        x += step;
        up = !up;
    }
    painter.add(egui::Shape::line(points, Stroke::new(1.0_f32, color)));
}

/// Problems under the pointer at a character position.
pub fn problems_at(
    problems: &[Problem],
    line: usize,
    column: usize,
    line_len: usize,
) -> Vec<&Problem> {
    problems
        .iter()
        .filter(|problem| problem.severity.listed() || problem.unnecessary || problem.deprecated)
        .filter(|problem| problem.on_line(line))
        .filter(|problem| {
            let (start, end) = columns_on_line(problem, line, line_len);
            (start..end).contains(&column)
        })
        .collect()
}

fn problem_tooltip(ui: &mut egui::Ui, problems: &[&Problem]) {
    ui.set_max_width(520.0);
    for problem in problems {
        ui.horizontal_top(|ui| {
            icons::show(ui, icon(problem.severity), 14.0, icons::ICON);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(&problem.message).color(theme::TEXT_BRIGHT));
                let origin = match (&problem.source, &problem.code) {
                    (Some(source), Some(code)) => format!("{source}({code})"),
                    (Some(source), None) => source.clone(),
                    (None, Some(code)) => code.clone(),
                    (None, None) => String::new(),
                };
                if !origin.is_empty() {
                    ui.label(egui::RichText::new(origin).color(theme::TEXT_INFO));
                }
            });
        });
    }
}

/// Shows the hover tooltip for problems at `line`/`column`.
pub fn hover_tooltip(
    response: &egui::Response,
    problems: &[Problem],
    line: usize,
    column: usize,
    line_len: usize,
) {
    let hovered = problems_at(problems, line, column, line_len);
    if !hovered.is_empty() {
        response
            .clone()
            .on_hover_ui_at_pointer(|ui| problem_tooltip(ui, &hovered));
    }
}

/// Error stripe: one mark per problem, scaled to the whole document.
/// Returns the problem position to jump to when a mark is clicked.
pub fn error_stripe(
    ui: &mut egui::Ui,
    rect: Rect,
    problems: &[Problem],
    line_count: usize,
    row_height: f32,
) -> Option<(usize, usize)> {
    let response = ui.interact(rect, ui.id().with("editor_error_stripe"), Sense::click());
    // Marks line up with their rows until the document is taller than the
    // editor; then the whole document is scaled onto the stripe.
    let usable = (rect.height() - 4.0).max(1.0);
    let document_height = (line_count.max(1) as f32 * row_height).max(usable);
    let mark_y = |problem: &Problem| {
        rect.top() + 2.0 + (problem.line as f32 + 0.5) * row_height * usable / document_height - 1.5
    };
    let mut listed: Vec<&Problem> = problems.iter().filter(|p| p.severity.listed()).collect();
    // Errors are painted last so they stay visible over warnings.
    listed.sort_by_key(|problem| std::cmp::Reverse(problem.severity));
    let painter = ui.painter_at(rect);
    for problem in &listed {
        let y = mark_y(problem);
        painter.rect_filled(
            Rect::from_min_max(
                egui::pos2(rect.left() + 3.0, y),
                egui::pos2(rect.right() - 3.0, y + 3.0),
            ),
            0.0,
            stripe_color(problem.severity),
        );
    }
    let pointer = response.hover_pos()?;
    let nearest = listed
        .iter()
        .rev()
        .filter(|problem| (mark_y(problem) + 1.5 - pointer.y).abs() <= 4.0)
        .min_by(|a, b| {
            let da = (mark_y(a) - pointer.y).abs();
            let db = (mark_y(b) - pointer.y).abs();
            da.total_cmp(&db)
        })
        .copied()?;
    let clicked = response.clicked();
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_ui_at_pointer(|ui| problem_tooltip(ui, &[nearest]));
    clicked.then_some((nearest.line, nearest.column))
}

pub enum WidgetAction {
    Next,
    Previous,
    ShowProblems,
}

/// The inspection widget at the editor's top right: counts per severity and
/// previous/next arrows, or a green check when the file is clean.
pub fn inspection_widget(
    ui: &mut egui::Ui,
    top_right: egui::Pos2,
    counts: ProblemCounts,
) -> Option<WidgetAction> {
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    let mut x = top_right.x - 6.0;
    let y = top_right.y + 13.0;
    let mut action = None;
    let painter = ui.painter().clone();
    if counts.total() == 0 {
        let rect = Rect::from_center_size(egui::pos2(x - 8.0, y), vec2(16.0, 16.0));
        let stroke = Stroke::new(1.6_f32, theme::SUCCESS);
        painter.line_segment(
            [
                rect.left_center() + vec2(2.0, 0.0),
                rect.center_bottom() + vec2(-1.0, -4.0),
            ],
            stroke,
        );
        painter.line_segment(
            [
                rect.center_bottom() + vec2(-1.0, -4.0),
                rect.right_top() + vec2(-2.0, 4.0),
            ],
            stroke,
        );
        ui.interact(rect, ui.id().with("inspection_widget_ok"), Sense::hover())
            .on_hover_text("No problems found");
        return None;
    }
    for (icon, forward) in [(Icon::ChevronDown, true), (Icon::ChevronUp, false)] {
        let rect = Rect::from_center_size(egui::pos2(x - 9.0, y), vec2(18.0, 18.0));
        let id = ui.id().with(("inspection_widget_arrow", forward));
        let response = ui.interact(rect, id, Sense::click());
        if response.hovered() {
            painter.rect_filled(rect, 4.0, theme::HOVER);
        }
        icons::paint(&painter, rect.shrink(2.0), icon, theme::TEXT_SECONDARY);
        let tooltip = if forward {
            format!("Next Highlighted Error ({})", crate::keys::display("F2"))
        } else {
            format!(
                "Previous Highlighted Error ({})",
                crate::keys::display("⇧F2")
            )
        };
        if response.on_hover_text(tooltip).clicked() {
            action = Some(if forward {
                WidgetAction::Next
            } else {
                WidgetAction::Previous
            });
        }
        x -= 20.0;
    }
    x -= 4.0;
    let mut tooltip = Vec::new();
    for (count, icon, singular, plural) in [
        (
            counts.weak_warnings,
            Icon::WeakWarning,
            "weak warning",
            "weak warnings",
        ),
        (counts.warnings, Icon::Warning, "warning", "warnings"),
        (counts.errors, Icon::Error, "error", "errors"),
    ] {
        if count == 0 {
            continue;
        }
        let galley =
            painter.layout_no_wrap(count.to_string(), font.clone(), *theme::TEXT_SECONDARY);
        let width = 14.0 + 3.0 + galley.size().x;
        let start = x - width;
        icons::paint(
            &painter,
            Rect::from_min_size(egui::pos2(start, y - 7.0), vec2(14.0, 14.0)),
            icon,
            icons::ICON,
        );
        painter.galley(
            egui::pos2(start + 17.0, y - galley.size().y / 2.0),
            galley,
            *theme::TEXT_SECONDARY,
        );
        x = start - 8.0;
        tooltip.insert(0, widgets::pluralize(count, singular, plural));
    }
    let counts_rect = Rect::from_min_max(
        egui::pos2(x + 4.0, y - 10.0),
        egui::pos2(top_right.x - 48.0, y + 10.0),
    );
    let response = ui.interact(
        counts_rect,
        ui.id().with("inspection_widget_counts"),
        Sense::click(),
    );
    if response.hovered() {
        painter.rect_filled(counts_rect, 4.0, theme::HOVER);
    }
    if response
        .on_hover_text(format!(
            "{}\nClick to open the Problems tool window",
            tooltip.join(", ")
        ))
        .clicked()
    {
        action = Some(WidgetAction::ShowProblems);
    }
    action
}

/// Opens `path` (project-relative) and puts the caret on the problem.
pub fn jump_to(app: &mut IdeApp, ctx: &egui::Context, path: &Path, line: usize, column: usize) {
    app.dispatch("editor.openFile", json!({"path": path}));
    let active = app
        .state
        .active_document
        .and_then(|index| app.state.documents.get(index));
    if active.is_some_and(|document| document.path == path) {
        crate::editor::move_caret_to(app, line, column);
        ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    File,
    ProjectErrors,
}

pub fn tool_window(app: &mut IdeApp, ui: &mut egui::Ui) {
    let tab_id = egui::Id::new("problems_tab");
    let mut tab: Tab = ui.data(|data| data.get_temp(tab_id).unwrap_or_default());
    app.state.refresh_problems();
    let active_path = app
        .state
        .active_document
        .and_then(|index| app.state.documents.get(index))
        .map(|document| document.path.clone());
    let file_problems: Vec<Problem> = app
        .state
        .active_problems()
        .into_iter()
        .filter(|problem| problem.severity.listed())
        .collect();
    let project_errors: Vec<(PathBuf, Vec<Problem>)> = app
        .state
        .problems
        .files()
        .iter()
        .map(|file| {
            let errors = file
                .problems
                .iter()
                .filter(|problem| problem.severity == Severity::Error)
                .cloned()
                .collect::<Vec<_>>();
            (file.path.clone(), errors)
        })
        .filter(|(_, errors)| !errors.is_empty())
        .collect();
    let project_error_count: usize = project_errors.iter().map(|(_, errors)| errors.len()).sum();

    // Header: title, tabs with counts, hide button.
    let (header, _) = ui.allocate_exact_size(
        vec2(
            ui.available_width(),
            widgets::tool_window_header_height(ui.ctx()),
        ),
        Sense::hover(),
    );
    let mut x = header.left() + theme::layout::CONTENT_INSET;
    x = row_text(
        ui,
        header,
        x,
        "Problems",
        theme::semibold(theme::UI_FONT_SIZE),
        theme::TEXT_BRIGHT,
    ) + 18.0;
    for (candidate, label, count) in [
        (Tab::File, "File", file_problems.len()),
        (Tab::ProjectErrors, "Project Errors", project_error_count),
    ] {
        let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
        let label_width = ui
            .painter()
            .layout_no_wrap(label.into(), font.clone(), *theme::TEXT)
            .size()
            .x;
        let count_text = if count > 0 {
            count.to_string()
        } else {
            String::new()
        };
        let count_width = ui
            .painter()
            .layout_no_wrap(count_text.clone(), font.clone(), *theme::TEXT)
            .size()
            .x;
        let width = label_width + if count > 0 { count_width + 6.0 } else { 0.0 } + 16.0;
        let rect = Rect::from_min_size(egui::pos2(x, header.top()), vec2(width, header.height()));
        let response = ui.interact(rect, egui::Id::new(("problems_tab", label)), Sense::click());
        if response.hovered() {
            ui.painter()
                .rect_filled(rect.shrink2(vec2(0.0, 4.0)), 4.0, theme::HOVER);
        }
        let end = row_text(ui, rect, x + 8.0, label, font.clone(), theme::TEXT_BRIGHT);
        if count > 0 {
            row_text(ui, rect, end + 6.0, &count_text, font, theme::TEXT_INFO);
        }
        if tab == candidate {
            ui.painter().line_segment(
                [
                    egui::pos2(rect.left() + 2.0, rect.bottom() - 1.0),
                    egui::pos2(rect.right() - 2.0, rect.bottom() - 1.0),
                ],
                Stroke::new(2.0_f32, theme::ACCENT),
            );
        }
        if response.clicked() {
            tab = candidate;
        }
        x = rect.right() + 4.0;
    }
    let hide_rect = Rect::from_min_size(
        egui::pos2(header.right() - 34.0, header.center().y - 11.0),
        vec2(22.0, 22.0),
    );
    let hide = ui.interact(hide_rect, egui::Id::new("problems_hide"), Sense::click());
    if hide.hovered() {
        ui.painter().rect_filled(hide_rect, 4.0, theme::HOVER);
    }
    icons::paint(
        ui.painter(),
        hide_rect.shrink(3.0),
        Icon::Minimize,
        icons::ICON,
    );
    if hide.on_hover_text("Hide").clicked() {
        app.toggle_tool_window(ToolWindow::Problems);
    }
    // "Fix with AI" (AI Assistant), left of Hide.
    let can_fix = app.action_enabled("ai.action.fixProblems");
    let fix_rect = hide_rect.translate(vec2(-26.0, 0.0));
    let mut fix_ui = ui.new_child(egui::UiBuilder::new().max_rect(fix_rect));
    if crate::ai_actions::problems_button(&mut fix_ui, can_fix) {
        app.dispatch("ai.action.fixProblems", serde_json::Value::Null);
    }
    ui.data_mut(|data| data.insert_temp(tab_id, tab));

    let root = app
        .state
        .project
        .as_ref()
        .map(|project| project.root().to_owned());
    let mut jump = None;
    // Left toolbar: Preview, Quick Fix, Group by Severity.
    let options_id = egui::Id::new("problems_options");
    let mut options: Options = ui
        .data(|data| data.get_temp(options_id))
        .unwrap_or_default();
    let body = ui.available_rect_before_wrap();
    let toolbar_rect = Rect::from_min_size(body.min, vec2(34.0, body.height()));
    ui.painter().line_segment(
        [toolbar_rect.right_top(), toolbar_rect.right_bottom()],
        Stroke::new(1.0_f32, *theme::TOOLBAR_SEPARATOR),
    );
    let mut toolbar = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(toolbar_rect.shrink2(vec2(4.0, 4.0)))
            .layout(egui::Layout::top_down(egui::Align::Center)),
    );
    toolbar.spacing_mut().item_spacing.y = 2.0;
    if icons::button_colored(
        &mut toolbar,
        Icon::Find,
        *theme::ICON,
        "Preview",
        options.preview,
    )
    .clicked()
    {
        options.preview = !options.preview;
    }
    let can_fix_here = options.selected.is_some();
    if toolbar
        .add_enabled_ui(can_fix_here, |ui| {
            icons::button(
                ui,
                Icon::Warning,
                "Show Quick Fixes for the Selected Problem",
            )
        })
        .inner
        .clicked()
        && let Some((path, line, column)) = options.selected.clone()
    {
        jump_to(app, &ui.ctx().clone(), &path, line, column);
        app.dispatch("code.showIntentions", serde_json::Value::Null);
    }
    if icons::button_colored(
        &mut toolbar,
        Icon::Structure,
        *theme::ICON,
        "Group by Severity",
        options.group_by_severity,
    )
    .clicked()
    {
        options.group_by_severity = !options.group_by_severity;
    }
    let previews: HashMap<(PathBuf, usize), String> = if options.preview {
        let mut previews = HashMap::new();
        for (path, problems) in std::iter::once((active_path.clone(), file_problems.clone()))
            .filter_map(|(path, problems)| Some((path?, problems)))
            .chain(project_errors.iter().cloned())
        {
            if let Some(document) = app.state.documents.iter().find(|doc| doc.path == path) {
                for problem in &problems {
                    if let Some(line) = document.text.line(problem.line) {
                        previews.insert(
                            (path.clone(), problem.line),
                            line.trim_end_matches(['\r', '\n']).trim().to_owned(),
                        );
                    }
                }
            }
        }
        previews
    } else {
        HashMap::new()
    };
    let mut selected = options.selected.clone();
    let content_rect =
        Rect::from_min_max(egui::pos2(toolbar_rect.right() + 1.0, body.top()), body.max);
    let mut content = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(content_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let ui = &mut content;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let view = GroupView {
                group_by_severity: options.group_by_severity,
                previews: &previews,
                selected: options.selected.as_ref(),
            };
            match tab {
                Tab::File => {
                    let Some(path) = &active_path else {
                        empty(ui, "No file is open");
                        return;
                    };
                    if file_problems.is_empty() {
                        empty(ui, &format!("No problems in {}", IdeApp::file_name(path)));
                        return;
                    }
                    file_group(ui, root.as_deref(), path, &file_problems, &view, &mut jump);
                }
                Tab::ProjectErrors => {
                    if project_errors.is_empty() {
                        empty(ui, "No errors found in the project");
                        return;
                    }
                    for (path, errors) in &project_errors {
                        file_group(ui, root.as_deref(), path, errors, &view, &mut jump);
                    }
                }
            }
        });
    if let Some((path, line, column)) = jump {
        selected = Some((path.clone(), line, column));
        jump_to(app, &ui.ctx().clone(), &path, line, column);
    }
    options.selected = selected;
    ui.data_mut(|data| data.insert_temp(options_id, options));
}

/// View switches of the Problems tool window (left toolbar) and the problem
/// last clicked.
#[derive(Clone, Default)]
struct Options {
    preview: bool,
    group_by_severity: bool,
    selected: Option<(PathBuf, usize, usize)>,
}

struct GroupView<'a> {
    group_by_severity: bool,
    previews: &'a HashMap<(PathBuf, usize), String>,
    selected: Option<&'a (PathBuf, usize, usize)>,
}

/// `/very/long/path/to/dir` shortened to `…/path/to/dir` within `max` chars.
fn elide_path(path: &str, max: usize) -> String {
    if path.chars().count() <= max {
        return path.to_owned();
    }
    let mut kept: Vec<&str> = Vec::new();
    let mut length = 2;
    for part in path.rsplit(['/', '\\']).filter(|part| !part.is_empty()) {
        if length + part.chars().count() + 1 > max && !kept.is_empty() {
            break;
        }
        length += part.chars().count() + 1;
        kept.push(part);
    }
    kept.reverse();
    format!("…/{}", kept.join("/"))
}

fn empty(ui: &mut egui::Ui, text: &str) {
    ui.add_space(theme::layout::SECTION_SPACING);
    ui.horizontal(|ui| {
        ui.add_space(theme::layout::CONTENT_INSET);
        widgets::icon_label(
            ui,
            Icon::Success,
            *theme::SUCCESS,
            text,
            *theme::TEXT_SECONDARY,
        );
    });
}

/// `App.java  /abs/dir  2 problems` followed by one row per problem.
fn file_group(
    ui: &mut egui::Ui,
    root: Option<&Path>,
    path: &Path,
    problems: &[Problem],
    view: &GroupView<'_>,
    jump: &mut Option<(PathBuf, usize, usize)>,
) {
    let name = IdeApp::file_name(path);
    let directory = root
        .map(|root| root.join(path))
        .unwrap_or_else(|| path.to_owned());
    let location = directory
        .parent()
        .map(|parent| elide_path(&parent.display().to_string(), 48));
    let count = widgets::pluralize(problems.len(), "problem", "problems");
    let secondary = match &location {
        Some(location) => format!("{location}  {count}"),
        None => count,
    };
    widgets::tree_row(
        ui,
        widgets::TreeRow {
            icon: Some((Icon::for_file(&name), *icons::ICON)),
            color: *theme::TEXT_BRIGHT,
            secondary: Some((&secondary, *theme::TEXT_INFO)),
            ..widgets::TreeRow::new(&name)
        },
    );
    let mut ordered: Vec<&Problem> = problems.iter().collect();
    if view.group_by_severity {
        ordered.sort_by_key(|problem| problem.severity);
    }
    let mut section = None;
    for problem in ordered {
        if view.group_by_severity && section != Some(problem.severity) {
            section = Some(problem.severity);
            let count = problems
                .iter()
                .filter(|other| other.severity == problem.severity)
                .count();
            widgets::tree_row(
                ui,
                widgets::TreeRow {
                    depth: 1,
                    font: theme::semibold(theme::UI_FONT_SIZE),
                    color: *theme::TEXT_SECONDARY,
                    ..widgets::TreeRow::new(&format!(
                        "{} {count}",
                        severity_title(problem.severity)
                    ))
                },
            );
        }
        let chosen = view
            .selected
            .is_some_and(|(p, line, _)| p == path && *line == problem.line);
        let first_line = problem.message.lines().next().unwrap_or_default();
        let line_label = format!(":{}", problem.line + 1);
        let response = widgets::tree_row(
            ui,
            widgets::TreeRow {
                depth: 1,
                selected: chosen,
                icon: Some((icon(problem.severity), *icons::ICON)),
                secondary: Some((&line_label, *theme::TEXT_INFO)),
                ..widgets::TreeRow::new(first_line)
            },
        )
        .response;
        if response.on_hover_text(&problem.message).clicked() {
            *jump = Some((path.to_owned(), problem.line, problem.column));
        }
        if let Some(code) = view.previews.get(&(path.to_owned(), problem.line)) {
            widgets::tree_row(
                ui,
                widgets::TreeRow {
                    depth: 1,
                    reserve_icon: true,
                    font: egui::FontId::monospace(theme::UI_FONT_SIZE - 1.0),
                    color: *theme::TEXT_INFO,
                    ..widgets::TreeRow::new(code)
                },
            );
        }
    }
}

fn severity_title(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "Errors",
        Severity::Warning => "Warnings",
        Severity::WeakWarning => "Weak Warnings",
        Severity::Hint => "Hints",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem(line: usize, column: usize, end_line: usize, end_column: usize) -> Problem {
        Problem {
            severity: Severity::Error,
            message: "m".into(),
            source: None,
            code: None,
            line,
            column,
            end_line,
            end_column,
            unnecessary: false,
            deprecated: false,
        }
    }

    #[test]
    fn long_directories_are_elided_from_the_left() {
        assert_eq!(elide_path("/a/b", 48), "/a/b");
        assert_eq!(
            elide_path(
                "/home/user/projects/sample/src/main/java/com/example/app",
                30
            ),
            "…/main/java/com/example/app"
        );
    }

    #[test]
    fn hover_hits_the_problem_range_and_widens_empty_ranges() {
        let problems = [
            problem(2, 4, 2, 8),
            problem(5, 3, 5, 3),
            problem(7, 6, 9, 2),
        ];
        assert_eq!(problems_at(&problems, 2, 4, 20).len(), 1);
        assert_eq!(problems_at(&problems, 2, 8, 20).len(), 0);
        assert_eq!(
            problems_at(&problems, 5, 3, 20).len(),
            1,
            "empty range covers one char"
        );
        assert_eq!(
            problems_at(&problems, 8, 0, 20).len(),
            1,
            "middle line of a multi-line range"
        );
        assert_eq!(problems_at(&problems, 9, 2, 20).len(), 0);
        // A problem at the end of a line still has a one-character target.
        assert_eq!(columns_on_line(&problem(1, 10, 1, 10), 1, 10), (10, 11));
    }
}

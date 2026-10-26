//! Git tool window, Log tab: branches pane, filterable commit list with a
//! commit graph, and the selected commit's changed files and message.

use crate::icons::{self, Icon};
use crate::widgets::{self, row_text, tool_window_header};
use crate::{IdeApp, theme};
use archipel_ide_core::DateRange;
use archipel_ide_core::ToolWindow;
use archipel_ide_git::{ChangeKind, CommitFile, CommitInfo, GraphEdge, GraphRow};
use eframe::egui::{self, Color32, Rect, RichText, Sense, Stroke, pos2, vec2};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

const BRANCHES_WIDTH: f32 = 240.0;
const DETAILS_WIDTH: f32 = 320.0;
const ROW: f32 = 24.0;
const LANE: f32 = 14.0;
const DATE_WIDTH: f32 = 120.0;
const AUTHOR_WIDTH: f32 = 110.0;

/// Lane colours, cycled by column (observed log graph palette, approximate).
const LANE_COLORS: [Color32; 6] = [
    Color32::from_rgb(0x4E, 0x9D, 0x6C),
    Color32::from_rgb(0x56, 0xA8, 0xF5),
    Color32::from_rgb(0xD5, 0x96, 0x37),
    Color32::from_rgb(0xB1, 0x89, 0xF5),
    Color32::from_rgb(0xE3, 0x77, 0x74),
    Color32::from_rgb(0x2A, 0xAC, 0xB8),
];

fn lane_color(column: usize) -> Color32 {
    LANE_COLORS[column % LANE_COLORS.len()]
}

pub fn filter_id() -> egui::Id {
    egui::Id::new("git_log_filter")
}

pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    let hide = tool_window_header(ui, "Git", |ui| {
        if icons::button(ui, Icon::Refresh, "Refresh").clicked() {
            app.dispatch("git.log.refresh", Value::Null);
        }
    });
    if hide {
        app.toggle_tool_window(ToolWindow::Git);
        return;
    }
    if app.state.git.is_none() {
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("The project is not a Git repository.").color(theme::TEXT_SECONDARY),
            );
        });
        return;
    }
    if !app.state.git_log.loaded {
        app.dispatch("git.log.refresh", Value::Null);
    }
    let mut actions: Vec<(&'static str, Value)> = Vec::new();
    tab_strip(app, ui, &mut actions);
    let area = ui.available_rect_before_wrap();
    ui.allocate_rect(area, Sense::hover());
    let branches_rect = Rect::from_min_size(area.min, vec2(BRANCHES_WIDTH, area.height()));
    let details_left = if app.state.git_log.hide_details {
        area.right()
    } else {
        (area.right() - DETAILS_WIDTH).max(branches_rect.right() + 200.0)
    };
    let details_rect = Rect::from_min_max(pos2(details_left, area.top()), area.max);
    let commits_rect = Rect::from_min_max(
        pos2(branches_rect.right(), area.top()),
        pos2(details_rect.left(), area.bottom()),
    );
    let divider = Stroke::new(1.0_f32, theme::POPUP_BORDER);
    ui.painter()
        .vline(branches_rect.right(), area.y_range(), divider);
    ui.painter()
        .vline(details_rect.left(), area.y_range(), divider);
    ui.scope_builder(egui::UiBuilder::new().max_rect(branches_rect), |ui| {
        ui.set_clip_rect(ui.clip_rect().intersect(branches_rect));
        branches_pane(app, ui, &mut actions);
    });
    ui.scope_builder(egui::UiBuilder::new().max_rect(commits_rect), |ui| {
        ui.set_clip_rect(ui.clip_rect().intersect(commits_rect));
        commits_pane(app, ui, &mut actions);
    });
    if !app.state.git_log.hide_details {
        ui.scope_builder(egui::UiBuilder::new().max_rect(details_rect), |ui| {
            ui.set_clip_rect(ui.clip_rect().intersect(details_rect));
            details_pane(app, ui, &mut actions);
        });
    }
    for (id, params) in actions {
        app.dispatch(id, params);
    }
}

/// The `Log` tab and one closable `History: <file>` tab per opened file history.
fn tab_strip(app: &IdeApp, ui: &mut egui::Ui, actions: &mut Vec<(&'static str, Value)>) {
    let log = &app.state.git_log;
    let active = log.active_history_tab();
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
    let mut left = rect.left() + 12.0;
    let tab = |ui: &mut egui::Ui, left: &mut f32, label: String, selected: bool, close| {
        let color = if selected {
            *theme::TEXT_BRIGHT
        } else {
            *theme::TEXT_SECONDARY
        };
        let galley =
            ui.painter()
                .layout_no_wrap(label.clone(), theme::semibold(theme::UI_FONT_SIZE), color);
        let width = galley.size().x;
        let hit = Rect::from_min_size(
            pos2(*left - 4.0, rect.top()),
            vec2(width + 8.0 + if close { 18.0 } else { 0.0 }, rect.height()),
        );
        let response = ui.interact(hit, ui.id().with(("log_tab", &label)), Sense::click());
        ui.painter().galley(
            pos2(*left, rect.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
        if selected {
            ui.painter().hline(
                *left..=*left + width,
                rect.bottom() - 1.0,
                Stroke::new(2.0_f32, theme::ACCENT),
            );
        }
        let mut closed = false;
        if close {
            let x = Rect::from_center_size(
                pos2(*left + width + 12.0, rect.center().y),
                vec2(14.0, 14.0),
            );
            let close_response = ui
                .interact(x, ui.id().with(("log_tab_close", &label)), Sense::click())
                .on_hover_text("Close tab");
            ui.painter().text(
                x.center(),
                egui::Align2::CENTER_CENTER,
                "×",
                egui::FontId::proportional(theme::UI_FONT_SIZE),
                *theme::TEXT_SECONDARY,
            );
            closed = close_response.clicked();
        }
        *left += width + 20.0 + if close { 18.0 } else { 0.0 };
        (response.clicked() && !closed, closed)
    };
    if tab(ui, &mut left, "Log".to_owned(), active.is_none(), false).0 && active.is_some() {
        actions.push(("git.log.selectTab", json!({})));
    }
    for path in &log.history_tabs {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let (clicked, closed) = tab(
            ui,
            &mut left,
            format!("History: {name}"),
            active == Some(path),
            true,
        );
        if closed {
            actions.push(("git.log.closeTab", json!({ "path": path })));
        } else if clicked && active != Some(path) {
            actions.push(("git.log.selectTab", json!({ "path": path })));
        }
    }
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0_f32, theme::POPUP_BORDER),
    );
}

fn branches_pane(app: &IdeApp, ui: &mut egui::Ui, actions: &mut Vec<(&'static str, Value)>) {
    let log = &app.state.git_log;
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    egui::ScrollArea::vertical()
        .id_salt("git_log_branches")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add_space(4.0);
            let mut entry = |ui: &mut egui::Ui,
                             depth: usize,
                             icon: Option<Icon>,
                             label: &str,
                             filter: Option<&str>,
                             bold: bool| {
                let selected = log.branch.as_deref() == filter && filter.is_some();
                let paint = icon.map(|icon| {
                    move |painter: &egui::Painter, rect: Rect| {
                        icons::paint(painter, rect, icon, theme::WARNING)
                    }
                });
                let out = widgets::tree_row(
                    ui,
                    widgets::TreeRow {
                        depth,
                        selected,
                        paint_icon: paint.as_ref().map(|p| p as &dyn Fn(&egui::Painter, Rect)),
                        font: if bold {
                            theme::semibold(theme::UI_FONT_SIZE)
                        } else {
                            font.clone()
                        },
                        ..widgets::TreeRow::new(label)
                    },
                );
                let response = out.response;
                if response.clicked() {
                    let branch = if selected { None } else { filter };
                    actions.push(("git.log.filterBranch", json!({ "branch": branch })));
                }
                response
            };
            entry(ui, 0, None, "HEAD (Current Branch)", Some("HEAD"), false)
                .on_hover_text("Show only commits reachable from HEAD");
            group(ui, "Local");
            for branch in &log.branches.local {
                let current = log.branches.current.as_deref() == Some(branch.name.as_str());
                let mut label = branch.name.clone();
                if branch.ahead > 0 {
                    label.push_str(&format!("  ↑{}", branch.ahead));
                }
                if branch.behind > 0 {
                    label.push_str(&format!("  ↓{}", branch.behind));
                }
                let response = entry(
                    ui,
                    1,
                    Some(Icon::Branch),
                    &label,
                    Some(&branch.name),
                    current,
                );
                if let Some(upstream) = &branch.upstream {
                    response.on_hover_text(format!("Tracking {upstream}"));
                }
            }
            if !log.branches.remote.is_empty() {
                group(ui, "Remote");
                for branch in &log.branches.remote {
                    entry(ui, 1, Some(Icon::Branch), branch, Some(branch), false);
                }
            }
            if !log.branches.tags.is_empty() {
                group(ui, "Tags");
                for tag in &log.branches.tags {
                    entry(ui, 1, None, tag, Some(tag), false);
                }
            }
        });
}

fn group(ui: &mut egui::Ui, title: &str) {
    widgets::tree_row(
        ui,
        widgets::TreeRow {
            expanded: Some(true),
            ..widgets::TreeRow::new(title)
        },
    );
}

fn commits_pane(app: &IdeApp, ui: &mut egui::Ui, actions: &mut Vec<(&'static str, Value)>) {
    let log = &app.state.git_log;
    ui.add_space(4.0);
    widgets::toolbar_row(ui, |ui| {
        ui.add_space(theme::layout::CONTENT_INSET);
        let mut filter = log.filter.clone();
        let response = widgets::search_field(ui, filter_id(), &mut filter, "Text or hash", 120.0);
        let (mut regex, mut case) = (log.regex, log.match_case);
        let regex_changed = toggle(ui, &mut regex, ".*", "Regular expression");
        let case_changed = toggle(ui, &mut case, "Cc", "Match case");
        if response.changed() || regex_changed || case_changed {
            actions.push((
                "git.log.filterText",
                json!({ "text": filter, "regex": regex, "matchCase": case }),
            ));
        }
        if log.regex_error() {
            ui.label(RichText::new("Invalid regex").color(theme::ERROR));
        }
        ui.add_space(4.0);
        let label = match &log.branch {
            Some(branch) => format!("Branch: {branch}"),
            None => "Branch".to_owned(),
        };
        egui::ComboBox::from_id_salt("git_log_branch_filter")
            .selected_text(label)
            .width(90.0)
            .show_ui(ui, |ui| {
                if ui.selectable_label(log.branch.is_none(), "All").clicked() {
                    actions.push(("git.log.filterBranch", json!({})));
                }
                for branch in log
                    .branches
                    .local
                    .iter()
                    .map(|b| &b.name)
                    .chain(&log.branches.remote)
                {
                    if ui
                        .selectable_label(log.branch.as_ref() == Some(branch), branch)
                        .clicked()
                    {
                        actions.push(("git.log.filterBranch", json!({ "branch": branch })));
                    }
                }
            });
        egui::ComboBox::from_id_salt("git_log_user_filter")
            .selected_text(match &log.user {
                Some(user) => format!("User: {user}"),
                None => "User".to_owned(),
            })
            .width(80.0)
            .show_ui(ui, |ui| {
                if ui.selectable_label(log.user.is_none(), "All").clicked() {
                    actions.push(("git.log.filterUser", json!({})));
                }
                for user in log.users() {
                    if ui
                        .selectable_label(log.user.as_deref() == Some(user.as_str()), &user)
                        .clicked()
                    {
                        actions.push(("git.log.filterUser", json!({ "user": user })));
                    }
                }
            });
        egui::ComboBox::from_id_salt("git_log_date_filter")
            .selected_text(log.date.label())
            .width(80.0)
            .show_ui(ui, |ui| {
                for range in DateRange::ALL {
                    let label = if range == DateRange::Any {
                        "Any date"
                    } else {
                        range.label()
                    };
                    if ui.selectable_label(log.date == range, label).clicked() {
                        actions.push(("git.log.filterDate", json!({ "range": range.id() })));
                    }
                }
            });
    });
    ui.add_space(2.0);
    widgets::toolbar_row(ui, |ui| {
        ui.add_space(theme::layout::CONTENT_INSET);
        // Paths: a repository-relative file or directory.
        let path_id = egui::Id::new("git_log_path_filter");
        let mut path: String = ui.data_mut(|data| {
            data.get_temp(path_id).unwrap_or_else(|| {
                log.path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            })
        });
        let response = ui.add(
            egui::TextEdit::singleline(&mut path)
                .id(path_id.with("edit"))
                .hint_text("Paths")
                .desired_width(140.0),
        );
        ui.data_mut(|data| data.insert_temp(path_id, path.clone()));
        if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
            actions.push(("git.log.filterPath", json!({ "path": path })));
        }
        if let Some(file_path) = &log.path {
            let name = file_path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            if ui
                .button(format!("Path: {name}  ×"))
                .on_hover_text(format!(
                    "History of {}; click to show all files",
                    file_path.display()
                ))
                .clicked()
            {
                ui.data_mut(|data| data.insert_temp(path_id, String::new()));
                actions.push(("git.log.clearPath", Value::Null));
            }
        }
        ui.add_space(8.0);
        if icons::button(ui, Icon::Refresh, "Refresh").clicked() {
            actions.push(("git.log.refresh", Value::Null));
        }
        let can_pick =
            log.selected.is_some() && app.commands.is_enabled("git.cherryPick", &app.state);
        if ui
            .add_enabled_ui(can_pick, |ui| {
                icons::button(ui, Icon::Rollback, "Cherry-Pick the selected commit")
            })
            .inner
            .clicked()
            && let Some(hash) = &log.selected
        {
            actions.push(("git.cherryPick", json!({ "commit": hash })));
        }
        if icons::button(
            ui,
            Icon::Diff,
            if log.hide_details {
                "Show Details"
            } else {
                "Hide Details"
            },
        )
        .clicked()
        {
            actions.push(("git.log.toggleDetails", Value::Null));
        }
    });
    ui.add_space(4.0);
    if let Some(error) = &log.error {
        ui.label(RichText::new(error).color(theme::ERROR));
        return;
    }
    let visible = log.visible();
    if visible.is_empty() {
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            let text = if log.commits.is_empty() {
                "No commits yet"
            } else {
                "No commits matching the filters"
            };
            ui.label(RichText::new(text).color(theme::TEXT_SECONDARY));
        });
        return;
    }
    // The graph only makes sense for the unfiltered history.
    let show_graph = log.filter.trim().is_empty();
    let lanes = if show_graph {
        log.graph.iter().map(row_width).max().unwrap_or(1)
    } else {
        0
    };
    let graph_width = lanes as f32 * LANE + 8.0;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs() as i64);
    egui::ScrollArea::vertical()
        .id_salt("git_log_commits")
        .auto_shrink([false, false])
        .show_rows(ui, ROW, visible.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for row in range {
                let index = visible[row];
                let commit = &log.commits[index];
                let selected = log.selected.as_ref() == Some(&commit.hash);
                let (rect, response) =
                    ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::click());
                let fill = if selected {
                    *theme::SELECTION
                } else if response.hovered() {
                    *theme::HOVER
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter()
                    .rect_filled(rect.shrink2(vec2(4.0, 0.0)), 4.0, fill);
                if show_graph && let Some(graph_row) = log.graph.get(index) {
                    paint_graph(ui.painter(), rect, graph_row, commit);
                }
                commit_row(ui, rect, commit, graph_width, now);
                if response.clicked() {
                    actions.push(("git.log.selectCommit", json!({ "hash": commit.hash })));
                }
            }
        });
}

/// A small text toggle button; returns whether it was clicked.
fn toggle(ui: &mut egui::Ui, on: &mut bool, label: &str, tooltip: &str) -> bool {
    let text = RichText::new(label).color(if *on {
        *theme::TEXT_BRIGHT
    } else {
        *theme::TEXT_SECONDARY
    });
    let clicked = ui
        .add(egui::Button::selectable(*on, text))
        .on_hover_text(tooltip)
        .clicked();
    if clicked {
        *on = !*on;
    }
    clicked
}

fn row_width(row: &GraphRow) -> usize {
    row.edges
        .iter()
        .map(|edge| match edge {
            GraphEdge::Through { column }
            | GraphEdge::Into { column }
            | GraphEdge::OutOf { column } => *column,
        })
        .chain([row.column])
        .max()
        .unwrap_or(0)
        + 1
}

fn paint_graph(painter: &egui::Painter, rect: Rect, row: &GraphRow, commit: &CommitInfo) {
    let x = |column: usize| rect.left() + 12.0 + column as f32 * LANE;
    let (top, mid, bottom) = (rect.top(), rect.center().y, rect.bottom());
    let node = pos2(x(row.column), mid);
    for edge in &row.edges {
        let (from, to, color) = match *edge {
            GraphEdge::Through { column } => {
                (pos2(x(column), top), pos2(x(column), bottom), column)
            }
            GraphEdge::Into { column } => (pos2(x(column), top), node, column),
            GraphEdge::OutOf { column } => (node, pos2(x(column), bottom), column.max(row.column)),
        };
        painter.line_segment([from, to], Stroke::new(1.5_f32, lane_color(color)));
    }
    let color = lane_color(row.column);
    if commit.parents.len() > 1 {
        painter.circle_stroke(node, 3.5, Stroke::new(1.5_f32, color));
        painter.circle_filled(node, 2.0, theme::ISLAND);
    } else {
        painter.circle_filled(node, 4.0, color);
    }
}

fn commit_row(ui: &egui::Ui, rect: Rect, commit: &CommitInfo, graph_width: f32, now: i64) {
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    let date = relative_time(now, commit.time);
    // Fixed author and date columns, like the reference table.
    let date_left = rect.right() - 12.0 - DATE_WIDTH;
    let author_left = date_left - 10.0 - AUTHOR_WIDTH;
    for (left, width, text, color) in [
        (date_left, DATE_WIDTH, date.as_str(), theme::TEXT_SECONDARY),
        (
            author_left,
            AUTHOR_WIDTH,
            commit.author.as_str(),
            theme::TEXT,
        ),
    ] {
        let cell = Rect::from_min_max(pos2(left, rect.top()), pos2(left + width, rect.bottom()));
        row_text(ui, cell, left, text, font.clone(), color);
    }
    // Ref labels (branches, tags) right-aligned before the author.
    let mut refs_left = author_left - 10.0;
    for label in commit.refs.iter().rev() {
        let text = label.strip_prefix("HEAD -> ").unwrap_or(label);
        let is_tag = text.starts_with("tag: ");
        let text = text.strip_prefix("tag: ").unwrap_or(text);
        let galley =
            ui.painter()
                .layout_no_wrap(text.to_owned(), font.clone(), *theme::TEXT_SECONDARY);
        let width = galley.size().x + 22.0;
        refs_left -= width;
        if refs_left < rect.left() + graph_width + 120.0 {
            break;
        }
        icons::paint(
            ui.painter(),
            Rect::from_min_size(pos2(refs_left, rect.center().y - 7.0), vec2(14.0, 14.0)),
            if is_tag { Icon::Commit } else { Icon::Branch },
            if label.starts_with("HEAD -> ") {
                theme::WARNING
            } else {
                theme::TEXT_SECONDARY
            },
        );
        ui.painter().galley(
            pos2(refs_left + 18.0, rect.center().y - galley.size().y / 2.0),
            galley,
            *theme::TEXT_SECONDARY,
        );
        refs_left -= 6.0;
    }
    let subject_rect = Rect::from_min_max(
        pos2(rect.left() + graph_width + 6.0, rect.top()),
        pos2(
            refs_left.max(rect.left() + graph_width + 40.0),
            rect.bottom(),
        ),
    );
    row_text(
        ui,
        subject_rect,
        subject_rect.left(),
        &commit.subject,
        font,
        theme::TEXT_BRIGHT,
    );
}

fn details_pane(app: &IdeApp, ui: &mut egui::Ui, actions: &mut Vec<(&'static str, Value)>) {
    let log = &app.state.git_log;
    let Some(details) = &log.details else {
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("No commit selected").color(theme::TEXT_SECONDARY));
        });
        return;
    };
    let commit = log
        .commits
        .iter()
        .find(|commit| commit.hash == details.hash);
    let files_height = (ui.available_height() * 0.55).max(80.0);
    egui::ScrollArea::vertical()
        .id_salt("git_log_files")
        .max_height(files_height)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add_space(4.0);
            let mut root = FileTree::default();
            for file in &details.files {
                root.insert(&file.path, file);
            }
            let total = details.files.len();
            ui.horizontal(|ui| {
                ui.add_space(theme::layout::CONTENT_INSET);
                ui.label(
                    RichText::new(widgets::pluralize(total, "file", "files"))
                        .color(theme::TEXT_SECONDARY),
                );
            });
            tree_rows(ui, &root, 0, "", &details.hash, actions);
        });
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.cursor().top(),
        Stroke::new(1.0_f32, theme::POPUP_BORDER),
    );
    egui::ScrollArea::vertical()
        .id_salt("git_log_message")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(8.0);
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(
                    theme::layout::CONTENT_INSET as i8,
                    0,
                ))
                .show(ui, |ui| {
                    let mut lines = details.message.lines();
                    if let Some(subject) = lines.next() {
                        ui.label(
                            RichText::new(subject)
                                .font(theme::semibold(theme::UI_FONT_SIZE))
                                .color(theme::TEXT_BRIGHT),
                        );
                    }
                    let body: Vec<&str> = lines.collect();
                    if !body.join("").trim().is_empty() {
                        ui.add_space(4.0);
                        ui.label(RichText::new(body.join("\n").trim()).color(theme::TEXT));
                    }
                    ui.add_space(8.0);
                    if let Some(commit) = commit {
                        ui.label(
                            RichText::new(format!(
                                "{} {} <{}> on {}",
                                commit.short,
                                commit.author,
                                commit.email,
                                absolute_time(commit.time)
                            ))
                            .color(theme::TEXT_SECONDARY),
                        );
                        if details.committer != commit.author {
                            ui.label(
                                RichText::new(format!(
                                    "committed by {} on {}",
                                    details.committer,
                                    absolute_time(details.commit_time)
                                ))
                                .color(theme::TEXT_SECONDARY),
                            );
                        }
                    }
                });
        });
}

/// Changed files grouped by directory.
#[derive(Default)]
struct FileTree<'a> {
    dirs: BTreeMap<String, FileTree<'a>>,
    files: Vec<&'a CommitFile>,
}

impl<'a> FileTree<'a> {
    fn insert(&mut self, path: &std::path::Path, file: &'a CommitFile) {
        let parts: Vec<String> = path
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect();
        let mut node = self;
        for dir in &parts[..parts.len().saturating_sub(1)] {
            node = node.dirs.entry(dir.clone()).or_default();
        }
        node.files.push(file);
    }

    fn count(&self) -> usize {
        self.files.len() + self.dirs.values().map(FileTree::count).sum::<usize>()
    }
}

fn tree_rows(
    ui: &mut egui::Ui,
    node: &FileTree,
    depth: usize,
    prefix: &str,
    hash: &str,
    actions: &mut Vec<(&'static str, Value)>,
) {
    for (name, child) in &node.dirs {
        let key = format!("{prefix}{name}/");
        let id = egui::Id::new(("git_log_dir_collapsed", &key));
        let collapsed: bool = ui.data(|data| data.get_temp(id)).unwrap_or(false);
        let count = widgets::pluralize(child.count(), "file", "files");
        let response = widgets::tree_row(
            ui,
            widgets::TreeRow {
                depth,
                expanded: Some(!collapsed),
                secondary: Some((&count, *theme::TEXT_INFO)),
                ..widgets::TreeRow::new(name)
            },
        )
        .response;
        if response.clicked() {
            ui.data_mut(|data| data.insert_temp(id, !collapsed));
        }
        if !collapsed {
            tree_rows(ui, child, depth + 1, &key, hash, actions);
        }
    }
    for file in &node.files {
        let name = IdeApp::file_name(&file.path);
        let response = widgets::tree_row(
            ui,
            widgets::TreeRow {
                depth,
                icon: Some((Icon::for_file(&name), *icons::ICON)),
                color: crate::toolwindows::change_color(Some(file.kind)),
                ..widgets::TreeRow::new(&name)
            },
        )
        .response;
        let response = response.on_hover_text(match file.kind {
            ChangeKind::Added => "Added",
            ChangeKind::Deleted => "Deleted",
            ChangeKind::Renamed => "Renamed",
            _ => "Modified",
        });
        if response.double_clicked() {
            actions.push((
                "git.log.showFileDiff",
                json!({ "hash": hash, "path": file.path }),
            ));
        }
    }
}

/// "just now", "15 minutes ago", "3 hours ago", "Yesterday", else a date.
pub(crate) fn relative_time(now: i64, time: i64) -> String {
    let delta = now - time;
    match delta {
        d if d < 60 => "just now".into(),
        d if d < 3_600 => widgets::pluralize((d / 60) as usize, "minute ago", "minutes ago"),
        d if d < 86_400 => widgets::pluralize((d / 3_600) as usize, "hour ago", "hours ago"),
        d if d < 2 * 86_400 => "Yesterday".into(),
        _ => absolute_time(time),
    }
}

/// UTC calendar date `YYYY-MM-DD HH:MM` (civil-from-days, no time zone data).
pub(crate) fn absolute_time(time: i64) -> String {
    let days = time.div_euclid(86_400);
    let seconds = time.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}",
        seconds / 3_600,
        seconds % 3_600 / 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_relative_and_absolute_times() {
        let now = 1_791_640_000;
        assert_eq!(relative_time(now, now - 5), "just now");
        assert_eq!(relative_time(now, now - 15 * 60), "15 minutes ago");
        assert_eq!(relative_time(now, now - 60), "1 minute ago");
        assert_eq!(relative_time(now, now - 3 * 3_600), "3 hours ago");
        assert_eq!(relative_time(now, now - 30 * 3_600), "Yesterday");
        assert_eq!(absolute_time(0), "1970-01-01 00:00");
        assert_eq!(absolute_time(951_782_400), "2000-02-29 00:00");
        assert_eq!(absolute_time(1_791_640_000), "2026-10-10 13:46");
    }

    #[test]
    fn graph_width_counts_all_lanes() {
        let row = GraphRow {
            column: 0,
            edges: vec![
                GraphEdge::OutOf { column: 0 },
                GraphEdge::OutOf { column: 2 },
            ],
        };
        assert_eq!(row_width(&row), 3);
    }
}

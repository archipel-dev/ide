//! TODO tool window (IJ-TW-030): the `Todo` index's comments for the
//! current file or the whole project, grouped by file, filtered by the
//! filters of Settings > Editor > TODO. Rows dispatch `todo.*` commands.

use crate::widgets::{ROW_HEIGHT, list_row, row_text, tool_window_header};
use crate::{IdeApp, theme};
use archipel_ide_core::{TodoGroup, TodoRow, TodoScope, TodoView, ToolWindow};
use eframe::egui::{self, Color32};
use serde_json::json;
use std::path::{Path, PathBuf};

/// One visible line of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Entry {
    /// A file heading: path, item count, collapsed.
    File(PathBuf, usize, bool),
    /// An item: index into the view's rows.
    Item(usize),
}

/// Flattens `rows` into the visible lines for the grouping.
pub(crate) fn entries(
    rows: &[TodoRow],
    group: TodoGroup,
    collapsed: &std::collections::HashSet<PathBuf>,
) -> Vec<Entry> {
    let mut out = Vec::with_capacity(rows.len() + 8);
    match group {
        TodoGroup::Flat => out.extend((0..rows.len()).map(Entry::Item)),
        TodoGroup::File => {
            let mut at = 0;
            while at < rows.len() {
                let path = &rows[at].path;
                let end = at + rows[at..].iter().take_while(|r| &r.path == path).count();
                let hidden = collapsed.contains(path);
                out.push(Entry::File(path.clone(), end - at, hidden));
                if !hidden {
                    out.extend((at..end).map(Entry::Item));
                }
                at = end;
            }
        }
    }
    out
}

fn pattern_color(view: &TodoView, pattern: usize) -> Color32 {
    match view.patterns.get(pattern).map(|p| p.color_key.as_str()) {
        Some("fixme") => *theme::WARNING,
        Some("todo") | None => *theme::TEXT_INFO,
        Some(_) => *theme::TEXT_BRIGHT,
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    let view = app.state.todo_view();
    let (scope, group, filter, selected) = {
        let todo = &app.state.analysis.todo;
        (todo.scope, todo.group, todo.filter, todo.selected)
    };
    let count = view.rows.len();
    let mut command: Option<(&str, serde_json::Value)> = None;
    let title = if count == 0 {
        "TODO".to_owned()
    } else {
        format!("TODO  {count}")
    };
    let hide = tool_window_header(ui, &title, |_| {});
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        for (candidate, label) in [
            (TodoScope::CurrentFile, "Current File"),
            (TodoScope::Project, "Project"),
        ] {
            if ui.selectable_label(scope == candidate, label).clicked() && scope != candidate {
                command = Some(("todo.setScope", json!({"scope": candidate.id()})));
            }
        }
        ui.separator();
        let by_file = group == TodoGroup::File;
        if ui.selectable_label(by_file, "Group by file").clicked() {
            command = Some((
                "todo.setGroup",
                json!({"group": if by_file { "flat" } else { "file" }}),
            ));
        }
        if !view.filters.is_empty() {
            ui.separator();
            let current = filter
                .and_then(|i| view.filters.get(i))
                .map_or("All", |f| f.name.as_str());
            egui::ComboBox::from_id_salt("todo_filter")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(filter.is_none(), "All").clicked() {
                        command = Some(("todo.setFilter", json!({"filter": null})));
                    }
                    for (index, candidate) in view.filters.iter().enumerate() {
                        if ui
                            .selectable_label(filter == Some(index), &candidate.name)
                            .clicked()
                        {
                            command = Some(("todo.setFilter", json!({"filter": index})));
                        }
                    }
                });
        }
    });
    crate::index_status::banner(ui, &app.state);
    if view.indexing && !app.state.analysis.index.is_attached() {
        ui.label(
            egui::RichText::new("Open a project to list TODO items").color(theme::TEXT_SECONDARY),
        );
    } else if count == 0 && !view.indexing {
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(egui::RichText::new("No TODO items found").color(theme::TEXT_SECONDARY));
        });
    }
    let lines = entries(&view.rows, group, &app.state.analysis.todo.collapsed);
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, ROW_HEIGHT, lines.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for line in &lines[range] {
                match line {
                    Entry::File(path, items, hidden) => {
                        let (rect, response) = list_row(ui, false);
                        let chevron = if *hidden {
                            crate::icons::Icon::ChevronRight
                        } else {
                            crate::icons::Icon::ChevronDown
                        };
                        crate::icons::paint(
                            ui.painter(),
                            egui::Rect::from_center_size(
                                egui::pos2(rect.left() + 18.0, rect.center().y),
                                egui::vec2(14.0, 14.0),
                            ),
                            chevron,
                            *theme::TEXT_SECONDARY,
                        );
                        let end = row_text(
                            ui,
                            rect,
                            rect.left() + 28.0,
                            &file_name(path),
                            font.clone(),
                            theme::TEXT_BRIGHT,
                        );
                        let dir = path
                            .parent()
                            .filter(|p| !p.as_os_str().is_empty())
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();
                        let end = row_text(
                            ui,
                            rect,
                            end + 8.0,
                            &dir,
                            font.clone(),
                            theme::TEXT_DISABLED,
                        );
                        row_text(
                            ui,
                            rect,
                            end + 8.0,
                            &items.to_string(),
                            font.clone(),
                            theme::TEXT_INFO,
                        );
                        if response.clicked() {
                            command = Some(("todo.toggleFile", json!({"path": path})));
                        }
                    }
                    Entry::Item(index) => {
                        let row = &view.rows[*index];
                        let (rect, response) = list_row(ui, selected == Some(*index));
                        let indent = if group == TodoGroup::File { 44.0 } else { 12.0 };
                        let x = rect.left() + indent;
                        let end = row_text(
                            ui,
                            rect,
                            x,
                            &format!("({})", row.line + 1),
                            font.clone(),
                            theme::TEXT_DISABLED,
                        );
                        let end = row_text(
                            ui,
                            rect,
                            end + 6.0,
                            &row.text,
                            font.clone(),
                            pattern_color(&view, row.pattern),
                        );
                        if group == TodoGroup::Flat {
                            row_text(
                                ui,
                                rect,
                                end + 10.0,
                                &row.path.display().to_string(),
                                font.clone(),
                                theme::TEXT_DISABLED,
                            );
                        }
                        if response.clicked() || response.double_clicked() {
                            command = Some(("todo.open", json!({"index": index})));
                        }
                    }
                }
            }
        });
    if let Some((id, params)) = command {
        app.dispatch(id, params);
    }
    if hide {
        app.toggle_tool_window(ToolWindow::Todo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn row(path: &str, line: usize) -> TodoRow {
        TodoRow {
            path: PathBuf::from(path),
            line,
            start: 0,
            text: "TODO x".into(),
            pattern: 0,
        }
    }

    #[test]
    fn rows_group_by_file_and_collapse() {
        let rows = vec![row("a.rs", 1), row("a.rs", 5), row("b.rs", 2)];
        let none = HashSet::new();
        assert_eq!(
            entries(&rows, TodoGroup::File, &none),
            vec![
                Entry::File("a.rs".into(), 2, false),
                Entry::Item(0),
                Entry::Item(1),
                Entry::File("b.rs".into(), 1, false),
                Entry::Item(2),
            ]
        );
        let collapsed: HashSet<PathBuf> = ["a.rs".into()].into();
        assert_eq!(
            entries(&rows, TodoGroup::File, &collapsed),
            vec![
                Entry::File("a.rs".into(), 2, true),
                Entry::File("b.rs".into(), 1, false),
                Entry::Item(2),
            ]
        );
        assert_eq!(
            entries(&rows, TodoGroup::Flat, &collapsed),
            vec![Entry::Item(0), Entry::Item(1), Entry::Item(2)]
        );
        assert!(entries(&[], TodoGroup::File, &none).is_empty());
    }
}

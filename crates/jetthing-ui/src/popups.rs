//! Small modal popups: New File/Directory, Rename, Delete, Go to Line, Recent Files.

use crate::dialog::{self, Outcome, Select, Shell};
use crate::icons;
use crate::native_dialog::Purpose;
use crate::widgets::row_text;
use crate::{IdeApp, theme};
use archipel_kit::components::{Checkbox, ListRow, PropertyGrid, TextField, VirtualList};
use archipel_kit::parts::{Popover, Surface};
use eframe::egui::{self, Rect, RichText, Sense, vec2};
use jetthing_core::{JavaKind, KotlinKind};
use serde_json::json;
use std::path::{Path, PathBuf};

pub enum Dialog {
    NewProject(Box<crate::new_project::Form>),
    /// Get from Version Control.
    Clone(Box<crate::clone_dialog::Form>),
    OpenProject {
        path: String,
        error: Option<String>,
    },
    NewEntry {
        parent: PathBuf,
        directory: bool,
        name: String,
    },
    /// IntelliJ's small "New Java Class" / "New Kotlin Class/File" popup: a name
    /// field above a kind list.
    NewClass {
        dir: PathBuf,
        kotlin: bool,
        name: String,
        kind: usize,
        error: Option<String>,
    },
    Rename {
        path: PathBuf,
        name: String,
    },
    Delete {
        path: PathBuf,
    },
    /// Refactor | Safe Delete: lists the remaining usages before deleting.
    SafeDelete {
        path: PathBuf,
        usages: Vec<jetthing_core::project_view::Usage>,
    },
    /// Refactor | Move: the destination directory is typed.
    Move {
        path: PathBuf,
        destination: String,
    },
    GotoLine {
        input: String,
    },
    Recent {
        query: String,
        selected: usize,
    },
    RecentLocations {
        selected: usize,
    },
    /// File Structure popup; `selected` starts at the declaration under the caret.
    FileStructure {
        query: String,
        selected: Option<usize>,
    },
}

impl Dialog {
    /// Stable name reported by the automation `state` request.
    pub fn kind(&self) -> &'static str {
        match self {
            Dialog::NewProject(_) => "newProject",
            Dialog::Clone(_) => "cloneRepository",
            Dialog::OpenProject { .. } => "openProject",
            Dialog::NewEntry { .. } => "newEntry",
            Dialog::NewClass { .. } => "newClass",
            Dialog::Rename { .. } => "rename",
            Dialog::Delete { .. } => "delete",
            Dialog::SafeDelete { .. } => "safeDelete",
            Dialog::Move { .. } => "move",
            Dialog::GotoLine { .. } => "gotoLine",
            Dialog::Recent { .. } => "recentFiles",
            Dialog::RecentLocations { .. } => "recentLocations",
            Dialog::FileStructure { .. } => "fileStructure",
        }
    }
}

fn input_id() -> egui::Id {
    egui::Id::new("dialog_input")
}

/// Parses "line[:column]" (1-based) as typed into Go to Line.
pub fn parse_line_column(input: &str) -> Option<(usize, Option<usize>)> {
    let mut parts = input.trim().splitn(2, [':', ',']);
    let line = parts.next()?.trim().parse::<usize>().ok()?;
    let column = match parts.next() {
        Some(column) if !column.trim().is_empty() => Some(column.trim().parse::<usize>().ok()?),
        _ => None,
    };
    Some((line, column))
}

/// A modal dialog box near the top of the window, on the kit's dialog surface
/// (`Surface::Dialog`: island radius, popup fill and border, dialog padding).
pub(crate) fn popup_frame(
    ctx: &egui::Context,
    id: &str,
    width: f32,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(
            screen.center().x - width / 2.0,
            screen.top() + screen.height() * 0.2,
        ))
        .show(ctx, |ui| {
            let frame = Surface::Dialog.frame(ctx);
            let padding = frame.inner_margin.sum().x;
            frame.show(ui, |ui| {
                ui.set_width(width - padding);
                add(ui);
            });
        })
        .response
}

pub(crate) fn title(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme::semibold(theme::UI_FONT_SIZE))
            .color(theme::TEXT_BRIGHT),
    );
    ui.add_space(6.0);
}

/// The labelled, validated single-line field every form dialog uses. `error` is the
/// message under the field; without one the dialog's stored error is shown (kept per
/// dialog in the context so the `Dialog` variants stay as their opener built them).
#[allow(clippy::too_many_arguments)]
fn field(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    id: &'static str,
    label: &str,
    text: &mut String,
    placeholder: &str,
    error: Option<&str>,
    focus: Option<Select>,
) -> egui::Response {
    let error = error.map(str::to_owned).or_else(|| dialog::error(ctx, id));
    let response = ui.add(
        TextField::new(text)
            .label(label)
            .placeholder(placeholder)
            .error(error.as_deref()),
    );
    if response.changed() {
        dialog::set_error(ctx, id, None);
    }
    if let Some(select) = focus {
        dialog::focus_first(ctx, &response, select);
    }
    response
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(mut dialog) = app.dialog.take() else {
        return;
    };
    let focus = std::mem::take(&mut app.dialog_focus);
    if focus {
        // A fresh dialog starts without the previous one's error.
        for id in [
            "open_project",
            "new_entry",
            "new_class",
            "rename",
            "move",
            "goto_line",
        ] {
            dialog::set_error(ctx, id, None);
        }
    }
    let on_open = |select| focus.then_some(select);
    let mut keep = true;
    let mut browse = None;
    match &mut dialog {
        Dialog::NewProject(form) => {
            keep = crate::new_project::show(app, ctx, form, focus, &mut browse);
        }
        Dialog::Clone(form) => {
            keep = crate::clone_dialog::show(app, ctx, form, focus, &mut browse);
        }
        Dialog::OpenProject { path, error } => {
            let outcome = Shell::new("open_project", "Open Project")
                .width(520.0)
                .ok("Open")
                .ok_enabled(!path.trim().is_empty())
                .secondary("Browse…", true)
                .show(ctx, |ui| {
                    let response = field(
                        ui,
                        ctx,
                        "open_project",
                        "Project directory",
                        path,
                        "/path/to/project",
                        error.as_deref(),
                        on_open(Select::All),
                    );
                    if response.changed() {
                        *error = None;
                    }
                    dialog::hint(ui, "Enter the full path to an existing folder.");
                });
            match outcome {
                Some(Outcome::Cancel) => keep = false,
                Some(Outcome::Secondary(_)) => browse = Some(Purpose::OpenProject),
                Some(Outcome::Ok) => match app.open_project_path(path, ctx) {
                    Ok(()) => keep = false,
                    Err(message) => *error = Some(dialog::friendly("Project directory", &message)),
                },
                None => {}
            }
        }
        Dialog::NewEntry {
            parent,
            directory,
            name,
        } => {
            let title = if *directory {
                "New Directory"
            } else {
                "New File"
            };
            let outcome = Shell::new("new_entry", title)
                .ok("Create")
                .ok_enabled(!name.trim().is_empty())
                .show(ctx, |ui| {
                    field(
                        ui,
                        ctx,
                        "new_entry",
                        "Name",
                        name,
                        if *directory { "directory" } else { "file.ext" },
                        None,
                        on_open(Select::All),
                    );
                    dialog::hint(
                        ui,
                        if *directory {
                            "Name, or a path like a/b/c"
                        } else {
                            "Name with extension, or a path like src/lib.rs"
                        },
                    );
                });
            match outcome {
                Some(Outcome::Cancel) => keep = false,
                Some(Outcome::Ok) => {
                    let path = parent.join(name.trim());
                    let id = if *directory {
                        "project.newDirectory"
                    } else {
                        "project.newFile"
                    };
                    match app
                        .commands
                        .execute(id, &mut app.state, json!({"path": path}))
                    {
                        Ok(_) => {
                            keep = false;
                            app.tree_selection = Some(path);
                            if !*directory {
                                ctx.memory_mut(|m| m.request_focus(crate::editor::editor_id()));
                            }
                        }
                        Err(e) => dialog::set_error(
                            ctx,
                            "new_entry",
                            Some(dialog::friendly("Name", &e.to_string())),
                        ),
                    }
                }
                _ => {}
            }
        }
        Dialog::NewClass {
            dir,
            kotlin,
            name,
            kind,
            error: class_error,
        } => {
            let kinds: Vec<(&str, &str)> = if *kotlin {
                KotlinKind::ALL
                    .iter()
                    .map(|k| (k.id(), k.label()))
                    .collect()
            } else {
                JavaKind::ALL.iter().map(|k| (k.id(), k.label())).collect()
            };
            // Up/Down pick the kind while the name field keeps the keyboard focus.
            let step = ctx.input_mut(|input| {
                let down = input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown);
                let up = input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp);
                down as isize - up as isize
            });
            *kind = (*kind as isize + step).clamp(0, kinds.len() as isize - 1) as usize;
            let mut double_clicked = false;
            let outcome = Shell::new(
                "new_class",
                if *kotlin {
                    "New Kotlin Class/File"
                } else {
                    "New Java Class"
                },
            )
            .ok("Create")
            .ok_enabled(!name.trim().is_empty())
            .show(ctx, |ui| {
                let response = field(
                    ui,
                    ctx,
                    "new_class",
                    "Name",
                    name,
                    "Name or com.example.Name",
                    class_error.as_deref(),
                    on_open(Select::All),
                );
                if response.changed() {
                    *class_error = None;
                }
                for (index, (_, label)) in kinds.iter().enumerate() {
                    let row = ListRow::new(*label).selected(index == *kind).show(ui);
                    if row.response.clicked() {
                        *kind = index;
                    }
                    double_clicked |= row.response.double_clicked();
                }
                dialog::hint(ui, "Name, or a qualified name like com.foo.Bar");
            });
            if double_clicked || outcome == Some(Outcome::Ok) {
                if !name.trim().is_empty() {
                    let command = if *kotlin {
                        "project.newKotlinFile"
                    } else {
                        "project.newJavaClass"
                    };
                    let params = json!({"dir": dir, "name": name.trim(), "kind": kinds[*kind].0});
                    match app.commands.execute(command, &mut app.state, params) {
                        Ok(result) => {
                            keep = false;
                            app.tree_selection = result
                                .get("path")
                                .and_then(|p| p.as_str())
                                .map(PathBuf::from);
                            ctx.memory_mut(|m| m.request_focus(crate::editor::editor_id()));
                        }
                        Err(e) => {
                            *class_error = Some(dialog::friendly("Name", &e.to_string()));
                        }
                    }
                }
            } else if outcome == Some(Outcome::Cancel) {
                keep = false;
            }
        }
        Dialog::Rename { path, name } => {
            let title = format!("Rename {}", IdeApp::file_name(path));
            // The name without its extension is selected, as in the reference IDE.
            let stem = Path::new(name.as_str())
                .file_stem()
                .map_or(0, |stem| stem.to_string_lossy().chars().count());
            let outcome = Shell::new("rename", title)
                .ok("Rename")
                .ok_enabled(!name.trim().is_empty())
                .show(ctx, |ui| {
                    field(
                        ui,
                        ctx,
                        "rename",
                        "New name",
                        name,
                        "",
                        None,
                        on_open(Select::Chars(stem)),
                    );
                });
            match outcome {
                Some(Outcome::Cancel) => keep = false,
                Some(Outcome::Ok) => {
                    match app.commands.execute(
                        "project.rename",
                        &mut app.state,
                        json!({"path": path, "name": name.trim()}),
                    ) {
                        Ok(result) => {
                            keep = false;
                            app.tree_selection = result["path"].as_str().map(PathBuf::from);
                        }
                        Err(e) => dialog::set_error(
                            ctx,
                            "rename",
                            Some(dialog::friendly("New name", &e.to_string())),
                        ),
                    }
                }
                _ => {}
            }
        }
        Dialog::Delete { path } => {
            let is_dir = app
                .state
                .files
                .iter()
                .any(|e| e.is_dir && e.relative_path == *path);
            let what = if is_dir { "directory" } else { "file" };
            let outcome = Shell::new("delete", format!("Delete {what}"))
                .width(400.0)
                .ok("Delete")
                .show(ctx, |ui| {
                    dialog::message(ui, format!("Delete {what} “{}”?", path.display()));
                    if is_dir {
                        dialog::hint(ui, "All files inside will be deleted.");
                    }
                });
            match outcome {
                Some(Outcome::Cancel) => keep = false,
                Some(Outcome::Ok) => {
                    keep = false;
                    app.dispatch("project.delete", json!({"path": path}));
                    app.tree_selection = None;
                }
                _ => {}
            }
        }
        Dialog::SafeDelete { path, usages } => {
            let label = if usages.is_empty() {
                "Delete"
            } else {
                "Delete Anyway"
            };
            let outcome = Shell::new("safe_delete", "Safe Delete")
                .width(520.0)
                .ok(label)
                .show(ctx, |ui| {
                    dialog::message(ui, format!("Delete “{}”?", path.display()));
                    if usages.is_empty() {
                        dialog::hint(ui, "No usages found.");
                    } else {
                        let count = usages.len();
                        let more = if count >= jetthing_core::project_view::MAX_USAGES {
                            "+"
                        } else {
                            ""
                        };
                        dialog::message(ui, format!("{count}{more} usage(s) of the name remain:"));
                        let mut grid = PropertyGrid::new("safe_delete_usages");
                        for usage in usages.iter() {
                            grid = grid.mono_row(
                                format!("{}:{}", usage.path.display(), usage.line),
                                usage.text.clone(),
                            );
                        }
                        grid.show(ui);
                    }
                });
            match outcome {
                Some(Outcome::Cancel) => keep = false,
                Some(Outcome::Ok) => {
                    keep = false;
                    app.dispatch("project.safeDelete", json!({"path": path, "force": true}));
                    app.tree_selection = None;
                }
                _ => {}
            }
        }
        Dialog::Move { path, destination } => {
            let title = format!("Move {}", IdeApp::file_name(path));
            let outcome = Shell::new("move", title)
                .width(440.0)
                .ok("Move")
                .show(ctx, |ui| {
                    field(
                        ui,
                        ctx,
                        "move",
                        "To directory",
                        destination,
                        "src/main",
                        None,
                        on_open(Select::All),
                    );
                    dialog::hint(ui, "A directory path relative to the project root.");
                });
            match outcome {
                Some(Outcome::Cancel) => keep = false,
                Some(Outcome::Ok) => {
                    let target = destination.trim().trim_matches('/').to_owned();
                    let moved = app
                        .commands
                        .execute("project.cutPaths", &mut app.state, json!({"paths": [path]}))
                        .and_then(|_| {
                            app.commands.execute(
                                "project.paste",
                                &mut app.state,
                                json!({"dir": target}),
                            )
                        });
                    match moved {
                        Ok(result) => {
                            keep = false;
                            app.tree_selection = result["paths"][0].as_str().map(PathBuf::from);
                        }
                        Err(e) => dialog::set_error(
                            ctx,
                            "move",
                            Some(dialog::friendly("To directory", &e.to_string())),
                        ),
                    }
                }
                _ => {}
            }
        }
        Dialog::GotoLine { input } => {
            let caret = app
                .state
                .active_document
                .and_then(|index| app.state.documents.get(index))
                .and_then(|d| d.text.char_to_line_column(d.selections.primary().head).ok())
                .map(|(l, c)| format!("{}:{}", l + 1, c + 1))
                .unwrap_or_default();
            let outcome = Shell::new("goto_line", "Go to Line:Column")
                .width(340.0)
                .ok("Go")
                .ok_enabled(!input.trim().is_empty())
                .show(ctx, |ui| {
                    field(
                        ui,
                        ctx,
                        "goto_line",
                        "Line[:column]",
                        input,
                        "",
                        None,
                        on_open(Select::All),
                    );
                    dialog::hint(ui, format!("Current position {caret}"));
                });
            match outcome {
                Some(Outcome::Cancel) => keep = false,
                Some(Outcome::Ok) => match parse_line_column(input) {
                    Some((line, column)) => {
                        let mut params = json!({"line": line});
                        if let Some(column) = column {
                            params["column"] = json!(column);
                        }
                        app.dispatch("editor.gotoLine", params);
                        ctx.memory_mut(|m| m.request_focus(crate::editor::editor_id()));
                        keep = false;
                    }
                    None => dialog::set_error(
                        ctx,
                        "goto_line",
                        Some("Line[:column] must be a line number, optionally followed by :column, like 42 or 42:7.".into()),
                    ),
                },
                _ => {}
            }
        }
        Dialog::Recent { query, selected } => {
            let entries: Vec<PathBuf> = jetthing_core::quick_nav::rank(
                query,
                app.state
                    .recent_files
                    .iter()
                    .map(|path| (path.clone(), IdeApp::file_name(path))),
                30,
            )
            .into_iter()
            .map(|ranked| ranked.item)
            .collect();
            if !entries.is_empty() {
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
                    *selected = (*selected + 1) % entries.len();
                }
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
                    *selected = (*selected + entries.len() - 1) % entries.len();
                }
            }
            *selected = (*selected).min(entries.len().saturating_sub(1));
            let mut open = ctx
                .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
                .then_some(*selected);
            popup_frame(ctx, "dialog_recent", 520.0, |ui| {
                ui.horizontal(|ui| {
                    title(ui, "Recent Files");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(
                            egui::TextEdit::singleline(query)
                                .id(input_id())
                                .frame(false)
                                .desired_width(160.0)
                                .hint_text("Type to filter"),
                        )
                        .request_focus();
                    });
                });
                if entries.is_empty() {
                    ui.label(RichText::new("No recent files").color(theme::TEXT_SECONDARY));
                }
                ui.spacing_mut().item_spacing.y = 0.0;
                for (index, path) in entries.iter().enumerate() {
                    let (rect, response) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
                    if index == *selected {
                        ui.painter().rect_filled(rect, 4.0, theme::SELECTION);
                    } else if response.hovered() {
                        ui.painter().rect_filled(rect, 4.0, theme::HOVER);
                    }
                    let name = IdeApp::file_name(path);
                    icons::paint(
                        ui.painter(),
                        Rect::from_min_size(rect.left_center() + vec2(8.0, -8.0), vec2(16.0, 16.0)),
                        app.file_icon(path),
                        *icons::ICON,
                    );
                    let x = row_text(
                        ui,
                        rect,
                        rect.left() + 32.0,
                        &name,
                        egui::FontId::proportional(theme::UI_FONT_SIZE),
                        *theme::TEXT_BRIGHT,
                    );
                    let parent = path
                        .parent()
                        .map(Path::display)
                        .map(|d| d.to_string())
                        .unwrap_or_default();
                    row_text(
                        ui,
                        rect,
                        x + 10.0,
                        &parent,
                        egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0),
                        *theme::TEXT_INFO,
                    );
                    if response.clicked() {
                        open = Some(index);
                    }
                }
            });
            if let Some(index) = open
                && let Some(path) = entries.get(index)
            {
                app.dispatch("editor.openFile", json!({"path": path}));
                ctx.memory_mut(|m| m.request_focus(crate::editor::editor_id()));
                keep = false;
            }
        }
        Dialog::RecentLocations { selected } => {
            let locations = app.state.quick_nav.locations.clone();
            if !locations.is_empty() {
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
                    *selected = (*selected + 1) % locations.len();
                }
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
                    *selected = (*selected + locations.len() - 1) % locations.len();
                }
            }
            *selected = (*selected).min(locations.len().saturating_sub(1));
            let mut open = ctx
                .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
                .then_some(*selected);
            popup_frame(ctx, "dialog_recent_locations", 640.0, |ui| {
                title(ui, "Recent Locations");
                if locations.is_empty() {
                    ui.label(RichText::new("No recent locations").color(theme::TEXT_SECONDARY));
                }
                ui.spacing_mut().item_spacing.y = 0.0;
                for (index, location) in locations.iter().take(30).enumerate() {
                    let (rect, response) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
                    if index == *selected {
                        ui.painter().rect_filled(rect, 4.0, theme::SELECTION);
                    } else if response.hovered() {
                        ui.painter().rect_filled(rect, 4.0, theme::HOVER);
                    }
                    icons::paint(
                        ui.painter(),
                        Rect::from_min_size(rect.left_center() + vec2(8.0, -8.0), vec2(16.0, 16.0)),
                        app.file_icon(&location.path),
                        *icons::ICON,
                    );
                    let trailing = format!(
                        "{}:{}",
                        IdeApp::file_name(&location.path),
                        location.line + 1
                    );
                    ui.painter().text(
                        rect.right_center() - vec2(10.0, 0.0),
                        egui::Align2::RIGHT_CENTER,
                        &trailing,
                        egui::FontId::proportional(theme::UI_FONT_SIZE),
                        *theme::TEXT_SECONDARY,
                    );
                    let preview = if location.preview.is_empty() {
                        "(blank line)"
                    } else {
                        &location.preview
                    };
                    row_text(
                        ui,
                        Rect::from_min_max(rect.min, egui::pos2(rect.right() - 150.0, rect.max.y)),
                        rect.left() + 32.0,
                        preview,
                        egui::FontId::monospace(theme::EDITOR_FONT_SIZE),
                        *theme::TEXT_BRIGHT,
                    );
                    if response.clicked() {
                        open = Some(index);
                    }
                }
            });
            if let Some(index) = open
                && let Some(location) = locations.get(index)
            {
                app.dispatch(
                    "editor.openFile",
                    json!({"path": location.path, "line": location.line + 1}),
                );
                ctx.memory_mut(|m| m.request_focus(crate::editor::editor_id()));
                keep = false;
            }
        }
        Dialog::FileStructure { query, selected } => {
            keep = file_structure(app, ctx, query, selected);
        }
    }
    if keep {
        app.dialog = Some(dialog);
    }
    if let Some(purpose) = browse {
        app.browse(purpose);
    }
}

/// File Structure popup (⌘F12): filterable outline of the active file, a kit `Popover`
/// with a `VirtualList`. Returns whether the popup stays open.
fn file_structure(
    app: &mut IdeApp,
    ctx: &egui::Context,
    query: &mut String,
    selected: &mut Option<usize>,
) -> bool {
    let Some(document) = app
        .state
        .active_document
        .and_then(|i| app.state.documents.get(i))
    else {
        return false;
    };
    let items = crate::structure_view::items(ctx, document);
    let mut rows = Vec::new();
    crate::structure_view::flatten(&items, 0, &mut rows);
    let rows: Vec<_> = rows
        .into_iter()
        .filter(|(_, item)| jetthing_core::quick_nav::matches(query, &item.name))
        .collect();
    if selected.is_none() {
        let current = crate::structure_view::item_at_caret(document, &items)
            .map(|item| item.name_range.clone());
        *selected = Some(
            rows.iter()
                .position(|(_, item)| Some(&item.name_range) == current.as_ref())
                .unwrap_or(0),
        );
    }
    // The filter field keeps the keyboard; Up/Down move the selection through it.
    let mut index = selected.unwrap_or(0);
    if !rows.is_empty() {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
            index = (index + 1) % rows.len();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
            index = (index + rows.len() - 1) % rows.len();
        }
    }
    index = index.min(rows.len().saturating_sub(1));
    let mut open = ctx
        .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        .then_some(index);
    let file = IdeApp::file_name(&document.path);
    let screen = ctx.content_rect();
    let width = 560.0_f32.min(screen.width() - 40.0);
    // Centred, 12 % from the top (interaction spec 3.2); the list is at most 70 % of the window.
    let anchor = Rect::from_min_size(
        egui::pos2(
            screen.center().x - width / 2.0,
            screen.top() + screen.height() * 0.12,
        ),
        vec2(width, 0.0),
    );
    let list_height = (screen.height() * 0.7 - 120.0).clamp(120.0, 420.0);
    let mut stay = true;
    let mut chosen = Some(index);
    Popover::new("dialog_file_structure", anchor)
        .width(width)
        .show(ctx, &mut stay, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(&file)
                        .font(archipel_kit::tokens::typography::semibold(
                            ctx,
                            archipel_kit::tokens::typography::TITLE,
                        ))
                        .color(archipel_kit::Theme::colors(ctx).text_strong),
                );
            });
            // Typing filters (speed search); the field has the id the popup always had.
            let response = ui.add(TextField::search(query).placeholder("Type to filter"));
            response.request_focus();
            if rows.is_empty() {
                dialog::hint(ui, "Nothing to show");
            } else {
                VirtualList::new("file_structure_list", rows.len(), &mut chosen)
                    .name("File structure")
                    .height(list_height)
                    .show(ui, |row| {
                        let (depth, item) = rows[row];
                        let depth = if query.is_empty() { depth } else { 0 };
                        let mut list_row = ListRow::new(&item.name)
                            .icon(crate::structure_view::icon(item.kind))
                            .depth(depth);
                        if let Some(detail) = &item.detail {
                            list_row = list_row.secondary(detail);
                        }
                        list_row
                    });
            }
            structure_options(ui);
        });
    if !stay {
        return false;
    }
    // A click or double-click in the list picks that row; the keyboard keeps `index`.
    if let Some(clicked) = chosen.filter(|clicked| *clicked != index) {
        open = Some(clicked);
    }
    *selected = Some(index);
    if let Some(item) = open
        .and_then(|index| rows.get(index))
        .map(|(_, item)| (*item).clone())
    {
        crate::structure_view::navigate(app, ctx, &item);
        return false;
    }
    true
}

/// Option checkboxes under the structure list. The tags-based outline has no
/// inherited members, anonymous classes or lambdas to show, so they are
/// disabled (and unchecked) rather than pretending to filter.
fn structure_options(ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        for label in [
            "Inherited members (Ctrl+F12)",
            "Anonymous classes (Ctrl+I)",
            "Lambdas (Ctrl+L)",
        ] {
            let mut unchecked = false;
            ui.add(Checkbox::new(&mut unchecked, label).enabled(false))
                .on_disabled_hover_text(
                    "Not available: the outline is built without type information",
                );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_line_and_column() {
        assert_eq!(parse_line_column("12"), Some((12, None)));
        assert_eq!(parse_line_column(" 12:5 "), Some((12, Some(5))));
        assert_eq!(parse_line_column("3,"), Some((3, None)));
        assert_eq!(parse_line_column("x"), None);
        assert_eq!(parse_line_column("4:y"), None);
    }
}

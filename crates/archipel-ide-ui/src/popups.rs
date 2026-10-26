//! Small modal popups: New File/Directory, Rename, Delete, Go to Line, Recent Files.

use crate::icons;
use crate::native_dialog::Purpose;
use crate::widgets::row_text;
use crate::{IdeApp, theme};
use archipel_ide_core::{JavaKind, KotlinKind};
use archipel_kit::parts::Surface;
use eframe::egui::{self, Rect, RichText, Sense, vec2};
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
        usages: Vec<archipel_ide_core::project_view::Usage>,
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

/// Dims the workbench and swallows clicks behind a modal dialog, so nothing
/// underneath can be activated while it is open (see `docs/ux/interaction-spec.md`).
/// Draw it before the dialog's own `Order::Foreground` area.
pub(crate) fn modal_backdrop(ctx: &egui::Context, id: &str) {
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Middle)
        .fixed_pos(screen.min)
        .interactable(true)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(screen.size(), Sense::click_and_drag());
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(80));
        });
}

pub(crate) fn title(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme::semibold(theme::UI_FONT_SIZE))
            .color(theme::TEXT_BRIGHT),
    );
    ui.add_space(6.0);
}

fn name_field(ui: &mut egui::Ui, text: &mut String, focus: bool) -> (bool, bool) {
    // TextEdit consumes Enter itself; capture submission before drawing it.
    let submit = ui.memory(|memory| memory.has_focus(input_id()))
        && ui.input(|input| input.key_pressed(egui::Key::Enter));
    let response = ui.add(
        egui::TextEdit::singleline(text)
            .id(input_id())
            .desired_width(f32::INFINITY)
            .margin(egui::Margin::symmetric(6, 5)),
    );
    if focus {
        response.request_focus();
    }
    (submit, response.changed())
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(mut dialog) = app.dialog.take() else {
        return;
    };
    let focus = std::mem::take(&mut app.dialog_focus);
    let mut keep = true;
    let mut error = None;
    let mut browse = None;
    match &mut dialog {
        Dialog::NewProject(form) => {
            keep = crate::new_project::show(app, ctx, form, focus, &mut browse);
        }
        Dialog::Clone(form) => {
            keep = crate::clone_dialog::show(app, ctx, form, focus, &mut browse);
        }
        Dialog::OpenProject {
            path,
            error: open_error,
        } => {
            let mut submit = false;
            popup_frame(ctx, "dialog_open_project", 520.0, |ui| {
                title(ui, "Open Project");
                ui.label("Project directory");
                let changed;
                (submit, changed) = name_field(ui, path, focus);
                if changed {
                    *open_error = None;
                }
                ui.label(
                    RichText::new("Enter the full path to an existing folder.")
                        .color(theme::TEXT_INFO),
                );
                if let Some(message) = open_error.as_ref() {
                    ui.label(RichText::new(message).color(theme::ERROR));
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Browse…").clicked() {
                        browse = Some(Purpose::OpenProject);
                    }
                    if ui
                        .add_enabled(!path.trim().is_empty(), egui::Button::new("Open"))
                        .clicked()
                    {
                        submit = true;
                    }
                    if ui.button("Cancel").clicked() {
                        keep = false;
                    }
                });
            });
            if submit && keep && !path.trim().is_empty() {
                match app.open_project_path(path, ctx) {
                    Ok(()) => keep = false,
                    Err(message) => *open_error = Some(message),
                }
            }
        }
        Dialog::NewEntry {
            parent,
            directory,
            name,
        } => {
            let label = if *directory {
                "New Directory"
            } else {
                "New File"
            };
            let mut submit = false;
            popup_frame(ctx, "dialog_new", 360.0, |ui| {
                title(ui, label);
                (submit, _) = name_field(ui, name, focus);
                let hint = if *directory {
                    "Name, or a path like a/b/c"
                } else {
                    "Name with extension, or a path like src/lib.rs"
                };
                ui.label(RichText::new(hint).color(theme::TEXT_INFO));
            });
            if submit && !name.trim().is_empty() {
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
                    Err(e) => error = Some(e.to_string()),
                }
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
            let mut submit = false;
            popup_frame(ctx, "dialog_new_class", 360.0, |ui| {
                title(
                    ui,
                    if *kotlin {
                        "New Kotlin Class/File"
                    } else {
                        "New Java Class"
                    },
                );
                let changed;
                (submit, changed) = name_field(ui, name, focus);
                if changed {
                    *class_error = None;
                }
                ui.add_space(4.0);
                for (index, (_, label)) in kinds.iter().enumerate() {
                    let (rect, response) = crate::widgets::list_row(ui, index == *kind);
                    let text_color = if index == *kind {
                        theme::TEXT_BRIGHT
                    } else {
                        theme::TEXT
                    };
                    row_text(
                        ui,
                        rect,
                        rect.left() + 12.0,
                        label,
                        egui::FontId::proportional(theme::UI_FONT_SIZE),
                        text_color,
                    );
                    if response.clicked() {
                        *kind = index;
                    }
                    if response.double_clicked() {
                        submit = true;
                    }
                }
                if let Some(message) = class_error.as_ref() {
                    ui.add_space(4.0);
                    ui.label(RichText::new(message).color(theme::ERROR));
                } else {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Name, or a qualified name like com.foo.Bar")
                            .color(theme::TEXT_INFO),
                    );
                }
            });
            if submit && !name.trim().is_empty() {
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
                    Err(e) => *class_error = Some(e.to_string()),
                }
            }
        }
        Dialog::Rename { path, name } => {
            let mut submit = false;
            let label = format!("Rename {}", IdeApp::file_name(path));
            popup_frame(ctx, "dialog_rename", 360.0, |ui| {
                title(ui, &label);
                (submit, _) = name_field(ui, name, focus);
            });
            if submit && !name.trim().is_empty() {
                match app.commands.execute(
                    "project.rename",
                    &mut app.state,
                    json!({"path": path, "name": name.trim()}),
                ) {
                    Ok(result) => {
                        keep = false;
                        app.tree_selection = result["path"].as_str().map(PathBuf::from);
                    }
                    Err(e) => error = Some(e.to_string()),
                }
            }
        }
        Dialog::Delete { path } => {
            let mut confirm =
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
            let mut cancel = false;
            let is_dir = app
                .state
                .files
                .iter()
                .any(|e| e.is_dir && e.relative_path == *path);
            popup_frame(ctx, "dialog_delete", 380.0, |ui| {
                title(ui, "Delete");
                let what = if is_dir { "directory" } else { "file" };
                ui.label(format!("Delete {what} \"{}\"?", path.display()));
                if is_dir {
                    ui.label(
                        RichText::new("All files inside will be deleted.").color(theme::TEXT_INFO),
                    );
                }
                ui.add_space(10.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let delete =
                        egui::Button::new(RichText::new("Delete").color(egui::Color32::WHITE))
                            .fill(theme::ACCENT);
                    if ui.add(delete).clicked() {
                        confirm = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
            if cancel {
                keep = false;
            } else if confirm {
                keep = false;
                app.dispatch("project.delete", json!({"path": path}));
                app.tree_selection = None;
            }
        }
        Dialog::SafeDelete { path, usages } => {
            let mut confirm = false;
            let mut cancel =
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            popup_frame(ctx, "dialog_safe_delete", 460.0, |ui| {
                title(ui, "Safe Delete");
                ui.label(format!("Delete \"{}\"?", path.display()));
                if usages.is_empty() {
                    ui.label(RichText::new("No usages found.").color(theme::TEXT_INFO));
                } else {
                    let count = usages.len();
                    let more = if count >= archipel_ide_core::project_view::MAX_USAGES {
                        "+"
                    } else {
                        ""
                    };
                    ui.label(
                        RichText::new(format!("{count}{more} usage(s) of the name remain:"))
                            .color(theme::WARNING),
                    );
                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .show(ui, |ui| {
                            for usage in usages.iter() {
                                ui.label(
                                    RichText::new(format!(
                                        "{}:{}  {}",
                                        usage.path.display(),
                                        usage.line,
                                        usage.text
                                    ))
                                    .monospace()
                                    .color(theme::TEXT_SECONDARY),
                                );
                            }
                        });
                }
                ui.add_space(10.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if usages.is_empty() {
                        "Delete"
                    } else {
                        "Delete Anyway"
                    };
                    let delete =
                        egui::Button::new(RichText::new(label).color(egui::Color32::WHITE))
                            .fill(theme::ACCENT);
                    if ui.add(delete).clicked() {
                        confirm = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
            if cancel {
                keep = false;
            } else if confirm {
                keep = false;
                app.dispatch("project.safeDelete", json!({"path": path, "force": true}));
                app.tree_selection = None;
            }
        }
        Dialog::Move { path, destination } => {
            let mut submit = false;
            let label = format!("Move {}", IdeApp::file_name(path));
            popup_frame(ctx, "dialog_move", 400.0, |ui| {
                title(ui, &label);
                ui.label(RichText::new("To directory (project-relative):").color(theme::TEXT_INFO));
                (submit, _) = name_field(ui, destination, focus);
            });
            if submit {
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
                    Err(e) => error = Some(e.to_string()),
                }
            }
        }
        Dialog::GotoLine { input } => {
            let mut submit = false;
            let caret = app
                .state
                .active_document
                .and_then(|index| app.state.documents.get(index))
                .and_then(|d| d.text.char_to_line_column(d.selections.primary().head).ok())
                .map(|(l, c)| format!("{}:{}", l + 1, c + 1))
                .unwrap_or_default();
            popup_frame(ctx, "dialog_goto", 300.0, |ui| {
                title(ui, "Go to Line:Column");
                (submit, _) = name_field(ui, input, focus);
                ui.label(
                    RichText::new(format!("Current position {caret}")).color(theme::TEXT_INFO),
                );
            });
            if submit {
                match parse_line_column(input) {
                    Some((line, column)) => {
                        let mut params = json!({"line": line});
                        if let Some(column) = column {
                            params["column"] = json!(column);
                        }
                        app.dispatch("editor.gotoLine", params);
                        ctx.memory_mut(|m| m.request_focus(crate::editor::editor_id()));
                        keep = false;
                    }
                    None => {
                        error = Some("Enter a line number, optionally followed by :column".into())
                    }
                }
            }
        }
        Dialog::Recent { query, selected } => {
            let entries: Vec<PathBuf> = archipel_ide_core::quick_nav::rank(
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
    if let Some(error) = error {
        app.state.status = error;
    }
    if keep {
        app.dialog = Some(dialog);
    }
    if let Some(purpose) = browse {
        app.browse(purpose);
    }
}

/// File Structure popup (⌘F12): filterable outline of the active file.
/// Returns whether the popup stays open.
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
        .filter(|(_, item)| archipel_ide_core::quick_nav::matches(query, &item.name))
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
    popup_frame(ctx, "dialog_file_structure", 560.0, |ui| {
        // Centered file name, as in IntelliJ; typing filters (speed search).
        ui.vertical_centered(|ui| title(ui, &file));
        ui.add(
            egui::TextEdit::singleline(query)
                .id(input_id())
                .desired_width(f32::INFINITY)
                .margin(egui::Margin::symmetric(6, 4))
                .hint_text("Type to filter"),
        )
        .request_focus();
        ui.add_space(4.0);
        if rows.is_empty() {
            ui.label(RichText::new("Nothing to show").color(theme::TEXT_SECONDARY));
        }
        egui::ScrollArea::vertical()
            .max_height(420.0)
            .min_scrolled_height(420.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for (row_index, (depth, item)) in rows.iter().enumerate() {
                    let depth = if query.is_empty() { *depth } else { 0 };
                    let response =
                        crate::structure_view::row(ui, depth, item, row_index == index, None)
                            .response;
                    if row_index == index {
                        response.scroll_to_me(None);
                    }
                    if response.clicked() {
                        open = Some(row_index);
                    }
                }
            });
        ui.add_space(4.0);
        structure_options(ui);
    });
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
            ui.add_enabled(false, egui::Checkbox::new(&mut unchecked, label))
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
    fn focused_name_field_submits_on_enter() {
        let ctx = egui::Context::default();
        let mut text = String::from("/project");
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                name_field(ui, &mut text, true);
            });
        });
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut submitted = false;
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                submitted = name_field(ui, &mut text, false).0;
            });
        });
        assert!(submitted);
    }

    #[test]
    fn parses_line_and_column() {
        assert_eq!(parse_line_column("12"), Some((12, None)));
        assert_eq!(parse_line_column(" 12:5 "), Some((12, Some(5))));
        assert_eq!(parse_line_column("3,"), Some((3, None)));
        assert_eq!(parse_line_column("x"), None);
        assert_eq!(parse_line_column("4:y"), None);
    }
}

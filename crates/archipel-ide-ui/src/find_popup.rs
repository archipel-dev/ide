//! Find in Files popup: query with option toggles, scope tabs, live results and a
//! read-only preview of the selected match.
//!
//! The search itself is the `search.project` command (with `background`, so the
//! Find tool window stays closed); this file renders its results as they stream in.

use crate::icons;
use crate::{IdeApp, theme, widgets};
use archipel_ide_core::SearchScopeKind;
use archipel_ide_core::TextMatch;
use eframe::egui::{self, Rect, Sense, vec2};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const DEBOUNCE: Duration = Duration::from_millis(120);
const ROW_HEIGHT: f32 = 24.0;
const PREVIEW_LINES: usize = 5;

/// Scope tabs, in display order. The last one picks among the named scopes.
const TABS: [(&str, SearchScopeKind); 3] = [
    ("In Project", SearchScopeKind::Project),
    ("Module", SearchScopeKind::Module),
    ("Directory", SearchScopeKind::Directory),
];
const CUSTOM_SCOPES: [(&str, SearchScopeKind); 3] = [
    ("Project Production Files", SearchScopeKind::ProductionFiles),
    ("Project Test Files", SearchScopeKind::TestFiles),
    ("Open Files", SearchScopeKind::OpenFiles),
];

#[derive(Default)]
pub struct FindPopup {
    pub open: bool,
    select_query: bool,
    selected: usize,
    /// Last time the query, options or scope changed and has not been searched yet.
    dirty_since: Option<Instant>,
    /// Text of the previewed file, loaded once per selected path.
    preview: Option<(PathBuf, Vec<String>)>,
    /// Name and mask being edited in the custom scope row.
    scope_edit: Option<(String, String)>,
}

impl FindPopup {
    /// A change is waiting for its debounced search.
    pub fn pending(&self) -> bool {
        self.open && self.dirty_since.is_some()
    }

    pub fn selected(&self) -> usize {
        self.selected
    }
}

impl IdeApp {
    /// Ctrl+Shift+F: opens the popup with the editor selection (or the previous
    /// query) fully selected, and searches right away.
    pub(crate) fn open_find_popup(&mut self, selection: Option<String>) {
        if self.state.project.is_none() {
            return;
        }
        if let Some(selection) = selection.filter(|s| !s.is_empty() && !s.contains('\n')) {
            self.search_input = selection;
        }
        if self.search_input.is_empty() {
            self.search_input = self.state.search_query();
        }
        let popup = &mut self.find_popup;
        popup.open = true;
        popup.select_query = true;
        popup.selected = 0;
        popup.dirty_since = Some(Instant::now() - DEBOUNCE);
    }
}

fn run_search(app: &mut IdeApp) {
    app.find_popup.dirty_since = None;
    app.find_popup.selected = 0;
    if app.search_input.is_empty() {
        return;
    }
    let scope = app.state.project_view.search_scope.clone();
    let kind = app.state.search_scope_kind();
    let options = &app.find_options;
    let params = json!({
        "query": app.search_input,
        "matchCase": options.match_case,
        "wholeWords": options.whole_words,
        "regex": options.regex,
        "fileMask": options.file_mask,
        "scope": scope.map(|s| s.display().to_string()).unwrap_or_default(),
        "scopeKind": kind.id(),
        "scopeName": app.state.search_custom_scope(),
        "background": true,
    });
    app.dispatch("search.project", params);
}

/// Re-opens the preview source when the selected result is in another file.
fn load_preview(app: &mut IdeApp, path: &std::path::Path) {
    if app
        .find_popup
        .preview
        .as_ref()
        .is_some_and(|(loaded, _)| loaded == path)
    {
        return;
    }
    let lines = app
        .state
        .project
        .as_ref()
        .and_then(|project| std::fs::read_to_string(project.root().join(path)).ok())
        .map(|text| text.lines().map(str::to_owned).collect())
        .unwrap_or_default();
    app.find_popup.preview = Some((path.to_owned(), lines));
}

fn preview_job(
    lines: &[String],
    found: &TextMatch,
    font: &egui::FontId,
) -> (egui::text::LayoutJob, usize) {
    let first = found.line.saturating_sub(1 + PREVIEW_LINES / 2);
    let last = (first + PREVIEW_LINES * 2).min(lines.len());
    let mut job = egui::text::LayoutJob::default();
    let plain = |color| egui::TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    };
    for (offset, line) in lines[first.min(last)..last].iter().enumerate() {
        let number = first + offset + 1;
        job.append(
            &format!("{number:>4}  "),
            0.0,
            plain(*theme::editor::LINE_NUMBER),
        );
        let line_chars: Vec<char> = line.chars().collect();
        if number == found.line {
            let start = (found.column - 1).min(line_chars.len());
            let end = (start + found.length).min(line_chars.len());
            let part = |range: std::ops::Range<usize>| line_chars[range].iter().collect::<String>();
            job.append(&part(0..start), 0.0, plain(*theme::editor::TEXT));
            let mut hit = plain(*theme::editor::TEXT);
            hit.background = *theme::editor::SEARCH_RESULT;
            job.append(&part(start..end), 0.0, hit);
            job.append(
                &part(end..line_chars.len()),
                0.0,
                plain(*theme::editor::TEXT),
            );
        } else {
            job.append(line, 0.0, plain(*theme::editor::TEXT));
        }
        job.append("\n", 0.0, plain(*theme::editor::TEXT));
    }
    (job, first)
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    if !app.find_popup.open {
        return;
    }
    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        app.find_popup.open = false;
        ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
        return;
    }
    let results = app.state.search_results();
    if !results.is_empty() {
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
            app.find_popup.selected = (app.find_popup.selected + 1) % results.len();
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
            app.find_popup.selected = (app.find_popup.selected + results.len() - 1) % results.len();
        }
    }
    app.find_popup.selected = app.find_popup.selected.min(results.len().saturating_sub(1));
    let enter = ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
    let to_window =
        ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter));
    let screen = ctx.content_rect();
    let width = 780.0_f32.min(screen.width() - 40.0);
    let height = (screen.height() * 0.7).max(360.0);
    let mut chosen = enter.then_some(app.find_popup.selected);
    let mut open_window = to_window;
    egui::Area::new(egui::Id::new("find_in_files_popup"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(
            screen.center().x - width / 2.0,
            screen.top() + screen.height() * 0.12,
        ))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .corner_radius(10)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_width(width - 20.0);
                    ui.set_height(height);
                    ui.spacing_mut().item_spacing.y = 6.0;
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Find in Files").strong());
                        let files: std::collections::BTreeSet<_> =
                            results.iter().map(|found| &found.path).collect();
                        let mut summary = format!(
                            "{} in {}",
                            widgets::pluralize(results.len(), "match", "matches"),
                            widgets::pluralize(files.len(), "file", "files")
                        );
                        if app.state.search_running() {
                            summary.push('…');
                        }
                        if !app.state.search_query().is_empty() {
                            ui.label(egui::RichText::new(summary).color(theme::TEXT_SECONDARY));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            changed |= widgets::search_field(
                                ui,
                                egui::Id::new("find_popup_mask"),
                                &mut app.find_options.file_mask,
                                "File mask, e.g. *.java, !*Test.java",
                                220.0,
                            )
                            .changed();
                            ui.label(
                                egui::RichText::new("File mask:").color(theme::TEXT_SECONDARY),
                            );
                        });
                    });
                    ui.horizontal(|ui| {
                        let id = crate::toolwindows::find_input_id();
                        if std::mem::take(&mut app.find_popup.select_query) {
                            let mut state = egui::text_edit::TextEditState::load(ui.ctx(), id)
                                .unwrap_or_default();
                            state
                                .cursor
                                .set_char_range(Some(egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(0),
                                    egui::text::CCursor::new(app.search_input.chars().count()),
                                )));
                            state.store(ui.ctx(), id);
                            ui.memory_mut(|memory| memory.request_focus(id));
                        }
                        let query = widgets::search_field(
                            ui,
                            id,
                            &mut app.search_input,
                            "Text to find",
                            width - 190.0,
                        );
                        changed |= query.changed();
                        // Typing goes to the query unless another popup field has focus.
                        let elsewhere = ui.memory(|memory| {
                            memory.focused().is_some_and(|focused| {
                                [
                                    egui::Id::new("find_popup_mask"),
                                    egui::Id::new("find_popup_dir"),
                                ]
                                .iter()
                                .any(|field| memory.has_focus(*field) && focused == *field)
                            })
                        });
                        if !query.has_focus() && !elsewhere {
                            ui.memory_mut(|memory| memory.request_focus(id));
                        }
                        let options = &mut app.find_options;
                        for (label, tooltip, value) in [
                            ("Cc", "Match Case", &mut options.match_case),
                            ("W", "Words", &mut options.whole_words),
                            (".*", "Regex", &mut options.regex),
                        ] {
                            let toggle = ui
                                .selectable_label(*value, egui::RichText::new(label).monospace())
                                .on_hover_text(tooltip);
                            if toggle.clicked() {
                                *value = !*value;
                                changed = true;
                            }
                        }
                    });
                    let scope_kind = app.state.search_scope_kind();
                    let custom_scope = app.state.search_custom_scope();
                    let custom_scopes = app.state.custom_scopes();
                    ui.horizontal(|ui| {
                        for (label, kind) in TABS {
                            if scope_tab(ui, label, scope_kind == kind) {
                                app.state.set_search_scope_kind(kind);
                                changed = true;
                            }
                        }
                        let custom = CUSTOM_SCOPES.iter().find(|(_, kind)| *kind == scope_kind);
                        let current = custom.map_or_else(
                            || match (&app.state.search_custom_scope(), scope_kind) {
                                (Some(name), SearchScopeKind::Custom) => name.clone(),
                                _ => "Scope".to_owned(),
                            },
                            |(label, _)| (*label).to_owned(),
                        );
                        let menu = egui::ComboBox::from_id_salt("find_popup_scope")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                for (label, kind) in CUSTOM_SCOPES {
                                    if ui.selectable_label(scope_kind == kind, label).clicked() {
                                        app.state.set_search_scope_kind(kind);
                                        changed = true;
                                    }
                                }
                                if !custom_scopes.is_empty() {
                                    ui.separator();
                                }
                                for scope in custom_scopes {
                                    let selected = scope_kind == SearchScopeKind::Custom
                                        && custom_scope.as_ref() == Some(&scope.name);
                                    if ui
                                        .selectable_label(selected, &scope.name)
                                        .on_hover_text(&scope.mask)
                                        .clicked()
                                    {
                                        app.state.select_custom_scope(&scope.name);
                                        app.find_popup.scope_edit = Some((scope.name, scope.mask));
                                        changed = true;
                                    }
                                }
                                ui.separator();
                                if ui.selectable_label(false, "New scope…").clicked() {
                                    app.find_popup.scope_edit =
                                        Some((String::new(), String::new()));
                                }
                            });
                        let _ = menu;
                    });
                    if app.find_popup.scope_edit.is_some() {
                        scope_editor(app, ui, width, &mut changed);
                    }
                    if scope_kind == SearchScopeKind::Directory {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Directory:").color(theme::TEXT_SECONDARY),
                            );
                            let mut text = app
                                .state
                                .project_view
                                .search_scope
                                .as_ref()
                                .map(|dir| dir.display().to_string())
                                .unwrap_or_default();
                            if widgets::search_field(
                                ui,
                                egui::Id::new("find_popup_dir"),
                                &mut text,
                                "Project-relative directory",
                                width - 120.0,
                            )
                            .changed()
                            {
                                app.state.project_view.search_scope =
                                    (!text.is_empty()).then(|| PathBuf::from(text));
                                changed = true;
                            }
                        });
                    }
                    if changed {
                        app.find_popup.dirty_since = Some(Instant::now());
                    }
                    ui.separator();
                    let list_height = (height - 190.0 - PREVIEW_LINES as f32 * 34.0).max(80.0);
                    if results.is_empty() {
                        ui.add_space(8.0);
                        ui.vertical_centered(|ui| {
                            let text = if app.search_input.is_empty() {
                                "Type to search the project"
                            } else if app.state.search_running() {
                                "Searching…"
                            } else {
                                "Nothing found"
                            };
                            ui.label(egui::RichText::new(text).color(theme::TEXT_SECONDARY));
                        });
                    }
                    egui::ScrollArea::vertical()
                        .id_salt("find_popup_results")
                        .max_height(list_height)
                        .auto_shrink([false, false])
                        .show_rows(ui, ROW_HEIGHT, results.len(), |ui, range| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            for index in range {
                                if result_row(app, ui, &results[index], index) {
                                    chosen = Some(index);
                                }
                            }
                        });
                    ui.separator();
                    if let Some(found) = results.get(app.find_popup.selected) {
                        load_preview(app, &found.path);
                        let lines = app
                            .find_popup
                            .preview
                            .as_ref()
                            .map(|(_, lines)| lines.as_slice())
                            .unwrap_or_default();
                        let font = egui::FontId::monospace(theme::UI_FONT_SIZE);
                        let (job, _) = preview_job(lines, found, &font);
                        let (rect, _) = ui.allocate_exact_size(
                            vec2(width - 20.0, PREVIEW_LINES as f32 * 2.0 * 17.0),
                            Sense::hover(),
                        );
                        ui.painter()
                            .rect_filled(rect, 4.0, *theme::editor::BACKGROUND);
                        let galley = ui.painter().layout_job(job);
                        ui.painter().with_clip_rect(rect).galley(
                            rect.min + vec2(6.0, 4.0),
                            galley,
                            *theme::editor::TEXT,
                        );
                    }
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("↑↓ select · Enter open · Esc close")
                                .color(theme::TEXT_SECONDARY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Open in Find Window  Ctrl+Enter").clicked() {
                                open_window = true;
                            }
                        });
                    });
                });
        });
    if app
        .find_popup
        .dirty_since
        .is_some_and(|since| since.elapsed() >= DEBOUNCE)
    {
        run_search(app);
    } else if app.find_popup.dirty_since.is_some() || app.state.search_running() {
        ctx.request_repaint_after(Duration::from_millis(30));
    }
    if ctx.input(|input| input.pointer.any_click()) && !ctx.is_pointer_over_area() {
        app.find_popup.open = false;
    }
    if open_window {
        if app.find_popup.dirty_since.is_some() {
            run_search(app);
        }
        app.find_popup.open = false;
        app.dispatch("toolwindow.activate.find", Value::Null);
        return;
    }
    if let Some(index) = chosen
        && let Some(found) = results.get(index)
    {
        app.find_popup.open = false;
        app.dispatch(
            "editor.openFile",
            json!({
                "path": found.path,
                "line": found.line,
                "column": found.column,
                "length": found.length,
            }),
        );
        ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
    }
}

/// Row to define, change or delete a custom scope: a name and a file mask such
/// as `*.java, src/api/**, !**/generated/**`.
fn scope_editor(app: &mut IdeApp, ui: &mut egui::Ui, width: f32, changed: &mut bool) {
    let Some((mut name, mut mask)) = app.find_popup.scope_edit.clone() else {
        return;
    };
    let mut save = false;
    let mut delete = false;
    let mut close = false;
    ui.horizontal(|ui| {
        widgets::search_field(
            ui,
            egui::Id::new("find_popup_scope_name"),
            &mut name,
            "Scope name",
            110.0,
        );
        widgets::search_field(
            ui,
            egui::Id::new("find_popup_scope_mask"),
            &mut mask,
            "*.java, src/**, !**/generated/**",
            (width - 330.0).max(120.0),
        );
        save = ui.button("Save").clicked();
        delete = ui
            .add_enabled(
                app.state
                    .custom_scopes()
                    .iter()
                    .any(|scope| scope.name == name),
                egui::Button::new("Delete"),
            )
            .clicked();
        close = ui.button("×").on_hover_text("Close").clicked();
    });
    if save {
        app.dispatch(
            "search.saveScope",
            json!({"name": name.trim(), "mask": mask}),
        );
        if app
            .state
            .custom_scopes()
            .iter()
            .any(|s| s.name == name.trim())
        {
            app.state.select_custom_scope(name.trim());
            *changed = true;
        }
    }
    if delete {
        app.dispatch("search.deleteScope", json!({"name": name}));
        *changed = true;
        close = true;
    }
    app.find_popup.scope_edit = (!close).then_some((name, mask));
}

fn scope_tab(ui: &mut egui::Ui, label: &str, selected: bool) -> bool {
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        *theme::TEXT_BRIGHT,
    );
    let (rect, response) =
        ui.allocate_exact_size(vec2(galley.size().x + 20.0, 26.0), Sense::click());
    if selected {
        ui.painter().rect_filled(rect, 4.0, theme::SELECTION);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme::HOVER);
    }
    let color = if selected {
        *theme::TEXT_BRIGHT
    } else {
        *theme::TEXT_SECONDARY
    };
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, color);
    response.clicked()
}

fn result_row(app: &mut IdeApp, ui: &mut egui::Ui, found: &TextMatch, index: usize) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if index == app.find_popup.selected {
        ui.painter().rect_filled(rect, 4.0, theme::SELECTION);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme::HOVER);
    }
    icons::paint(
        ui.painter(),
        Rect::from_min_size(
            egui::pos2(rect.left() + 6.0, rect.center().y - 8.0),
            vec2(16.0, 16.0),
        ),
        app.file_icon(&found.path),
        *icons::ICON,
    );
    let location = ui.painter().layout_no_wrap(
        format!(
            "{} {}",
            found
                .path
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default(),
            found.line
        ),
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        *theme::TEXT_SECONDARY,
    );
    let x = rect.right() - 10.0 - location.size().x;
    ui.painter().galley(
        egui::pos2(x, rect.center().y - location.size().y / 2.0),
        location,
        *theme::TEXT_SECONDARY,
    );
    let galley = ui
        .painter()
        .layout_job(crate::toolwindows::highlighted_preview(
            &found.preview,
            &found.highlights,
        ));
    ui.painter()
        .with_clip_rect(rect.with_max_x(x - 8.0))
        .galley(
            egui::pos2(rect.left() + 30.0, rect.center().y - galley.size().y / 2.0),
            galley,
            *theme::editor::TEXT,
        );
    if response.hovered() && ui.input(|input| input.pointer.delta() != egui::Vec2::ZERO) {
        app.find_popup.selected = index;
    }
    response.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reopening_keeps_the_previous_query_and_results_follow_it() {
        let (mut app, ctx) = crate::tests::headless_app("find-popup", "a.txt", "needle\n");
        app.search_input = "needle".into();
        app.open_find_popup(None);
        assert!(app.find_popup.open && app.find_popup.select_query);
        for _ in 0..200 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| app.frame(ctx));
            if app.state.search_result_count() > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(app.state.search_result_count(), 1, "results without Enter");
        assert!(
            app.state
                .tool_windows
                .visible(archipel_ide_core::Anchor::Bottom)
                .is_none(),
            "live search leaves the Find tool window closed"
        );
        // The next open takes the editor selection over the old query.
        app.open_find_popup(Some("other".into()));
        assert_eq!(app.search_input, "other");
    }
}

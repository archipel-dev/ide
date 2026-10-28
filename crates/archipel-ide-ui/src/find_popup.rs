//! Find in Files and Replace in Files popup: query with option toggles, scope tabs, live
//! results and a read-only preview of the selected match.
//!
//! The search itself is the `search.project` command (with `background`, so the Find tool
//! window stays closed); this file renders its results as they stream in. What the popup says
//! comes from the core's model (`find_in_files`, spec `docs/specs/search/02-find-in-files.md`):
//! the validation messages, the info label, the text of an empty result and the confirmation of
//! Replace All; the replacement is the `search.replaceInFiles` command.

use crate::icons;
use crate::{IdeApp, theme, widgets};
use archipel_ide_core::SearchScopeKind;
use archipel_ide_core::TextMatch;
use archipel_ide_core::find_in_files::{
    FindRequest, FindResults, Scope, empty_text, replace_confirmation, validate,
};
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
    /// Replace in Files (Ctrl+Shift+R): the replace field and Replace All are shown.
    replace: bool,
    replacement: String,
    preserve_case: bool,
    /// The check box *File mask*: the mask applies only while it is on (SRCH-FIP-021).
    file_mask_on: bool,
    /// The confirmation of Replace All over several files (SRCH-FIP-071): title and message.
    confirm: Option<(String, String)>,
    /// The error of the last replacement (`Replace Error`).
    error: Option<String>,
}

impl FindPopup {
    /// A change is waiting for its debounced search.
    pub fn pending(&self) -> bool {
        self.open && self.dirty_since.is_some()
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The confirmation of Replace All is showing.
    pub fn confirm_open(&self) -> bool {
        self.open && self.confirm.is_some()
    }

    pub fn cancel_confirmation(&mut self) {
        self.confirm = None;
    }
}

impl IdeApp {
    /// Ctrl+Shift+F: opens the popup with the editor selection (or the previous
    /// query) fully selected, and searches right away.
    pub(crate) fn open_find_popup(&mut self, selection: Option<String>, replace: bool) {
        if !self.state.has_project() {
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
        popup.replace = replace;
        popup.confirm = None;
        popup.error = None;
        popup.select_query = true;
        popup.selected = 0;
        popup.dirty_since = Some(Instant::now() - DEBOUNCE);
    }
}

/// What the popup asks the model to find: the query, the options and the scope.
fn request(app: &IdeApp) -> FindRequest {
    let scope = match (app.state.search_scope_kind(), app.state.search_directory()) {
        (SearchScopeKind::Directory, Some(path)) => Scope::Directory {
            path,
            recursive: true,
        },
        _ => Scope::Project,
    };
    FindRequest {
        query: app.search_input.clone(),
        options: app.find_options.clone(),
        file_mask_on: app.find_popup.file_mask_on,
        scope,
    }
}

/// Why the search cannot run (SRCH-FIP-030): the message of the model.
fn invalid(app: &IdeApp) -> Option<String> {
    validate(&request(app), |directory| {
        app.state
            .project()
            .is_some_and(|project| project.root().join(directory).is_dir())
    })
    .err()
}

fn run_search(app: &mut IdeApp) {
    app.find_popup.dirty_since = None;
    app.find_popup.selected = 0;
    if app.search_input.is_empty() || invalid(app).is_some() {
        return;
    }
    let scope = app.state.search_directory();
    let kind = app.state.search_scope_kind();
    let options = &app.find_options;
    let params = json!({
        "query": app.search_input,
        "matchCase": options.match_case,
        "wholeWords": options.whole_words,
        "regex": options.regex,
        "fileMask": if app.find_popup.file_mask_on { options.file_mask.as_str() } else { "" },
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

/// Popup width (UI-POP-040: 652 px).
const FIND_POPUP_WIDTH: f32 = 652.0;

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    if !app.find_popup.open {
        return;
    }
    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        app.find_popup.open = false;
        ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
        return;
    }
    let message = invalid(app);
    // Nothing is listed while the request cannot run (SRCH-FIP-030).
    let results = if message.is_some() {
        Vec::new()
    } else {
        app.state.search_results()
    };
    if !results.is_empty() {
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
            app.find_popup.selected = (app.find_popup.selected + 1) % results.len();
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
            app.find_popup.selected = (app.find_popup.selected + results.len() - 1) % results.len();
        }
    }
    app.find_popup.selected = app.find_popup.selected.min(results.len().saturating_sub(1));
    // Alt+Shift+Enter is Replace All in replace mode; Alt+Shift+Backspace resets the filters.
    let replace_all = app.find_popup.replace
        && ctx.input_mut(|input| {
            input.consume_key(
                egui::Modifiers::ALT | egui::Modifiers::SHIFT,
                egui::Key::Enter,
            )
        });
    if ctx.input_mut(|input| {
        input.consume_key(
            egui::Modifiers::ALT | egui::Modifiers::SHIFT,
            egui::Key::Backspace,
        )
    }) {
        reset_filters(app);
    }
    // The mnemonics of the options (SRCH-FIP key table): Alt+C, Alt+W, Alt+X, Alt+E, Alt+K.
    let mut toggled = false;
    for (key, option) in [
        (egui::Key::C, 0),
        (egui::Key::W, 1),
        (egui::Key::X, 2),
        (egui::Key::E, 3),
        (egui::Key::K, 4),
    ] {
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, key)) {
            let popup = &mut app.find_popup;
            match option {
                0 => app.find_options.match_case = !app.find_options.match_case,
                1 => app.find_options.whole_words = !app.find_options.whole_words,
                2 => app.find_options.regex = !app.find_options.regex,
                3 if popup.replace => popup.preserve_case = !popup.preserve_case,
                4 => popup.file_mask_on = !popup.file_mask_on,
                _ => continue,
            }
            toggled = true;
        }
    }
    if toggled {
        app.find_popup.dirty_since = Some(Instant::now());
    }
    let enter = ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
    let to_window =
        ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter));
    let screen = ctx.content_rect();
    let width = FIND_POPUP_WIDTH.min(screen.width() - 40.0);
    let height = (screen.height() * 0.7).max(360.0);
    let mut chosen = (enter && !app.find_popup.replace && app.find_popup.confirm.is_none())
        .then_some(app.find_popup.selected);
    let mut open_window = to_window && !app.find_popup.replace;
    // Enter answers the confirmation of Replace All with its default button.
    let mut confirmed = enter && app.find_popup.confirm.is_some();
    let mut replace_clicked = replace_all;
    // Enter replaces the selected result in replace mode (SRCH-FIP-070).
    let replace_selected =
        (enter && app.find_popup.replace && !confirmed).then_some(app.find_popup.selected);
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
                        let title = if app.find_popup.replace {
                            "Replace in Files"
                        } else {
                            "Find in Files"
                        };
                        ui.label(egui::RichText::new(title).strong());
                        if let Some(info) = info_label(app, &results, message.as_deref()) {
                            ui.label(egui::RichText::new(info).color(theme::TEXT_SECONDARY));
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
                            changed |= ui
                                .checkbox(&mut app.find_popup.file_mask_on, "File mask")
                                .on_hover_text("File mask (Alt+K)")
                                .changed();
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
                                    egui::Id::new("find_popup_replacement"),
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
                    if app.find_popup.replace {
                        ui.horizontal(|ui| {
                            widgets::search_field(
                                ui,
                                egui::Id::new("find_popup_replacement"),
                                &mut app.find_popup.replacement,
                                "Replace",
                                width - 190.0,
                            );
                            let preserve = ui
                                .selectable_label(
                                    app.find_popup.preserve_case,
                                    egui::RichText::new("AB").monospace(),
                                )
                                .on_hover_text("Preserve Case (Alt+E)");
                            if preserve.clicked() {
                                app.find_popup.preserve_case = !app.find_popup.preserve_case;
                            }
                        });
                        if let Some(error) = &app.find_popup.error {
                            ui.label(egui::RichText::new(error).color(theme::ERROR));
                        }
                    }
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
                                .search_directory()
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
                                app.state.set_search_directory(
                                    (!text.is_empty()).then(|| PathBuf::from(text)),
                                );
                                changed = true;
                            }
                        });
                    }
                    if changed {
                        app.find_popup.dirty_since = Some(Instant::now());
                    }
                    ui.separator();
                    let list_height = (height - 190.0 - PREVIEW_LINES as f32 * 34.0).max(80.0);
                    if let Some((title, text)) = app.find_popup.confirm.clone() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(title).strong());
                            ui.label(text);
                            if ui.button("Replace").clicked() {
                                confirmed = true;
                            }
                            if ui.button("Cancel").clicked() {
                                app.find_popup.confirm = None;
                            }
                        });
                    }
                    if results.is_empty() {
                        ui.add_space(8.0);
                        ui.vertical_centered(|ui| {
                            for line in empty_results_text(app, message.as_deref()).lines() {
                                ui.label(egui::RichText::new(line).color(theme::TEXT_SECONDARY));
                            }
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
                            if app.find_popup.replace {
                                if ui
                                    .add_enabled(
                                        !results.is_empty(),
                                        egui::Button::new("Replace All"),
                                    )
                                    .on_hover_text("Replace All (Alt+Shift+Enter)")
                                    .clicked()
                                {
                                    replace_clicked = true;
                                }
                            } else if ui.button("Open in Find Window  Ctrl+Enter").clicked() {
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
    if replace_clicked && !results.is_empty() {
        start_replace_all(app, &results);
    }
    if confirmed {
        run_replace(app);
    }
    if let Some(found) = replace_selected.and_then(|index| results.get(index)) {
        replace_row(app, found);
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

/// The info label beside the title: `N matches in M files`, with `+` while the search runs
/// (SRCH-FIP-040); none while the request is invalid or nothing was searched yet.
fn info_label(app: &IdeApp, results: &[TextMatch], message: Option<&str>) -> Option<String> {
    (message.is_none() && !app.state.search_query().is_empty())
        .then(|| FindResults::label_of(results, app.state.search_running()))
}

/// What the results area says while it lists nothing (SRCH-FIP-030, 040, 046).
fn empty_results_text(app: &IdeApp, message: Option<&str>) -> String {
    if let Some(message) = message {
        message.to_owned()
    } else if app.state.search_running() {
        "Searching…".to_owned()
    } else if app.search_input.is_empty() {
        "Type to search the project".to_owned()
    } else {
        empty_text(&request(app))
    }
}

/// Alt+Shift+Backspace: file mask off, Match case, Words and Regex off (SRCH-FIP-022).
fn reset_filters(app: &mut IdeApp) {
    app.find_popup.file_mask_on = false;
    app.find_options.match_case = false;
    app.find_options.whole_words = false;
    app.find_options.regex = false;
    app.find_popup.dirty_since = Some(Instant::now());
}

/// Replace All (SRCH-FIP-071): over several files the popup asks first.
fn start_replace_all(app: &mut IdeApp, results: &[TextMatch]) {
    app.find_popup.error = None;
    let found = FindResults::from_rows(results.to_vec());
    match replace_confirmation(&found, &app.search_input, &app.find_popup.replacement) {
        Some(confirmation) => app.find_popup.confirm = Some(confirmation),
        None => run_replace(app),
    }
}

/// Replace (selected): one occurrence of one result; the list is searched again.
fn replace_row(app: &mut IdeApp, found: &TextMatch) {
    app.find_popup.error = None;
    let options = &app.find_options;
    let params = json!({
        "query": app.search_input,
        "replacement": app.find_popup.replacement,
        "matchCase": options.match_case,
        "wholeWords": options.whole_words,
        "regex": options.regex,
        "preserveCase": app.find_popup.preserve_case,
        "path": found.path,
        "line": found.line,
        "column": found.column,
        "length": found.length,
    });
    match app
        .commands
        .execute("search.replaceRow", &mut app.state, params)
    {
        Ok(_) => app.find_popup.dirty_since = Some(Instant::now()),
        Err(error) => app.find_popup.error = Some(format!("Replace Error: {error}")),
    }
}

/// Runs `search.replaceInFiles` for the popup's request; the popup closes when it worked.
fn run_replace(app: &mut IdeApp) {
    app.find_popup.confirm = None;
    let options = &app.find_options;
    let params = json!({
        "query": app.search_input,
        "replacement": app.find_popup.replacement,
        "matchCase": options.match_case,
        "wholeWords": options.whole_words,
        "regex": options.regex,
        "preserveCase": app.find_popup.preserve_case,
        "fileMask": if app.find_popup.file_mask_on { options.file_mask.as_str() } else { "" },
        "scope": app
            .state
            .search_directory()
            .filter(|_| app.state.search_scope_kind() == SearchScopeKind::Directory)
            .map(|scope| scope.display().to_string())
            .unwrap_or_default(),
    });
    match app
        .commands
        .execute("search.replaceInFiles", &mut app.state, params)
    {
        Ok(outcome) => match outcome.get("error").and_then(Value::as_str) {
            Some(error) => app.find_popup.error = Some(format!("Replace Error: {error}")),
            None => {
                if let Some(notification) = outcome.get("notification").and_then(Value::as_str) {
                    app.state.set_status(notification.to_owned());
                }
                app.find_popup.open = false;
            }
        },
        Err(error) => app.find_popup.error = Some(error.to_string()),
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
    let galley = ui.painter().layout_job(crate::toolwindows::popup_preview(
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
    use crate::tests::{click, idle, press, type_text};

    #[test]
    fn reopening_keeps_the_previous_query_and_results_follow_it() {
        let (mut app, ctx) = crate::tests::headless_app("find-popup", "a.txt", "needle\n");
        app.search_input = "needle".into();
        app.open_find_popup(None, false);
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
        app.open_find_popup(Some("other".into()), false);
        assert_eq!(app.search_input, "other");
    }

    /// A project with the given files and the first one open.
    fn project(name: &str, files: &[(&str, &str)]) -> (IdeApp, egui::Context, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-ui-test-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        for (path, text) in files {
            std::fs::write(root.join(path), text).unwrap();
        }
        let app = IdeApp::new(Some(root.clone()), vec![root.join(files[0].0)]);
        let ctx = egui::Context::default();
        crate::install_theme(&ctx);
        (app, ctx, root)
    }

    /// Runs frames until the background search of the popup has delivered its results.
    fn wait_for_results(app: &mut IdeApp, ctx: &egui::Context, count: usize) {
        for _ in 0..300 {
            idle(app, ctx);
            if app.state.search_result_count() >= count && !app.state.search_running() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("the search found {} rows", app.state.search_result_count());
    }

    fn results_of(app: &IdeApp) -> Vec<TextMatch> {
        app.state.search_results()
    }

    #[test]
    fn find_in_files_shows_the_model_label_and_the_validation_messages() {
        let (mut app, ctx, root) = project(
            "fip-label",
            &[
                ("A.txt", "needle needle\nother\n"),
                ("B.txt", "a needle\n"),
                ("C.txt", "nothing\n"),
            ],
        );
        app.focus_editor(&ctx);
        idle(&mut app, &ctx);
        press(&mut app, &ctx, "Ctrl+Shift+F");
        idle(&mut app, &ctx);
        assert!(app.find_popup.open && !app.find_popup.replace);
        // Nothing typed: the model asks for a query (SRCH-FIP-030).
        let message = invalid(&app);
        assert_eq!(
            message.as_deref(),
            Some("Type search query to find in files")
        );
        type_text(&mut app, &ctx, "needle");
        wait_for_results(&mut app, &ctx, 2);
        assert_eq!(invalid(&app), None);
        let rows = results_of(&app);
        assert_eq!(
            info_label(&app, &rows, None).as_deref(),
            Some("3 matches in 2 files"),
            "occurrences, not rows (SRCH-FIP-040)"
        );
        // The rows are grouped by file, in path order, with the preview of the line.
        assert_eq!(rows[0].path, PathBuf::from("A.txt"));
        assert_eq!(rows[1].path, PathBuf::from("B.txt"));
        assert_eq!(rows[0].preview, "needle needle");
        // Regex with an invalid pattern: the message replaces the results.
        press(&mut app, &ctx, "Alt+X");
        type_text(&mut app, &ctx, "(");
        let message = invalid(&app).expect("a bad pattern");
        assert!(
            message.starts_with("Bad pattern \"needle(\": "),
            "{message}"
        );
        assert_eq!(empty_results_text(&app, Some(&message)), message);
        // A regex that matches the empty string.
        for _ in 0..7 {
            press(&mut app, &ctx, "Backspace");
        }
        type_text(&mut app, &ctx, "a*");
        assert_eq!(
            invalid(&app).as_deref(),
            Some("Regular expression matches empty string")
        );
        // A file mask with a semicolon (SRCH-FIP-021).
        press(&mut app, &ctx, "Alt+X");
        press(&mut app, &ctx, "Alt+K");
        app.find_options.file_mask = "*.txt;*.md".into();
        assert_eq!(
            invalid(&app).as_deref(),
            Some("File masks should be comma-separated")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn an_empty_result_lists_the_options_that_narrowed_it() {
        let (mut app, ctx, root) = project("fip-empty", &[("A.txt", "needle\n")]);
        app.focus_editor(&ctx);
        idle(&mut app, &ctx);
        press(&mut app, &ctx, "Ctrl+Shift+F");
        idle(&mut app, &ctx);
        press(&mut app, &ctx, "Alt+C");
        type_text(&mut app, &ctx, "NEEDLE");
        // The search runs 120 ms after the last change and finds nothing.
        for _ in 0..300 {
            idle(&mut app, &ctx);
            if !app.find_popup.pending() && !app.state.search_running() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            empty_results_text(&app, None),
            "Nothing found.\nSearch option used: Match case\nClear option Alt+Shift+Backspace"
        );
        // Alt+Shift+Backspace resets the filters (SRCH-FIP-022).
        press(&mut app, &ctx, "Alt+Shift+Backspace");
        assert!(!app.find_options.match_case);
        wait_for_results(&mut app, &ctx, 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn replace_all_over_two_files_asks_first_and_replaces_with_the_status() {
        let (mut app, ctx, root) = project(
            "fip-replace-all",
            &[
                ("A.txt", "needle needle\nother\n"),
                ("B.txt", "a needle\n"),
                ("C.txt", "nothing\n"),
            ],
        );
        app.focus_editor(&ctx);
        idle(&mut app, &ctx);
        press(&mut app, &ctx, "Ctrl+Shift+R");
        idle(&mut app, &ctx);
        assert!(app.find_popup.open && app.find_popup.replace);
        type_text(&mut app, &ctx, "needle");
        wait_for_results(&mut app, &ctx, 2);
        // A click into the replace field, then the replacement.
        let field = ctx
            .read_response(egui::Id::new("find_popup_replacement"))
            .expect("the replace field")
            .rect;
        click(&mut app, &ctx, field.center());
        type_text(&mut app, &ctx, "pin");
        assert_eq!(app.find_popup.replacement, "pin");
        // Alt+Shift+Enter is Replace All; two files: the confirmation comes first.
        press(&mut app, &ctx, "Alt+Shift+Enter");
        let (title, message) = app.find_popup.confirm.clone().expect("a confirmation");
        assert_eq!(title, "Replace All");
        assert_eq!(
            message,
            "Replace 3 occurrences of 'needle' across 2 files with 'pin'?"
        );
        // Escape answers the confirmation, not the popup.
        press(&mut app, &ctx, "Escape");
        assert!(app.find_popup.confirm.is_none() && app.find_popup.open);
        assert_eq!(
            std::fs::read_to_string(root.join("B.txt")).unwrap(),
            "a needle\n"
        );
        press(&mut app, &ctx, "Alt+Shift+Enter");
        press(&mut app, &ctx, "Enter");
        assert!(
            !app.find_popup.open,
            "the popup closes after the replacement"
        );
        assert_eq!(app.state.status(), "3 occurrences replaced");
        // A.txt is open and clean: it follows the file; B.txt was written.
        assert_eq!(
            std::fs::read_to_string(root.join("A.txt")).unwrap(),
            "pin pin\nother\n"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("B.txt")).unwrap(),
            "a pin\n"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("C.txt")).unwrap(),
            "nothing\n"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn enter_in_replace_mode_replaces_the_selected_result_only() {
        let (mut app, ctx, root) = project(
            "fip-replace-row",
            &[("A.txt", "one needle\n"), ("B.txt", "two needle\n")],
        );
        app.focus_editor(&ctx);
        idle(&mut app, &ctx);
        press(&mut app, &ctx, "Ctrl+Shift+R");
        idle(&mut app, &ctx);
        type_text(&mut app, &ctx, "needle");
        wait_for_results(&mut app, &ctx, 2);
        let field = ctx
            .read_response(egui::Id::new("find_popup_replacement"))
            .expect("the replace field")
            .rect;
        click(&mut app, &ctx, field.center());
        type_text(&mut app, &ctx, "pin");
        // The first row is selected: Enter replaces it and keeps the popup open.
        press(&mut app, &ctx, "Enter");
        assert!(app.find_popup.open);
        assert_eq!(
            std::fs::read_to_string(root.join("A.txt")).unwrap(),
            "one pin\n"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("B.txt")).unwrap(),
            "two needle\n"
        );
        // The list is searched again and now has the row of B.txt only.
        wait_for_results(&mut app, &ctx, 1);
        for _ in 0..100 {
            idle(&mut app, &ctx);
            if app.state.search_result_count() == 1 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(results_of(&app)[0].path, PathBuf::from("B.txt"));
        let _ = std::fs::remove_dir_all(root);
    }
}

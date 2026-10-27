//! "Search Everywhere" popup: Classes, Files, Symbols and Actions behind one field.
//!
//! What each tab lists and how it ranks is decided in `archipel_ide_core::quick_nav`;
//! this file only renders it and turns a chosen row into a command.

use crate::icons::{self, Icon, TypeKind};
use crate::{IdeApp, theme};
use archipel_ide_core::quick_nav::{
    ActionCandidate, Hit, Ranked, SearchSources, SearchTab, SymbolEntry, file_name, search,
};
use archipel_ide_language::structure::SymbolKind;
use eframe::egui::{self, Rect, Sense, vec2};
use serde_json::{Value, json};
use std::time::Duration;

const MAX_ROWS: usize = 40;
/// Time spent indexing symbols per frame while the popup is open.
const INDEX_BUDGET: Duration = Duration::from_millis(15);

pub type Tab = SearchTab;

#[derive(Default)]
pub struct Palette {
    pub open: bool,
    query: String,
    selected: usize,
    just_opened: bool,
    tab: SearchTab,
    /// Query last sent to the language server.
    server_query: String,
    /// "Include non-project items": library symbols from the language server.
    include_non_project: bool,
    /// A keyboard move happened: scroll the selected row into view.
    scroll_to_selected: bool,
}

impl Palette {
    /// Selected tab, as reported by the automation `state` request.
    pub fn tab_name(&self) -> &'static str {
        self.tab.id()
    }

    pub fn show_tab(&mut self, tab: SearchTab) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
        self.just_opened = true;
        self.tab = tab;
        self.server_query.clear();
    }
}

/// Window-level actions (keymap `UiAction`s) that are not typed commands; they are
/// searchable alongside commands, with or without an open project.
const UI_ACTIONS: [(&str, &str); 3] = [
    ("NewProject", "New Project…"),
    ("OpenFile", "Open…"),
    ("Exit", "Exit"),
];

/// Id prefix of a [`UI_ACTIONS`] candidate (the rest is the keymap action id).
const UI_ACTION_PREFIX: &str = "ui:";

fn actions(app: &IdeApp, include_disabled: bool) -> Vec<ActionCandidate> {
    let ui_actions = UI_ACTIONS
        .iter()
        .map(|(action, label)| ActionCandidate {
            enabled: app.menu_action_enabled(action),
            shortcut: app.action_shortcut(action).unwrap_or_default(),
            id: format!("{UI_ACTION_PREFIX}{action}"),
            label: (*label).into(),
            category: "File".into(),
        })
        .filter(|candidate| include_disabled || candidate.enabled);
    app.commands
        .specs()
        .into_iter()
        .filter(IdeApp::parameterless)
        .filter(|spec| include_disabled || app.commands.is_enabled(&spec.id, &app.state))
        // Android actions only apply to Android projects; the SDK manager is global.
        .filter(|spec| {
            spec.category != "Android"
                || app.state.android.project.is_some()
                || app.commands.is_enabled(&spec.id, &app.state)
        })
        .map(|spec| ActionCandidate {
            enabled: app.commands.is_enabled(&spec.id, &app.state),
            shortcut: app.command_shortcut(&spec),
            id: spec.id,
            label: spec.label,
            category: spec.category,
        })
        .chain(ui_actions)
        .collect()
}

fn items(app: &mut IdeApp) -> Vec<Ranked<Hit>> {
    let query = app.palette.query.clone();
    let tab = app.palette.tab;
    if matches!(
        tab,
        SearchTab::All | SearchTab::Classes | SearchTab::Symbols
    ) {
        app.state.refresh_symbol_index(INDEX_BUDGET);
        if app.palette.server_query != query {
            app.palette.server_query = query.clone();
            app.state.request_server_symbols(&query);
        }
        app.state.poll_server_symbols();
    }
    if matches!(tab, SearchTab::All | SearchTab::Text) {
        app.state.request_text_search(&query);
        app.state.poll_text_search();
    }
    let actions = if matches!(tab, SearchTab::All | SearchTab::Actions) {
        // The All tab never lists disabled actions; the Actions tab has the checkbox.
        actions(
            app,
            tab == SearchTab::Actions && app.state.actions().read().include_disabled,
        )
    } else {
        Vec::new()
    };
    let manager = app.state.search_everywhere().unwrap_or_default();
    let navigation = manager.read();
    let sources = SearchSources {
        files: &app.state.files,
        recent: &app.state.recent_files,
        symbols: navigation.index.all().collect(),
        server_symbols: &navigation.server.results,
        actions: &actions,
        text: &navigation.text_results,
        include_non_project: app.palette.include_non_project,
    };
    let mut found = search(tab, &query, &sources);
    if tab == SearchTab::All && query.is_empty() {
        // Nothing typed: recent files only, like the reference IDE.
        found.retain(|hit| matches!(hit.item, Hit::File { .. }));
    }
    found.truncate(if tab == SearchTab::All { 24 } else { MAX_ROWS });
    found
}

fn symbol_icon(symbol: &SymbolEntry) -> Icon {
    match symbol.kind {
        SymbolKind::Class => Icon::JavaType(TypeKind::Class),
        SymbolKind::Interface => Icon::JavaType(TypeKind::Interface),
        SymbolKind::Enum => Icon::JavaType(TypeKind::Enum),
        _ => Icon::File,
    }
}

/// Paints `text` with the characters at `positions` emphasised; returns the end x.
fn highlighted_text(
    ui: &egui::Ui,
    row: Rect,
    x: f32,
    text: &str,
    positions: &[usize],
    color: egui::Color32,
) -> f32 {
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    let mut job = egui::text::LayoutJob::default();
    for (index, ch) in text.chars().enumerate() {
        let matched = positions.contains(&index);
        job.append(
            &ch.to_string(),
            0.0,
            egui::text::TextFormat {
                font_id: font.clone(),
                color: if matched { *theme::MODIFIED } else { color },
                background: if matched {
                    theme::SELECTION_DISABLED.gamma_multiply(0.6)
                } else {
                    egui::Color32::TRANSPARENT
                },
                ..Default::default()
            },
        );
    }
    let galley = ui.painter().layout_job(job);
    let width = galley.size().x;
    let pos = egui::pos2(x, row.center().y - galley.size().y / 2.0);
    ui.painter().with_clip_rect(row).galley(pos, galley, color);
    x + width
}

/// Moves the caret to a symbol or `file:line` and focuses the editor.
fn open_location(
    app: &mut IdeApp,
    ctx: &egui::Context,
    path: &std::path::Path,
    line: Option<usize>,
    right_split: bool,
) {
    let mut params = json!({"path": path});
    if let Some(line) = line {
        params["line"] = json!(line);
    }
    if right_split {
        app.dispatch("editor.openInRightSplit", json!({"path": path}));
    }
    app.dispatch("editor.openFile", params);
    ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
}

/// One result row. Returns the index when the row was clicked.
fn row(
    app: &mut IdeApp,
    ui: &mut egui::Ui,
    width: f32,
    index: usize,
    ranked: &Ranked<Hit>,
    scroll_selected: bool,
) -> Option<usize> {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 26.0), Sense::click());
    let selected = index == app.palette.selected;
    if selected && scroll_selected {
        response.scroll_to_me(None);
    }
    let row = rect.shrink2(vec2(6.0, 1.0));
    if selected {
        ui.painter().rect_filled(row, 4.0, theme::SELECTION);
    } else if response.hovered() {
        ui.painter().rect_filled(row, 4.0, theme::HOVER);
    }
    let icon_rect = Rect::from_min_size(
        egui::pos2(row.left() + 8.0, row.center().y - 8.0),
        vec2(16.0, 16.0),
    );
    let small = egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0);
    // (title, inline detail, right-aligned text, icon, title colour, highlighted positions)
    let mut positions = ranked.positions.clone();
    let (title, detail, trailing, icon, color) = match &ranked.item {
        Hit::Action(action) => (
            action.label.clone(),
            action.shortcut.clone(),
            action.category.clone(),
            Icon::Run,
            if action.enabled {
                *theme::TEXT_BRIGHT
            } else {
                *theme::TEXT_DISABLED
            },
        ),
        Hit::File { path, line } => (
            file_name(path),
            path.parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            line.map(|line| format!(":{line}")).unwrap_or_default(),
            app.file_icon(path),
            *theme::TEXT_BRIGHT,
        ),
        Hit::Symbol(symbol) => (
            symbol.name.clone(),
            symbol.container.clone().unwrap_or_default(),
            format!("{}:{}", file_name(&symbol.path), symbol.line + 1),
            symbol_icon(symbol),
            *theme::TEXT_BRIGHT,
        ),
        Hit::Text(found) => {
            positions = found.highlights.iter().flat_map(|r| r.clone()).collect();
            (
                found.preview.clone(),
                String::new(),
                format!("{} {}", file_name(&found.path), found.line),
                app.file_icon(&found.path),
                *theme::TEXT_BRIGHT,
            )
        }
    };
    let icon_color = if matches!(ranked.item, Hit::Action(_)) {
        *theme::TEXT_INFO
    } else {
        *icons::ICON
    };
    icons::paint(ui.painter(), icon_rect, icon, icon_color);
    // Trailing text first, so the title is clipped before it.
    let trailing_galley = ui.painter().layout_no_wrap(
        trailing,
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        *theme::TEXT_SECONDARY,
    );
    let trailing_x = row.right() - 10.0 - trailing_galley.size().x;
    ui.painter().galley(
        egui::pos2(trailing_x, row.center().y - trailing_galley.size().y / 2.0),
        trailing_galley,
        *theme::TEXT_SECONDARY,
    );
    let body = row.with_max_x(trailing_x - 8.0);
    let x = highlighted_text(ui, body, row.left() + 32.0, &title, &positions, color);
    crate::widgets::row_text(ui, body, x + 10.0, &detail, small, *theme::TEXT_SECONDARY);
    if response.hovered() && ui.input(|input| input.pointer.delta() != egui::Vec2::ZERO) {
        app.palette.selected = index;
    }
    response.clicked().then_some(index)
}

/// Selected item's path (or action id), the Assign Shortcut hint on the Actions tab.
fn footer(app: &IdeApp, ui: &mut egui::Ui, width: f32, selected: Option<&Ranked<Hit>>) {
    let (footer, _) = ui.allocate_exact_size(vec2(width, 28.0), Sense::hover());
    ui.painter().rect_filled(
        footer,
        egui::CornerRadius {
            nw: 0,
            ne: 0,
            sw: 10,
            se: 10,
        },
        theme::FRAME,
    );
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0);
    let left = match selected.map(|ranked| &ranked.item) {
        Some(Hit::File { path, .. }) => path.display().to_string(),
        Some(Hit::Symbol(symbol)) => symbol.path.display().to_string(),
        Some(Hit::Text(found)) => found.path.display().to_string(),
        Some(Hit::Action(action)) => action.id.clone(),
        None => {
            "↑↓ to navigate · Enter to open · Shift+Enter to open in right split · Esc to close"
                .to_owned()
        }
    };
    let right = if app.palette.tab == SearchTab::Actions {
        "Assign Shortcut  Alt+Enter"
    } else {
        ""
    };
    let galley =
        ui.painter()
            .layout_no_wrap(right.to_owned(), font.clone(), *theme::TEXT_SECONDARY);
    let right_width = galley.size().x;
    ui.painter().galley(
        footer.right_center() - vec2(14.0 + right_width, galley.size().y / 2.0),
        galley,
        *theme::TEXT_SECONDARY,
    );
    ui.painter()
        .with_clip_rect(footer.with_max_x(footer.right() - right_width - 28.0))
        .text(
            footer.left_center() + vec2(14.0, 0.0),
            egui::Align2::LEFT_CENTER,
            left,
            font,
            *theme::TEXT_INFO,
        );
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    if !app.palette.open {
        return;
    }
    for (modifiers, backward) in [
        (egui::Modifiers::NONE, false),
        (egui::Modifiers::SHIFT, true),
    ] {
        if ctx.input_mut(|input| input.consume_key(modifiers, egui::Key::Tab)) {
            app.palette.tab = app.palette.tab.step(backward);
            app.palette.selected = 0;
        }
    }
    let found = items(app);
    if app.state.text_search_running() {
        ctx.request_repaint_after(std::time::Duration::from_millis(30));
    }
    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        app.palette.open = false;
        return;
    }
    if !found.is_empty() {
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
            app.palette.selected = (app.palette.selected + 1) % found.len();
            app.palette.scroll_to_selected = true;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
            app.palette.selected = (app.palette.selected + found.len() - 1) % found.len();
            app.palette.scroll_to_selected = true;
        }
    }
    app.palette.selected = app.palette.selected.min(found.len().saturating_sub(1));
    // Shift+Enter opens the hit in the right split.
    let right_split =
        ctx.input_mut(|input| input.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter));
    let enter = ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        || right_split;
    let screen = ctx.content_rect();
    let width = 700.0_f32.min(screen.width() - 40.0);
    // Alt+Enter on an action: Assign Shortcut (opens the keymap page).
    let assign = app.palette.tab == SearchTab::Actions
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::Enter));
    let list_height = (screen.height() * 0.7 - 150.0).max(120.0);
    let mut chosen = enter.then_some(app.palette.selected);
    let mut close = false;
    egui::Area::new(egui::Id::new("search_everywhere"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(
            screen.center().x - width / 2.0,
            screen.top() + screen.height() * 0.12,
        ))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .corner_radius(10)
                .inner_margin(egui::Margin::same(0))
                .show(ui, |ui| {
                    ui.set_width(width);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    // Header tabs (Tab switches).
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        for tab in Tab::ALL {
                            let galley = ui.painter().layout_no_wrap(
                                tab.label().to_owned(),
                                egui::FontId::proportional(theme::UI_FONT_SIZE),
                                *theme::TEXT_BRIGHT,
                            );
                            let (rect, response) = ui.allocate_exact_size(
                                vec2(galley.size().x + 20.0, 32.0),
                                Sense::click(),
                            );
                            let pill = rect.shrink2(vec2(0.0, 5.0));
                            if tab == app.palette.tab {
                                ui.painter().rect_filled(pill, 4.0, theme::SELECTION);
                            } else if response.hovered() {
                                ui.painter().rect_filled(pill, 4.0, theme::HOVER);
                            }
                            let color = if tab == app.palette.tab {
                                *theme::TEXT_BRIGHT
                            } else {
                                *theme::TEXT_SECONDARY
                            };
                            ui.painter()
                                .galley(rect.center() - galley.size() / 2.0, galley, color);
                            if response.clicked() {
                                app.palette.tab = tab;
                                app.palette.selected = 0;
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(10.0);
                            match app.palette.tab {
                                SearchTab::Actions => {
                                    let actions = app.state.actions();
                                    let mut include = actions.read().include_disabled;
                                    if ui
                                        .checkbox(&mut include, "Include disabled actions")
                                        .changed()
                                    {
                                        actions.write().include_disabled = include;
                                    }
                                }
                                SearchTab::Files | SearchTab::Text => {}
                                _ => {
                                    ui.checkbox(
                                        &mut app.palette.include_non_project,
                                        "Include non-project items",
                                    );
                                }
                            }
                        });
                    });
                    ui.painter().hline(
                        ui.min_rect().x_range(),
                        ui.min_rect().bottom(),
                        egui::Stroke::new(1.0_f32, theme::POPUP_BORDER),
                    );
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        icons::show(ui, Icon::Search, 16.0, *theme::TEXT_SECONDARY);
                        let input = ui.add(
                            egui::TextEdit::singleline(&mut app.palette.query)
                                .frame(false)
                                .font(egui::FontId::proportional(theme::UI_FONT_SIZE + 2.0))
                                .desired_width(width - 60.0)
                                .margin(egui::Margin::symmetric(4, 10))
                                .hint_text(
                                    "Type to search classes, files, symbols, actions and text",
                                ),
                        );
                        if app.palette.just_opened || !input.has_focus() {
                            input.request_focus();
                            app.palette.just_opened = false;
                        }
                        if input.changed() {
                            app.palette.selected = 0;
                        }
                    });
                    ui.painter().hline(
                        ui.min_rect().x_range(),
                        ui.min_rect().bottom(),
                        egui::Stroke::new(1.0_f32, theme::POPUP_BORDER),
                    );
                    ui.add_space(4.0);
                    if found.is_empty() {
                        ui.add_space(12.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("Nothing found").color(theme::TEXT_SECONDARY),
                            );
                        });
                        ui.add_space(12.0);
                    }
                    let scroll_selected = std::mem::take(&mut app.palette.scroll_to_selected);
                    egui::ScrollArea::vertical()
                        .max_height(list_height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            for (index, ranked) in found.iter().enumerate() {
                                if let Some(click) =
                                    row(app, ui, width, index, ranked, scroll_selected)
                                {
                                    chosen = Some(click);
                                }
                            }
                        });
                    ui.add_space(4.0);
                    footer(app, ui, width, found.get(app.palette.selected));
                });
        });
    if ctx.input(|input| input.pointer.any_click()) && !ctx.is_pointer_over_area() {
        close = true;
    }
    if let Some(index) = chosen
        && let Some(ranked) = found.get(index)
    {
        match &ranked.item {
            Hit::Action(action) => {
                if action.enabled {
                    let id = action.id.clone();
                    app.palette.open = false;
                    match id.strip_prefix(UI_ACTION_PREFIX) {
                        Some(action) => app.run_menu_action(action, ctx),
                        None => app.dispatch(&id, Value::Null),
                    }
                }
            }
            Hit::File { path, line } => {
                app.palette.open = false;
                open_location(app, ctx, path, *line, right_split);
            }
            Hit::Symbol(symbol) => {
                app.palette.open = false;
                open_location(app, ctx, &symbol.path, Some(symbol.line + 1), right_split);
            }
            Hit::Text(found) => {
                app.palette.open = false;
                if right_split {
                    app.dispatch("editor.openInRightSplit", json!({"path": found.path}));
                }
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
    }
    if assign && let Some(Hit::Action(action)) = found.get(app.palette.selected).map(|r| &r.item) {
        app.keymap_search = Some(action.label.clone());
        app.palette.open = false;
        app.dispatch("settings.open", json!({"page": "preferences.keymap"}));
    }
    if close {
        app.palette.open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_action_offers_window_actions_and_hides_android_outside_android_projects() {
        let app = IdeApp::with_update_check(None, Vec::new(), false);
        let found = actions(&app, false);
        for label in ["New Project…", "Open…", "Exit"] {
            assert!(
                found.iter().any(|a| a.label == label && a.enabled),
                "{label}"
            );
        }
        assert!(found.iter().all(|a| a.id != "android.app.run"));
        assert!(found.iter().any(|a| a.id == "android.sdk.manager"));
    }
}

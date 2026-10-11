//! "Search Everywhere" popup: Classes, Files, Symbols and Actions behind one field.
//!
//! What each tab lists and how it ranks is decided in `jetthing_core::quick_nav`;
//! this file only renders it and turns a chosen row into a command.

use crate::IdeApp;
use crate::icons::{self, Icon, TypeKind};
use crate::widgets::{self, row_text};
use crate::{popup_row, popup_stack};
use archipel_kit::Theme;
use archipel_kit::components::{Checkbox, Kbd, Tab as TabButton, TabBar, TextField};
use archipel_kit::parts::{Surface, scroll_vertical};
use archipel_kit::tokens::size;
use eframe::egui::{self, Rect, Response};
use jetthing_core::quick_nav::{
    ActionCandidate, Hit, Ranked, SearchSources, SearchTab, SymbolEntry, file_name, search,
};
use jetthing_language::structure::SymbolKind;
use serde_json::{Value, json};
use std::time::Duration;

const MAX_ROWS: usize = 40;
/// Time spent indexing symbols per frame while the popup is open.
const INDEX_BUDGET: Duration = Duration::from_millis(15);

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
            tab == SearchTab::Actions && app.state.include_disabled_actions,
        )
    } else {
        Vec::new()
    };
    let navigation = &app.state.quick_nav;
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

/// Popup width at a 1280 px window, like the reference IDE's.
const PREFERRED_WIDTH: f32 = 670.0;
/// Height of the tab strip, search field and footer around the result list.
const CHROME_HEIGHT: f32 = 112.0;

/// A layout of `text` with the characters at `positions` emphasised (speed search).
fn highlighted_job(
    ctx: &egui::Context,
    text: &str,
    positions: &[usize],
    color: egui::Color32,
) -> egui::text::LayoutJob {
    let colors = Theme::colors(ctx);
    let font = egui::FontId::proportional(archipel_kit::tokens::typography::BODY);
    let mut job = egui::text::LayoutJob::default();
    for (index, ch) in text.chars().enumerate() {
        let matched = positions.contains(&index);
        job.append(
            &ch.to_string(),
            0.0,
            egui::text::TextFormat {
                font_id: font.clone(),
                color: if matched { colors.text_strong } else { color },
                background: if matched {
                    colors.warning.gamma_multiply(0.35)
                } else {
                    egui::Color32::TRANSPARENT
                },
                ..Default::default()
            },
        );
    }
    job
}

/// What a result row shows: title, inline detail, right-aligned text, icon, whether
/// it is dimmed, and the characters of the title that matched the query.
struct RowText {
    title: String,
    detail: String,
    trailing: String,
    icon: Icon,
    action: bool,
    dimmed: bool,
    positions: Vec<usize>,
}

fn row_text_of(app: &IdeApp, ranked: &Ranked<Hit>) -> RowText {
    let mut positions = ranked.positions.clone();
    let (title, detail, trailing, icon, action, dimmed) = match &ranked.item {
        Hit::Action(action) => (
            action.label.clone(),
            action.shortcut.clone(),
            action.category.clone(),
            Icon::Run,
            true,
            !action.enabled,
        ),
        Hit::File { path, line } => (
            file_name(path),
            path.parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            line.map(|line| format!(":{line}")).unwrap_or_default(),
            app.file_icon(path),
            false,
            false,
        ),
        Hit::Symbol(symbol) => (
            symbol.name.clone(),
            symbol.container.clone().unwrap_or_default(),
            format!("{}:{}", file_name(&symbol.path), symbol.line + 1),
            symbol_icon(symbol),
            false,
            false,
        ),
        Hit::Text(found) => {
            positions = found.highlights.iter().flat_map(|r| r.clone()).collect();
            (
                found.preview.clone(),
                String::new(),
                format!("{} {}", file_name(&found.path), found.line),
                app.file_icon(&found.path),
                false,
                false,
            )
        }
    };
    RowText {
        title,
        detail,
        trailing,
        icon,
        action,
        dimmed,
        positions,
    }
}

/// One result row: the kit's row pill and states, with the matched characters of the
/// title emphasised (the kit's `ListRow` cannot highlight a match yet; see the PR's
/// "Kit gaps"). A disabled action is dimmed and cannot be activated.
fn row(ui: &mut egui::Ui, selected: bool, text: &RowText) -> Response {
    let name = if text.detail.is_empty() {
        text.title.clone()
    } else {
        format!("{} {}", text.title, text.detail)
    };
    let (rect, response) = popup_row::row(ui, selected, !text.dimmed, &name);
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let colors = Theme::colors(ui.ctx());
    let inset = size::ROW_PILL_INSET + 6.0;
    let icon_color = if text.action {
        colors.accent
    } else {
        colors.icon
    };
    icons::paint(
        ui.painter(),
        widgets::icon_rect(rect.left() + inset, rect.center().y),
        text.icon,
        icon_color,
    );
    let small = egui::FontId::proportional(archipel_kit::tokens::typography::SMALL + 1.0);
    // Trailing text first, so the title is clipped before it.
    let trailing_x = rect.right() - inset - 4.0;
    let trailing_left = if text.trailing.is_empty() {
        trailing_x
    } else {
        let galley = ui.painter().layout_no_wrap(
            text.trailing.clone(),
            small.clone(),
            colors.text_secondary,
        );
        let left = trailing_x - galley.size().x;
        ui.painter().galley(
            egui::pos2(left, rect.center().y - galley.size().y / 2.0),
            galley,
            colors.text_secondary,
        );
        left
    };
    let body = rect.with_max_x(trailing_left - 8.0);
    let color = if text.dimmed {
        colors.text_disabled
    } else if selected {
        colors.text_strong
    } else {
        colors.text
    };
    let job = highlighted_job(ui.ctx(), &text.title, &text.positions, color);
    let galley = ui.painter().layout_job(job);
    let x = rect.left() + inset + size::ICON + size::ICON_TEXT_GAP;
    let end = x + galley.size().x;
    ui.painter().with_clip_rect(body).galley(
        egui::pos2(x, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
    row_text(
        ui,
        body,
        end + size::SECONDARY_GAP,
        &text.detail,
        small,
        colors.text_secondary,
    );
    response
}

/// Footer text for the selected hit: its path, or its id for an action.
fn footer_text(selected: Option<&Ranked<Hit>>) -> String {
    match selected.map(|ranked| &ranked.item) {
        Some(Hit::File { path, .. }) => path.display().to_string(),
        Some(Hit::Symbol(symbol)) => symbol.path.display().to_string(),
        Some(Hit::Text(found)) => found.path.display().to_string(),
        Some(Hit::Action(action)) => action.label.clone(),
        None => {
            "↑↓ to navigate · Enter to open · Shift+Enter to open in right split · Esc to close"
                .to_owned()
        }
    }
}

/// Selected item's path, the Assign Shortcut hint on the Actions tab.
fn footer(app: &IdeApp, ui: &mut egui::Ui, selected: Option<&Ranked<Hit>>) {
    let colors = Theme::colors(ui.ctx());
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(10.0);
            if app.palette.tab == SearchTab::Actions {
                ui.add(Kbd::parse("Alt+Enter"));
                ui.label(egui::RichText::new("Assign Shortcut").color(colors.text_secondary));
            } else if selected.is_some() {
                ui.add(Kbd::parse("Shift+Enter"));
                ui.label(egui::RichText::new("Open in Right Split").color(colors.text_secondary));
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(footer_text(selected)).color(colors.text_secondary),
                    )
                    .truncate(),
                );
            });
        });
    });
    ui.add_space(4.0);
}

/// The popup's content, drawn in the popup frame (or any panel, in tests): tabs, search
/// field, result list and footer. Returns the clicked row.
fn body(
    app: &mut IdeApp,
    ui: &mut egui::Ui,
    found: &[Ranked<Hit>],
    list_height: f32,
) -> Option<usize> {
    let ctx = ui.ctx().clone();
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let tabs: Vec<TabButton> = SearchTab::ALL
            .iter()
            .map(|tab| TabButton::new(tab.label()))
            .collect();
        let current = SearchTab::ALL
            .iter()
            .position(|tab| *tab == app.palette.tab)
            .unwrap_or(0);
        if let Some(index) = TabBar::new(&tabs, current).show(ui).selected {
            app.palette.tab = SearchTab::ALL[index];
            app.palette.selected = 0;
            app.palette.just_opened = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(10.0);
            match app.palette.tab {
                SearchTab::Actions => {
                    let option = ui.add(Checkbox::new(
                        &mut app.state.include_disabled_actions,
                        "Include disabled actions",
                    ));
                    app.palette.just_opened |= option.changed();
                }
                SearchTab::Files | SearchTab::Text => {}
                _ => {
                    let option = ui.add(Checkbox::new(
                        &mut app.palette.include_non_project,
                        "Include non-project items",
                    ));
                    app.palette.just_opened |= option.changed();
                }
            }
        });
    });
    ui.separator();
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let input = ui.add(
            TextField::search(&mut app.palette.query)
                .placeholder("Type to search classes, files, symbols, actions and text")
                .width(ui.available_width() - 8.0),
        );
        // The kit's field names itself only through a visible label (a kit gap): name it.
        input.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Search everywhere")
        });
        // The field keeps typing focus: on open, after a tab or option click, and whenever
        // nothing else has it.
        if app.palette.just_opened || ctx.memory(|memory| memory.focused().is_none()) {
            input.request_focus();
            app.palette.just_opened = false;
        }
        if input.changed() {
            app.palette.selected = 0;
        }
    });
    ui.add_space(4.0);
    let mut clicked = None;
    if found.is_empty() {
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("Nothing found").color(Theme::colors(&ctx).text_secondary),
            );
        });
        ui.add_space(12.0);
    }
    let scroll_id = egui::Id::new("search_everywhere_list");
    let row_height = size::ROW_HEIGHT;
    let offset: f32 = ctx.data(|d| d.get_temp(scroll_id)).unwrap_or(0.0);
    let mut area = scroll_vertical()
        .id_salt(scroll_id)
        .max_height(list_height)
        .auto_shrink([false, false]);
    if std::mem::take(&mut app.palette.scroll_to_selected) {
        let top = app.palette.selected as f32 * row_height;
        if top < offset {
            area = area.vertical_scroll_offset(top);
        } else if top + row_height > offset + list_height {
            area = area.vertical_scroll_offset(top + row_height - list_height);
        }
    }
    let rows: Vec<RowText> = found
        .iter()
        .map(|ranked| row_text_of(app, ranked))
        .collect();
    let selected = app.palette.selected;
    let output = area.show_rows(ui, row_height, rows.len(), |ui, range| {
        ui.spacing_mut().item_spacing.y = 0.0;
        for index in range {
            if row(ui, index == selected, &rows[index]).clicked() {
                clicked = Some(index);
            }
        }
    });
    ctx.data_mut(|d| d.insert_temp(scroll_id, output.state.offset.y));
    ui.separator();
    footer(app, ui, found.get(app.palette.selected));
    clicked
}

/// Moves the selection by `delta` rows; wraps around at the ends only for single steps.
fn move_selection(app: &mut IdeApp, delta: isize, len: usize) {
    let last = len as isize - 1;
    let current = app.palette.selected as isize;
    let next = if delta.abs() == 1 {
        (current + delta).rem_euclid(len as isize)
    } else {
        (current + delta).clamp(0, last)
    };
    app.palette.selected = next as usize;
    app.palette.scroll_to_selected = true;
}

/// Rows a PageUp or PageDown moves by.
fn page_rows(list_height: f32) -> isize {
    ((list_height / size::ROW_HEIGHT) as isize - 1).max(1)
}

/// Popup size at the current window size: about 670×400 at 1280×800.
fn popup_size(screen: Rect) -> (f32, f32) {
    let width = PREFERRED_WIDTH.min(screen.width() - 40.0);
    let list = ((screen.height() * 0.5).clamp(300.0, 560.0) - CHROME_HEIGHT)
        .min(screen.height() * 0.7 - CHROME_HEIGHT)
        .max(96.0);
    (width, list)
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    if !app.palette.open {
        return;
    }
    // Escape is the popup stack's (`IdeApp::close_popup`); the rest is handled here.
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
    if app.state.quick_nav.text.is_running() {
        ctx.request_repaint_after(std::time::Duration::from_millis(30));
    }
    let screen = ctx.content_rect();
    let (width, list_height) = popup_size(screen);
    if !found.is_empty() {
        let page = page_rows(list_height);
        for (key, delta) in [
            (egui::Key::ArrowDown, 1),
            (egui::Key::ArrowUp, -1),
            (egui::Key::PageDown, page),
            (egui::Key::PageUp, -page),
        ] {
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key)) {
                move_selection(app, delta, found.len());
            }
        }
    }
    app.palette.selected = app.palette.selected.min(found.len().saturating_sub(1));
    // Shift+Enter opens the hit in the right split.
    let right_split =
        ctx.input_mut(|input| input.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter));
    let enter = ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        || right_split;
    // Alt+Enter on an action: Assign Shortcut (opens the keymap page).
    let assign = app.palette.tab == SearchTab::Actions
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::Enter));
    let mut chosen = enter.then_some(app.palette.selected);
    let shown = egui::Area::new(egui::Id::new("search_everywhere"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(
            screen.center().x - width / 2.0,
            screen.top() + screen.height() * 0.12,
        ))
        .show(ctx, |ui| {
            Surface::Popup
                .frame(ctx)
                .inner_margin(egui::Margin::ZERO)
                .show(ui, |ui| {
                    ui.set_width(width);
                    if let Some(click) = body(app, ui, &found, list_height) {
                        chosen = Some(click);
                    }
                })
        });
    let close = popup_stack::pressed_outside(ctx, shown.response.rect, None, None)
        || popup_stack::window_focus_lost(ctx);
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
pub(crate) fn body_for_tests(app: &mut IdeApp, ui: &mut egui::Ui) {
    let found = items(app);
    body(app, ui, &found, 240.0);
}

#[cfg(test)]
pub(crate) fn result_row_for_tests(ui: &mut egui::Ui, enabled: bool, title: &str) -> Response {
    let text = RowText {
        title: title.to_owned(),
        detail: String::new(),
        trailing: String::new(),
        icon: Icon::File,
        action: false,
        dimmed: !enabled,
        positions: vec![0],
    };
    row(ui, false, &text)
}

#[cfg(test)]
pub(crate) fn items_for_tests(app: &mut IdeApp) -> Vec<Ranked<Hit>> {
    items(app)
}

#[cfg(test)]
pub(crate) fn set_query_for_tests(app: &mut IdeApp, query: &str) {
    app.palette.query = query.to_owned();
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

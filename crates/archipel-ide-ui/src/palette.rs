//! "Search Everywhere" popup: Classes, Files, Symbols and Actions behind one field.
//!
//! What each tab lists and how it ranks is decided in `archipel_ide_core::quick_nav`;
//! this file only renders it and turns a chosen row into a command.

use crate::icons::{self, Icon, TypeKind};
use crate::{IdeApp, theme};
use archipel_ide_actions::ActionPlace;
use archipel_ide_core::quick_nav::{ActionCandidate, Hit, Ranked, SearchTab, SymbolEntry};
use archipel_ide_language::structure::SymbolKind;
use archipel_ide_view::popups::search::{self as model, ActionExtra, SearchRow};
use eframe::egui::{self, Rect, Sense, vec2};
use serde_json::json;

/// Search Everywhere geometry (UI-POP-010/011).
const POPUP_WIDTH: f32 = 705.0;
const POPUP_HEIGHT: f32 = 700.0;
const ROW_HEIGHT: f32 = 24.0;
const HEADER_HEIGHT: f32 = 41.0;
const SEARCH_ROW_HEIGHT: f32 = 40.0;
const FOOTER_HEIGHT: f32 = 29.0;
const TAB_UNDERLINE: f32 = 3.0;
/// Header, search row, divider, 4 px above and below the list, footer.
const CHROME_HEIGHT: f32 = HEADER_HEIGHT + SEARCH_ROW_HEIGHT + 1.0 + 8.0 + FOOTER_HEIGHT;

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
const UI_ACTIONS: [ActionExtra; 3] = [
    ActionExtra {
        id: "NewProject",
        label: "New Project…",
        category: "File",
    },
    ActionExtra {
        id: "OpenFile",
        label: "Open…",
        category: "File",
    },
    ActionExtra {
        id: "Exit",
        label: "Exit",
        category: "File",
    },
];

/// Id prefix of a [`UI_ACTIONS`] candidate (the rest is the keymap action id).
const UI_ACTION_PREFIX: &str = "ui:";

fn actions(app: &IdeApp, include_disabled: bool) -> Vec<ActionCandidate> {
    app.with_view(|cx| {
        model::action_candidates(
            cx,
            &UI_ACTIONS,
            UI_ACTION_PREFIX,
            &[],
            include_disabled,
            // A command no keymap action maps to shows its own default.
            &|spec| crate::keys::display(spec.shortcut.as_deref().unwrap_or_default()),
        )
    })
}

fn items(app: &mut IdeApp) -> Vec<Ranked<Hit>> {
    let query = app.palette.query.clone();
    let tab = app.palette.tab;
    let mut server_query = std::mem::take(&mut app.palette.server_query);
    model::prepare(&mut app.state, tab, &query, &mut server_query);
    app.palette.server_query = server_query;
    let candidates = if model::needs_actions(tab) {
        // The All tab never lists disabled actions; the Actions tab has the checkbox.
        actions(
            app,
            tab == SearchTab::Actions && app.state.actions().read().include_disabled,
        )
    } else {
        Vec::new()
    };
    let include_non_project = app.palette.include_non_project;
    app.with_view(|cx| model::hits(cx, tab, &query, include_non_project, &candidates))
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
                color: if matched {
                    theme::search_match_text()
                } else {
                    color
                },
                background: if matched {
                    *theme::SEARCH_MATCH
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

/// The icon of a hit: files by their content, symbols by their kind.
fn hit_icon(app: &IdeApp, hit: &Hit) -> Icon {
    match hit {
        Hit::Action(_) => Icon::Run,
        Hit::File { path, .. } => app.file_icon(path),
        Hit::Symbol(symbol) => symbol_icon(symbol),
        Hit::Text(found) => app.file_icon(&found.path),
    }
}

/// One result row. Returns the index when the row was clicked.
fn row(
    app: &mut IdeApp,
    ui: &mut egui::Ui,
    width: f32,
    index: usize,
    model: &SearchRow,
    scroll_selected: bool,
) -> Option<usize> {
    if let Some(group) = model.group {
        // A separator row above the first row of a group (UI-POP-012).
        let (rect, _) = ui.allocate_exact_size(vec2(width, ROW_HEIGHT), Sense::hover());
        crate::widgets::row_text(
            ui,
            rect,
            rect.left() + 16.0,
            group,
            egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0),
            *theme::TEXT_SECONDARY,
        );
    }
    let (rect, response) = ui.allocate_exact_size(vec2(width, ROW_HEIGHT), Sense::click());
    let selected = index == app.palette.selected;
    if selected && scroll_selected {
        response.scroll_to_me(None);
    }
    // Selection 8 px from the popup border (UI-POP-002).
    let row = rect.shrink2(vec2(8.0, 0.0));
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
    let icon = hit_icon(app, &model.hit);
    let color = if model.enabled {
        *theme::TEXT_BRIGHT
    } else {
        *theme::TEXT_DISABLED
    };
    let icon_color = if matches!(model.hit, Hit::Action(_)) {
        *theme::TEXT_INFO
    } else {
        *icons::ICON
    };
    icons::paint(ui.painter(), icon_rect, icon, icon_color);
    // Trailing text first, so the title is clipped before it.
    let trailing_galley = ui.painter().layout_no_wrap(
        model.trailing.clone(),
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
    let x = highlighted_text(
        ui,
        body,
        row.left() + 32.0,
        &model.title,
        &model.positions,
        color,
    );
    crate::widgets::row_text(
        ui,
        body,
        x + 10.0,
        &model.inline,
        small,
        *theme::TEXT_SECONDARY,
    );
    if response.hovered() && ui.input(|input| input.pointer.delta() != egui::Vec2::ZERO) {
        app.palette.selected = index;
    }
    response.clicked().then_some(index)
}

/// Selected item's path (or action id), the Assign Shortcut hint on the Actions tab.
fn footer(ui: &mut egui::Ui, width: f32, footer: &model::Footer) {
    let (band, _) = ui.allocate_exact_size(vec2(width, FOOTER_HEIGHT), Sense::hover());
    ui.painter().rect_filled(
        band,
        egui::CornerRadius {
            nw: 0,
            ne: 0,
            sw: 10,
            se: 10,
        },
        theme::BAND,
    );
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0);
    // The link at the right: the alternative way to open the selected item (UI-POP-011).
    let galley = ui
        .painter()
        .layout_no_wrap(footer.link.to_owned(), font.clone(), *theme::LINK);
    let right_width = galley.size().x;
    ui.painter().galley(
        band.right_center() - vec2(14.0 + right_width, galley.size().y / 2.0),
        galley,
        *theme::LINK,
    );
    ui.painter()
        .with_clip_rect(band.with_max_x(band.right() - right_width - 28.0))
        .text(
            band.left_center() + vec2(14.0, 0.0),
            egui::Align2::LEFT_CENTER,
            &footer.left,
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
    let rows = model::rows(app.palette.tab, !app.palette.query.is_empty(), &found);
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
    let width = POPUP_WIDTH.min(screen.width() - 40.0);
    // Alt+Enter on an action: Assign Shortcut (opens the keymap page).
    let assign = app.palette.tab == SearchTab::Actions
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::Enter));
    // 700 px popup (UI-POP-010) minus header, search row, dividers, paddings and footer.
    let list_height = (POPUP_HEIGHT.min(screen.height() * 0.71) - CHROME_HEIGHT).max(120.0);
    let mut chosen = enter.then_some(app.palette.selected);
    let mut close = false;
    egui::Area::new(egui::Id::new("search_everywhere"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(
            screen.center().x - width / 2.0,
            screen.top() + (screen.height() * 0.19).round(),
        ))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .corner_radius(10)
                .inner_margin(egui::Margin::same(0))
                .show(ui, |ui| {
                    ui.set_width(width);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    // Header band with the tabs (Tab switches), 41 px (UI-POP-011).
                    let (band, _) =
                        ui.allocate_exact_size(vec2(width, HEADER_HEIGHT), Sense::hover());
                    ui.painter().rect_filled(
                        band,
                        egui::CornerRadius {
                            nw: 10,
                            ne: 10,
                            sw: 0,
                            se: 0,
                        },
                        theme::BAND,
                    );
                    ui.scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(band)
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                        |ui| {
                            ui.add_space(8.0);
                            for tab in model::tabs() {
                                let galley = ui.painter().layout_no_wrap(
                                    tab.label().to_owned(),
                                    egui::FontId::proportional(theme::UI_FONT_SIZE),
                                    *theme::TEXT_BRIGHT,
                                );
                                let (rect, response) = ui.allocate_exact_size(
                                    vec2(galley.size().x + 24.0, HEADER_HEIGHT),
                                    Sense::click(),
                                );
                                let selected = tab == app.palette.tab;
                                if !selected && response.hovered() {
                                    ui.painter().rect_filled(
                                        rect.shrink2(vec2(0.0, 6.0)),
                                        4.0,
                                        theme::HOVER,
                                    );
                                }
                                let color = if selected {
                                    *theme::TEXT_BRIGHT
                                } else {
                                    *theme::TEXT_SECONDARY
                                };
                                ui.painter().galley(
                                    rect.center() - galley.size() / 2.0,
                                    galley,
                                    color,
                                );
                                if selected {
                                    // The selected tab: a 3 px underline, not a pill.
                                    ui.painter().rect_filled(
                                        Rect::from_min_max(
                                            egui::pos2(rect.left(), rect.bottom() - TAB_UNDERLINE),
                                            rect.right_bottom(),
                                        ),
                                        0.0,
                                        theme::ACCENT,
                                    );
                                }
                                if response.clicked() {
                                    app.palette.tab = tab;
                                    app.palette.selected = 0;
                                }
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add_space(10.0);
                                    let actions = app.state.actions();
                                    let include_disabled = actions.read().include_disabled;
                                    if let Some(checkbox) = model::checkbox(
                                        app.palette.tab,
                                        app.palette.include_non_project,
                                        include_disabled,
                                    ) {
                                        let mut checked = checkbox.checked;
                                        if ui.checkbox(&mut checked, checkbox.label).changed() {
                                            if app.palette.tab == SearchTab::Actions {
                                                actions.write().include_disabled = checked;
                                            } else {
                                                app.palette.include_non_project = checked;
                                            }
                                        }
                                    }
                                },
                            );
                        },
                    );
                    let (search_row, _) =
                        ui.allocate_exact_size(vec2(width, SEARCH_ROW_HEIGHT), Sense::hover());
                    ui.scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(search_row)
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                        |ui| {
                            ui.add_space(19.0);
                            icons::show(ui, Icon::Search, 16.0, *theme::TEXT_SECONDARY);
                            let input = ui.add(
                                egui::TextEdit::singleline(&mut app.palette.query)
                                    .frame(false)
                                    .font(egui::FontId::proportional(theme::UI_FONT_SIZE + 2.0))
                                    .desired_width(width - 60.0)
                                    .margin(egui::Margin::symmetric(4, 0))
                                    .hint_text(model::PLACEHOLDER),
                            );
                            if app.palette.just_opened || !input.has_focus() {
                                input.request_focus();
                                app.palette.just_opened = false;
                            }
                            if input.changed() {
                                app.palette.selected = 0;
                            }
                            // The hint at the right (UI-POP-011).
                            if app.palette.query.is_empty() {
                                ui.painter().text(
                                    search_row.right_center() - vec2(16.0, 0.0),
                                    egui::Align2::RIGHT_CENTER,
                                    model::HINT,
                                    egui::FontId::proportional(theme::UI_FONT_SIZE),
                                    *theme::TEXT_INFO,
                                );
                            }
                        },
                    );
                    ui.painter().hline(
                        search_row.x_range(),
                        search_row.bottom(),
                        egui::Stroke::new(1.0_f32, theme::BAND),
                    );
                    ui.add_space(1.0);
                    ui.add_space(4.0);
                    if found.is_empty() {
                        ui.add_space(12.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new(model::empty_text(false))
                                    .color(theme::TEXT_SECONDARY),
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
                            for (index, shown) in rows.iter().enumerate() {
                                if let Some(click) =
                                    row(app, ui, width, index, shown, scroll_selected)
                                {
                                    chosen = Some(click);
                                }
                            }
                        });
                    ui.add_space(4.0);
                    let selected = rows.get(app.palette.selected);
                    footer(ui, width, &model::footer(app.palette.tab, selected));
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
                    let id = id.strip_prefix(UI_ACTION_PREFIX).unwrap_or(&id);
                    app.run_action(id, ActionPlace::FindAction, ctx);
                }
            }
            hit => {
                app.palette.open = false;
                for (command, params) in hit.open_commands(right_split) {
                    app.dispatch(command, params);
                }
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

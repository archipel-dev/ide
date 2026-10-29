//! Navigation UI: keyboard shortcuts, the chooser / Show Usages popup, and
//! the usages view of the Find tool window. Requests and history live in
//! `archipel_ide_core::quick_nav`.

use crate::icons::{self, Icon};
use crate::widgets::{self, list_row, row_text};
use crate::{IdeApp, theme};
use archipel_ide_core::{NavigationItem, ToolWindow};
use archipel_ide_view::popups::chooser::{self, ChoiceRow};
use eframe::egui::{self, Key, Modifiers, Rect, Sense, Stroke, vec2};
use serde_json::{Value, json};

/// Link color used for Ctrl+hover (`CTRL_CLICKABLE` / hyperlink blue).
pub const LINK: egui::Color32 = egui::Color32::from_rgb(0x54, 0x8A, 0xF7);
const POPUP_ROWS: usize = 12;

fn selected_id() -> egui::Id {
    egui::Id::new("navigation_choice_selected")
}

/// Navigation shortcuts (IntelliJ default keymaps) and the chooser's keys.
pub fn handle_shortcuts(app: &mut IdeApp, ctx: &egui::Context) {
    let consume = |modifiers: Modifiers, key: Key| {
        ctx.input_mut(|input| {
            let before = input.events.len();
            input.events.retain(|event| {
                !matches!(
                    event,
                    egui::Event::Key { key: k, modifiers: m, pressed: true, .. }
                        if *k == key && m.matches_exact(modifiers)
                )
            });
            input.events.len() != before
        })
    };
    if app.state.navigation().choices.is_some() {
        choices_keys(app, ctx, &consume);
    }
    // Go to Declaration/Type/Implementation, Find/Show Usages and keyboard
    // Back/Forward are keymap actions (`ACTION_COMMANDS`); only the mouse side
    // buttons are handled here.
    let (back, forward) = ctx.input(|input| {
        (
            input.pointer.button_pressed(egui::PointerButton::Extra1),
            input.pointer.button_pressed(egui::PointerButton::Extra2),
        )
    });
    if back && app.state.navigation().can_go_back() {
        app.dispatch("navigate.back", Value::Null);
        ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
    }
    if forward && app.state.navigation().can_go_forward() {
        app.dispatch("navigate.forward", Value::Null);
        ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
    }
}

fn choices_keys(app: &mut IdeApp, ctx: &egui::Context, consume: &dyn Fn(Modifiers, Key) -> bool) {
    let count = app
        .state
        .navigation
        .choices
        .as_ref()
        .map_or(0, |list| list.items.len());
    let mut selected: usize = ctx.data(|data| data.get_temp(selected_id()).unwrap_or(0));
    if consume(Modifiers::NONE, Key::ArrowDown) && count > 0 {
        selected = (selected + 1) % count;
    }
    if consume(Modifiers::NONE, Key::ArrowUp) && count > 0 {
        selected = (selected + count - 1) % count;
    }
    ctx.data_mut(|data| data.insert_temp(selected_id(), selected));
    if consume(Modifiers::NONE, Key::Escape) {
        app.state.navigation_mut().choices = None;
        return;
    }
    if consume(Modifiers::NONE, Key::Enter) {
        open_choice(app, ctx, selected);
    }
}

fn open_choice(app: &mut IdeApp, ctx: &egui::Context, index: usize) {
    let Some(item) = app
        .state
        .navigation
        .choices
        .as_ref()
        .and_then(|list| list.items.get(index))
        .cloned()
    else {
        return;
    };
    app.state.navigation_mut().choices = None;
    goto(app, ctx, &item);
}

fn goto(app: &mut IdeApp, ctx: &egui::Context, item: &NavigationItem) {
    app.dispatch(
        "navigate.goto",
        json!({"path": item.path, "line": item.line, "column": item.column}),
    );
    ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
}

/// The chooser (several declarations or implementations) and Show Usages popup,
/// anchored below the caret.
pub fn choices_popup(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(list) = app.state.navigation().choices.clone() else {
        ctx.data_mut(|data| data.remove::<usize>(selected_id()));
        return;
    };
    let selected: usize = ctx.data(|data| data.get_temp(selected_id()).unwrap_or(0));
    let anchor = app
        .caret_screen_pos
        .unwrap_or_else(|| ctx.content_rect().center());
    let mut open = None;
    let mut close = false;
    let response = egui::Area::new(egui::Id::new("navigation_choices"))
        .order(egui::Order::Foreground)
        .fixed_pos(anchor + vec2(-20.0, 4.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    ui.set_width(620.0);
                    ui.horizontal(|ui| {
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(&list.title)
                                .font(theme::semibold(theme::UI_FONT_SIZE))
                                .color(theme::TEXT_BRIGHT),
                        );
                        ui.label(
                            egui::RichText::new(chooser::header(&list).count)
                                .color(theme::TEXT_INFO),
                        );
                    });
                    ui.add_space(2.0);
                    egui::ScrollArea::vertical()
                        .max_height(POPUP_ROWS as f32 * 22.0)
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            for (index, item) in list.items.iter().enumerate() {
                                let (rect, response) = ui.allocate_exact_size(
                                    vec2(ui.available_width(), 22.0),
                                    Sense::click(),
                                );
                                if index == selected {
                                    ui.painter().rect_filled(
                                        rect,
                                        4.0,
                                        theme::COMPLETION_SELECTION,
                                    );
                                    ui.scroll_to_rect(rect, None);
                                } else if response.hovered() {
                                    ui.painter().rect_filled(rect, 4.0, theme::HOVER);
                                }
                                item_row(ui, rect, item, true);
                                if response.clicked() {
                                    open = Some(index);
                                }
                            }
                        });
                });
        })
        .response;
    if ctx.input(|input| input.pointer.any_pressed())
        && !response.contains_pointer()
        && open.is_none()
    {
        close = true;
    }
    if let Some(index) = open {
        open_choice(app, ctx, index);
    } else if close {
        app.state.navigation_mut().choices = None;
    }
}

/// `icon  file:line  code` row shared by the popup and the usages view.
fn item_row(ui: &egui::Ui, rect: Rect, item: &NavigationItem, with_file: bool) {
    let name = crate::IdeApp::file_name(&item.path);
    let ui_font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    let mono = egui::FontId::monospace(theme::EDITOR_FONT_SIZE);
    let mut x = rect.left() + 8.0;
    if with_file {
        icons::paint(
            ui.painter(),
            Rect::from_min_size(egui::pos2(x, rect.center().y - 8.0), vec2(16.0, 16.0)),
            Icon::for_file(&name),
            icons::ICON,
        );
        x += 22.0;
    }
    let ChoiceRow {
        before,
        symbol,
        after,
        ..
    } = chooser::row(item);
    let line_label = format!("{}", item.line + 1);
    let gutter =
        ui.painter()
            .layout_no_wrap(line_label.clone(), ui_font.clone(), *theme::TEXT_INFO);
    if !with_file {
        ui.painter().galley(
            egui::pos2(
                x + 28.0 - gutter.size().x,
                rect.center().y - gutter.size().y / 2.0,
            ),
            gutter,
            *theme::TEXT_INFO,
        );
        x += 40.0;
    }
    x = row_text(ui, rect, x, &before, mono.clone(), theme::editor::TEXT);
    let symbol_start = x;
    x = row_text(ui, rect, x, &symbol, mono.clone(), theme::TEXT_BRIGHT);
    if !symbol.is_empty() {
        ui.painter().rect_filled(
            Rect::from_min_max(
                egui::pos2(symbol_start, rect.top() + 3.0),
                egui::pos2(x, rect.bottom() - 3.0),
            ),
            2.0,
            theme::editor::SEARCH_RESULT.gamma_multiply(0.6),
        );
        row_text(
            ui,
            rect,
            symbol_start,
            &symbol,
            mono.clone(),
            theme::TEXT_BRIGHT,
        );
    }
    x = row_text(ui, rect, x, &after, mono, theme::editor::TEXT);
    if with_file {
        let location = chooser::row(item).location;
        let galley = ui
            .painter()
            .layout_no_wrap(location, ui_font, *theme::TEXT_INFO);
        let pos = egui::pos2(
            (rect.right() - galley.size().x - 10.0).max(x + 16.0),
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter()
            .with_clip_rect(rect)
            .galley(pos, galley, *theme::TEXT_INFO);
    }
}

/// Header of the Find tool window: `Find in Files` and, after Find Usages,
/// a `Usages of x` tab. Returns `true` when the usages tab is selected.
pub fn find_header(app: &mut IdeApp, ui: &mut egui::Ui) -> bool {
    let (header, _) = ui.allocate_exact_size(
        vec2(
            ui.available_width(),
            widgets::tool_window_header_height(ui.ctx()),
        ),
        Sense::hover(),
    );
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    let mut x = header.left() + 12.0;
    x = row_text(
        ui,
        header,
        x,
        "Find",
        theme::semibold(theme::UI_FONT_SIZE),
        theme::TEXT_BRIGHT,
    ) + 14.0;
    let usages_title = app
        .state
        .navigation
        .usages
        .as_ref()
        .map(|usages| usages.title.clone());
    let mut tabs = vec![(false, "Find in Files".to_owned())];
    if let Some(title) = usages_title {
        tabs.push((true, title));
    }
    let show_tabs = tabs.len() > 1;
    for (usages, label) in tabs {
        if !show_tabs {
            row_text(ui, header, x, &label, font.clone(), theme::TEXT_SECONDARY);
            break;
        }
        let width = ui
            .painter()
            .layout_no_wrap(label.clone(), font.clone(), *theme::TEXT)
            .size()
            .x
            + 16.0;
        let rect = Rect::from_min_size(egui::pos2(x, header.top()), vec2(width, header.height()));
        let response = ui.interact(rect, egui::Id::new(("find_tab", usages)), Sense::click());
        if response.hovered() {
            ui.painter()
                .rect_filled(rect.shrink2(vec2(0.0, 4.0)), 4.0, theme::HOVER);
        }
        row_text(ui, rect, x + 8.0, &label, font.clone(), theme::TEXT_BRIGHT);
        if app.state.navigation().usages_selected == usages {
            ui.painter().line_segment(
                [
                    egui::pos2(rect.left() + 2.0, rect.bottom() - 1.0),
                    egui::pos2(rect.right() - 2.0, rect.bottom() - 1.0),
                ],
                Stroke::new(2.0_f32, theme::ACCENT),
            );
        }
        if response.clicked() {
            app.state.navigation_mut().usages_selected = usages;
        }
        x = rect.right() + 4.0;
    }
    let mut right = header.right() - 12.0;
    let mut button = |ui: &mut egui::Ui, icon: Icon, color: egui::Color32, tooltip: &str| {
        let rect = Rect::from_min_size(
            egui::pos2(right - 22.0, header.center().y - 11.0),
            vec2(22.0, 22.0),
        );
        right -= 24.0;
        let response = ui.interact(
            rect,
            egui::Id::new(("find_header_button", tooltip)),
            Sense::click(),
        );
        if response.hovered() {
            ui.painter().rect_filled(rect, 4.0, theme::HOVER);
        }
        icons::paint(ui.painter(), rect.shrink(3.0), icon, color);
        response.on_hover_text(tooltip).clicked()
    };
    if button(ui, Icon::Minimize, *icons::ICON, "Hide") {
        app.toggle_tool_window(ToolWindow::Find);
    }
    let usages_selected =
        app.state.navigation().usages_selected && app.state.navigation().usages.is_some();
    if usages_selected {
        if button(ui, Icon::Close, *icons::ICON, "Close Usages Tab") {
            app.state.navigation_mut().usages = None;
            app.state.navigation_mut().usages_selected = false;
        }
    } else if app.state.search_running() && button(ui, Icon::Stop, *theme::STOP_RED, "Cancel") {
        app.dispatch("search.cancel", Value::Null);
    }
    usages_selected
}

/// Usages grouped by file, like the reference Find tool window.
pub fn usages_view(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(list) = app.state.navigation().usages.clone() else {
        return;
    };
    let mut groups: Vec<(&std::path::Path, Vec<&NavigationItem>)> = Vec::new();
    for item in &list.items {
        match groups.last_mut() {
            Some((path, items)) if *path == item.path.as_path() => items.push(item),
            _ => groups.push((item.path.as_path(), vec![item])),
        }
    }
    let mut open = None;
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        let mut summary = format!(
            "{} in {}",
            widgets::pluralize(list.items.len(), "usage", "usages"),
            widgets::pluralize(groups.len(), "file", "files")
        );
        if list.truncated {
            summary.push_str(" (limit reached)");
        }
        ui.label(egui::RichText::new(summary).color(theme::TEXT_SECONDARY));
    });
    ui.add_space(4.0);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (path, items) in &groups {
                let (rect, _) = list_row(ui, false);
                let name = IdeApp::file_name(path);
                icons::paint(
                    ui.painter(),
                    Rect::from_min_size(
                        egui::pos2(rect.left() + 12.0, rect.center().y - 8.0),
                        vec2(16.0, 16.0),
                    ),
                    Icon::for_file(&name),
                    icons::ICON,
                );
                let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
                let x = row_text(
                    ui,
                    rect,
                    rect.left() + 34.0,
                    &name,
                    font.clone(),
                    theme::TEXT_BRIGHT,
                );
                let parent = path
                    .parent()
                    .map(|parent| parent.display().to_string())
                    .unwrap_or_default();
                let x = row_text(ui, rect, x + 8.0, &parent, font.clone(), theme::TEXT_INFO);
                row_text(
                    ui,
                    rect,
                    x + 8.0,
                    &widgets::pluralize(items.len(), "usage", "usages"),
                    font,
                    theme::TEXT_INFO,
                );
                for item in items {
                    let (rect, response) = list_row(ui, false);
                    let inner =
                        Rect::from_min_max(egui::pos2(rect.left() + 26.0, rect.top()), rect.max);
                    item_row(ui, inner, item, false);
                    if response.clicked() {
                        open = Some((*item).clone());
                    }
                }
            }
        });
    if let Some(item) = open {
        goto(app, &ui.ctx().clone(), &item);
    }
}

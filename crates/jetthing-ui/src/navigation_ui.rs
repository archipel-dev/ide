//! Navigation UI: keyboard shortcuts, the chooser / Show Usages popup, and
//! the usages view of the Find tool window. Requests and history live in
//! `jetthing_core::quick_nav`.

use crate::icons::{self, Icon};
use crate::widgets::{self, row_text};
use crate::{IdeApp, popup_row, popup_stack};
use archipel_kit::components::{IconButton, ListRow, Tab, TabBar};
use archipel_kit::parts::{Surface, Tone, scroll_vertical};
use archipel_kit::tokens::size;
use archipel_kit::{Glyph, Theme};
use eframe::egui::{self, Key, Modifiers, Rect, vec2};
use jetthing_core::{NavigationItem, ToolWindow};
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
    if app.state.navigation.choices.is_some() {
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
    if back && app.state.navigation.can_go_back() {
        app.dispatch("navigate.back", Value::Null);
        ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
    }
    if forward && app.state.navigation.can_go_forward() {
        app.dispatch("navigate.forward", Value::Null);
        ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
    }
}

fn filter_id() -> egui::Id {
    egui::Id::new("navigation_choice_filter")
}

/// Items of the chooser that match what was typed (indices into its items).
fn visible_items(list: &jetthing_core::NavigationList, filter: &str) -> Vec<usize> {
    list.items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            filter.is_empty()
                || jetthing_core::quick_nav::matches(filter, &IdeApp::file_name(&item.path))
                || item
                    .line_text
                    .to_lowercase()
                    .contains(&filter.to_lowercase())
        })
        .map(|(index, _)| index)
        .collect()
}

/// The chooser's keys: ↑/↓ (wrapping), Home/End, PageUp/PageDown, Enter, and typing to
/// filter the rows. Escape is the popup stack's.
fn choices_keys(app: &mut IdeApp, ctx: &egui::Context, consume: &dyn Fn(Modifiers, Key) -> bool) {
    let Some(list) = app.state.navigation.choices.as_ref() else {
        return;
    };
    let mut filter: String = ctx
        .data(|data| data.get_temp(filter_id()))
        .unwrap_or_default();
    let mut selected: usize = ctx.data(|data| data.get_temp(selected_id()).unwrap_or(0));
    let visible = visible_items(list, &filter);
    let len = visible.len() as isize;
    let mut position = visible
        .iter()
        .position(|index| *index == selected)
        .map_or(0, |p| p as isize);
    if len > 0 {
        for (key, step) in [
            (Key::ArrowDown, 1),
            (Key::ArrowUp, -1),
            (Key::PageDown, POPUP_ROWS as isize),
            (Key::PageUp, -(POPUP_ROWS as isize)),
        ] {
            if consume(Modifiers::NONE, key) {
                position = if step.abs() == 1 {
                    (position + step).rem_euclid(len)
                } else {
                    (position + step).clamp(0, len - 1)
                };
            }
        }
        if consume(Modifiers::NONE, Key::Home) {
            position = 0;
        }
        if consume(Modifiers::NONE, Key::End) {
            position = len - 1;
        }
    }
    let before = filter.clone();
    ctx.input_mut(|input| {
        input.events.retain(|event| match event {
            egui::Event::Text(text) if !text.chars().any(char::is_control) => {
                filter.push_str(text);
                false
            }
            _ => true,
        });
    });
    if consume(Modifiers::NONE, Key::Backspace) {
        filter.pop();
    }
    if filter != before {
        position = 0;
    }
    let visible = visible_items(list, &filter);
    if let Some(index) = visible.get(position.max(0) as usize) {
        selected = *index;
    }
    ctx.data_mut(|data| {
        data.insert_temp(selected_id(), selected);
        data.insert_temp(filter_id(), filter);
    });
    if consume(Modifiers::NONE, Key::Enter) && visible.contains(&selected) {
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
    app.state.navigation.choices = None;
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
    let Some(list) = app.state.navigation.choices.clone() else {
        ctx.data_mut(|data| {
            data.remove::<usize>(selected_id());
            data.remove::<String>(filter_id());
        });
        return;
    };
    let selected: usize = ctx.data(|data| data.get_temp(selected_id()).unwrap_or(0));
    let filter: String = ctx
        .data(|data| data.get_temp(filter_id()))
        .unwrap_or_default();
    let visible = visible_items(&list, &filter);
    let anchor = app
        .caret_screen_pos
        .unwrap_or_else(|| ctx.content_rect().center());
    let colors = Theme::colors(ctx);
    let mut open = None;
    let response = egui::Area::new(egui::Id::new("navigation_choices"))
        .order(egui::Order::Foreground)
        .fixed_pos(anchor + vec2(-20.0, 4.0))
        .show(ctx, |ui| {
            Surface::Popup.frame(ctx).show(ui, |ui| {
                ui.set_width(620.0);
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(&list.title)
                            .font(archipel_kit::tokens::typography::semibold(
                                ui.ctx(),
                                archipel_kit::tokens::typography::BODY,
                            ))
                            .color(colors.text_strong),
                    );
                    let count = widgets::pluralize(visible.len(), "result", "results");
                    let suffix = if list.truncated { "+" } else { "" };
                    ui.label(
                        egui::RichText::new(format!("{count}{suffix}"))
                            .color(colors.text_secondary),
                    );
                    if !filter.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("· filter: {filter}"))
                                .color(colors.text_secondary),
                        );
                    }
                });
                ui.add_space(2.0);
                scroll_vertical()
                    .id_salt("navigation_choices_list")
                    .max_height(POPUP_ROWS as f32 * size::ROW_HEIGHT)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for index in visible.iter().copied() {
                            let item = &list.items[index];
                            let (rect, response) = choice_row(ui, index == selected, true, item);
                            if index == selected {
                                ui.scroll_to_rect(rect, None);
                            }
                            if response.clicked() {
                                open = Some(index);
                            }
                        }
                    });
            })
        })
        .response;
    let outside = popup_stack::pressed_outside(ctx, response.rect, None, None);
    if let Some(index) = open {
        open_choice(app, ctx, index);
    } else if outside || popup_stack::window_focus_lost(ctx) {
        app.state.navigation.choices = None;
    }
}

/// A chooser row, named like the location it opens.
fn choice_row(
    ui: &mut egui::Ui,
    selected: bool,
    enabled: bool,
    item: &NavigationItem,
) -> (Rect, egui::Response) {
    let name = format!("{} {}", IdeApp::file_name(&item.path), item.line + 1);
    let (rect, response) = popup_row::row(ui, selected, enabled, &name);
    if ui.is_rect_visible(rect) {
        item_row(ui, rect, item, true);
    }
    (rect, response)
}

/// A usage row in the Find tool window's usages view.
fn usage_row(ui: &mut egui::Ui, enabled: bool, item: &NavigationItem) -> egui::Response {
    let name = format!(
        "Usage in {} line {}",
        IdeApp::file_name(&item.path),
        item.line + 1
    );
    let (rect, response) = popup_row::row(ui, false, enabled, &name);
    if ui.is_rect_visible(rect) {
        let inner = Rect::from_min_max(egui::pos2(rect.left() + 22.0, rect.top()), rect.max);
        item_row(ui, inner, item, false);
    }
    response
}

/// `icon  file:line  code` row shared by the popup and the usages view. The symbol in
/// the code preview is emphasised with the editor's search-result background.
fn item_row(ui: &egui::Ui, rect: Rect, item: &NavigationItem, with_file: bool) {
    let colors = Theme::colors(ui.ctx());
    let name = crate::IdeApp::file_name(&item.path);
    let ui_font = egui::FontId::proportional(archipel_kit::tokens::typography::BODY);
    let mono = egui::FontId::monospace(archipel_kit::tokens::typography::CODE);
    let mut x = rect.left() + size::ROW_PILL_INSET + 6.0;
    if with_file {
        icons::paint(
            ui.painter(),
            widgets::icon_rect(x, rect.center().y),
            Icon::for_file(&name),
            colors.icon,
        );
        x += size::ICON + size::ICON_TEXT_GAP;
    }
    let code = item.line_text.trim_start();
    let trimmed = item.line_text.chars().count() - code.chars().count();
    let start = item.column.saturating_sub(trimmed);
    let length = item.end_column.saturating_sub(item.column);
    if !with_file {
        // Line number column.
        x = row_text(
            ui,
            rect,
            x,
            &format!("{}", item.line + 1),
            ui_font.clone(),
            colors.text_secondary,
        )
        .max(x + 28.0)
            + 12.0;
    }
    let editor = &colors_editor(ui.ctx());
    let mut job = egui::text::LayoutJob::default();
    for (index, ch) in code.chars().enumerate() {
        let inside = index >= start && index < start + length;
        job.append(
            &ch.to_string(),
            0.0,
            egui::text::TextFormat {
                font_id: mono.clone(),
                color: if inside { colors.text_strong } else { editor.0 },
                background: if inside {
                    editor.1.gamma_multiply(0.6)
                } else {
                    egui::Color32::TRANSPARENT
                },
                ..Default::default()
            },
        );
    }
    let galley = ui.painter().layout_job(job);
    let code_width = galley.size().x;
    let clip = rect.with_max_x(rect.right() - size::ROW_PILL_INSET - 4.0);
    ui.painter().with_clip_rect(clip).galley(
        egui::pos2(x, rect.center().y - galley.size().y / 2.0),
        galley,
        colors.text,
    );
    if with_file {
        let location = format!("{} {}", name, item.line + 1);
        let galley = ui
            .painter()
            .layout_no_wrap(location, ui_font, colors.text_secondary);
        let pos = egui::pos2(
            (rect.right() - galley.size().x - 10.0).max(x + code_width + 16.0),
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter()
            .with_clip_rect(rect)
            .galley(pos, galley, colors.text_secondary);
    }
}

/// The editor's text color and search-result background.
fn colors_editor(ctx: &egui::Context) -> (egui::Color32, egui::Color32) {
    let editor = Theme::current(ctx).editor;
    (editor.text, editor.search_result)
}

/// Header of the Find tool window: `Find in Files` and, after Find Usages,
/// a `Usages of x` tab. Returns `true` when the usages tab is selected.
pub fn find_header(app: &mut IdeApp, ui: &mut egui::Ui) -> bool {
    let colors = Theme::colors(ui.ctx());
    let usages_title = app
        .state
        .navigation
        .usages
        .as_ref()
        .map(|usages| usages.title.clone());
    let mut tabs = vec![Tab::new("Find in Files")];
    if let Some(title) = &usages_title {
        tabs.push(Tab::new(title.clone()));
    }
    let selected = usize::from(app.state.navigation.usages_selected && usages_title.is_some());
    let usages_selected = selected == 1;
    let mut hide = false;
    let mut close_usages = false;
    let mut cancel = false;
    ui.horizontal(|ui| {
        ui.set_min_height(widgets::TOOL_WINDOW_HEADER_HEIGHT);
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new("Find")
                .font(archipel_kit::tokens::typography::semibold(
                    ui.ctx(),
                    archipel_kit::tokens::typography::BODY,
                ))
                .color(colors.text_strong),
        );
        ui.add_space(8.0);
        if tabs.len() > 1 {
            if let Some(index) = TabBar::new(&tabs, selected).show(ui).selected {
                app.state.navigation.usages_selected = index == 1;
            }
        } else {
            ui.label(egui::RichText::new("Find in Files").color(colors.text_secondary));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            hide = ui
                .add(IconButton::new(Glyph::Minimize, "Hide").small(true))
                .clicked();
            if usages_selected {
                close_usages = ui
                    .add(IconButton::new(Glyph::Close, "Close Usages Tab").small(true))
                    .clicked();
            } else if app.state.search.is_running() {
                cancel = ui
                    .add(
                        IconButton::new(Glyph::Stop, "Cancel")
                            .small(true)
                            .tone(Tone::Stop),
                    )
                    .clicked();
            }
        });
    });
    if hide {
        app.toggle_tool_window(ToolWindow::Find);
    }
    if close_usages {
        app.state.navigation.usages = None;
        app.state.navigation.usages_selected = false;
    }
    if cancel {
        app.dispatch("search.cancel", Value::Null);
    }
    usages_selected && !close_usages
}

/// Usages grouped by file, like the reference Find tool window.
pub fn usages_view(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(list) = app.state.navigation.usages.clone() else {
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
        ui.label(egui::RichText::new(summary).color(Theme::colors(ui.ctx()).text_secondary));
    });
    ui.add_space(4.0);
    scroll_vertical()
        .id_salt("usages_view")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (path, items) in &groups {
                let name = IdeApp::file_name(path);
                let parent = path
                    .parent()
                    .map(|parent| parent.display().to_string())
                    .unwrap_or_default();
                let count = widgets::pluralize(items.len(), "usage", "usages");
                ListRow::new(name.clone())
                    .icon(Icon::for_file(&name).kit())
                    .secondary(format!("{parent}  {count}"))
                    .flat(true)
                    .show(ui);
                for item in items {
                    if usage_row(ui, true, item).clicked() {
                        open = Some((*item).clone());
                    }
                }
            }
        });
    if let Some(item) = open {
        goto(app, &ui.ctx().clone(), &item);
    }
}

#[cfg(test)]
pub(crate) fn choice_row_for_tests(
    ui: &mut egui::Ui,
    enabled: bool,
    item: &NavigationItem,
) -> egui::Response {
    choice_row(ui, false, enabled, item).1
}

#[cfg(test)]
pub(crate) fn usage_row_for_tests(
    ui: &mut egui::Ui,
    enabled: bool,
    item: &NavigationItem,
) -> egui::Response {
    usage_row(ui, enabled, item)
}

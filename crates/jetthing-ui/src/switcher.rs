//! The Switcher (Ctrl+Tab) popup and the glue that turns `navigate.*` requests
//! into popups. Cycling, ordering and committing live in `jetthing_core::quick_nav`.

use crate::icons::Icon;
use crate::{IdeApp, popup_stack, popups};
use archipel_kit::components::{ListRow, SectionHeader};
use archipel_kit::parts::{Surface, scroll_vertical};
use archipel_kit::tokens::size;
use eframe::egui;
use jetthing_core::ToolWindow;
use jetthing_core::quick_nav::{self, NavigationRequest, Switcher, SwitcherEntry, file_name};
use serde_json::Value;

/// Opens the popup a `navigate.*` command asked for and records the caret as a
/// recent location. Runs once per frame, after input has been dispatched.
pub fn frame(app: &mut IdeApp, ctx: &egui::Context) {
    app.state.record_caret_location();
    match app.state.quick_nav.request.take() {
        Some(NavigationRequest::SearchEverywhere(tab)) => app.palette.show_tab(tab),
        Some(NavigationRequest::RecentFiles) => app.open_dialog(popups::Dialog::Recent {
            query: String::new(),
            // Like the reference IDE, the first entry is the current file.
            selected: usize::from(app.state.recent_files.len() > 1),
        }),
        Some(NavigationRequest::RecentLocations) => {
            app.open_dialog(popups::Dialog::RecentLocations {
                selected: usize::from(app.state.quick_nav.locations.len() > 1),
            })
        }
        None => {}
    }
    // Held Ctrl was part of opening the Switcher: releasing it commits.
    let ctrl = ctx.input(|input| input.modifiers.ctrl);
    if app.state.quick_nav.switcher.is_none() {
        app.switcher_ctrl = None;
    } else if app.switcher_ctrl.is_none() {
        app.switcher_ctrl = Some(ctrl);
    } else if app.switcher_ctrl == Some(true) && !ctrl {
        commit(app, ctx);
    }
}

fn commit(app: &mut IdeApp, ctx: &egui::Context) {
    app.switcher_ctrl = None;
    let is_file = matches!(
        app.state
            .quick_nav
            .switcher
            .as_ref()
            .map(|s| s.selected_entry()),
        Some(SwitcherEntry::File(_))
    );
    app.dispatch("navigate.switcherCommit", Value::Null);
    if is_file {
        ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
    }
}

fn window_icon(window: ToolWindow) -> Icon {
    match window {
        ToolWindow::Project => Icon::Project,
        ToolWindow::Commit => Icon::Commit,
        ToolWindow::Find => Icon::Find,
        ToolWindow::Build => Icon::Build,
        ToolWindow::Problems => Icon::Problems,
        ToolWindow::Terminal => Icon::Terminal,
        ToolWindow::DeviceManager => Icon::Device,
        ToolWindow::Logcat => Icon::Logcat,
        ToolWindow::Git => Icon::Branch,
        ToolWindow::BuildVariants | ToolWindow::ResourceManager => Icon::Variants,
        ToolWindow::RunningDevices => Icon::Device,
        ToolWindow::Structure => Icon::Structure,
        ToolWindow::Gradle | ToolWindow::Bazel => Icon::Model,
        ToolWindow::AiChat => Icon::Ai,
        ToolWindow::Debug => Icon::Debug,
        ToolWindow::Hierarchy => Icon::Structure,
    }
}

/// Entries shown for `filter` (indices into the switcher's entries): those whose name
/// matches what the user typed, all of them when nothing was typed.
fn visible_entries(app: &IdeApp, switcher: &Switcher, filter: &str) -> Vec<usize> {
    switcher
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| quick_nav::matches(filter, &entry_name(app, entry)))
        .map(|(index, _)| index)
        .collect()
}

fn entry_name(_app: &IdeApp, entry: &SwitcherEntry) -> String {
    match entry {
        SwitcherEntry::File(path) => file_name(path),
        SwitcherEntry::ToolWindow(window) => window.title().to_owned(),
    }
}

fn filter_id() -> egui::Id {
    egui::Id::new("switcher_filter")
}

/// Keys of the open Switcher: ↑/↓ (wrapping), Home/End, PageUp/PageDown, Enter, and
/// typing to filter. Escape is the popup stack's.
fn handle_keys(app: &mut IdeApp, ctx: &egui::Context, visible: &[usize], list_height: f32) {
    let consume = |key| ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key));
    let Some(selected) = app
        .state
        .quick_nav
        .switcher
        .as_ref()
        .map(|switcher| switcher.selected)
    else {
        return;
    };
    let position = visible.iter().position(|index| *index == selected);
    let len = visible.len() as isize;
    let page = ((list_height / size::ROW_HEIGHT) as isize - 1).max(1);
    let mut target = position.map(|p| p as isize);
    for (key, step) in [
        (egui::Key::ArrowDown, 1),
        (egui::Key::ArrowUp, -1),
        (egui::Key::PageDown, page),
        (egui::Key::PageUp, -page),
    ] {
        if consume(key) && len > 0 {
            let current = target.unwrap_or(-1);
            target = Some(if step.abs() == 1 {
                (current + step).rem_euclid(len)
            } else {
                (current + step).clamp(0, len - 1)
            });
        }
    }
    if consume(egui::Key::Home) && len > 0 {
        target = Some(0);
    }
    if consume(egui::Key::End) && len > 0 {
        target = Some(len - 1);
    }
    if let Some(index) = target.and_then(|t| visible.get(t as usize).copied())
        && let Some(switcher) = app.state.quick_nav.switcher.as_mut()
    {
        switcher.selected = index;
    }
    // Typing filters the list; the selection follows to the first match.
    let mut filter: String = ctx
        .data(|data| data.get_temp(filter_id()))
        .unwrap_or_default();
    let before = filter.clone();
    ctx.input_mut(|input| {
        input.events.retain(|event| match event {
            egui::Event::Text(text) if !text.chars().any(char::is_control) => {
                filter.push_str(text);
                false
            }
            _ => true,
        });
        if input.consume_key(egui::Modifiers::NONE, egui::Key::Backspace) {
            filter.pop();
        }
    });
    if filter != before {
        ctx.data_mut(|data| data.insert_temp(filter_id(), filter));
    }
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(switcher) = app.state.quick_nav.switcher.clone() else {
        ctx.data_mut(|data| data.remove::<String>(filter_id()));
        return;
    };
    let screen = ctx.content_rect();
    let list_height = (screen.height() * 0.5).clamp(160.0, 420.0);
    let filter: String = ctx
        .data(|data| data.get_temp(filter_id()))
        .unwrap_or_default();
    let visible = visible_entries(app, &switcher, &filter);
    handle_keys(app, ctx, &visible, list_height);
    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
        return commit(app, ctx);
    }
    let filter: String = ctx
        .data(|data| data.get_temp(filter_id()))
        .unwrap_or_default();
    // A filter that changed this frame moves the selection to its first match.
    let switcher = app.state.quick_nav.switcher.clone().unwrap_or(switcher);
    let visible = visible_entries(app, &switcher, &filter);
    if let Some(first) = visible.first().copied()
        && !visible.contains(&switcher.selected)
        && let Some(state) = app.state.quick_nav.switcher.as_mut()
    {
        state.selected = first;
    }
    let selected = app
        .state
        .quick_nav
        .switcher
        .as_ref()
        .map_or(switcher.selected, |s| s.selected);
    let width = 420.0_f32.min(screen.width() - 40.0);
    let mut clicked = None;
    let shown = egui::Area::new(egui::Id::new("switcher"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(
            screen.center().x - width / 2.0,
            screen.top() + screen.height() * 0.2,
        ))
        .show(ctx, |ui| {
            Surface::Popup.frame(ctx).show(ui, |ui| {
                ui.set_width(width);
                clicked = list(app, ui, &switcher, &visible, &filter, selected, list_height);
            })
        });
    let outside = popup_stack::pressed_outside(ctx, shown.response.rect, None, None);
    if let Some(index) = clicked
        && let Some(state) = app.state.quick_nav.switcher.as_mut()
    {
        state.selected = index;
        commit(app, ctx);
    } else if outside || popup_stack::window_focus_lost(ctx) {
        app.dispatch("navigate.switcherCancel", Value::Null);
    }
}

/// The popup's rows (kit `ListRow`s under section headers) and, while typing, the
/// filter. Returns the clicked entry.
fn list(
    app: &IdeApp,
    ui: &mut egui::Ui,
    switcher: &Switcher,
    visible: &[usize],
    filter: &str,
    selected: usize,
    list_height: f32,
) -> Option<usize> {
    let mut clicked = None;
    if !filter.is_empty() {
        ListRow::new(format!("Filter: {filter}"))
            .icon(Icon::Search.kit())
            .flat(true)
            .enabled(false)
            .show(ui);
    }
    scroll_vertical()
        .id_salt("switcher_list")
        .max_height(list_height)
        .auto_shrink([true, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut section = "";
            for index in visible.iter().copied() {
                let entry = &switcher.entries[index];
                let wanted = match entry {
                    SwitcherEntry::File(_) => "Files",
                    SwitcherEntry::ToolWindow(_) => "Tool Windows",
                };
                if wanted != section {
                    section = wanted;
                    ui.add(SectionHeader::new(wanted));
                }
                let (name, detail, icon) = match entry {
                    SwitcherEntry::File(path) => (
                        file_name(path),
                        path.parent()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default(),
                        app.file_icon(path),
                    ),
                    SwitcherEntry::ToolWindow(window) => (
                        window.title().to_owned(),
                        String::new(),
                        window_icon(*window),
                    ),
                };
                let mut row = ListRow::new(name)
                    .icon(icon.kit())
                    .selected(index == selected)
                    .flat(true);
                if !detail.is_empty() {
                    row = row.secondary(detail);
                }
                let shown = row.show(ui);
                if index == selected {
                    shown.response.scroll_to_me(None);
                }
                if shown.response.clicked() {
                    clicked = Some(index);
                }
            }
            if visible.is_empty() {
                ListRow::new("Nothing found")
                    .flat(true)
                    .enabled(false)
                    .show(ui);
            }
        });
    clicked
}

#[cfg(test)]
pub(crate) fn list_for_tests(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(switcher) = app.state.quick_nav.switcher.clone() else {
        return;
    };
    let visible = visible_entries(app, &switcher, "");
    list(app, ui, &switcher, &visible, "", switcher.selected, 240.0);
}

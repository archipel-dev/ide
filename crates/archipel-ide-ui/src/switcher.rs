//! The Switcher (Ctrl+Tab) popup and the glue that turns `navigate.*` requests
//! into popups. Cycling, ordering and committing live in `archipel_ide_core::quick_nav`.

use crate::icons::{self, Icon};
use crate::{IdeApp, popups, theme};
use archipel_ide_core::ToolWindow;
use archipel_ide_core::quick_nav::{NavigationRequest, SwitcherEntry, file_name};
use eframe::egui::{self, Rect, Sense, vec2};
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

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(switcher) = app.state.quick_nav.switcher.clone() else {
        return;
    };
    let backward = |ctx: &egui::Context, key| {
        ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key))
    };
    if backward(ctx, egui::Key::ArrowDown) {
        app.state.switcher_advance(false);
    }
    if backward(ctx, egui::Key::ArrowUp) {
        app.state.switcher_advance(true);
    }
    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
        return commit(app, ctx);
    }
    let selected = app
        .state
        .quick_nav
        .switcher
        .as_ref()
        .map_or(switcher.selected, |s| s.selected);
    let screen = ctx.content_rect();
    let width = 420.0_f32.min(screen.width() - 40.0);
    let mut clicked = None;
    egui::Area::new(egui::Id::new("switcher"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(
            screen.center().x - width / 2.0,
            screen.top() + screen.height() * 0.2,
        ))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .corner_radius(10)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(width - 16.0);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let mut section = "";
                    for (index, entry) in switcher.entries.iter().enumerate() {
                        let wanted = match entry {
                            SwitcherEntry::File(_) => "Files",
                            SwitcherEntry::ToolWindow(_) => "Tool Windows",
                        };
                        if wanted != section {
                            section = wanted;
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(wanted)
                                    .strong()
                                    .color(theme::TEXT_SECONDARY),
                            );
                            ui.add_space(2.0);
                        }
                        let (rect, response) = ui
                            .allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
                        if index == selected {
                            ui.painter().rect_filled(rect, 4.0, theme::SELECTION);
                        } else if response.hovered() {
                            ui.painter().rect_filled(rect, 4.0, theme::HOVER);
                        }
                        let icon_rect = Rect::from_min_size(
                            rect.left_center() + vec2(8.0, -8.0),
                            vec2(16.0, 16.0),
                        );
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
                        icons::paint(ui.painter(), icon_rect, icon, *icons::ICON);
                        let x = crate::widgets::row_text(
                            ui,
                            rect,
                            rect.left() + 32.0,
                            &name,
                            egui::FontId::proportional(theme::UI_FONT_SIZE),
                            *theme::TEXT_BRIGHT,
                        );
                        crate::widgets::row_text(
                            ui,
                            rect,
                            x + 10.0,
                            &detail,
                            egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0),
                            *theme::TEXT_INFO,
                        );
                        if response.clicked() {
                            clicked = Some(index);
                        }
                    }
                });
        });
    if let Some(index) = clicked
        && let Some(state) = app.state.quick_nav.switcher.as_mut()
    {
        state.selected = index;
        commit(app, ctx);
    }
}

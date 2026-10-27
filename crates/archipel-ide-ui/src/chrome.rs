//! Workbench chrome: merged main toolbar, tool window stripes and status bar.

use crate::icons::{self, Icon};
use crate::widgets::{self, chrome_frame, glow_through_frame};
use crate::{IdeApp, keys, theme};
use archipel_ide_core::{Anchor, ToolWindow};
use eframe::egui::{self, Color32, Rect, Sense, vec2};
use serde_json::{Value, json};

pub const TOOLBAR_HEIGHT: f32 = 40.0;
pub const STATUS_BAR_HEIGHT: f32 = 26.0;
pub const STRIPE_WIDTH: f32 = 40.0;

/// Width reserved at the left of the main toolbar for the macOS traffic-light
/// buttons, which overlay the content in the unified title bar.
pub const MAC_TRAFFIC_LIGHT_INSET: f32 = 72.0;

/// Left inset of the toolbar content: room for the traffic lights on macOS only.
pub fn toolbar_leading_inset(macos: bool) -> f32 {
    if macos { MAC_TRAFFIC_LIGHT_INSET } else { 0.0 }
}

/// Whether the window draws its own title bar (no OS decorations).
pub fn custom_title_bar() -> bool {
    !cfg!(target_os = "macos")
}

pub fn main_toolbar(app: &mut IdeApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("main_toolbar")
        .exact_height(TOOLBAR_HEIGHT)
        .show_separator_line(false)
        .frame(chrome_frame(ctx))
        .show(ctx, |ui| {
            let bar = ui.max_rect();
            let glow = app
                .state
                .settings()
                .read()
                .bool(archipel_ide_settings::ids::PROJECT_COLORS)
                .unwrap_or(true)
                .then(|| crate::toolbar_gradient::reserve(ctx));
            // Empty toolbar space drags the window, like a native title bar.
            let drag = ui.interact(bar, egui::Id::new("toolbar_drag"), Sense::click_and_drag());
            if drag.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
            let mut content = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(bar.shrink2(vec2(6.0, 0.0)))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            if custom_title_bar() && app.state.project.is_some() {
                let active = app
                    .state
                    .active_document
                    .and_then(|index| app.state.documents.get(index));
                let title = window_title(&app.project_name(), active.map(|doc| doc.path.as_path()));
                ui.painter().text(
                    bar.center(),
                    egui::Align2::CENTER_CENTER,
                    &title,
                    egui::FontId::proportional(theme::UI_FONT_SIZE),
                    *theme::TEXT_SECONDARY,
                );
            }
            let ui = &mut content;
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.add_space(toolbar_leading_inset(cfg!(target_os = "macos")));
            if !crate::main_menu::show(app, ui) {
                ui.add_space(4.0);
                let badge = project_widget(app, ui);
                if let Some(slot) = glow {
                    let anchor = crate::toolbar_gradient::Anchor {
                        bar,
                        badge_left: badge.left(),
                        badge_right: badge.right(),
                    };
                    let color = theme::avatar_color(&app.project_name());
                    crate::toolbar_gradient::paint(ctx, slot, anchor, color);
                }
                vcs_widget(app, ui);
                ui.add_space(8.0);
                crate::agents_ui::mode_switch(app, ui);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if custom_title_bar() {
                    window_controls(ui);
                    ui.add_space(8.0);
                }
                if icons::button(
                    ui,
                    Icon::Search,
                    &format!(
                        "Search Everywhere ({})",
                        app.action_shortcut("SearchEverywhere").unwrap_or_default()
                    ),
                )
                .clicked()
                {
                    app.dispatch("navigate.searchEverywhere", serde_json::Value::Null);
                }
                ui.add_space(6.0);
                run_widget(app, ui);
                crate::device_manager::device_selector(app, ui);
            });
        });
}

/// Returns the project badge's rectangle.
fn project_widget(app: &mut IdeApp, ui: &mut egui::Ui) -> Rect {
    let name = app.project_name();
    let galley = ui.painter().layout_no_wrap(
        name.clone(),
        theme::semibold(theme::UI_FONT_SIZE),
        *theme::TEXT_BRIGHT,
    );
    let width = 8.0 + 20.0 + 8.0 + galley.size().x + 22.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
    let fill = if response.hovered() {
        *theme::HOVER
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    let badge = Rect::from_min_size(rect.left_center() + vec2(8.0, -10.0), vec2(20.0, 20.0));
    ui.painter()
        .rect_filled(badge, 4.0, theme::avatar_color(&name));
    ui.painter().text(
        badge.center(),
        egui::Align2::CENTER_CENTER,
        theme::project_initials(&name),
        theme::semibold(10.0),
        Color32::WHITE,
    );
    let text_pos = egui::pos2(badge.right() + 8.0, rect.center().y - galley.size().y / 2.0);
    let text_right = text_pos.x + galley.size().x;
    ui.painter().galley(text_pos, galley, *theme::TEXT_BRIGHT);
    icons::paint(
        ui.painter(),
        Rect::from_min_size(
            egui::pos2(text_right + 4.0, rect.center().y - 7.0),
            vec2(14.0, 14.0),
        ),
        Icon::ChevronDown,
        *theme::TEXT_SECONDARY,
    );
    let root = app
        .state
        .project
        .as_ref()
        .map(|project| project.root().display().to_string());
    let response = response.on_hover_text(root.clone().unwrap_or_default());
    egui::Popup::menu(&response).show(|ui| {
        ui.set_min_width(260.0);
        if let Some(root) = &root {
            ui.label(egui::RichText::new(root).color(theme::TEXT_SECONDARY));
            ui.separator();
        }
        if ui.button("Open Project…").clicked() {
            app.open_project_dialog();
            ui.close();
        }
        if ui.button("New Project…").clicked() {
            app.new_project_dialog();
            ui.close();
        }
        if ui.button("Refresh Plugins").clicked() {
            app.dispatch("plugin.refresh", Value::Null);
            ui.close();
        }
        ui.label(
            egui::RichText::new(widgets::pluralize(
                app.state
                    .plugin_host
                    .registry
                    .entries()
                    .iter()
                    .filter(|entry| entry.is_active())
                    .count(),
                "plugin active",
                "plugins active",
            ))
            .color(theme::TEXT_INFO),
        );
    });
    badge
}

fn vcs_widget(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(git) = app.state.git_status() else {
        return;
    };
    let branch = git.branch.clone();
    let response = widgets::flat_widget(
        ui,
        Some((Icon::Branch, *icons::ICON)),
        &branch,
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        *theme::TEXT_BRIGHT,
        true,
        28.0,
    )
    .on_hover_text("Git Branch");
    egui::Popup::menu(&response)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| crate::branches_popup::menu(app, ui));
}

fn run_widget(app: &mut IdeApp, ui: &mut egui::Ui) {
    let running = app.state.execution().is_running();
    // Right-to-left layout: rightmost item first.
    if running
        && icons::button_colored(
            ui,
            Icon::Stop,
            *theme::STOP_RED,
            &format!("Stop ({})", app.action_shortcut("Stop").unwrap_or_default()),
            false,
        )
        .clicked()
    {
        app.dispatch("run.cancel", Value::Null);
    }
    let detected = app.state.detected_build.clone();
    // Right-to-left: the Debug button sits right of Run, as in the reference.
    if app.state.is_debugging() {
        if icons::button_colored(ui, Icon::Stop, theme::STOP_RED, "Stop Debugging", false).clicked()
        {
            app.dispatch("debug.stop", Value::Null);
        }
    } else {
        let can_debug = app.commands.is_enabled("debug.start", &app.state);
        let color = if can_debug {
            theme::RUN_GREEN
        } else {
            theme::TEXT_DISABLED
        };
        if ui
            .add_enabled_ui(can_debug, |ui| {
                icons::button_colored(
                    ui,
                    Icon::Debug,
                    color,
                    &format!("Debug ({})", keys::display("⌃D")),
                    false,
                )
            })
            .inner
            .clicked()
        {
            app.dispatch("debug.start", Value::Null);
        }
    }
    let can_build = app.commands.is_enabled("build.project", &app.state);
    let build_tip = match &detected {
        None if app.state.project.is_some() => "No supported build system detected".to_owned(),
        _ => format!(
            "Build Project ({})",
            app.action_shortcut("CompileDirty").unwrap_or_default()
        ),
    };
    let build = ui.add_enabled_ui(can_build, |ui| icons::button(ui, Icon::Build, &build_tip));
    if build.inner.clicked() {
        app.dispatch("build.project", Value::Null);
    }
    // Android apps have no `gradlew run`; Run deploys 'app' to the selected device.
    let android_run = app.state.android.project.is_some();
    let run_command = if android_run {
        "android.app.deploy"
    } else {
        "run.project"
    };
    let can_run = app.commands.is_enabled(run_command, &app.state);
    let run_color = if can_run {
        *theme::RUN_GREEN
    } else {
        *theme::TEXT_DISABLED
    };
    let run_tip = match &detected {
        _ if android_run => format!(
            "Run 'app' ({})",
            app.action_shortcut("Run").unwrap_or_default()
        ),
        _ if app.state.execution().configurations().selected().is_some() => {
            format!(
                "Run '{}' ({})",
                app.state.run_widget_label().unwrap_or_default(),
                app.action_shortcut("Run").unwrap_or_default()
            )
        }
        Some(build) if build.run_unavailable.is_some() => {
            format!(
                "Run unavailable: {}",
                build.run_unavailable.as_deref().unwrap_or_default()
            )
        }
        None if app.state.project.is_some() => "No supported build system detected".to_owned(),
        _ => format!("Run ({})", app.action_shortcut("Run").unwrap_or_default()),
    };
    let run = ui.add_enabled_ui(can_run, |ui| {
        icons::button_colored(ui, Icon::Run, run_color, &run_tip, false)
    });
    // Disabled widgets do not show the normal tooltip; keep the reason discoverable.
    if !can_run {
        run.response.on_disabled_hover_text(&run_tip);
    } else if run.inner.clicked() {
        app.dispatch(run_command, Value::Null);
    }
    if let Some(label) = app.state.run_widget_label() {
        let response = widgets::flat_widget(
            ui,
            None,
            &label,
            egui::FontId::proportional(theme::UI_FONT_SIZE),
            *theme::TEXT_BRIGHT,
            true,
            28.0,
        );
        egui::Popup::menu(&response).show(|ui| {
            ui.set_min_width(240.0);
            let selected = app
                .state
                .execution()
                .configurations()
                .selected()
                .map(|config| config.name.clone());
            let names: Vec<(String, bool)> = app
                .state
                .execution()
                .configurations()
                .configurations()
                .iter()
                .map(|config| (config.name.clone(), config.temporary))
                .collect();
            let idle = !app.state.execution().is_running();
            // `Current File` runs the entry point or test at the caret.
            let clicked = run_row(ui, "Current File", false, selected.is_none(), false);
            if clicked == RowAction::Select {
                app.dispatch("runConfig.selectCurrentFile", Value::Null);
                ui.close();
            }
            for (name, temporary) in names {
                let action = run_row(
                    ui,
                    &name,
                    temporary,
                    selected.as_deref() == Some(name.as_str()),
                    idle,
                );
                match action {
                    RowAction::None => {}
                    RowAction::Select => {
                        app.dispatch("runConfig.select", json!({"name": name}));
                        ui.close();
                    }
                    RowAction::Run => {
                        app.dispatch("runConfig.run", json!({"name": name}));
                        ui.close();
                    }
                    RowAction::Debug => {
                        app.dispatch("runConfig.select", json!({"name": name}));
                        app.dispatch("debug.start", Value::Null);
                        ui.close();
                    }
                    RowAction::More => {
                        app.dispatch("runConfig.edit", json!({"name": name}));
                        ui.close();
                    }
                }
            }
            if let Some(build) = &detected
                && selected.is_none()
                && let Some(reason) = &build.run_unavailable
            {
                ui.label(
                    egui::RichText::new(format!("Run unavailable: {reason}"))
                        .color(theme::TEXT_INFO)
                        .size(theme::UI_FONT_SIZE - 1.0),
                );
            }
            ui.separator();
            let enabled = app.commands.is_enabled("runConfig.edit", &app.state);
            if ui
                .add_enabled(enabled, egui::Button::new("Edit Configurations…"))
                .clicked()
            {
                app.dispatch("runConfig.edit", Value::Null);
                ui.close();
            }
        });
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RowAction {
    None,
    Select,
    Run,
    Debug,
    More,
}

/// One row of the run widget popup; Run / Debug / ⋮ buttons appear on hover.
/// `buttons` is false while a task runs (or for `Current File`, which has no extras).
fn run_row(
    ui: &mut egui::Ui,
    name: &str,
    temporary: bool,
    selected: bool,
    buttons: bool,
) -> RowAction {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width().max(240.0), 24.0), Sense::click());
    if response.hovered() || selected {
        let fill = if response.hovered() {
            *theme::HOVER
        } else {
            *theme::SELECTION
        };
        ui.painter().rect_filled(rect, 2.0, fill);
    }
    let color = if temporary {
        *theme::TEXT_SECONDARY
    } else {
        *theme::TEXT
    };
    ui.painter().text(
        rect.left_center() + vec2(8.0, 0.0),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        color,
    );
    let mut action = if response.clicked() {
        RowAction::Select
    } else {
        RowAction::None
    };
    if buttons && response.hovered() {
        let slots = [
            (RowAction::Run, Some(Icon::Run), *theme::RUN_GREEN, "Run"),
            (RowAction::Debug, Some(Icon::Debug), *icons::ICON, "Debug"),
            (RowAction::More, None, *icons::ICON, "More actions"),
        ];
        for (index, (kind, icon, tint, tip)) in slots.into_iter().rev().enumerate() {
            let slot = Rect::from_center_size(
                egui::pos2(rect.right() - 14.0 - 22.0 * index as f32, rect.center().y),
                vec2(20.0, 20.0),
            );
            let hit = ui.interact(slot, response.id.with(tip), Sense::click());
            if hit.hovered() {
                ui.painter().rect_filled(slot, 3.0, *theme::PRESSED);
            }
            match icon {
                Some(icon) => icons::paint(ui.painter(), slot.shrink(3.0), icon, tint),
                None => {
                    for step in [-4.0, 0.0, 4.0] {
                        ui.painter()
                            .circle_filled(slot.center() + vec2(0.0, step), 1.2, tint);
                    }
                }
            }
            if hit.on_hover_text(tip).clicked() {
                action = kind;
            }
        }
    }
    action
}

pub(crate) fn window_controls(ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let control = |ui: &mut egui::Ui, kind: u8, hover: Color32| {
        let (rect, response) = ui.allocate_exact_size(vec2(40.0, TOOLBAR_HEIGHT), Sense::click());
        if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, hover);
        }
        let center = rect.center();
        let stroke = egui::Stroke::new(1.0_f32, icons::ICON);
        match kind {
            0 => {
                ui.painter()
                    .line_segment([center + vec2(-5.0, 4.0), center + vec2(5.0, 4.0)], stroke);
            }
            1 => {
                ui.painter().rect_stroke(
                    Rect::from_center_size(center, vec2(10.0, 10.0)),
                    1.0,
                    stroke,
                    egui::StrokeKind::Middle,
                );
            }
            _ => {
                ui.painter()
                    .line_segment([center + vec2(-5.0, -5.0), center + vec2(5.0, 5.0)], stroke);
                ui.painter()
                    .line_segment([center + vec2(5.0, -5.0), center + vec2(-5.0, 5.0)], stroke);
            }
        }
        response
    };
    if control(ui, 2, Color32::from_rgb(0xC4, 0x2B, 0x1C)).clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    if control(ui, 1, *theme::HOVER).clicked() {
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
    if control(ui, 0, *theme::HOVER).clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }
}

/// Edge handles that resize an undecorated window.
pub fn window_resize_handles(ctx: &egui::Context) {
    if !custom_title_bar() || ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
        return;
    }
    use egui::ResizeDirection as Dir;
    let screen = ctx.content_rect();
    let edge = 5.0;
    let handles = [
        (
            Rect::from_min_max(
                screen.left_top(),
                screen.left_top() + vec2(edge, edge) * 2.0,
            ),
            Dir::NorthWest,
            egui::CursorIcon::ResizeNorthWest,
        ),
        (
            Rect::from_min_max(
                screen.right_top() - vec2(edge * 2.0, 0.0),
                screen.right_top() + vec2(0.0, edge * 2.0),
            ),
            Dir::NorthEast,
            egui::CursorIcon::ResizeNorthEast,
        ),
        (
            Rect::from_min_max(
                screen.left_bottom() - vec2(0.0, edge * 2.0),
                screen.left_bottom() + vec2(edge * 2.0, 0.0),
            ),
            Dir::SouthWest,
            egui::CursorIcon::ResizeSouthWest,
        ),
        (
            Rect::from_min_max(
                screen.right_bottom() - vec2(edge, edge) * 2.0,
                screen.right_bottom(),
            ),
            Dir::SouthEast,
            egui::CursorIcon::ResizeSouthEast,
        ),
        (
            Rect::from_min_max(screen.left_top(), screen.right_top() + vec2(0.0, edge)),
            Dir::North,
            egui::CursorIcon::ResizeVertical,
        ),
        (
            Rect::from_min_max(
                screen.left_bottom() - vec2(0.0, edge),
                screen.right_bottom(),
            ),
            Dir::South,
            egui::CursorIcon::ResizeVertical,
        ),
        (
            Rect::from_min_max(screen.left_top(), screen.left_bottom() + vec2(edge, 0.0)),
            Dir::West,
            egui::CursorIcon::ResizeHorizontal,
        ),
        (
            Rect::from_min_max(screen.right_top() - vec2(edge, 0.0), screen.right_bottom()),
            Dir::East,
            egui::CursorIcon::ResizeHorizontal,
        ),
    ];
    egui::Area::new(egui::Id::new("window_resize_handles"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.left_top())
        .interactable(true)
        .show(ctx, |ui| {
            for (index, (rect, direction, cursor)) in handles.into_iter().enumerate() {
                let response = ui
                    .interact(rect, egui::Id::new(("resize", index)), Sense::drag())
                    .on_hover_cursor(cursor);
                if response.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
                }
            }
        });
}

pub(crate) fn stripe_button(app: &mut IdeApp, ui: &mut egui::Ui, window: ToolWindow, icon: Icon) {
    let selected = app.state.tool_windows.is_visible(window);
    let (rect, response) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
    let fill = if selected {
        *theme::ACCENT
    } else if response.hovered() {
        *theme::HOVER
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    let color = if selected {
        Color32::WHITE
    } else {
        *icons::ICON
    };
    icons::paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), vec2(16.0, 16.0)),
        icon,
        color,
    );
    let mut badge = None;
    if window == ToolWindow::Problems {
        let counts = app.state.problems.counts();
        if counts.errors > 0 {
            badge = Some(theme::ERROR);
        } else if counts.warnings > 0 {
            badge = Some(theme::WARNING);
        }
    }
    if window == ToolWindow::Build && app.state.execution().is_running() {
        badge = Some(theme::RUN_GREEN);
    }
    if window == ToolWindow::Agents {
        let counts = app.state.agents.counts();
        if counts.needs_input > 0 {
            badge = Some(theme::WARNING);
        } else if counts.working > 0 {
            badge = Some(theme::RUN_GREEN);
        }
    }
    if let Some(color) = badge {
        ui.painter()
            .circle_filled(rect.right_top() + vec2(-7.0, 7.0), 3.0, color);
    }
    let tooltip = match window.shortcut() {
        "" => window.title().to_owned(),
        shortcut => format!("{} ({})", window.title(), keys::display(shortcut)),
    };
    if response.on_hover_text(tooltip).clicked() {
        app.toggle_tool_window(window);
    }
}

pub fn left_stripe(app: &mut IdeApp, ctx: &egui::Context) {
    egui::SidePanel::left("left_stripe")
        .exact_width(STRIPE_WIDTH)
        .resizable(false)
        .show_separator_line(false)
        .frame(glow_through_frame().inner_margin(egui::Margin::symmetric(4, 6)))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.vertical(|ui| {
                stripe_button(app, ui, ToolWindow::Project, Icon::Project);
                stripe_button(app, ui, ToolWindow::Commit, Icon::Commit);
                stripe_button(app, ui, ToolWindow::Structure, Icon::Structure);
                if !app.state.agents.sessions().is_empty()
                    || app.state.tool_windows.is_visible(ToolWindow::Agents)
                {
                    stripe_button(
                        app,
                        ui,
                        ToolWindow::Agents,
                        Icon::Kit(crate::agents_ui::glyph(crate::agents_ui::Concept::Agent).into()),
                    );
                }
                if app.state.android.project.is_some() {
                    stripe_button(app, ui, ToolWindow::ResourceManager, Icon::Variants);
                    stripe_button(app, ui, ToolWindow::BuildVariants, Icon::Variants);
                }
                let (rect, _) = ui.allocate_exact_size(vec2(32.0, 1.0), Sense::hover());
                ui.painter().line_segment(
                    [
                        rect.left_center() + vec2(6.0, 0.0),
                        rect.right_center() - vec2(6.0, 0.0),
                    ],
                    egui::Stroke::new(1.0_f32, theme::TOOLBAR_SEPARATOR),
                );
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    stripe_button(app, ui, ToolWindow::Git, Icon::Branch);
                    if app.state.hierarchy.current.is_some() {
                        stripe_button(app, ui, ToolWindow::Hierarchy, Icon::Structure);
                    }
                    if app.state.android.project.is_some() {
                        stripe_button(app, ui, ToolWindow::Logcat, Icon::Logcat);
                    }
                    stripe_button(app, ui, ToolWindow::Problems, Icon::Problems);
                    stripe_button(app, ui, ToolWindow::Todo, Icon::Success);
                    stripe_button(app, ui, ToolWindow::Terminal, Icon::Terminal);
                    stripe_button(app, ui, ToolWindow::Build, Icon::Build);
                    stripe_button(app, ui, ToolWindow::Debug, Icon::Debug);
                    stripe_button(app, ui, ToolWindow::Find, Icon::Find);
                });
            });
        });
}

/// Built-in right-anchored tool windows offered for the open project, top to bottom.
pub(crate) fn right_stripe_windows(app: &IdeApp) -> Vec<(ToolWindow, Icon)> {
    let mut windows = Vec::new();
    if app.state.is_gradle_project() || app.state.tool_windows.is_visible(ToolWindow::Gradle) {
        windows.push((ToolWindow::Gradle, Icon::Model));
    }
    if app.state.is_bazel_project() || app.state.tool_windows.is_visible(ToolWindow::Bazel) {
        windows.push((ToolWindow::Bazel, Icon::Model));
    }
    if app.state.android.project.is_some()
        || app.state.tool_windows.is_visible(ToolWindow::DeviceManager)
    {
        windows.push((ToolWindow::DeviceManager, Icon::Device));
    }
    if app.state.android.project.is_some()
        || app
            .state
            .tool_windows
            .is_visible(ToolWindow::RunningDevices)
    {
        windows.push((ToolWindow::RunningDevices, Icon::Device));
    }
    windows
}

/// Right tool window stripe: the project's right-anchored tool windows (Gradle,
/// Bazel, Device Manager) and AI Chat, which is always offered. Plugin tool
/// windows join this stripe (`plugin_ui::right_side`).
pub fn right_stripe(app: &mut IdeApp, ctx: &egui::Context) {
    let windows = right_stripe_windows(app);
    egui::SidePanel::right("right_stripe")
        .exact_width(STRIPE_WIDTH)
        .resizable(false)
        .show_separator_line(false)
        .frame(chrome_frame(ctx).inner_margin(egui::Margin::symmetric(4, 6)))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            for (window, icon) in windows {
                stripe_button(app, ui, window, icon);
            }
            stripe_button(app, ui, ToolWindow::AiChat, Icon::Ai);
        });
}

/// First line of `text`, shortened to `max` characters with an ellipsis.
fn first_line_truncated(text: &str, max: usize) -> String {
    let line = text.lines().next().unwrap_or_default();
    if line.chars().count() > max {
        let mut short: String = line.chars().take(max.saturating_sub(1)).collect();
        short.push('…');
        short
    } else {
        line.to_owned()
    }
}

/// Whether the file on disk is read-only; probed at most once per second.
fn is_read_only(ctx: &egui::Context, path: &std::path::Path) -> bool {
    let now = ctx.input(|input| input.time);
    let id = egui::Id::new(("read_only_probe", path));
    if let Some((at, value)) = ctx.data(|data| data.get_temp::<(f64, bool)>(id))
        && now - at < 1.0
    {
        return value;
    }
    let value = std::fs::metadata(path).is_ok_and(|meta| meta.permissions().readonly());
    ctx.data_mut(|data| data.insert_temp(id, (now, value)));
    value
}

/// Window title like the reference IDE: `project – File.java [module.sourceSet]`.
/// The module is the directory above `src/` (or the project itself) joined with
/// the source set (`main`, `test`, …) when the file lives under `src/<set>/`.
pub fn window_title(project: &str, file: Option<&std::path::Path>) -> String {
    let Some(file) = file else {
        return format!("{project} – Archipel IDE");
    };
    let parts: Vec<String> = file
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    let name = parts.last().cloned().unwrap_or_default();
    let module = match parts.iter().position(|part| part == "src") {
        Some(src) if src + 2 < parts.len() => {
            let owner = if src == 0 {
                project.to_owned()
            } else {
                parts[src - 1].clone()
            };
            format!("{owner}.{}", parts[src + 1])
        }
        _ => project.to_owned(),
    };
    format!("{project} – {name} [{module}]")
}

pub fn status_bar(app: &mut IdeApp, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("status_bar")
        .exact_height(STATUS_BAR_HEIGHT)
        .show_separator_line(false)
        .frame(chrome_frame(ctx).inner_margin(egui::Margin::symmetric(8, 0)))
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let font = egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0);
                // Navigation bar breadcrumbs: project › directories › file, then
                // the declarations around the caret (class › method).
                let mut crumbs = vec![app.project_name()];
                let mut code_crumbs = Vec::new();
                let mut file_replaced = false;
                if let Some(document) = app
                    .state
                    .active_document
                    .and_then(|index| app.state.documents.get(index))
                {
                    crumbs.extend(
                        document
                            .path
                            .components()
                            .map(|part| part.as_os_str().to_string_lossy().into_owned()),
                    );
                    code_crumbs = crate::structure_view::breadcrumbs(ctx, document);
                    // Like IntelliJ, a top-level declaration named after the file
                    // stands in for the file itself (`App.java` → `App`).
                    let stem = document.path.file_stem().map(|stem| stem.to_string_lossy());
                    if code_crumbs.first().map(|(_, name)| name.as_str()) == stem.as_deref() {
                        crumbs.pop();
                        file_replaced = true;
                    }
                }
                let last = crumbs.len().saturating_sub(1);
                let file_icon = app
                    .state
                    .active_document
                    .and_then(|index| app.state.documents.get(index))
                    .map(|document| app.file_icon(&document.path));
                for (index, crumb) in crumbs.iter().enumerate() {
                    let icon = if index == 0 {
                        Some((Icon::Project, *theme::TEXT_SECONDARY))
                    } else if index == last && !file_replaced {
                        Some((
                            file_icon.unwrap_or_else(|| Icon::for_file(crumb)),
                            *icons::ICON,
                        ))
                    } else {
                        None
                    };
                    let color = if index == last && !file_replaced {
                        *theme::TEXT
                    } else {
                        *theme::TEXT_SECONDARY
                    };
                    widgets::flat_widget(ui, icon, crumb, font.clone(), color, false, 22.0);
                    if index != last {
                        icons::show(ui, Icon::ChevronRight, 12.0, *theme::TEXT_INFO);
                    }
                }
                for (kind, name) in code_crumbs {
                    icons::show(ui, Icon::ChevronRight, 12.0, theme::TEXT_INFO);
                    let (rect, _) = ui.allocate_exact_size(vec2(20.0, 22.0), Sense::hover());
                    icons::paint(
                        ui.painter(),
                        Rect::from_center_size(rect.center(), vec2(14.0, 14.0)),
                        Icon::Kit(crate::structure_view::icon(kind)),
                        theme::ICON,
                    );
                    widgets::flat_widget(ui, None, &name, font.clone(), theme::TEXT, false, 22.0);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    crate::plugin_ui::status_widgets(app, ui, &font, None);
                    if app.state.project.is_some() {
                        crate::ai_chat::status_widget(app, ui, font.clone());
                    }
                    let secondary = theme::TEXT_SECONDARY;
                    crate::agents_ui::status_widget(app, ui, font.clone());
                    crate::lsp_status::status_widget(app, ui, font.clone());
                    crate::index_status::status_widget(app, ui, font.clone());
                    crate::toolchain_status::status_widget(app, ui, font.clone());
                    crate::external_status::status_widget(app, ui, font.clone());
                    let active = app
                        .state
                        .active_document
                        .and_then(|index| app.state.documents.get(index));
                    if let Some(document) = active {
                        // Bounded detection: never copy the whole buffer per frame.
                        let indent = archipel_ide_text::format::detect_indent(&document.text)
                            .unwrap_or(archipel_ide_text::format::Indent::Spaces(4));
                        let separator =
                            archipel_ide_text::format::detect_line_separator(&document.text);
                        let caret = document.selections.primary().head;
                        let (line, column) =
                            document.text.char_to_line_column(caret).unwrap_or((0, 0));
                        widgets::flat_widget(
                            ui,
                            None,
                            &indent.label(),
                            font.clone(),
                            secondary,
                            false,
                            22.0,
                        );
                        widgets::flat_widget(
                            ui,
                            None,
                            "UTF-8",
                            font.clone(),
                            secondary,
                            false,
                            22.0,
                        );
                        widgets::flat_widget(
                            ui,
                            None,
                            separator.label(),
                            font.clone(),
                            secondary,
                            false,
                            22.0,
                        )
                        .on_hover_text("Line separator");
                        widgets::flat_widget(
                            ui,
                            None,
                            &crate::editor_keys::caret_position_label(
                                &document.text,
                                &document.selections,
                                (line, column),
                            ),
                            font.clone(),
                            secondary,
                            false,
                            22.0,
                        );
                        if is_read_only(ctx, &app.state.absolute_path(&document.path)) {
                            widgets::flat_widget(
                                ui,
                                None,
                                "Read-only",
                                font.clone(),
                                *theme::WARNING,
                                false,
                                22.0,
                            )
                            .on_hover_text("File is read-only");
                        }
                        if document.selections.column_mode() {
                            widgets::flat_widget(
                                ui,
                                None,
                                "Column",
                                font.clone(),
                                secondary,
                                false,
                                22.0,
                            );
                        }
                    }
                    let update_state = app.state.update_state();
                    if let archipel_ide_core::UpdateState::Available { version, .. }
                    | archipel_ide_core::UpdateState::Installed(version) = &update_state
                    {
                        let label =
                            if matches!(update_state, archipel_ide_core::UpdateState::Installed(_))
                            {
                                "Restart to update".to_owned()
                            } else {
                                format!("Update {version}")
                            };
                        if widgets::flat_widget(
                            ui,
                            Some((Icon::Refresh, *theme::MODIFIED)),
                            &label,
                            font.clone(),
                            *theme::MODIFIED,
                            false,
                            22.0,
                        )
                        .clicked()
                        {
                            app.state.notifications().show_update_balloon();
                        }
                    }
                    if app.state.execution().is_running() || app.state.search_running() {
                        widgets::busy_spinner(ui, 12.0, *theme::ACCENT);
                        ui.add_space(6.0);
                    }
                    if !app.state.status.is_empty() {
                        let status = first_line_truncated(&app.state.status, 70);
                        // Whatever room the right-hand widgets left, minus a gap
                        // to the breadcrumbs, so a long message is elided instead
                        // of painting over the navigation bar.
                        let max_width = (ui.available_width() - 24.0).max(40.0);
                        let response = widgets::flat_widget_elided(
                            ui,
                            &status,
                            font.clone(),
                            *theme::TEXT_INFO,
                            max_width,
                            22.0,
                        );
                        let response = response.on_hover_text(app.state.status.clone());
                        if response.clicked()
                            && app.state.tool_windows.visible(Anchor::Bottom).is_none()
                        {
                            app.dispatch("toolwindow.toggle", json!({"id": "build"}));
                        }
                    }
                });
            });
        });
}

#[cfg(test)]
mod tests {
    #[test]
    fn window_title_names_file_and_module() {
        use std::path::Path;
        assert_eq!(super::window_title("p", None), "p – Archipel IDE");
        assert_eq!(
            super::window_title("java-gradle", Some(Path::new("src/main/java/App.java"))),
            "java-gradle – App.java [java-gradle.main]"
        );
        assert_eq!(
            super::window_title("p", Some(Path::new("app/src/test/java/T.java"))),
            "p – T.java [app.test]"
        );
        assert_eq!(
            super::window_title("p", Some(Path::new("README.md"))),
            "p – README.md [p]"
        );
    }

    #[test]
    fn only_macos_reserves_room_for_the_traffic_lights() {
        assert_eq!(toolbar_leading_inset(true), MAC_TRAFFIC_LIGHT_INSET);
        assert_eq!(toolbar_leading_inset(false), 0.0);
    }
    use super::*;

    #[test]
    fn truncates_status_messages() {
        assert_eq!(first_line_truncated("short", 10), "short");
        assert_eq!(first_line_truncated("abcdefghijkl", 5), "abcd…");
        assert_eq!(first_line_truncated("one\ntwo", 10), "one");
    }
}

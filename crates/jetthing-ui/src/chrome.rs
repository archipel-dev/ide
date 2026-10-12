//! Workbench chrome on archipel-kit: the integrated main toolbar (`WindowToolbar` with the
//! project, VCS and run widgets), the tool window stripes (`ToolStripe`) and the status bar
//! (`Breadcrumbs`, `StatusBarItem`), laid out like Android Studio's Islands UI.

use crate::icons::Icon;
use crate::widgets::{self, chrome_frame};
use crate::{IdeApp, keys, theme};
use archipel_kit::behaviour::WidgetExt;
use archipel_kit::components::{
    Avatar, Breadcrumbs, IconButton, MenuHeader, MenuItem, MenuSeparator, Spinner, StatusBarItem,
    StripeButton, ToolStripe, ToolbarButton, Tooltip, WindowChrome, WindowToolbar,
};
use archipel_kit::parts::Slot;
use archipel_kit::{Glyph, Tone, tokens};
use eframe::egui::{self, Rect, Sense, vec2};
use jetthing_core::{Anchor, ToolWindow};
use serde_json::{Value, json};

/// Height of the status bar on the frame (Android Studio: 30 pt).
pub const STATUS_BAR_HEIGHT: f32 = tokens::size::STATUS_BAR_HEIGHT;
/// Width of a tool window stripe including the gap to the islands.
pub const STRIPE_WIDTH: f32 = tokens::stripe::WIDTH;

/// How the main window meets the native chrome: on macOS the toolbar sits in the
/// transparent title area beside the traffic lights; elsewhere the window is undecorated and
/// the toolbar draws minimize, maximize and close (`custom_title_bar`).
pub fn window_chrome() -> WindowChrome {
    if cfg!(target_os = "macos") {
        WindowChrome::Integrated
    } else {
        WindowChrome::Borderless
    }
}

/// Whether the window draws its own title bar (no OS decorations).
pub fn custom_title_bar() -> bool {
    !cfg!(target_os = "macos")
}

/// The project's color for the toolbar glow, when "Use project colors in main toolbar" is on.
fn project_color(app: &IdeApp) -> Option<egui::Color32> {
    app.state
        .settings
        .bool(jetthing_settings::ids::PROJECT_COLORS)
        .unwrap_or(true)
        .then(|| theme::avatar_color(&app.project_name()))
}

/// "Action (Shortcut)" for a tooltip, or just the action.
fn with_shortcut(action: &str, shortcut: Option<String>) -> Tooltip {
    let tooltip = Tooltip::new(action);
    match shortcut {
        Some(shortcut) if !shortcut.is_empty() => tooltip.shortcut(shortcut),
        _ => tooltip,
    }
}

/// A toolbar widget that opens an egui menu below it; it stays filled while the menu is
/// open (Android Studio's project, VCS and run configuration widgets).
pub(crate) fn toolbar_menu(
    ui: &mut egui::Ui,
    button: ToolbarButton<'_>,
    tooltip: Option<Tooltip>,
    menu: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    // The button's id is the next auto id; its menu's id derives from it.
    let popup = ui.next_auto_id().with("popup");
    let open = egui::Popup::is_id_open(ui.ctx(), popup);
    let response = button.dropdown(true).open(open).show(ui);
    if let Some(tooltip) = tooltip
        && !open
    {
        tooltip.show_for(&response);
    }
    egui::Popup::menu(&response)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(menu);
    response
}

pub fn main_toolbar(app: &mut IdeApp, ctx: &egui::Context) {
    let mut toolbar = WindowToolbar::new("main_toolbar").chrome(window_chrome());
    if let Some(color) = project_color(app) {
        toolbar = toolbar.project_color(color);
    }
    toolbar.show(ctx, |ui| {
        if !crate::main_menu::show(app, ui) {
            project_widget(app, ui);
            vcs_widget(app, ui);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = tokens::size::TOOLBAR_GAP;
            settings_widget(app, ui);
            let search = IconButton::new(Glyph::Search, "Search Everywhere")
                .large(true)
                .with(with_shortcut(
                    "Search Everywhere",
                    app.action_shortcut("SearchEverywhere"),
                ));
            if ui.add(search).clicked() {
                app.dispatch("navigate.searchEverywhere", Value::Null);
            }
            ui.add_space(tokens::toolbar::GAP);
            run_widget(app, ui);
            crate::device_manager::device_selector(app, ui);
        });
    });
}

/// The project widget: the project's avatar tile, its name and a menu of project actions.
fn project_widget(app: &mut IdeApp, ui: &mut egui::Ui) {
    let name = app.project_name();
    let root = app
        .state
        .project
        .as_ref()
        .map(|project| project.root().display().to_string());
    let tile_name = name.clone();
    let button =
        ToolbarButton::new(name).leading(Slot::ui_width(tokens::toolbar::ICON, move |ui| {
            ui.add(Avatar::new(tile_name).size(tokens::toolbar::ICON));
        }));
    let tooltip = root.clone().map(Tooltip::new);
    let active_plugins = app
        .state
        .plugin_host
        .registry
        .entries()
        .iter()
        .filter(|entry| entry.is_active())
        .count();
    toolbar_menu(ui, button, tooltip, |ui| {
        ui.set_min_width(260.0);
        if let Some(root) = &root {
            ui.add(MenuHeader::new(root.clone()));
            ui.add(MenuSeparator);
        }
        if ui.add(MenuItem::new("Open Project…")).clicked() {
            app.open_project_dialog();
            ui.close();
        }
        if ui.add(MenuItem::new("New Project…")).clicked() {
            app.new_project_dialog();
            ui.close();
        }
        ui.add(MenuSeparator);
        let plugins = widgets::pluralize(active_plugins, "plugin active", "plugins active");
        if ui
            .add(MenuItem::new(format!("Refresh Plugins ({plugins})")))
            .clicked()
        {
            app.dispatch("plugin.refresh", Value::Null);
            ui.close();
        }
    });
}

fn vcs_widget(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(git) = app.state.git.as_ref() else {
        return;
    };
    let button = ToolbarButton::new(git.branch.clone()).icon(Glyph::Branch);
    toolbar_menu(ui, button, Some(Tooltip::new("Git Branch")), |ui| {
        crate::branches_popup::menu(app, ui)
    });
}

/// The settings gear at the right end of the toolbar and its menu.
fn settings_widget(app: &mut IdeApp, ui: &mut egui::Ui) {
    let popup = ui.next_auto_id().with("popup");
    let open = egui::Popup::is_id_open(ui.ctx(), popup);
    let button = IconButton::new(Glyph::Settings, "IDE and Project Settings")
        .large(true)
        .selected(open);
    let response = ui.add(button);
    if !open {
        Tooltip::new("IDE and Project Settings").show_for(&response);
    }
    egui::Popup::menu(&response)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(220.0);
            let mut settings = MenuItem::new("Settings…");
            if let Some(shortcut) = app.action_shortcut("ShowSettings") {
                settings = settings.shortcut(shortcut);
            }
            if ui.add(settings).clicked() {
                app.dispatch("settings.open", Value::Null);
                ui.close();
            }
            if ui.add(MenuItem::new("Plugins…")).clicked() {
                app.dispatch("plugin.manager.open", Value::Null);
                ui.close();
            }
        });
}

fn run_widget(app: &mut IdeApp, ui: &mut egui::Ui) {
    let running = app.state.runner.is_running();
    // Right-to-left layout: rightmost item first (Stop, Debug, Run, configuration, Build).
    if running {
        let stop = IconButton::new(Glyph::Stop, "Stop")
            .large(true)
            .tone(Tone::Stop)
            .with(with_shortcut("Stop", app.action_shortcut("Stop")));
        if ui.add(stop).clicked() {
            app.dispatch("run.cancel", Value::Null);
        }
    }
    let detected = app.state.detected_build.clone();
    if app.state.is_debugging() {
        let stop = IconButton::new(Glyph::Stop, "Stop Debugging")
            .large(true)
            .tone(Tone::Stop)
            .with(Tooltip::new("Stop Debugging"));
        if ui.add(stop).clicked() {
            app.dispatch("debug.stop", Value::Null);
        }
    } else {
        let can_debug = app.commands.is_enabled("debug.start", &app.state);
        let debug = IconButton::new(Glyph::Debug, "Debug")
            .large(true)
            .tone(Tone::Run)
            .enabled(can_debug)
            .with(with_shortcut("Debug", Some(keys::display("⌃D"))));
        if ui.add(debug).clicked() {
            app.dispatch("debug.start", Value::Null);
        }
    }
    // Android apps have no `gradlew run`; Run deploys 'app' to the selected device.
    let android_run = app.state.android.project.is_some();
    let run_command = if android_run {
        "android.app.deploy"
    } else {
        "run.project"
    };
    let can_run = app.commands.is_enabled(run_command, &app.state);
    let run_shortcut = app.action_shortcut("Run");
    let run_tip = match &detected {
        _ if android_run => with_shortcut("Run 'app'", run_shortcut),
        _ if app.state.run_configurations.selected().is_some() => with_shortcut(
            &format!("Run '{}'", app.state.run_widget_label().unwrap_or_default()),
            run_shortcut,
        ),
        Some(build) if build.run_unavailable.is_some() => Tooltip::new(format!(
            "Run unavailable: {}",
            build.run_unavailable.as_deref().unwrap_or_default()
        )),
        None if app.state.project.is_some() => Tooltip::new("No supported build system detected"),
        _ => with_shortcut("Run", run_shortcut),
    };
    let run = IconButton::new(Glyph::Run, "Run")
        .large(true)
        .tone(Tone::Run)
        .enabled(can_run)
        .with(run_tip);
    if ui.add(run).clicked() {
        app.dispatch(run_command, Value::Null);
    }
    if let Some(label) = app.state.run_widget_label() {
        run_configurations(app, ui, label, detected.as_ref());
    }
    let can_build = app.commands.is_enabled("build.project", &app.state);
    let build_tip = match &detected {
        None if app.state.project.is_some() => Tooltip::new("No supported build system detected"),
        _ => with_shortcut("Build Project", app.action_shortcut("CompileDirty")),
    };
    let build = IconButton::new(Glyph::Build, "Build Project")
        .large(true)
        .enabled(can_build)
        .with(build_tip);
    if ui.add(build).clicked() {
        app.dispatch("build.project", Value::Null);
    }
}

/// The run configuration widget and its menu: Current File, the configurations with Run,
/// Debug and Edit submenus, and Edit Configurations….
fn run_configurations(
    app: &mut IdeApp,
    ui: &mut egui::Ui,
    label: String,
    detected: Option<&jetthing_core::DetectedBuild>,
) {
    let selected = app
        .state
        .run_configurations
        .selected()
        .map(|config| config.name.clone());
    let names: Vec<String> = app
        .state
        .run_configurations
        .configurations()
        .iter()
        .map(|config| config.name.clone())
        .collect();
    let idle = !app.state.runner.is_running();
    let unavailable = detected
        .filter(|_| selected.is_none())
        .and_then(|build| build.run_unavailable.clone());
    let can_edit = app.commands.is_enabled("runConfig.edit", &app.state);
    let mut action = None;
    toolbar_menu(
        ui,
        ToolbarButton::new(label),
        Some(Tooltip::new("Run / Debug Configurations")),
        |ui| {
            ui.set_min_width(240.0);
            // `Current File` runs the entry point or test at the caret.
            if ui
                .add(MenuItem::new("Current File").checked(selected.is_none()))
                .clicked()
            {
                action = Some(RunAction::CurrentFile);
            }
            for name in &names {
                let current = selected.as_deref() == Some(name.as_str());
                let item = MenuItem::new(name.clone()).checked(current);
                let item = if idle {
                    item.submenu(|ui| {
                        if ui.add(MenuItem::new("Run").icon(Glyph::Run)).clicked() {
                            action = Some(RunAction::Run(name.clone()));
                        }
                        if ui.add(MenuItem::new("Debug").icon(Glyph::Debug)).clicked() {
                            action = Some(RunAction::Debug(name.clone()));
                        }
                        if ui.add(MenuItem::new("Edit…").icon(Glyph::Edit)).clicked() {
                            action = Some(RunAction::Edit(name.clone()));
                        }
                    })
                } else {
                    item
                };
                if ui.add(item).clicked() && action.is_none() {
                    action = Some(RunAction::Select(name.clone()));
                }
            }
            if let Some(reason) = &unavailable {
                ui.add(MenuItem::new(format!("Run unavailable: {reason}")).enabled(false));
            }
            ui.add(MenuSeparator);
            if ui
                .add(MenuItem::new("Edit Configurations…").enabled(can_edit))
                .clicked()
            {
                action = Some(RunAction::EditAll);
            }
            if action.is_some() {
                ui.close();
            }
        },
    );
    match action {
        None => {}
        Some(RunAction::CurrentFile) => app.dispatch("runConfig.selectCurrentFile", Value::Null),
        Some(RunAction::Select(name)) => app.dispatch("runConfig.select", json!({"name": name})),
        Some(RunAction::Run(name)) => app.dispatch("runConfig.run", json!({"name": name})),
        Some(RunAction::Debug(name)) => {
            app.dispatch("runConfig.select", json!({"name": name}));
            app.dispatch("debug.start", Value::Null);
        }
        Some(RunAction::Edit(name)) => app.dispatch("runConfig.edit", json!({"name": name})),
        Some(RunAction::EditAll) => app.dispatch("runConfig.edit", Value::Null),
    }
}

enum RunAction {
    CurrentFile,
    Select(String),
    Run(String),
    Debug(String),
    Edit(String),
    EditAll,
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

/// Remembers where the island of `anchor`'s tool window is this frame, so the stripe can
/// tell next frame whether the keyboard focus is inside it (the active tool window).
pub(crate) fn remember_island(ctx: &egui::Context, anchor: Anchor, rect: Rect) {
    ctx.data_mut(|data| data.insert_temp(island_key(anchor), rect));
}

fn island_key(anchor: Anchor) -> egui::Id {
    egui::Id::new(("tool_window_island", anchor))
}

/// Whether the keyboard focus is inside the island of `anchor`'s tool window.
fn island_has_focus(ctx: &egui::Context, anchor: Anchor) -> bool {
    let Some(island) = ctx.data(|data| data.get_temp::<Rect>(island_key(anchor))) else {
        return false;
    };
    ctx.memory(|memory| memory.focused())
        .and_then(|id| ctx.read_response(id))
        .is_some_and(|response| island.contains_rect(response.rect))
}

/// The stripe button of a built-in tool window: filled while it is open, accent-filled
/// while it has the focus; its tooltip names it with its shortcut.
fn stripe_button(
    app: &IdeApp,
    ctx: &egui::Context,
    window: ToolWindow,
    icon: Icon,
) -> StripeButton {
    let open = app.state.tool_windows.is_visible(window);
    let button = StripeButton::new(icon, window.title())
        .open(open)
        .active(open && island_has_focus(ctx, window.anchor()));
    match window.shortcut() {
        "" => button,
        shortcut => button.shortcut(keys::display(shortcut)),
    }
}

/// Paints the project color's glow on the window background, under the toolbar's glow:
/// it shows through the stripes and the gaps between the islands, as in Android Studio.
pub fn project_glow(app: &IdeApp, ctx: &egui::Context) {
    if let Some(color) = project_color(app) {
        let painter = ctx.layer_painter(egui::LayerId::background());
        archipel_kit::parts::project_glow(&painter, ctx.input(|i| i.viewport_rect()), color);
    }
}

/// The left stripe: the left tool windows at the top, the bottom ones at the bottom.
pub fn left_stripe(app: &mut IdeApp, ctx: &egui::Context) {
    let mut top = vec![
        (ToolWindow::Project, Icon::Project),
        (ToolWindow::Commit, Icon::Commit),
        (ToolWindow::Structure, Icon::Structure),
    ];
    if app.state.android.project.is_some() {
        top.push((ToolWindow::ResourceManager, Icon::Variants));
        top.push((ToolWindow::BuildVariants, Icon::Variants));
    }
    let mut bottom = vec![
        (ToolWindow::Find, Icon::Find),
        (ToolWindow::Debug, Icon::Debug),
        (ToolWindow::Build, Icon::Build),
        (ToolWindow::Terminal, Icon::Terminal),
        (ToolWindow::Problems, Icon::Problems),
    ];
    if app.state.android.project.is_some() {
        bottom.push((ToolWindow::Logcat, Icon::Logcat));
    }
    if app.state.hierarchy.current.is_some() {
        bottom.push((ToolWindow::Hierarchy, Icon::Structure));
    }
    bottom.push((ToolWindow::Git, Icon::Branch));
    // Top to bottom, as on the reference's stripe: Git sits lowest.
    bottom.reverse();
    egui::SidePanel::left("left_stripe")
        .exact_width(STRIPE_WIDTH)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::new())
        .show(ctx, |ui| {
            let mut stripe = ToolStripe::new("left_stripe");
            for (window, icon) in &top {
                stripe = stripe.top(stripe_button(app, ctx, *window, *icon));
            }
            for (window, icon) in &bottom {
                stripe = stripe.bottom(stripe_button(app, ctx, *window, *icon));
            }
            let windows: Vec<ToolWindow> = top.iter().chain(&bottom).map(|(w, _)| *w).collect();
            if let Some(index) = stripe.show(ui) {
                app.toggle_tool_window(windows[index]);
            }
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
    windows.push((ToolWindow::AiChat, Icon::Ai));
    windows
}

/// A plugin's tool window on the right stripe.
pub(crate) struct PluginStripeEntry {
    pub plugin: String,
    pub id: String,
    pub title: String,
    /// A kit glyph name (`"database"`, `"globe"`, …); other text falls back to the plugin
    /// glyph.
    pub icon: String,
}

/// The right stripe: the project's right-anchored tool windows (Gradle, Bazel, Device
/// Manager, Running Devices), AI Chat, and the plugins' tool windows.
pub(crate) fn right_stripe(app: &mut IdeApp, ctx: &egui::Context, plugins: &[PluginStripeEntry]) {
    let built_in = right_stripe_windows(app);
    egui::SidePanel::right("right_stripe")
        .exact_width(STRIPE_WIDTH)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::new())
        .show(ctx, |ui| {
            let mut stripe = ToolStripe::new("right_stripe");
            for (window, icon) in &built_in {
                stripe = stripe.top(stripe_button(app, ctx, *window, *icon));
            }
            for entry in plugins {
                let open = app.state.plugin_host.tool_window.as_ref()
                    == Some(&(entry.plugin.clone(), entry.id.clone()));
                let glyph = Glyph::ALL
                    .iter()
                    .copied()
                    .find(|glyph| glyph.name() == entry.icon)
                    .unwrap_or(Glyph::Plugin);
                stripe = stripe.top(
                    StripeButton::new(glyph, entry.title.clone())
                        .open(open)
                        .active(open && island_has_focus(ctx, Anchor::Right)),
                );
            }
            match stripe.show(ui) {
                Some(index) if index < built_in.len() => {
                    app.toggle_tool_window(built_in[index].0);
                }
                Some(index) => {
                    let entry = &plugins[index - built_in.len()];
                    app.dispatch(
                        "plugin.toolwindow.toggle",
                        json!({"plugin": entry.plugin, "id": entry.id}),
                    );
                }
                None => {}
            }
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
        return format!("{project} – Jetthing");
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

/// The status bar on the frame: the navigation path at the left (project › directories ›
/// file › declarations around the caret), status text and widgets at the right.
pub fn status_bar(app: &mut IdeApp, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("status_bar")
        .exact_height(STATUS_BAR_HEIGHT)
        .show_separator_line(false)
        .frame(chrome_frame(ctx).inner_margin(egui::Margin::symmetric(
            tokens::size::STATUS_BAR_INSET as i8,
            0,
        )))
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                navigation_path(app, ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_widgets(app, ui);
                });
            });
        });
}

/// Navigation bar breadcrumbs: project › directories › file, then the declarations around
/// the caret (class › method), with a top-level declaration named after the file standing
/// in for the file itself (`App.java` → `App`), as in the reference.
fn navigation_path(app: &IdeApp, ui: &mut egui::Ui) {
    let mut crumbs: Vec<(String, Option<archipel_kit::Icon>)> =
        vec![(app.project_name(), Some(Glyph::Project.into()))];
    if let Some(document) = app
        .state
        .active_document
        .and_then(|index| app.state.documents.get(index))
    {
        let parts: Vec<String> = document
            .path
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect();
        let code = crate::structure_view::breadcrumbs(ui.ctx(), document);
        let stem = document.path.file_stem().map(|stem| stem.to_string_lossy());
        let file_replaced = code.first().map(|(_, name)| name.as_str()) == stem.as_deref();
        let last = parts.len().saturating_sub(1);
        for (index, part) in parts.into_iter().enumerate() {
            if index == last {
                if !file_replaced {
                    crumbs.push((part, Some(app.file_icon(&document.path).kit())));
                }
            } else {
                crumbs.push((part, None));
            }
        }
        for (kind, name) in code {
            crumbs.push((name, Some(crate::structure_view::icon(kind).into())));
        }
    }
    let mut path = Breadcrumbs::new();
    for (label, icon) in crumbs {
        path = path.crumb(label, icon);
    }
    let _ = path.show(ui);
}

/// The right-hand status bar widgets, laid out right to left.
fn status_widgets(app: &mut IdeApp, ui: &mut egui::Ui) {
    crate::plugin_ui::status_widgets(app, ui, None);
    if app.state.project.is_some() {
        crate::ai_chat::status_widget(app, ui);
    }
    crate::lsp_status::status_widget(app, ui);
    crate::toolchain_status::status_widget(app, ui);
    if let Some(document) = app
        .state
        .active_document
        .and_then(|index| app.state.documents.get(index))
    {
        // Bounded detection: never copy the whole buffer per frame.
        let indent = jetthing_text::format::detect_indent(&document.text)
            .unwrap_or(jetthing_text::format::Indent::Spaces(4));
        let separator = jetthing_text::format::detect_line_separator(&document.text);
        let caret = document.selections.primary().head;
        let (line, column) = document.text.char_to_line_column(caret).unwrap_or((0, 0));
        let caret_label = crate::editor_keys::caret_position_label(
            &document.text,
            &document.selections,
            (line, column),
        );
        let read_only = is_read_only(ui.ctx(), &app.state.absolute_path(&document.path));
        let column_mode = document.selections.column_mode();
        ui.add(StatusBarItem::new(indent.label()).with(Tooltip::new("Indentation")));
        ui.add(StatusBarItem::new("UTF-8").with(Tooltip::new("File encoding")));
        ui.add(StatusBarItem::new(separator.label()).with(Tooltip::new("Line separator")));
        ui.add(StatusBarItem::new(caret_label).with(Tooltip::new("Caret position")));
        if read_only {
            ui.add(
                StatusBarItem::new("Read-only")
                    .tone(Tone::Warning)
                    .with(Tooltip::new("File is read-only")),
            );
        }
        if column_mode {
            ui.add(StatusBarItem::new("Column").with(Tooltip::new("Column selection mode")));
        }
    }
    if let jetthing_core::UpdateState::Available { version, .. }
    | jetthing_core::UpdateState::Installed(version) = &app.state.update_state
    {
        let label = if matches!(
            app.state.update_state,
            jetthing_core::UpdateState::Installed(_)
        ) {
            "Restart to update".to_owned()
        } else {
            format!("Update {version}")
        };
        if ui
            .add(
                StatusBarItem::new(label)
                    .icon(Glyph::Refresh)
                    .tone(Tone::Modified),
            )
            .clicked()
        {
            app.state.update_notification = true;
        }
    }
    if app.state.runner.is_running() || app.state.search.is_running() {
        ui.add(Spinner::new().size(tokens::size::ICON).label("Working"));
        ui.add_space(tokens::size::ICON_TEXT_GAP);
    }
    if !app.state.status.is_empty() {
        let status = first_line_truncated(&app.state.status, 70);
        // Whatever room the right-hand widgets left, minus a gap to the breadcrumbs, so a
        // long message is elided instead of painting over the navigation bar.
        let max_width = (ui.available_width() - 24.0).max(40.0);
        let response = ui.add(
            StatusBarItem::new(status)
                .max_width(max_width)
                .with(Tooltip::new(app.state.status.clone())),
        );
        if response.clicked() && app.state.tool_windows.visible(Anchor::Bottom).is_none() {
            app.dispatch("toolwindow.toggle", json!({"id": "build"}));
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn window_title_names_file_and_module() {
        use std::path::Path;
        assert_eq!(super::window_title("p", None), "p – Jetthing");
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

    use super::*;

    #[test]
    fn truncates_status_messages() {
        assert_eq!(first_line_truncated("short", 10), "short");
        assert_eq!(first_line_truncated("abcdefghijkl", 5), "abcd…");
        assert_eq!(first_line_truncated("one\ntwo", 10), "one");
    }
}

//! The main toolbar (row 0 of the terminal, the top bar of the window) and the popups of its
//! widgets (TUI-GUI-020..026): hamburger, project widget, VCS widget, Code | Agents switch,
//! device selector, run configuration widget, Run, Build, Debug (Stop Debugging while
//! debugging), Stop (while a task runs), Search Everywhere and Settings.

use crate::cx::ViewCx;
use archipel_ide_core::agents::WorkbenchMode;
use archipel_kit_model::{
    Avatar, Glyph, MenuEntry, MenuItem, Shortcut, Tone, Toolbar, ToolbarButton, ToolbarDropdown,
    ToolbarItem, Tooltip,
};

/// Ids of the toolbar's widgets.
pub mod ids {
    /// The hamburger.
    pub const MAIN_MENU: &str = "main_menu";
    /// The project widget.
    pub const PROJECT: &str = "project";
    /// The VCS widget.
    pub const VCS: &str = "vcs";
    /// The Code | Agents switch.
    pub const MODE: &str = "mode";
    /// The Android device selector.
    pub const DEVICE: &str = "device";
    /// The run configuration widget.
    pub const RUN_CONFIG: &str = "run_config";
    /// Run.
    pub const RUN: &str = "run";
    /// Build.
    pub const BUILD: &str = "build";
    /// Debug or Stop Debugging.
    pub const DEBUG: &str = "debug";
    /// Stop.
    pub const STOP: &str = "stop";
    /// Search Everywhere.
    pub const SEARCH: &str = "search";
    /// Settings.
    pub const SETTINGS: &str = "settings";
}

/// The id of a menu row that runs command `command` with `params`.
pub fn command_row(command: &str, params: &serde_json::Value) -> String {
    format!("cmd:{command}\u{1f}{params}")
}

fn tip(cx: &ViewCx, title: impl Into<String>, action: Option<&str>) -> Tooltip {
    Tooltip::new(title).shortcut_opt(action.and_then(|a| cx.shortcut(a)))
}

/// The label of the Android device selector and whether a device is selected.
pub fn device_label(cx: &ViewCx) -> (String, bool) {
    let tooling = cx.state.android_tooling();
    let hub = tooling.devices();
    match hub.selected_device() {
        Some(device) => (device.display_name(), true),
        None if hub.devices.is_empty() => ("No Devices".into(), false),
        None => ("No Device Selected".into(), false),
    }
}

/// The text of the Run button's tooltip.
fn run_tip(cx: &ViewCx, android: bool) -> String {
    let state = cx.state;
    let detected = state.detected_build();
    let run = |cx: &ViewCx| {
        cx.shortcut("Run")
            .map(|s| s.label(cx.caps.notation()))
            .unwrap_or_default()
    };
    match detected {
        _ if android => format!("Run 'app' ({})", run(cx)),
        _ if state.execution().configurations().selected().is_some() => format!(
            "Run '{}' ({})",
            state.run_widget_label().unwrap_or_default(),
            run(cx)
        ),
        Some(build) if build.run_unavailable.is_some() => format!(
            "Run unavailable: {}",
            build.run_unavailable.as_deref().unwrap_or_default()
        ),
        None if state.has_project() => "No supported build system detected".to_owned(),
        _ => format!("Run ({})", run(cx)),
    }
}

/// The main toolbar for the state. The hamburger is always first.
pub fn main_toolbar(cx: &ViewCx) -> Toolbar {
    let state = cx.state;
    let mut bar = Toolbar::new("main");
    bar.leading.push(ToolbarItem::Button(
        ToolbarButton::new(ids::MAIN_MENU, Glyph::Menu, "Main Menu").tooltip(tip(
            cx,
            "Main Menu",
            Some("MainMenu"),
        )),
    ));
    // Project widget.
    let name = project_name(cx);
    let root = state
        .project
        .as_ref()
        .map(|project| project.root().display().to_string());
    let mut project = ToolbarDropdown::new(ids::PROJECT, name.clone())
        .avatar(Avatar::of(&name))
        .max_chars(14);
    project.shrink_order = 2;
    project.tooltip = root.map(Tooltip::new);
    bar.leading.push(ToolbarItem::Dropdown(project));
    // VCS widget.
    if let Some(git) = state.git_status() {
        let mut vcs = ToolbarDropdown::new(ids::VCS, git.branch.clone())
            .icon(Glyph::Branch)
            .tooltip(Tooltip::new("Git Branch"))
            .max_chars(12);
        vcs.tone = Tone::Strong;
        vcs.shrink_order = 1;
        bar.leading.push(ToolbarItem::Dropdown(vcs));
    }
    bar.leading.push(ToolbarItem::Segmented {
        id: ids::MODE.into(),
        options: vec![
            WorkbenchMode::Code.label().to_owned(),
            WorkbenchMode::Agents.label().to_owned(),
        ],
        selected: usize::from(state.workbench_mode() == WorkbenchMode::Agents),
    });

    // Right cluster, left to right.
    let android = state.is_android_project();
    if android {
        let (label, active) = device_label(cx);
        let mut device = ToolbarDropdown::new(ids::DEVICE, label).icon(Glyph::Device);
        device.tone = if active { Tone::Strong } else { Tone::Info };
        bar.trailing.push(ToolbarItem::Dropdown(device));
    }
    if let Some(label) = state.run_widget_label() {
        let mut config = ToolbarDropdown::new(ids::RUN_CONFIG, label).max_chars(16);
        config.tooltip = Some(Tooltip::new("Run Configuration"));
        bar.trailing.push(ToolbarItem::Dropdown(config));
    }
    // Run.
    let run_command = if android {
        "android.app.deploy"
    } else {
        "run.project"
    };
    let can_run = cx.action_enabled(run_command);
    let mut run = ToolbarButton::new(ids::RUN, Glyph::Run, "Run")
        .tone(if can_run { Tone::Run } else { Tone::Disabled })
        .enabled(can_run);
    let run_title = run_tip(cx, android);
    run.tooltip = Some(Tooltip::new(run_title));
    bar.trailing.push(ToolbarItem::Button(run));
    // Build.
    let can_build = cx.action_enabled("build.project");
    let build_tip = match state.detected_build() {
        None if state.has_project() => Tooltip::new("No supported build system detected"),
        _ => tip(cx, "Build Project", Some("CompileDirty")),
    };
    bar.trailing.push(ToolbarItem::Button(
        ToolbarButton::new(ids::BUILD, Glyph::Build, "Build Project")
            .enabled(can_build)
            .tooltip(build_tip),
    ));
    // Debug or Stop Debugging.
    if state.is_debugging() {
        bar.trailing.push(ToolbarItem::Button(
            ToolbarButton::new(ids::DEBUG, Glyph::Stop, "Stop Debugging")
                .tone(Tone::Stop)
                .tooltip(Tooltip::new("Stop Debugging")),
        ));
    } else {
        let can_debug = cx.action_enabled("debug.start");
        bar.trailing.push(ToolbarItem::Button(
            ToolbarButton::new(ids::DEBUG, Glyph::Debug, "Debug")
                .tone(if can_debug { Tone::Run } else { Tone::Disabled })
                .enabled(can_debug)
                .tooltip(tip(cx, "Debug", Some("Debug"))),
        ));
    }
    if state.execution().is_running() {
        bar.trailing.push(ToolbarItem::Button(
            ToolbarButton::new(ids::STOP, Glyph::Stop, "Stop")
                .tone(Tone::Stop)
                .tooltip(tip(cx, "Stop", Some("Stop"))),
        ));
    }
    bar.trailing.push(ToolbarItem::Button(
        ToolbarButton::new(ids::SEARCH, Glyph::Search, "Search Everywhere").tooltip(tip(
            cx,
            "Search Everywhere",
            Some("SearchEverywhere"),
        )),
    ));
    bar.trailing.push(ToolbarItem::Button(
        ToolbarButton::new(ids::SETTINGS, Glyph::Settings, "Settings").tooltip(tip(
            cx,
            "Settings",
            Some("ShowSettings"),
        )),
    ));
    bar
}

/// The project's name for the widget.
pub fn project_name(cx: &ViewCx) -> String {
    cx.state
        .project
        .as_ref()
        .and_then(|project| {
            project
                .root()
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "No project".into())
}

/// The popup of the project widget: the root path, Open Project…, New Project…, Refresh
/// Plugins and the number of active plugins.
pub fn project_popup(cx: &ViewCx) -> Vec<MenuEntry> {
    let state = cx.state;
    let mut out = Vec::new();
    if let Some(project) = state.project.as_ref() {
        let mut root = MenuItem::new("project.root", project.root().display().to_string());
        root.enabled = false;
        out.push(MenuEntry::Item(root));
        out.push(MenuEntry::separator());
    }
    out.push(MenuEntry::Item(
        MenuItem::new("OpenFile", "Open Project…").mnemonic(0),
    ));
    out.push(MenuEntry::Item(
        MenuItem::new("NewProject", "New Project…").mnemonic(0),
    ));
    out.push(MenuEntry::Item(MenuItem::new(
        command_row("plugin.refresh", &serde_json::Value::Null),
        "Refresh Plugins",
    )));
    let active = state
        .plugin_host_service()
        .registry()
        .entries()
        .iter()
        .filter(|entry| entry.is_active())
        .count();
    let mut count = MenuItem::new(
        "project.plugins",
        crate::header::pluralize(active, "plugin active", "plugins active"),
    );
    count.enabled = false;
    out.push(MenuEntry::Item(count));
    out
}

/// The rows of the run configuration popup: `Current File`, each configuration, the detected
/// targets, the `Run unavailable` line, `Edit Configurations…`.
pub fn run_config_popup(cx: &ViewCx) -> Vec<MenuEntry> {
    let state = cx.state;
    let execution = state.execution();
    let configs = execution.configurations();
    let selected = configs.selected().map(|config| config.name.clone());
    let mut out = Vec::new();
    let mut current = MenuItem::new(
        command_row("runConfig.selectCurrentFile", &serde_json::Value::Null),
        "Current File",
    );
    current.checked = Some(selected.is_none());
    out.push(MenuEntry::Item(current));
    for config in configs.configurations() {
        let mut item = MenuItem::new(
            command_row(
                "runConfig.select",
                &serde_json::json!({ "name": config.name }),
            ),
            config.name.clone(),
        );
        item.checked = Some(selected.as_deref() == Some(config.name.as_str()));
        out.push(MenuEntry::Item(item));
    }
    // What the build system detected (Cargo, Gradle, Maven, npm, Go): Build, Test and a Run that
    // can start. A target that cannot start is greyed out with its reason; the Run one is
    // explained by the `Run unavailable` line below.
    for target in state.detected_targets() {
        match target.unavailable {
            None => out.push(MenuEntry::Item(MenuItem::new(
                command_row(target.command, &serde_json::Value::Null),
                target.label,
            ))),
            Some(reason) if target.command != "run.project" => {
                let mut item = MenuItem::new(
                    format!("detected.{}", target.command),
                    format!("{}: {reason}", target.label),
                );
                item.enabled = false;
                out.push(MenuEntry::Item(item));
            }
            Some(_) => {}
        }
    }
    if let Some(build) = state.detected_build()
        && selected.is_none()
        && let Some(reason) = &build.run_unavailable
    {
        let mut line = MenuItem::new("run.unavailable", format!("Run unavailable: {reason}"));
        line.enabled = false;
        out.push(MenuEntry::Item(line));
    }
    out.push(MenuEntry::separator());
    let mut edit = MenuItem::new(
        command_row("runConfig.edit", &serde_json::Value::Null),
        "Edit Configurations…",
    );
    edit.enabled = cx.action_enabled("runConfig.edit");
    out.push(MenuEntry::Item(edit));
    out
}

/// The popup of the VCS widget: operation in progress, Fetch, and the local and remote
/// branches, each with its actions.
pub fn vcs_popup(cx: &ViewCx) -> Vec<MenuEntry> {
    let state = cx.state;
    let Some(git) = state.git_status() else {
        return Vec::new();
    };
    let current = git.branch.clone();
    let branches = state.git_log_snapshot().branches.clone();
    let busy = state.remote_busy();
    let mut out = Vec::new();
    if let Some(operation) = state.git_pending() {
        let mut line = MenuItem::new("vcs.pending", format!("{} in progress", operation.label()));
        line.enabled = false;
        out.push(MenuEntry::Item(line));
        out.push(MenuEntry::Item(MenuItem::new(
            command_row("git.abortOperation", &serde_json::Value::Null),
            archipel_ide_core::abort_label(operation),
        )));
        out.push(MenuEntry::separator());
    }
    out.push(MenuEntry::Item(
        MenuItem::new(command_row("git.fetch", &serde_json::Value::Null), "Fetch").enabled(!busy),
    ));
    out.push(MenuEntry::Separator {
        title: Some("Local".into()),
    });
    let branch_actions = |name: &str, remote: bool| -> Vec<MenuEntry> {
        let is_current = name == current;
        let mut rows = Vec::new();
        let row = |command: &str, params: serde_json::Value, label: String| {
            MenuEntry::Item(MenuItem::new(command_row(command, &params), label))
        };
        if !is_current {
            rows.push(row(
                "git.checkout",
                serde_json::json!({ "branch": name }),
                "Checkout".into(),
            ));
        }
        if !remote && is_current {
            rows.push(row("git.pull", serde_json::Value::Null, "Update".into()));
        }
        if !is_current {
            rows.push(row(
                "git.merge",
                serde_json::json!({ "branch": name }),
                format!("Merge '{name}' into '{current}'"),
            ));
            rows.push(row(
                "git.rebase",
                serde_json::json!({ "onto": name }),
                format!("Rebase '{current}' onto '{name}'"),
            ));
        }
        if !remote && !is_current {
            rows.push(row(
                "git.deleteBranch",
                serde_json::json!({ "branch": name }),
                "Delete".into(),
            ));
        }
        rows
    };
    for branch in &branches.local {
        let mut label = branch.name.clone();
        if branch.ahead > 0 {
            label.push_str(&format!("  ↑{}", branch.ahead));
        }
        if branch.behind > 0 {
            label.push_str(&format!("  ↓{}", branch.behind));
        }
        let mut item = MenuItem::submenu(
            format!("branch:{}", branch.name),
            label,
            branch_actions(&branch.name, false),
        );
        item.icon = Some(Glyph::Branch.into());
        item.checked = (branch.name == current).then_some(true);
        out.push(MenuEntry::Item(item));
    }
    if !branches.remote.is_empty() {
        out.push(MenuEntry::Separator {
            title: Some("Remote".into()),
        });
        for name in &branches.remote {
            let mut item = MenuItem::submenu(
                format!("remote:{name}"),
                name.clone(),
                branch_actions(name, true),
            );
            item.icon = Some(Glyph::Branch.into());
            out.push(MenuEntry::Item(item));
        }
    }
    out
}

/// The shortcut text of Search Everywhere's tooltip, for tests.
pub fn search_shortcut(cx: &ViewCx) -> Option<Shortcut> {
    cx.shortcut("SearchEverywhere")
}

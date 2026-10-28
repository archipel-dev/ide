//! The tool window stripes: which buttons, in which order, with which badges (TUI-GUI-055..060).

use crate::cx::ViewCx;
use crate::icons::{activation_action, tool_window_icon};
use archipel_ide_core::ToolWindow;
use archipel_kit_model::{IconKey, StripeAnchor, StripeButton, Tone, ToolStripe, Tooltip};

/// The id of the stripe button of `window`.
pub fn button_id(window: ToolWindow) -> String {
    window.id().to_owned()
}

/// The id of the stripe button of a plugin tool window.
pub fn plugin_button_id(plugin: &str, id: &str) -> String {
    format!("plugin:{plugin}/{id}")
}

fn button(cx: &ViewCx, window: ToolWindow) -> StripeButton {
    let state = cx.state;
    let mut tip = Tooltip::new(window.title());
    if let Some(action) = activation_action(window) {
        tip = tip.shortcut_opt(cx.shortcut(action));
    }
    let mut button = StripeButton::new(button_id(window), tool_window_icon(window), tip);
    button.selected = state.tool_windows().is_visible(window);
    button.focused =
        button.selected && cx.ui.focused_window.is_none_or(|focused| focused == window);
    button.badge = match window {
        ToolWindow::Problems => {
            let counts = state.problems().counts();
            if counts.errors > 0 {
                Some(Tone::Error)
            } else if counts.warnings > 0 {
                Some(Tone::Warning)
            } else {
                None
            }
        }
        ToolWindow::Build if state.execution().is_running() => Some(Tone::Run),
        ToolWindow::Agents => {
            let counts = state.agents_manager().counts();
            if counts.needs_input > 0 {
                Some(Tone::Warning)
            } else if counts.working > 0 {
                Some(Tone::Run)
            } else {
                None
            }
        }
        _ => None,
    };
    button
}

/// The left stripe: top group Project, Commit, Structure, Agents (with sessions or visible),
/// Resource Manager and Build Variants (Android); bottom group from the bottom up Git,
/// Hierarchy (when shown), Logcat (Android), Problems, TODO, Terminal, Build, Debug, Find.
pub fn left_stripe(cx: &ViewCx) -> ToolStripe {
    let state = cx.state;
    let mut stripe = ToolStripe::new(StripeAnchor::Left);
    stripe.top.push(button(cx, ToolWindow::Project));
    stripe.top.push(button(cx, ToolWindow::Commit));
    stripe.top.push(button(cx, ToolWindow::Structure));
    if !state.agents_manager().sessions().is_empty()
        || state.tool_windows().is_visible(ToolWindow::Agents)
    {
        stripe.top.push(button(cx, ToolWindow::Agents));
    }
    let android = state.is_android_project();
    if android {
        stripe.top.push(button(cx, ToolWindow::ResourceManager));
        stripe.top.push(button(cx, ToolWindow::BuildVariants));
    }
    stripe.bottom.push(button(cx, ToolWindow::Git));
    if state.hierarchy().current.is_some() {
        stripe.bottom.push(button(cx, ToolWindow::Hierarchy));
    }
    if android {
        stripe.bottom.push(button(cx, ToolWindow::Logcat));
    }
    for window in [
        ToolWindow::Problems,
        ToolWindow::Todo,
        ToolWindow::Terminal,
        ToolWindow::Build,
        ToolWindow::Debug,
        ToolWindow::Find,
    ] {
        stripe.bottom.push(button(cx, window));
    }
    stripe
}

/// Built-in right-anchored windows offered for the open project, top to bottom.
pub fn right_stripe_windows(cx: &ViewCx) -> Vec<ToolWindow> {
    let state = cx.state;
    let visible = |window| state.tool_windows().is_visible(window);
    let mut windows = Vec::new();
    if state.is_gradle_project() || visible(ToolWindow::Gradle) {
        windows.push(ToolWindow::Gradle);
    }
    if state.is_bazel_project() || visible(ToolWindow::Bazel) {
        windows.push(ToolWindow::Bazel);
    }
    if state.is_android_project() || visible(ToolWindow::DeviceManager) {
        windows.push(ToolWindow::DeviceManager);
    }
    if state.is_android_project() || visible(ToolWindow::RunningDevices) {
        windows.push(ToolWindow::RunningDevices);
    }
    windows
}

/// Contributed plugin tool windows as (plugin, id, title, stripe text).
pub fn plugin_windows(cx: &ViewCx) -> Vec<(String, String, String, String)> {
    cx.state
        .plugin_host_service()
        .registry()
        .entries()
        .iter()
        .filter(|entry| entry.is_active())
        .flat_map(|entry| {
            entry.contributions.tool_windows.iter().map(|window| {
                let icon = window.icon_text.clone().unwrap_or_else(|| {
                    window
                        .title
                        .chars()
                        .take(1)
                        .collect::<String>()
                        .to_uppercase()
                });
                (
                    entry.id().to_owned(),
                    window.id.clone(),
                    window.title.clone(),
                    icon,
                )
            })
        })
        .collect()
}

/// The right stripe: the project's right-anchored windows, AI Chat, then plugin windows.
pub fn right_stripe(cx: &ViewCx) -> ToolStripe {
    let mut stripe = ToolStripe::new(StripeAnchor::Right);
    for window in right_stripe_windows(cx) {
        stripe.top.push(button(cx, window));
    }
    stripe.top.push(button(cx, ToolWindow::AiChat));
    let active = cx.state.plugin_host_service().tool_window();
    for (plugin, id, title, icon) in plugin_windows(cx) {
        let mut b = StripeButton::new(
            plugin_button_id(&plugin, &id),
            IconKey::Text(icon),
            Tooltip::new(title),
        );
        b.selected = active.as_ref() == Some(&(plugin.clone(), id.clone()));
        b.focused = b.selected;
        stripe.top.push(b);
    }
    stripe
}

/// The stripe menu of a button: Hide (the kit's View Mode and Move to are the GUI's window
/// manager and not part of the terminal).
pub fn stripe_menu(cx: &ViewCx, id: &str) -> Vec<archipel_kit_model::MenuEntry> {
    use archipel_kit_model::{MenuEntry, MenuItem};
    let visible = ToolWindow::parse(id).is_some_and(|w| cx.state.tool_windows().is_visible(w));
    vec![MenuEntry::Item(
        MenuItem::new(format!("stripe.hide:{id}"), "Hide").enabled(visible),
    )]
}

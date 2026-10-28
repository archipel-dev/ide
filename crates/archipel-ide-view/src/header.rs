//! Tool window headers (TUI-GUI-070..073): the title, a view selector chevron, content
//! tabs with counts, the action buttons of the window and Hide.

use crate::cx::ViewCx;
use crate::toolbar::command_row;
use archipel_ide_core::ToolWindow;
use archipel_kit_model::{Glyph, PanelHeader, PanelTab, ToolbarButton, Tooltip};

/// `1 plugin active` / `3 plugins active`.
pub fn pluralize(count: usize, one: &str, many: &str) -> String {
    if count == 1 {
        format!("{count} {one}")
    } else {
        format!("{count} {many}")
    }
}

fn action(id: &str, glyph: Glyph, name: &str) -> ToolbarButton {
    ToolbarButton::new(id, glyph, name)
}

/// The id of a header action that runs command `command`.
pub fn command_action(command: &str) -> String {
    command_row(command, &serde_json::Value::Null)
}

/// The header of `window`.
pub fn tool_window_header(cx: &ViewCx, window: ToolWindow) -> PanelHeader {
    let state = cx.state;
    let mut header = PanelHeader::new(window.id(), window.title());
    header.focused = cx.ui.focused_window.is_none_or(|focused| focused == window);
    match window {
        ToolWindow::Project => {
            header.view_selector = true;
            header.actions = vec![
                action("project.new", Glyph::Plus, "New"),
                ToolbarButton::new(
                    command_action("project.locateOpenFile"),
                    Glyph::Search,
                    "Select Opened File",
                )
                .enabled(cx.action_enabled("project.locateOpenFile")),
                ToolbarButton::new(command_action("project.expandAll"), Glyph::ExpandAll, "Expand All"),
                ToolbarButton::new(
                    command_action("project.collapseAll"),
                    Glyph::CollapseAll,
                    "Collapse All",
                ),
                action("project.options", Glyph::MoreVertical, "Options"),
            ];
        }
        ToolWindow::Commit | ToolWindow::Git => {
            let command = if window == ToolWindow::Commit {
                "git.refresh"
            } else {
                "git.log.refresh"
            };
            header.actions = vec![ToolbarButton::new(
                command_action(command),
                Glyph::Refresh,
                "Refresh",
            )];
        }
        ToolWindow::ResourceManager => {
            header.actions = vec![ToolbarButton::new(
                command_action("android.resourceManager.refresh"),
                Glyph::Refresh,
                "Refresh",
            )];
        }
        ToolWindow::Problems => {
            let counts = state.problems().counts();
            header.tabs = vec![
                PanelTab {
                    id: "file".into(),
                    title: "File".into(),
                    count: Some(u32::try_from(counts.errors + counts.warnings).unwrap_or(u32::MAX)),
                    selected: true,
                    closable: false,
                },
                PanelTab {
                    id: "project".into(),
                    title: "Project Errors".into(),
                    count: Some(u32::try_from(counts.errors).unwrap_or(u32::MAX)),
                    selected: false,
                    closable: false,
                },
            ];
        }
        ToolWindow::Terminal => {
            let tabs = state.terminal_tab_titles();
            let many = tabs.len() > 1;
            header.tabs = tabs
                .iter()
                .enumerate()
                .map(|(index, (title, active))| PanelTab {
                    id: index.to_string(),
                    title: title.clone(),
                    count: None,
                    selected: *active,
                    closable: many,
                })
                .collect();
            header.actions = vec![
                ToolbarButton::new(command_action("terminal.newTab"), Glyph::Plus, "New Session"),
                ToolbarButton::new(
                    command_action("terminal.restart"),
                    Glyph::Refresh,
                    "Restart shell",
                ),
            ];
        }
        _ => {}
    }
    header.actions.iter_mut().for_each(|a| {
        if a.tooltip.is_none() {
            a.tooltip = Some(Tooltip::new(a.name.clone()));
        }
    });
    header
}

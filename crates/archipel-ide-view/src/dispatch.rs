//! What an interaction means: a component event becomes effects (core commands, actions,
//! requests the frontend handles) so both frontends do the same for the same click.

use crate::cx::ViewCx;
use crate::toolbar::ids;
use archipel_ide_actions::ActionPlace;
use archipel_ide_core::ToolWindow;
use archipel_kit_model::{
    MenuEvent, PanelHeaderEvent, StatusBarEvent, StripeEvent, TabEvent, ToolbarEvent,
};
use serde_json::{Value, json};
use std::path::PathBuf;

/// A surface the frontend opens itself (a dialog, a popup, a field).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// An action the frontend implements (Go to Line, Find, the New dialogs, the clipboard).
    Ui(crate::actions::UiAction),
    /// Paste (the system clipboard belongs to the frontend).
    Paste,
    /// Find in Files scoped to a node of the project tree.
    FindInPathIn(PathBuf),
    /// Safe Delete dialog.
    SafeDelete(PathBuf),
    /// Move dialog.
    Move(PathBuf),
    /// Delete dialog.
    Delete(PathBuf),
    /// Rename dialog.
    Rename(PathBuf),
    /// Open a project folder.
    OpenProject(String),
    /// Show the update balloon.
    ShowUpdateBalloon,
    /// Show Git dialogs of the stash commands.
    GitStash { pop: bool },
    /// Help > Terminal Keyboard Setup… (the terminal's keyboard notice).
    KeyboardSetup,
}

/// A popup of the chrome: the frontend opens it at the widget that asked, with the entries
/// of the matching builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Popup {
    /// The project widget (`toolbar::project_popup`).
    Project,
    /// The VCS widget (`toolbar::vcs_popup`).
    Vcs,
    /// The run configuration widget (`toolbar::run_config_popup`).
    RunConfig,
    /// The Android device selector.
    Device,
    /// The menu of a status bar widget (`StatusBarItem::menu`).
    Status(String),
    /// The context menu of a target.
    Context(crate::context_menu::Target),
    /// The Options menu of the Project header.
    ProjectOptions,
    /// The New menu of the Project header.
    ProjectNew,
    /// The stripe menu of a tool window.
    Stripe(String),
}

/// What the frontend does.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Run a typed command.
    Command {
        /// The command id.
        id: String,
        /// Its parameters.
        params: Value,
    },
    /// Start an action as the user would from `place` and apply its effect.
    Action {
        /// The action id.
        id: String,
        /// Where it was started.
        place: ActionPlace,
    },
    /// Toggle a tool window (hidden windows show and take focus).
    ToggleToolWindow(ToolWindow),
    /// Hide a tool window.
    HideToolWindow(ToolWindow),
    /// Open a surface.
    Request(Request),
    /// Open a popup.
    Popup(Popup),
    /// Expand the menu bar and open the menu of header `n` when `Some`.
    OpenMenuBar(Option<usize>),
    /// Copy text to the clipboard.
    CopyText(String),
    /// Show a status message.
    Status(String),
    /// Select a node of the project tree.
    SelectTreeNode(Option<PathBuf>),
    /// Move the caret to a position (zero-based line and column) and focus the editor.
    GotoPosition {
        /// The line.
        line: usize,
        /// The column.
        column: usize,
    },
    /// The user asked to quit.
    Quit,
}

/// A component event of the chrome.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The main toolbar.
    Toolbar(ToolbarEvent),
    /// The main menu or a popup menu: a row was chosen.
    Menu(MenuEvent),
    /// A stripe.
    Stripe(StripeEvent),
    /// A tool window header (the window it belongs to).
    Header(ToolWindow, PanelHeaderEvent),
    /// The editor tab strip.
    Tab(TabEvent),
    /// The status bar.
    Status(StatusBarEvent),
}

fn command(id: &str, params: Value) -> Effect {
    Effect::Command {
        id: id.to_owned(),
        params,
    }
}

/// The effects of choosing the menu row `id`: an action, a recent project, a command row
/// (`toolbar::command_row`) or a command.
pub fn menu_row(cx: &ViewCx, id: &str) -> Vec<Effect> {
    if id == crate::main_menu::KEYBOARD_SETUP {
        return vec![Effect::Request(Request::KeyboardSetup)];
    }
    if let Some(path) = id.strip_prefix(crate::main_menu::RECENT_PREFIX) {
        return vec![Effect::Request(Request::OpenProject(path.to_owned()))];
    }
    if let Some(rest) = id.strip_prefix("cmd:") {
        let (command_id, params) = rest.split_once('\u{1f}').unwrap_or((rest, "null"));
        let params = serde_json::from_str(params).unwrap_or(Value::Null);
        return vec![command(command_id, params)];
    }
    if let Some(window) = id.strip_prefix("stripe.hide:").and_then(ToolWindow::parse) {
        return vec![Effect::HideToolWindow(window)];
    }
    match id {
        "git.stash" => return vec![Effect::Request(Request::GitStash { pop: false })],
        "git.stash.pop" => return vec![Effect::Request(Request::GitStash { pop: true })],
        _ => {}
    }
    let _ = cx;
    vec![Effect::Action {
        id: id.to_owned(),
        place: ActionPlace::MainMenu,
    }]
}

/// The tool window of a stripe button id.
pub fn window_of(id: &str) -> Option<ToolWindow> {
    ToolWindow::parse(id)
}

/// The effects of `event`.
pub fn dispatch(cx: &ViewCx, event: &Event) -> Vec<Effect> {
    match event {
        Event::Toolbar(ToolbarEvent::Click { id }) => match id.as_str() {
            ids::MAIN_MENU => vec![Effect::OpenMenuBar(None)],
            ids::PROJECT => vec![Effect::Popup(Popup::Project)],
            ids::VCS => vec![Effect::Popup(Popup::Vcs)],
            ids::RUN_CONFIG => vec![Effect::Popup(Popup::RunConfig)],
            ids::DEVICE => vec![Effect::Popup(Popup::Device)],
            ids::RUN => {
                let id = if cx.state.is_android_project() {
                    "android.app.deploy"
                } else {
                    "run.project"
                };
                vec![command(id, Value::Null)]
            }
            ids::BUILD => vec![command("build.project", Value::Null)],
            ids::DEBUG => {
                let id = if cx.state.is_debugging() {
                    "debug.stop"
                } else {
                    "debug.start"
                };
                vec![command(id, Value::Null)]
            }
            ids::STOP => vec![command("run.cancel", Value::Null)],
            ids::SEARCH => vec![command("navigate.searchEverywhere", Value::Null)],
            ids::SETTINGS => vec![Effect::Action {
                id: "ShowSettings".into(),
                place: ActionPlace::Toolbar,
            }],
            _ => Vec::new(),
        },
        Event::Toolbar(ToolbarEvent::Select { id, index }) if id == ids::MODE => {
            let mode = if *index == 1 { "agents" } else { "code" };
            vec![command("workbench.setMode", json!({ "mode": mode }))]
        }
        Event::Toolbar(_) => Vec::new(),
        Event::Menu(MenuEvent::Chosen { id }) => menu_row(cx, id),
        Event::Menu(_) => Vec::new(),
        Event::Stripe(StripeEvent::Toggle { id }) => match window_of(id) {
            Some(window) => vec![Effect::ToggleToolWindow(window)],
            None => match id.strip_prefix("plugin:").and_then(|r| r.split_once('/')) {
                Some((plugin, window)) => vec![command(
                    "plugin.toolwindow.toggle",
                    json!({"plugin": plugin, "id": window}),
                )],
                None => Vec::new(),
            },
        },
        Event::Stripe(StripeEvent::Hide { id }) => match window_of(id) {
            Some(window) if cx.state.tool_windows().is_visible(window) => {
                vec![Effect::HideToolWindow(window)]
            }
            _ => Vec::new(),
        },
        Event::Stripe(StripeEvent::Context { id }) => {
            vec![Effect::Popup(Popup::Stripe(id.clone()))]
        }
        Event::Header(window, PanelHeaderEvent::Hide) => vec![Effect::HideToolWindow(*window)],
        Event::Header(_, PanelHeaderEvent::Action { id }) => match id.as_str() {
            "project.new" => vec![Effect::Popup(Popup::ProjectNew)],
            "project.options" => vec![Effect::Popup(Popup::ProjectOptions)],
            other => menu_row(cx, other),
        },
        Event::Header(ToolWindow::Terminal, PanelHeaderEvent::SelectTab { id }) => {
            vec![command(
                "terminal.selectTab",
                json!({"index": id.parse::<usize>().unwrap_or(0)}),
            )]
        }
        Event::Header(ToolWindow::Terminal, PanelHeaderEvent::CloseTab { id }) => {
            vec![command(
                "terminal.closeTab",
                json!({"index": id.parse::<usize>().unwrap_or(0)}),
            )]
        }
        Event::Header(_, _) => Vec::new(),
        Event::Tab(TabEvent::Context { id }) => match crate::tabs::document_index(cx, id) {
            Some(index) => vec![Effect::Popup(Popup::Context(
                crate::context_menu::Target::Tab(index),
            ))],
            None => Vec::new(),
        },
        Event::Tab(TabEvent::Unpin { id }) => match crate::tabs::document_index(cx, id) {
            Some(index) => vec![command("editor.togglePin", json!({"index": index}))],
            None => Vec::new(),
        },
        Event::Tab(event) => crate::tabs::command_for(cx, event)
            .map(|(id, params)| vec![command(&id, params)])
            .unwrap_or_default(),
        Event::Status(StatusBarEvent::Click { id }) => status_click(cx, id),
        Event::Status(StatusBarEvent::Crumb { id }) => crumb_click(cx, id),
        Event::Stripe(_) | Event::Status(_) => Vec::new(),
    }
}

fn status_click(cx: &ViewCx, id: &str) -> Vec<Effect> {
    use crate::status::ids as s;
    match id {
        s::MESSAGE => {
            if cx
                .state
                .tool_windows()
                .visible(archipel_ide_core::Anchor::Bottom)
                .is_none()
            {
                vec![command("toolwindow.toggle", json!({"id": "build"}))]
            } else {
                Vec::new()
            }
        }
        s::UPDATE => vec![Effect::Request(Request::ShowUpdateBalloon)],
        s::CARET => vec![Effect::Request(Request::Ui(
            crate::actions::UiAction::GotoLine,
        ))],
        s::AI => vec![command("toolwindow.activate.aiChat", Value::Null)],
        s::AGENTS => vec![command("toolwindow.activate.agents", Value::Null)],
        other => vec![Effect::Popup(Popup::Status(other.to_owned()))],
    }
}

fn crumb_click(cx: &ViewCx, id: &str) -> Vec<Effect> {
    if let Some(n) = id
        .strip_prefix("code:")
        .and_then(|n| n.parse::<usize>().ok())
        && let Some(crumb) = cx.ui.code_crumbs.get(n)
    {
        return vec![Effect::GotoPosition {
            line: crumb.line,
            column: crumb.column,
        }];
    }
    // A path crumb reveals the file in the project tree.
    vec![command("project.locateOpenFile", Value::Null)]
}

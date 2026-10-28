//! The terminal IDE: the GUI's chrome (toolbar, main menu, stripes, tool window headers, tab
//! strip, status bar) drawn from the view crate's props, the tool window bodies and the
//! editor over the IDE core.
//!
//! Every user action is a typed command or an action of the core's registry; the frontend
//! only asks (`Ide::perform`), draws what the core answers and mirrors the editor buffers
//! (see [`crate::view`]). What the chrome shows and what a click means is decided by
//! `archipel-ide-view`; this file applies its [`Effect`]s.

use crate::chrome::ChromeUi;
use crate::dialogs::{Input, InputContent};
use crate::ide::{Ide, StartOptions};
use crate::picker::{PickerKind, PickerOverlay, PickerShared, Shared, Target};
use crate::shell::{Focus, Shell};
use crate::view::DocView;
use archipel_ide_actions::{ActionEffect, ActionPlace};
use archipel_ide_core::{Anchor, ToolWindow as CoreWindow};
use archipel_ide_view::actions::UiAction;
use archipel_ide_view::dispatch::{Effect, Popup, Request};
use archipel_kit::action_system::{Keymap, Os};
use archipel_kit::notification::{Notification, NotificationGroup};
use archipel_kit::openapi::ui::ExitCode;
use archipel_kit::ui::Status;
use archipel_kit_tui::App;
use archipel_kit_tui::balloon::Balloons;
use archipel_kit_tui::collections::{JBTree, TreeNode};
use archipel_kit_tui::dialog::Dialog;
use archipel_kit_tui::editor::EditorComponent;
use archipel_kit_tui::hit::Id;
use archipel_kit_tui::input::{Event, KeyCode, KeyEventKind};
use archipel_kit_tui::overlay::{Handled, Layers, Overlay};
use archipel_kit_tui::popup::Placement;
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::runtime::Runtime;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

pub(crate) const TREE: Id = Id::new("tui.tree");
pub(crate) const EDITOR: Id = Id::new("tui.editor");
pub(crate) const RUN: Id = Id::new("tui.run");
pub(crate) const PROBLEMS: Id = Id::new("tui.problems");
pub(crate) const FIND: Id = Id::new("tui.find");
pub(crate) const COMMIT: Id = Id::new("tui.commit");
pub(crate) const TERMINAL: Id = Id::new("tui.terminal");
pub(crate) const STRIPE_TICK: Id = Id::new("tui.errorstripe");
pub(crate) const WELCOME: Id = Id::new("tui.welcome");

/// What layers (pickers, dialogs) tell the app.
pub enum Msg {
    Picked(Target, bool),
    Dialog(DialogKind, ExitCode, Input),
}

/// Which dialog a result belongs to.
#[derive(Debug, Clone)]
pub enum DialogKind {
    GotoLine,
    FindInFiles,
    NewEntry {
        parent: PathBuf,
        directory: bool,
    },
    OpenProject,
    Commit {
        paths: Vec<PathBuf>,
    },
    Delete {
        path: PathBuf,
    },
    /// A message only: OK closes it.
    Notice,
    /// Closing a tab or quitting with edits that could not be saved.
    DiscardUnsaved {
        close: Option<usize>,
        quit: bool,
    },
}

/// A node of the project tree: what a row stands for.
#[derive(Debug, Clone)]
pub(crate) struct TreeEntry {
    pub path: Option<PathBuf>,
    pub kind: archipel_ide_core::project_view::RowKind,
    pub group: Option<&'static str>,
}

/// The terminal IDE.
pub struct TuiApp {
    pub(crate) ide: Ide,
    pub(crate) layers: Layers<Msg>,
    pub(crate) shell: Shell,
    pub(crate) chrome: ChromeUi,
    pub(crate) views: Vec<DocView>,
    pub(crate) tree: JBTree,
    pub(crate) tree_nodes: Vec<TreeNode>,
    pub(crate) tree_map: HashMap<String, TreeEntry>,
    pub(crate) tree_source: Option<Arc<Vec<archipel_ide_core::project_view::TreeRow>>>,
    pub(crate) balloons: Rc<RefCell<Balloons>>,
    pub(crate) picker: Option<Shared>,
    pub(crate) se_server_query: String,
    pub(crate) panels: crate::panels::Panels,
    pub(crate) popups: crate::popups::Popups,
    pub(crate) now: f64,
    pub(crate) busy: bool,
    pub(crate) theme_name: String,
    pub(crate) ascii: bool,
    pub(crate) read_only: Option<(PathBuf, f64, bool)>,
    pub(crate) crumb_cache: Option<(PathBuf, u64, Arc<Vec<archipel_ide_language::StructureItem>>)>,
    pub(crate) stripe_buckets: Vec<(u64, usize)>,
    pub(crate) welcome_rows: Vec<(Id, Effect)>,
    pub(crate) notice: crate::notice::Notice,
    /// The keymap the runtime resolves; kept to show shortcuts.
    pub(crate) quit: bool,
}

impl TuiApp {
    /// Starts the IDE core with `options` and builds the frontend around it.
    pub fn new(options: StartOptions) -> Self {
        Self::with_ide(Ide::start(options))
    }

    /// A frontend over a started core.
    pub fn with_ide(ide: Ide) -> Self {
        let balloons = Rc::new(RefCell::new(Balloons::new()));
        let mut layers = Layers::new();
        layers.push_ambient(BalloonLayer(balloons.clone()));
        let mut app = Self {
            ide,
            layers,
            shell: Shell::default(),
            chrome: ChromeUi::default(),
            views: Vec::new(),
            tree: JBTree::new(),
            tree_nodes: Vec::new(),
            tree_map: HashMap::new(),
            tree_source: None,
            balloons,
            picker: None,
            se_server_query: String::new(),
            panels: crate::panels::Panels::default(),
            popups: crate::popups::Popups::default(),
            now: 0.0,
            busy: false,
            theme_name: String::new(),
            ascii: std::env::var("ARCHIPEL_TUI_GLYPHS")
                .is_ok_and(|v| v.eq_ignore_ascii_case("ascii")),
            read_only: None,
            crumb_cache: None,
            stripe_buckets: Vec::new(),
            welcome_rows: Vec::new(),
            notice: crate::notice::Notice::default(),
            quit: false,
        };
        // Like the GUI: the Project window is open when a project is.
        app.ide.state_mut().tool_windows_mut().hide_all();
        if app.ide.state().has_project() {
            app.ide
                .state_mut()
                .tool_windows_mut()
                .show(CoreWindow::Project);
        }
        app
    }

    /// The core, for tests and for the caller that shuts it down.
    pub fn ide(&self) -> &Ide {
        &self.ide
    }

    /// The outlines of the chrome drawn in the last frame next to the ones of its props, for
    /// the parity tests (TUI-GUI-191).
    pub fn chrome_parity(&self, rt: &Runtime, buf: &Buffer) -> Vec<crate::chrome::Pair> {
        self.chrome.parity(buf, ChromeUi::notation(rt))
    }

    /// The main menu control, for tests that open its menus.
    pub fn menu_control(&mut self) -> &mut archipel_kit_tui::menubar::MenuBarControl {
        &mut self.chrome.menu
    }

    /// The menu bar the last frame built.
    pub fn built_menu(&self) -> Option<&archipel_kit_model::MenuBar> {
        self.chrome.built.as_ref().map(|c| &c.menu)
    }

    /// Keeps the one-time keyboard notice's marker at `path` (tests).
    pub fn keyboard_notice_marker(&mut self, path: std::path::PathBuf) {
        self.notice.marker_at(path);
    }

    /// Switches between Nerd Font glyphs and their ASCII fallbacks (`ARCHIPEL_TUI_GLYPHS=ascii`
    /// at start); the geometry is the same.
    pub fn set_ascii_glyphs(&mut self, on: bool) {
        self.ascii = on;
    }

    /// The window geometry state, mutably.
    pub fn shell_mut(&mut self) -> &mut Shell {
        &mut self.shell
    }

    /// The core, mutably.
    pub fn ide_mut(&mut self) -> &mut Ide {
        &mut self.ide
    }

    /// Whether the user asked to quit.
    pub fn wants_quit(&self) -> bool {
        self.quit
    }

    /// The keymap for the runtime: the kit's terminal keymap with the IDE's shortcuts.
    pub fn keymap(&self, os: Os) -> Keymap {
        let active = self.ide.state().keymap_state();
        crate::keys::build_keymap(os, &active.keymap, |id| {
            self.ide.knows_action(id) || Self::is_shell_action(id) || id == "ChooseRunConfiguration"
        })
    }

    /// Whether the shell implements action `id` (window layout keys).
    pub fn is_shell_action(id: &str) -> bool {
        matches!(
            id,
            "HideActiveWindow"
                | "JumpToLastWindow"
                | "MaximizeToolWindow"
                | "ResizeToolWindowLeft"
                | "ResizeToolWindowRight"
                | "ResizeToolWindowUp"
                | "ResizeToolWindowDown"
        )
    }

    /// Shows a balloon.
    pub(crate) fn notify(&mut self, status: Status, title: &str, body: &str) {
        let notification =
            Notification::new(status, title).group(NotificationGroup::balloon_group("ide"));
        self.balloons
            .borrow_mut()
            .notify(notification, (!body.is_empty()).then_some(body));
    }

    // ------------------------------------------------------------------ core glue

    /// Brings the editor buffers and the core documents in line, then the lists the panels and
    /// the tab line show. Cheap when nothing changed.
    pub(crate) fn sync_all(&mut self) {
        self.sync_views();
        self.refresh_tree();
        self.refresh_panels();
    }

    /// Mirrors every open document: what the editor widget did by itself (a click, a fold
    /// chevron, Replace in its find bar) goes to the core as commands, then the core's state
    /// (text, carets, folds) is shown in every editor model.
    pub(crate) fn sync_views(&mut self) {
        let paths: Vec<PathBuf> = self
            .ide
            .state()
            .documents()
            .iter()
            .map(|document| document.path.clone())
            .collect();
        let mut old: HashMap<PathBuf, DocView> = std::mem::take(&mut self.views)
            .into_iter()
            .map(|view| (view.path.clone(), view))
            .collect();
        let mut views = Vec::with_capacity(paths.len());
        for (index, path) in paths.iter().enumerate() {
            let view = match old.remove(path) {
                Some(view) => view,
                None => match self.ide.state().document(index) {
                    Some(document) => DocView::open(document),
                    None => continue,
                },
            };
            views.push(view);
        }
        self.views = views;
        if let Some(index) = self.active_view() {
            let outgoing = match self.ide.state().document(index) {
                Some(document) => self.views[index].outgoing(document),
                None => Vec::new(),
            };
            for (id, params) in outgoing {
                let _ = self.ide.dispatch(id, params);
            }
        }
        for (index, view) in self.views.iter_mut().enumerate() {
            view.refresh(self.ide.state_mut(), index);
        }
    }

    /// The index of the active document that the editor shows, if any.
    pub(crate) fn active_view(&self) -> Option<usize> {
        let index = self.ide.state().active_index()?;
        (index < self.views.len()).then_some(index)
    }

    /// Runs a command after the editor buffers were pushed to the core.
    pub(crate) fn command(&mut self, rt: &mut Runtime, id: &str, params: Value) {
        self.sync_views();
        // Closing a tab saves it first (the GUI autosaves on close) and asks when it cannot.
        if id == "editor.close" && params["saved"].as_bool() != Some(true) {
            let index = params["index"]
                .as_u64()
                .map(|i| i as usize)
                .or_else(|| self.ide.state().active_index());
            if let Some(index) = index {
                self.close_document_tab(rt, index);
            }
            return;
        }
        if let Some(why) = unsupported(id) {
            let label = self
                .ide
                .specs()
                .into_iter()
                .find(|spec| spec.id == id)
                .map(|spec| spec.label)
                .unwrap_or_else(|| id.to_owned());
            self.not_available(rt, &label, why);
            return;
        }
        // Activating a window that is already shown focuses it; once it has the focus, the
        // same key hides it (the GUI's Alt+1).
        if let Some(window) = id
            .strip_prefix("toolwindow.activate.")
            .and_then(CoreWindow::parse)
            && self.ide.state().tool_windows().is_visible(window)
        {
            if self.shell.focused_tool() == Some(window) && window != CoreWindow::Terminal {
                self.ide.state_mut().tool_windows_mut().hide(window);
                self.shell.focus_editor();
            } else {
                self.shell.focus_tool(window);
            }
            self.after_input(rt);
            return;
        }
        let shown_before = self
            .ide
            .state()
            .tool_windows()
            .visible(Anchor::Left)
            .into_iter()
            .chain(self.ide.state().tool_windows().visible(Anchor::Bottom))
            .chain(self.ide.state().tool_windows().visible(Anchor::Right))
            .collect::<Vec<_>>();
        let result = self.ide.dispatch(id, params.clone());
        match result {
            Ok(_) => {}
            // A disabled command is not an error: its item is greyed out, and a key that
            // reaches it does nothing (never a toast with an internal id).
            Err(error) if error.contains("disabled") => {}
            Err(error) => {
                let label = self
                    .ide
                    .specs()
                    .into_iter()
                    .find(|spec| spec.id == id)
                    .map(|spec| spec.label)
                    .unwrap_or_else(|| "Command failed".to_owned());
                self.notify(Status::Error, &label, &error);
            }
        }
        // A window a toolwindow command showed takes the focus; one it hid gives it back.
        if id.starts_with("toolwindow.") {
            self.follow_tool_windows(&shown_before);
        }
        self.after_input(rt);
    }

    /// Focus follows what the core showed or hid.
    fn follow_tool_windows(&mut self, before: &[CoreWindow]) {
        let layout = self.ide.state().tool_windows();
        let now: Vec<CoreWindow> = [Anchor::Left, Anchor::Bottom, Anchor::Right]
            .into_iter()
            .filter_map(|anchor| layout.visible(anchor))
            .collect();
        if let Some(new) = now.iter().find(|w| !before.contains(w)).copied() {
            self.shell.focus_tool(new);
            if new == CoreWindow::Terminal {
                self.ensure_terminal();
            }
        } else if let Some(focused) = self.shell.focused_tool()
            && !now.contains(&focused)
        {
            self.shell.focus_editor();
        }
    }

    /// A dialog saying a function needs the window: names the fallback (TUI-GUI-142).
    pub(crate) fn not_available(&mut self, rt: &mut Runtime, what: &str, why: &str) {
        let dialog = Dialog::new("Not available in the terminal yet")
            .message(format!(
                "{what}: {why}.\nStart archipel-ide without --tui to use it; the project is the same."
            ))
            .primary("OK")
            .on_exit(|code| vec![Msg::Dialog(DialogKind::Notice, code, Input::default())]);
        self.layers.push(rt, dialog);
    }

    /// Everything that follows input: buffers, requests the core made, layout.
    pub(crate) fn after_input(&mut self, rt: &mut Runtime) {
        self.sync_views();
        self.ide.state_mut().record_caret_location();
        self.process_requests(rt);
        self.refresh_tree();
        self.refresh_panels();
    }

    /// Opens what a `navigate.*` command asked for.
    fn process_requests(&mut self, rt: &mut Runtime) {
        use archipel_ide_core::quick_nav::{NavigationRequest, SearchTab};
        match self.ide.state().take_navigation_request() {
            Some(NavigationRequest::SearchEverywhere(tab)) => {
                self.open_picker(rt, PickerKind::SearchEverywhere, tab_index(tab))
            }
            Some(NavigationRequest::RecentFiles) => self.open_picker(rt, PickerKind::Recent, 0),
            Some(NavigationRequest::RecentLocations) => self.open_picker(rt, PickerKind::Recent, 1),
            None => {}
        }
        if self.ide.state().switcher_open() {
            // The Switcher needs Ctrl held and released; the terminal shows Recent Files.
            let _ = self.ide.dispatch("navigate.switcherCancel", Value::Null);
            self.open_picker(rt, PickerKind::Recent, 0);
        }
        let _ = SearchTab::All;
        if let Some(focus) = self
            .ide
            .state()
            .execution()
            .take_edit_configurations_request()
        {
            let _ = focus;
            self.open_picker(rt, PickerKind::Configs, 0);
        }
        if self.ide.state_mut().settings_request.take().is_some() {
            self.ide
                .state_mut()
                .set_status("Settings are not available in the terminal".to_owned());
        }
    }

    // ------------------------------------------------------------------ actions

    fn editor_focused(&self) -> bool {
        self.layers.is_empty()
            && self.shell.focus == Focus::Editor
            && !self.views.is_empty()
            && !self.chrome.modal()
    }

    /// Whether the find bar of the active editor is open.
    pub(crate) fn find_open(&self) -> bool {
        self.active_view()
            .and_then(|index| self.views.get(index))
            .is_some_and(|view| view.state.find_open())
    }

    /// Typed text goes to the core's input layer like in the GUI (pairs, overtype, surround,
    /// live templates, completion). Returns whether `key` was typed text.
    fn type_key(&mut self, rt: &mut Runtime, key: &archipel_kit_tui::input::KeyEvent) -> bool {
        let Some(typed) = archipel_kit_tui::input::typed_char(key) else {
            return false;
        };
        self.command(
            rt,
            "editor.type",
            json!({ "text": typed.to_string(), "live": true }),
        );
        true
    }

    /// Gives `event` to the kit's editor widget (its find bar, the mouse). Returns whether it
    /// used the event.
    pub(crate) fn forward_to_editor_widget(&mut self, rt: &mut Runtime, event: &Event) -> bool {
        let Some(index) = self.active_view() else {
            return false;
        };
        let view = &mut self.views[index];
        let crate::view::DocView {
            editor,
            state,
            highlighter,
            language,
            highlights,
            ..
        } = view;
        let out = EditorComponent::new(EDITOR, editor)
            .highlighter(highlighter)
            .language(language)
            .highlights(highlights)
            .bracket_folding(false)
            .wrap(true)
            .handle(rt, state, event);
        if let Some(hint) = out.hint {
            self.ide.state_mut().set_status(hint);
        }
        out.consumed
    }

    pub(crate) fn terminal_focused(&self) -> bool {
        self.layers.is_empty() && self.shell.focused_tool() == Some(CoreWindow::Terminal)
    }

    /// The clipboard actions of the focused editor: the terminal owns the clipboard, so the
    /// view crate hides them for key presses and the TUI runs them itself.
    fn clipboard_action(&mut self, rt: &mut Runtime, id: &str) -> bool {
        if !self.editor_focused() {
            return false;
        }
        match id {
            "$Copy" => self.run_ui_action(rt, UiAction::Copy),
            "$Cut" => self.run_ui_action(rt, UiAction::Cut),
            "$Paste" => self.run_ui_action(rt, UiAction::Paste),
            "EditorPasteSimple" => {
                let text = rt.clipboard.get().replace("\r\n", "\n");
                if !text.is_empty() {
                    self.command(rt, "editor.paste", json!({ "text": text, "raw": true }));
                }
            }
            _ => return false,
        }
        true
    }

    /// The actions of the window layout the shell owns: hide, jump, maximize, resize.
    fn shell_action(&mut self, id: &str) -> bool {
        let focused = self.shell.focused_tool();
        match id {
            "HideActiveWindow" => {
                let Some(window) = focused else { return false };
                self.ide.state_mut().tool_windows_mut().hide(window);
                self.shell.focus_editor();
            }
            "JumpToLastWindow" => match (focused, self.shell.last_tool) {
                (Some(_), _) => self.shell.focus_editor(),
                (None, Some(window)) if self.ide.state().tool_windows().is_visible(window) => {
                    self.shell.focus_tool(window);
                }
                _ => return false,
            },
            "MaximizeToolWindow" => {
                let Some(window) = focused else { return false };
                self.shell.maximized = (self.shell.maximized != Some(window)).then_some(window);
            }
            "ResizeToolWindowLeft"
            | "ResizeToolWindowRight"
            | "ResizeToolWindowUp"
            | "ResizeToolWindowDown" => {
                let Some(window) = focused else { return false };
                let anchor = window.anchor();
                let grow = match (anchor, id) {
                    (Anchor::Left, "ResizeToolWindowRight")
                    | (Anchor::Right, "ResizeToolWindowLeft") => 2,
                    (Anchor::Left, "ResizeToolWindowLeft")
                    | (Anchor::Right, "ResizeToolWindowRight") => -2,
                    (Anchor::Bottom, "ResizeToolWindowUp") => 2,
                    (Anchor::Bottom, "ResizeToolWindowDown") => -2,
                    _ => return false,
                };
                self.shell.resize(anchor, grow);
            }
            _ => return false,
        }
        true
    }

    /// Starts action `id` as a key press would. Returns whether the IDE took the key.
    fn try_action(&mut self, rt: &mut Runtime, id: &str) -> bool {
        if self.clipboard_action(rt, id) || self.shell_action(id) {
            return true;
        }
        if id == "ChooseRunConfiguration" {
            self.open_chrome_popup(rt, Popup::RunConfig, None);
            return true;
        }
        // Alt+digit: a shown window takes the focus, the same key hides it once it has it.
        if let Some(binding) = archipel_ide_core::keymap_commands::command_for_action(id)
            && binding.command == "toolwindow.toggle"
            && let Some(window) = serde_json::from_str::<Value>(binding.params)
                .ok()
                .and_then(|p| p["id"].as_str().and_then(CoreWindow::parse))
        {
            self.command(
                rt,
                &format!("toolwindow.activate.{}", window.id()),
                Value::Null,
            );
            return true;
        }
        // The kit's own ids for what the IDE calls differently.
        let id = match id {
            "CloseActiveTab" => "CloseContent",
            other => other,
        };
        if !self.ide.knows_action(id) {
            return false;
        }
        let editor_focused = self.editor_focused();
        self.sync_views();
        let Some(effect) = self.ide.perform(id, ActionPlace::Keymap, editor_focused) else {
            return false;
        };
        self.apply_effect(rt, effect);
        true
    }

    /// Does what an action asked for.
    pub(crate) fn apply_effect(&mut self, rt: &mut Runtime, effect: ActionEffect) {
        match effect {
            ActionEffect::None => {}
            ActionEffect::Request(request) => {
                if let Some(action) = request.downcast_ref::<UiAction>() {
                    self.run_ui_action(rt, *action);
                }
            }
            ActionEffect::Command { id, params } => self.command(rt, &id, params),
        }
    }

    /// The tool windows that show a header, with whether they are focused.
    pub(crate) fn chrome_windows(&self) -> Vec<(CoreWindow, bool)> {
        let focused = self.shell.focused_tool();
        let layout = self.ide.state().tool_windows();
        [Anchor::Left, Anchor::Right, Anchor::Bottom]
            .into_iter()
            .filter_map(|anchor| layout.visible(anchor))
            .map(|w| (w, focused == Some(w)))
            .collect()
    }

    /// Applies the effects of the view crate's dispatch.
    pub(crate) fn apply_effects(&mut self, rt: &mut Runtime, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Command { id, params } => self.command(rt, &id, params),
                Effect::Action { id, place } => self.run_action(rt, &id, place),
                Effect::ToggleToolWindow(window) => {
                    self.command(rt, "toolwindow.toggle", json!({ "id": window.id() }));
                }
                Effect::HideToolWindow(window) => {
                    self.ide.state_mut().tool_windows_mut().hide(window);
                    if self.shell.focused_tool() == Some(window) {
                        self.shell.focus_editor();
                    }
                }
                Effect::Request(request) => self.run_request(rt, request),
                Effect::Popup(popup) => self.open_chrome_popup(rt, popup, None),
                Effect::OpenMenuBar(index) => {
                    self.chrome.menu.expand();
                    if let (Some(index), Some(built)) = (index, self.chrome.built.clone()) {
                        self.chrome.menu.open_menu(rt, &built.menu, index, true);
                    }
                }
                Effect::CopyText(text) => rt.clipboard.set_content(text, false),
                Effect::Status(text) => self.ide.state_mut().set_status(text),
                Effect::SelectTreeNode(path) => self.ide.data.tree_selection = path,
                Effect::GotoPosition { line, column } => {
                    self.command(
                        rt,
                        "editor.gotoLine",
                        json!({ "line": line + 1, "column": column + 1 }),
                    );
                    self.shell.focus_editor();
                }
                Effect::Quit => self.quit_now(rt),
            }
        }
    }

    /// Opens a popup of the chrome: the entries come from the view crate; `at` places a
    /// context menu at the pointer, else it opens below the widget.
    pub(crate) fn open_chrome_popup(
        &mut self,
        rt: &mut Runtime,
        popup: Popup,
        at: Option<(u16, u16)>,
    ) {
        let entries = {
            let cx = self.ide.cx();
            crate::chrome::popup_entries(&cx, self.chrome.built.as_ref(), &popup)
        };
        let placement = match (at, self.chrome.anchor_of(&popup)) {
            (Some((x, y)), _) => Placement::At(x, y),
            (None, Some(anchor)) => Placement::Below(anchor),
            _ => Placement::At(rt.screen.x + 1, rt.screen.y + 1),
        };
        self.chrome.open_popup(rt, popup, entries, placement, false);
    }

    /// The keyboard notice (TUI-GUI-159): what this terminal can send and the leader chords.
    pub(crate) fn keyboard_notice(&mut self, rt: &mut Runtime) {
        let leader = archipel_ide_view::keys::LEADER;
        let text = crate::notice::text(&leader.replace("ctrl ", "Ctrl+"), rt.caps.kitty);
        let dialog = Dialog::new("Terminal keyboard")
            .message(text)
            .primary("OK")
            .on_exit(|code| vec![Msg::Dialog(DialogKind::Notice, code, Input::default())]);
        self.layers.push(rt, dialog);
    }

    /// Surfaces the frontend opens for a request of the view crate.
    fn run_request(&mut self, rt: &mut Runtime, request: Request) {
        match request {
            Request::Ui(action) => self.run_ui_action(rt, action),
            Request::Paste => self.run_ui_action(rt, UiAction::Paste),
            Request::FindInPathIn(path) => {
                self.ide.state_mut().project_view_mut().search_scope =
                    (!path.as_os_str().is_empty()).then(|| path.clone());
                let kind = if self.ide.state().project_view().search_scope.is_some() {
                    archipel_ide_core::SearchScopeKind::Directory
                } else {
                    archipel_ide_core::SearchScopeKind::Project
                };
                self.ide.state_mut().set_search_scope_kind(kind);
                self.run_ui_action(rt, UiAction::FindInPath);
            }
            Request::Delete(path) => {
                self.ide.data.tree_selection = Some(path);
                self.confirm_delete(rt);
            }
            Request::SafeDelete(_) => {
                self.not_available(rt, "Safe Delete", "it needs the usages dialog")
            }
            Request::Move(_) => self.not_available(rt, "Move", "it needs the move dialog"),
            Request::Rename(_) => self.not_available(rt, "Rename", "it needs the rename dialog"),
            Request::OpenProject(path) => {
                match self.ide.open_project(Path::new(&path)) {
                    Ok(()) => {
                        self.ide
                            .state_mut()
                            .tool_windows_mut()
                            .show(CoreWindow::Project);
                    }
                    Err(error) => self.notify(Status::Error, "Open Project", &error),
                }
                self.after_input(rt);
            }
            Request::GitStash { .. } => {
                self.not_available(rt, "Stash", "it needs the stash dialog")
            }
            Request::KeyboardSetup => self.keyboard_notice(rt),
            Request::ShowUpdateBalloon => self.ide.state().notifications().show_update_balloon(),
        }
    }

    /// Quits: saves what can be saved first and asks when something could not be.
    pub(crate) fn quit_now(&mut self, rt: &mut Runtime) {
        self.sync_views();
        let dirty = |app: &Self| {
            app.ide
                .state()
                .documents()
                .iter()
                .any(|d| d.text.is_dirty())
        };
        if dirty(self) {
            let _ = self.ide.state_mut().save_all();
        }
        if dirty(self) {
            let names: Vec<String> = self
                .ide
                .state()
                .documents()
                .iter()
                .filter(|d| d.text.is_dirty())
                .map(|d| d.path.display().to_string())
                .collect();
            let kind = DialogKind::DiscardUnsaved {
                close: None,
                quit: true,
            };
            let dialog = Dialog::new("Unsaved changes")
                .message(format!(
                    "These files could not be saved:\n{}\nQuit and discard the changes?",
                    names.join("\n")
                ))
                .primary("Discard and Quit")
                .secondary("Cancel")
                .danger(true)
                .on_exit(move |code| vec![Msg::Dialog(kind.clone(), code, Input::default())]);
            self.layers.push(rt, dialog);
            return;
        }
        self.ide.shutdown();
        self.quit = true;
        rt.quit = true;
    }

    /// Closes the tab of document `index`: unsaved edits are saved first (as the GUI does);
    /// when that fails the user is asked.
    pub(crate) fn close_document_tab(&mut self, rt: &mut Runtime, index: usize) {
        self.sync_views();
        let dirty = self
            .ide
            .state()
            .document(index)
            .is_some_and(|d| d.text.is_dirty());
        if dirty {
            let _ = self.ide.state_mut().save(index);
            let still = self
                .ide
                .state()
                .document(index)
                .is_some_and(|d| d.text.is_dirty());
            if still {
                let name = self
                    .ide
                    .state()
                    .document(index)
                    .map(|d| d.path.display().to_string())
                    .unwrap_or_default();
                let kind = DialogKind::DiscardUnsaved {
                    close: Some(index),
                    quit: false,
                };
                let dialog = Dialog::new("Unsaved changes")
                    .message(format!(
                        "{name} could not be saved.\nClose the tab and discard the changes?"
                    ))
                    .primary("Discard")
                    .secondary("Cancel")
                    .danger(true)
                    .on_exit(move |code| vec![Msg::Dialog(kind.clone(), code, Input::default())]);
                self.layers.push(rt, dialog);
                return;
            }
        }
        self.command(rt, "editor.close", json!({ "index": index, "saved": true }));
    }

    /// Starts action `id` from a menu or Find Action.
    pub fn run_action(&mut self, rt: &mut Runtime, id: &str, place: ActionPlace) {
        if self.clipboard_action(rt, id) || self.shell_action(id) {
            return;
        }
        if id == "ChooseRunConfiguration" {
            self.open_chrome_popup(rt, Popup::RunConfig, None);
            return;
        }
        self.sync_views();
        // A disabled action does nothing: its entries are greyed out, so no message names it.
        if let Some(effect) = self.ide.perform(id, place, self.editor_focused()) {
            self.apply_effect(rt, effect);
        }
    }

    pub(crate) fn run_ui_action(&mut self, rt: &mut Runtime, action: UiAction) {
        match action {
            UiAction::GotoLine => self.input_dialog(
                rt,
                "Go to Line:Column",
                "Line:Column",
                "",
                DialogKind::GotoLine,
                |content| content.placeholder("12 or 12:5"),
            ),
            UiAction::FindInPath => {
                let query = self.selected_text().unwrap_or_default();
                let options = self.ide.state().search_options();
                self.input_dialog(
                    rt,
                    "Find in Files",
                    "Find",
                    &query,
                    DialogKind::FindInFiles,
                    move |content| {
                        content
                            .option("Match case", options.match_case)
                            .option("Words", options.whole_words)
                            .option("Regex", options.regex)
                    },
                )
            }
            UiAction::Find | UiAction::Replace => {
                if let Some(index) = self.active_view() {
                    let view = &mut self.views[index];
                    view.state
                        .open_find(&view.editor, action == UiAction::Replace);
                    self.shell.focus_editor();
                }
            }
            UiAction::FindNext | UiAction::FindPrevious => {
                let id = if action == UiAction::FindNext {
                    "editor.findNext"
                } else {
                    "editor.findPrevious"
                };
                self.command(rt, id, Value::Null);
            }
            UiAction::SaveAll => self.command(rt, "editor.saveAll", Value::Null),
            UiAction::NewFile | UiAction::NewDirectory => {
                let directory = action == UiAction::NewDirectory;
                let parent = self.new_entry_parent();
                let title = if directory {
                    "New Directory"
                } else {
                    "New File"
                };
                let shown = parent.display().to_string();
                self.input_dialog(
                    rt,
                    title,
                    &format!("Name in {}", if shown.is_empty() { "." } else { &shown }),
                    "",
                    DialogKind::NewEntry { parent, directory },
                    |content| content,
                )
            }
            UiAction::OpenProject => {
                let here = std::env::current_dir()
                    .map(|dir| dir.display().to_string())
                    .unwrap_or_default();
                self.input_dialog(
                    rt,
                    "Open Project",
                    "Folder",
                    &here,
                    DialogKind::OpenProject,
                    |content| content,
                )
            }
            UiAction::Copy | UiAction::Cut => {
                let id = if action == UiAction::Copy {
                    "editor.copy"
                } else {
                    "editor.cut"
                };
                self.sync_views();
                match self.ide.dispatch(id, Value::Null) {
                    Ok(result) if result["skipped"].as_bool() != Some(true) => {
                        let text = result["text"].as_str().unwrap_or_default().to_owned();
                        let whole_line = result["wholeLine"].as_bool().unwrap_or(false);
                        rt.clipboard.set_content(text, whole_line);
                    }
                    _ => {}
                }
                self.after_input(rt);
            }
            UiAction::Paste => {
                let text = rt.clipboard.get().replace("\r\n", "\n");
                if !text.is_empty() {
                    let params = json!({ "text": text, "wholeLine": rt.clipboard.is_whole_line() });
                    self.command(rt, "editor.paste", params);
                }
            }
            UiAction::Exit => self.quit_now(rt),
            UiAction::FileStructure => {
                self.not_available(rt, "File Structure", "the structure popup is not built yet")
            }
            UiAction::NewProject | UiAction::ManageProjects => {
                self.not_available(rt, "New Project", "the project wizard is not built yet")
            }
            UiAction::NewJavaClass | UiAction::NewKotlinFile => {
                self.not_available(rt, "New Class", "the class wizard is not built yet")
            }
            UiAction::Push => self.not_available(rt, "Push", "the push dialog is not built yet"),
        }
    }

    pub(crate) fn selected_text(&self) -> Option<String> {
        self.ide
            .state()
            .selected_text()
            .filter(|text| !text.contains('\n'))
    }

    /// The directory New File goes into: the selected directory of the tree, or the parent of
    /// the selected file.
    fn new_entry_parent(&self) -> PathBuf {
        let root = self.ide.state().project_root();
        match (&self.ide.data.tree_selection, root) {
            (Some(path), Some(root)) => {
                if root.join(path).is_dir() {
                    path.clone()
                } else {
                    path.parent().map(Path::to_path_buf).unwrap_or_default()
                }
            }
            _ => PathBuf::new(),
        }
    }

    /// Opens a dialog with one text field.
    pub(crate) fn input_dialog(
        &mut self,
        rt: &mut Runtime,
        title: &str,
        label: &str,
        initial: &str,
        kind: DialogKind,
        configure: impl FnOnce(InputContent) -> InputContent,
    ) {
        let shared: Rc<RefCell<Input>> = Rc::default();
        let content = configure(InputContent::new(label, initial, shared.clone()));
        let dialog = Dialog::new(title)
            .primary("OK")
            .secondary("Cancel")
            .content(content)
            .on_exit(move |code| vec![Msg::Dialog(kind.clone(), code, shared.borrow().clone())]);
        self.layers.push(rt, dialog);
    }

    /// Applies a message of a layer.
    pub(crate) fn apply(&mut self, rt: &mut Runtime, msg: Msg) {
        match msg {
            Msg::Picked(target, alternate) => self.picked(rt, target, alternate),
            Msg::Dialog(kind, code, input) => {
                if code == ExitCode::Primary {
                    self.dialog_done(rt, kind, input);
                }
            }
        }
    }

    fn picked(&mut self, rt: &mut Runtime, target: Target, _alternate: bool) {
        match target {
            Target::File {
                path,
                line,
                column,
                length,
            } => {
                let mut params = json!({ "path": path });
                if let Some(line) = line {
                    params["line"] = json!(line);
                }
                if let Some(column) = column {
                    params["column"] = json!(column);
                }
                if let Some(length) = length {
                    params["length"] = json!(length);
                }
                self.command(rt, "editor.openFile", params);
                self.focus_editor();
            }
            Target::Action(id) => self.run_action(rt, &id, ActionPlace::FindAction),
            Target::Config(name) => {
                self.command(rt, "runConfig.select", json!({ "name": name }));
                self.command(rt, "runConfig.run", json!({ "name": name }));
            }
            Target::Nothing => {}
        }
    }

    fn dialog_done(&mut self, rt: &mut Runtime, kind: DialogKind, input: Input) {
        match kind {
            DialogKind::GotoLine => match self.ide.state().goto_line_target(&input.text) {
                Some((line, column)) => {
                    self.command(
                        rt,
                        "editor.gotoLine",
                        json!({ "line": line, "column": column }),
                    );
                }
                None => self
                    .ide
                    .state_mut()
                    .set_status("Enter a line number, like 12 or 12:5".to_owned()),
            },
            DialogKind::FindInFiles => {
                if input.text.is_empty() {
                    self.ide
                        .state_mut()
                        .set_status("Enter the text to find".to_owned());
                    return;
                }
                let flag = |index: usize| input.flags.get(index).copied().unwrap_or(false);
                self.command(
                    rt,
                    "search.project",
                    json!({
                        "query": input.text,
                        "matchCase": flag(0),
                        "wholeWords": flag(1),
                        "regex": flag(2),
                    }),
                );
                self.ide
                    .state_mut()
                    .tool_windows_mut()
                    .show(CoreWindow::Find);
                self.shell.focus_tool(CoreWindow::Find);
            }
            DialogKind::NewEntry { parent, directory } => {
                if input.text.trim().is_empty() {
                    self.ide.state_mut().set_status("Enter a name".to_owned());
                    return;
                }
                let path = parent.join(input.text.trim());
                let id = if directory {
                    "project.newDirectory"
                } else {
                    "project.newFile"
                };
                self.command(rt, id, json!({ "path": path }));
                if !directory {
                    self.command(rt, "editor.openFile", json!({ "path": path }));
                    self.focus_editor();
                }
            }
            DialogKind::OpenProject => {
                if input.text.trim().is_empty() {
                    return;
                }
                let path = PathBuf::from(input.text.trim());
                match self.ide.open_project(&path) {
                    Ok(()) => self
                        .ide
                        .state_mut()
                        .tool_windows_mut()
                        .show(CoreWindow::Project),
                    Err(error) => self.notify(Status::Error, "Open Project", &error),
                }
                self.after_input(rt);
            }
            DialogKind::Delete { path } => {
                self.command(rt, "project.delete", json!({ "path": path }));
            }
            DialogKind::Notice => {}
            DialogKind::DiscardUnsaved { close, quit } => {
                if let Some(index) = close {
                    // Discard: the tab closes with its edits.
                    self.command(rt, "editor.close", json!({ "index": index, "saved": true }));
                }
                if quit {
                    self.ide.shutdown();
                    self.quit = true;
                    rt.quit = true;
                }
            }
            DialogKind::Commit { paths } => {
                if input.text.trim().is_empty() {
                    self.ide
                        .state_mut()
                        .set_status("Enter a commit message".to_owned());
                    return;
                }
                self.command(
                    rt,
                    "git.commit",
                    json!({ "message": input.text, "paths": paths, "amend": false }),
                );
            }
        }
    }

    /// Asks before deleting the file or directory selected in the tree.
    pub(crate) fn confirm_delete(&mut self, rt: &mut Runtime) {
        let Some(path) = self.ide.data.tree_selection.clone() else {
            return;
        };
        if path.as_os_str().is_empty() {
            return;
        }
        let kind = DialogKind::Delete { path: path.clone() };
        let dialog = Dialog::new("Delete")
            .message(format!("Delete {}?", path.display()))
            .primary("Delete")
            .secondary("Cancel")
            .danger(true)
            .on_exit(move |code| vec![Msg::Dialog(kind.clone(), code, Input::default())]);
        self.layers.push(rt, dialog);
    }

    pub(crate) fn focus_editor(&mut self) {
        self.shell.focus_editor();
    }

    // ------------------------------------------------------------------ pickers

    /// Opens a picker of `kind`.
    pub(crate) fn open_picker(&mut self, rt: &mut Runtime, kind: PickerKind, tab: usize) {
        let (title, tabs): (&str, Vec<&'static str>) = match kind {
            PickerKind::SearchEverywhere => (
                "Search Everywhere",
                archipel_ide_core::quick_nav::SearchTab::ALL
                    .iter()
                    .map(|tab| tab.label())
                    .collect(),
            ),
            PickerKind::Files => ("Go to File", Vec::new()),
            PickerKind::Actions => ("Find Action", Vec::new()),
            PickerKind::Recent => ("Recent", vec!["Files", "Locations"]),
            PickerKind::Configs => ("Run Configurations", Vec::new()),
        };
        let shared: Shared = Rc::new(RefCell::new(PickerShared::new(kind, title, tabs, tab)));
        self.picker = Some(shared.clone());
        let overlay = PickerOverlay::new(shared, |_, target, alt| vec![Msg::Picked(target, alt)]);
        self.layers.push(rt, overlay);
        self.refresh_picker();
    }

    /// Starts the shell of the Terminal window when it is shown for the first time.
    pub(crate) fn ensure_terminal(&mut self) {
        if !self.ide.state().terminal().has_session()
            && let Err(error) = self.ide.state_mut().open_terminal()
        {
            self.ide.state_mut().set_status(error);
        }
    }

    // ------------------------------------------------------------------ autosave

    fn autosave(&mut self) {
        use archipel_ide_settings::ids;
        let on = self
            .ide
            .state()
            .settings()
            .read()
            .bool(ids::AUTOSAVE_FOCUS)
            .unwrap_or(true);
        if on {
            self.sync_views();
            if self
                .ide
                .state()
                .documents()
                .iter()
                .any(|d| d.text.is_dirty())
            {
                let _ = self.ide.state_mut().save_all();
            }
        }
    }
}

/// Why a command cannot run in the terminal, when it needs a surface this frontend lacks.
fn unsupported(id: &str) -> Option<&'static str> {
    const NEEDS_GUI: [&str; 9] = [
        "debug.",
        "android.",
        "ai.",
        "agents.",
        "hierarchy.",
        "plugin.manager",
        "history.",
        "diff.",
        "settings.",
    ];
    if NEEDS_GUI.iter().any(|prefix| id.starts_with(prefix)) {
        return Some("not available in the terminal yet");
    }
    match id {
        "refactor.rename" | "code.generate" | "navigate.showUsages" => {
            Some("not available in the terminal yet")
        }
        _ => None,
    }
}

pub(crate) fn tab_index(tab: archipel_ide_core::quick_nav::SearchTab) -> usize {
    archipel_ide_core::quick_nav::SearchTab::ALL
        .iter()
        .position(|candidate| *candidate == tab)
        .unwrap_or(0)
}

/// An ambient layer that shares the balloon stack with the app.
struct BalloonLayer(Rc<RefCell<Balloons>>);

impl Overlay<Msg> for BalloonLayer {
    fn modal(&self) -> bool {
        false
    }

    fn event(&mut self, rt: &mut Runtime, event: &Event) -> Handled<Msg> {
        <Balloons as Overlay<Msg>>::event(&mut self.0.borrow_mut(), rt, event)
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        <Balloons as Overlay<Msg>>::draw(&mut self.0.borrow_mut(), rt, area, buf);
    }

    fn tick(&mut self, rt: &mut Runtime) -> Vec<Msg> {
        <Balloons as Overlay<Msg>>::tick(&mut self.0.borrow_mut(), rt)
    }

    fn deadline(&self) -> Option<f64> {
        self.0.borrow().deadline()
    }
}

impl App for TuiApp {
    fn event(&mut self, rt: &mut Runtime, event: &Event) {
        self.now = rt.now;
        let (consumed, messages) = self.layers.event(rt, event);
        for message in messages {
            self.apply(rt, message);
        }
        if consumed {
            self.after_input(rt);
            return;
        }
        if matches!(event, Event::FocusLost) {
            self.autosave();
        }
        if rt.double_shift() {
            self.open_picker(rt, PickerKind::SearchEverywhere, 0);
            self.after_input(rt);
            return;
        }
        if let Event::Key(key) = event
            && key.kind != KeyEventKind::Release
            && self.popup_key(rt, key)
        {
            self.after_input(rt);
            return;
        }
        if let Event::Paste(text) = event
            && self.terminal_focused()
        {
            let _ = self.ide.dispatch("terminal.paste", json!({ "text": text }));
            self.after_input(rt);
            return;
        }
        if let Event::Paste(text) = event
            && self.editor_focused()
            && !self.find_open()
        {
            // The terminal's own paste (Ctrl+Shift+V, Cmd+V): the core pastes it like the GUI.
            let text = text.replace("\r\n", "\n").replace('\r', "\n");
            let whole_line = rt.clipboard.is_whole_line();
            self.command(
                rt,
                "editor.paste",
                json!({ "text": text, "wholeLine": whole_line }),
            );
            return;
        }
        // The find bar of the editor widget owns the keys while it is open.
        if let Event::Key(key) = event
            && key.kind != KeyEventKind::Release
            && self.editor_focused()
            && self.find_open()
            && self.forward_to_editor_widget(rt, event)
        {
            self.after_input(rt);
            return;
        }
        let chrome_event = matches!(event, Event::Mouse(_))
            || self.chrome.modal()
            || matches!(event, Event::Key(k) if k.kind != KeyEventKind::Release && self.chrome.menu_expanded());
        if chrome_event && self.layers.is_empty() {
            let windows = self.chrome_windows();
            let routed = self.chrome.route(rt, event, &windows);
            if let Some((kind, id)) = routed.chosen {
                let effects = {
                    let cx = self.ide.cx();
                    crate::chrome::choice_effects(&cx, &kind, &id)
                };
                self.apply_effects(rt, effects);
            }
            for e in routed.events {
                let effects = {
                    let cx = self.ide.cx();
                    archipel_ide_view::dispatch(&cx, &e)
                };
                self.apply_effects(rt, effects);
            }
            if let (Event::Mouse(_), Some(pointer)) = (event, rt.pointer().cloned()) {
                self.shell.handle_pointer(&pointer);
            }
            if routed.consumed {
                self.after_input(rt);
                return;
            }
        }
        if let Event::Key(key) = event
            && key.kind != KeyEventKind::Release
        {
            if self.terminal_focused() && self.terminal_key(rt, key) {
                self.after_input(rt);
                return;
            }
            // Esc in a tool window returns to the editor, as in the GUI.
            if key.code == KeyCode::Esc
                && key.modifiers.is_empty()
                && self.layers.is_empty()
                && !self.tree.state.speed.is_active()
                && self
                    .shell
                    .focused_tool()
                    .is_some_and(|w| w != CoreWindow::Terminal)
            {
                self.shell.focus_editor();
                self.after_input(rt);
                return;
            }
            let mut ids = rt.actions().to_vec();
            if rt.chord_completed() {
                // The chord's own action first: `Ctrl+K ]` is Forward, not the editor's `Ctrl+]`.
                let chord =
                    |id: &String| archipel_ide_view::keys::CHORDS.iter().any(|(a, _)| a == id);
                ids.sort_by_key(|id| !chord(id));
            }
            // A speed search in the tree or a list keeps Esc for itself.
            let esc_for_widget = key.code == KeyCode::Esc && self.tree.state.speed.is_active();
            if !esc_for_widget {
                for id in &ids {
                    if self.try_action(rt, id) {
                        self.after_input(rt);
                        return;
                    }
                }
            }
            if self.editor_focused() && self.type_key(rt, key) {
                self.after_input(rt);
                return;
            }
        }
        self.route(rt, event);
        self.after_input(rt);
    }

    fn tick(&mut self, rt: &mut Runtime) {
        self.now = rt.now;
        let messages = self.layers.tick(rt);
        for message in messages {
            self.apply(rt, message);
        }
        self.busy = self.ide.poll();
        self.after_input(rt);
        self.refresh_picker();
        self.autoscroll(rt);
        self.run_finished_notice();
        if self.layers.is_empty() && self.notice.due(rt.caps.kitty) {
            self.keyboard_notice(rt);
        }
    }

    fn deadline(&self, rt: &Runtime) -> Option<f64> {
        let mut next = self.layers.deadline();
        let mut consider = |at: f64| next = Some(next.map_or(at, |n| n.min(at)));
        if let Some(at) = rt.tooltip.deadline() {
            consider(at);
        }
        if let Some(view) = self.active_view().and_then(|i| self.views.get(i))
            && let Some(at) = view.state.deadline()
        {
            consider(at);
        }
        // Background work changes the screen without input: poll it.
        if self.busy || self.panels.running {
            consider(rt.now + 0.1);
        } else {
            consider(rt.now + 0.25);
        }
        next
    }

    fn draw(&mut self, rt: &mut Runtime, area: Rect, buf: &mut Buffer) {
        self.now = rt.now;
        // Like the GUI's editor: a region that hides a caret is shown.
        if let Some(index) = self.ide.state().active_index()
            && let Some(document) = self.ide.state_mut().document_mut(index)
        {
            document.reveal_carets();
        }
        self.sync_all();
        self.draw_all(rt, area, buf);
    }
}

//! The terminal IDE: the NvChad layout of the kit's `examples/ide.rs` driven by the IDE core.
//!
//! Layout: the project tree on the left, the editor with its tab line in the middle, a bottom
//! tool window (Run, Problems, Find, Terminal), the Commit window on the left and a status
//! line. Every user action is a typed command or an action of the core's registry; the
//! frontend only asks (`Ide::perform`), draws what the core answers and mirrors the editor
//! buffers (see [`crate::view`]).

use crate::actions::TuiAction;
use crate::dialogs::{Input, InputContent};
use crate::ide::{Ide, StartOptions};
use crate::picker::{PickerKind, PickerOverlay, PickerShared, Shared, Target};
use crate::view::DocView;
use archipel_ide_actions::{ActionEffect, ActionPlace};
use archipel_ide_core::{Anchor, ToolWindow as CoreWindow};
use archipel_kit::action_system::{Keymap, Os};
use archipel_kit::notification::{Notification, NotificationGroup};
use archipel_kit::openapi::ui::ExitCode;
use archipel_kit::ui::Status;
use archipel_kit::wm::ToolWindowAnchor;
use archipel_kit_tui::App;
use archipel_kit_tui::balloon::Balloons;
use archipel_kit_tui::collections::{JBTree, TreeNode};
use archipel_kit_tui::components::TabInfo;
use archipel_kit_tui::dashboard::Dashboard;
use archipel_kit_tui::dialog::Dialog;
use archipel_kit_tui::hit::Id;
use archipel_kit_tui::input::{Event, KeyCode, KeyEventKind};
use archipel_kit_tui::overlay::{Handled, Layers, Overlay};
use archipel_kit_tui::ratatui::buffer::Buffer;
use archipel_kit_tui::ratatui::layout::Rect;
use archipel_kit_tui::runtime::Runtime;
use archipel_kit_tui::statusbar::StatusBarControl;
use archipel_kit_tui::tabs::{TabsControl, TabsState};
use archipel_kit_tui::workbench::{Workbench, WorkbenchEvent, WorkbenchFocus};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

pub(crate) const TREE: Id = Id::new("tui.tree");
pub(crate) const EDITOR: Id = Id::new("tui.editor");
pub(crate) const TABS: Id = Id::new("tui.tabs");
pub(crate) const RUN: Id = Id::new("tui.run");
pub(crate) const PROBLEMS: Id = Id::new("tui.problems");
pub(crate) const FIND: Id = Id::new("tui.find");
pub(crate) const COMMIT: Id = Id::new("tui.commit");
pub(crate) const TERMINAL: Id = Id::new("tui.terminal");
pub(crate) const STATUS: Id = Id::new("tui.status");
pub(crate) const DASH: Id = Id::new("tui.dashboard");
pub(crate) const TAB_BUTTONS: Id = Id::new("tui.tabbuttons");

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
    NewEntry { parent: PathBuf, directory: bool },
    OpenProject,
    Commit { paths: Vec<PathBuf> },
    Delete { path: PathBuf },
}

/// A node of the project tree: what a row stands for.
#[derive(Debug, Clone)]
pub(crate) struct TreeEntry {
    pub path: Option<PathBuf>,
    pub kind: archipel_ide_core::project_view::RowKind,
    pub group: Option<&'static str>,
}

/// The tool windows of the core that this frontend shows, with the ids of their kit windows.
pub(crate) const WINDOWS: [(CoreWindow, &str); 6] = [
    (CoreWindow::Project, "project"),
    (CoreWindow::Commit, "commit"),
    (CoreWindow::Build, "run"),
    (CoreWindow::Problems, "problems"),
    (CoreWindow::Find, "find"),
    (CoreWindow::Terminal, "terminal"),
];

pub(crate) fn kit_id(window: CoreWindow) -> Option<&'static str> {
    WINDOWS
        .iter()
        .find(|(core, _)| *core == window)
        .map(|(_, id)| *id)
}

pub(crate) fn core_window(id: &str) -> Option<CoreWindow> {
    WINDOWS
        .iter()
        .find(|(_, kit)| *kit == id)
        .map(|(core, _)| *core)
}

/// The terminal IDE.
pub struct TuiApp {
    pub(crate) ide: Ide,
    pub(crate) layers: Layers<Msg>,
    pub(crate) wb: Workbench,
    pub(crate) views: Vec<DocView>,
    pub(crate) tree: JBTree,
    pub(crate) tree_nodes: Vec<TreeNode>,
    pub(crate) tree_map: HashMap<String, TreeEntry>,
    pub(crate) tree_source: Option<Arc<Vec<archipel_ide_core::project_view::TreeRow>>>,
    pub(crate) tabs: Vec<TabInfo>,
    pub(crate) tab_state: TabsState,
    pub(crate) tab_control: TabsControl,
    pub(crate) status: StatusBarControl,
    pub(crate) balloons: Rc<RefCell<Balloons>>,
    pub(crate) dashboard: Dashboard,
    pub(crate) picker: Option<Shared>,
    pub(crate) se_server_query: String,
    pub(crate) panels: crate::panels::Panels,
    pub(crate) popups: crate::popups::Popups,
    pub(crate) last_layout: (Option<CoreWindow>, Option<CoreWindow>),
    pub(crate) now: f64,
    pub(crate) busy: bool,
    pub(crate) buttons_rect: Rect,
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
            wb: crate::layout::workbench(),
            views: Vec::new(),
            tree: JBTree::new(),
            tree_nodes: Vec::new(),
            tree_map: HashMap::new(),
            tree_source: None,
            tabs: Vec::new(),
            tab_state: TabsState::new(0),
            tab_control: TabsControl::new(TABS),
            status: crate::layout::status_control(),
            balloons,
            dashboard: crate::layout::dashboard(),
            picker: None,
            se_server_query: String::new(),
            panels: crate::panels::Panels::default(),
            popups: crate::popups::Popups::default(),
            last_layout: (None, None),
            now: 0.0,
            busy: false,
            buttons_rect: Rect::default(),
            quit: false,
        };
        app.tab_control.reorderable = false;
        // NvChad starts with the tree only; the bottom windows open when they have something
        // to show.
        app.wb.state.hide_all();
        if app.ide.state().has_project() {
            app.wb.show("project");
        }
        app.sync_layout();
        app
    }

    /// The core, for tests and for the caller that shuts it down.
    pub fn ide(&self) -> &Ide {
        &self.ide
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
            self.ide.knows_action(id) || id == "ChooseRunConfiguration"
        })
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
        self.refresh_tabs();
        self.refresh_tree();
        self.refresh_panels();
    }

    /// Mirrors every open document.
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
            let mut view = match old.remove(path) {
                Some(view) => view,
                None => match self.ide.state().document(index) {
                    Some(document) => DocView::open(document),
                    None => continue,
                },
            };
            view.sync(self.ide.state_mut(), index);
            views.push(view);
        }
        self.views = views;
    }

    fn refresh_tabs(&mut self) {
        let state = self.ide.state();
        self.tabs = state
            .documents()
            .iter()
            .map(|document| {
                let name = document
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut tab = TabInfo::new(name);
                tab.icon = Some(crate::layout::file_icon_key(&document.path));
                tab.modified = document.text.is_dirty();
                tab
            })
            .collect();
        self.tab_state.selected = state
            .active_index()
            .unwrap_or(0)
            .min(self.tabs.len().saturating_sub(1));
    }

    /// The index of the active document that the editor shows, if any.
    pub(crate) fn active_view(&self) -> Option<usize> {
        let index = self.ide.state().active_index()?;
        (index < self.views.len()).then_some(index)
    }

    /// Runs a command after the editor buffers were pushed to the core.
    pub(crate) fn command(&mut self, rt: &mut Runtime, id: &str, params: Value) {
        self.sync_views();
        // Tool windows are the workbench's: the kit keeps focus and layout, the core follows.
        if self.tool_window_command(id, &params) {
            self.after_input(rt);
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
            self.ide.state_mut().set_status(format!("{label}: {why}"));
            return;
        }
        if let Err(error) = self.ide.dispatch(id, params) {
            self.notify(Status::Error, id, &error);
        }
        self.after_input(rt);
    }

    /// `toolwindow.*` commands act on the workbench.
    fn tool_window_command(&mut self, id: &str, params: &Value) -> bool {
        let window = match id {
            "toolwindow.hideAll" => {
                self.wb.perform("HideAllWindows");
                return true;
            }
            "toolwindow.toggle" => params
                .get("id")
                .and_then(Value::as_str)
                .and_then(CoreWindow::parse),
            other => other
                .strip_prefix("toolwindow.activate.")
                .and_then(CoreWindow::parse),
        };
        let Some(window) = window else {
            return false;
        };
        let Some(kit) = kit_id(window) else {
            self.ide.state_mut().set_status(format!(
                "{} is not available in the terminal yet",
                window.title()
            ));
            return true;
        };
        if id == "toolwindow.toggle" {
            self.wb.toggle(kit);
        } else {
            self.wb.activate(kit);
        }
        true
    }

    /// Mirrors the core's tool window layout into the workbench and the workbench's back.
    pub(crate) fn sync_layout(&mut self) {
        let core_now = {
            let layout = self.ide.state().tool_windows();
            (layout.visible(Anchor::Left), layout.visible(Anchor::Bottom))
        };
        if core_now != self.last_layout {
            for (now, before, anchor) in [
                (core_now.0, self.last_layout.0, ToolWindowAnchor::Left),
                (core_now.1, self.last_layout.1, ToolWindowAnchor::Bottom),
            ] {
                if now == before {
                    continue;
                }
                match now.and_then(kit_id) {
                    Some(id) => {
                        // Shown by a command (a search finished, a task started): do not
                        // steal the keyboard from the editor.
                        self.wb.show(id);
                    }
                    None => {
                        if let Some(visible) = self.wb.visible_window(anchor).map(|w| w.id.clone())
                            && before.and_then(kit_id) == Some(visible.as_str())
                        {
                            self.wb.hide(&visible);
                        }
                    }
                }
            }
        }
        // And back: what the workbench shows is what the core is told.
        let left = self
            .wb
            .visible_window(ToolWindowAnchor::Left)
            .and_then(|w| core_window(&w.id));
        let bottom = self
            .wb
            .visible_window(ToolWindowAnchor::Bottom)
            .and_then(|w| core_window(&w.id));
        {
            let layout = self.ide.state_mut().tool_windows_mut();
            for (visible, anchor) in [(left, Anchor::Left), (bottom, Anchor::Bottom)] {
                if layout.visible(anchor) == visible {
                    continue;
                }
                match visible {
                    Some(window) => layout.show(window),
                    None => {
                        if let Some(current) = layout.visible(anchor) {
                            layout.hide(current);
                        }
                    }
                }
            }
        }
        self.last_layout = (
            self.ide.state().tool_windows().visible(Anchor::Left),
            self.ide.state().tool_windows().visible(Anchor::Bottom),
        );
    }

    /// Everything that follows input: buffers, requests the core made, layout.
    pub(crate) fn after_input(&mut self, rt: &mut Runtime) {
        self.sync_views();
        self.ide.state_mut().record_caret_location();
        self.process_requests(rt);
        self.sync_layout();
        self.refresh_tabs();
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
            && matches!(self.wb.focus, WorkbenchFocus::Editor)
            && !self.views.is_empty()
    }

    pub(crate) fn terminal_focused(&self) -> bool {
        self.layers.is_empty() && self.wb.focused_tool() == Some("terminal")
    }

    /// Starts action `id` as a key press would. Returns whether the IDE took the key.
    fn try_action(&mut self, rt: &mut Runtime, id: &str) -> bool {
        // The kit's own ids for what the IDE calls differently.
        let id = match id {
            "CloseActiveTab" => "CloseContent",
            "ChooseRunConfiguration" => {
                self.open_picker(rt, PickerKind::Configs, 0);
                return true;
            }
            other => other,
        };
        if !self.ide.knows_action(id) {
            return false;
        }
        let editor_focused = self.editor_focused();
        if editor_focused && EDITOR_OWNED.contains(&id) {
            return false;
        }
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
                if let Some(action) = request.downcast_ref::<TuiAction>() {
                    self.run_tui_action(rt, *action);
                }
            }
            ActionEffect::Command { id, params } => self.command(rt, &id, params),
        }
    }

    /// Starts action `id` from a menu or Find Action.
    pub(crate) fn run_action(&mut self, rt: &mut Runtime, id: &str, place: ActionPlace) {
        self.sync_views();
        if let Some(effect) = self.ide.perform(id, place, self.editor_focused()) {
            self.apply_effect(rt, effect);
        } else {
            self.ide
                .state_mut()
                .set_status(format!("{id} is not available here"));
        }
    }

    fn run_tui_action(&mut self, rt: &mut Runtime, action: TuiAction) {
        match action {
            TuiAction::GotoLine => self.input_dialog(
                rt,
                "Go to Line:Column",
                "Line:Column",
                "",
                DialogKind::GotoLine,
                |content| content.placeholder("12 or 12:5"),
            ),
            TuiAction::FindInPath => {
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
            TuiAction::SaveAll => self.command(rt, "editor.saveAll", Value::Null),
            TuiAction::NewFile | TuiAction::NewDirectory => {
                let directory = action == TuiAction::NewDirectory;
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
            TuiAction::OpenProject => {
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
            TuiAction::NextTheme => {
                rt.look = crate::layout::next_theme(&rt.look);
            }
            TuiAction::Exit => {
                self.ide.shutdown();
                self.quit = true;
                rt.quit = true;
            }
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
                self.wb.activate("find");
                self.sync_layout();
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
                    Ok(()) => self.wb.show("project"),
                    Err(error) => self.notify(Status::Error, "Open Project", &error),
                }
                self.after_input(rt);
            }
            DialogKind::Delete { path } => {
                self.command(rt, "project.delete", json!({ "path": path }));
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
        self.wb.focus = WorkbenchFocus::Editor;
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

    // ------------------------------------------------------------------ workbench events

    pub(crate) fn handle_workbench(&mut self, events: Vec<WorkbenchEvent>, rt: &mut Runtime) {
        for event in events {
            if let WorkbenchEvent::Activated(id) = &event
                && id == "terminal"
            {
                self.ensure_terminal();
            }
        }
        self.sync_layout();
        let _ = rt;
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
                let _ = self.ide.dispatch("editor.saveAll", Value::Null);
            }
        }
    }
}

/// Reference actions whose work the terminal editor does itself on its own model.
const EDITOR_OWNED: [&str; 28] = [
    "$Undo",
    "$Redo",
    "$Copy",
    "$Cut",
    "$Paste",
    "$SelectAll",
    "EditorDuplicate",
    "EditorDeleteLine",
    "CommentByLineComment",
    "MoveLineUp",
    "MoveLineDown",
    "SelectNextOccurrence",
    "UnselectPreviousOccurrence",
    "SelectAllOccurrences",
    "EditorAddCaretPerSelectedLine",
    "EditorToggleColumnMode",
    "EditorJoinLines",
    "EditorToggleCase",
    "EditorSplitLine",
    "EditorStartNewLineBefore",
    "EditorSelectWord",
    "EditorUnSelectWord",
    "CollapseRegion",
    "ExpandRegion",
    "CollapseSelection",
    "CollapseAllRegions",
    "ExpandAllRegions",
    "EditorIndentSelection",
];

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
        if let Event::Mouse(_) = event {
            let events = self.wb.handle(rt, event);
            self.handle_workbench(events, rt);
        }
        if let Event::Key(key) = event
            && key.kind != KeyEventKind::Release
        {
            if self.terminal_focused() && self.terminal_key(rt, key) {
                self.after_input(rt);
                return;
            }
            let mut ids = rt.actions().to_vec();
            if rt.chord_completed() {
                // The chord's own action first: `Ctrl+K ]` is Forward, not the editor's `Ctrl+]`.
                let chord = |id: &String| crate::keys::FALLBACKS.iter().any(|(a, _)| a == id);
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
                for id in &ids {
                    if let Some(events) = self.wb.perform(id) {
                        self.handle_workbench(events, rt);
                        self.after_input(rt);
                        return;
                    }
                }
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
        self.sync_all();
        self.draw_all(rt, area, buf);
    }
}

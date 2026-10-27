//! Native workbench shell.
//!
//! Layout follows the observable structure of the New UI "Islands" workbench:
//! a merged main toolbar, a tool window stripe, rounded editor/tool window
//! islands and a status bar with navigation breadcrumbs. Every user action is
//! dispatched through the typed command registry.

mod about;
mod agents_ui;
mod ai_actions;
mod ai_agent;
mod ai_chat;
mod ai_inline;
mod ai_terminal;
mod ai_vcs;
mod android_run_ui;
mod android_sdk;
mod android_view;
mod bazel_window;
mod blame_gutter;
mod branches_popup;
mod build_variants_ui;
mod build_window;
mod change_popup;
mod chrome;
mod clone_dialog;
mod completion_ui;
mod console_links;
mod context_menus;
mod control;
mod crash_notice;
mod debug_dialogs;
mod debug_window;
mod device_manager;
mod diff_ui;
mod disk_banner;
mod doc_ui;
mod editor;
mod editor_keys;
mod external_status;
mod file_icon;
mod find_popup;
mod findbar;
mod first_run;
mod folding;
mod git_log_ui;
mod gradle_banner;
mod gradle_window;
mod gutter_popup;
mod hierarchy_ui;
mod history_ui;
mod icons;
mod index_status;
mod inspections_settings;
mod intentions_ui;
mod keybindings;
mod keys;
mod layout_editor_ui;
mod logcat_ui;
mod lsp_status;
mod main_menu;
mod markdown_ui;
mod merged_manifest_ui;
mod native_dialog;
mod navigation_ui;
mod new_project;
mod notifications;
mod palette;
mod plugin_manager;
mod plugin_ui;
mod popups;
mod problems_ui;
mod push_dialog;
mod refactor_ui;
mod resource_manager_ui;
mod run_config_dialog;
mod running_devices_ui;
mod settings_dialog;
mod settings_keymap;
mod settings_live_templates;
mod settings_pages;
mod structure_view;
mod switcher;
mod syntax;
mod terminal_ui;
mod test_view;
mod theme;
mod theme_engine;
mod todo_window;
mod toolbar_gradient;
mod toolchain_banner;
mod toolchain_status;
mod toolwindows;
#[cfg(test)]
mod ui_audit;
mod welcome;
mod widgets;

use archipel_ide_command::{CommandRegistry, CommandSpec};
use archipel_ide_core::agents::WorkbenchMode;
use archipel_ide_core::{Anchor, IdeState, ToolWindow, built_in_commands};
use archipel_ide_settings::ids;
use eframe::egui;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub use archipel_ide_core::{build_info, diagnostics};
pub use archipel_ide_plugins::{PluginFactory, PluginLoader, PluginSet};
pub use theme::install as install_theme;

pub struct IdeApp {
    state: IdeState,
    commands: CommandRegistry<IdeState>,
    palette: palette::Palette,
    /// Whether Ctrl was down when the Switcher opened (`None` while it is closed).
    switcher_ctrl: Option<bool>,
    terminal_selection: Option<(terminal_ui::GridPoint, terminal_ui::GridPoint)>,
    search_input: String,
    find_popup: find_popup::FindPopup,
    /// Keymap page filter requested by Find Action's Assign Shortcut.
    keymap_search: Option<String>,
    /// Find in Files option toggles and file mask.
    find_options: archipel_ide_core::SearchOptions,
    /// Query and options of the last search shown in the Find inputs, so a
    /// search started elsewhere (`search.project` from automation, the palette
    /// or a context menu) fills them in.
    find_synced: (String, archipel_ide_core::SearchOptions),
    tree_selection: Option<PathBuf>,
    /// Active editor file last revealed in the Project tree.
    tree_revealed: Option<PathBuf>,
    /// Editor context menu, rebuilt only while the popup is open or opening.
    editor_menu: Option<context_menus::Model>,
    window_title: String,
    caret_screen_pos: Option<egui::Pos2>,
    last_shift_release: Option<Instant>,
    shift_tainted: bool,
    last_bottom: Option<ToolWindow>,
    /// Right-anchored tool window seen last frame (Gradle, Bazel, Device Manager).
    last_right: Option<ToolWindow>,
    /// Active editor file seen last frame; opening another one focuses the editor.
    last_active_file: Option<PathBuf>,
    /// A single click in the Project tree opened a preview: keep the keyboard in the tree.
    pub(crate) keep_tree_focus: bool,
    /// Set by controls that open a file with a click (tree double-click): the
    /// editor is focused on the next frame, see `toolwindows::project`.
    pub(crate) focus_editor_next_frame: bool,
    find: findbar::FindBar,
    /// Text of the last whole-line copy, so pasting it inserts a line.
    clipboard_line: Option<String>,
    dialog: Option<popups::Dialog>,
    settings_dialog: Option<settings_dialog::SettingsDialog>,
    dialog_focus: bool,
    /// Pending choices in the first-run "Customize Archipel IDE" wizard.
    customize: Option<first_run::Draft>,
    /// Where the first-run dialog's Skip button was drawn last frame (for UI automation:
    /// the control state reports it so scripts do not depend on the dialog layout).
    first_run_skip: Option<egui::Rect>,
    commit_message: String,
    /// Commit checkbox overrides; by default tracked changes are included and
    /// unversioned files are not.
    commit_overrides: std::collections::HashMap<PathBuf, bool>,
    window_focused: bool,
    /// First stroke of a two-stroke chord waiting for its second stroke.
    pending_chord: Option<archipel_ide_keymap::KeyStroke>,
    /// The main menu titles are expanded in the toolbar.
    menu_bar_open: bool,
    menu_bar_popup_seen: bool,
    /// UI automation socket, when `ARCHIPEL_CONTROL_SOCKET` is set.
    control: Option<control::Control>,
    android_sdk: android_sdk::SdkPage,
    logcat: logcat_ui::LogcatView,
    android_view: android_view::AndroidViewState,
    /// Whether background services can wake this window yet.
    waker_installed: bool,
    repaint_hook_installed: bool,
    /// Contributed command ids currently registered in `commands`.
    plugin_commands: Vec<String>,
    /// Registry generation the contributed commands were synced at.
    plugin_generation: u64,
    /// Amend toggle of the Commit tool window.
    commit_amend: bool,
    /// The OS file dialog implementation (replaced by a fake in tests).
    file_dialogs: std::sync::Arc<dyn native_dialog::FileDialogs>,
    /// A native dialog that is open right now.
    pending_dialog: Option<native_dialog::PendingDialog>,
    /// Java type kinds peeked from disk for file icons.
    kind_cache: std::cell::RefCell<file_icon::KindCache>,
    welcome: welcome::State,
    /// Open push dialog.
    push_dialog: Option<push_dialog::PushDialog>,
    /// Open branch name/confirmation dialog.
    git_dialog: Option<branches_popup::GitDialog>,
    /// Open gutter change-marker popup.
    change_popup: Option<change_popup::ChangePopup>,
    deploy_balloon: android_run_ui::DeployBalloon,
    /// Markdown preview, scroll sync and floating bar state.
    markdown_ui: markdown_ui::State,
    doc_view: doc_ui::DocView,
    ai_ui: ai_chat::AiChatUi,
    agents_ui: agents_ui::State,
}

impl IdeApp {
    /// Opens `project` (if any) and then each of `files`, relative to the project root
    /// or absolute inside it.
    pub fn new(project: Option<PathBuf>, files: Vec<PathBuf>) -> Self {
        Self::with_plugins(project, files, Vec::new())
    }

    /// Like [`Self::new`], with compiled-in plugins. Plugin state is read from
    /// the per-user configuration directory.
    pub fn with_plugins(
        project: Option<PathBuf>,
        files: Vec<PathBuf>,
        bundled: Vec<PluginFactory>,
    ) -> Self {
        Self::with_plugin_set(
            project,
            files,
            PluginSet {
                bundled,
                ..PluginSet::default()
            },
        )
    }

    /// Like [`Self::with_plugins`], also registering script plugin runtimes.
    pub fn with_plugin_set(
        project: Option<PathBuf>,
        files: Vec<PathBuf>,
        plugins: PluginSet,
    ) -> Self {
        let check_updates = archipel_ide_compat::env_var_os("ARCHIPEL_NO_UPDATE_CHECK").is_none();
        let mut app = Self::build(project, files, plugins, check_updates);
        if let Some(dir) = diagnostics::active_log_dir() {
            app.state.check_crash_reports(&dir);
        }
        if let Some(dir) = archipel_ide_settings::default_config_dir() {
            app.state.detect_first_run(&dir);
        }
        // Indexes persist across restarts (tests keep them in memory).
        let index_dir = archipel_ide_compat::env_var_os("ARCHIPEL_INDEX_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(archipel_ide_lsp::registry::cache_dir);
        app.state.enable_index_persistence(Some(index_dir));
        app
    }

    #[cfg(test)]
    fn with_update_check(
        project: Option<PathBuf>,
        files: Vec<PathBuf>,
        check_updates: bool,
    ) -> Self {
        Self::build(project, files, PluginSet::default(), check_updates)
    }

    fn build(
        project: Option<PathBuf>,
        files: Vec<PathBuf>,
        plugins: PluginSet,
        check_updates: bool,
    ) -> Self {
        let commands = built_in_commands();
        let mut state = IdeState::default();
        if let Some(dir) = archipel_ide_settings::default_config_dir() {
            state.load_user_settings(&dir);
        }
        state.init_plugins(archipel_ide_plugins::PluginStore::user(), plugins);
        // Tests use in-memory statistics; the app keeps the user's.
        state.completion_stats = archipel_ide_core::CompletionStats::user();
        if !cfg!(test) {
            state.load_recent_projects();
        }
        state.init_ai(None);
        if let Some(path) = project {
            if let Err(error) = commands.execute("project.open", &mut state, json!({"path": path}))
            {
                state.status = error.to_string();
            }
            let _ = commands.execute("plugin.refresh", &mut state, Value::Null);
        }
        for file in files {
            let relative = state
                .project
                .as_ref()
                .and_then(|project| {
                    let absolute = file.canonicalize().ok()?;
                    absolute
                        .strip_prefix(project.root())
                        .ok()
                        .map(Path::to_path_buf)
                })
                .unwrap_or(file);
            if let Err(error) =
                commands.execute("editor.openFile", &mut state, json!({"path": relative}))
            {
                state.status = error.to_string();
            }
        }
        state.sync_agents_project();
        if state.project.is_some() && agents_demo_enabled() {
            state.seed_agents_demo();
        }
        if check_updates
            && state
                .settings()
                .read()
                .bool(ids::CHECK_UPDATES)
                .unwrap_or(true)
        {
            let _ = commands.execute("app.checkForUpdates", &mut state, json!({"silent": true}));
        }
        Self {
            state,
            commands,
            palette: palette::Palette::default(),
            switcher_ctrl: None,
            terminal_selection: None,
            search_input: String::new(),
            find_popup: Default::default(),
            keymap_search: None,
            find_options: archipel_ide_core::SearchOptions::default(),
            find_synced: Default::default(),
            tree_selection: None,
            tree_revealed: None,
            editor_menu: None,
            window_title: String::new(),
            caret_screen_pos: None,
            last_shift_release: None,
            shift_tainted: false,
            last_bottom: None,
            last_right: None,
            last_active_file: None,
            keep_tree_focus: false,
            focus_editor_next_frame: false,
            find: findbar::FindBar::default(),
            clipboard_line: None,
            dialog: None,
            settings_dialog: None,
            dialog_focus: false,
            customize: None,
            first_run_skip: None,
            commit_message: String::new(),
            commit_overrides: std::collections::HashMap::new(),
            window_focused: true,
            pending_chord: None,
            menu_bar_open: false,
            menu_bar_popup_seen: false,
            control: None,
            android_sdk: android_sdk::SdkPage::default(),
            logcat: logcat_ui::LogcatView::default(),
            android_view: android_view::AndroidViewState::default(),
            waker_installed: false,
            repaint_hook_installed: false,
            plugin_commands: Vec::new(),
            plugin_generation: u64::MAX,
            change_popup: None,
            git_dialog: None,
            push_dialog: None,
            deploy_balloon: android_run_ui::DeployBalloon::default(),
            markdown_ui: markdown_ui::State::default(),
            commit_amend: false,
            doc_view: doc_ui::DocView::default(),
            ai_ui: ai_chat::AiChatUi::default(),
            agents_ui: agents_ui::State::default(),
            file_dialogs: std::sync::Arc::new(native_dialog::SystemDialogs),
            pending_dialog: None,
            kind_cache: Default::default(),
            welcome: Default::default(),
        }
    }

    /// Standalone app mode (`archipel-ide --app <plugin-id>`): a window showing
    /// only the plugin's app view.
    /// Installs missing toolchains whenever a project opens, starting with the open one.
    pub fn enable_toolchain_auto_setup(&mut self) {
        self.state.toolchains().write().auto_setup = true;
        self.state.auto_setup_toolchains();
    }

    pub fn standalone(plugin: &str, plugins: PluginSet) -> Result<Self, String> {
        let mut app = Self::with_plugin_set(None, Vec::new(), plugins);
        plugin_ui::app_mode_error(&app.state.plugin_host.registry, plugin)?;
        app.state.plugin_host.app_mode = Some(plugin.to_owned());
        Ok(app)
    }

    /// Window title for standalone app mode.
    pub fn app_title(&self) -> Option<String> {
        let plugin = self.state.plugin_host.app_mode.as_deref()?;
        let entry = self.state.plugin_host.registry.get(plugin)?;
        Some(entry.manifest.app_title())
    }

    fn dispatch(&mut self, id: &str, params: Value) {
        // Tab navigation walks the agent threads while Agents mode shows them.
        let id = match (self.state.workbench_mode, id) {
            (WorkbenchMode::Agents, "editor.nextTab") => "agents.nextThread",
            (WorkbenchMode::Agents, "editor.previousTab") => "agents.previousThread",
            _ => id,
        };
        if let Err(error) = self.commands.execute(id, &mut self.state, params) {
            diagnostics::log(diagnostics::Level::Warn, &format!("{id}: {error}"));
            self.state.status = error.to_string();
        } else if id == "project.close" {
            self.project_closed();
        }
    }

    /// Resets project-bound UI state after `project.close`.
    fn project_closed(&mut self) {
        self.tree_selection = None;
        self.tree_revealed = None;
        self.commit_overrides.clear();
        self.commit_message.clear();
        self.search_input.clear();
        self.terminal_selection = None;
        self.dialog = None;
        self.find = findbar::FindBar::default();
        self.dispatch("plugin.refresh", Value::Null);
    }

    pub(crate) fn open_dialog(&mut self, dialog: popups::Dialog) {
        self.dialog = Some(dialog);
        self.dialog_focus = true;
    }

    /// File › Open…: the OS folder dialog, or the in-app path field when none exists.
    fn open_project_dialog(&mut self) {
        self.browse(native_dialog::Purpose::OpenProject);
    }

    fn open_project_fallback(&mut self) {
        self.open_dialog(popups::Dialog::OpenProject {
            path: String::new(),
            error: None,
        });
    }

    /// Starts a native folder dialog without blocking the UI; `poll_native_dialog`
    /// delivers the result. A path queued by `dialog.injectPath` answers instead.
    fn browse(&mut self, purpose: native_dialog::Purpose) {
        if self.pending_dialog.is_some() {
            return;
        }
        let (title, start_dir) = match purpose {
            native_dialog::Purpose::OpenProject => ("Open Project", None),
            native_dialog::Purpose::NewProjectLocation => {
                let start = match &self.dialog {
                    Some(popups::Dialog::NewProject(form)) => Some(PathBuf::from(&form.location)),
                    _ => None,
                };
                ("Choose Project Location", start)
            }
            native_dialog::Purpose::CloneLocation => {
                let start = match &self.dialog {
                    Some(popups::Dialog::Clone(form)) => Some(PathBuf::from(&form.parent)),
                    _ => None,
                };
                ("Choose Clone Location", start)
            }
        };
        let dialogs: std::sync::Arc<dyn native_dialog::FileDialogs> =
            match self.state.injected_dialog_paths.pop_front() {
                Some(path) => std::sync::Arc::new(native_dialog::InjectedDialog(path)),
                None => self.file_dialogs.clone(),
            };
        let request = native_dialog::DialogRequest {
            title: title.into(),
            start_dir,
            folder: true,
        };
        self.pending_dialog = Some(native_dialog::PendingDialog::spawn(
            dialogs, purpose, request,
        ));
    }

    fn poll_native_dialog(&mut self, ctx: &egui::Context) {
        let Some(pending) = &self.pending_dialog else {
            return;
        };
        let Ok(outcome) = pending.poll() else {
            ctx.request_repaint_after(Duration::from_millis(50));
            return;
        };
        let purpose = pending.purpose;
        self.pending_dialog = None;
        use native_dialog::{DialogOutcome, Purpose};
        match (purpose, outcome) {
            (_, DialogOutcome::Cancelled) => {}
            (Purpose::OpenProject, DialogOutcome::Picked(path)) => {
                self.dialog = None;
                if let Err(message) = self.open_project_path(&path.to_string_lossy(), ctx) {
                    self.state.status = message;
                }
            }
            (Purpose::NewProjectLocation, DialogOutcome::Picked(path)) => {
                if let Some(popups::Dialog::NewProject(form)) = &mut self.dialog {
                    form.location = path.to_string_lossy().into_owned();
                }
            }
            (Purpose::CloneLocation, DialogOutcome::Picked(path)) => {
                if let Some(popups::Dialog::Clone(form)) = &mut self.dialog {
                    form.set_parent(path.to_string_lossy().into_owned());
                }
            }
            (Purpose::OpenProject, DialogOutcome::Unavailable) => self.open_project_fallback(),
            (Purpose::NewProjectLocation | Purpose::CloneLocation, DialogOutcome::Unavailable) => {
                self.state.status =
                    "No native file dialog is available; type the location instead".into();
            }
        }
    }

    fn new_project_dialog(&mut self) {
        let jdks = self.state.jdk_choices();
        self.open_dialog(popups::Dialog::NewProject(Box::new(
            new_project::Form::new(jdks),
        )));
    }

    /// Get from Version Control (welcome screen, File > New > Project from Version Control).
    fn clone_dialog(&mut self) {
        self.open_dialog(popups::Dialog::Clone(Box::default()));
    }

    fn open_project_path(&mut self, path: &str, ctx: &egui::Context) -> Result<(), String> {
        self.commands
            .execute(
                "project.open",
                &mut self.state,
                json!({"path": path.trim()}),
            )
            .map_err(|error| error.to_string())?;
        self.project_opened(ctx);
        Ok(())
    }

    fn project_opened(&mut self, ctx: &egui::Context) {
        self.dispatch("plugin.refresh", Value::Null);
        self.state.sync_agents_project();
        if agents_demo_enabled() {
            self.state.seed_agents_demo();
        }
        self.tree_selection = None;
        self.tree_revealed = None;
        self.commit_overrides.clear();
        self.commit_message.clear();
        self.search_input.clear();
        self.terminal_selection = None;
        self.find = findbar::FindBar::default();
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "{} – Archipel IDE",
            self.project_name()
        )));
    }

    /// Keeps the OS window title at `project – File.java [module]`.
    fn sync_window_title(&mut self, ctx: &egui::Context) {
        if self.state.project.is_none() {
            return;
        }
        let active = self
            .state
            .active_document
            .and_then(|index| self.state.documents.get(index));
        let title = chrome::window_title(&self.project_name(), active.map(|d| d.path.as_path()));
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }

    /// Settings, the Android SDK page and the Merged Manifest view are modal
    /// pages that read Escape themselves; nothing behind them may consume it.
    fn modal_page_open(&self) -> bool {
        self.settings_dialog.is_some()
            || self.state.settings_request.is_some()
            || self.state.android.sdk_manager_open
            || self.state.android.merged_manifest.open
    }

    fn editor_focused(&self, ctx: &egui::Context) -> bool {
        ctx.memory(|memory| memory.has_focus(editor::editor_id()))
    }

    fn primary_selection_text(&self) -> Option<String> {
        let document = self.state.documents.get(self.state.active_document?)?;
        let primary = document.selections.primary();
        document.text.slice(primary.start(), primary.end()).ok()
    }

    /// Saves modified files silently (autosave); nothing to save is not an error.
    fn autosave(&mut self) {
        if self.state.documents.iter().any(|d| d.text.is_dirty()) {
            self.dispatch("editor.saveAll", Value::Null);
        }
    }

    fn toggle_tool_window(&mut self, window: ToolWindow) {
        self.dispatch("toolwindow.toggle", json!({"id": window.id()}));
    }

    fn file_name(path: &Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
            .to_owned()
    }

    fn project_name(&self) -> String {
        self.state
            .project
            .as_ref()
            .map(|project| Self::file_name(project.root()))
            .unwrap_or_else(|| "No project".into())
    }

    /// Commands that can run without parameters (all declared fields optional).
    fn parameterless(spec: &CommandSpec) -> bool {
        match serde_json::from_str::<Value>(&spec.params) {
            Ok(Value::Object(fields)) => fields
                .values()
                .all(|value| value.as_str().is_some_and(|kind| kind.ends_with('?'))),
            _ => false,
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};
        let completion_open = self
            .state
            .completion
            .as_ref()
            .is_some_and(|session| session.visible());
        let consumed_chord = std::cell::Cell::new(false);
        // Exact modifier matching: egui's consume_key ignores extra Shift/Alt, which
        // would let ⌘F swallow ⇧⌘F.
        let consume = |modifiers: Modifiers, key: Key| {
            let hit = ctx.input_mut(|input| {
                let before = input.events.len();
                input.events.retain(|event| {
                    !matches!(
                        event,
                        egui::Event::Key { key: k, modifiers: m, pressed: true, .. }
                            if *k == key && m.matches_exact(modifiers)
                    )
                });
                input.events.len() != before
            });
            if hit && (modifiers.alt || modifiers.ctrl || modifiers.command) {
                consumed_chord.set(true);
            }
            hit
        };
        if popup_owns_keyboard(ctx) {
            return;
        }
        if completion_open
            && !self.palette.open
            && !self.find_popup.open
            && self.dialog.is_none()
            && self.editor_focused(ctx)
        {
            for (key, delta) in [
                (Key::ArrowDown, 1),
                (Key::ArrowUp, -1),
                (Key::PageDown, 10),
                (Key::PageUp, -10),
            ] {
                if consume(Modifiers::NONE, key) {
                    self.state.move_completion_selection(delta);
                }
            }
            if consume(Modifiers::NONE, Key::Enter) {
                self.dispatch("completion.accept", json!({"replace": false}));
            }
            if consume(Modifiers::NONE, Key::Tab) {
                self.dispatch("completion.accept", json!({"replace": true}));
            }
        } else if self.state.snippet.is_some()
            && !self.palette.open
            && !self.find_popup.open
            && self.dialog.is_none()
            && self.editor_focused(ctx)
        {
            if consume(Modifiers::NONE, Key::Tab) || consume(Modifiers::NONE, Key::Enter) {
                self.dispatch("snippet.next", Value::Null);
            }
            if consume(Modifiers::SHIFT, Key::Tab) {
                self.dispatch("snippet.previous", Value::Null);
            }
        }
        if self.modal_page_open() {
            // The dialog is modal and handles its own keys (Escape closes it).
            return;
        }
        let terminal_focused = (ctx
            .memory(|memory| memory.has_focus(toolwindows::terminal_input_id()))
            || agents_ui::composer_focused(ctx))
            && self.dialog.is_none()
            && !self.palette.open
            && !self.find_popup.open;
        if self.dispatch_keymap(ctx, terminal_focused) {
            consumed_chord.set(true);
        }
        // Archipel IDE additions that the reference keymaps leave unbound.
        if consume(Modifiers::COMMAND, Key::O) {
            self.open_project_dialog();
        }
        if consume(Modifiers::CTRL, Key::Backtick) {
            self.toggle_tool_window(ToolWindow::Terminal);
        }
        // Archipel.ToggleAgentsMode. Ctrl+Alt+A is Git Add in the reference keymaps, so the
        // default is Ctrl+Alt+Shift+X (free in all 26 bundled keymaps).
        if consume(
            Modifiers::COMMAND | Modifiers::ALT | Modifiers::SHIFT,
            Key::X,
        ) {
            self.dispatch("workbench.toggleMode", Value::Null);
        }
        // Archipel.NewAgentThread: Ctrl+Alt+Shift+Y (also unbound in all bundled keymaps).
        if consume(
            Modifiers::COMMAND | Modifiers::ALT | Modifiers::SHIFT,
            Key::Y,
        ) {
            self.dispatch("agents.newThread", Value::Null);
        }
        self.detect_double_shift(ctx);
        // The Run/Debug Configurations modal closes itself on Escape.
        if !terminal_focused
            && !run_config_dialog::is_open(ctx)
            && consume(Modifiers::NONE, Key::Escape)
        {
            if egui::Popup::is_any_open(ctx) {
                // Context menus and main-menu dropdowns close first.
                egui::Popup::close_all(ctx);
            } else if gutter_popup::is_open(ctx) {
                gutter_popup::close(ctx);
            } else if self.state.debugger().state().evaluate_dialog.is_some() {
                self.dispatch("debug.closeEvaluateDialog", Value::Null);
            } else if self.state.debugger().state().editing_breakpoint.is_some() {
                self.dispatch("debug.closeBreakpointEditor", Value::Null);
            } else if self.menu_bar_open {
                self.menu_bar_open = false;
            } else if self.state.analysis.history.dialog.is_some() {
                self.dispatch("history.close", Value::Null);
            } else if self.state.about_open {
                self.dispatch("app.closeAbout", Value::Null);
            } else if self.state.first_run_open {
                self.dispatch("app.skipCustomize", Value::Null);
            } else if self.dialog.is_some() {
                self.dialog = None;
            } else if self.git_dialog.is_some() {
                self.git_dialog = None;
            } else if self.push_dialog.is_some() {
                self.push_dialog = None;
            } else if self.change_popup.is_some() {
                self.change_popup = None;
            } else if self.state.quick_nav.switcher.is_some() {
                self.dispatch("navigate.switcherCancel", Value::Null);
            } else if self.state.ai.pending_apply.is_some() {
                self.dispatch("ai.chat.applyCancel", Value::Null);
            } else if self.palette.open || self.find_popup.open {
                self.palette.open = false;
                self.find_popup.open = false;
            } else if self.state.docs.popup.is_some() {
                self.dispatch("language.closeQuickDoc", Value::Null);
            } else if completion_open {
                // Only a visible popup takes Escape; a hidden session must not
                // swallow the key meant for the find bar or a tool window.
                self.dispatch("completion.close", Value::Null);
            } else if self.state.signature.popup.is_some() {
                self.dispatch("language.closeParameterInfo", Value::Null);
            } else if self.state.snippet.is_some() {
                self.dispatch("snippet.exit", Value::Null);
            } else if self.find.open {
                // One Escape closes the find bar from the query field or the
                // editor. Do not test focus here: egui already surrendered the
                // focused text field on this very Escape, so nothing is focused.
                self.find.open = false;
                ctx.memory_mut(|memory| memory.request_focus(editor::editor_id()));
            } else if self.state.tool_windows.visible(Anchor::Bottom).is_some()
                && !self.palette.open
                && !self.find_popup.open
            {
                // Escape returns focus to the editor; keep the tool window open.
                ctx.memory_mut(|memory| memory.request_focus(editor::editor_id()));
            }
        }
        // Unbound Alt chords (e.g. Alt+7) must not type their digit either; the
        // terminal gets them as shell input.
        if !terminal_focused {
            ctx.input_mut(|input| keys::strip_alt_chord_text(&mut input.events));
        }
        // Some platforms also emit the chord's character as text (e.g. Alt+1 → "1");
        // a consumed shortcut must not type into the focused field.
        if consumed_chord.get() {
            ctx.input_mut(|input| {
                input
                    .events
                    .retain(|event| !matches!(event, egui::Event::Text(_)))
            });
        }
    }

    /// Activating a tool window moves keyboard focus into it; hiding it returns
    /// focus to the editor.
    fn follow_tool_window_focus(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.focus_editor_next_frame)
            && self.state.active_document.is_some()
            && self.dialog.is_none()
            && !self.palette.open
            && !self.find_popup.open
        {
            ctx.memory_mut(|memory| memory.request_focus(editor::editor_id()));
        }
        // Opening or switching to a file (tree, Go to File, search results,
        // automation) moves keyboard focus into its editor, as in the reference.
        let active = self
            .state
            .active_document
            .and_then(|index| self.state.documents.get(index))
            .map(|document| document.path.clone());
        if active != self.last_active_file {
            if active.is_some()
                && self.dialog.is_none()
                && !self.palette.open
                && !self.find_popup.open
                && !self.keep_tree_focus
            {
                ctx.memory_mut(|memory| memory.request_focus(editor::editor_id()));
            }
            self.last_active_file = active;
        }
        self.keep_tree_focus = false;
        // Hiding a right-anchored tool window also hands focus back to the editor.
        let right = self.state.tool_windows.visible(Anchor::Right);
        if right != self.last_right {
            if right.is_none() && self.state.active_document.is_some() {
                ctx.memory_mut(|memory| memory.request_focus(editor::editor_id()));
            }
            self.last_right = right;
        }
        let bottom = self.state.tool_windows.visible(Anchor::Bottom);
        if bottom == self.last_bottom {
            return;
        }
        let target = match bottom {
            Some(ToolWindow::Terminal) => Some(toolwindows::terminal_input_id()),
            Some(ToolWindow::Find) => Some(toolwindows::find_input_id()),
            None if self.state.active_document.is_some() => Some(editor::editor_id()),
            _ => None,
        };
        if let Some(id) = target {
            ctx.memory_mut(|memory| memory.request_focus(id));
        }
        self.last_bottom = bottom;
    }

    /// "Search Everywhere" opens on a quick double press of Shift alone.
    fn detect_double_shift(&mut self, ctx: &egui::Context) {
        // egui reports no key events for Shift itself, so any key or text event
        // while Shift is held means it was used as a modifier.
        let used_as_modifier = ctx.input(|input| {
            input.events.iter().any(|event| {
                matches!(
                    event,
                    egui::Event::Key { pressed: true, .. } | egui::Event::Text(_)
                )
            })
        });
        if used_as_modifier {
            self.shift_tainted = true;
            self.last_shift_release = None;
        }
        let shift_down = ctx.input(|input| input.modifiers.shift_only());
        let was_down = ctx.data(|data| {
            data.get_temp::<bool>(egui::Id::new("shift_down"))
                .unwrap_or(false)
        });
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("shift_down"), shift_down));
        if shift_down && !was_down {
            self.shift_tainted = false;
        }
        if was_down && !shift_down && !self.shift_tainted {
            let now = Instant::now();
            if self
                .last_shift_release
                .is_some_and(|last| now.duration_since(last) < Duration::from_millis(400))
            {
                self.run_double_modifier(archipel_ide_keymap::Modifier::Shift, ctx);
                self.last_shift_release = None;
            } else {
                self.last_shift_release = Some(now);
            }
        }
    }
}

/// `ARCHIPEL_AGENTS_DEMO=1` starts the IDE with a few scripted agent tasks, so Agents mode
/// can be tried before the real backend exists.
fn agents_demo_enabled() -> bool {
    archipel_ide_compat::env_var("ARCHIPEL_AGENTS_DEMO")
        .is_ok_and(|value| matches!(value.trim(), "1" | "true" | "yes" | "on"))
}

/// An open menu (main menu, context menu, toolbar widget popup) owns the
/// keyboard, as in the reference IDE: Escape closes it, and no workbench
/// shortcut fires underneath it. Returns whether a popup was open.
fn popup_owns_keyboard(ctx: &egui::Context) -> bool {
    if !egui::Popup::is_any_open(ctx) {
        return false;
    }
    let escape = ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    if escape {
        egui::Popup::close_all(ctx);
    }
    // Chords pressed while a menu is open must not reach the workbench either.
    ctx.input_mut(|input| {
        input.events.retain(|event| {
            !matches!(
                event,
                egui::Event::Key { modifiers, pressed: true, .. }
                    if modifiers.alt || modifiers.ctrl || modifiers.command
            )
        })
    });
    true
}

impl eframe::App for IdeApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::FRAME.to_normalized_gamma_f32()
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        self.control_before_frame(ctx, raw_input);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.state.shutdown_plugins();
        if self.state.project.is_some() {
            self.autosave();
            self.dispatch("workspace.save", Value::Null);
        }
    }
}

impl IdeApp {
    /// Builds one workbench frame. Separate from `eframe::App::update` so a
    /// headless `egui::Context` can drive it (typing-latency bench).
    pub fn frame(&mut self, ctx: &egui::Context) {
        if !self.waker_installed {
            let ctx = ctx.clone();
            self.state.waker.set(move || ctx.request_repaint());
            self.waker_installed = true;
        }
        self.state.poll_files();
        self.poll_native_dialog(ctx);
        if !self.repaint_hook_installed {
            let ctx = ctx.clone();
            self.state
                .repaint
                .set(std::sync::Arc::new(move || ctx.request_repaint()));
            self.repaint_hook_installed = true;
        }
        // Ctrl+-/= are Collapse/Expand in the editor (IntelliJ keymap), not UI zoom.
        ctx.options_mut(|options| options.zoom_with_keyboard = false);
        if self.state.poll_agents() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        self.state.poll_search();
        self.state.poll_runner();
        self.state.poll_debug();
        if self.state.is_debugging() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        self.state.poll_lsp();
        self.state.poll_analysis();
        if !self.state.analysis.index.is_idle() {
            // Indexing progress changes the status bar without user input.
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        self.state.poll_updates();
        if self.state.poll_android() {
            ctx.request_repaint();
        }
        self.state.poll_git();
        self.state.poll_toolchains();
        if self.state.poll_project_model()
            || self.state.project_model().is_some_and(|m| m.is_busy())
        {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        if self.state.toolchains().read().service.is_busy() {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        self.state.poll_remote();
        if self.state.clone_busy() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if self.state.poll_clone() {
            if matches!(self.dialog, Some(popups::Dialog::Clone(_))) {
                self.dialog = None;
            }
            self.project_opened(ctx);
        }
        if self.state.lsp.is_busy() {
            ctx.request_repaint_after(Duration::from_millis(100));
        } else if self.state.lsp.has_servers() || self.state.android.devices.is_tracking() {
            // Diagnostics, server messages and device changes arrive without user input.
            ctx.request_repaint_after(Duration::from_millis(500));
        }
        self.state.poll_ai();
        let agent_running = self
            .state
            .ai
            .agent
            .as_ref()
            .is_some_and(|a| a.phase == archipel_ide_core::AgentPhase::RunningCommand);
        if self.state.ai.service.is_busy() || agent_running {
            ctx.request_repaint_after(Duration::from_millis(30));
        }
        plugin_ui::sync(self, ctx);
        if let Some(plugin) = self.state.plugin_host.app_mode.clone() {
            plugin_ui::handle_keybindings(self, ctx);
            chrome::window_resize_handles(ctx);
            plugin_ui::app_window(self, ctx, &plugin);
            switcher::frame(self, ctx);
            palette::show(self, ctx);
            find_popup::show(self, ctx);
            switcher::show(self, ctx);
            plugin_ui::notifications(self, ctx);
            self.control_after_frame(ctx);
            return;
        }
        if self.state.search.is_running() || self.state.execution().is_running() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        if self.state.poll_terminals() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        // Like the reference IDE, save all files when the window loses focus.
        let focused = ctx.input(|input| input.focused);
        if self.window_focused
            && !focused
            && self
                .state
                .settings()
                .read()
                .bool(ids::AUTOSAVE_FOCUS)
                .unwrap_or(true)
        {
            self.autosave();
        }
        if !self.window_focused && focused {
            // Pick up changes made in other applications right away.
            self.state.request_git_refresh();
        }
        self.window_focused = focused;
        ai_actions::tick(self, ctx);
        ai_inline::tick(self, ctx);
        ai_inline::handle_keys(self, ctx);
        if self.dialog.is_none()
            && !self.palette.open
            && !self.find_popup.open
            && !ctx.memory(|memory| memory.has_focus(toolwindows::terminal_input_id()))
        {
            navigation_ui::handle_shortcuts(self, ctx);
            intentions_ui::handle(self, ctx);
        }
        self.handle_shortcuts(ctx);
        plugin_ui::handle_keybindings(self, ctx);
        self.follow_tool_window_focus(ctx);
        {
            let settings = self.state.settings();
            theme::apply_settings(ctx, &settings.read());
        }
        self.sync_window_title(ctx);
        if self.modal_page_open() {
            // Command-opened pages must not leave keyboard focus in the editor.
            ctx.memory_mut(|memory| memory.surrender_focus(editor::editor_id()));
        }

        chrome::window_resize_handles(ctx);
        if self.state.project.is_none() && welcome::active(self) {
            welcome::show(self, ctx);
        } else {
            chrome::main_toolbar(self, ctx);
            chrome::status_bar(self, ctx);
        }
        if self.state.project.is_some() && self.state.workbench_mode == WorkbenchMode::Agents {
            agents_ui::show(self, ctx);
        } else if self.state.project.is_some() {
            chrome::left_stripe(self, ctx);
            plugin_ui::right_side(self, ctx);
            toolwindows::left(self, ctx);
            device_manager::right(self, ctx);
            running_devices_ui::right(self, ctx);
            ai_chat::right(self, ctx);
            gradle_window::show(self, ctx);
            bazel_window::show(self, ctx);
            toolwindows::bottom(self, ctx);
            editor::central(self, ctx);
        }
        change_popup::show(self, ctx);
        branches_popup::show_dialog(self, ctx);
        push_dialog::show(self, ctx);
        let lookup = completion_ui::show(self, ctx);
        doc_ui::show_completion_panel(self, ctx, lookup);
        doc_ui::show_quick_doc(self, ctx);
        doc_ui::show_parameter_info(self, ctx);
        navigation_ui::choices_popup(self, ctx);
        intentions_ui::popup(self, ctx);
        refactor_ui::popup(self, ctx);
        switcher::frame(self, ctx);
        palette::show(self, ctx);
        find_popup::show(self, ctx);
        switcher::show(self, ctx);
        popups::show(self, ctx);
        settings_dialog::show(self, ctx);
        android_sdk::show(self, ctx);
        ai_chat::diff_preview(self, ctx);
        if let Some(focus) = self.state.execution().take_edit_configurations_request() {
            run_config_dialog::open(self, ctx, Some(focus.as_str()).filter(|f| !f.is_empty()));
        }
        run_config_dialog::show(self, ctx);
        gutter_popup::show(self, ctx);
        debug_dialogs::show(self, ctx);
        notifications::show(self, ctx);
        agents_ui::notices(self, ctx);
        crash_notice::show(self, ctx);
        first_run::show(self, ctx);
        about::show(self, ctx);
        history_ui::show(self, ctx);
        android_run_ui::show(self, ctx);
        lsp_status::notices(self, ctx);
        plugin_ui::notifications(self, ctx);
        if self.settings_dialog.is_some() {
            settings_dialog::backdrop_to_top(ctx);
        }
        // Include operations started by controls in this frame, and keep
        // polling even when their notification balloon is hidden.
        if self.state.updates.is_busy() {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        self.control_after_frame(ctx);
    }

    /// Read-only view of the IDE state (tests, benches, automation).
    pub fn state(&self) -> &IdeState {
        &self.state
    }

    /// Gives keyboard focus to the editor surface.
    pub fn focus_editor(&self, ctx: &egui::Context) {
        ctx.memory_mut(|memory| memory.request_focus(editor::editor_id()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_closes_an_open_menu_and_shortcuts_do_not_run_under_it() {
        let ctx = egui::Context::default();
        let key = |key, modifiers| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        let menu = egui::Id::new("tree_context_menu");
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::Popup::open_id(ctx, menu)
        });
        assert!(egui::Popup::is_any_open(&ctx));
        let input = egui::RawInput {
            events: vec![
                key(egui::Key::Num6, egui::Modifiers::ALT),
                key(egui::Key::Escape, egui::Modifiers::NONE),
            ],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            assert!(popup_owns_keyboard(ctx));
            let left = ctx.input(|input| input.events.len());
            assert_eq!(left, 0, "Alt+6 and Escape were consumed by the menu");
        });
        assert!(!egui::Popup::is_any_open(&ctx));
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            assert!(!popup_owns_keyboard(ctx), "no menu, shortcuts run normally");
        });
    }

    /// A headless workbench with `file` open, for frame-level UI tests.
    pub(crate) fn headless_app(name: &str, file: &str, text: &str) -> (IdeApp, egui::Context) {
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-ui-test-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(file), text).unwrap();
        let app = IdeApp::new(Some(root.clone()), vec![root.join(file)]);
        let ctx = egui::Context::default();
        install_theme(&ctx);
        (app, ctx)
    }

    pub(crate) fn frame_input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1440.0, 900.0),
            )),
            focused: true,
            events,
            ..Default::default()
        }
    }

    #[test]
    fn escape_in_the_editor_keeps_keyboard_focus() {
        let (mut app, ctx) = headless_app("escape-focus", "a.txt", "hello\n");
        for _ in 0..3 {
            app.focus_editor(&ctx);
            let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        }
        assert!(ctx.memory(|m| m.has_focus(editor::editor_id())));
        let escape = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = ctx.run(frame_input(vec![escape]), |ctx| app.frame(ctx));
        let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        assert!(
            ctx.memory(|m| m.has_focus(editor::editor_id())),
            "Escape with no popup open keeps the editor focused"
        );
        // Typing right after Escape still reaches the buffer.
        let typed = egui::Event::Text("x".into());
        let _ = ctx.run(frame_input(vec![typed]), |ctx| app.frame(ctx));
        assert_eq!(app.state.documents[0].text.text(), "xhello\n");
    }

    #[test]
    fn ctrl_w_extends_the_selection_through_the_keymap() {
        let source = "class A {\n    int f() { return 1 + 42; }\n}\n";
        let (mut app, ctx) = headless_app("extend-selection", "A.java", source);
        for keymap in ["$default", "Default for XWin"] {
            app.dispatch(
                "settings.set",
                json!({"id": "keymap.active", "value": keymap}),
            );
            let caret = source.find("42").unwrap();
            app.state.documents[0]
                .selections
                .set_single(archipel_ide_text::Selection::caret(caret));
            for _ in 0..2 {
                app.focus_editor(&ctx);
                let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
            }
            let press = |key, modifiers| egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            };
            let selected = |app: &IdeApp| {
                let document = &app.state.documents[0];
                let selection = document.selections.primary();
                document
                    .text
                    .slice(selection.start(), selection.end())
                    .unwrap()
            };
            let ctrl_w = press(egui::Key::W, egui::Modifiers::CTRL);
            let _ = ctx.run(frame_input(vec![ctrl_w.clone()]), |ctx| app.frame(ctx));
            assert_eq!(selected(&app), "42", "{keymap}");
            let _ = ctx.run(frame_input(vec![ctrl_w]), |ctx| app.frame(ctx));
            assert_eq!(selected(&app), "1 + 42", "{keymap}");
            let shrink = press(egui::Key::W, egui::Modifiers::CTRL | egui::Modifiers::SHIFT);
            let _ = ctx.run(frame_input(vec![shrink]), |ctx| app.frame(ctx));
            assert_eq!(selected(&app), "42", "{keymap}");
            assert_eq!(app.state.documents.len(), 1, "Ctrl+W never closes the tab");
        }
        let close = egui::Event::Key {
            key: egui::Key::F4,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::CTRL,
        };
        let _ = ctx.run(frame_input(vec![close]), |ctx| app.frame(ctx));
        assert!(app.state.documents.is_empty(), "Ctrl+F4 closes the tab");
    }

    #[test]
    fn opening_a_file_by_command_focuses_its_editor() {
        let (mut app, ctx) = headless_app("open-focus", "a.txt", "hello\n");
        let root = app.state.project.as_ref().unwrap().root().to_owned();
        std::fs::write(root.join("b.txt"), "other\n").unwrap();
        let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        ctx.memory_mut(|m| m.surrender_focus(editor::editor_id()));
        app.dispatch("editor.openFile", json!({"path": "b.txt"}));
        for _ in 0..2 {
            let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        }
        assert!(ctx.memory(|m| m.has_focus(editor::editor_id())));
        let _ = ctx.run(frame_input(vec![egui::Event::Text("x".into())]), |ctx| {
            app.frame(ctx)
        });
        let active = app.state.active_document.unwrap();
        assert_eq!(app.state.documents[active].text.text(), "xother\n");
    }

    #[test]
    fn escape_closes_the_run_configurations_dialog() {
        let (mut app, ctx) = headless_app("run-config-escape", "a.txt", "hello\n");
        app.dispatch("runConfig.edit", Value::Null);
        for _ in 0..2 {
            let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        }
        assert!(run_config_dialog::is_open(&ctx));
        let escape = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let _ = ctx.run(frame_input(vec![escape]), |ctx| app.frame(ctx));
        let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        assert!(!run_config_dialog::is_open(&ctx));
    }

    #[test]
    fn hiding_a_right_tool_window_returns_focus_to_the_editor() {
        let (mut app, ctx) = headless_app("right-focus", "a.txt", "hello\n");
        app.dispatch("toolwindow.toggle", json!({"id": "deviceManager"}));
        for _ in 0..2 {
            let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        }
        ctx.memory_mut(|m| m.surrender_focus(editor::editor_id()));
        app.dispatch("toolwindow.toggle", json!({"id": "deviceManager"}));
        for _ in 0..2 {
            let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        }
        assert!(ctx.memory(|m| m.has_focus(editor::editor_id())));
    }

    #[test]
    fn escape_closes_command_opened_modal_pages() {
        let (mut app, ctx) = headless_app("modal-escape", "a.txt", "hello\n");
        let escape = || {
            frame_input(vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }])
        };
        app.state.android.sdk_manager_open = true;
        for _ in 0..2 {
            let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        }
        let _ = ctx.run(escape(), |ctx| app.frame(ctx));
        assert!(!app.state.android.sdk_manager_open);
        app.dispatch("settings.open", Value::Null);
        for _ in 0..2 {
            let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        }
        assert!(app.settings_dialog.is_some());
        let _ = ctx.run(escape(), |ctx| app.frame(ctx));
        assert!(app.settings_dialog.is_none());
    }

    #[test]
    fn modal_dialogs_render_above_notification_balloons() {
        let (mut app, ctx) = headless_app("balloon-order", "a.txt", "hello\n");
        app.state.notifications().show_update_balloon();
        let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        app.dispatch("settings.open", Value::Null);
        for _ in 0..3 {
            let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        }
        assert!(app.settings_dialog.is_some());
        let balloon = ctx
            .memory(|m| m.area_rect(egui::Id::new("notification_balloon")))
            .expect("the update balloon is shown");
        let dialog = ctx
            .memory(|m| m.area_rect(egui::Id::new("settings_dialog")))
            .expect("the settings dialog is shown");
        let overlap = balloon.intersect(dialog);
        assert!(overlap.is_positive(), "{balloon:?} vs {dialog:?}");
        let top = ctx.layer_id_at(overlap.center()).unwrap();
        assert_eq!(top.id, egui::Id::new("settings_dialog"));
        // Outside the dialog the dimming backdrop still covers the balloon.
        let outside = egui::pos2(balloon.right() - 2.0, balloon.bottom() - 2.0);
        if !dialog.contains(outside) {
            let top = ctx.layer_id_at(outside).unwrap();
            assert_eq!(top.id, egui::Id::new("settings_backdrop"));
        }
    }

    fn run_frames(app: &mut IdeApp, ctx: &egui::Context, until: impl Fn(&IdeApp) -> bool) {
        for _ in 0..200 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| app.frame(ctx));
            if until(app) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("condition not reached");
    }

    fn temp_project(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-dialog-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }

    #[test]
    fn open_project_uses_the_injected_native_dialog() {
        let root = temp_project("open");
        let fake = std::sync::Arc::new(native_dialog::FakeDialogs::new(
            native_dialog::DialogOutcome::Picked(root.clone()),
        ));
        let mut app = IdeApp::with_update_check(None, Vec::new(), false);
        app.file_dialogs = fake.clone();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        app.open_project_dialog();
        run_frames(&mut app, &ctx, |app| app.state.project.is_some());
        assert_eq!(app.state.project.as_ref().unwrap().root(), root);
        assert!(
            app.dialog.is_none(),
            "no in-app picker when the native dialog answers"
        );
        let requests = fake.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].folder);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn unavailable_native_dialog_falls_back_to_the_in_app_picker() {
        let mut app = IdeApp::with_update_check(None, Vec::new(), false);
        app.file_dialogs = std::sync::Arc::new(native_dialog::FakeDialogs::new(
            native_dialog::DialogOutcome::Unavailable,
        ));
        let ctx = egui::Context::default();
        theme::install(&ctx);
        app.open_project_dialog();
        run_frames(&mut app, &ctx, |app| app.dialog.is_some());
        assert_eq!(app.dialog.as_ref().unwrap().kind(), "openProject");
        // A cancelled dialog does nothing.
        app.dialog = None;
        app.file_dialogs = std::sync::Arc::new(native_dialog::FakeDialogs::new(
            native_dialog::DialogOutcome::Cancelled,
        ));
        app.open_project_dialog();
        for _ in 0..20 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| app.frame(ctx));
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(app.dialog.is_none() && app.state.project.is_none());
    }

    #[test]
    fn injected_paths_answer_before_the_system_dialog() {
        let root = temp_project("inject");
        let mut app = IdeApp::with_update_check(None, Vec::new(), false);
        app.file_dialogs = std::sync::Arc::new(native_dialog::FakeDialogs::new(
            native_dialog::DialogOutcome::Cancelled,
        ));
        app.dispatch("dialog.injectPath", json!({"path": root}));
        let ctx = egui::Context::default();
        theme::install(&ctx);
        app.open_project_dialog();
        run_frames(&mut app, &ctx, |app| app.state.project.is_some());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn parameterless_commands_are_detected_from_their_schema() {
        let specs = built_in_commands().specs();
        let find = |id: &str| specs.iter().find(|spec| spec.id == id).unwrap();
        assert!(IdeApp::parameterless(find("editor.save")));
        assert!(IdeApp::parameterless(find("editor.close")));
        assert!(!IdeApp::parameterless(find("editor.openFile")));
        assert!(!IdeApp::parameterless(find("toolwindow.toggle")));
    }
}

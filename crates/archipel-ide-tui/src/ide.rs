//! The IDE core as the terminal frontend drives it: the state, the command registry and the
//! action registry, plus the per-frame polling that the window loop of the GUI does.
//!
//! Everything the frontend does goes through [`Ide::dispatch`] (typed commands) or
//! [`Ide::perform`] (actions asked from a `DataContext`); the frontend never decides whether
//! a command applies and never touches the project, the daemon or the runner itself.

use archipel_ide_actions::{ActionEffect, ActionPlace, ActionRegistry, Presentation};
use archipel_ide_command::{CommandRegistry, CommandSpec};
use archipel_ide_core::ide_actions::{CommandActions, IdeDataProvider};
use archipel_ide_core::{IdeState, built_in_commands};
use archipel_ide_platform::{DataContext, data_keys};
use archipel_ide_plugins::PluginSet;
use archipel_ide_view::{Caps, UiData, ViewCx};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// How the IDE is started.
#[derive(Default)]
pub struct StartOptions {
    /// The project folder to open.
    pub project: Option<PathBuf>,
    /// Files to open, relative to the project or absolute inside it.
    pub files: Vec<PathBuf>,
    /// Compiled-in and script plugins.
    pub plugins: PluginSet,
    /// Install missing toolchains as soon as a project opens.
    pub toolchain_setup: bool,
    /// Tests keep indexes in memory and do not read the user's configuration.
    pub isolated: bool,
}

/// The state and the registries.
pub struct Ide {
    pub(crate) state: IdeState,
    commands: CommandRegistry<IdeState>,
    command_actions: CommandActions,
    plugin_generation: u64,
    plugin_commands: Vec<String>,
    /// What only this frontend knows (the Find bar text, the tree selection, the focused
    /// tool window, the breadcrumbs source), shared with the view builders.
    pub data: UiData,
    /// What the keyboard can send; set by the app from the runtime.
    pub caps: Caps,
}

impl Ide {
    /// Starts the IDE core, opens `options.project` and the files.
    pub fn start(options: StartOptions) -> Self {
        let commands = built_in_commands();
        let mut state = IdeState::default();
        if !options.isolated
            && let Some(dir) = archipel_ide_settings::default_config_dir()
        {
            state.load_user_settings(&dir);
        }
        let store = if options.isolated {
            archipel_ide_plugins::PluginStore::default()
        } else {
            archipel_ide_plugins::PluginStore::user()
        };
        state.init_plugins(store, options.plugins);
        if !options.isolated {
            state.completion_stats = archipel_ide_core::CompletionStats::user();
            state.load_recent_projects();
        }
        state.init_ai(None);
        let mut ide = Self {
            state,
            commands,
            command_actions: CommandActions::default(),
            plugin_generation: u64::MAX,
            plugin_commands: Vec::new(),
            data: UiData::default(),
            caps: Caps::kitty(),
        };
        archipel_ide_view::actions::register(ide.state.action_registry());
        ide.sync_actions();
        archipel_ide_view::actions::register_context_rules(ide.state.action_registry());
        if let Some(path) = options.project {
            // The error is in the status line.
            let _ = ide.open_project(&path);
        }
        for file in options.files {
            ide.open_file_arg(&file);
        }
        ide.state.sync_agents_project();
        if options.toolchain_setup {
            ide.state.toolchains().write().auto_setup = true;
            ide.state.auto_setup_toolchains();
        }
        if !options.isolated {
            let index_dir = archipel_ide_compat::env_var_os("ARCHIPEL_INDEX_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(archipel_ide_lsp::registry::cache_dir);
            ide.state.enable_index_persistence(Some(index_dir));
        }
        ide
    }

    /// Opens a project folder through the same command the GUI uses (project model import
    /// and toolchain checks start from there).
    pub fn open_project(&mut self, path: &Path) -> Result<(), String> {
        self.dispatch("project.open", json!({ "path": path }))?;
        let _ = self.dispatch("plugin.refresh", Value::Null);
        Ok(())
    }

    fn open_file_arg(&mut self, file: &Path) {
        let relative = self
            .state
            .project()
            .and_then(|project| {
                let absolute = file.canonicalize().ok()?;
                absolute
                    .strip_prefix(project.root())
                    .ok()
                    .map(Path::to_path_buf)
            })
            .unwrap_or_else(|| file.to_path_buf());
        let _ = self.dispatch("editor.openFile", json!({ "path": relative }));
    }

    /// Runs a command; an error lands in the status line like in the GUI.
    pub fn dispatch(&mut self, id: &str, params: Value) -> Result<Value, String> {
        match self.commands.execute(id, &mut self.state, params) {
            Ok(value) => Ok(value),
            Err(error) => {
                let message = error.to_string();
                // A disabled command is not a failure: its menu item is greyed out and a key
                // that reaches it does nothing (no status with an internal id).
                if message.starts_with("command is disabled") {
                    return Err(message);
                }
                archipel_ide_core::diagnostics::log(
                    archipel_ide_core::diagnostics::Level::Warn,
                    &format!("{id}: {message}"),
                );
                self.state.set_status(message.clone());
                Err(message)
            }
        }
    }

    /// Whether command `id` is enabled for the current state.
    pub fn command_enabled(&self, id: &str) -> bool {
        self.commands.is_enabled(id, &self.state)
    }

    /// The registered commands.
    pub fn specs(&self) -> Vec<CommandSpec> {
        self.commands.specs()
    }

    /// Keeps the command-backed actions in step with the registered commands.
    pub fn sync_actions(&mut self) {
        let result = self
            .command_actions
            .sync(self.state.application().extensions(), &self.commands);
        if let Err(error) = result {
            archipel_ide_core::diagnostics::log(
                archipel_ide_core::diagnostics::Level::Warn,
                &format!("action registration: {error}"),
            );
        }
    }

    /// A read-only context for the view builders.
    pub fn cx(&self) -> ViewCx<'_> {
        ViewCx::new(&self.state, &self.commands, &self.data, self.caps)
    }

    /// The text of the shortcut of `action` for this keyboard (the GUI's text with the kitty
    /// protocol).
    pub fn shortcut_label(&self, action: &str) -> Option<String> {
        self.cx()
            .shortcut(action)
            .map(|shortcut| shortcut.label(self.caps.notation()))
    }

    /// Runs `f` with the action registry and the current data context.
    pub fn with_data<R>(
        &self,
        extra: impl for<'a> FnOnce(DataContext<'a>) -> DataContext<'a>,
        f: impl FnOnce(ActionRegistry<'_>, &DataContext<'_>) -> R,
    ) -> R {
        let ide = IdeDataProvider::new(&self.state, &self.commands);
        let data = extra(
            DataContext::new()
                .with_provider(&ide)
                .with_provider(&self.data),
        );
        f(self.state.action_registry(), &data)
    }

    /// What action `id` looks like in `place` now; an unknown id is disabled.
    pub fn presentation(&self, id: &str, place: ActionPlace) -> Presentation {
        self.with_data(|d| d, |registry, data| registry.update(id, place, data))
            .unwrap_or_else(Presentation::disabled)
    }

    /// The effect of starting `id` from `place`, `None` when it is unknown or disabled.
    pub fn perform(
        &self,
        id: &str,
        place: ActionPlace,
        editor_focused: bool,
    ) -> Option<ActionEffect> {
        self.with_data(
            |data| data.with_value(&data_keys::EDITOR_FOCUSED, editor_focused),
            |registry, data| registry.perform(id, place, data),
        )
    }

    /// Whether the registry knows action `id`.
    pub fn knows_action(&self, id: &str) -> bool {
        self.state.action_registry().contains(id) || id == "EditorPasteSimple"
    }

    /// Everything the GUI's frame polls, minus what only a window needs. Returns whether
    /// something is still running that changes the screen without input (indexing, a task).
    pub fn poll(&mut self) -> bool {
        let state = &mut self.state;
        state.poll_files();
        state.poll_search();
        state.poll_runner();
        state.poll_debug();
        state.poll_lsp();
        state.poll_analysis();
        state.poll_git();
        state.poll_toolchains();
        let model_busy = state.poll_project_model()
            || state.project_model().is_some_and(|model| model.is_busy());
        state.poll_remote();
        state.poll_text_search();
        state.poll_server_symbols();
        state.poll_terminals();
        let busy = model_busy
            || !state.analysis().index.is_idle()
            || state.search_running()
            || state.execution().is_running()
            || state.lsp().is_busy()
            || state.toolchains().read().service.is_busy();
        // Plugins add and remove commands and queue commands of their own.
        state.poll_plugins();
        let generation = state.plugin_host_service().registry().generation();
        if generation != self.plugin_generation {
            archipel_ide_core::sync_plugin_commands(
                &mut self.commands,
                &mut self.plugin_commands,
                &mut self.state,
            );
            self.sync_actions();
            self.plugin_generation = self.state.plugin_host_service().registry().generation();
        }
        for _ in 0..8 {
            let queued = self.state.plugin_host_service().take_queued_commands();
            if queued.is_empty() {
                break;
            }
            for (id, params) in queued {
                let _ = self.dispatch(&id, params);
            }
        }
        busy
    }

    /// Saves everything the IDE keeps and stops background work.
    pub fn shutdown(&mut self) {
        self.state.shutdown_plugins();
        if self.state.has_project() {
            if self.state.documents().iter().any(|d| d.text.is_dirty()) {
                let _ = self.dispatch("editor.saveAll", Value::Null);
            }
            let _ = self.dispatch("workspace.save", Value::Null);
        }
    }

    /// The state, for reads.
    pub fn state(&self) -> &IdeState {
        &self.state
    }

    /// The state, for what the frontend refreshes in place (lazy highlights, the viewport).
    pub fn state_mut(&mut self) -> &mut IdeState {
        &mut self.state
    }
}

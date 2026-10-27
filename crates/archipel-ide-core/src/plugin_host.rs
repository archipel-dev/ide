//! Plugin host: activates plugins, gives each a scoped [`PluginHost`], routes
//! contributed commands, and keeps plugin tool window / editor tab state.

use crate::{IdeState, spec, string_param, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry, CommandResult};
use archipel_ide_plugins::{
    EditorTabRequest, Notification, Permission, Plugin, PluginError, PluginHost, PluginManifest,
    PluginRegistry, PluginRuntime, PluginSet, PluginStatus, PluginStore, StatusWidgetState,
    ViewContext, ViewPlacement, user_config_dir,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A plugin view open in the editor area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginTab {
    pub plugin: String,
    pub view: String,
    pub title: String,
    pub key: String,
    pub path: Option<PathBuf>,
}

impl PluginTab {
    pub fn context(&self) -> ViewContext {
        ViewContext {
            view: self.view.clone(),
            placement: ViewPlacement::EditorTab {
                key: self.key.clone(),
                path: self.path.clone(),
            },
        }
    }
}

/// A status bar widget contributed by an active plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginStatusWidget {
    pub plugin: String,
    pub id: String,
    pub state: StatusWidgetState,
    pub command: Option<String>,
}

#[derive(Default)]
pub struct PluginHostState {
    pub registry: PluginRegistry,
    /// Commands queued by plugins via [`PluginHost::dispatch`]; the UI runs them.
    pub queued_commands: Vec<(String, Value)>,
    /// Visible plugin tool window `(plugin, tool window id)` on the right anchor.
    pub tool_window: Option<(String, String)>,
    pub tabs: Vec<PluginTab>,
    /// Active plugin tab; while set it is shown instead of the active document.
    pub active_tab: Option<usize>,
    /// Plugin shown alone in standalone app mode (`--app <id>`).
    pub app_mode: Option<String>,
    /// Per-user directory that `.archipelplugin` archives install into.
    pub install_dir: Option<PathBuf>,
    pub repaint: bool,
    refresh: BTreeSet<String>,
}

/// The per-user directory with installed plugins (`<config>/plugins`).
pub fn user_plugin_dir() -> Option<PathBuf> {
    user_config_dir().map(|dir| dir.join("plugins"))
}

/// Directories searched for plugin manifests, in priority order, with a label
/// for the Plugins page: `ARCHIPEL_PLUGINS_DIR`, `plugins/` next to the
/// executable, the per-user directory and the project's `plugins/`.
pub fn plugin_dirs(project: Option<&Path>) -> Vec<(&'static str, PathBuf)> {
    let mut dirs = Vec::new();
    if let Some(dir) =
        archipel_ide_compat::env_var_os("ARCHIPEL_PLUGINS_DIR").filter(|v| !v.is_empty())
    {
        dirs.push(("Bundled", PathBuf::from(dir)));
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("plugins")))
    {
        dirs.push(("Bundled", dir));
    }
    if let Some(dir) = user_plugin_dir() {
        dirs.push(("User", dir));
    }
    if let Some(project) = project {
        dirs.push(("Project", project.join("plugins")));
    }
    dirs.dedup_by(|a, b| a.1 == b.1);
    dirs
}

/// Local plugin registries listed by the Marketplace tab, in priority order:
/// `ARCHIPEL_REGISTRY_DIR`, `registry/` next to the executable and the per-user
/// `registry` directory. Each holds `.archipelplugin` archives.
pub fn plugin_registry_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) =
        archipel_ide_compat::env_var_os("ARCHIPEL_REGISTRY_DIR").filter(|v| !v.is_empty())
    {
        dirs.push(PathBuf::from(dir));
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("registry")))
    {
        dirs.push(dir);
    }
    if let Some(dir) = user_config_dir() {
        dirs.push(dir.join("registry"));
    }
    dirs
}

struct HostAdapter<'a> {
    state: &'a mut IdeState,
    id: String,
}

impl PluginHost for HostAdapter<'_> {
    fn plugin_id(&self) -> &str {
        &self.id
    }

    fn project_root(&self) -> Option<&Path> {
        self.state.project.as_ref().map(|project| project.root())
    }

    fn show_status(&mut self, message: &str) {
        self.state.status = message.into();
    }

    fn notify(&mut self, notification: Notification) {
        self.state
            .notifications()
            .notify_plugin(&self.id, notification);
    }

    fn dispatch(&mut self, command: &str, params: Value) {
        self.state
            .plugin_host_service()
            .queue_command(command, params);
    }

    fn open_tool_window(&mut self, id: &str) {
        self.state.show_plugin_tool_window(&self.id.clone(), id);
    }

    fn open_editor_tab(&mut self, request: EditorTabRequest) {
        self.state.open_plugin_tab(&self.id.clone(), request);
    }

    fn setting(&self, key: &str) -> Option<Value> {
        self.state.plugin_setting(&self.id, key)
    }

    fn set_setting(&mut self, key: &str, value: Value) -> Result<(), PluginError> {
        self.state
            .plugin_host_service()
            .registry_mut()
            .store
            .set_setting(&self.id, key, value)
    }

    fn data_dir(&self) -> Result<PathBuf, PluginError> {
        let dir = self
            .state
            .plugin_host_service()
            .registry()
            .store
            .data_dir(&self.id)
            .ok_or_else(|| {
                PluginError::Io(
                    self.id.clone(),
                    "no per-user configuration directory (set ARCHIPEL_CONFIG_DIR)".into(),
                )
            })?;
        std::fs::create_dir_all(&dir)
            .map_err(|error| PluginError::Io(dir.display().to_string(), error.to_string()))?;
        Ok(dir)
    }

    fn has_permission(&self, permission: Permission) -> bool {
        self.state
            .plugin_host_service()
            .registry()
            .get(&self.id)
            .is_some_and(|entry| entry.manifest.has_permission(permission))
    }

    fn refresh_contributions(&mut self) {
        self.state
            .plugin_host_service()
            .write()
            .refresh
            .insert(self.id.clone());
    }

    fn request_repaint(&mut self) {
        self.state.plugin_host_service().write().repaint = true;
    }
}

impl IdeState {
    /// Sets up the registry with the user's store, compiled-in plugins and the
    /// per-user plugin directory, then activates enabled plugins.
    pub fn init_plugins(&mut self, store: PluginStore, set: PluginSet) {
        let mut registry = PluginRegistry::new(store);
        let mut runtime = PluginRuntime::default();
        for loader in set.loaders {
            runtime.add_loader(loader);
        }
        for factory in set.bundled {
            if let Err(error) = registry.add_bundled(&mut runtime, factory) {
                registry.problems.push((PathBuf::from("<bundled>"), error));
            }
        }
        self.plugin_runtime = runtime;
        {
            let plugins = self.plugin_host_service();
            let mut state = plugins.write();
            state.registry = registry;
            state.install_dir = user_plugin_dir();
        }
        self.discover_plugins();
    }

    /// Rediscovers manifests in the plugin directories ([`plugin_dirs`]) and
    /// activates newly enabled plugins. The per-user directory is the install
    /// directory (`.archipelplugin` archives are unpacked there).
    pub fn discover_plugins(&mut self) {
        let project = self.project.as_ref().map(|p| p.root().to_owned());
        let install_dir = self.plugin_host_service().install_dir();
        let roots: Vec<PathBuf> = plugin_dirs(project.as_deref())
            .into_iter()
            .filter_map(|(label, dir)| {
                if label == "User" {
                    install_dir.clone()
                } else {
                    Some(dir)
                }
            })
            .collect();
        for root in roots {
            self.plugin_host_service()
                .registry_mut()
                .discover(&mut self.plugin_runtime, &root);
        }
        self.activate_pending_plugins();
    }

    pub fn activate_pending_plugins(&mut self) {
        let plugins = self.plugin_host_service();
        let pending = plugins.registry().pending_activation();
        for id in pending {
            let problem = plugins.registry().dependency_problem(&id);
            if let Some(problem) = problem {
                plugins.set_status(&id, PluginStatus::Failed(problem));
                continue;
            }
            if !self.plugin_runtime.has_code(&id) {
                // Declarative plugins have nothing to run.
                plugins.set_status(&id, PluginStatus::Active);
                continue;
            }
            let ensured = self
                .plugin_runtime
                .ensure_instance(&plugins.registry(), &id);
            if let Err(error) = ensured {
                plugins.set_status(&id, PluginStatus::Failed(error.to_string()));
                continue;
            }
            let Some(mut instance) = self.plugin_runtime.take_instance(&id) else {
                continue;
            };
            let result = {
                let mut host = HostAdapter {
                    state: self,
                    id: id.clone(),
                };
                instance.activate(&mut host)
            };
            let contributions = instance.contributions();
            let problems = contributions.problems();
            self.plugin_runtime.restore_instance(&id, instance);
            let status = match result {
                Err(error) => PluginStatus::Failed(error.to_string()),
                Ok(()) if !problems.is_empty() => PluginStatus::Failed(problems.join("; ")),
                Ok(()) => PluginStatus::Active,
            };
            plugins.registry_mut().set_contributions(&id, contributions);
            plugins.set_status(&id, status);
        }
    }

    /// Runs `f` with the active plugin `id` and a host scoped to it.
    pub fn with_plugin<R>(
        &mut self,
        id: &str,
        f: impl FnOnce(&mut dyn Plugin, &mut dyn PluginHost) -> R,
    ) -> Option<R> {
        if !self.plugin_host_service().is_active(id) {
            return None;
        }
        let mut instance = self.plugin_runtime.take_instance(id)?;
        let result = {
            let mut host = HostAdapter {
                state: self,
                id: id.into(),
            };
            f(instance.as_mut(), &mut host)
        };
        let plugins = self.plugin_host_service();
        let refresh = plugins.write().refresh.remove(id);
        if refresh {
            plugins
                .registry_mut()
                .set_contributions(id, instance.contributions());
        }
        self.plugin_runtime.restore_instance(id, instance);
        Some(result)
    }

    /// Enables or disables a plugin, (de)activating its code immediately.
    pub fn set_plugin_enabled(&mut self, id: &str, enabled: bool) -> Result<(), PluginError> {
        if !enabled {
            self.deactivate_plugin(id);
        }
        self.plugin_host_service()
            .registry_mut()
            .set_enabled(id, enabled)?;
        if enabled {
            self.activate_pending_plugins();
        }
        Ok(())
    }

    /// Runs `deactivate`, drops the instance and closes the plugin's views.
    fn deactivate_plugin(&mut self, id: &str) {
        let plugins = self.plugin_host_service();
        if plugins.is_active(id) {
            self.with_plugin(id, |plugin, host| plugin.deactivate(host));
            // A fresh instance is created on re-enable.
            drop(self.plugin_runtime.take_instance(id));
            {
                let mut state = plugins.write();
                state.tabs.retain(|tab| tab.plugin != id);
                state.active_tab = None;
            }
            if plugins
                .tool_window()
                .is_some_and(|(plugin, _)| plugin == id)
            {
                plugins.set_tool_window(None);
            }
        }
    }

    /// Whether `id` was installed from an archive and can be uninstalled.
    pub fn plugin_uninstallable(&self, id: &str) -> bool {
        let plugins = self.plugin_host_service();
        let Some(install_dir) = plugins.install_dir() else {
            return false;
        };
        plugins.registry().get(id).is_some_and(|entry| {
            matches!(&entry.source, archipel_ide_plugins::PluginSource::Directory(dir)
                if dir.parent() == Some(install_dir.as_path()))
        })
    }

    fn install_dir(&self) -> Result<PathBuf, PluginError> {
        self.plugin_host_service().install_dir().ok_or_else(|| {
            PluginError::Io(
                "plugins".into(),
                "no per-user configuration directory; set ARCHIPEL_CONFIG_DIR".into(),
            )
        })
    }

    /// Installs a `.archipelplugin` archive into the per-user plugin directory and
    /// activates it (replacing an installed version without a restart).
    pub fn install_plugin_archive(
        &mut self,
        archive: &Path,
    ) -> Result<PluginManifest, PluginError> {
        let install_dir = self.install_dir()?;
        let manifest = archipel_ide_plugins::archive::read_manifest(archive)?;
        let provider = self
            .plugin_host_service()
            .registry()
            .get(&manifest.id)
            .map(|entry| entry.source.clone());
        if let Some(source) = provider
            && !self.plugin_uninstallable(&manifest.id)
        {
            return Err(PluginError::Manifest(format!(
                "{} is already provided by {}; it cannot be replaced from an archive",
                manifest.id,
                match &source {
                    archipel_ide_plugins::PluginSource::Bundled =>
                        "this Archipel IDE build".to_owned(),
                    archipel_ide_plugins::PluginSource::Directory(dir) => dir.display().to_string(),
                }
            )));
        }
        self.deactivate_plugin(&manifest.id);
        // Rediscovery must load the new files, not keep the old entry.
        self.plugin_host_service()
            .registry_mut()
            .remove(&mut self.plugin_runtime, &manifest.id);
        let manifest = archipel_ide_plugins::archive::install(archive, &install_dir)?;
        self.discover_plugins();
        self.status = format!("Installed plugin {} {}", manifest.name, manifest.version);
        Ok(manifest)
    }

    /// Deactivates and removes an installed plugin.
    pub fn uninstall_plugin(&mut self, id: &str) -> Result<(), PluginError> {
        if self.plugin_host_service().registry().get(id).is_none() {
            return Err(PluginError::NotFound(id.into()));
        }
        if !self.plugin_uninstallable(id) {
            return Err(PluginError::Manifest(format!(
                "{id} is not an installed plugin (bundled and project plugins cannot be uninstalled)"
            )));
        }
        let install_dir = self.install_dir()?;
        self.deactivate_plugin(id);
        archipel_ide_plugins::archive::uninstall(&install_dir, id)?;
        // Forget the user's enable choice together with the plugin.
        self.plugin_host_service()
            .registry_mut()
            .store
            .set_disabled(id, false)?;
        self.discover_plugins();
        self.status = format!("Uninstalled plugin {id}");
        Ok(())
    }

    /// Deactivates every plugin (IDE exit).
    pub fn shutdown_plugins(&mut self) {
        let active: Vec<String> = self
            .plugin_host_service()
            .registry()
            .entries()
            .iter()
            .filter(|entry| entry.is_active())
            .map(|entry| entry.id().to_owned())
            .collect();
        for id in active {
            self.with_plugin(&id, |plugin, host| plugin.deactivate(host));
        }
    }

    pub fn execute_plugin_command(&mut self, command: &str, params: Value) -> CommandResult {
        let owner = self
            .plugin_host_service()
            .registry()
            .command_owner(command)
            .map(str::to_owned)
            .ok_or_else(|| CommandError::Disabled(command.into()))?;
        self.with_plugin(&owner, |plugin, host| plugin.execute(command, params, host))
            .unwrap_or_else(|| Err(CommandError::Disabled(command.into())))
    }

    /// Lets active plugins collect background results.
    pub fn poll_plugins(&mut self) {
        let ids: Vec<String> = self
            .plugin_host_service()
            .registry()
            .entries()
            .iter()
            .filter(|entry| entry.is_active() && self.plugin_runtime.has_code(entry.id()))
            .map(|entry| entry.id().to_owned())
            .collect();
        for id in ids {
            self.with_plugin(&id, |plugin, host| plugin.poll(host));
        }
    }

    /// Takes and clears the repaint request set by plugins.
    pub fn take_plugin_repaint(&mut self) -> bool {
        std::mem::take(&mut self.plugin_host_service().write().repaint)
    }

    pub fn plugin_status_widgets(&self) -> Vec<PluginStatusWidget> {
        let plugins = self.plugin_host_service();
        let registry = plugins.registry();
        registry
            .entries()
            .iter()
            .filter(|entry| entry.is_active())
            .flat_map(|entry| {
                entry
                    .contributions
                    .status_widgets
                    .iter()
                    .filter_map(|widget| {
                        // Instances are only absent while a plugin call is running.
                        let state = self.plugin_status_widget(entry.id(), &widget.id)?;
                        Some(PluginStatusWidget {
                            plugin: entry.id().to_owned(),
                            id: widget.id.clone(),
                            state,
                            command: widget.command.clone(),
                        })
                    })
            })
            .collect()
    }

    /// The active plugin editor view contributed for `path`'s extension, if any.
    pub fn plugin_editor_for(&self, path: &Path) -> Option<(String, String)> {
        let extension = path.extension()?.to_str()?;
        let plugins = self.plugin_host_service();
        let registry = plugins.registry();
        registry
            .entries()
            .iter()
            .filter(|entry| entry.is_active())
            .find_map(|entry| {
                let editor = entry.contributions.editors.iter().find(|editor| {
                    editor
                        .file_extensions
                        .iter()
                        .any(|e| e.trim_start_matches('.').eq_ignore_ascii_case(extension))
                })?;
                Some((entry.id().to_owned(), editor.view.clone()))
            })
    }

    /// Opens `path` in the plugin editor that handles its extension. Returns
    /// whether one did.
    pub fn open_in_plugin_editor(&mut self, path: &Path) -> bool {
        let Some((plugin, view)) = self.plugin_editor_for(path) else {
            return false;
        };
        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let request = EditorTabRequest::new(view, title)
            .key(path.to_string_lossy())
            .path(path);
        self.open_plugin_tab(&plugin, request);
        self.touch_recent(path);
        true
    }

    fn plugin_status_widget(&self, plugin: &str, id: &str) -> Option<StatusWidgetState> {
        self.plugin_runtime
            .instance(plugin)
            .and_then(|instance| instance.status_widget(id))
    }

    /// Stored value of a plugin setting, or the default declared on its settings page.
    pub fn plugin_setting(&self, plugin: &str, key: &str) -> Option<Value> {
        let plugins = self.plugin_host_service();
        let registry = plugins.registry();
        registry.store.setting(plugin, key).cloned().or_else(|| {
            registry.get(plugin).and_then(|entry| {
                entry
                    .contributions
                    .settings_pages
                    .iter()
                    .flat_map(|page| &page.fields)
                    .find(|field| field.key == key)
                    .map(|field| field.kind.default_value())
            })
        })
    }

    pub fn set_plugin_setting(
        &mut self,
        plugin: &str,
        key: &str,
        value: Value,
    ) -> Result<(), String> {
        let plugins = self.plugin_host_service();
        let kind = {
            let registry = plugins.registry();
            let entry = registry
                .get(plugin)
                .ok_or_else(|| format!("plugin not found: {plugin}"))?;
            entry
                .contributions
                .settings_pages
                .iter()
                .flat_map(|page| &page.fields)
                .find(|field| field.key == key)
                .map(|field| field.kind.clone())
                .ok_or_else(|| format!("plugin {plugin} has no setting `{key}`"))?
        };
        let value = kind.coerce(value)?;
        plugins
            .registry_mut()
            .store
            .set_setting(plugin, key, value)
            .map_err(|error| error.to_string())
    }

    fn show_plugin_tool_window(&mut self, plugin: &str, id: &str) {
        self.plugin_host_service()
            .set_tool_window(Some((plugin.into(), id.into())));
        // Plugin tool windows share the right anchor with the built-in ones.
        self.tool_windows.hide(crate::ToolWindow::DeviceManager);
        self.tool_windows.hide(crate::ToolWindow::RunningDevices);
    }

    /// Toggles a contributed tool window; returns whether it is visible.
    pub fn toggle_plugin_tool_window(&mut self, plugin: &str, id: &str) -> Result<bool, String> {
        let plugins = self.plugin_host_service();
        let known = plugins.registry().get(plugin).is_some_and(|entry| {
            entry.is_active() && entry.contributions.tool_windows.iter().any(|w| w.id == id)
        });
        if !known {
            return Err(format!("plugin {plugin} has no active tool window `{id}`"));
        }
        let target = (plugin.to_owned(), id.to_owned());
        if plugins.tool_window().as_ref() == Some(&target) {
            plugins.set_tool_window(None);
            Ok(false)
        } else {
            plugins.set_tool_window(Some(target));
            self.tool_windows.hide(crate::ToolWindow::DeviceManager);
            self.tool_windows.hide(crate::ToolWindow::RunningDevices);
            Ok(true)
        }
    }

    /// Opens (or focuses the tab with the same key of) a plugin editor view.
    pub fn open_plugin_tab(&mut self, plugin: &str, request: EditorTabRequest) {
        {
            let plugins = self.plugin_host_service();
            let mut state = plugins.write();
            let tabs = &mut state.tabs;
            let index = match tabs
                .iter()
                .position(|tab| tab.plugin == plugin && tab.key == request.key)
            {
                Some(index) => {
                    tabs[index].title = request.title;
                    index
                }
                None => {
                    tabs.push(PluginTab {
                        plugin: plugin.into(),
                        view: request.view,
                        title: request.title,
                        key: request.key,
                        path: request.path,
                    });
                    tabs.len() - 1
                }
            };
            state.active_tab = Some(index);
        }
        self.diff_active = false;
    }

    pub fn close_plugin_tab(&mut self, index: usize) -> Result<(), String> {
        let plugins = self.plugin_host_service();
        let mut state = plugins.write();
        if index >= state.tabs.len() {
            return Err("plugin tab does not exist".into());
        }
        state.tabs.remove(index);
        state.active_tab = match state.active_tab {
            Some(active) if active == index => None,
            Some(active) if active > index => Some(active - 1),
            other => other,
        };
        Ok(())
    }

    /// The plugin tab that is shown instead of the active document, if any.
    pub fn active_plugin_tab(&self) -> Option<PluginTab> {
        let plugins = self.plugin_host_service();
        let state = plugins.read();
        state.tabs.get(state.active_tab?).cloned()
    }
}

/// Re-registers contributed commands after plugin changes. `registered`
/// tracks the ids this function added; built-in commands are never replaced.
pub fn sync_plugin_commands(
    registry: &mut CommandRegistry<IdeState>,
    registered: &mut Vec<String>,
    state: &mut IdeState,
) {
    for id in registered.drain(..) {
        registry.unregister(&id);
    }
    for (plugin, spec) in state.plugin_host_service().registry().active_commands() {
        let id = spec.id.clone();
        if registry.contains(&id) {
            state.status =
                format!("Plugin {plugin}: command {id} conflicts with an existing command");
            continue;
        }
        let command_id = id.clone();
        let owner = plugin.clone();
        let result = registry.register(Command::new(
            spec,
            move |state: &IdeState| state.plugin_host_service().is_active(&owner),
            move |state: &mut IdeState, params| state.execute_plugin_command(&command_id, params),
        ));
        if result.is_ok() {
            registered.push(id);
        }
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let mut add = |command| registry.register(command).expect("unique built-in id");
    add(Command::new(
        spec("plugin.refresh", "Refresh Plugins", "Plugins", None, "{}"),
        |_| true,
        |state: &mut IdeState, _| {
            state.discover_plugins();
            Ok(json!({"count": state.plugin_host_service().registry().entries().len()}))
        },
    ));
    add(Command::new(
        // Not under "Plugins": the JSON result is for the CLI, not the File menu.
        spec("plugin.list", "List Plugins", "Developer", None, "{}"),
        |_| true,
        |state: &mut IdeState, _| {
            let plugins: Vec<Value> = state
                .plugin_host_service().registry()
                .entries()
                .iter()
                .map(|entry| {
                    json!({
                        "id": entry.id(),
                        "name": entry.manifest.name,
                        "version": entry.manifest.version,
                        "kind": entry.manifest.kind.label(),
                        "status": entry.status.label(),
                        "reason": match &entry.status {
                            PluginStatus::Failed(reason) | PluginStatus::Unsupported(reason) => Some(reason),
                            _ => None,
                        },
                    })
                })
                .collect();
            Ok(json!({"plugins": plugins}))
        },
    ));
    for (id, label, enabled) in [
        ("plugin.enable", "Enable Plugin", true),
        ("plugin.disable", "Disable Plugin", false),
    ] {
        add(Command::new(
            spec(id, label, "Plugins", None, r#"{"id":"string"}"#),
            |_| true,
            move |state: &mut IdeState, params| {
                let id = string_param(&params, "id")?;
                state
                    .set_plugin_enabled(&id, enabled)
                    .map_err(|error| CommandError::Failed(error.to_string()))?;
                Ok(
                    json!({"id": id, "status": state.plugin_host_service().registry().get(&id).map(|e| e.status.label())}),
                )
            },
        ));
    }
    add(Command::new(
        spec("plugin.manager.open", "Plugins…", "Plugins", None, "{}"),
        |_| true,
        |state: &mut IdeState, _| {
            state.settings_request = Some("preferences.pluginManager".into());
            Ok(Value::Null)
        },
    ));
    add(Command::new(
        spec(
            "plugin.install",
            "Install Plugin from Disk",
            "Plugins",
            None,
            r#"{"path":"string"}"#,
        ),
        |_| true,
        |state: &mut IdeState, params| {
            let path = string_param(&params, "path")?;
            let manifest = state
                .install_plugin_archive(Path::new(path.trim()))
                .map_err(|error| CommandError::Failed(error.to_string()))?;
            Ok(json!({
                "id": manifest.id,
                "version": manifest.version,
                "status": state.plugin_host_service().registry().get(&manifest.id).map(|e| e.status.label()),
            }))
        },
    ));
    add(Command::new(
        spec(
            "plugin.uninstall",
            "Uninstall Plugin",
            "Plugins",
            None,
            r#"{"id":"string"}"#,
        ),
        |_| true,
        |state: &mut IdeState, params| {
            let id = string_param(&params, "id")?;
            state
                .uninstall_plugin(&id)
                .map_err(|error| CommandError::Failed(error.to_string()))?;
            Ok(json!({"id": id}))
        },
    ));
    add(Command::new(
        spec(
            "plugin.toolwindow.toggle",
            "Toggle Plugin Tool Window",
            "Window",
            None,
            r#"{"plugin":"string","id":"string"}"#,
        ),
        |_| true,
        |state: &mut IdeState, params| {
            let visible = state
                .toggle_plugin_tool_window(
                    &string_param(&params, "plugin")?,
                    &string_param(&params, "id")?,
                )
                .map_err(CommandError::Failed)?;
            Ok(json!({"visible": visible}))
        },
    ));
    add(Command::new(
        spec(
            "plugin.tab.activate",
            "Activate Plugin Tab",
            "Window",
            None,
            r#"{"index":"number?"}"#,
        ),
        |_| true,
        |state: &mut IdeState, params| {
            // Without an index the editor (or the standalone app view) is shown again.
            if params.get("index").is_none_or(Value::is_null) {
                state.plugin_host_service().set_active_tab(None);
                return Ok(Value::Null);
            }
            let index = usize_param(&params, "index")?;
            if index >= state.plugin_host_service().tab_count() {
                return Err(CommandError::InvalidParams(
                    "plugin tab does not exist".into(),
                ));
            }
            state.plugin_host_service().set_active_tab(Some(index));
            state.diff_active = false;
            Ok(Value::Null)
        },
    ));
    add(Command::new(
        spec(
            "plugin.tab.close",
            "Close Plugin Tab",
            "Window",
            None,
            r#"{"index":"number?"}"#,
        ),
        |state: &IdeState| state.plugin_host_service().active_tab().is_some(),
        |state: &mut IdeState, params| {
            let index = match params.get("index") {
                Some(_) => usize_param(&params, "index")?,
                None => state
                    .plugin_host_service()
                    .active_tab()
                    .ok_or_else(|| CommandError::Failed("no plugin tab is active".into()))?,
            };
            state
                .close_plugin_tab(index)
                .map_err(CommandError::Failed)?;
            Ok(Value::Null)
        },
    ));
    add(Command::new(
        spec(
            "plugin.setSetting",
            "Change Plugin Setting",
            "Plugins",
            None,
            r#"{"plugin":"string","key":"string","value":"any"}"#,
        ),
        |_| true,
        |state: &mut IdeState, params| {
            let plugin = string_param(&params, "plugin")?;
            let key = string_param(&params, "key")?;
            let value = params
                .get("value")
                .cloned()
                .ok_or_else(|| CommandError::InvalidParams("missing value".into()))?;
            state
                .set_plugin_setting(&plugin, &key, value)
                .map_err(CommandError::Failed)?;
            Ok(Value::Null)
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;
    use archipel_ide_plugins::{
        CommandContribution, Contributions, PluginManifest, SettingField, SettingKind,
        SettingsPage, StatusWidgetContribution, ToolWindowContribution,
    };
    use std::rc::Rc;

    struct Counter {
        manifest: PluginManifest,
        count: u32,
        fail: bool,
    }

    impl Plugin for Counter {
        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }

        fn contributions(&self) -> Contributions {
            Contributions::new()
                .command(CommandContribution::new("counter.increment", "Increment"))
                .command(CommandContribution::new("counter.open", "Open Counter"))
                .tool_window(ToolWindowContribution::new("counter", "Counter", "main"))
                .status_widget(StatusWidgetContribution::new("count"))
                .settings_page(
                    SettingsPage::new("counter", "Counter").field(SettingField::new(
                        "step",
                        "Step",
                        SettingKind::Number {
                            default: 1.0,
                            min: Some(1.0),
                            max: Some(5.0),
                        },
                    )),
                )
        }

        fn activate(&mut self, _host: &mut dyn PluginHost) -> Result<(), PluginError> {
            if self.fail {
                return Err(PluginError::Activation {
                    plugin: self.manifest.id.clone(),
                    message: "boom".into(),
                });
            }
            Ok(())
        }

        fn execute(&mut self, command: &str, _: Value, host: &mut dyn PluginHost) -> CommandResult {
            match command {
                "counter.increment" => {
                    let step = host.setting("step").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    self.count += step as u32;
                    host.show_status(&format!("count {}", self.count));
                    host.dispatch("toolwindow.hideAll", Value::Null);
                    Ok(json!(self.count))
                }
                "counter.open" => {
                    host.open_tool_window("counter");
                    host.open_editor_tab(EditorTabRequest::new("main", "Counter"));
                    host.open_editor_tab(EditorTabRequest::new("main", "Counter (again)"));
                    Ok(Value::Null)
                }
                other => Err(CommandError::Unknown(other.into())),
            }
        }

        fn status_widget(&self, _id: &str) -> Option<StatusWidgetState> {
            Some(StatusWidgetState {
                text: self.count.to_string(),
                tooltip: None,
            })
        }
    }

    fn factory(id: &'static str, fail: bool) -> archipel_ide_plugins::PluginFactory {
        Rc::new(move || {
            Box::new(Counter {
                manifest: PluginManifest::parse(&format!(
                    "id='{id}'\nname='Counter'\nversion='1'\napi_version=1\nkind='rust'\n[app]\nview='main'\n"
                ))
                .unwrap(),
                count: 0,
                fail,
            })
        })
    }

    fn setup(fail: bool) -> (IdeState, CommandRegistry<IdeState>, Vec<String>) {
        let mut state = IdeState::default();
        state.init_plugins(
            PluginStore::in_memory(),
            PluginSet {
                bundled: vec![factory("test.counter", fail)],
                ..PluginSet::default()
            },
        );
        let mut registry = built_in_commands();
        let mut registered = Vec::new();
        sync_plugin_commands(&mut registry, &mut registered, &mut state);
        (state, registry, registered)
    }

    #[test]
    fn contributed_commands_run_through_the_shared_registry() {
        let (mut state, registry, registered) = setup(false);
        assert_eq!(registered, ["counter.increment", "counter.open"]);
        assert!(
            registry
                .specs()
                .iter()
                .any(|spec| spec.id == "counter.increment")
        );
        assert_eq!(
            registry
                .execute("counter.increment", &mut state, Value::Null)
                .unwrap(),
            json!(1)
        );
        assert_eq!(state.status, "count 1");
        assert_eq!(
            state.plugin_host_service().read().queued_commands[0].0,
            "toolwindow.hideAll"
        );
        registry
            .execute(
                "plugin.setSetting",
                &mut state,
                json!({"plugin": "test.counter", "key": "step", "value": 99}),
            )
            .unwrap();
        // Clamped to the declared maximum.
        assert_eq!(
            state.plugin_setting("test.counter", "step"),
            Some(json!(5.0))
        );
        assert!(
            registry
                .execute(
                    "plugin.setSetting",
                    &mut state,
                    json!({"plugin": "test.counter", "key": "nope", "value": 1})
                )
                .is_err()
        );
        registry
            .execute("counter.increment", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.plugin_status_widgets()[0].state.text, "6");
    }

    #[test]
    fn files_open_in_the_plugin_editor_for_their_extension() {
        let (mut state, _, _) = setup(false);
        assert!(!state.open_in_plugin_editor(Path::new("a.http")));
        assert!(state.plugin_host_service().write().tabs.is_empty());
        state
            .plugin_host_service()
            .registry_mut()
            .set_contributions(
                "test.counter",
                Contributions::new().editor(
                    archipel_ide_plugins::EditorContribution::new("main", "Counter")
                        .file_extension("http"),
                ),
            );
        assert!(state.open_in_plugin_editor(Path::new("dir/a.HTTP")));
        let tab = state.active_plugin_tab().unwrap();
        assert_eq!((tab.view.as_str(), tab.title.as_str()), ("main", "a.HTTP"));
        assert_eq!(tab.path.as_deref(), Some(Path::new("dir/a.HTTP")));
        assert!(!state.open_in_plugin_editor(Path::new("a.rs")));
    }

    #[test]
    fn views_open_as_tool_windows_and_deduplicated_tabs() {
        let (mut state, registry, _) = setup(false);
        registry
            .execute("counter.open", &mut state, Value::Null)
            .unwrap();
        assert_eq!(
            state.plugin_host_service().tool_window(),
            Some(("test.counter".into(), "counter".into()))
        );
        assert_eq!(state.plugin_host_service().tab_count(), 1);
        assert_eq!(state.active_plugin_tab().unwrap().title, "Counter (again)");
        registry
            .execute(
                "plugin.toolwindow.toggle",
                &mut state,
                json!({"plugin": "test.counter", "id": "counter"}),
            )
            .unwrap();
        assert_eq!(state.plugin_host_service().tool_window(), None);
        assert!(
            registry
                .execute(
                    "plugin.toolwindow.toggle",
                    &mut state,
                    json!({"plugin": "test.counter", "id": "x"})
                )
                .is_err()
        );
        registry
            .execute("plugin.tab.close", &mut state, Value::Null)
            .unwrap();
        assert!(state.plugin_host_service().read().tabs.is_empty());
        assert!(
            registry
                .execute("plugin.tab.activate", &mut state, json!({"index": 3}))
                .is_err()
        );
    }

    #[test]
    fn disabling_unregisters_commands_and_closes_views() {
        let (mut state, mut registry, mut registered) = setup(false);
        registry
            .execute("counter.open", &mut state, Value::Null)
            .unwrap();
        registry
            .execute("counter.increment", &mut state, Value::Null)
            .unwrap();
        registry
            .execute("plugin.disable", &mut state, json!({"id": "test.counter"}))
            .unwrap();
        assert!(state.plugin_host_service().read().tabs.is_empty());
        assert_eq!(state.plugin_host_service().tool_window(), None);
        sync_plugin_commands(&mut registry, &mut registered, &mut state);
        assert!(registered.is_empty());
        assert!(matches!(
            registry.execute("counter.increment", &mut state, Value::Null),
            Err(CommandError::Unknown(_))
        ));
        registry
            .execute("plugin.enable", &mut state, json!({"id": "test.counter"}))
            .unwrap();
        sync_plugin_commands(&mut registry, &mut registered, &mut state);
        // Re-enabling starts a fresh instance.
        assert_eq!(
            registry
                .execute("counter.increment", &mut state, Value::Null)
                .unwrap(),
            json!(1)
        );
        let list = registry
            .execute("plugin.list", &mut state, Value::Null)
            .unwrap();
        assert_eq!(list["plugins"][0]["status"], "Active");
        assert!(
            registry
                .execute("plugin.enable", &mut state, json!({"id": "no.such"}))
                .is_err()
        );
    }

    #[test]
    fn failed_activation_is_reported_and_contributes_nothing() {
        let (mut state, registry, registered) = setup(true);
        assert!(registered.is_empty());
        let list = registry
            .execute("plugin.list", &mut state, Value::Null)
            .unwrap();
        assert_eq!(list["plugins"][0]["status"], "Failed");
        assert!(
            list["plugins"][0]["reason"]
                .as_str()
                .unwrap()
                .contains("boom")
        );
        assert!(state.with_plugin("test.counter", |_, _| ()).is_none());
    }

    #[test]
    fn archives_install_activate_and_uninstall_without_restart() {
        let base =
            std::env::temp_dir().join(format!("archipel-ide-host-install-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let source = base.join("theme");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(
            source.join("archipel-plugin.toml"),
            "id='x.theme'\nname='Theme'\nversion='1.0'\napi_version=1\n",
        )
        .unwrap();
        let archive = base.join("theme.archipelplugin");
        archipel_ide_plugins::archive::pack(&source, &archive).unwrap();
        let (mut state, registry, _) = setup(false);
        state.plugin_host_service().write().install_dir = Some(base.join("plugins"));
        let result = registry
            .execute("plugin.install", &mut state, json!({"path": archive}))
            .unwrap();
        assert_eq!(result["status"], "Active");
        assert!(state.plugin_uninstallable("x.theme"));
        assert!(!state.plugin_uninstallable("test.counter"));
        registry
            .execute("plugin.disable", &mut state, json!({"id": "x.theme"}))
            .unwrap();
        registry
            .execute("plugin.uninstall", &mut state, json!({"id": "x.theme"}))
            .unwrap();
        assert!(
            state
                .plugin_host_service()
                .registry()
                .get("x.theme")
                .is_none()
        );
        assert!(
            !state
                .plugin_host_service()
                .registry()
                .store
                .is_disabled("x.theme")
        );
        let error = registry
            .execute(
                "plugin.uninstall",
                &mut state,
                json!({"id": "test.counter"}),
            )
            .unwrap_err();
        assert!(
            error.to_string().contains("cannot be uninstalled"),
            "{error}"
        );
        // A bundled id cannot be shadowed by an archive.
        std::fs::write(
            source.join("archipel-plugin.toml"),
            "id='test.counter'\nname='Fake'\nversion='9'\napi_version=1\n",
        )
        .unwrap();
        archipel_ide_plugins::archive::pack(&source, &archive).unwrap();
        let error = registry
            .execute("plugin.install", &mut state, json!({"path": archive}))
            .unwrap_err();
        assert!(
            error.to_string().contains("this Archipel IDE build"),
            "{error}"
        );
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn plugin_commands_never_replace_built_ins() {
        let mut state = IdeState::default();
        state.init_plugins(
            PluginStore::in_memory(),
            PluginSet {
                bundled: vec![factory("test.counter", false)],
                ..PluginSet::default()
            },
        );
        let mut registry = built_in_commands();
        registry
            .register(Command::new(
                spec("counter.increment", "Built-in", "Test", None, "{}"),
                |_| true,
                |_, _| Ok(json!("built-in")),
            ))
            .unwrap();
        let mut registered = Vec::new();
        sync_plugin_commands(&mut registry, &mut registered, &mut state);
        assert_eq!(registered, ["counter.open"]);
        assert!(state.status.contains("conflicts"));
        assert_eq!(
            registry
                .execute("counter.increment", &mut state, Value::Null)
                .unwrap(),
            json!("built-in")
        );
    }
}

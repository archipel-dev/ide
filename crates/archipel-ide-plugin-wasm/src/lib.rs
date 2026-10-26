//! WebAssembly component plugins (plugin API 1 for WASM).
//!
//! A plugin is a component targeting the `archipel:plugin/plugin` world in
//! `wit/plugin.wit` (for example a Rust crate built for `wasm32-wasip2` with
//! `wit-bindgen`). Components built for the world's name before the rename to Archipel IDE
//! (`jetthing:plugin@1.0.0`, `wit-legacy/plugin.wit`) are still accepted: the two worlds are
//! identical apart from the package name, and the loader picks the one the component
//! imports. It runs in wasmtime with:
//!
//! - no ambient authority: WASI gets no environment, arguments or stdin; the
//!   project is mounted at `/project` only with the `fs.project` permission and
//!   network access requires `network`;
//! - a 64 MiB memory cap and a fuel budget per call (a runaway call traps and
//!   the instance is recreated);
//! - host calls buffered during a guest call and applied to the IDE afterwards.
//!
//! See `docs/plugins/wasm.md` and `plugins/hello-wasm`.

use archipel_ide_plugin_api::{
    CommandError, CommandResult, Contributions, EditorTabRequest, Notification, NotificationLevel,
    Permission, Plugin, PluginError, PluginHost, PluginKind, PluginManifest, StatusWidgetState,
    Value as Json, ViewContext, ViewFrame, ViewNode, ViewPlacement,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{FsPerms, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "plugin",
    });
}

/// The same world under its name before the rename (compatibility with plugins built for it).
mod legacy_bindings {
    wasmtime::component::bindgen!({
        path: "wit-legacy",
        world: "plugin",
    });
}

/// Import namespace of the pre-rename world (see `legacy_bindings`).
const LEGACY_WORLD_PREFIX: &str = "jetthing:plugin/";

pub const MEMORY_LIMIT: usize = 64 * 1024 * 1024;
const MAX_COMPONENT_BYTES: u64 = 64 * 1024 * 1024;
/// Fuel for `activate`, commands and `poll` (roughly instructions).
const CALL_FUEL: u64 = 2_000_000_000;
/// Fuel for one view frame or status widget.
const VIEW_FUEL: u64 = 200_000_000;

/// Runtime for `kind = "wasm"` plugins. One engine is shared by all plugins.
#[derive(Clone)]
pub struct WasmLoader {
    engine: Engine,
}

impl WasmLoader {
    pub fn new() -> Result<Self, String> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.consume_fuel(true);
        Engine::new(&config)
            .map(|engine| Self { engine })
            .map_err(|error| error.to_string())
    }
}

impl archipel_ide_plugins::PluginLoader for WasmLoader {
    fn kind(&self) -> PluginKind {
        PluginKind::Wasm
    }

    fn load(&self, manifest: &PluginManifest, dir: &Path) -> Result<Box<dyn Plugin>, PluginError> {
        Ok(Box::new(WasmPlugin::load(
            &self.engine,
            manifest.clone(),
            dir,
        )?))
    }
}

/// Something the guest asked for during a call.
#[derive(Debug, Clone, PartialEq)]
enum Action {
    Status(String),
    Notify(Notification),
    Dispatch(String, Json),
    OpenToolWindow(String),
    OpenTab(EditorTabRequest),
    SetSetting(String, Json),
    Repaint,
}

struct State {
    wasi: WasiCtx,
    table: ResourceTable,
    limits: StoreLimits,
    permissions: Vec<Permission>,
    project_root: Option<String>,
    /// Settings snapshot (declared keys), updated by `set-setting`.
    settings: BTreeMap<String, Json>,
    actions: Vec<Action>,
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

/// Implements the host interface of either world for [`State`]: both have the same functions.
macro_rules! impl_host {
    ($host:path) => {
        impl $host for State {
            fn status(&mut self, message: String) {
                self.actions.push(Action::Status(message));
            }

            fn notify(&mut self, level: String, title: String, body: String) {
                let level = match level.as_str() {
                    "error" => NotificationLevel::Error,
                    "warning" => NotificationLevel::Warning,
                    _ => NotificationLevel::Info,
                };
                self.actions
                    .push(Action::Notify(Notification { level, title, body }));
            }

            fn dispatch(&mut self, command: String, params_json: String) {
                let params = serde_json::from_str(&params_json).unwrap_or(Json::Null);
                self.actions.push(Action::Dispatch(command, params));
            }

            fn open_tool_window(&mut self, id: String) {
                self.actions.push(Action::OpenToolWindow(id));
            }

            fn open_tab(&mut self, view: String, title: String, key: String) {
                let mut request = EditorTabRequest::new(view, title);
                if !key.is_empty() {
                    request = request.key(key);
                }
                self.actions.push(Action::OpenTab(request));
            }

            fn setting(&mut self, key: String) -> Option<String> {
                self.settings.get(&key).map(Json::to_string)
            }

            fn set_setting(&mut self, key: String, value_json: String) -> Result<(), String> {
                let value: Json =
                    serde_json::from_str(&value_json).map_err(|e| format!("invalid JSON: {e}"))?;
                self.settings.insert(key.clone(), value.clone());
                self.actions.push(Action::SetSetting(key, value));
                Ok(())
            }

            fn project_root(&mut self) -> Option<String> {
                self.project_root.clone()
            }

            fn has_permission(&mut self, name: String) -> bool {
                self.permissions.iter().any(|p| p.id() == name)
            }

            fn request_repaint(&mut self) {
                self.actions.push(Action::Repaint);
            }
        }
    };
}

impl_host!(bindings::archipel::plugin::host::Host);
impl_host!(legacy_bindings::jetthing::plugin::host::Host);

struct Instance {
    store: Store<State>,
    plugin: Guest,
}

/// The instantiated guest, bound to the current world or to the pre-rename one.
enum Guest {
    Current(bindings::Plugin),
    Legacy(legacy_bindings::Plugin),
}

macro_rules! guest_call {
    ($self:ident, $method:ident($($arg:expr),*)) => {
        match $self {
            Guest::Current(plugin) => plugin.$method($($arg),*),
            Guest::Legacy(plugin) => plugin.$method($($arg),*),
        }
    };
}

impl Guest {
    fn call_contributions(&self, store: &mut Store<State>) -> wasmtime::Result<String> {
        guest_call!(self, call_contributions(store))
    }

    fn call_activate(&self, store: &mut Store<State>) -> wasmtime::Result<Result<(), String>> {
        guest_call!(self, call_activate(store))
    }

    fn call_deactivate(&self, store: &mut Store<State>) -> wasmtime::Result<()> {
        guest_call!(self, call_deactivate(store))
    }

    fn call_execute(
        &self,
        store: &mut Store<State>,
        command: &str,
        params_json: &str,
    ) -> wasmtime::Result<Result<String, String>> {
        guest_call!(self, call_execute(store, command, params_json))
    }

    fn call_render_view(
        &self,
        store: &mut Store<State>,
        view: &str,
        placement_json: &str,
    ) -> wasmtime::Result<String> {
        guest_call!(self, call_render_view(store, view, placement_json))
    }

    fn call_on_view_event(
        &self,
        store: &mut Store<State>,
        view: &str,
        event_json: &str,
    ) -> wasmtime::Result<()> {
        guest_call!(self, call_on_view_event(store, view, event_json))
    }

    fn call_status_widget(
        &self,
        store: &mut Store<State>,
        id: &str,
    ) -> wasmtime::Result<Option<String>> {
        guest_call!(self, call_status_widget(store, id))
    }

    fn call_poll(&self, store: &mut Store<State>) -> wasmtime::Result<()> {
        guest_call!(self, call_poll(store))
    }
}

/// Whether `component` was built for the pre-rename world (it imports its host interface).
fn targets_legacy_world(engine: &Engine, component: &Component) -> bool {
    component
        .component_type()
        .imports(engine)
        .any(|(name, _)| name.starts_with(LEGACY_WORLD_PREFIX))
}

pub struct WasmPlugin {
    manifest: PluginManifest,
    engine: Engine,
    component: Component,
    linker: Linker<State>,
    instance: RefCell<Instance>,
    contributions: Contributions,
    /// Project mounted at `/project` when the instance was created.
    mounted_project: Option<PathBuf>,
    view_error: Option<String>,
}

fn load_error(manifest: &PluginManifest, error: impl std::fmt::Display) -> PluginError {
    PluginError::Activation {
        plugin: manifest.id.clone(),
        message: error.to_string(),
    }
}

/// Human-readable error, naming fuel exhaustion explicitly.
fn describe(error: &wasmtime::Error) -> String {
    if let Some(trap) = error.downcast_ref::<wasmtime::Trap>()
        && *trap == wasmtime::Trap::OutOfFuel
    {
        return "plugin exceeded its time budget (out of fuel)".into();
    }
    format!("{error:#}")
}

impl WasmPlugin {
    pub fn load(
        engine: &Engine,
        manifest: PluginManifest,
        dir: &Path,
    ) -> Result<Self, PluginError> {
        let entry = manifest.entry.clone().ok_or_else(|| {
            PluginError::Manifest(format!("{}: WASM plugins need an `entry`", manifest.id))
        })?;
        let path = dir.join(&entry);
        let size = std::fs::metadata(&path)
            .map_err(|e| load_error(&manifest, format!("{}: {e}", path.display())))?
            .len();
        if size > MAX_COMPONENT_BYTES {
            return Err(load_error(
                &manifest,
                format!("{entry} is larger than 64 MiB"),
            ));
        }
        let component = Component::from_file(engine, &path).map_err(|e| {
            load_error(
                &manifest,
                format!("{entry} is not a valid component: {e:#}"),
            )
        })?;
        let mut linker = Linker::new(engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker).map_err(|e| load_error(&manifest, e))?;
        bindings::Plugin::add_to_linker::<State, wasmtime::component::HasSelf<State>>(
            &mut linker,
            |state| state,
        )
        .map_err(|e| load_error(&manifest, e))?;
        legacy_bindings::Plugin::add_to_linker::<State, wasmtime::component::HasSelf<State>>(
            &mut linker,
            |state| state,
        )
        .map_err(|e| load_error(&manifest, e))?;
        let instance = instantiate(engine, &component, &linker, &manifest, None)
            .map_err(|e| load_error(&manifest, describe(&e)))?;
        let mut plugin = Self {
            manifest,
            engine: engine.clone(),
            component,
            linker,
            instance: RefCell::new(instance),
            contributions: Contributions::default(),
            mounted_project: None,
            view_error: None,
        };
        let json = plugin
            .call(VIEW_FUEL, None, |p, store| p.call_contributions(store))
            .map_err(|e| load_error(&plugin.manifest, e))?;
        plugin.contributions = serde_json::from_str(&json).map_err(|e| {
            load_error(
                &plugin.manifest,
                format!("contributions() returned invalid JSON: {e}"),
            )
        })?;
        Ok(plugin)
    }

    /// Runs `f` with fuel, refreshing the host snapshot before and applying
    /// buffered actions after. A trap recreates the instance.
    fn call<R>(
        &self,
        fuel: u64,
        host: Option<&mut dyn PluginHost>,
        f: impl FnOnce(&Guest, &mut Store<State>) -> wasmtime::Result<R>,
    ) -> Result<R, String> {
        let mut instance = self.instance.borrow_mut();
        let Instance { store, plugin } = &mut *instance;
        if let Some(host) = &host {
            let state = store.data_mut();
            state.project_root = host.project_root().map(|root| root.display().to_string());
            for page in &self.contributions.settings_pages {
                for field in &page.fields {
                    match host.setting(&field.key) {
                        Some(value) => state.settings.insert(field.key.clone(), value),
                        None => state.settings.remove(&field.key),
                    };
                }
            }
        }
        store.set_fuel(fuel).map_err(|e| e.to_string())?;
        let result = f(plugin, store);
        let actions = std::mem::take(&mut store.data_mut().actions);
        if let Some(host) = host {
            apply(host, actions);
        }
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                let message = describe(&error);
                // The instance may be poisoned after a trap; start a fresh one.
                match instantiate(
                    &self.engine,
                    &self.component,
                    &self.linker,
                    &self.manifest,
                    self.mounted_project.as_deref(),
                ) {
                    Ok(fresh) => *instance = fresh,
                    Err(error) => {
                        return Err(format!("{message}; restart failed: {}", describe(&error)));
                    }
                }
                Err(message)
            }
        }
    }
}

fn instantiate(
    engine: &Engine,
    component: &Component,
    linker: &Linker<State>,
    manifest: &PluginManifest,
    project: Option<&Path>,
) -> wasmtime::Result<Instance> {
    let mut wasi = WasiCtxBuilder::new();
    wasi.inherit_stderr();
    if manifest.has_permission(Permission::Network) {
        wasi.inherit_network().allow_ip_name_lookup(true);
    }
    if manifest.has_permission(Permission::ProjectFiles)
        && let Some(project) = project
    {
        wasi.preopened_dir(project, "/project", FsPerms::ReadWrite)?;
    }
    let state = State {
        wasi: wasi.build(),
        table: ResourceTable::new(),
        limits: StoreLimitsBuilder::new()
            .memory_size(MEMORY_LIMIT)
            .instances(64)
            .build(),
        permissions: manifest.permissions.clone(),
        project_root: project.map(|p| p.display().to_string()),
        settings: BTreeMap::new(),
        actions: Vec::new(),
    };
    let mut store = Store::new(engine, state);
    store.limiter(|state| &mut state.limits);
    store.set_fuel(CALL_FUEL)?;
    let plugin = if targets_legacy_world(engine, component) {
        Guest::Legacy(legacy_bindings::Plugin::instantiate(
            &mut store, component, linker,
        )?)
    } else {
        Guest::Current(bindings::Plugin::instantiate(
            &mut store, component, linker,
        )?)
    };
    Ok(Instance { store, plugin })
}

fn apply(host: &mut dyn PluginHost, actions: Vec<Action>) {
    for action in actions {
        match action {
            Action::Status(message) => host.show_status(&message),
            Action::Notify(notification) => host.notify(notification),
            Action::Dispatch(command, params) => host.dispatch(&command, params),
            Action::OpenToolWindow(id) => host.open_tool_window(&id),
            Action::OpenTab(request) => host.open_editor_tab(request),
            Action::SetSetting(key, value) => {
                if let Err(error) = host.set_setting(&key, value) {
                    host.show_status(&error.to_string());
                }
            }
            Action::Repaint => host.request_repaint(),
        }
    }
}

fn placement_json(view: &ViewContext) -> String {
    let placement = match &view.placement {
        ViewPlacement::ToolWindow { id } => serde_json::json!({"kind": "tool_window", "id": id}),
        ViewPlacement::EditorTab { key, path } => {
            serde_json::json!({"kind": "editor_tab", "key": key, "path": path})
        }
        ViewPlacement::App => serde_json::json!({"kind": "app"}),
        ViewPlacement::SettingsPage { id } => {
            serde_json::json!({"kind": "settings_page", "id": id})
        }
    };
    placement.to_string()
}

impl Plugin for WasmPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn contributions(&self) -> Contributions {
        self.contributions.clone()
    }

    fn activate(&mut self, host: &mut dyn PluginHost) -> Result<(), PluginError> {
        // Mount the project now that the host knows it.
        let project = host.project_root().map(Path::to_path_buf);
        if project != self.mounted_project && self.manifest.has_permission(Permission::ProjectFiles)
        {
            let fresh = instantiate(
                &self.engine,
                &self.component,
                &self.linker,
                &self.manifest,
                project.as_deref(),
            )
            .map_err(|e| load_error(&self.manifest, describe(&e)))?;
            *self.instance.borrow_mut() = fresh;
            self.mounted_project = project;
        }
        self.call(CALL_FUEL, Some(host), |p, store| p.call_activate(store))
            .and_then(|result| result)
            .map_err(|e| load_error(&self.manifest, e))
    }

    fn deactivate(&mut self, host: &mut dyn PluginHost) {
        if let Err(error) = self.call(CALL_FUEL, Some(host), |p, store| p.call_deactivate(store)) {
            eprintln!("[{}] deactivate failed: {error}", self.manifest.id);
        }
    }

    fn execute(&mut self, command: &str, params: Json, host: &mut dyn PluginHost) -> CommandResult {
        if !self.contributions.commands.iter().any(|c| c.id == command) {
            return Err(CommandError::Unknown(command.into()));
        }
        let output = self
            .call(CALL_FUEL, Some(host), |p, store| {
                p.call_execute(store, command, &params.to_string())
            })
            .and_then(|result| result)
            .map_err(|e| CommandError::Failed(format!("{command}: {e}")))?;
        if output.trim().is_empty() {
            return Ok(Json::Null);
        }
        serde_json::from_str(&output)
            .map_err(|e| CommandError::Failed(format!("{command}: result is not JSON: {e}")))
    }

    fn render_view(
        &mut self,
        view: &ViewContext,
        frame: &mut ViewFrame<'_>,
        host: &mut dyn PluginHost,
    ) {
        let Some(ui) = archipel_ide_plugin_ui::ui(frame) else {
            return;
        };
        let rendered = self
            .call(VIEW_FUEL, Some(&mut *host), |p, store| {
                p.call_render_view(store, &view.view, &placement_json(view))
            })
            .and_then(|json| {
                serde_json::from_str::<ViewNode>(&json)
                    .map_err(|e| format!("view `{}` returned invalid JSON: {e}", view.view))
            });
        let node = match rendered {
            Ok(node) => node,
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, format!("WASM error: {error}"));
                self.view_error = Some(error);
                return;
            }
        };
        self.view_error = None;
        let scope = format!("{}:{}", self.manifest.id, view.view);
        let events = archipel_ide_plugin_ui::render_node(ui, &scope, &node);
        for event in events {
            let event = serde_json::to_string(&event).unwrap_or_default();
            if let Err(error) = self.call(CALL_FUEL, Some(&mut *host), |p, store| {
                p.call_on_view_event(store, &view.view, &event)
            }) {
                host.show_status(&format!("{}: {error}", self.manifest.id));
            }
        }
    }

    fn status_widget(&self, id: &str) -> Option<StatusWidgetState> {
        let text = self
            .call(VIEW_FUEL, None, |p, store| p.call_status_widget(store, id))
            .ok()
            .flatten()?;
        Some(StatusWidgetState {
            text,
            tooltip: None,
        })
    }

    fn poll(&mut self, host: &mut dyn PluginHost) {
        if let Err(error) = self.call(CALL_FUEL, Some(host), |p, store| p.call_poll(store)) {
            eprintln!("[{}] poll failed: {error}", self.manifest.id);
        }
    }
}

#[cfg(test)]
mod tests;

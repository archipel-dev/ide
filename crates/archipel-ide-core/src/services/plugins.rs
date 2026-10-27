//! The plugin registry and the views plugins contribute, as an application service (IntelliJ:
//! `PluginManager` and the plugin descriptors).
//!
//! [`PluginHostService`] owns what the Plugins page and the window show: the registry
//! (manifests, status, contributions, the user's settings and enable choices), the plugin tool
//! window on the right anchor, the plugin tabs in the editor area, the commands plugins queued
//! and the app mode of a standalone plugin window. It is an application service: plugins are
//! per user, not per project.
//!
//! The running plugin code (a Lua state, a WASM store) is not `Send`, so the instances stay in
//! the `PluginRuntime` that `IdeState` owns on the UI thread. A plugin call takes its instance
//! out of the runtime and runs with a host that talks to this service in short, separate
//! locks; nothing here is held across plugin code.
//!
//! What happened is announced on [`PLUGIN_TOPIC`]: a plugin changed status (activated, failed,
//! disabled) and a plugin tool window was shown or hidden. The UI reads through narrow
//! accessors that return copies, plus [`PluginHostService::registry`], a read guard of the
//! registry that must not be held while a command runs.

use super::Cell;
use crate::plugin_host::{PluginHostState, PluginTab};
use archipel_ide_platform::{MessageBus, Topic};
use archipel_ide_plugins::{PluginRegistry, PluginStatus};
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLockReadGuard, RwLockWriteGuard,
};
use serde_json::Value;
use std::path::PathBuf;

/// Reports changes of plugins (IntelliJ: `DynamicPluginListener`).
pub trait PluginListener: Send + Sync {
    /// The status of plugin `id` changed.
    fn plugin_status_changed(&self, _id: &str, _status: &PluginStatus) {}
    /// The plugin tool window on the right anchor changed: `Some((plugin, window id))` when one
    /// is showing.
    fn plugin_tool_window_changed(&self, _window: Option<&(String, String)>) {}
}

/// On the application's message bus.
pub const PLUGIN_TOPIC: Topic<dyn PluginListener> = Topic::new("PluginListener");

/// The application service: `state.plugin_host_service()`.
pub struct PluginHostService {
    state: Cell<PluginHostState>,
    bus: MessageBus,
}

impl PluginHostService {
    pub fn new(bus: MessageBus) -> Self {
        Self {
            state: Cell::new(PluginHostState::default(), "the plugin host"),
            bus,
        }
    }

    pub(crate) fn read(&self) -> RwLockReadGuard<'_, PluginHostState> {
        self.state.read()
    }

    pub(crate) fn write(&self) -> RwLockWriteGuard<'_, PluginHostState> {
        self.state.write()
    }

    /// The registry: entries, problems and the user's choices. Do not hold the guard while a
    /// command runs: commands write the registry.
    pub fn registry(&self) -> MappedRwLockReadGuard<'_, PluginRegistry> {
        self.state.read_map(|state| &state.registry)
    }

    pub(crate) fn registry_mut(&self) -> MappedRwLockWriteGuard<'_, PluginRegistry> {
        self.state.write_map(|state| &mut state.registry)
    }

    /// Moves plugin `id` to `status` and announces it (no event when it did not change).
    pub(crate) fn set_status(&self, id: &str, status: PluginStatus) {
        {
            let mut registry = self.registry_mut();
            if registry.get(id).is_none_or(|entry| entry.status == status) {
                return;
            }
            registry.set_status(id, status.clone());
        }
        self.bus
            .publish(PLUGIN_TOPIC, |l| l.plugin_status_changed(id, &status));
    }

    // Queries (copies).

    /// Increments whenever entries, statuses or contributions change.
    pub fn generation(&self) -> u64 {
        self.read().registry.generation()
    }

    /// A copy of the entry of plugin `id`.
    pub fn entry(&self, id: &str) -> Option<archipel_ide_plugins::PluginEntry> {
        self.read().registry.get(id).cloned()
    }

    /// The installed version of plugin `id`.
    pub fn version(&self, id: &str) -> Option<String> {
        self.read()
            .registry
            .get(id)
            .map(|entry| entry.manifest.version.clone())
    }

    pub fn is_active(&self, id: &str) -> bool {
        self.read().registry.is_active(id)
    }

    /// The visible plugin tool window `(plugin, tool window id)` on the right anchor.
    pub fn tool_window(&self) -> Option<(String, String)> {
        self.read().tool_window.clone()
    }

    pub fn set_tool_window(&self, window: Option<(String, String)>) {
        {
            let mut state = self.write();
            if state.tool_window == window {
                return;
            }
            state.tool_window = window.clone();
        }
        self.bus.publish(PLUGIN_TOPIC, |l| {
            l.plugin_tool_window_changed(window.as_ref())
        });
    }

    pub fn tab_count(&self) -> usize {
        self.read().tabs.len()
    }

    pub fn tab_title(&self, index: usize) -> Option<String> {
        self.read().tabs.get(index).map(|tab| tab.title.clone())
    }

    pub fn tabs(&self) -> Vec<PluginTab> {
        self.read().tabs.clone()
    }

    /// Active plugin tab; while set it is shown instead of the active document.
    pub fn active_tab(&self) -> Option<usize> {
        self.read().active_tab
    }

    pub fn set_active_tab(&self, tab: Option<usize>) {
        self.write().active_tab = tab;
    }

    /// The plugin shown alone in standalone app mode (`--app <id>`).
    pub fn app_mode(&self) -> Option<String> {
        self.read().app_mode.clone()
    }

    pub fn set_app_mode(&self, plugin: Option<String>) {
        self.write().app_mode = plugin;
    }

    /// Takes the commands plugins queued; the UI runs them.
    pub fn take_queued_commands(&self) -> Vec<(String, Value)> {
        std::mem::take(&mut self.write().queued_commands)
    }

    pub(crate) fn queue_command(&self, command: &str, params: Value) {
        self.write().queued_commands.push((command.into(), params));
    }

    /// Per-user directory that `.archipelplugin` archives install into.
    pub fn install_dir(&self) -> Option<PathBuf> {
        self.read().install_dir.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_platform::{Application, ComponentManager, ServiceDescriptor};
    use archipel_ide_plugins::{Plugin, PluginManifest, PluginRuntime};
    use parking_lot::Mutex;
    use std::rc::Rc;
    use std::sync::Arc;

    #[derive(Default)]
    struct Recorder(Mutex<Vec<String>>);

    impl PluginListener for Recorder {
        fn plugin_status_changed(&self, id: &str, status: &PluginStatus) {
            self.0.lock().push(format!("{id} {}", status.label()));
        }
        fn plugin_tool_window_changed(&self, window: Option<&(String, String)>) {
            self.0.lock().push(format!("window {window:?}"));
        }
    }

    struct Dummy(PluginManifest);

    impl Plugin for Dummy {
        fn manifest(&self) -> &PluginManifest {
            &self.0
        }
    }

    fn setup() -> (Application, Arc<PluginHostService>, Arc<Recorder>) {
        let application = Application::new();
        application
            .register_service(ServiceDescriptor::new(|app: &Application| {
                PluginHostService::new(app.message_bus().clone())
            }))
            .unwrap();
        let recorder = Arc::new(Recorder::default());
        application
            .message_bus()
            .subscribe(application.disposable(), PLUGIN_TOPIC, recorder.clone())
            .unwrap();
        let service = application.service::<PluginHostService>();
        (application, service, recorder)
    }

    fn register(service: &PluginHostService, id: &'static str) -> PluginRuntime {
        let mut runtime = PluginRuntime::default();
        let manifest = PluginManifest::parse(&format!(
            "id='{id}'\nname='N'\nversion='1'\napi_version=1\nkind='rust'\n"
        ))
        .unwrap();
        service
            .registry_mut()
            .add_bundled(
                &mut runtime,
                Rc::new(move || Box::new(Dummy(manifest.clone()))),
            )
            .unwrap();
        runtime
    }

    #[test]
    fn status_changes_are_announced_once() {
        let (_application, service, recorder) = setup();
        let _runtime = register(&service, "a.app");
        service.set_status("a.app", PluginStatus::Active);
        service.set_status("a.app", PluginStatus::Active);
        service.set_status("a.app", PluginStatus::Failed("boom".into()));
        service.set_status("a.missing", PluginStatus::Active);
        assert_eq!(*recorder.0.lock(), ["a.app Active", "a.app Failed"]);
        assert!(!service.is_active("a.app"));
    }

    #[test]
    fn tool_window_and_tabs_are_copies_and_announced() {
        let (_application, service, recorder) = setup();
        service.set_tool_window(Some(("a.app".into(), "panel".into())));
        service.set_tool_window(Some(("a.app".into(), "panel".into())));
        service.set_tool_window(None);
        assert_eq!(recorder.0.lock().len(), 2);
        service.write().tabs.push(PluginTab {
            plugin: "a.app".into(),
            view: "v".into(),
            title: "Title".into(),
            key: "k".into(),
            path: None,
        });
        service.set_active_tab(Some(0));
        assert_eq!(service.tab_title(0).as_deref(), Some("Title"));
        assert_eq!(service.tab_count(), 1);
        assert_eq!(service.active_tab(), Some(0));
    }

    #[test]
    fn a_listener_may_read_the_registry_while_it_is_told() {
        struct Reader(Arc<PluginHostService>, Mutex<Option<bool>>);
        impl PluginListener for Reader {
            fn plugin_status_changed(&self, id: &str, _: &PluginStatus) {
                *self.1.lock() = Some(self.0.registry().get(id).is_some());
            }
        }
        let (application, service, _) = setup();
        let reader = Arc::new(Reader(service.clone(), Mutex::new(None)));
        application
            .message_bus()
            .subscribe(application.disposable(), PLUGIN_TOPIC, reader.clone())
            .unwrap();
        let _runtime = register(&service, "a.app");
        service.set_status("a.app", PluginStatus::Active);
        assert_eq!(*reader.1.lock(), Some(true));
    }

    #[test]
    fn queued_commands_are_taken_once() {
        let (_application, service, _) = setup();
        service.queue_command("a.run", serde_json::json!({"x": 1}));
        assert_eq!(service.take_queued_commands().len(), 1);
        assert!(service.take_queued_commands().is_empty());
    }
}

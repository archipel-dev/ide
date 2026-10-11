//! Plugin registry: discovery, enable/disable state, settings persistence and
//! the loaded plugin instances. Activation needs a [`PluginHost`], which the
//! application core provides.

pub mod archive;
pub mod catalog;
mod store;

pub use jetthing_plugin_api::*;
pub use store::{PluginStore, user_config_dir};

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Creates a bundled (compiled-in) plugin instance.
pub type PluginFactory = Rc<dyn Fn() -> Box<dyn Plugin>>;

/// A runtime that instantiates directory plugins of one [`PluginKind`]
/// (Lua scripts, WASM components).
pub trait PluginLoader {
    fn kind(&self) -> PluginKind;
    /// Loads the plugin whose manifest is in `dir`.
    fn load(&self, manifest: &PluginManifest, dir: &Path) -> Result<Box<dyn Plugin>, PluginError>;
}

/// Plugins compiled into the build plus the runtimes for script plugins.
#[derive(Default, Clone)]
pub struct PluginSet {
    pub bundled: Vec<PluginFactory>,
    pub loaders: Vec<Rc<dyn PluginLoader>>,
}

/// How an entry's code is instantiated.
#[derive(Clone)]
enum Code {
    Factory(PluginFactory),
    Loader(Rc<dyn PluginLoader>, PathBuf),
}

impl Code {
    fn instantiate(&self, manifest: &PluginManifest) -> Result<Box<dyn Plugin>, PluginError> {
        match self {
            Code::Factory(factory) => Ok(factory()),
            Code::Loader(loader, dir) => loader.load(manifest, dir),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginSource {
    /// Compiled into this Jetthing build.
    Bundled,
    /// Loaded from a plugin directory (per-user or project `plugins/`).
    Directory(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginStatus {
    /// Enabled; code has not been activated yet.
    Enabled,
    Active,
    Disabled,
    /// Activation failed; the message is shown on the Plugins page.
    Failed(String),
    /// The plugin cannot run in this build (e.g. runtime not available).
    Unsupported(String),
}

impl PluginStatus {
    pub fn label(&self) -> &str {
        match self {
            PluginStatus::Enabled => "Enabled",
            PluginStatus::Active => "Active",
            PluginStatus::Disabled => "Disabled",
            PluginStatus::Failed(_) => "Failed",
            PluginStatus::Unsupported(_) => "Unsupported",
        }
    }
}

pub struct PluginEntry {
    pub manifest: PluginManifest,
    pub source: PluginSource,
    pub status: PluginStatus,
    pub contributions: Contributions,
    instance: Option<Box<dyn Plugin>>,
    code: Option<Code>,
}

impl PluginEntry {
    pub fn id(&self) -> &str {
        &self.manifest.id
    }

    pub fn is_active(&self) -> bool {
        self.status == PluginStatus::Active
    }

    /// Whether the user can toggle this plugin on the Plugins page.
    pub fn can_toggle(&self) -> bool {
        !matches!(self.status, PluginStatus::Unsupported(_))
    }
}

#[derive(Default)]
pub struct PluginRegistry {
    entries: Vec<PluginEntry>,
    /// Manifests that failed to load, with the path that was read.
    pub problems: Vec<(PathBuf, PluginError)>,
    pub store: PluginStore,
    loaders: Vec<Rc<dyn PluginLoader>>,
    generation: u64,
}

impl PluginRegistry {
    pub fn new(store: PluginStore) -> Self {
        Self {
            store,
            ..Self::default()
        }
    }

    /// Increments whenever entries, statuses or contributions change.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    fn touch(&mut self) {
        self.generation += 1;
    }

    pub fn entries(&self) -> &[PluginEntry] {
        &self.entries
    }

    pub fn get(&self, id: &str) -> Option<&PluginEntry> {
        self.entries.iter().find(|entry| entry.id() == id)
    }

    fn get_mut(&mut self, id: &str) -> Option<&mut PluginEntry> {
        self.entries.iter_mut().find(|entry| entry.id() == id)
    }

    pub fn is_active(&self, id: &str) -> bool {
        self.get(id).is_some_and(PluginEntry::is_active)
    }

    /// Registers a compiled-in plugin.
    pub fn add_bundled(&mut self, factory: PluginFactory) -> Result<(), PluginError> {
        let instance = factory();
        let manifest = instance.manifest().clone();
        manifest.validate()?;
        self.insert(PluginEntry {
            status: self.initial_status(&manifest.id),
            manifest,
            source: PluginSource::Bundled,
            contributions: Contributions::default(),
            instance: Some(instance),
            code: Some(Code::Factory(factory)),
        })
    }

    /// Registers the runtime for script plugins of `loader.kind()`.
    pub fn add_loader(&mut self, loader: Rc<dyn PluginLoader>) {
        self.loaders
            .retain(|existing| existing.kind() != loader.kind());
        self.loaders.push(loader);
    }

    /// Forgets an entry (its instance is dropped without `deactivate`; the
    /// host deactivates first).
    pub fn remove(&mut self, id: &str) {
        self.entries.retain(|entry| entry.id() != id);
        self.touch();
    }

    fn initial_status(&self, id: &str) -> PluginStatus {
        if self.store.is_disabled(id) {
            PluginStatus::Disabled
        } else {
            PluginStatus::Enabled
        }
    }

    fn insert(&mut self, entry: PluginEntry) -> Result<(), PluginError> {
        if self.get(entry.id()).is_some() {
            return Err(PluginError::Manifest(format!(
                "duplicate plugin id {}",
                entry.id()
            )));
        }
        self.entries.push(entry);
        self.touch();
        Ok(())
    }

    /// Loads every `<dir>/<plugin>/archipel-plugin.toml` (or
    /// `jetthing-plugin.toml`, or legacy `plugin.toml`) below `root`. Entries previously discovered from `root`
    /// are replaced. Problems are collected in [`Self::problems`].
    pub fn discover(&mut self, root: &Path) {
        // Entries still present on disk are kept as they are, so active
        // plugins survive a rediscovery; vanished ones are dropped.
        self.entries.retain(|entry| match &entry.source {
            PluginSource::Directory(dir) if dir.parent() == Some(root) => {
                MANIFEST_FILES.iter().any(|name| dir.join(name).is_file())
            }
            _ => true,
        });
        self.problems.retain(|(path, _)| !path.starts_with(root));
        let Ok(dirs) = fs::read_dir(root) else {
            self.touch();
            return;
        };
        let mut dirs: Vec<PathBuf> = dirs
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        dirs.sort();
        for dir in dirs {
            let Some(path) = MANIFEST_FILES
                .iter()
                .map(|name| dir.join(name))
                .find(|path| path.is_file())
            else {
                continue;
            };
            let manifest = fs::read_to_string(&path)
                .map_err(|error| PluginError::Io(path.display().to_string(), error.to_string()))
                .and_then(|text| PluginManifest::parse(&text));
            // The source tree of a compiled-in plugin (e.g. `plugins/hello` when
            // the Jetthing repository is open as a project) is not a second copy,
            // and an entry discovered earlier from this directory is kept.
            if let Ok(manifest) = &manifest
                && self.get(&manifest.id).is_some_and(|entry| {
                    entry.source == PluginSource::Bundled
                        || entry.source == PluginSource::Directory(dir.clone())
                })
            {
                continue;
            }
            let result = manifest.and_then(|manifest| {
                let loader = self
                    .loaders
                    .iter()
                    .find(|loader| loader.kind() == manifest.kind)
                    .cloned();
                let (status, code) = match (manifest.kind, loader) {
                    (PluginKind::Declarative, _) => (self.initial_status(&manifest.id), None),
                    (PluginKind::Rust, _) => (
                        PluginStatus::Unsupported(
                            "Rust plugins are compiled into Jetthing; third-party Rust code loads as a WASM component".into(),
                        ),
                        None,
                    ),
                    (_, Some(loader)) => (
                        self.initial_status(&manifest.id),
                        Some(Code::Loader(loader, dir.clone())),
                    ),
                    (kind, None) => (
                        PluginStatus::Unsupported(format!(
                            "the {} runtime is not available in this build",
                            kind.label()
                        )),
                        None,
                    ),
                };
                self.insert(PluginEntry {
                    manifest,
                    source: PluginSource::Directory(dir.clone()),
                    status,
                    contributions: Contributions::default(),
                    instance: None,
                    code,
                })
            });
            if let Err(error) = result {
                self.problems.push((path, error));
            }
        }
        self.touch();
    }

    /// Ids of enabled plugins whose code is waiting for activation, in
    /// dependency order (dependencies first).
    pub fn pending_activation(&self) -> Vec<String> {
        let mut order = Vec::new();
        let mut visiting = Vec::new();
        for entry in &self.entries {
            self.visit(entry.id(), &mut order, &mut visiting);
        }
        order
    }

    fn visit(&self, id: &str, order: &mut Vec<String>, visiting: &mut Vec<String>) {
        if order.iter().any(|done| done == id) || visiting.iter().any(|v| v == id) {
            return;
        }
        let Some(entry) = self.get(id) else { return };
        if entry.status != PluginStatus::Enabled {
            return;
        }
        visiting.push(id.to_owned());
        for dependency in &entry.manifest.dependencies {
            self.visit(dependency, order, visiting);
        }
        visiting.pop();
        order.push(id.to_owned());
    }

    /// Why `id` cannot activate now (missing or inactive dependency).
    pub fn dependency_problem(&self, id: &str) -> Option<String> {
        let entry = self.get(id)?;
        entry.manifest.dependencies.iter().find_map(|dependency| {
            match self.get(dependency).map(|d| &d.status) {
                None => Some(format!(
                    "requires plugin {dependency}, which is not installed"
                )),
                Some(PluginStatus::Active) => None,
                Some(_) => Some(format!("requires plugin {dependency}, which is not active")),
            }
        })
    }

    /// Takes the instance out so it can run with a host borrowing the rest
    /// of the application; return it with [`Self::restore_instance`].
    pub fn take_instance(&mut self, id: &str) -> Option<Box<dyn Plugin>> {
        self.get_mut(id)?.instance.take()
    }

    /// The loaded instance (absent while taken or for code-less plugins).
    pub fn instance(&self, id: &str) -> Option<&dyn Plugin> {
        self.get(id)?.instance.as_deref()
    }

    pub fn restore_instance(&mut self, id: &str, instance: Box<dyn Plugin>) {
        if let Some(entry) = self.get_mut(id) {
            entry.instance = Some(instance);
        }
    }

    /// Ensures an instance exists for a plugin with code (loading scripts).
    pub fn ensure_instance(&mut self, id: &str) -> Result<(), PluginError> {
        if let Some(entry) = self.get_mut(id)
            && entry.instance.is_none()
            && let Some(code) = entry.code.clone()
        {
            entry.instance = Some(code.instantiate(&entry.manifest)?);
        }
        Ok(())
    }

    pub fn has_code(&self, id: &str) -> bool {
        self.get(id).is_some_and(|entry| entry.code.is_some())
    }

    pub fn set_status(&mut self, id: &str, status: PluginStatus) {
        if let Some(entry) = self.get_mut(id) {
            entry.status = status;
            self.touch();
        }
    }

    pub fn set_contributions(&mut self, id: &str, contributions: Contributions) {
        if let Some(entry) = self.get_mut(id) {
            entry.contributions = contributions;
            self.touch();
        }
    }

    /// Persists the user's choice. Returns the entry's new status, or an error
    /// for unknown or unsupported plugins. The caller (de)activates code.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), PluginError> {
        let entry = self
            .get(id)
            .ok_or_else(|| PluginError::NotFound(id.into()))?;
        if let PluginStatus::Unsupported(reason) = &entry.status {
            return Err(PluginError::Activation {
                plugin: id.into(),
                message: reason.clone(),
            });
        }
        let current = entry.status.clone();
        self.store.set_disabled(id, !enabled)?;
        let status = match (enabled, current) {
            (true, PluginStatus::Active) => PluginStatus::Active,
            (true, _) => PluginStatus::Enabled,
            (false, _) => PluginStatus::Disabled,
        };
        self.set_status(id, status);
        Ok(())
    }

    /// Active plugins' commands as `(plugin id, spec)`.
    pub fn active_commands(&self) -> Vec<(String, jetthing_command::CommandSpec)> {
        self.entries
            .iter()
            .filter(|entry| entry.is_active())
            .flat_map(|entry| {
                entry
                    .contributions
                    .command_specs()
                    .into_iter()
                    .map(|spec| (entry.id().to_owned(), spec))
            })
            .collect()
    }

    /// The plugin that contributes `command`, if active.
    pub fn command_owner(&self, command: &str) -> Option<&str> {
        self.entries
            .iter()
            .filter(|entry| entry.is_active())
            .find(|entry| entry.contributions.commands.iter().any(|c| c.id == command))
            .map(PluginEntry::id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jetthing_plugin_api::testing::RecordingHost;

    struct Dummy(PluginManifest);

    impl Plugin for Dummy {
        fn manifest(&self) -> &PluginManifest {
            &self.0
        }
    }

    fn manifest(id: &str, deps: &[&str]) -> PluginManifest {
        let deps = deps
            .iter()
            .map(|d| format!("'{d}'"))
            .collect::<Vec<_>>()
            .join(",");
        PluginManifest::parse(&format!(
            "id='{id}'\nname='N'\nversion='1'\napi_version=1\nkind='rust'\ndependencies=[{deps}]\n"
        ))
        .unwrap()
    }

    fn bundled(registry: &mut PluginRegistry, id: &'static str, deps: &'static [&'static str]) {
        registry
            .add_bundled(Rc::new(move || Box::new(Dummy(manifest(id, deps)))))
            .unwrap();
    }

    #[test]
    fn activation_order_respects_dependencies_and_disabled_state() {
        let mut registry = PluginRegistry::default();
        bundled(&mut registry, "a.app", &["a.base"]);
        bundled(&mut registry, "a.base", &[]);
        bundled(&mut registry, "a.off", &[]);
        registry.set_enabled("a.off", false).unwrap();
        assert_eq!(registry.pending_activation(), ["a.base", "a.app"]);
        assert!(
            registry
                .dependency_problem("a.app")
                .unwrap()
                .contains("not active")
        );
        registry.set_status("a.base", PluginStatus::Active);
        assert_eq!(registry.dependency_problem("a.app"), None);
        assert!(
            registry
                .add_bundled(Rc::new(|| Box::new(Dummy(manifest("a.app", &[])))))
                .is_err()
        );
        let mut instance = registry.take_instance("a.app").unwrap();
        assert!(registry.take_instance("a.app").is_none());
        let mut host = RecordingHost::new("a.app");
        assert!(instance.activate(&mut host).is_ok());
        registry.restore_instance("a.app", instance);
        assert!(registry.take_instance("a.app").is_some());
    }

    #[test]
    fn discovers_archipel_and_jetthing_manifests() {
        let root = std::env::temp_dir().join(format!("jetthing-manifests-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (dir, file, id) in [
            ("new", "archipel-plugin.toml", "x.archipel"),
            ("old", "jetthing-plugin.toml", "x.jetthing"),
        ] {
            fs::create_dir_all(root.join(dir)).unwrap();
            fs::write(
                root.join(dir).join(file),
                format!("id='{id}'\nname='N'\nversion='1'\napi_version=1\n"),
            )
            .unwrap();
        }
        let mut registry = PluginRegistry::default();
        registry.discover(&root);
        assert!(registry.problems.is_empty(), "{:?}", registry.problems);
        assert!(registry.get("x.archipel").is_some());
        assert!(registry.get("x.jetthing").is_some());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn discovers_directory_plugins_and_reports_problems() {
        let root = std::env::temp_dir().join(format!("jetthing-plugins-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (dir, file, text) in [
            (
                "rust",
                LEGACY_MANIFEST_FILE,
                "id='dev.jetthing.rust'\nname='Rust'\nversion='0.1.0'\napi_version=1\n",
            ),
            (
                "lua",
                MANIFEST_FILE,
                "id='x.lua'\nname='L'\nversion='1'\napi_version=1\nkind='lua'\nentry='main.lua'\n",
            ),
            (
                "broken",
                MANIFEST_FILE,
                "id='nodot'\nname='B'\nversion='1'\napi_version=1\n",
            ),
        ] {
            fs::create_dir_all(root.join(dir)).unwrap();
            fs::write(root.join(dir).join(file), text).unwrap();
        }
        fs::create_dir_all(root.join("bundled")).unwrap();
        fs::write(
            root.join("bundled").join(MANIFEST_FILE),
            "id='a.app'\nname='Source'\nversion='1'\napi_version=1\nkind='rust'\n",
        )
        .unwrap();
        let mut registry = PluginRegistry::default();
        bundled(&mut registry, "a.app", &[]);
        registry.discover(&root);
        registry.discover(&root); // rediscovery replaces, does not duplicate
        assert_eq!(registry.entries().len(), 3);
        assert_eq!(registry.get("a.app").unwrap().source, PluginSource::Bundled);
        assert_eq!(
            registry.get("dev.jetthing.rust").unwrap().status,
            PluginStatus::Enabled
        );
        assert!(matches!(
            registry.get("x.lua").unwrap().status,
            PluginStatus::Unsupported(_)
        ));
        assert!(registry.set_enabled("x.lua", true).is_err());
        assert_eq!(registry.problems.len(), 1);
        assert!(
            registry.problems[0]
                .1
                .to_string()
                .contains("reverse-domain")
        );
        fs::remove_dir_all(&root).unwrap();
    }
}

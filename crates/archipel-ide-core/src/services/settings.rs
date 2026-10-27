//! Settings as an application service (IntelliJ: the application-level half of
//! `PropertiesComponent` and the `Configurable` registry): the registry of pages and
//! definitions, and the stored values with their application/project layering.

use archipel_ide_platform::ServiceState;
use archipel_ide_settings::{SettingValue, SettingsError, SettingsRegistry, SettingsStore};

/// The application service: `application.service::<SettingsService>()`.
pub type SettingsService = ServiceState<Settings>;

/// The settings registry (pages and definitions) and the stored values.
#[derive(Debug, Clone)]
pub struct Settings {
    pub registry: SettingsRegistry,
    pub store: SettingsStore,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            registry: SettingsRegistry::intellij(),
            store: SettingsStore::default(),
        }
    }
}

impl Settings {
    pub fn bool(&self, id: &str) -> Option<bool> {
        self.store.get_bool(&self.registry, id)
    }

    pub fn int(&self, id: &str) -> Option<i64> {
        self.store.get_int(&self.registry, id)
    }

    pub fn float(&self, id: &str) -> Option<f64> {
        self.store.get_float(&self.registry, id)
    }

    pub fn text(&self, id: &str) -> Option<String> {
        self.store.get_text(&self.registry, id)
    }

    /// The value of `id`, or its default.
    pub fn get(&self, id: &str) -> Result<SettingValue, SettingsError> {
        self.store.get(&self.registry, id)
    }

    /// The lines of a list setting (empty if `id` is not a list).
    pub fn list(&self, id: &str) -> Vec<String> {
        match self.get(id) {
            Ok(SettingValue::List(lines)) => lines,
            _ => Vec::new(),
        }
    }

    /// Validates and stores a value (in memory; `persist_settings` writes it).
    pub fn set(&mut self, id: &str, value: SettingValue) -> Result<(), SettingsError> {
        self.store.set(&self.registry, id, value)
    }
}

//! Settings as an application service (IntelliJ: the application-level half of
//! `PropertiesComponent` and the `Configurable` registry): the registry of pages and
//! definitions, and the stored values with their application/project layering.

use archipel_ide_platform::{MessageBus, ServiceState, Topic};
use archipel_ide_settings::{SettingValue, SettingsError, SettingsRegistry, SettingsStore};
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};

/// Reports that stored settings changed (IntelliJ: `SettingsChangeListener`,
/// `ApplicationActivationListener`-style option sync). Derived state subscribes instead of being
/// called by whoever changed a setting: the keymap, the Gradle options.
pub trait SettingsListener: Send + Sync {
    /// Settings were set, reset, applied or (re)loaded. `settings` is the new state; reading it
    /// is allowed, writing it is not (the publisher holds it).
    fn settings_changed(&self, _settings: &Settings) {}
}

/// On the application's message bus.
pub const SETTINGS_TOPIC: Topic<dyn SettingsListener> = Topic::new("SettingsListener");

/// The application service: `application.service::<SettingsService>()`. Whoever changes the
/// stored values calls [`SettingsService::notify_changed`] once the change is complete.
pub struct SettingsService {
    bus: MessageBus,
    state: ServiceState<Settings>,
}

impl SettingsService {
    pub fn new(bus: MessageBus) -> Self {
        Self {
            bus,
            state: ServiceState::new(Settings::default()),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<'_, Settings> {
        self.state.read()
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, Settings> {
        self.state.write()
    }

    pub fn with<R>(&self, f: impl FnOnce(&Settings) -> R) -> R {
        self.state.with(f)
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut Settings) -> R) -> R {
        self.state.with_mut(f)
    }

    /// Announces [`SETTINGS_TOPIC`]. Call without holding a write guard.
    pub fn notify_changed(&self) {
        let settings = self.state.read();
        self.bus
            .publish(SETTINGS_TOPIC, |l| l.settings_changed(&settings));
    }
}

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

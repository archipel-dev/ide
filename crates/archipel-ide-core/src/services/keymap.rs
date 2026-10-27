//! The active keymap as an application service (IntelliJ: `KeymapManager`).
//!
//! [`KeymapManager`] owns the effective keymap (the selected bundled keymap plus the user's
//! overrides) and keeps it in step with the settings: it subscribes to [`SETTINGS_TOPIC`] and
//! rebuilds itself whenever `keymap.active` or `keymap.overrides` may have changed, so nobody
//! calls it after changing a setting. Readers take a cheap [`Arc`] snapshot.

use super::settings::{SETTINGS_TOPIC, Settings, SettingsListener};
use archipel_ide_keymap::{Desktop, Keymap, KeymapSet, Overrides};
use archipel_ide_platform::{DisposableId, MessageBus};
use archipel_ide_settings::ids;
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The effective keymap: the selected bundled keymap plus the user's overrides for it.
#[derive(Debug, Clone)]
pub struct KeymapState {
    pub keymap: Keymap,
    /// Overrides of all keymaps, keyed by keymap id (persisted in `keymap.overrides`).
    pub overrides: BTreeMap<String, Overrides>,
    pub error: Option<String>,
}

impl Default for KeymapState {
    fn default() -> Self {
        let mut state = Self {
            keymap: Keymap::new("", BTreeMap::new()),
            overrides: BTreeMap::new(),
            error: None,
        };
        state.rebuild(Desktop::detect().default_keymap());
        state
    }
}

impl KeymapState {
    fn rebuild(&mut self, id: &str) {
        let overrides = self.overrides.get(id).cloned().unwrap_or_default();
        match KeymapSet::shared().and_then(|set| set.keymap(id, &overrides)) {
            Ok(keymap) => {
                self.keymap = keymap;
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    /// Parses `keymap<TAB>action<TAB>shortcut…` lines; malformed lines are skipped and
    /// reported.
    fn parse_overrides(lines: &[String]) -> (BTreeMap<String, Overrides>, Option<String>) {
        let mut overrides: BTreeMap<String, Overrides> = BTreeMap::new();
        let mut error = None;
        for line in lines {
            let Some((keymap, rest)) = line.split_once('\t') else {
                continue;
            };
            match Overrides::from_lines(&[rest.to_owned()]) {
                Ok(parsed) => overrides
                    .entry(keymap.to_owned())
                    .or_default()
                    .actions
                    .extend(parsed.actions),
                Err(parse) => error = Some(format!("ignored keymap override: {parse}")),
            }
        }
        (overrides, error)
    }

    /// The overrides as `keymap<TAB>action<TAB>shortcut…` lines (the `keymap.overrides` setting).
    pub fn override_lines(&self) -> Vec<String> {
        self.overrides
            .iter()
            .flat_map(|(keymap, overrides)| {
                overrides
                    .to_lines()
                    .into_iter()
                    .map(move |line| format!("{keymap}\t{line}"))
            })
            .collect()
    }
}

#[derive(Default)]
struct Inner {
    state: parking_lot::RwLock<Arc<KeymapState>>,
    /// A problem found by the last rebuild, for the status bar; taken once.
    notice: Mutex<Option<String>>,
}

impl Inner {
    /// Rebuilds the keymap from the `keymap.active` and `keymap.overrides` settings.
    fn reload(&self, settings: &Settings) {
        let id = settings
            .text(ids::KEYMAP)
            .unwrap_or_else(|| Desktop::detect().default_keymap().into());
        let (overrides, error) =
            KeymapState::parse_overrides(&settings.list(ids::KEYMAP_OVERRIDES));
        let mut guard = self.state.write();
        let state = Arc::make_mut(&mut guard);
        state.overrides = overrides;
        state.rebuild(&id);
        let notice = error.or_else(|| state.error.clone());
        drop(guard);
        if notice.is_some() {
            *self.notice.lock() = notice;
        }
    }
}

struct Follow(Arc<Inner>);

impl SettingsListener for Follow {
    fn settings_changed(&self, settings: &Settings) {
        self.0.reload(settings);
    }
}

/// The application service: `application.service::<KeymapManager>()`.
pub struct KeymapManager {
    inner: Arc<Inner>,
}

impl KeymapManager {
    /// Builds the keymap from `settings` and follows them on `bus` until `owner` is disposed.
    pub fn connect(bus: &MessageBus, owner: DisposableId, settings: &Settings) -> Self {
        let inner = Arc::new(Inner::default());
        inner.reload(settings);
        let _ = bus.subscribe(owner, SETTINGS_TOPIC, Arc::new(Follow(inner.clone())));
        Self { inner }
    }

    /// The effective keymap (cheap: the snapshot never changes).
    pub fn snapshot(&self) -> Arc<KeymapState> {
        self.inner.state.read().clone()
    }

    /// Changes the in-memory overrides; the caller then stores them in the settings, which
    /// rebuilds the keymap through [`SETTINGS_TOPIC`].
    pub fn with_state_mut<R>(&self, f: impl FnOnce(&mut KeymapState) -> R) -> R {
        f(Arc::make_mut(&mut self.inner.state.write()))
    }

    /// A problem the last rebuild found (an unreadable override, an unknown keymap id).
    pub fn take_notice(&self) -> Option<String> {
        self.inner.notice.lock().take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::SettingsService;
    use archipel_ide_platform::{Application, ComponentManager, ServiceDescriptor};
    use archipel_ide_settings::SettingValue;

    fn application() -> Application {
        let app = Application::new();
        app.register_service(ServiceDescriptor::new(|app: &Application| {
            SettingsService::new(app.message_bus().clone())
        }))
        .unwrap();
        app.register_service(ServiceDescriptor::new(|app: &Application| {
            let settings = app.service::<SettingsService>();
            KeymapManager::connect(app.message_bus(), app.disposable(), &settings.read())
        }))
        .unwrap();
        app
    }

    #[test]
    fn the_keymap_follows_the_settings_topic_without_being_called() {
        let app = application();
        let keymap = app.service::<KeymapManager>();
        let settings = app.service::<SettingsService>();
        settings.with_mut(|s| {
            s.set(ids::KEYMAP, SettingValue::Text("Default for KDE".into()))
                .unwrap()
        });
        assert_ne!(
            keymap.snapshot().keymap.id,
            "Default for KDE",
            "not yet announced"
        );
        settings.notify_changed();
        assert_eq!(keymap.snapshot().keymap.id, "Default for KDE");
    }

    #[test]
    fn a_keymap_created_late_reads_the_current_settings() {
        let app = Application::new();
        app.register_service(ServiceDescriptor::new(|app: &Application| {
            SettingsService::new(app.message_bus().clone())
        }))
        .unwrap();
        app.service::<SettingsService>().with_mut(|s| {
            s.set(ids::KEYMAP, SettingValue::Text("Mac OS X 10.5+".into()))
                .unwrap()
        });
        let settings = app.service::<SettingsService>();
        let keymap = KeymapManager::connect(app.message_bus(), app.disposable(), &settings.read());
        assert_eq!(keymap.snapshot().keymap.id, "Mac OS X 10.5+");
    }

    #[test]
    fn snapshots_are_not_changed_by_later_rebuilds() {
        let app = application();
        let keymap = app.service::<KeymapManager>();
        let before = keymap.snapshot();
        let settings = app.service::<SettingsService>();
        settings.with_mut(|s| {
            s.set(ids::KEYMAP, SettingValue::Text("Default for KDE".into()))
                .unwrap()
        });
        settings.notify_changed();
        assert_ne!(before.keymap.id, keymap.snapshot().keymap.id);
    }
}

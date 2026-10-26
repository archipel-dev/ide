//! Per-user plugin state: disabled plugins and plugin settings, stored as
//! JSON in `<config>/plugins.json`.

use archipel_ide_plugin_api::{PluginError, Value};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Archipel IDE's per-user configuration directory: `$ARCHIPEL_CONFIG_DIR`, or the
/// platform default (`~/.config/archipel`, `~/Library/Application
/// Support/archipel`, `%APPDATA%\archipel`).
pub fn user_config_dir() -> Option<PathBuf> {
    if let Some(dir) = archipel_ide_compat::env_var_os("ARCHIPEL_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    let home = || std::env::var_os("HOME").map(PathBuf::from);
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home().map(|home| home.join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| home().map(|home| home.join(".config")))
    };
    // A directory left by the Jetthing versions is moved to the new name on first use.
    base.map(|base| archipel_ide_compat::adopt_legacy_dir(base.join("archipel")))
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
struct StoreData {
    #[serde(default)]
    disabled: BTreeSet<String>,
    #[serde(default)]
    settings: BTreeMap<String, BTreeMap<String, Value>>,
}

impl StoreData {
    /// Bundled plugins were renamed `dev.jetthing.*` -> `dev.archipel.*`; carry over the
    /// disabled flags and settings saved under the old ids.
    fn migrate_legacy_ids(&mut self) {
        self.disabled = std::mem::take(&mut self.disabled)
            .into_iter()
            .map(|id| archipel_ide_compat::current_plugin_id(&id))
            .collect();
        for (id, values) in std::mem::take(&mut self.settings) {
            self.settings
                .entry(archipel_ide_compat::current_plugin_id(&id))
                .or_default()
                .extend(values);
        }
    }
}

/// Plugin state backed by a JSON file, or kept in memory when `path` is `None`.
#[derive(Debug, Default)]
pub struct PluginStore {
    path: Option<PathBuf>,
    data: StoreData,
    /// Problem reading the file at startup (the store then starts empty).
    pub load_error: Option<String>,
}

impl PluginStore {
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// Loads `path`; a missing file is an empty store, a corrupt one is
    /// reported in [`Self::load_error`] and replaced on the next save.
    pub fn load(path: PathBuf) -> Self {
        let (data, load_error) = match fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(data) => (data, None),
                Err(error) => (
                    StoreData::default(),
                    Some(format!("{}: {error}", path.display())),
                ),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                (StoreData::default(), None)
            }
            Err(error) => (
                StoreData::default(),
                Some(format!("{}: {error}", path.display())),
            ),
        };
        let mut data: StoreData = data;
        data.migrate_legacy_ids();
        Self {
            path: Some(path),
            data,
            load_error,
        }
    }

    /// The user's store at `<config>/plugins.json` (in memory without a config dir).
    pub fn user() -> Self {
        user_config_dir()
            .map(|dir| Self::load(dir.join("plugins.json")))
            .unwrap_or_default()
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Directory for a plugin's own files, next to the store.
    pub fn data_dir(&self, plugin: &str) -> Option<PathBuf> {
        let dir = self
            .path
            .as_deref()
            .and_then(Path::parent)
            .map(|dir| dir.join("plugin-data").join(plugin))?;
        // Data of a bundled plugin saved under its old `dev.jetthing.*` id moves along.
        if !dir.exists()
            && let Some(old) = plugin.strip_prefix("dev.archipel.")
        {
            let legacy = dir.with_file_name(format!("dev.jetthing.{old}"));
            if legacy.is_dir() {
                let _ = fs::rename(legacy, &dir);
            }
        }
        Some(dir)
    }

    pub fn is_disabled(&self, plugin: &str) -> bool {
        self.data.disabled.contains(plugin)
    }

    pub fn set_disabled(&mut self, plugin: &str, disabled: bool) -> Result<(), PluginError> {
        let changed = if disabled {
            self.data.disabled.insert(plugin.into())
        } else {
            self.data.disabled.remove(plugin)
        };
        if changed { self.save() } else { Ok(()) }
    }

    pub fn setting(&self, plugin: &str, key: &str) -> Option<&Value> {
        self.data.settings.get(plugin)?.get(key)
    }

    pub fn set_setting(
        &mut self,
        plugin: &str,
        key: &str,
        value: Value,
    ) -> Result<(), PluginError> {
        self.data
            .settings
            .entry(plugin.into())
            .or_default()
            .insert(key.into(), value);
        self.save()
    }

    fn save(&self) -> Result<(), PluginError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let io =
            |error: std::io::Error| PluginError::Io(path.display().to_string(), error.to_string());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io)?;
        }
        let text = serde_json::to_string_pretty(&self.data)
            .map_err(|error| PluginError::Io(path.display().to_string(), error.to_string()))?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, text).map_err(io)?;
        fs::rename(&temporary, path).map_err(io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_disabled_plugins_and_settings() {
        let dir = std::env::temp_dir().join(format!("archipel-ide-store-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("plugins.json");
        let mut store = PluginStore::load(path.clone());
        assert!(store.load_error.is_none());
        store.set_disabled("a.b", true).unwrap();
        store
            .set_setting("a.b", "greeting", Value::from("hi"))
            .unwrap();
        let reloaded = PluginStore::load(path.clone());
        assert!(reloaded.is_disabled("a.b"));
        assert_eq!(
            reloaded.setting("a.b", "greeting"),
            Some(&Value::from("hi"))
        );
        assert_eq!(reloaded.data_dir("a.b"), Some(dir.join("plugin-data/a.b")));
        fs::write(&path, "{not json").unwrap();
        assert!(PluginStore::load(path).load_error.is_some());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ids_saved_under_the_old_jetthing_names_are_migrated() {
        let dir =
            std::env::temp_dir().join(format!("archipel-ide-store-legacy-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("plugin-data/dev.jetthing.hello")).unwrap();
        fs::write(dir.join("plugin-data/dev.jetthing.hello/x"), "1").unwrap();
        let path = dir.join("plugins.json");
        fs::write(
            &path,
            r#"{"disabled":["dev.jetthing.hello","com.example.x"],"settings":{"dev.jetthing.hello":{"k":1}}}"#,
        )
        .unwrap();
        let store = PluginStore::load(path);
        assert!(store.is_disabled("dev.archipel.hello"));
        assert!(store.is_disabled("com.example.x"));
        assert_eq!(
            store.setting("dev.archipel.hello", "k"),
            Some(&Value::from(1))
        );
        let data = store.data_dir("dev.archipel.hello").unwrap();
        assert!(data.join("x").is_file(), "plugin data moved to the new id");
        fs::remove_dir_all(&dir).unwrap();
    }
}

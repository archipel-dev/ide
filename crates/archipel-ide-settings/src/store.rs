//! Layered setting values and TOML persistence.

use crate::{Scope, SettingValue, SettingsError, SettingsRegistry};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Values that differ from their defaults, per scope.
///
/// Reading a setting checks the project layer, then the application layer, then the
/// default. Writing goes to the setting's own scope; project-scoped settings are written
/// to the application layer while no project is open (they then act as the defaults for
/// projects, like "Settings for New Projects"). Unknown keys read from disk are kept so
/// settings of disabled plugins survive a round trip.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SettingsStore {
    application: BTreeMap<String, SettingValue>,
    project: Option<BTreeMap<String, SettingValue>>,
    application_path: Option<PathBuf>,
    project_path: Option<PathBuf>,
}

/// File name used for both layers.
pub const SETTINGS_FILE: &str = "settings.toml";

/// Per-user configuration directory: `ARCHIPEL_CONFIG_DIR`, else the platform location
/// (`~/Library/Application Support/Archipel`, `%APPDATA%\Archipel`,
/// `$XDG_CONFIG_HOME/archipel` or `~/.config/archipel`).
///
/// A directory left by the Jetthing versions (`.../jetthing`) is moved to the new name on
/// first use (see `archipel_ide_compat::adopt_legacy_dir`); `JETTHING_CONFIG_DIR` is still read.
pub fn default_config_dir() -> Option<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    if let Some(dir) = archipel_ide_compat::env_var_os("ARCHIPEL_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    let dir = if cfg!(target_os = "macos") {
        var("HOME").map(|home| PathBuf::from(home).join("Library/Application Support/Archipel"))
    } else if cfg!(windows) {
        var("APPDATA").map(|dir| PathBuf::from(dir).join("Archipel"))
    } else {
        var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| var("HOME").map(|home| PathBuf::from(home).join(".config")))
            .map(|dir| dir.join("archipel"))
    };
    dir.map(archipel_ide_compat::adopt_legacy_dir)
}

impl SettingsStore {
    /// Loads the application layer from `dir/settings.toml` (missing file = defaults)
    /// and remembers the path for [`SettingsStore::save`].
    pub fn load_application(&mut self, dir: &Path) -> Result<(), SettingsError> {
        let path = dir.join(SETTINGS_FILE);
        // A file that cannot be parsed is left untouched: the layer starts empty and is
        // not written back, so a typo never destroys the user's settings.
        let result = read_layer(&path);
        self.application_path = result.is_ok().then_some(path);
        self.application = result.clone().unwrap_or_default();
        result.map(|_| ())
    }

    /// Loads the project layer from `<project>/.archipel/settings.toml`.
    pub fn load_project(&mut self, project_root: &Path) -> Result<(), SettingsError> {
        let path = project_root.join(".archipel").join(SETTINGS_FILE);
        // Projects from before the rename keep their file in `.jetthing/` until the first save.
        let result = read_layer(&archipel_ide_compat::project_read_path(path.clone()));
        self.project_path = result.is_ok().then_some(path);
        self.project = Some(result.clone().unwrap_or_default());
        result.map(|_| ())
    }

    pub fn close_project(&mut self) {
        self.project = None;
        self.project_path = None;
    }

    pub fn has_project(&self) -> bool {
        self.project.is_some()
    }

    /// Effective value of a registered setting.
    pub fn get(
        &self,
        registry: &SettingsRegistry,
        id: &str,
    ) -> Result<SettingValue, SettingsError> {
        let def = registry
            .setting(id)
            .ok_or_else(|| SettingsError::UnknownSetting(id.into()))?;
        let stored = self
            .project
            .as_ref()
            .and_then(|layer| layer.get(id))
            .or_else(|| self.application.get(id));
        // A value on disk that no longer validates (hand edit, changed options) falls
        // back to the default instead of failing.
        Ok(stored
            .and_then(|value| def.validate(value).ok())
            .unwrap_or_else(|| def.default.clone()))
    }

    pub fn get_bool(&self, registry: &SettingsRegistry, id: &str) -> Option<bool> {
        self.get(registry, id).ok()?.as_bool()
    }

    pub fn get_int(&self, registry: &SettingsRegistry, id: &str) -> Option<i64> {
        self.get(registry, id).ok()?.as_int()
    }

    pub fn get_float(&self, registry: &SettingsRegistry, id: &str) -> Option<f64> {
        self.get(registry, id).ok()?.as_float()
    }

    pub fn get_text(&self, registry: &SettingsRegistry, id: &str) -> Option<String> {
        self.get(registry, id).ok()?.as_str().map(Into::into)
    }

    /// Validates and stores `value`; storing the default removes the override.
    pub fn set(
        &mut self,
        registry: &SettingsRegistry,
        id: &str,
        value: SettingValue,
    ) -> Result<(), SettingsError> {
        let def = registry
            .setting(id)
            .ok_or_else(|| SettingsError::UnknownSetting(id.into()))?;
        let value = def.validate(&value)?;
        let layer = match (def.scope, self.project.as_mut()) {
            (Scope::Project, Some(project)) => project,
            _ => &mut self.application,
        };
        if value == def.default {
            layer.remove(id);
        } else {
            layer.insert(id.into(), value);
        }
        // An application value is shadowed by a stale project override otherwise.
        if def.scope == Scope::Application
            && let Some(project) = self.project.as_mut()
        {
            project.remove(id);
        }
        Ok(())
    }

    /// Removes the override so the default applies again.
    pub fn reset(&mut self, registry: &SettingsRegistry, id: &str) -> Result<(), SettingsError> {
        let def = registry
            .setting(id)
            .ok_or_else(|| SettingsError::UnknownSetting(id.into()))?;
        self.set(registry, id, def.default.clone())
    }

    /// Whether the effective value differs from the default.
    pub fn is_overridden(&self, registry: &SettingsRegistry, id: &str) -> bool {
        match (registry.setting(id), self.get(registry, id)) {
            (Some(def), Ok(value)) => value != def.default,
            _ => false,
        }
    }

    /// Writes both layers to the paths they were loaded from. Layers without a path
    /// (tests, headless use) are kept in memory only.
    pub fn save(&self) -> Result<(), SettingsError> {
        if let Some(path) = &self.application_path {
            write_layer(path, &self.application)?;
        }
        if let (Some(path), Some(layer)) = (&self.project_path, &self.project) {
            // Do not create `.archipel/settings.toml` for projects without overrides.
            if !layer.is_empty() || path.exists() {
                write_layer(path, layer)?;
            }
        }
        Ok(())
    }

    /// Serializes one layer as TOML (sorted keys).
    pub fn to_toml(&self, scope: Scope) -> String {
        let layer = match scope {
            Scope::Application => Some(&self.application),
            Scope::Project => self.project.as_ref(),
        };
        layer.map(layer_to_toml).unwrap_or_default()
    }

    /// Parses a layer from TOML text.
    pub fn parse_layer(text: &str) -> Result<BTreeMap<String, SettingValue>, String> {
        let table: toml::Table = toml::from_str(text).map_err(|error| error.to_string())?;
        let mut layer = BTreeMap::new();
        let settings = match table.get("settings") {
            Some(toml::Value::Table(settings)) => settings,
            Some(_) => return Err("`settings` must be a table".into()),
            None => return Ok(layer),
        };
        for (key, value) in settings {
            let value = match value {
                toml::Value::Boolean(value) => SettingValue::Bool(*value),
                toml::Value::Integer(value) => SettingValue::Int(*value),
                toml::Value::Float(value) => SettingValue::Float(*value),
                toml::Value::String(value) => SettingValue::Text(value.clone()),
                toml::Value::Array(items) => SettingValue::List(
                    items
                        .iter()
                        .map(|item| {
                            item.as_str()
                                .map(Into::into)
                                .ok_or_else(|| format!("`{key}` must be a list of strings"))
                        })
                        .collect::<Result<_, _>>()?,
                ),
                _ => return Err(format!("unsupported value for `{key}`")),
            };
            layer.insert(key.clone(), legacy_value(key, value));
        }
        Ok(layer)
    }
}

/// Option ids that were renamed with the product; files written by older versions
/// still hold the old id (`gradle.buildRunUsing = "jetthing"` became `"archipel-ide"`).
fn legacy_value(key: &str, value: SettingValue) -> SettingValue {
    match value {
        SettingValue::Text(text)
            if key == crate::intellij::ids::GRADLE_BUILD_RUN && text == "jetthing" =>
        {
            SettingValue::Text("archipel-ide".into())
        }
        other => other,
    }
}

fn layer_to_toml(layer: &BTreeMap<String, SettingValue>) -> String {
    let mut settings = toml::Table::new();
    for (key, value) in layer {
        let value = match value {
            SettingValue::Bool(value) => toml::Value::Boolean(*value),
            SettingValue::Int(value) => toml::Value::Integer(*value),
            SettingValue::Float(value) => toml::Value::Float(*value),
            SettingValue::Text(value) => toml::Value::String(value.clone()),
            SettingValue::List(items) => {
                toml::Value::Array(items.iter().cloned().map(toml::Value::String).collect())
            }
        };
        settings.insert(key.clone(), value);
    }
    let mut root = toml::Table::new();
    root.insert("settings".into(), toml::Value::Table(settings));
    toml::to_string(&root).unwrap_or_default()
}

fn read_layer(path: &Path) -> Result<BTreeMap<String, SettingValue>, SettingsError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => {
            return Err(SettingsError::Read {
                path: path.display().to_string(),
                reason: error.to_string(),
            });
        }
    };
    SettingsStore::parse_layer(&text).map_err(|reason| SettingsError::Read {
        path: path.display().to_string(),
        reason,
    })
}

fn write_layer(path: &Path, layer: &BTreeMap<String, SettingValue>) -> Result<(), SettingsError> {
    let error = |reason: std::io::Error| SettingsError::Write {
        path: path.display().to_string(),
        reason: reason.to_string(),
    };
    // First write into a project that only has `.jetthing/` copies it to `.archipel/`.
    archipel_ide_compat::project_write_path(path.to_path_buf());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(error)?;
    }
    // Write to a sibling file first so a crash never leaves a truncated file behind.
    let temporary = path.with_extension("toml.tmp");
    std::fs::write(&temporary, layer_to_toml(layer)).map_err(error)?;
    std::fs::rename(&temporary, path).map_err(error)
}

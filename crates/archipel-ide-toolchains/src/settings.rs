//! `toolchains.toml`: user (`<config>/archipel/toolchains.toml`) and project
//! (`<project>/.archipel/toolchains.toml`) settings.
//!
//! ```toml
//! [components.jdk]
//! version = "21"            # pin a version or prefix
//! distribution = "temurin"  # provider-specific choice
//!
//! [components.rust-analyzer]
//! path = "/opt/rust-analyzer/bin/rust-analyzer"  # use an existing installation
//! ```

use crate::ToolchainError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "toolchains.toml";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentSettings {
    /// Version or version prefix to use (`21`, `1.80.0`, `lts`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// An existing installation: the executable or its home directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// Provider-specific distribution (`temurin`, `corretto`, `zulu`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distribution: Option<String>,
}

impl ComponentSettings {
    fn overlay(&mut self, other: ComponentSettings) {
        if other.version.is_some() {
            self.version = other.version;
        }
        if other.path.is_some() {
            self.path = other.path;
        }
        if other.distribution.is_some() {
            self.distribution = other.distribution;
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolchainSettings {
    #[serde(default)]
    pub components: BTreeMap<String, ComponentSettings>,
}

impl ToolchainSettings {
    pub fn parse(text: &str, file: &Path) -> Result<Self, ToolchainError> {
        toml::from_str(text).map_err(|error| ToolchainError::Settings {
            file: file.display().to_string(),
            detail: error.to_string(),
        })
    }

    pub fn component(&self, id: &str) -> Option<&ComponentSettings> {
        self.components.get(id)
    }

    /// `<config>/archipel/toolchains.toml`.
    pub fn user_file() -> Option<PathBuf> {
        crate::config_dir().map(|dir| dir.join(FILE_NAME))
    }

    pub fn project_file(root: &Path) -> PathBuf {
        root.join(".archipel").join(FILE_NAME)
    }

    fn read(file: &Path) -> Result<Option<Self>, ToolchainError> {
        // Projects from before the rename keep the file in `.jetthing/` until the first save.
        let file = archipel_ide_compat::project_read_path(file.to_path_buf());
        match std::fs::read_to_string(&file) {
            Ok(text) => Self::parse(&text, &file).map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// User settings overlaid with project settings, field by field.
    pub fn load(project_root: Option<&Path>) -> Result<Self, ToolchainError> {
        Self::load_from(
            Self::user_file().as_deref(),
            project_root.map(Self::project_file).as_deref(),
        )
    }

    pub fn load_from(user: Option<&Path>, project: Option<&Path>) -> Result<Self, ToolchainError> {
        let mut merged = Self::default();
        for file in [user, project].into_iter().flatten() {
            if let Some(settings) = Self::read(file)? {
                for (id, component) in settings.components {
                    merged.components.entry(id).or_default().overlay(component);
                }
            }
        }
        Ok(merged)
    }

    /// Updates one component in `file` (creating it), keeping other entries.
    pub fn update_file(
        file: &Path,
        id: &str,
        update: impl FnOnce(&mut ComponentSettings),
    ) -> Result<(), ToolchainError> {
        let mut settings = Self::read(file)?.unwrap_or_default();
        update(settings.components.entry(id.to_owned()).or_default());
        settings
            .components
            .retain(|_, component| *component != ComponentSettings::default());
        let text = toml::to_string_pretty(&settings).map_err(|error| ToolchainError::Settings {
            file: file.display().to_string(),
            detail: error.to_string(),
        })?;
        // First write into a project that only has `.jetthing/` copies it to `.archipel/`.
        archipel_ide_compat::project_write_path(file.to_path_buf());
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file, text)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_overrides_user_per_field_and_updates_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let user = dir.path().join("user.toml");
        let project = dir.path().join("project.toml");
        std::fs::write(
            &user,
            "[components.jdk]\nversion = \"17\"\ndistribution = \"zulu\"\n",
        )
        .unwrap();
        std::fs::write(&project, "[components.jdk]\nversion = \"21\"\n").unwrap();
        let merged = ToolchainSettings::load_from(Some(&user), Some(&project)).unwrap();
        let jdk = merged.component("jdk").unwrap();
        assert_eq!(jdk.version.as_deref(), Some("21"));
        assert_eq!(jdk.distribution.as_deref(), Some("zulu"));

        ToolchainSettings::update_file(&user, "taplo", |c| c.path = Some("/opt/taplo".into()))
            .unwrap();
        let reread = ToolchainSettings::load_from(Some(&user), None).unwrap();
        assert_eq!(reread.components.len(), 2);
        ToolchainSettings::update_file(&user, "taplo", |c| c.path = None).unwrap();
        let reread = ToolchainSettings::load_from(Some(&user), None).unwrap();
        assert!(reread.component("taplo").is_none());
    }

    #[test]
    fn typos_are_reported_with_the_file() {
        let error =
            ToolchainSettings::parse("[components.jdk]\nversoin = \"21\"", Path::new("t.toml"))
                .unwrap_err();
        assert!(error.to_string().contains("t.toml"), "{error}");
    }
}

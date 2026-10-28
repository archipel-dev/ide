//! SDKs (`Sdk`, `SdkTypeId`) and libraries (`Library`, `OrderRootType`).

use crate::path::normalize;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SdkType {
    /// `JavaSdk`: a JDK.
    #[default]
    Jdk,
    /// A Rust toolchain (sysroot).
    Rust,
    Other,
}

/// A configured SDK. For a JDK the class roots are the platform modules image
/// (`lib/modules`) and the source roots hold `lib/src.zip` when present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sdk {
    pub name: String,
    pub sdk_type: SdkType,
    pub home: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default)]
    pub classes: Vec<PathBuf>,
    #[serde(default)]
    pub sources: Vec<PathBuf>,
}

impl Sdk {
    pub fn new(name: impl Into<String>, sdk_type: SdkType, home: impl AsRef<Path>) -> Self {
        Self {
            name: name.into(),
            sdk_type,
            home: normalize(home.as_ref()),
            version: None,
            classes: Vec::new(),
            sources: Vec::new(),
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// A JDK with the conventional layout (`lib/modules`, `lib/src.zip`), checked on disk.
    pub fn jdk_at(home: impl AsRef<Path>, version: Option<&str>) -> Self {
        let home = normalize(home.as_ref());
        let name = match version {
            Some(version) => format!("JDK {version}"),
            None => format!(
                "JDK {}",
                home.file_name().and_then(|n| n.to_str()).unwrap_or("home")
            ),
        };
        let mut sdk = Self::new(name, SdkType::Jdk, &home);
        sdk.version = version.map(str::to_owned);
        let modules = home.join("lib").join("modules");
        if modules.exists() {
            sdk.classes.push(modules);
        } else {
            // A JDK 8 keeps its runtime classes in `rt.jar` (JAVA-RES-036).
            for rt in [
                home.join("jre").join("lib").join("rt.jar"),
                home.join("lib").join("rt.jar"),
            ] {
                if rt.exists() {
                    sdk.classes.push(rt);
                    break;
                }
            }
        }
        for source in [home.join("lib").join("src.zip"), home.join("src.zip")] {
            if source.exists() {
                sdk.sources.push(source);
                break;
            }
        }
        sdk
    }
}

/// Maven coordinates of a library, when the importer knows them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Coordinates {
    pub group: String,
    pub artifact: String,
    pub version: String,
}

impl Coordinates {
    pub fn new(
        group: impl Into<String>,
        artifact: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            group: group.into(),
            artifact: artifact.into(),
            version: version.into(),
        }
    }
}

impl std::fmt::Display for Coordinates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.group, self.artifact, self.version)
    }
}

/// A library with the three root kinds of `OrderRootType`: classes (`CLASSES`), sources
/// (`SOURCES`) and documentation (`DOCUMENTATION`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub classes: Vec<PathBuf>,
    #[serde(default)]
    pub sources: Vec<PathBuf>,
    #[serde(default)]
    pub javadoc: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<Coordinates>,
}

impl Library {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            classes: Vec::new(),
            sources: Vec::new(),
            javadoc: Vec::new(),
            coordinates: None,
        }
    }

    pub fn with_classes(mut self, root: impl AsRef<Path>) -> Self {
        self.classes.push(normalize(root.as_ref()));
        self
    }

    pub fn with_sources(mut self, root: impl AsRef<Path>) -> Self {
        self.sources.push(normalize(root.as_ref()));
        self
    }

    pub fn with_javadoc(mut self, root: impl AsRef<Path>) -> Self {
        self.javadoc.push(normalize(root.as_ref()));
        self
    }

    pub fn with_coordinates(mut self, coordinates: Coordinates) -> Self {
        self.coordinates = Some(coordinates);
        self
    }

    pub fn all_roots(&self) -> impl Iterator<Item = &PathBuf> {
        self.classes
            .iter()
            .chain(&self.sources)
            .chain(&self.javadoc)
    }
}

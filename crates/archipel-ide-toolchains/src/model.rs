//! Component model: what a component is and what one release of it looks like.

use crate::version::ToolVersion;
use serde::{Deserialize, Serialize};

/// The role a component plays for a language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentKind {
    /// Compilers, runtimes and SDKs (JDK, Rust toolchain, Node.js, Android SDK).
    Sdk,
    LanguageServer,
    BuildTool,
    Debugger,
    Formatter,
    Linter,
}

impl ComponentKind {
    pub fn label(self) -> &'static str {
        match self {
            ComponentKind::Sdk => "SDK",
            ComponentKind::LanguageServer => "Language server",
            ComponentKind::BuildTool => "Build tool",
            ComponentKind::Debugger => "Debugger",
            ComponentKind::Formatter => "Formatter",
            ComponentKind::Linter => "Linter",
        }
    }
}

/// Static description of a component, independent of version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentSpec {
    /// Stable id used in settings, pins and commands (`jdk`, `jdtls`, `rust-analyzer`).
    pub id: String,
    /// Display name ("JDK", "Eclipse JDT Language Server").
    pub name: String,
    pub kind: ComponentKind,
    /// Executable names; the first one is the component's primary executable.
    pub executables: Vec<String>,
    /// Directories inside an installation that hold the executables, tried in
    /// order (`bin`, `Contents/Home/bin` for macOS JDKs, `""` for the root).
    pub bin_dirs: Vec<String>,
    /// Environment variables that point at an existing installation home
    /// (`JAVA_HOME`, `ANDROID_HOME`); `<home>/<bin_dir>/<executable>` is checked.
    #[serde(default)]
    pub home_env: Vec<String>,
    /// Ids of components this one needs at runtime (`jdtls` needs `jdk`).
    #[serde(default)]
    pub requires: Vec<String>,
    /// Project homepage, shown in Settings.
    #[serde(default)]
    pub homepage: Option<String>,
}

impl ComponentSpec {
    pub fn new(id: &str, name: &str, kind: ComponentKind, executable: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            executables: vec![executable.into()],
            bin_dirs: vec!["bin".into(), String::new()],
            home_env: Vec::new(),
            requires: Vec::new(),
            homepage: None,
        }
    }

    pub fn primary_executable(&self) -> &str {
        self.executables
            .first()
            .map_or(self.id.as_str(), String::as_str)
    }
}

/// How the expected digest of an artifact is obtained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Checksum {
    /// Known hex SHA-256 digest.
    Sha256(String),
    /// URL of a `sha256sum`-style file (`<hex>  <name>` or a bare digest).
    Sha256Url(String),
}

/// Archive layout of a download.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum ArchiveKind {
    TarGz,
    TarXz,
    Zip,
    /// A single gzip-compressed file, stored as `file_name` (rust-analyzer).
    Gz {
        file_name: String,
    },
    /// A single uncompressed file, stored as `file_name` (bazelisk).
    Raw {
        file_name: String,
    },
}

impl ArchiveKind {
    /// Guesses the archive kind from a URL or file name.
    pub fn from_file_name(name: &str) -> Option<ArchiveKind> {
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
            Some(ArchiveKind::TarGz)
        } else if lower.ends_with(".tar.xz") || lower.ends_with(".txz") {
            Some(ArchiveKind::TarXz)
        } else if lower.ends_with(".zip") {
            Some(ArchiveKind::Zip)
        } else {
            None
        }
    }
}

/// One downloadable file for one platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub url: String,
    pub checksum: Checksum,
    pub archive: ArchiveKind,
    /// Expected size in bytes, if the source publishes it.
    #[serde(default)]
    pub size: Option<u64>,
}

/// A resolved release of a component for a specific platform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    pub component: String,
    pub version: ToolVersion,
    pub artifact: Artifact,
    /// License the user must accept before installing (Android SDK packages).
    #[serde(default)]
    pub license: Option<String>,
}

impl Release {
    /// Download file name: the last URL path segment without query/fragment.
    pub fn file_name(&self) -> String {
        file_name_from_url(&self.artifact.url)
    }
}

pub(crate) fn file_name_from_url(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let name = path.rsplit('/').next().unwrap_or_default();
    let name: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
        .collect();
    if name.is_empty() || name.chars().all(|c| c == '.') {
        "download".into()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_are_sanitised() {
        assert_eq!(
            file_name_from_url("https://x/a/OpenJDK21U-jdk_x64_linux.tar.gz?raw=1"),
            "OpenJDK21U-jdk_x64_linux.tar.gz"
        );
        assert_eq!(file_name_from_url("https://x/../"), "download");
        assert_eq!(file_name_from_url("https://x/.."), "download");
        assert_eq!(
            ArchiveKind::from_file_name("node-v20.tar.xz"),
            Some(ArchiveKind::TarXz)
        );
        assert_eq!(ArchiveKind::from_file_name("bazelisk-linux-amd64"), None);
    }
}

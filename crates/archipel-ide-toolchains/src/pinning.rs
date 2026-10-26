//! Project-level toolchain pins.
//!
//! Archipel IDE respects the version files other tools already use, so a project
//! opens with the same toolchain on the command line and in the IDE.
//! `.archipel/toolchains.toml` overrides them. Detected component ids: `jdk`,
//! `rust`, `bazel`, `node`, `python`, `go`.

use crate::{ToolchainSettings, VersionRequest};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PinSource {
    /// `.archipel/toolchains.toml`
    Archipel,
    /// `rust-toolchain.toml` or `rust-toolchain`
    RustToolchain,
    /// `.bazelversion` (read by bazelisk)
    BazelVersion,
    /// `JavaLanguageVersion.of(n)` / `jvmToolchain(n)` in Gradle build scripts
    GradleToolchain,
    /// `gradle/gradle-daemon-jvm.properties`
    GradleDaemonJvm,
    /// `.sdkmanrc`
    Sdkman,
    /// `.nvmrc` or `.node-version`
    NodeVersion,
    /// `.python-version`
    PythonVersion,
    /// `go`/`toolchain` directives in `go.mod`
    GoMod,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pin {
    pub component: String,
    pub request: VersionRequest,
    pub source: PinSource,
    /// File the pin was read from.
    pub file: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ProjectPins {
    pub pins: Vec<Pin>,
}

impl ProjectPins {
    /// Reads the version files under `root`. `settings` are the merged
    /// `toolchains.toml` settings; their `version` entries win.
    pub fn detect(root: &Path, settings: &ToolchainSettings) -> ProjectPins {
        let mut pins = ProjectPins::default();
        let archipel_file = ToolchainSettings::project_file(root);
        for (id, component) in &settings.components {
            if let Some(version) = &component.version {
                pins.add(
                    id,
                    VersionRequest::parse(version),
                    PinSource::Archipel,
                    &archipel_file,
                );
            }
        }
        for name in ["rust-toolchain.toml", "rust-toolchain"] {
            let file = root.join(name);
            if let Some(channel) = read(&file).and_then(|text| rust_channel(&text)) {
                pins.add(
                    "rust",
                    VersionRequest::parse(&channel),
                    PinSource::RustToolchain,
                    &file,
                );
            }
        }
        let file = root.join(".bazelversion");
        if let Some(version) = read(&file).and_then(|text| first_line(&text)) {
            pins.add(
                "bazel",
                VersionRequest::parse(&version),
                PinSource::BazelVersion,
                &file,
            );
        }
        for file in gradle_scripts(root) {
            if let Some(version) = read(&file).and_then(|text| gradle_java_version(&text)) {
                pins.add(
                    "jdk",
                    VersionRequest::Version(version),
                    PinSource::GradleToolchain,
                    &file,
                );
            }
        }
        let file = root.join("gradle").join("gradle-daemon-jvm.properties");
        if let Some(version) = read(&file).and_then(|text| property(&text, "toolchainVersion")) {
            pins.add(
                "jdk",
                VersionRequest::parse(&version),
                PinSource::GradleDaemonJvm,
                &file,
            );
        }
        let file = root.join(".sdkmanrc");
        if let Some(version) = read(&file)
            .and_then(|text| property(&text, "java"))
            .and_then(|value| leading_version(&value))
        {
            pins.add(
                "jdk",
                VersionRequest::Version(version),
                PinSource::Sdkman,
                &file,
            );
        }
        for name in [".nvmrc", ".node-version"] {
            let file = root.join(name);
            if let Some(request) = read(&file)
                .and_then(|text| first_line(&text))
                .map(|line| node_request(&line))
            {
                pins.add("node", request, PinSource::NodeVersion, &file);
            }
        }
        let file = root.join(".python-version");
        if let Some(version) = read(&file).and_then(|text| first_line(&text)) {
            pins.add(
                "python",
                VersionRequest::parse(&version),
                PinSource::PythonVersion,
                &file,
            );
        }
        let file = root.join("go.mod");
        if let Some(version) = read(&file).and_then(|text| go_version(&text)) {
            pins.add(
                "go",
                VersionRequest::Version(version),
                PinSource::GoMod,
                &file,
            );
        }
        pins
    }

    /// Keeps the first pin per component (sources are read in priority order).
    fn add(&mut self, component: &str, request: VersionRequest, source: PinSource, file: &Path) {
        if self.get(component).is_none() {
            self.pins.push(Pin {
                component: component.to_owned(),
                request,
                source,
                file: file.to_owned(),
            });
        }
    }

    pub fn get(&self, component: &str) -> Option<&Pin> {
        self.pins.iter().find(|pin| pin.component == component)
    }

    /// The pinned request for `component`, else [`VersionRequest::Latest`].
    pub fn request(&self, component: &str) -> VersionRequest {
        self.get(component)
            .map_or(VersionRequest::Latest, |pin| pin.request.clone())
    }
}

fn read(file: &Path) -> Option<String> {
    // Version files are tiny; refuse anything large rather than read it.
    let metadata = fs::metadata(file).ok()?;
    (metadata.is_file() && metadata.len() < 1024 * 1024)
        .then(|| fs::read_to_string(file).ok())
        .flatten()
}

fn first_line(text: &str) -> Option<String> {
    text.lines()
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .find(|line| !line.is_empty())
        .map(str::to_owned)
}

fn rust_channel(text: &str) -> Option<String> {
    if let Ok(value) = text.parse::<toml::Table>()
        && let Some(toolchain) = value.get("toolchain").and_then(|t| t.as_table())
    {
        return toolchain
            .get("channel")
            .and_then(|channel| channel.as_str())
            .map(str::to_owned);
    }
    // Legacy `rust-toolchain`: a bare channel name.
    first_line(text).filter(|line| !line.contains(['[', '=']))
}

fn property(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == key).then(|| value.trim().to_owned())
    })
}

/// `21.0.2-tem` → `21.0.2`.
fn leading_version(value: &str) -> Option<String> {
    let version: String = value
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let version = version.trim_end_matches('.');
    (!version.is_empty()).then(|| version.to_owned())
}

fn node_request(line: &str) -> VersionRequest {
    match line.trim().to_ascii_lowercase().as_str() {
        "node" | "stable" | "current" | "latest" => VersionRequest::Latest,
        "lts/*" => VersionRequest::Channel("lts".into()),
        other => VersionRequest::parse(other),
    }
}

fn go_version(text: &str) -> Option<String> {
    let directive = |name: &str| {
        text.lines().find_map(|line| {
            let rest = line.trim().strip_prefix(name)?;
            rest.starts_with(char::is_whitespace)
                .then(|| rest.trim().trim_start_matches("go").to_owned())
                .filter(|version| version.starts_with(|c: char| c.is_ascii_digit()))
        })
    };
    directive("toolchain").or_else(|| directive("go"))
}

/// Root and first-level module build scripts (Android apps configure the
/// toolchain in `app/build.gradle.kts`).
fn gradle_scripts(root: &Path) -> Vec<PathBuf> {
    let names = ["build.gradle.kts", "build.gradle"];
    let mut scripts: Vec<PathBuf> = names.iter().map(|name| root.join(name)).collect();
    if let Ok(entries) = fs::read_dir(root) {
        let mut modules: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_dir()
                    && !path
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with('.'))
            })
            .collect();
        modules.sort();
        for module in modules {
            scripts.extend(names.iter().map(|name| module.join(name)));
        }
    }
    scripts.into_iter().filter(|path| path.is_file()).collect()
}

/// `JavaLanguageVersion.of(17)`, `JavaLanguageVersion.of("17")`, `jvmToolchain(17)`.
fn gradle_java_version(text: &str) -> Option<String> {
    for marker in ["JavaLanguageVersion.of(", "jvmToolchain("] {
        for (index, _) in text.match_indices(marker) {
            let line_start = text[..index].rfind('\n').map_or(0, |i| i + 1);
            if text[line_start..index].trim_start().starts_with("//") {
                continue;
            }
            let rest = text[index + marker.len()..]
                .trim_start()
                .trim_start_matches('"');
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if !digits.is_empty() {
                return Some(digits);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, name: &str, text: &str) {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn detects_version_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "rust-toolchain.toml",
            "[toolchain]\nchannel = \"1.80.0\"\ncomponents = [\"rustfmt\"]\n",
        );
        write(root, ".bazelversion", "7.3.1\n");
        write(root, ".nvmrc", "lts/*\n");
        write(root, ".python-version", "# pyenv\n3.12.4\n");
        write(
            root,
            "go.mod",
            "module x\n\ngo 1.22\n\ntoolchain go1.22.3\n",
        );
        write(
            root,
            "app/build.gradle.kts",
            "// jvmToolchain(8)\nkotlin {\n    jvmToolchain(17)\n}\n",
        );
        write(root, ".sdkmanrc", "java=21.0.2-tem\n");
        let pins = ProjectPins::detect(root, &ToolchainSettings::default());
        assert_eq!(
            pins.request("rust"),
            VersionRequest::Version("1.80.0".into())
        );
        assert_eq!(
            pins.request("bazel"),
            VersionRequest::Version("7.3.1".into())
        );
        assert_eq!(pins.request("node"), VersionRequest::Channel("lts".into()));
        assert_eq!(
            pins.request("python"),
            VersionRequest::Version("3.12.4".into())
        );
        assert_eq!(pins.request("go"), VersionRequest::Version("1.22.3".into()));
        // The Gradle toolchain wins over .sdkmanrc; comments are ignored.
        let jdk = pins.get("jdk").unwrap();
        assert_eq!(jdk.request, VersionRequest::Version("17".into()));
        assert_eq!(jdk.source, PinSource::GradleToolchain);
        assert_eq!(pins.request("kotlin"), VersionRequest::Latest);
    }

    #[test]
    fn archipel_ide_settings_override_and_legacy_files_parse() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "rust-toolchain", "nightly-2024-05-01\n");
        write(
            root,
            "build.gradle",
            "java { toolchain { languageVersion = JavaLanguageVersion.of(\"11\") } }",
        );
        write(root, "go.mod", "module x\ngo 1.21.0\n");
        let settings =
            ToolchainSettings::parse("[components.jdk]\nversion = \"21\"\n", Path::new("t.toml"))
                .unwrap();
        let pins = ProjectPins::detect(root, &settings);
        assert_eq!(
            pins.request("rust"),
            VersionRequest::Channel("nightly-2024-05-01".into())
        );
        assert_eq!(pins.get("jdk").unwrap().source, PinSource::Archipel);
        assert_eq!(pins.request("jdk"), VersionRequest::Version("21".into()));
        assert_eq!(pins.request("go"), VersionRequest::Version("1.21.0".into()));
    }
}

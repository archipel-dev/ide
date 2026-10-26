//! Android SDK discovery and installed-package inventory.
//!
//! Discovery order follows the Android Gradle Plugin: an explicit IDE setting,
//! then the project's `local.properties` `sdk.dir`, then `ANDROID_HOME`, then
//! the deprecated `ANDROID_SDK_ROOT`, then the per-OS default install folder.
//! Installed packages are read from each package's `package.xml` (written by
//! `sdkmanager` and Android Studio), falling back to `source.properties` for
//! packages unpacked by hand, such as a manually extracted `cmdline-tools`.

use crate::{HostEnvironment, properties};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Where an SDK candidate came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SdkSource {
    /// Configured in the IDE's Android SDK page.
    Setting,
    /// `sdk.dir` in the project's `local.properties`.
    LocalProperties,
    AndroidHome,
    AndroidSdkRoot,
    /// The per-OS default install location.
    Default,
}

impl SdkSource {
    pub fn label(self) -> &'static str {
        match self {
            SdkSource::Setting => "IDE setting",
            SdkSource::LocalProperties => "local.properties sdk.dir",
            SdkSource::AndroidHome => "ANDROID_HOME",
            SdkSource::AndroidSdkRoot => "ANDROID_SDK_ROOT",
            SdkSource::Default => "default location",
        }
    }
}

/// One location that discovery looked at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SdkProbe {
    pub source: SdkSource,
    pub path: PathBuf,
    /// Why the location was rejected; `None` for the accepted one.
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PackageCategory {
    Platform,
    SystemImage,
    Sources,
    BuildTools,
    PlatformTools,
    CommandLineTools,
    Emulator,
    Ndk,
    CMake,
    Other,
}

impl PackageCategory {
    fn from_path(path: &str) -> Self {
        match path.split(';').next().unwrap_or_default() {
            "platforms" => PackageCategory::Platform,
            "system-images" => PackageCategory::SystemImage,
            "sources" => PackageCategory::Sources,
            "build-tools" => PackageCategory::BuildTools,
            "platform-tools" => PackageCategory::PlatformTools,
            "cmdline-tools" | "tools" => PackageCategory::CommandLineTools,
            "emulator" => PackageCategory::Emulator,
            "ndk" | "ndk-bundle" => PackageCategory::Ndk,
            "cmake" => PackageCategory::CMake,
            _ => PackageCategory::Other,
        }
    }

    /// Whether the package belongs on the "SDK Platforms" tab rather than
    /// "SDK Tools".
    pub fn is_platform_component(self) -> bool {
        matches!(
            self,
            PackageCategory::Platform | PackageCategory::SystemImage | PackageCategory::Sources
        )
    }
}

/// An installed SDK package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkPackage {
    /// `sdkmanager` path, e.g. `platforms;android-35` or `build-tools;35.0.0`.
    pub path: String,
    pub display_name: String,
    /// Dotted revision, e.g. `2` or `35.0.0` or `37.0.0 rc1`.
    pub revision: String,
    pub category: PackageCategory,
    /// API level for platforms, system images and sources (`35`, `36.1`, `36-ext18`).
    pub api_level: Option<String>,
    pub obsolete: bool,
    /// Install directory relative to the SDK root.
    pub location: PathBuf,
}

impl SdkPackage {
    /// Name shown in the SDK Platforms table: the Android release name when the
    /// API level is known (`Android 15.0 ("VanillaIceCream")`).
    pub fn platform_title(&self) -> String {
        if self.category == PackageCategory::Platform
            && let Some(name) = self
                .api_level
                .as_deref()
                .and_then(|api| api.split(['.', '-']).next())
                .and_then(|major| major.parse::<u32>().ok())
                .and_then(api_release_name)
        {
            return name;
        }
        self.display_name.clone()
    }
}

/// Release name for an API level, e.g. 35 → `Android 15.0 ("VanillaIceCream")`.
pub fn api_release_name(api: u32) -> Option<String> {
    let (version, codename) = match api {
        21 => ("5.0", "Lollipop"),
        22 => ("5.1", "Lollipop"),
        23 => ("6.0", "Marshmallow"),
        24 => ("7.0", "Nougat"),
        25 => ("7.1.1", "Nougat"),
        26 => ("8.0", "Oreo"),
        27 => ("8.1", "Oreo"),
        28 => ("9.0", "Pie"),
        29 => ("10.0", "Q"),
        30 => ("11.0", "R"),
        31 => ("12.0", "S"),
        32 => ("12L", "Sv2"),
        33 => ("13.0", "Tiramisu"),
        34 => ("14.0", "UpsideDownCake"),
        35 => ("15.0", "VanillaIceCream"),
        36 => ("16.0", "Baklava"),
        37 => ("17.0", "CinnamonBun"),
        _ => return None,
    };
    Some(format!("Android {version} (\"{codename}\")"))
}

/// A validated SDK installation and its packages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AndroidSdk {
    pub root: PathBuf,
    pub source: SdkSource,
    pub packages: Vec<SdkPackage>,
}

impl AndroidSdk {
    /// Scans `root` for installed packages. `root` must already be validated.
    pub fn load(root: &Path, source: SdkSource) -> Self {
        Self {
            root: root.to_owned(),
            source,
            packages: scan_packages(root),
        }
    }

    /// Path of an SDK executable such as `adb`, `emulator` or `avdmanager`,
    /// if installed.
    pub fn tool(&self, name: &str) -> Option<PathBuf> {
        let exe = |dir: PathBuf| {
            [
                name.to_string(),
                format!("{name}.exe"),
                format!("{name}.bat"),
            ]
            .into_iter()
            .map(|file| dir.join(file))
            .find(|path| path.is_file())
        };
        match name {
            "adb" => exe(self.root.join("platform-tools")),
            "emulator" => exe(self.root.join("emulator")),
            _ => {
                // cmdline-tools/latest wins, then any versioned folder, then legacy tools.
                let mut dirs = vec![self.root.join("cmdline-tools/latest/bin")];
                let mut versioned: Vec<_> = self
                    .packages
                    .iter()
                    .filter(|p| p.category == PackageCategory::CommandLineTools)
                    .map(|p| self.root.join(&p.location).join("bin"))
                    .collect();
                versioned.sort();
                versioned.reverse();
                dirs.extend(versioned);
                dirs.into_iter().find_map(exe)
            }
        }
    }

    pub fn platforms(&self) -> impl Iterator<Item = &SdkPackage> {
        self.packages
            .iter()
            .filter(|p| p.category.is_platform_component())
    }

    pub fn tools(&self) -> impl Iterator<Item = &SdkPackage> {
        self.packages
            .iter()
            .filter(|p| !p.category.is_platform_component())
    }
}

/// Outcome of SDK discovery, including every location that was tried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SdkDetection {
    pub sdk: Option<AndroidSdk>,
    pub probes: Vec<SdkProbe>,
}

impl SdkDetection {
    /// Searches the standard locations. `setting` is an SDK path configured in
    /// the IDE; `project_root` supplies `local.properties`.
    pub fn detect(
        setting: Option<&Path>,
        project_root: Option<&Path>,
        env: &dyn HostEnvironment,
    ) -> Self {
        let mut candidates: Vec<(SdkSource, PathBuf)> = Vec::new();
        if let Some(path) = setting {
            candidates.push((SdkSource::Setting, path.to_owned()));
        }
        if let Some(root) = project_root
            && let Some(dir) = local_properties_sdk_dir(root)
        {
            candidates.push((SdkSource::LocalProperties, dir));
        }
        for (var, source) in [
            ("ANDROID_HOME", SdkSource::AndroidHome),
            ("ANDROID_SDK_ROOT", SdkSource::AndroidSdkRoot),
        ] {
            if let Some(value) = env.var(var) {
                candidates.push((source, PathBuf::from(value)));
            }
        }
        if let Some(path) = default_location(env) {
            candidates.push((SdkSource::Default, path));
        }

        let mut probes = Vec::new();
        for (source, path) in candidates {
            match validate(&path) {
                Ok(()) => {
                    probes.push(SdkProbe {
                        source,
                        path: path.clone(),
                        problem: None,
                    });
                    return Self {
                        sdk: Some(AndroidSdk::load(&path, source)),
                        probes,
                    };
                }
                Err(problem) => probes.push(SdkProbe {
                    source,
                    path,
                    problem: Some(problem),
                }),
            }
        }
        Self { sdk: None, probes }
    }

    /// Actionable explanation when no SDK was found.
    pub fn problem(&self) -> Option<String> {
        if self.sdk.is_some() {
            return None;
        }
        let mut message = String::from(
            "Android SDK not found. Set ANDROID_HOME, add sdk.dir to local.properties, \
             or choose the SDK location in Android SDK settings.",
        );
        for probe in &self.probes {
            message.push_str(&format!(
                "\n  {}: {} ({})",
                probe.source.label(),
                probe.path.display(),
                probe.problem.as_deref().unwrap_or("ok")
            ));
        }
        Some(message)
    }
}

/// Checks that `path` looks like an Android SDK root.
pub fn validate(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err("does not exist".into());
    }
    if !path.is_dir() {
        return Err("is not a directory".into());
    }
    const MARKERS: [&str; 6] = [
        "platform-tools",
        "platforms",
        "build-tools",
        "cmdline-tools",
        "emulator",
        "tools",
    ];
    if MARKERS.iter().any(|marker| path.join(marker).is_dir()) {
        Ok(())
    } else {
        Err("contains no platform-tools, platforms, build-tools or cmdline-tools".into())
    }
}

/// `sdk.dir` from `<root>/local.properties`, resolved against `root` when relative.
pub fn local_properties_sdk_dir(root: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(root.join("local.properties")).ok()?;
    let dir = properties::parse(&text).remove("sdk.dir")?;
    let dir = dir.trim();
    if dir.is_empty() {
        return None;
    }
    let path = PathBuf::from(dir);
    Some(if path.is_absolute() {
        path
    } else {
        root.join(path)
    })
}

/// Records `sdk` as `sdk.dir` in `<root>/local.properties`, as Android Studio
/// does on sync, preserving the file's other entries.
pub fn write_local_properties_sdk_dir(root: &Path, sdk: &Path) -> std::io::Result<()> {
    let path = root.join("local.properties");
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    std::fs::write(
        path,
        properties::set(&existing, "sdk.dir", &sdk.to_string_lossy()),
    )
}

/// Where Android Studio's setup wizard installs the SDK.
pub fn default_location(env: &dyn HostEnvironment) -> Option<PathBuf> {
    match env.os() {
        "windows" => env
            .var("LOCALAPPDATA")
            .map(|dir| PathBuf::from(dir).join("Android").join("Sdk")),
        "macos" => env.home_dir().map(|home| home.join("Library/Android/sdk")),
        _ => env.home_dir().map(|home| home.join("Android/Sdk")),
    }
}

fn scan_packages(root: &Path) -> Vec<SdkPackage> {
    let mut packages = Vec::new();
    let mut walker = walkdir::WalkDir::new(root)
        .min_depth(1)
        .max_depth(4)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter();
    while let Some(entry) = walker.next() {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_dir() {
            continue;
        }
        let dir = entry.path();
        let name = entry.file_name().to_string_lossy();
        if name.starts_with('.') || (entry.depth() == 1 && name == "licenses") {
            walker.skip_current_dir();
            continue;
        }
        let Ok(relative) = dir.strip_prefix(root) else {
            continue;
        };
        let package = std::fs::read_to_string(dir.join("package.xml"))
            .ok()
            .and_then(|xml| parse_package_xml(&xml, relative))
            .or_else(|| {
                std::fs::read_to_string(dir.join("source.properties"))
                    .ok()
                    .and_then(|text| parse_source_properties(&text, relative))
            });
        if let Some(package) = package {
            packages.push(package);
            walker.skip_current_dir();
        }
    }
    packages.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then_with(|| api_sort_key(b).cmp(&api_sort_key(a)))
            .then_with(|| b.path.cmp(&a.path))
    });
    packages
}

fn api_sort_key(package: &SdkPackage) -> (u32, u32) {
    let api = package.api_level.as_deref().unwrap_or_default();
    let mut parts = api.split(['.', '-']);
    let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor)
}

fn sdk_path_from_location(relative: &Path) -> String {
    relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(";")
}

/// Parses a package's `package.xml` (`<localPackage path=…>`).
pub(crate) fn parse_package_xml(xml: &str, location: &Path) -> Option<SdkPackage> {
    let document = roxmltree::Document::parse(xml).ok()?;
    let local = document
        .descendants()
        .find(|node| node.has_tag_name("localPackage"))?;
    let path = local
        .attribute("path")
        .map(str::to_owned)
        .unwrap_or_else(|| sdk_path_from_location(location));
    let child = |name: &str| local.children().find(|n| n.has_tag_name(name));
    let text = |node: roxmltree::Node, name: &str| {
        node.children()
            .find(|n| n.has_tag_name(name))
            .and_then(|n| n.text())
            .map(|t| t.trim().to_owned())
    };
    let revision = child("revision")
        .map(|rev| {
            let mut out = ["major", "minor", "micro"]
                .iter()
                .filter_map(|part| text(rev, part))
                .collect::<Vec<_>>()
                .join(".");
            if let Some(preview) = text(rev, "preview") {
                out.push_str(&format!(" rc{preview}"));
            }
            out
        })
        .unwrap_or_default();
    let details = child("type-details");
    let api_level = details.and_then(|d| {
        let api = text(d, "api-level")?;
        let minor = text(d, "api-minor-level").filter(|m| m != "0");
        let mut level = match minor {
            Some(minor) => format!("{api}.{minor}"),
            None => api,
        };
        let base = text(d, "base-extension").is_none_or(|b| b == "true");
        if !base && let Some(ext) = text(d, "extension-level") {
            level.push_str(&format!("-ext{ext}"));
        }
        Some(level)
    });
    let category = PackageCategory::from_path(&path);
    let display_name = child("display-name")
        .and_then(|n| n.text())
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| path.clone());
    Some(SdkPackage {
        display_name,
        revision,
        category,
        api_level,
        obsolete: local.attribute("obsolete") == Some("true"),
        location: location.to_owned(),
        path,
    })
}

/// Fallback for packages without `package.xml`. The `sdkmanager` path is
/// derived from the folder, since e.g. `cmdline-tools/latest` records its
/// versioned path in `Pkg.Path`.
pub(crate) fn parse_source_properties(text: &str, location: &Path) -> Option<SdkPackage> {
    let props = properties::parse(text);
    let description = props.get("Pkg.Desc")?;
    let path = sdk_path_from_location(location);
    Some(SdkPackage {
        category: PackageCategory::from_path(&path),
        display_name: description.clone(),
        revision: props.get("Pkg.Revision").cloned().unwrap_or_default(),
        api_level: props.get("AndroidVersion.ApiLevel").cloned(),
        obsolete: props.get("Pkg.Obsolete").is_some_and(|v| v == "true"),
        location: location.to_owned(),
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::ffi::OsString;

    pub(crate) struct FakeEnv {
        pub vars: HashMap<&'static str, PathBuf>,
        pub home: Option<PathBuf>,
        pub os: &'static str,
    }

    impl HostEnvironment for FakeEnv {
        fn var(&self, name: &str) -> Option<OsString> {
            self.vars.get(name).map(|p| p.clone().into_os_string())
        }
        fn home_dir(&self) -> Option<PathBuf> {
            self.home.clone()
        }
        fn os(&self) -> &str {
            self.os
        }
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "archipel-ide-android-sdk-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const PLATFORM_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><ns2:repository xmlns:ns2="http://schemas.android.com/repository/android/common/02" xmlns:ns11="http://schemas.android.com/sdk/android/repo/repository2/03"><license id="android-sdk-license" type="text">Terms</license><localPackage path="platforms;android-35" obsolete="false"><type-details xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:type="ns11:platformDetailsType"><api-level>35</api-level><extension-level>13</extension-level><base-extension>true</base-extension><layoutlib api="15"/></type-details><revision><major>2</major></revision><display-name>Android SDK Platform 35</display-name><uses-license ref="android-sdk-license"/></localPackage></ns2:repository>"#;

    fn generic_xml(path: &str, name: &str, rev: (u32, u32, u32)) -> String {
        format!(
            r#"<?xml version="1.0"?><ns2:repository xmlns:ns2="http://schemas.android.com/repository/android/common/02"><localPackage path="{path}" obsolete="false"><revision><major>{}</major><minor>{}</minor><micro>{}</micro></revision><display-name>{name}</display-name></localPackage></ns2:repository>"#,
            rev.0, rev.1, rev.2
        )
    }

    pub(crate) fn fake_sdk(root: &Path) {
        let write = |rel: &str, text: &str| {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("platforms/android-35/package.xml", PLATFORM_XML);
        write(
            "platforms/android-36-ext18/package.xml",
            r#"<r><localPackage path="platforms;android-36-ext18"><type-details><api-level>36</api-level><extension-level>18</extension-level><base-extension>false</base-extension></type-details><revision><major>1</major></revision><display-name>Android SDK Platform 36-ext18</display-name></localPackage></r>"#,
        );
        write(
            "build-tools/35.0.0/package.xml",
            &generic_xml(
                "build-tools;35.0.0",
                "Android SDK Build-Tools 35",
                (35, 0, 0),
            ),
        );
        write(
            "platform-tools/package.xml",
            &generic_xml("platform-tools", "Android SDK Platform-Tools", (37, 0, 1)),
        );
        write("platform-tools/adb", "#!/bin/sh\n");
        write(
            "cmdline-tools/latest/source.properties",
            "Pkg.Revision=19.0\nPkg.Path=cmdline-tools;19.0\nPkg.Desc=Android SDK Command-line Tools\n",
        );
        write("cmdline-tools/latest/bin/avdmanager", "#!/bin/sh\n");
        write("licenses/android-sdk-license", "abc\n");
        write("platforms/android-35/broken/package.xml", "not xml");
    }

    #[test]
    fn inventories_package_xml_and_source_properties() {
        let root = temp("inventory");
        fake_sdk(&root);
        let sdk = AndroidSdk::load(&root, SdkSource::AndroidHome);
        let paths: Vec<_> = sdk.packages.iter().map(|p| p.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "platforms;android-36-ext18",
                "platforms;android-35",
                "build-tools;35.0.0",
                "platform-tools",
                "cmdline-tools;latest",
            ]
        );
        let platform = &sdk.packages[1];
        assert_eq!(platform.api_level.as_deref(), Some("35"));
        assert_eq!(platform.revision, "2");
        assert_eq!(
            platform.platform_title(),
            "Android 15.0 (\"VanillaIceCream\")"
        );
        assert_eq!(sdk.packages[0].api_level.as_deref(), Some("36-ext18"));
        assert_eq!(sdk.packages[2].revision, "35.0.0");
        assert_eq!(
            sdk.packages[4].display_name,
            "Android SDK Command-line Tools"
        );
        assert_eq!(sdk.platforms().count(), 2);
        assert_eq!(sdk.tools().count(), 3);
        assert_eq!(sdk.tool("adb"), Some(root.join("platform-tools/adb")));
        assert_eq!(
            sdk.tool("avdmanager"),
            Some(root.join("cmdline-tools/latest/bin/avdmanager"))
        );
        assert_eq!(sdk.tool("emulator"), None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn discovery_prefers_local_properties_then_env_then_default() {
        let base = temp("discovery");
        let home_sdk = base.join("home/Android/Sdk");
        let env_sdk = base.join("env-sdk");
        let project_sdk = base.join("project-sdk");
        for dir in [&home_sdk, &env_sdk, &project_sdk] {
            std::fs::create_dir_all(dir.join("platform-tools")).unwrap();
        }
        let project = base.join("project");
        std::fs::create_dir_all(&project).unwrap();
        let mut env = FakeEnv {
            vars: HashMap::from([
                ("ANDROID_HOME", base.join("missing")),
                ("ANDROID_SDK_ROOT", env_sdk.clone()),
            ]),
            home: Some(base.join("home")),
            os: "linux",
        };

        let found = SdkDetection::detect(None, Some(&project), &env);
        let sdk = found.sdk.as_ref().unwrap();
        assert_eq!(sdk.source, SdkSource::AndroidSdkRoot);
        assert_eq!(found.probes[0].problem.as_deref(), Some("does not exist"));

        std::fs::write(project.join("local.properties"), "sdk.dir=../project-sdk\n").unwrap();
        let found = SdkDetection::detect(None, Some(&project), &env);
        assert_eq!(found.sdk.unwrap().source, SdkSource::LocalProperties);

        std::fs::remove_file(project.join("local.properties")).unwrap();
        env.vars.clear();
        let found = SdkDetection::detect(None, Some(&project), &env);
        assert_eq!(found.sdk.as_ref().unwrap().root, home_sdk);
        assert_eq!(found.sdk.unwrap().source, SdkSource::Default);

        let found = SdkDetection::detect(Some(&env_sdk), Some(&project), &env);
        assert_eq!(found.sdk.unwrap().source, SdkSource::Setting);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn missing_sdk_reports_every_probe() {
        let base = temp("missing");
        std::fs::create_dir_all(base.join("empty")).unwrap();
        let env = FakeEnv {
            vars: HashMap::from([("ANDROID_HOME", base.join("empty"))]),
            home: None,
            os: "linux",
        };
        let found = SdkDetection::detect(None, None, &env);
        assert!(found.sdk.is_none());
        let message = found.problem().unwrap();
        assert!(message.contains("ANDROID_HOME"), "{message}");
        assert!(message.contains("contains no platform-tools"), "{message}");
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn default_locations_per_os() {
        let env = |os| FakeEnv {
            vars: HashMap::from([("LOCALAPPDATA", PathBuf::from("C:/Users/me/AppData/Local"))]),
            home: Some(PathBuf::from("/home/me")),
            os,
        };
        assert_eq!(
            default_location(&env("linux")),
            Some(PathBuf::from("/home/me/Android/Sdk"))
        );
        assert_eq!(
            default_location(&env("macos")),
            Some(PathBuf::from("/home/me/Library/Android/sdk"))
        );
        assert_eq!(
            default_location(&env("windows")),
            Some(PathBuf::from("C:/Users/me/AppData/Local/Android/Sdk"))
        );
    }

    #[test]
    fn malformed_package_files_are_ignored() {
        assert!(parse_package_xml("<oops", Path::new("x")).is_none());
        assert!(parse_package_xml("<repository/>", Path::new("x")).is_none());
        assert!(parse_source_properties("Pkg.Revision=1", Path::new("x")).is_none());
    }
}

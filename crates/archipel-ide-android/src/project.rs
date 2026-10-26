//! Android Gradle project detection.
//!
//! A project is an Android project when one of the Gradle modules listed in
//! `settings.gradle(.kts)` (or the root build itself) applies an Android Gradle
//! Plugin (`com.android.application`, `com.android.library`, …), directly or
//! through a version-catalog alias. Gradle scripts are not evaluated: values
//! that are not literals (`compileSdk = libs.versions.sdk.get().toInt()`) are
//! reported as unknown rather than guessed.

use serde::Serialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{0} is not valid XML: {1}")]
    Xml(PathBuf, String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModuleKind {
    Application,
    Library,
    Test,
    DynamicFeature,
}

impl ModuleKind {
    const PLUGINS: [(&'static str, ModuleKind); 4] = [
        ("com.android.application", ModuleKind::Application),
        ("com.android.library", ModuleKind::Library),
        ("com.android.test", ModuleKind::Test),
        ("com.android.dynamic-feature", ModuleKind::DynamicFeature),
    ];

    fn from_plugin(id: &str) -> Option<Self> {
        Self::PLUGINS
            .iter()
            .find(|(plugin, _)| *plugin == id)
            .map(|(_, kind)| *kind)
    }
}

/// The activity started by the launcher icon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherActivity {
    /// Fully qualified class name, e.g. `com.example.app.MainActivity`.
    pub class_name: String,
    /// As written in the manifest, e.g. `.MainActivity`.
    pub declared_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidModule {
    /// Gradle path, e.g. `:app`; `:` for the root project.
    pub gradle_path: String,
    /// Module directory relative to the project root.
    pub dir: PathBuf,
    pub kind: ModuleKind,
    /// Build script relative to the project root.
    pub build_file: PathBuf,
    pub namespace: Option<String>,
    pub application_id: Option<String>,
    pub compile_sdk: Option<String>,
    pub min_sdk: Option<String>,
    pub target_sdk: Option<String>,
    /// `src/main/AndroidManifest.xml` relative to the project root, if present.
    pub manifest: Option<PathBuf>,
    pub launcher: Option<LauncherActivity>,
    /// Build types × product flavors declared in the build script.
    pub variants: crate::variants::VariantModel,
}

impl AndroidModule {
    /// The application ID used to launch the app: `applicationId`, else
    /// `namespace`, else the manifest `package`.
    pub fn package_name(&self) -> Option<&str> {
        self.application_id.as_deref().or(self.namespace.as_deref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidProject {
    pub root: PathBuf,
    pub agp_version: Option<String>,
    pub has_gradle_wrapper: bool,
    pub modules: Vec<AndroidModule>,
    /// Non-fatal problems, such as an unparsable manifest.
    pub problems: Vec<String>,
}

impl AndroidProject {
    /// Detects an Android Gradle project rooted at `root`. Returns `None` when
    /// no module applies an Android Gradle Plugin.
    pub fn detect(root: &Path) -> Option<Self> {
        let settings = read_first(root, &["settings.gradle.kts", "settings.gradle"]);
        let root_build = read_first(root, &["build.gradle.kts", "build.gradle"]);
        let catalog = Catalog::load(&root.join("gradle/libs.versions.toml"));
        let mut paths = vec![":".to_string()];
        if let Some((_, text)) = &settings {
            paths.extend(included_projects(text));
        }
        let mut problems = Vec::new();
        let mut modules = Vec::new();
        for gradle_path in paths {
            let dir: PathBuf = gradle_path
                .split(':')
                .filter(|part| !part.is_empty())
                .collect();
            let Some((build_name, script)) =
                read_first(&root.join(&dir), &["build.gradle.kts", "build.gradle"])
            else {
                continue;
            };
            let script = strip_comments(&script);
            let Some(kind) = applied_android_plugin(&script, &catalog) else {
                continue;
            };
            let manifest_rel = dir.join("src/main/AndroidManifest.xml");
            let manifest_abs = root.join(&manifest_rel);
            let mut module = AndroidModule {
                build_file: dir.join(build_name),
                namespace: dsl_value(&script, &["namespace"]),
                application_id: dsl_value(&script, &["applicationId"]),
                compile_sdk: dsl_value(&script, &["compileSdk", "compileSdkVersion"]),
                min_sdk: dsl_value(&script, &["minSdk", "minSdkVersion"]),
                target_sdk: dsl_value(&script, &["targetSdk", "targetSdkVersion"]),
                manifest: manifest_abs.is_file().then(|| manifest_rel.clone()),
                launcher: None,
                variants: crate::variants::VariantModel::parse(&script),
                gradle_path,
                dir,
                kind,
            };
            if module.manifest.is_some() {
                match Manifest::read(&manifest_abs) {
                    Ok(manifest) => {
                        if module.namespace.is_none() {
                            module.namespace = manifest.package.clone();
                        }
                        let base = module.namespace.clone().unwrap_or_default();
                        module.launcher = manifest.launcher.map(|declared_name| LauncherActivity {
                            class_name: qualify(&base, &declared_name),
                            declared_name,
                        });
                    }
                    Err(error) => problems.push(error.to_string()),
                }
            }
            modules.push(module);
        }
        if modules.is_empty() {
            return None;
        }
        let agp_version = catalog.agp_version().or_else(|| {
            root_build
                .as_ref()
                .and_then(|(_, text)| agp_version_in_script(&strip_comments(text)))
        });
        Some(Self {
            root: root.to_owned(),
            agp_version,
            has_gradle_wrapper: root.join("gradlew").is_file()
                || root.join("gradlew.bat").is_file(),
            modules,
            problems,
        })
    }

    /// The module to run: the first application module, preferring `:app`.
    pub fn app_module(&self) -> Option<&AndroidModule> {
        let apps = || {
            self.modules
                .iter()
                .filter(|m| m.kind == ModuleKind::Application)
        };
        apps()
            .find(|m| m.gradle_path == ":app")
            .or_else(|| apps().next())
    }
}

fn read_first(dir: &Path, names: &[&str]) -> Option<(String, String)> {
    names.iter().find_map(|name| {
        std::fs::read_to_string(dir.join(name))
            .ok()
            .map(|text| (name.to_string(), text))
    })
}

/// Removes `//` and `/* */` comments outside string literals.
fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(ch) = chars.next() {
        if let Some(q) = quote {
            out.push(ch);
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            } else if ch == q || ch == '\n' {
                quote = None;
            }
            continue;
        }
        match (ch, chars.peek()) {
            ('"' | '\'', _) => {
                quote = Some(ch);
                out.push(ch);
            }
            ('/', Some('/')) => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut prev = ' ';
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                    }
                    if prev == '*' && c == '/' {
                        break;
                    }
                    prev = c;
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Quoted string literals in `text`.
fn string_literals(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '"' || ch == '\'' {
            let literal: String = chars.by_ref().take_while(|c| *c != ch).collect();
            out.push(literal);
        }
    }
    out
}

/// Project paths from `include(":app", ":lib")` / `include ':app', ':lib'`.
fn included_projects(settings: &str) -> Vec<String> {
    let text = strip_comments(settings);
    let mut projects = Vec::new();
    let mut rest = text.as_str();
    while let Some(index) = rest.find("include") {
        let before = rest[..index].chars().next_back();
        let after_keyword = &rest[index + "include".len()..];
        rest = after_keyword;
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
            || after_keyword
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        let trimmed = after_keyword.trim_start();
        let statement = if let Some(args) = trimmed.strip_prefix('(') {
            args.split(')').next().unwrap_or_default().to_owned()
        } else {
            // Groovy: arguments run to the end of the line, continuing after a trailing comma.
            let mut statement = String::new();
            for line in trimmed.lines() {
                statement.push_str(line);
                if !line.trim_end().ends_with(',') {
                    break;
                }
            }
            statement
        };
        for literal in string_literals(&statement) {
            let path = if literal.starts_with(':') {
                literal
            } else {
                format!(":{literal}")
            };
            if path.len() > 1 && !path.contains(['/', '\\']) && !projects.contains(&path) {
                projects.push(path);
            }
        }
    }
    projects
}

fn applied_android_plugin(script: &str, catalog: &Catalog) -> Option<ModuleKind> {
    // `id(...) apply false` only puts the plugin on the classpath (root scripts).
    let script: String = script
        .lines()
        .filter(|line| !line.contains("apply false") && !line.contains("apply(false)"))
        .collect::<Vec<_>>()
        .join("\n");
    let script = script.as_str();
    for literal in string_literals(script) {
        if let Some(kind) = ModuleKind::from_plugin(&literal) {
            return Some(kind);
        }
    }
    // alias(libs.plugins.android.application)
    let mut rest = script;
    while let Some(index) = rest.find("alias(") {
        rest = &rest[index + "alias(".len()..];
        let reference = rest.split(')').next().unwrap_or_default().trim();
        if let Some(key) = reference
            .split_once(".plugins.")
            .map(|(_, key)| normalize_alias(key))
            && let Some(kind) = catalog
                .plugin_id(&key)
                .and_then(|id| ModuleKind::from_plugin(&id))
        {
            return Some(kind);
        }
    }
    None
}

/// Value of the first `key = value` / `key value` / `key(value)` DSL assignment
/// that is a string or integer literal.
fn dsl_value(script: &str, keys: &[&str]) -> Option<String> {
    // Statements end at newlines, braces and semicolons (`android { namespace = "x" }`).
    for line in script.split(['\n', '{', '}', ';']) {
        let line = line.trim();
        for key in keys {
            let Some(rest) = line.strip_prefix(key) else {
                continue;
            };
            if rest.starts_with(|c: char| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let value = rest
                .trim_start()
                .trim_start_matches(['=', '('])
                .trim()
                .trim_end_matches(')')
                .trim();
            if let Some(literal) = string_literals(value).into_iter().next()
                && (value.starts_with('"') || value.starts_with('\''))
            {
                return Some(literal);
            }
            if !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()) {
                return Some(value.to_owned());
            }
        }
    }
    None
}

fn agp_version_in_script(script: &str) -> Option<String> {
    for line in script.lines() {
        if let Some(index) = line.find("com.android.tools.build:gradle:") {
            let version: String = line[index + "com.android.tools.build:gradle:".len()..]
                .chars()
                .take_while(|c| !matches!(c, '"' | '\'' | ')' | ' '))
                .collect();
            if !version.is_empty() {
                return Some(version);
            }
        }
        let literals = string_literals(line);
        if literals
            .first()
            .is_some_and(|id| ModuleKind::from_plugin(id).is_some())
            && let Some(after) = line.split_once(" version ").map(|(_, after)| after)
            && let Some(version) = string_literals(after).into_iter().next()
        {
            return Some(version);
        }
    }
    None
}

fn normalize_alias(key: &str) -> String {
    key.trim()
        .chars()
        .map(|c| if matches!(c, '-' | '_') { '.' } else { c })
        .collect::<String>()
        .to_ascii_lowercase()
}

/// The subset of `gradle/libs.versions.toml` needed for plugin resolution.
#[derive(Default)]
struct Catalog {
    versions: toml::Table,
    plugins: toml::Table,
}

impl Catalog {
    fn load(path: &Path) -> Self {
        let Some(table) = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| text.parse::<toml::Table>().ok())
        else {
            return Self::default();
        };
        let section = |name: &str| {
            table
                .get(name)
                .and_then(toml::Value::as_table)
                .cloned()
                .unwrap_or_default()
        };
        Self {
            versions: section("versions"),
            plugins: section("plugins"),
        }
    }

    fn plugin(&self, key: &str) -> Option<&toml::Value> {
        self.plugins
            .iter()
            .find(|(name, _)| normalize_alias(name) == key)
            .map(|(_, value)| value)
    }

    fn plugin_id(&self, key: &str) -> Option<String> {
        match self.plugin(key)? {
            toml::Value::String(notation) => notation.split(':').next().map(str::to_owned),
            toml::Value::Table(table) => table.get("id")?.as_str().map(str::to_owned),
            _ => None,
        }
    }

    fn agp_version(&self) -> Option<String> {
        self.plugins.values().find_map(|value| match value {
            toml::Value::String(notation) => {
                let (id, version) = notation.split_once(':')?;
                ModuleKind::from_plugin(id).map(|_| version.to_owned())
            }
            toml::Value::Table(table) => {
                ModuleKind::from_plugin(table.get("id")?.as_str()?)?;
                match table.get("version")? {
                    toml::Value::String(version) => Some(version.clone()),
                    toml::Value::Table(version) => {
                        let reference = version.get("ref")?.as_str()?;
                        self.versions.get(reference)?.as_str().map(str::to_owned)
                    }
                    _ => None,
                }
            }
            _ => None,
        })
    }
}

const ANDROID_NS: &str = "http://schemas.android.com/apk/res/android";

/// Facts read from an `AndroidManifest.xml`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Manifest {
    pub package: Option<String>,
    /// `android:name` of the launcher activity as declared.
    pub launcher: Option<String>,
}

impl Manifest {
    pub fn read(path: &Path) -> Result<Self, ProjectError> {
        let text = std::fs::read_to_string(path).map_err(|source| ProjectError::Io {
            path: path.to_owned(),
            source,
        })?;
        Self::parse(&text).map_err(|error| ProjectError::Xml(path.to_owned(), error))
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let document = roxmltree::Document::parse(text).map_err(|e| e.to_string())?;
        let root = document.root_element();
        let package = root.attribute("package").map(str::to_owned);
        let launcher = root
            .descendants()
            .filter(|node| node.has_tag_name("activity") || node.has_tag_name("activity-alias"))
            .filter(|activity| activity.attribute((ANDROID_NS, "enabled")) != Some("false"))
            .find(|activity| {
                activity
                    .children()
                    .filter(|node| node.has_tag_name("intent-filter"))
                    .any(|filter| {
                        let has = |tag: &str, values: &[&str]| {
                            filter.children().any(|node| {
                                node.has_tag_name(tag)
                                    && node
                                        .attribute((ANDROID_NS, "name"))
                                        .is_some_and(|name| values.contains(&name))
                            })
                        };
                        has("action", &["android.intent.action.MAIN"])
                            && has(
                                "category",
                                &[
                                    "android.intent.category.LAUNCHER",
                                    "android.intent.category.LEANBACK_LAUNCHER",
                                ],
                            )
                    })
            })
            .and_then(|activity| activity.attribute((ANDROID_NS, "name")))
            .map(str::to_owned);
        Ok(Self { package, launcher })
    }
}

/// Resolves a manifest class name against the namespace the way aapt does:
/// `.Main` and `Main` become `<namespace>.Main`.
fn qualify(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        return name.trim_start_matches('.').to_owned();
    }
    if let Some(relative) = name.strip_prefix('.') {
        format!("{namespace}.{relative}")
    } else if name.contains('.') {
        name.to_owned()
    } else {
        format!("{namespace}.{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "archipel-ide-android-project-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    const MANIFEST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    xmlns:tools="http://schemas.android.com/tools">
    <application android:label="@string/app_name">
        <activity android:name=".SettingsActivity" android:exported="false" />
        <activity android:name=".MainActivity" android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>"#;

    /// Mirrors the Android Studio "Empty Views Activity" template (Kotlin DSL
    /// with a version catalog).
    pub(crate) fn studio_template(root: &Path) {
        write(
            root,
            "settings.gradle.kts",
            r#"pluginManagement {
    repositories { google(); mavenCentral(); gradlePluginPortal() }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories { google(); mavenCentral() }
}
rootProject.name = "My Application"
include(":app")
// include(":disabled")
include(":core:ui")
"#,
        );
        write(
            root,
            "build.gradle.kts",
            "plugins {\n    alias(libs.plugins.android.application) apply false\n    alias(libs.plugins.kotlin.android) apply false\n}\n",
        );
        write(
            root,
            "gradle/libs.versions.toml",
            r#"[versions]
agp = "8.13.0"
kotlin = "2.0.21"

[libraries]
androidx-core-ktx = { group = "androidx.core", name = "core-ktx", version = "1.10.1" }

[plugins]
android-application = { id = "com.android.application", version.ref = "agp" }
android-library = { id = "com.android.library", version.ref = "agp" }
kotlin-android = { id = "org.jetbrains.kotlin.android", version.ref = "kotlin" }
"#,
        );
        write(
            root,
            "app/build.gradle.kts",
            r#"plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
}

android {
    namespace = "com.example.myapplication"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.example.myapplication"
        minSdk = 24
        targetSdk = 36 // latest
        versionCode = 1
    }
}
"#,
        );
        write(root, "app/src/main/AndroidManifest.xml", MANIFEST);
        write(
            root,
            "core/ui/build.gradle.kts",
            "plugins { alias(libs.plugins.android.library) }\nandroid {\n    namespace = \"com.example.core.ui\"\n    compileSdk = libs.versions.compileSdk.get().toInt()\n}\n",
        );
        write(root, "gradlew", "#!/bin/sh\n");
    }

    #[test]
    fn detects_studio_template_with_version_catalog() {
        let root = temp("template");
        studio_template(&root);
        let project = AndroidProject::detect(&root).unwrap();
        assert_eq!(project.agp_version.as_deref(), Some("8.13.0"));
        assert!(project.has_gradle_wrapper);
        assert_eq!(project.modules.len(), 2);
        let app = project.app_module().unwrap();
        assert_eq!(app.gradle_path, ":app");
        assert_eq!(app.kind, ModuleKind::Application);
        assert_eq!(app.namespace.as_deref(), Some("com.example.myapplication"));
        assert_eq!(app.package_name(), Some("com.example.myapplication"));
        assert_eq!(app.compile_sdk.as_deref(), Some("36"));
        assert_eq!(app.min_sdk.as_deref(), Some("24"));
        assert_eq!(app.target_sdk.as_deref(), Some("36"));
        assert_eq!(
            app.manifest.as_deref(),
            Some(Path::new("app/src/main/AndroidManifest.xml"))
        );
        assert_eq!(
            app.launcher.as_ref().unwrap().class_name,
            "com.example.myapplication.MainActivity"
        );
        let library = &project.modules[1];
        assert_eq!(library.gradle_path, ":core:ui");
        assert_eq!(library.dir, PathBuf::from("core/ui"));
        assert_eq!(library.kind, ModuleKind::Library);
        assert_eq!(library.compile_sdk, None, "non-literal values stay unknown");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn detects_legacy_groovy_project() {
        let root = temp("groovy");
        write(
            &root,
            "settings.gradle",
            "include ':app',\n    ':lib'\ninclude 'wear'\nincludeBuild('build-logic')\n",
        );
        write(
            &root,
            "build.gradle",
            "buildscript {\n  dependencies { classpath 'com.android.tools.build:gradle:7.4.2' }\n}\n",
        );
        write(
            &root,
            "app/build.gradle",
            "apply plugin: 'com.android.application'\nandroid {\n  compileSdkVersion 33\n  defaultConfig {\n    applicationId 'org.legacy.app'\n    minSdkVersion 21\n  }\n}\n",
        );
        write(
            &root,
            "app/src/main/AndroidManifest.xml",
            r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="org.legacy"><application><activity-alias android:name="Launcher" android:targetActivity=".Main"><intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter></activity-alias></application></manifest>"#,
        );
        write(&root, "lib/build.gradle", "plugins { id 'java-library' }\n");
        write(
            &root,
            "wear/build.gradle",
            "plugins { id 'com.android.application' }\n",
        );
        write(&root, "wear/src/main/AndroidManifest.xml", "<manifest");
        let project = AndroidProject::detect(&root).unwrap();
        assert_eq!(project.agp_version.as_deref(), Some("7.4.2"));
        assert!(!project.has_gradle_wrapper);
        let paths: Vec<_> = project
            .modules
            .iter()
            .map(|m| m.gradle_path.as_str())
            .collect();
        assert_eq!(paths, [":app", ":wear"]);
        let app = &project.modules[0];
        assert_eq!(app.compile_sdk.as_deref(), Some("33"));
        assert_eq!(app.min_sdk.as_deref(), Some("21"));
        assert_eq!(app.application_id.as_deref(), Some("org.legacy.app"));
        assert_eq!(app.namespace.as_deref(), Some("org.legacy"));
        assert_eq!(
            app.launcher.as_ref().unwrap().class_name,
            "org.legacy.Launcher"
        );
        assert_eq!(project.problems.len(), 1, "{:?}", project.problems);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn one_line_blocks() {
        let script = "android { namespace = \"com.example.app\"; defaultConfig { minSdk = 26 } }";
        assert_eq!(
            dsl_value(script, &["namespace"]).as_deref(),
            Some("com.example.app")
        );
        assert_eq!(dsl_value(script, &["minSdk"]).as_deref(), Some("26"));
    }

    #[test]
    fn plugins_block_with_version_in_root_script() {
        let script =
            "plugins {\n    id(\"com.android.application\") version \"8.5.0\" apply false\n}\n";
        assert_eq!(agp_version_in_script(script).as_deref(), Some("8.5.0"));
    }

    #[test]
    fn non_android_projects_are_not_detected() {
        let root = temp("plain");
        write(&root, "settings.gradle.kts", "include(\"app\")\n");
        write(&root, "app/build.gradle.kts", "plugins { application }\n");
        assert!(AndroidProject::detect(&root).is_none());
        let empty = temp("empty");
        assert!(AndroidProject::detect(&empty).is_none());
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(empty).unwrap();
    }

    #[test]
    fn qualifies_class_names_like_aapt() {
        assert_eq!(qualify("a.b", ".Main"), "a.b.Main");
        assert_eq!(qualify("a.b", "Main"), "a.b.Main");
        assert_eq!(qualify("a.b", "x.y.Main"), "x.y.Main");
        assert_eq!(qualify("", ".Main"), "Main");
    }

    #[test]
    fn manifest_without_launcher() {
        let manifest = Manifest::parse(
            r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android"><application><activity android:name=".A" android:enabled="false"><intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter></activity></application></manifest>"#,
        )
        .unwrap();
        assert_eq!(manifest.launcher, None);
        assert!(Manifest::parse("<<<").is_err());
    }
}

//! Inspection profiles: TOML files with layering.
//!
//! ```toml
//! base = "Default"                 # optional; every profile layers on a base
//!
//! [tools.EmptyCatchBlock]
//! enabled = true
//! level = "error"                  # error | warning | weak_warning | information | text_attributes
//!
//! [[tools.TodoComment.scopes]]     # per-scope overrides, first match wins
//! scope = "tests"                  # all | production | tests | generated | <glob>
//! enabled = false
//!
//! [tools.LongLine.options]
//! max = 140
//! ```
//!
//! Profiles live in `<project>/.archipel/inspection-profiles/<name>.toml`
//! (project) and `<config>/inspection-profiles/<name>.toml` (application); a
//! project profile shadows an application profile of the same name. The
//! built-in `Default` profile is empty (every tool at its own default); the
//! selected profile name is the `inspections.profile` setting (default
//! `Project Default`, which is `Default` until a file of that name exists).

use archipel_ide_analysis::inspect::{
    InspectionOptions, Level, LocalInspection, OptionKind, OptionValue,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::engine::ToolConfig;

pub const DEFAULT_PROFILE: &str = "Default";
pub const PROJECT_DEFAULT_PROFILE: &str = "Project Default";

/// `Level` <-> its TOML spelling.
pub fn level_name(level: Level) -> &'static str {
    match level {
        Level::Error => "error",
        Level::Warning => "warning",
        Level::WeakWarning => "weak_warning",
        Level::Information => "information",
        Level::TextAttributes => "text_attributes",
    }
}

pub fn parse_level(name: &str) -> Option<Level> {
    Some(match name {
        "error" => Level::Error,
        "warning" => Level::Warning,
        "weak_warning" => Level::WeakWarning,
        "information" => Level::Information,
        "text_attributes" => Level::TextAttributes,
        _ => return None,
    })
}

/// Per-scope override of one tool.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScopeEntry {
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
}

/// State of one tool in a profile; absent fields inherit.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<ScopeEntry>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub options: BTreeMap<String, toml::Value>,
}

impl ToolEntry {
    fn is_empty(&self) -> bool {
        self.enabled.is_none()
            && self.level.is_none()
            && self.scopes.is_empty()
            && self.options.is_empty()
    }

    /// `self` layered over `base`.
    fn over(&self, base: &ToolEntry) -> ToolEntry {
        let mut scopes = base.scopes.clone();
        for scope in &self.scopes {
            match scopes.iter_mut().find(|s| s.scope == scope.scope) {
                Some(existing) => *existing = scope.clone(),
                None => scopes.push(scope.clone()),
            }
        }
        let mut options = base.options.clone();
        options.extend(self.options.iter().map(|(k, v)| (k.clone(), v.clone())));
        ToolEntry {
            enabled: self.enabled.or(base.enabled),
            level: self.level.clone().or_else(|| base.level.clone()),
            scopes,
            options,
        }
    }
}

/// One profile file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tools: BTreeMap<String, ToolEntry>,
}

/// Why a profile could not be read or resolved.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ProfileError {
    #[error("invalid inspection profile: {0}")]
    Parse(String),
    #[error("unknown inspection profile `{0}`")]
    Unknown(String),
    #[error("inspection profile `{0}` layers on itself")]
    Cycle(String),
    #[error("cannot read inspection profiles in {path}: {message}")]
    Io { path: PathBuf, message: String },
}

impl ProfileFile {
    pub fn from_toml(text: &str) -> Result<Self, ProfileError> {
        toml::from_str(text).map_err(|e| ProfileError::Parse(e.to_string()))
    }

    pub fn to_toml(&self) -> Result<String, ProfileError> {
        toml::to_string_pretty(self).map_err(|e| ProfileError::Parse(e.to_string()))
    }

    fn entry_mut(&mut self, tool: &str) -> &mut ToolEntry {
        self.tools.entry(tool.to_owned()).or_default()
    }

    /// Drops entries that say nothing.
    pub fn prune(&mut self) {
        self.tools.retain(|_, entry| !entry.is_empty());
    }

    pub fn set_enabled(&mut self, tool: &str, enabled: Option<bool>) {
        self.entry_mut(tool).enabled = enabled;
        self.prune();
    }

    pub fn set_level(&mut self, tool: &str, level: Option<Level>) {
        self.entry_mut(tool).level = level.map(|l| level_name(l).to_owned());
        self.prune();
    }

    pub fn set_option(&mut self, tool: &str, option: &str, value: Option<toml::Value>) {
        let entry = self.entry_mut(tool);
        match value {
            Some(value) => {
                entry.options.insert(option.to_owned(), value);
            }
            None => {
                entry.options.remove(option);
            }
        }
        self.prune();
    }
}

/// Whether `path` is in the named scope.
pub fn scope_matches(scope: &str, path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let segments: Vec<&str> = normalized.split('/').collect();
    let name = segments.last().copied().unwrap_or("");
    let dirs = &segments[..segments.len().saturating_sub(1)];
    let is_test = dirs.iter().any(|d| {
        matches!(
            *d,
            "test" | "tests" | "__tests__" | "androidTest" | "testFixtures" | "spec" | "specs"
        )
    }) || name.ends_with("Test.java")
        || name.ends_with("Tests.java")
        || name.ends_with("Test.kt")
        || name.ends_with("_test.go")
        || name.ends_with("_test.rs")
        || (name.starts_with("test_") && name.ends_with(".py"))
        || name.ends_with("_test.py")
        || name.contains(".test.")
        || name.contains(".spec.");
    let is_generated = dirs.iter().any(|d| {
        matches!(
            *d,
            "generated" | "build" | "target" | "node_modules" | "dist" | "out"
        )
    }) || name.contains(".generated.")
        || name.ends_with(".g.dart");
    match scope {
        "all" | "" => true,
        "tests" => is_test,
        "production" => !is_test && !is_generated,
        "generated" => is_generated,
        glob => glob_match(glob, &normalized),
    }
}

/// `*` (any run without `/`), `**` (any run) and `?`.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    fn go(p: &[u8], t: &[u8]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some(b'*') => {
                let deep = p.get(1) == Some(&b'*');
                let rest = &p[if deep { 2 } else { 1 }..];
                (0..=t.len()).any(|i| go(rest, &t[i..]) && (deep || !t[..i].contains(&b'/')))
            }
            Some(b'?') => !t.is_empty() && t[0] != b'/' && go(&p[1..], &t[1..]),
            Some(c) => t.first() == Some(c) && go(&p[1..], &t[1..]),
        }
    }
    // A pattern without a slash matches the file name anywhere.
    if !pattern.contains('/') {
        let name = text.rsplit('/').next().unwrap_or(text);
        return go(pattern.as_bytes(), name.as_bytes());
    }
    go(pattern.as_bytes(), text.as_bytes())
}

/// A profile with its base chain applied.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectiveProfile {
    pub name: String,
    tools: BTreeMap<String, ToolEntry>,
}

/// A problem found while checking a profile against the registered tools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileIssue {
    UnknownTool(String),
    UnknownLevel { tool: String, level: String },
    UnknownOption { tool: String, option: String },
    BadOptionValue { tool: String, option: String },
}

impl EffectiveProfile {
    pub fn entry(&self, tool: &str) -> Option<&ToolEntry> {
        self.tools.get(tool)
    }

    /// The `(enabled, level)` of `tool` for a file at `path`: the first
    /// matching scope override, else the profile value, else the tool's own
    /// default.
    pub fn state_of(&self, tool: &dyn LocalInspection, path: &Path) -> (bool, Level) {
        self.state_for(
            tool.id(),
            tool.enabled_by_default(),
            tool.default_level(),
            path,
        )
    }

    /// [`EffectiveProfile::state_of`] for a tool known by id and defaults.
    pub fn state_for(&self, id: &str, enabled: bool, level: Level, path: &Path) -> (bool, Level) {
        let (mut enabled, mut level) = (enabled, level);
        if let Some(entry) = self.tools.get(id) {
            if let Some(value) = entry.enabled {
                enabled = value;
            }
            if let Some(value) = entry.level.as_deref().and_then(parse_level) {
                level = value;
            }
            if let Some(scope) = entry.scopes.iter().find(|s| scope_matches(&s.scope, path)) {
                if let Some(value) = scope.enabled {
                    enabled = value;
                }
                if let Some(value) = scope.level.as_deref().and_then(parse_level) {
                    level = value;
                }
            }
        }
        (enabled, level)
    }

    /// Option values of `tool`: schema defaults overridden by the profile;
    /// values of the wrong type or range fall back (ints are clamped).
    pub fn options_of(&self, tool: &dyn LocalInspection) -> InspectionOptions {
        let mut options = InspectionOptions::default();
        let entry = self.tools.get(tool.id());
        for def in tool.options() {
            let given = entry.and_then(|e| e.options.get(def.id));
            let value = match (&def.kind, given) {
                (OptionKind::Bool(_), Some(toml::Value::Boolean(v))) => OptionValue::Bool(*v),
                (OptionKind::Bool(d), _) => OptionValue::Bool(*d),
                (OptionKind::Int { min, max, .. }, Some(toml::Value::Integer(v))) => {
                    OptionValue::Int((*v).clamp(*min, *max))
                }
                (OptionKind::Int { default, .. }, _) => OptionValue::Int(*default),
                (OptionKind::Text(_), Some(toml::Value::String(v))) => OptionValue::Text(v.clone()),
                (OptionKind::Text(d), _) => OptionValue::Text((*d).to_owned()),
                (OptionKind::Choice { choices, .. }, Some(toml::Value::String(v)))
                    if choices.contains(&v.as_str()) =>
                {
                    OptionValue::Text(v.clone())
                }
                (OptionKind::Choice { default, .. }, _) => OptionValue::Text((*default).to_owned()),
            };
            options.0.insert(def.id, value);
        }
        options
    }

    /// `tool` configured for a file at `path`, or `None` when disabled.
    pub fn configure(&self, tool: &Arc<dyn LocalInspection>, path: &Path) -> Option<ToolConfig> {
        let (enabled, level) = self.state_of(tool.as_ref(), path);
        enabled.then(|| ToolConfig {
            tool: tool.clone(),
            level,
            options: self.options_of(tool.as_ref()),
        })
    }

    /// Checks the profile against the known tools.
    pub fn validate(&self, tools: &[Arc<dyn LocalInspection>]) -> Vec<ProfileIssue> {
        let mut issues = Vec::new();
        for (id, entry) in &self.tools {
            let Some(tool) = tools.iter().find(|t| t.id() == id) else {
                issues.push(ProfileIssue::UnknownTool(id.clone()));
                continue;
            };
            let levels = entry
                .level
                .iter()
                .chain(entry.scopes.iter().filter_map(|s| s.level.as_ref()));
            for level in levels {
                if parse_level(level).is_none() {
                    issues.push(ProfileIssue::UnknownLevel {
                        tool: id.clone(),
                        level: level.clone(),
                    });
                }
            }
            for (name, value) in &entry.options {
                match tool.options().iter().find(|d| d.id == name) {
                    None => issues.push(ProfileIssue::UnknownOption {
                        tool: id.clone(),
                        option: name.clone(),
                    }),
                    Some(def) => {
                        let ok = matches!(
                            (&def.kind, value),
                            (OptionKind::Bool(_), toml::Value::Boolean(_))
                                | (OptionKind::Int { .. }, toml::Value::Integer(_))
                                | (OptionKind::Text(_), toml::Value::String(_))
                        ) || matches!((&def.kind, value), (OptionKind::Choice { choices, .. }, toml::Value::String(v)) if choices.contains(&v.as_str()));
                        if !ok {
                            issues.push(ProfileIssue::BadOptionValue {
                                tool: id.clone(),
                                option: name.clone(),
                            });
                        }
                    }
                }
            }
        }
        issues
    }
}

/// Where a profile came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    BuiltIn,
    App,
    Project,
}

/// All known profiles.
#[derive(Debug, Clone, Default)]
pub struct ProfileStore {
    profiles: BTreeMap<String, (Origin, ProfileFile)>,
}

impl ProfileStore {
    pub fn new() -> Self {
        let mut store = Self::default();
        store.profiles.insert(
            DEFAULT_PROFILE.to_owned(),
            (Origin::BuiltIn, ProfileFile::default()),
        );
        store
    }

    /// Adds a profile; a project profile shadows an app one of the same
    /// name, an app one never shadows a project one.
    pub fn insert(&mut self, name: &str, origin: Origin, file: ProfileFile) {
        if let Some((existing, _)) = self.profiles.get(name)
            && (*existing == Origin::Project && origin == Origin::App)
        {
            return;
        }
        self.profiles.insert(name.to_owned(), (origin, file));
    }

    pub fn remove(&mut self, name: &str) -> bool {
        name != DEFAULT_PROFILE && self.profiles.remove(name).is_some()
    }

    pub fn get(&self, name: &str) -> Option<&ProfileFile> {
        self.profiles.get(name).map(|(_, file)| file)
    }

    pub fn origin(&self, name: &str) -> Option<Origin> {
        self.profiles.get(name).map(|(origin, _)| *origin)
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.profiles.keys().cloned().collect();
        if !names.iter().any(|n| n == PROJECT_DEFAULT_PROFILE) {
            names.push(PROJECT_DEFAULT_PROFILE.to_owned());
            names.sort();
        }
        names
    }

    /// Reads every `*.toml` of `dir`; unreadable files are reported, the
    /// rest still load.
    pub fn load_dir(&mut self, dir: &Path, origin: Origin) -> Vec<ProfileError> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut errors = Vec::new();
        let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            match std::fs::read_to_string(&path) {
                Ok(text) => match ProfileFile::from_toml(&text) {
                    Ok(file) => self.insert(name, origin, file),
                    Err(error) => errors.push(match error {
                        ProfileError::Parse(m) => {
                            ProfileError::Parse(format!("{}: {m}", path.display()))
                        }
                        other => other,
                    }),
                },
                Err(error) => errors.push(ProfileError::Io {
                    path,
                    message: error.to_string(),
                }),
            }
        }
        errors
    }

    /// Writes profile `name` to `dir/<name>.toml`.
    pub fn save(&self, name: &str, dir: &Path) -> Result<(), ProfileError> {
        let file = self
            .get(name)
            .ok_or_else(|| ProfileError::Unknown(name.to_owned()))?;
        let io = |error: std::io::Error| ProfileError::Io {
            path: dir.to_owned(),
            message: error.to_string(),
        };
        std::fs::create_dir_all(dir).map_err(io)?;
        std::fs::write(dir.join(format!("{name}.toml")), file.to_toml()?).map_err(io)
    }

    /// The profile `name` with its base chain applied.
    pub fn resolve(&self, name: &str) -> Result<EffectiveProfile, ProfileError> {
        let mut chain: Vec<&ProfileFile> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut current = name.to_owned();
        loop {
            if !seen.insert(current.clone()) {
                return Err(ProfileError::Cycle(current));
            }
            let file = match self.get(&current) {
                Some(file) => file,
                // `Project Default` without a file is the built-in default.
                None if current == PROJECT_DEFAULT_PROFILE => {
                    current = DEFAULT_PROFILE.to_owned();
                    continue;
                }
                None => return Err(ProfileError::Unknown(current)),
            };
            chain.push(file);
            let next = match &file.base {
                Some(base) => Some(base.clone()),
                None if current != DEFAULT_PROFILE => Some(DEFAULT_PROFILE.to_owned()),
                None => None,
            };
            match next {
                Some(next) => current = next,
                None => break,
            }
        }
        let mut tools: BTreeMap<String, ToolEntry> = BTreeMap::new();
        for file in chain.into_iter().rev() {
            for (id, entry) in &file.tools {
                let merged = match tools.get(id) {
                    Some(base) => entry.over(base),
                    None => entry.clone(),
                };
                tools.insert(id.clone(), merged);
            }
        }
        Ok(EffectiveProfile {
            name: name.to_owned(),
            tools,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
base = "Strict"

[tools.EmptyCatchBlock]
enabled = true
level = "error"

[[tools.TodoComment.scopes]]
scope = "tests"
enabled = false

[tools.LongLine.options]
max = 140
"#;

    #[test]
    fn profiles_round_trip_through_toml() {
        let file = ProfileFile::from_toml(SAMPLE).unwrap();
        assert_eq!(file.base.as_deref(), Some("Strict"));
        assert_eq!(
            file.tools["LongLine"].options["max"],
            toml::Value::Integer(140)
        );
        let again = ProfileFile::from_toml(&file.to_toml().unwrap()).unwrap();
        assert_eq!(file, again);
        assert!(matches!(
            ProfileFile::from_toml("tools = 3"),
            Err(ProfileError::Parse(_))
        ));
        // Empty entries are pruned when editing.
        let mut edited = file.clone();
        edited.set_enabled("EmptyCatchBlock", None);
        edited.set_level("EmptyCatchBlock", None);
        assert!(!edited.tools.contains_key("EmptyCatchBlock"));
    }

    #[test]
    fn layering_applies_base_chain_then_overrides() {
        let mut store = ProfileStore::new();
        store.insert(
            "Strict",
            Origin::App,
            ProfileFile::from_toml(
                "[tools.EmptyCatchBlock]\nlevel = \"warning\"\nenabled = false\n[tools.LongLine.options]\nmax = 100\nother = 1\n[[tools.X.scopes]]\nscope = \"tests\"\nlevel = \"information\"\n",
            )
            .unwrap(),
        );
        store.insert(
            "Mine",
            Origin::Project,
            ProfileFile::from_toml(SAMPLE).unwrap(),
        );
        let effective = store.resolve("Mine").unwrap();
        let catch = effective.entry("EmptyCatchBlock").unwrap();
        assert_eq!(catch.enabled, Some(true), "own value wins");
        assert_eq!(catch.level.as_deref(), Some("error"));
        let long = effective.entry("LongLine").unwrap();
        assert_eq!(long.options["max"], toml::Value::Integer(140));
        assert_eq!(
            long.options["other"],
            toml::Value::Integer(1),
            "inherited option"
        );
        assert_eq!(effective.entry("X").unwrap().scopes.len(), 1);
        // Project Default without a file is the built-in default.
        assert_eq!(
            store.resolve(PROJECT_DEFAULT_PROFILE).unwrap().tools.len(),
            0
        );
        assert_eq!(
            store.resolve("Nope"),
            Err(ProfileError::Unknown("Nope".into()))
        );
    }

    #[test]
    fn cycles_and_shadowing_are_handled() {
        let mut store = ProfileStore::new();
        store.insert(
            "A",
            Origin::App,
            ProfileFile {
                base: Some("B".into()),
                ..Default::default()
            },
        );
        store.insert(
            "B",
            Origin::App,
            ProfileFile {
                base: Some("A".into()),
                ..Default::default()
            },
        );
        assert!(matches!(store.resolve("A"), Err(ProfileError::Cycle(_))));
        store.insert(
            "P",
            Origin::Project,
            ProfileFile::from_toml("[tools.A]\nenabled = true").unwrap(),
        );
        store.insert("P", Origin::App, ProfileFile::default());
        assert_eq!(
            store.origin("P"),
            Some(Origin::Project),
            "app never shadows project"
        );
        assert!(!store.remove(DEFAULT_PROFILE));
        assert!(store.names().contains(&PROJECT_DEFAULT_PROFILE.to_owned()));
    }

    #[test]
    fn scopes_match_by_path() {
        assert!(scope_matches(
            "tests",
            Path::new("src/test/java/FooTest.java")
        ));
        assert!(scope_matches("tests", Path::new("pkg/foo_test.go")));
        assert!(!scope_matches("tests", Path::new("src/main/java/Foo.java")));
        assert!(scope_matches(
            "production",
            Path::new("src/main/java/Foo.java")
        ));
        assert!(!scope_matches(
            "production",
            Path::new("build/gen/Foo.java")
        ));
        assert!(scope_matches("generated", Path::new("build/gen/Foo.java")));
        assert!(scope_matches("*.json", Path::new("a/b/c.json")));
        assert!(scope_matches("src/**/*.rs", Path::new("src/a/b/c.rs")));
        assert!(!scope_matches("src/*.rs", Path::new("src/a/c.rs")));
        assert!(scope_matches("all", Path::new("x")));
    }

    #[test]
    fn directories_load_and_save() {
        let dir = std::env::temp_dir().join(format!("archipel-profiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = ProfileStore::new();
        store.insert(
            "Mine",
            Origin::Project,
            ProfileFile::from_toml(SAMPLE).unwrap(),
        );
        store.save("Mine", &dir).unwrap();
        std::fs::write(dir.join("Broken.toml"), "tools = [").unwrap();
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        let mut loaded = ProfileStore::new();
        let errors = loaded.load_dir(&dir, Origin::Project);
        assert_eq!(errors.len(), 1);
        assert_eq!(loaded.get("Mine"), store.get("Mine"));
        assert!(store.save("Nope", &dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

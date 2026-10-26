//! Bazel integration: workspace detection, target model from `bazel query`, and
//! per-target build/run/test configurations.
//!
//! Targets are listed with `bazel query 'kind(rule, //...)' --output=label_kind`,
//! run with the user's `bazelisk`/`bazel` from PATH. The result is cached in
//! `.archipel/bazel/targets.json` so reopening a project shows the last sync.

use archipel_ide_runner::build_system::Bazel;
use archipel_ide_runner::{RunConfiguration, RunTarget, RunnerError, TaskKind, TaskPlan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const STATE_DIR: &str = ".archipel/bazel";
pub const TARGETS_FILE: &str = "targets.json";
pub const QUERY: &str = "kind(rule, //...)";

/// Statically detected Bazel workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BazelWorkspace {
    pub root: PathBuf,
    /// `MODULE.bazel` (Bzlmod) rather than a legacy `WORKSPACE` file.
    pub bzlmod: bool,
    /// Version pinned in `.bazelversion` (honoured by bazelisk).
    pub version: Option<String>,
}

impl BazelWorkspace {
    pub fn detect(root: &Path) -> Option<Self> {
        if !Bazel::is_workspace(root) {
            return None;
        }
        Some(Self {
            root: root.to_owned(),
            bzlmod: root.join("MODULE.bazel").is_file(),
            version: std::fs::read_to_string(root.join(".bazelversion"))
                .ok()
                .map(|text| text.trim().to_owned())
                .filter(|version| !version.is_empty()),
        })
    }

    /// Plan that lists all rule targets (the Bazel "sync").
    pub fn query_plan(&self) -> Result<TaskPlan, RunnerError> {
        Ok(TaskPlan::new(
            "Bazel Sync",
            Bazel::program()?,
            ["query", QUERY, "--output=label_kind", "--keep_going"],
            &self.root,
        )
        .with_kind(TaskKind::Sync))
    }
}

/// What a rule can do, derived from its kind (`java_binary` → Run, `*_test` → Test).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetRole {
    Binary,
    Test,
    Library,
    Other,
}

impl TargetRole {
    pub fn of(kind: &str) -> Self {
        if kind.ends_with("_test") || kind == "test_suite" {
            TargetRole::Test
        } else if kind.ends_with("_binary") || kind.ends_with("_image") {
            TargetRole::Binary
        } else if kind.ends_with("_library") || kind.ends_with("_import") {
            TargetRole::Library
        } else {
            TargetRole::Other
        }
    }

    /// Bazel command a double-click performs.
    pub fn default_command(self) -> &'static str {
        match self {
            TargetRole::Binary => "run",
            TargetRole::Test => "test",
            TargetRole::Library | TargetRole::Other => "build",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BazelTarget {
    /// Canonical label, `//pkg/path:name`.
    pub label: String,
    /// Rule kind, e.g. `java_binary`.
    pub kind: String,
}

impl BazelTarget {
    pub fn role(&self) -> TargetRole {
        TargetRole::of(&self.kind)
    }

    /// `//lib:greeter` → `lib`; `//:app` → `` (root package).
    pub fn package(&self) -> &str {
        let body = self.label.trim_start_matches('@').trim_start_matches("//");
        body.split_once(':').map_or(body, |(package, _)| package)
    }

    pub fn name(&self) -> &str {
        self.label
            .rsplit_once(':')
            .map_or(&self.label, |(_, name)| name)
    }

    /// Temporary run configuration for `command` (`build`, `run` or `test`).
    pub fn configuration(&self, command: &str) -> RunConfiguration {
        RunConfiguration::new(
            format!("{command} {}", self.label),
            RunTarget::Bazel {
                command: command.to_owned(),
                target: self.label.clone(),
                options: Vec::new(),
                program_arguments: Vec::new(),
            },
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BazelTargets {
    pub targets: Vec<BazelTarget>,
}

impl BazelTargets {
    /// Parses `bazel query --output=label_kind` lines (`java_binary rule //:app`).
    /// Generated helper rules (names containing `_deployjars_internal_rule` etc.) and
    /// lines that are not results are ignored.
    pub fn parse_label_kind<'a>(lines: impl IntoIterator<Item = &'a str>) -> Self {
        let mut targets: Vec<BazelTarget> = lines
            .into_iter()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let kind = parts.next()?;
                if parts.next()? != "rule" {
                    return None;
                }
                let label = parts.next()?;
                if parts.next().is_some() || !(label.starts_with("//") || label.starts_with('@')) {
                    return None;
                }
                Some(BazelTarget {
                    label: label.to_owned(),
                    kind: kind.to_owned(),
                })
            })
            .filter(|target| !target.name().ends_with("_internal_rule"))
            .collect();
        targets.sort_by(|a, b| (a.package(), a.name()).cmp(&(b.package(), b.name())));
        targets.dedup();
        Self { targets }
    }

    /// Targets grouped by package (`//` for the root package), packages sorted.
    pub fn by_package(&self) -> BTreeMap<String, Vec<&BazelTarget>> {
        let mut packages: BTreeMap<String, Vec<&BazelTarget>> = BTreeMap::new();
        for target in &self.targets {
            packages
                .entry(format!("//{}", target.package()))
                .or_default()
                .push(target);
        }
        packages
    }

    pub fn get(&self, label: &str) -> Option<&BazelTarget> {
        self.targets.iter().find(|target| target.label == label)
    }

    /// Targets declared in the BUILD file `build_file` (relative to the workspace root).
    pub fn in_build_file(&self, build_file: &Path) -> Vec<&BazelTarget> {
        let package = build_file
            .parent()
            .map(|dir| dir.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        self.targets
            .iter()
            .filter(|target| target.package() == package)
            .collect()
    }

    pub fn save(&self, root: &Path) -> Result<(), String> {
        // The first save in a project from before the rename copies `.jetthing/` to `.archipel/`.
        let dir = archipel_ide_compat::project_write_path(root.join(STATE_DIR));
        std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|error| error.to_string())?;
        std::fs::write(dir.join(TARGETS_FILE), json).map_err(|error| error.to_string())
    }

    pub fn load_cached(root: &Path) -> Option<Result<Self, String>> {
        let text = std::fs::read_to_string(archipel_ide_compat::project_read_path(
            root.join(STATE_DIR).join(TARGETS_FILE),
        ))
        .ok()?;
        Some(
            serde_json::from_str(&text)
                .map_err(|error| format!("invalid Bazel target cache: {error}")),
        )
    }
}

/// Lines where rules start, with the rule's `name`, for gutter icons in BUILD files.
pub fn rule_lines(build_file_text: &str) -> Vec<(usize, String)> {
    let mut rules = Vec::new();
    let mut rule_start: Option<usize> = None;
    for (index, line) in build_file_text.lines().enumerate() {
        let code = line.split('#').next().unwrap_or_default();
        if !line.starts_with([' ', '\t', ')']) && code.contains('(') {
            rule_start = Some(index);
        }
        if let Some(name) = name_attribute(code)
            && let Some(start) = rule_start.take()
        {
            rules.push((start, name));
        }
    }
    rules
}

/// Value of a `name = "..."` attribute on a line, if present.
fn name_attribute(code: &str) -> Option<String> {
    let mut search = code;
    while let Some(position) = search.find("name") {
        let before = search[..position].chars().next_back();
        let rest = &search[position + 4..];
        let boundary = before.is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        if boundary && let Some(value) = rest.trim_start().strip_prefix('=') {
            let value = value.trim_start();
            let quote = value.chars().next().filter(|c| *c == '"' || *c == '\'')?;
            let body = &value[1..];
            return body.find(quote).map(|end| body[..end].to_owned());
        }
        search = rest;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUERY_OUTPUT: &str = "Loading: 0 packages loaded
java_binary rule //:app
deploy_jars rule //:app_deployjars_internal_rule
sh_test rule //:app_test
sh_binary rule //:hello
java_library rule //lib:greeter
java_test rule //lib:greeter_test
INFO: Elapsed time: 0.1s";

    #[test]
    fn parses_label_kind_output() {
        let targets = BazelTargets::parse_label_kind(QUERY_OUTPUT.lines());
        let labels: Vec<_> = targets.targets.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "//:app",
                "//:app_test",
                "//:hello",
                "//lib:greeter",
                "//lib:greeter_test"
            ]
        );
        let app = targets.get("//:app").unwrap();
        assert_eq!(
            (app.package(), app.name(), app.role()),
            ("", "app", TargetRole::Binary)
        );
        assert_eq!(
            targets.get("//lib:greeter").unwrap().role(),
            TargetRole::Library
        );
        let packages = targets.by_package();
        assert_eq!(packages.keys().collect::<Vec<_>>(), ["//", "//lib"]);
        assert_eq!(packages["//lib"].len(), 2);
    }

    #[test]
    fn roles_and_configurations() {
        assert_eq!(TargetRole::of("py_test").default_command(), "test");
        assert_eq!(TargetRole::of("oci_image").default_command(), "run");
        assert_eq!(TargetRole::of("filegroup"), TargetRole::Other);
        let target = BazelTarget {
            label: "//:hello".into(),
            kind: "sh_binary".into(),
        };
        let config = target.configuration("run");
        assert_eq!(config.name, "run //:hello");
        assert!(config.validate().is_ok());
        assert_eq!(config.target.task_kind(), TaskKind::Run);
    }

    #[test]
    fn finds_rule_lines_in_build_files() {
        let build = r#"load("@rules_java//java:defs.bzl", "java_binary")

java_binary(
    name = "app",
    srcs = ["App.java"],
)

# sh_test(name = "commented")
sh_test(name = 'app_test', srcs = ["t.sh"])
"#;
        assert_eq!(
            rule_lines(build),
            [(2, "app".to_owned()), (8, "app_test".to_owned())]
        );
        let targets = BazelTargets::parse_label_kind(QUERY_OUTPUT.lines());
        assert_eq!(targets.in_build_file(Path::new("lib/BUILD.bazel")).len(), 2);
        assert_eq!(targets.in_build_file(Path::new("BUILD")).len(), 3);
    }

    #[test]
    fn detects_workspace_and_caches_targets() {
        let root = std::env::temp_dir().join(format!("archipel-ide-bazel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert!(BazelWorkspace::detect(&root).is_none());
        std::fs::write(root.join("MODULE.bazel"), "module(name = \"x\")").unwrap();
        std::fs::write(root.join(".bazelversion"), "7.4.1\n").unwrap();
        let workspace = BazelWorkspace::detect(&root).unwrap();
        assert!(workspace.bzlmod);
        assert_eq!(workspace.version.as_deref(), Some("7.4.1"));
        let targets = BazelTargets::parse_label_kind(QUERY_OUTPUT.lines());
        targets.save(&root).unwrap();
        assert_eq!(BazelTargets::load_cached(&root).unwrap().unwrap(), targets);
        std::fs::remove_dir_all(root).unwrap();
    }
}

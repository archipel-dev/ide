//! Bazel (best effort): `bazel query 'kind(<jvm rules>, //...)' --output=xml` gives the
//! rules with their `srcs`, `resources`, `deps`, `runtime_deps` and `exports`.
//!
//! * Every JVM rule with sources becomes a module named after its label (`app:app_lib`, the
//!   root package is just the rule name). The content root is the package directory.
//! * Source roots are inferred, not declared: the root of a Java source file is the file's
//!   path minus its `package` declaration (`src/main/java/com/acme/App.java` with
//!   `package com.acme;` gives `src/main/java`). Without a package line the file's directory
//!   is used. Resource roots follow `resource_strip_prefix`.
//! * `*_test` rules are test modules; their roots are test folders.
//! * `deps`/`exports` on other imported rules are module dependencies, `runtime_deps` runtime
//!   ones. External labels (`@maven//:org_junit_...`) are libraries named `Bazel: <label>`;
//!   their jars are looked up in the `rules_jvm_external` repository of the output base when it
//!   was already fetched, otherwise the library has no roots.
//!
//! Known gaps: no aspects (so no `javacopts`, annotation processor output or per-target Java
//! version), no `genrule`/generated sources, no Kotlin/Scala source set specifics, `cc_*` and
//! other languages are ignored, query needs the Bazel server and external repositories
//! (network on the first run).

use crate::data::*;
use crate::jdk::detect_jdk;
use crate::process::run_plan;
use crate::resolver::{ImportError, ProjectResolver, ResolveRequest};
use archipel_ide_bazel::BazelWorkspace;
use archipel_ide_projectmodel::{DependencyScope, ModuleType};
use archipel_ide_runner::{TaskKind, TaskPlan, build_system::Bazel};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct BazelProjectResolver;

const TIME_LIMIT: Duration = Duration::from_secs(30 * 60);
pub const JVM_QUERY: &str = "kind(\"(java|kt_jvm)_(library|binary|test|import) rule\", //...)";

impl ProjectResolver for BazelProjectResolver {
    fn system_id(&self) -> ProjectSystemId {
        ProjectSystemId::Bazel
    }

    fn is_applicable(&self, root: &Path) -> bool {
        Bazel::is_workspace(root)
    }

    fn is_build_input(&self, path: &Path) -> bool {
        path.file_name().and_then(|n| n.to_str()).is_some_and(|name| {
            matches!(
                name,
                "BUILD" | "BUILD.bazel" | "MODULE.bazel" | "WORKSPACE" | "WORKSPACE.bazel" | ".bazelrc" | ".bazelversion"
            ) || name.ends_with(".bzl")
        })
    }

    fn resolve(&self, request: &ResolveRequest) -> Result<DataNode, ImportError> {
        let workspace = BazelWorkspace::detect(&request.root)
            .ok_or_else(|| ImportError::Invalid("not a Bazel workspace".into()))?;
        let program = Bazel::program().map_err(|e| ImportError::ToolMissing(e.to_string()))?;
        request.progress("Bazel: querying JVM targets");
        let plan = TaskPlan::new(
            "Bazel Sync",
            &program,
            [
                "query",
                JVM_QUERY,
                "--output=xml",
                "--keep_going",
                "--noimplicit_deps",
            ],
            &workspace.root,
        )
        .with_kind(TaskKind::Sync);
        let captured = run_plan(&plan, &request.cancel, TIME_LIMIT)?;
        // Exit code 3 is "partial result" under --keep_going.
        if !(captured.success() || captured.code == Some(3) && !captured.stdout.is_empty()) {
            return Err(ImportError::ToolFailed {
                tool: "bazel query".into(),
                message: captured.tail(),
            });
        }
        let rules = parse_query_xml(&captured.stdout)?;
        let external = external_jars(&program, &workspace.root, request);
        Ok(to_data_nodes(&workspace.root, &rules, &external))
    }
}

/// Jars of the `rules_jvm_external` repositories in the output base, keyed by the flat
/// `group_artifact` name used in `@maven//:` labels.
fn external_jars(program: &str, root: &Path, request: &ResolveRequest) -> BTreeMap<String, PathBuf> {
    let plan = TaskPlan::new("Bazel Info", program, ["info", "output_base"], root);
    let Ok(captured) = run_plan(&plan, &request.cancel, Duration::from_secs(60)) else {
        return BTreeMap::new();
    };
    if !captured.success() {
        return BTreeMap::new();
    }
    let external = PathBuf::from(captured.stdout.trim()).join("external");
    let mut jars = BTreeMap::new();
    let Ok(repos) = std::fs::read_dir(&external) else {
        return jars;
    };
    for repo in repos.flatten() {
        let name = repo.file_name().to_string_lossy().into_owned();
        if !name.contains("maven") {
            continue;
        }
        collect_jars(&repo.path().join("v1"), &mut jars, 0);
    }
    jars
}

fn collect_jars(dir: &Path, jars: &mut BTreeMap<String, PathBuf>, depth: usize) {
    if depth > 14 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jars(&path, jars, depth + 1);
        } else if let Some(key) = jar_key(&path) {
            jars.entry(key).or_insert(path);
        }
    }
}

/// `.../org/springframework/boot/spring-boot-starter/3.5.0/spring-boot-starter-3.5.0.jar`
/// gives `org_springframework_boot_spring_boot_starter`.
pub fn jar_key(jar: &Path) -> Option<String> {
    let name = jar.file_name()?.to_str()?;
    let stem = name.strip_suffix(".jar")?;
    if stem.ends_with("-sources") || stem.ends_with("-javadoc") {
        return None;
    }
    let version_dir = jar.parent()?;
    let artifact_dir = version_dir.parent()?;
    let version = version_dir.file_name()?.to_str()?;
    let artifact = artifact_dir.file_name()?.to_str()?;
    if !stem.starts_with(&format!("{artifact}-{version}")) {
        return None;
    }
    // Everything after `maven2` (or the host directory) is the group path.
    let text = artifact_dir.parent()?.to_string_lossy().into_owned();
    let group = text.split("/maven2/").nth(1)?.replace('/', ".");
    Some(flat_name(&format!("{group}_{artifact}")))
}

fn flat_name(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

// ---------------------------------------------------------------------------------------
// Query output
// ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rule {
    pub label: String,
    pub kind: String,
    pub srcs: Vec<String>,
    pub resources: Vec<String>,
    pub deps: Vec<String>,
    pub runtime_deps: Vec<String>,
    pub exports: Vec<String>,
    pub jars: Vec<String>,
    pub resource_strip_prefix: Option<String>,
}

impl Rule {
    pub fn is_test(&self) -> bool {
        self.kind.ends_with("_test")
    }
}

/// Parses `bazel query --output=xml`.
pub fn parse_query_xml(xml: &str) -> Result<Vec<Rule>, ImportError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut rules = Vec::new();
    let mut current: Option<Rule> = None;
    let mut list: Option<String> = None;
    let attr = |e: &quick_xml::events::BytesStart<'_>, key: &str| -> Option<String> {
        e.attributes()
            .flatten()
            .find(|a| a.key.as_ref() == key.as_bytes())
            .and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()))
    };
    loop {
        match reader
            .read_event()
            .map_err(|e| ImportError::Invalid(format!("unexpected bazel query output: {e}")))?
        {
            Event::Start(e) => match e.name().as_ref() {
                b"rule" => {
                    current = Some(Rule {
                        label: attr(&e, "name").unwrap_or_default(),
                        kind: attr(&e, "class").unwrap_or_default(),
                        ..Rule::default()
                    });
                }
                b"list" if current.is_some() => list = attr(&e, "name"),
                _ => {}
            },
            Event::Empty(e) => {
                let Some(rule) = current.as_mut() else {
                    continue;
                };
                match (e.name().as_ref(), list.as_deref()) {
                    (b"label", Some(name)) => {
                        let value = attr(&e, "value").unwrap_or_default();
                        match name {
                            "srcs" => rule.srcs.push(value),
                            "resources" => rule.resources.push(value),
                            "deps" => rule.deps.push(value),
                            "runtime_deps" => rule.runtime_deps.push(value),
                            "exports" => rule.exports.push(value),
                            "jars" => rule.jars.push(value),
                            _ => {}
                        }
                    }
                    (b"string", None)
                        if attr(&e, "name").as_deref() == Some("resource_strip_prefix") =>
                    {
                        rule.resource_strip_prefix = attr(&e, "value").filter(|v| !v.is_empty());
                    }
                    _ => {}
                }
            }
            Event::End(e) => match e.name().as_ref() {
                b"list" => list = None,
                b"rule" => {
                    if let Some(rule) = current.take() {
                        rules.push(rule);
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(rules)
}

// ---------------------------------------------------------------------------------------
// Data nodes
// ---------------------------------------------------------------------------------------

/// `//a/b:c` to `(a/b, c)`; `//:c` to `("", c)`; `:c` is relative to `package`.
pub fn split_label<'a>(label: &'a str, package: &'a str) -> Option<(&'a str, &'a str)> {
    if let Some(rest) = label.strip_prefix("//") {
        return Some(rest.split_once(':').unwrap_or((rest, rest.rsplit('/').next()?)));
    }
    label.strip_prefix(':').map(|name| (package, name))
}

fn module_name(package: &str, name: &str) -> String {
    if package.is_empty() {
        name.to_owned()
    } else {
        format!("{package}:{name}")
    }
}

/// Workspace path of a source or resource file label (`//pkg:dir/File.java`).
fn file_of_label(label: &str, package: &str) -> Option<String> {
    let (pkg, name) = split_label(label, package)?;
    Some(if pkg.is_empty() {
        name.to_owned()
    } else {
        format!("{pkg}/{name}")
    })
}

/// The directory that is the source root of `file`, from its `package` declaration.
pub fn infer_source_root(workspace: &Path, file: &str) -> PathBuf {
    let path = workspace.join(file);
    let dir = path.parent().unwrap_or(workspace).to_owned();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return dir;
    };
    let package = text.lines().find_map(|line| {
        let line = line.trim();
        line.strip_prefix("package ")
            .map(|rest| rest.trim_end_matches(';').trim().trim_matches('`').to_owned())
    });
    let Some(package) = package.filter(|p| !p.is_empty()) else {
        return dir;
    };
    let segments = package.split('.').count();
    let mut root = dir.clone();
    for segment in package.split('.').rev() {
        if root.file_name().is_some_and(|n| n == segment) {
            root.pop();
        } else {
            // The directory layout does not match the package: keep the file's directory.
            let _ = segments;
            return dir;
        }
    }
    root
}

pub fn to_data_nodes(
    workspace: &Path,
    rules: &[Rule],
    external: &BTreeMap<String, PathBuf>,
) -> DataNode {
    let jdk = detect_jdk();
    let name = workspace
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "workspace".into());
    let mut project = DataNode::new(NodeData::Project(ProjectData {
        system: ProjectSystemId::Bazel,
        name,
        root: workspace.to_owned(),
        sdk: jdk.as_ref().map(|j| j.name.clone()),
    }));
    if let Some(jdk) = jdk {
        project.add(NodeData::Sdk(jdk));
    }
    let bazel_links: Vec<PathBuf> = std::fs::read_dir(workspace)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("bazel-"))
        .map(|e| e.path())
        .collect();

    // Rules that become modules: JVM rules with sources.
    let package_of = |label: &str| -> String {
        split_label(label, "")
            .map(|(p, _)| p.to_owned())
            .unwrap_or_default()
    };
    let importable: Vec<&Rule> = rules
        .iter()
        .filter(|r| !r.srcs.is_empty() && !r.kind.ends_with("_import"))
        .collect();
    let mut libraries: BTreeMap<String, LibraryData> = BTreeMap::new();
    let mut modules = Vec::new();
    for rule in &importable {
        let package = package_of(&rule.label);
        let Some((_, target)) = split_label(&rule.label, "") else {
            continue;
        };
        let dir = workspace.join(&package);
        let mut module = DataNode::new(NodeData::Module(ModuleData {
            id: rule.label.clone(),
            name: module_name(&package, target),
            module_type: ModuleType::Java,
            path: dir.clone(),
            language_level: None,
            compile_output: Vec::new(),
            test_compile_output: Vec::new(),
        }));
        let mut content = ContentRootData::new(&dir);
        let (source_type, resource_type) = if rule.is_test() {
            (
                ExternalSystemSourceType::Test,
                ExternalSystemSourceType::TestResource,
            )
        } else {
            (
                ExternalSystemSourceType::Source,
                ExternalSystemSourceType::Resource,
            )
        };
        let mut source_roots: Vec<PathBuf> = Vec::new();
        for src in &rule.srcs {
            let Some(file) = file_of_label(src, &package) else {
                continue;
            };
            let root = infer_source_root(workspace, &file);
            if !source_roots.contains(&root) {
                source_roots.push(root);
            }
        }
        for root in source_roots {
            content.store_path(source_type, root);
        }
        let mut resource_roots: Vec<PathBuf> = Vec::new();
        for resource in &rule.resources {
            let Some(file) = file_of_label(resource, &package) else {
                continue;
            };
            let root = match &rule.resource_strip_prefix {
                Some(prefix) => {
                    let relative = dir.join(prefix);
                    if relative.is_dir() {
                        relative
                    } else {
                        workspace.join(prefix)
                    }
                }
                None => workspace.join(&file).parent().unwrap_or(workspace).to_owned(),
            };
            if !resource_roots.contains(&root) {
                resource_roots.push(root);
            }
        }
        for root in resource_roots {
            content.store_path(resource_type, root);
        }
        if package.is_empty() {
            for link in &bazel_links {
                content.store_path(ExternalSystemSourceType::Excluded, link);
            }
            content.store_path(ExternalSystemSourceType::Excluded, workspace.join(".archipel"));
        }
        module.add(NodeData::ContentRoot(content));

        let groups = [
            (&rule.deps, DependencyScope::Compile),
            (&rule.exports, DependencyScope::Compile),
            (&rule.runtime_deps, DependencyScope::Runtime),
        ];
        for (labels, scope) in groups {
            for dep in labels {
                let absolute = if dep.starts_with(':') {
                    format!("//{package}{dep}")
                } else {
                    dep.clone()
                };
                if let Some(target) = importable.iter().find(|r| r.label == absolute) {
                    if target.label != rule.label {
                        module.add(NodeData::ModuleDependency(ModuleDependencyData {
                            target: target.label.clone(),
                            scope,
                            exported: false,
                        }));
                    }
                } else if absolute.starts_with('@') {
                    let library = format!("Bazel: {absolute}");
                    libraries.entry(library.clone()).or_insert_with(|| {
                        let mut data = LibraryData::new(&library);
                        let key = absolute
                            .rsplit_once("//:")
                            .map(|(_, n)| n.to_owned())
                            .unwrap_or_default();
                        if let Some(jar) = external.get(&key) {
                            data.binary.push(jar.clone());
                            let (sources, docs) = crate::artifacts::sibling_artifacts(jar);
                            data.source = sources;
                            data.documentation = docs;
                        }
                        data
                    });
                    module.add(NodeData::LibraryDependency(LibraryDependencyData {
                        library,
                        scope,
                        exported: false,
                    }));
                }
            }
        }
        modules.push(module);
    }
    for library in libraries.into_values() {
        project.add(NodeData::Library(library));
    }
    project.children.extend(modules);
    let skipped = rules.len() - importable.len();
    if skipped > 0 {
        project.add(NodeData::Diagnostic(DiagnosticData {
            severity: Severity::Info,
            message: format!("Bazel: {skipped} JVM targets without sources were not imported as modules"),
        }));
    }
    project
}

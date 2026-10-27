//! Cargo: `cargo metadata` gives packages, targets and the resolved dependency graph.
//!
//! Workspace members become modules (content root = package directory; `lib`/`bin` targets
//! give source folders, `test`/`bench`/`example` targets give test folders; `target` is
//! excluded). Registry and path dependencies become libraries whose source root is the
//! package directory; their transitive closure is attached to each member with the scope of
//! the edge kind (normal = compile, build = provided, dev = test). The Rust SDK is the
//! sysroot of `rustc`, with the standard library sources when `rust-src` is installed.

use crate::data::*;
use crate::process::run_plan;
use crate::resolver::{ImportError, ProjectResolver, ResolveRequest};
use archipel_ide_projectmodel::{Coordinates, DependencyScope, ModuleType, SdkType};
use archipel_ide_runner::{TaskKind, TaskPlan, find_program};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct CargoProjectResolver;

const TIME_LIMIT: Duration = Duration::from_secs(10 * 60);

impl ProjectResolver for CargoProjectResolver {
    fn system_id(&self) -> ProjectSystemId {
        ProjectSystemId::Cargo
    }

    fn is_applicable(&self, root: &Path) -> bool {
        root.join("Cargo.toml").is_file()
    }

    fn is_build_input(&self, path: &Path) -> bool {
        path.file_name()
            .is_some_and(|n| n == "Cargo.toml" || n == "Cargo.lock" || n == "build.rs")
            || path.ends_with(".cargo/config.toml")
    }

    fn resolve(&self, request: &ResolveRequest) -> Result<DataNode, ImportError> {
        let cargo = find_program("cargo")
            .ok_or_else(|| ImportError::ToolMissing("Install Rust with rustup (https://rustup.rs).".into()))?
            .display()
            .to_string();
        let attempts: &[bool] = if request.settings.offline_only {
            &[true]
        } else {
            &[true, false]
        };
        let mut metadata = None;
        let mut last = None;
        for &offline in attempts {
            request.cancel.check()?;
            request.progress(if offline {
                "Cargo: reading metadata from the local registry cache"
            } else {
                "Cargo: fetching dependency metadata"
            });
            let mut args = vec!["metadata".to_owned(), "--format-version".into(), "1".into()];
            if offline {
                args.push("--offline".into());
            }
            let plan = TaskPlan::new("Cargo Metadata", &cargo, args, &request.root)
                .with_kind(TaskKind::Sync);
            let captured = run_plan(&plan, &request.cancel, TIME_LIMIT)?;
            if captured.success() {
                metadata = Some(parse_metadata(&captured.stdout)?);
                break;
            }
            last = Some(ImportError::ToolFailed {
                tool: "cargo metadata".into(),
                message: captured.tail(),
            });
        }
        let metadata = metadata.ok_or_else(|| {
            last.unwrap_or_else(|| ImportError::Invalid("cargo metadata produced nothing".into()))
        })?;
        let sdk = rust_sdk(&cargo, request);
        Ok(to_data_nodes(&request.root, &metadata, sdk))
    }
}

fn rust_sdk(cargo: &str, request: &ResolveRequest) -> Option<SdkData> {
    let rustc = Path::new(cargo)
        .parent()
        .map(|dir| dir.join("rustc"))
        .filter(|p| p.is_file())
        .map(|p| p.display().to_string())
        .or_else(|| find_program("rustc").map(|p| p.display().to_string()))?;
    let run = |args: &[&str]| {
        let plan = TaskPlan::new("rustc", &rustc, args.iter().copied(), &request.root);
        run_plan(&plan, &request.cancel, Duration::from_secs(30))
            .ok()
            .filter(|c| c.success())
            .map(|c| c.stdout.trim().to_owned())
    };
    let sysroot = PathBuf::from(run(&["--print", "sysroot"])?);
    let version = run(&["--version"])
        .and_then(|v| v.split_whitespace().nth(1).map(str::to_owned));
    Some(SdkData {
        name: format!("Rust {}", version.as_deref().unwrap_or("toolchain")),
        kind: SdkType::Rust,
        home: sysroot.clone(),
        version,
        sources: vec![sysroot.join("lib/rustlib/src/rust/library")],
    })
}

// ---------------------------------------------------------------------------------------
// `cargo metadata` JSON
// ---------------------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub packages: Vec<Package>,
    #[serde(default)]
    pub workspace_members: Vec<String>,
    #[serde(default)]
    pub workspace_root: PathBuf,
    #[serde(default)]
    pub target_directory: PathBuf,
    #[serde(default)]
    pub resolve: Option<Resolve>,
}

#[derive(Debug, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub id: String,
    #[serde(default)]
    pub edition: Option<String>,
    pub manifest_path: PathBuf,
    #[serde(default)]
    pub targets: Vec<Target>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    #[serde(default)]
    pub kind: Vec<String>,
    pub src_path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Resolve {
    #[serde(default)]
    pub nodes: Vec<Node>,
}

#[derive(Debug, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(default)]
    pub deps: Vec<NodeDep>,
}

#[derive(Debug, Deserialize)]
pub struct NodeDep {
    pub pkg: String,
    #[serde(default)]
    pub dep_kinds: Vec<DepKind>,
}

#[derive(Debug, Deserialize)]
pub struct DepKind {
    #[serde(default)]
    pub kind: Option<String>,
}

pub fn parse_metadata(json: &str) -> Result<Metadata, ImportError> {
    serde_json::from_str(json)
        .map_err(|e| ImportError::Invalid(format!("unexpected `cargo metadata` output: {e}")))
}

// ---------------------------------------------------------------------------------------
// Data nodes
// ---------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeKind {
    Normal,
    Build,
    Dev,
}

fn edges(node: &Node, wanted: EdgeKind) -> impl Iterator<Item = &str> {
    node.deps.iter().filter_map(move |dep| {
        let kinds: Vec<EdgeKind> = if dep.dep_kinds.is_empty() {
            vec![EdgeKind::Normal]
        } else {
            dep.dep_kinds
                .iter()
                .map(|k| match k.kind.as_deref() {
                    Some("dev") => EdgeKind::Dev,
                    Some("build") => EdgeKind::Build,
                    _ => EdgeKind::Normal,
                })
                .collect()
        };
        kinds.contains(&wanted).then_some(dep.pkg.as_str())
    })
}

/// Packages reachable from `member` starting with `first`-kind edges, then normal edges.
fn closure<'a>(
    nodes: &'a HashMap<&str, &'a Node>,
    member: &str,
    first: EdgeKind,
) -> Vec<&'a str> {
    let mut out: Vec<&str> = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut queue: Vec<&str> = nodes
        .get(member)
        .map(|n| edges(n, first).collect())
        .unwrap_or_default();
    while let Some(id) = queue.pop() {
        if id == member || !seen.insert(id) {
            continue;
        }
        out.push(id);
        if let Some(node) = nodes.get(id) {
            queue.extend(edges(node, EdgeKind::Normal));
        }
    }
    out.sort_unstable();
    out
}

pub fn to_data_nodes(root: &Path, metadata: &Metadata, sdk: Option<SdkData>) -> DataNode {
    let packages: HashMap<&str, &Package> =
        metadata.packages.iter().map(|p| (p.id.as_str(), p)).collect();
    let nodes: HashMap<&str, &Node> = metadata
        .resolve
        .iter()
        .flat_map(|r| &r.nodes)
        .map(|n| (n.id.as_str(), n))
        .collect();
    let members: Vec<&Package> = metadata
        .workspace_members
        .iter()
        .filter_map(|id| packages.get(id.as_str()).copied())
        .collect();
    let workspace_root = if metadata.workspace_root.as_os_str().is_empty() {
        root.to_owned()
    } else {
        metadata.workspace_root.clone()
    };
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .or_else(|| members.first().map(|p| p.name.clone()))
        .unwrap_or_else(|| "project".into());
    let mut project = DataNode::new(NodeData::Project(ProjectData {
        system: ProjectSystemId::Cargo,
        name,
        root: root.to_owned(),
        sdk: sdk.as_ref().map(|s| s.name.clone()),
    }));
    if let Some(sdk) = sdk {
        project.add(NodeData::Sdk(sdk));
    }
    let member_ids: BTreeSet<&str> = members.iter().map(|p| p.id.as_str()).collect();
    let mut libraries: BTreeMap<String, LibraryData> = BTreeMap::new();
    let mut modules = Vec::new();
    for member in &members {
        let dir = member
            .manifest_path
            .parent()
            .map(Path::to_owned)
            .unwrap_or_else(|| root.to_owned());
        let mut module = DataNode::new(NodeData::Module(ModuleData {
            id: member.id.clone(),
            name: member.name.clone(),
            module_type: ModuleType::Rust,
            path: dir.clone(),
            language_level: member.edition.clone(),
            compile_output: Vec::new(),
            test_compile_output: Vec::new(),
        }));
        let mut content = ContentRootData::new(&dir);
        for target in &member.targets {
            use ExternalSystemSourceType as T;
            let kinds: Vec<&str> = target.kind.iter().map(String::as_str).collect();
            if kinds == ["custom-build"] {
                continue;
            }
            let is_test = kinds
                .iter()
                .all(|k| matches!(*k, "test" | "bench" | "example"));
            let folder = if is_test {
                // `tests/a.rs` -> `tests`, `examples/x/main.rs` -> `examples`.
                target
                    .src_path
                    .strip_prefix(&dir)
                    .ok()
                    .and_then(|rel| rel.components().next())
                    .map(|first| dir.join(first.as_os_str()))
                    .filter(|p| *p != target.src_path)
            } else {
                target.src_path.parent().map(Path::to_owned)
            };
            let Some(folder) = folder.filter(|f| *f != dir) else {
                continue;
            };
            // `src/bin` is already under `src`.
            let kind = if is_test { T::Test } else { T::Source };
            if !content
                .folders
                .iter()
                .any(|(k, p)| *k == kind && folder.starts_with(p))
            {
                content.folders.retain(|(k, p)| !(*k == kind && p.starts_with(&folder)));
                content.store_path(kind, folder);
            }
        }
        content.store_path(ExternalSystemSourceType::Excluded, dir.join("target"));
        if metadata.target_directory.starts_with(&dir) {
            content.store_path(
                ExternalSystemSourceType::Excluded,
                &metadata.target_directory,
            );
        }
        if dir == workspace_root {
            content.store_path(ExternalSystemSourceType::Excluded, dir.join(".archipel"));
        }
        module.add(NodeData::ContentRoot(content));

        let normal = closure(&nodes, &member.id, EdgeKind::Normal);
        let build = closure(&nodes, &member.id, EdgeKind::Build);
        let dev = closure(&nodes, &member.id, EdgeKind::Dev);
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let ordered = [
            (&normal, DependencyScope::Compile),
            (&build, DependencyScope::Provided),
            (&dev, DependencyScope::Test),
        ];
        for (ids, scope) in ordered {
            for id in ids {
                if !seen.insert(id) {
                    continue;
                }
                if member_ids.contains(id) {
                    module.add(NodeData::ModuleDependency(ModuleDependencyData {
                        target: (*id).to_owned(),
                        scope,
                        exported: false,
                    }));
                    continue;
                }
                let Some(package) = packages.get(id) else {
                    continue;
                };
                let library = format!("Cargo: {}:{}", package.name, package.version);
                libraries.entry(library.clone()).or_insert_with(|| {
                    let mut data = LibraryData::new(&library);
                    if let Some(dir) = package.manifest_path.parent() {
                        data.source.push(dir.to_owned());
                    }
                    data.coordinates =
                        Some(Coordinates::new("crates", &package.name, &package.version));
                    data
                });
                module.add(NodeData::LibraryDependency(LibraryDependencyData {
                    library,
                    scope,
                    exported: false,
                }));
            }
        }
        modules.push(module);
    }
    for library in libraries.into_values() {
        project.add(NodeData::Library(library));
    }
    project.children.extend(modules);
    project
}

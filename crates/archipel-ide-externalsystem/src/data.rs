//! `DataNode` trees: what a [`crate::ProjectResolver`] reports about a build.
//!
//! The shape follows IntelliJ's external system data model (`ProjectKeys`): a `PROJECT` node
//! has `MODULE`, `LIBRARY` and `SDK` children; a `MODULE` has `CONTENT_ROOT`,
//! `MODULE_DEPENDENCY` and `LIBRARY_DEPENDENCY` children. Resolvers only describe; the
//! [`crate::service`] data services apply the nodes to the project model.

use archipel_ide_projectmodel::{Coordinates, DependencyScope, ModuleType};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// `ProjectSystemId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectSystemId {
    Gradle,
    Maven,
    Cargo,
    Bazel,
}

impl ProjectSystemId {
    pub fn id(self) -> &'static str {
        match self {
            Self::Gradle => "gradle",
            Self::Maven => "maven",
            Self::Cargo => "cargo",
            Self::Bazel => "bazel",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Gradle => "Gradle",
            Self::Maven => "Maven",
            Self::Cargo => "Cargo",
            Self::Bazel => "Bazel",
        }
    }
}

/// The keys of the node kinds (`ProjectKeys`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DataKey {
    Project,
    Module,
    ContentRoot,
    Library,
    LibraryDependency,
    ModuleDependency,
    Sdk,
    Diagnostic,
}

/// `ExternalSystemSourceType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExternalSystemSourceType {
    Source,
    Test,
    Resource,
    TestResource,
    SourceGenerated,
    TestGenerated,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectData {
    pub system: ProjectSystemId,
    pub name: String,
    /// The directory that holds the build's root file.
    pub root: PathBuf,
    /// Name of the SDK node used by default (`ProjectRootManager.setProjectSdk`).
    #[serde(default)]
    pub sdk: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleData {
    /// Id inside the external build (`:app`, `group:artifact`, a Bazel label, a package id).
    pub id: String,
    /// Name of the module in the IDE.
    pub name: String,
    pub module_type: ModuleType,
    /// Directory of the module's build file.
    pub path: PathBuf,
    #[serde(default)]
    pub language_level: Option<String>,
    #[serde(default)]
    pub compile_output: Vec<PathBuf>,
    #[serde(default)]
    pub test_compile_output: Vec<PathBuf>,
}

/// `ContentRootData`: one content root with its typed folders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentRootData {
    pub root: PathBuf,
    #[serde(default)]
    pub folders: Vec<(ExternalSystemSourceType, PathBuf)>,
}

impl ContentRootData {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            folders: Vec::new(),
        }
    }

    /// `ContentRootData.storePath`.
    pub fn store_path(&mut self, kind: ExternalSystemSourceType, path: impl Into<PathBuf>) {
        let path = path.into();
        if !self.folders.iter().any(|(k, p)| *k == kind && *p == path) {
            self.folders.push((kind, path));
        }
    }

    pub fn paths(&self, kind: ExternalSystemSourceType) -> impl Iterator<Item = &PathBuf> {
        self.folders
            .iter()
            .filter(move |(k, _)| *k == kind)
            .map(|(_, p)| p)
    }
}

/// `LibraryData`: a library of the project library table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryData {
    pub name: String,
    #[serde(default)]
    pub binary: Vec<PathBuf>,
    #[serde(default)]
    pub source: Vec<PathBuf>,
    #[serde(default)]
    pub documentation: Vec<PathBuf>,
    #[serde(default)]
    pub coordinates: Option<Coordinates>,
}

impl LibraryData {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            binary: Vec::new(),
            source: Vec::new(),
            documentation: Vec::new(),
            coordinates: None,
        }
    }
}

/// `LibraryDependencyData`: the owner module is the parent node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDependencyData {
    /// Name of the [`LibraryData`] node.
    pub library: String,
    pub scope: DependencyScope,
    #[serde(default)]
    pub exported: bool,
}

/// `ModuleDependencyData`: the owner module is the parent node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleDependencyData {
    /// `ModuleData.id` of the target, in the same system.
    pub target: String,
    pub scope: DependencyScope,
    #[serde(default)]
    pub exported: bool,
}

/// An SDK the build uses (`JavaSdkData`, a Rust toolchain).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkData {
    pub name: String,
    pub kind: archipel_ide_projectmodel::SdkType,
    pub home: PathBuf,
    #[serde(default)]
    pub version: Option<String>,
    /// Source roots to attach (JDK `src.zip`, Rust `library/` sources) when they exist.
    #[serde(default)]
    pub sources: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// Something the resolver wants the user to know (unresolved dependency, gap in the import).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticData {
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "key", content = "data", rename_all = "camelCase")]
pub enum NodeData {
    Project(ProjectData),
    Module(ModuleData),
    ContentRoot(ContentRootData),
    Library(LibraryData),
    LibraryDependency(LibraryDependencyData),
    ModuleDependency(ModuleDependencyData),
    Sdk(SdkData),
    Diagnostic(DiagnosticData),
}

impl NodeData {
    pub fn key(&self) -> DataKey {
        match self {
            Self::Project(_) => DataKey::Project,
            Self::Module(_) => DataKey::Module,
            Self::ContentRoot(_) => DataKey::ContentRoot,
            Self::Library(_) => DataKey::Library,
            Self::LibraryDependency(_) => DataKey::LibraryDependency,
            Self::ModuleDependency(_) => DataKey::ModuleDependency,
            Self::Sdk(_) => DataKey::Sdk,
            Self::Diagnostic(_) => DataKey::Diagnostic,
        }
    }
}

/// `DataNode<T>`: typed data with children.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataNode {
    #[serde(flatten)]
    pub data: NodeData,
    #[serde(default)]
    pub children: Vec<DataNode>,
}

impl DataNode {
    pub fn new(data: NodeData) -> Self {
        Self {
            data,
            children: Vec::new(),
        }
    }

    pub fn key(&self) -> DataKey {
        self.data.key()
    }

    /// `DataNode.createChild`.
    pub fn add_child(&mut self, child: DataNode) -> &mut DataNode {
        self.children.push(child);
        self.children.last_mut().expect("just pushed")
    }

    pub fn add(&mut self, data: NodeData) -> &mut DataNode {
        self.add_child(DataNode::new(data))
    }

    /// Direct children with `key` (`ExternalSystemApiUtil.findAll`).
    pub fn find_all(&self, key: DataKey) -> impl Iterator<Item = &DataNode> {
        self.children.iter().filter(move |c| c.key() == key)
    }

    /// Visits this node and all descendants with the parent of each.
    pub fn walk<'a>(&'a self, visit: &mut impl FnMut(&'a DataNode, Option<&'a DataNode>)) {
        self.walk_inner(None, visit);
    }

    fn walk_inner<'a>(
        &'a self,
        parent: Option<&'a DataNode>,
        visit: &mut impl FnMut(&'a DataNode, Option<&'a DataNode>),
    ) {
        visit(self, parent);
        for child in &self.children {
            child.walk_inner(Some(self), visit);
        }
    }

    pub fn module(&self) -> Option<&ModuleData> {
        match &self.data {
            NodeData::Module(module) => Some(module),
            _ => None,
        }
    }

    /// Number of nodes in the tree.
    pub fn size(&self) -> usize {
        1 + self.children.iter().map(DataNode::size).sum::<usize>()
    }
}

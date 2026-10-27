//! Content entries and source folders (`ContentEntry`, `SourceFolder`).

use crate::path::normalize;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The kind of a source folder, after the JPS module source root types
/// (`JavaSourceRootType.SOURCE`/`TEST_SOURCE`, `JavaResourceRootType.RESOURCE`/`TEST_RESOURCE`)
/// and the "generated" property of Java source root properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JpsModuleSourceRootType {
    Source,
    TestSource,
    Resource,
    TestResource,
    /// Generated production sources (annotation processor output, protobuf, ...).
    Generated,
    /// Generated test sources.
    GeneratedTest,
}

impl JpsModuleSourceRootType {
    pub const ALL: [Self; 6] = [
        Self::Source,
        Self::TestSource,
        Self::Resource,
        Self::TestResource,
        Self::Generated,
        Self::GeneratedTest,
    ];

    /// `SourceFolder.isTestSource()`.
    pub fn is_for_tests(self) -> bool {
        matches!(
            self,
            Self::TestSource | Self::TestResource | Self::GeneratedTest
        )
    }

    pub fn is_resource(self) -> bool {
        matches!(self, Self::Resource | Self::TestResource)
    }

    pub fn is_generated(self) -> bool {
        matches!(self, Self::Generated | Self::GeneratedTest)
    }

    /// Stable id used in JSON (control socket, caches).
    pub fn id(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::TestSource => "testSource",
            Self::Resource => "resource",
            Self::TestResource => "testResource",
            Self::Generated => "generated",
            Self::GeneratedTest => "generatedTest",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceFolder {
    pub path: PathBuf,
    pub root_type: JpsModuleSourceRootType,
    /// Package of the folder itself (`SourceFolder.getPackagePrefix`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub package_prefix: String,
}

impl SourceFolder {
    pub fn new(path: impl AsRef<Path>, root_type: JpsModuleSourceRootType) -> Self {
        Self {
            path: normalize(path.as_ref()),
            root_type,
            package_prefix: String::new(),
        }
    }

    pub fn with_package_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.package_prefix = prefix.into();
        self
    }

    pub fn is_test_source(&self) -> bool {
        self.root_type.is_for_tests()
    }
}

/// A content root of a module with its source and excluded folders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentEntry {
    pub root: PathBuf,
    #[serde(default)]
    pub source_folders: Vec<SourceFolder>,
    #[serde(default)]
    pub excluded_folders: Vec<PathBuf>,
}

impl ContentEntry {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: normalize(root.as_ref()),
            source_folders: Vec::new(),
            excluded_folders: Vec::new(),
        }
    }

    /// `ContentEntry.addSourceFolder`; a path that is already a source folder is replaced.
    pub fn add_source_folder(&mut self, folder: SourceFolder) -> &mut Self {
        self.source_folders.retain(|f| f.path != folder.path);
        self.source_folders.push(folder);
        self
    }

    pub fn add_excluded_folder(&mut self, path: impl AsRef<Path>) -> &mut Self {
        let path = normalize(path.as_ref());
        if !self.excluded_folders.contains(&path) {
            self.excluded_folders.push(path);
        }
        self
    }

    pub fn contains(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }
}

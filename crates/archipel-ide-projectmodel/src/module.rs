//! `Module` and what hangs off it.

use crate::order::OrderEntry;
use crate::roots::{ContentEntry, SourceFolder};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// `ModuleType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModuleType {
    #[default]
    Java,
    Rust,
    Generic,
}

/// Where a module came from (`ExternalSystemModulePropertyManager`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalProject {
    /// `gradle`, `maven`, `cargo`, `bazel`.
    pub system: String,
    /// The module's id inside the external build (`:app`, `groupId:artifactId`, a label).
    pub id: String,
    /// The module's directory (where its build file lives).
    pub path: PathBuf,
}

/// `CompilerModuleExtension`: where the build writes class files.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerOutput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Module {
    pub name: String,
    #[serde(default)]
    pub module_type: ModuleType,
    #[serde(default)]
    pub content_entries: Vec<ContentEntry>,
    /// The `ModuleRootManager` dependency list, in classpath order.
    #[serde(default)]
    pub order_entries: Vec<OrderEntry>,
    #[serde(default)]
    pub compiler_output: CompilerOutput,
    /// Java language level, e.g. `17` (`LanguageLevelModuleExtension`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_project: Option<ExternalProject>,
}

impl Module {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            module_type: ModuleType::Java,
            content_entries: Vec::new(),
            order_entries: vec![OrderEntry::InheritedSdk, OrderEntry::ModuleSource],
            compiler_output: CompilerOutput::default(),
            language_level: None,
            external_project: None,
        }
    }

    /// `ModuleRootManager.getContentRoots`.
    pub fn content_roots(&self) -> impl Iterator<Item = &PathBuf> {
        self.content_entries.iter().map(|entry| &entry.root)
    }

    /// `ModuleRootManager.getSourceRoots(includingTests)`-style listing of every source folder.
    pub fn source_folders(&self) -> impl Iterator<Item = &SourceFolder> {
        self.content_entries
            .iter()
            .flat_map(|entry| &entry.source_folders)
    }

    pub fn excluded_folders(&self) -> impl Iterator<Item = &PathBuf> {
        self.content_entries
            .iter()
            .flat_map(|entry| &entry.excluded_folders)
    }

    /// The content entry for `root`, created when missing (`ModifiableRootModel.addContentEntry`).
    pub fn content_entry_mut(&mut self, root: impl AsRef<std::path::Path>) -> &mut ContentEntry {
        let root = crate::path::normalize(root.as_ref());
        let index = match self.content_entries.iter().position(|e| e.root == root) {
            Some(index) => index,
            None => {
                self.content_entries.push(ContentEntry::new(root));
                self.content_entries.len() - 1
            }
        };
        &mut self.content_entries[index]
    }

    /// Appends an order entry unless an equal one exists. Returns whether it was added.
    pub fn add_order_entry(&mut self, entry: OrderEntry) -> bool {
        if self.order_entries.contains(&entry) {
            return false;
        }
        self.order_entries.push(entry);
        true
    }
}

//! `ProjectModel`: the module table, the project library table and the SDK table.

use crate::file_index::ProjectFileIndex;
use crate::module::Module;
use crate::order::{ClasspathKind, OrderEntry};
use crate::sdk::{Library, Sdk};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectModel {
    pub name: String,
    pub root: PathBuf,
    /// Name of the project SDK (`ProjectRootManager.getProjectSdk`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_sdk: Option<String>,
    #[serde(default)]
    pub sdks: Vec<Sdk>,
    /// The project library table (`LibraryTablesRegistrar.getLibraryTable(project)`).
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub modules: Vec<Module>,
}

impl ProjectModel {
    pub fn new(name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            root: root.into(),
            ..Self::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// `ModuleManager.findModuleByName`.
    pub fn module(&self, name: &str) -> Option<&Module> {
        self.modules.iter().find(|module| module.name == name)
    }

    pub fn module_mut(&mut self, name: &str) -> Option<&mut Module> {
        self.modules.iter_mut().find(|module| module.name == name)
    }

    pub fn library(&self, name: &str) -> Option<&Library> {
        self.libraries.iter().find(|library| library.name == name)
    }

    pub fn sdk(&self, name: &str) -> Option<&Sdk> {
        self.sdks.iter().find(|sdk| sdk.name == name)
    }

    /// `ModuleRootManager.getSdk`: the module's own SDK or, for an inherited entry, the project SDK.
    pub fn module_sdk(&self, module: &Module) -> Option<&Sdk> {
        module.order_entries.iter().find_map(|entry| match entry {
            OrderEntry::ModuleSdk { sdk } => self.sdk(sdk),
            OrderEntry::InheritedSdk => self.project_sdk.as_deref().and_then(|name| self.sdk(name)),
            _ => None,
        })
    }

    /// Modules a module depends on directly, in order, for scopes that are on `kind`.
    pub fn module_dependencies<'a>(
        &'a self,
        module: &'a Module,
        kind: ClasspathKind,
    ) -> impl Iterator<Item = &'a Module> + 'a {
        module
            .order_entries
            .iter()
            .filter_map(move |entry| match entry {
                OrderEntry::Module { module, scope, .. } if scope.is_included_in(kind) => {
                    self.module(module)
                }
                _ => None,
            })
    }

    /// The module and everything it depends on (`OrderEnumerator.recursively()`), the module
    /// first, then dependencies in classpath order. Compile classpaths only follow exported
    /// entries of dependencies; runtime classpaths follow everything.
    pub fn modules_with_dependencies<'a>(
        &'a self,
        module: &'a Module,
        kind: ClasspathKind,
    ) -> Vec<&'a Module> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        self.collect_modules(module, kind, true, &mut seen, &mut out);
        out
    }

    fn collect_modules<'a>(
        &'a self,
        module: &'a Module,
        kind: ClasspathKind,
        is_root: bool,
        seen: &mut HashSet<&'a str>,
        out: &mut Vec<&'a Module>,
    ) {
        if !seen.insert(&module.name) {
            return;
        }
        out.push(module);
        let next = if is_root { kind } else { kind.production() };
        for entry in &module.order_entries {
            let OrderEntry::Module {
                module: name,
                scope,
                ..
            } = entry
            else {
                continue;
            };
            if !scope.is_included_in(next) || !(is_root || kind.is_runtime() || entry.is_exported())
            {
                continue;
            }
            if let Some(dependency) = self.module(name) {
                self.collect_modules(dependency, kind, false, seen, out);
            }
        }
    }

    /// Libraries on the given classpath of `module`, in order, without duplicates.
    pub fn libraries_for<'a>(
        &'a self,
        module: &'a Module,
        kind: ClasspathKind,
    ) -> Vec<&'a Library> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for (index, owner) in self
            .modules_with_dependencies(module, kind)
            .into_iter()
            .enumerate()
        {
            let owner_kind = if index == 0 { kind } else { kind.production() };
            for entry in &owner.order_entries {
                if let OrderEntry::Library {
                    library,
                    scope,
                    exported,
                } = entry
                    && scope.is_included_in(owner_kind)
                    && (index == 0 || kind.is_runtime() || *exported)
                    && let Some(library) = self.library(library)
                    && seen.insert(library.name.as_str())
                {
                    out.push(library);
                }
            }
        }
        out
    }

    /// `OrderEnumerator.classesRoots`: the compiler output of the module (and of the modules it
    /// depends on) followed by library class roots, for one classpath.
    pub fn classpath(&self, module: &Module, kind: ClasspathKind) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        let mut push = |path: &PathBuf| {
            if !out.contains(path) {
                out.push(path.clone());
            }
        };
        for (index, owner) in self
            .modules_with_dependencies(module, kind)
            .into_iter()
            .enumerate()
        {
            if index == 0 && kind.is_test() {
                owner.compiler_output.test.iter().for_each(&mut push);
            }
            owner.compiler_output.production.iter().for_each(&mut push);
        }
        for library in self.libraries_for(module, kind) {
            library.classes.iter().for_each(&mut push);
        }
        out
    }

    /// Builds the `ProjectFileIndex` for this model.
    pub fn file_index(self: &Arc<Self>) -> ProjectFileIndex {
        ProjectFileIndex::new(Arc::clone(self))
    }

    pub fn library_count(&self) -> usize {
        self.libraries.len()
    }

    /// Replaces a library with the same name or appends it.
    pub fn put_library(&mut self, library: Library) {
        match self.libraries.iter_mut().find(|l| l.name == library.name) {
            Some(existing) => *existing = library,
            None => self.libraries.push(library),
        }
    }

    pub fn put_sdk(&mut self, sdk: Sdk) {
        match self.sdks.iter_mut().find(|s| s.name == sdk.name) {
            Some(existing) => *existing = sdk,
            None => self.sdks.push(sdk),
        }
    }
}

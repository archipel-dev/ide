//! The resolve scope of a file (spec 02 section 2.7, PLAT-SCOPE-054/062/067): the file's
//! module with its dependencies, libraries and SDK, tests included only for test sources.
//! A scope is a value: unit id to classpath rank.

use crate::classpath::{ClasspathSnapshot, ClasspathUnit, Completeness, UnitRole};
use crate::index::{JavaIndex, UnitKind};
use archipel_ide_projectmodel::{Module, ProjectFileIndex, ProjectModel};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolveScope {
    ranks: HashMap<u32, u32>,
    /// The module the scope was computed for.
    pub module: Option<String>,
    /// The file is test code (test sources and test-scope libraries are included).
    pub test: bool,
    /// Why the scope is what it is (diagnostics).
    pub kind: ScopeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScopeKind {
    /// Module, dependencies, libraries, SDK.
    #[default]
    Module,
    /// A file under content but outside every source root: only itself (JAVA-RES-030).
    FileOnly,
    /// A file outside every module (scratch file, folder without a build file): the ad-hoc
    /// module with the SDK and the file itself.
    AdHoc,
}

impl ResolveScope {
    pub fn contains(&self, unit: u32) -> bool {
        self.ranks.contains_key(&unit)
    }

    /// Classpath position of a unit (lower is better).
    pub fn rank(&self, unit: u32) -> Option<u32> {
        self.ranks.get(&unit).copied()
    }

    /// `(unit, classpath rank)` of every unit in the scope.
    pub fn iter(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.ranks.iter().map(|(u, r)| (*u, *r))
    }

    pub fn len(&self) -> usize {
        self.ranks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ranks.is_empty()
    }

    /// A scope holding exactly these units (tests, tools).
    pub fn of_units(units: impl IntoIterator<Item = u32>) -> Self {
        let mut s = Self::default();
        for (i, u) in units.into_iter().enumerate() {
            s.ranks.entry(u).or_insert(i as u32);
        }
        s
    }

    /// The resolve scope of the Java file at `path` (JAVA-RES-029, 030, 033; PLAT-SCOPE-062).
    pub fn for_file(
        index: &JavaIndex,
        model: &ProjectModel,
        files: &ProjectFileIndex,
        path: &Path,
    ) -> Self {
        Self::snapshot_for_file(index, model, files, path, |_, _| Completeness::Complete).scope()
    }

    /// The classpath snapshot behind [`Self::for_file`]; `completeness` says how complete the
    /// classpath of the file's module is.
    pub fn snapshot_for_file(
        index: &JavaIndex,
        model: &ProjectModel,
        files: &ProjectFileIndex,
        path: &Path,
        completeness: impl FnOnce(&ProjectModel, &Module) -> Completeness,
    ) -> ClasspathSnapshot {
        let Some(module) = files.module_for_file(path) else {
            // Outside every module: the ad-hoc module (JDK and the file itself).
            return ClasspathSnapshot::ad_hoc(index, model, path);
        };
        if !files.is_in_source_content(path) {
            // Under content but outside every source root: only itself (JAVA-RES-030).
            let cat = index.inner.catalog.read();
            let mut units = Vec::new();
            if let Some(id) = cat.by_path.get(path) {
                units.push(ClasspathUnit {
                    id: *id,
                    role: UnitRole::Source {
                        module: module.name.clone(),
                        test: false,
                    },
                    kind: UnitKind::Source,
                    path: path.to_path_buf(),
                });
            }
            return ClasspathSnapshot {
                module: Some(module.name.clone()),
                test: false,
                kind: ScopeKind::FileOnly,
                language_level: module.language_level.clone(),
                units,
                completeness: Completeness::Complete,
            };
        }
        let test = files.is_in_test_source_content(path);
        let completeness = completeness(model, module);
        ClasspathSnapshot::for_module(index, model, module, test, completeness)
    }
}

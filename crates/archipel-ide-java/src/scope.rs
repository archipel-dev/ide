//! The resolve scope of a file (spec 02 section 2.7, PLAT-SCOPE-054/062/067): the file's
//! module with its dependencies, libraries and SDK, tests included only for test sources.
//! A scope is a value: unit id to classpath rank.

use crate::index::JavaIndex;
use archipel_ide_projectmodel::{ClasspathKind, ProjectFileIndex, ProjectModel};
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
    /// A file outside every module (scratch): everything indexed.
    Everything,
}

impl ResolveScope {
    pub fn contains(&self, unit: u32) -> bool {
        self.ranks.contains_key(&unit)
    }

    /// Classpath position of a unit (lower is better).
    pub fn rank(&self, unit: u32) -> Option<u32> {
        self.ranks.get(&unit).copied()
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
    pub fn for_file(index: &JavaIndex, model: &ProjectModel, files: &ProjectFileIndex, path: &Path) -> Self {
        let cat = index.inner.catalog.read();
        let Some(module) = files.module_for_file(path) else {
            // Outside every module: everything that is indexed, plus the file itself.
            let mut s = Self { kind: ScopeKind::Everything, ..Self::default() };
            for (i, id) in cat.units.keys().enumerate() {
                s.ranks.insert(*id, i as u32);
            }
            return s;
        };
        if !files.is_in_source_content(path) {
            let mut s = Self { module: Some(module.name.clone()), kind: ScopeKind::FileOnly, ..Self::default() };
            if let Some(id) = cat.by_path.get(path) {
                s.ranks.insert(*id, 0);
            }
            return s;
        }
        let test = files.is_in_test_source_content(path);
        let kind = if test { ClasspathKind::TestCompile } else { ClasspathKind::ProductionCompile };
        let mut s = Self { module: Some(module.name.clone()), test, ..Self::default() };
        let mut rank = 0u32;
        let mut put = |s: &mut Self, unit: u32| {
            s.ranks.entry(unit).or_insert_with(|| {
                rank += 1;
                rank
            });
        };
        for (i, m) in model.modules_with_dependencies(module, kind).into_iter().enumerate() {
            if let Some((prod, tests)) = cat.modules.get(&m.name) {
                // Own tests only for test code; a dependency's tests are never on our classpath.
                for u in prod {
                    put(&mut s, *u);
                }
                if i == 0 && test {
                    for u in tests {
                        put(&mut s, *u);
                    }
                }
            }
        }
        for lib in model.libraries_for(module, kind) {
            if let Some(units) = cat.libraries.get(&lib.name) {
                for u in units {
                    put(&mut s, *u);
                }
            }
        }
        if let Some(sdk) = model.module_sdk(module)
            && let Some(units) = cat.sdks.get(&sdk.name)
        {
            for u in units {
                put(&mut s, *u);
            }
        }
        s
    }
}

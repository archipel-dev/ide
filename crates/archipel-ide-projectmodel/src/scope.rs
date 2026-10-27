//! `GlobalSearchScope`-style scopes over the project model.
//!
//! A scope answers "does this file belong?". The constructors mirror the factory methods of
//! IntelliJ: `projectScope`, `projectProductionScope`, `moduleScope`,
//! `moduleWithDependenciesScope`, `moduleWithDependenciesAndLibrariesScope`, `allScope`,
//! `everythingScope`.

use crate::file_index::ProjectFileIndex;
use crate::model::ProjectModel;
use crate::order::ClasspathKind;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Which part of module content a scope takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentFilter {
    pub production: bool,
    pub tests: bool,
}

impl ContentFilter {
    pub const ALL: Self = Self {
        production: true,
        tests: true,
    };
    pub const PRODUCTION: Self = Self {
        production: true,
        tests: false,
    };
    pub const TESTS: Self = Self {
        production: false,
        tests: true,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobalSearchScope {
    /// Every file, in or outside the project (`everythingScope`).
    Everything,
    /// Module content and libraries (`allScope`).
    All,
    /// Content of all modules, production and tests (`projectScope`).
    Project,
    /// Production content of all modules (`projectProductionScope`).
    ProjectProduction,
    /// Test content of all modules.
    ProjectTests,
    /// Classes, sources and documentation roots of every project library.
    Libraries,
    /// Content of the named modules and the roots of the named libraries.
    Modules {
        modules: BTreeSet<String>,
        libraries: BTreeSet<String>,
        content: ContentFilter,
    },
    /// Everything below a directory (or the file itself).
    Directory(PathBuf),
    Union(Vec<GlobalSearchScope>),
    Intersection(Vec<GlobalSearchScope>),
    Not(Box<GlobalSearchScope>),
}

impl GlobalSearchScope {
    /// `moduleScope`: only the module's own content.
    pub fn module_scope(module: &str) -> Self {
        Self::Modules {
            modules: BTreeSet::from([module.to_owned()]),
            libraries: BTreeSet::new(),
            content: ContentFilter::ALL,
        }
    }

    /// `moduleWithDependenciesScope`: the module and the modules it depends on; the tests of the
    /// module are included, the tests of dependencies never are on the classpath but their
    /// production code is.
    pub fn module_with_dependencies(model: &ProjectModel, module: &str) -> Self {
        Self::dependencies_scope(model, module, false, true)
    }

    /// `moduleWithDependenciesAndLibrariesScope`.
    pub fn module_with_dependencies_and_libraries(model: &ProjectModel, module: &str) -> Self {
        Self::dependencies_scope(model, module, true, true)
    }

    /// Like `moduleWithDependenciesScope(module, includeTests = false)`: production code only.
    pub fn module_production_with_dependencies(
        model: &ProjectModel,
        module: &str,
        with_libraries: bool,
    ) -> Self {
        Self::dependencies_scope(model, module, with_libraries, false)
    }

    fn dependencies_scope(
        model: &ProjectModel,
        module: &str,
        with_libraries: bool,
        include_tests: bool,
    ) -> Self {
        let Some(root) = model.module(module) else {
            return Self::module_scope(module);
        };
        let kind = if include_tests {
            ClasspathKind::TestRuntime
        } else {
            ClasspathKind::ProductionRuntime
        };
        let modules = model.modules_with_dependencies(root, kind);
        // Libraries of the dependency chain: a runtime view so nothing reachable is missing.
        let libraries = if with_libraries {
            model
                .libraries_for(root, kind)
                .into_iter()
                .map(|l| l.name.clone())
                .collect()
        } else {
            BTreeSet::new()
        };
        let own = Self::Modules {
            modules: BTreeSet::from([root.name.clone()]),
            libraries: BTreeSet::new(),
            content: if include_tests {
                ContentFilter::ALL
            } else {
                ContentFilter::PRODUCTION
            },
        };
        let others = Self::Modules {
            modules: modules.iter().skip(1).map(|m| m.name.clone()).collect(),
            libraries,
            content: ContentFilter::PRODUCTION,
        };
        own.union_with(others)
    }

    pub fn union_with(self, other: Self) -> Self {
        match (self, other) {
            (Self::Everything, _) | (_, Self::Everything) => Self::Everything,
            (Self::Union(mut a), Self::Union(b)) => {
                a.extend(b);
                Self::Union(a)
            }
            (Self::Union(mut a), b) => {
                a.push(b);
                Self::Union(a)
            }
            (a, b) => Self::Union(vec![a, b]),
        }
    }

    pub fn intersect_with(self, other: Self) -> Self {
        match (self, other) {
            (Self::Everything, b) => b,
            (a, Self::Everything) => a,
            (a, b) => Self::Intersection(vec![a, b]),
        }
    }

    pub fn negate(self) -> Self {
        match self {
            Self::Not(inner) => *inner,
            other => Self::Not(Box::new(other)),
        }
    }

    /// `GlobalSearchScope.contains(file)`.
    pub fn contains(&self, index: &ProjectFileIndex, path: &Path) -> bool {
        match self {
            Self::Everything => true,
            Self::All => index.is_in_project(path),
            Self::Project => index.is_in_content(path),
            Self::ProjectProduction => {
                index.is_in_content(path) && !index.is_in_test_source_content(path)
            }
            Self::ProjectTests => index.is_in_test_source_content(path),
            Self::Libraries => index.is_in_library(path),
            Self::Modules {
                modules,
                libraries,
                content,
            } => {
                if let Some(module) = index.module_for_file(path) {
                    let test = index.is_in_test_source_content(path);
                    return modules.contains(&module.name)
                        && if test {
                            content.tests
                        } else {
                            content.production
                        };
                }
                index
                    .library_for_path(path)
                    .is_some_and(|library| libraries.contains(&library.name))
            }
            Self::Directory(dir) => crate::path::normalize(path).starts_with(dir),
            Self::Union(scopes) => scopes.iter().any(|s| s.contains(index, path)),
            Self::Intersection(scopes) => scopes.iter().all(|s| s.contains(index, path)),
            Self::Not(inner) => !inner.contains(index, path),
        }
    }

    /// `isSearchInLibraries`.
    pub fn is_search_in_libraries(&self) -> bool {
        match self {
            Self::Everything | Self::All | Self::Libraries => true,
            Self::Modules { libraries, .. } => !libraries.is_empty(),
            Self::Union(scopes) => scopes.iter().any(Self::is_search_in_libraries),
            Self::Intersection(scopes) => scopes.iter().all(Self::is_search_in_libraries),
            _ => false,
        }
    }
}

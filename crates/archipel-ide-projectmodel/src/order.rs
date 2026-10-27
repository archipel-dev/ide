//! Order entries and dependency scopes (`OrderEntry`, `DependencyScope`).

use serde::{Deserialize, Serialize};

/// Which classpath a dependency is on (`JpsJavaClasspathKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClasspathKind {
    ProductionCompile,
    ProductionRuntime,
    TestCompile,
    TestRuntime,
}

impl ClasspathKind {
    pub fn is_test(self) -> bool {
        matches!(self, Self::TestCompile | Self::TestRuntime)
    }

    pub fn is_runtime(self) -> bool {
        matches!(self, Self::ProductionRuntime | Self::TestRuntime)
    }

    /// The production classpath of the same phase, used for the dependencies of
    /// dependencies (a dependency's tests are never on someone else's classpath).
    pub fn production(self) -> Self {
        if self.is_runtime() {
            Self::ProductionRuntime
        } else {
            Self::ProductionCompile
        }
    }
}

/// How far a dependency reaches (`DependencyScope`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DependencyScope {
    /// All classpaths.
    #[default]
    Compile,
    /// Test classpaths only.
    Test,
    /// Runtime classpaths only (production and test).
    Runtime,
    /// Compile classpaths and the test runtime classpath, not the production runtime one.
    Provided,
}

impl DependencyScope {
    pub fn is_included_in(self, kind: ClasspathKind) -> bool {
        use ClasspathKind::*;
        match self {
            Self::Compile => true,
            Self::Test => matches!(kind, TestCompile | TestRuntime),
            Self::Runtime => matches!(kind, ProductionRuntime | TestRuntime),
            Self::Provided => !matches!(kind, ProductionRuntime),
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Compile => "compile",
            Self::Test => "test",
            Self::Runtime => "runtime",
            Self::Provided => "provided",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "compile" => Self::Compile,
            "test" => Self::Test,
            "runtime" => Self::Runtime,
            "provided" => Self::Provided,
            _ => return None,
        })
    }

    /// Scope of something that is on the compile and the runtime classpath of the
    /// given kinds. Used by importers that classify jars from classpath listings.
    pub fn from_classpaths(compile: bool, runtime: bool, test_only: bool) -> Self {
        match (compile, runtime, test_only) {
            (_, _, true) => Self::Test,
            (true, true, _) => Self::Compile,
            (true, false, _) => Self::Provided,
            (false, _, _) => Self::Runtime,
        }
    }
}

/// An entry of a module's dependency list, in order (`OrderEntry`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OrderEntry {
    /// `InheritedJdkOrderEntry`: the project SDK.
    InheritedSdk,
    /// `JdkOrderEntry`: an SDK chosen for the module.
    ModuleSdk { sdk: String },
    /// `ModuleSourceOrderEntry`: the module's own sources.
    ModuleSource,
    /// `LibraryOrderEntry`, referring to a project library by name.
    #[serde(rename_all = "camelCase")]
    Library {
        library: String,
        #[serde(default)]
        scope: DependencyScope,
        #[serde(default)]
        exported: bool,
    },
    /// `ModuleOrderEntry`, referring to another module by name.
    #[serde(rename_all = "camelCase")]
    Module {
        module: String,
        #[serde(default)]
        scope: DependencyScope,
        #[serde(default)]
        exported: bool,
    },
}

impl OrderEntry {
    pub fn library(name: impl Into<String>, scope: DependencyScope) -> Self {
        Self::Library {
            library: name.into(),
            scope,
            exported: false,
        }
    }

    pub fn module(name: impl Into<String>, scope: DependencyScope) -> Self {
        Self::Module {
            module: name.into(),
            scope,
            exported: false,
        }
    }

    pub fn scope(&self) -> Option<DependencyScope> {
        match self {
            Self::Library { scope, .. } | Self::Module { scope, .. } => Some(*scope),
            _ => None,
        }
    }

    pub fn is_exported(&self) -> bool {
        matches!(
            self,
            Self::Library { exported: true, .. } | Self::Module { exported: true, .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ClasspathKind::*;

    #[test]
    fn scopes_match_the_jps_classpath_kinds() {
        let table = [
            (DependencyScope::Compile, [true, true, true, true]),
            (DependencyScope::Test, [false, false, true, true]),
            (DependencyScope::Runtime, [false, true, false, true]),
            (DependencyScope::Provided, [true, false, true, true]),
        ];
        for (scope, expected) in table {
            let got = [
                ProductionCompile,
                ProductionRuntime,
                TestCompile,
                TestRuntime,
            ]
            .map(|kind| scope.is_included_in(kind));
            assert_eq!(got, expected, "{scope:?}");
        }
    }

    #[test]
    fn classifies_from_classpath_membership() {
        assert_eq!(
            DependencyScope::from_classpaths(true, true, false),
            DependencyScope::Compile
        );
        assert_eq!(
            DependencyScope::from_classpaths(true, false, false),
            DependencyScope::Provided
        );
        assert_eq!(
            DependencyScope::from_classpaths(false, true, false),
            DependencyScope::Runtime
        );
        assert_eq!(
            DependencyScope::from_classpaths(false, true, true),
            DependencyScope::Test
        );
    }

    #[test]
    fn serializes_with_a_type_tag() {
        let entry = OrderEntry::library("Gradle: a:b:1", DependencyScope::Test);
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["type"], "library");
        assert_eq!(json["scope"], "test");
        assert_eq!(serde_json::from_value::<OrderEntry>(json).unwrap(), entry);
    }
}

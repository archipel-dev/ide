//! The classpath snapshot of a module and scope (spec 02 section 2.7, JAVA-RES-029, 030, 033;
//! PLAT-SCOPE-054, 062..067).
//!
//! A snapshot is an immutable value derived from the project model: the ordered units (own
//! sources, module dependencies, libraries by scope, SDK), the language level and whether
//! the answer is complete. [`ResolveScope`] is a view over it; the status bar and the control
//! socket show [`Completeness`], so a missing jar is explained, never silent.

use crate::index::{JavaIndex, UnitKind};
use crate::scope::{ResolveScope, ScopeKind};
use archipel_ide_projectmodel::{ClasspathKind, Module, ProjectModel};
use std::path::{Path, PathBuf};

/// What a unit on the classpath is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitRole {
    /// A source file of a module (own, or of a dependency).
    Source {
        module: String,
        test: bool,
    },
    Library(String),
    Sdk(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClasspathUnit {
    pub id: u32,
    pub role: UnitRole,
    pub kind: UnitKind,
    pub path: PathBuf,
}

/// Whether the classpath is the complete truth, and why not.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Completeness {
    #[default]
    Complete,
    /// The project model is being (re)imported.
    Importing,
    /// The import finished with notes (dependencies were not resolved, a resolver failed).
    ImportFailed(Vec<String>),
    /// Libraries of the module whose jars are not on disk.
    MissingLibraries(Vec<String>),
    /// A module imported from a build tool without any library.
    NoLibraries,
}

impl Completeness {
    pub fn is_complete(&self) -> bool {
        matches!(self, Completeness::Complete)
    }

    /// One line for the status bar and the control socket; `None` when complete.
    pub fn describe(&self) -> Option<String> {
        match self {
            Completeness::Complete => None,
            Completeness::Importing => Some("project import in progress".into()),
            Completeness::ImportFailed(notes) => Some(format!(
                "import finished with problems: {}",
                notes
                    .first()
                    .map_or("see the Build tool window", String::as_str)
            )),
            Completeness::MissingLibraries(names) => Some(format!(
                "{} librar{} missing on disk: {}",
                names.len(),
                if names.len() == 1 { "y" } else { "ies" },
                names.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
            )),
            Completeness::NoLibraries => {
                Some("the imported project has no libraries on its classpath".into())
            }
        }
    }
}

/// The classpath of one module for production or test code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClasspathSnapshot {
    /// `None` for the ad-hoc module of a file outside every module.
    pub module: Option<String>,
    pub test: bool,
    pub kind: ScopeKind,
    pub language_level: Option<String>,
    pub units: Vec<ClasspathUnit>,
    pub completeness: Completeness,
}

impl ClasspathSnapshot {
    /// The classpath of `module` for production (`test` false) or test code.
    pub fn for_module(
        index: &JavaIndex,
        model: &ProjectModel,
        module: &Module,
        test: bool,
        completeness: Completeness,
    ) -> Self {
        let kind = if test {
            ClasspathKind::TestCompile
        } else {
            ClasspathKind::ProductionCompile
        };
        let cat = index.inner.catalog.read();
        let mut units: Vec<ClasspathUnit> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut put = |units: &mut Vec<ClasspathUnit>, id: u32, role: UnitRole| {
            if !seen.insert(id) {
                return;
            }
            let (path, ukind) = cat
                .units
                .get(&id)
                .map(|u| (u.path.clone(), u.kind.clone()))
                .unwrap_or_default();
            units.push(ClasspathUnit {
                id,
                role,
                kind: ukind,
                path,
            });
        };
        for (i, m) in model
            .modules_with_dependencies(module, kind)
            .into_iter()
            .enumerate()
        {
            if let Some((prod, tests)) = cat.modules.get(&m.name) {
                for u in prod {
                    put(
                        &mut units,
                        *u,
                        UnitRole::Source {
                            module: m.name.clone(),
                            test: false,
                        },
                    );
                }
                // Own tests only for test code; a dependency's tests are never on our classpath.
                if i == 0 && test {
                    for u in tests {
                        put(
                            &mut units,
                            *u,
                            UnitRole::Source {
                                module: m.name.clone(),
                                test: true,
                            },
                        );
                    }
                }
            }
        }
        for lib in model.libraries_for(module, kind) {
            if let Some(ids) = cat.libraries.get(&lib.name) {
                for u in ids {
                    put(&mut units, *u, UnitRole::Library(lib.name.clone()));
                }
            }
        }
        if let Some(sdk) = model.module_sdk(module)
            && let Some(ids) = cat.sdks.get(&sdk.name)
        {
            for u in ids {
                put(&mut units, *u, UnitRole::Sdk(sdk.name.clone()));
            }
        }
        Self {
            module: Some(module.name.clone()),
            test,
            kind: ScopeKind::Module,
            language_level: module.language_level.clone(),
            units,
            completeness,
        }
    }

    /// The ad-hoc module of a file outside every module (scratch file, folder without a build
    /// file): the project SDK (else the first JDK of the model) and the file itself.
    pub fn ad_hoc(index: &JavaIndex, model: &ProjectModel, file: &Path) -> Self {
        let cat = index.inner.catalog.read();
        let mut units = Vec::new();
        let sdk = model
            .project_sdk
            .as_deref()
            .and_then(|n| model.sdk(n))
            .filter(|s| cat.sdks.contains_key(&s.name))
            .or_else(|| model.sdks.iter().find(|s| cat.sdks.contains_key(&s.name)));
        if let Some(sdk) = sdk
            && let Some(ids) = cat.sdks.get(&sdk.name)
        {
            for u in ids {
                units.push(ClasspathUnit {
                    id: *u,
                    role: UnitRole::Sdk(sdk.name.clone()),
                    kind: cat
                        .units
                        .get(u)
                        .map(|x| x.kind.clone())
                        .unwrap_or(UnitKind::Jdk),
                    path: cat.units.get(u).map(|x| x.path.clone()).unwrap_or_default(),
                });
            }
        }
        if let Some(id) = cat.by_path.get(file) {
            units.insert(
                0,
                ClasspathUnit {
                    id: *id,
                    role: UnitRole::Source {
                        module: String::new(),
                        test: false,
                    },
                    kind: UnitKind::Source,
                    path: file.to_path_buf(),
                },
            );
        }
        Self {
            module: None,
            test: false,
            kind: ScopeKind::AdHoc,
            language_level: None,
            units,
            completeness: Completeness::Complete,
        }
    }

    /// The resolve scope over this snapshot: unit id to classpath rank (from 1).
    pub fn scope(&self) -> ResolveScope {
        let mut s = ResolveScope::of_units(self.units.iter().map(|u| u.id));
        s.module = self.module.clone();
        s.test = self.test;
        s.kind = self.kind;
        s
    }

    /// Library names on the classpath (diagnostics, status).
    pub fn libraries(&self) -> impl Iterator<Item = &str> {
        self.units.iter().filter_map(|u| match &u.role {
            UnitRole::Library(n) => Some(n.as_str()),
            _ => None,
        })
    }
}

/// The completeness of the classpath of `module` given the import state.
pub fn completeness_of(
    model: &ProjectModel,
    module: &Module,
    loading: bool,
    notes: &[String],
) -> Completeness {
    if loading {
        return Completeness::Importing;
    }
    if !notes.is_empty() {
        return Completeness::ImportFailed(notes.to_vec());
    }
    let libs = model.libraries_for(module, ClasspathKind::TestCompile);
    let missing: Vec<String> = libs
        .iter()
        .filter(|l| l.classes.iter().any(|c| !c.exists()))
        .map(|l| l.name.clone())
        .collect();
    if !missing.is_empty() {
        return Completeness::MissingLibraries(missing);
    }
    if libs.is_empty() && module.external_project.is_some() {
        return Completeness::NoLibraries;
    }
    Completeness::Complete
}

/// The JDK home found without a project: `JAVA_HOME`, else the `java` on `PATH`.
pub fn detect_jdk_home() -> Option<PathBuf> {
    let usable = |home: &Path| {
        home.join("lib/modules").exists()
            || home.join("jre/lib/rt.jar").exists()
            || home.join("lib/rt.jar").exists()
    };
    if let Some(home) = std::env::var_os("JAVA_HOME").map(PathBuf::from)
        && usable(&home)
    {
        return Some(home);
    }
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let java = dir.join("java");
        if let Ok(real) = std::fs::canonicalize(&java)
            && let Some(home) = real.parent().and_then(Path::parent)
            && usable(home)
        {
            return Some(home.to_path_buf());
        }
    }
    None
}

/// The synthetic model of a folder without an imported project: one module over the folder
/// (content root and source root) and the detected JDK. `None` without a JDK.
pub fn ad_hoc_model(root: &Path) -> Option<ProjectModel> {
    use archipel_ide_projectmodel::{JpsModuleSourceRootType, Sdk, SourceFolder};
    let home = detect_jdk_home()?;
    let mut model = ProjectModel::new("ad-hoc", root);
    let mut module = Module::new("ad-hoc");
    let entry = module.content_entry_mut(root);
    entry.add_source_folder(SourceFolder::new(root, JpsModuleSourceRootType::Source));
    for skip in [
        ".git",
        ".idea",
        ".gradle",
        ".archipel",
        "node_modules",
        "target",
        "build",
        "out",
    ] {
        entry.excluded_folders.push(root.join(skip));
    }
    model.modules.push(module);
    let sdk = Sdk::jdk_at(&home, None);
    model.project_sdk = Some(sdk.name.clone());
    model.put_sdk(sdk);
    Some(model)
}

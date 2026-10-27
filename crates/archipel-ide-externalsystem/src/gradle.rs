//! Gradle: the init script writes the configured model (projects, source sets with roots,
//! resolved `compileClasspath`/`runtimeClasspath` per source set); this resolver turns it
//! into data nodes.
//!
//! Gradle is the wrapper or, without one, the managed/`PATH` Gradle (`GradleBuild::program`).
//! The first run is `--offline`, so a warm Gradle cache gives the model without network;
//! when that fails or leaves dependencies unresolved, the model is requested again online
//! (unless the user asked for offline only).
//!
//! One module per Gradle project (not per source set): the source sets of a project become
//! source folders of its module, and `test` style source sets contribute `test` scoped
//! dependencies. IntelliJ can also create a module per source set; see the known gaps in
//! `docs/architecture/project-model.md`.

use crate::artifacts::sibling_artifacts;
use crate::data::*;
use crate::process::run_plan;
use crate::resolver::{ImportError, ProjectResolver, ResolveRequest};
use archipel_ide_gradle::model::{STATE_DIR, SourceSetModel};
use archipel_ide_gradle::{DependencyModel, GradleBuild, GradleModel};
use archipel_ide_projectmodel::{Coordinates, DependencyScope, ModuleType, SdkType};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct GradleProjectResolver;

const TIME_LIMIT: Duration = Duration::from_secs(20 * 60);

impl ProjectResolver for GradleProjectResolver {
    fn system_id(&self) -> ProjectSystemId {
        ProjectSystemId::Gradle
    }

    fn is_applicable(&self, root: &Path) -> bool {
        GradleBuild::is_gradle_root(root)
    }

    fn is_build_input(&self, path: &Path) -> bool {
        archipel_ide_gradle::is_build_input(path)
    }

    fn resolve(&self, request: &ResolveRequest) -> Result<DataNode, ImportError> {
        let build = GradleBuild::detect(&request.root)
            .ok_or_else(|| ImportError::Invalid("not a Gradle build".into()))?;
        let model = load_model(&build, request)?;
        Ok(to_data_nodes(&model))
    }
}

fn load_model(build: &GradleBuild, request: &ResolveRequest) -> Result<GradleModel, ImportError> {
    let dir = archipel_ide_compat::project_write_path(build.root.join(STATE_DIR));
    std::fs::create_dir_all(&dir)?;
    let file = dir.join("import-model.json");
    let attempts: &[bool] = if request.settings.offline_only {
        &[true]
    } else {
        &[true, false]
    };
    let mut last_error = None;
    let mut fallback: Option<GradleModel> = None;
    for (n, &offline) in attempts.iter().enumerate() {
        request.cancel.check()?;
        request.progress(if offline {
            "Gradle: reading the project model from the local cache"
        } else {
            "Gradle: resolving dependencies"
        });
        let _ = std::fs::remove_file(&file);
        let plan = build
            .model_plan(&file, offline)
            .map_err(|e| ImportError::ToolMissing(e.to_string()))?;
        let captured = run_plan(&plan, &request.cancel, TIME_LIMIT)?;
        if !captured.success() {
            last_error = Some(ImportError::ToolFailed {
                tool: "Gradle".into(),
                message: captured.tail(),
            });
            continue;
        }
        let text = std::fs::read_to_string(&file).map_err(|_| ImportError::ToolFailed {
            tool: "Gradle".into(),
            message: "the build finished but did not write the project model".into(),
        })?;
        let model = GradleModel::parse(&text).map_err(ImportError::Invalid)?;
        let complete = model.projects.iter().all(|p| p.unresolved.is_empty());
        if complete || n + 1 == attempts.len() {
            return Ok(model);
        }
        // Offline left holes: keep it as the result if the online attempt fails too.
        fallback = Some(model);
    }
    fallback.ok_or_else(|| {
        last_error.unwrap_or_else(|| ImportError::Invalid("Gradle produced no model".into()))
    })
}

/// IntelliJ names Gradle modules `<root project>.<path segments>`.
pub(crate) fn module_name(root_name: &str, path: &str) -> String {
    let rest = path.trim_start_matches(':').replace(':', ".");
    if rest.is_empty() {
        root_name.to_owned()
    } else {
        format!("{root_name}.{rest}")
    }
}

pub(crate) fn library_name(dep: &DependencyModel) -> Option<String> {
    let coordinates = dep.coordinates()?;
    Some(match dep.classifier.as_deref().filter(|c| !c.is_empty()) {
        Some(classifier) => format!("Gradle: {coordinates}:{classifier}"),
        None => format!("Gradle: {coordinates}"),
    })
}

/// Turns a Gradle model into a project node. Pure apart from checking which source and
/// javadoc jars exist next to the dependency jars.
pub fn to_data_nodes(model: &GradleModel) -> DataNode {
    let root_project = model.root();
    let root_name = root_project
        .map(|p| p.name.clone())
        .or_else(|| {
            model
                .root_dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "project".into());
    let sdk_name = model.jdk.as_ref().map(|jdk| format!("JDK {}", jdk.version));
    let mut project = DataNode::new(NodeData::Project(ProjectData {
        system: ProjectSystemId::Gradle,
        name: root_name.clone(),
        root: model.root_dir.clone(),
        sdk: sdk_name.clone(),
    }));
    if let (Some(jdk), Some(name)) = (&model.jdk, &sdk_name) {
        project.add(NodeData::Sdk(SdkData {
            name: name.clone(),
            kind: SdkType::Jdk,
            home: jdk.home.clone(),
            version: Some(jdk.version.clone()),
            sources: vec![jdk.home.join("lib").join("src.zip")],
        }));
    }

    // Libraries are collected for the whole build first, then each module refers to them.
    let mut libraries: BTreeMap<String, LibraryData> = BTreeMap::new();
    let mut modules: Vec<DataNode> = Vec::new();
    for gradle in &model.projects {
        let mut module = DataNode::new(NodeData::Module(ModuleData {
            id: gradle.path.clone(),
            name: module_name(&root_name, &gradle.path),
            module_type: ModuleType::Java,
            path: gradle.dir.clone(),
            language_level: gradle.language_level.clone(),
            compile_output: Vec::new(),
            test_compile_output: Vec::new(),
        }));
        let mut content = ContentRootData::new(&gradle.dir);
        let (mut production_output, mut test_output) = (Vec::new(), Vec::new());
        for set in &gradle.source_sets {
            store_source_set(&mut content, set);
            let outputs = set
                .classes_dirs
                .iter()
                .chain(set.resources_dir.iter())
                .cloned();
            if set.is_test {
                test_output.extend(outputs);
            } else if set.name == "main" || production_output.is_empty() {
                production_output.extend(outputs);
            }
        }
        let build_dir = gradle
            .build_dir
            .clone()
            .unwrap_or_else(|| gradle.dir.join("build"));
        content.store_path(ExternalSystemSourceType::Excluded, build_dir);
        content.store_path(
            ExternalSystemSourceType::Excluded,
            gradle.dir.join(".gradle"),
        );
        if gradle.path == ":" {
            content.store_path(
                ExternalSystemSourceType::Excluded,
                gradle.dir.join(".archipel"),
            );
        }
        if let NodeData::Module(data) = &mut module.data {
            data.compile_output = production_output;
            data.test_compile_output = test_output;
        }
        module.add(NodeData::ContentRoot(content));

        // Dependencies: production sets decide compile/runtime/provided, test sets add
        // `test` scoped ones that production does not already have.
        let mut deps: Vec<(DepKey, DependencyScope, &DependencyModel)> = Vec::new();
        for set in gradle.source_sets.iter().filter(|s| !s.is_test) {
            for entry in classify(set) {
                if !deps.iter().any(|(k, _, _)| *k == entry.0) {
                    deps.push(entry);
                }
            }
        }
        let production_count = deps.len();
        for set in gradle.source_sets.iter().filter(|s| s.is_test) {
            for (key, _, dep) in classify(set) {
                if !deps.iter().any(|(k, _, _)| *k == key) {
                    deps.push((key, DependencyScope::Test, dep));
                }
            }
        }
        debug_assert!(deps.len() >= production_count);
        for (key, scope, dep) in deps {
            match key {
                DepKey::Project(path) => {
                    module.add(NodeData::ModuleDependency(ModuleDependencyData {
                        target: path,
                        scope,
                        exported: false,
                    }));
                }
                DepKey::File(_) => {
                    let Some(name) = library_name(dep) else {
                        continue;
                    };
                    libraries
                        .entry(name.clone())
                        .or_insert_with(|| library_data(&name, dep));
                    module.add(NodeData::LibraryDependency(LibraryDependencyData {
                        library: name,
                        scope,
                        exported: false,
                    }));
                }
            }
        }
        for problem in gradle.unresolved.iter().take(10) {
            module.add(NodeData::Diagnostic(DiagnosticData {
                severity: Severity::Warning,
                message: format!("{}: unresolved dependency {problem}", gradle.path),
            }));
        }
        modules.push(module);
    }
    for library in libraries.into_values() {
        project.add(NodeData::Library(library));
    }
    project.children.extend(modules);
    project
}

fn store_source_set(content: &mut ContentRootData, set: &SourceSetModel) {
    use ExternalSystemSourceType as T;
    let (source, resource, generated) = if set.is_test {
        (T::Test, T::TestResource, T::TestGenerated)
    } else {
        (T::Source, T::Resource, T::SourceGenerated)
    };
    let sources: &Vec<PathBuf> = if set.sources.is_empty() {
        &set.java
    } else {
        &set.sources
    };
    // Generated roots first so a generated directory is not also listed as a plain source.
    for path in &set.generated {
        content.store_path(generated, path);
    }
    for path in sources {
        if !set.generated.contains(path) && !already_stored(content, path) {
            content.store_path(source, path);
        }
    }
    for path in &set.resources {
        if !already_stored(content, path) {
            content.store_path(resource, path);
        }
    }
}

fn already_stored(content: &ContentRootData, path: &Path) -> bool {
    content.folders.iter().any(|(_, p)| p == path)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DepKey {
    Project(String),
    File(PathBuf),
}

fn key_of(dep: &DependencyModel) -> Option<DepKey> {
    match (&dep.project_path, &dep.file) {
        (Some(path), _) => Some(DepKey::Project(path.clone())),
        (None, Some(file)) => Some(DepKey::File(file.clone())),
        _ => None,
    }
}

/// Dependencies of one source set in classpath order, with the scope they imply.
type Classified<'a> = Vec<(DepKey, DependencyScope, &'a DependencyModel)>;

fn classify(set: &SourceSetModel) -> Classified<'_> {
    fn keyed(deps: &[DependencyModel]) -> Vec<(DepKey, &DependencyModel)> {
        deps.iter()
            .filter_map(|d| key_of(d).map(|k| (k, d)))
            .collect()
    }
    let compile = keyed(&set.compile_classpath);
    let runtime = keyed(&set.runtime_classpath);
    let mut out: Classified<'_> = Vec::new();
    for (key, dep) in compile.iter().chain(runtime.iter()) {
        if out.iter().any(|(k, _, _)| k == key) {
            continue;
        }
        let scope = DependencyScope::from_classpaths(
            compile.iter().any(|(k, _)| k == key),
            runtime.iter().any(|(k, _)| k == key),
            false,
        );
        out.push((key.clone(), scope, dep));
    }
    out
}

fn library_data(name: &str, dep: &DependencyModel) -> LibraryData {
    let mut library = LibraryData::new(name);
    if let Some(file) = &dep.file {
        library.binary.push(file.clone());
        let (sources, docs) = sibling_artifacts(file);
        library.source = sources;
        library.documentation = docs;
    }
    if let (Some(group), Some(artifact), Some(version)) = (&dep.group, &dep.name, &dep.version) {
        library.coordinates = Some(Coordinates::new(group, artifact, version));
    }
    library
}

//! The data services applied to hand-built node trees.

use archipel_ide_externalsystem::*;
use archipel_ide_projectmodel::{
    DependencyScope, JpsModuleSourceRootType, ModuleType, OrderEntry, SdkType,
};
use std::path::PathBuf;
use std::sync::Arc;

fn module(id: &str, name: &str, dir: &str) -> DataNode {
    DataNode::new(NodeData::Module(ModuleData {
        id: id.into(),
        name: name.into(),
        module_type: ModuleType::Java,
        path: dir.into(),
        language_level: Some("21".into()),
        compile_output: vec![format!("{dir}/build/classes/java/main").into()],
        test_compile_output: Vec::new(),
    }))
}

fn project(system: ProjectSystemId, name: &str, root: &str) -> DataNode {
    DataNode::new(NodeData::Project(ProjectData {
        system,
        name: name.into(),
        root: root.into(),
        sdk: Some("JDK 21".into()),
    }))
}

fn tree() -> DataNode {
    let mut root = project(ProjectSystemId::Gradle, "demo", "/w/demo");
    root.add(NodeData::Sdk(SdkData {
        name: "JDK 21".into(),
        kind: SdkType::Jdk,
        home: "/nonexistent-jdk".into(),
        version: Some("21".into()),
        sources: vec!["/nonexistent-jdk/lib/src.zip".into()],
    }));
    let mut lib = LibraryData::new("Gradle: org.assertj:assertj-core:3.26.0");
    lib.binary.push("/m2/assertj-core-3.26.0.jar".into());
    lib.source
        .push("/m2/assertj-core-3.26.0-sources.jar".into());
    root.add(NodeData::Library(lib));

    let mut app = module(":app", "demo.app", "/w/demo/app");
    let mut content = ContentRootData::new("/w/demo/app");
    content.store_path(
        ExternalSystemSourceType::Source,
        "/w/demo/app/src/main/java",
    );
    content.store_path(ExternalSystemSourceType::Test, "/w/demo/app/src/test/java");
    content.store_path(
        ExternalSystemSourceType::TestResource,
        "/w/demo/app/src/test/resources",
    );
    content.store_path(
        ExternalSystemSourceType::SourceGenerated,
        "/w/demo/app/build/gen",
    );
    content.store_path(ExternalSystemSourceType::Excluded, "/w/demo/app/build");
    app.add(NodeData::ContentRoot(content));
    app.add(NodeData::ModuleDependency(ModuleDependencyData {
        target: ":lib".into(),
        scope: DependencyScope::Compile,
        exported: false,
    }));
    app.add(NodeData::LibraryDependency(LibraryDependencyData {
        library: "Gradle: org.assertj:assertj-core:3.26.0".into(),
        scope: DependencyScope::Test,
        exported: false,
    }));
    // The same library on the runtime classpath widens the scope.
    app.add(NodeData::LibraryDependency(LibraryDependencyData {
        library: "Gradle: org.assertj:assertj-core:3.26.0".into(),
        scope: DependencyScope::Runtime,
        exported: false,
    }));
    app.add(NodeData::LibraryDependency(LibraryDependencyData {
        library: "Gradle: missing:lib:1".into(),
        scope: DependencyScope::Compile,
        exported: false,
    }));
    root.add_child(app);
    root.add_child(module(":lib", "demo.lib", "/w/demo/lib"));
    root
}

#[test]
fn services_build_the_model_from_nodes() {
    let manager = ProjectDataManager::new();
    let models = manager.import_data(&[tree()]);
    let model = Arc::new(models.model);
    assert_eq!(model.name, "demo");
    assert_eq!(model.project_sdk.as_deref(), Some("JDK 21"));
    assert!(
        model.sdk("JDK 21").unwrap().sources.is_empty(),
        "missing src.zip is not attached"
    );
    let app = model.module("demo.app").unwrap();
    assert_eq!(app.external_project.as_ref().unwrap().system, "gradle");
    assert_eq!(app.external_project.as_ref().unwrap().id, ":app");
    assert_eq!(app.language_level.as_deref(), Some("21"));
    assert!(
        app.order_entries
            .contains(&OrderEntry::module("demo.lib", DependencyScope::Compile))
    );
    assert_eq!(
        app.order_entries
            .iter()
            .filter(|e| matches!(e, OrderEntry::Library { .. }))
            .count(),
        1
    );
    let library_scope = app
        .order_entries
        .iter()
        .find_map(|e| match e {
            OrderEntry::Library { scope, .. } => Some(*scope),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        library_scope,
        DependencyScope::Runtime,
        "test + runtime keeps the wider of the two that is not test"
    );
    let types: Vec<_> = app.source_folders().map(|f| f.root_type).collect();
    assert_eq!(
        types,
        [
            JpsModuleSourceRootType::Source,
            JpsModuleSourceRootType::TestSource,
            JpsModuleSourceRootType::TestResource,
            JpsModuleSourceRootType::Generated
        ]
    );
    assert_eq!(app.excluded_folders().count(), 1);
    // The unknown library is reported, not silently dropped.
    assert!(
        models
            .diagnostics
            .iter()
            .any(|d| d.message.contains("missing:lib:1")),
        "{:?}",
        models.diagnostics
    );
    // And the file index answers questions on the result.
    let index = model.file_index();
    assert!(index.is_in_test_source(&PathBuf::from("/w/demo/app/src/test/java/T.java")));
    assert!(index.is_excluded(&PathBuf::from("/w/demo/app/build/classes/X.class")));
    assert_eq!(
        index
            .library_for_path(&PathBuf::from("/m2/assertj-core-3.26.0.jar"))
            .unwrap()
            .name,
        "Gradle: org.assertj:assertj-core:3.26.0"
    );
}

#[test]
fn nodes_round_trip_through_json() {
    let tree = tree();
    let json = serde_json::to_string(&tree).unwrap();
    let back: DataNode = serde_json::from_str(&json).unwrap();
    assert_eq!(back, tree);
    assert_eq!(tree.find_all(DataKey::Module).count(), 2);
    assert!(tree.size() > 8);
}

#[test]
fn two_systems_share_one_model_and_disambiguate_names() {
    let mut cargo = project(ProjectSystemId::Cargo, "native", "/w/demo");
    cargo.add_child(module("pkg-1", "demo.app", "/w/demo/native"));
    let models = ProjectDataManager::new().import_data(&[tree(), cargo]);
    let names: Vec<_> = models
        .model
        .modules
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    assert_eq!(names, ["demo.app", "demo.lib", "demo.app (Cargo)"]);
    assert_eq!(
        models.model.name, "demo",
        "the first project names the model"
    );
}

#[test]
fn custom_services_can_be_registered() {
    struct Count;
    impl ProjectDataService for Count {
        fn target_key(&self) -> DataKey {
            DataKey::Module
        }
        fn order(&self) -> i32 {
            1000
        }
        fn import_data(&self, items: &[ImportItem<'_>], models: &mut ModifiableModelsProvider) {
            models.report(Severity::Info, format!("{} modules seen", items.len()));
        }
    }
    let mut manager = ProjectDataManager::new();
    manager.register(Box::new(Count));
    let models = manager.import_data(&[tree()]);
    assert!(
        models
            .diagnostics
            .iter()
            .any(|d| d.message == "2 modules seen")
    );
}

struct FakeResolver {
    applicable: bool,
    fail: bool,
}

impl ProjectResolver for FakeResolver {
    fn system_id(&self) -> ProjectSystemId {
        ProjectSystemId::Gradle
    }
    fn is_applicable(&self, _: &std::path::Path) -> bool {
        self.applicable
    }
    fn is_build_input(&self, path: &std::path::Path) -> bool {
        path.ends_with("build.gradle")
    }
    fn resolve(&self, _: &ResolveRequest) -> Result<DataNode, ImportError> {
        if self.fail {
            Err(ImportError::ToolMissing("no gradle".into()))
        } else {
            Ok(tree())
        }
    }
}

#[test]
fn import_pipeline_reports_failures_and_empty_projects() {
    let root = PathBuf::from("/w/empty");
    let request = ResolveRequest::new(&root);
    let manager = ProjectDataManager::new();

    let mut registry = ExternalSystemRegistry::empty();
    let empty = import_project(&registry, &manager, &request).unwrap();
    assert!(empty.model.is_empty() && empty.systems.is_empty());
    assert_eq!(empty.model.name, "empty");

    registry.register(Arc::new(FakeResolver {
        applicable: true,
        fail: true,
    }));
    let error = import_project(&registry, &manager, &request).unwrap_err();
    assert!(error.to_string().contains("no gradle"));

    registry.register(Arc::new(FakeResolver {
        applicable: true,
        fail: false,
    }));
    let result = import_project(&registry, &manager, &request).unwrap();
    assert_eq!(result.systems, [ProjectSystemId::Gradle]);
    assert_eq!(result.model.modules.len(), 2);
    assert_eq!(
        result.errors().count(),
        1,
        "the failed resolver is a diagnostic"
    );
    assert!(registry.is_build_input(&root, &root.join("build.gradle")));
    assert!(!registry.is_build_input(&root, &root.join("Other.java")));
}

#[test]
fn cancelled_imports_stop_early() {
    let mut registry = ExternalSystemRegistry::empty();
    registry.register(Arc::new(FakeResolver {
        applicable: true,
        fail: false,
    }));
    let request = ResolveRequest::new("/w/x");
    request.cancel.cancel();
    let error = import_project(&registry, &ProjectDataManager::new(), &request).unwrap_err();
    assert!(matches!(error, ImportError::Cancelled));
}

#[test]
fn model_cache_round_trips() {
    let root = std::env::temp_dir().join(format!("archipel-es-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    assert!(load_cached_model(&root).is_none());
    let model = ProjectDataManager::new().import_data(&[tree()]).model;
    save_cached_model(&root, &model).unwrap();
    assert_eq!(load_cached_model(&root).unwrap(), model);
    std::fs::remove_dir_all(&root).unwrap();
}

//! Imports Spring Boot projects made by the IDE's own generator with the real build tools.
//! All of these need network access (or warm caches) and take minutes, so they are ignored:
//! `cargo test -p archipel-ide-externalsystem --test spring -- --ignored --test-threads=1`.

use archipel_ide_externalsystem::jar::provider_of;
use archipel_ide_externalsystem::*;
use archipel_ide_project::generator::Language;
use archipel_ide_project::spring::{SpringBuild, SpringSpec};
use archipel_ide_project::spring_catalog::Catalog;
use archipel_ide_projectmodel::{ClasspathKind, DependencyScope, OrderEntry};
use std::path::PathBuf;
use std::sync::Arc;

fn generate(build: SpringBuild, name: &str) -> PathBuf {
    let catalog = Catalog::builtin();
    let spec = SpringSpec {
        language: Language::Java,
        build,
        group: "com.example".into(),
        artifact: name.into(),
        package: "com.example.demo".into(),
        description: "Demo".into(),
        java_version: 21,
        boot_version: catalog
            .boot_versions
            .first()
            .cloned()
            .unwrap_or_else(|| "3.5.0".into()),
        war: false,
        dependencies: ["web", "data-jpa", "lombok", "h2", "validation"]
            .map(String::from)
            .to_vec(),
    };
    let root =
        std::env::temp_dir().join(format!("archipel-es-spring-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (path, text) in spec.files(name, &catalog).expect("valid spec") {
        let target = root.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, text).unwrap();
    }
    root
}

fn import(root: &PathBuf) -> ImportResult {
    let started = std::time::Instant::now();
    let result = import_project(
        &ExternalSystemRegistry::with_defaults(),
        &ProjectDataManager::new(),
        &ResolveRequest::new(root),
    )
    .unwrap_or_else(|e| panic!("import failed: {e}"));
    eprintln!(
        "imported {} in {:.1}s: {} modules, {} libraries, diagnostics {:?}",
        root.display(),
        started.elapsed().as_secs_f32(),
        result.model.modules.len(),
        result.model.libraries.len(),
        result.diagnostics
    );
    result
}

fn assert_spring_project(result: &ImportResult, root: &PathBuf, module: &str) {
    let model = Arc::new(result.model.clone());
    let module = model.module(module).unwrap_or_else(|| {
        panic!(
            "no module {module}: {:?}",
            model.modules.iter().map(|m| &m.name).collect::<Vec<_>>()
        )
    });
    let root = std::fs::canonicalize(root).unwrap();
    let index = model.file_index();
    let main = root.join("src/main/java/com/example/demo");
    let test = root.join("src/test/java/com/example/demo");
    let main_file = std::fs::read_dir(&main)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let test_file = std::fs::read_dir(&test)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(index.is_in_source_content(&main_file) && !index.is_in_test_source(&main_file));
    assert!(index.is_in_test_source(&test_file));
    assert_eq!(index.module_for_file(&main_file).unwrap().name, module.name);
    assert_eq!(
        index.package_name_for_file(&main_file).as_deref(),
        Some("com.example.demo")
    );
    assert!(index.is_in_source_content(&root.join("src/main/resources/application.properties")));
    // Spring and test libraries on the right classpaths, with jars on disk.
    let on = |kind: ClasspathKind, class: &str| {
        provider_of(&model, module, kind, class).map(|l| l.name.clone())
    };
    assert!(
        on(
            ClasspathKind::ProductionCompile,
            "org.springframework.boot.SpringApplication"
        )
        .is_some(),
        "Spring Boot is on the production compile classpath"
    );
    assert!(
        on(
            ClasspathKind::ProductionCompile,
            "org.springframework.web.bind.annotation.RestController"
        )
        .is_some()
    );
    assert!(
        on(
            ClasspathKind::ProductionCompile,
            "org.assertj.core.api.Assertions"
        )
        .is_none(),
        "AssertJ is test only"
    );
    let assertj = on(
        ClasspathKind::TestCompile,
        "org.assertj.core.api.Assertions",
    );
    assert!(
        assertj
            .as_deref()
            .is_some_and(|n| n.contains("assertj-core")),
        "{assertj:?}"
    );
    assert!(on(ClasspathKind::TestCompile, "org.junit.jupiter.api.Test").is_some());
    let jar = provider_of(
        &model,
        module,
        ClasspathKind::TestCompile,
        "org.assertj.core.api.Assertions",
    )
    .unwrap();
    assert!(jar.classes[0].is_file());
    assert_eq!(
        index.library_for_path(&jar.classes[0]).unwrap().name,
        jar.name
    );
    assert!(index.is_in_library_classes(&jar.classes[0]));
    // Lombok is compile only (provided / annotation processor).
    let scope = |part: &str| {
        module.order_entries.iter().find_map(|e| match e {
            OrderEntry::Library { library, scope, .. } if library.contains(part) => Some(*scope),
            _ => None,
        })
    };
    assert!(
        matches!(
            scope("lombok"),
            Some(DependencyScope::Provided | DependencyScope::Compile)
        ),
        "{:?}",
        scope("lombok")
    );
    assert_eq!(scope("assertj-core"), Some(DependencyScope::Test));
    // Gradle sees h2 through `spring-boot-h2console` at compile scope; Maven declares it runtime.
    assert!(
        matches!(
            scope("h2"),
            Some(DependencyScope::Runtime | DependencyScope::Compile)
        ),
        "{:?}",
        scope("h2")
    );
    let with_sources = model
        .libraries
        .iter()
        .filter(|l| !l.sources.is_empty())
        .count();
    eprintln!(
        "{} of {} libraries have source jars",
        with_sources,
        model.libraries.len()
    );
    assert!(
        model
            .sdk(model.project_sdk.as_deref().expect("project SDK"))
            .is_some()
    );
    assert!(
        index.is_excluded(&root.join("build/classes/java/main/x.class"))
            || index.is_excluded(&root.join("target/classes/x.class"))
    );
}

#[test]
#[ignore = "needs Gradle and network access"]
fn spring_boot_gradle_project() {
    let root = generate(SpringBuild::GradleKotlin, "demo-gradle");
    let result = import(&root);
    assert_eq!(result.systems, [ProjectSystemId::Gradle]);
    assert_spring_project(&result, &root, "demo-gradle");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[ignore = "needs Maven and network access"]
fn spring_boot_maven_project() {
    let root = generate(SpringBuild::Maven, "demo-maven");
    let result = import(&root);
    assert_eq!(result.systems, [ProjectSystemId::Maven]);
    assert_spring_project(&result, &root, "demo-maven");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[ignore = "needs Bazel and network access"]
fn spring_boot_bazel_project() {
    let root = generate(SpringBuild::Bazel, "demo-bazel");
    let result = import(&root);
    assert_eq!(result.systems, [ProjectSystemId::Bazel]);
    let model = &result.model;
    assert!(
        model.module("app_lib").is_some() && model.module("app_test").is_some(),
        "{:?}",
        model.modules.iter().map(|m| &m.name).collect::<Vec<_>>()
    );
    let index = Arc::new(model.clone()).file_index();
    let root = std::fs::canonicalize(&root).unwrap();
    assert!(index.is_in_test_source(
        &root.join("src/test/java/com/example/demo/DemoBazelApplicationTests.java")
    ));
    let _ = std::fs::remove_dir_all(&root);
}

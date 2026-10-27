//! Gradle resolver against a real Gradle, with a file based Maven repository so no network is
//! needed. Skipped when Gradle or a JDK is missing or `ARCHIPEL_SKIP_GRADLE_IT` is set.

use archipel_ide_externalsystem::*;
use archipel_ide_projectmodel::{ClasspathKind, DependencyScope, OrderEntry};
use archipel_ide_runner::find_program;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// Publishes `demo:<name>:1.0` into a file repository: pom, jar and a sources jar.
fn publish(repo: &Path, name: &str, depends_on: Option<&str>) {
    let dir = repo.join(format!("demo/{name}/1.0"));
    let dependency = depends_on
        .map(|d| {
            format!(
                "<dependencies><dependency><groupId>demo</groupId><artifactId>{d}</artifactId><version>1.0</version></dependency></dependencies>"
            )
        })
        .unwrap_or_default();
    write(
        &dir,
        &format!("{name}-1.0.pom"),
        &format!(
            "<project xmlns=\"http://maven.apache.org/POM/4.0.0\"><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>{name}</artifactId><version>1.0</version>{dependency}</project>"
        ),
    );
    write(&dir, &format!("{name}-1.0.jar"), "");
    write(&dir, &format!("{name}-1.0-sources.jar"), "");
}

#[test]
fn imports_a_multi_module_build_with_scopes_and_sources() {
    if archipel_ide_compat::env_var_os("ARCHIPEL_SKIP_GRADLE_IT").is_some() {
        eprintln!("skipping: ARCHIPEL_SKIP_GRADLE_IT is set");
        return;
    }
    if find_program("gradle").is_none() || find_program("java").is_none() {
        eprintln!("skipping: Gradle and a JDK must be on PATH");
        return;
    }
    let base = std::env::temp_dir().join(format!("archipel-es-gradle-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let repo = base.join("repo");
    let root = base.join("fixture");
    for (name, dep) in [
        ("api-lib", Some("transitive")),
        ("transitive", None),
        ("annot", None),
        ("driver", None),
        ("testlib", None),
    ] {
        publish(&repo, name, dep);
    }
    let repositories = format!(
        "repositories {{ maven {{ url = uri(\"{}\") }} }}\n",
        repo.display()
    );
    write(
        &root,
        "settings.gradle.kts",
        "rootProject.name = \"fixture\"\ninclude(\":app\", \":lib\")\n",
    );
    write(
        &root,
        "lib/build.gradle.kts",
        &format!(
            "plugins {{ `java-library` }}\n{repositories}dependencies {{ api(\"demo:api-lib:1.0\") }}\n"
        ),
    );
    write(
        &root,
        "app/build.gradle.kts",
        &format!(
            "plugins {{ application }}\n{repositories}dependencies {{\n  implementation(project(\":lib\"))\n  compileOnly(\"demo:annot:1.0\")\n  runtimeOnly(\"demo:driver:1.0\")\n  testImplementation(\"demo:testlib:1.0\")\n}}\napplication {{ mainClass = \"demo.App\" }}\n"
        ),
    );
    write(
        &root,
        "app/src/main/java/demo/App.java",
        "package demo;\npublic class App { public static void main(String[] a) {} }\n",
    );
    write(
        &root,
        "app/src/test/java/demo/AppTest.java",
        "package demo;\nclass AppTest {}\n",
    );
    write(&root, "app/src/main/resources/app.properties", "a=b\n");

    let registry = ExternalSystemRegistry::with_defaults();
    let request = ResolveRequest::new(&root);
    let result = import_project(&registry, &ProjectDataManager::new(), &request)
        .unwrap_or_else(|e| panic!("import failed: {e}"));
    assert_eq!(result.systems, [ProjectSystemId::Gradle]);
    let model = Arc::new(result.model.clone());
    assert_eq!(model.name, "fixture");
    let names: Vec<_> = model.modules.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        names,
        ["fixture", "fixture.app", "fixture.lib"],
        "{:?}",
        result.diagnostics
    );

    let index = model.file_index();
    let canonical = std::fs::canonicalize(&root).unwrap();
    let app_main = canonical.join("app/src/main/java/demo/App.java");
    let app_test = canonical.join("app/src/test/java/demo/AppTest.java");
    assert_eq!(
        index.module_for_file(&app_main).unwrap().name,
        "fixture.app"
    );
    assert!(index.is_in_source_content(&app_main) && !index.is_in_test_source(&app_main));
    assert!(index.is_in_test_source(&app_test));
    assert!(index.is_under_source_root_of_type(
        &canonical.join("app/src/main/resources/app.properties"),
        &[archipel_ide_projectmodel::JpsModuleSourceRootType::Resource]
    ));
    assert!(index.is_excluded(&canonical.join("app/build/classes/java/main/demo/App.class")));
    assert_eq!(
        index.package_name_for_file(&app_main).as_deref(),
        Some("demo")
    );

    let app = model.module("fixture.app").unwrap();
    assert!(
        app.order_entries
            .contains(&OrderEntry::module("fixture.lib", DependencyScope::Compile))
    );
    let scope_of = |lib: &str| {
        app.order_entries.iter().find_map(|e| match e {
            OrderEntry::Library { library, scope, .. } if library.contains(lib) => Some(*scope),
            _ => None,
        })
    };
    assert_eq!(scope_of("api-lib"), Some(DependencyScope::Compile));
    assert_eq!(scope_of("transitive"), Some(DependencyScope::Compile));
    assert_eq!(scope_of("annot"), Some(DependencyScope::Provided));
    assert_eq!(scope_of("driver"), Some(DependencyScope::Runtime));
    assert_eq!(scope_of("testlib"), Some(DependencyScope::Test));

    let api = model.library("Gradle: demo:api-lib:1.0").unwrap();
    assert_eq!(
        api.sources.len(),
        1,
        "the sources jar next to the jar is attached"
    );
    let jar = api.classes[0].clone();
    assert_eq!(index.library_for_path(&jar).unwrap().name, api.name);
    assert!(index.is_in_library_classes(&jar));
    assert!(index.is_in_library_sources(&api.sources[0]));

    let test_classpath = model.classpath(app, ClasspathKind::TestCompile);
    assert!(
        test_classpath
            .iter()
            .any(|p| p.ends_with("testlib-1.0.jar"))
    );
    let production = model.classpath(app, ClasspathKind::ProductionRuntime);
    assert!(production.iter().any(|p| p.ends_with("driver-1.0.jar")));
    assert!(!production.iter().any(|p| p.ends_with("testlib-1.0.jar")));
    assert!(model.sdk(model.project_sdk.as_deref().unwrap()).is_some());
    let _ = PathBuf::new();
    let _ = std::fs::remove_dir_all(&base);
}

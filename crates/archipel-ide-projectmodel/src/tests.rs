use super::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn p(path: &str) -> PathBuf {
    PathBuf::from(path)
}

/// `app` (application, depends on `lib`, spring, lombok (provided), runtime driver, test libs)
/// and `lib` (exports guava).
fn fixture() -> Arc<ProjectModel> {
    let mut model = ProjectModel::new("demo", "/w/demo");
    model.put_sdk(Sdk::new("JDK 21", SdkType::Jdk, "/jdk"));
    model.project_sdk = Some("JDK 21".into());

    let mut app = Module::new("app");
    {
        let entry = app.content_entry_mut("/w/demo/app");
        entry
            .add_source_folder(SourceFolder::new(
                "/w/demo/app/src/main/java",
                JpsModuleSourceRootType::Source,
            ))
            .add_source_folder(SourceFolder::new(
                "/w/demo/app/src/main/resources",
                JpsModuleSourceRootType::Resource,
            ))
            .add_source_folder(SourceFolder::new(
                "/w/demo/app/src/test/java",
                JpsModuleSourceRootType::TestSource,
            ))
            .add_source_folder(SourceFolder::new(
                "/w/demo/app/src/test/resources",
                JpsModuleSourceRootType::TestResource,
            ))
            .add_source_folder(
                SourceFolder::new(
                    "/w/demo/app/build/generated/sources/annotationProcessor/java/main",
                    JpsModuleSourceRootType::Generated,
                )
                .with_package_prefix("gen"),
            )
            .add_excluded_folder("/w/demo/app/build")
            .add_excluded_folder("/w/demo/app/.gradle");
    }
    app.compiler_output.production = Some(p("/w/demo/app/build/classes/java/main"));
    app.compiler_output.test = Some(p("/w/demo/app/build/classes/java/test"));
    app.add_order_entry(OrderEntry::module("lib", DependencyScope::Compile));
    app.add_order_entry(OrderEntry::library("spring", DependencyScope::Compile));
    app.add_order_entry(OrderEntry::library("lombok", DependencyScope::Provided));
    app.add_order_entry(OrderEntry::library("h2", DependencyScope::Runtime));
    app.add_order_entry(OrderEntry::library("assertj", DependencyScope::Test));

    let mut lib = Module::new("lib");
    lib.content_entry_mut("/w/demo/lib")
        .add_source_folder(SourceFolder::new(
            "/w/demo/lib/src/main/java",
            JpsModuleSourceRootType::Source,
        ));
    lib.content_entry_mut("/w/demo/lib")
        .add_source_folder(SourceFolder::new(
            "/w/demo/lib/src/test/java",
            JpsModuleSourceRootType::TestSource,
        ));
    lib.compiler_output.production = Some(p("/w/demo/lib/build/classes/java/main"));
    lib.add_order_entry(OrderEntry::Library {
        library: "guava".into(),
        scope: DependencyScope::Compile,
        exported: true,
    });
    lib.add_order_entry(OrderEntry::library("internal", DependencyScope::Compile));

    model.modules = vec![app, lib];
    model.libraries = vec![
        Library::new("spring")
            .with_classes("/m2/spring-core-6.jar")
            .with_sources("/m2/spring-core-6-sources.jar"),
        Library::new("lombok").with_classes("/m2/lombok.jar"),
        Library::new("h2").with_classes("/m2/h2.jar"),
        Library::new("assertj")
            .with_classes("/m2/assertj-core-3.jar")
            .with_sources("/m2/assertj-core-3-sources.jar")
            .with_javadoc("/m2/assertj-core-3-javadoc.jar")
            .with_coordinates(Coordinates::new("org.assertj", "assertj-core", "3")),
        Library::new("guava").with_classes("/m2/guava.jar"),
        Library::new("internal").with_classes("/m2/internal.jar"),
    ];
    Arc::new(model)
}

#[test]
fn file_to_module_and_roots() {
    let model = fixture();
    let index = model.file_index();
    let main = p("/w/demo/app/src/main/java/demo/App.java");
    assert_eq!(index.module_for_file(&main).unwrap().name, "app");
    assert!(index.is_in_source_content(&main) && index.is_in_source(&main));
    assert!(!index.is_in_test_source_content(&main));
    assert!(index.is_in_production_source(&main));

    let test = p("/w/demo/app/src/test/java/demo/AppTest.java");
    assert!(index.is_in_test_source(&test) && index.is_in_test_source_content(&test));
    assert!(index.is_in_source_content(&test));
    assert!(index.is_under_source_root_of_type(&test, &[JpsModuleSourceRootType::TestSource]));
    assert!(!index.is_under_source_root_of_type(&test, &[JpsModuleSourceRootType::Source]));

    let resource = p("/w/demo/app/src/test/resources/application.yml");
    assert!(index.is_in_test_source_content(&resource));
    assert_eq!(
        index.source_root_for_file(&resource).unwrap().root_type,
        JpsModuleSourceRootType::TestResource
    );

    // The build file is content, but not source.
    let build_file = p("/w/demo/app/build.gradle.kts");
    assert!(index.is_in_content(&build_file));
    assert!(!index.is_in_source_content(&build_file));
    assert_eq!(
        index.content_root_for_file(&build_file),
        Some(Path::new("/w/demo/app"))
    );
    // Outside every module.
    assert!(index.module_for_file(&p("/w/demo/README.md")).is_none());
    assert!(!index.is_in_project(&p("/elsewhere/x.java")));
}

#[test]
fn excluded_folders_and_nested_roots() {
    let model = fixture();
    let index = model.file_index();
    let class = p("/w/demo/app/build/classes/java/main/demo/App.class");
    assert!(index.is_excluded(&class));
    assert!(index.module_for_file(&class).is_none());
    assert!(!index.is_in_content(&class));
    // Asking without honoring exclusion still finds the owner.
    assert_eq!(
        index.module_for_file_ext(&class, false).unwrap().name,
        "app"
    );
    // A source folder inside an excluded directory stays a source folder.
    let generated =
        p("/w/demo/app/build/generated/sources/annotationProcessor/java/main/gen/X.java");
    assert!(!index.is_excluded(&generated));
    assert!(index.is_in_generated_sources(&generated));
    assert_eq!(index.module_for_file(&generated).unwrap().name, "app");
}

#[test]
fn package_names_follow_source_roots_and_prefixes() {
    let model = fixture();
    let index = model.file_index();
    assert_eq!(
        index.package_name_by_directory(Path::new("/w/demo/app/src/main/java/com/acme/web")),
        Some("com.acme.web".into())
    );
    assert_eq!(
        index.package_name_for_file(Path::new("/w/demo/app/src/main/java/App.java")),
        Some(String::new())
    );
    assert_eq!(
        index.package_name_by_directory(Path::new(
            "/w/demo/app/build/generated/sources/annotationProcessor/java/main/a/b"
        )),
        Some("gen.a.b".into())
    );
    assert_eq!(
        index.package_name_by_directory(Path::new("/w/demo/app/docs")),
        None
    );
}

#[test]
fn libraries_by_jar_path() {
    let model = fixture();
    let index = model.file_index();
    let jar = p("/m2/assertj-core-3.jar");
    assert_eq!(index.library_for_path(&jar).unwrap().name, "assertj");
    assert!(index.is_in_library_classes(&jar));
    assert!(!index.is_in_library_sources(&jar));
    assert!(index.is_in_library_sources(&p("/m2/assertj-core-3-sources.jar")));
    assert!(index.is_in_source(&p("/m2/assertj-core-3-sources.jar")));
    // Docs are library content but neither classes nor sources.
    let docs = p("/m2/assertj-core-3-javadoc.jar");
    assert!(index.is_in_library(&docs) && !index.is_in_library_classes(&docs));
    // A class inside the jar.
    let entry = p("/m2/assertj-core-3.jar!/org/assertj/core/api/Assertions.class");
    assert_eq!(index.library_for_path(&entry).unwrap().name, "assertj");
    assert!(index.is_in_library_classes(&entry));
    assert_eq!(
        index
            .library_for_path(&jar)
            .and_then(|l| l.coordinates.as_ref())
            .map(ToString::to_string)
            .as_deref(),
        Some("org.assertj:assertj-core:3")
    );
    assert!(index.library_for_path(&p("/m2/unknown.jar")).is_none());
    // Order entries that refer to the library.
    let entries = index.order_entries_for_file(&jar);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0.name, "app");
}

#[test]
fn sdk_roots_are_found() {
    let mut model = (*fixture()).clone();
    model.sdks[0].classes.push(p("/jdk/lib/modules"));
    let index = Arc::new(model).file_index();
    assert!(index.is_in_sdk(&p("/jdk/lib/modules!/java.base/java/lang/String.class")));
    assert_eq!(
        index.sdk_for_path(&p("/jdk/lib/modules")).unwrap().name,
        "JDK 21"
    );
}

#[test]
fn classpaths_respect_scopes_and_exports() {
    let model = fixture();
    let app = model.module("app").unwrap();
    let names = |kind| -> Vec<String> {
        model
            .classpath(app, kind)
            .iter()
            .map(|p| p.display().to_string())
            .collect()
    };
    let production_compile = names(ClasspathKind::ProductionCompile);
    assert!(production_compile.contains(&"/m2/spring-core-6.jar".to_string()));
    assert!(production_compile.contains(&"/m2/lombok.jar".to_string()));
    assert!(!production_compile.contains(&"/m2/h2.jar".to_string()));
    assert!(!production_compile.contains(&"/m2/assertj-core-3.jar".to_string()));
    // `lib` contributes its output and its exported library, not its private one.
    assert!(production_compile.contains(&"/w/demo/lib/build/classes/java/main".to_string()));
    assert!(production_compile.contains(&"/m2/guava.jar".to_string()));
    assert!(!production_compile.contains(&"/m2/internal.jar".to_string()));

    let production_runtime = names(ClasspathKind::ProductionRuntime);
    assert!(production_runtime.contains(&"/m2/h2.jar".to_string()));
    assert!(!production_runtime.contains(&"/m2/lombok.jar".to_string()));
    // At runtime the private library of `lib` is needed too.
    assert!(production_runtime.contains(&"/m2/internal.jar".to_string()));

    let test_compile = names(ClasspathKind::TestCompile);
    assert_eq!(test_compile[0], "/w/demo/app/build/classes/java/test");
    assert_eq!(test_compile[1], "/w/demo/app/build/classes/java/main");
    assert!(test_compile.contains(&"/m2/assertj-core-3.jar".to_string()));
    assert!(!test_compile.contains(&"/m2/h2.jar".to_string()));
    let test_runtime = names(ClasspathKind::TestRuntime);
    assert!(test_runtime.contains(&"/m2/h2.jar".to_string()));
    assert!(test_runtime.contains(&"/m2/lombok.jar".to_string()));
}

#[test]
fn module_sdk_resolves_inherited_entries() {
    let model = fixture();
    let app = model.module("app").unwrap();
    assert_eq!(model.module_sdk(app).unwrap().name, "JDK 21");
}

#[test]
fn project_and_test_scopes() {
    let model = fixture();
    let index = model.file_index();
    let main = p("/w/demo/app/src/main/java/demo/App.java");
    let test = p("/w/demo/app/src/test/java/demo/AppTest.java");
    let lib_test = p("/w/demo/lib/src/test/java/LibTest.java");
    let jar = p("/m2/spring-core-6.jar");
    let build_output = p("/w/demo/app/build/classes/java/main/App.class");

    let production = GlobalSearchScope::ProjectProduction;
    assert!(production.contains(&index, &main));
    assert!(!production.contains(&index, &test));
    assert!(!production.contains(&index, &jar));

    let tests = GlobalSearchScope::ProjectTests;
    assert!(tests.contains(&index, &test) && tests.contains(&index, &lib_test));
    assert!(!tests.contains(&index, &main));

    let project = GlobalSearchScope::Project;
    assert!(project.contains(&index, &main) && project.contains(&index, &test));
    assert!(!project.contains(&index, &jar) && !project.contains(&index, &build_output));

    let all = GlobalSearchScope::All;
    assert!(all.contains(&index, &jar) && all.contains(&index, &main));
    assert!(!all.contains(&index, &p("/etc/hosts")));
    assert!(GlobalSearchScope::Everything.contains(&index, &p("/etc/hosts")));
    assert!(GlobalSearchScope::Libraries.contains(&index, &jar));
    assert!(!GlobalSearchScope::Libraries.contains(&index, &main));
    assert!(all.is_search_in_libraries() && !project.is_search_in_libraries());
}

#[test]
fn module_dependency_scopes() {
    let model = fixture();
    let index = model.file_index();
    let app_main = p("/w/demo/app/src/main/java/demo/App.java");
    let app_test = p("/w/demo/app/src/test/java/demo/AppTest.java");
    let lib_main = p("/w/demo/lib/src/main/java/Lib.java");
    let lib_test = p("/w/demo/lib/src/test/java/LibTest.java");
    let guava = p("/m2/guava.jar");
    let assertj = p("/m2/assertj-core-3.jar");

    let module_only = GlobalSearchScope::module_scope("app");
    assert!(module_only.contains(&index, &app_main) && module_only.contains(&index, &app_test));
    assert!(!module_only.contains(&index, &lib_main));

    let with_deps = GlobalSearchScope::module_with_dependencies(&model, "app");
    assert!(with_deps.contains(&index, &app_main) && with_deps.contains(&index, &app_test));
    assert!(with_deps.contains(&index, &lib_main));
    assert!(!with_deps.contains(&index, &lib_test));
    assert!(!with_deps.contains(&index, &guava));

    let with_libs = GlobalSearchScope::module_with_dependencies_and_libraries(&model, "app");
    assert!(with_libs.contains(&index, &guava) && with_libs.contains(&index, &assertj));
    assert!(with_libs.is_search_in_libraries());
    assert!(!with_libs.contains(&index, &p("/m2/unknown.jar")));

    let production = GlobalSearchScope::module_production_with_dependencies(&model, "app", true);
    assert!(production.contains(&index, &app_main) && !production.contains(&index, &app_test));
    // The test-only library is not part of the production scope.
    assert!(production.contains(&index, &guava));
    // Libraries named by the scope are included even when only test code uses them.
    let lib_only = GlobalSearchScope::module_with_dependencies_and_libraries(&model, "lib");
    assert!(!lib_only.contains(&index, &app_main));
}

#[test]
fn scope_combinators() {
    let model = fixture();
    let index = model.file_index();
    let main = p("/w/demo/app/src/main/java/demo/App.java");
    let test = p("/w/demo/app/src/test/java/demo/AppTest.java");
    let jar = p("/m2/guava.jar");
    let both = GlobalSearchScope::ProjectProduction.union_with(GlobalSearchScope::ProjectTests);
    assert!(both.contains(&index, &main) && both.contains(&index, &test));
    assert!(!both.contains(&index, &jar));
    let not_tests =
        GlobalSearchScope::Project.intersect_with(GlobalSearchScope::ProjectTests.negate());
    assert!(not_tests.contains(&index, &main) && !not_tests.contains(&index, &test));
    let dir = GlobalSearchScope::Directory(p("/w/demo/lib"));
    assert!(dir.contains(&index, &p("/w/demo/lib/src/main/java/Lib.java")));
    assert!(!dir.contains(&index, &main));
    assert_eq!(
        GlobalSearchScope::Everything.union_with(GlobalSearchScope::Libraries),
        GlobalSearchScope::Everything
    );
    assert_eq!(
        GlobalSearchScope::Libraries.negate().negate(),
        GlobalSearchScope::Libraries
    );
}

#[test]
fn nested_module_owns_its_files() {
    let mut model = ProjectModel::new("n", "/p");
    let mut outer = Module::new("outer");
    outer
        .content_entry_mut("/p")
        .add_excluded_folder("/p/inner");
    let mut inner = Module::new("inner");
    inner
        .content_entry_mut("/p/inner")
        .add_source_folder(SourceFolder::new(
            "/p/inner/src",
            JpsModuleSourceRootType::Source,
        ));
    model.modules = vec![outer, inner];
    let index = Arc::new(model).file_index();
    assert_eq!(
        index
            .module_for_file(Path::new("/p/inner/src/A.java"))
            .unwrap()
            .name,
        "inner"
    );
    assert_eq!(
        index
            .module_for_file(Path::new("/p/inner/pom.xml"))
            .unwrap()
            .name,
        "inner"
    );
    assert_eq!(
        index.module_for_file(Path::new("/p/a.txt")).unwrap().name,
        "outer"
    );
}

#[test]
fn round_trips_through_json() {
    let model = fixture();
    let json = serde_json::to_string(&*model).unwrap();
    let back: ProjectModel = serde_json::from_str(&json).unwrap();
    assert_eq!(&back, &*model);
}

#[test]
fn paths_are_normalized_on_the_way_in() {
    let folder = SourceFolder::new("/a/./b/../c", JpsModuleSourceRootType::Source);
    assert_eq!(folder.path, p("/a/c"));
    let mut entry = ContentEntry::new("/a//b/");
    entry.add_source_folder(folder.clone());
    entry.add_source_folder(folder);
    assert_eq!(entry.source_folders.len(), 1);
    assert_eq!(entry.root, p("/a/b"));
}

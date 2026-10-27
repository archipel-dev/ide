//! Bazel resolver on canned `bazel query --output=xml` output and source files; one ignored
//! test runs a real Bazel.

use archipel_ide_externalsystem::bazel::*;
use archipel_ide_externalsystem::*;
use archipel_ide_projectmodel::{DependencyScope, JpsModuleSourceRootType, OrderEntry};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const XML: &str = r#"<?xml version="1.1" encoding="UTF-8" standalone="no"?>
<query version="2">
  <rule class="java_library" location="/ws/BUILD.bazel:3:13" name="//:app_lib">
    <string name="name" value="app_lib"/>
    <list name="srcs">
      <label value="//:src/main/java/com/acme/App.java"/>
      <label value="//:src/main/java/com/acme/web/Controller.java"/>
    </list>
    <list name="resources"><label value="//:src/main/resources/application.properties"/></list>
    <string name="resource_strip_prefix" value="src/main/resources"/>
    <list name="deps">
      <label value="@maven//:org_springframework_boot_spring_boot_starter"/>
      <label value="//lib:greeter"/>
    </list>
  </rule>
  <rule class="java_binary" location="/ws/BUILD.bazel:20:12" name="//:app">
    <string name="name" value="app"/>
    <list name="runtime_deps"><label value="//:app_lib"/></list>
  </rule>
  <rule class="java_test" location="/ws/BUILD.bazel:30:10" name="//:app_test">
    <string name="name" value="app_test"/>
    <list name="srcs"><label value="//:src/test/java/com/acme/AppTests.java"/></list>
    <list name="deps">
      <label value=":app_lib"/>
      <label value="@maven//:org_junit_platform_junit_platform_console"/>
    </list>
    <list name="runtime_deps"><label value="@maven//:org_slf4j_slf4j_simple"/></list>
  </rule>
  <rule class="java_library" location="/ws/lib/BUILD.bazel:1:13" name="//lib:greeter">
    <string name="name" value="greeter"/>
    <list name="srcs"><label value="//lib:Greeter.java"/></list>
    <list name="exports"><label value="@maven//:com_google_guava_guava"/></list>
  </rule>
</query>"#;

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn workspace(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("archipel-es-bazel-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    write(&root, "MODULE.bazel", "module(name = \"demo\")\n");
    write(
        &root,
        "src/main/java/com/acme/App.java",
        "package com.acme;\npublic class App {}\n",
    );
    write(
        &root,
        "src/main/java/com/acme/web/Controller.java",
        "package com.acme.web;\nclass Controller {}\n",
    );
    write(&root, "src/main/resources/application.properties", "a=b\n");
    write(
        &root,
        "src/test/java/com/acme/AppTests.java",
        "package com.acme;\nclass AppTests {}\n",
    );
    // No package declaration: the directory is the root.
    write(&root, "lib/Greeter.java", "public class Greeter {}\n");
    std::fs::create_dir_all(root.join("bazel-out")).unwrap();
    root
}

#[test]
fn parses_query_xml() {
    let rules = parse_query_xml(XML).unwrap();
    assert_eq!(rules.len(), 4);
    let lib = &rules[0];
    assert_eq!(lib.label, "//:app_lib");
    assert_eq!(lib.kind, "java_library");
    assert_eq!(lib.srcs.len(), 2);
    assert_eq!(
        lib.resource_strip_prefix.as_deref(),
        Some("src/main/resources")
    );
    assert_eq!(lib.deps.len(), 2);
    assert_eq!(rules[1].runtime_deps, ["//:app_lib"]);
    assert!(rules[2].is_test());
    assert_eq!(rules[3].exports, ["@maven//:com_google_guava_guava"]);
}

#[test]
fn labels_and_source_roots() {
    assert_eq!(split_label("//a/b:c", ""), Some(("a/b", "c")));
    assert_eq!(split_label("//:c", ""), Some(("", "c")));
    assert_eq!(split_label(":c", "pkg"), Some(("pkg", "c")));
    assert_eq!(split_label("@maven//:x", ""), None);
    let root = workspace("roots");
    assert_eq!(
        infer_source_root(&root, "src/main/java/com/acme/web/Controller.java"),
        root.join("src/main/java")
    );
    assert_eq!(
        infer_source_root(&root, "lib/Greeter.java"),
        root.join("lib")
    );
    // A file whose directory does not match its package keeps its own directory.
    write(
        &root,
        "odd/Wrong.java",
        "package not.matching;\nclass Wrong {}\n",
    );
    assert_eq!(infer_source_root(&root, "odd/Wrong.java"), root.join("odd"));
    assert_eq!(
        infer_source_root(&root, "missing/File.java"),
        root.join("missing")
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn targets_become_modules_with_inferred_roots_and_dependencies() {
    let root = workspace("nodes");
    let rules = parse_query_xml(XML).unwrap();
    let mut external = BTreeMap::new();
    external.insert(
        "org_springframework_boot_spring_boot_starter".to_owned(),
        PathBuf::from("/ext/starter.jar"),
    );
    let tree = to_data_nodes(&root, &rules, &external);
    let models = ProjectDataManager::new().import_data(&[tree]);
    let model = Arc::new(models.model);
    let names: Vec<_> = model.modules.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        names,
        ["app_lib", "app_test", "lib:greeter"],
        "targets without srcs are not modules"
    );
    assert!(
        models
            .diagnostics
            .iter()
            .any(|d| d.message.contains("without sources")),
        "{:?}",
        models.diagnostics
    );

    let app_lib = model.module("app_lib").unwrap();
    let folders: Vec<_> = app_lib
        .source_folders()
        .map(|f| (f.root_type, f.path.clone()))
        .collect();
    assert!(folders.contains(&(JpsModuleSourceRootType::Source, root.join("src/main/java"))));
    assert!(folders.contains(&(
        JpsModuleSourceRootType::Resource,
        root.join("src/main/resources")
    )));
    assert!(app_lib.excluded_folders().any(|p| p.ends_with("bazel-out")));
    assert!(
        app_lib
            .order_entries
            .contains(&OrderEntry::module("lib:greeter", DependencyScope::Compile))
    );
    let starter = model
        .library("Bazel: @maven//:org_springframework_boot_spring_boot_starter")
        .unwrap();
    assert_eq!(starter.classes, [PathBuf::from("/ext/starter.jar")]);

    let test = model.module("app_test").unwrap();
    assert!(
        test.source_folders()
            .all(|f| f.root_type == JpsModuleSourceRootType::TestSource)
    );
    assert!(
        test.order_entries
            .contains(&OrderEntry::module("app_lib", DependencyScope::Compile))
    );
    assert!(test.order_entries.iter().any(|e| matches!(
        e,
        OrderEntry::Library { library, scope: DependencyScope::Runtime, .. } if library.contains("slf4j")
    )));
    // Libraries whose jars were not found are still listed, without roots.
    assert!(
        model
            .library("Bazel: @maven//:org_slf4j_slf4j_simple")
            .unwrap()
            .classes
            .is_empty()
    );

    let index = model.file_index();
    assert!(index.is_in_test_source(&root.join("src/test/java/com/acme/AppTests.java")));
    assert!(index.is_in_source_content(&root.join("src/main/java/com/acme/App.java")));
    assert_eq!(
        index
            .package_name_for_file(&root.join("src/main/java/com/acme/web/Controller.java"))
            .as_deref(),
        Some("com.acme.web")
    );
    let greeter = model.module("lib:greeter").unwrap();
    assert_eq!(greeter.content_roots().next().unwrap(), &root.join("lib"));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn jar_keys_match_rules_jvm_external_labels() {
    let jar = Path::new(
        "/out/external/rules_jvm_external++maven+maven/v1/https/repo.maven.apache.org/maven2/org/springframework/boot/spring-boot-starter/3.5.0/spring-boot-starter-3.5.0.jar",
    );
    assert_eq!(
        jar_key(jar).as_deref(),
        Some("org_springframework_boot_spring_boot_starter")
    );
    assert_eq!(jar_key(Path::new("/x/maven2/a/b/1/b-1-sources.jar")), None);
}

/// Needs Bazel (or Bazelisk) and network for Bzlmod dependencies.
#[test]
#[ignore = "needs Bazel and network access"]
fn imports_a_real_bazel_workspace() {
    if archipel_ide_runner::find_program("bazel").is_none()
        && archipel_ide_runner::find_program("bazelisk").is_none()
    {
        eprintln!("skipping: Bazel is not installed");
        return;
    }
    let root = workspace("real");
    write(
        &root,
        "BUILD.bazel",
        "java_library(name = \"app_lib\", srcs = glob([\"src/main/java/**/*.java\"]))\njava_test(name = \"app_test\", srcs = glob([\"src/test/java/**/*.java\"]), test_class = \"com.acme.AppTests\", deps = [\":app_lib\"])\n",
    );
    let result = import_project(
        &ExternalSystemRegistry::with_defaults(),
        &ProjectDataManager::new(),
        &ResolveRequest::new(&root),
    )
    .unwrap();
    assert!(
        result.model.module("app_lib").is_some(),
        "{:?}",
        result.diagnostics
    );
    let _ = std::fs::remove_dir_all(&root);
}

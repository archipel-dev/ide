//! Cargo resolver: canned `cargo metadata` output, and the real `cargo metadata` on a
//! workspace with path dependencies (no network).

use archipel_ide_externalsystem::cargo::*;
use archipel_ide_externalsystem::*;
use archipel_ide_projectmodel::{DependencyScope, JpsModuleSourceRootType, ModuleType, OrderEntry};
use archipel_ide_runner::find_program;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const METADATA: &str = r#"{
  "packages": [
    {"name":"app","version":"0.1.0","id":"app 0.1.0 (path+file:///w/app)","edition":"2021",
     "manifest_path":"/w/app/Cargo.toml",
     "targets":[
       {"kind":["bin"],"src_path":"/w/app/src/main.rs"},
       {"kind":["bin"],"src_path":"/w/app/src/bin/tool.rs"},
       {"kind":["test"],"src_path":"/w/app/tests/it.rs"},
       {"kind":["example"],"src_path":"/w/app/examples/demo/main.rs"},
       {"kind":["custom-build"],"src_path":"/w/app/build.rs"}]},
    {"name":"util","version":"0.1.0","id":"util 0.1.0 (path+file:///w/util)","edition":"2021",
     "manifest_path":"/w/util/Cargo.toml","targets":[{"kind":["lib"],"src_path":"/w/util/src/lib.rs"}]},
    {"name":"serde","version":"1.0.200","id":"serde 1.0.200 (registry+x)","manifest_path":"/reg/serde-1.0.200/Cargo.toml","targets":[]},
    {"name":"serde_derive","version":"1.0.200","id":"serde_derive 1.0.200 (registry+x)","manifest_path":"/reg/serde_derive-1.0.200/Cargo.toml","targets":[]},
    {"name":"cc","version":"1.0.0","id":"cc 1.0.0 (registry+x)","manifest_path":"/reg/cc-1.0.0/Cargo.toml","targets":[]},
    {"name":"tempfile","version":"3.0.0","id":"tempfile 3.0.0 (registry+x)","manifest_path":"/reg/tempfile-3.0.0/Cargo.toml","targets":[]}
  ],
  "workspace_members":["app 0.1.0 (path+file:///w/app)","util 0.1.0 (path+file:///w/util)"],
  "workspace_root":"/w",
  "target_directory":"/w/target",
  "resolve":{"nodes":[
    {"id":"app 0.1.0 (path+file:///w/app)","deps":[
      {"name":"util","pkg":"util 0.1.0 (path+file:///w/util)","dep_kinds":[{"kind":null,"target":null}]},
      {"name":"cc","pkg":"cc 1.0.0 (registry+x)","dep_kinds":[{"kind":"build","target":null}]},
      {"name":"tempfile","pkg":"tempfile 3.0.0 (registry+x)","dep_kinds":[{"kind":"dev","target":null}]}]},
    {"id":"util 0.1.0 (path+file:///w/util)","deps":[
      {"name":"serde","pkg":"serde 1.0.200 (registry+x)","dep_kinds":[{"kind":null,"target":null}]}]},
    {"id":"serde 1.0.200 (registry+x)","deps":[
      {"name":"serde_derive","pkg":"serde_derive 1.0.200 (registry+x)","dep_kinds":[{"kind":null,"target":null}]}]},
    {"id":"serde_derive 1.0.200 (registry+x)","deps":[]},
    {"id":"cc 1.0.0 (registry+x)","deps":[]},
    {"id":"tempfile 3.0.0 (registry+x)","deps":[]}
  ]}
}"#;

fn scope(
    model: &archipel_ide_projectmodel::ProjectModel,
    module: &str,
    part: &str,
) -> Option<DependencyScope> {
    model
        .module(module)?
        .order_entries
        .iter()
        .find_map(|e| match e {
            OrderEntry::Library { library, scope, .. } if library.contains(part) => Some(*scope),
            OrderEntry::Module { module, scope, .. } if module.contains(part) => Some(*scope),
            _ => None,
        })
}

#[test]
fn canned_metadata_becomes_modules_and_libraries() {
    let metadata = parse_metadata(METADATA).unwrap();
    let tree = to_data_nodes(Path::new("/w"), &metadata, None);
    let model = Arc::new(ProjectDataManager::new().import_data(&[tree]).model);
    let names: Vec<_> = model.modules.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["app", "util"]);
    let app = model.module("app").unwrap();
    assert_eq!(app.module_type, ModuleType::Rust);
    assert_eq!(app.language_level.as_deref(), Some("2021"));
    let folders: Vec<_> = app
        .source_folders()
        .map(|f| (f.root_type, f.path.display().to_string()))
        .collect();
    assert_eq!(
        folders,
        [
            (JpsModuleSourceRootType::Source, "/w/app/src".to_string()),
            (
                JpsModuleSourceRootType::TestSource,
                "/w/app/tests".to_string()
            ),
            (
                JpsModuleSourceRootType::TestSource,
                "/w/app/examples".to_string()
            ),
        ],
        "src/bin is inside src, build.rs is not a source folder"
    );
    assert!(
        app.excluded_folders()
            .any(|p| p == Path::new("/w/app/target"))
    );

    // Workspace member dependency, transitive registry crates with the scope of the edge kind.
    assert_eq!(scope(&model, "app", "util"), Some(DependencyScope::Compile));
    assert_eq!(
        scope(&model, "app", "serde:"),
        Some(DependencyScope::Compile)
    );
    assert_eq!(
        scope(&model, "app", "serde_derive"),
        Some(DependencyScope::Compile)
    );
    assert_eq!(scope(&model, "app", "cc:"), Some(DependencyScope::Provided));
    assert_eq!(
        scope(&model, "app", "tempfile"),
        Some(DependencyScope::Test)
    );
    let serde = model.library("Cargo: serde:1.0.200").unwrap();
    assert_eq!(serde.sources, [PathBuf::from("/reg/serde-1.0.200")]);
    assert!(serde.classes.is_empty());

    let index = model.file_index();
    assert!(index.is_in_test_source(Path::new("/w/app/tests/it.rs")));
    assert!(index.is_in_source_content(Path::new("/w/app/src/bin/tool.rs")));
    assert!(index.is_in_library_sources(Path::new("/reg/serde-1.0.200/src/lib.rs")));
    assert!(!index.is_in_library_classes(Path::new("/reg/serde-1.0.200/src/lib.rs")));
    assert!(index.is_excluded(Path::new("/w/app/target/debug/app")));
}

#[test]
fn rejects_garbage() {
    assert!(parse_metadata("not json").is_err());
}

#[test]
fn imports_a_real_workspace_with_path_dependencies() {
    if find_program("cargo").is_none() {
        eprintln!("skipping: cargo is not on PATH");
        return;
    }
    let root = std::env::temp_dir().join(format!("archipel-es-cargo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let write = |path: &str, text: &str| {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"app\", \"util\"]\nresolver = \"2\"\n",
    );
    write(
        "app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[dependencies]\nutil = { path = \"../util\" }\n[dev-dependencies]\nhelper = { path = \"../../helper-crate\" }\n",
    );
    write("app/src/main.rs", "fn main() { util::hi(); }\n");
    write("app/tests/it.rs", "#[test]\nfn t() {}\n");
    write(
        "util/Cargo.toml",
        "[package]\nname = \"util\"\nversion = \"0.2.0\"\nedition = \"2021\"\n",
    );
    write("util/src/lib.rs", "pub fn hi() {}\n");
    // A path dependency outside the workspace is a library, not a module.
    let helper = root.parent().unwrap().join("helper-crate");
    let _ = std::fs::remove_dir_all(&helper);
    std::fs::create_dir_all(helper.join("src")).unwrap();
    std::fs::write(
        helper.join("Cargo.toml"),
        "[package]\nname = \"helper\"\nversion = \"0.3.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(helper.join("src/lib.rs"), "").unwrap();

    let result = import_project(
        &ExternalSystemRegistry::with_defaults(),
        &ProjectDataManager::new(),
        &ResolveRequest::new(&root),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(result.systems, [ProjectSystemId::Cargo]);
    let model = Arc::new(result.model);
    let names: Vec<_> = model.modules.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["app", "util"]);
    assert_eq!(scope(&model, "app", "util"), Some(DependencyScope::Compile));
    assert_eq!(scope(&model, "app", "helper"), Some(DependencyScope::Test));
    let helper_lib = model.library("Cargo: helper:0.3.0").unwrap();
    assert!(helper_lib.sources[0].ends_with("helper-crate"));
    let index = model.file_index();
    let canonical = std::fs::canonicalize(&root).unwrap();
    assert!(index.is_in_test_source(&canonical.join("app/tests/it.rs")));
    assert_eq!(
        index
            .module_for_file(&canonical.join("util/src/lib.rs"))
            .unwrap()
            .name,
        "util"
    );
    assert!(index.is_excluded(&canonical.join("app/target/debug/x")));
    if let Some(sdk) = model.project_sdk.as_deref().and_then(|n| model.sdk(n)) {
        assert!(sdk.name.starts_with("Rust"));
    }
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&helper);
}

//! Java stub indexes on the project model: JAVA-STUB-011..018, JAVA-RES-026, 029, 030, 042,
//! JAVA-STUB-026 (dumb mode).

use archipel_ide_java::index::{JavaIndex, JavaIndexState};
use archipel_ide_java::scope::{ResolveScope, ScopeKind};
use archipel_ide_projectmodel::{
    DependencyScope, JpsModuleSourceRootType, Library, Module, OrderEntry, ProjectModel, Sdk, SourceFolder,
};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

fn write(p: &Path, text: &str) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

/// A jar with the given compiled classes (built with javac and `jar` via the zip crate).
fn make_jar(dir: &Path, name: &str, sources: &[(&str, &str)]) -> Option<PathBuf> {
    let work = dir.join(format!("{name}-work"));
    let mut args = vec!["-d".to_string(), work.join("out").display().to_string()];
    for (n, t) in sources {
        write(&work.join("src").join(n), t);
        args.push(work.join("src").join(n).display().to_string());
    }
    std::fs::create_dir_all(work.join("out")).unwrap();
    let ok = std::process::Command::new("javac")
        .args(&args)
        .env_remove("JAVA_TOOL_OPTIONS")
        .status()
        .ok()?
        .success();
    if !ok {
        return None;
    }
    let jar = dir.join(format!("{name}.jar"));
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
    for e in walkdir::WalkDir::new(work.join("out")).into_iter().filter_map(Result::ok) {
        if e.file_type().is_file() {
            let rel = e.path().strip_prefix(work.join("out")).unwrap().to_string_lossy().replace('\\', "/");
            zip.start_file(rel, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(&std::fs::read(e.path()).unwrap()).unwrap();
        }
    }
    zip.finish().unwrap();
    Some(jar)
}

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    model: Arc<ProjectModel>,
}

fn fixture() -> Option<Fixture> {
    let dir = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(dir.path()).unwrap();
    let main_lib = make_jar(&root, "main-lib", &[("lib/Util.java", "package lib; public class Util { public static int twice(int x){return 2*x;} }")])?;
    let test_lib = make_jar(&root, "test-lib", &[("tl/Check.java", "package tl; public class Check { public static void assertIt(Object o){} public static class Nested {} }")])?;
    write(&root.join("app/src/main/java/demo/Main.java"), "package demo; public class Main { public static void run() {} }");
    write(&root.join("app/src/test/java/demo/MainTest.java"), "package demo; public class MainTest {}");
    write(&root.join("core/src/main/java/core/Core.java"), "package core; public class Core {}");
    write(&root.join("core/src/test/java/core/CoreTest.java"), "package core; public class CoreTest {}");
    write(&root.join("app/scratch/Loose.java"), "package loose; public class Loose {}");
    let mut model = ProjectModel::new("demo", &root);
    let mut core = Module::new("core");
    core.content_entry_mut(root.join("core"))
        .add_source_folder(SourceFolder::new(root.join("core/src/main/java"), JpsModuleSourceRootType::Source))
        .add_source_folder(SourceFolder::new(root.join("core/src/test/java"), JpsModuleSourceRootType::TestSource));
    let mut app = Module::new("app");
    app.content_entry_mut(root.join("app"))
        .add_source_folder(SourceFolder::new(root.join("app/src/main/java"), JpsModuleSourceRootType::Source))
        .add_source_folder(SourceFolder::new(root.join("app/src/test/java"), JpsModuleSourceRootType::TestSource));
    app.add_order_entry(OrderEntry::module("core", DependencyScope::Compile));
    app.add_order_entry(OrderEntry::library("main-lib", DependencyScope::Compile));
    app.add_order_entry(OrderEntry::library("test-lib", DependencyScope::Test));
    model.put_library(Library::new("main-lib").with_classes(&main_lib));
    model.put_library(Library::new("test-lib").with_classes(&test_lib));
    model.modules.push(core);
    model.modules.push(app);
    let jdk = PathBuf::from("/usr/lib/jvm/java-21-openjdk-amd64");
    if jdk.join("lib/modules").exists() {
        model.put_sdk(Sdk::jdk_at(&jdk, Some("21")));
        model.project_sdk = Some("JDK 21".into());
    }
    Some(Fixture { _dir: dir, root, model: Arc::new(model) })
}

#[test]
fn indexes_modules_libraries_and_the_jdk_with_test_scoping() {
    let Some(fx) = fixture() else { return };
    let index = JavaIndex::in_memory();
    assert_eq!(index.state(), JavaIndexState::Absent);
    // Dumb mode: queries are unavailable, not empty (JAVA-STUB-026).
    assert!(index.find_class("demo.Main", &ResolveScope::default()).is_err());
    index.sync(fx.model.clone());
    assert!(index.wait_ready(Duration::from_secs(60)));
    let files = fx.model.file_index();
    let main = fx.root.join("app/src/main/java/demo/Main.java");
    let test = fx.root.join("app/src/test/java/demo/MainTest.java");
    let prod = ResolveScope::for_file(&index, &fx.model, &files, &main);
    let tests = ResolveScope::for_file(&index, &fx.model, &files, &test);
    // JAVA-RES-029: the test library is visible to test code only.
    assert!(index.find_class("tl.Check", &prod).unwrap().is_none());
    let check = index.find_class("tl.Check", &tests).unwrap().expect("test lib in test scope");
    assert!(check.class().compiled);
    assert!(index.find_class("lib.Util", &prod).unwrap().is_some());
    // Module sources: own tests only for test files; never a dependency's tests.
    assert!(index.find_class("demo.MainTest", &prod).unwrap().is_none());
    assert!(index.find_class("demo.MainTest", &tests).unwrap().is_some());
    assert!(index.find_class("core.Core", &prod).unwrap().is_some());
    assert!(index.find_class("core.CoreTest", &tests).unwrap().is_none());
    // Nested classes by FQN and by short name; method names; constructors are indexed too.
    let nested = index.find_class("tl.Check.Nested", &tests).unwrap().unwrap();
    assert_eq!(nested.class().name, "Nested");
    assert_eq!(index.classes_by_short_name("Nested", &tests).unwrap().len(), 1);
    let m = index.methods_named("assertIt", &tests).unwrap();
    assert_eq!(m.len(), 1);
    assert!(m[0].method().unwrap().flags & archipel_ide_java::stub::flags::STATIC != 0);
    assert!(index.methods_named("assertIt", &prod).unwrap().is_empty());
    assert!(!index.methods_named("Util", &prod).unwrap().is_empty(), "constructors are in the method-name index (JAVA-STUB-013)");
    // Packages (JAVA-RES-027), including parents.
    assert!(index.has_package("demo", &prod).unwrap());
    assert!(index.has_package("tl", &tests).unwrap() && !index.has_package("tl", &prod).unwrap());
    // JDK classes come from the image.
    if fx.model.sdks.is_empty() {
        return;
    }
    let list = index.find_class("java.util.List", &prod).unwrap().expect("List");
    assert!(list.class().compiled);
    assert!(index.find_class("java.lang.ProcessHandle", &prod).unwrap().is_some());
    assert!(index.has_package("java.util", &prod).unwrap() && index.has_package("java", &prod).unwrap());
    // Ordering across the classpath: JDK last. `List` in short names: java.util and java.awt both.
    let lists = index.classes_by_short_name("List", &prod).unwrap();
    assert!(lists.iter().any(|h| h.qname() == "java.util.List") && lists.iter().any(|h| h.qname() == "java.awt.List"));
    // Super-class index: short names of extends/implements, `Enum` for enums (JAVA-STUB-011).
    let enums = index.inheritors_of("Enum", &prod).unwrap();
    assert!(enums.iter().any(|h| h.qname() == "java.util.concurrent.TimeUnit"));
    // Deterministic order.
    assert_eq!(
        index.classes_by_short_name("List", &prod).unwrap().iter().map(|h| h.qname().to_string()).collect::<Vec<_>>(),
        lists.iter().map(|h| h.qname().to_string()).collect::<Vec<_>>()
    );
}

#[test]
fn files_outside_source_roots_resolve_only_themselves() {
    // JAVA-RES-030
    let Some(fx) = fixture() else { return };
    let index = JavaIndex::in_memory();
    index.sync_blocking(fx.model.clone());
    let files = fx.model.file_index();
    let loose = fx.root.join("app/scratch/Loose.java");
    let s = ResolveScope::for_file(&index, &fx.model, &files, &loose);
    assert_eq!(s.kind, ScopeKind::FileOnly);
    assert!(index.find_class("demo.Main", &s).unwrap().is_none());
    assert!(index.find_class("java.lang.String", &s).unwrap().is_none());
}

#[test]
fn updates_are_incremental_and_persistent() {
    // JAVA-STUB-001, 003, 018, JAVA-RES-042
    let Some(fx) = fixture() else { return };
    let store = tempfile::tempdir().unwrap();
    let files = fx.model.file_index();
    let main = fx.root.join("app/src/main/java/demo/Main.java");
    {
        let index = JavaIndex::open(Some(store.path()));
        assert!(index.is_persistent());
        index.sync_blocking(fx.model.clone());
        let scope = ResolveScope::for_file(&index, &fx.model, &files, &main);
        assert!(index.find_class("demo.Main", &scope).unwrap().is_some());
        let epoch = index.epoch();
        // A body edit changes nothing observable and a re-save of equal content reparses nothing.
        index.update_source(&main, "package demo; public class Main { public static void run() { int x = 1; } }", Some(("app", false)));
        let after_body = index.epoch();
        assert!(after_body > epoch);
        index.update_source(&main, "package demo; public class Main { public static void run() { int x = 1; } }", Some(("app", false)));
        assert_eq!(index.epoch(), after_body, "equal content is not reindexed");
        // A new method shows up by name.
        index.update_source(&main, "package demo; public class Main { public static void run() {} public static void extra() {} }", Some(("app", false)));
        assert_eq!(index.methods_named("extra", &scope).unwrap().len(), 1);
    }
    // Restart: the indexes are complete on disk, so queries work at once (no dumb mode) ...
    let index = JavaIndex::open(Some(store.path()));
    assert!(index.is_ready(), "a complete persisted index is ready immediately");
    // ... and a model sync only touches what changed on disk.
    write(&fx.root.join("app/src/main/java/demo/Added.java"), "package demo; public class Added {}");
    index.sync_blocking(fx.model.clone());
    let scope = ResolveScope::for_file(&index, &fx.model, &files, &main);
    assert!(index.find_class("demo.Added", &scope).unwrap().is_some());
    assert!(index.find_class("lib.Util", &scope).unwrap().is_some());
    // The stale unsaved text of Main was replaced by the file on disk.
    assert!(index.methods_named("extra", &scope).unwrap().is_empty());
    // Deleting a file removes its entries.
    std::fs::remove_file(fx.root.join("app/src/main/java/demo/Added.java")).unwrap();
    index.sync_blocking(fx.model.clone());
    assert!(index.find_class("demo.Added", &scope).unwrap().is_none());
}

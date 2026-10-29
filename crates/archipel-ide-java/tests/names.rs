//! M1 of `docs/architecture/java-engine-plan.md`: the classpath snapshot per module and scope,
//! the ordered short-name index with kinds, flags and annotation targets, package listing, the
//! JDK (jimage and a JDK 8 `rt.jar`), the shared library cache and the ad-hoc module. All
//! libraries are our own hermetic fixture jars (`tests/fixtures`, built with the JDK).

mod common;

use archipel_ide_java::classpath::{Completeness, UnitRole, ad_hoc_model};
use archipel_ide_java::index::JavaIndex;
use archipel_ide_java::names::{
    MatchClass, NameEntry, is_discouraged, is_internal_package, target,
};
use archipel_ide_java::scope::{ResolveScope, ScopeKind};
use archipel_ide_java::stub::ClassKind;
use archipel_ide_projectmodel::{
    DependencyScope, JpsModuleSourceRootType, Library, Module, OrderEntry, ProjectModel, Sdk,
    SourceFolder,
};
use common::*;
use std::path::PathBuf;
use std::sync::Arc;

fn any(_: &NameEntry) -> bool {
    true
}

fn fqns(hits: &[archipel_ide_java::index::NameHit]) -> Vec<String> {
    hits.iter().map(|h| h.entry.fqn.to_string()).collect()
}

fn junit_project() -> Proj {
    project(
        &[
            (
                "src/main/java/p/App.java",
                "package p;\npublic class App {}\n",
            ),
            (
                "src/test/java/p/AppTest.java",
                "package p;\nclass AppTest {}\n",
            ),
        ],
        &[("junit5", fixture_jar("junit5"), DependencyScope::Test)],
    )
}

/// JAVA-STUB-011, 017: prefix and camel-hump queries over the short names, in the scope of the
/// file; `ArLi` finds `ArrayList`, `Te` finds the JUnit `Test` in test code only.
#[test]
fn java_stub_011_prefix_and_camel_hump_queries() {
    let p = junit_project();
    let test_scope = p.project.scope_for(&p.path("src/test/java/p/AppTest.java"));
    let prod_scope = p.project.scope_for(&p.path("src/main/java/p/App.java"));
    let index = &p.project.index;

    let hits = index.names_matching(&test_scope, "ArLi", &any, 50).unwrap();
    let names = fqns(&hits);
    assert!(
        names.contains(&"java.util.ArrayList".to_string()),
        "{names:?}"
    );
    assert!(
        hits.iter()
            .find(|h| &*h.entry.fqn == "java.util.ArrayList")
            .unwrap()
            .class
            == MatchClass::Hump
    );

    let te = fqns(&index.names_matching(&test_scope, "Te", &any, 0).unwrap());
    assert!(
        te.contains(&"org.junit.jupiter.api.Test".to_string()),
        "{te:?}"
    );
    let te_prod = fqns(&index.names_matching(&prod_scope, "Te", &any, 0).unwrap());
    assert!(!te_prod.contains(&"org.junit.jupiter.api.Test".to_string()));

    // Exact and prefix matches come before humps; one hit per FQN.
    let list = index.names_matching(&test_scope, "List", &any, 0).unwrap();
    assert_eq!(list[0].class, MatchClass::Exact);
    let all = fqns(&list);
    let mut dedup = all.clone();
    dedup.sort();
    dedup.dedup();
    assert_eq!(all.len(), dedup.len(), "one hit per FQN");

    let sb = fqns(&index.names_matching(&test_scope, "sb", &any, 0).unwrap());
    assert!(
        sb.contains(&"java.lang.StringBuilder".to_string()),
        "{sb:?}"
    );
    let npe = fqns(&index.names_matching(&test_scope, "NPE", &any, 0).unwrap());
    assert!(
        npe.contains(&"java.lang.NullPointerException".to_string()),
        "{npe:?}"
    );
}

/// JAVA-STUB-011: the summary of a name carries kind, flags, deprecation and annotation
/// targets, so position filters need no stub load.
#[test]
fn java_stub_011_summary_has_kind_flags_and_targets() {
    let p = junit_project();
    let scope = p.project.scope_for(&p.path("src/test/java/p/AppTest.java"));
    let index = &p.project.index;
    let one = |name: &str, fqn: &str| {
        index
            .names_matching(&scope, name, &any, 0)
            .unwrap()
            .into_iter()
            .find(|h| &*h.entry.fqn == fqn)
            .unwrap_or_else(|| panic!("{fqn} not found"))
            .entry
    };
    assert_eq!(
        one("Runnable", "java.lang.Runnable").kind,
        ClassKind::Interface
    );
    assert_eq!(one("Thread", "java.lang.Thread").kind, ClassKind::Class);
    assert_eq!(
        one("TimeUnit", "java.util.concurrent.TimeUnit").kind,
        ClassKind::Enum
    );
    let state = one("State", "java.lang.Thread.State");
    assert!(state.is_nested());
    assert_eq!(state.kind, ClassKind::Enum);
    assert_eq!(state.qualified_name(), "Thread.State");
    assert_eq!(state.top_fqn(), "java.lang.Thread");
    assert!(one("Observable", "java.util.Observable").deprecated);
    assert!(!one("Objects", "java.util.Objects").deprecated);
    assert!(one("Objects", "java.util.Objects").is_public());
    assert!(one("AbstractList", "java.util.AbstractList").is_abstract());

    let test = one("Test", "org.junit.jupiter.api.Test");
    assert_eq!(test.kind, ClassKind::Annotation);
    assert_eq!(test.targets, Some(target::ANNOTATION_TYPE | target::METHOD));
    let display = one("DisplayName", "org.junit.jupiter.api.DisplayName");
    assert_eq!(display.targets, Some(target::TYPE | target::METHOD));
    // `@Deprecated` of the JDK: a class-file `@Target` is read too.
    let deprecated = one("Deprecated", "java.lang.Deprecated");
    assert_eq!(deprecated.kind, ClassKind::Annotation);
    assert!(
        deprecated
            .targets
            .is_some_and(|t| t & target::METHOD != 0 && t & target::TYPE != 0)
    );
    // An annotation without `@Target` has none recorded.
    let functional = one("FunctionalInterface", "java.lang.FunctionalInterface");
    assert_eq!(functional.targets, Some(target::TYPE));
}

/// JAVA-RES-027: classes of a package and its child packages.
#[test]
fn java_res_027_package_listing() {
    let p = junit_project();
    let scope = p.project.scope_for(&p.path("src/test/java/p/AppTest.java"));
    let index = &p.project.index;
    let classes: Vec<String> = index
        .classes_in_package("java.util.concurrent", &scope)
        .unwrap()
        .iter()
        .map(|e| e.fqn.to_string())
        .collect();
    assert!(classes.contains(&"java.util.concurrent.ConcurrentHashMap".to_string()));
    assert!(
        classes
            .iter()
            .all(|c| c.starts_with("java.util.concurrent."))
    );
    let subs = index.subpackages("java.util", &scope).unwrap();
    for expected in ["concurrent", "function", "stream"] {
        assert!(subs.contains(&expected.to_string()), "{subs:?}");
    }
    assert!(!subs.iter().any(|s| s.contains('.')));
    let top = index.subpackages("", &scope).unwrap();
    assert!(
        top.contains(&"java".to_string()) && top.contains(&"org".to_string()),
        "{top:?}"
    );
    let jupiter = index
        .classes_in_package("org.junit.jupiter.api", &scope)
        .unwrap();
    assert!(
        jupiter
            .iter()
            .any(|e| &*e.fqn == "org.junit.jupiter.api.Test")
    );
    // The package of the test library is not visible from production code.
    let prod = p.project.scope_for(&p.path("src/main/java/p/App.java"));
    assert!(
        index
            .classes_in_package("org.junit.jupiter.api", &prod)
            .unwrap()
            .is_empty()
    );
    assert!(!index.has_package("org.junit", &prod).unwrap());
}

/// `Throwable` subtypes by short names, transitively (the candidate set of `throws`/`catch`).
#[test]
fn java_cmp_009_throwable_closure_follows_inheritors() {
    let p = junit_project();
    let scope = p.project.scope_for(&p.path("src/main/java/p/App.java"));
    let set = p
        .project
        .index
        .inheritor_closure("Throwable", &scope)
        .unwrap();
    for expected in [
        "java.lang.Exception",
        "java.io.IOException",
        "java.io.FileNotFoundException",
        "java.lang.IllegalArgumentException",
        "java.lang.Error",
    ] {
        assert!(set.contains(expected), "{expected}");
    }
    assert!(!set.contains("java.lang.String"));
}

/// JAVA-CMP-025: platform-internal packages are recognized, discouraged ones too.
#[test]
fn java_cmp_025_internal_and_discouraged_packages() {
    assert!(is_internal_package("sun.misc"));
    assert!(is_internal_package("com.sun.net.httpserver"));
    assert!(is_internal_package("jdk.internal.misc"));
    assert!(!is_internal_package("java.sql"));
    assert!(!is_internal_package("com.sunrise.api"));
    assert!(is_discouraged("java.awt", "java.awt.List"));
    assert!(is_discouraged("org.omg.CORBA", "org.omg.CORBA.Any"));
    assert!(!is_discouraged("java.util", "java.util.List"));
    // The JDK image indexes the exported `sun.misc`: the contributor hides it, the index has it.
    let p = junit_project();
    let scope = p.project.scope_for(&p.path("src/main/java/p/App.java"));
    let hits = p
        .project
        .index
        .names_matching(&scope, "Unsafe", &any, 0)
        .unwrap();
    assert!(fqns(&hits).contains(&"sun.misc.Unsafe".to_string()));
}

/// JAVA-RES-036: a JDK 8 keeps its classes in `rt.jar`; internal packages sit next to the API.
#[test]
fn java_res_036_jdk_8_rt_jar_is_indexed() {
    let dir = tempfile::tempdir().unwrap();
    let home = std::fs::canonicalize(dir.path()).unwrap().join("jdk8");
    std::fs::create_dir_all(home.join("jre/lib")).unwrap();
    std::fs::copy(fixture_jar("legacy-rt"), home.join("jre/lib/rt.jar")).unwrap();
    let sdk = Sdk::jdk_at(&home, Some("8"));
    assert!(
        sdk.classes.iter().any(|c| c.ends_with("rt.jar")),
        "{:?}",
        sdk.classes
    );

    let root = home.parent().unwrap().join("proj");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/A.java"), "class A {}\n").unwrap();
    let mut model = ProjectModel::new("demo", &root);
    let mut app = Module::new("app");
    app.content_entry_mut(&root)
        .add_source_folder(SourceFolder::new(
            root.join("src"),
            JpsModuleSourceRootType::Source,
        ));
    model.modules.push(app);
    model.project_sdk = Some(sdk.name.clone());
    model.put_sdk(sdk);
    let model = Arc::new(model);
    let index = JavaIndex::in_memory();
    index.sync_blocking(model.clone());
    let project = archipel_ide_java::api::JavaProject::new(index, model, false);
    let scope = project.scope_for(&root.join("src/A.java"));
    let names = fqns(&project.index.names_matching(&scope, "", &any, 0).unwrap());
    for expected in [
        "legacy.util.Seq",
        "sun.misc.Unsafe",
        "com.sun.tools.Launcher",
    ] {
        assert!(names.contains(&expected.to_string()), "{names:?}");
    }
}

/// PLAT-IDX-034: two projects that use the same jar share its stubs through the library cache.
#[test]
fn plat_idx_034_shared_library_cache() {
    let jar = fixture_jar("hamcrest");
    let cache = tempfile::tempdir().unwrap();
    let model = |name: &str| {
        let dir = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/A.java"), "class A {}\n").unwrap();
        let mut model = ProjectModel::new(name, &root);
        let mut app = Module::new("app");
        app.content_entry_mut(&root)
            .add_source_folder(SourceFolder::new(
                root.join("src"),
                JpsModuleSourceRootType::Source,
            ));
        model.put_library(Library::new("hamcrest").with_classes(&jar));
        app.add_order_entry(OrderEntry::library("hamcrest", DependencyScope::Compile));
        model.modules.push(app);
        (dir, Arc::new(model))
    };
    let (_d1, m1) = model("one");
    let first = JavaIndex::in_memory();
    first.set_shared_cache(Some(cache.path().to_path_buf()));
    first.sync_blocking(m1);
    assert_eq!(first.shared_cache_hits(), 0);
    let (_d2, m2) = model("two");
    let second = JavaIndex::in_memory();
    second.set_shared_cache(Some(cache.path().to_path_buf()));
    second.sync_blocking(m2.clone());
    assert_eq!(second.shared_cache_hits(), 1, "the jar came from the cache");
    // The cached stubs answer like the read ones.
    let project = archipel_ide_java::api::JavaProject::new(second, m2.clone(), false);
    let a = m2.modules[0]
        .content_roots()
        .next()
        .unwrap()
        .join("src/A.java");
    let scope = project.scope_for(&a);
    let names = fqns(
        &project
            .index
            .names_matching(&scope, "Matcher", &any, 0)
            .unwrap(),
    );
    assert!(
        names.contains(&"org.hamcrest.Matcher".to_string()),
        "{names:?}"
    );
    assert!(names.contains(&"org.hamcrest.MatcherAssert".to_string()));
}

/// JAVA-STUB-018: after an incremental update the name table equals the one of a fresh index.
#[test]
fn java_stub_018_incremental_update_equals_fresh_index() {
    let v1 =
        "package p;\npublic class Widget { public static class Part {} }\ninterface Hidden {}\n";
    let v2 = "package p;\npublic class Widget {}\npublic @interface Marker {}\nenum Mode { A }\n";
    let path = "src/main/java/p/Widget.java";
    let edited = project(&[(path, v1)], &[]);
    edited
        .project
        .index
        .update_source(&edited.path(path), v2, Some(("app", false)));
    let fresh = project(&[(path, v2)], &[]);
    let names = |p: &Proj| {
        let scope = p.project.scope_for(&p.path(path));
        let mut out: Vec<(String, ClassKind, u32)> = p
            .project
            .index
            .names_matching(&scope, "", &|e| e.package() == "p", 0)
            .unwrap()
            .into_iter()
            .map(|h| (h.entry.fqn.to_string(), h.entry.kind, h.entry.flags))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    };
    assert_eq!(names(&edited), names(&fresh));
    assert!(
        names(&fresh)
            .iter()
            .any(|(n, k, _)| n == "p.Marker" && *k == ClassKind::Annotation)
    );
    assert!(
        !names(&edited)
            .iter()
            .any(|(n, _, _)| n == "p.Hidden" || n == "p.Widget.Part")
    );
    // A removed file leaves no names.
    edited.project.index.remove_source(&edited.path(path));
    assert!(names(&edited).is_empty());
}

/// PLAT-IDX-030..036 (warm start): a persisted index answers after a restart without rereading
/// the unchanged units, names included.
#[test]
fn plat_idx_warm_start_keeps_the_names() {
    let p = junit_project();
    let cache = tempfile::tempdir().unwrap();
    {
        let index = JavaIndex::open(Some(cache.path()));
        index.sync_blocking(p.project.model.clone());
        let scope = ResolveScope::for_file(
            &index,
            &p.project.model,
            &p.project.files,
            &p.path("src/test/java/p/AppTest.java"),
        );
        assert!(
            !index
                .names_matching(&scope, "Te", &any, 0)
                .unwrap()
                .is_empty()
        );
    }
    let index = JavaIndex::open(Some(cache.path()));
    index.sync_blocking(p.project.model.clone());
    let scope = ResolveScope::for_file(
        &index,
        &p.project.model,
        &p.project.files,
        &p.path("src/test/java/p/AppTest.java"),
    );
    let te = fqns(&index.names_matching(&scope, "Te", &any, 0).unwrap());
    assert!(
        te.contains(&"org.junit.jupiter.api.Test".to_string()),
        "{te:?}"
    );
    assert!(te.contains(&"p.AppTest".to_string()), "{te:?}");
}

/// PLAT-SCOPE-062, JAVA-RES-029, 030: the classpath snapshot of a module and scope lists its
/// units in classpath order with their roles; tests only for test code.
#[test]
fn plat_scope_062_classpath_snapshot_per_module_and_scope() {
    let p = junit_project();
    let test = p
        .project
        .classpath_for(&p.path("src/test/java/p/AppTest.java"));
    let prod = p.project.classpath_for(&p.path("src/main/java/p/App.java"));
    assert_eq!(test.module.as_deref(), Some("app"));
    assert!(test.test && !prod.test);
    assert!(test.libraries().any(|l| l == "junit5"));
    assert!(!prod.libraries().any(|l| l == "junit5"));
    // Own sources first, then libraries, then the SDK.
    let roles: Vec<&UnitRole> = test.units.iter().map(|u| &u.role).collect();
    let first_lib = roles
        .iter()
        .position(|r| matches!(r, UnitRole::Library(_)))
        .unwrap();
    let first_sdk = roles
        .iter()
        .position(|r| matches!(r, UnitRole::Sdk(_)))
        .unwrap();
    assert!(matches!(roles[0], UnitRole::Source { test: false, .. }));
    assert!(first_lib < first_sdk);
    assert!(
        test.units
            .iter()
            .any(|u| matches!(u.role, UnitRole::Source { test: true, .. }))
    );
    assert!(
        !prod
            .units
            .iter()
            .any(|u| matches!(u.role, UnitRole::Source { test: true, .. }))
    );
    // The scope is a view over the snapshot.
    assert_eq!(
        test.scope(),
        p.project.scope_for(&p.path("src/test/java/p/AppTest.java"))
    );
    assert!(test.completeness.is_complete());
}

/// JAVA-ERR-003, project-model gaps: the completeness flag says why a jar is missing.
#[test]
fn java_err_003_classpath_completeness_reasons() {
    let p = junit_project();
    let file = p.path("src/test/java/p/AppTest.java");
    let with = |loading: bool, notes: Vec<String>| {
        archipel_ide_java::api::JavaProject::new(
            p.project.index.clone(),
            p.project.model.clone(),
            loading,
        )
        .with_import_notes(notes)
    };
    assert_eq!(
        with(false, vec![]).classpath_for(&file).completeness,
        Completeness::Complete
    );
    assert_eq!(
        with(true, vec![]).classpath_for(&file).completeness,
        Completeness::Importing
    );
    let failed = with(
        false,
        vec!["Maven: dependencies were not resolved (offline)".into()],
    );
    let c = failed.classpath_for(&file).completeness;
    assert!(matches!(&c, Completeness::ImportFailed(n) if n.len() == 1));
    assert!(
        c.describe()
            .unwrap()
            .contains("dependencies were not resolved")
    );
    assert_eq!(failed.completeness(), c);

    // A library whose jar is not on disk.
    let mut model = (*p.project.model).clone();
    model.put_library(Library::new("ghost").with_classes("/nonexistent/ghost.jar"));
    model.modules[0].add_order_entry(OrderEntry::library("ghost", DependencyScope::Compile));
    let model = Arc::new(model);
    let ghost = archipel_ide_java::api::JavaProject::new(p.project.index.clone(), model, false);
    match ghost.classpath_for(&file).completeness {
        Completeness::MissingLibraries(names) => assert_eq!(names, ["ghost"]),
        other => panic!("{other:?}"),
    }

    // A module imported from a build tool without any library.
    let mut model = ProjectModel::new("bare", p.root.clone());
    let mut m = Module::new("bare");
    m.external_project = Some(archipel_ide_projectmodel::ExternalProject {
        system: "maven".into(),
        id: "g:bare".into(),
        path: p.root.clone(),
    });
    m.content_entry_mut(&p.root)
        .add_source_folder(SourceFolder::new(
            p.root.join("src/main/java"),
            JpsModuleSourceRootType::Source,
        ));
    model.modules.push(m);
    let bare =
        archipel_ide_java::api::JavaProject::new(p.project.index.clone(), Arc::new(model), false);
    assert_eq!(bare.completeness(), Completeness::NoLibraries);
    assert!(
        bare.completeness()
            .describe()
            .unwrap()
            .contains("no libraries")
    );
}

/// JAVA-RES-030 and the ad-hoc module of M1: a file outside every module resolves against the
/// JDK and itself instead of silently getting no analysis.
#[test]
fn java_res_030_ad_hoc_module_for_files_outside_every_module() {
    let dir = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(dir.path()).unwrap();
    std::fs::write(
        root.join("Scratch.java"),
        "class Scratch { java.util.List<String> xs; }\n",
    )
    .unwrap();
    let model = ad_hoc_model(&root).expect("a JDK on this machine");
    let model = Arc::new(model);
    let index = JavaIndex::in_memory();
    index.sync_blocking(model.clone());
    let project = archipel_ide_java::api::JavaProject::new(index, model, false);
    let scratch = root.join("Scratch.java");
    assert!(project.applies_to(&scratch));
    // A file of another directory (outside the ad-hoc content root) still gets the JDK.
    let elsewhere = PathBuf::from("/somewhere/else/Other.java");
    assert!(project.applies_to(&elsewhere));
    let snapshot = project.classpath_for(&elsewhere);
    assert_eq!(snapshot.kind, ScopeKind::AdHoc);
    assert!(snapshot.module.is_none());
    assert!(
        snapshot
            .units
            .iter()
            .any(|u| matches!(u.role, UnitRole::Sdk(_)))
    );
    let scope = project.scope_for(&elsewhere);
    let names = fqns(
        &project
            .index
            .names_matching(&scope, "ArrayList", &any, 0)
            .unwrap(),
    );
    assert!(names.contains(&"java.util.ArrayList".to_string()));
    // The analysis runs: a missing class is an error, JDK classes resolve.
    let a = project.analyze_text(
        &scratch,
        "class Scratch { java.util.List<String> xs; Missing m; }\n",
    );
    assert!(
        a.problems.iter().any(|p| p.message.contains("Missing")),
        "{:?}",
        a.problems
    );
}

/// The scan looks at the first letter of a name before it runs the matcher and only scans the
/// rest for later-hump matches when the first pass did not fill the limit: with no limit the
/// answer is exactly the matcher's over every name, whatever the pattern starts with.
#[test]
fn the_first_letter_scan_finds_what_the_plain_matcher_finds() {
    use archipel_ide_java::names::NameMatcher;
    let p = junit_project();
    let scope = p.project.scope_for(&p.path("src/test/java/p/AppTest.java"));
    let index = &p.project.index;
    let all = index.names_matching(&scope, "", &any, 0).unwrap();
    for pattern in [
        "Arr", "arr", "ArLi", "Builder", "bu", "Test", "IOE", "_", "x", "StrBu", "E",
    ] {
        let m = NameMatcher::new(pattern);
        let mut want: Vec<String> = all
            .iter()
            .filter(|h| m.classify(h.entry.name()).is_some())
            .map(|h| h.entry.fqn.to_string())
            .collect();
        let mut got = fqns(&index.names_matching(&scope, pattern, &any, 0).unwrap());
        want.sort();
        got.sort();
        assert_eq!(got, want, "pattern `{pattern}`");
    }
    // With a limit the best matches are the same: later-hump matches only fill what is left.
    let limited = fqns(&index.names_matching(&scope, "Arr", &any, 5).unwrap());
    assert_eq!(limited.len(), 5);
    let m = NameMatcher::new("Arr");
    assert!(limited.iter().all(|f| {
        let name = f.rsplit('.').next().unwrap();
        m.classify(name).is_some()
    }));
    assert!(
        limited
            .iter()
            .take(3)
            .all(|f| f.rsplit('.').next().unwrap().starts_with("Arr")),
        "{limited:?}"
    );
}

/// JAVA-STUB-014: the static member name index lists non-private static methods, fields and
/// enum constants by name; instance members and constructors are not in it.
#[test]
fn java_stub_014_the_static_member_name_index() {
    use archipel_ide_java::names::NameMatcher;
    let p = junit_project();
    let scope = p.project.scope_for(&p.path("src/test/java/p/AppTest.java"));
    let index = &p.project.index;
    let hits = index
        .static_members_matching(&scope, &NameMatcher::new("assertTr"), 10, 50)
        .unwrap();
    assert!(
        hits.iter().any(|h| {
            !h.field
                && h.class.class().qname == "org.junit.jupiter.api.Assertions"
                && h.class.class().methods[h.index].name == "assertTrue"
        }),
        "{} hits",
        hits.len()
    );
    // `size` is an instance method everywhere in the JDK collections: no static `List.size`.
    let hits = index
        .static_members_matching(&scope, &NameMatcher::new("size"), 5, 200)
        .unwrap();
    assert!(
        hits.iter()
            .all(|h| h.field || h.class.class().methods[h.index].flags & 8 != 0),
        "only static members are indexed"
    );
    // Enum constants and interface fields count as static fields.
    let hits = index
        .static_members_matching(&scope, &NameMatcher::new("MAX_VALUE"), 5, 50)
        .unwrap();
    assert!(
        hits.iter()
            .any(|h| h.field && h.class.class().qname == "java.lang.Integer"),
        "Integer.MAX_VALUE"
    );
}

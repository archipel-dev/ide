//! Language pack lifecycle and conformance (PLAT-LANG-005, -011, -104): installing registers
//! extensions on the points, uninstalling removes exactly the pack's contributions, the
//! hosts see the difference through the rebuilt registries and nothing fails to compile or run
//! when a pack is missing.

use archipel_ide_analysis::daemon::{Pass, PassCx, PassId, PassOutput};
use archipel_ide_analysis::inspect::InspectionSuppressor;
use archipel_ide_analysis::langext::{
    self, FILE_TYPE, HIGHLIGHTING_PASS, IMPORT_FIX_PROVIDER, LANGUAGE, PARSER_DEFINITION,
    ParserDefinition, registry_from_extensions,
};
use archipel_ide_analysis::{Cancelled, ServiceRegistry};
use archipel_ide_langpacks::{
    LANGUAGE_ACTIVITY, LanguagePack, LanguagePacks, PackContext, PackError,
};
use archipel_ide_language::{LanguageId, Order, Registration};
use archipel_ide_lsp::{LANGUAGE_SERVER_PROVIDER, ServerRegistry};
use archipel_ide_platform::Application;
use archipel_ide_toolchains::providers::{
    TOOLCHAIN_REQUIREMENT_PROVIDER, project_requirements, requirements_from,
};
use std::path::Path;
use std::sync::Arc;

const JAVA: LanguageId = LanguageId("java");

fn setup() -> (Application, LanguagePacks) {
    let app = Application::new();
    let packs = LanguagePacks::new(&app);
    let errors = packs.install_builtin();
    assert!(errors.is_empty(), "{errors:?}");
    (app, packs)
}

fn pass_ids(registry: &ServiceRegistry, language: LanguageId) -> Vec<&'static str> {
    registry
        .entries::<dyn Pass>(language)
        .into_iter()
        .map(|(registration, _)| registration.id)
        .collect()
}

#[test]
fn the_builtin_packs_register_their_extensions() {
    let (app, packs) = setup();
    assert_eq!(
        packs.installed(),
        [
            "java", "rust", "bazel", "kotlin", "toml", "markdown", "catalog"
        ]
    );
    let ext = app.extensions();
    let registry = registry_from_extensions(ext, ServiceRegistry::new());
    // PLAT-LANG-050: the native resolve pass of Java, and only for Java.
    assert_eq!(pass_ids(&registry, JAVA), ["java-resolve"]);
    assert!(pass_ids(&registry, LanguageId("rust")).is_empty());
    assert_eq!(
        registry
            .entries::<dyn InspectionSuppressor>(JAVA)
            .iter()
            .map(|(r, _)| r.id)
            .collect::<Vec<_>>(),
        ["java-suppress-warnings"]
    );
    // The data query of spec 08 section 6: which services does a language have.
    let java: Vec<_> = packs
        .services_of(JAVA)
        .iter()
        .map(|s| (s.point, s.id))
        .collect();
    assert!(java.contains(&("dev.archipel.language.highlightingPass", "java-resolve")));
    assert!(java.contains(&("dev.archipel.language.parserDefinition", "tree-sitter")));
    // Languages, file types, servers, requirements, activity and import fixes.
    let languages = ext.ids(LANGUAGE);
    for id in [
        "java",
        "rust",
        "kotlin",
        "toml",
        "markdown",
        "markdown_inline",
        "starlark",
    ] {
        assert!(languages.iter().any(|l| l == id), "{id}");
    }
    assert_eq!(
        langext::language_for_path(ext, Path::new("src/App.java")),
        Some(JAVA)
    );
    assert_eq!(
        langext::language_for_path(ext, Path::new("pkg/BUILD")),
        Some(LanguageId("starlark"))
    );
    assert_eq!(ext.ids(LANGUAGE_ACTIVITY), ["java"]);
    assert_eq!(ext.ids(IMPORT_FIX_PROVIDER), ["java"]);
    assert_eq!(ext.ids(TOOLCHAIN_REQUIREMENT_PROVIDER).len(), 4);
}

#[test]
fn removing_the_java_pack_removes_every_java_feature() {
    let (app, packs) = setup();
    let ext = app.extensions();
    assert!(packs.uninstall("java"));
    assert!(!packs.uninstall("java"));
    assert!(!packs.is_installed("java"));
    let registry = registry_from_extensions(ext, ServiceRegistry::new());
    assert!(pass_ids(&registry, JAVA).is_empty());
    assert!(registry.all::<dyn InspectionSuppressor>(JAVA).is_empty());
    assert!(packs.services_of(JAVA).is_empty());
    assert!(!ext.ids(LANGUAGE).iter().any(|l| l == "java"));
    assert_eq!(langext::language_for_path(ext, Path::new("App.java")), None);
    assert!(ext.ids(LANGUAGE_ACTIVITY).is_empty());
    assert!(ext.ids(IMPORT_FIX_PROVIDER).is_empty());
    // No jdtls, no Java requirement.
    let servers = ServerRegistry::from_extensions(ext);
    assert!(servers.for_path(Path::new("App.java")).is_none());
    assert!(servers.for_path(Path::new("lib.rs")).is_some());
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("build.gradle"), "").unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
    let requirements = requirements_from(
        &ext.extension_list(TOOLCHAIN_REQUIREMENT_PROVIDER),
        dir.path(),
    );
    assert_eq!(
        requirements.iter().map(|r| r.language).collect::<Vec<_>>(),
        ["Rust"]
    );
    // Other languages are untouched.
    assert_eq!(
        langext::language_for_path(ext, Path::new("main.rs")),
        Some(LanguageId("rust"))
    );
    // And the pack can come back.
    packs
        .install(&archipel_ide_langpacks::packs::java::JavaPack::new())
        .unwrap();
    assert_eq!(
        pass_ids(&registry_from_extensions(ext, ServiceRegistry::new()), JAVA),
        ["java-resolve"]
    );
}

#[test]
fn disposing_the_application_removes_all_packs() {
    let (app, _packs) = setup();
    app.dispose();
    let ext = app.extensions();
    assert!(ext.ids(LANGUAGE).is_empty());
    assert!(ext.ids(FILE_TYPE).is_empty());
    assert!(ext.ids(LANGUAGE_SERVER_PROVIDER).is_empty());
}

#[test]
fn a_pack_cannot_be_installed_twice_and_duplicates_roll_back() {
    let (_app, packs) = setup();
    let java = archipel_ide_langpacks::packs::java::JavaPack::new();
    assert_eq!(
        packs.install(&java),
        Err(PackError::AlreadyInstalled("java"))
    );
    // A pack that declares a language that already exists is rejected and leaves nothing behind.
    struct Clash;
    impl LanguagePack for Clash {
        fn id(&self) -> &'static str {
            "clash"
        }
        fn display_name(&self) -> &'static str {
            "Clash"
        }
        fn register(&self, cx: &mut PackContext<'_>) {
            cx.language(langext::Language::new("kotlin", "Kotlin again"));
        }
    }
    assert!(matches!(
        packs.install(&Clash),
        Err(PackError::Extension { pack: "clash", .. })
    ));
    assert!(!packs.is_installed("clash"));
    // And an unknown grammar is an error, not a panic.
    struct Unknown;
    impl LanguagePack for Unknown {
        fn id(&self) -> &'static str {
            "unknown"
        }
        fn display_name(&self) -> &'static str {
            "Unknown"
        }
        fn register(&self, cx: &mut PackContext<'_>) {
            cx.tree_sitter_language("cobol", |l| l);
        }
    }
    assert!(matches!(
        packs.install(&Unknown),
        Err(PackError::UnknownLanguage { .. })
    ));
}

struct Marker(&'static str);
impl Pass for Marker {
    fn id(&self) -> PassId {
        PassId(self.0)
    }
    fn after(&self) -> &[PassId] {
        &[]
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn visible_range_first(&self) -> bool {
        false
    }
    fn collect(&self, _: &PassCx<'_>) -> Result<PassOutput, Cancelled> {
        Err(Cancelled)
    }
}

#[test]
fn loading_order_places_another_packs_extensions_around_the_javas() {
    let (app, packs) = setup();
    struct Around;
    impl LanguagePack for Around {
        fn id(&self) -> &'static str {
            "around"
        }
        fn display_name(&self) -> &'static str {
            "Around"
        }
        fn register(&self, cx: &mut PackContext<'_>) {
            cx.language_extension(
                HIGHLIGHTING_PASS,
                Registration::new("last", JAVA).order(Order::Last),
                Arc::new(Marker("last")),
            );
            cx.language_extension(
                HIGHLIGHTING_PASS,
                Registration::new("before-resolve", JAVA).order(Order::Before("java-resolve")),
                Arc::new(Marker("before")),
            );
            cx.language_extension(
                HIGHLIGHTING_PASS,
                Registration::any("first").order(Order::First),
                Arc::new(Marker("first")),
            );
        }
    }
    packs.install(&Around).unwrap();
    let registry = registry_from_extensions(app.extensions(), ServiceRegistry::new());
    assert_eq!(
        pass_ids(&registry, JAVA),
        ["first", "before-resolve", "java-resolve", "last"]
    );
    // Uninstalling the other pack takes its passes back and keeps Java's.
    assert!(packs.uninstall("around"));
    let registry = registry_from_extensions(app.extensions(), ServiceRegistry::new());
    assert_eq!(pass_ids(&registry, JAVA), ["java-resolve"]);
}

#[test]
fn every_language_of_a_pack_has_a_parser_that_parses_a_small_corpus() {
    // PLAT-LANG-104: instantiate what is registered for the language and exercise it.
    let (app, _packs) = setup();
    let registry = registry_from_extensions(app.extensions(), ServiceRegistry::new());
    let corpus = [
        ("java", "class A { void f() { int x = 1; } }\n"),
        ("rust", "fn f() { let x = 1; }\n"),
        ("kotlin", "fun f() { val x = 1 }\n"),
        ("toml", "[a]\nb = 1\n"),
        ("markdown", "# Title\n\ntext\n"),
        ("starlark", "def f():\n    return 1\n"),
    ];
    for (id, source) in corpus {
        let parser: Arc<dyn ParserDefinition> = registry
            .get(LanguageId(id))
            .unwrap_or_else(|| panic!("no parser definition for {id}"));
        let def = parser.definition();
        assert_eq!(def.id, id);
        let mut tree_parser = tree_sitter::Parser::new();
        tree_parser.set_language(&def.grammar()).unwrap();
        let tree = tree_parser.parse(source, None).unwrap();
        assert!(!tree.root_node().has_error(), "{id}: {}", tree.root_node());
    }
    assert_eq!(PARSER_DEFINITION.spec(), "PLAT-LANG-030, PLAT-LANG-033");
}

#[test]
fn language_servers_and_requirements_match_the_old_builtin_wiring() {
    let (app, _packs) = setup();
    let ext = app.extensions();
    let from_packs = ServerRegistry::from_extensions(ext);
    let builtin = ServerRegistry::default();
    for path in [
        "src/main.rs",
        "App.java",
        "Main.kt",
        "build.gradle.kts",
        "pkg/BUILD",
        "MODULE.bazel",
        "app.py",
        "main.go",
        "index.tsx",
        "main.cpp",
        "pom.xml",
        "Cargo.toml",
        "config.yaml",
        "a.json",
        "README.md",
    ] {
        let id = |registry: &ServerRegistry| {
            registry
                .for_path(Path::new(path))
                .map(|(p, language)| (p.spec().id, language))
        };
        assert_eq!(id(&from_packs), id(&builtin), "{path}");
    }
    // The requirements come out the same as the built-in detection, in the same order.
    let providers = ext.extension_list(TOOLCHAIN_REQUIREMENT_PROVIDER);
    let dir = tempfile::tempdir().unwrap();
    for files in [
        vec![],
        vec!["build.gradle"],
        vec!["settings.gradle.kts", "MODULE.bazel", "Cargo.toml"],
        vec!["pom.xml", "mvnw", "build.gradle", "gradlew"],
        vec!["Cargo.toml", "src/main/kotlin/A.kt"],
    ] {
        let root = dir.path().join(files.join("-").replace('/', "_"));
        std::fs::create_dir_all(&root).unwrap();
        for file in &files {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "").unwrap();
        }
        assert_eq!(
            requirements_from(&providers, &root),
            project_requirements(&root),
            "{files:?}"
        );
    }
}

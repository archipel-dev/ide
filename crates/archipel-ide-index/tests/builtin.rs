//! Built-in indexes over a small project, readers, dumb mode and overlay.

mod common;

use archipel_ide_analysis::resolve::SearchScope;
use archipel_ide_analysis::{Change, FileRev, IndexNotReady, IndexState};
use archipel_ide_index::indexes::{context, ids, trigram_key};
use archipel_ide_index::readers::{self, SymbolLookup};
use archipel_ide_index::todo::{TodoPattern, TodoSettings};
use archipel_ide_language::structure::SymbolKind;
use common::{host, index, service, settle, write};
use std::path::{Path, PathBuf};

const JAVA: &str = r#"package com.acme.shop;

import java.util.List;

// TODO: wire the checkout
/** Orders. FIXME later */
public class Cart extends Base implements Checkout, Comparable<Cart> {
    private int total;
    public static void add(String item) { String s = "TODO not a todo"; }
    @Test
    void checkoutWorks() {}
}

interface Checkout { void pay(); }
class Base {}
"#;

const RUST: &str = r#"
pub trait Shape { fn area(&self) -> f64; }
pub struct Circle { r: f64 }
impl Shape for Circle { fn area(&self) -> f64 { 3.14 * self.r * self.r } }
// fixme: more shapes
fn helper() {}
"#;

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("dir");
    write(dir.path(), "src/Cart.java", JAVA);
    write(dir.path(), "src/shape.rs", RUST);
    write(dir.path(), "notes.txt", "plain notes\nTODO call back\n");
    write(dir.path(), "data.bin", "\0\0\0binary");
    write(dir.path(), "target/ignored.rs", "fn ignored() {}");
    dir
}

#[test]
fn file_names_and_types_are_indexed_and_excluded_dirs_skipped() {
    let dir = project();
    let (mut host, mut svc) = (host(), service(None));
    index(&mut svc, &mut host, dir.path());
    let snapshot = host.snapshot();
    let names = readers::files_matching(&snapshot, &|name| name.contains("cart")).expect("ready");
    assert_eq!(names.len(), 1);
    assert_eq!(names[0].path, Path::new("src/Cart.java"));
    assert!(
        readers::files_matching(&snapshot, &|name| name.contains("ignored"))
            .expect("ready")
            .is_empty(),
        "target/ is excluded"
    );
    let java = readers::files_of_type(&snapshot, "java").expect("ready");
    assert_eq!(java.len(), 1);
    let plain = readers::files_of_type(&snapshot, "plain_text").expect("ready");
    assert!(plain.len() >= 2, "notes.txt and data.bin");
    assert!(!svc.status().dumb);
}

#[test]
fn stub_indexes_find_declarations_without_parsing() {
    let dir = project();
    let (mut host, mut svc) = (host(), service(None));
    index(&mut svc, &mut host, dir.path());
    let snapshot = host.snapshot();

    let lookup = SymbolLookup::new(&snapshot).expect("ready");
    let cart = lookup.by_name("Cart");
    assert_eq!(cart.len(), 1);
    assert_eq!(cart[0].kind, SymbolKind::Class);
    assert_eq!(cart[0].path, Path::new("src/Cart.java"));
    assert_eq!(cart[0].line, 6);
    let classes: Vec<_> = lookup.classes().into_iter().map(|h| h.name).collect();
    assert!(classes.contains(&"Cart".to_string()) && classes.contains(&"Checkout".to_string()));
    assert!(
        classes.contains(&"Circle".to_string()),
        "rust struct is class-like"
    );
    let add = lookup.by_name("add");
    assert_eq!(add.len(), 1);
    assert_eq!(add[0].container.as_deref(), Some("Cart"));
    assert!(add[0].flags & archipel_ide_index::stubs::flags::PUBLIC != 0);
    assert!(add[0].flags & archipel_ide_index::stubs::flags::STATIC != 0);
    let tests = lookup.by_name("checkoutWorks");
    assert!(
        tests[0].flags & archipel_ide_index::stubs::flags::TEST != 0,
        "@Test"
    );
    assert!(lookup.all().len() >= 8);

    let qualified = readers::by_qualified_name(&snapshot, "com.acme.shop.Cart.add").expect("ready");
    assert_eq!(qualified.len(), 1);
    let subtypes = readers::subtypes_of(&snapshot, "Base").expect("ready");
    assert_eq!(
        subtypes.iter().map(|h| h.name.as_str()).collect::<Vec<_>>(),
        ["Cart"]
    );
    let implementers = readers::subtypes_of(&snapshot, "Checkout").expect("ready");
    assert_eq!(implementers.len(), 1);
    let shapes = readers::subtypes_of(&snapshot, "Shape").expect("ready");
    assert_eq!(
        shapes.iter().map(|h| h.name.as_str()).collect::<Vec<_>>(),
        ["Circle"]
    );
    let members = readers::members_named(&snapshot, "Cart", "total").expect("ready");
    assert_eq!(members.len(), 1);
    let all_members: Vec<_> = readers::members_of(&snapshot, "Cart")
        .expect("ready")
        .into_iter()
        .map(|h| h.name)
        .collect();
    assert!(all_members.contains(&"add".to_string()) && all_members.contains(&"total".to_string()));

    let file = snapshot.file_id(Path::new("src/Cart.java")).expect("id");
    let tree = readers::stub_tree_of(&snapshot, file)
        .expect("ready")
        .expect("tree");
    assert_eq!(tree.package.as_deref(), Some("com.acme.shop"));
    assert!(tree.nodes.iter().any(|n| n.name == "Cart"));
}

#[test]
fn words_know_their_context() {
    let dir = project();
    let (mut host, mut svc) = (host(), service(None));
    index(&mut svc, &mut host, dir.path());
    let snapshot = host.snapshot();
    let cart = snapshot.file_id(Path::new("src/Cart.java")).expect("id");
    let mask = |word: &str| {
        readers::files_with_word(&snapshot, word)
            .expect("ready")
            .into_iter()
            .find(|(f, _)| *f == cart)
            .map_or(0, |(_, m)| m)
    };
    assert_eq!(
        mask("checkout"),
        context::COMMENT,
        "only in the line comment"
    );
    assert_eq!(mask("wire"), context::COMMENT);
    assert_eq!(mask("item"), context::CODE);
    assert!(mask("TODO") & context::COMMENT != 0 && mask("TODO") & context::STRING != 0);
    assert_eq!(mask("not"), context::STRING);
    assert_eq!(mask("nonexistentword"), 0);
}

#[test]
fn trigrams_narrow_literal_searches() {
    let dir = project();
    let (mut host, mut svc) = (host(), service(None));
    index(&mut svc, &mut host, dir.path());
    let snapshot = host.snapshot();
    let found = readers::trigram_candidates(&snapshot, "Checkout")
        .expect("ready")
        .expect("narrowed");
    let cart = snapshot.file_id(Path::new("src/Cart.java")).expect("id");
    assert!(found.contains(cart));
    assert_eq!(found.len(), 1);
    assert_eq!(
        readers::trigram_candidates(&snapshot, "ab").expect("ready"),
        None
    );
    let none = readers::trigram_candidates(&snapshot, "zzzqqq")
        .expect("ready")
        .expect("narrowed");
    assert!(none.is_empty());
    assert_eq!(trigram_key(b'A', b'b', b'C'), trigram_key(b'a', b'B', b'c'));
}

#[test]
fn todo_ignores_strings_and_finds_comments_and_plain_text() {
    let dir = project();
    let (mut host, mut svc) = (host(), service(None));
    index(&mut svc, &mut host, dir.path());
    let snapshot = host.snapshot();
    let hits = readers::todo_hits(&snapshot, &SearchScope::Project).expect("ready");
    let texts: Vec<(String, String)> = hits
        .iter()
        .map(|h| (h.path.display().to_string(), h.item.text.clone()))
        .collect();
    assert!(
        texts.contains(&("src/Cart.java".into(), "TODO: wire the checkout".into())),
        "{texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|(p, t)| p == "src/Cart.java" && t.starts_with("FIXME later")),
        "{texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|(p, t)| p == "src/shape.rs" && t == "fixme: more shapes")
    );
    assert!(
        texts
            .iter()
            .any(|(p, t)| p == "notes.txt" && t == "TODO call back")
    );
    assert!(
        !texts.iter().any(|(_, t)| t.contains("not a todo")),
        "TODO inside a string literal is ignored: {texts:?}"
    );
    let java = hits
        .iter()
        .find(|h| h.item.text.starts_with("TODO: wire"))
        .expect("hit");
    assert_eq!(java.item.line, 4);
    assert_eq!(
        &JAVA[java.item.start as usize..java.item.end as usize],
        "TODO: wire the checkout"
    );
}

#[test]
fn dumb_mode_is_per_index_and_require_returns_index_not_ready() {
    let dir = tempfile::tempdir().expect("dir");
    for n in 0..40 {
        write(
            dir.path(),
            &format!("src/F{n}.java"),
            &format!("class F{n} {{ void m{n}() {{}} }}"),
        );
    }
    let (mut host, mut svc) = (host(), service(None));
    svc.attach(&mut host, dir.path());
    // Right after attach nothing is ready, so everything is blocked.
    let snap = host.snapshot();
    assert_eq!(
        snap.require::<archipel_ide_index::indexes::ShortNameIndex>()
            .err(),
        Some(IndexNotReady(ids::SHORT_NAME))
    );
    assert!(snap.is_dumb());
    assert_eq!(snap.index_state(ids::FILE_NAME), IndexState::Absent);
    // Names first: Go to File works while contents are still building.
    let mut guard = 0;
    while host.snapshot().index_state(ids::FILE_NAME) != IndexState::Ready {
        svc.poll_step(&mut host);
        guard += 1;
        assert!(guard < 20, "names become ready after scan");
    }
    let snap = host.snapshot();
    assert!(
        snap.require::<archipel_ide_index::indexes::FileNameIndex>()
            .is_ok()
    );
    assert!(snap.is_dumb(), "content indexes are still building");
    assert!(matches!(
        snap.index_state(ids::WORDS),
        IndexState::Building { .. }
    ));
    assert!(
        snap.require::<archipel_ide_index::indexes::WordsIndex>()
            .is_err()
    );
    assert!(readers::files_matching(&snap, &|_| true).is_ok());
    assert!(
        SymbolLookup::new(&snap).is_err(),
        "degrades to IndexNotReady"
    );
    assert!(svc.status().is_busy());
    settle(&mut svc, &mut host);
    let snap = host.snapshot();
    assert!(!snap.is_dumb());
    for info in svc.status().indexes {
        assert_eq!(info.state, IndexState::Ready, "{:?}", info.id);
    }
    assert_eq!(SymbolLookup::new(&snap).expect("ready").classes().len(), 40);
    assert_eq!(svc.status().phase, archipel_ide_index::Phase::Idle);
}

#[test]
fn open_buffers_overlay_the_persistent_index() {
    let dir = project();
    let (mut host, mut svc) = (host(), service(None));
    index(&mut svc, &mut host, dir.path());
    let cart_path = PathBuf::from("src/Cart.java");
    let file = host.file_id(&cart_path);
    let snap = host.snapshot();
    assert!(
        readers::files_with_word(&snap, "unsavedword")
            .expect("ready")
            .is_empty()
    );

    // Open the buffer and type: the unsaved text is visible to queries.
    let text = format!("{JAVA}\nclass Unsaved {{ int unsavedword; }}\n");
    host.apply(Change::Open {
        file,
        rev: FileRev(1),
        text: text.into(),
    });
    let events = host.take_events();
    svc.on_events(&mut host, &events);
    settle(&mut svc, &mut host);
    let snap = host.snapshot();
    let words = readers::files_with_word(&snap, "unsavedword").expect("ready");
    assert_eq!(words.len(), 1, "overlay entries are served");
    let lookup = SymbolLookup::new(&snap).expect("ready");
    assert_eq!(lookup.by_name("Unsaved").len(), 1);
    assert_eq!(
        lookup.by_name("Cart").len(),
        1,
        "unchanged declarations stay"
    );
    assert!(svc.overlay().expect("overlay").has_overlay(file));
    assert!(svc.stats().buffers >= 1);

    // Remove a declaration in the buffer: persisted entry is shadowed.
    let edited = JAVA.replace("class Base {}", "");
    host.apply(Change::Reset {
        file,
        new_rev: FileRev(2),
        text: edited.into(),
        syntax: None,
    });
    let events = host.take_events();
    svc.on_events(&mut host, &events);
    settle(&mut svc, &mut host);
    let snap = host.snapshot();
    let lookup = SymbolLookup::new(&snap).expect("ready");
    assert!(
        lookup.by_name("Base").is_empty(),
        "overlay wins over the persistent entry"
    );
    assert!(lookup.by_name("Unsaved").is_empty());
    assert!(
        readers::files_with_word(&snap, "unsavedword")
            .expect("ready")
            .is_empty()
    );
    assert!(
        readers::subtypes_of(&snap, "Base")
            .expect("ready")
            .iter()
            .all(|h| h.name != "Base"),
    );

    // Closing reconciles with the disk (the buffer was never saved).
    host.apply(Change::Close { file });
    let events = host.take_events();
    svc.on_events(&mut host, &events);
    settle(&mut svc, &mut host);
    let snap = host.snapshot();
    assert!(!svc.overlay().expect("overlay").has_overlay(file));
    assert_eq!(
        SymbolLookup::new(&snap)
            .expect("ready")
            .by_name("Base")
            .len(),
        1
    );
}

#[test]
fn changing_todo_patterns_rebuilds_only_the_todo_index() {
    let dir = project();
    let (mut host, mut svc) = (host(), service(None));
    index(&mut svc, &mut host, dir.path());
    let before = svc.stats();
    let versions: Vec<_> = svc
        .status()
        .indexes
        .iter()
        .map(|i| (i.id, i.version.clone()))
        .collect();
    svc.set_todo_settings(
        &mut host,
        TodoSettings {
            patterns: vec![TodoPattern::new(r"\bcall\b.*", false, "todo")],
            ..TodoSettings::default()
        },
    );
    let status = svc.status();
    assert!(
        matches!(
            status.state_of(ids::TODO),
            IndexState::Absent | IndexState::Building { .. }
        ),
        "todo is rebuilding"
    );
    assert_eq!(
        status.state_of(ids::WORDS),
        IndexState::Ready,
        "others stay ready"
    );
    assert_eq!(status.state_of(ids::SHORT_NAME), IndexState::Ready);
    settle(&mut svc, &mut host);
    let after: Vec<_> = svc
        .status()
        .indexes
        .iter()
        .map(|i| (i.id, i.version.clone()))
        .collect();
    for ((id, a), (_, b)) in versions.iter().zip(&after) {
        assert_eq!(a != b, *id == ids::TODO, "{id:?}");
    }
    let snap = host.snapshot();
    let hits = readers::todo_hits(&snap, &SearchScope::Project).expect("ready");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].item.text, "call back");
    // Contents were read, hashed and skipped for every other index: nothing
    // but Todo was recomputed (no parse-driven writes elsewhere).
    assert_eq!(
        svc.stats().indexed - before.indexed,
        3,
        "text files re-run Todo only"
    );
}

#[test]
fn trigrams_are_built_in_a_second_pass_after_navigation_is_ready() {
    let dir = tempfile::tempdir().expect("dir");
    for n in 0..40 {
        write(
            dir.path(),
            &format!("src/F{n}.java"),
            &format!("class F{n} {{ void m{n}() {{}} }}"),
        );
    }
    let (mut host, mut svc) = (host(), service(None));
    svc.attach(&mut host, dir.path());
    let mut guard = 0;
    while host.snapshot().index_state(ids::WORDS) != IndexState::Ready {
        svc.poll_step(&mut host);
        guard += 1;
        assert!(guard < 50);
    }
    let snap = host.snapshot();
    assert!(
        !snap.is_dumb(),
        "trigrams do not keep the IDE dumb (text scan fallback)"
    );
    assert!(SymbolLookup::new(&snap).is_ok(), "navigation is ready");
    assert!(
        snap.require::<archipel_ide_index::indexes::TrigramIndex>()
            .is_err(),
        "trigram is not ready yet"
    );
    assert!(svc.status().is_busy());
    settle(&mut svc, &mut host);
    let snap = host.snapshot();
    assert_eq!(snap.index_state(ids::TRIGRAM), IndexState::Ready);
    assert!(
        readers::trigram_candidates(&snap, "class F7")
            .expect("ready")
            .is_some_and(|f| f.len() == 1)
    );
}

#[test]
fn the_stub_interface_hash_ignores_bodies_and_positions_but_not_declarations() {
    use archipel_ide_analysis::InterfaceSummarizer;
    use archipel_ide_language::{FileId, SyntaxSnapshot, registry};
    let hash = |text: &str| {
        let lang = registry::by_id("java").expect("java");
        let snap = SyntaxSnapshot::parse(FileId(1), FileRev(1), lang, text.into()).expect("parse");
        archipel_ide_index::stubs::StubInterface.interface_hash(&snap)
    };
    let base = "class A extends B { int f; void run() { int x = 1; } }";
    assert_eq!(
        hash(base),
        hash("class A extends B { int f; void run() { int y = 2; foo(); } }")
    );
    assert_eq!(
        hash(base),
        hash("\n\nclass A extends B { int f;\n void run() { } }"),
        "positions do not matter"
    );
    assert_ne!(
        hash(base),
        hash("class A extends B { int f; void run() { } void more() {} }")
    );
    assert_ne!(
        hash(base),
        hash("class A extends C { int f; void run() { } }"),
        "supertypes are interface"
    );
    assert_ne!(
        hash(base),
        hash("class A extends B { public int f; void run() { } }"),
        "modifiers are interface"
    );
}

#[test]
fn todo_settings_round_trip_through_their_text_form() {
    use archipel_ide_index::todo::{
        TodoFilter, format_filters, format_patterns, parse_filters, parse_patterns,
    };
    let patterns = vec![
        TodoPattern::new(r"\btodo\b.*", false, "todo"),
        TodoPattern::new(r"\bHACK\b.*", true, "hack"),
    ];
    let lines = format_patterns(&patterns);
    assert_eq!(lines[1], r"s hack \bHACK\b.*");
    assert_eq!(parse_patterns(&lines), patterns);
    assert_eq!(
        parse_patterns(&[
            "x todo a".into(),
            "i todo (unclosed".into(),
            "garbage".into()
        ]),
        vec![],
        "invalid lines are dropped"
    );
    let filters = vec![TodoFilter {
        name: "Hacks".into(),
        patterns: vec![1],
    }];
    assert_eq!(parse_filters(&format_filters(&filters)), filters);
}

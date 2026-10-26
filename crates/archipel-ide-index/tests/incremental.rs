//! Incremental indexing equals a fresh full index; persistence across
//! restarts; the real (threaded) executor.

mod common;

use archipel_ide_analysis::vfs::DiskVfs;
use archipel_ide_analysis::{AnalysisHost, HostConfig, IndexState, default_registry};
use archipel_ide_index::indexes::ids;
use archipel_ide_index::readers::{self, SymbolLookup};
use archipel_ide_index::{IndexConfig, IndexService};
use common::{dump, host, index, service, settle, write};
use proptest::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
enum Op {
    Write {
        slot: usize,
        content: u8,
        variant: u8,
    },
    Touch {
        slot: usize,
    },
    Delete {
        slot: usize,
    },
    Rename {
        from: usize,
        to: usize,
    },
    RenameDir {
        from: u8,
        to: u8,
    },
    Settle,
}

const SLOTS: usize = 12;

fn slot_path(slot: usize) -> String {
    let dir = slot % 3;
    let ext = ["java", "rs", "txt", "md"][(slot / 3) % 4];
    format!("d{dir}/file{}.{ext}", slot % 5)
}

fn content(kind: u8, variant: u8, ext: &str) -> String {
    let n = variant % 4;
    match (ext, kind % 3) {
        ("java", 0) => format!(
            "package p{n};\npublic class C{n} extends Base{n} {{ int f{n}; void m{n}() {{ String s = \"w{n}\"; }} }}\n"
        ),
        ("java", 1) => format!("// TODO fix {n}\ninterface I{n} {{ void run{n}(); }}\n"),
        ("java", _) => format!("class K{n} implements I{n} {{}}\n"),
        ("rs", 0) => format!("pub struct S{n};\nimpl T{n} for S{n} {{ fn go{n}(&self) {{}} }}\n"),
        ("rs", 1) => format!("// FIXME later {n}\npub fn free{n}() {{ let word{n} = 1; }}\n"),
        ("rs", _) => format!("pub trait T{n} {{ fn go{n}(&self); }}\n"),
        (_, 0) => format!("notes {n}\nTODO call {n}\n"),
        (_, 1) => format!("# heading {n}\nsome words here {n}\n"),
        _ => String::new(),
    }
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => (0..SLOTS, any::<u8>(), any::<u8>()).prop_map(|(slot, content, variant)| Op::Write { slot, content, variant }),
        1 => (0..SLOTS).prop_map(|slot| Op::Touch { slot }),
        2 => (0..SLOTS).prop_map(|slot| Op::Delete { slot }),
        2 => (0..SLOTS, 0..SLOTS).prop_map(|(from, to)| Op::Rename { from, to }),
        1 => (0u8..3, 3u8..6).prop_map(|(from, to)| Op::RenameDir { from, to }),
        2 => Just(Op::Settle),
    ]
}

fn apply(
    op: &Op,
    root: &Path,
    svc: &mut IndexService,
    host: &mut AnalysisHost,
    explicit_moves: bool,
) {
    match op {
        Op::Write {
            slot,
            content: kind,
            variant,
        } => {
            let relative = slot_path(*slot);
            let ext = relative.rsplit('.').next().expect("ext").to_string();
            write(root, &relative, &content(*kind, *variant, &ext));
            svc.notify_paths(host, &[PathBuf::from(relative)]);
        }
        Op::Touch { slot } => {
            let relative = slot_path(*slot);
            if let Ok(text) = std::fs::read_to_string(root.join(&relative)) {
                std::fs::write(root.join(&relative), text).expect("touch");
                svc.notify_paths(host, &[PathBuf::from(relative)]);
            }
        }
        Op::Delete { slot } => {
            let relative = slot_path(*slot);
            let _ = std::fs::remove_file(root.join(&relative));
            svc.notify_paths(host, &[PathBuf::from(relative)]);
        }
        Op::Rename { from, to } => {
            let (a, b) = (slot_path(*from), slot_path(*to));
            if a == b || !root.join(&a).is_file() {
                return;
            }
            std::fs::create_dir_all(root.join(&b).parent().expect("parent")).expect("dirs");
            std::fs::rename(root.join(&a), root.join(&b)).expect("rename");
            if explicit_moves {
                svc.notify_moved(host, Path::new(&a), Path::new(&b));
            } else {
                svc.notify_paths(host, &[PathBuf::from(a), PathBuf::from(b)]);
            }
        }
        Op::RenameDir { from, to } => {
            let (a, b) = (format!("d{from}"), format!("d{to}"));
            if !root.join(&a).is_dir() || root.join(&b).exists() {
                return;
            }
            std::fs::rename(root.join(&a), root.join(&b)).expect("rename dir");
            svc.notify_paths(host, &[PathBuf::from(a), PathBuf::from(b)]);
        }
        Op::Settle => {
            assert!(svc.run_until_idle(host, Duration::from_secs(60)));
        }
    }
}

fn fresh_dump(root: &Path) -> common::Dump {
    let (mut host, mut svc) = (host(), service(None));
    index(&mut svc, &mut host, root);
    dump(&svc, &host)
}

fn run_sequence(ops: &[Op], explicit_moves: bool) {
    let dir = tempfile::tempdir().expect("dir");
    let root = dir.path();
    write(root, "d0/seed.java", "class Seed {}\n");
    write(root, "d1/seed.txt", "TODO seed\n");
    let (mut host, mut svc) = (host(), service(None));
    index(&mut svc, &mut host, root);
    for op in ops {
        apply(op, root, &mut svc, &mut host, explicit_moves);
    }
    settle(&mut svc, &mut host);
    let incremental = dump(&svc, &host);
    let fresh = fresh_dump(root);
    if incremental != fresh {
        let only_inc: Vec<_> = incremental
            .keys()
            .filter(|k| !fresh.contains_key(*k))
            .take(5)
            .collect();
        let only_fresh: Vec<_> = fresh
            .keys()
            .filter(|k| !incremental.contains_key(*k))
            .take(5)
            .collect();
        let differing: Vec<_> = incremental
            .iter()
            .filter(|(k, v)| fresh.get(*k).is_some_and(|f| f != *v))
            .map(|(k, _)| k)
            .take(5)
            .collect();
        panic!(
            "incremental != fresh after {ops:?}\nonly incremental: {only_inc:?}\nonly fresh: {only_fresh:?}\ndiffering: {differing:?}"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// After random edit, rename and delete sequences the incrementally
    /// maintained index equals a fresh full index of the final tree.
    #[test]
    fn incremental_index_equals_a_fresh_full_index(ops in prop::collection::vec(op_strategy(), 1..24)) {
        run_sequence(&ops, false);
    }

    /// Same with explicit moves (file ids are kept across the rename).
    #[test]
    fn incremental_index_with_explicit_moves_equals_a_fresh_full_index(ops in prop::collection::vec(op_strategy(), 1..24)) {
        run_sequence(&ops, true);
    }
}

#[test]
fn same_length_rewrite_within_one_mtime_tick_is_picked_up() {
    let dir = tempfile::tempdir().expect("dir");
    write(dir.path(), "a.java", "class Aaaa {}\n");
    let (mut host, mut svc) = (host(), service(None));
    index(&mut svc, &mut host, dir.path());
    write(dir.path(), "a.java", "class Bbbb {}\n");
    svc.notify_paths(&mut host, &[PathBuf::from("a.java")]);
    settle(&mut svc, &mut host);
    let snap = host.snapshot();
    let lookup = SymbolLookup::new(&snap).expect("ready");
    assert!(lookup.by_name("Aaaa").is_empty());
    assert_eq!(lookup.by_name("Bbbb").len(), 1);
}

#[test]
fn the_index_survives_a_restart_and_only_changed_files_are_reindexed() {
    let project = tempfile::tempdir().expect("project");
    let cache = tempfile::tempdir().expect("cache");
    for n in 0..30 {
        write(
            project.path(),
            &format!("src/A{n}.java"),
            &format!("class A{n} {{ void m{n}() {{}} }}\n"),
        );
    }
    let cart_id;
    {
        let (mut host, mut svc) = (host(), service(Some(cache.path())));
        index(&mut svc, &mut host, project.path());
        assert_eq!(svc.stats().indexed, 30);
        assert_eq!(
            svc.status().store,
            archipel_ide_index::StoreKind::Persistent
        );
        cart_id = host.lookup(Path::new("src/A7.java")).expect("id");
        svc.detach(&mut host);
    }
    // (the first session's host and service are dropped, closing the store)
    // Between sessions: one file changes, one disappears, one appears.
    write(
        project.path(),
        "src/A3.java",
        "class Changed3 { void other() {} }\n",
    );
    std::fs::remove_file(project.path().join("src/A4.java")).expect("rm");
    write(project.path(), "src/New.java", "class New {}\n");

    let (mut host, mut svc) = (host(), service(Some(cache.path())));
    svc.attach(&mut host, project.path());
    {
        // Ready before any work: the persisted index answers immediately.
        let snap = host.snapshot();
        for info in svc.status().indexes {
            assert_eq!(info.state, IndexState::Ready, "{:?} restored", info.id);
        }
        assert!(!snap.is_dumb());
        assert!(SymbolLookup::new(&snap).expect("ready").by_name("A7").len() == 1);
        assert_eq!(
            host.lookup(Path::new("src/A7.java")),
            Some(cart_id),
            "file ids are stable"
        );
        settle(&mut svc, &mut host);
        assert_eq!(svc.stats().indexed, 2, "only the changed and the new file");
        let snap = host.snapshot();
        let lookup = SymbolLookup::new(&snap).expect("ready");
        assert!(lookup.by_name("A3").is_empty());
        assert_eq!(lookup.by_name("Changed3").len(), 1);
        assert!(lookup.by_name("A4").is_empty(), "deleted file is gone");
        assert_eq!(lookup.by_name("New").len(), 1);
        assert!(
            readers::files_matching(&snap, &|n| n == "a4.java")
                .expect("ready")
                .is_empty()
        );
        assert_eq!(
            readers::files_matching(&snap, &|n| n == "new.java")
                .expect("ready")
                .len(),
            1
        );
    }
    // Third session: nothing changed, nothing indexed.
    svc.detach(&mut host);
    drop((svc, host));
    let (mut host, mut svc) = (common::host(), service(Some(cache.path())));
    index(&mut svc, &mut host, project.path());
    assert_eq!(svc.stats().indexed, 0);
    assert_eq!(svc.stats().unchanged, 0);
}

#[test]
fn an_unusable_store_directory_falls_back_to_memory() {
    let project = tempfile::tempdir().expect("project");
    write(project.path(), "A.java", "class A {}\n");
    let blocker = tempfile::NamedTempFile::new().expect("file");
    let (mut host, mut svc) = (host(), service(Some(&blocker.path().join("sub"))));
    index(&mut svc, &mut host, project.path());
    let status = svc.status();
    assert_eq!(status.store, archipel_ide_index::StoreKind::Memory);
    assert!(status.store_error.is_some());
    assert!(SymbolLookup::new(&host.snapshot()).is_ok());
}

#[test]
fn a_rebuild_command_drops_one_index_or_all() {
    let project = tempfile::tempdir().expect("project");
    write(project.path(), "A.java", "class A {} // TODO x\n");
    let (mut host, mut svc) = (host(), service(None));
    index(&mut svc, &mut host, project.path());
    svc.rebuild(&mut host, Some(ids::WORDS));
    assert!(matches!(
        svc.status().state_of(ids::WORDS),
        IndexState::Absent | IndexState::Building { .. }
    ));
    assert!(host.snapshot().is_dumb());
    settle(&mut svc, &mut host);
    assert_eq!(svc.status().state_of(ids::WORDS), IndexState::Ready);
    assert!(!host.snapshot().is_dumb());
    svc.rebuild(&mut host, None);
    settle(&mut svc, &mut host);
    assert!(
        svc.status()
            .indexes
            .iter()
            .all(|i| i.state == IndexState::Ready)
    );
    assert_eq!(
        readers::files_with_word(&host.snapshot(), "A")
            .expect("ready")
            .len(),
        1
    );
}

#[test]
fn the_threaded_executor_gives_the_same_result() {
    let project = tempfile::tempdir().expect("project");
    for n in 0..300 {
        write(
            project.path(),
            &format!("m{}/F{n}.java", n % 7),
            &format!(
                "package m{};\n// TODO t{n}\nclass F{n} extends Base {{ int x{n}; void f{n}() {{}} }}\n",
                n % 7
            ),
        );
    }
    let (mut inline_host, mut inline) = (host(), service(None));
    index(&mut inline, &mut inline_host, project.path());
    let expected = dump(&inline, &inline_host);

    let mut threaded_host = AnalysisHost::new(
        HostConfig {
            threads: 3,
            ..HostConfig::testing()
        },
        Arc::new(DiskVfs),
        default_registry(),
    );
    let mut cfg = IndexConfig::testing();
    cfg.max_parallel = 3;
    let mut threaded = IndexService::new(cfg);
    threaded.attach(&mut threaded_host, project.path());
    assert!(threaded.run_until_idle(&mut threaded_host, Duration::from_secs(60)));
    assert_eq!(dump(&threaded, &threaded_host), expected);
    assert!(!threaded_host.snapshot().is_dumb());
}

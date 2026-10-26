//! Host behavior: snapshots, edits, events, journal, revocation, caches.

use archipel_ide_analysis::cache::Slot;
use archipel_ide_analysis::cancel::CHECK_INTERVAL;
use archipel_ide_analysis::job::{JobSpec, Priority, Revoke, RevokeScope};
use archipel_ide_analysis::testing::{Fixture, RUST_SAMPLE, byte_edit};
use archipel_ide_analysis::vfs::VfsChange;
use archipel_ide_analysis::{
    AnalysisEvent, AnalysisHost, CancelToken, Cancelled, Change, FileId, FileRev, HostConfig,
    IndexId, IndexStamp, IndexState, Snapshot,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn edit(
    fixture: &mut Fixture,
    file: FileId,
    rev: u64,
    range: std::ops::Range<usize>,
    text: &str,
) -> Arc<str> {
    let old = fixture.host.snapshot().file_text(file).expect("text");
    let mut new = old.to_string();
    new.replace_range(range.clone(), text);
    let new: Arc<str> = new.into();
    fixture.host.apply(Change::Edit {
        file,
        edits: vec![byte_edit(&old, range, text)],
        new_rev: FileRev(rev),
        text: new.clone(),
        syntax: None,
    });
    new
}

#[test]
fn snapshots_are_isolated_from_later_changes() {
    let mut fixture = Fixture::new();
    let rust = fixture.rust;
    fixture.open(rust);
    let before = fixture.host.snapshot();
    let at = RUST_SAMPLE.find("42").expect("42");
    edit(&mut fixture, rust, 2, at..at + 2, "43");
    let after = fixture.host.snapshot();
    assert!(before.file_text(rust).expect("text").contains("42"));
    assert!(after.file_text(rust).expect("text").contains("43"));
    assert_eq!(before.file_rev(rust), Some(FileRev(1)));
    assert_eq!(after.file_rev(rust), Some(FileRev(2)));
    assert!(after.generation() > before.generation());
    let syntax = after.syntax(rust).expect("syntax");
    assert_eq!(syntax.rev(), FileRev(2));
    assert!(syntax.text().contains("43"));
    // Snapshots move across threads.
    let handle = std::thread::spawn(move || before.syntax(rust).map(|s| s.root().kind()));
    assert_eq!(handle.join().expect("thread"), Some("source_file"));
}

#[test]
fn edits_publish_changed_ranges_and_journal_maps_offsets() {
    let mut fixture = Fixture::new();
    let rust = fixture.rust;
    fixture.open(rust);
    fixture.host.take_events();
    let at = RUST_SAMPLE.find("doubled + 42").expect("expr");
    edit(&mut fixture, rust, 2, at..at, "1 + ");
    edit(&mut fixture, rust, 3, 0..0, "// head\n");
    let events = fixture.host.take_events();
    let ranges: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            AnalysisEvent::SyntaxChanged { file, rev, ranges } if *file == rust => {
                Some((*rev, ranges.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(ranges.len(), 2);
    assert!(
        ranges[0]
            .1
            .iter()
            .any(|r| r.start <= at as u32 && at as u32 + 4 <= r.end)
    );
    let snap = fixture.host.snapshot();
    let map = snap.edits_since(rust, FileRev(1)).expect("journal");
    let struct_at = RUST_SAMPLE.find("pub struct").expect("struct");
    assert_eq!(map.map(struct_at), struct_at + 4 + 8);
    assert!(
        snap.edits_since(rust, FileRev(3))
            .expect("same")
            .is_identity()
    );
    assert!(snap.edits_since(rust, FileRev(9)).is_none());
}

#[test]
fn vfs_batches_update_stamps_and_ids_survive_moves() {
    let mut fixture = Fixture::new();
    let java = fixture.java;
    let before = fixture.host.snapshot().file_rev(java);
    fixture.vfs.write(
        Path::new(archipel_ide_analysis::testing::JAVA_PATH),
        "class B {}",
    );
    let mut table = fixture.host.snapshot().files().clone();
    let batch = VfsChange::classify(
        &mut table,
        &*fixture.vfs,
        [PathBuf::from(archipel_ide_analysis::testing::JAVA_PATH)],
    );
    assert_eq!(batch.modified, vec![java]);
    fixture.host.apply(Change::Vfs(batch));
    let snap = fixture.host.snapshot();
    assert!(snap.file_rev(java) > before);
    assert_eq!(
        snap.syntax(java).expect("parsed").text().as_ref(),
        "class B {}"
    );
    let moved = VfsChange {
        moved: vec![(
            java,
            PathBuf::from(archipel_ide_analysis::testing::JAVA_PATH),
            PathBuf::from("/fixture/src/B.java"),
        )],
        ..Default::default()
    };
    fixture.host.apply(Change::Vfs(moved));
    assert_eq!(
        fixture.host.lookup(Path::new("/fixture/src/B.java")),
        Some(java)
    );
    fixture.host.apply(Change::Vfs(VfsChange {
        deleted: vec![java],
        ..Default::default()
    }));
    let snap = fixture.host.snapshot();
    assert_eq!(snap.file_rev(java), None);
    assert!(snap.path(java).is_none());
}

#[test]
fn revoked_jobs_return_cancelled_within_256_checks() {
    let mut host = AnalysisHost::with_defaults(HostConfig {
        threads: 2,
        ..HostConfig::default()
    });
    let file = host.file_id(Path::new("/x/a.rs"));
    host.apply(Change::Open {
        file,
        rev: FileRev(1),
        text: "fn a() {}".into(),
    });
    let started = Arc::new(AtomicUsize::new(0));
    let checks_after_cancel = Arc::new(AtomicUsize::new(0));
    let (started2, after2) = (started.clone(), checks_after_cancel.clone());
    let handle = host.spawn(
        JobSpec::new(
            Priority::Daemon,
            Revoke::OnAnyChangeOf(RevokeScope::Files(vec![file])),
        ),
        move |_snap: &Snapshot, cancel: &CancelToken| {
            let mut checkpoint = cancel.checkpoint();
            started2.store(1, Ordering::SeqCst);
            let mut observed = false;
            loop {
                if cancel.is_cancelled() {
                    observed = true;
                }
                if observed {
                    after2.fetch_add(1, Ordering::SeqCst);
                }
                checkpoint.tick()?;
            }
            #[allow(unreachable_code)]
            Ok::<(), Cancelled>(())
        },
    );
    while started.load(Ordering::SeqCst) == 0 {
        std::thread::yield_now();
    }
    // An unrelated change does not revoke it.
    let other = host.file_id(Path::new("/x/b.rs"));
    host.apply(Change::Open {
        file: other,
        rev: FileRev(1),
        text: "".into(),
    });
    assert!(!handle.token().is_cancelled());
    host.apply(Change::Edit {
        file,
        edits: vec![archipel_ide_analysis::testing::byte_edit(
            "fn a() {}",
            3..4,
            "b",
        )],
        new_rev: FileRev(2),
        text: "fn b() {}".into(),
        syntax: None,
    });
    assert_eq!(handle.wait(), Err(Cancelled));
    let after = checks_after_cancel.load(Ordering::SeqCst);
    assert!(
        after <= 256 && after <= CHECK_INTERVAL as usize,
        "{after} checks after revoke"
    );
}

#[test]
fn coalesced_jobs_cancel_their_predecessor() {
    let mut host = AnalysisHost::with_defaults(HostConfig {
        threads: 1,
        ..HostConfig::default()
    });
    let gate = Arc::new(AtomicUsize::new(0));
    let g = gate.clone();
    let first = host.spawn(
        JobSpec::new(Priority::Background, Revoke::Never).coalesce("k", 1),
        move |_, cancel| {
            while g.load(Ordering::SeqCst) == 0 {
                cancel.check()?;
                std::thread::yield_now();
            }
            Ok(1)
        },
    );
    let second = host.spawn(
        JobSpec::new(Priority::Background, Revoke::Never).coalesce("k", 1),
        |_, _| Ok(2),
    );
    assert_eq!(first.wait(), Err(Cancelled));
    assert_eq!(second.wait(), Ok(2));
    gate.store(1, Ordering::SeqCst);
}

const DECLS: Slot<FileId, Vec<String>> = Slot::new("decls");
const BODY: Slot<FileId, usize> = Slot::new("body-len");

#[test]
fn cached_values_follow_file_and_interface_stamps_with_early_cutoff() {
    let mut fixture = Fixture::new();
    let rust = fixture.rust;
    let java = fixture.java;
    fixture.open(rust);
    fixture.host.poll();
    let runs = AtomicUsize::new(0);
    let body_runs = AtomicUsize::new(0);
    let cancel = CancelToken::new();
    // A value of the Java file that depends on the Rust file's interface,
    // and one that depends on its text.
    let read = |snap: &Snapshot| {
        let db = snap.db();
        let decls = db
            .cached(DECLS, java, &cancel, |cx| {
                runs.fetch_add(1, Ordering::SeqCst);
                cx.interface(rust);
                Ok(snap
                    .syntax(rust)
                    .map(|s| s.structure().into_iter().map(|i| i.name).collect())
                    .unwrap_or_default())
            })
            .expect("decls");
        let body = db
            .cached(BODY, rust, &cancel, |cx| {
                body_runs.fetch_add(1, Ordering::SeqCst);
                cx.file(rust);
                Ok(snap.file_text(rust).map(|t| t.len()).unwrap_or(0))
            })
            .expect("body");
        (decls, *body)
    };
    let (decls, _) = read(&fixture.host.snapshot());
    assert_eq!(*decls, vec!["answer".to_string(), "Point".to_string()]);
    read(&fixture.host.snapshot());
    assert_eq!(
        (
            runs.load(Ordering::SeqCst),
            body_runs.load(Ordering::SeqCst)
        ),
        (1, 1)
    );
    // Edit inside the function body: text changes, interface does not.
    let at = RUST_SAMPLE.find("x * 2").expect("body");
    edit(&mut fixture, rust, 2, at + 4..at + 5, "3");
    fixture.host.poll();
    let (_, len) = read(&fixture.host.snapshot());
    assert_eq!(len, RUST_SAMPLE.len());
    assert_eq!(
        runs.load(Ordering::SeqCst),
        1,
        "early cutoff keeps interface dependents"
    );
    assert_eq!(
        body_runs.load(Ordering::SeqCst),
        2,
        "text dependents recompute"
    );
    // Renaming a declaration changes the interface.
    let name = RUST_SAMPLE.find("Point").expect("struct");
    edit(&mut fixture, rust, 3, name..name + 5, "Spot");
    fixture.host.poll();
    let (decls, _) = read(&fixture.host.snapshot());
    assert_eq!(*decls, vec!["answer".to_string(), "Spot".to_string()]);
    assert_eq!(runs.load(Ordering::SeqCst), 2);
}

#[test]
fn index_commits_and_dumb_mode_are_stamped() {
    let mut fixture = Fixture::new();
    let snap = fixture.host.snapshot();
    assert_eq!(snap.index_state(IndexId("FileName")), IndexState::Absent);
    fixture.host.apply(Change::DumbMode(true));
    fixture.host.apply(Change::IndexCommit {
        index: IndexId("FileName"),
        stamp: IndexStamp(1),
        state: IndexState::Ready,
    });
    let snap = fixture.host.snapshot();
    assert!(snap.is_dumb());
    assert_eq!(snap.index_state(IndexId("FileName")), IndexState::Ready);
    assert_eq!(snap.stamps().index(IndexId("FileName")), IndexStamp(1));
    let events = fixture.host.take_events();
    assert!(events.contains(&AnalysisEvent::DumbModeChanged(true)));
    assert!(events.contains(&AnalysisEvent::IndexUpdated(IndexId("FileName"))));
}

#[test]
fn closed_files_parse_lazily_and_fragments_resolve() {
    let fixture = Fixture::new();
    let snap = fixture.host.snapshot();
    let markdown = snap.syntax(fixture.markdown).expect("markdown");
    let at = markdown.text().find("main").expect("main") as u32;
    assert_eq!(markdown.fragment_at(at).map(|f| f.lang().id), Some("rust"));
    let again = snap.syntax(fixture.markdown).expect("cached");
    assert!(again.ptr_eq(&markdown));
    assert_eq!(
        snap.language_id(fixture.java).map(|l| l.as_str()),
        Some("java")
    );
}

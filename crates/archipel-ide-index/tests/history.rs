//! Local History store: blobs, revision log, retention, recovery.

use archipel_ide_index::history::{BlobId, BlobStore, Cause, History, Retention};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

fn clock() -> (Arc<AtomicU64>, Arc<dyn Fn() -> u64 + Send + Sync>) {
    let time = Arc::new(AtomicU64::new(1_000_000));
    let read = time.clone();
    (time, Arc::new(move || read.load(Ordering::SeqCst)))
}

#[test]
fn blobs_are_content_addressed_compressed_and_verified() {
    let dir = tempfile::tempdir().expect("dir");
    let store = BlobStore::open(dir.path()).expect("open");
    let text = "fn main() {}\n".repeat(200);
    let id = store.put(text.as_bytes()).expect("put");
    assert_eq!(id, BlobId::of(text.as_bytes()));
    assert_eq!(store.put(text.as_bytes()).expect("again"), id, "idempotent");
    assert!(store.contains(&id));
    assert!(store.size(&id) < text.len() as u64 / 4, "compressed");
    assert_eq!(store.get(&id).expect("get"), text.as_bytes());
    assert_eq!(store.list().len(), 1);
    // A damaged blob is detected, never returned.
    let hex = id.hex();
    let file = dir.path().join(&hex[..2]).join(&hex[2..]);
    std::fs::write(&file, b"garbage").expect("damage");
    assert!(store.get(&id).is_err());
    store.remove(&id);
    assert!(matches!(
        store.get(&id),
        Err(archipel_ide_index::history::BlobError::Missing(_))
    ));
}

#[test]
fn revisions_are_per_path_newest_first_and_skip_unchanged_content() {
    let dir = tempfile::tempdir().expect("dir");
    let (time, clock) = clock();
    let history = History::open_with_clock(dir.path(), clock).expect("open");
    let a = Path::new("src/A.java");
    let b = Path::new("src/B.java");
    assert!(
        history
            .record(a, b"v1", Cause::Initial)
            .expect("rec")
            .is_some()
    );
    time.fetch_add(10, Ordering::SeqCst);
    assert!(
        history
            .record(a, b"v1", Cause::Save)
            .expect("rec")
            .is_none(),
        "unchanged"
    );
    assert!(
        history
            .record(a, b"v2", Cause::Save)
            .expect("rec")
            .is_some()
    );
    time.fetch_add(10, Ordering::SeqCst);
    history.record(b, b"other", Cause::Save).expect("rec");
    time.fetch_add(10, Ordering::SeqCst);
    history.record(a, b"v3", Cause::External).expect("rec");
    history.label("before big refactoring").expect("label");

    let revisions = history.revisions(a);
    assert_eq!(revisions.len(), 3);
    assert_eq!(revisions[0].cause, Cause::External);
    assert_eq!(
        history.content(&revisions[0]).expect("content"),
        Some(b"v3".to_vec())
    );
    assert_eq!(
        history.content(&revisions[2]).expect("content"),
        Some(b"v1".to_vec())
    );
    assert!(revisions[0].id > revisions[1].id && revisions[1].id > revisions[2].id);
    assert_eq!(history.revisions(b).len(), 1);
    assert!(history.revisions(Path::new("src/None.java")).is_empty());

    let recent = history.recent(3);
    assert_eq!(
        recent.iter().map(|r| r.cause.label()).collect::<Vec<_>>(),
        ["Label: before big refactoring", "External change", "Saved"]
    );
    assert_eq!(history.paths(), vec![a.to_path_buf(), b.to_path_buf()]);

    history.record_deletion(b).expect("delete");
    assert!(history.record_deletion(b).expect("again").is_none());
    assert_eq!(history.latest(b).expect("latest").cause, Cause::Deleted);
    assert_eq!(
        history
            .content(&history.latest(b).expect("latest"))
            .expect("content"),
        None
    );
}

#[test]
fn history_survives_a_restart() {
    let dir = tempfile::tempdir().expect("dir");
    {
        let history = History::open(dir.path()).expect("open");
        history
            .record(Path::new("a.txt"), b"hello", Cause::Save)
            .expect("rec");
    }
    let history = History::open(dir.path()).expect("reopen");
    let revisions = history.revisions(Path::new("a.txt"));
    assert_eq!(revisions.len(), 1);
    assert_eq!(
        history.content(&revisions[0]).expect("c"),
        Some(b"hello".to_vec())
    );
    // ids keep growing after a restart
    let next = history
        .record(Path::new("a.txt"), b"again", Cause::Save)
        .expect("rec")
        .expect("new");
    assert!(next.id > revisions[0].id);
}

#[test]
fn retention_removes_old_revisions_and_unreferenced_blobs() {
    let dir = tempfile::tempdir().expect("dir");
    let (time, clock) = clock();
    let history = History::open_with_clock(dir.path(), clock).expect("open");
    let p = Path::new("p.txt");
    history.record(p, b"old content", Cause::Save).expect("rec");
    history
        .record(Path::new("q.txt"), b"old content", Cause::Save)
        .expect("rec");
    time.fetch_add(10 * 24 * 3600 * 1000, Ordering::SeqCst);
    history
        .record(p, b"fresh content", Cause::Save)
        .expect("rec");
    assert_eq!(history.blobs().list().len(), 2);
    let stats = history
        .purge(Retention {
            max_age_ms: 5 * 24 * 3600 * 1000,
            max_bytes: u64::MAX,
        })
        .expect("purge");
    assert_eq!(stats.revisions_removed, 2, "both old revisions");
    assert_eq!(
        stats.blobs_removed, 1,
        "the shared blob is deleted once nothing uses it"
    );
    assert_eq!(history.revisions(p).len(), 1);
    assert!(history.revisions(Path::new("q.txt")).is_empty());
    assert_eq!(history.blobs().list().len(), 1);

    // Size cap: the oldest go first.
    time.fetch_add(10, Ordering::SeqCst);
    history
        .record(p, &vec![b'x'; 50_000], Cause::Save)
        .expect("rec");
    time.fetch_add(10, Ordering::SeqCst);
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let noise: Vec<u8> = (0..20_000)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 24) as u8
        })
        .collect();
    history.record(p, &noise, Cause::Save).expect("rec");
    let stats = history
        .purge(Retention {
            max_age_ms: u64::MAX / 2,
            max_bytes: 5_000,
        })
        .expect("purge");
    assert!(stats.revisions_removed >= 2, "{stats:?}");
    let left = history.revisions(p);
    assert_eq!(left.len(), 1);
    assert_eq!(history.content(&left[0]).expect("c"), Some(noise));
}

#[test]
fn a_damaged_log_is_replaced_without_error() {
    let dir = tempfile::tempdir().expect("dir");
    {
        let history = History::open(dir.path()).expect("open");
        history
            .record(Path::new("a.txt"), b"x", Cause::Save)
            .expect("rec");
    }
    std::fs::write(dir.path().join("revisions.redb"), vec![7u8; 30_000]).expect("damage");
    let history = History::open(dir.path()).expect("recovers");
    assert!(history.revisions(Path::new("a.txt")).is_empty());
    history
        .record(Path::new("a.txt"), b"y", Cause::Save)
        .expect("usable again");
    assert_eq!(history.revisions(Path::new("a.txt")).len(), 1);
    // The orphaned blob of the lost log is reclaimed by the next purge.
    history.purge(Retention::default()).expect("purge");
    assert_eq!(history.blobs().list().len(), 1);
}

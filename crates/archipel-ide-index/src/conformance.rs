//! Conformance suite every [`DurableStore`] backend must pass (run on the
//! in-memory and the `redb` store by the tests of this crate).

use crate::store::{DurableStore, FileRecord};
use archipel_ide_analysis::{FileId, IndexId};
use std::path::PathBuf;

const IDX: IndexId = IndexId("Conformance");
const OTHER: IndexId = IndexId("Conformance:Other");

fn kv(key: &[u8], value: &[u8]) -> (Vec<u8>, Vec<u8>) {
    (key.to_vec(), value.to_vec())
}

/// Panics (test failure) when `store` violates the storage contract. `store`
/// must be fresh (empty).
pub fn run(store: &dyn DurableStore) {
    assert_eq!(store.open_index(IDX, "1"), Ok(false), "fresh index");
    assert_eq!(store.open_index(OTHER, "1"), Ok(false));
    let (f1, f2, f3) = (FileId(1), FileId(2), FileId(70_000));

    // put + commit makes entries visible; postings are per (key, file).
    store.put(
        IDX,
        f1,
        vec![kv(b"a", b"1"), kv(b"ab", b"2"), kv(b"\x00\xff", b"3")],
    );
    store.put(IDX, f2, vec![kv(b"a", b"4")]);
    store.put(IDX, f3, vec![kv(b"a", b"5"), kv(b"zz", b"")]);
    let stamp1 = store.commit(IDX);
    let mut a = store.get(IDX, b"a");
    a.sort();
    assert_eq!(
        a,
        vec![
            (f1, b"1".to_vec()),
            (f2, b"4".to_vec()),
            (f3, b"5".to_vec())
        ],
        "postings of `a` (and no postings of `ab`)"
    );
    assert_eq!(store.get(IDX, b"ab"), vec![(f1, b"2".to_vec())]);
    assert_eq!(store.get(IDX, b"\x00\xff"), vec![(f1, b"3".to_vec())]);
    assert_eq!(store.get(IDX, b"zz"), vec![(f3, Vec::new())]);
    assert!(store.get(IDX, b"missing").is_empty());
    assert!(store.get(OTHER, b"a").is_empty(), "indexes are separate");
    assert_eq!(
        store.keys(IDX),
        vec![
            b"\x00\xff".to_vec(),
            b"a".to_vec(),
            b"ab".to_vec(),
            b"zz".to_vec()
        ],
        "keys are sorted and distinct"
    );
    let mut forward = store.forward(IDX, f1);
    forward.sort();
    assert_eq!(
        forward,
        vec![b"\x00\xff".to_vec(), b"a".to_vec(), b"ab".to_vec()]
    );

    // Replacing a file's entries removes what it no longer has.
    store.put(IDX, f1, vec![kv(b"a", b"9"), kv(b"new", b"")]);
    let stamp2 = store.commit(IDX);
    assert!(stamp2 > stamp1, "stamps increase");
    assert!(store.get(IDX, b"ab").is_empty());
    assert!(store.get(IDX, b"\x00\xff").is_empty());
    assert!(store.get(IDX, b"a").contains(&(f1, b"9".to_vec())));
    assert_eq!(store.get(IDX, b"new"), vec![(f1, Vec::new())]);
    let mut forward = store.forward(IDX, f1);
    forward.sort();
    assert_eq!(forward, vec![b"a".to_vec(), b"new".to_vec()]);

    // Removal.
    store.remove_file(IDX, f2);
    store.remove_file(IDX, f3);
    store.commit(IDX);
    assert_eq!(
        store.get(IDX, b"a"),
        vec![(f1, b"9".to_vec())],
        "only f1 remains"
    );
    assert!(store.forward(IDX, f2).is_empty());
    assert_eq!(store.keys(IDX), vec![b"a".to_vec(), b"new".to_vec()]);

    // Writes become visible at commit only for backends with batching; both
    // behaviors satisfy "visible after commit".
    store.put(IDX, f2, vec![kv(b"late", b"")]);
    store.commit(IDX);
    assert_eq!(store.get(IDX, b"late"), vec![(f2, Vec::new())]);

    // Indexed hashes and completeness.
    assert_eq!(store.indexed_hash(IDX, f1), None);
    store.set_indexed_hash(IDX, f1, Some(42));
    store.set_indexed_hash(IDX, f2, Some(u64::MAX));
    store.commit(IDX);
    assert_eq!(store.indexed_hash(IDX, f1), Some(42));
    assert_eq!(store.indexed_hash(IDX, f2), Some(u64::MAX));
    store.set_indexed_hash(IDX, f1, None);
    store.commit(IDX);
    assert_eq!(store.indexed_hash(IDX, f1), None);
    assert!(!store.is_complete(IDX));
    store.set_complete(IDX, true);
    assert!(store.is_complete(IDX));

    // Same version keeps the data, another version drops it.
    assert_eq!(store.open_index(IDX, "1"), Ok(true));
    assert!(!store.get(IDX, b"a").is_empty());
    assert_eq!(store.open_index(IDX, "2"), Ok(false), "version bump");
    assert!(store.get(IDX, b"a").is_empty(), "rebuilt from scratch");
    assert_eq!(store.indexed_hash(IDX, f2), None);
    assert!(!store.is_complete(IDX));
    assert!(store.keys(IDX).is_empty());
    store.put(OTHER, f1, vec![kv(b"keep", b"")]);
    store.commit(OTHER);
    assert_eq!(store.get(OTHER, b"keep").len(), 1, "other index untouched");

    // clear
    store.clear(OTHER);
    assert!(store.get(OTHER, b"keep").is_empty());
    assert!(store.forward(OTHER, f1).is_empty());

    // File table.
    let record = |id: u32, path: &str| FileRecord {
        id: FileId(id),
        path: PathBuf::from(path),
        len: 10 + u64::from(id),
        mtime_ns: Some(1_700_000_000_000_000_000),
        hash: 7,
    };
    store.save_files(&[record(1, "a/b.rs"), record(5, "c.java")], &[]);
    store.save_files(
        &[FileRecord {
            mtime_ns: None,
            ..record(2, "d.kt")
        }],
        &[FileId(1)],
    );
    let mut files = store.load_files();
    files.sort_by_key(|r| r.id);
    assert_eq!(files.iter().map(|r| r.id.0).collect::<Vec<_>>(), vec![2, 5]);
    assert_eq!(files[0].mtime_ns, None);
    assert_eq!(files[1], record(5, "c.java"));
    assert!(store.take_failed().is_empty());
}

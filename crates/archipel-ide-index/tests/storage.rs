//! Storage conformance on both backends, persistence across reopen,
//! corruption and truncation recovery.

use archipel_ide_analysis::index::IndexStore;
use archipel_ide_analysis::{FileId, IndexId};
use archipel_ide_index::store::{DurableStore, MemStore, StoreKind};
use archipel_ide_index::{RedbStore, conformance};

#[test]
fn memory_store_conforms() {
    conformance::run(&MemStore::new());
}

#[test]
fn redb_store_conforms() {
    let dir = tempfile::tempdir().expect("dir");
    let store = RedbStore::open(dir.path()).expect("open");
    assert_eq!(store.kind(), StoreKind::Persistent);
    conformance::run(&store);
}

const IDX: IndexId = IndexId("Words");

fn fill(store: &RedbStore) {
    assert_eq!(store.open_index(IDX, "1"), Ok(false));
    for file in 0..200u32 {
        store.put(
            IDX,
            FileId(file),
            vec![
                (format!("key{}", file % 7).into_bytes(), vec![file as u8]),
                (b"all".to_vec(), Vec::new()),
            ],
        );
        store.set_indexed_hash(IDX, FileId(file), Some(u64::from(file) + 1));
    }
    store.commit(IDX);
    store.set_complete(IDX, true);
}

#[test]
fn data_survives_a_restart() {
    let dir = tempfile::tempdir().expect("dir");
    {
        let store = RedbStore::open(dir.path()).expect("open");
        fill(&store);
        store.flush();
    }
    let store = RedbStore::open(dir.path()).expect("reopen");
    assert_eq!(store.open_index(IDX, "1"), Ok(true), "kept");
    assert!(store.is_complete(IDX));
    assert_eq!(store.get(IDX, b"all").len(), 200);
    assert_eq!(store.indexed_hash(IDX, FileId(5)), Some(6));
}

#[test]
fn a_truncated_index_file_is_rebuilt_without_error() {
    let dir = tempfile::tempdir().expect("dir");
    let path = {
        let store = RedbStore::open(dir.path()).expect("open");
        fill(&store);
        store.flush();
        store.index_path(IDX)
    };
    let len = std::fs::metadata(&path).expect("meta").len();
    assert!(len > 8192);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("open file");
    file.set_len(len / 2).expect("truncate");
    drop(file);
    let store = RedbStore::open(dir.path()).expect("reopen never fails");
    let kept = store.open_index(IDX, "1").expect("open index never fails");
    assert!(!kept, "damaged index is dropped");
    assert!(!store.is_complete(IDX));
    assert!(store.get(IDX, b"all").is_empty());
    // And it is usable again.
    store.put(IDX, FileId(1), vec![(b"x".to_vec(), Vec::new())]);
    store.commit(IDX);
    assert_eq!(store.get(IDX, b"x").len(), 1);
}

#[test]
fn a_garbage_index_file_is_rebuilt_without_error() {
    let dir = tempfile::tempdir().expect("dir");
    let path = {
        let store = RedbStore::open(dir.path()).expect("open");
        store.index_path(IDX)
    };
    std::fs::write(&path, vec![0xAB; 20_000]).expect("garbage");
    let store = RedbStore::open(dir.path()).expect("open");
    assert_eq!(store.open_index(IDX, "1"), Ok(false));
    store.put(IDX, FileId(3), vec![(b"k".to_vec(), Vec::new())]);
    store.commit(IDX);
    assert_eq!(store.get(IDX, b"k").len(), 1);
}

#[test]
fn a_garbage_file_table_resets_the_whole_store() {
    let dir = tempfile::tempdir().expect("dir");
    {
        let store = RedbStore::open(dir.path()).expect("open");
        fill(&store);
        store.flush();
    }
    std::fs::write(dir.path().join("files.redb"), b"not a database").expect("garbage");
    let store = RedbStore::open(dir.path()).expect("open");
    assert_eq!(
        store.open_index(IDX, "1"),
        Ok(false),
        "stale indexes dropped with the table"
    );
    assert!(store.load_files().is_empty());
}

#[test]
fn a_version_bump_rebuilds_only_that_index() {
    let dir = tempfile::tempdir().expect("dir");
    let other = IndexId("Trigram");
    {
        let store = RedbStore::open(dir.path()).expect("open");
        fill(&store);
        store.open_index(other, "1").expect("open");
        store.put(other, FileId(1), vec![(b"abc".to_vec(), Vec::new())]);
        store.commit(other);
        store.flush();
    }
    let store = RedbStore::open(dir.path()).expect("reopen");
    assert_eq!(store.open_index(IDX, "2"), Ok(false), "bumped");
    assert_eq!(store.open_index(other, "1"), Ok(true), "untouched");
    assert_eq!(store.get(other, b"abc").len(), 1);
    assert!(store.get(IDX, b"all").is_empty());
}

#[test]
fn an_open_index_file_is_never_deleted_by_a_second_opener() {
    let dir = tempfile::tempdir().expect("dir");
    let first = RedbStore::open(dir.path()).expect("open");
    fill(&first);
    // A second store on the same directory (another window): the files are
    // locked, so it reports an error instead of replacing them.
    assert!(RedbStore::open(dir.path()).is_err());
    assert_eq!(first.get(IDX, b"all").len(), 200, "first store intact");
}

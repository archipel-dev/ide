//! Storage contracts of the index layer.
//!
//! The byte-level [`IndexStore`] trait (inverted map + forward map) lives in
//! `archipel-ide-analysis` so other packages can read indexes without this
//! crate. [`DurableStore`] adds what the indexing scheduler needs on top:
//! per-index versions and completeness, the "indexed content hash" per file
//! (the skip-unchanged check) and the persistent file table, so file ids stay
//! stable across restarts. [`MemStore`] is the in-memory implementation used
//! by tests, by projects without a cache directory and as the fallback when
//! the persistent store cannot be opened.

use archipel_ide_analysis::index::{IndexStore, MemIndexStore};
use archipel_ide_analysis::{FileId, IndexId, IndexStamp};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

/// Which backend a store is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreKind {
    Memory,
    Persistent,
}

/// What the scheduler remembers about one file between runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecord {
    pub id: FileId,
    /// Path in the form the analysis host's file table uses (project-relative).
    pub path: PathBuf,
    pub len: u64,
    /// Modification time as nanoseconds since the epoch.
    pub mtime_ns: Option<u64>,
    /// Content hash at the time the file was last read.
    pub hash: u64,
}

impl FileRecord {
    pub fn mtime(time: Option<SystemTime>) -> Option<u64> {
        let nanos = time?
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()?
            .as_nanos();
        u64::try_from(nanos).ok()
    }
}

/// An [`IndexStore`] with versions, completeness, content hashes and a file
/// table. Writes (`put`, `remove_file`, `set_indexed_hash`) become visible to
/// readers at `commit` (MVCC: a reader sees the last committed state).
pub trait DurableStore: IndexStore {
    /// Prepares `index` for `version`. Data stored under a different version
    /// (or a damaged file) is dropped. Returns true when existing data of
    /// the same version is kept.
    fn open_index(&self, index: IndexId, version: &str) -> Result<bool, StoreError>;
    /// Content hash `file` had when it was last indexed into `index`.
    fn indexed_hash(&self, index: IndexId, file: FileId) -> Option<u64>;
    /// Records (or with `None` forgets) the indexed hash; pending until `commit`.
    fn set_indexed_hash(&self, index: IndexId, file: FileId, hash: Option<u64>);
    /// The index finished its initial build (it may be queried after a restart).
    fn is_complete(&self, index: IndexId) -> bool;
    fn set_complete(&self, index: IndexId, complete: bool);
    /// Persistent file table.
    fn load_files(&self) -> Vec<FileRecord>;
    fn save_files(&self, upserts: &[FileRecord], removed: &[FileId]);
    /// Indexes whose storage reported an error since the last call (the
    /// scheduler drops and rebuilds them).
    fn take_failed(&self) -> Vec<IndexId>;
    fn kind(&self) -> StoreKind;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    #[error("cannot open the index store at {path}: {message}")]
    Open { path: PathBuf, message: String },
    #[error("index store error: {0}")]
    Storage(String),
}

#[derive(Default)]
struct MemMeta {
    versions: HashMap<IndexId, String>,
    complete: HashMap<IndexId, bool>,
    hashes: HashMap<(IndexId, FileId), u64>,
    files: HashMap<FileId, FileRecord>,
}

/// In-memory [`DurableStore`] (writes are visible immediately).
#[derive(Default)]
pub struct MemStore {
    inner: MemIndexStore,
    meta: Mutex<MemMeta>,
}

impl MemStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl IndexStore for MemStore {
    fn put(&self, index: IndexId, file: FileId, entries: Vec<(Vec<u8>, Vec<u8>)>) {
        self.inner.put(index, file, entries);
    }
    fn remove_file(&self, index: IndexId, file: FileId) {
        self.inner.remove_file(index, file);
    }
    fn get(&self, index: IndexId, key: &[u8]) -> Vec<(FileId, Vec<u8>)> {
        self.inner.get(index, key)
    }
    fn keys(&self, index: IndexId) -> Vec<Vec<u8>> {
        self.inner.keys(index)
    }
    fn forward(&self, index: IndexId, file: FileId) -> Vec<Vec<u8>> {
        self.inner.forward(index, file)
    }
    fn commit(&self, index: IndexId) -> IndexStamp {
        self.inner.commit(index)
    }
    fn clear(&self, index: IndexId) {
        self.inner.clear(index);
        let mut meta = self.meta.lock();
        meta.hashes.retain(|(id, _), _| *id != index);
        meta.complete.remove(&index);
    }
}

impl DurableStore for MemStore {
    fn open_index(&self, index: IndexId, version: &str) -> Result<bool, StoreError> {
        let meta = self.meta.lock();
        let same = meta.versions.get(&index).is_some_and(|v| v == version);
        if !same {
            drop(meta);
            self.clear(index);
            self.meta.lock().versions.insert(index, version.to_string());
        }
        Ok(same)
    }
    fn indexed_hash(&self, index: IndexId, file: FileId) -> Option<u64> {
        self.meta.lock().hashes.get(&(index, file)).copied()
    }
    fn set_indexed_hash(&self, index: IndexId, file: FileId, hash: Option<u64>) {
        let mut meta = self.meta.lock();
        match hash {
            Some(hash) => meta.hashes.insert((index, file), hash),
            None => meta.hashes.remove(&(index, file)),
        };
    }
    fn is_complete(&self, index: IndexId) -> bool {
        self.meta
            .lock()
            .complete
            .get(&index)
            .copied()
            .unwrap_or(false)
    }
    fn set_complete(&self, index: IndexId, complete: bool) {
        self.meta.lock().complete.insert(index, complete);
    }
    fn load_files(&self) -> Vec<FileRecord> {
        let mut files: Vec<_> = self.meta.lock().files.values().cloned().collect();
        files.sort_by_key(|record| record.id);
        files
    }
    fn save_files(&self, upserts: &[FileRecord], removed: &[FileId]) {
        let mut meta = self.meta.lock();
        for record in upserts {
            meta.files.insert(record.id, record.clone());
        }
        for id in removed {
            meta.files.remove(id);
        }
    }
    fn take_failed(&self) -> Vec<IndexId> {
        Vec::new()
    }
    fn kind(&self) -> StoreKind {
        StoreKind::Memory
    }
}

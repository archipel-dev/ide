//! Persistent [`DurableStore`] on `redb` (pure Rust, MVCC, crash safe,
//! MIT OR Apache-2.0). One database file per index so a version bump or a
//! damaged file rebuilds only that index, plus `files.redb` with the file
//! table.
//!
//! Layout of an index database:
//! - `inv`: `key ++ file_id_be` -> value (the inverted map; one row per posting),
//! - `fwd`: `file_id` -> the file's keys (the forward map, for removal),
//! - `hash`: `file_id` -> content hash the entries were computed from,
//! - `meta`: `version`, `complete`.
//!
//! Writes are buffered per index and applied in one transaction by `commit`
//! (the batch boundary of the scheduler); readers see the last commit.
//! A database that cannot be opened is deleted and recreated empty; one that
//! fails while in use is reported by [`DurableStore::take_failed`] so the
//! scheduler drops and rebuilds that index.

use crate::codec::{Reader, Writer};
use crate::store::{DurableStore, FileRecord, StoreError, StoreKind};
use archipel_ide_analysis::index::IndexStore;
use archipel_ide_analysis::{FileId, IndexId, IndexStamp};
use parking_lot::{Mutex, RwLock};
use redb::{Database, Durability, ReadableDatabase, ReadableTable, TableDefinition};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Format of the whole store; a different value discards the directory.
pub const STORE_FORMAT: u32 = 1;

/// Page cache per database file (redb defaults to 1 GiB each).
const CACHE_BYTES: usize = 24 * 1024 * 1024;

const INV: TableDefinition<&[u8], &[u8]> = TableDefinition::new("inv");
const FWD: TableDefinition<u32, &[u8]> = TableDefinition::new("fwd");
const HASH: TableDefinition<u32, u64> = TableDefinition::new("hash");
const META: TableDefinition<&str, &[u8]> = TableDefinition::new("meta");
const FILES: TableDefinition<u32, &[u8]> = TableDefinition::new("files");

enum Op {
    Put(FileId, Vec<(Vec<u8>, Vec<u8>)>),
    Remove(FileId),
    Hash(FileId, Option<u64>),
}

struct IndexDb {
    db: Database,
    pending: Mutex<Vec<Op>>,
    commit_lock: Mutex<()>,
    stamp: AtomicU64,
}

/// See the module docs.
pub struct RedbStore {
    dir: PathBuf,
    indexes: RwLock<HashMap<IndexId, Arc<IndexDb>>>,
    files: Mutex<Option<Database>>,
    failed: Mutex<Vec<IndexId>>,
}

fn composite(key: &[u8], file: FileId) -> Vec<u8> {
    let mut out = Vec::with_capacity(key.len() + 4);
    out.extend_from_slice(key);
    out.extend_from_slice(&file.0.to_be_bytes());
    out
}

fn encode_keys(keys: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut w = Writer::new(&mut out);
    w.varint(keys.len() as u64);
    for key in keys {
        w.blob(key);
    }
    out
}

fn decode_keys(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut r = Reader::new(bytes);
    let Some(count) = r.varint() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for _ in 0..count {
        let Some(key) = r.blob() else { break };
        out.push(key.to_vec());
    }
    out
}

fn sanitize(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Runs `f`, turning a panic inside redb (it asserts on truncated files)
/// into a storage error: a damaged index is rebuilt, never a crash.
fn guarded<T>(f: impl FnOnce() -> Result<T, redb::Error>) -> Result<T, redb::Error> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .unwrap_or_else(|_| Err(redb::Error::Corrupted("storage panicked".into())))
}

fn open_database(path: &Path) -> Result<Database, StoreError> {
    let open = |path: &Path| -> Result<Database, redb::Error> {
        guarded(|| {
            redb::Builder::new()
                .set_cache_size(CACHE_BYTES)
                .create(path)
                .map_err(redb::Error::from)
        })
    };
    match open(path) {
        Ok(db) => Ok(db),
        // Another process (or this one) uses the file: never delete it.
        Err(error @ redb::Error::DatabaseAlreadyOpen) => Err(StoreError::Open {
            path: path.to_path_buf(),
            message: error.to_string(),
        }),
        Err(_) => {
            // Damaged or foreign file: a corrupted index is dropped and
            // rebuilt, never a startup failure.
            let _ = std::fs::remove_file(path);
            open(path).map_err(|error| StoreError::Open {
                path: path.to_path_buf(),
                message: error.to_string(),
            })
        }
    }
}

fn is_missing_table(error: &redb::Error) -> bool {
    matches!(error, redb::Error::TableDoesNotExist(_))
}

fn read_meta(db: &Database, key: &str) -> Option<Vec<u8>> {
    let run = || -> Result<Option<Vec<u8>>, redb::Error> {
        let txn = db.begin_read()?;
        let table = txn.open_table(META)?;
        Ok(table.get(key)?.map(|guard| guard.value().to_vec()))
    };
    guarded(run).ok().flatten()
}

fn write_meta(db: &Database, entries: &[(&str, Option<&[u8]>)]) -> Result<(), redb::Error> {
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(META)?;
        for (key, value) in entries {
            match value {
                Some(value) => {
                    table.insert(*key, *value)?;
                }
                None => {
                    table.remove(*key)?;
                }
            }
        }
    }
    txn.commit()?;
    Ok(())
}

fn wipe(db: &Database) -> Result<(), redb::Error> {
    let txn = db.begin_write()?;
    txn.delete_table(INV)?;
    txn.delete_table(FWD)?;
    txn.delete_table(HASH)?;
    txn.delete_table(META)?;
    txn.open_table(INV)?;
    txn.open_table(FWD)?;
    txn.open_table(HASH)?;
    txn.open_table(META)?;
    txn.commit()?;
    Ok(())
}

fn apply(db: &Database, ops: Vec<Op>) -> Result<(), redb::Error> {
    guarded(|| apply_ops(db, ops))
}

fn apply_ops(db: &Database, ops: Vec<Op>) -> Result<(), redb::Error> {
    let mut txn = db.begin_write()?;
    txn.set_durability(Durability::None)?;
    {
        let mut inv = txn.open_table(INV)?;
        let mut fwd = txn.open_table(FWD)?;
        let mut hash = txn.open_table(HASH)?;
        let drop_old = |inv: &mut redb::Table<'_, &[u8], &[u8]>,
                        fwd: &mut redb::Table<'_, u32, &[u8]>,
                        file: FileId|
         -> Result<(), redb::Error> {
            let old = fwd.remove(file.0)?.map(|guard| guard.value().to_vec());
            if let Some(old) = old {
                for key in decode_keys(&old) {
                    inv.remove(composite(&key, file).as_slice())?;
                }
            }
            Ok(())
        };
        for op in ops {
            match op {
                Op::Put(file, mut entries) => {
                    drop_old(&mut inv, &mut fwd, file)?;
                    entries.sort_by(|a, b| a.0.cmp(&b.0));
                    let mut keys = Vec::with_capacity(entries.len());
                    for (key, value) in entries {
                        inv.insert(composite(&key, file).as_slice(), value.as_slice())?;
                        keys.push(key);
                    }
                    if !keys.is_empty() {
                        fwd.insert(file.0, encode_keys(&keys).as_slice())?;
                    }
                }
                Op::Remove(file) => drop_old(&mut inv, &mut fwd, file)?,
                Op::Hash(file, Some(value)) => {
                    hash.insert(file.0, value)?;
                }
                Op::Hash(file, None) => {
                    hash.remove(file.0)?;
                }
            }
        }
    }
    txn.commit()?;
    Ok(())
}

impl RedbStore {
    /// Opens (creating when needed) the store in `dir`. A directory written
    /// by another [`STORE_FORMAT`] is emptied.
    pub fn open(dir: &Path) -> Result<Self, StoreError> {
        std::fs::create_dir_all(dir).map_err(|error| StoreError::Open {
            path: dir.to_path_buf(),
            message: error.to_string(),
        })?;
        let files = open_database(&dir.join("files.redb"))?;
        let format = STORE_FORMAT.to_string();
        let current = read_meta(&files, "format");
        if current.as_deref() != Some(format.as_bytes()) {
            // Different or unknown format: start over (index files too).
            drop(files);
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().is_some_and(|ext| ext == "redb") {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
            let files = open_database(&dir.join("files.redb"))?;
            let _ = write_meta(&files, &[("format", Some(format.as_bytes()))]);
            return Ok(Self::with_files(dir, files));
        }
        Ok(Self::with_files(dir, files))
    }

    fn with_files(dir: &Path, files: Database) -> Self {
        Self {
            dir: dir.to_path_buf(),
            indexes: RwLock::new(HashMap::new()),
            files: Mutex::new(Some(files)),
            failed: Mutex::new(Vec::new()),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Path of the database file of `index`.
    pub fn index_path(&self, index: IndexId) -> PathBuf {
        self.dir.join(format!("{}.redb", sanitize(index.0)))
    }

    fn db(&self, index: IndexId) -> Option<Arc<IndexDb>> {
        self.indexes.read().get(&index).cloned()
    }

    fn fail(&self, index: IndexId) {
        let mut failed = self.failed.lock();
        if !failed.contains(&index) {
            failed.push(index);
        }
    }

    fn read<T: Default>(
        &self,
        index: IndexId,
        f: impl FnOnce(&redb::ReadTransaction) -> Result<T, redb::Error>,
    ) -> T {
        let Some(db) = self.db(index) else {
            return T::default();
        };
        let result = guarded(|| {
            let txn = db.db.begin_read()?;
            f(&txn)
        });
        match result {
            Ok(value) => value,
            Err(error) => {
                if !is_missing_table(&error) {
                    self.fail(index);
                }
                T::default()
            }
        }
    }
}

impl IndexStore for RedbStore {
    fn put(&self, index: IndexId, file: FileId, entries: Vec<(Vec<u8>, Vec<u8>)>) {
        if let Some(db) = self.db(index) {
            db.pending.lock().push(Op::Put(file, entries));
        }
    }

    fn remove_file(&self, index: IndexId, file: FileId) {
        if let Some(db) = self.db(index) {
            db.pending.lock().push(Op::Remove(file));
        }
    }

    fn get(&self, index: IndexId, key: &[u8]) -> Vec<(FileId, Vec<u8>)> {
        self.read(index, |txn| {
            let table = txn.open_table(INV)?;
            let lo = composite(key, FileId(0));
            let hi = composite(key, FileId(u32::MAX));
            let mut out = Vec::new();
            for row in table.range(lo.as_slice()..=hi.as_slice())? {
                let (k, v) = row?;
                let k = k.value();
                if k.len() != key.len() + 4 {
                    continue;
                }
                let mut id = [0u8; 4];
                id.copy_from_slice(&k[key.len()..]);
                out.push((FileId(u32::from_be_bytes(id)), v.value().to_vec()));
            }
            Ok(out)
        })
    }

    fn keys(&self, index: IndexId) -> Vec<Vec<u8>> {
        self.read(index, |txn| {
            let table = txn.open_table(INV)?;
            let mut out: Vec<Vec<u8>> = Vec::new();
            for row in table.iter()? {
                let (k, _) = row?;
                let k = k.value();
                if k.len() < 4 {
                    continue;
                }
                let key = &k[..k.len() - 4];
                if out.last().is_none_or(|last| last.as_slice() != key) {
                    out.push(key.to_vec());
                }
            }
            Ok(out)
        })
    }

    fn forward(&self, index: IndexId, file: FileId) -> Vec<Vec<u8>> {
        self.read(index, |txn| {
            let table = txn.open_table(FWD)?;
            Ok(table
                .get(file.0)?
                .map(|guard| decode_keys(guard.value()))
                .unwrap_or_default())
        })
    }

    fn commit(&self, index: IndexId) -> IndexStamp {
        let Some(db) = self.db(index) else {
            return IndexStamp(0);
        };
        let _serial = db.commit_lock.lock();
        let ops = std::mem::take(&mut *db.pending.lock());
        if !ops.is_empty()
            && let Err(error) = apply(&db.db, ops)
        {
            let _ = error;
            self.fail(index);
        }
        IndexStamp(db.stamp.fetch_add(1, Ordering::SeqCst) + 1)
    }

    fn clear(&self, index: IndexId) {
        let Some(db) = self.db(index) else {
            return;
        };
        let _serial = db.commit_lock.lock();
        db.pending.lock().clear();
        if wipe(&db.db).is_err() {
            self.fail(index);
        }
        db.stamp.fetch_add(1, Ordering::SeqCst);
    }
}

impl DurableStore for RedbStore {
    fn open_index(&self, index: IndexId, version: &str) -> Result<bool, StoreError> {
        if let Some(open) = self.db(index) {
            // Already open (the file is locked): only the version can differ.
            let _serial = open.commit_lock.lock();
            let same = read_meta(&open.db, "version").as_deref() == Some(version.as_bytes());
            if !same {
                open.pending.lock().clear();
                let reset = wipe(&open.db)
                    .and_then(|()| write_meta(&open.db, &[("version", Some(version.as_bytes()))]));
                if reset.is_err() {
                    self.fail(index);
                }
            }
            return Ok(same);
        }
        let path = self.index_path(index);
        let db = open_database(&path)?;
        let same = read_meta(&db, "version").as_deref() == Some(version.as_bytes());
        if !same {
            let reset =
                wipe(&db).and_then(|()| write_meta(&db, &[("version", Some(version.as_bytes()))]));
            if let Err(error) = reset {
                // Unusable file: replace it.
                drop(db);
                let _ = std::fs::remove_file(&path);
                let db = open_database(&path)?;
                write_meta(&db, &[("version", Some(version.as_bytes()))])
                    .map_err(|error| StoreError::Storage(error.to_string()))?;
                self.install(index, db);
                let _ = error;
                return Ok(false);
            }
        } else {
            // Probe the tables the first read would use; a truncated file
            // that still opens fails here and is replaced.
            let probe = || -> Result<(), redb::Error> {
                let txn = db.begin_read()?;
                let inv = txn.open_table(INV)?;
                let _ = inv.first()?;
                let fwd = txn.open_table(FWD)?;
                let _ = fwd.last()?;
                let hash = txn.open_table(HASH)?;
                let _ = hash.last()?;
                Ok(())
            };
            if guarded(probe).is_err() {
                drop(db);
                let _ = std::fs::remove_file(&path);
                let db = open_database(&path)?;
                write_meta(&db, &[("version", Some(version.as_bytes()))])
                    .map_err(|error| StoreError::Storage(error.to_string()))?;
                self.install(index, db);
                return Ok(false);
            }
        }
        self.install(index, db);
        Ok(same)
    }

    fn indexed_hash(&self, index: IndexId, file: FileId) -> Option<u64> {
        self.read(index, |txn| {
            let table = txn.open_table(HASH)?;
            Ok(table.get(file.0)?.map(|guard| guard.value()))
        })
    }

    fn set_indexed_hash(&self, index: IndexId, file: FileId, hash: Option<u64>) {
        if let Some(db) = self.db(index) {
            db.pending.lock().push(Op::Hash(file, hash));
        }
    }

    fn is_complete(&self, index: IndexId) -> bool {
        self.db(index)
            .and_then(|db| read_meta(&db.db, "complete"))
            .is_some_and(|value| value == [1])
    }

    fn set_complete(&self, index: IndexId, complete: bool) {
        let Some(db) = self.db(index) else {
            return;
        };
        let value: &[u8] = &[1];
        let entry = if complete { Some(value) } else { None };
        if write_meta(&db.db, &[("complete", entry)]).is_err() {
            self.fail(index);
        }
    }

    fn load_files(&self) -> Vec<FileRecord> {
        let guard = self.files.lock();
        let Some(db) = guard.as_ref() else {
            return Vec::new();
        };
        let run = || -> Result<Vec<FileRecord>, redb::Error> {
            let txn = db.begin_read()?;
            let table = txn.open_table(FILES)?;
            let mut out = Vec::new();
            for row in table.iter()? {
                let (id, value) = row?;
                if let Some(record) = decode_record(FileId(id.value()), value.value()) {
                    out.push(record);
                }
            }
            Ok(out)
        };
        run().unwrap_or_default()
    }

    fn save_files(&self, upserts: &[FileRecord], removed: &[FileId]) {
        let guard = self.files.lock();
        let Some(db) = guard.as_ref() else {
            return;
        };
        let run = || -> Result<(), redb::Error> {
            let mut txn = db.begin_write()?;
            txn.set_durability(Durability::None)?;
            {
                let mut table = txn.open_table(FILES)?;
                for id in removed {
                    table.remove(id.0)?;
                }
                for record in upserts {
                    table.insert(record.id.0, encode_record(record).as_slice())?;
                }
            }
            txn.commit()?;
            Ok(())
        };
        let _ = run();
    }

    fn take_failed(&self) -> Vec<IndexId> {
        std::mem::take(&mut *self.failed.lock())
    }

    fn kind(&self) -> StoreKind {
        StoreKind::Persistent
    }
}

impl RedbStore {
    fn install(&self, index: IndexId, db: Database) {
        self.indexes.write().insert(
            index,
            Arc::new(IndexDb {
                db,
                pending: Mutex::new(Vec::new()),
                commit_lock: Mutex::new(()),
                stamp: AtomicU64::new(0),
            }),
        );
    }

    /// Forces everything committed so far to disk (an `Immediate` commit).
    pub fn flush(&self) {
        for db in self.indexes.read().values() {
            let run = || -> Result<(), redb::Error> {
                let txn = db.db.begin_write()?;
                txn.commit()?;
                Ok(())
            };
            let _ = run();
        }
        if let Some(db) = self.files.lock().as_ref() {
            let run = || -> Result<(), redb::Error> {
                let txn = db.begin_write()?;
                txn.commit()?;
                Ok(())
            };
            let _ = run();
        }
    }
}

fn encode_record(record: &FileRecord) -> Vec<u8> {
    let mut out = Vec::new();
    let mut w = Writer::new(&mut out);
    w.str(&record.path.to_string_lossy());
    w.varint(record.len);
    match record.mtime_ns {
        Some(mtime) => {
            w.u8(1);
            w.varint(mtime);
        }
        None => w.u8(0),
    }
    w.varint(record.hash);
    out
}

fn decode_record(id: FileId, bytes: &[u8]) -> Option<FileRecord> {
    let mut r = Reader::new(bytes);
    let path = PathBuf::from(r.str()?);
    let len = r.varint()?;
    let mtime_ns = if r.u8()? == 1 {
        Some(r.varint()?)
    } else {
        None
    };
    let hash = r.varint()?;
    Some(FileRecord {
        id,
        path,
        len,
        mtime_ns,
        hash,
    })
}

//! The revision log on `redb`.
//!
//! Tables: `revs` (`path 0x00 id_be` -> revision), `by_time` (`time_be id_be`
//! -> path) for the recent-changes view and retention, `refs` (blob id ->
//! number of revisions using it) so unused blobs are deleted, `meta`.

use super::blob::{BlobError, BlobId, BlobStore};
use crate::codec::{Reader, Writer};
use parking_lot::Mutex;
use redb::{Database, Durability, ReadableDatabase, ReadableTable, TableDefinition};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const REVS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("revs");
const BY_TIME: TableDefinition<&[u8], &[u8]> = TableDefinition::new("by_time");
const REFS: TableDefinition<&[u8], u32> = TableDefinition::new("refs");
const META: TableDefinition<&str, u64> = TableDefinition::new("meta");

const FORMAT: u64 = 1;

/// Why a revision exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cause {
    /// The content before the first save was recorded.
    Initial,
    Save,
    /// The file changed outside the editor.
    External,
    /// Before a refactoring (its label).
    Refactoring(String),
    /// Before a VCS operation (its name).
    Vcs(String),
    /// A manual label (not tied to content).
    Label(String),
    /// A revert (the content that was replaced is kept as a revision).
    Revert,
    /// The file was deleted.
    Deleted,
}

impl Cause {
    pub fn label(&self) -> String {
        match self {
            Cause::Initial => "Initial".into(),
            Cause::Save => "Saved".into(),
            Cause::External => "External change".into(),
            Cause::Refactoring(name) => format!("Refactoring: {name}"),
            Cause::Vcs(name) => format!("VCS: {name}"),
            Cause::Label(text) => format!("Label: {text}"),
            Cause::Revert => "Before revert".into(),
            Cause::Deleted => "Deleted".into(),
        }
    }

    fn encode(&self, w: &mut Writer<'_>) {
        let (tag, text) = match self {
            Cause::Initial => (0, ""),
            Cause::Save => (1, ""),
            Cause::External => (2, ""),
            Cause::Refactoring(t) => (3, t.as_str()),
            Cause::Vcs(t) => (4, t.as_str()),
            Cause::Label(t) => (5, t.as_str()),
            Cause::Revert => (6, ""),
            Cause::Deleted => (7, ""),
        };
        w.u8(tag);
        w.str(text);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let tag = r.u8()?;
        let text = r.str()?.to_string();
        Some(match tag {
            0 => Cause::Initial,
            1 => Cause::Save,
            2 => Cause::External,
            3 => Cause::Refactoring(text),
            4 => Cause::Vcs(text),
            5 => Cause::Label(text),
            6 => Cause::Revert,
            7 => Cause::Deleted,
            _ => return None,
        })
    }
}

/// One entry of the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    /// Monotonic id (newer revisions have larger ids).
    pub id: u64,
    /// Project-relative path (empty for project-wide labels).
    pub path: PathBuf,
    /// Milliseconds since the epoch.
    pub time_ms: u64,
    /// The content; `None` for deletions and labels.
    pub blob: Option<BlobId>,
    pub len: u64,
    pub cause: Cause,
}

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("local history store: {0}")]
    Store(String),
    #[error(transparent)]
    Blob(#[from] BlobError),
}

impl From<redb::Error> for HistoryError {
    fn from(error: redb::Error) -> Self {
        HistoryError::Store(error.to_string())
    }
}

macro_rules! store_err {
    ($($kind:ty),*) => {$(
        impl From<$kind> for HistoryError {
            fn from(error: $kind) -> Self {
                HistoryError::Store(error.to_string())
            }
        }
    )*};
}
store_err!(
    redb::TransactionError,
    redb::TableError,
    redb::StorageError,
    redb::CommitError,
    redb::DatabaseError
);

/// How long and how much history is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retention {
    pub max_age_ms: u64,
    /// Upper bound of the compressed blobs on disk; the oldest revisions go first.
    pub max_bytes: u64,
}

impl Default for Retention {
    fn default() -> Self {
        Self {
            max_age_ms: 5 * 24 * 3600 * 1000,
            max_bytes: 256 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PurgeStats {
    pub revisions_removed: usize,
    pub blobs_removed: usize,
}

fn rev_key(path: &Path, id: u64) -> Vec<u8> {
    let mut key = path.to_string_lossy().into_owned().into_bytes();
    key.push(0);
    key.extend_from_slice(&id.to_be_bytes());
    key
}

fn time_key(time_ms: u64, id: u64) -> [u8; 16] {
    let mut key = [0u8; 16];
    key[..8].copy_from_slice(&time_ms.to_be_bytes());
    key[8..].copy_from_slice(&id.to_be_bytes());
    key
}

fn encode_rev(rev: &Revision) -> Vec<u8> {
    let mut out = Vec::new();
    let mut w = Writer::new(&mut out);
    w.varint(rev.id);
    w.varint(rev.time_ms);
    match &rev.blob {
        Some(blob) => {
            w.u8(1);
            w.blob(&blob.0);
        }
        None => w.u8(0),
    }
    w.varint(rev.len);
    rev.cause.encode(&mut w);
    out
}

fn decode_rev(path: PathBuf, bytes: &[u8]) -> Option<Revision> {
    let mut r = Reader::new(bytes);
    let id = r.varint()?;
    let time_ms = r.varint()?;
    let blob = if r.u8()? == 1 {
        Some(BlobId::from_slice(r.blob()?)?)
    } else {
        None
    };
    let len = r.varint()?;
    let cause = Cause::decode(&mut r)?;
    Some(Revision {
        id,
        path,
        time_ms,
        blob,
        len,
        cause,
    })
}

/// The Local History of one project.
pub struct History {
    db: Database,
    blobs: BlobStore,
    clock: Arc<dyn Fn() -> u64 + Send + Sync>,
    write: Mutex<()>,
}

impl History {
    /// Opens (creating when needed) the history in `dir`. A damaged log is
    /// replaced by an empty one; the blobs stay and are reclaimed by `purge`.
    pub fn open(dir: &Path) -> Result<Self, HistoryError> {
        Self::open_with_clock(dir, Arc::new(now_ms))
    }

    pub fn open_with_clock(
        dir: &Path,
        clock: Arc<dyn Fn() -> u64 + Send + Sync>,
    ) -> Result<Self, HistoryError> {
        std::fs::create_dir_all(dir).map_err(|e| HistoryError::Store(e.to_string()))?;
        let blobs =
            BlobStore::open(&dir.join("blobs")).map_err(|e| HistoryError::Store(e.to_string()))?;
        let path = dir.join("revisions.redb");
        let open = |path: &Path| -> Result<Database, HistoryError> {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Database::create(path)))
                .map_err(|_| HistoryError::Store("log panicked".into()))?
                .map_err(HistoryError::from)
        };
        let healthy = |db: &Database| -> bool {
            let run = || -> Result<bool, redb::Error> {
                let txn = db.begin_read()?;
                let meta = txn.open_table(META)?;
                Ok(meta.get("format")?.map(|g| g.value()) == Some(FORMAT))
            };
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(run))
                .ok()
                .and_then(Result::ok)
                .unwrap_or(false)
        };
        let mut db = open(&path);
        if let Ok(opened) = &db {
            if !healthy(opened) && opened.begin_read().is_ok() {
                // New or foreign file: initialize below.
            }
        } else {
            let _ = std::fs::remove_file(&path);
            db = open(&path);
        }
        let db = db?;
        let init = || -> Result<(), redb::Error> {
            let txn = db.begin_write()?;
            {
                let mut meta = txn.open_table(META)?;
                if meta.get("format")?.map(|g| g.value()) != Some(FORMAT) {
                    meta.insert("format", FORMAT)?;
                    meta.insert("next_id", 1)?;
                }
                txn.open_table(REVS)?;
                txn.open_table(BY_TIME)?;
                txn.open_table(REFS)?;
            }
            txn.commit()?;
            Ok(())
        };
        let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(init))
            .ok()
            .and_then(Result::ok)
            .is_some();
        if !ok {
            // Damaged beyond use: start a new log.
            drop(db);
            let _ = std::fs::remove_file(&path);
            let db = open(&path)?;
            let txn = db.begin_write().map_err(redb::Error::from)?;
            {
                let mut meta = txn.open_table(META).map_err(redb::Error::from)?;
                meta.insert("format", FORMAT).map_err(redb::Error::from)?;
                meta.insert("next_id", 1).map_err(redb::Error::from)?;
                txn.open_table(REVS).map_err(redb::Error::from)?;
                txn.open_table(BY_TIME).map_err(redb::Error::from)?;
                txn.open_table(REFS).map_err(redb::Error::from)?;
            }
            txn.commit().map_err(redb::Error::from)?;
            return Ok(Self {
                db,
                blobs,
                clock,
                write: Mutex::new(()),
            });
        }
        Ok(Self {
            db,
            blobs,
            clock,
            write: Mutex::new(()),
        })
    }

    pub fn blobs(&self) -> &BlobStore {
        &self.blobs
    }

    fn insert(
        &self,
        path: &Path,
        blob: Option<BlobId>,
        len: u64,
        cause: Cause,
    ) -> Result<Revision, HistoryError> {
        let _serial = self.write.lock();
        let time_ms = (self.clock)();
        let mut txn = self.db.begin_write()?;
        txn.set_durability(Durability::None)
            .map_err(redb::Error::from)?;
        let rev = {
            let mut meta = txn.open_table(META)?;
            let id = meta.get("next_id")?.map_or(1, |g| g.value());
            meta.insert("next_id", id + 1)?;
            let rev = Revision {
                id,
                path: path.to_path_buf(),
                time_ms,
                blob,
                len,
                cause,
            };
            let mut revs = txn.open_table(REVS)?;
            revs.insert(rev_key(path, id).as_slice(), encode_rev(&rev).as_slice())?;
            let mut by_time = txn.open_table(BY_TIME)?;
            by_time.insert(
                time_key(time_ms, id).as_slice(),
                path.to_string_lossy().as_bytes(),
            )?;
            if let Some(blob) = &rev.blob {
                let mut refs = txn.open_table(REFS)?;
                let count = refs.get(blob.0.as_slice())?.map_or(0, |g| g.value());
                refs.insert(blob.0.as_slice(), count + 1)?;
            }
            rev
        };
        txn.commit()?;
        Ok(rev)
    }

    /// Records `content` of `path`. Returns `None` when it equals the latest
    /// revision of the path (nothing changed).
    pub fn record(
        &self,
        path: &Path,
        content: &[u8],
        cause: Cause,
    ) -> Result<Option<Revision>, HistoryError> {
        let id = BlobId::of(content);
        if let Some(latest) = self.latest(path)
            && latest.blob == Some(id)
        {
            return Ok(None);
        }
        let id = self.blobs.put(content)?;
        Ok(Some(self.insert(
            path,
            Some(id),
            content.len() as u64,
            cause,
        )?))
    }

    /// Records that `path` was deleted.
    pub fn record_deletion(&self, path: &Path) -> Result<Option<Revision>, HistoryError> {
        if self
            .latest(path)
            .is_some_and(|r| r.blob.is_none() && r.cause == Cause::Deleted)
        {
            return Ok(None);
        }
        Ok(Some(self.insert(path, None, 0, Cause::Deleted)?))
    }

    /// A manual label: a marker in the project's timeline.
    pub fn label(&self, text: &str) -> Result<Revision, HistoryError> {
        self.insert(Path::new(""), None, 0, Cause::Label(text.to_string()))
    }

    /// Revisions of `path`, newest first.
    pub fn revisions(&self, path: &Path) -> Vec<Revision> {
        let run = || -> Result<Vec<Revision>, redb::Error> {
            let txn = self.db.begin_read()?;
            let revs = txn.open_table(REVS)?;
            let lo = rev_key(path, 0);
            let hi = rev_key(path, u64::MAX);
            let mut out = Vec::new();
            for row in revs.range(lo.as_slice()..=hi.as_slice())? {
                let (_, value) = row?;
                if let Some(rev) = decode_rev(path.to_path_buf(), value.value()) {
                    out.push(rev);
                }
            }
            out.reverse();
            Ok(out)
        };
        run().unwrap_or_default()
    }

    pub fn latest(&self, path: &Path) -> Option<Revision> {
        self.revisions(path).into_iter().next()
    }

    /// The most recent revisions of the whole project, newest first.
    pub fn recent(&self, limit: usize) -> Vec<Revision> {
        let run = || -> Result<Vec<Revision>, redb::Error> {
            let txn = self.db.begin_read()?;
            let by_time = txn.open_table(BY_TIME)?;
            let revs = txn.open_table(REVS)?;
            let mut out = Vec::new();
            for row in by_time.iter()?.rev() {
                if out.len() >= limit {
                    break;
                }
                let (key, path) = row?;
                let key = key.value();
                let mut id = [0u8; 8];
                id.copy_from_slice(&key[8..16]);
                let id = u64::from_be_bytes(id);
                let path = PathBuf::from(String::from_utf8_lossy(path.value()).into_owned());
                let rk = rev_key(&path, id);
                if let Some(value) = revs.get(rk.as_slice())?
                    && let Some(rev) = decode_rev(path.clone(), value.value())
                {
                    out.push(rev);
                }
            }
            Ok(out)
        };
        run().unwrap_or_default()
    }

    /// The content of a revision (`None` for deletions and labels).
    pub fn content(&self, rev: &Revision) -> Result<Option<Vec<u8>>, HistoryError> {
        match &rev.blob {
            Some(blob) => Ok(Some(self.blobs.get(blob)?)),
            None => Ok(None),
        }
    }

    /// Every path that has revisions.
    pub fn paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = self
            .recent(usize::MAX)
            .into_iter()
            .map(|rev| rev.path)
            .filter(|path| !path.as_os_str().is_empty())
            .collect();
        paths.sort();
        paths.dedup();
        paths
    }

    fn remove_revisions(&self, doomed: &[Revision]) -> Result<PurgeStats, HistoryError> {
        let _serial = self.write.lock();
        let mut stats = PurgeStats::default();
        let mut gone: Vec<BlobId> = Vec::new();
        let mut txn = self.db.begin_write()?;
        txn.set_durability(Durability::None)
            .map_err(redb::Error::from)?;
        {
            let mut revs = txn.open_table(REVS)?;
            let mut by_time = txn.open_table(BY_TIME)?;
            let mut refs = txn.open_table(REFS)?;
            for rev in doomed {
                revs.remove(rev_key(&rev.path, rev.id).as_slice())?;
                by_time.remove(time_key(rev.time_ms, rev.id).as_slice())?;
                stats.revisions_removed += 1;
                if let Some(blob) = &rev.blob {
                    let count = refs.get(blob.0.as_slice())?.map_or(0, |g| g.value());
                    if count <= 1 {
                        refs.remove(blob.0.as_slice())?;
                        gone.push(*blob);
                    } else {
                        refs.insert(blob.0.as_slice(), count - 1)?;
                    }
                }
            }
        }
        txn.commit()?;
        for blob in gone {
            self.blobs.remove(&blob);
            stats.blobs_removed += 1;
        }
        Ok(stats)
    }

    /// Applies the retention policy: revisions older than `max_age_ms`, then
    /// the oldest revisions until the blobs fit in `max_bytes`; blobs no
    /// revision references are deleted.
    pub fn purge(&self, retention: Retention) -> Result<PurgeStats, HistoryError> {
        let now = (self.clock)();
        let all = self.recent(usize::MAX);
        let cutoff = now.saturating_sub(retention.max_age_ms);
        let (old, mut kept): (Vec<_>, Vec<_>) =
            all.into_iter().partition(|rev| rev.time_ms < cutoff);
        let mut stats = self.remove_revisions(&old)?;
        // Size: kept is newest first; evict from the end, but never the
        // newest revision of a path.
        let mut newest = std::collections::HashSet::new();
        let protected: std::collections::HashSet<u64> = kept
            .iter()
            .filter(|rev| newest.insert(rev.path.clone()))
            .map(|rev| rev.id)
            .collect();
        let mut total: u64 = self.blobs.list().iter().map(|(_, size)| size).sum();
        let mut doomed = Vec::new();
        while total > retention.max_bytes {
            let Some(rev) = kept.pop() else { break };
            if protected.contains(&rev.id) {
                continue;
            }
            if let Some(blob) = &rev.blob {
                total = total.saturating_sub(self.blobs.size(blob));
            }
            doomed.push(rev);
        }
        let more = self.remove_revisions(&doomed)?;
        stats.revisions_removed += more.revisions_removed;
        stats.blobs_removed += more.blobs_removed;
        // Orphans (a crash between blob write and log insert).
        let referenced: std::collections::HashSet<BlobId> = self
            .recent(usize::MAX)
            .into_iter()
            .filter_map(|rev| rev.blob)
            .collect();
        for (blob, _) in self.blobs.list() {
            if !referenced.contains(&blob) {
                self.blobs.remove(&blob);
                stats.blobs_removed += 1;
            }
        }
        Ok(stats)
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

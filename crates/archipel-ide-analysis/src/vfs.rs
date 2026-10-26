//! VFS facade: reads file contents for the analysis world (disk or memory)
//! and classifies watcher batches into [`VfsChange`]s. Open editor buffers
//! shadow disk contents through the [`DocumentOverlay`].

use crate::file::{FileId, FileRev, FileTable};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Files larger than this are not read for analysis (same cap as Find in Files).
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VfsError {
    #[error("{0} does not exist")]
    NotFound(PathBuf),
    #[error("{path} is too large to analyze ({len} bytes)")]
    TooLarge { path: PathBuf, len: u64 },
    #[error("{0} is binary")]
    Binary(PathBuf),
    #[error("cannot read {path}: {message}")]
    Io { path: PathBuf, message: String },
    #[error("unknown file id {0:?}")]
    UnknownFile(FileId),
}

/// Size and modification time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskMeta {
    pub len: u64,
    pub mtime: Option<std::time::SystemTime>,
}

/// Content source for files that are not open.
pub trait Vfs: Send + Sync {
    /// UTF-8 text of `path` (size-capped, binary files rejected).
    fn read(&self, path: &Path) -> Result<Arc<str>, VfsError>;
    fn metadata(&self, path: &Path) -> Option<DiskMeta>;
    fn exists(&self, path: &Path) -> bool {
        self.metadata(path).is_some()
    }
}

/// The real file system.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiskVfs;

/// True when `bytes` look binary (a NUL in the first 8 KiB).
pub fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8 * 1024).any(|byte| *byte == 0)
}

impl Vfs for DiskVfs {
    fn read(&self, path: &Path) -> Result<Arc<str>, VfsError> {
        let io = |error: std::io::Error| match error.kind() {
            std::io::ErrorKind::NotFound => VfsError::NotFound(path.to_path_buf()),
            _ => VfsError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            },
        };
        let len = std::fs::metadata(path).map_err(io)?.len();
        if len > MAX_FILE_BYTES {
            return Err(VfsError::TooLarge {
                path: path.to_path_buf(),
                len,
            });
        }
        let bytes = std::fs::read(path).map_err(io)?;
        if is_binary(&bytes) {
            return Err(VfsError::Binary(path.to_path_buf()));
        }
        Ok(String::from_utf8_lossy(&bytes).into())
    }

    fn metadata(&self, path: &Path) -> Option<DiskMeta> {
        let meta = std::fs::metadata(path).ok()?;
        meta.is_file().then(|| DiskMeta {
            len: meta.len(),
            mtime: meta.modified().ok(),
        })
    }
}

/// In-memory file system for tests and fixtures.
#[derive(Debug, Default)]
pub struct MemVfs {
    files: parking_lot::RwLock<HashMap<PathBuf, Arc<str>>>,
}

impl MemVfs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_files<'a>(files: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        let vfs = Self::new();
        for (path, text) in files {
            vfs.write(Path::new(path), text);
        }
        vfs
    }

    pub fn write(&self, path: &Path, text: &str) {
        self.files.write().insert(path.to_path_buf(), text.into());
    }

    pub fn delete(&self, path: &Path) {
        self.files.write().remove(path);
    }

    pub fn paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<_> = self.files.read().keys().cloned().collect();
        paths.sort();
        paths
    }
}

impl Vfs for MemVfs {
    fn read(&self, path: &Path) -> Result<Arc<str>, VfsError> {
        let text = self
            .files
            .read()
            .get(path)
            .cloned()
            .ok_or_else(|| VfsError::NotFound(path.to_path_buf()))?;
        if text.len() as u64 > MAX_FILE_BYTES {
            return Err(VfsError::TooLarge {
                path: path.to_path_buf(),
                len: text.len() as u64,
            });
        }
        Ok(text)
    }

    fn metadata(&self, path: &Path) -> Option<DiskMeta> {
        self.files.read().get(path).map(|text| DiskMeta {
            len: text.len() as u64,
            mtime: None,
        })
    }
}

/// A batch of file-system changes, coalesced (one batch per frame).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VfsChange {
    pub created: Vec<FileId>,
    pub deleted: Vec<FileId>,
    /// `(file, from, to)`: the id is kept.
    pub moved: Vec<(FileId, PathBuf, PathBuf)>,
    pub modified: Vec<FileId>,
}

impl VfsChange {
    pub fn is_empty(&self) -> bool {
        self.created.is_empty()
            && self.deleted.is_empty()
            && self.moved.is_empty()
            && self.modified.is_empty()
    }

    /// Every file the batch touches.
    pub fn files(&self) -> impl Iterator<Item = FileId> + '_ {
        self.created
            .iter()
            .chain(&self.deleted)
            .chain(&self.modified)
            .copied()
            .chain(self.moved.iter().map(|(file, _, _)| *file))
    }

    /// Classifies the paths a watcher reported (`ProjectWatcher::poll`'s
    /// `FileChanges::paths`): known and present = modified, known and gone =
    /// deleted, new and present = created; other paths (directories,
    /// vanished temp files) are ignored. Interns created paths.
    pub fn classify(
        table: &mut FileTable,
        vfs: &dyn Vfs,
        paths: impl IntoIterator<Item = PathBuf>,
    ) -> Self {
        let mut change = Self::default();
        let mut paths: Vec<PathBuf> = paths.into_iter().collect();
        paths.sort();
        paths.dedup();
        for path in paths {
            let exists = vfs.exists(&path);
            match (table.lookup(&path), exists) {
                (Some(id), true) => change.modified.push(id),
                (Some(id), false) => change.deleted.push(id),
                (None, true) => change.created.push(table.intern(&path)),
                (None, false) => {}
            }
        }
        change
    }
}

/// One open buffer.
#[derive(Debug, Clone)]
pub struct OverlayEntry {
    pub rev: FileRev,
    pub text: Arc<str>,
}

/// Open editor buffers shadow disk contents (IntelliJ: PSI is built from the
/// document, not the disk, while a file is open).
#[derive(Debug, Clone, Default)]
pub struct DocumentOverlay {
    open: HashMap<FileId, OverlayEntry>,
}

impl DocumentOverlay {
    pub fn set(&mut self, file: FileId, rev: FileRev, text: Arc<str>) {
        self.open.insert(file, OverlayEntry { rev, text });
    }

    pub fn close(&mut self, file: FileId) -> Option<OverlayEntry> {
        self.open.remove(&file)
    }

    pub fn get(&self, file: FileId) -> Option<&OverlayEntry> {
        self.open.get(&file)
    }

    pub fn is_open(&self, file: FileId) -> bool {
        self.open.contains_key(&file)
    }

    pub fn files(&self) -> impl Iterator<Item = FileId> + '_ {
        self.open.keys().copied()
    }

    pub fn len(&self) -> usize {
        self.open.len()
    }

    pub fn is_empty(&self) -> bool {
        self.open.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_batches_and_overlay() {
        let vfs = MemVfs::with_files([("/p/a.rs", "fn a() {}"), ("/p/new.rs", "")]);
        let mut table = FileTable::new();
        let a = table.intern(Path::new("/p/a.rs"));
        let gone = table.intern(Path::new("/p/gone.rs"));
        let change = VfsChange::classify(
            &mut table,
            &vfs,
            ["/p/a.rs", "/p/gone.rs", "/p/new.rs", "/p/tmp~", "/p/a.rs"].map(PathBuf::from),
        );
        assert_eq!(change.modified, vec![a]);
        assert_eq!(change.deleted, vec![gone]);
        assert_eq!(change.created.len(), 1);
        assert_eq!(change.files().count(), 3);
        let mut overlay = DocumentOverlay::default();
        overlay.set(a, FileRev(3), "fn b() {}".into());
        assert_eq!(overlay.get(a).map(|e| &*e.text), Some("fn b() {}"));
        assert!(overlay.close(a).is_some() && overlay.is_empty());
    }

    #[test]
    fn disk_reads_are_capped_and_reject_binaries() {
        let dir = std::env::temp_dir().join(format!("archipel-vfs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let text = dir.join("a.txt");
        let binary = dir.join("b.bin");
        std::fs::write(&text, "hello").expect("write");
        std::fs::write(&binary, [0u8, 1, 2]).expect("write");
        assert_eq!(DiskVfs.read(&text).as_deref(), Ok("hello"));
        assert!(matches!(DiskVfs.read(&binary), Err(VfsError::Binary(_))));
        assert!(matches!(
            DiskVfs.read(&dir.join("missing")),
            Err(VfsError::NotFound(_))
        ));
        assert_eq!(DiskVfs.metadata(&text).map(|m| m.len), Some(5));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

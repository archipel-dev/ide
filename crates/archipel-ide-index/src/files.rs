//! Where the indexed files come from: the file lister (project scan) and a
//! VFS that resolves project-relative paths.

use archipel_ide_analysis::CancelToken;
use archipel_ide_analysis::vfs::{DiskMeta, MemVfs, Vfs, VfsError};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

/// Directories that are never indexed (build output, VCS and tool data).
pub const DEFAULT_EXCLUDES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".archipel",
    ".jetthing",
    ".gradle",
    ".idea",
    "target",
    "node_modules",
];

/// One file found by a scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanEntry {
    /// Path relative to the project root.
    pub path: PathBuf,
    pub len: u64,
    pub mtime: Option<std::time::SystemTime>,
}

/// Enumerates the files of a project (or of one directory of it).
pub trait FileLister: Send + Sync {
    /// Files under `dir` (absolute) with paths relative to `root`.
    fn list(
        &self,
        root: &Path,
        dir: &Path,
        excludes: &[String],
        cancel: &CancelToken,
    ) -> Vec<ScanEntry>;
}

/// Lists the real file system (symbolic links are not followed).
#[derive(Debug, Clone, Copy, Default)]
pub struct DiskLister;

impl FileLister for DiskLister {
    fn list(
        &self,
        root: &Path,
        dir: &Path,
        excludes: &[String],
        cancel: &CancelToken,
    ) -> Vec<ScanEntry> {
        let mut out = Vec::new();
        let walk = WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                entry.depth() == 0
                    || !entry.file_type().is_dir()
                    || !entry
                        .file_name()
                        .to_str()
                        .is_some_and(|name| excludes.iter().any(|ex| ex == name))
            });
        for (seen, entry) in walk.enumerate() {
            if seen % 256 == 0 && cancel.is_cancelled() {
                break;
            }
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() {
                continue;
            }
            let Ok(relative) = entry.path().strip_prefix(root) else {
                continue;
            };
            let Ok(meta) = entry.metadata() else { continue };
            out.push(ScanEntry {
                path: relative.to_path_buf(),
                len: meta.len(),
                mtime: meta.modified().ok(),
            });
        }
        out
    }
}

/// Lists a [`MemVfs`] (tests): files whose absolute path is under `dir`.
pub struct MemLister(pub Arc<MemVfs>);

impl FileLister for MemLister {
    fn list(
        &self,
        root: &Path,
        dir: &Path,
        excludes: &[String],
        _cancel: &CancelToken,
    ) -> Vec<ScanEntry> {
        self.0
            .paths()
            .into_iter()
            .filter(|path| path.starts_with(dir))
            .filter_map(|path| {
                let relative = path.strip_prefix(root).ok()?.to_path_buf();
                if relative.components().any(|c| {
                    c.as_os_str()
                        .to_str()
                        .is_some_and(|name| excludes.iter().any(|ex| ex == name))
                }) {
                    return None;
                }
                let meta = self.0.metadata(&path)?;
                Some(ScanEntry {
                    path: relative,
                    len: meta.len,
                    mtime: meta.mtime,
                })
            })
            .collect()
    }
}

/// Resolves project-relative paths against the project root before reading.
pub struct RootedVfs {
    pub root: PathBuf,
    pub inner: Arc<dyn Vfs>,
}

impl RootedVfs {
    pub fn absolute(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }
}

impl Vfs for RootedVfs {
    fn read(&self, path: &Path) -> Result<Arc<str>, VfsError> {
        self.inner.read(&self.absolute(path))
    }

    fn metadata(&self, path: &Path) -> Option<DiskMeta> {
        self.inner.metadata(&self.absolute(path))
    }
}

//! File identities: [`FileTable`] interns paths to stable [`FileId`]s.

pub use archipel_ide_language::{FileId, FileRev, LanguageId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Path <-> id table. Ids are dense, never reused (a deleted file's id stays
/// retired) and survive renames reported through [`FileTable::rename`].
#[derive(Debug, Clone, Default)]
pub struct FileTable {
    paths: Vec<Option<Arc<Path>>>,
    ids: HashMap<Arc<Path>, FileId>,
}

impl FileTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// The id of `path`, allocating one for a new path.
    pub fn intern(&mut self, path: &Path) -> FileId {
        if let Some(id) = self.ids.get(path) {
            return *id;
        }
        let id = FileId(u32::try_from(self.paths.len()).unwrap_or(u32::MAX));
        let path: Arc<Path> = path.into();
        self.paths.push(Some(path.clone()));
        self.ids.insert(path, id);
        id
    }

    pub fn lookup(&self, path: &Path) -> Option<FileId> {
        self.ids.get(path).copied()
    }

    pub fn path(&self, id: FileId) -> Option<&Path> {
        self.paths
            .get(id.0 as usize)
            .and_then(|path| path.as_deref())
    }

    /// Moves `id` to `to`, keeping the id. A file previously at `to` loses
    /// its id (it was overwritten). Returns false for unknown ids.
    pub fn rename(&mut self, id: FileId, to: &Path) -> bool {
        let Some(Some(old)) = self.paths.get(id.0 as usize).cloned() else {
            return false;
        };
        self.ids.remove(&old);
        if let Some(replaced) = self.ids.remove(to)
            && let Some(slot) = self.paths.get_mut(replaced.0 as usize)
        {
            *slot = None;
        }
        let to: Arc<Path> = to.into();
        self.paths[id.0 as usize] = Some(to.clone());
        self.ids.insert(to, id);
        true
    }

    /// Retires `id` (the file was deleted).
    pub fn remove(&mut self, id: FileId) -> Option<PathBuf> {
        let path = self.paths.get_mut(id.0 as usize)?.take()?;
        self.ids.remove(&path);
        Some(path.to_path_buf())
    }

    /// Number of live files.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Upper bound of allocated ids (for bitsets).
    pub fn id_bound(&self) -> u32 {
        self.paths.len() as u32
    }

    pub fn iter(&self) -> impl Iterator<Item = (FileId, &Path)> {
        self.paths
            .iter()
            .enumerate()
            .filter_map(|(index, path)| Some((FileId(index as u32), path.as_deref()?)))
    }

    /// Live entries for persistence (the index store keeps ids stable across
    /// restarts).
    pub fn export(&self) -> Vec<(FileId, PathBuf)> {
        self.iter()
            .map(|(id, path)| (id, path.to_path_buf()))
            .collect()
    }

    /// Rebuilds a table from [`FileTable::export`]ed entries; ids between
    /// them stay retired.
    pub fn restore(entries: impl IntoIterator<Item = (FileId, PathBuf)>) -> Self {
        let mut table = Self::new();
        for (id, path) in entries {
            let index = id.0 as usize;
            if table.paths.len() <= index {
                table.paths.resize(index + 1, None);
            }
            if table.ids.contains_key(path.as_path()) {
                continue;
            }
            let path: Arc<Path> = path.into();
            table.paths[index] = Some(path.clone());
            table.ids.insert(path, id);
        }
        table
    }
}

/// What is known about one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMeta {
    pub id: FileId,
    pub path: PathBuf,
    pub len: u64,
    pub mtime: Option<std::time::SystemTime>,
    /// Hash of the bytes, computed lazily by whoever needs it (indexing).
    pub content_hash: Option<u64>,
    pub language: Option<LanguageId>,
    pub is_open: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_stable_across_renames_and_never_reused() {
        let mut table = FileTable::new();
        let a = table.intern(Path::new("/p/a.rs"));
        let b = table.intern(Path::new("/p/b.rs"));
        assert_ne!(a, b);
        assert_eq!(table.intern(Path::new("/p/a.rs")), a);
        assert!(table.rename(a, Path::new("/p/c.rs")));
        assert_eq!(table.lookup(Path::new("/p/c.rs")), Some(a));
        assert_eq!(table.lookup(Path::new("/p/a.rs")), None);
        assert_eq!(table.remove(b), Some(PathBuf::from("/p/b.rs")));
        let new = table.intern(Path::new("/p/b.rs"));
        assert_ne!(new, b);
        // Renaming onto an existing file retires the overwritten id.
        let d = table.intern(Path::new("/p/d.rs"));
        assert!(table.rename(a, Path::new("/p/d.rs")));
        assert_eq!(table.path(d), None);
        assert_eq!(table.lookup(Path::new("/p/d.rs")), Some(a));
        let restored = FileTable::restore(table.export());
        assert_eq!(restored.lookup(Path::new("/p/d.rs")), Some(a));
        assert_eq!(restored.lookup(Path::new("/p/b.rs")), Some(new));
        assert_eq!(restored.len(), table.len());
        assert!(!table.rename(FileId(99), Path::new("/x")));
    }
}

//! Modification stamps: the inputs a cached value can depend on, and the
//! current value of each. Validation compares recorded stamps with current
//! ones on read (no eager invalidation fan-out).

use crate::file::{FileId, FileRev, LanguageId};
use std::collections::HashMap;
use std::sync::Arc;

/// Version of one file's declaration summary ("interface"). Bumped only when
/// the summary hash changes, so edits inside a function body keep every
/// interface-dependent cache of other files valid (early cutoff).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct InterfaceEpoch(pub u64);

/// Stable id of an index (`"FileName"`, `"Words"`, `"Stub:ShortName"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndexId(pub &'static str);

/// Commit counter of one index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct IndexStamp(pub u64);

/// One input of a cached value, with the stamp it had when the value was
/// computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dep {
    /// That file's text.
    File(FileId, FileRev),
    /// That file's declaration summary.
    Interface(FileId, InterfaceEpoch),
    /// Any file of the language changed its interface.
    LanguageInterface(LanguageId, u64),
    /// An index received writes.
    Index(IndexId, IndexStamp),
    /// Entered or left dumb mode.
    DumbMode(u64),
    Settings(u64),
    /// Never valid (volatile values).
    Never,
}

const CHUNK: usize = 256;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct FileStamp {
    rev: u64,
    epoch: u64,
    /// The file exists (has been stamped and not deleted).
    live: bool,
}

/// Current stamps. Cloning is cheap (per-file stamps live in shared chunks
/// copied on write), so every snapshot carries its own consistent copy.
#[derive(Debug, Clone, Default)]
pub struct Stamps {
    files: Vec<Arc<[FileStamp; CHUNK]>>,
    languages: Arc<HashMap<LanguageId, u64>>,
    indexes: Arc<HashMap<IndexId, IndexStamp>>,
    dumb: u64,
    settings: u64,
}

impl Stamps {
    pub fn new() -> Self {
        Self::default()
    }

    fn file_stamp(&self, file: FileId) -> FileStamp {
        let index = file.0 as usize;
        self.files
            .get(index / CHUNK)
            .map(|chunk| chunk[index % CHUNK])
            .unwrap_or_default()
    }

    fn file_stamp_mut(&mut self, file: FileId) -> &mut FileStamp {
        let index = file.0 as usize;
        while self.files.len() <= index / CHUNK {
            self.files.push(Arc::new([FileStamp::default(); CHUNK]));
        }
        &mut Arc::make_mut(&mut self.files[index / CHUNK])[index % CHUNK]
    }

    /// Current revision of `file` (`None` for unknown or deleted files).
    pub fn file_rev(&self, file: FileId) -> Option<FileRev> {
        let stamp = self.file_stamp(file);
        stamp.live.then_some(FileRev(stamp.rev))
    }

    pub fn interface_epoch(&self, file: FileId) -> Option<InterfaceEpoch> {
        let stamp = self.file_stamp(file);
        stamp.live.then_some(InterfaceEpoch(stamp.epoch))
    }

    pub fn language_interface(&self, language: LanguageId) -> u64 {
        self.languages.get(&language).copied().unwrap_or(0)
    }

    pub fn index(&self, index: IndexId) -> IndexStamp {
        self.indexes.get(&index).copied().unwrap_or_default()
    }

    pub fn dumb(&self) -> u64 {
        self.dumb
    }

    pub fn settings(&self) -> u64 {
        self.settings
    }

    pub fn set_file_rev(&mut self, file: FileId, rev: FileRev) {
        let stamp = self.file_stamp_mut(file);
        stamp.rev = rev.0;
        stamp.live = true;
    }

    /// Bumps `file`'s interface epoch (and its language's counter).
    pub fn bump_interface(&mut self, file: FileId, language: Option<LanguageId>) -> InterfaceEpoch {
        let stamp = self.file_stamp_mut(file);
        stamp.epoch += 1;
        stamp.live = true;
        let epoch = InterfaceEpoch(stamp.epoch);
        if let Some(language) = language {
            *Arc::make_mut(&mut self.languages)
                .entry(language)
                .or_default() += 1;
        }
        epoch
    }

    pub fn delete_file(&mut self, file: FileId) {
        let stamp = self.file_stamp_mut(file);
        stamp.live = false;
        stamp.epoch += 1;
    }

    pub fn set_index(&mut self, index: IndexId, stamp: IndexStamp) {
        Arc::make_mut(&mut self.indexes).insert(index, stamp);
    }

    pub fn bump_dumb(&mut self) {
        self.dumb += 1;
    }

    pub fn bump_settings(&mut self) {
        self.settings += 1;
    }

    /// The dependency on `dep`'s input as it is now.
    pub fn current(&self, dep: &Dep) -> Option<Dep> {
        Some(match *dep {
            Dep::File(file, _) => Dep::File(file, self.file_rev(file)?),
            Dep::Interface(file, _) => Dep::Interface(file, self.interface_epoch(file)?),
            Dep::LanguageInterface(language, _) => {
                Dep::LanguageInterface(language, self.language_interface(language))
            }
            Dep::Index(index, _) => Dep::Index(index, self.index(index)),
            Dep::DumbMode(_) => Dep::DumbMode(self.dumb),
            Dep::Settings(_) => Dep::Settings(self.settings),
            Dep::Never => return None,
        })
    }

    /// True when `dep` still has the stamp it was recorded with.
    pub fn is_current(&self, dep: &Dep) -> bool {
        self.current(dep).is_some_and(|current| current == *dep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deps_validate_against_current_stamps() {
        let mut stamps = Stamps::new();
        let file = FileId(300);
        assert!(!stamps.is_current(&Dep::File(file, FileRev(0))));
        stamps.set_file_rev(file, FileRev(4));
        let snapshot = stamps.clone();
        let file_dep = Dep::File(file, FileRev(4));
        let iface = Dep::Interface(file, snapshot.interface_epoch(file).expect("live"));
        assert!(stamps.is_current(&file_dep) && stamps.is_current(&iface));
        stamps.set_file_rev(file, FileRev(5));
        assert!(!stamps.is_current(&file_dep));
        assert!(stamps.is_current(&iface));
        // The older copy is unaffected (copy on write).
        assert!(snapshot.is_current(&file_dep));
        stamps.bump_interface(file, Some(LanguageId("rust")));
        assert!(!stamps.is_current(&iface));
        assert_eq!(stamps.language_interface(LanguageId("rust")), 1);
        assert!(!stamps.is_current(&Dep::Never));
        stamps.delete_file(file);
        assert_eq!(stamps.file_rev(file), None);
        stamps.set_index(IndexId("Words"), IndexStamp(2));
        assert!(stamps.is_current(&Dep::Index(IndexId("Words"), IndexStamp(2))));
        let dumb = Dep::DumbMode(stamps.dumb());
        stamps.bump_dumb();
        assert!(!stamps.is_current(&dumb));
    }
}

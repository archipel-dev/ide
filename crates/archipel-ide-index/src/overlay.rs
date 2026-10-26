//! Open-buffer overlay (IntelliJ flushes document changes into the index
//! before a query; we layer an in-memory index of the unsaved text over the
//! persistent one). For a file with an overlay entry, queries see the
//! overlay's entries and ignore the persistent ones; closing the buffer drops
//! the overlay and the scheduler reindexes the file from disk.

use crate::store::DurableStore;
use archipel_ide_analysis::index::{IndexStore, MemIndexStore};
use archipel_ide_analysis::{FileId, IndexId, IndexStamp};
use parking_lot::RwLock;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// An [`IndexStore`] that merges an in-memory overlay over a persistent base.
///
/// Writes through this type go to the overlay; the scheduler writes disk
/// contents to the base directly. Both are visible to readers holding this
/// store (the one the analysis host serves through `Snapshot::require`).
pub struct OverlayStore {
    base: Arc<dyn DurableStore>,
    layer: MemIndexStore,
    shadow: RwLock<HashMap<IndexId, HashSet<FileId>>>,
    stamp: AtomicU64,
}

impl OverlayStore {
    pub fn new(base: Arc<dyn DurableStore>) -> Self {
        Self {
            base,
            layer: MemIndexStore::new(),
            shadow: RwLock::new(HashMap::new()),
            stamp: AtomicU64::new(0),
        }
    }

    pub fn base(&self) -> &Arc<dyn DurableStore> {
        &self.base
    }

    /// True when `file` has overlay entries in some index.
    pub fn has_overlay(&self, file: FileId) -> bool {
        self.shadow
            .read()
            .values()
            .any(|files| files.contains(&file))
    }

    /// Files with overlay entries.
    pub fn overlaid_files(&self) -> Vec<FileId> {
        let mut files: BTreeSet<FileId> = BTreeSet::new();
        for set in self.shadow.read().values() {
            files.extend(set.iter().copied());
        }
        files.into_iter().collect()
    }

    /// Forgets the overlay of `file` (buffer closed or reconciled with disk).
    pub fn drop_file(&self, file: FileId) {
        let mut shadow = self.shadow.write();
        for (index, files) in shadow.iter_mut() {
            if files.remove(&file) {
                self.layer.remove_file(*index, file);
            }
        }
        self.stamp.fetch_add(1, Ordering::SeqCst);
    }

    /// Drops the overlay entries of one index (its settings changed).
    pub fn clear_layer(&self, index: IndexId) {
        self.layer.clear(index);
        self.shadow.write().remove(&index);
        self.stamp.fetch_add(1, Ordering::SeqCst);
    }

    fn is_shadowed(&self, index: IndexId, file: FileId) -> bool {
        self.shadow
            .read()
            .get(&index)
            .is_some_and(|files| files.contains(&file))
    }
}

impl IndexStore for OverlayStore {
    fn put(&self, index: IndexId, file: FileId, entries: Vec<(Vec<u8>, Vec<u8>)>) {
        self.shadow.write().entry(index).or_default().insert(file);
        self.layer.put(index, file, entries);
    }

    fn remove_file(&self, index: IndexId, file: FileId) {
        self.shadow.write().entry(index).or_default().insert(file);
        self.layer.remove_file(index, file);
    }

    fn get(&self, index: IndexId, key: &[u8]) -> Vec<(FileId, Vec<u8>)> {
        let mut merged: BTreeMap<FileId, Vec<u8>> = BTreeMap::new();
        let shadow = self.shadow.read();
        let shadowed = shadow.get(&index);
        for (file, value) in self.base.get(index, key) {
            if !shadowed.is_some_and(|files| files.contains(&file)) {
                merged.insert(file, value);
            }
        }
        drop(shadow);
        for (file, value) in self.layer.get(index, key) {
            merged.insert(file, value);
        }
        merged.into_iter().collect()
    }

    fn keys(&self, index: IndexId) -> Vec<Vec<u8>> {
        let mut base_keys = self.base.keys(index);
        let shadowed: Vec<FileId> = self
            .shadow
            .read()
            .get(&index)
            .map(|files| files.iter().copied().collect())
            .unwrap_or_default();
        if shadowed.is_empty() {
            let layer = self.layer.keys(index);
            if layer.is_empty() {
                return base_keys;
            }
        }
        if !shadowed.is_empty() {
            let mut stale: HashSet<Vec<u8>> = HashSet::new();
            for file in &shadowed {
                stale.extend(self.base.forward(index, *file));
            }
            base_keys.retain(|key| {
                !stale.contains(key)
                    || self
                        .base
                        .get(index, key)
                        .iter()
                        .any(|(file, _)| !shadowed.contains(file))
            });
        }
        let mut all: BTreeSet<Vec<u8>> = base_keys.into_iter().collect();
        all.extend(self.layer.keys(index));
        all.into_iter().collect()
    }

    fn forward(&self, index: IndexId, file: FileId) -> Vec<Vec<u8>> {
        if self.is_shadowed(index, file) {
            self.layer.forward(index, file)
        } else {
            self.base.forward(index, file)
        }
    }

    fn commit(&self, index: IndexId) -> IndexStamp {
        let base = self.base.commit(index);
        let own = self.stamp.fetch_add(1, Ordering::SeqCst) + 1;
        IndexStamp(base.0 + own)
    }

    fn clear(&self, index: IndexId) {
        self.base.clear(index);
        self.layer.clear(index);
        self.shadow.write().remove(&index);
    }
}

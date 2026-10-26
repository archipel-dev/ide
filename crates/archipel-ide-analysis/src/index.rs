//! Index contracts (implemented by WP2's `archipel-ide-index`): the
//! `FileBasedIndexExtension` analogue, per-index readiness (dumb mode is per
//! index, D7) and the byte-level store both backends implement. An in-memory
//! store ([`MemIndexStore`]) serves tests and the open-buffer overlay.

use crate::cancel::Cancelled;
use crate::file::{FileId, FileMeta};
use crate::host::Snapshot;
use crate::resolve::{FileSet, SearchScope};
use crate::stamps::{IndexId, IndexStamp};
use archipel_ide_language::SyntaxSnapshot;
use parking_lot::RwLock;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;

/// Readiness of one index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexState {
    Absent,
    Building { done: u32, total: u32 },
    Ready,
    Failed,
}

/// The index cannot answer yet (a normal value, never a panic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("index {0:?} is not ready")]
pub struct IndexNotReady(pub IndexId);

/// Scheduling order of indexes (cheap name indexes first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndexPriority {
    Names,
    Stubs,
    Words,
    Trigrams,
    Todo,
}

/// A key of an index (persisted as bytes).
pub trait IndexKey: Hash + Eq + Ord + Clone + Send + Sync + 'static {
    fn encode(&self, out: &mut Vec<u8>);
    fn decode(bytes: &[u8]) -> Option<Self>;
}

/// A value of an index (persisted as bytes).
pub trait IndexValue: Clone + Send + Sync + 'static {
    fn encode(&self, out: &mut Vec<u8>);
    fn decode(bytes: &[u8]) -> Option<Self>;
}

impl IndexKey for String {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.as_bytes());
    }
    fn decode(bytes: &[u8]) -> Option<Self> {
        String::from_utf8(bytes.to_vec()).ok()
    }
}

impl IndexKey for u64 {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_be_bytes());
    }
    fn decode(bytes: &[u8]) -> Option<Self> {
        Some(u64::from_be_bytes(bytes.try_into().ok()?))
    }
}

impl IndexKey for u32 {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_be_bytes());
    }
    fn decode(bytes: &[u8]) -> Option<Self> {
        Some(u32::from_be_bytes(bytes.try_into().ok()?))
    }
}

impl IndexValue for () {
    fn encode(&self, _: &mut Vec<u8>) {}
    fn decode(_: &[u8]) -> Option<Self> {
        Some(())
    }
}

impl IndexValue for u32 {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_be_bytes());
    }
    fn decode(bytes: &[u8]) -> Option<Self> {
        Some(u32::from_be_bytes(bytes.try_into().ok()?))
    }
}

impl IndexValue for String {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.as_bytes());
    }
    fn decode(bytes: &[u8]) -> Option<Self> {
        String::from_utf8(bytes.to_vec()).ok()
    }
}

/// What an indexer sees of one file. The syntax is parsed once per file and
/// shared by every index.
pub struct IndexInput<'a> {
    pub file: &'a FileMeta,
    pub text: &'a str,
    pub syntax: Option<&'a SyntaxSnapshot>,
}

/// A file-based index (`FileBasedIndexExtension`).
pub trait FileIndex: Send + Sync + 'static {
    const ID: IndexId;
    const VERSION: u32;
    type Key: IndexKey;
    type Value: IndexValue;
    /// Input filter (language, size, not binary).
    fn accepts(&self, f: &FileMeta) -> bool;
    /// `false` = name/metadata only (cheap, built during the scan).
    fn depends_on_content(&self) -> bool {
        true
    }
    fn index(
        &self,
        input: &IndexInput<'_>,
        out: &mut dyn FnMut(Self::Key, Self::Value),
    ) -> Result<(), Cancelled>;
    fn dumb_aware(&self) -> bool {
        false
    }
    fn priority(&self) -> IndexPriority;
}

/// Typed queries over one index.
pub trait IndexQuery<K, V> {
    fn files_for_key(&self, key: &K) -> Result<FileSet, IndexNotReady>;
    fn values(&self, key: &K, scope: &SearchScope) -> Result<Vec<(FileId, V)>, IndexNotReady>;
    fn all_keys(&self, matcher: &dyn Fn(&K) -> bool) -> Result<Vec<K>, IndexNotReady>;
}

/// Byte-level storage of all indexes: inverted map (key -> files with
/// values) plus forward map (file -> keys) for incremental removal. Both the
/// persistent backend and [`MemIndexStore`] implement it.
pub trait IndexStore: Send + Sync {
    /// Replaces every entry of `file` in `index`.
    fn put(&self, index: IndexId, file: FileId, entries: Vec<(Vec<u8>, Vec<u8>)>);
    fn remove_file(&self, index: IndexId, file: FileId);
    /// `(file, value)` pairs stored under `key`.
    fn get(&self, index: IndexId, key: &[u8]) -> Vec<(FileId, Vec<u8>)>;
    /// Every key of `index` (sorted).
    fn keys(&self, index: IndexId) -> Vec<Vec<u8>>;
    /// Keys stored for `file` (the forward map).
    fn forward(&self, index: IndexId, file: FileId) -> Vec<Vec<u8>>;
    /// Makes pending writes visible; returns the index's new stamp.
    fn commit(&self, index: IndexId) -> IndexStamp;
    /// Drops all data of `index` (version change, corruption).
    fn clear(&self, index: IndexId);
}

#[derive(Default)]
struct MemIndex {
    inverted: BTreeMap<Vec<u8>, BTreeMap<FileId, Vec<u8>>>,
    forward: HashMap<FileId, BTreeSet<Vec<u8>>>,
    stamp: u64,
}

/// In-memory [`IndexStore`].
#[derive(Default)]
pub struct MemIndexStore {
    indexes: RwLock<HashMap<IndexId, MemIndex>>,
}

impl MemIndexStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl IndexStore for MemIndexStore {
    fn put(&self, index: IndexId, file: FileId, entries: Vec<(Vec<u8>, Vec<u8>)>) {
        self.remove_file(index, file);
        let mut indexes = self.indexes.write();
        let data = indexes.entry(index).or_default();
        let forward = data.forward.entry(file).or_default();
        for (key, value) in entries {
            forward.insert(key.clone());
            data.inverted.entry(key).or_default().insert(file, value);
        }
    }

    fn remove_file(&self, index: IndexId, file: FileId) {
        let mut indexes = self.indexes.write();
        let Some(data) = indexes.get_mut(&index) else {
            return;
        };
        for key in data.forward.remove(&file).unwrap_or_default() {
            if let Some(files) = data.inverted.get_mut(&key) {
                files.remove(&file);
                if files.is_empty() {
                    data.inverted.remove(&key);
                }
            }
        }
    }

    fn get(&self, index: IndexId, key: &[u8]) -> Vec<(FileId, Vec<u8>)> {
        self.indexes
            .read()
            .get(&index)
            .and_then(|data| data.inverted.get(key))
            .map(|files| files.iter().map(|(f, v)| (*f, v.clone())).collect())
            .unwrap_or_default()
    }

    fn keys(&self, index: IndexId) -> Vec<Vec<u8>> {
        self.indexes
            .read()
            .get(&index)
            .map(|data| data.inverted.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn forward(&self, index: IndexId, file: FileId) -> Vec<Vec<u8>> {
        self.indexes
            .read()
            .get(&index)
            .and_then(|data| data.forward.get(&file))
            .map(|keys| keys.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn commit(&self, index: IndexId) -> IndexStamp {
        let mut indexes = self.indexes.write();
        let data = indexes.entry(index).or_default();
        data.stamp += 1;
        IndexStamp(data.stamp)
    }

    fn clear(&self, index: IndexId) {
        self.indexes.write().remove(&index);
    }
}

/// Runs `indexer` on one file and stores its output (the generic part of
/// the indexing pipeline; WP2 adds scheduling, hashing and persistence).
pub fn index_file<I: FileIndex>(
    store: &dyn IndexStore,
    indexer: &I,
    input: &IndexInput<'_>,
) -> Result<(), Cancelled> {
    if !indexer.accepts(input.file) {
        store.remove_file(I::ID, input.file.id);
        return Ok(());
    }
    let mut entries = Vec::new();
    indexer.index(input, &mut |key, value| {
        let (mut k, mut v) = (Vec::new(), Vec::new());
        key.encode(&mut k);
        value.encode(&mut v);
        entries.push((k, v));
    })?;
    store.put(I::ID, input.file.id, entries);
    Ok(())
}

/// Typed read access to a ready index.
pub struct IndexReader<I: FileIndex> {
    store: Arc<dyn IndexStore>,
    _index: PhantomData<fn() -> I>,
}

impl<I: FileIndex> IndexQuery<I::Key, I::Value> for IndexReader<I> {
    fn files_for_key(&self, key: &I::Key) -> Result<FileSet, IndexNotReady> {
        let mut bytes = Vec::new();
        key.encode(&mut bytes);
        Ok(self
            .store
            .get(I::ID, &bytes)
            .into_iter()
            .map(|(file, _)| file)
            .collect())
    }

    fn values(
        &self,
        key: &I::Key,
        scope: &SearchScope,
    ) -> Result<Vec<(FileId, I::Value)>, IndexNotReady> {
        let mut bytes = Vec::new();
        key.encode(&mut bytes);
        Ok(self
            .store
            .get(I::ID, &bytes)
            .into_iter()
            .filter(|(file, _)| scope.contains(*file))
            .filter_map(|(file, value)| Some((file, I::Value::decode(&value)?)))
            .collect())
    }

    fn all_keys(&self, matcher: &dyn Fn(&I::Key) -> bool) -> Result<Vec<I::Key>, IndexNotReady> {
        Ok(self
            .store
            .keys(I::ID)
            .iter()
            .filter_map(|bytes| I::Key::decode(bytes))
            .filter(|key| matcher(key))
            .collect())
    }
}

impl Snapshot {
    /// A reader for index `I` when it is [`IndexState::Ready`]
    /// (`IndexNotReady` otherwise: features degrade per index, D7).
    pub fn require<I: FileIndex>(&self) -> Result<IndexReader<I>, IndexNotReady> {
        if self.index_state(I::ID) != IndexState::Ready {
            return Err(IndexNotReady(I::ID));
        }
        let store = self.index_store().ok_or(IndexNotReady(I::ID))?.clone();
        Ok(IndexReader {
            store,
            _index: PhantomData,
        })
    }
}

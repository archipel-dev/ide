//! Cached values with dependency stamps (`CachedValue` analogue).
//!
//! [`Db::cached`] returns the stored value when every recorded [`Dep`] still
//! has its stamp, else recomputes it. Dependencies are recorded by the
//! computation through a [`DepCollector`]; nested cached reads propagate their
//! dependencies to the outer value. Storage is one bounded LRU per host
//! ([`CacheStore`], byte-weighed), dropped on memory pressure.

use crate::cancel::{CancelToken, Cancelled};
use crate::file::{FileId, LanguageId};
use crate::stamps::{Dep, IndexId, Stamps};
use parking_lot::Mutex;
use smallvec::SmallVec;
use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasher, Hash, RandomState};
use std::marker::PhantomData;
use std::sync::Arc;

/// Default capacity of the value cache (spec budget: 64 MiB).
pub const DEFAULT_CACHE_BYTES: usize = 64 * 1024 * 1024;

/// A typed cache slot: one kind of cached value, keyed by `K`.
pub struct Slot<K, T> {
    pub id: &'static str,
    weigh: fn(&T) -> usize,
    _types: PhantomData<fn(K) -> T>,
}

impl<K, T> Clone for Slot<K, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K, T> Copy for Slot<K, T> {}

fn shallow_size<T>(_: &T) -> usize {
    std::mem::size_of::<T>()
}

impl<K, T> Slot<K, T> {
    /// A slot whose values weigh `size_of::<T>()`.
    pub const fn new(id: &'static str) -> Self {
        Self {
            id,
            weigh: shallow_size::<T>,
            _types: PhantomData,
        }
    }

    /// A slot with a byte weigher for values owning heap data.
    pub const fn weighed(id: &'static str, weigh: fn(&T) -> usize) -> Self {
        Self {
            id,
            weigh,
            _types: PhantomData,
        }
    }
}

/// A computed value and the inputs it was computed from.
#[derive(Debug, Clone)]
pub struct Cached<T> {
    pub value: Arc<T>,
    pub deps: SmallVec<[Dep; 4]>,
}

/// Records the inputs of one computation.
pub struct DepCollector<'a> {
    stamps: &'a Stamps,
    cancel: &'a CancelToken,
    deps: SmallVec<[Dep; 4]>,
}

impl<'a> DepCollector<'a> {
    pub fn new(stamps: &'a Stamps, cancel: &'a CancelToken) -> Self {
        Self {
            stamps,
            cancel,
            deps: SmallVec::new(),
        }
    }

    pub fn cancel(&self) -> &CancelToken {
        self.cancel
    }

    pub fn check(&self) -> Result<(), Cancelled> {
        self.cancel.check()
    }

    pub fn stamps(&self) -> &Stamps {
        self.stamps
    }

    /// Records `dep` as is.
    pub fn add(&mut self, dep: Dep) {
        if !self.deps.contains(&dep) {
            self.deps.push(dep);
        }
    }

    /// Depends on the current text revision of `file`.
    pub fn file(&mut self, file: FileId) {
        self.add(
            self.stamps
                .current(&Dep::File(file, Default::default()))
                .unwrap_or(Dep::Never),
        );
    }

    /// Depends on the current interface epoch of `file` (survives edits that
    /// keep its declarations).
    pub fn interface(&mut self, file: FileId) {
        self.add(
            self.stamps
                .current(&Dep::Interface(file, Default::default()))
                .unwrap_or(Dep::Never),
        );
    }

    pub fn language_interface(&mut self, language: LanguageId) {
        self.add(Dep::LanguageInterface(
            language,
            self.stamps.language_interface(language),
        ));
    }

    pub fn index(&mut self, index: IndexId) {
        self.add(Dep::Index(index, self.stamps.index(index)));
    }

    pub fn dumb_mode(&mut self) {
        self.add(Dep::DumbMode(self.stamps.dumb()));
    }

    pub fn settings(&mut self) {
        self.add(Dep::Settings(self.stamps.settings()));
    }

    pub fn volatile(&mut self) {
        self.add(Dep::Never);
    }

    pub fn deps(&self) -> &[Dep] {
        &self.deps
    }
}

struct Entry {
    key: Box<dyn Any + Send + Sync>,
    value: Arc<dyn Any + Send + Sync>,
    deps: SmallVec<[Dep; 4]>,
    weight: usize,
    tick: u64,
}

#[derive(Default)]
struct Lru {
    entries: HashMap<(&'static str, u64), Entry>,
    order: BTreeMap<u64, (&'static str, u64)>,
    bytes: usize,
    tick: u64,
}

/// Cache statistics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub entries: usize,
    pub bytes: usize,
    pub hits: u64,
    pub misses: u64,
}

/// The shared value store (one per host; snapshots share it).
pub struct CacheStore {
    lru: Mutex<Lru>,
    capacity: usize,
    hasher: RandomState,
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
}

impl std::fmt::Debug for CacheStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheStore")
            .field("stats", &self.stats())
            .finish()
    }
}

impl Default for CacheStore {
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_BYTES)
    }
}

const ENTRY_OVERHEAD: usize = 96;

impl CacheStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            lru: Mutex::new(Lru::default()),
            capacity,
            hasher: RandomState::new(),
            hits: Default::default(),
            misses: Default::default(),
        }
    }

    pub fn stats(&self) -> CacheStats {
        let lru = self.lru.lock();
        CacheStats {
            entries: lru.entries.len(),
            bytes: lru.bytes,
            hits: self.hits.load(std::sync::atomic::Ordering::Relaxed),
            misses: self.misses.load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Drops every value (low-memory hook).
    pub fn clear(&self) {
        let mut lru = self.lru.lock();
        *lru = Lru::default();
    }

    /// Evicts least recently used values until at most `bytes` remain.
    pub fn trim(&self, bytes: usize) {
        let mut lru = self.lru.lock();
        trim(&mut lru, bytes);
    }

    fn get<K, T>(&self, slot: &Slot<K, T>, hash: u64, key: &K, stamps: &Stamps) -> Option<Cached<T>>
    where
        K: Eq + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        let mut lru = self.lru.lock();
        lru.tick += 1;
        let tick = lru.tick;
        let lru = &mut *lru;
        let entry = lru.entries.get_mut(&(slot.id, hash))?;
        if entry.key.downcast_ref::<K>() != Some(key)
            || !entry.deps.iter().all(|dep| stamps.is_current(dep))
        {
            return None;
        }
        lru.order.remove(&entry.tick);
        entry.tick = tick;
        lru.order.insert(tick, (slot.id, hash));
        let value = entry.value.clone().downcast::<T>().ok()?;
        Some(Cached {
            value,
            deps: entry.deps.clone(),
        })
    }

    fn put<K, T>(&self, slot: &Slot<K, T>, hash: u64, key: K, cached: &Cached<T>)
    where
        K: Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        let weight = (slot.weigh)(&cached.value) + ENTRY_OVERHEAD;
        if weight > self.capacity {
            return;
        }
        let mut lru = self.lru.lock();
        lru.tick += 1;
        let tick = lru.tick;
        if let Some(old) = lru.entries.remove(&(slot.id, hash)) {
            lru.order.remove(&old.tick);
            lru.bytes -= old.weight;
        }
        lru.entries.insert(
            (slot.id, hash),
            Entry {
                key: Box::new(key),
                value: cached.value.clone(),
                deps: cached.deps.clone(),
                weight,
                tick,
            },
        );
        lru.order.insert(tick, (slot.id, hash));
        lru.bytes += weight;
        let capacity = self.capacity;
        trim(&mut lru, capacity);
    }
}

fn trim(lru: &mut Lru, bytes: usize) {
    while lru.bytes > bytes {
        let Some((_, key)) = lru.order.pop_first() else {
            break;
        };
        if let Some(entry) = lru.entries.remove(&key) {
            lru.bytes -= entry.weight;
        }
    }
}

/// Cached-value access for one consistent view (a snapshot's stamps).
#[derive(Clone)]
pub struct Db {
    stamps: Stamps,
    store: Arc<CacheStore>,
}

impl std::fmt::Debug for Db {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Db").finish_non_exhaustive()
    }
}

impl Db {
    pub fn new(stamps: Stamps, store: Arc<CacheStore>) -> Self {
        Self { stamps, store }
    }

    pub fn stamps(&self) -> &Stamps {
        &self.stamps
    }

    pub fn store(&self) -> &Arc<CacheStore> {
        &self.store
    }

    /// The value of `slot` for `key`: stored if still valid, else computed by
    /// `f` (which records its dependencies) and stored. A cancelled
    /// computation stores nothing.
    pub fn cached<K, T>(
        &self,
        slot: Slot<K, T>,
        key: K,
        cancel: &CancelToken,
        f: impl FnOnce(&mut DepCollector<'_>) -> Result<T, Cancelled>,
    ) -> Result<Arc<T>, Cancelled>
    where
        K: Hash + Eq + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        self.cached_with_deps(slot, key, cancel, f)
            .map(|cached| cached.value)
    }

    /// [`Db::cached`] inside another computation: the inner value's
    /// dependencies become the outer value's.
    pub fn cached_in<K, T>(
        &self,
        outer: &mut DepCollector<'_>,
        slot: Slot<K, T>,
        key: K,
        f: impl FnOnce(&mut DepCollector<'_>) -> Result<T, Cancelled>,
    ) -> Result<Arc<T>, Cancelled>
    where
        K: Hash + Eq + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        let cancel = outer.cancel.clone();
        let cached = self.cached_with_deps(slot, key, &cancel, f)?;
        for dep in cached.deps {
            outer.add(dep);
        }
        Ok(cached.value)
    }

    fn cached_with_deps<K, T>(
        &self,
        slot: Slot<K, T>,
        key: K,
        cancel: &CancelToken,
        f: impl FnOnce(&mut DepCollector<'_>) -> Result<T, Cancelled>,
    ) -> Result<Cached<T>, Cancelled>
    where
        K: Hash + Eq + Send + Sync + 'static,
        T: Send + Sync + 'static,
    {
        use std::sync::atomic::Ordering::Relaxed;
        let hash = self.store.hasher.hash_one(&key);
        if let Some(cached) = self.store.get(&slot, hash, &key, &self.stamps) {
            self.store.hits.fetch_add(1, Relaxed);
            return Ok(cached);
        }
        self.store.misses.fetch_add(1, Relaxed);
        cancel.check()?;
        let mut collector = DepCollector::new(&self.stamps, cancel);
        let value = f(&mut collector)?;
        let cached = Cached {
            value: Arc::new(value),
            deps: collector.deps,
        };
        if !cached.deps.contains(&Dep::Never) {
            self.store.put(&slot, hash, key, &cached);
        }
        Ok(cached)
    }
}

/// Resident set size of this process, where the platform exposes it cheaply
/// (Linux `/proc/self/statm`); `None` elsewhere. The host polls it and clears
/// the cache above its limit (IntelliJ's low-memory flush).
pub fn resident_bytes() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(pages * 4096)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::FileRev;

    const LEN: Slot<u32, usize> = Slot::new("len");

    #[test]
    fn values_are_reused_until_a_dependency_changes() {
        let store = Arc::new(CacheStore::default());
        let mut stamps = Stamps::new();
        stamps.set_file_rev(FileId(1), FileRev(1));
        let cancel = CancelToken::new();
        let mut runs = 0;
        let mut compute = |db: &Db| {
            *db.cached(LEN, 1, &cancel, |cx| {
                runs += 1;
                cx.file(FileId(1));
                Ok(5)
            })
            .expect("value")
        };
        let db = Db::new(stamps.clone(), store.clone());
        assert_eq!(compute(&db), 5);
        assert_eq!(compute(&db), 5);
        stamps.set_file_rev(FileId(1), FileRev(2));
        assert_eq!(compute(&Db::new(stamps.clone(), store.clone())), 5);
        assert_eq!(runs, 2);
        assert_eq!(store.stats().hits, 1);
    }

    #[test]
    fn cancelled_and_volatile_values_are_not_stored_and_lru_trims() {
        let store = Arc::new(CacheStore::new(4 * ENTRY_OVERHEAD));
        let db = Db::new(Stamps::new(), store.clone());
        let cancel = CancelToken::new();
        let err = db.cached(LEN, 1, &cancel, |_| Err::<usize, _>(Cancelled));
        assert_eq!(err, Err(Cancelled));
        db.cached(LEN, 2, &cancel, |cx| {
            cx.volatile();
            Ok(1)
        })
        .expect("value");
        assert_eq!(store.stats().entries, 0);
        for key in 0..10 {
            db.cached(LEN, key, &cancel, |_| Ok(1)).expect("value");
        }
        assert!(store.stats().entries <= 3);
        assert!(store.stats().bytes <= 4 * ENTRY_OVERHEAD);
        store.clear();
        assert_eq!(store.stats().entries, 0);
        assert!(resident_bytes().is_none_or(|bytes| bytes > 0));
    }
}

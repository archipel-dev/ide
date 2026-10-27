//! The Java stub indexes on the project model (JAVA-STUB-011..018, JAVA-RES-042).
//!
//! Granule: an index *unit* (a source file, a jar, a class directory or the JDK image).
//! Units are keyed by a persistent id in the durable store of `archipel-ide-index`
//! (`RedbStore`, or `MemStore` without a cache directory), with their length, mtime and
//! content hash, so a restart or a model refresh only reparses units that changed
//! (JAVA-STUB-001, 018). Indexes (all keyed by unresolved short text, JAVA-STUB-011):
//!
//! | Index id | Key | Value |
//! |---|---|---|
//! | `Java:Stub` | top-level class FQN | [`StubRecord`] (package, imports, class tree) |
//! | `Java:Class` | FQN of every named class (nested included) | [`ClassRef`] |
//! | `Java:ShortName` | simple name of every named class | `ClassRef` |
//! | `Java:Method` | method or constructor name (JAVA-STUB-013) | [`MemberRef`] |
//! | `Java:Field` | field and enum constant name | `MemberRef` |
//! | `Java:Annotation` | annotation short name | `ClassRef` |
//! | `Java:SuperClass` | short name of every extends/implements type, `Enum`, `Annotation` | `ClassRef` |
//! | `Java:Package` | package and every parent package | `u8` (1: has classes) |
//!
//! Queries take a [`ResolveScope`] (JAVA-STUB-017) and return [`NotReady`] while the
//! indexes are not complete (dumb mode, JAVA-STUB-026).

use crate::scope::ResolveScope;
use crate::source;
use crate::stub::*;
use crate::unit::{self, UnitStubs};
use archipel_ide_analysis::{FileId, IndexId};
use archipel_ide_index::hash::content_hash;
use archipel_ide_index::store::{DurableStore, FileRecord, MemStore};
use archipel_ide_index::RedbStore;
use archipel_ide_projectmodel::{JpsModuleSourceRootType, ProjectModel, SdkType};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

pub mod ids {
    use archipel_ide_analysis::IndexId;
    pub const STUB: IndexId = IndexId("Java:Stub");
    pub const CLASS: IndexId = IndexId("Java:Class");
    pub const SHORT_NAME: IndexId = IndexId("Java:ShortName");
    pub const METHOD: IndexId = IndexId("Java:Method");
    pub const FIELD: IndexId = IndexId("Java:Field");
    pub const ANNOTATION: IndexId = IndexId("Java:Annotation");
    pub const SUPER_CLASS: IndexId = IndexId("Java:SuperClass");
    pub const PACKAGE: IndexId = IndexId("Java:Package");
    pub const ALL: [IndexId; 8] = [STUB, CLASS, SHORT_NAME, METHOD, FIELD, ANNOTATION, SUPER_CLASS, PACKAGE];
}

/// Version of the stub format and of every index; a change reindexes (JAVA-STUB-028).
pub const STUB_VERSION: &str = "java-stubs-1";

/// The index cannot answer yet (dumb mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the Java indexes are not ready")]
pub struct NotReady;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaIndexState {
    /// No project model has been synced.
    Absent,
    Building { done: u32, total: u32 },
    Ready,
}

/// What `Java:Stub` stores per top-level class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StubRecord {
    pub package: String,
    pub imports: Vec<ImportStub>,
    pub class: ClassStub,
}

/// A class (possibly nested) inside a top-level stub.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassRef {
    pub top: String,
    /// Nested class simple names below `top`.
    pub path: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberRef {
    pub class: ClassRef,
    pub index: u32,
}

/// A class found through the indexes.
#[derive(Debug, Clone)]
pub struct ClassHit {
    pub unit: u32,
    /// Position of the unit in the resolve scope (classpath order, lower first).
    pub rank: u32,
    pub record: Arc<StubRecord>,
    pub path: Vec<String>,
    /// A local or anonymous class of the current file (not in any index): its own stub.
    pub local: Option<Arc<ClassStub>>,
    /// Resolved supertypes of a local or anonymous class (resolved at its declaration site).
    pub local_supers: Option<Arc<Vec<ClassHit>>>,
}

impl ClassHit {
    pub fn class(&self) -> &ClassStub {
        if let Some(l) = &self.local {
            return l;
        }
        let mut c = &self.record.class;
        for name in &self.path {
            match c.inner_named(name) {
                Some(n) => c = n,
                None => break,
            }
        }
        c
    }
    pub fn package(&self) -> &str {
        &self.record.package
    }
    pub fn qname(&self) -> &str {
        &self.class().qname
    }
    pub fn is_top_level(&self) -> bool {
        self.path.is_empty()
    }
    /// The hit of a member class.
    pub fn inner(&self, name: &str) -> Option<ClassHit> {
        self.class().inner_named(name)?;
        let mut path = self.path.clone();
        path.push(name.to_string());
        Some(ClassHit { unit: self.unit, rank: self.rank, record: self.record.clone(), path, local: None, local_supers: None })
    }
}

#[derive(Debug, Clone)]
pub struct MemberHit {
    pub class: ClassHit,
    pub index: usize,
}

impl MemberHit {
    pub fn method(&self) -> Option<&MethodStub> {
        self.class.class().methods.get(self.index)
    }
    pub fn field(&self) -> Option<&FieldStub> {
        self.class.class().fields.get(self.index)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitKind {
    Source,
    Jar,
    ClassDir,
    Jdk,
}

#[derive(Debug, Clone)]
pub struct UnitInfo {
    pub id: u32,
    pub path: PathBuf,
    pub kind: UnitKind,
}

#[derive(Default)]
pub(crate) struct Catalog {
    pub units: HashMap<u32, UnitInfo>,
    pub by_path: HashMap<PathBuf, u32>,
    /// module name -> (production units, test units)
    pub modules: HashMap<String, (Vec<u32>, Vec<u32>)>,
    pub libraries: HashMap<String, Vec<u32>>,
    pub sdks: HashMap<String, Vec<u32>>,
    pub next_id: u32,
}

pub struct Inner {
    store: Arc<dyn DurableStore>,
    pub(crate) catalog: RwLock<Catalog>,
    state: AtomicU8,
    done: AtomicU64,
    total: AtomicU64,
    generation: AtomicU64,
    /// Bumped by every commit (a cache key for resolve results).
    epoch: AtomicU64,
    cache: Mutex<HashMap<(u32, String), Arc<StubRecord>>>,
    persistent: bool,
    pub log: Mutex<Vec<String>>,
}

/// The Java indexes of one project. Cheap to clone.
#[derive(Clone)]
pub struct JavaIndex {
    pub(crate) inner: Arc<Inner>,
}

const S_ABSENT: u8 = 0;
const S_BUILDING: u8 = 1;
const S_READY: u8 = 2;

// ---- entry building -------------------------------------------------------------

type Entries = BTreeMap<&'static str, Vec<(Vec<u8>, Vec<u8>)>>;
type Grouped = BTreeMap<&'static str, BTreeMap<String, Vec<Vec<u8>>>>;

fn enc<T: Serialize>(v: &T) -> Vec<u8> {
    postcard::to_stdvec(v).unwrap_or_default()
}

fn add(out: &mut Grouped, id: IndexId, key: &str, value: Vec<u8>) {
    out.entry(id.0).or_default().entry(key.to_string()).or_default().push(value);
}

/// The store keeps one value per (key, unit): several values of a key are one list.
fn finish(grouped: Grouped) -> Entries {
    grouped
        .into_iter()
        .map(|(id, keys)| {
            let list = keys
                .into_iter()
                .map(|(k, mut values)| {
                    values.sort();
                    values.dedup();
                    (k.into_bytes(), enc(&values))
                })
                .collect();
            (id, list)
        })
        .collect()
}

fn dec_list(bytes: &[u8]) -> Vec<Vec<u8>> {
    postcard::from_bytes(bytes).unwrap_or_default()
}

/// All index entries one compilation unit contributes (JAVA-STUB-011, 012, 013).
pub fn entries_for(files: &[FileStub]) -> Entries {
    let mut out = Grouped::new();
    for f in files {
        // Package and every parent package.
        if !f.package.is_empty() && !f.classes.is_empty() {
            let mut p = f.package.as_str();
            add(&mut out, ids::PACKAGE, p, vec![1]);
            while let Some((parent, _)) = p.rsplit_once('.') {
                add(&mut out, ids::PACKAGE, parent, vec![0]);
                p = parent;
            }
        }
        for top in &f.classes {
            let record = StubRecord {
                package: f.package.clone(),
                imports: f.imports.clone(),
                class: top.clone(),
            };
            add(&mut out, ids::STUB, &top.qname, enc(&record));
            let mut path = Vec::new();
            top.walk(&mut path, &mut |p, c| {
                let cref = ClassRef {
                    top: top.qname.clone(),
                    path: p[1..].iter().map(|s| s.to_string()).collect(),
                };
                let bytes = enc(&cref);
                add(&mut out, ids::CLASS, &c.qname, bytes.clone());
                add(&mut out, ids::SHORT_NAME, &c.name, bytes.clone());
                for (i, m) in c.methods.iter().enumerate() {
                    let mref = MemberRef { class: cref.clone(), index: i as u32 };
                    add(&mut out, ids::METHOD, &m.name, enc(&mref));
                }
                for (i, fl) in c.fields.iter().enumerate() {
                    let mref = MemberRef { class: cref.clone(), index: i as u32 };
                    add(&mut out, ids::FIELD, &fl.name, enc(&mref));
                }
                let mut annotations: Vec<&str> = c.annotations.iter().map(|a| annotation_short_name(a)).collect();
                annotations.sort_unstable();
                annotations.dedup();
                for a in annotations {
                    add(&mut out, ids::ANNOTATION, a, bytes.clone());
                }
                let mut supers: Vec<&str> = c
                    .extends
                    .iter()
                    .chain(&c.implements)
                    .map(|t| short_type_name(t))
                    .collect();
                if c.kind == ClassKind::Enum {
                    supers.push("Enum");
                }
                if c.kind == ClassKind::Annotation {
                    supers.push("Annotation");
                }
                supers.sort_unstable();
                supers.dedup();
                for s in supers {
                    add(&mut out, ids::SUPER_CLASS, s, bytes.clone());
                }
            });
        }
    }
    finish(out)
}

/// Drops private members of compiled classes except constructors (kept so that
/// "has private access" can be reported); saves most of the persisted size.
fn prune_compiled(c: &mut ClassStub) {
    c.fields.retain(|f| f.flags & flags::PRIVATE == 0);
    c.methods.retain(|m| m.flags & flags::PRIVATE == 0 || m.is_ctor);
    c.inner.iter_mut().for_each(prune_compiled);
}

impl JavaIndex {
    /// Opens the indexes in `dir` (persistent) or in memory.
    pub fn open(dir: Option<&Path>) -> Self {
        let (store, persistent, note): (Arc<dyn DurableStore>, bool, Option<String>) = match dir {
            Some(dir) => match RedbStore::open(dir) {
                Ok(s) => (Arc::new(s), true, None),
                Err(e) => (Arc::new(MemStore::new()), false, Some(e.to_string())),
            },
            None => (Arc::new(MemStore::new()), false, None),
        };
        let mut reused = true;
        for id in ids::ALL {
            match store.open_index(id, STUB_VERSION) {
                Ok(kept) => reused &= kept && store.is_complete(id),
                Err(_) => reused = false,
            }
        }
        let mut catalog = Catalog::default();
        for rec in store.load_files() {
            catalog.next_id = catalog.next_id.max(rec.id.0 + 1);
            catalog.by_path.insert(rec.path.clone(), rec.id.0);
        }
        let inner = Inner {
            store,
            catalog: RwLock::new(catalog),
            state: AtomicU8::new(if reused { S_READY } else { S_ABSENT }),
            done: AtomicU64::new(0),
            total: AtomicU64::new(0),
            generation: AtomicU64::new(0),
            epoch: AtomicU64::new(1),
            cache: Mutex::new(HashMap::new()),
            persistent,
            log: Mutex::new(note.into_iter().collect()),
        };
        Self { inner: Arc::new(inner) }
    }

    pub fn in_memory() -> Self {
        Self::open(None)
    }

    pub fn is_persistent(&self) -> bool {
        self.inner.persistent
    }

    pub fn state(&self) -> JavaIndexState {
        match self.inner.state.load(Ordering::Acquire) {
            S_READY => JavaIndexState::Ready,
            S_BUILDING => JavaIndexState::Building {
                done: self.inner.done.load(Ordering::Relaxed) as u32,
                total: self.inner.total.load(Ordering::Relaxed) as u32,
            },
            _ => JavaIndexState::Absent,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.state() == JavaIndexState::Ready
    }

    /// Changes with every commit; resolve caches key on it.
    pub fn epoch(&self) -> u64 {
        self.inner.epoch.load(Ordering::Acquire)
    }

    fn ready(&self) -> Result<(), NotReady> {
        if self.is_ready() { Ok(()) } else { Err(NotReady) }
    }

    /// Blocks until the indexes are ready (tests, headless tools).
    pub fn wait_ready(&self, timeout: std::time::Duration) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if self.is_ready() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        self.is_ready()
    }

    // ---- sync ---------------------------------------------------------------------

    /// Enumerates the units of `model`, reindexes the ones that changed on a background
    /// thread and removes the ones that are gone (JAVA-STUB-018). Replaces a running sync.
    pub fn sync(&self, model: Arc<ProjectModel>) {
        let generation = self.inner.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let plan = self.plan(&model);
        let first_build = self.inner.state.load(Ordering::Acquire) == S_ABSENT;
        if first_build {
            self.inner.state.store(S_BUILDING, Ordering::Release);
        }
        let this = self.clone();
        self.inner.done.store(0, Ordering::Relaxed);
        self.inner.total.store(plan.work.len() as u64, Ordering::Relaxed);
        std::thread::spawn(move || this.run(plan, generation));
    }

    /// Synchronous variant of [`Self::sync`].
    pub fn sync_blocking(&self, model: Arc<ProjectModel>) {
        let generation = self.inner.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let plan = self.plan(&model);
        self.inner.state.store(S_BUILDING, Ordering::Release);
        self.inner.done.store(0, Ordering::Relaxed);
        self.inner.total.store(plan.work.len() as u64, Ordering::Relaxed);
        self.run(plan, generation);
    }

    fn unit_id(&self, path: &Path) -> u32 {
        let mut cat = self.inner.catalog.write();
        if let Some(id) = cat.by_path.get(path) {
            return *id;
        }
        let id = cat.next_id.max(1);
        cat.next_id = id + 1;
        cat.by_path.insert(path.to_path_buf(), id);
        id
    }

    fn plan(&self, model: &ProjectModel) -> Plan {
        let mut work: Vec<Work> = Vec::new();
        let mut modules: HashMap<String, (Vec<u32>, Vec<u32>)> = HashMap::new();
        let mut libraries: HashMap<String, Vec<u32>> = HashMap::new();
        let mut sdks: HashMap<String, Vec<u32>> = HashMap::new();
        let mut seen: HashMap<PathBuf, u32> = HashMap::new();
        let mut units: HashMap<u32, UnitInfo> = HashMap::new();
        let mut push = |this: &Self, path: PathBuf, kind: UnitKind, release: u32| -> u32 {
            if let Some(id) = seen.get(&path) {
                return *id;
            }
            let id = this.unit_id(&path);
            seen.insert(path.clone(), id);
            units.insert(id, UnitInfo { id, path: path.clone(), kind: kind.clone() });
            work.push(Work { id, path, kind, release });
            id
        };
        for module in &model.modules {
            let release: u32 = module.language_level.as_deref().and_then(|l| l.trim_start_matches("1.").parse().ok()).unwrap_or(21);
            let entry = modules.entry(module.name.clone()).or_default();
            let excluded: Vec<&PathBuf> = module.excluded_folders().collect();
            for folder in module.source_folders() {
                if !matches!(
                    folder.root_type,
                    JpsModuleSourceRootType::Source
                        | JpsModuleSourceRootType::TestSource
                        | JpsModuleSourceRootType::Generated
                        | JpsModuleSourceRootType::GeneratedTest
                ) {
                    continue;
                }
                let test = folder.is_test_source();
                let walker = walkdir::WalkDir::new(&folder.path)
                    .into_iter()
                    .filter_entry(|e| !excluded.iter().any(|x| e.path() == x.as_path()));
                for e in walker.filter_map(Result::ok) {
                    if e.file_type().is_file() && e.path().extension().and_then(|x| x.to_str()) == Some("java") {
                        let id = push(self, e.path().to_path_buf(), UnitKind::Source, release);
                        let list = if test { &mut entry.1 } else { &mut entry.0 };
                        if !list.contains(&id) {
                            list.push(id);
                        }
                    }
                }
            }
        }
        for lib in &model.libraries {
            let list = libraries.entry(lib.name.clone()).or_default();
            for root in &lib.classes {
                let kind = if root.is_dir() { UnitKind::ClassDir } else { UnitKind::Jar };
                if root.exists() {
                    let id = push(self, root.clone(), kind, 21);
                    if !list.contains(&id) {
                        list.push(id);
                    }
                }
            }
        }
        for sdk in &model.sdks {
            if sdk.sdk_type != SdkType::Jdk {
                continue;
            }
            let list = sdks.entry(sdk.name.clone()).or_default();
            for root in &sdk.classes {
                if root.is_file() {
                    let id = push(self, root.clone(), UnitKind::Jdk, 21);
                    if !list.contains(&id) {
                        list.push(id);
                    }
                }
            }
        }
        // Units that disappeared.
        let gone: Vec<u32> = {
            let cat = self.inner.catalog.read();
            cat.by_path.iter().filter(|(p, id)| !seen.contains_key(*p) && **id != 0).map(|(_, id)| *id).collect()
        };
        Plan { work, units, modules, libraries, sdks, gone }
    }

    fn signature(path: &Path, kind: &UnitKind) -> (u64, Option<u64>) {
        match kind {
            UnitKind::ClassDir => {
                // Listing signature: names, sizes and mtimes of the class files.
                let mut acc: Vec<u8> = Vec::new();
                for e in walkdir::WalkDir::new(path).sort_by_file_name().into_iter().filter_map(Result::ok) {
                    if e.path().extension().and_then(|x| x.to_str()) == Some("class")
                        && let Ok(md) = e.metadata()
                    {
                        acc.extend(e.path().as_os_str().as_encoded_bytes());
                        acc.extend(md.len().to_le_bytes());
                        let m = md.modified().ok().and_then(|t| FileRecord::mtime(Some(t))).unwrap_or(0);
                        acc.extend(m.to_le_bytes());
                    }
                }
                (acc.len() as u64, Some(content_hash(&acc)))
            }
            _ => match std::fs::metadata(path) {
                Ok(md) => (md.len(), FileRecord::mtime(md.modified().ok())),
                Err(_) => (0, None),
            },
        }
    }

    fn run(&self, plan: Plan, generation: u64) {
        let Plan { work, units, modules, libraries, sdks, gone } = plan;
        let store = self.inner.store.clone();
        // Publish the catalog first: the scope can be computed while units are still being read.
        {
            let mut cat = self.inner.catalog.write();
            cat.units = units;
            cat.modules = modules;
            cat.libraries = libraries;
            cat.sdks = sdks;
        }
        let records: HashMap<u32, FileRecord> = store.load_files().into_iter().map(|r| (r.id.0, r)).collect();
        let cursor = AtomicU64::new(0);
        let workers = std::thread::available_parallelism().map_or(2, |n| n.get().clamp(1, 4));
        let upserts: Mutex<Vec<FileRecord>> = Mutex::new(Vec::new());
        let changed = AtomicU64::new(0);
        std::thread::scope(|s| {
            for _ in 0..workers {
                s.spawn(|| {
                    loop {
                        if self.inner.generation.load(Ordering::Acquire) != generation {
                            return;
                        }
                        let i = cursor.fetch_add(1, Ordering::AcqRel) as usize;
                        let Some(w) = work.get(i) else { return };
                        let (len, mtime) = Self::signature(&w.path, &w.kind);
                        let prev = records.get(&w.id);
                        let unchanged_stat = prev.is_some_and(|r| {
                            r.len == len
                                && r.mtime_ns == mtime
                                && mtime.is_some()
                                && store.indexed_hash(ids::STUB, FileId(w.id)) == Some(r.hash)
                        });
                        if !unchanged_stat {
                            let result = self.index_unit(w, len);
                            match result {
                                Ok(hash) => {
                                    changed.fetch_add(1, Ordering::Relaxed);
                                    upserts.lock().push(FileRecord { id: FileId(w.id), path: w.path.clone(), len, mtime_ns: mtime, hash });
                                }
                                Err(e) => {
                                    self.inner.log.lock().push(format!("{}: {e}", w.path.display()));
                                    for id in ids::ALL {
                                        store.remove_file(id, FileId(w.id));
                                    }
                                    upserts.lock().push(FileRecord { id: FileId(w.id), path: w.path.clone(), len, mtime_ns: None, hash: 0 });
                                }
                            }
                        }
                        self.inner.done.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
        });
        if self.inner.generation.load(Ordering::Acquire) != generation {
            return;
        }
        for id in &gone {
            for index in ids::ALL {
                store.remove_file(index, FileId(*id));
                store.set_indexed_hash(index, FileId(*id), None);
            }
        }
        {
            let mut cat = self.inner.catalog.write();
            for id in &gone {
                if let Some(p) = cat.by_path.iter().find(|(_, v)| **v == *id).map(|(p, _)| p.clone()) {
                    cat.by_path.remove(&p);
                }
            }
        }
        let removed: Vec<FileId> = gone.iter().map(|i| FileId(*i)).collect();
        store.save_files(&upserts.lock(), &removed);
        for index in ids::ALL {
            store.commit(index);
            store.set_complete(index, true);
        }
        self.inner.cache.lock().clear();
        self.inner.epoch.fetch_add(1, Ordering::AcqRel);
        self.inner.state.store(S_READY, Ordering::Release);
    }

    /// Reads and indexes one unit; returns its content hash.
    fn index_unit(&self, w: &Work, len: u64) -> Result<u64, String> {
        let (stubs, hash): (UnitStubs, u64) = match w.kind {
            UnitKind::Source => {
                let bytes = std::fs::read(&w.path).map_err(|e| e.to_string())?;
                let hash = content_hash(&bytes);
                if self.inner.store.indexed_hash(ids::STUB, FileId(w.id)) == Some(hash) {
                    return Ok(hash);
                }
                let text = String::from_utf8_lossy(&bytes);
                (UnitStubs { files: vec![source::build_file_stub(&text)], skipped: 0 }, hash)
            }
            UnitKind::Jar => {
                let u = unit::read_jar(&w.path, w.release)?;
                let hash = Self::signature(&w.path, &w.kind).1.unwrap_or(len);
                (u, hash)
            }
            UnitKind::ClassDir => {
                let u = unit::read_class_dir(&w.path);
                (u, Self::signature(&w.path, &w.kind).1.unwrap_or(0))
            }
            UnitKind::Jdk => {
                let u = unit::read_jdk_image(&w.path)?;
                (u, Self::signature(&w.path, &w.kind).1.unwrap_or(len))
            }
        };
        let mut files = stubs.files;
        if w.kind != UnitKind::Source {
            files.iter_mut().flat_map(|f| f.classes.iter_mut()).for_each(prune_compiled);
        }
        self.write_unit(w.id, &files, hash);
        Ok(hash)
    }

    fn write_unit(&self, id: u32, files: &[FileStub], hash: u64) {
        let entries = entries_for(files);
        for index in ids::ALL {
            let list = entries.get(index.0).cloned().unwrap_or_default();
            self.inner.store.put(index, FileId(id), list);
            self.inner.store.set_indexed_hash(index, FileId(id), Some(hash));
        }
    }

    /// Reindexes one source file from `text` (an unsaved buffer or a file the watcher saw).
    /// A no-op when the stub did not change (JAVA-STUB-003, 018).
    pub fn update_source(&self, path: &Path, text: &str, module: Option<(&str, bool)>) {
        let id = self.unit_id(path);
        {
            let mut cat = self.inner.catalog.write();
            cat.units.entry(id).or_insert(UnitInfo { id, path: path.to_path_buf(), kind: UnitKind::Source });
            if let Some((m, test)) = module {
                let e = cat.modules.entry(m.to_string()).or_default();
                let list = if test { &mut e.1 } else { &mut e.0 };
                if !list.contains(&id) {
                    list.push(id);
                }
            }
        }
        let hash = content_hash(text.as_bytes());
        if self.inner.store.indexed_hash(ids::STUB, FileId(id)) == Some(hash) {
            return;
        }
        let stub = source::build_file_stub(text);
        self.write_unit(id, std::slice::from_ref(&stub), hash);
        for index in ids::ALL {
            self.inner.store.commit(index);
        }
        self.inner.cache.lock().clear();
        self.inner.epoch.fetch_add(1, Ordering::AcqRel);
    }

    /// Forgets a deleted source file.
    pub fn remove_source(&self, path: &Path) {
        let id = {
            let cat = self.inner.catalog.read();
            cat.by_path.get(path).copied()
        };
        let Some(id) = id else { return };
        for index in ids::ALL {
            self.inner.store.remove_file(index, FileId(id));
            self.inner.store.set_indexed_hash(index, FileId(id), None);
            self.inner.store.commit(index);
        }
        let mut cat = self.inner.catalog.write();
        cat.units.remove(&id);
        for (a, b) in cat.modules.values_mut() {
            a.retain(|u| *u != id);
            b.retain(|u| *u != id);
        }
        drop(cat);
        self.inner.cache.lock().clear();
        self.inner.epoch.fetch_add(1, Ordering::AcqRel);
    }

    pub fn unit(&self, id: u32) -> Option<UnitInfo> {
        self.inner.catalog.read().units.get(&id).cloned()
    }

    pub fn unit_for_path(&self, path: &Path) -> Option<u32> {
        self.inner.catalog.read().by_path.get(path).copied()
    }

    // ---- queries ------------------------------------------------------------------

    fn record(&self, unit: u32, top: &str) -> Option<Arc<StubRecord>> {
        let key = (unit, top.to_string());
        if let Some(r) = self.inner.cache.lock().get(&key) {
            return Some(r.clone());
        }
        let rows = self.inner.store.get(ids::STUB, top.as_bytes());
        let (_, bytes) = rows.into_iter().find(|(f, _)| f.0 == unit)?;
        let rec: StubRecord = postcard::from_bytes(&dec_list(&bytes).into_iter().next()?).ok()?;
        let rec = Arc::new(rec);
        let mut cache = self.inner.cache.lock();
        if cache.len() > 8192 {
            cache.clear();
        }
        cache.insert(key, rec.clone());
        Some(rec)
    }

    fn hits(&self, index: IndexId, key: &str, scope: &ResolveScope) -> Vec<(u32, u32, Vec<u8>)> {
        let mut out: Vec<(u32, u32, Vec<u8>)> = Vec::new();
        for (f, v) in self.inner.store.get(index, key.as_bytes()) {
            if let Some(r) = scope.rank(f.0) {
                out.extend(dec_list(&v).into_iter().map(|one| (f.0, r, one)));
            }
        }
        // Deterministic order (PLAT-IDX-041): classpath rank, then unit id.
        out.sort_by_key(|(u, r, _)| (*r, *u));
        out
    }

    fn class_hit(&self, unit: u32, rank: u32, cref: &ClassRef) -> Option<ClassHit> {
        let record = self.record(unit, &cref.top)?;
        Some(ClassHit { unit, rank, record, path: cref.path.clone(), local: None, local_supers: None })
    }

    /// All classes with this FQN in scope, best first (`find_classes`, PLAT-IDX-091).
    pub fn find_classes(&self, fqn: &str, scope: &ResolveScope) -> Result<Vec<ClassHit>, NotReady> {
        self.ready()?;
        let mut out = Vec::new();
        for (unit, rank, v) in self.hits(ids::CLASS, fqn, scope) {
            if let Ok(cref) = postcard::from_bytes::<ClassRef>(&v)
                && let Some(h) = self.class_hit(unit, rank, &cref)
            {
                out.push(h);
            }
        }
        Ok(out)
    }

    /// The best class with this FQN (`find_class`: the earliest on the classpath;
    /// a source copy beats a class-file copy of the same dependency, JAVA-RES-026).
    pub fn find_class(&self, fqn: &str, scope: &ResolveScope) -> Result<Option<ClassHit>, NotReady> {
        Ok(self.find_classes(fqn, scope)?.into_iter().next())
    }

    pub fn classes_by_short_name(&self, name: &str, scope: &ResolveScope) -> Result<Vec<ClassHit>, NotReady> {
        self.ready()?;
        let mut out = Vec::new();
        for (unit, rank, v) in self.hits(ids::SHORT_NAME, name, scope) {
            if let Ok(cref) = postcard::from_bytes::<ClassRef>(&v)
                && let Some(h) = self.class_hit(unit, rank, &cref)
            {
                out.push(h);
            }
        }
        Ok(out)
    }

    fn members(&self, index: IndexId, name: &str, scope: &ResolveScope) -> Vec<MemberHit> {
        let mut out = Vec::new();
        for (unit, rank, v) in self.hits(index, name, scope) {
            if let Ok(m) = postcard::from_bytes::<MemberRef>(&v)
                && let Some(class) = self.class_hit(unit, rank, &m.class)
            {
                out.push(MemberHit { class, index: m.index as usize });
            }
        }
        out
    }

    /// Methods and constructors named `name` (constructors are filtered by consumers).
    pub fn methods_named(&self, name: &str, scope: &ResolveScope) -> Result<Vec<MemberHit>, NotReady> {
        self.ready()?;
        Ok(self.members(ids::METHOD, name, scope))
    }

    pub fn fields_named(&self, name: &str, scope: &ResolveScope) -> Result<Vec<MemberHit>, NotReady> {
        self.ready()?;
        Ok(self.members(ids::FIELD, name, scope))
    }

    /// Classes annotated with an annotation of this short name (the consumer resolves to confirm).
    pub fn annotated_with(&self, short: &str, scope: &ResolveScope) -> Result<Vec<ClassHit>, NotReady> {
        self.ready()?;
        Ok(self.refs(ids::ANNOTATION, short, scope))
    }

    /// Direct inheritor candidates by short name (JAVA-STUB-011).
    pub fn inheritors_of(&self, short: &str, scope: &ResolveScope) -> Result<Vec<ClassHit>, NotReady> {
        self.ready()?;
        Ok(self.refs(ids::SUPER_CLASS, short, scope))
    }

    fn refs(&self, index: IndexId, key: &str, scope: &ResolveScope) -> Vec<ClassHit> {
        self.hits(index, key, scope)
            .into_iter()
            .filter_map(|(unit, rank, v)| {
                let cref: ClassRef = postcard::from_bytes(&v).ok()?;
                self.class_hit(unit, rank, &cref)
            })
            .collect()
    }

    /// Whether some class or directory in scope lives in `package` or below (JAVA-RES-027).
    pub fn has_package(&self, package: &str, scope: &ResolveScope) -> Result<bool, NotReady> {
        self.ready()?;
        Ok(!self.hits(ids::PACKAGE, package, scope).is_empty())
    }

    /// Every key of a class-name index (all short names), for completion (06).
    pub fn all_short_names(&self) -> Result<Vec<String>, NotReady> {
        self.ready()?;
        Ok(self
            .inner
            .store
            .keys(ids::SHORT_NAME)
            .into_iter()
            .filter_map(|k| String::from_utf8(k).ok())
            .collect())
    }
}

struct Work {
    id: u32,
    path: PathBuf,
    kind: UnitKind,
    release: u32,
}

struct Plan {
    work: Vec<Work>,
    units: HashMap<u32, UnitInfo>,
    modules: HashMap<String, (Vec<u32>, Vec<u32>)>,
    libraries: HashMap<String, Vec<u32>>,
    sdks: HashMap<String, Vec<u32>>,
    gone: Vec<u32>,
}

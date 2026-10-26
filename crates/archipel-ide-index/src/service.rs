//! The indexing scheduler: scan, name indexes, content sweep, overlay for
//! open buffers, per-index readiness and dumb mode.
//!
//! [`IndexService`] lives next to the `AnalysisHost` (UI thread, `&mut`).
//! Per frame the app calls [`IndexService::poll`]; background work runs as
//! host jobs with `Priority::Background` (so interactive jobs never queue
//! behind indexing) that send results through a shared sink.
//!
//! Order of work (spec 1.6): (1) scan the tree and build `FileName` +
//! `FileType` (makes Go to File work almost immediately), (2) open and
//! recently changed files, (3) source files of languages with a grammar,
//! (4) everything else. Every file is read once, hashed, parsed once and fed
//! to all content indexes that have not seen this content yet. Batches
//! commit after ~50 ms or `batch_files` files and publish
//! `Change::IndexCommit` so readers refresh.

use crate::files::{DEFAULT_EXCLUDES, DiskLister, FileLister, RootedVfs, ScanEntry};
use crate::hash::content_hash;
use crate::indexes::MAX_SYNTAX_BYTES;
use crate::overlay::OverlayStore;
use crate::pipeline::{ErasedIndex, IndexSet};
use crate::progress::{IndexInfo, IndexStatus, Phase, SkippedFile};
use crate::redb_store::RedbStore;
use crate::store::{DurableStore, FileRecord, MemStore};
use crate::todo::TodoSettings;
use archipel_ide_analysis::index::{FileIndex, IndexInput, IndexStore};
use archipel_ide_analysis::vfs::{DiskVfs, Vfs, VfsChange, VfsError};
use archipel_ide_analysis::{
    AnalysisEvent, AnalysisHost, CancelToken, Change, FileId, FileMeta, FileRev, FileTable,
    IndexId, IndexStamp, IndexState, JobPriority, JobSpec, Revoke, Snapshot,
};
use archipel_ide_language::{SyntaxSnapshot, registry};
use parking_lot::Mutex;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Marks a record whose file was skipped (binary, too large).
const SKIPPED_HASH: u64 = u64::MAX;
/// Skipped files listed individually in the status.
const MAX_LISTED_SKIPS: usize = 200;

/// Priority tiers of the content queue (lower runs first).
mod tier {
    pub const OPEN: u8 = 0;
    pub const CHANGED: u8 = 1;
    pub const SOURCE: u8 = 2;
    pub const REST: u8 = 3;
}

/// Tuning and wiring of the service.
#[derive(Clone)]
pub struct IndexConfig {
    /// Directory of the persistent store (`<cache>/archipel/index/<project-hash>`);
    /// `None` keeps everything in memory.
    pub store_dir: Option<PathBuf>,
    /// Content batches running at once.
    pub max_parallel: usize,
    /// Files per content batch.
    pub batch_files: usize,
    /// A batch commits after this long even if it has files left.
    pub batch_time: Duration,
    /// Quiet time after typing before an open buffer is reindexed.
    pub overlay_debounce: Duration,
    pub exclude_dirs: Vec<String>,
    /// Pause after every batch (zero in the app; scenes and demos slow
    /// indexing down to show its progress).
    pub throttle: Duration,
    pub todo: TodoSettings,
    pub vfs: Arc<dyn Vfs>,
    pub lister: Arc<dyn FileLister>,
}

impl Default for IndexConfig {
    fn default() -> Self {
        let cores = std::thread::available_parallelism().map_or(2, |n| n.get());
        Self {
            store_dir: None,
            max_parallel: cores.saturating_sub(1).clamp(1, 3),
            batch_files: 48,
            batch_time: Duration::from_millis(50),
            overlay_debounce: Duration::from_millis(300),
            exclude_dirs: DEFAULT_EXCLUDES.iter().map(|s| (*s).to_string()).collect(),
            throttle: Duration::ZERO,
            todo: TodoSettings::default(),
            vfs: Arc::new(DiskVfs),
            lister: Arc::new(DiskLister),
        }
    }
}

impl IndexConfig {
    /// Deterministic settings for tests (no debounce, small batches).
    pub fn testing() -> Self {
        Self {
            max_parallel: 1,
            batch_files: 16,
            batch_time: Duration::from_secs(5),
            overlay_debounce: Duration::ZERO,
            ..Self::default()
        }
    }
}

// ---- messages from jobs ---------------------------------------------------------

#[derive(Debug, Clone)]
struct WorkItem {
    file: FileId,
    path: PathBuf,
    known: Option<FileRecord>,
    buffer: bool,
    /// Second pass: only the text-only indexes.
    text_pass: bool,
}

#[derive(Debug)]
enum Outcome {
    Indexed {
        hash: u64,
        len: u64,
        mtime_ns: Option<u64>,
    },
    Unchanged {
        hash: u64,
        len: u64,
        mtime_ns: Option<u64>,
    },
    Skipped {
        reason: String,
        len: u64,
        mtime_ns: Option<u64>,
    },
    Gone,
    BufferIndexed,
    BufferDropped,
    /// Not processed (time budget or cancellation): queue it again.
    Deferred,
}

struct ContentResult {
    items: Vec<(WorkItem, Outcome)>,
    stamps: Vec<(IndexId, IndexStamp)>,
}

enum Msg {
    Scan {
        entries: Vec<ScanEntry>,
        dir: Option<PathBuf>,
        millis: u64,
    },
    Names {
        files: usize,
        stamps: Vec<(IndexId, IndexStamp)>,
    },
    Content(ContentResult),
    Removed {
        stamps: Vec<(IndexId, IndexStamp)>,
    },
}

/// Counters of what the sweep did (diagnostics and tests).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// Files whose content was (re)indexed.
    pub indexed: u64,
    /// Files skipped because their content was already indexed.
    pub unchanged: u64,
    /// Open buffers reindexed into the overlay.
    pub buffers: u64,
}

struct RunGuard(Arc<AtomicUsize>);

impl Drop for RunGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

// ---- the service ----------------------------------------------------------------

struct Attached {
    root: PathBuf,
    vfs: Arc<RootedVfs>,
    durable: Arc<dyn DurableStore>,
    overlay: Arc<OverlayStore>,
    store_error: Option<String>,
    records: HashMap<FileId, FileRecord>,
    queue: BTreeSet<(u8, u64, FileId)>,
    queued: HashMap<FileId, (u8, u64)>,
    seq: u64,
    names_queue: Vec<WorkItem>,
    names_in_flight: usize,
    /// Files waiting for the text-only pass (Trigram).
    text_queue: Vec<WorkItem>,
    text_total: u32,
    text_done: u32,
    text_started: bool,
    removal_queue: Vec<FileId>,
    in_flight: HashSet<FileId>,
    requeue: HashSet<FileId>,
    buffer_dirty: HashMap<FileId, Instant>,
    buffer_in_flight: HashSet<FileId>,
    results: Arc<Mutex<Vec<Msg>>>,
    jobs: Arc<AtomicUsize>,
    scans_running: usize,
    states: HashMap<IndexId, IndexState>,
    sweep_done: u32,
    sweep_total: u32,
    dumb: bool,
    save_upserts: HashMap<FileId, FileRecord>,
    save_removed: Vec<FileId>,
    skipped: Vec<SkippedFile>,
    skipped_total: u32,
    retries: HashMap<IndexId, u8>,
    scan_ms: Option<u64>,
    /// Files the watcher reported: reread even if size and mtime match.
    force: HashSet<FileId>,
    stats: Stats,
    last_change: Option<VfsChange>,
    /// The full scan has been reconciled (names and sweep were planned).
    scanned: bool,
}

/// See the module docs.
pub struct IndexService {
    cfg: IndexConfig,
    set: IndexSet,
    att: Option<Attached>,
}

impl IndexService {
    pub fn new(cfg: IndexConfig) -> Self {
        let set = IndexSet::builtin(&cfg.todo);
        Self {
            cfg,
            set,
            att: None,
        }
    }

    pub fn config(&self) -> &IndexConfig {
        &self.cfg
    }

    pub fn indexes(&self) -> &IndexSet {
        &self.set
    }

    /// Registers an additional (language-defined) index. Takes effect for
    /// the next attach, or immediately (queueing all files) when attached.
    pub fn register<I: FileIndex>(&mut self, host: &mut AnalysisHost, index: I) {
        self.set.add(Arc::new(crate::pipeline::Typed::new(index)));
        self.after_set_change(host, Some(I::ID));
    }

    pub fn is_attached(&self) -> bool {
        self.att.is_some()
    }

    pub fn root(&self) -> Option<&Path> {
        self.att.as_ref().map(|att| att.root.as_path())
    }

    /// The store the host serves to readers (`Snapshot::require`).
    pub fn overlay(&self) -> Option<&Arc<OverlayStore>> {
        self.att.as_ref().map(|att| &att.overlay)
    }

    /// Starts indexing `root` (project-relative paths are interned in the
    /// host's file table). Opens the persistent store when `store_dir` is
    /// set; falls back to memory with the reason in the status.
    pub fn attach(&mut self, host: &mut AnalysisHost, root: &Path) {
        self.detach(host);
        let (durable, store_error): (Arc<dyn DurableStore>, Option<String>) =
            match self.cfg.store_dir.as_deref() {
                Some(dir) => match RedbStore::open(dir) {
                    Ok(store) => (Arc::new(store), None),
                    Err(error) => (Arc::new(MemStore::new()), Some(error.to_string())),
                },
                None => (Arc::new(MemStore::new()), None),
            };
        let mut durable = durable;
        let mut store_error = store_error;
        let mut states = HashMap::new();
        let mut reused_all = true;
        for index in self.set.all() {
            match durable.open_index(index.id(), &index.version()) {
                Ok(reused) => {
                    let ready = reused && durable.is_complete(index.id());
                    reused_all &= ready;
                    states.insert(
                        index.id(),
                        if ready {
                            IndexState::Ready
                        } else {
                            IndexState::Absent
                        },
                    );
                }
                Err(error) => {
                    // One unusable index file: use memory for this session.
                    store_error = Some(error.to_string());
                    durable = Arc::new(MemStore::new());
                    states.clear();
                    reused_all = false;
                    for index in self.set.all() {
                        let _ = durable.open_index(index.id(), &index.version());
                        states.insert(index.id(), IndexState::Absent);
                    }
                    break;
                }
            }
        }
        // Stable file ids across restarts: restore the persisted table when
        // the host has none yet (otherwise ids would not match the stored
        // postings, so start from scratch).
        let mut records: HashMap<FileId, FileRecord> = HashMap::new();
        let persisted = durable.load_files();
        if !persisted.is_empty() {
            let table = FileTable::restore(persisted.iter().map(|r| (r.id, r.path.clone())));
            if host.restore_files(table) {
                records = persisted.into_iter().map(|r| (r.id, r)).collect();
            } else {
                for index in self.set.all() {
                    durable.clear(index.id());
                    let _ = durable.open_index(index.id(), &index.version());
                    states.insert(index.id(), IndexState::Absent);
                }
                durable.save_files(&[], &persisted.iter().map(|r| r.id).collect::<Vec<_>>());
                reused_all = false;
            }
        }
        let _ = reused_all;
        let overlay = Arc::new(OverlayStore::new(durable.clone()));
        host.set_index_store(overlay.clone());
        let vfs = Arc::new(RootedVfs {
            root: root.to_path_buf(),
            inner: self.cfg.vfs.clone(),
        });
        let mut att = Attached {
            root: root.to_path_buf(),
            vfs,
            durable,
            overlay,
            store_error,
            records,
            queue: BTreeSet::new(),
            queued: HashMap::new(),
            seq: 0,
            names_queue: Vec::new(),
            names_in_flight: 0,
            text_queue: Vec::new(),
            text_total: 0,
            text_done: 0,
            text_started: false,
            removal_queue: Vec::new(),
            in_flight: HashSet::new(),
            requeue: HashSet::new(),
            buffer_dirty: HashMap::new(),
            buffer_in_flight: HashSet::new(),
            results: Arc::default(),
            jobs: Arc::default(),
            scans_running: 0,
            states,
            sweep_done: 0,
            sweep_total: 0,
            dumb: false,
            save_upserts: HashMap::new(),
            save_removed: Vec::new(),
            skipped: Vec::new(),
            skipped_total: 0,
            retries: HashMap::new(),
            scan_ms: None,
            force: HashSet::new(),
            stats: Stats::default(),
            last_change: None,
            scanned: false,
        };
        att.publish_all(host, &self.set);
        att.update_dumb(host, &self.set);
        att.start_scan(host, &self.cfg, None);
        self.att = Some(att);
    }

    /// Stops indexing (project closed). Persistent data stays on disk.
    pub fn detach(&mut self, host: &mut AnalysisHost) {
        if let Some(mut att) = self.att.take() {
            att.flush_records();
            if att.dumb {
                host.apply(Change::DumbMode(false));
            }
        }
    }

    /// Whether nothing is queued or running.
    pub fn is_idle(&self) -> bool {
        self.att.as_ref().is_none_or(Attached::is_idle)
    }

    pub fn stats(&self) -> Stats {
        self.att.as_ref().map(|att| att.stats).unwrap_or_default()
    }

    /// One scheduling round (deterministic stepping for tests).
    pub fn poll_step(&mut self, host: &mut AnalysisHost) -> bool {
        let Some(att) = self.att.as_mut() else {
            return false;
        };
        att.step(host, &self.cfg, &self.set)
    }

    pub fn status(&self) -> IndexStatus {
        match &self.att {
            Some(att) => att.status(&self.set),
            None => IndexStatus::detached(),
        }
    }

    /// Per-frame work: applies finished batches, starts new ones. Returns
    /// true when anything changed.
    pub fn poll(&mut self, host: &mut AnalysisHost) -> bool {
        let Some(att) = self.att.as_mut() else {
            return false;
        };
        att.poll(host, &self.cfg, &self.set)
    }

    /// Runs [`IndexService::poll`] (and the host's) until the service is
    /// idle or `timeout` passes. For tests, benches and `index.rebuild --wait`.
    pub fn run_until_idle(&mut self, host: &mut AnalysisHost, timeout: Duration) -> bool {
        let started = Instant::now();
        loop {
            host.poll();
            self.poll(host);
            if self.is_idle() {
                return true;
            }
            if started.elapsed() > timeout {
                return false;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Feeds host events (open, edit, close, file changes).
    pub fn on_events(&mut self, host: &mut AnalysisHost, events: &[AnalysisEvent]) {
        let Some(att) = self.att.as_mut() else {
            return;
        };
        let snapshot = host.snapshot();
        for event in events {
            match event {
                AnalysisEvent::FileOpened(file) => {
                    att.enqueue(*file, tier::OPEN, &snapshot, &self.set);
                    // A restored buffer may differ from the disk.
                    att.buffer_dirty.insert(*file, Instant::now());
                }
                AnalysisEvent::SyntaxChanged { file, .. } => {
                    att.buffer_dirty.insert(*file, Instant::now());
                }
                AnalysisEvent::FileClosed(file) => {
                    att.buffer_dirty.remove(file);
                    if att.overlay.has_overlay(*file) {
                        att.overlay.drop_file(*file);
                        att.stamp_all(host, &self.set);
                    }
                    att.enqueue(*file, tier::CHANGED, &snapshot, &self.set);
                }
                AnalysisEvent::FilesChanged(batch) if att.last_change.as_ref() != Some(batch) => {
                    att.apply_change(batch, &snapshot, &self.set);
                }
                _ => {}
            }
        }
    }

    /// A watcher batch: project-relative paths whose content or existence
    /// changed. Classifies them, tells the host and schedules the work.
    pub fn notify_paths(&mut self, host: &mut AnalysisHost, paths: &[PathBuf]) {
        let Some(att) = self.att.as_mut() else {
            return;
        };
        let mut change = VfsChange::default();
        let mut rescan = Vec::new();
        let mut gone_dirs = Vec::new();
        for path in paths {
            if path.components().any(|c| {
                c.as_os_str()
                    .to_str()
                    .is_some_and(|n| self.cfg.exclude_dirs.iter().any(|e| e == n))
            }) {
                continue;
            }
            let absolute = att.vfs.absolute(path);
            let known = host.lookup(path);
            if absolute.is_dir() {
                rescan.push(absolute);
                continue;
            }
            match (known, att.vfs.metadata(path).is_some()) {
                (Some(id), true) => change.modified.push(id),
                (Some(id), false) => change.deleted.push(id),
                (None, true) => change.created.push(host.file_id(path)),
                (None, false) => gone_dirs.push(path.clone()),
            }
        }
        // A removed directory takes its files along.
        if !gone_dirs.is_empty() {
            let snapshot = host.snapshot();
            for (id, known) in snapshot.files().iter() {
                if gone_dirs.iter().any(|dir| known.starts_with(dir))
                    && !change.deleted.contains(&id)
                {
                    change.deleted.push(id);
                }
            }
        }
        if !change.is_empty() {
            host.apply(Change::Vfs(change.clone()));
            let snapshot = host.snapshot();
            att.apply_change(&change, &snapshot, &self.set);
            att.last_change = Some(change);
        }
        for dir in rescan {
            att.start_scan(host, &self.cfg, Some(dir));
        }
    }

    /// An explicit rename or move (ids are kept).
    pub fn notify_moved(&mut self, host: &mut AnalysisHost, from: &Path, to: &Path) {
        let Some(att) = self.att.as_mut() else {
            return;
        };
        if let Some(id) = host.lookup(from) {
            // A file previously at `to` is overwritten and retired.
            let replaced = host.lookup(to).filter(|replaced| *replaced != id);
            let change = VfsChange {
                moved: vec![(id, from.to_path_buf(), to.to_path_buf())],
                deleted: replaced.into_iter().collect(),
                ..VfsChange::default()
            };
            host.apply(Change::Vfs(change.clone()));
            let snapshot = host.snapshot();
            att.apply_change(&change, &snapshot, &self.set);
            att.last_change = Some(change);
        }
    }

    /// Drops and rebuilds `which` (all indexes when `None`).
    pub fn rebuild(&mut self, host: &mut AnalysisHost, which: Option<IndexId>) {
        let Some(att) = self.att.as_mut() else {
            return;
        };
        let snapshot = host.snapshot();
        match which {
            None => {
                for index in self.set.all() {
                    att.reset_index(index.as_ref());
                }
                att.records.clear();
                att.scanned = false;
                att.publish_all(host, &self.set);
                att.update_dumb(host, &self.set);
                att.start_scan(host, &self.cfg, None);
            }
            Some(id) => {
                if let Some(index) = self.set.get(id) {
                    att.reset_index(index.as_ref());
                    att.publish(host, id, IndexStamp(0));
                    att.queue_everything(&snapshot, &self.set, index.as_ref());
                    att.update_dumb(host, &self.set);
                }
            }
        }
    }

    /// New TODO settings (Settings > Editor > TODO): rebuilds only `Todo`
    /// when patterns or multiline scanning changed.
    pub fn set_todo_settings(&mut self, host: &mut AnalysisHost, settings: TodoSettings) {
        let rebuild = !self.cfg.todo.same_index_config(&settings);
        self.cfg.todo = settings;
        if rebuild {
            self.set.set_todo(&self.cfg.todo);
            self.after_set_change(host, Some(crate::indexes::ids::TODO));
        }
    }

    /// Sets the TODO settings without rebuilding (before the next attach).
    pub fn set_todo_settings_quiet(&mut self, settings: TodoSettings) {
        self.cfg.todo = settings;
        self.set.set_todo(&self.cfg.todo);
    }

    pub fn todo_settings(&self) -> &TodoSettings {
        &self.cfg.todo
    }

    /// Pause after every batch (diagnostics, scenes).
    pub fn set_throttle(&mut self, pause: Duration) {
        self.cfg.throttle = pause;
    }

    /// Where the persistent index lives (`None`: memory only). Applies to the
    /// next [`IndexService::attach`].
    pub fn set_store_dir(&mut self, dir: Option<PathBuf>) {
        self.cfg.store_dir = dir;
    }

    fn after_set_change(&mut self, host: &mut AnalysisHost, id: Option<IndexId>) {
        let Some(att) = self.att.as_mut() else {
            return;
        };
        let Some(id) = id else { return };
        let Some(index) = self.set.get(id) else {
            return;
        };
        let version = index.version();
        let kept = att.durable.open_index(id, &version).unwrap_or(false);
        att.overlay.clear_layer(id);
        let state = if kept && att.durable.is_complete(id) {
            IndexState::Ready
        } else {
            att.durable.set_complete(id, false);
            IndexState::Absent
        };
        att.states.insert(id, state);
        att.publish(host, id, IndexStamp(0));
        if state != IndexState::Ready {
            let snapshot = host.snapshot();
            att.queue_everything(&snapshot, &self.set, index.as_ref());
        }
        att.update_dumb(host, &self.set);
    }
}

// ---- Attached -------------------------------------------------------------------

fn language_of(path: &Path) -> Option<&'static archipel_ide_language::LanguageDef> {
    registry::detect(path, "")
}

impl Attached {
    fn is_idle(&self) -> bool {
        self.queue.is_empty()
            && self.text_queue.is_empty()
            && self
                .states
                .values()
                .all(|state| matches!(state, IndexState::Ready | IndexState::Failed))
            && self.names_queue.is_empty()
            && self.removal_queue.is_empty()
            && self.jobs.load(Ordering::SeqCst) == 0
            && self.scans_running == 0
            && self.results.lock().is_empty()
            && self.buffer_dirty.is_empty()
            && self.scanned
    }

    fn status(&self, set: &IndexSet) -> IndexStatus {
        let jobs = self.jobs.load(Ordering::SeqCst) as u32;
        let queued = (self.queue.len() + self.names_queue.len() + self.text_queue.len()) as u32;
        let (done, total) = if self.text_started {
            (self.text_done, self.text_total)
        } else {
            (self.sweep_done, self.sweep_total)
        };
        let phase = if self.scans_running > 0 || !self.scanned {
            Phase::Scanning
        } else if queued > 0 || jobs > 0 || !self.results.lock().is_empty() {
            Phase::Indexing
        } else {
            Phase::Idle
        };
        IndexStatus {
            attached: true,
            root: Some(self.root.clone()),
            phase,
            dumb: self.dumb,
            done,
            total,
            queued,
            running_jobs: jobs,
            files: self.records.len() as u32,
            indexes: set
                .all()
                .iter()
                .map(|index| IndexInfo {
                    id: index.id(),
                    state: self
                        .states
                        .get(&index.id())
                        .copied()
                        .unwrap_or(IndexState::Absent),
                    version: index.version(),
                    dumb_aware: index.dumb_aware(),
                    priority: index.priority(),
                })
                .collect(),
            skipped: self.skipped.clone(),
            skipped_total: self.skipped_total,
            store: self.durable.kind(),
            store_error: self.store_error.clone(),
            scan_ms: self.scan_ms,
        }
    }

    fn state(&self, id: IndexId) -> IndexState {
        self.states.get(&id).copied().unwrap_or(IndexState::Absent)
    }

    fn publish(&self, host: &mut AnalysisHost, id: IndexId, stamp: IndexStamp) {
        host.apply(Change::IndexCommit {
            index: id,
            stamp,
            state: self.state(id),
        });
    }

    fn publish_all(&self, host: &mut AnalysisHost, set: &IndexSet) {
        for index in set.all() {
            self.publish(host, index.id(), IndexStamp(0));
        }
    }

    /// Publishes a new stamp for every index (overlay changed).
    fn stamp_all(&self, host: &mut AnalysisHost, set: &IndexSet) {
        for index in set.content() {
            let stamp = self.overlay.commit(index.id());
            self.publish(host, index.id(), stamp);
        }
    }

    fn update_dumb(&mut self, host: &mut AnalysisHost, set: &IndexSet) {
        let dumb = set.all().iter().any(|index| {
            !index.dumb_aware()
                && matches!(
                    self.state(index.id()),
                    IndexState::Absent | IndexState::Building { .. }
                )
        });
        if dumb != self.dumb {
            self.dumb = dumb;
            host.apply(Change::DumbMode(dumb));
        }
    }

    fn reset_index(&mut self, index: &dyn ErasedIndex) {
        self.overlay.clear(index.id());
        let _ = self.durable.open_index(index.id(), &index.version());
        self.durable.set_complete(index.id(), false);
        self.states.insert(index.id(), IndexState::Absent);
        if index.text_only() {
            self.text_started = false;
            self.text_queue.clear();
        }
    }

    /// Queues every known file for `index` (settings or version changed).
    fn queue_everything(&mut self, snapshot: &Snapshot, set: &IndexSet, index: &dyn ErasedIndex) {
        if index.text_only() {
            // Rebuilt by the text pass once everything else is ready.
            return;
        }
        let content = index.depends_on_content();
        let ids: Vec<FileId> = self.records.keys().copied().collect();
        if ids.is_empty() {
            return;
        }
        for id in ids {
            if content {
                self.enqueue(id, tier::REST, snapshot, set);
            } else if let Some(record) = self.records.get(&id) {
                self.names_queue.push(WorkItem {
                    file: id,
                    path: record.path.clone(),
                    known: Some(record.clone()),
                    buffer: false,
                    text_pass: false,
                });
            }
        }
        self.reset_progress();
    }

    fn reset_progress(&mut self) {
        self.sweep_done = 0;
        self.sweep_total = self.queue.len() as u32;
    }

    fn enqueue(&mut self, file: FileId, wanted: u8, snapshot: &Snapshot, _set: &IndexSet) {
        if self.in_flight.contains(&file) {
            self.requeue.insert(file);
            return;
        }
        let Some(path) = snapshot.path(file) else {
            return;
        };
        let tier = if wanted == tier::REST && language_of(path).is_some_and(|l| l.has_tags()) {
            tier::SOURCE
        } else {
            wanted
        };
        match self.queued.get(&file).copied() {
            Some((existing, _)) if existing <= tier => {}
            Some((existing, seq)) => {
                self.queue.remove(&(existing, seq, file));
                self.insert_queue(file, tier);
            }
            None => {
                self.insert_queue(file, tier);
                self.sweep_total += 1;
            }
        }
    }

    fn insert_queue(&mut self, file: FileId, tier: u8) {
        self.seq += 1;
        self.queue.insert((tier, self.seq, file));
        self.queued.insert(file, (tier, self.seq));
    }

    fn apply_change(&mut self, batch: &VfsChange, snapshot: &Snapshot, set: &IndexSet) {
        for file in &batch.deleted {
            if !self.removal_queue.contains(file) {
                self.removal_queue.push(*file);
            }
            if let Some((tier, seq)) = self.queued.remove(file) {
                self.queue.remove(&(tier, seq, *file));
            }
        }
        for file in batch
            .created
            .iter()
            .chain(batch.moved.iter().map(|(f, _, _)| f))
        {
            if let Some(path) = snapshot.path(*file) {
                self.names_queue.push(WorkItem {
                    file: *file,
                    path: path.to_path_buf(),
                    known: None,
                    buffer: false,
                    text_pass: false,
                });
            }
            self.force.insert(*file);
            self.enqueue(*file, tier::CHANGED, snapshot, set);
        }
        for file in &batch.modified {
            self.force.insert(*file);
            self.enqueue(*file, tier::CHANGED, snapshot, set);
        }
    }

    fn start_scan(&mut self, host: &mut AnalysisHost, cfg: &IndexConfig, dir: Option<PathBuf>) {
        self.scans_running += 1;
        let lister = cfg.lister.clone();
        let excludes = cfg.exclude_dirs.clone();
        let root = self.root.clone();
        let target = dir.clone().unwrap_or_else(|| root.clone());
        let sink = self.results.clone();
        let jobs = self.jobs.clone();
        jobs.fetch_add(1, Ordering::SeqCst);
        let guard = RunGuard(jobs);
        host.spawn_with(
            JobSpec::new(JobPriority::Background, Revoke::Never),
            move |_, cancel| {
                let _guard = guard;
                let started = Instant::now();
                let entries = lister.list(&root, &target, &excludes, cancel);
                sink.lock().push(Msg::Scan {
                    entries,
                    dir,
                    millis: started.elapsed().as_millis() as u64,
                });
                Ok(())
            },
            |_| {},
        );
    }

    // ---- poll ----

    fn poll(&mut self, host: &mut AnalysisHost, cfg: &IndexConfig, set: &IndexSet) -> bool {
        let mut changed = false;
        let inline = host.config().threads == 0;
        for _ in 0..1_000_000 {
            let progressed = self.step(host, cfg, set);
            changed |= progressed;
            if !progressed || !inline {
                break;
            }
        }
        changed
    }

    /// One drain-and-schedule round. True when something happened.
    fn step(&mut self, host: &mut AnalysisHost, cfg: &IndexConfig, set: &IndexSet) -> bool {
        let mut progressed = false;
        let messages = std::mem::take(&mut *self.results.lock());
        for message in messages {
            progressed = true;
            self.apply_message(host, cfg, set, message);
        }
        for failed in self.durable.take_failed() {
            progressed = true;
            self.on_index_failed(host, set, failed);
        }
        // Buffers that were quiet long enough.
        let due: Vec<FileId> = self
            .buffer_dirty
            .iter()
            .filter(|(file, since)| {
                since.elapsed() >= cfg.overlay_debounce && !self.buffer_in_flight.contains(*file)
            })
            .map(|(file, _)| *file)
            .collect();
        let snapshot = host.snapshot();
        let mut buffer_items = Vec::new();
        for file in due {
            self.buffer_dirty.remove(&file);
            if let Some(path) = snapshot.path(file) {
                self.buffer_in_flight.insert(file);
                buffer_items.push(WorkItem {
                    file,
                    path: path.to_path_buf(),
                    known: None,
                    buffer: true,
                    text_pass: false,
                });
            }
        }
        if !buffer_items.is_empty() {
            progressed = true;
            self.spawn_content(host, cfg, set, buffer_items);
        }
        progressed |= self.schedule(host, cfg, set, &snapshot);
        if self.queue.is_empty()
            && self.names_queue.is_empty()
            && self.removal_queue.is_empty()
            && self.text_queue.is_empty()
            && self.jobs.load(Ordering::SeqCst) == 0
            && self.scans_running == 0
            && self.scanned
            && self.results.lock().is_empty()
            && self.finish_if_ready(host, set)
        {
            progressed = true;
        }
        if self.save_upserts.len() >= 2000 {
            self.flush_records();
        }
        progressed
    }

    fn schedule(
        &mut self,
        host: &mut AnalysisHost,
        cfg: &IndexConfig,
        set: &IndexSet,
        snapshot: &Snapshot,
    ) -> bool {
        let mut started = false;
        let capacity = cfg.max_parallel.max(1);
        // Removals run alone: a job writing entries of a file while its
        // removal runs would resurrect them. Pending removals hold back new
        // jobs until the running ones are done.
        if !self.removal_queue.is_empty() {
            if self.jobs.load(Ordering::SeqCst) == 0 {
                let files = std::mem::take(&mut self.removal_queue);
                self.spawn_removal(host, set, files);
                started = true;
            }
            return started;
        }
        // Name indexes ahead of contents.
        while !self.names_queue.is_empty() && self.jobs.load(Ordering::SeqCst) < capacity + 1 {
            let take = self.names_queue.len().min(4000);
            let items: Vec<WorkItem> = self
                .names_queue
                .drain(..take)
                .filter(|item| snapshot.path(item.file).is_some())
                .collect();
            if !items.is_empty() || self.names_queue.is_empty() {
                self.spawn_names(host, set, items);
            }
            started = true;
        }
        // Content only after the names pass has been issued and finished.
        if self.names_in_flight == 0 && self.names_queue.is_empty() {
            while self.jobs.load(Ordering::SeqCst) < capacity && !self.queue.is_empty() {
                let mut items = Vec::new();
                while items.len() < cfg.batch_files {
                    let Some(&(tier, seq, file)) = self.queue.iter().next() else {
                        break;
                    };
                    self.queue.remove(&(tier, seq, file));
                    self.queued.remove(&file);
                    let Some(path) = snapshot.path(file) else {
                        continue;
                    };
                    self.in_flight.insert(file);
                    items.push(WorkItem {
                        file,
                        path: path.to_path_buf(),
                        known: if self.force.remove(&file) {
                            None
                        } else {
                            self.records.get(&file).cloned()
                        },
                        buffer: false,
                        text_pass: false,
                    });
                }
                if items.is_empty() {
                    break;
                }
                self.spawn_content(host, cfg, set, items);
                started = true;
            }
        }
        // Text-only indexes (Trigram) after everything else is done.
        if self.names_in_flight == 0
            && self.names_queue.is_empty()
            && self.queue.is_empty()
            && self.in_flight.is_empty()
        {
            while self.jobs.load(Ordering::SeqCst) < capacity && !self.text_queue.is_empty() {
                let take = self.text_queue.len().min(cfg.batch_files * 2);
                let items: Vec<WorkItem> = self.text_queue.drain(..take).collect();
                self.spawn_content(host, cfg, set, items);
                started = true;
            }
        }
        started
    }

    fn job_ctx(&self, cfg: &IndexConfig, set: &IndexSet) -> JobCtx {
        JobCtx {
            text_active: set
                .text_content()
                .map(|index| index.id())
                .filter(|id| self.state(*id) != IndexState::Absent)
                .collect(),
            set: set.clone(),
            durable: self.durable.clone(),
            overlay: self.overlay.clone(),
            vfs: self.vfs.clone(),
            batch_time: cfg.batch_time,
            throttle: cfg.throttle,
        }
    }

    fn spawn_content(
        &mut self,
        host: &mut AnalysisHost,
        cfg: &IndexConfig,
        set: &IndexSet,
        items: Vec<WorkItem>,
    ) {
        let ctx = self.job_ctx(cfg, set);
        let sink = self.results.clone();
        let jobs = self.jobs.clone();
        jobs.fetch_add(1, Ordering::SeqCst);
        let guard = RunGuard(jobs);
        host.spawn_with(
            JobSpec::new(JobPriority::Background, Revoke::Never),
            move |snapshot, cancel| {
                let _guard = guard;
                let result = run_content_batch(&ctx, items, snapshot, cancel);
                sink.lock().push(Msg::Content(result));
                Ok(())
            },
            |_| {},
        );
    }

    fn spawn_names(&mut self, host: &mut AnalysisHost, set: &IndexSet, items: Vec<WorkItem>) {
        let set = set.clone();
        let durable = self.durable.clone();
        let vfs = self.vfs.clone();
        let sink = self.results.clone();
        let jobs = self.jobs.clone();
        jobs.fetch_add(1, Ordering::SeqCst);
        self.names_in_flight += 1;
        let guard = RunGuard(jobs);
        host.spawn_with(
            JobSpec::new(JobPriority::Background, Revoke::Never),
            move |_, cancel| {
                let _guard = guard;
                let mut touched: Vec<IndexId> = Vec::new();
                let store: &dyn archipel_ide_analysis::index::IndexStore = &*durable;
                for item in &items {
                    if cancel.is_cancelled() {
                        break;
                    }
                    let meta_disk = vfs.metadata(&item.path);
                    let meta = FileMeta {
                        id: item.file,
                        path: item.path.clone(),
                        len: meta_disk.map_or(0, |m| m.len),
                        mtime: meta_disk.and_then(|m| m.mtime),
                        content_hash: None,
                        language: language_of(&item.path).map(|l| l.language_id()),
                        is_open: false,
                    };
                    let input = IndexInput {
                        file: &meta,
                        text: "",
                        syntax: None,
                    };
                    let _ = set.run_names(store, &input);
                }
                for index in set.names() {
                    touched.push(index.id());
                }
                let stamps = touched
                    .into_iter()
                    .map(|id| (id, durable.commit(id)))
                    .collect();
                sink.lock().push(Msg::Names {
                    files: items.len(),
                    stamps,
                });
                Ok(())
            },
            |_| {},
        );
    }

    fn spawn_removal(&mut self, host: &mut AnalysisHost, set: &IndexSet, files: Vec<FileId>) {
        let set = set.clone();
        let durable = self.durable.clone();
        let overlay = self.overlay.clone();
        let sink = self.results.clone();
        let jobs = self.jobs.clone();
        jobs.fetch_add(1, Ordering::SeqCst);
        let guard = RunGuard(jobs);
        for file in &files {
            self.records.remove(file);
            self.save_upserts.remove(file);
            self.save_removed.push(*file);
        }
        host.spawn_with(
            JobSpec::new(JobPriority::Background, Revoke::Never),
            move |_, _| {
                let _guard = guard;
                for file in &files {
                    overlay.drop_file(*file);
                    for index in set.all() {
                        durable.remove_file(index.id(), *file);
                        durable.set_indexed_hash(index.id(), *file, None);
                    }
                }
                let stamps = set
                    .all()
                    .iter()
                    .map(|index| (index.id(), overlay.commit(index.id())))
                    .collect();
                sink.lock().push(Msg::Removed { stamps });
                Ok(())
            },
            |_| {},
        );
    }

    fn apply_message(
        &mut self,
        host: &mut AnalysisHost,
        cfg: &IndexConfig,
        set: &IndexSet,
        message: Msg,
    ) {
        match message {
            Msg::Scan {
                entries,
                dir,
                millis,
            } => {
                self.scans_running = self.scans_running.saturating_sub(1);
                if dir.is_none() {
                    self.scan_ms = Some(millis);
                }
                self.reconcile(host, cfg, set, entries, dir);
            }
            Msg::Names { files, stamps } => {
                self.names_in_flight = self.names_in_flight.saturating_sub(1);
                let _ = files;
                let names_done = self.names_queue.is_empty() && self.names_in_flight == 0;
                if names_done {
                    for index in set.names() {
                        if self.state(index.id()) != IndexState::Ready {
                            self.durable.set_complete(index.id(), true);
                            self.states.insert(index.id(), IndexState::Ready);
                        }
                    }
                }
                for (id, stamp) in stamps {
                    self.publish(host, id, stamp);
                }
            }
            Msg::Removed { stamps } => {
                for (id, stamp) in stamps {
                    self.publish(host, id, stamp);
                }
            }
            Msg::Content(result) => self.apply_content(host, set, result),
        }
    }

    fn reconcile(
        &mut self,
        host: &mut AnalysisHost,
        _cfg: &IndexConfig,
        set: &IndexSet,
        entries: Vec<ScanEntry>,
        dir: Option<PathBuf>,
    ) {
        let full = dir.is_none();
        let everything_complete = set
            .all()
            .iter()
            .all(|index| self.state(index.id()) == IndexState::Ready);
        let names_complete = set
            .names()
            .all(|index| self.state(index.id()) == IndexState::Ready);
        let mut seen: HashSet<FileId> = HashSet::with_capacity(entries.len());
        let mut created = Vec::new();
        let snapshot_before = host.snapshot();
        for entry in &entries {
            let known_before = snapshot_before.file_id(&entry.path);
            let id = host.file_id(&entry.path);
            if known_before.is_none() {
                created.push(id);
            }
            seen.insert(id);
        }
        let snapshot = host.snapshot();
        for entry in &entries {
            let Some(id) = snapshot.file_id(&entry.path) else {
                continue;
            };
            let mtime_ns = FileRecord::mtime(entry.mtime);
            let is_new = !self.records.contains_key(&id);
            let unchanged = self
                .records
                .get(&id)
                .is_some_and(|r| r.len == entry.len && r.mtime_ns == mtime_ns);
            if is_new {
                self.records.insert(
                    id,
                    FileRecord {
                        id,
                        path: entry.path.clone(),
                        len: entry.len,
                        mtime_ns,
                        hash: 0,
                    },
                );
            }
            if !names_complete || is_new {
                self.names_queue.push(WorkItem {
                    file: id,
                    path: entry.path.clone(),
                    known: None,
                    buffer: false,
                    text_pass: false,
                });
            }
            if !unchanged || !everything_complete {
                self.enqueue(id, tier::REST, &snapshot, set);
            }
        }
        // Files that vanished.
        let missing: Vec<FileId> = self
            .records
            .iter()
            .filter(|(id, record)| {
                !seen.contains(id)
                    && match &dir {
                        None => true,
                        Some(dir) => self.vfs.absolute(&record.path).starts_with(dir),
                    }
            })
            .map(|(id, _)| *id)
            .collect();
        if !missing.is_empty() {
            host.apply(Change::Vfs(VfsChange {
                deleted: missing.clone(),
                ..VfsChange::default()
            }));
            for id in missing {
                if !self.removal_queue.contains(&id) {
                    self.removal_queue.push(id);
                }
                if let Some((tier, seq)) = self.queued.remove(&id) {
                    self.queue.remove(&(tier, seq, id));
                }
            }
        }
        if full {
            self.scanned = true;
            // Indexes that are not complete are now Building.
            for index in set.all() {
                if !index.text_only() && self.state(index.id()) != IndexState::Ready {
                    self.states.insert(
                        index.id(),
                        IndexState::Building {
                            done: 0,
                            total: self.sweep_total,
                        },
                    );
                    self.publish(host, index.id(), IndexStamp(0));
                }
            }
            self.update_dumb(host, set);
        }
        if !created.is_empty() && !full {
            host.apply(Change::Vfs(VfsChange {
                created,
                ..VfsChange::default()
            }));
        }
    }

    fn apply_content(&mut self, host: &mut AnalysisHost, set: &IndexSet, result: ContentResult) {
        let ContentResult { items, stamps } = result;
        let snapshot = host.snapshot();
        let mut gone = Vec::new();
        for (item, outcome) in items {
            if item.text_pass {
                match outcome {
                    Outcome::Gone => gone.push(item.file),
                    Outcome::Deferred => self.text_queue.push(item),
                    _ => self.text_done += 1,
                }
                continue;
            }
            self.in_flight.remove(&item.file);
            self.buffer_in_flight.remove(&item.file);
            if !item.buffer && snapshot.path(item.file).is_none() {
                // Deleted while the batch ran: its entries must go again.
                if !self.removal_queue.contains(&item.file) {
                    self.removal_queue.push(item.file);
                }
                continue;
            }
            let record = |len: u64, mtime_ns: Option<u64>, hash: u64| FileRecord {
                id: item.file,
                path: item.path.clone(),
                len,
                mtime_ns,
                hash,
            };
            let outcome_kind = u8::from(!matches!(outcome, Outcome::Indexed { .. }));
            match outcome {
                Outcome::Indexed {
                    hash,
                    len,
                    mtime_ns,
                }
                | Outcome::Unchanged {
                    hash,
                    len,
                    mtime_ns,
                } => {
                    if matches!(outcome_kind, 0) {
                        self.stats.indexed += 1;
                    } else {
                        self.stats.unchanged += 1;
                    }
                    let record = record(len, mtime_ns, hash);
                    self.save_upserts.insert(item.file, record.clone());
                    self.records.insert(item.file, record);
                    self.sweep_done += 1;
                }
                Outcome::Skipped {
                    reason,
                    len,
                    mtime_ns,
                } => {
                    let record = record(len, mtime_ns, SKIPPED_HASH);
                    self.save_upserts.insert(item.file, record.clone());
                    self.records.insert(item.file, record);
                    self.skipped_total += 1;
                    if self.skipped.len() < MAX_LISTED_SKIPS {
                        self.skipped.push(SkippedFile {
                            path: item.path.clone(),
                            reason,
                        });
                    }
                    self.sweep_done += 1;
                }
                Outcome::Gone => {
                    gone.push(item.file);
                    self.sweep_done += 1;
                }
                Outcome::BufferIndexed => self.stats.buffers += 1,
                Outcome::BufferDropped => {}
                Outcome::Deferred => {
                    if item.buffer {
                        self.buffer_dirty.insert(item.file, Instant::now());
                    } else {
                        self.enqueue(item.file, tier::CHANGED, &snapshot, set);
                        self.sweep_total = self.sweep_total.saturating_sub(1);
                    }
                }
            }
            if self.requeue.remove(&item.file) {
                self.enqueue(item.file, tier::CHANGED, &snapshot, set);
            }
        }
        if !gone.is_empty() {
            host.apply(Change::Vfs(VfsChange {
                deleted: gone.clone(),
                ..VfsChange::default()
            }));
            self.removal_queue.extend(gone);
        }
        self.sweep_done = self.sweep_done.min(self.sweep_total.max(self.sweep_done));
        if self.sweep_total < self.sweep_done {
            self.sweep_total = self.sweep_done;
        }
        for (id, stamp) in stamps {
            if let IndexState::Building { .. } = self.state(id) {
                self.states.insert(
                    id,
                    IndexState::Building {
                        done: self.sweep_done,
                        total: self.sweep_total,
                    },
                );
            }
            self.publish(host, id, stamp);
        }
    }

    /// Everything queued is done: mark the indexes ready.
    fn finish_if_ready(&mut self, host: &mut AnalysisHost, set: &IndexSet) -> bool {
        let mut changed = false;
        for index in set.all() {
            if !index.text_only()
                && matches!(
                    self.state(index.id()),
                    IndexState::Building { .. } | IndexState::Absent
                )
            {
                self.durable.set_complete(index.id(), true);
                self.states.insert(index.id(), IndexState::Ready);
                let stamp = self.overlay.commit(index.id());
                self.publish(host, index.id(), stamp);
                changed = true;
            }
        }
        if changed {
            self.update_dumb(host, set);
        }
        // Second pass: text-only indexes (Trigram), after navigation is ready.
        let pending_text: Vec<IndexId> = set
            .text_content()
            .map(|index| index.id())
            .filter(|id| !matches!(self.state(*id), IndexState::Ready | IndexState::Failed))
            .collect();
        if !pending_text.is_empty() {
            if self.text_started {
                for id in pending_text {
                    self.durable.set_complete(id, true);
                    self.states.insert(id, IndexState::Ready);
                    let stamp = self.overlay.commit(id);
                    self.publish(host, id, stamp);
                }
                self.text_started = false;
                self.text_total = 0;
                self.text_done = 0;
            } else {
                self.text_started = true;
                self.text_queue = self
                    .records
                    .values()
                    .map(|record| WorkItem {
                        file: record.id,
                        path: record.path.clone(),
                        known: Some(record.clone()),
                        buffer: false,
                        text_pass: true,
                    })
                    .collect();
                self.text_queue.sort_by_key(|item| item.file);
                self.text_total = self.text_queue.len() as u32;
                self.text_done = 0;
                for id in pending_text {
                    self.states.insert(
                        id,
                        IndexState::Building {
                            done: 0,
                            total: self.text_total,
                        },
                    );
                    self.publish(host, id, IndexStamp(0));
                }
            }
            changed = true;
        }
        if !self.save_upserts.is_empty() || !self.save_removed.is_empty() {
            self.flush_records();
            changed = true;
        }
        if self.sweep_total != 0 || self.sweep_done != 0 {
            self.sweep_total = 0;
            self.sweep_done = 0;
            changed = true;
        }
        changed
    }

    fn flush_records(&mut self) {
        if self.save_upserts.is_empty() && self.save_removed.is_empty() {
            return;
        }
        let upserts: Vec<FileRecord> = self.save_upserts.drain().map(|(_, r)| r).collect();
        let removed = std::mem::take(&mut self.save_removed);
        self.durable.save_files(&upserts, &removed);
    }

    fn on_index_failed(&mut self, host: &mut AnalysisHost, set: &IndexSet, id: IndexId) {
        let retries = self.retries.entry(id).or_default();
        if *retries >= 1 {
            self.states.insert(id, IndexState::Failed);
            self.publish(host, id, IndexStamp(0));
            self.update_dumb(host, set);
            return;
        }
        *retries += 1;
        if let Some(index) = set.get(id) {
            self.reset_index(index.as_ref());
            self.publish(host, id, IndexStamp(0));
            let snapshot = host.snapshot();
            self.queue_everything(&snapshot, set, index.as_ref());
            self.update_dumb(host, set);
        }
    }
}

// ---- job bodies -------------------------------------------------------------------

struct JobCtx {
    /// Text-only indexes that already build (updated with every edit).
    text_active: Vec<IndexId>,
    set: IndexSet,
    durable: Arc<dyn DurableStore>,
    overlay: Arc<OverlayStore>,
    vfs: Arc<RootedVfs>,
    batch_time: Duration,
    throttle: Duration,
}

impl JobCtx {
    /// The content indexes a batch feeds: in the main pass everything but
    /// the text-only indexes that have not started; in the text pass only
    /// the text-only ones.
    fn content_for(&self, text_pass: bool) -> Vec<&Arc<dyn ErasedIndex>> {
        if text_pass {
            self.set.text_content().collect()
        } else {
            self.set
                .content()
                .filter(|i| !i.text_only() || self.text_active.contains(&i.id()))
                .collect()
        }
    }
}

fn run_content_batch(
    ctx: &JobCtx,
    items: Vec<WorkItem>,
    snapshot: &Snapshot,
    cancel: &CancelToken,
) -> ContentResult {
    let started = Instant::now();
    let mut done = Vec::with_capacity(items.len());
    let mut touched: HashSet<IndexId> = HashSet::new();
    for (position, item) in items.into_iter().enumerate() {
        let out_of_time = position > 0 && started.elapsed() >= ctx.batch_time;
        let outcome = if cancel.is_cancelled() || out_of_time {
            Outcome::Deferred
        } else {
            let attempt = std::panic::catch_unwind(AssertUnwindSafe(|| {
                if item.buffer {
                    process_buffer(ctx, &item, snapshot, cancel, &mut touched)
                } else {
                    process_disk(ctx, &item, cancel, &mut touched)
                }
            }));
            attempt.unwrap_or_else(|_| Outcome::Skipped {
                reason: "internal indexing error".into(),
                len: 0,
                mtime_ns: None,
            })
        };
        done.push((item, outcome));
    }
    let stamps = touched
        .into_iter()
        .map(|id| (id, ctx.overlay.commit(id)))
        .collect();
    if !ctx.throttle.is_zero() {
        std::thread::sleep(ctx.throttle);
    }
    ContentResult {
        items: done,
        stamps,
    }
}

/// The hash recorded for indexed content: the text plus the language the
/// file is indexed as (a renamed file with the same text but another
/// extension must be reindexed).
fn indexed_hash_of(path: &Path, text: &str) -> u64 {
    let lang = registry::detect(path, text).map_or("", |lang| lang.id);
    content_hash(text.as_bytes()) ^ crate::hash::word_hash(lang).rotate_left(17)
}

fn file_meta(
    item: &WorkItem,
    text: &str,
    mtime: Option<std::time::SystemTime>,
    hash: u64,
    open: bool,
) -> (
    FileMeta,
    Option<&'static archipel_ide_language::LanguageDef>,
) {
    let lang = registry::detect(&item.path, text);
    (
        FileMeta {
            id: item.file,
            path: item.path.clone(),
            len: text.len() as u64,
            mtime,
            content_hash: Some(hash),
            language: lang.map(|l| l.language_id()),
            is_open: open,
        },
        lang,
    )
}

fn process_disk(
    ctx: &JobCtx,
    item: &WorkItem,
    cancel: &CancelToken,
    touched: &mut HashSet<IndexId>,
) -> Outcome {
    let Some(disk) = ctx.vfs.metadata(&item.path) else {
        return Outcome::Gone;
    };
    let mtime_ns = FileRecord::mtime(disk.mtime);
    let content = ctx.content_for(item.text_pass);
    if let Some(known) = &item.known
        && known.hash != 0
        && known.len == disk.len
        && known.mtime_ns == mtime_ns
        && (known.hash == SKIPPED_HASH
            || content
                .iter()
                .all(|i| ctx.durable.indexed_hash(i.id(), item.file) == Some(known.hash)))
    {
        return Outcome::Unchanged {
            hash: known.hash,
            len: disk.len,
            mtime_ns,
        };
    }
    let text = match ctx.vfs.read(&item.path) {
        Ok(text) => text,
        Err(VfsError::NotFound(_)) => return Outcome::Gone,
        Err(error) => {
            for index in &content {
                ctx.durable.remove_file(index.id(), item.file);
                ctx.durable.set_indexed_hash(index.id(), item.file, None);
                touched.insert(index.id());
            }
            let reason = match error {
                VfsError::Binary(_) => "binary file".to_string(),
                VfsError::TooLarge { len, .. } => format!("too large ({len} bytes)"),
                other => other.to_string(),
            };
            return Outcome::Skipped {
                reason,
                len: disk.len,
                mtime_ns,
            };
        }
    };
    let hash = indexed_hash_of(&item.path, &text);
    let stale: Vec<IndexId> = content
        .iter()
        .filter(|i| ctx.durable.indexed_hash(i.id(), item.file) != Some(hash))
        .map(|i| i.id())
        .collect();
    if stale.is_empty() {
        return Outcome::Unchanged {
            hash,
            len: disk.len,
            mtime_ns,
        };
    }
    let (meta, lang) = file_meta(item, &text, disk.mtime, hash, false);
    let needs_syntax = content
        .iter()
        .any(|i| stale.contains(&i.id()) && !i.text_only());
    let syntax = lang
        .filter(|_| needs_syntax && meta.len <= MAX_SYNTAX_BYTES)
        .and_then(|lang| SyntaxSnapshot::parse(item.file, FileRev(0), lang, text.clone()));
    let input = IndexInput {
        file: &meta,
        text: &text,
        syntax: syntax.as_ref(),
    };
    let store: &dyn archipel_ide_analysis::index::IndexStore = &*ctx.durable;
    if ctx
        .set
        .run_content(store, &input, Some(&stale), cancel)
        .is_err()
    {
        return Outcome::Deferred;
    }
    for id in stale {
        ctx.durable.set_indexed_hash(id, item.file, Some(hash));
        touched.insert(id);
    }
    Outcome::Indexed {
        hash,
        len: disk.len,
        mtime_ns,
    }
}

fn process_buffer(
    ctx: &JobCtx,
    item: &WorkItem,
    snapshot: &Snapshot,
    cancel: &CancelToken,
    touched: &mut HashSet<IndexId>,
) -> Outcome {
    let Some(entry) = snapshot.overlay().get(item.file) else {
        return Outcome::BufferDropped;
    };
    let text = entry.text.clone();
    let hash = indexed_hash_of(&item.path, &text);
    let content = ctx.content_for(false);
    let equal_to_disk = content
        .iter()
        .all(|i| ctx.durable.indexed_hash(i.id(), item.file) == Some(hash));
    if equal_to_disk {
        if ctx.overlay.has_overlay(item.file) {
            ctx.overlay.drop_file(item.file);
            for index in &content {
                touched.insert(index.id());
            }
        }
        return Outcome::BufferDropped;
    }
    let (meta, lang) = file_meta(item, &text, None, hash, true);
    let syntax = if meta.len > MAX_SYNTAX_BYTES {
        None
    } else {
        snapshot.syntax(item.file).or_else(|| {
            lang.and_then(|l| SyntaxSnapshot::parse(item.file, FileRev(0), l, text.clone()))
        })
    };
    let input = IndexInput {
        file: &meta,
        text: &text,
        syntax: syntax.as_ref(),
    };
    let store: &dyn archipel_ide_analysis::index::IndexStore = &*ctx.overlay;
    let ids: Vec<IndexId> = content.iter().map(|i| i.id()).collect();
    if ctx
        .set
        .run_content(store, &input, Some(&ids), cancel)
        .is_err()
    {
        return Outcome::Deferred;
    }
    for index in &content {
        touched.insert(index.id());
    }
    Outcome::BufferIndexed
}

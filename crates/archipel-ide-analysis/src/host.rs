//! The analysis world: [`AnalysisHost`] (owned by the UI thread, the only
//! mutator) and immutable [`Snapshot`]s for background work (D4: readers
//! never block writers).
//!
//! `apply(Change)` bumps stamps, records the edit journal, publishes
//! [`AnalysisEvent`]s and revokes running jobs registered for the change.
//! Taking a snapshot is O(1) (one `Arc` clone); holding one never blocks
//! `apply` (copy on write).

use crate::cache::{CacheStore, DEFAULT_CACHE_BYTES, Db, resident_bytes};
use crate::cancel::{CancelToken, Cancelled};
use crate::file::{FileId, FileRev, FileTable, LanguageId};
use crate::index::{IndexState, IndexStore};
use crate::interface::{InterfaceSummarizer, StructureInterface};
use crate::job::{Executor, JobHandle, JobSpec, Revoke, RevokeScope, default_threads};
use crate::stamps::{IndexId, IndexStamp, InterfaceEpoch, Stamps};
use crate::vfs::{DocumentOverlay, Vfs, VfsChange, VfsError};
use archipel_ide_language::{
    ByteEdit, EditMap, LanguageDef, Registration, ServiceRegistry, SyntaxSnapshot, SyntaxTree,
    registry,
};
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, Weak};
use std::time::{Duration, Instant};

/// Host tuning.
#[derive(Debug, Clone)]
pub struct HostConfig {
    /// Byte budget of the cached-value LRU.
    pub cache_bytes: usize,
    /// Syntax trees of closed files kept (LRU).
    pub closed_trees: usize,
    /// Worker threads; 0 = no executor (everything synchronous, for tests).
    pub threads: usize,
    /// Edit-journal entries kept per open file (for anchors and range
    /// mapping of late results).
    pub journal_depth: usize,
    /// Quiet time after the last edit of a file before its interface hash is
    /// recomputed (0 = on the next `poll`).
    pub interface_debounce: Duration,
    /// Resident-memory limit above which `poll` clears the value cache.
    pub memory_limit: Option<u64>,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            cache_bytes: DEFAULT_CACHE_BYTES,
            closed_trees: 64,
            threads: default_threads(),
            journal_depth: 512,
            interface_debounce: Duration::from_millis(300),
            memory_limit: None,
        }
    }
}

impl HostConfig {
    /// Synchronous, deterministic settings for tests.
    pub fn testing() -> Self {
        Self {
            threads: 0,
            interface_debounce: Duration::ZERO,
            ..Self::default()
        }
    }
}

/// A syntax tree computed by the caller (the editor already reparses
/// incrementally), so the host does not parse the same text twice.
#[derive(Debug, Clone)]
pub struct SyntaxUpdate {
    pub snapshot: SyntaxSnapshot,
    /// Changed byte ranges in the new text (see `Reparse`).
    pub changed: Vec<Range<usize>>,
}

/// The only way to mutate the analysis world.
#[derive(Debug, Clone)]
pub enum Change {
    /// A buffer was opened (its text shadows the disk from now on).
    Open {
        file: FileId,
        rev: FileRev,
        text: Arc<str>,
    },
    /// Buffer edits, as journaled by the text engine.
    Edit {
        file: FileId,
        edits: Vec<ByteEdit>,
        new_rev: FileRev,
        text: Arc<str>,
        syntax: Option<SyntaxUpdate>,
    },
    /// The buffer was replaced wholesale (reload, `EditJournal::Reset`).
    Reset {
        file: FileId,
        new_rev: FileRev,
        text: Arc<str>,
        syntax: Option<SyntaxSnapshot>,
    },
    Close {
        file: FileId,
    },
    /// A coalesced watcher batch (see [`VfsChange::classify`]).
    Vfs(VfsChange),
    SettingsChanged,
    IndexCommit {
        index: IndexId,
        stamp: IndexStamp,
        state: IndexState,
    },
    DumbMode(bool),
    /// A file's interface hash for `rev` (from the indexer or a host job).
    InterfaceComputed {
        file: FileId,
        rev: FileRev,
        hash: u64,
    },
}

/// What changed; drained by subscribers (daemon, indexes, code vision).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisEvent {
    /// The PSI tree-change analogue: syntax changed in `ranges` (new text).
    SyntaxChanged {
        file: FileId,
        rev: FileRev,
        ranges: Vec<Range<u32>>,
    },
    InterfaceChanged {
        file: FileId,
        epoch: InterfaceEpoch,
    },
    FileOpened(FileId),
    FileClosed(FileId),
    FilesChanged(VfsChange),
    IndexUpdated(IndexId),
    SettingsChanged,
    DumbModeChanged(bool),
}

/// Edit journal, newest first; persistent (cheap to share with snapshots).
#[derive(Debug)]
struct JournalNode {
    from: FileRev,
    to: FileRev,
    map: EditMap,
    depth: usize,
    prev: Option<Arc<JournalNode>>,
}

#[derive(Debug, Clone)]
struct OpenFile {
    rev: FileRev,
    text: Arc<str>,
    syntax: Option<SyntaxSnapshot>,
    journal: Option<Arc<JournalNode>>,
}

struct TreeLru {
    capacity: usize,
    entries: VecDeque<((FileId, FileRev), Option<SyntaxSnapshot>)>,
}

impl TreeLru {
    fn get(&mut self, key: (FileId, FileRev)) -> Option<Option<SyntaxSnapshot>> {
        let index = self.entries.iter().position(|(k, _)| *k == key)?;
        let entry = self.entries.remove(index)?;
        let value = entry.1.clone();
        self.entries.push_back(entry);
        Some(value)
    }

    fn put(&mut self, key: (FileId, FileRev), value: Option<SyntaxSnapshot>) {
        self.entries.retain(|(k, _)| *k != key);
        self.entries.push_back((key, value));
        while self.entries.len() > self.capacity {
            self.entries.pop_front();
        }
    }
}

#[derive(Clone)]
struct WorldView {
    generation: u64,
    files: Arc<FileTable>,
    overlay: DocumentOverlay,
    open: HashMap<FileId, OpenFile>,
    stamps: Stamps,
    dumb: bool,
    index_states: Arc<HashMap<IndexId, IndexState>>,
    index_store: Option<Arc<dyn IndexStore>>,
    registry: Arc<ServiceRegistry>,
    cache: Arc<CacheStore>,
    vfs: Arc<dyn Vfs>,
    closed_trees: Arc<Mutex<TreeLru>>,
}

/// An immutable view of the analysis world. Cheap to clone; `Send + Sync`.
#[derive(Clone)]
pub struct Snapshot {
    view: Arc<WorldView>,
}

impl std::fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Snapshot")
            .field("generation", &self.view.generation)
            .field("open", &self.view.open.len())
            .finish()
    }
}

impl Snapshot {
    /// Bumped by every applied change.
    pub fn generation(&self) -> u64 {
        self.view.generation
    }

    pub fn files(&self) -> &FileTable {
        &self.view.files
    }

    pub fn path(&self, file: FileId) -> Option<&Path> {
        self.view.files.path(file)
    }

    pub fn file_id(&self, path: &Path) -> Option<FileId> {
        self.view.files.lookup(path)
    }

    pub fn is_open(&self, file: FileId) -> bool {
        self.view.open.contains_key(&file)
    }

    pub fn open_files(&self) -> impl Iterator<Item = FileId> + '_ {
        self.view.open.keys().copied()
    }

    pub fn overlay(&self) -> &DocumentOverlay {
        &self.view.overlay
    }

    pub fn file_rev(&self, file: FileId) -> Option<FileRev> {
        self.view.stamps.file_rev(file)
    }

    pub fn stamps(&self) -> &Stamps {
        &self.view.stamps
    }

    pub fn registry(&self) -> &ServiceRegistry {
        &self.view.registry
    }

    pub fn registry_arc(&self) -> Arc<ServiceRegistry> {
        self.view.registry.clone()
    }

    /// Cached values validated against this snapshot's stamps.
    pub fn db(&self) -> Db {
        Db::new(self.view.stamps.clone(), self.view.cache.clone())
    }

    pub fn is_dumb(&self) -> bool {
        self.view.dumb
    }

    pub fn index_state(&self, index: IndexId) -> IndexState {
        self.view
            .index_states
            .get(&index)
            .copied()
            .unwrap_or(IndexState::Absent)
    }

    pub fn index_store(&self) -> Option<&Arc<dyn IndexStore>> {
        self.view.index_store.as_ref()
    }

    /// Text of `file`: the open buffer, else the disk contents.
    pub fn file_text(&self, file: FileId) -> Result<Arc<str>, VfsError> {
        if let Some(entry) = self.view.overlay.get(file) {
            return Ok(entry.text.clone());
        }
        let path = self.path(file).ok_or(VfsError::UnknownFile(file))?;
        self.view.vfs.read(path)
    }

    /// Language of `file` (by name, extension or shebang).
    pub fn language(&self, file: FileId) -> Option<&'static LanguageDef> {
        if let Some(syntax) = self.view.open.get(&file).and_then(|f| f.syntax.as_ref()) {
            return Some(syntax.lang());
        }
        let path = self.path(file)?;
        let head = self
            .view
            .overlay
            .get(file)
            .map(|entry| entry.text.clone())
            .unwrap_or_else(|| "".into());
        registry::detect(path, &head).or_else(|| {
            // Shebang detection needs the first line of closed files.
            path.extension()
                .is_none()
                .then(|| self.view.vfs.read(path).ok())
                .flatten()
                .and_then(|text| registry::detect(path, &text))
        })
    }

    pub fn language_id(&self, file: FileId) -> Option<LanguageId> {
        self.language(file).map(LanguageDef::language_id)
    }

    /// Syntax of `file` at this snapshot's revision: the open buffer's tree,
    /// or a parse of the disk contents (LRU-cached).
    pub fn syntax(&self, file: FileId) -> Option<SyntaxSnapshot> {
        if let Some(open) = self.view.open.get(&file) {
            if let Some(syntax) = &open.syntax {
                return Some(syntax.clone());
            }
            let lang = registry::detect(self.path(file)?, &open.text)?;
            return SyntaxSnapshot::parse(file, open.rev, lang, open.text.clone());
        }
        let rev = self.file_rev(file).unwrap_or_default();
        if let Some(cached) = self.view.closed_trees.lock().get((file, rev)) {
            return cached;
        }
        let parsed = (|| {
            let text = self.file_text(file).ok()?;
            let lang = registry::detect(self.path(file)?, &text)?;
            SyntaxSnapshot::parse(file, rev, lang, text)
        })();
        self.view
            .closed_trees
            .lock()
            .put((file, rev), parsed.clone());
        parsed
    }

    /// The composed edits of `file` from revision `from` to this snapshot's
    /// revision (`None` when the journal no longer reaches `from`).
    pub fn edits_since(&self, file: FileId, from: FileRev) -> Option<EditMap> {
        let open = self.view.open.get(&file)?;
        if open.rev == from {
            return Some(EditMap::new());
        }
        let mut maps = Vec::new();
        let mut node = open.journal.as_deref();
        while let Some(current) = node {
            maps.push(&current.map);
            if current.from == from {
                return Some(
                    maps.into_iter()
                        .rev()
                        .fold(EditMap::new(), |acc, map| acc.compose(map)),
                );
            }
            if current.from < from {
                return None;
            }
            node = current.prev.as_deref();
        }
        None
    }
}

/// Owned by the UI thread. See the module docs.
pub struct AnalysisHost {
    view: Arc<WorldView>,
    config: HostConfig,
    events: Vec<AnalysisEvent>,
    trees: HashMap<FileId, SyntaxTree>,
    interface_hashes: HashMap<FileId, u64>,
    /// Files whose interface must be recomputed, with their last edit time.
    interface_pending: HashMap<FileId, Instant>,
    interface_results: Arc<Mutex<Vec<(FileId, FileRev, u64)>>>,
    executor: OnceLock<Executor>,
    revokers: Vec<(Revoke, CancelToken, Weak<()>)>,
    coalesced: HashMap<(&'static str, u64), CancelToken>,
    wake: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for AnalysisHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalysisHost")
            .field("generation", &self.view.generation)
            .field("open", &self.view.open.len())
            .field("files", &self.view.files.len())
            .finish()
    }
}

/// A registry with every language's base declared and its `services` hook
/// run, plus the generic "any language" services.
pub fn default_registry() -> ServiceRegistry {
    let mut registry = ServiceRegistry::new();
    for lang in registry::LANGUAGES {
        if let Some(base) = lang.base_id() {
            registry.declare_base(lang.language_id(), base);
        }
    }
    registry.register::<dyn InterfaceSummarizer>(
        Registration::any("structure-interface").dumb_aware(true),
        Arc::new(StructureInterface),
    );
    crate::editing::register_generic_services(&mut registry);
    for lang in registry::LANGUAGES {
        (lang.services)(&mut registry);
    }
    registry
}

impl AnalysisHost {
    pub fn new(config: HostConfig, vfs: Arc<dyn Vfs>, registry: ServiceRegistry) -> Self {
        let view = WorldView {
            generation: 0,
            files: Arc::new(FileTable::new()),
            overlay: DocumentOverlay::default(),
            open: HashMap::new(),
            stamps: Stamps::new(),
            dumb: false,
            index_states: Arc::default(),
            index_store: None,
            registry: Arc::new(registry),
            cache: Arc::new(CacheStore::new(config.cache_bytes)),
            vfs,
            closed_trees: Arc::new(Mutex::new(TreeLru {
                capacity: config.closed_trees,
                entries: VecDeque::new(),
            })),
        };
        Self {
            view: Arc::new(view),
            config,
            events: Vec::new(),
            trees: HashMap::new(),
            interface_hashes: HashMap::new(),
            interface_pending: HashMap::new(),
            interface_results: Arc::default(),
            executor: OnceLock::new(),
            revokers: Vec::new(),
            coalesced: HashMap::new(),
            wake: None,
        }
    }

    /// The real file system and [`default_registry`].
    pub fn with_defaults(config: HostConfig) -> Self {
        Self::new(config, Arc::new(crate::vfs::DiskVfs), default_registry())
    }

    pub fn config(&self) -> &HostConfig {
        &self.config
    }

    /// O(1).
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            view: self.view.clone(),
        }
    }

    pub fn registry(&self) -> &ServiceRegistry {
        &self.view.registry
    }

    /// Replaces the service registry (packages register at startup).
    pub fn set_registry(&mut self, registry: ServiceRegistry) {
        self.view_mut().registry = Arc::new(registry);
    }

    /// Installs a persisted file table (stable ids across restarts, so index
    /// postings stay valid). Only possible while no file was interned yet;
    /// returns false otherwise.
    pub fn restore_files(&mut self, table: FileTable) -> bool {
        if !self.view.files.is_empty() {
            return false;
        }
        let view = self.view_mut();
        for (id, _) in table.iter() {
            view.stamps.set_file_rev(id, FileRev(0));
        }
        view.files = Arc::new(table);
        true
    }

    pub fn set_index_store(&mut self, store: Arc<dyn IndexStore>) {
        self.view_mut().index_store = Some(store);
    }

    pub fn cache(&self) -> &Arc<CacheStore> {
        &self.view.cache
    }

    /// Called when background results arrive (request a repaint).
    pub fn set_wake(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
        if let Some(executor) = self.executor.get() {
            let wake = wake.clone();
            executor.set_wake(move || wake());
        }
        self.wake = Some(wake);
    }

    /// The id of `path` (interned on first use; ids survive renames).
    pub fn file_id(&mut self, path: &Path) -> FileId {
        if let Some(id) = self.view.files.lookup(path) {
            return id;
        }
        let view = self.view_mut();
        let id = Arc::make_mut(&mut view.files).intern(path);
        if view.stamps.file_rev(id).is_none() {
            view.stamps.set_file_rev(id, FileRev(0));
        }
        id
    }

    pub fn lookup(&self, path: &Path) -> Option<FileId> {
        self.view.files.lookup(path)
    }

    fn view_mut(&mut self) -> &mut WorldView {
        Arc::make_mut(&mut self.view)
    }

    /// Drains the published events.
    pub fn take_events(&mut self) -> Vec<AnalysisEvent> {
        std::mem::take(&mut self.events)
    }

    /// Current syntax of an open file (the tree typed handlers need
    /// synchronously; IntelliJ commits the document first).
    pub fn sync_syntax(&mut self, file: FileId) -> Option<SyntaxSnapshot> {
        let open = self.view.open.get(&file)?;
        if let Some(syntax) = &open.syntax {
            return Some(syntax.clone());
        }
        let (rev, text) = (open.rev, open.text.clone());
        let lang = registry::detect(self.view.files.path(file)?, &text)?;
        let tree = SyntaxTree::parse(lang, &text)?;
        let snapshot = SyntaxSnapshot::from_tree(file, rev, &tree, text);
        self.trees.insert(file, tree);
        if let Some(open) = self.view_mut().open.get_mut(&file) {
            open.syntax = Some(snapshot.clone());
        }
        Some(snapshot)
    }

    /// Applies one change. See the module docs.
    pub fn apply(&mut self, change: Change) {
        let mut touched: Vec<FileId> = Vec::new();
        let mut settings = false;
        match change {
            Change::Open { file, rev, text } => {
                self.open(file, rev, text, None);
                self.events.push(AnalysisEvent::FileOpened(file));
                touched.push(file);
            }
            Change::Edit {
                file,
                edits,
                new_rev,
                text,
                syntax,
            } => {
                self.edit(file, &edits, new_rev, text, syntax);
                touched.push(file);
            }
            Change::Reset {
                file,
                new_rev,
                text,
                syntax,
            } => {
                let len = text.len() as u32;
                self.trees.remove(&file);
                self.open(file, new_rev, text, syntax);
                self.events.push(AnalysisEvent::SyntaxChanged {
                    file,
                    rev: new_rev,
                    ranges: std::iter::once(0..len).collect(),
                });
                touched.push(file);
            }
            Change::Close { file } => {
                let view = self.view_mut();
                view.open.remove(&file);
                view.overlay.close(file);
                // The disk contents may differ from the closed buffer.
                let rev = view.stamps.file_rev(file).unwrap_or_default();
                view.stamps.set_file_rev(file, FileRev(rev.0 + 1));
                self.trees.remove(&file);
                self.interface_pending.insert(file, Instant::now());
                self.events.push(AnalysisEvent::FileClosed(file));
                touched.push(file);
            }
            Change::Vfs(batch) => {
                self.apply_vfs(&batch);
                touched.extend(batch.files());
                self.events.push(AnalysisEvent::FilesChanged(batch));
            }
            Change::SettingsChanged => {
                self.view_mut().stamps.bump_settings();
                self.events.push(AnalysisEvent::SettingsChanged);
                settings = true;
            }
            Change::IndexCommit {
                index,
                stamp,
                state,
            } => {
                let view = self.view_mut();
                view.stamps.set_index(index, stamp);
                Arc::make_mut(&mut view.index_states).insert(index, state);
                self.events.push(AnalysisEvent::IndexUpdated(index));
            }
            Change::DumbMode(dumb) => {
                let view = self.view_mut();
                if view.dumb != dumb {
                    view.dumb = dumb;
                    view.stamps.bump_dumb();
                    self.events.push(AnalysisEvent::DumbModeChanged(dumb));
                }
            }
            Change::InterfaceComputed { file, rev, hash } => {
                if self.view.stamps.file_rev(file) == Some(rev) {
                    self.record_interface(file, hash);
                }
            }
        }
        self.view_mut().generation += 1;
        self.revoke(&touched, settings);
    }

    fn open(&mut self, file: FileId, rev: FileRev, text: Arc<str>, syntax: Option<SyntaxSnapshot>) {
        let syntax = syntax.or_else(|| {
            let lang = registry::detect(self.view.files.path(file)?, &text)?;
            let tree = SyntaxTree::parse(lang, &text)?;
            let snapshot = SyntaxSnapshot::from_tree(file, rev, &tree, text.clone());
            self.trees.insert(file, tree);
            Some(snapshot)
        });
        let view = self.view_mut();
        view.overlay.set(file, rev, text.clone());
        view.open.insert(
            file,
            OpenFile {
                rev,
                text,
                syntax,
                journal: None,
            },
        );
        view.stamps.set_file_rev(file, rev);
        self.interface_pending.insert(file, Instant::now());
    }

    fn edit(
        &mut self,
        file: FileId,
        edits: &[ByteEdit],
        rev: FileRev,
        text: Arc<str>,
        syntax: Option<SyntaxUpdate>,
    ) {
        let Some(previous) = self.view.open.get(&file).cloned() else {
            // Edits to a buffer the host never saw: treat as an open.
            self.open(file, rev, text, syntax.map(|update| update.snapshot));
            return;
        };
        let (snapshot, mut ranges) = match syntax {
            Some(update) => (Some(update.snapshot), update.changed),
            None => match self.trees.get_mut(&file) {
                Some(tree) => match tree.reparse(&text, edits) {
                    Some(reparse) => (
                        Some(SyntaxSnapshot::from_tree(file, rev, tree, text.clone())),
                        reparse.changed.into_iter().chain(reparse.edited).collect(),
                    ),
                    None => (None, std::iter::once(0..text.len()).collect()),
                },
                None => (None, Vec::new()),
            },
        };
        let map = EditMap::from_edits(edits);
        if ranges.is_empty() {
            ranges = edits
                .iter()
                .map(|edit| map.map(edit.start_byte)..map.map(edit.new_end_byte))
                .collect();
        }
        let depth = previous.journal.as_ref().map_or(0, |node| node.depth) + 1;
        let journal = Arc::new(JournalNode {
            from: previous.rev,
            to: rev,
            map,
            depth,
            prev: previous.journal.clone(),
        });
        let journal = if depth > self.config.journal_depth.max(1) * 2 {
            truncate_journal(&journal, self.config.journal_depth.max(1))
        } else {
            journal
        };
        let view = self.view_mut();
        view.overlay.set(file, rev, text.clone());
        view.open.insert(
            file,
            OpenFile {
                rev,
                text: text.clone(),
                syntax: snapshot,
                journal: Some(journal),
            },
        );
        view.stamps.set_file_rev(file, rev);
        self.interface_pending.insert(file, Instant::now());
        let len = text.len();
        let mut ranges: Vec<Range<u32>> = ranges
            .into_iter()
            .map(|range| range.start.min(len) as u32..range.end.min(len) as u32)
            .collect();
        ranges.sort_by_key(|range| (range.start, range.end));
        ranges.dedup();
        self.events
            .push(AnalysisEvent::SyntaxChanged { file, rev, ranges });
    }

    fn apply_vfs(&mut self, batch: &VfsChange) {
        let view = Arc::make_mut(&mut self.view);
        for (file, _, to) in &batch.moved {
            Arc::make_mut(&mut view.files).rename(*file, to);
        }
        for file in batch.created.iter().chain(&batch.modified) {
            if view.open.contains_key(file) {
                // The buffer shadows the disk; conflicts are the editor's.
                continue;
            }
            let rev = view.stamps.file_rev(*file).map_or(0, |rev| rev.0 + 1);
            view.stamps.set_file_rev(*file, FileRev(rev));
            let language = view
                .files
                .path(*file)
                .and_then(|path| registry::detect(path, ""))
                .map(LanguageDef::language_id);
            let epoch = view.stamps.bump_interface(*file, language);
            self.events
                .push(AnalysisEvent::InterfaceChanged { file: *file, epoch });
            self.interface_hashes.remove(file);
        }
        let view = Arc::make_mut(&mut self.view);
        for file in &batch.deleted {
            view.stamps.delete_file(*file);
            Arc::make_mut(&mut view.files).remove(*file);
            view.open.remove(file);
            view.overlay.close(*file);
            self.trees.remove(file);
            self.interface_hashes.remove(file);
            self.interface_pending.remove(file);
        }
    }

    fn record_interface(&mut self, file: FileId, hash: u64) {
        let changed = self.interface_hashes.insert(file, hash) != Some(hash);
        if changed {
            let language = self
                .view
                .open
                .get(&file)
                .and_then(|open| open.syntax.as_ref())
                .map(|syntax| syntax.lang().language_id());
            let epoch = self.view_mut().stamps.bump_interface(file, language);
            self.events
                .push(AnalysisEvent::InterfaceChanged { file, epoch });
        }
    }

    /// Computes `file`'s interface hash now (early cutoff: the epoch bumps only
    /// when the hash changed). Returns the current epoch.
    pub fn sync_interface(&mut self, file: FileId) -> Option<InterfaceEpoch> {
        self.interface_pending.remove(&file);
        let snapshot = self.snapshot();
        let syntax = snapshot.syntax(file);
        let hash = match &syntax {
            Some(syntax) => interface_hash(&snapshot, syntax),
            None => {
                // Without a grammar every revision is a new interface.
                let rev = snapshot.file_rev(file)?;
                u64::MAX ^ rev.0
            }
        };
        self.record_interface(file, hash);
        self.view.stamps.interface_epoch(file)
    }

    /// Per-frame housekeeping on the UI thread: runs job completions, applies
    /// background interface results, starts debounced interface jobs and
    /// enforces the memory limit. Returns true when anything changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        if let Some(executor) = self.executor.get() {
            changed |= executor.run_completions() > 0;
        }
        let results = std::mem::take(&mut *self.interface_results.lock());
        for (file, rev, hash) in results {
            self.apply(Change::InterfaceComputed { file, rev, hash });
            changed = true;
        }
        let now = Instant::now();
        let due: Vec<FileId> = self
            .interface_pending
            .iter()
            .filter(|(_, at)| now.duration_since(**at) >= self.config.interface_debounce)
            .map(|(file, _)| *file)
            .collect();
        for file in due {
            self.interface_pending.remove(&file);
            if self.config.threads == 0 {
                self.sync_interface(file);
                changed = true;
                continue;
            }
            let Some(rev) = self.view.stamps.file_rev(file) else {
                continue;
            };
            let results = self.interface_results.clone();
            let _ = self.spawn(
                JobSpec::new(
                    crate::job::Priority::Daemon,
                    Revoke::OnAnyChangeOf(RevokeScope::Files(vec![file])),
                )
                .coalesce("interface", u64::from(file.0)),
                move |snap, cancel| {
                    let syntax = snap.syntax(file).ok_or(Cancelled)?;
                    cancel.check()?;
                    let hash = interface_hash(snap, &syntax);
                    results.lock().push((file, rev, hash));
                    Ok(())
                },
            );
        }
        if let Some(limit) = self.config.memory_limit
            && resident_bytes().is_some_and(|rss| rss > limit)
        {
            self.view.cache.clear();
        }
        changed
    }

    fn executor(&self) -> &Executor {
        self.executor.get_or_init(|| {
            let executor = Executor::new(self.config.threads.max(1));
            if let Some(wake) = self.wake.clone() {
                executor.set_wake(move || wake());
            }
            executor
        })
    }

    /// Runs `task` on a snapshot of the current world in the background.
    /// Revoked per `spec.revoke`; a job with the same coalescing key cancels
    /// the previous one. With `threads == 0` the task runs inline.
    pub fn spawn<T: Send + 'static>(
        &mut self,
        spec: JobSpec,
        task: impl FnOnce(&Snapshot, &CancelToken) -> Result<T, Cancelled> + Send + 'static,
    ) -> JobHandle<T> {
        let cancel = self.register_job(&spec);
        let alive = Arc::new(());
        self.revokers
            .push((spec.revoke.clone(), cancel.clone(), Arc::downgrade(&alive)));
        let snapshot = self.snapshot();
        if self.config.threads == 0 {
            let executor = InlineResult::run(cancel.clone(), move |token| {
                let _alive = alive;
                task(&snapshot, token)
            });
            return executor;
        }
        self.executor().spawn(spec.priority, cancel, move |token| {
            let _alive = alive;
            task(&snapshot, token)
        })
    }

    /// Like [`AnalysisHost::spawn`]; `on_done` runs in [`AnalysisHost::poll`].
    pub fn spawn_with<T: Send + 'static>(
        &mut self,
        spec: JobSpec,
        task: impl FnOnce(&Snapshot, &CancelToken) -> Result<T, Cancelled> + Send + 'static,
        on_done: impl FnOnce(Result<T, Cancelled>) + Send + 'static,
    ) -> CancelToken {
        let cancel = self.register_job(&spec);
        let alive = Arc::new(());
        self.revokers
            .push((spec.revoke.clone(), cancel.clone(), Arc::downgrade(&alive)));
        let snapshot = self.snapshot();
        if self.config.threads == 0 {
            let result = cancel.check().and_then(|()| task(&snapshot, &cancel));
            drop(alive);
            on_done(result);
            return cancel;
        }
        self.executor().spawn_with(
            spec.priority,
            cancel,
            move |token| {
                let _alive = alive;
                task(&snapshot, token)
            },
            on_done,
        )
    }

    fn register_job(&mut self, spec: &JobSpec) -> CancelToken {
        let cancel = CancelToken::new();
        if let Some(key) = spec.coalesce
            && let Some(previous) = self.coalesced.insert(key, cancel.clone())
        {
            previous.cancel();
        }
        cancel
    }

    fn revoke(&mut self, touched: &[FileId], settings: bool) {
        self.revokers.retain(|(revoke, token, alive)| {
            if alive.strong_count() == 0 || token.is_cancelled() {
                return false;
            }
            let hit = match revoke {
                Revoke::Never => false,
                Revoke::OnAnyChangeOf(RevokeScope::All) => true,
                Revoke::OnAnyChangeOf(RevokeScope::Files(files)) => {
                    files.iter().any(|file| touched.contains(file))
                }
                Revoke::OnAnyChangeOf(RevokeScope::Settings) => settings,
            };
            if hit {
                token.cancel();
            }
            !hit
        });
        self.coalesced.retain(|_, token| !token.is_cancelled());
    }

    /// Running jobs registered for revocation (diagnostics).
    pub fn live_jobs(&self) -> usize {
        self.revokers
            .iter()
            .filter(|(_, token, alive)| alive.strong_count() > 0 && !token.is_cancelled())
            .count()
    }
}

/// Inline execution for hosts without threads.
struct InlineResult;

impl InlineResult {
    fn run<T: Send + 'static>(
        cancel: CancelToken,
        task: impl FnOnce(&CancelToken) -> Result<T, Cancelled>,
    ) -> JobHandle<T> {
        let result = cancel.check().and_then(|()| task(&cancel));
        JobHandle::ready(result, cancel)
    }
}

fn interface_hash(snapshot: &Snapshot, syntax: &SyntaxSnapshot) -> u64 {
    snapshot
        .registry()
        .get::<dyn InterfaceSummarizer>(syntax.lang().language_id())
        .map_or_else(
            || StructureInterface.interface_hash(syntax),
            |summarizer| summarizer.interface_hash(syntax),
        )
}

fn truncate_journal(head: &Arc<JournalNode>, keep: usize) -> Arc<JournalNode> {
    let mut nodes = Vec::new();
    let mut node = Some(head.clone());
    while let Some(current) = node {
        if nodes.len() == keep {
            break;
        }
        node = current.prev.clone();
        nodes.push(current);
    }
    let mut prev: Option<Arc<JournalNode>> = None;
    for (depth, current) in nodes.into_iter().rev().enumerate() {
        prev = Some(Arc::new(JournalNode {
            from: current.from,
            to: current.to,
            map: current.map.clone(),
            depth: depth + 1,
            prev,
        }));
    }
    prev.unwrap_or_else(|| head.clone())
}

/// Paths of every open buffer and its revision (diagnostics).
pub fn describe_open(snapshot: &Snapshot) -> Vec<(PathBuf, FileRev)> {
    let mut out: Vec<_> = snapshot
        .open_files()
        .filter_map(|file| Some((snapshot.path(file)?.to_path_buf(), snapshot.file_rev(file)?)))
        .collect();
    out.sort();
    out
}

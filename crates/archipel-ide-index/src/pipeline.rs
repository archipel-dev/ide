//! Running indexes over one file: the object-safe wrapper around
//! [`FileIndex`] and the per-file entry point shared by the scheduler, the
//! overlay and the tests.

use crate::indexes::{
    FileNameIndex, FileTypeIndex, MembersIndex, QualifiedIndex, ShortNameIndex, StubsIndex,
    SuperTypesIndex, TrigramIndex, WordsIndex, clear_stub_cache, ids,
};
use crate::stubs::StubBuilders;
use crate::todo::{TodoIndex, TodoSettings};
use archipel_ide_analysis::index::IndexStore;
use archipel_ide_analysis::index::{FileIndex, IndexInput, IndexPriority, index_file};
use archipel_ide_analysis::{CancelToken, Cancelled, FileMeta, IndexId};
use std::sync::Arc;

/// A [`FileIndex`] with its type erased (so one list holds every index).
pub trait ErasedIndex: Send + Sync {
    fn id(&self) -> IndexId;
    /// `VERSION` plus the index's settings salt; a different string rebuilds
    /// the index.
    fn version(&self) -> String;
    fn priority(&self) -> IndexPriority;
    fn depends_on_content(&self) -> bool;
    fn dumb_aware(&self) -> bool;
    /// The index needs only the text, never a syntax tree (`Trigrams`
    /// priority). Such indexes are built in a second pass after every other
    /// index is ready, so navigation does not wait for them.
    fn text_only(&self) -> bool;
    fn accepts(&self, file: &FileMeta) -> bool;
    /// Indexes `input` into `store` (replacing the file's previous entries).
    fn run(&self, store: &dyn IndexStore, input: &IndexInput<'_>) -> Result<(), Cancelled>;
}

/// Wraps a typed index.
pub struct Typed<I: FileIndex> {
    index: I,
    salt: String,
}

impl<I: FileIndex> Typed<I> {
    pub fn new(index: I) -> Self {
        Self {
            index,
            salt: String::new(),
        }
    }

    /// An index whose output depends on settings: `salt` changes with them.
    pub fn with_salt(index: I, salt: impl Into<String>) -> Self {
        Self {
            index,
            salt: salt.into(),
        }
    }
}

impl<I: FileIndex> ErasedIndex for Typed<I> {
    fn id(&self) -> IndexId {
        I::ID
    }
    fn version(&self) -> String {
        if self.salt.is_empty() {
            I::VERSION.to_string()
        } else {
            format!("{}-{}", I::VERSION, self.salt)
        }
    }
    fn priority(&self) -> IndexPriority {
        self.index.priority()
    }
    fn depends_on_content(&self) -> bool {
        self.index.depends_on_content()
    }
    fn dumb_aware(&self) -> bool {
        self.index.dumb_aware()
    }
    fn text_only(&self) -> bool {
        self.index.priority() == IndexPriority::Trigrams
    }
    fn accepts(&self, file: &FileMeta) -> bool {
        self.index.accepts(file)
    }
    fn run(&self, store: &dyn IndexStore, input: &IndexInput<'_>) -> Result<(), Cancelled> {
        index_file(store, &self.index, input)
    }
}

/// The indexes of the app, ordered by priority.
#[derive(Clone)]
pub struct IndexSet {
    indexes: Vec<Arc<dyn ErasedIndex>>,
    builders: Arc<StubBuilders>,
}

impl IndexSet {
    /// Every built-in index with the default TODO patterns.
    pub fn builtin(todo: &TodoSettings) -> Self {
        let builders = Arc::new(StubBuilders::default());
        let mut set = Self {
            indexes: Vec::new(),
            builders: builders.clone(),
        };
        set.add(Arc::new(Typed::new(FileNameIndex)));
        set.add(Arc::new(Typed::new(FileTypeIndex)));
        set.add(Arc::new(Typed::new(StubsIndex {
            builders: builders.clone(),
        })));
        set.add(Arc::new(Typed::new(ShortNameIndex {
            builders: builders.clone(),
        })));
        set.add(Arc::new(Typed::new(QualifiedIndex {
            builders: builders.clone(),
        })));
        set.add(Arc::new(Typed::new(SuperTypesIndex {
            builders: builders.clone(),
        })));
        set.add(Arc::new(Typed::new(MembersIndex { builders })));
        set.add(Arc::new(Typed::new(WordsIndex)));
        set.add(Arc::new(Typed::new(TrigramIndex)));
        set.set_todo(todo);
        set
    }

    pub fn builders(&self) -> &Arc<StubBuilders> {
        &self.builders
    }

    /// Adds (or replaces, by id) an index.
    pub fn add(&mut self, index: Arc<dyn ErasedIndex>) {
        self.indexes.retain(|existing| existing.id() != index.id());
        self.indexes.push(index);
        self.indexes.sort_by_key(|index| index.priority());
    }

    /// Replaces the TODO index with one for `settings`.
    pub fn set_todo(&mut self, settings: &TodoSettings) {
        let salt = TodoIndex::salt(settings);
        self.add(Arc::new(Typed::with_salt(TodoIndex::new(settings), salt)));
    }

    pub fn all(&self) -> &[Arc<dyn ErasedIndex>] {
        &self.indexes
    }

    pub fn get(&self, id: IndexId) -> Option<&Arc<dyn ErasedIndex>> {
        self.indexes.iter().find(|index| index.id() == id)
    }

    pub fn content(&self) -> impl Iterator<Item = &Arc<dyn ErasedIndex>> {
        self.indexes
            .iter()
            .filter(|index| index.depends_on_content())
    }

    /// Content indexes built from syntax and text.
    pub fn main_content(&self) -> impl Iterator<Item = &Arc<dyn ErasedIndex>> {
        self.indexes
            .iter()
            .filter(|index| index.depends_on_content() && !index.text_only())
    }

    /// Text-only content indexes (built in the second pass).
    pub fn text_content(&self) -> impl Iterator<Item = &Arc<dyn ErasedIndex>> {
        self.indexes
            .iter()
            .filter(|index| index.depends_on_content() && index.text_only())
    }

    pub fn names(&self) -> impl Iterator<Item = &Arc<dyn ErasedIndex>> {
        self.indexes
            .iter()
            .filter(|index| !index.depends_on_content())
    }

    /// Runs the content indexes (or only `only`) over one file. One parse,
    /// one stub tree: `input.syntax` is shared by every index.
    pub fn run_content(
        &self,
        store: &dyn IndexStore,
        input: &IndexInput<'_>,
        only: Option<&[IndexId]>,
        cancel: &CancelToken,
    ) -> Result<(), Cancelled> {
        clear_stub_cache();
        for index in self.content() {
            if only.is_some_and(|only| !only.contains(&index.id())) {
                continue;
            }
            cancel.check()?;
            if index.accepts(input.file) {
                index.run(store, input)?;
            } else {
                store.remove_file(index.id(), input.file.id);
            }
        }
        clear_stub_cache();
        Ok(())
    }

    /// Runs the name/metadata indexes over one file.
    pub fn run_names(
        &self,
        store: &dyn IndexStore,
        input: &IndexInput<'_>,
    ) -> Result<(), Cancelled> {
        for index in self.names() {
            if index.accepts(input.file) {
                index.run(store, input)?;
            } else {
                store.remove_file(index.id(), input.file.id);
            }
        }
        Ok(())
    }
}

/// Index ids in a stable order (diagnostics, status).
pub fn builtin_ids() -> [IndexId; 10] {
    [
        ids::FILE_NAME,
        ids::FILE_TYPE,
        ids::STUBS,
        ids::SHORT_NAME,
        ids::QUALIFIED,
        ids::SUPER_TYPES,
        ids::MEMBERS,
        ids::WORDS,
        ids::TRIGRAM,
        ids::TODO,
    ]
}

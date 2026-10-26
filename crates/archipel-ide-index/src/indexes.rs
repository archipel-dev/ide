//! The built-in file-based indexes (spec 1.6): `FileName`, `FileType`,
//! `Words`, `Trigram` and the stub-derived `Stubs`, `Stub:ShortName`,
//! `Stub:Qualified`, `Stub:SuperTypes`, `Stub:Members`. `Todo` is in
//! [`crate::todo`].

use crate::codec::{Reader, Writer};
use crate::hash::word_hash;
use crate::stubs::{
    ExtraKey, NO_PARENT, StubBuilders, StubTree, is_container, is_symbol, kind_code, kind_from_code,
};
use archipel_ide_analysis::index::{FileIndex, IndexInput, IndexKey, IndexPriority, IndexValue};
use archipel_ide_analysis::vfs::MAX_FILE_BYTES;
use archipel_ide_analysis::{Cancelled, FileMeta, IndexId};
use archipel_ide_language::SyntaxSnapshot;
use archipel_ide_language::structure::SymbolKind;
use std::cell::RefCell;
use std::sync::Arc;

/// Ids of the built-in indexes.
pub mod ids {
    use archipel_ide_analysis::IndexId;
    pub const FILE_NAME: IndexId = IndexId("FileName");
    pub const FILE_TYPE: IndexId = IndexId("FileType");
    pub const WORDS: IndexId = IndexId("Words");
    pub const TRIGRAM: IndexId = IndexId("Trigram");
    pub const TODO: IndexId = IndexId("Todo");
    pub const STUBS: IndexId = IndexId("Stubs");
    pub const SHORT_NAME: IndexId = IndexId("Stub:ShortName");
    pub const QUALIFIED: IndexId = IndexId("Stub:Qualified");
    pub const SUPER_TYPES: IndexId = IndexId("Stub:SuperTypes");
    pub const MEMBERS: IndexId = IndexId("Stub:Members");
}

/// Largest file whose syntax tree is built for indexing.
pub const MAX_SYNTAX_BYTES: u64 = 1024 * 1024;
/// Largest file indexed for trigrams.
pub const MAX_TRIGRAM_BYTES: u64 = 512 * 1024;

// ---- FileName ------------------------------------------------------------------

/// Lower-case file name -> () (metadata only; built during the scan).
#[derive(Debug, Clone, Copy, Default)]
pub struct FileNameIndex;

impl FileIndex for FileNameIndex {
    const ID: IndexId = ids::FILE_NAME;
    const VERSION: u32 = 1;
    type Key = String;
    type Value = ();
    fn accepts(&self, _: &FileMeta) -> bool {
        true
    }
    fn depends_on_content(&self) -> bool {
        false
    }
    fn index(
        &self,
        input: &IndexInput<'_>,
        out: &mut dyn FnMut(String, ()),
    ) -> Result<(), Cancelled> {
        if let Some(name) = input.file.path.file_name().and_then(|n| n.to_str()) {
            out(name.to_lowercase(), ());
        }
        Ok(())
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn priority(&self) -> IndexPriority {
        IndexPriority::Names
    }
}

// ---- FileType ------------------------------------------------------------------

/// Language id (`"plain_text"` for files without a grammar) -> ().
#[derive(Debug, Clone, Copy, Default)]
pub struct FileTypeIndex;

pub const PLAIN_TEXT: &str = "plain_text";

impl FileIndex for FileTypeIndex {
    const ID: IndexId = ids::FILE_TYPE;
    const VERSION: u32 = 1;
    type Key = String;
    type Value = ();
    fn accepts(&self, _: &FileMeta) -> bool {
        true
    }
    fn depends_on_content(&self) -> bool {
        false
    }
    fn index(
        &self,
        input: &IndexInput<'_>,
        out: &mut dyn FnMut(String, ()),
    ) -> Result<(), Cancelled> {
        out(
            input
                .file
                .language
                .map_or_else(|| PLAIN_TEXT.to_string(), |lang| lang.0.to_string()),
            (),
        );
        Ok(())
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn priority(&self) -> IndexPriority {
        IndexPriority::Names
    }
}

// ---- Words ---------------------------------------------------------------------

/// Where a word occurs ([`WordsIndex`] values).
pub mod context {
    pub const CODE: u32 = 1;
    pub const COMMENT: u32 = 2;
    pub const STRING: u32 = 4;
    pub const LITERAL: u32 = 8;
}

/// Hash of an identifier word -> mask of the contexts it occurs in.
#[derive(Debug, Clone, Copy, Default)]
pub struct WordsIndex;

#[derive(Clone, Copy)]
struct ContextRange {
    start: u32,
    end: u32,
    mask: u32,
}

/// Comment and string ranges of `syntax` (sorted, non-overlapping).
fn context_ranges(syntax: &SyntaxSnapshot) -> Vec<ContextRange> {
    let sets = syntax.lang().token_set_table();
    let mut out = Vec::new();
    let mut cursor = syntax.tree().walk();
    loop {
        let node = cursor.node();
        let ty = archipel_ide_language::ElementType(node.kind_id());
        let mask = if sets.comments.contains(ty) {
            context::COMMENT
        } else if sets.string_literals.contains(ty) {
            context::STRING
        } else {
            0
        };
        if mask != 0 {
            out.push(ContextRange {
                start: node.start_byte() as u32,
                end: node.end_byte() as u32,
                mask,
            });
        } else if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return out;
            }
        }
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// Longest word indexed (longer runs are data, not identifiers).
const MAX_WORD_CHARS: usize = 64;

/// Calls `f(word, byte offset)` for every identifier-like word of `text`.
pub fn for_each_word(text: &str, mut f: impl FnMut(&str, usize)) {
    let mut start: Option<usize> = None;
    for (at, c) in text.char_indices() {
        if is_word_char(c) {
            start.get_or_insert(at);
        } else if let Some(begin) = start.take() {
            f(&text[begin..at], begin);
        }
    }
    if let Some(begin) = start {
        f(&text[begin..], begin);
    }
}

impl FileIndex for WordsIndex {
    const ID: IndexId = ids::WORDS;
    const VERSION: u32 = 1;
    type Key = u64;
    type Value = u32;
    fn accepts(&self, f: &FileMeta) -> bool {
        f.len <= MAX_FILE_BYTES
    }
    fn index(
        &self,
        input: &IndexInput<'_>,
        out: &mut dyn FnMut(u64, u32),
    ) -> Result<(), Cancelled> {
        let ranges = input.syntax.map(context_ranges).unwrap_or_default();
        let mut next = 0usize;
        let mut words: std::collections::HashMap<u64, u32> = std::collections::HashMap::new();
        for_each_word(input.text, |word, at| {
            if word.chars().count() > MAX_WORD_CHARS {
                return;
            }
            let at = at as u32;
            while next < ranges.len() && ranges[next].end <= at {
                next += 1;
            }
            let mut mask = match ranges.get(next) {
                Some(range) if range.start <= at => range.mask,
                _ => context::CODE,
            };
            if mask == context::CODE && word.starts_with(|c: char| c.is_ascii_digit()) {
                mask = context::LITERAL;
            }
            *words.entry(word_hash(word)).or_default() |= mask;
        });
        for (hash, mask) in words {
            out(hash, mask);
        }
        Ok(())
    }
    fn priority(&self) -> IndexPriority {
        IndexPriority::Words
    }
}

// ---- Trigram -------------------------------------------------------------------

/// 24-bit trigram of lower-cased bytes -> ().
#[derive(Debug, Clone, Copy, Default)]
pub struct TrigramIndex;

/// The trigram key of three bytes (ASCII lower-cased).
pub fn trigram_key(a: u8, b: u8, c: u8) -> u32 {
    (u32::from(a.to_ascii_lowercase()) << 16)
        | (u32::from(b.to_ascii_lowercase()) << 8)
        | u32::from(c.to_ascii_lowercase())
}

/// Trigrams of `text` (distinct, sorted); lines are separate (no trigram
/// spans a newline).
pub fn trigrams(text: &str) -> Vec<u32> {
    let bytes = text.as_bytes();
    let mut keys: Vec<u32> = bytes
        .windows(3)
        .filter(|w| w.iter().all(|b| !matches!(b, b'\n' | b'\r' | 0)))
        .map(|w| trigram_key(w[0], w[1], w[2]))
        .collect();
    keys.sort_unstable();
    keys.dedup();
    keys
}

impl FileIndex for TrigramIndex {
    const ID: IndexId = ids::TRIGRAM;
    const VERSION: u32 = 1;
    type Key = u32;
    type Value = ();
    fn accepts(&self, f: &FileMeta) -> bool {
        f.len <= MAX_TRIGRAM_BYTES
    }
    fn index(&self, input: &IndexInput<'_>, out: &mut dyn FnMut(u32, ())) -> Result<(), Cancelled> {
        for key in trigrams(input.text) {
            out(key, ());
        }
        Ok(())
    }
    /// Find in Files falls back to a text scan while the index is built.
    fn dumb_aware(&self) -> bool {
        true
    }
    fn priority(&self) -> IndexPriority {
        IndexPriority::Trigrams
    }
}

// ---- Stub indexes --------------------------------------------------------------

/// One declaration found through a stub index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubEntry {
    /// Node index inside the file's [`StubTree`] ([`NO_PARENT`] for relations
    /// without a declaration).
    pub stub: u32,
    pub kind: SymbolKind,
    pub flags: u16,
    /// Zero-based line of the name.
    pub line: u32,
    /// Short name of the declaration.
    pub name: String,
    /// Short name of the containing declaration.
    pub container: Option<String>,
}

impl IndexValue for StubEntry {
    fn encode(&self, out: &mut Vec<u8>) {
        let mut w = Writer::new(out);
        w.varint(u64::from(self.stub));
        w.u8(kind_code(self.kind));
        w.varint(u64::from(self.flags));
        w.varint(u64::from(self.line));
        w.str(&self.name);
        w.str(self.container.as_deref().unwrap_or(""));
    }
    fn decode(bytes: &[u8]) -> Option<Self> {
        let mut r = Reader::new(bytes);
        Some(Self {
            stub: r.u32()?,
            kind: kind_from_code(r.u8()?),
            flags: u16::try_from(r.varint()?).ok()?,
            line: r.u32()?,
            name: r.str()?.to_string(),
            container: Some(r.str()?.to_string()).filter(|c| !c.is_empty()),
        })
    }
}

impl IndexValue for StubTree {
    fn encode(&self, out: &mut Vec<u8>) {
        StubTree::encode(self, out);
    }
    fn decode(bytes: &[u8]) -> Option<Self> {
        StubTree::decode(bytes)
    }
}

/// `(container short name, member name)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MemberKey {
    pub container: String,
    pub name: String,
}

impl MemberKey {
    pub fn new(container: &str, name: &str) -> Self {
        Self {
            container: container.to_string(),
            name: name.to_string(),
        }
    }
}

impl IndexKey for MemberKey {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.container.as_bytes());
        out.push(0);
        out.extend_from_slice(self.name.as_bytes());
    }
    fn decode(bytes: &[u8]) -> Option<Self> {
        let split = bytes.iter().position(|b| *b == 0)?;
        Some(Self {
            container: String::from_utf8(bytes[..split].to_vec()).ok()?,
            name: String::from_utf8(bytes[split + 1..].to_vec()).ok()?,
        })
    }
}

thread_local! {
    static LAST_TREE: RefCell<Option<(TreeKey, Arc<StubTree>)>> = const { RefCell::new(None) };
}

#[derive(PartialEq, Eq)]
struct TreeKey {
    file: u32,
    ptr: usize,
    len: usize,
    lang: &'static str,
}

/// Forgets the stub tree cached for this thread. The pipeline calls it
/// before every file; direct callers of the stub indexes need not.
pub fn clear_stub_cache() {
    LAST_TREE.with(|cell| *cell.borrow_mut() = None);
}

/// The stub tree of `input`, built once per file and thread and shared by
/// the five stub indexes (one parse, one tree).
pub fn stub_tree(builders: &StubBuilders, input: &IndexInput<'_>) -> Option<Arc<StubTree>> {
    let syntax = input.syntax?;
    if !syntax.lang().has_tags() {
        return None;
    }
    let key = TreeKey {
        file: input.file.id.0,
        ptr: input.text.as_ptr() as usize,
        len: input.text.len(),
        lang: syntax.lang().id,
    };
    LAST_TREE.with(|cell| {
        let mut cell = cell.borrow_mut();
        if let Some((cached, tree)) = cell.as_ref()
            && *cached == key
        {
            return Some(tree.clone());
        }
        let tree = Arc::new(builders.build(syntax));
        *cell = Some((key, tree.clone()));
        Some(tree)
    })
}

fn entry(tree: &StubTree, index: usize) -> Option<StubEntry> {
    let node = tree.nodes.get(index)?;
    Some(StubEntry {
        stub: index as u32,
        kind: node.kind,
        flags: node.flags,
        line: node.line,
        name: node.name.clone(),
        container: tree.container_name(index).map(str::to_string),
    })
}

fn stub_accepts(f: &FileMeta) -> bool {
    f.len <= MAX_SYNTAX_BYTES
}

/// File -> its serialized declarations.
pub struct StubsIndex {
    pub builders: Arc<StubBuilders>,
}

impl FileIndex for StubsIndex {
    const ID: IndexId = ids::STUBS;
    const VERSION: u32 = 1;
    type Key = u32;
    type Value = StubTree;
    fn accepts(&self, f: &FileMeta) -> bool {
        stub_accepts(f)
    }
    fn index(
        &self,
        input: &IndexInput<'_>,
        out: &mut dyn FnMut(u32, StubTree),
    ) -> Result<(), Cancelled> {
        if let Some(tree) = stub_tree(&self.builders, input)
            && !tree.nodes.is_empty()
        {
            out(input.file.id.0, (*tree).clone());
        }
        Ok(())
    }
    fn dumb_aware(&self) -> bool {
        false
    }
    fn priority(&self) -> IndexPriority {
        IndexPriority::Stubs
    }
}

macro_rules! stub_index {
    ($(#[$doc:meta])* $name:ident, $id:expr, $key:ty, |$tree:ident, $emit:ident| $body:block) => {
        $(#[$doc])*
        pub struct $name {
            pub builders: Arc<StubBuilders>,
        }

        impl FileIndex for $name {
            const ID: IndexId = $id;
            const VERSION: u32 = 1;
            type Key = $key;
            type Value = StubEntry;
            fn accepts(&self, f: &FileMeta) -> bool {
                stub_accepts(f)
            }
            fn index(
                &self,
                input: &IndexInput<'_>,
                out: &mut dyn FnMut($key, StubEntry),
            ) -> Result<(), Cancelled> {
                let Some($tree) = stub_tree(&self.builders, input) else {
                    return Ok(());
                };
                let $tree = &*$tree;
                let $emit = out;
                $body
                Ok(())
            }
            fn priority(&self) -> IndexPriority {
                IndexPriority::Stubs
            }
        }
    };
}

stub_index!(
    /// Declaration short name -> entry (Go to Class / Symbol).
    ShortNameIndex, ids::SHORT_NAME, String, |tree, emit| {
        for (at, node) in tree.nodes.iter().enumerate() {
            if is_symbol(node.kind) && !node.name.is_empty() && let Some(e) = entry(tree, at) {
                emit(node.name.clone(), e);
            }
        }
    }
);

stub_index!(
    /// Qualified name -> entry (imports, FQN navigation).
    QualifiedIndex, ids::QUALIFIED, String, |tree, emit| {
        for (at, node) in tree.nodes.iter().enumerate() {
            if is_symbol(node.kind) && !node.name.is_empty()
                && let (Some(q), Some(e)) = (tree.qualified_name(at), entry(tree, at))
            {
                emit(q, e);
            }
        }
    }
);

stub_index!(
    /// Super type short name -> the subtype's entry (Go to Implementation,
    /// type hierarchy).
    SuperTypesIndex, ids::SUPER_TYPES, String, |tree, emit| {
        for (at, node) in tree.nodes.iter().enumerate() {
            for key in [ExtraKey::Extends, ExtraKey::Implements] {
                for sup in node.extras_of(key) {
                    if let Some(e) = entry(tree, at) {
                        emit(sup.to_string(), e);
                    }
                }
            }
        }
        for (sub, sup) in &tree.relations {
            emit(
                sup.clone(),
                StubEntry {
                    stub: NO_PARENT,
                    kind: SymbolKind::Class,
                    flags: 0,
                    line: 0,
                    name: sub.clone(),
                    container: None,
                },
            );
        }
    }
);

stub_index!(
    /// `(container, member name)` -> entry (member completion, overriding).
    MembersIndex, ids::MEMBERS, MemberKey, |tree, emit| {
        for (at, node) in tree.nodes.iter().enumerate() {
            if node.parent == NO_PARENT || node.name.is_empty() || !is_symbol(node.kind) {
                continue;
            }
            let Some(parent) = tree.nodes.get(node.parent as usize) else {
                continue;
            };
            if is_container(parent.kind)
                && let Some(e) = entry(tree, at)
            {
                emit(MemberKey::new(&parent.name, &node.name), e);
            }
        }
    }
);

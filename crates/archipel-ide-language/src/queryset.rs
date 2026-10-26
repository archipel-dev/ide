//! Compiled, cached queries of every language: `folds.scm`, `indents.scm`,
//! `brackets.scm`, `textobjects.scm`, `sticky.scm`, `doc.scm`, `locals.scm` and
//! `tags.scm`. A query that fails to compile is reported by
//! [`compile_error`] (a test keeps that empty for every language) and behaves
//! as absent, so a broken data file degrades one feature, never the editor.

use crate::registry::{LANGUAGES, LanguageSpec};
use std::sync::OnceLock;
use tree_sitter::Query;

/// One query file of a language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryKind {
    Folds,
    Indents,
    Brackets,
    TextObjects,
    Sticky,
    Doc,
    Locals,
    Tags,
}

impl QueryKind {
    pub const ALL: [QueryKind; 8] = [
        QueryKind::Folds,
        QueryKind::Indents,
        QueryKind::Brackets,
        QueryKind::TextObjects,
        QueryKind::Sticky,
        QueryKind::Doc,
        QueryKind::Locals,
        QueryKind::Tags,
    ];

    /// File name of the query (`folds.scm`).
    pub fn file_name(self) -> &'static str {
        match self {
            QueryKind::Folds => "folds.scm",
            QueryKind::Indents => "indents.scm",
            QueryKind::Brackets => "brackets.scm",
            QueryKind::TextObjects => "textobjects.scm",
            QueryKind::Sticky => "sticky.scm",
            QueryKind::Doc => "doc.scm",
            QueryKind::Locals => "locals.scm",
            QueryKind::Tags => "tags.scm",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// The query text of `kind` for `spec` (tags are concatenated).
pub fn source(spec: &LanguageSpec, kind: QueryKind) -> Option<String> {
    let extra = spec.queries().extra;
    match kind {
        QueryKind::Folds => spec.folds.map(str::to_owned),
        QueryKind::Indents => extra.indents.map(str::to_owned),
        QueryKind::Brackets => extra.brackets.map(str::to_owned),
        QueryKind::TextObjects => extra.textobjects.map(str::to_owned),
        QueryKind::Sticky => extra.sticky.map(str::to_owned),
        QueryKind::Doc => extra.doc_comments.map(str::to_owned),
        QueryKind::Locals => spec.locals.map(str::to_owned),
        QueryKind::Tags => (!spec.tags.is_empty()).then(|| spec.tags.join("\n")),
    }
}

type Slot = OnceLock<Result<Query, String>>;

fn slots() -> &'static Vec<[Slot; 8]> {
    static CACHE: OnceLock<Vec<[Slot; 8]>> = OnceLock::new();
    CACHE.get_or_init(|| {
        LANGUAGES
            .iter()
            .map(|_| std::array::from_fn(|_| OnceLock::new()))
            .collect()
    })
}

fn compiled(
    spec: &'static LanguageSpec,
    kind: QueryKind,
) -> Option<&'static Result<Query, String>> {
    let index = LANGUAGES
        .iter()
        .position(|candidate| std::ptr::eq(candidate, spec))?;
    let source = source(spec, kind)?;
    Some(
        slots()[index][kind.index()].get_or_init(|| {
            Query::new(&spec.grammar(), &source).map_err(|error| error.to_string())
        }),
    )
}

/// The compiled query, `None` when the language has none or it is invalid.
pub fn query(spec: &'static LanguageSpec, kind: QueryKind) -> Option<&'static Query> {
    compiled(spec, kind)?.as_ref().ok()
}

/// Why the query of `kind` does not compile (`None` when absent or valid).
pub fn compile_error(spec: &'static LanguageSpec, kind: QueryKind) -> Option<String> {
    compiled(spec, kind)?.as_ref().err().cloned()
}

/// The value of `#set! key "value"` of the pattern, if any.
pub fn setting<'q>(query: &'q Query, pattern: usize, key: &str) -> Option<&'q str> {
    query
        .property_settings(pattern)
        .iter()
        .find(|property| &*property.key == key)
        .map(|property| property.value.as_deref().unwrap_or(""))
}

/// One capture of a query run: capture name, node and the pattern it came from.
#[derive(Debug, Clone, Copy)]
pub struct Captured<'t> {
    pub name: &'static str,
    pub node: tree_sitter::Node<'t>,
    pub pattern: usize,
}

/// Captures of `query` on nodes intersecting `range` (the ancestors of the
/// range included), in document order. Cheap for small ranges: only the path
/// from the root to the range and the nodes inside it are visited.
pub fn captures_in<'t>(
    query: &'static Query,
    tree: &'t tree_sitter::Tree,
    source: &str,
    range: std::ops::Range<usize>,
) -> Vec<Captured<'t>> {
    use tree_sitter::{QueryCursor, StreamingIterator};
    let names = query.capture_names();
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(range.start..range.end.max(range.start + 1));
    let mut out = Vec::new();
    let mut matches = cursor.matches(query, tree.root_node(), source.as_bytes());
    while let Some(found) = matches.next() {
        for capture in found.captures {
            out.push(Captured {
                name: names[capture.index as usize],
                node: capture.node,
                pattern: found.pattern_index,
            });
        }
    }
    out.sort_by_key(|captured| (captured.node.start_byte(), captured.pattern));
    out
}

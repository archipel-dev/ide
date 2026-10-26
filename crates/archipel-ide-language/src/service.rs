//! The language service the editor calls (spec Part 5): everything the kit's
//! editor model cannot know without a parser, behind one trait over byte
//! offsets and pure-data answers.
//!
//! The kit repository has no such trait yet; this is the IDE-side definition
//! with the shape proposed for `archipel_kit::host::LanguageServices`, so
//! moving it there is a rename plus a thin adapter (`DocView` becomes the
//! kit's `DocView { doc, id, revision, path_hint }`, `EditSet` its
//! `EditSet`). `highlights`, `occurrences`, `hover` and `syntax_highlighter`
//! of the proposal come from the daemon, resolve and documentation packages
//! and are composed by the host, not here.
//!
//! [`QueryServices`] implements the trait from the query files and the
//! generic engines; [`NoLanguageServices`] answers like a plain-text editor.

use crate::brackets::{self, PairKind};
use crate::commenter;
use crate::edit::{EditSet, TypedDecision};
use crate::folding::FoldKind;
use crate::indent::{IndentEngine, IndentOptions};
use crate::lines::LineIndex;
use crate::move_statement;
use crate::node::SyntaxSnapshot;
use crate::smart_enter;
use crate::sticky::{self, StickyScope};
use crate::structure::{StructureItem, SymbolKind};
use crate::syntax_data::{LanguageData, PLAIN_TEXT};
use crate::textobjects;
use crate::typing;
use std::ops::Range;
use std::sync::{Arc, Mutex};

/// A document revision and the identity under which results are cached.
#[derive(Clone, Copy)]
pub struct DocView<'a> {
    pub snapshot: &'a SyntaxSnapshot,
    pub id: u64,
    pub revision: u64,
    pub path_hint: Option<&'a str>,
}

/// Coarse class of the token at an offset (word stops, double click, delete word).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TokenClass {
    Identifier,
    Keyword,
    String,
    StringEscape,
    Number,
    Operator,
    Punctuation,
    Comment,
    DocComment,
    Whitespace,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenInfo {
    pub class: TokenClass,
    pub range: Range<usize>,
}

/// One breadcrumb (outermost first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crumb {
    pub label: String,
    pub kind: SymbolKind,
    pub range: Range<usize>,
}

/// A fold region as the editor ingests it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldSpec {
    pub range: Range<usize>,
    pub placeholder: String,
    pub collapsed_by_default: bool,
    pub kind: FoldKind,
    pub group: Option<u32>,
}

/// What the editor needs from the language layer. Every method has a default
/// that reproduces a plain-text editor, so hosts implement what they have.
pub trait LanguageServices {
    /// Editing data (comment prefixes, pairs, quotes) of the language at `offset`.
    fn language_at(&self, _doc: &DocView<'_>, _offset: usize) -> &'static LanguageData {
        &PLAIN_TEXT
    }
    /// Token class at `offset`.
    fn token_at(&self, _doc: &DocView<'_>, _offset: usize) -> Option<TokenInfo> {
        None
    }
    /// Extend Selection: the smallest syntactic range strictly containing `range`.
    fn extend_selection(&self, _doc: &DocView<'_>, _range: Range<usize>) -> Option<Range<usize>> {
        None
    }
    /// Both delimiters of the bracket or tag at `offset` (syntax-aware).
    fn matching_brace(&self, _doc: &DocView<'_>, _offset: usize) -> Option<(usize, usize)> {
        None
    }
    /// The innermost code block around `offset` (Code Block Start/End), both braces.
    fn code_block(&self, _doc: &DocView<'_>, _offset: usize) -> Option<Range<usize>> {
        None
    }
    /// Breadcrumb path at `offset`.
    fn breadcrumbs(&self, _doc: &DocView<'_>, _offset: usize) -> Vec<Crumb> {
        Vec::new()
    }
    /// Fold regions of the whole document.
    fn fold_regions(&self, _doc: &DocView<'_>) -> Vec<FoldSpec> {
        Vec::new()
    }
    /// Sticky scopes for the first visible line, outermost first, at most `max`.
    fn sticky_scopes(
        &self,
        _doc: &DocView<'_>,
        _first_visible_line: usize,
        _max: usize,
    ) -> Vec<StickyScope> {
        Vec::new()
    }
    /// Decision before `typed` is inserted at `caret`.
    fn before_typed(
        &self,
        _doc: &DocView<'_>,
        _caret: usize,
        _selection: Option<Range<usize>>,
        _typed: char,
    ) -> TypedDecision {
        TypedDecision::Default
    }
    /// Edits after `typed` was inserted; `doc` already holds the new text and
    /// `caret` is the position after the character.
    fn after_typed(&self, _doc: &DocView<'_>, _caret: usize, _typed: char) -> Option<EditSet> {
        None
    }
    /// Enter: indentation of the new line, `{|}` splitting, comment continuation.
    fn on_enter(&self, _doc: &DocView<'_>, _selection: Range<usize>) -> Option<EditSet> {
        None
    }
    /// Complete Current Statement.
    fn smart_enter(&self, _doc: &DocView<'_>, _caret: usize) -> Option<EditSet> {
        None
    }
    /// Move Statement Up/Down; `None` falls back to Move Line.
    fn move_statement(
        &self,
        _doc: &DocView<'_>,
        _range: Range<usize>,
        _up: bool,
    ) -> Option<EditSet> {
        None
    }
    /// Indentation in columns for `line`; `None` leaves the line alone.
    fn indent_for_line(&self, _doc: &DocView<'_>, _line: usize) -> Option<usize> {
        None
    }
    /// Auto-Indent Lines: the new indentation of every line of `lines` that changes.
    fn reindent_lines(&self, _doc: &DocView<'_>, _lines: Range<usize>) -> Vec<(usize, usize)> {
        Vec::new()
    }
    /// Comment with Block Comment.
    fn toggle_block_comment(&self, _doc: &DocView<'_>, _range: Range<usize>) -> Option<EditSet> {
        None
    }
    /// Next/Previous Method target from `offset`.
    fn method_boundary(&self, _doc: &DocView<'_>, _offset: usize, _next: bool) -> Option<usize> {
        None
    }
}

/// Answers like a plain-text editor (every default of the trait).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoLanguageServices;

impl LanguageServices for NoLanguageServices {}

struct Cached {
    key: (u64, u64),
    lines: Arc<LineIndex>,
    structure: Arc<Vec<StructureItem>>,
    options: IndentOptions,
}

/// The query-driven implementation: caches the line index, structure and
/// indentation style per `(document id, revision)` so per-frame calls (sticky
/// scopes, brace matching) stay cheap.
#[derive(Default)]
pub struct QueryServices {
    cache: Mutex<Option<Cached>>,
    indent_override: Mutex<Option<IndentOptions>>,
}

impl QueryServices {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fixes the indentation style instead of detecting it from the text
    /// (code style settings, `.editorconfig`).
    pub fn set_indent_options(&self, options: Option<IndentOptions>) {
        if let Ok(mut slot) = self.indent_override.lock() {
            *slot = options;
        }
    }

    fn cached(
        &self,
        doc: &DocView<'_>,
    ) -> (Arc<LineIndex>, Arc<Vec<StructureItem>>, IndentOptions) {
        let key = (doc.id, doc.revision);
        let mut slot = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if slot.as_ref().is_none_or(|cached| cached.key != key) {
            let text = doc.snapshot.text();
            let options = self
                .indent_override
                .lock()
                .ok()
                .and_then(|slot| *slot)
                .unwrap_or_else(|| IndentOptions::detect(text));
            *slot = Some(Cached {
                key,
                lines: Arc::new(LineIndex::new(text)),
                structure: Arc::new(doc.snapshot.structure()),
                options,
            });
        }
        let cached = slot.as_ref().expect("filled above");
        (
            cached.lines.clone(),
            cached.structure.clone(),
            cached.options,
        )
    }

    /// The indentation style used for `doc`.
    pub fn indent_options(&self, doc: &DocView<'_>) -> IndentOptions {
        self.cached(doc).2
    }
}

fn classify(snap: &SyntaxSnapshot, offset: usize) -> Option<TokenInfo> {
    let inner = commenter::innermost(snap, offset);
    let leaf = inner.root().find_leaf_at(u32::try_from(offset).ok()?)?;
    let table = inner.lang().token_set_table();
    let data = crate::syntax_data::language_data(inner.lang().id).unwrap_or(&PLAIN_TEXT);
    // Walk up to the outermost string/comment around the leaf.
    for node in std::iter::once(leaf).chain(leaf.ancestors()) {
        let ty = crate::ElementType(node.raw().kind_id());
        if table.comments.contains(ty) {
            let text = node.text();
            let doc = data
                .doc_block_open
                .is_some_and(|open| text.starts_with(open) && !text.starts_with("/**/"))
                || data
                    .doc_line_prefix
                    .is_some_and(|prefix| text.starts_with(prefix));
            return Some(TokenInfo {
                class: if doc {
                    TokenClass::DocComment
                } else {
                    TokenClass::Comment
                },
                range: node.byte_range(),
            });
        }
        if table.string_literals.contains(ty) {
            let escape = leaf.kind().contains("escape") && leaf.byte_range() != node.byte_range();
            return Some(TokenInfo {
                class: if escape {
                    TokenClass::StringEscape
                } else {
                    TokenClass::String
                },
                range: if escape {
                    leaf.byte_range()
                } else {
                    node.byte_range()
                },
            });
        }
    }
    let kind = leaf.kind();
    let range = leaf.byte_range();
    let class = if table
        .identifiers
        .contains(crate::ElementType(leaf.raw().kind_id()))
    {
        TokenClass::Identifier
    } else if kind.contains("number") || kind.contains("integer") || kind.contains("float") {
        TokenClass::Number
    } else if (!leaf.is_named() && kind.chars().all(|c| c.is_alphabetic() || c == '_'))
        || data.keywords.contains(&leaf.text())
    {
        TokenClass::Keyword
    } else if !leaf.is_named()
        && matches!(kind, "(" | ")" | "[" | "]" | "{" | "}" | "," | ";" | ".")
    {
        TokenClass::Punctuation
    } else if !leaf.is_named() && !kind.is_empty() {
        TokenClass::Operator
    } else {
        TokenClass::Other
    };
    Some(TokenInfo { class, range })
}

fn path_to(items: &[StructureItem], offset: usize) -> Vec<Crumb> {
    crate::structure::path_at(items, offset)
        .into_iter()
        .map(|item| Crumb {
            label: item.name.clone(),
            kind: item.kind,
            range: item.range.clone(),
        })
        .collect()
}

impl LanguageServices for QueryServices {
    fn language_at(&self, doc: &DocView<'_>, offset: usize) -> &'static LanguageData {
        commenter::data_at(doc.snapshot, offset)
    }

    fn token_at(&self, doc: &DocView<'_>, offset: usize) -> Option<TokenInfo> {
        classify(doc.snapshot, offset)
    }

    fn extend_selection(&self, doc: &DocView<'_>, range: Range<usize>) -> Option<Range<usize>> {
        doc.snapshot.expand_selection(range)
    }

    fn matching_brace(&self, doc: &DocView<'_>, offset: usize) -> Option<(usize, usize)> {
        brackets::pair_at(doc.snapshot, offset).map(|pair| pair.offsets())
    }

    fn code_block(&self, doc: &DocView<'_>, offset: usize) -> Option<Range<usize>> {
        let pair = brackets::enclosing_block(doc.snapshot, offset)?;
        let (open, close) = pair.offsets();
        debug_assert!(matches!(pair.kind, PairKind::Block | PairKind::Brace));
        Some(open..close)
    }

    fn breadcrumbs(&self, doc: &DocView<'_>, offset: usize) -> Vec<Crumb> {
        let (_, structure, _) = self.cached(doc);
        path_to(&structure, offset)
    }

    fn fold_regions(&self, doc: &DocView<'_>) -> Vec<FoldSpec> {
        doc.snapshot
            .folds_in(0..doc.snapshot.text().len())
            .into_iter()
            .map(|fold| FoldSpec {
                range: fold.start_byte..fold.end_byte,
                placeholder: fold.placeholder,
                collapsed_by_default: fold.collapsed_by_default,
                kind: fold.kind,
                group: fold.group,
            })
            .collect()
    }

    fn sticky_scopes(
        &self,
        doc: &DocView<'_>,
        first_visible_line: usize,
        max: usize,
    ) -> Vec<StickyScope> {
        let (lines, structure, _) = self.cached(doc);
        sticky::sticky_scopes_with(doc.snapshot, &lines, &structure, first_visible_line, max)
    }

    fn before_typed(
        &self,
        doc: &DocView<'_>,
        caret: usize,
        selection: Option<Range<usize>>,
        typed: char,
    ) -> TypedDecision {
        typing::before_typed(doc.snapshot, caret, selection, typed)
    }

    fn after_typed(&self, doc: &DocView<'_>, caret: usize, typed: char) -> Option<EditSet> {
        let options = self.cached(doc).2;
        typing::after_typed(doc.snapshot, caret, typed, options)
    }

    fn on_enter(&self, doc: &DocView<'_>, selection: Range<usize>) -> Option<EditSet> {
        let options = self.cached(doc).2;
        typing::on_enter(doc.snapshot, selection, options)
    }

    fn smart_enter(&self, doc: &DocView<'_>, caret: usize) -> Option<EditSet> {
        let options = self.cached(doc).2;
        smart_enter::smart_enter(doc.snapshot, caret, options)
    }

    fn move_statement(&self, doc: &DocView<'_>, range: Range<usize>, up: bool) -> Option<EditSet> {
        move_statement::move_statement(doc.snapshot, range, up)
    }

    fn indent_for_line(&self, doc: &DocView<'_>, line: usize) -> Option<usize> {
        let options = self.cached(doc).2;
        let engine = IndentEngine::new(doc.snapshot, options);
        engine.for_line(line, &|row| engine.indent_of(row))
    }

    fn reindent_lines(&self, doc: &DocView<'_>, lines: Range<usize>) -> Vec<(usize, usize)> {
        let options = self.cached(doc).2;
        IndentEngine::new(doc.snapshot, options).reindent(lines)
    }

    fn toggle_block_comment(&self, doc: &DocView<'_>, range: Range<usize>) -> Option<EditSet> {
        commenter::toggle_block_comment(doc.snapshot, range)
    }

    fn method_boundary(&self, doc: &DocView<'_>, offset: usize, next: bool) -> Option<usize> {
        if next {
            textobjects::next_method(doc.snapshot, offset)
        } else {
            textobjects::previous_method(doc.snapshot, offset)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(snapshot: &SyntaxSnapshot, revision: u64) -> DocView<'_> {
        DocView {
            snapshot,
            id: 1,
            revision,
            path_hint: None,
        }
    }

    fn snap(id: &str, source: &str) -> SyntaxSnapshot {
        let spec = crate::registry::by_id(id).expect("language");
        SyntaxSnapshot::parse(Default::default(), Default::default(), spec, source.into())
            .expect("parse")
    }

    #[test]
    fn plain_text_services_answer_with_defaults() {
        let snapshot = snap("java", "class A {}\n");
        let doc = view(&snapshot, 1);
        let none = NoLanguageServices;
        assert!(none.extend_selection(&doc, 0..0).is_none());
        assert!(none.fold_regions(&doc).is_empty());
        assert_eq!(
            none.before_typed(&doc, 0, None, '('),
            TypedDecision::Default
        );
        assert_eq!(none.language_at(&doc, 0).id, "plain");
    }

    #[test]
    fn query_services_drive_the_editor_features() {
        let source = "class A {\n    void f() {\n        g(1);\n    }\n}\n";
        let snapshot = snap("java", source);
        let doc = view(&snapshot, 1);
        let services = QueryServices::new();
        let at = source.find("g(1)").unwrap();
        assert_eq!(services.language_at(&doc, at).id, "java");
        assert_eq!(
            services.token_at(&doc, at).map(|t| t.class),
            Some(TokenClass::Identifier)
        );
        assert_eq!(
            services
                .token_at(&doc, source.find("void").unwrap())
                .map(|t| t.class),
            Some(TokenClass::Keyword)
        );
        assert_eq!(services.extend_selection(&doc, at..at), Some(at..at + 1));
        let crumbs: Vec<_> = services
            .breadcrumbs(&doc, at)
            .into_iter()
            .map(|c| c.label)
            .collect();
        assert_eq!(crumbs, ["A", "f"]);
        let block = services.code_block(&doc, at).expect("block");
        assert_eq!(&source[block.start..=block.start], "{");
        assert_eq!(services.fold_regions(&doc).len(), 2);
        assert_eq!(services.sticky_scopes(&doc, 3, 5).len(), 2);
        assert_eq!(services.indent_for_line(&doc, 2), Some(8));
        assert!(
            services
                .matching_brace(&doc, source.find('{').unwrap())
                .is_some()
        );
    }
}

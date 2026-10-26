//! Structural editing services (implemented by WP4 and language crates):
//! commenter, braces, quotes, folding, structure, extend selection, indent,
//! typed handlers, smart enter, move statement, sticky scopes, breadcrumbs.
//!
//! The generic tree-sitter implementations of folding, structure, extend
//! selection and brace matching are registered for every language
//! ([`register_generic_services`]); languages override them per service.

use crate::cancel::{CancelToken, Cancelled};
use crate::modcommand::TextEdit;
use archipel_ide_language::structure::{StructureItem, SymbolKind};
use archipel_ide_language::{FoldRange, Registration, ServiceRegistry, SyntaxSnapshot};
use std::ops::Range;
use std::sync::Arc;

/// Pure edits plus the resulting caret/selection (applied in one undo group).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditSet {
    pub edits: Vec<TextEdit>,
    pub caret: Option<u32>,
    pub selection: Option<Range<u32>>,
}

/// Before a typed character is inserted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TypedDecision {
    #[default]
    Default,
    /// Step over the existing closer.
    Overtype,
    InsertPair(char),
    Wrap {
        open: char,
        close: char,
    },
    /// Swallow the character.
    Consume,
}

/// Indentation settings in effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndentOptions {
    pub indent_size: u32,
    pub tab_size: u32,
    pub use_tabs: bool,
}

impl Default for IndentOptions {
    fn default() -> Self {
        Self {
            indent_size: 4,
            tab_size: 4,
            use_tabs: false,
        }
    }
}

pub trait Commenter: Send + Sync {
    fn line_comment_prefix(&self) -> Option<&str>;
    fn block_comment(&self) -> Option<(&str, &str)>;
    fn doc_comment_prefix(&self) -> Option<&str> {
        None
    }
}

/// One bracket pair (`PairedBraceMatcher`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BracePair {
    pub open: &'static str,
    pub close: &'static str,
    /// Delimits a structural block (`{}`), used by code-block navigation.
    pub structural: bool,
}

pub trait BraceMatcher: Send + Sync {
    fn pairs(&self) -> &[BracePair];
    /// Partner of the bracket token starting at `offset` (both starts).
    fn matching(&self, syntax: &SyntaxSnapshot, offset: u32) -> Option<(u32, u32)>;
}

pub trait QuoteHandler: Send + Sync {
    /// Typing `quote` at `offset` should insert a closing quote too.
    fn should_pair(&self, syntax: &SyntaxSnapshot, offset: u32, quote: char) -> bool;
}

pub trait FoldingBuilder: Send + Sync {
    fn folds(
        &self,
        syntax: &SyntaxSnapshot,
        range: Range<u32>,
        cancel: &CancelToken,
    ) -> Result<Vec<FoldRange>, Cancelled>;
}

pub trait StructureProvider: Send + Sync {
    fn structure(
        &self,
        syntax: &SyntaxSnapshot,
        cancel: &CancelToken,
    ) -> Result<Vec<StructureItem>, Cancelled>;
}

/// Extend Selection step (several handlers may contribute; the smallest
/// strictly larger range wins).
pub trait ExtendSelectionHandler: Send + Sync {
    fn extend(&self, syntax: &SyntaxSnapshot, range: Range<u32>) -> Option<Range<u32>>;
}

pub trait Indenter: Send + Sync {
    /// Indentation (columns) of `line`; `None` = copy the previous line.
    fn indent_for_line(
        &self,
        syntax: &SyntaxSnapshot,
        line: u32,
        options: &IndentOptions,
    ) -> Option<u32>;
}

pub trait TypedHandler: Send + Sync {
    fn before_typed(&self, syntax: &SyntaxSnapshot, caret: u32, typed: char) -> TypedDecision;
    fn after_typed(&self, syntax: &SyntaxSnapshot, caret: u32, typed: char) -> Option<EditSet>;
    fn on_enter(
        &self,
        _syntax: &SyntaxSnapshot,
        _caret: u32,
        _options: &IndentOptions,
    ) -> Option<EditSet> {
        None
    }
}

pub trait SmartEnterProcessor: Send + Sync {
    fn smart_enter(&self, syntax: &SyntaxSnapshot, caret: u32) -> Option<EditSet>;
}

pub trait MoveStatementHandler: Send + Sync {
    /// `None` = fall back to Move Line.
    fn move_statement(
        &self,
        syntax: &SyntaxSnapshot,
        range: Range<u32>,
        up: bool,
    ) -> Option<EditSet>;
}

/// A scope whose header stays pinned while its body scrolls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickyScope {
    pub header: Range<u32>,
    pub start_line: u32,
    pub end_line: u32,
}

pub trait StickyScopesProvider: Send + Sync {
    fn sticky_scopes(
        &self,
        syntax: &SyntaxSnapshot,
        first_visible_line: u32,
        max: usize,
    ) -> Vec<StickyScope>;
}

/// One breadcrumb.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crumb {
    pub label: String,
    pub kind: Option<SymbolKind>,
    pub range: Range<u32>,
}

pub trait BreadcrumbsProvider: Send + Sync {
    fn breadcrumbs(&self, syntax: &SyntaxSnapshot, offset: u32) -> Vec<Crumb>;
}

fn u32_range(range: Range<usize>) -> Range<u32> {
    range.start as u32..range.end as u32
}

/// Generic folding from `folds.scm` and the grammar.
#[derive(Debug, Clone, Copy, Default)]
pub struct GenericFolding;

impl FoldingBuilder for GenericFolding {
    fn folds(
        &self,
        syntax: &SyntaxSnapshot,
        range: Range<u32>,
        cancel: &CancelToken,
    ) -> Result<Vec<FoldRange>, Cancelled> {
        cancel.check()?;
        Ok(syntax.folds_in(range.start as usize..range.end as usize))
    }
}

/// Generic structure from `tags.scm`.
#[derive(Debug, Clone, Copy, Default)]
pub struct GenericStructure;

impl StructureProvider for GenericStructure {
    fn structure(
        &self,
        syntax: &SyntaxSnapshot,
        cancel: &CancelToken,
    ) -> Result<Vec<StructureItem>, Cancelled> {
        cancel.check()?;
        Ok(syntax.structure())
    }
}

/// Generic tree-walk Extend Selection.
#[derive(Debug, Clone, Copy, Default)]
pub struct GenericExtendSelection;

impl ExtendSelectionHandler for GenericExtendSelection {
    fn extend(&self, syntax: &SyntaxSnapshot, range: Range<u32>) -> Option<Range<u32>> {
        syntax
            .expand_selection(range.start as usize..range.end as usize)
            .map(u32_range)
    }
}

const GENERIC_PAIRS: &[BracePair] = &[
    BracePair {
        open: "(",
        close: ")",
        structural: false,
    },
    BracePair {
        open: "[",
        close: "]",
        structural: false,
    },
    BracePair {
        open: "{",
        close: "}",
        structural: true,
    },
];

/// Generic syntax-aware bracket matching.
#[derive(Debug, Clone, Copy, Default)]
pub struct GenericBraceMatcher;

impl BraceMatcher for GenericBraceMatcher {
    fn pairs(&self) -> &[BracePair] {
        GENERIC_PAIRS
    }

    fn matching(&self, syntax: &SyntaxSnapshot, offset: u32) -> Option<(u32, u32)> {
        syntax
            .matching_bracket(offset as usize)
            .map(|(a, b)| (a as u32, b as u32))
    }
}

/// Registers the generic tree-sitter services for every language
/// (dumb-aware, overridable per language).
pub fn register_generic_services(registry: &mut ServiceRegistry) {
    registry.register::<dyn FoldingBuilder>(
        Registration::any("generic-folding").dumb_aware(true),
        Arc::new(GenericFolding),
    );
    registry.register::<dyn StructureProvider>(
        Registration::any("generic-structure").dumb_aware(true),
        Arc::new(GenericStructure),
    );
    registry.register::<dyn ExtendSelectionHandler>(
        Registration::any("generic-extend-selection").dumb_aware(true),
        Arc::new(GenericExtendSelection),
    );
    registry.register::<dyn BraceMatcher>(
        Registration::any("generic-braces").dumb_aware(true),
        Arc::new(GenericBraceMatcher),
    );
}

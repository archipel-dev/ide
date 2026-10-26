//! `LanguageDef`: everything the platform knows about one language (the
//! `ParserDefinition` + `Language` + `FileType` + highlighter-factory
//! analogue), as static data plus a hook registering optional services.

use crate::element::{ElementTypeTable, TokenSetSpec, TokenSets};
use crate::extensions::ServiceRegistry;
use crate::ids::LanguageId;
use std::sync::OnceLock;
use tree_sitter_language::LanguageFn;

/// How much of a language is understood natively (spec section 3.1). Gates
/// which tools may fire (D6): a G0 language never reports "unresolved".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum CompletenessTier {
    /// Syntax only (tree-sitter + queries).
    #[default]
    G0,
    /// + file-local resolve (`locals.scm`).
    G1,
    /// + project-level name resolution.
    G2,
    /// + types, members, inheritance.
    G3,
    /// + generics, inference, data flow.
    G4,
}

/// Query files beyond the ones every grammar crate ships (filled per language
/// by the structural-features package; `None` = generic fallback).
#[derive(Debug, Clone, Copy, Default)]
pub struct ExtraQueries {
    pub indents: Option<&'static str>,
    pub brackets: Option<&'static str>,
    pub textobjects: Option<&'static str>,
    pub sticky: Option<&'static str>,
    pub todo: Option<&'static str>,
    pub doc_comments: Option<&'static str>,
    pub reads_writes: Option<&'static str>,
}

impl ExtraQueries {
    pub const NONE: ExtraQueries = ExtraQueries {
        indents: None,
        brackets: None,
        textobjects: None,
        sticky: None,
        todo: None,
        doc_comments: None,
        reads_writes: None,
    };
}

/// Borrowed view of every query of a language.
#[derive(Debug, Clone, Copy)]
pub struct LanguageQueries {
    pub highlights: &'static [&'static str],
    pub injections: Option<&'static str>,
    pub locals: Option<&'static str>,
    pub folds: Option<&'static str>,
    pub tags: &'static [&'static str],
    pub extra: ExtraQueries,
}

/// Lazily built per-language tables (the static part of [`LanguageDef`]).
#[derive(Debug)]
pub struct LanguageTables {
    element_types: OnceLock<ElementTypeTable>,
    token_sets: OnceLock<TokenSets>,
}

impl LanguageTables {
    pub const fn new() -> Self {
        Self {
            element_types: OnceLock::new(),
            token_sets: OnceLock::new(),
        }
    }
}

impl Default for LanguageTables {
    fn default() -> Self {
        Self::new()
    }
}

/// One registered language. File-type matching is `extensions`, `file_names`
/// and `shebangs` (see [`crate::registry::detect`]).
pub struct LanguageDef {
    /// Stable identifier (also used as the injection language name).
    pub id: &'static str,
    /// Display name shown in the status bar and menus.
    pub name: &'static str,
    /// File extensions without the leading dot, matched case-insensitively.
    pub extensions: &'static [&'static str],
    /// Exact file names (e.g. `BUILD.bazel`).
    pub file_names: &'static [&'static str],
    /// Interpreter names matched against a `#!` first line.
    pub shebangs: &'static [&'static str],
    pub(crate) grammar: LanguageFn,
    /// Highlight queries, concatenated in order. Base-language queries come first
    /// (C before C++, JavaScript before TypeScript), as upstream configures them.
    pub(crate) highlights: &'static [&'static str],
    pub(crate) injections: Option<&'static str>,
    pub(crate) locals: Option<&'static str>,
    /// Folding query (`folds.scm`) when the grammar ships one.
    pub(crate) folds: Option<&'static str>,
    /// Tags queries (definitions for Structure/breadcrumbs), concatenated.
    pub(crate) tags: &'static [&'static str],
    /// Capture-name overrides for languages whose IntelliJ keys differ from
    /// the defaults (e.g. `tag` → `HTML_TAG_NAME`). Data, not code.
    pub(crate) capture_keys: &'static [(&'static str, &'static str)],
    /// Base language id (services fall back to it: TypeScript -> JavaScript).
    pub base: Option<&'static str>,
    pub tier: CompletenessTier,
    /// Token sets by kind name, resolved on first use.
    pub token_sets: TokenSetSpec,
    pub extra_queries: ExtraQueries,
    /// Registers the language's optional services (overrides of the generic
    /// "any language" ones).
    pub services: fn(&mut ServiceRegistry),
    pub(crate) tables: LanguageTables,
}

/// The `services` hook of a language without its own services.
pub fn no_services(_: &mut ServiceRegistry) {}

impl LanguageDef {
    pub fn language_id(&self) -> LanguageId {
        LanguageId(self.id)
    }

    pub fn display_name(&self) -> &'static str {
        self.name
    }

    pub fn base_id(&self) -> Option<LanguageId> {
        self.base.map(LanguageId)
    }

    pub fn grammar(&self) -> tree_sitter::Language {
        self.grammar.into()
    }

    pub fn queries(&self) -> LanguageQueries {
        LanguageQueries {
            highlights: self.highlights,
            injections: self.injections,
            locals: self.locals,
            folds: self.folds,
            tags: self.tags,
            extra: self.extra_queries,
        }
    }

    pub fn has_tags(&self) -> bool {
        !self.tags.is_empty()
    }

    pub fn has_injections(&self) -> bool {
        self.injections.is_some()
    }

    pub fn has_locals(&self) -> bool {
        self.locals.is_some()
    }

    /// Kinds and fields of the grammar (built on first use).
    pub fn element_types(&self) -> &ElementTypeTable {
        self.tables
            .element_types
            .get_or_init(|| ElementTypeTable::build(&self.grammar()))
    }

    /// Resolved token sets.
    pub fn token_set_table(&self) -> &TokenSets {
        self.tables
            .token_sets
            .get_or_init(|| TokenSets::resolve(&self.token_sets, self.element_types()))
    }

    /// Names in [`LanguageDef::token_sets`] the grammar does not define (data
    /// errors; a registry test keeps this empty).
    pub fn unknown_token_names(&self) -> Vec<&'static str> {
        let table = self.element_types();
        self.token_sets
            .names()
            .filter(|name| !table.contains_name(name))
            .collect()
    }
}

impl std::fmt::Debug for LanguageDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LanguageDef").field("id", &self.id).finish()
    }
}

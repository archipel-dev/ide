//! Declarations as navigation targets, shared by resolve (WP5), indexes (WP2)
//! and the LSP provider (WP8).

use crate::file::{FileId, LanguageId};
use archipel_ide_language::Anchor;
pub use archipel_ide_language::structure::SymbolKind;
use std::ops::Range;
use std::sync::Arc;

/// How certain a semantic answer is (D6). Ordered: `Exact` is best.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Confidence {
    Exact,
    Likely,
    Heuristic,
}

/// A language server's key (`"rust-analyzer"`, or server id + root).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ServerKey(pub Arc<str>);

/// A library (JDK, jar, crate) providing declarations outside the project.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LibraryId(pub Arc<str>);

/// Where an answer came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Provenance {
    Native(LanguageId),
    Lsp(ServerKey),
    Library(LibraryId),
}

/// A declaration (PsiNamedElement + NavigationItem). Persistable, cheap to
/// clone; `anchor` is re-resolved against the live tree on navigation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Target {
    pub file: FileId,
    pub anchor: Anchor,
    pub name: Arc<str>,
    pub name_range: Range<u32>,
    pub kind: SymbolKind,
    pub container: Option<Arc<str>>,
    pub provenance: Provenance,
    pub confidence: Confidence,
}

impl Target {
    /// Dedupe key across providers: same file and name range.
    pub fn location_key(&self) -> (FileId, Range<u32>) {
        (self.file, self.name_range.clone())
    }
}

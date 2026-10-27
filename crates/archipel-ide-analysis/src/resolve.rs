//! Reference and resolve contracts (implemented by WP5's
//! `archipel-ide-resolve`, per-language crates and the LSP provider).

use crate::cancel::{CancelToken, Cancelled};
use crate::file::FileId;
use crate::host::Snapshot;
use crate::modcommand::TextEdit;
use crate::target::{Confidence, Target};
use archipel_ide_language::structure::SymbolKind;
use archipel_ide_language::{Anchor, SyntaxNode, SyntaxSnapshot, TokenSet};
use smallvec::SmallVec;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

/// A set of files as a bitset over [`FileId`]s (O(1) `contains`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileSet {
    bits: Vec<u64>,
}

impl FileSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, file: FileId) {
        let word = file.0 as usize / 64;
        if word >= self.bits.len() {
            self.bits.resize(word + 1, 0);
        }
        self.bits[word] |= 1 << (file.0 % 64);
    }

    pub fn contains(&self, file: FileId) -> bool {
        self.bits
            .get(file.0 as usize / 64)
            .is_some_and(|word| word & (1 << (file.0 % 64)) != 0)
    }

    pub fn len(&self) -> usize {
        self.bits
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = FileId> + '_ {
        self.bits.iter().enumerate().flat_map(|(index, word)| {
            (0..64u32)
                .filter(move |bit| word & (1u64 << bit) != 0)
                .map(move |bit| FileId(index as u32 * 64 + bit))
        })
    }

    pub fn union(&self, other: &FileSet) -> FileSet {
        let len = self.bits.len().max(other.bits.len());
        FileSet {
            bits: (0..len)
                .map(|i| self.bits.get(i).unwrap_or(&0) | other.bits.get(i).unwrap_or(&0))
                .collect(),
        }
    }

    pub fn intersection(&self, other: &FileSet) -> FileSet {
        FileSet {
            bits: self
                .bits
                .iter()
                .zip(&other.bits)
                .map(|(a, b)| a & b)
                .collect(),
        }
    }
}

impl FromIterator<FileId> for FileSet {
    fn from_iter<I: IntoIterator<Item = FileId>>(iter: I) -> Self {
        let mut set = FileSet::new();
        for file in iter {
            set.insert(file);
        }
        set
    }
}

/// `GlobalSearchScope` analogue. `contains` is O(1) for file sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchScope {
    Project,
    /// Files under a directory (resolved to a [`FileSet`] by the caller for
    /// speed; `contains` answers false until then).
    Directory(PathBuf),
    Files(FileSet),
    Module(Arc<str>),
    Libraries,
    CurrentFile(FileId),
    Union(Vec<SearchScope>),
    Intersection(Vec<SearchScope>),
}

impl SearchScope {
    pub fn contains(&self, file: FileId) -> bool {
        match self {
            SearchScope::Project => true,
            SearchScope::Directory(_) | SearchScope::Module(_) | SearchScope::Libraries => false,
            SearchScope::Files(files) => files.contains(file),
            SearchScope::CurrentFile(current) => *current == file,
            SearchScope::Union(scopes) => scopes.iter().any(|scope| scope.contains(file)),
            SearchScope::Intersection(scopes) => scopes.iter().all(|scope| scope.contains(file)),
        }
    }
}

/// A completion variant (`LookupElement`).
#[derive(Debug, Clone, PartialEq)]
pub struct LookupItem {
    pub label: Arc<str>,
    pub kind: Option<SymbolKind>,
    /// Text inserted (defaults to the label).
    pub insert_text: Option<Arc<str>>,
    pub detail: Option<Arc<str>>,
    pub target: Option<Target>,
    /// Higher sorts first among equal weigher results.
    pub priority: i32,
}

impl LookupItem {
    pub fn new(label: impl Into<Arc<str>>) -> Self {
        Self {
            label: label.into(),
            kind: None,
            insert_text: None,
            detail: None,
            target: None,
            priority: 0,
        }
    }
}

/// One resolve answer; `valid = false` for candidates that fail checks
/// (wrong arity, inaccessible) but are still worth showing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveHit {
    pub target: Target,
    pub valid: bool,
}

/// Multi-resolve result.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolveResults(pub SmallVec<[ResolveHit; 1]>);

impl ResolveResults {
    pub fn single(target: Target) -> Self {
        Self(smallvec::smallvec![ResolveHit {
            target,
            valid: true
        }])
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The best confidence among the valid hits.
    pub fn confidence(&self) -> Option<Confidence> {
        self.0
            .iter()
            .filter(|hit| hit.valid)
            .map(|hit| hit.target.confidence)
            .min()
    }
}

/// Context of a resolve.
pub struct ResolveCx<'a> {
    pub snap: &'a Snapshot,
    pub cancel: &'a CancelToken,
    pub scope: &'a SearchScope,
    /// Resolve in broken code (completion, unfinished statements).
    pub incomplete_code: bool,
}

/// A reference from a syntax node to declarations (`PsiReference`).
pub trait Reference: Send + Sync {
    fn node(&self) -> &Anchor;
    fn range_in_node(&self) -> Range<u32>;
    fn canonical_text(&self) -> Arc<str>;
    /// Multi-resolve by default.
    fn resolve(&self, cx: &ResolveCx<'_>) -> Result<ResolveResults, Cancelled>;
    /// Completion variants.
    fn variants(&self, cx: &ResolveCx<'_>) -> Result<Vec<LookupItem>, Cancelled>;
    /// Default: the resolve results contain `t`.
    fn is_reference_to(&self, t: &Target, cx: &ResolveCx<'_>) -> Result<bool, Cancelled> {
        Ok(self.resolve(cx)?.0.iter().any(|hit| hit.target == *t))
    }
    /// The edit renaming this reference (`handleElementRename`).
    fn rename_edit(&self, new_name: &str) -> Option<TextEdit>;
    /// Unresolved is not an error.
    fn is_soft(&self) -> bool;
}

/// `PsiReferenceContributor`: finds the references of a node.
pub trait ReferenceProvider: Send + Sync {
    /// Visit only these element types.
    fn interesting(&self) -> &TokenSet;
    fn references(&self, n: SyntaxNode<'_>, out: &mut Vec<Box<dyn Reference>>);
    /// Priority for the merge rule (PLAT-PSI-111): default 0, higher +100, lower -100.
    fn priority(&self) -> f32 {
        0.0
    }
}

/// How complete a resolver is (whether "unresolved" may be reported).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveCompleteness {
    Complete,
    PartialNoUnresolvedDiagnostics,
}

/// The per-language core resolver.
pub trait Resolver: Send + Sync {
    fn resolve_at(
        &self,
        snap: &SyntaxSnapshot,
        leaf: SyntaxNode<'_>,
        cx: &ResolveCx<'_>,
    ) -> Result<ResolveResults, Cancelled>;
    fn completeness(&self) -> ResolveCompleteness;
    /// Optional visible-name enumeration for completion: calls `sink` for
    /// every declaration visible at `leaf` until it returns false.
    fn scope_processor_hint(
        &self,
        _snap: &SyntaxSnapshot,
        _leaf: SyntaxNode<'_>,
        _cx: &ResolveCx<'_>,
        _sink: &mut dyn FnMut(&Target) -> bool,
    ) -> Result<(), Cancelled> {
        Ok(())
    }
}

/// How a usage accesses its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UsageKind {
    Read,
    Write,
    ReadWrite,
    Declaration,
    Import,
    Doc,
    Comment,
    String,
}

/// One found usage.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Usage {
    pub file: FileId,
    pub range: Range<u32>,
    pub kind: UsageKind,
    pub confidence: Confidence,
}

/// `ReferencesSearch` analogue: streams usages of `target` in `scope`;
/// `sink` returns false to stop.
pub trait UsageSearcher: Send + Sync {
    fn search(
        &self,
        target: &Target,
        cx: &ResolveCx<'_>,
        sink: &mut dyn FnMut(Usage) -> bool,
    ) -> Result<(), Cancelled>;
}

/// Read/write classification of a reference (`ReadWriteAccessDetector`).
pub trait ReadWriteAccessDetector: Send + Sync {
    fn access(&self, reference: SyntaxNode<'_>) -> UsageKind;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_sets_and_scopes() {
        let set: FileSet = [FileId(1), FileId(70), FileId(1)].into_iter().collect();
        assert_eq!(set.len(), 2);
        assert!(set.contains(FileId(70)) && !set.contains(FileId(2)));
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![FileId(1), FileId(70)]);
        let other: FileSet = [FileId(70), FileId(3)].into_iter().collect();
        assert_eq!(
            set.intersection(&other).iter().collect::<Vec<_>>(),
            vec![FileId(70)]
        );
        assert_eq!(set.union(&other).len(), 3);
        let scope = SearchScope::Intersection(vec![
            SearchScope::Project,
            SearchScope::Union(vec![
                SearchScope::Files(set),
                SearchScope::CurrentFile(FileId(9)),
            ]),
        ]);
        assert!(scope.contains(FileId(9)) && scope.contains(FileId(1)));
        assert!(!scope.contains(FileId(3)));
    }
}

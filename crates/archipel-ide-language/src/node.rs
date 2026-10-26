//! The "PSI" layer: an immutable [`SyntaxSnapshot`] of one file revision and
//! borrowing [`SyntaxNode`] handles into it.
//!
//! A snapshot is cheap to clone (one `Arc`), `Send + Sync`, and never changes:
//! background work holds one while the editor keeps typing. Nodes are `Copy`
//! handles valid as long as the snapshot is borrowed; to keep a reference
//! across revisions use an [`crate::Anchor`].

use crate::def::LanguageDef;
use crate::element::{ElementType, FieldId};
use crate::highlight::HighlightLayer;
use crate::ids::{FileId, FileRev};
use crate::injection;
use crate::structure::StructureItem;
use crate::syntax::SyntaxTree;
use crate::{Diagnostic, FoldRange};
use std::ops::Range;
use std::sync::{Arc, OnceLock};
use tree_sitter::{Node, Tree};

/// Immutable text of a snapshot (shared with the analysis that produced it).
pub type TextSnapshot = Arc<str>;

/// One ERROR or MISSING node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    pub range: Range<u32>,
    /// `true` for a MISSING (zero-width, expected) token.
    pub missing: bool,
    pub message: Arc<str>,
}

/// An injected-language region (Markdown fence, `<script>`, Rust macro body)
/// with its own snapshot. Injected trees are parsed over the host text with
/// included ranges, so their offsets ARE host offsets: the offset map is the
/// identity and fragment nodes can be compared with host nodes directly.
#[derive(Clone, Debug)]
pub struct InjectedFragment {
    /// Host byte ranges the fragment is parsed from (sorted, disjoint).
    pub host_ranges: Vec<Range<u32>>,
    pub snapshot: SyntaxSnapshot,
}

impl InjectedFragment {
    pub fn lang(&self) -> &'static LanguageDef {
        self.snapshot.lang()
    }

    /// Host offset of a fragment offset (identity, see the type docs).
    pub fn to_host(&self, offset: u32) -> u32 {
        offset
    }

    pub fn contains(&self, offset: u32) -> bool {
        self.host_ranges
            .iter()
            .any(|range| range.start <= offset && offset < range.end)
    }
}

struct Inner {
    file: FileId,
    rev: FileRev,
    lang: &'static LanguageDef,
    text: TextSnapshot,
    tree: Tree,
    depth: u8,
    fragments: OnceLock<Arc<[InjectedFragment]>>,
    errors: OnceLock<Arc<[SyntaxError]>>,
}

/// Immutable parse result of one file revision. See the module docs.
#[derive(Clone)]
pub struct SyntaxSnapshot {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for SyntaxSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyntaxSnapshot")
            .field("file", &self.inner.file)
            .field("rev", &self.inner.rev)
            .field("lang", &self.inner.lang.id)
            .finish()
    }
}

impl SyntaxSnapshot {
    /// Wraps an already parsed `tree` of `text` (the tree is ref-counted; this
    /// does not copy it).
    pub fn new(
        file: FileId,
        rev: FileRev,
        lang: &'static LanguageDef,
        text: TextSnapshot,
        tree: Tree,
    ) -> Self {
        Self::with_depth(file, rev, lang, text, tree, 0)
    }

    fn with_depth(
        file: FileId,
        rev: FileRev,
        lang: &'static LanguageDef,
        text: TextSnapshot,
        tree: Tree,
        depth: u8,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                file,
                rev,
                lang,
                text,
                tree,
                depth,
                fragments: OnceLock::new(),
                errors: OnceLock::new(),
            }),
        }
    }

    /// Snapshot of an incrementally maintained [`SyntaxTree`].
    pub fn from_tree(file: FileId, rev: FileRev, syntax: &SyntaxTree, text: TextSnapshot) -> Self {
        Self::new(file, rev, syntax.language(), text, syntax.tree().clone())
    }

    /// Parses `text` from scratch.
    pub fn parse(
        file: FileId,
        rev: FileRev,
        lang: &'static LanguageDef,
        text: TextSnapshot,
    ) -> Option<Self> {
        let syntax = SyntaxTree::parse(lang, &text)?;
        Some(Self::from_tree(file, rev, &syntax, text))
    }

    pub fn file(&self) -> FileId {
        self.inner.file
    }

    pub fn rev(&self) -> FileRev {
        self.inner.rev
    }

    pub fn lang(&self) -> &'static LanguageDef {
        self.inner.lang
    }

    pub fn text(&self) -> &TextSnapshot {
        &self.inner.text
    }

    pub fn tree(&self) -> &Tree {
        &self.inner.tree
    }

    /// True when both handles share the same immutable parse.
    pub fn ptr_eq(&self, other: &SyntaxSnapshot) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    pub fn root(&self) -> SyntaxNode<'_> {
        SyntaxNode {
            snap: self,
            node: self.inner.tree.root_node(),
        }
    }

    /// The leaf at `offset` (IntelliJ `findElementAt`).
    pub fn node_at(&self, offset: u32) -> Option<SyntaxNode<'_>> {
        self.root().find_leaf_at(offset)
    }

    /// The smallest node covering `range`.
    pub fn covering(&self, range: Range<u32>) -> SyntaxNode<'_> {
        self.root().covering(range)
    }

    /// ERROR and MISSING nodes (computed once).
    pub fn errors(&self) -> &[SyntaxError] {
        self.inner.errors.get_or_init(|| {
            let mut out = Vec::new();
            collect_errors(self.inner.tree.root_node(), &mut out);
            out.into()
        })
    }

    /// Injected fragments (computed once, nested up to the injection depth cap).
    pub fn fragments(&self) -> &[InjectedFragment] {
        self.inner.fragments.get_or_init(|| {
            if usize::from(self.inner.depth) >= injection::MAX_DEPTH {
                return Arc::from(Vec::new());
            }
            let text = &self.inner.text;
            injection::injections(self.inner.lang, &self.inner.tree, text, 0..text.len())
                .into_iter()
                .filter_map(|found| {
                    let tree = found.parse(text)?;
                    Some(InjectedFragment {
                        host_ranges: found
                            .ranges
                            .iter()
                            .map(|range| range.start_byte as u32..range.end_byte as u32)
                            .collect(),
                        snapshot: SyntaxSnapshot::with_depth(
                            self.inner.file,
                            self.inner.rev,
                            found.spec,
                            text.clone(),
                            tree,
                            self.inner.depth + 1,
                        ),
                    })
                })
                .collect::<Vec<_>>()
                .into()
        })
    }

    /// The innermost injected fragment containing `offset`, if any.
    pub fn fragment_at(&self, offset: u32) -> Option<&InjectedFragment> {
        let fragment = self
            .fragments()
            .iter()
            .find(|fragment| fragment.contains(offset))?;
        Some(fragment.snapshot.fragment_at(offset).unwrap_or(fragment))
    }

    // ---- generic features over the snapshot (the `LanguageAnalysis` facade
    // calls these, so both report identical results) ----

    /// Syntax highlights of `range`.
    pub fn highlight(&self, range: Range<usize>) -> HighlightLayer {
        crate::highlight::highlight(self.inner.lang, &self.inner.tree, &self.inner.text, range)
            .unwrap_or_default()
    }

    /// Declarations of the file, nested.
    pub fn structure(&self) -> Vec<StructureItem> {
        crate::structure::structure(self.inner.lang, &self.inner.tree, &self.inner.text)
    }

    /// Folding regions intersecting `range`.
    pub fn folds_in(&self, range: Range<usize>) -> Vec<FoldRange> {
        crate::folding::fold_ranges(self.inner.lang, &self.inner.tree, &self.inner.text, range)
    }

    /// Extend Selection from `range`.
    pub fn expand_selection(&self, range: Range<usize>) -> Option<Range<usize>> {
        crate::selection::expand(&self.inner.tree, &self.inner.text, range)
    }

    /// Matching bracket of the bracket token starting at `byte`.
    pub fn matching_bracket(&self, byte: usize) -> Option<(usize, usize)> {
        crate::brackets::matching_bracket(&self.inner.tree, byte)
    }

    /// Syntax diagnostics (row/column, capped like the editor's).
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        crate::collect_diagnostics(self.inner.tree.root_node(), &mut out);
        out
    }
}

fn collect_errors(node: Node<'_>, out: &mut Vec<SyntaxError>) {
    if !(node.has_error() || node.is_missing()) {
        return;
    }
    let range = node.start_byte() as u32..node.end_byte() as u32;
    if node.is_missing() {
        out.push(SyntaxError {
            range,
            missing: true,
            message: format!("expected {}", node.kind()).into(),
        });
        return;
    }
    if node.is_error() {
        out.push(SyntaxError {
            range,
            missing: false,
            message: "syntax error".into(),
        });
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_errors(child, out);
    }
}

/// A borrowing handle to one node of a [`SyntaxSnapshot`]. `Copy`; navigation
/// allocates nothing.
#[derive(Clone, Copy)]
pub struct SyntaxNode<'s> {
    snap: &'s SyntaxSnapshot,
    node: Node<'s>,
}

impl std::fmt::Debug for SyntaxNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{:?}", self.kind(), self.range())
    }
}

impl PartialEq for SyntaxNode<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.snap.ptr_eq(other.snap) && self.node == other.node
    }
}

impl Eq for SyntaxNode<'_> {}

impl<'s> SyntaxNode<'s> {
    fn wrap(&self, node: Node<'s>) -> Self {
        Self {
            snap: self.snap,
            node,
        }
    }

    pub fn snapshot(&self) -> &'s SyntaxSnapshot {
        self.snap
    }

    /// The underlying tree-sitter node (for queries).
    pub fn raw(&self) -> Node<'s> {
        self.node
    }

    pub fn element_type(&self) -> ElementType {
        ElementType(self.node.kind_id())
    }

    pub fn kind(&self) -> &'static str {
        self.node.kind()
    }

    pub fn range(&self) -> Range<u32> {
        self.node.start_byte() as u32..self.node.end_byte() as u32
    }

    pub fn byte_range(&self) -> Range<usize> {
        self.node.byte_range()
    }

    pub fn text(&self) -> &'s str {
        self.snap
            .inner
            .text
            .get(self.node.byte_range())
            .unwrap_or("")
    }

    pub fn is_named(&self) -> bool {
        self.node.is_named()
    }

    pub fn is_error(&self) -> bool {
        self.node.is_error()
    }

    pub fn is_missing(&self) -> bool {
        self.node.is_missing()
    }

    pub fn parent(&self) -> Option<Self> {
        self.node.parent().map(|node| self.wrap(node))
    }

    pub fn first_child(&self) -> Option<Self> {
        self.node.child(0).map(|node| self.wrap(node))
    }

    pub fn last_child(&self) -> Option<Self> {
        let count = self.node.child_count();
        count
            .checked_sub(1)
            .and_then(|last| self.node.child(last))
            .map(|node| self.wrap(node))
    }

    pub fn next_sibling(&self) -> Option<Self> {
        self.node.next_sibling().map(|node| self.wrap(node))
    }

    pub fn prev_sibling(&self) -> Option<Self> {
        self.node.prev_sibling().map(|node| self.wrap(node))
    }

    /// All children, tokens included.
    pub fn children(&self) -> impl Iterator<Item = SyntaxNode<'s>> + 's {
        let first = self.first_child();
        std::iter::successors(first, |node| node.next_sibling())
    }

    pub fn named_children(&self) -> impl Iterator<Item = SyntaxNode<'s>> + 's {
        self.children().filter(|node| node.is_named())
    }

    /// Parent, grandparent, ... up to the root.
    pub fn ancestors(&self) -> impl Iterator<Item = SyntaxNode<'s>> + 's {
        std::iter::successors(self.parent(), |node| node.parent())
    }

    /// The child in the grammar field `field`.
    pub fn child_by_field(&self, field: FieldId) -> Option<Self> {
        self.node
            .child_by_field_id(field.0)
            .map(|node| self.wrap(node))
    }

    pub fn child_by_field_name(&self, name: &str) -> Option<Self> {
        self.node
            .child_by_field_name(name)
            .map(|node| self.wrap(node))
    }

    /// The declared name of this node: its `name` field, or the node itself
    /// when it is an identifier-like leaf.
    pub fn name_node(&self) -> Option<Self> {
        if let Some(name) = self.child_by_field_name("name") {
            return Some(name);
        }
        let identifiers = &self.snap.inner.lang.token_set_table().identifiers;
        identifiers.contains(self.element_type()).then_some(*self)
    }

    /// The deepest node starting at or spanning `offset`.
    pub fn find_leaf_at(&self, offset: u32) -> Option<Self> {
        let offset = offset as usize;
        if offset < self.node.start_byte() || offset > self.node.end_byte() {
            return None;
        }
        let mut node = self.node.descendant_for_byte_range(offset, offset)?;
        while let Some(child) = (0..node.child_count())
            .filter_map(|index| node.child(index))
            .find(|child| child.start_byte() <= offset && offset < child.end_byte())
        {
            node = child;
        }
        Some(self.wrap(node))
    }

    /// The smallest descendant covering `range` (IntelliJ `getParentOfType`
    /// starting point).
    pub fn covering(&self, range: Range<u32>) -> Self {
        self.node
            .descendant_for_byte_range(range.start as usize, range.end as usize)
            .map(|node| self.wrap(node))
            .unwrap_or(*self)
    }

    /// Pre-order traversal of this subtree (tokens included).
    pub fn preorder(&self) -> Preorder<'s> {
        Preorder {
            snap: self.snap,
            cursor: self.node.walk(),
            done: false,
        }
    }
}

/// Pre-order iterator over a subtree; see [`SyntaxNode::preorder`].
pub struct Preorder<'s> {
    snap: &'s SyntaxSnapshot,
    cursor: tree_sitter::TreeCursor<'s>,
    done: bool,
}

impl<'s> Iterator for Preorder<'s> {
    type Item = SyntaxNode<'s>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let node = SyntaxNode {
            snap: self.snap,
            node: self.cursor.node(),
        };
        if !self.cursor.goto_first_child() {
            loop {
                if self.cursor.goto_next_sibling() {
                    break;
                }
                if !self.cursor.goto_parent() {
                    self.done = true;
                    break;
                }
            }
        }
        Some(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(lang: &str, text: &str) -> SyntaxSnapshot {
        let lang = crate::registry::by_id(lang).expect("language");
        SyntaxSnapshot::parse(FileId(1), FileRev(1), lang, text.into()).expect("parse")
    }

    #[test]
    fn navigation_and_names() {
        let snap = snap("rust", "fn alpha() { let x = 1; }\nstruct Beta;\n");
        let root = snap.root();
        assert_eq!(root.kind(), "source_file");
        let items: Vec<_> = root.named_children().map(|n| n.kind()).collect();
        assert_eq!(items, ["function_item", "struct_item"]);
        let function = root.first_child().expect("fn");
        assert_eq!(function.name_node().map(|n| n.text()), Some("alpha"));
        assert_eq!(
            function
                .next_sibling()
                .and_then(|n| n.name_node())
                .map(|n| n.text()),
            Some("Beta")
        );
        let leaf = snap.node_at(17).expect("leaf");
        assert_eq!(leaf.text(), "x");
        assert_eq!(
            leaf.ancestors().last().map(|n| n.kind()),
            Some("source_file")
        );
        let name = root
            .snapshot()
            .lang()
            .element_types()
            .field("name")
            .expect("field");
        assert_eq!(
            function.child_by_field(name).map(|n| n.text()),
            Some("alpha")
        );
        assert_eq!(snap.covering(13..22).kind(), "let_declaration");
        assert_eq!(root.preorder().count(), {
            fn count(node: SyntaxNode<'_>) -> usize {
                1 + node.children().map(count).sum::<usize>()
            }
            count(root)
        });
        assert!(snap.errors().is_empty());
        let broken = super::tests::snap("rust", "fn broken( {");
        assert!(!broken.errors().is_empty());
        assert_eq!(broken.diagnostics().is_empty(), broken.errors().is_empty());
    }

    #[test]
    fn injected_fragments_use_host_offsets() {
        let text = "# T\n\n```rust\nfn main() {}\n```\n";
        let snap = snap("markdown", text);
        let at = text.find("main").expect("main") as u32;
        let fragment = snap.fragment_at(at).expect("fragment");
        assert_eq!(fragment.lang().id, "rust");
        let leaf = fragment.snapshot.node_at(at).expect("leaf");
        assert_eq!(leaf.text(), "main");
        assert_eq!(fragment.to_host(at), at);
    }

    #[test]
    fn snapshots_are_send_and_sync() {
        fn check<T: Send + Sync + Clone>() {}
        check::<SyntaxSnapshot>();
    }
}

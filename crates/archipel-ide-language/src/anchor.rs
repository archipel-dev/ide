//! Anchors: persistent references to syntax nodes that survive edits (the
//! `SmartPsiElementPointer` analogue). Tree-sitter nodes are only meaningful
//! inside one tree, so an anchor records what identifies the node (kind,
//! range, name) and finds it again in a later snapshot through the
//! [`EditMap`] of the edits in between.

use crate::element::ElementType;
use crate::ids::{FileId, FileRev, LanguageId};
use crate::node::{SyntaxNode, SyntaxSnapshot};
use archipel_ide_text::EditMap;
use std::ops::Range;

/// Most nodes inspected by the name-based fallback search.
const SEARCH_BUDGET: usize = 20_000;

/// See the module docs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Anchor {
    pub file: FileId,
    /// Language of the node (an injected fragment's language for nodes in
    /// fragments); element types are per language.
    pub lang: LanguageId,
    pub kind: ElementType,
    pub range: Range<u32>,
    /// Hash of the node's declared name, 0 for unnamed nodes.
    pub name_hash: u32,
    /// Revision the anchor was taken at.
    pub rev: FileRev,
}

/// FNV-1a, never 0 (0 means "no name").
pub fn name_hash(name: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in name.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash.max(1)
}

impl Anchor {
    pub fn new(node: SyntaxNode<'_>) -> Self {
        let snap = node.snapshot();
        Self {
            file: snap.file(),
            lang: snap.lang().language_id(),
            kind: node.element_type(),
            range: node.range(),
            name_hash: node.name_node().map_or(0, |name| name_hash(name.text())),
            rev: snap.rev(),
        }
    }

    pub fn is_named(&self) -> bool {
        self.name_hash != 0
    }

    fn matches(&self, node: SyntaxNode<'_>) -> bool {
        node.element_type() == self.kind
            && (!self.is_named()
                || node
                    .name_node()
                    .is_some_and(|name| name_hash(name.text()) == self.name_hash))
    }

    /// The node in `snap` this anchor designates, where `edits` maps offsets
    /// of [`Anchor::rev`] to `snap`'s revision. The result always has the
    /// anchor's kind (and name, for named nodes); `None` when the node is gone.
    ///
    /// Resolution: map the range; accept the node with exactly that range and
    /// kind; otherwise (named nodes only) the nearest node of the same kind
    /// and name inside the closest enclosing node that has one.
    pub fn resolve<'s>(&self, snap: &'s SyntaxSnapshot, edits: &EditMap) -> Option<SyntaxNode<'s>> {
        if snap.file() != self.file {
            return None;
        }
        let mapped = edits.map_range(self.range.start as usize..self.range.end as usize);
        let mapped = mapped.start as u32..mapped.end as u32;
        let snap = if snap.lang().language_id() == self.lang {
            snap
        } else {
            let fragment = snap.fragment_at(mapped.start)?;
            (fragment.lang().language_id() == self.lang).then_some(&fragment.snapshot)?
        };
        if (mapped.end as usize) > snap.text().len() {
            return None;
        }
        let covering = snap.covering(mapped.clone());
        let exact = std::iter::once(covering)
            .chain(covering.ancestors())
            .take_while(|node| node.range() == mapped)
            .find(|node| self.matches(*node));
        if exact.is_some() {
            return exact;
        }
        if !self.is_named() {
            return None;
        }
        let mut budget = SEARCH_BUDGET;
        for scope in std::iter::once(covering).chain(covering.ancestors()) {
            let mut best: Option<SyntaxNode<'s>> = None;
            for node in scope.preorder() {
                if budget == 0 {
                    return best;
                }
                budget -= 1;
                if self.matches(node) {
                    let distance = |n: &SyntaxNode<'_>| n.range().start.abs_diff(mapped.start);
                    if best.is_none_or(|best| distance(&node) < distance(&best)) {
                        best = Some(node);
                    }
                }
            }
            if best.is_some() {
                return best;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_text::{ByteEdit, BytePoint};

    fn parse(text: &str, rev: u64) -> SyntaxSnapshot {
        let lang = crate::registry::by_id("java").expect("java");
        SyntaxSnapshot::parse(FileId(7), FileRev(rev), lang, text.into()).expect("parse")
    }

    fn insert(at: usize, text: &str) -> ByteEdit {
        ByteEdit {
            start_byte: at,
            old_end_byte: at,
            new_end_byte: at + text.len(),
            start_point: BytePoint::default(),
            old_end_point: BytePoint::default(),
            new_end_point: BytePoint::default(),
        }
    }

    #[test]
    fn anchors_follow_edits_and_renames_fail() {
        let before = "class A {\n  void run() {}\n  int size() { return 1; }\n}\n";
        let old = parse(before, 1);
        let method = old
            .root()
            .preorder()
            .find(|node| node.kind() == "method_declaration" && node.text().contains("size"))
            .expect("method");
        let anchor = Anchor::new(method);
        assert!(anchor.is_named());
        assert_eq!(
            anchor.resolve(&old, &EditMap::new()).map(|n| n.text()),
            Some(method.text())
        );
        // Insert a new member before it: the method moves.
        let added = "  int added;\n";
        let after = format!("{}{}{}", &before[..10], added, &before[10..]);
        let new = parse(&after, 2);
        let map = EditMap::from_edits(&[insert(10, added)]);
        let found = anchor.resolve(&new, &map).expect("resolved");
        assert_eq!(found.text(), method.text());
        assert_eq!(
            found.range().start,
            method.range().start + added.len() as u32
        );
        // Without the edit map the name-based fallback still finds it.
        assert_eq!(
            anchor.resolve(&new, &EditMap::new()).map(|n| n.text()),
            Some(method.text())
        );
        // Renaming the method makes it a different element.
        let renamed = parse(&before.replace("size", "count"), 3);
        assert!(anchor.resolve(&renamed, &EditMap::new()).is_none());
        // Other files never resolve.
        let lang = crate::registry::by_id("java").expect("java");
        let other = SyntaxSnapshot::parse(FileId(8), FileRev(1), lang, before.into()).expect("p");
        assert!(anchor.resolve(&other, &EditMap::new()).is_none());
    }

    #[test]
    fn name_hash_is_never_zero() {
        assert_ne!(name_hash(""), 0);
        assert_ne!(name_hash("a"), name_hash("b"));
    }
}

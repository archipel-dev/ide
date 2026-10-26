//! Matching-bracket model over the syntax tree: only real bracket tokens
//! match (never brackets inside strings or comments), and the partner is the
//! complementary token among the same node's children.
//!
//! `(`, `[` and `{` always pair. A language's `brackets.scm` adds the rest
//! with `@open.<kind>` / `@close.<kind>` captures: `block` (the braces of a
//! code block, used by Code Block Start/End), `angle` (generic arguments, never
//! comparison operators) and `tag` (the start and end tag nodes of markup
//! elements, matched through their parent element).

use crate::node::{SyntaxNode, SyntaxSnapshot};
use crate::queryset::{self, QueryKind};
use std::ops::Range;
use tree_sitter::{Node, Tree};

const PAIRS: &[(&str, &str)] = &[("(", ")"), ("[", "]"), ("{", "}"), ("<", ">")];

/// What a pair delimits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PairKind {
    Paren,
    Bracket,
    /// `{}` that is not a code block (object literals, initializers).
    Brace,
    /// `{}` of a code block, class body or similar.
    Block,
    Angle,
    /// A start tag and its end tag.
    Tag,
}

/// Both delimiters of a pair, as the start offsets of their tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pair {
    pub open: usize,
    pub close: usize,
    pub kind: PairKind,
}

impl Pair {
    /// The offsets as `(first, second)` in document order.
    pub fn offsets(&self) -> (usize, usize) {
        (self.open.min(self.close), self.open.max(self.close))
    }
}

fn bracket_token(node: Node<'_>) -> Option<(&'static str, &'static str, bool)> {
    if node.is_named() || node.child_count() > 0 || node.is_missing() {
        return None;
    }
    let kind = node.kind();
    PAIRS.iter().find_map(|&(open, close)| {
        if kind == open {
            Some((open, close, true))
        } else if kind == close {
            Some((open, close, false))
        } else {
            None
        }
    })
}

/// The partner of a bracket token among its siblings (depth counting).
fn sibling_partner(node: Node<'_>, open: &str, close: &str, forward: bool) -> Option<usize> {
    let parent = node.parent()?;
    let mut cursor = parent.walk();
    let siblings: Vec<Node<'_>> = parent.children(&mut cursor).collect();
    let index = siblings
        .iter()
        .position(|sibling| sibling.id() == node.id())?;
    let mut depth = 0usize;
    let ordered: Box<dyn Iterator<Item = &Node<'_>>> = if forward {
        Box::new(siblings[index + 1..].iter())
    } else {
        Box::new(siblings[..index].iter().rev())
    };
    for sibling in ordered {
        if sibling.is_named() || sibling.is_missing() {
            continue;
        }
        let kind = sibling.kind();
        let (same, partner) = if forward {
            (open, close)
        } else {
            (close, open)
        };
        if kind == same {
            depth += 1;
        } else if kind == partner {
            if depth == 0 {
                return Some(sibling.start_byte());
            }
            depth -= 1;
        }
    }
    None
}

/// The bracket token starting at `byte` and its partner, as byte offsets
/// `(open, close)`. `None` when `byte` is not a bracket token or it has no
/// partner (for example inside an ERROR node). Only `()`, `[]` and `{}`; see
/// [`pair_at`] for the language-specific pairs.
pub fn matching_bracket(tree: &Tree, byte: usize) -> Option<(usize, usize)> {
    let node = tree
        .root_node()
        .descendant_for_byte_range(byte, byte.saturating_add(1))?;
    if node.start_byte() != byte {
        return None;
    }
    let (open, close, forward) = bracket_token(node)?;
    if open == "<" {
        return None;
    }
    let other = sibling_partner(node, open, close, forward)?;
    let first = node.start_byte();
    Some((first.min(other), first.max(other)))
}

/// Role of a classified node and the kind named after the capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Open,
    Close,
}

fn parse_kind(suffix: &str) -> Option<PairKind> {
    Some(match suffix {
        "paren" => PairKind::Paren,
        "bracket" => PairKind::Bracket,
        "brace" => PairKind::Brace,
        "block" => PairKind::Block,
        "angle" => PairKind::Angle,
        "tag" => PairKind::Tag,
        _ => return None,
    })
}

/// The `@open.<kind>` / `@close.<kind>` classification of `node` by the
/// language's `brackets.scm` (the last matching pattern wins).
fn classify(snap: &SyntaxSnapshot, node: Node<'_>) -> Option<(Role, PairKind)> {
    let query = queryset::query(snap.lang(), QueryKind::Brackets)?;
    let found = queryset::captures_in(query, snap.tree(), snap.text(), node.byte_range());
    found
        .iter()
        .filter(|captured| captured.node.id() == node.id())
        .filter_map(|captured| {
            let (role, suffix) = if let Some(rest) = captured.name.strip_prefix("open.") {
                (Role::Open, rest)
            } else {
                (Role::Close, captured.name.strip_prefix("close.")?)
            };
            Some((captured.pattern, role, parse_kind(suffix)?))
        })
        .max_by_key(|(pattern, _, _)| *pattern)
        .map(|(_, role, kind)| (role, kind))
}

fn standard_kind(text: &str) -> PairKind {
    match text {
        "(" | ")" => PairKind::Paren,
        "[" | "]" => PairKind::Bracket,
        "<" | ">" => PairKind::Angle,
        _ => PairKind::Brace,
    }
}

fn token_pair(snap: &SyntaxSnapshot, node: Node<'_>) -> Option<Pair> {
    let (open, close, forward) = bracket_token(node)?;
    let classified = classify(snap, node);
    let kind = match (open, classified) {
        // Angle brackets are operators unless the language says otherwise.
        ("<", Some((_, PairKind::Angle))) => PairKind::Angle,
        ("<", _) => return None,
        (_, Some((_, kind))) if kind != PairKind::Angle => kind,
        _ => standard_kind(open),
    };
    let other = sibling_partner(node, open, close, forward)?;
    let here = node.start_byte();
    // Unbalanced siblings (broken code) can pair one-sided: require the
    // partner to point back, so pairs are always symmetric.
    let partner = node
        .parent()?
        .children(&mut node.parent()?.walk())
        .find(|sibling| sibling.start_byte() == other && !sibling.is_named())?;
    if sibling_partner(partner, open, close, !forward) != Some(here) {
        return None;
    }
    Some(if forward {
        Pair {
            open: here,
            close: other,
            kind,
        }
    } else {
        Pair {
            open: other,
            close: here,
            kind,
        }
    })
}

fn tag_pair(snap: &SyntaxSnapshot, leaf: Node<'_>) -> Option<Pair> {
    let mut node = Some(leaf);
    for _ in 0..4 {
        let current = node?;
        if current.byte_range().len() <= 4096
            && let Some((role, PairKind::Tag)) = classify(snap, current)
        {
            let parent = current.parent()?;
            let mut cursor = parent.walk();
            let siblings: Vec<Node<'_>> = parent.children(&mut cursor).collect();
            let wanted = if role == Role::Open {
                Role::Close
            } else {
                Role::Open
            };
            let partner = siblings.iter().rev().find(|sibling| {
                sibling.id() != current.id()
                    && matches!(classify(snap, **sibling), Some((r, PairKind::Tag)) if r == wanted)
            })?;
            let (open, close) = if role == Role::Open {
                (current.start_byte(), partner.start_byte())
            } else {
                (partner.start_byte(), current.start_byte())
            };
            return Some(Pair {
                open,
                close,
                kind: PairKind::Tag,
            });
        }
        node = current.parent();
    }
    None
}

/// The pair whose delimiter token starts at `offset` or, for markup tags,
/// whose start or end tag contains `offset`. Brackets inside strings and
/// comments never pair.
pub fn pair_at(snap: &SyntaxSnapshot, offset: usize) -> Option<Pair> {
    let token = snap
        .tree()
        .root_node()
        .descendant_for_byte_range(offset, offset.saturating_add(1))?;
    if token.start_byte() == offset
        && let Some(pair) = token_pair(snap, token)
    {
        return Some(pair);
    }
    if snap.lang().queries().extra.brackets.is_some() {
        return tag_pair(snap, token);
    }
    None
}

/// Every pair of the document, in order of the opening delimiter.
pub fn all_pairs(snap: &SyntaxSnapshot) -> Vec<Pair> {
    let mut starts: Vec<usize> = Vec::new();
    for node in snap.root().preorder() {
        if bracket_token(node.raw()).is_some() {
            starts.push(node.byte_range().start);
        }
    }
    if let Some(query) = queryset::query(snap.lang(), QueryKind::Brackets) {
        let text = snap.text();
        for captured in queryset::captures_in(query, snap.tree(), text, 0..text.len()) {
            if captured.name.starts_with("open.tag") || captured.name.starts_with("close.tag") {
                starts.push(captured.node.start_byte());
            }
        }
    }
    starts.sort_unstable();
    starts.dedup();
    let mut pairs: Vec<Pair> = starts
        .into_iter()
        .filter_map(|start| pair_at(snap, start))
        .collect();
    pairs.sort_by_key(|pair| (pair.open, pair.close));
    pairs.dedup();
    pairs
}

/// The innermost code block (`@open.block` pair) around `offset`, or the
/// innermost `{}`/`()`/`[]` pair when the language marks none.
pub fn enclosing_block(snap: &SyntaxSnapshot, offset: usize) -> Option<Pair> {
    let root = snap.root();
    let leaf: SyntaxNode<'_> = root.find_leaf_at(u32::try_from(offset).ok()?)?;
    let mut fallback = None;
    for node in std::iter::once(leaf).chain(leaf.ancestors()) {
        let raw = node.raw();
        let (Some(first), Some(last)) =
            (raw.child(0), raw.child(raw.child_count().saturating_sub(1)))
        else {
            continue;
        };
        if first.id() == last.id() {
            continue;
        }
        let Some(pair) = token_pair(snap, first) else {
            continue;
        };
        if pair.close != last.start_byte() || !(pair.open < offset && offset <= pair.close) {
            continue;
        }
        if pair.kind == PairKind::Block {
            return Some(pair);
        }
        if fallback.is_none() && pair.kind == PairKind::Brace {
            fallback = Some(pair);
        }
    }
    fallback
}

/// Range of a pair including both delimiters.
pub fn pair_range(pair: Pair, snap: &SyntaxSnapshot) -> Range<usize> {
    let (open, close) = pair.offsets();
    let end = snap
        .root()
        .find_leaf_at(u32::try_from(close).unwrap_or(0))
        .map_or(close + 1, |leaf| leaf.byte_range().end);
    open..end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SyntaxTree, registry};

    fn pair(id: &str, source: &str, at: usize) -> Option<(usize, usize)> {
        let spec = registry::by_id(id).expect("language");
        let syntax = SyntaxTree::parse(spec, source).expect("parse");
        matching_bracket(syntax.tree(), at)
    }

    fn snap(id: &str, source: &str) -> SyntaxSnapshot {
        let spec = registry::by_id(id).expect("language");
        SyntaxSnapshot::parse(Default::default(), Default::default(), spec, source.into())
            .expect("parse")
    }

    #[test]
    fn brackets_match_structurally_and_skip_strings() {
        let source = "class A { void f(int[] a) { g(\"(\", a[0]); } }\n";
        let open = source.find('{').unwrap();
        let close = source.rfind('}').unwrap();
        assert_eq!(pair("java", source, open), Some((open, close)));
        assert_eq!(pair("java", source, close), Some((open, close)));
        let call = source.find("g(").unwrap() + 1;
        let call_close = source.find(");").unwrap();
        assert_eq!(pair("java", source, call), Some((call, call_close)));
        // The parenthesis inside the string literal is not a bracket token.
        let quoted = source.find("\"(\"").unwrap() + 1;
        assert_eq!(pair("java", source, quoted), None);
        assert_eq!(pair("java", source, 0), None);
    }

    #[test]
    fn kotlin_and_json_brackets() {
        let source = "fun f() { listOf(1, 2) }\n";
        let open = source.find("listOf(").unwrap() + 6;
        assert_eq!(
            pair("kotlin", source, open),
            Some((open, source.find(") }").unwrap()))
        );
        let json = "{\"a\": [1, {\"b\": 2}]}";
        assert_eq!(pair("json", json, 6), Some((6, json.len() - 2)));
    }

    #[test]
    fn angle_brackets_pair_only_in_type_arguments() {
        let source = "fn f(a: Vec<Vec<u8>>) -> bool { 1 < 2 && 3 > 2 }\n";
        let snapshot = snap("rust", source);
        let outer = source.find('<').unwrap();
        let pair = pair_at(&snapshot, outer).expect("generic pair");
        assert_eq!(pair.kind, PairKind::Angle);
        assert_eq!(&source[pair.close..=pair.close], ">");
        assert_eq!(pair.close, source.find(">>").unwrap() + 1);
        let compare = source.find("1 < 2").unwrap() + 2;
        assert_eq!(pair_at(&snapshot, compare), None);
    }

    #[test]
    fn markup_tags_pair_start_and_end_tags() {
        let source = "<div>\n  <p>hi</p>\n</div>\n";
        let snapshot = snap("html", source);
        let pair = pair_at(&snapshot, source.find("div").unwrap()).expect("tag pair");
        assert_eq!(pair.kind, PairKind::Tag);
        assert_eq!(pair.open, 0);
        assert_eq!(pair.close, source.rfind("</div>").unwrap());
        let inner = pair_at(&snapshot, source.find("</p>").unwrap() + 3).expect("inner");
        assert_eq!(inner.open, source.find("<p>").unwrap());
    }

    #[test]
    fn code_block_is_the_innermost_block_pair() {
        let source = "class A {\n    void f() {\n        g(1);\n    }\n}\n";
        let snapshot = snap("java", source);
        let at = source.find("g(1)").unwrap();
        let block = enclosing_block(&snapshot, at).expect("block");
        assert_eq!(block.kind, PairKind::Block);
        assert_eq!(block.open, source.find("() {").unwrap() + 3);
        assert_eq!(block.close, source.find("    }\n}").unwrap() + 4);
    }
}

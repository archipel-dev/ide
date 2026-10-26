//! Matching-bracket model over the syntax tree: only real bracket tokens
//! match (never brackets inside strings or comments), and the partner is the
//! complementary token among the same node's children.

use tree_sitter::{Node, Tree};

const PAIRS: &[(&str, &str)] = &[("(", ")"), ("[", "]"), ("{", "}")];

fn bracket_token(node: Node<'_>) -> Option<(&'static str, &'static str, bool)> {
    if node.is_named() || node.child_count() > 0 {
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

/// The bracket token starting at `byte` and its partner, as byte offsets
/// `(open, close)`. `None` when `byte` is not a bracket token or it has no
/// partner (for example inside an ERROR node).
pub fn matching_bracket(tree: &Tree, byte: usize) -> Option<(usize, usize)> {
    let node = tree
        .root_node()
        .descendant_for_byte_range(byte, byte.saturating_add(1))?;
    if node.start_byte() != byte {
        return None;
    }
    let (open, close, forward) = bracket_token(node)?;
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
        if sibling.is_named() {
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
                let (a, b) = (node.start_byte(), sibling.start_byte());
                return Some((a.min(b), a.max(b)));
            }
            depth -= 1;
        }
    }
    None
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
}

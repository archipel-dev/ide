//! Extend Selection (IntelliJ `EditorSelectWord`) over the syntax tree: each
//! step selects the next larger syntactic unit: the word under the caret,
//! the token, the contents of the enclosing brackets or quotes, the bracketed
//! node, and so on up to the whole file.

use std::ops::Range;
use tree_sitter::{Node, Tree};

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

/// The word around `range` within `bounds` (for text inside comments and strings).
fn word_at(source: &str, range: &Range<usize>, bounds: Range<usize>) -> Option<Range<usize>> {
    let bytes = source.as_bytes();
    let mut start = range.start;
    while start > bounds.start && is_word_byte(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = range.end;
    while end < bounds.end && is_word_byte(bytes[end]) {
        end += 1;
    }
    (start < end && source.is_char_boundary(start) && source.is_char_boundary(end))
        .then_some(start..end)
}

/// The content between a node's first and last anonymous child (brackets,
/// quotes, tag delimiters), with surrounding whitespace trimmed.
fn interior(node: Node<'_>, source: &str) -> Option<Range<usize>> {
    if node.child_count() < 2 {
        return None;
    }
    let first = node.child(0)?;
    let last = node.child(node.child_count() - 1)?;
    if first.is_named() || last.is_named() || first.end_byte() > last.start_byte() {
        return None;
    }
    let inner = source.get(first.end_byte()..last.start_byte())?;
    let leading = inner.len() - inner.trim_start().len();
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return None;
    }
    let start = first.end_byte() + leading;
    Some(start..start + trimmed.len())
}

/// True when a named child of `node` covers `range` (the walk will reach
/// the word through that child instead).
fn has_named_child_at(node: Node<'_>, range: &Range<usize>) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| child.start_byte() <= range.start && range.end <= child.end_byte())
}

fn strictly_contains(outer: &Range<usize>, inner: &Range<usize>) -> bool {
    outer.start <= inner.start && inner.end <= outer.end && outer != inner
}

/// The smallest syntactic range that strictly contains `range` (byte offsets).
/// Returns `None` when the selection already covers the whole file.
pub fn expand(tree: &Tree, source: &str, range: Range<usize>) -> Option<Range<usize>> {
    let range = range.start.min(source.len())..range.end.min(source.len());
    let bytes = source.as_bytes();
    // A caret right after a word belongs to that word, as in IntelliJ.
    let probe = if range.is_empty()
        && range.start > 0
        && is_word_byte(bytes[range.start - 1])
        && bytes
            .get(range.start)
            .is_none_or(|byte| !is_word_byte(*byte))
    {
        range.start - 1..range.start
    } else {
        range.clone()
    };
    let mut node = tree
        .root_node()
        .descendant_for_byte_range(probe.start, probe.end)?;
    loop {
        let node_range = node.byte_range();
        // Words inside comments, strings and other long leaves come first.
        if !has_named_child_at(node, &probe)
            && let Some(word) = word_at(source, &probe, node_range.clone())
            && strictly_contains(&node_range, &word)
            && (word.start <= range.start && range.end <= word.end && word != range)
        {
            return Some(word);
        }
        if let Some(inner) = interior(node, source)
            && strictly_contains(&inner, &range)
        {
            return Some(inner);
        }
        if strictly_contains(&node_range, &range) {
            return Some(node_range);
        }
        node = node.parent()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SyntaxTree, registry};

    fn steps(id: &str, source: &str, caret: &str) -> Vec<String> {
        let spec = registry::by_id(id).expect("language");
        let syntax = SyntaxTree::parse(spec, source).expect("parse");
        let at = source.find(caret).expect("caret marker");
        let mut range = at..at;
        let mut out = Vec::new();
        while let Some(next) = expand(syntax.tree(), source, range.clone()) {
            assert!(next.start <= range.start && range.end <= next.end && next != range);
            out.push(source[next.clone()].to_owned());
            range = next;
        }
        out
    }

    #[test]
    fn java_expands_word_expression_block_and_class() {
        let source = "class A {\n    int f(int x) {\n        return x + 42;\n    }\n}\n";
        let steps = steps("java", source, "42");
        assert_eq!(steps[0], "42");
        assert_eq!(steps[1], "x + 42");
        assert_eq!(steps[2], "return x + 42;");
        assert!(steps.contains(&"int f(int x) {\n        return x + 42;\n    }".to_owned()));
        assert_eq!(steps.last().map(String::as_str), Some(source));
    }

    #[test]
    fn strings_and_comments_select_words_then_contents() {
        let source = "val s = \"hello big world\" // tail note\n";
        let steps = steps("kotlin", source, "big");
        assert_eq!(steps[0], "big");
        assert!(steps.contains(&"hello big world".to_owned()));
        assert!(steps.contains(&"\"hello big world\"".to_owned()));

        let source = "fn f() {} // tail note\n";
        let steps = super::tests::steps("rust", source, "note");
        assert_eq!(steps[0], "note");
        assert_eq!(steps[1], "// tail note");
    }

    #[test]
    fn caret_after_a_word_selects_it_and_bad_ranges_are_clamped() {
        let source = "fn main() { foo(); }\n";
        let spec = registry::by_id("rust").expect("rust");
        let syntax = SyntaxTree::parse(spec, source).expect("parse");
        let at = source.find("foo").unwrap() + 3;
        assert_eq!(expand(syntax.tree(), source, at..at), Some(at - 3..at));
        assert!(expand(syntax.tree(), source, 0..source.len()).is_none());
        let clamped = expand(syntax.tree(), source, 500..900);
        assert!(clamped.is_none_or(|range| range.end <= source.len()));
    }
}

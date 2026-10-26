//! Move Statement Up/Down (IntelliJ `MoveStatementUp/Down`): swaps the
//! statement or declaration at the caret with its neighbor in the same list,
//! keeping the comments and attributes attached to it. Elements are the
//! children of the `@select.group` containers of `textobjects.scm`; only
//! elements that sit on their own lines move, anything else answers `None`
//! and the host falls back to Move Line.

use crate::edit::{Edit, EditSet};
use crate::lines::LineIndex;
use crate::node::SyntaxSnapshot;
use crate::queryset::{self, QueryKind};
use std::ops::Range;
use tree_sitter::Node;

/// An element of a list with the leading comments and attributes that move
/// with it.
#[derive(Debug, Clone)]
struct Element {
    /// Whole range including attached leading siblings.
    range: Range<usize>,
}

fn attached_ids(snap: &SyntaxSnapshot) -> std::collections::HashSet<usize> {
    let mut out = std::collections::HashSet::new();
    if let Some(query) = queryset::query(snap.lang(), QueryKind::Doc) {
        let text = snap.text();
        for captured in queryset::captures_in(query, snap.tree(), text, 0..text.len()) {
            if captured.name == "doc.skip" || captured.name == "doc" {
                out.insert(captured.node.id());
            }
        }
    }
    out
}

fn container_node<'t>(snap: &'t SyntaxSnapshot, range: &Range<usize>) -> Option<Node<'t>> {
    let mut node = snap
        .tree()
        .root_node()
        .descendant_for_byte_range(range.start, range.end)?;
    while node.byte_range() != *range {
        node = node.parent()?;
        if node.byte_range().len() < range.len() {
            return None;
        }
    }
    // The outermost node with this exact range (a wrapper has the same range).
    while let Some(parent) = node.parent() {
        if parent.byte_range() == *range {
            node = parent;
        } else {
            break;
        }
    }
    Some(node)
}

fn elements_of(
    snap: &SyntaxSnapshot,
    container: Node<'_>,
    attached: &std::collections::HashSet<usize>,
    lines: &LineIndex,
) -> Vec<Element> {
    let comments = &snap.lang().token_set_table().comments;
    let is_leading = |node: Node<'_>| {
        comments.contains(crate::ElementType(node.kind_id())) || attached.contains(&node.id())
    };
    let mut cursor = container.walk();
    let children: Vec<Node<'_>> = container.named_children(&mut cursor).collect();
    let mut out: Vec<Element> = Vec::new();
    let mut leading: Vec<Node<'_>> = Vec::new();
    for child in children {
        if is_leading(child) {
            leading.push(child);
            continue;
        }
        // Comments and attributes on the lines directly above move along.
        let mut start = child.start_byte();
        let mut expected = lines.line_of(child.start_byte());
        for node in leading.drain(..).rev() {
            let end_line = lines.line_of(node.end_byte().saturating_sub(1));
            if end_line + 1 >= expected {
                start = node.start_byte();
                expected = lines.line_of(node.start_byte());
            } else {
                break;
            }
        }
        out.push(Element {
            range: start..child.end_byte(),
        });
    }
    out
}

/// Moves the statements covered by `range` one position up or down; `None`
/// when no movable neighbor exists (Move Line then takes over).
pub fn move_statement(snap: &SyntaxSnapshot, range: Range<usize>, up: bool) -> Option<EditSet> {
    let text = snap.text();
    let lines = LineIndex::new(text);
    let range = range.start.min(text.len())..range.end.min(text.len());
    let first_line = lines.line_of(range.start);
    let last_line = lines.line_of(if range.end > range.start {
        range.end - 1
    } else {
        range.start
    });
    let line_span = lines.line_start(first_line)
        ..lines
            .line_range(last_line, text)
            .end
            .max(lines.line_start(first_line));
    let attached = attached_ids(snap);
    let groups = crate::textobjects::groups_around(snap, &line_span);
    for group in groups {
        let Some(container) = container_node(snap, &group.container) else {
            continue;
        };
        let elements = elements_of(snap, container, &attached, &lines);
        let line_of_start = |e: &Element| lines.line_of(e.range.start);
        let line_of_end = |e: &Element| lines.line_of(e.range.end.saturating_sub(1));
        // Elements touched by the selection's lines.
        let touched: Vec<usize> = elements
            .iter()
            .enumerate()
            .filter(|(_, e)| line_of_start(e) <= last_line && line_of_end(e) >= first_line)
            .map(|(index, _)| index)
            .collect();
        let (Some(&i), Some(&j)) = (touched.first(), touched.last()) else {
            continue;
        };
        // Every element must sit on its own lines.
        let alone = |index: usize| {
            let e = &elements[index];
            let before = index.checked_sub(1).map(|k| line_of_end(&elements[k]));
            let after = elements.get(index + 1).map(&line_of_start);
            before.is_none_or(|line| line < line_of_start(e))
                && after.is_none_or(|line| line > line_of_end(e))
        };
        if !(i..=j).all(alone) {
            continue;
        }
        let neighbor = if up {
            i.checked_sub(1)
        } else {
            (j + 1 < elements.len()).then_some(j + 1)
        };
        let Some(neighbor) = neighbor else { continue };
        if !alone(neighbor) {
            continue;
        }
        let block = elements[i].range.start..elements[j].range.end;
        let other = elements[neighbor].range.clone();
        let (first, second) = if up {
            (&other, &block)
        } else {
            (&block, &other)
        };
        // The pieces start at their line start and end after the element (a
        // trailing `,` stays in place; a trailing comment moves along).
        let piece_end = |end: usize| {
            let line_end = lines
                .line_range(lines.line_of(end.saturating_sub(1)), text)
                .end;
            let tail = &text[end..line_end];
            let after_comma = tail.trim_start().strip_prefix(',').unwrap_or(tail);
            if after_comma.trim().is_empty() {
                end
            } else {
                line_end
            }
        };
        let first_text_start = lines.line_start(lines.line_of(first.start));
        let first_text_end = piece_end(first.end);
        let second_text_start = lines.line_start(lines.line_of(second.start));
        let second_text_end = piece_end(second.end);
        if first_text_end > second_text_start {
            continue;
        }
        let a = &text[first_text_start..first_text_end];
        let gap = &text[first_text_end..second_text_start];
        let b = &text[second_text_start..second_text_end];
        let swapped = format!("{b}{gap}{a}");
        let (block_old_start, block_new_start) = if up {
            (second_text_start, first_text_start)
        } else {
            (first_text_start, first_text_start + b.len() + gap.len())
        };
        let map = |offset: usize| (offset + block_new_start).saturating_sub(block_old_start);
        let selection = (range.start != range.end).then(|| map(range.start)..map(range.end));
        return Some(EditSet {
            edits: vec![Edit::replace(first_text_start..second_text_end, swapped)],
            caret: selection.is_none().then(|| map(range.start)),
            selection,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(id: &str, source: &str) -> SyntaxSnapshot {
        let spec = crate::registry::by_id(id).expect("language");
        SyntaxSnapshot::parse(Default::default(), Default::default(), spec, source.into())
            .expect("parse")
    }

    fn moved(lang: &str, source: &str, up: bool) -> Option<String> {
        let at = source.find('|').expect("caret");
        let text = source.replacen('|', "", 1);
        let snapshot = snap(lang, &text);
        let set = move_statement(&snapshot, at..at, up)?;
        let mut out = set.apply(&text);
        out.insert(set.caret?, '|');
        Some(out)
    }

    #[test]
    fn statements_swap_with_their_neighbors_keeping_the_caret() {
        let source =
            "class A {\n    void f() {\n        a();\n        b|();\n        c();\n    }\n}\n";
        assert_eq!(
            moved("java", source, true).as_deref(),
            Some(
                "class A {\n    void f() {\n        b|();\n        a();\n        c();\n    }\n}\n"
            )
        );
        assert_eq!(
            moved("java", source, false).as_deref(),
            Some(
                "class A {\n    void f() {\n        a();\n        c();\n        b|();\n    }\n}\n"
            )
        );
        // Nothing above the first statement: fall back to Move Line.
        let first = "class A {\n    void f() {\n        a|();\n        b();\n    }\n}\n";
        assert!(moved("java", first, true).is_none());
    }

    #[test]
    fn methods_move_with_their_doc_comments() {
        let source = "class A {\n    /** One. */\n    void one() {}\n\n    /** Two. */\n    void tw|o() {}\n}\n";
        assert_eq!(
            moved("java", source, true).as_deref(),
            Some(
                "class A {\n    /** Two. */\n    void tw|o() {}\n\n    /** One. */\n    void one() {}\n}\n"
            )
        );
    }
}

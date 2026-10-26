//! Incrementally maintained syntax tree for one document.

use crate::highlight::{self, HighlightLayer};
use crate::registry::LanguageSpec;
use archipel_ide_text::{ByteEdit, BytePoint};
use std::ops::Range;
use tree_sitter::{InputEdit, Parser, Point, Tree};

/// A parsed document: the language and its current Tree-sitter tree.
#[derive(Clone)]
pub struct SyntaxTree {
    spec: &'static LanguageSpec,
    tree: Tree,
}

impl std::fmt::Debug for SyntaxTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyntaxTree")
            .field("language", &self.spec.id)
            .field("root", &self.tree.root_node().kind())
            .finish()
    }
}

fn parser(spec: &LanguageSpec) -> Option<Parser> {
    let mut parser = Parser::new();
    parser.set_language(&spec.grammar()).ok()?;
    Some(parser)
}

fn point(point: BytePoint) -> Point {
    Point::new(point.row, point.column)
}

fn input_edit(edit: &ByteEdit) -> InputEdit {
    InputEdit {
        start_byte: edit.start_byte,
        old_end_byte: edit.old_end_byte,
        new_end_byte: edit.new_end_byte,
        start_position: point(edit.start_point),
        old_end_position: point(edit.old_end_point),
        new_end_position: point(edit.new_end_point),
    }
}

/// Maps a byte range in pre-edit coordinates through `edit`.
pub(crate) fn map_range(range: Range<usize>, edit: &ByteEdit) -> Range<usize> {
    let map = |offset: usize| {
        if offset <= edit.start_byte {
            offset
        } else if offset >= edit.old_end_byte {
            offset + edit.new_end_byte - edit.old_end_byte
        } else {
            edit.new_end_byte
        }
    };
    map(range.start)..map(range.end).max(map(range.start))
}

/// Collects the line ranges of ERROR nodes, descending only into erroneous subtrees.
fn error_ranges(node: tree_sitter::Node<'_>, source: &str, ranges: &mut Vec<Range<usize>>) {
    if !node.has_error() {
        return;
    }
    if node.is_error() {
        ranges.push(line_range(source, node.byte_range()));
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        error_ranges(child, source, ranges);
    }
}

/// Grows a byte range to the start of its first line and the end of its last.
pub(crate) fn line_range(source: &str, range: Range<usize>) -> Range<usize> {
    let bytes = source.as_bytes();
    let end = range.end.min(bytes.len());
    let start = range.start.min(end);
    let line_start = bytes[..start]
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |index| index + 1);
    let line_end = bytes[end..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(bytes.len(), |index| end + index);
    line_start..line_end
}

/// Sorts and merges overlapping or touching ranges.
pub(crate) fn merge_ranges(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    ranges.sort_by_key(|range| range.start);
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
    for range in ranges {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    merged
}

impl SyntaxTree {
    /// Parses `source` from scratch.
    pub fn parse(spec: &'static LanguageSpec, source: &str) -> Option<Self> {
        let tree = parser(spec)?.parse(source, None)?;
        Some(Self { spec, tree })
    }

    pub fn language(&self) -> &'static LanguageSpec {
        self.spec
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// Applies buffer edits and reparses `source` (the post-edit text) reusing
    /// the old tree. Returns the byte ranges whose highlighting may have
    /// changed (edited text plus syntactically changed ranges), or `None` if
    /// reparsing failed and the caller should rebuild from scratch.
    pub fn apply_edits(&mut self, source: &str, edits: &[ByteEdit]) -> Option<Vec<Range<usize>>> {
        let mut edited: Vec<Range<usize>> = Vec::with_capacity(edits.len());
        for edit in edits {
            self.tree.edit(&input_edit(edit));
            for range in &mut edited {
                *range = map_range(range.clone(), edit);
            }
            edited.push(edit.start_byte..edit.new_end_byte);
        }
        // Tokens touching an edit (an identifier shortened by a deletion) can
        // change their captures, so edited ranges cover whole lines.
        for range in &mut edited {
            *range = line_range(source, range.clone());
        }
        let tree = parser(self.spec)?.parse(source, Some(&self.tree))?;
        edited.extend(
            self.tree
                .changed_ranges(&tree)
                .map(|range| range.start_byte..range.end_byte),
        );
        // `changed_ranges` does not see children relabelled inside an ERROR
        // node that itself kept its extent, so error regions are re-highlighted.
        error_ranges(tree.root_node(), source, &mut edited);
        self.tree = tree;
        Some(
            merge_ranges(edited)
                .into_iter()
                .map(|range| range.start.min(source.len())..range.end.min(source.len()))
                .collect(),
        )
    }

    /// Highlights `range` of `source` with the language's highlight query.
    pub fn highlight(&self, source: &str, range: Range<usize>) -> HighlightLayer {
        highlight::highlight(self.spec, &self.tree, source, range).unwrap_or_default()
    }

    /// Re-highlights `range` grown to whole captures; see
    /// [`highlight::highlight_covering`].
    pub fn highlight_covering(
        &self,
        source: &str,
        range: Range<usize>,
    ) -> (Range<usize>, HighlightLayer) {
        highlight::highlight_covering(self.spec, &self.tree, source, range.clone())
            .unwrap_or((range, HighlightLayer::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_in_a_large_file_reparses_a_small_region() {
        let mut source = String::from("public class Big {\n");
        for i in 0..2000 {
            source.push_str(&format!("    int field{i} = {i}; // c\n"));
        }
        source.push_str("}\n");
        let spec = crate::registry::by_id("java").expect("java");
        let mut syntax = SyntaxTree::parse(spec, &source).expect("parse");
        let at = source.len() / 2;
        let edit = ByteEdit {
            start_byte: at,
            old_end_byte: at,
            new_end_byte: at + 1,
            start_point: BytePoint {
                row: source[..at].matches('\n').count(),
                column: at - source[..at].rfind('\n').map_or(0, |i| i + 1),
            },
            old_end_point: BytePoint::default(),
            new_end_point: BytePoint::default(),
        };
        let edit = ByteEdit {
            old_end_point: edit.start_point,
            new_end_point: BytePoint {
                column: edit.start_point.column + 1,
                ..edit.start_point
            },
            ..edit
        };
        source.insert(at, 'x');
        let ranges = syntax.apply_edits(&source, &[edit]).expect("reparse");
        let touched: usize = ranges.iter().map(|range| range.len()).sum();
        assert!(touched < 200, "re-highlight {touched} bytes: {ranges:?}");
        assert_eq!(
            syntax.tree().root_node().to_sexp(),
            SyntaxTree::parse(spec, &source)
                .expect("parse")
                .tree()
                .root_node()
                .to_sexp()
        );
    }

    #[test]
    fn ranges_map_through_edits_and_merge() {
        let edit = ByteEdit {
            start_byte: 4,
            old_end_byte: 6,
            new_end_byte: 9,
            start_point: BytePoint::default(),
            old_end_point: BytePoint::default(),
            new_end_point: BytePoint::default(),
        };
        assert_eq!(map_range(0..2, &edit), 0..2);
        assert_eq!(map_range(6..8, &edit), 9..11);
        assert_eq!(map_range(5..7, &edit), 9..10);
        assert_eq!(merge_ranges(vec![5..7, 0..2, 2..3, 6..9]), vec![0..3, 5..9]);
        assert_eq!(line_range("ab\ncd\nef", 4..4), 3..5);
        assert_eq!(line_range("ab\ncd\nef", 1..7), 0..8);
        assert_eq!(line_range("ab", 9..12), 0..2);
    }
}

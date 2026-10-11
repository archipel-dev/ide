//! Maps document lines to visual rows when code-folding regions are collapsed,
//! and paints gutter fold markers.

use archipel_kit::Theme;
use archipel_kit::icons::{self, Glyph};
use archipel_kit::parts::row_pill;
use archipel_kit::tokens::size::ROW_PILL_INSET;
use eframe::egui::{self, Color32, Rect};
use jetthing_core::Document;

/// A collapsed region as rendered on its start row.
#[derive(Debug, Clone, PartialEq)]
pub struct CollapsedRow {
    /// Character offset where the placeholder replaces the text.
    pub start_char: usize,
    /// Line and character offset where visible text resumes.
    pub end_line: usize,
    pub end_char: usize,
    pub placeholder: String,
}

/// Visual layout of a document with collapsed regions.
#[derive(Debug, Clone, Default)]
pub struct FoldView {
    /// Hidden document lines as inclusive ranges, sorted and disjoint.
    hidden: Vec<(usize, usize)>,
    /// Collapsed regions keyed by their (visible) start line.
    collapsed: Vec<(usize, CollapsedRow)>,
}

impl FoldView {
    pub fn new(document: &Document) -> Self {
        let mut view = Self::default();
        let text = &document.text;
        let len_bytes = text.char_to_byte(text.len_chars());
        for fold in document.folds.collapsed() {
            if fold.end_byte > len_bytes || fold.start_byte >= fold.end_byte {
                continue;
            }
            let start_char = text.byte_to_char(fold.start_byte);
            let end_char = text.byte_to_char(fold.end_byte);
            let start_line = text.char_to_line(start_char);
            let end_line = text.char_to_line(end_char);
            if end_line <= start_line
                || view.is_hidden(start_line)
                || view.collapsed_on(start_line).is_some()
            {
                continue;
            }
            view.hidden.push((start_line + 1, end_line));
            view.collapsed.push((
                start_line,
                CollapsedRow {
                    start_char,
                    end_line,
                    end_char,
                    placeholder: fold.placeholder.clone(),
                },
            ));
        }
        view
    }

    pub fn is_hidden(&self, line: usize) -> bool {
        self.hidden
            .iter()
            .any(|&(first, last)| first <= line && line <= last)
    }

    pub fn collapsed_on(&self, line: usize) -> Option<&CollapsedRow> {
        self.collapsed
            .iter()
            .find(|(start, _)| *start == line)
            .map(|(_, row)| row)
    }

    /// Hidden lines before `line`.
    fn hidden_before(&self, line: usize) -> usize {
        self.hidden
            .iter()
            .map(|&(first, last)| {
                if line <= first {
                    0
                } else {
                    line.min(last + 1) - first
                }
            })
            .sum()
    }

    pub fn row_count(&self, line_count: usize) -> usize {
        line_count - self.hidden_before(line_count)
    }

    /// Visual row of a document line (a hidden line maps to its region's row).
    pub fn line_to_row(&self, line: usize) -> usize {
        let line = self
            .hidden
            .iter()
            .find(|&&(first, last)| first <= line && line <= last)
            .map_or(line, |&(first, _)| first - 1);
        line - self.hidden_before(line)
    }

    /// Document line shown on a visual row.
    pub fn row_to_line(&self, row: usize) -> usize {
        let mut line = row;
        for &(first, last) in &self.hidden {
            if line >= first {
                line += last + 1 - first;
            } else {
                break;
            }
        }
        line
    }
}

/// Width and height of the gutter fold marker's hover pill.
pub const MARKER_SIZE: f32 = 16.0;

/// Paints the gutter fold marker: the kit chevron (`›` collapsed, `⌄` expanded) on a hover
/// pill. Colors are the kit's editor tokens.
pub fn paint_marker(
    painter: &egui::Painter,
    kit: &Theme,
    center: egui::Pos2,
    collapsed: bool,
    hovered: bool,
) {
    let rect = Rect::from_center_size(center, egui::vec2(MARKER_SIZE, MARKER_SIZE));
    let color = if hovered {
        kit.colors.text
    } else {
        kit.editor.line_number_caret
    };
    if hovered {
        // `row_pill` insets by the row pill inset on both sides.
        row_pill(
            painter,
            rect.expand2(egui::vec2(ROW_PILL_INSET, 0.0)),
            kit.colors.hover,
        );
    }
    let glyph = if collapsed {
        Glyph::ChevronRight
    } else {
        Glyph::ChevronDown
    };
    icons::paint(painter, rect.shrink(2.0), glyph, color);
}

/// Colors of a collapsed region's placeholder box: (background, text), brighter under the
/// pointer.
pub fn placeholder_colors(kit: &Theme, hovered: bool) -> (Color32, Color32) {
    if hovered {
        (kit.colors.pressed, kit.colors.text)
    } else {
        (kit.editor.matched_brace, kit.editor.line_number_caret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(hidden: &[(usize, usize)]) -> FoldView {
        FoldView {
            hidden: hidden.to_vec(),
            collapsed: Vec::new(),
        }
    }

    #[test]
    fn rows_skip_hidden_lines() {
        // Lines 2..=4 and 8..=8 hidden in a 12-line document.
        let view = view(&[(2, 4), (8, 8)]);
        assert_eq!(view.row_count(12), 8);
        let rows: Vec<_> = (0..8).map(|row| view.row_to_line(row)).collect();
        assert_eq!(rows, vec![0, 1, 5, 6, 7, 9, 10, 11]);
        for (row, line) in rows.iter().enumerate() {
            assert_eq!(view.line_to_row(*line), row);
        }
        assert_eq!(
            view.line_to_row(3),
            1,
            "hidden lines map to their region row"
        );
        assert!(view.is_hidden(4) && !view.is_hidden(5));
    }

    #[test]
    fn documents_build_views_from_collapsed_regions() {
        let source = "class A {\n    void f() {\n        g();\n    }\n}\n";
        let mut document = Document::new("A.java".into(), source);
        assert!(document.toggle_fold_at_line(1));
        let view = FoldView::new(&document);
        assert_eq!(view.row_count(document.text.len_lines()), 4);
        let row = view.collapsed_on(1).expect("collapsed row");
        assert_eq!(row.placeholder, "{ g(); }");
        assert_eq!(row.end_line, 3);
        assert_eq!(&source[row.start_char..row.start_char + 1], "{");
    }
}

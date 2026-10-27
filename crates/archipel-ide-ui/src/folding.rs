//! Maps document lines to visual rows when code-folding regions are collapsed,
//! and paints gutter fold markers.

use archipel_ide_core::Document;
use eframe::egui::{self, Color32, Stroke};

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
        let folds = document.folds.collapsed();
        let mut index = 0;
        while index < folds.len() {
            let fold = &folds[index];
            index += 1;
            if fold.end_byte > len_bytes || fold.start_byte >= fold.end_byte {
                continue;
            }
            let start_char = text.byte_to_char(fold.start_byte);
            let mut end_char = text.byte_to_char(fold.end_byte);
            let mut placeholder = fold.placeholder.clone();
            // The halves of a grouped region (a one-line method) are one placeholder.
            while fold.group != 0
                && index < folds.len()
                && folds[index].group == fold.group
                && folds[index].end_byte <= len_bytes
            {
                let next = &folds[index];
                let between = text
                    .slice(end_char, text.byte_to_char(next.start_byte))
                    .unwrap_or_default();
                placeholder.push_str(&between);
                placeholder.push_str(&next.placeholder);
                end_char = text.byte_to_char(next.end_byte);
                index += 1;
            }
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
                    placeholder,
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
    #[cfg(test)]
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

/// Paints an IntelliJ-style chevron: `›` for a collapsed region, `⌄` for an
/// expanded one.
pub fn paint_marker(painter: &egui::Painter, center: egui::Pos2, collapsed: bool, color: Color32) {
    let size = 3.5;
    let stroke = Stroke::new(1.3_f32, color);
    let points = if collapsed {
        vec![
            center + egui::vec2(-size / 2.0, -size),
            center + egui::vec2(size / 2.0, 0.0),
            center + egui::vec2(-size / 2.0, size),
        ]
    } else {
        vec![
            center + egui::vec2(-size, -size / 2.0),
            center + egui::vec2(0.0, size / 2.0),
            center + egui::vec2(size, -size / 2.0),
        ]
    };
    painter.add(egui::Shape::line(points, stroke));
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
        // The two halves of a one-line method form one placeholder that starts behind `)`.
        assert_eq!(row.placeholder, " { g(); }");
        assert_eq!(row.end_line, 3);
        assert_eq!(&source[row.start_char..row.start_char + 1], " ");
    }
}

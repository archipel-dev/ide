//! Multi-caret and column (block) selection actions.
//!
//! These only change the [`SelectionSet`]; they never edit text, so they add no
//! undo steps and produce no edit notifications.

use crate::ops::{self, FindOptions, is_word, line_bounds, selection_lines};
use crate::{Selection, SelectionSet, TextDocument};

/// Word range at `position` if the caret touches a word character.
fn word_range(doc: &TextDocument, position: usize) -> Option<(usize, usize)> {
    let (start, end) = ops::word_at(doc, position);
    let is_word_range = start < end
        && doc
            .slice(start, end)
            .is_ok_and(|text| text.chars().all(is_word));
    is_word_range.then_some((start, end))
}

/// Expands every bare caret that touches a word to that word. Returns `true`
/// when at least one caret was expanded (the first press of Alt+J / ⌃G).
fn expand_carets_to_words(doc: &TextDocument, selections: &mut SelectionSet) -> bool {
    let mut expanded = false;
    let next: Vec<Selection> = selections
        .selections()
        .iter()
        .map(|selection| {
            if selection.is_caret()
                && let Some((start, end)) = word_range(doc, selection.head)
            {
                expanded = true;
                return Selection {
                    anchor: start,
                    head: end,
                };
            }
            *selection
        })
        .collect();
    if expanded {
        let occurrences = std::mem::take(&mut selections.occurrences);
        selections.set_all(next);
        selections.occurrences = occurrences;
        selections.occurrence_whole_words = true;
    }
    expanded
}

/// The selection the occurrence search continues from: the most recently added
/// occurrence that still exists, otherwise the last selection in the document.
fn latest_selection(selections: &SelectionSet) -> Selection {
    selections
        .occurrences
        .iter()
        .rev()
        .find(|s| selections.selections.contains(s))
        .copied()
        .unwrap_or_else(|| *selections.selections.last().unwrap_or(&Selection::caret(0)))
}

fn occurrence_matches(doc: &TextDocument, needle: &str, whole_words: bool) -> Vec<(usize, usize)> {
    ops::find_all(
        &doc.text(),
        needle,
        FindOptions {
            match_case: true,
            whole_words,
            regex: false,
        },
    )
}

/// Add Selection for Next Occurrence (Alt+J / ⌃G). The first press on a bare caret
/// selects the word under it; each further press adds the next case-sensitive
/// match of the selected text after the latest occurrence, wrapping at the end
/// of the file. Returns `false` when nothing could be added.
pub fn select_next_occurrence(doc: &TextDocument, selections: &mut SelectionSet) -> bool {
    if selections.selections().iter().any(|s| s.is_caret()) {
        if selections.selections().len() == 1 {
            selections.occurrences.clear();
        }
        return expand_carets_to_words(doc, selections);
    }
    let latest = latest_selection(selections);
    let Ok(needle) = doc.slice(latest.start(), latest.end()) else {
        return false;
    };
    if selections.selections().len() == 1 {
        // A fresh search from an explicit selection matches substrings.
        let whole_word = selections.occurrence_whole_words
            && word_range(doc, latest.start()) == Some((latest.start(), latest.end()));
        selections.occurrence_whole_words = whole_word;
    }
    let matches = occurrence_matches(doc, &needle, selections.occurrence_whole_words);
    let taken = |(start, end): (usize, usize)| {
        selections
            .selections()
            .iter()
            .any(|s| s.start() < end && start < s.end())
    };
    let next = matches
        .iter()
        .copied()
        .filter(|m| m.0 >= latest.end())
        .chain(matches.iter().copied().filter(|m| m.0 < latest.end()))
        .find(|m| !taken(*m));
    let Some((start, end)) = next else {
        return false;
    };
    let added = Selection {
        anchor: start,
        head: end,
    };
    selections.add(added);
    selections.occurrences.push(added);
    true
}

/// Unselect Occurrence (Alt+Shift+J / ⌃⇧G): removes the most recently added
/// occurrence. Returns `false` when only one selection remains.
pub fn unselect_occurrence(selections: &mut SelectionSet) -> bool {
    if selections.selections().len() < 2 {
        // With one caret only the selection goes (ED-CARET-076).
        let only = selections.primary();
        if only.is_caret() {
            return false;
        }
        selections.set_single(Selection::caret(only.head));
        return true;
    }
    let target = loop {
        match selections.occurrences.pop() {
            Some(candidate) if selections.selections.contains(&candidate) => break candidate,
            Some(_) => continue,
            None => break *selections.selections.last().unwrap_or(&Selection::caret(0)),
        }
    };
    let remaining: Vec<Selection> = selections
        .selections()
        .iter()
        .copied()
        .filter(|s| *s != target)
        .collect();
    let occurrences = std::mem::take(&mut selections.occurrences);
    selections.set_all(remaining);
    selections.occurrences = occurrences;
    true
}

/// Select All Occurrences (Ctrl+Alt+Shift+J / ⌃⌘G): one selection per match of the
/// selected text, or of the word at a bare caret (whole words). Returns the
/// number of selections.
pub fn select_all_occurrences(doc: &TextDocument, selections: &mut SelectionSet) -> usize {
    let primary = selections.primary();
    let (range, whole_words) = if primary.is_caret() {
        match word_range(doc, primary.head) {
            Some(range) => (range, true),
            None => return selections.selections().len(),
        }
    } else {
        let range = (primary.start(), primary.end());
        let whole = selections.occurrence_whole_words && word_range(doc, range.0) == Some(range);
        (range, whole)
    };
    let Ok(needle) = doc.slice(range.0, range.1) else {
        return selections.selections().len();
    };
    let matches = occurrence_matches(doc, &needle, whole_words);
    if matches.is_empty() {
        return selections.selections().len();
    }
    selections.set_single(Selection {
        anchor: range.0,
        head: range.1,
    });
    let all: Vec<Selection> = matches
        .into_iter()
        .map(|(start, end)| Selection {
            anchor: start,
            head: end,
        })
        .collect();
    selections.set_all(all);
    selections.occurrence_whole_words = whole_words;
    selections.selections().len()
}

/// Clone Caret Above/Below (double Ctrl + ↑/↓ / double ⌥ + ↑/↓): adds a caret on
/// the adjacent line at the same column (clamped to the line end) for every
/// caret. Returns `false` when no new caret fits inside the document.
pub fn clone_caret(doc: &TextDocument, selections: &mut SelectionSet, above: bool) -> bool {
    let before = selections.selections().len();
    let last_line = doc.len_lines().saturating_sub(1);
    let mut added = Vec::new();
    for selection in selections.selections() {
        let Ok((line, column)) = doc.char_to_line_column(selection.head) else {
            continue;
        };
        let target = if above {
            line.checked_sub(1)
        } else {
            (line < last_line).then_some(line + 1)
        };
        if let Some(target) = target {
            let (start, end, _) = line_bounds(doc, target);
            added.push(Selection::caret((start + column).min(end)));
        }
    }
    let mut all = selections.selections().to_vec();
    all.extend(added);
    let occurrences = std::mem::take(&mut selections.occurrences);
    selections.set_all(all);
    selections.occurrences = occurrences;
    selections.selections().len() > before
}

/// Add Carets to Ends of Selected Lines (Alt+Shift+G / ⌥⇧G).
pub fn carets_at_line_ends(doc: &TextDocument, selections: &mut SelectionSet) {
    let carets: Vec<Selection> = selections
        .selections()
        .iter()
        .flat_map(|selection| {
            let (first, last) = selection_lines(doc, *selection);
            (first..=last).map(|line| Selection::caret(line_bounds(doc, line).1))
        })
        .collect();
    selections.set_single(Selection::caret(0));
    selections.set_all(carets);
}

/// A line/column point used by block selections. Columns are character columns
/// and may lie beyond the end of a short line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockPoint {
    pub line: usize,
    pub column: usize,
}

/// Rectangular selection between two points: one selection per line, each
/// clamped to its line (no virtual space).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockSelection {
    pub anchor: BlockPoint,
    pub head: BlockPoint,
}

impl BlockSelection {
    pub fn at(point: BlockPoint) -> Self {
        Self {
            anchor: point,
            head: point,
        }
    }

    /// Point for a character offset.
    pub fn point(doc: &TextDocument, offset: usize) -> BlockPoint {
        let (line, column) = doc
            .char_to_line_column(offset.min(doc.len_chars()))
            .unwrap_or((0, 0));
        BlockPoint { line, column }
    }

    /// The per-line selections, ordered from the anchor line to the head line
    /// (the selection set re-sorts them by offset).
    pub fn selections(&self, doc: &TextDocument) -> Vec<Selection> {
        let last_line = doc.len_lines().saturating_sub(1);
        let (from, to) = (
            self.anchor.line.min(last_line),
            self.head.line.min(last_line),
        );
        let lines: Vec<usize> = if from <= to {
            (from..=to).collect()
        } else {
            (to..=from).rev().collect()
        };
        lines
            .into_iter()
            .map(|line| {
                let (start, end, _) = line_bounds(doc, line);
                Selection {
                    anchor: (start + self.anchor.column).min(end),
                    head: (start + self.head.column).min(end),
                }
            })
            .collect()
    }

    /// Replaces `selections` with this block and remembers it so later column
    /// mode movement keeps the intended columns on short lines.
    pub fn apply(&self, doc: &TextDocument, selections: &mut SelectionSet) {
        selections.set_single(Selection::caret(0));
        selections.set_all(self.selections(doc));
        selections.block = Some(*self);
    }

    /// Moves the head by whole lines and columns, saturating at the document start.
    pub fn moved(&self, doc: &TextDocument, lines: isize, columns: isize) -> Self {
        let last_line = doc.len_lines().saturating_sub(1);
        let line = self.head.line.saturating_add_signed(lines).min(last_line);
        Self {
            anchor: self.anchor,
            head: BlockPoint {
                line,
                column: self.head.column.saturating_add_signed(columns),
            },
        }
    }
}

impl SelectionSet {
    /// Column selection mode (Alt+Shift+Insert / ⌘⇧8): Shift+arrows and mouse
    /// drags create rectangular selections.
    pub fn column_mode(&self) -> bool {
        self.column_mode
    }

    pub fn set_column_mode(&mut self, enabled: bool) {
        self.column_mode = enabled;
        if !enabled {
            self.block = None;
        }
    }

    /// The remembered block if it still describes the current selections;
    /// otherwise a fresh block at the primary caret.
    pub fn current_block(&self, doc: &TextDocument) -> BlockSelection {
        if let Some(block) = self.block {
            let mut expected = SelectionSet::default();
            expected.set_all(block.selections(doc));
            if expected.selections() == self.selections() {
                return block;
            }
        }
        let primary = self.primary();
        BlockSelection {
            anchor: BlockSelection::point(doc, primary.anchor),
            head: BlockSelection::point(doc, primary.head),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(doc: &TextDocument, selections: &SelectionSet) -> Vec<String> {
        selections
            .selections()
            .iter()
            .map(|s| doc.slice(s.start(), s.end()).unwrap())
            .collect()
    }

    #[test]
    fn next_occurrence_selects_word_then_adds_matches_and_wraps() {
        let doc = TextDocument::new("foo food foo\nfoo");
        let mut selections = SelectionSet::single(10);
        assert!(select_next_occurrence(&doc, &mut selections));
        assert_eq!(
            selections.selections(),
            &[Selection {
                anchor: 9,
                head: 12
            }]
        );
        // Whole words: `food` is skipped; continues after the latest, then wraps.
        assert!(select_next_occurrence(&doc, &mut selections));
        assert!(select_next_occurrence(&doc, &mut selections));
        assert_eq!(selections.selections().len(), 3);
        assert_eq!(selections.selections()[0], Selection { anchor: 0, head: 3 });
        assert!(!select_next_occurrence(&doc, &mut selections), "all taken");
        assert!(unselect_occurrence(&mut selections));
        assert_eq!(
            selections.selections(),
            &[
                Selection {
                    anchor: 9,
                    head: 12
                },
                Selection {
                    anchor: 13,
                    head: 16
                }
            ]
        );
        assert!(unselect_occurrence(&mut selections));
        // The last selection collapses to a caret (ED-CARET-076), then nothing is left to do.
        assert!(unselect_occurrence(&mut selections));
        assert!(!unselect_occurrence(&mut selections));
    }

    #[test]
    fn next_occurrence_from_selection_matches_substrings_case_sensitively() {
        let doc = TextDocument::new("ab Ab xaby ab");
        let mut selections = SelectionSet::default();
        selections.set_single(Selection { anchor: 0, head: 2 });
        assert!(select_next_occurrence(&doc, &mut selections));
        assert!(select_next_occurrence(&doc, &mut selections));
        assert_eq!(texts(&doc, &selections), ["ab", "ab", "ab"]);
        assert_eq!(selections.selections()[1].start(), 7);
    }

    #[test]
    fn select_all_occurrences_of_word_or_selection() {
        let doc = TextDocument::new("x xy x\nx");
        let mut selections = SelectionSet::single(0);
        assert_eq!(select_all_occurrences(&doc, &mut selections), 3);
        let mut selections = SelectionSet::default();
        selections.set_single(Selection { anchor: 0, head: 1 });
        assert_eq!(select_all_occurrences(&doc, &mut selections), 4);
        let mut selections = SelectionSet::single(1);
        let doc = TextDocument::new("  ");
        assert_eq!(select_all_occurrences(&doc, &mut selections), 1);
    }

    #[test]
    fn clones_carets_vertically_with_clamped_columns() {
        let doc = TextDocument::new("abcd\nab\nabcd");
        let mut selections = SelectionSet::single(3);
        assert!(clone_caret(&doc, &mut selections, false));
        assert!(clone_caret(&doc, &mut selections, false));
        assert_eq!(
            selections.selections(),
            &[
                Selection::caret(3),
                Selection::caret(7),
                Selection::caret(10)
            ]
        );
        assert!(!clone_caret(&doc, &mut selections, false), "at last line");
        let mut selections = SelectionSet::single(1);
        assert!(!clone_caret(&doc, &mut selections, true), "at first line");
    }

    #[test]
    fn carets_at_ends_of_selected_lines() {
        let doc = TextDocument::new("a\nbb\nccc\n");
        let mut selections = SelectionSet::default();
        selections.set_single(Selection { anchor: 0, head: 6 });
        carets_at_line_ends(&doc, &mut selections);
        assert_eq!(
            selections.selections(),
            &[
                Selection::caret(1),
                Selection::caret(4),
                Selection::caret(8)
            ]
        );
    }

    #[test]
    fn block_selection_clamps_short_lines_and_keeps_columns() {
        let doc = TextDocument::new("abcdef\nab\nabcdef");
        let block = BlockSelection {
            anchor: BlockPoint { line: 0, column: 1 },
            head: BlockPoint { line: 2, column: 4 },
        };
        let mut selections = SelectionSet::default();
        block.apply(&doc, &mut selections);
        assert_eq!(texts(&doc, &selections), ["bcd", "b", "bcd"]);
        assert_eq!(selections.current_block(&doc), block);
        let shrunk = selections.current_block(&doc).moved(&doc, -1, 1);
        shrunk.apply(&doc, &mut selections);
        assert_eq!(texts(&doc, &selections), ["bcde", "b"]);
        // Upwards blocks keep the anchor line.
        let up = shrunk.moved(&doc, -5, 0);
        assert_eq!(up.head.line, 0);
        selections.set_single(Selection::caret(2));
        assert_eq!(
            selections.current_block(&doc),
            BlockSelection::at(BlockPoint { line: 0, column: 2 })
        );
    }
}

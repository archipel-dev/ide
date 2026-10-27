//! Text transformations: Join Lines, Toggle Case, Sort/Reverse Lines, Split Line
//! and Start New Line Before. Each mutating action is one undo step.

use crate::ops::{line_bounds, selection_lines};
use crate::{Selection, SelectionSet, TextChange, TextDocument, TextError};

fn line_text(doc: &TextDocument, line: usize) -> String {
    let (start, end, _) = line_bounds(doc, line);
    doc.slice(start, end).unwrap_or_default()
}

fn indentation(text: &str) -> &str {
    let width = text.len() - text.trim_start_matches([' ', '\t']).len();
    &text[..width]
}

/// Number of lines, not counting the empty line after a final newline.
fn real_line_count(doc: &TextDocument) -> usize {
    let count = doc.len_lines();
    if count > 1 && line_text(doc, count - 1).is_empty() {
        count - 1
    } else {
        count
    }
}

/// Text that results from joining `next` onto `current` and the separator used.
fn join_pair(current: &str, next: &str, line_comment: Option<&str>) -> (String, String) {
    let mut rest = next.trim_start_matches([' ', '\t']);
    let current_trimmed = current.trim_end_matches([' ', '\t']);
    // Comment continuation: `// a` + `// b` -> `// a b`.
    if let Some(prefix) = line_comment
        && current_trimmed.trim_start().starts_with(prefix)
        && let Some(stripped) = rest.strip_prefix(prefix)
    {
        rest = stripped.trim_start_matches([' ', '\t']);
    }
    let no_space = current_trimmed.is_empty()
        || rest.is_empty()
        || current_trimmed.ends_with(['(', '['])
        || rest.starts_with([')', ']', ',', ';', '.']);
    let separator = if no_space { "" } else { " " };
    (current_trimmed.to_owned(), format!("{separator}{rest}"))
}

/// Join Lines (Ctrl+Shift+J / ⌃⇧J). A caret joins its line with the next one; a
/// multi-line selection joins all of its lines. Indentation of the joined line
/// collapses to one space (none next to brackets/punctuation), and line-comment
/// prefixes of continued comments are removed. Carets land at the last join.
pub fn join_lines(
    doc: &mut TextDocument,
    selections: &mut SelectionSet,
    line_comment: Option<&str>,
) -> Result<(), TextError> {
    let real_lines = real_line_count(doc);
    let mut joins: Vec<usize> = selections
        .selections()
        .iter()
        .flat_map(|selection| {
            let (first, last) = selection_lines(doc, *selection);
            let last = if last > first { last - 1 } else { first };
            first..=last
        })
        .filter(|line| line + 1 < real_lines)
        .collect();
    joins.sort_unstable();
    joins.dedup();
    if joins.is_empty() {
        return Ok(());
    }
    let mut groups: Vec<(usize, usize)> = Vec::new();
    for line in joins {
        match groups.last_mut() {
            Some(group) if group.1 + 1 == line => group.1 = line,
            _ => groups.push((line, line)),
        }
    }
    let mut changes = Vec::new();
    let mut carets = Vec::new();
    let mut shift: isize = 0;
    let selection_lines_of: Vec<(usize, usize)> = selections
        .selections()
        .iter()
        .filter(|s| !s.is_caret())
        .map(|s| selection_lines(doc, *s))
        .collect();
    for (first, last) in groups {
        let from_selection = selection_lines_of
            .iter()
            .any(|(a, b)| *a <= last + 1 && first <= *b);
        let start = line_bounds(doc, first).0;
        let end = line_bounds(doc, last + 1).1;
        let mut joined = line_text(doc, first);
        let mut caret = 0;
        for line in first + 1..=last + 1 {
            let (head, tail) = join_pair(&joined, &line_text(doc, line), line_comment);
            caret = head.chars().count();
            joined = head + &tail;
        }
        let new_start = start.saturating_add_signed(shift);
        // A joined selection stays selected (ED-LINE-020); a caret lands at the last join.
        carets.push(if from_selection {
            Selection {
                anchor: new_start,
                head: new_start + joined.chars().count(),
            }
        } else {
            Selection::caret(new_start + caret)
        });
        shift += joined.chars().count() as isize - (end - start) as isize;
        changes.push(TextChange {
            start_char: start,
            end_char: end,
            inserted: joined,
        });
    }
    doc.edit_many(changes)?;
    selections.set_single(Selection::caret(0));
    selections.set_all(carets);
    Ok(())
}

/// The conversion Toggle Case applies to `texts`: all lower case becomes upper case, anything
/// else lower case; one direction for every selection (ED-LINE-050).
fn converts_to_lower(texts: &[String]) -> bool {
    texts.iter().any(|text| text.to_lowercase() != *text)
}

/// IntelliJ's toggle rule for one text: all-lowercase text becomes uppercase, anything else
/// becomes lowercase.
pub fn toggled_case(text: &str) -> String {
    if converts_to_lower(&[text.to_owned()]) {
        text.to_lowercase()
    } else {
        text.to_uppercase()
    }
}

/// Toggle Case (Ctrl+Shift+U / ⌘⇧U) on each selection, or on the word at a bare caret, which
/// ends up selected. Selections keep covering the converted text.
pub fn toggle_case(doc: &mut TextDocument, selections: &mut SelectionSet) -> Result<(), TextError> {
    let ranges: Vec<(usize, usize)> = selections
        .selections()
        .iter()
        .map(|selection| {
            if selection.is_caret() {
                crate::ops::word_at(doc, selection.head)
            } else {
                (selection.start(), selection.end())
            }
        })
        .collect();
    let mut texts = Vec::new();
    for &(start, end) in &ranges {
        texts.push(doc.slice(start, end)?);
    }
    let lower = converts_to_lower(&texts);
    let mut changes = Vec::new();
    let mut next = Vec::new();
    let mut shift: isize = 0;
    for ((selection, (start, end)), text) in selections.selections().iter().zip(ranges).zip(texts) {
        let converted = if lower {
            text.to_lowercase()
        } else {
            text.to_uppercase()
        };
        let new_len = converted.chars().count();
        let new_start = start.saturating_add_signed(shift);
        next.push(if selection.head >= selection.anchor {
            Selection {
                anchor: new_start,
                head: new_start + new_len,
            }
        } else {
            Selection {
                anchor: new_start + new_len,
                head: new_start,
            }
        });
        shift += new_len as isize - (end - start) as isize;
        changes.push(TextChange {
            start_char: start,
            end_char: end,
            inserted: converted,
        });
    }
    doc.edit_many(changes)?;
    selections.set_all(next);
    Ok(())
}

/// Lines targeted by Sort/Reverse Lines: the primary selection's lines, or the
/// whole file when nothing is selected.
fn target_lines(doc: &TextDocument, selections: &SelectionSet) -> (usize, usize) {
    let primary = selections.primary();
    if selections.selections().len() == 1 && primary.is_caret() {
        (0, real_line_count(doc).saturating_sub(1))
    } else {
        selection_lines(doc, primary)
    }
}

/// Rewrites the target lines. `reorder` returns, for every output line, the index of the input
/// line it comes from. Carets follow their line; selections keep their offsets.
fn rewrite_lines(
    doc: &mut TextDocument,
    selections: &mut SelectionSet,
    reorder: impl FnOnce(&[String]) -> Vec<usize>,
) -> Result<(), TextError> {
    let (first, last) = target_lines(doc, selections);
    let start = line_bounds(doc, first).0;
    let end = line_bounds(doc, last).1;
    let lines: Vec<String> = (first..=last).map(|line| line_text(doc, line)).collect();
    let separator = if doc.slice(start, doc.len_chars())?.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let order = reorder(&lines);
    let output: Vec<&str> = order.iter().map(|&i| lines[i].as_str()).collect();
    let before = selections.selections().to_vec();
    let ages = selections.ages().to_vec();
    let line_starts: Vec<usize> = (first..=last)
        .map(|line| line_bounds(doc, line).0)
        .collect();
    let joined = output.join(separator);
    let mut new_starts = Vec::with_capacity(output.len());
    let mut at = start;
    for line in &output {
        new_starts.push(at);
        at += line.chars().count() + separator.chars().count();
    }
    doc.edit(start, end, &joined)?;
    let len = doc.len_chars();
    let next: Vec<Selection> = before
        .iter()
        .map(|selection| {
            if !selection.is_caret() || selection.head < start || selection.head > end {
                return *selection;
            }
            // The caret stays on its line: find the line it was on and where that line went.
            let line = line_starts
                .iter()
                .rposition(|&s| s <= selection.head)
                .unwrap_or(0);
            let column = selection.head - line_starts[line];
            let moved = order
                .iter()
                .position(|&source| source == line)
                .or_else(|| {
                    // A removed duplicate: its first equal line.
                    order
                        .iter()
                        .position(|&source| lines[source] == lines[line])
                })
                .unwrap_or(line);
            Selection::caret(
                (new_starts[moved] + column).min(new_starts[moved] + output[moved].chars().count()),
            )
        })
        .collect();
    selections.set_all_aged(next, ages);
    selections.clamp(len);
    Ok(())
}

/// Sort Lines: sorts the target lines lexicographically (by code point).
pub fn sort_lines(doc: &mut TextDocument, selections: &mut SelectionSet) -> Result<(), TextError> {
    rewrite_lines(doc, selections, |lines| {
        let mut order: Vec<usize> = (0..lines.len()).collect();
        order.sort_by(|&a, &b| lines[a].cmp(&lines[b]).then(a.cmp(&b)));
        order
    })
}

/// Reverse Lines: reverses the order of the target lines.
pub fn reverse_lines(
    doc: &mut TextDocument,
    selections: &mut SelectionSet,
) -> Result<(), TextError> {
    rewrite_lines(doc, selections, |lines| (0..lines.len()).rev().collect())
}

/// Delete Duplicate Lines (ED-LINE-052): keeps the first of equal target lines.
pub fn unique_lines(
    doc: &mut TextDocument,
    selections: &mut SelectionSet,
) -> Result<(), TextError> {
    rewrite_lines(doc, selections, |lines| {
        let mut seen = std::collections::HashSet::new();
        (0..lines.len())
            .filter(|&i| seen.insert(lines[i].clone()))
            .collect()
    })
}

/// Split Line (Ctrl+Enter / ⌘↩): breaks the line at each caret, indenting the
/// new line like the current one, while the caret stays before the break.
pub fn split_line(doc: &mut TextDocument, selections: &mut SelectionSet) -> Result<(), TextError> {
    let mut changes = Vec::new();
    let mut next = Vec::new();
    let mut shift: isize = 0;
    for selection in selections.selections() {
        let line = doc.char_to_line(selection.start());
        let text = line_text(doc, line);
        let inserted = format!("\n{}", indentation(&text));
        next.push(Selection::caret(
            selection.start().saturating_add_signed(shift),
        ));
        shift += inserted.chars().count() as isize - (selection.end() - selection.start()) as isize;
        changes.push(TextChange {
            start_char: selection.start(),
            end_char: selection.end(),
            inserted,
        });
    }
    doc.edit_many(changes)?;
    selections.set_all(next);
    Ok(())
}

/// Start New Line Before Current (Ctrl+Alt+Enter / ⌘⌥↩): opens an indented line
/// above each caret line and moves the caret onto it.
pub fn start_new_line_before(
    doc: &mut TextDocument,
    selections: &mut SelectionSet,
) -> Result<(), TextError> {
    let mut lines: Vec<usize> = selections
        .selections()
        .iter()
        .map(|selection| doc.char_to_line(selection.head))
        .collect();
    lines.sort_unstable();
    lines.dedup();
    let mut changes = Vec::new();
    let mut next = Vec::new();
    let mut shift = 0usize;
    for line in lines {
        let start = line_bounds(doc, line).0;
        let indent = indentation(&line_text(doc, line)).to_owned();
        let indent_len = indent.chars().count();
        next.push(Selection::caret(start + shift + indent_len));
        shift += indent_len + 1;
        changes.push(TextChange {
            start_char: start,
            end_char: start,
            inserted: format!("{indent}\n"),
        });
    }
    doc.edit_many(changes)?;
    selections.set_single(Selection::caret(0));
    selections.set_all(next);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(text: &str, selection: Selection) -> (TextDocument, SelectionSet) {
        let mut selections = SelectionSet::default();
        selections.set_single(selection);
        (TextDocument::new(text), selections)
    }

    #[test]
    fn joins_line_with_next_collapsing_indentation() {
        let (mut doc, mut sel) = setup("let x =   \n    1 + 2;\nnext", Selection::caret(2));
        join_lines(&mut doc, &mut sel, Some("//")).unwrap();
        assert_eq!(doc.text(), "let x = 1 + 2;\nnext");
        assert_eq!(sel.primary(), Selection::caret(7));
        assert!(doc.undo());
        assert_eq!(doc.text(), "let x =   \n    1 + 2;\nnext");
    }

    #[test]
    fn joins_selected_lines_brackets_and_comments() {
        let (mut doc, mut sel) = setup(
            "call(\n    a,\n    b\n)\n",
            Selection {
                anchor: 0,
                head: 20,
            },
        );
        join_lines(&mut doc, &mut sel, None).unwrap();
        assert_eq!(doc.text(), "call(a, b)\n");
        let (mut doc, mut sel) = setup("    // one\n    // two", Selection::caret(0));
        join_lines(&mut doc, &mut sel, Some("//")).unwrap();
        assert_eq!(doc.text(), "    // one two");
        // Last line: nothing to join.
        let (mut doc, mut sel) = setup("a\nb\n", Selection::caret(2));
        join_lines(&mut doc, &mut sel, None).unwrap();
        assert_eq!(doc.text(), "a\nb\n");
        assert!(!doc.can_undo());
        // Empty next line joins without a space.
        let (mut doc, mut sel) = setup("a\n\nb", Selection::caret(0));
        join_lines(&mut doc, &mut sel, None).unwrap();
        assert_eq!(doc.text(), "a\nb");
    }

    #[test]
    fn joins_at_multiple_carets_in_one_undo_step() {
        let (mut doc, mut sel) = setup("a\nb\nc\nd", Selection::caret(0));
        sel.add(Selection::caret(4));
        join_lines(&mut doc, &mut sel, None).unwrap();
        assert_eq!(doc.text(), "a b\nc d");
        assert_eq!(
            sel.selections(),
            &[Selection::caret(1), Selection::caret(5)]
        );
        assert!(doc.undo());
        assert_eq!(doc.text(), "a\nb\nc\nd");
    }

    #[test]
    fn toggles_case_of_selection_or_word() {
        assert_eq!(toggled_case("abc"), "ABC");
        assert_eq!(toggled_case("Abc"), "abc");
        assert_eq!(toggled_case("straße"), "STRASSE");
        let (mut doc, mut sel) = setup("foo Bar", Selection::caret(1));
        toggle_case(&mut doc, &mut sel).unwrap();
        assert_eq!(doc.text(), "FOO Bar");
        // The word at a bare caret ends up selected (ED-LINE-050).
        assert_eq!(sel.primary(), Selection { anchor: 0, head: 3 });
        sel.set_single(Selection { anchor: 7, head: 4 });
        toggle_case(&mut doc, &mut sel).unwrap();
        assert_eq!(doc.text(), "FOO bar");
        assert_eq!(sel.primary(), Selection { anchor: 7, head: 4 });
        let (mut doc, mut sel) = setup("ß x", Selection { anchor: 0, head: 1 });
        sel.add(Selection { anchor: 2, head: 3 });
        toggle_case(&mut doc, &mut sel).unwrap();
        assert_eq!(doc.text(), "SS X");
        assert_eq!(sel.selections()[1], Selection { anchor: 3, head: 4 });
    }

    #[test]
    fn sorts_and_reverses_selected_lines_or_whole_file() {
        let (mut doc, mut sel) = setup("c\na\nb\n", Selection::caret(0));
        sort_lines(&mut doc, &mut sel).unwrap();
        assert_eq!(doc.text(), "a\nb\nc\n");
        reverse_lines(&mut doc, &mut sel).unwrap();
        assert_eq!(doc.text(), "c\nb\na\n");
        let (mut doc, mut sel) = setup("z\ny\nx\nw", Selection { anchor: 2, head: 6 });
        sort_lines(&mut doc, &mut sel).unwrap();
        assert_eq!(doc.text(), "z\nx\ny\nw");
        let (mut doc, mut sel) = setup("b\r\na\r\n", Selection::caret(0));
        sort_lines(&mut doc, &mut sel).unwrap();
        assert_eq!(doc.text(), "a\r\nb\r\n");
    }

    #[test]
    fn split_line_and_start_new_line_before() {
        let (mut doc, mut sel) = setup("    ab", Selection::caret(5));
        split_line(&mut doc, &mut sel).unwrap();
        assert_eq!(doc.text(), "    a\n    b");
        assert_eq!(sel.primary(), Selection::caret(5));
        let (mut doc, mut sel) = setup("x\n  y", Selection::caret(4));
        start_new_line_before(&mut doc, &mut sel).unwrap();
        assert_eq!(doc.text(), "x\n  \n  y");
        assert_eq!(sel.primary(), Selection::caret(4));
    }
}

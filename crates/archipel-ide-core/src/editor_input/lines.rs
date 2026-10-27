//! Line commands: indent, comment, duplicate, transpose, cut to line boundary, auto-indent
//! (ED-TYPE-081 to ED-TYPE-083, ED-LINE-002, ED-LINE-040 to ED-LINE-053, ED-CLIP-006).

use super::indent;
use super::{
    Bias, CaretEdit, EditorOptions, RegionKind, View, apply_caret_edits, apply_changes,
    map_through, view_of,
};
use crate::Document;
use archipel_ide_text::{Selection, TextChange, UndoKind};

fn is_blank(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// The lines a selection touches; a non-empty selection ending at a line start excludes that
/// line (ED-TYPE-081).
fn lines_of(view: &View, sel: Selection) -> (usize, usize) {
    let first = view.line_start(sel.start());
    let mut last = view.line_start(sel.end());
    if !sel.is_caret() && last > first && last == sel.end() {
        last = view.line_start(last - 1);
    }
    (first, last)
}

/// Starts of all lines touched by the selections, ascending and distinct.
fn line_starts(view: &View, selections: &[Selection]) -> Vec<usize> {
    let mut starts = Vec::new();
    for sel in selections {
        let (first, last) = lines_of(view, *sel);
        let mut at = first;
        loop {
            starts.push(at);
            if at >= last {
                break;
            }
            at = view.line_end(at) + 1;
        }
    }
    starts.sort_unstable();
    starts.dedup();
    starts
}

/// Moves a selection through `changes`: its start stays in front of text inserted at it, its
/// end moves behind; a caret moves with the text after it.
fn map_selection(changes: &[TextChange], sel: Selection) -> Selection {
    if sel.is_caret() {
        let at = map_through(changes, sel.head, Bias::Right);
        return Selection::caret(at);
    }
    let start = map_through(changes, sel.start(), Bias::Left);
    let end = map_through(changes, sel.end(), Bias::Right);
    if sel.head >= sel.anchor {
        Selection {
            anchor: start,
            head: end,
        }
    } else {
        Selection {
            anchor: end,
            head: start,
        }
    }
}

/// Indent Selection / Indent Line or Selection: one unit at the start of every touched line.
pub(crate) fn indent_lines(doc: &mut Document, opts: &EditorOptions) -> Result<bool, String> {
    let view = view_of(doc);
    let unit = indent::unit(&view, opts);
    let selections = doc.selections.selections().to_vec();
    let starts = line_starts(&view, &selections);
    let multi = selections
        .iter()
        .any(|s| lines_of(&view, *s).0 != lines_of(&view, *s).1);
    let changes: Vec<TextChange> = starts
        .iter()
        .filter(|&&start| !(multi && view.line_end(start) == start))
        .map(|&start| TextChange {
            start_char: start,
            end_char: start,
            inserted: unit.text.clone(),
        })
        .collect();
    let ages = doc.selections.ages().to_vec();
    let result = apply_changes(doc, UndoKind::Other, changes, |changes, _| {
        selections
            .iter()
            .map(|s| map_selection(changes, *s))
            .collect()
    });
    let _ = ages;
    result
}

/// Unindent Line or Selection (ED-TYPE-082): at most one unit of leading blanks goes.
pub(crate) fn unindent_lines(doc: &mut Document, opts: &EditorOptions) -> Result<bool, String> {
    let view = view_of(doc);
    let unit = indent::unit(&view, opts);
    let width = unit.width.max(1);
    let selections = doc.selections.selections().to_vec();
    let starts = line_starts(&view, &selections);
    let changes: Vec<TextChange> = starts
        .iter()
        .filter_map(|&start| {
            let mut end = start;
            if view.at(start) == Some('\t') {
                end = start + 1;
            } else {
                while end < start + width && view.at(end) == Some(' ') {
                    end += 1;
                }
            }
            (end > start).then(|| TextChange {
                start_char: start,
                end_char: end,
                inserted: String::new(),
            })
        })
        .collect();
    apply_changes(doc, UndoKind::Other, changes, |changes, _| {
        selections
            .iter()
            .map(|s| map_selection(changes, *s))
            .collect()
    })
}

/// Tab at a caret without selection: spaces up to the next indent stop, or a tab character
/// (ED-TYPE-080).
pub(crate) fn tab_at_caret(doc: &mut Document, opts: &EditorOptions) -> Result<bool, String> {
    let view = view_of(doc);
    let unit = indent::unit(&view, opts);
    let edits: Vec<CaretEdit> = doc
        .selections
        .selections()
        .iter()
        .map(|sel| {
            let start = sel.start();
            let column = start - view.line_start(start);
            let text = if unit.text == "\t" {
                "\t".to_owned()
            } else {
                " ".repeat(unit.width - column % unit.width)
            };
            let caret = text.chars().count();
            CaretEdit::new(start, sel.end(), text, caret)
        })
        .collect();
    super::apply_caret_edits(doc, UndoKind::Tab, edits)
}

/// Whether Tab indents the selection (a non-blank selection) rather than inserting a tab step.
pub(crate) fn selection_has_text(doc: &Document) -> bool {
    let view = view_of(doc);
    doc.selections.selections().iter().any(|s| {
        !s.is_caret()
            && view.chars[s.start()..s.end()]
                .iter()
                .any(|c| !c.is_whitespace())
    })
}

/// What the comment command does to one line.
struct LineEdit {
    start: usize,
    end: usize,
    text: String,
}

/// Comment with Line Comment (ED-LINE-040 to ED-LINE-042).
pub(crate) fn comment_line(doc: &mut Document, opts: &EditorOptions) -> Result<bool, String> {
    let view = view_of(doc);
    let Some(prefix) = view.lang.line_comment else {
        return Ok(false);
    };
    // Languages with `#` comments follow their own style: a space after the prefix, at the indent.
    let script = view.lang.family == super::Family::Script;
    let (at_first_column, with_space) = if script {
        (false, true)
    } else {
        (opts.line_comment_first_column, opts.line_comment_space)
    };
    let selections = doc.selections.selections().to_vec();
    let starts = line_starts(&view, &selections);
    let comment_at = |start: usize| -> Option<usize> {
        // The offset of the comment prefix on the line, when the line is commented.
        let first = view.first_non_blank(start);
        let text = view.text(first, first + prefix.chars().count());
        (text == prefix).then_some(first)
    };
    let non_blank: Vec<usize> = starts
        .iter()
        .copied()
        .filter(|&s| !view.is_blank_line(s))
        .collect();
    let all_commented = !non_blank.is_empty() && non_blank.iter().all(|&s| comment_at(s).is_some());
    let mut edits: Vec<LineEdit> = Vec::new();
    for &start in &starts {
        let blank = view.is_blank_line(start);
        if all_commented {
            let Some(at) = comment_at(start) else {
                continue;
            };
            let mut end = at + prefix.chars().count();
            if with_space && view.at(end) == Some(' ') {
                end += 1;
            }
            edits.push(LineEdit {
                start: at,
                end,
                text: String::new(),
            });
        } else {
            if blank && starts.len() > 1 {
                continue;
            }
            let at = if at_first_column {
                start
            } else {
                view.first_non_blank(start).min(view.line_end(start))
            };
            let text = if with_space && !blank {
                format!("{prefix} ")
            } else {
                prefix.to_owned()
            };
            edits.push(LineEdit {
                start: at,
                end: at,
                text,
            });
        }
    }
    if edits.is_empty() {
        return Ok(false);
    }
    let changes: Vec<TextChange> = edits
        .iter()
        .map(|e| TextChange {
            start_char: e.start,
            end_char: e.end,
            inserted: e.text.clone(),
        })
        .collect();
    let move_down = opts.comment_moves_caret
        && selections.len() == 1
        && selections[0].is_caret()
        && !view.is_blank_line(view.line_start(selections[0].head));
    let move_to = if move_down {
        let head = selections[0].head;
        let column = head - view.line_start(head);
        let end = view.line_end(head);
        (end < view.len()).then(|| (end + 1, column))
    } else {
        None
    };
    apply_changes(doc, UndoKind::Other, changes, |changes, _| {
        if let Some((next_line, column)) = move_to {
            let start = map_through(changes, next_line, Bias::Left);
            let line_end = view.line_end(next_line);
            let line_end = map_through(changes, line_end, Bias::Right);
            return vec![Selection::caret((start + column).min(line_end))];
        }
        // The caret on a line that gets a comment prefix at its start stays after it.
        selections
            .iter()
            .map(|s| map_selection(changes, *s))
            .collect()
    })
}

/// Comment with Block Comment (ED-LINE-044).
pub(crate) fn comment_block(doc: &mut Document, opts: &EditorOptions) -> Result<bool, String> {
    let view = view_of(doc);
    let Some((open, close)) = view.lang.block_comment else {
        return Ok(false);
    };
    let space = if opts.block_comment_space { " " } else { "" };
    let selections = doc.selections.selections().to_vec();
    let sel = *selections.last().unwrap_or(&Selection::caret(0));
    // Inside a block comment: remove its delimiters.
    if let Some(region) = view.inside(sel.start())
        && matches!(region.kind, RegionKind::BlockComment)
        && region.closed
        && sel.end() <= region.end - close.len()
    {
        let open_end = region.start + open.chars().count();
        let close_start = region.end - close.chars().count();
        let mut inner_start = open_end;
        let mut inner_end = close_start;
        if opts.block_comment_space {
            if view.at(inner_start) == Some(' ') {
                inner_start += 1;
            }
            if inner_end > inner_start && view.at(inner_end - 1) == Some(' ') {
                inner_end -= 1;
            }
        }
        let changes = vec![
            TextChange {
                start_char: region.start,
                end_char: inner_start,
                inserted: String::new(),
            },
            TextChange {
                start_char: inner_end,
                end_char: region.end,
                inserted: String::new(),
            },
        ];
        return apply_changes(doc, UndoKind::Other, changes, |changes, _| {
            selections
                .iter()
                .map(|s| Selection {
                    anchor: map_through(changes, s.anchor, Bias::Right),
                    head: map_through(changes, s.head, Bias::Right),
                })
                .collect()
        });
    }
    if sel.is_caret() {
        let text = format!("{open}{space}{space}{close}");
        let caret = open.chars().count() + space.len();
        let edit = CaretEdit::new(sel.head, sel.head, text, caret);
        return apply_caret_edits(doc, UndoKind::Other, vec![edit]);
    }
    let changes = vec![
        TextChange {
            start_char: sel.start(),
            end_char: sel.start(),
            inserted: format!("{open}{space}"),
        },
        TextChange {
            start_char: sel.end(),
            end_char: sel.end(),
            inserted: format!("{space}{close}"),
        },
    ];
    apply_changes(doc, UndoKind::Other, changes, |changes, _| {
        selections
            .iter()
            .map(|s| map_selection(changes, *s))
            .collect()
    })
}

/// Duplicate Entire Lines (ED-LINE-002): the lines of the selection are copied below and the
/// copy is selected.
pub(crate) fn duplicate_lines(doc: &mut Document) -> Result<bool, String> {
    let view = view_of(doc);
    let selections = doc.selections.selections().to_vec();
    let mut changes = Vec::new();
    let mut next = Vec::new();
    let mut shift = 0usize;
    for sel in &selections {
        let (first, last) = lines_of(&view, *sel);
        let end = (view.line_end(last) + 1).min(view.len());
        let mut text = view.text(first, end);
        let missing_break = !text.ends_with('\n');
        if missing_break {
            text.insert(0, '\n');
        }
        let added = text.chars().count();
        changes.push(TextChange {
            start_char: end,
            end_char: end,
            inserted: text,
        });
        let copy_start = end + shift + usize::from(missing_break);
        next.push(Selection {
            anchor: copy_start,
            head: end + shift + added + usize::from(missing_break) * 0,
        });
        shift += added;
    }
    apply_changes(doc, UndoKind::Other, changes, |_, _| next)
}

/// Transpose Characters (ED-LINE-051).
pub(crate) fn transpose(doc: &mut Document) -> Result<bool, String> {
    let view = view_of(doc);
    let edits: Vec<CaretEdit> = doc
        .selections
        .selections()
        .iter()
        .map(|sel| {
            let head = sel.head;
            let line_start = view.line_start(head);
            let line_end = view.line_end(head);
            if head == 0 || line_end - line_start < 2 {
                return CaretEdit::move_to(head, head);
            }
            // At the line end the last two characters swap; elsewhere the two around the caret.
            let (left, right) = if head >= line_end {
                (head - 2, head)
            } else if head == line_start {
                return CaretEdit::move_to(head, head);
            } else {
                (head - 1, head + 1)
            };
            let a = view.chars[left];
            let b = view.chars[left + 1];
            CaretEdit::new(
                left,
                right,
                format!("{b}{a}"),
                right - left + (head - left).saturating_sub(right - left) + 0,
            )
            .with_caret_at(head - left + usize::from(head < line_end))
        })
        .collect();
    apply_caret_edits(doc, UndoKind::Other, edits)
}

impl CaretEdit {
    fn with_caret_at(mut self, caret: usize) -> Self {
        self.select = (caret, caret);
        self
    }
}

/// Cut up to Line End and Cut Line Backward (ED-CLIP-006): the text goes to the clipboard.
pub(crate) fn cut_to_line_boundary(doc: &mut Document, forward: bool) -> Result<String, String> {
    let view = view_of(doc);
    let mut cut = Vec::new();
    let edits: Vec<CaretEdit> = doc
        .selections
        .selections()
        .iter()
        .map(|sel| {
            let head = sel.head;
            let (start, end) = if forward {
                let end = view.line_end(head);
                // At the line end (or before blanks only) the line break goes.
                if view.chars[head..end].iter().all(|c| is_blank(*c)) && end < view.len() {
                    (head, end + 1)
                } else {
                    (head, end)
                }
            } else {
                let start = view.line_start(head);
                if start == head && head > 0 {
                    (head - 1, head)
                } else {
                    (start, head)
                }
            };
            cut.push(view.text(start, end));
            CaretEdit::new(start, end, "", 0)
        })
        .collect();
    apply_caret_edits(doc, UndoKind::Other, edits)?;
    Ok(cut.join("\n"))
}

/// Auto-Indent Lines (ED-LINE-053): every touched line gets the indent the code style gives it.
pub(crate) fn auto_indent(doc: &mut Document, opts: &EditorOptions) -> Result<bool, String> {
    let view = view_of(doc);
    if !indent::has_formatter(&view) {
        return Ok(false);
    }
    let selections = doc.selections.selections().to_vec();
    let starts = line_starts(&view, &selections);
    let unit = indent::unit(&view, opts);
    let mut changes = Vec::new();
    for &start in &starts {
        if view.is_blank_line(start) {
            continue;
        }
        let first = view.first_non_blank(start);
        let wanted = indent::absolute(&view, &unit, start);
        let current = view.text(start, first);
        if wanted != current {
            changes.push(TextChange {
                start_char: start,
                end_char: first,
                inserted: wanted,
            });
        }
    }
    apply_changes(doc, UndoKind::Other, changes, |changes, _| {
        selections
            .iter()
            .map(|s| Selection {
                anchor: map_through(changes, s.anchor, Bias::Right),
                head: map_through(changes, s.head, Bias::Right),
            })
            .collect()
    })
}

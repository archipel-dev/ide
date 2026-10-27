//! Enter and its relatives (ED-TYPE-060 to ED-TYPE-071, ED-TYPE-090 to ED-TYPE-094).

use super::indent;
use super::{CaretEdit, EditorOptions, RegionKind, View, apply_caret_edits, view_of};
use crate::Document;
use archipel_ide_text::{Selection, UndoKind};

fn is_blank(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// The blanks of the current line before `limit`.
fn leading_blanks(view: &View, line_start: usize, limit: usize) -> String {
    view.chars[line_start..limit.min(view.len())]
        .iter()
        .take_while(|c| is_blank(**c))
        .collect()
}

/// The edit that replaces `start..end` (the selection) by a line break with the indent the
/// code style and the comment around the caret ask for.
pub(crate) fn newline_edit(
    view: &View,
    opts: &EditorOptions,
    start: usize,
    end: usize,
) -> CaretEdit {
    let line_start = view.line_start(start);
    let line_end = view.line_end(end);
    let after: &[char] = &view.chars[end.min(view.len())..line_end];
    let after_blank = after.iter().all(|c| is_blank(*c));
    let before_all_blank = view.chars[line_start..start].iter().all(|c| is_blank(*c));
    let basic_indent = leading_blanks(view, line_start, start);

    // In the leading blanks of a line with text only the blanks are copied (ED-TYPE-063).
    if before_all_blank && !after_blank && start < line_end {
        let text = format!("\n{basic_indent}");
        let caret = text.chars().count();
        return CaretEdit::new(start, end, text, caret);
    }
    if let Some(edit) = comment_enter(view, opts, start, end, line_start, after, after_blank) {
        return edit;
    }
    let smart = opts.smart_indent && indent::has_formatter(view) && view.lang.is_code();
    let in_literal = view.in_literal(start);
    if smart && !in_literal {
        let prev = start
            .checked_sub(1)
            .and_then(|i| view.at(i))
            .filter(|_| start > line_start);
        let next = view.at(end);
        let line_indent = view.indent_at(line_start);
        let unit = indent::unit(view, opts);
        // Between a bracket pair (ED-TYPE-064).
        if let (Some(open), Some(close)) = (prev, next)
            && matches!((open, close), ('{', '}') | ('(', ')') | ('[', ']'))
            && view.is_code(start - 1)
        {
            let inner = format!("{line_indent}{}", unit.text);
            let text = format!("\n{inner}\n{line_indent}");
            return CaretEdit::new(start, end, text, 1 + inner.chars().count());
        }
        // After an unmatched opening brace (ED-TYPE-065).
        if prev == Some('{')
            && opts.pair_brace_on_enter
            && after_blank
            && view.is_code(start - 1)
            && view.lang.family != super::Family::Script
        {
            let (opens, closes) = view.bracket_counts('{', '}');
            if opens > closes {
                let inner = format!("{line_indent}{}", unit.text);
                let text = format!("\n{inner}\n{line_indent}}}");
                return CaretEdit::new(start, line_end.max(end), text, 1 + inner.chars().count());
            }
        }
        // Smart indent: the indent the code style gives the new line.
        let open = view.open_brackets(start);
        let mut new_indent = match open.last() {
            None => String::new(),
            Some((offset, bracket)) => {
                let opener_indent = view.indent_at(view.line_start(*offset));
                if *bracket == '{' {
                    format!("{opener_indent}{}", unit.text)
                } else {
                    format!("{opener_indent}{}{}", unit.text, unit.text)
                }
            }
        };
        // A closing bracket that follows moves the line out.
        let first_after = after.iter().find(|c| !is_blank(**c)).copied();
        if matches!(first_after, Some('}' | ')' | ']'))
            && let Some((offset, _)) = open.last()
        {
            new_indent = view.indent_at(view.line_start(*offset));
        }
        // Without an unclosed bracket the indent of the statement is kept.
        if open.is_empty() {
            new_indent = basic_indent.clone();
        }
        let blanks_after = after.iter().take_while(|c| is_blank(**c)).count();
        let text = format!("\n{new_indent}");
        let caret = text.chars().count();
        return CaretEdit::new(start, end + blanks_after, text, caret);
    }
    let text = format!("\n{basic_indent}");
    let caret = text.chars().count();
    CaretEdit::new(start, end, text, caret)
}

/// Enter inside a comment (ED-TYPE-066, ED-TYPE-067).
fn comment_enter(
    view: &View,
    opts: &EditorOptions,
    start: usize,
    end: usize,
    line_start: usize,
    after: &[char],
    after_blank: bool,
) -> Option<CaretEdit> {
    let region = view.inside(start)?;
    let line_indent = view.indent_at(line_start);
    match region.kind {
        RegionKind::LineComment => {
            if after_blank {
                return None;
            }
            let prefix_len = view.lang.line_comment.map_or(2, |p| p.chars().count());
            let mut spacing_end = region.start + prefix_len;
            while view.at(spacing_end).is_some_and(is_blank) {
                spacing_end += 1;
            }
            let prefix = view.text(region.start, spacing_end);
            let blanks_after = after.iter().take_while(|c| is_blank(**c)).count();
            let text = format!("\n{line_indent}{prefix}");
            let caret = text.chars().count();
            Some(CaretEdit::new(start, end + blanks_after, text, caret))
        }
        RegionKind::BlockComment | RegionKind::DocComment => {
            let doc = region.kind == RegionKind::DocComment;
            let opening_line = view.line_start(region.start) == line_start;
            if opening_line {
                if !region.closed && opts.close_block_comment {
                    let text = format!("\n{line_indent} * \n{line_indent} */");
                    let caret = 1 + line_indent.chars().count() + 3;
                    return Some(CaretEdit::new(start, end, text, caret));
                }
                if doc {
                    let text = format!("\n{line_indent} * ");
                    let caret = text.chars().count();
                    return Some(CaretEdit::new(start, end, text, caret));
                }
                let text = format!("\n{line_indent}");
                let caret = text.chars().count();
                return Some(CaretEdit::new(start, end, text, caret));
            }
            let first = view.first_non_blank(line_start);
            if view.at(first) == Some('*') && view.at(first + 1) != Some('/') {
                let mut after_star = first + 1;
                while view.at(after_star).is_some_and(is_blank) && after_star < start {
                    after_star += 1;
                }
                let spacing = view.text(first + 1, after_star);
                let spacing = if spacing.is_empty() {
                    " ".to_owned()
                } else {
                    spacing
                };
                let text = format!("\n{line_indent}*{spacing}");
                let caret = text.chars().count();
                return Some(CaretEdit::new(start, end, text, caret));
            }
            let text = format!("\n{line_indent}");
            let caret = text.chars().count();
            Some(CaretEdit::new(start, end, text, caret))
        }
        _ => None,
    }
}

/// Enter (ED-TYPE-060). In overwrite mode only the caret moves (ED-TYPE-063).
pub(crate) fn enter(doc: &mut Document, opts: &EditorOptions) -> Result<bool, String> {
    let view = view_of(doc);
    let overwrite = doc.input.overwrite;
    let edits: Vec<CaretEdit> = doc
        .selections
        .selections()
        .iter()
        .map(|&sel| {
            if overwrite && sel.is_caret() {
                let end = view.line_end(sel.head);
                if end >= view.len() {
                    // On the last line a line break is appended first.
                    return CaretEdit {
                        start: end,
                        end,
                        text: "\n".into(),
                        select: (1, 1),
                        scope: None,
                    };
                }
                return CaretEdit::move_to(sel.head, end + 1);
            }
            newline_edit(&view, opts, sel.start(), sel.end())
        })
        .collect();
    doc.input.goal = None;
    apply_caret_edits(doc, UndoKind::Enter, edits)
}

/// Start New Line (Shift+Enter): the line break goes at the line end (ED-TYPE-069).
pub(crate) fn start_new_line(doc: &mut Document, opts: &EditorOptions) -> Result<bool, String> {
    let view = view_of(doc);
    let edits: Vec<CaretEdit> = doc
        .selections
        .selections()
        .iter()
        .map(|sel| {
            let at = view.line_end(sel.end());
            newline_edit(&view, opts, at, at)
        })
        .collect();
    doc.input.goal = None;
    apply_caret_edits(doc, UndoKind::Enter, edits)
}

/// Start New Line Before Current (Ctrl+Alt+Enter): an empty indented line above (ED-TYPE-069).
pub(crate) fn start_new_line_before(
    doc: &mut Document,
    opts: &EditorOptions,
) -> Result<bool, String> {
    let view = view_of(doc);
    let edits: Vec<CaretEdit> = doc
        .selections
        .selections()
        .iter()
        .map(|sel| {
            let line_start = view.line_start(sel.start());
            let indent = if opts.smart_indent && indent::has_formatter(&view) {
                indent::proper(&view, opts, line_start)
            } else {
                view.indent_at(line_start)
            };
            let caret = indent.chars().count();
            CaretEdit::new(line_start, line_start, format!("{indent}\n"), caret)
        })
        .collect();
    doc.input.goal = None;
    apply_caret_edits(doc, UndoKind::Enter, edits)
}

/// Split Line (Ctrl+Enter): breaks the line at the caret and leaves the caret before the
/// break (ED-TYPE-070).
pub(crate) fn split_line(doc: &mut Document, opts: &EditorOptions) -> Result<bool, String> {
    let view = view_of(doc);
    let edits: Vec<CaretEdit> = doc
        .selections
        .selections()
        .iter()
        .map(|sel| {
            let line_start = view.line_start(sel.start());
            let only_blanks = view.chars[line_start..sel.start()]
                .iter()
                .all(|c| is_blank(*c));
            if only_blanks {
                // A copy of the blanks goes above; the caret stays with its text.
                let blanks = view.text(line_start, sel.start());
                return CaretEdit::new(
                    sel.start(),
                    sel.start(),
                    format!("\n{blanks}"),
                    1 + blanks.chars().count(),
                );
            }
            let mut edit = newline_edit(&view, opts, sel.start(), sel.end());
            edit.select = (0, 0);
            edit
        })
        .collect();
    doc.input.goal = None;
    apply_caret_edits(doc, UndoKind::Enter, edits)
}

/// Complete Current Statement (Ctrl+Shift+Enter): the language fixers, else Start New Line
/// (ED-TYPE-090 to ED-TYPE-094).
pub(crate) fn complete_statement(doc: &mut Document, opts: &EditorOptions) -> Result<bool, String> {
    let view = view_of(doc);
    if doc.selections.selections().len() == 1
        && view.lang.family == super::Family::Java
        && let Some(edit) = java_complete(&view, opts, doc.selections.primary())
    {
        return apply_caret_edits(doc, UndoKind::Other, vec![edit]);
    }
    start_new_line(doc, opts)
}

/// The Java fixers for the statement at the caret that the cases cover: an unclosed
/// parenthesis, a missing semicolon, an `if` without its block.
fn java_complete(view: &View, opts: &EditorOptions, sel: Selection) -> Option<CaretEdit> {
    if !sel.is_caret() || view.inside(sel.head).is_some() {
        return None;
    }
    let head = sel.head;
    let line_start = view.line_start(head);
    let line_end = view.line_end(head);
    let line = view.text(line_start, line_end);
    let trimmed = line.trim_end();
    // Only complete when the caret is at the end of the statement.
    if head < line_start + trimmed.chars().count() {
        return None;
    }
    let body = trimmed.trim_start();
    if body.is_empty() || body.ends_with(';') || body.ends_with('{') || body.ends_with('}') {
        return None;
    }
    let indent = view.indent_at(line_start);
    // Close the parentheses that are still open on this line.
    let open_before = view.open_brackets(head);
    let open_parens = open_before
        .iter()
        .filter(|(offset, c)| *c == '(' && *offset >= line_start)
        .count();
    let mut fixed = trimmed.to_owned();
    for _ in 0..open_parens {
        fixed.push(')');
    }
    let unit = indent::unit(view, opts);
    let compound = [
        "if ",
        "if(",
        "while ",
        "while(",
        "for ",
        "for(",
        "synchronized ",
    ]
    .iter()
    .any(|keyword| body.starts_with(keyword));
    let end_of_line_text = fixed.chars().count();
    if compound {
        let inner = format!("{indent}{}", unit.text);
        let text = format!("{fixed} {{\n{inner}\n{indent}}}");
        let caret = end_of_line_text + 3 + inner.chars().count();
        return Some(CaretEdit::new(
            line_start,
            line_start + trimmed.chars().count(),
            text,
            caret,
        ));
    }
    let text = format!("{fixed};\n{indent}");
    let caret = text.chars().count();
    Some(CaretEdit::new(
        line_start,
        line_start + trimmed.chars().count(),
        text,
        caret,
    ))
}

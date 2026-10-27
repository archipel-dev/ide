//! Paste (ED-TYPE-100 to ED-TYPE-107) and Copy / Cut with and without a selection (ED-CLIP-001
//! to ED-CLIP-003).

use super::indent;
use super::{Bias, EditorOptions, PasteMode, View, apply_changes, map_through, view_of};
use crate::Document;
use archipel_ide_text::{Selection, TextChange, UndoKind};

/// Largest paste that is re-indented line by line.
const REINDENT_LIMIT_LINES: usize = 2000;

fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Text for the clipboard: the selections joined by line breaks, or the caret lines (whole
/// lines, flagged) when nothing is selected. `None` when copying a line is switched off.
pub(crate) fn copy_text(doc: &Document, opts: &EditorOptions) -> Option<(String, bool)> {
    let selections = doc.selections.selections();
    if selections.iter().all(|s| s.is_caret()) {
        if !opts.copy_line_without_selection {
            return None;
        }
        let view = view_of(doc);
        let mut lines: Vec<usize> = selections.iter().map(|s| view.line_start(s.head)).collect();
        lines.dedup();
        let mut text = String::new();
        for start in lines {
            text.push_str(&view.text(start, view.line_end(start)));
            text.push('\n');
        }
        return Some((text, true));
    }
    let parts: Vec<String> = selections
        .iter()
        .filter(|s| !s.is_caret())
        .map(|s| doc.text.slice(s.start(), s.end()).unwrap_or_default())
        .collect();
    Some((parts.join("\n"), false))
}

/// Cut: removes what Copy puts on the clipboard. After cutting lines the caret goes to the
/// start of the line that moved up.
pub(crate) fn cut(
    doc: &mut Document,
    opts: &EditorOptions,
) -> Result<Option<(String, bool)>, String> {
    let Some((text, whole)) = copy_text(doc, opts) else {
        return Ok(None);
    };
    if whole {
        let view = view_of(doc);
        let mut changes = Vec::new();
        let mut starts: Vec<usize> = doc
            .selections
            .selections()
            .iter()
            .map(|s| view.line_start(s.head))
            .collect();
        starts.dedup();
        for start in &starts {
            let end = (view.line_end(*start) + 1).min(view.len());
            // The last line takes the break before it along.
            let from = if end == view.len() && !view.text(*start, end).ends_with('\n') && *start > 0
            {
                *start - 1
            } else {
                *start
            };
            changes.push(TextChange {
                start_char: from,
                end_char: end,
                inserted: String::new(),
            });
        }
        let first = starts[0];
        apply_changes(doc, UndoKind::Other, changes, |changes, d| {
            let at = map_through(changes, first, Bias::Left).min(d.text.len_chars());
            vec![Selection::caret(at)]
        })?;
        // The caret goes to the start of the line that took the place of the cut one.
        let view = view_of(doc);
        let at = doc.selections.primary().head;
        doc.selections
            .set_single(Selection::caret(view.line_start(at)));
    } else {
        let changes: Vec<TextChange> = doc
            .selections
            .selections()
            .iter()
            .filter(|s| !s.is_caret())
            .map(|s| TextChange {
                start_char: s.start(),
                end_char: s.end(),
                inserted: String::new(),
            })
            .collect();
        let before: Vec<Selection> = doc.selections.selections().to_vec();
        apply_changes(doc, UndoKind::Other, changes, |changes, _| {
            before
                .iter()
                .map(|s| Selection::caret(map_through(changes, s.start(), Bias::Left)))
                .collect()
        })?;
    }
    doc.input.sticky = false;
    Ok(Some((text, whole)))
}

/// Paste (ED-TYPE-100). `whole_line`: the clipboard holds a line copied without selection.
/// `raw` pastes the text untouched (Paste as Plain Text, ED-TYPE-106).
pub(crate) fn paste(
    doc: &mut Document,
    opts: &EditorOptions,
    text: &str,
    whole_line: bool,
    raw: bool,
) -> Result<bool, String> {
    let text = normalize(text);
    if text.is_empty() {
        return Ok(false);
    }
    let view = view_of(doc);
    let selections = doc.selections.selections().to_vec();
    let ages = doc.selections.ages().to_vec();
    let all_carets = selections.iter().all(|s| s.is_caret());
    // A line copied without selection goes above the caret line (ED-TYPE-104).
    if whole_line && all_carets {
        let changes: Vec<TextChange> = selections
            .iter()
            .map(|s| {
                let start = view.line_start(s.head);
                TextChange {
                    start_char: start,
                    end_char: start,
                    inserted: text.clone(),
                }
            })
            .collect();
        return apply_changes(doc, UndoKind::Other, changes, |changes, _| {
            selections
                .iter()
                .map(|s| Selection::caret(map_through(changes, s.head, Bias::Right)))
                .collect()
        });
    }
    let lines: Vec<&str> = text
        .strip_suffix('\n')
        .unwrap_or(&text)
        .split('\n')
        .collect();
    // One clipboard line per caret when the counts agree (ED-TYPE-100).
    let pieces: Vec<String> = if selections.len() > 1 && lines.len() == selections.len() {
        lines.iter().map(|l| (*l).to_owned()).collect()
    } else {
        vec![text.clone(); selections.len()]
    };
    let single = selections.len() == 1;
    let sel = selections[0];
    let mut pasted = pieces.clone();
    if single && !raw && view.lang.is_code() && !text.trim().is_empty() {
        pasted[0] = if let Some(region) = view.inside(sel.start())
            && region.is_literal()
        {
            escape_for_literal(&text, view.at(region.start).unwrap_or('"'))
        } else {
            reindent(&view, opts, sel, &text)
        };
    }
    let mut changes = Vec::new();
    let mut next = Vec::new();
    let mut shift: isize = 0;
    for (s, piece) in selections.iter().zip(&pasted) {
        let added = piece.chars().count();
        let start = s.start().saturating_add_signed(shift);
        next.push(Selection::caret(start + added));
        shift += added as isize - (s.end() - s.start()) as isize;
        changes.push(TextChange {
            start_char: s.start(),
            end_char: s.end(),
            inserted: piece.clone(),
        });
    }
    let result = apply_changes(doc, UndoKind::Other, changes, |_, _| next);
    let _ = ages;
    result
}

/// Escapes text for pasting into a string literal (ED-TYPE-103).
fn escape_for_literal(text: &str, quote: char) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out
}

/// The pasted text with the indent `Reformat on paste` gives its lines (ED-TYPE-102).
fn reindent(view: &View, opts: &EditorOptions, sel: Selection, text: &str) -> String {
    if opts.paste == PasteMode::None || !text.contains('\n') {
        return text.to_owned();
    }
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.len() > REINDENT_LIMIT_LINES {
        return text.to_owned();
    }
    match opts.paste {
        PasteMode::IndentBlock => {
            let line_start = view.line_start(sel.start());
            let column = sel.start() - line_start;
            let first_indent = lines[0]
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .count();
            let pad = " ".repeat(column);
            let mut out = String::from(lines[0].trim_start_matches([' ', '\t']));
            for line in &lines[1..] {
                out.push('\n');
                if line.trim().is_empty() {
                    continue;
                }
                let strip = line
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .count()
                    .min(first_indent);
                out.push_str(&pad);
                out.push_str(&line[strip..]);
            }
            out
        }
        _ => {
            if !indent::has_formatter(view) {
                return text.to_owned();
            }
            // Indent each line: the lines after the first get the indent of their place.
            let prefix = view.text(0, sel.start());
            let suffix = view.text(sel.end(), view.len());
            let mut working = format!("{prefix}{}{suffix}", lines[0]);
            let mut out = String::from(lines[0]);
            for line in &lines[1..] {
                let start_of_line = working.chars().count() - suffix.chars().count();
                // Append the line break and the line after the text so far.
                let body = line.trim_start_matches([' ', '\t']);
                working = format!(
                    "{}\n{}{}",
                    working.chars().take(start_of_line).collect::<String>(),
                    body,
                    suffix
                );
                let line_start = start_of_line + 1;
                let probe = View::of_text(&working, view.lang);
                let wanted = if body.is_empty() {
                    String::new()
                } else {
                    indent::proper_for_line(&probe, opts, line_start)
                };
                working = format!(
                    "{}{}{}",
                    working.chars().take(line_start).collect::<String>(),
                    wanted,
                    working.chars().skip(line_start).collect::<String>()
                );
                out.push('\n');
                out.push_str(&wanted);
                out.push_str(body);
            }
            out
        }
    }
}

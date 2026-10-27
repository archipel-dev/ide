//! Typing, Backspace and Delete (ED-TYPE-001 to ED-TYPE-049): per caret, in one undo step.
//!
//! Typed characters pass the handlers in the order of ED-TYPE section 1: overwrite mode, surround
//! on typing, overtype of a closing bracket or quote, pairing of an opening bracket or quote.

use super::indent;
use super::words;
use super::{CaretEdit, EditorOptions, UnindentMode, View, apply_caret_edits, view_of};
use crate::Document;
use archipel_ide_text::{Selection, UndoKind};

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

fn closer_of(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '<' => Some('>'),
        '"' => Some('"'),
        '\'' => Some('\''),
        '`' => Some('`'),
        _ => None,
    }
}

fn opener_of(close: char) -> Option<char> {
    match close {
        ')' => Some('('),
        ']' => Some('['),
        '}' => Some('{'),
        _ => None,
    }
}

fn is_quote(c: char) -> bool {
    matches!(c, '"' | '\'' | '`')
}

/// Characters before which a bracket pair may be inserted (ED-TYPE-010): blanks, closers,
/// separators, the end of the text and comments.
fn pair_allowed_before(view: &View, head: usize) -> bool {
    match view.at(head) {
        None => true,
        Some(c) if c.is_whitespace() || matches!(c, ')' | ']' | '}' | ',' | ';' | '>') => true,
        Some('/') => matches!(view.at(head + 1), Some('/' | '*')),
        Some(_) => false,
    }
}

/// `List` in `List<`: the identifier before `head` starts with an upper-case letter.
fn generics_context(view: &View, head: usize) -> bool {
    if !view.lang.has_generics() {
        return false;
    }
    let mut start = head;
    while start > 0 && view.at(start - 1).is_some_and(is_word_char) {
        start -= 1;
    }
    start < head && view.at(start).is_some_and(char::is_uppercase)
}

fn backslashes_before(view: &View, head: usize) -> usize {
    let mut count = 0;
    while head > count && view.at(head - count - 1) == Some('\\') {
        count += 1;
    }
    count
}

/// Number of `quote` characters in the line after `head` that are not escaped.
fn quotes_after(view: &View, head: usize, quote: char) -> usize {
    let end = view.line_end(head);
    let mut count = 0;
    let mut i = head;
    while i < end {
        match view.chars[i] {
            '\\' => i += 1,
            c if c == quote => count += 1,
            _ => {}
        }
        i += 1;
    }
    count
}

/// Typed text: one character goes through the handlers, anything longer (an input method's
/// commit) is inserted as it is.
pub(crate) fn type_text(
    doc: &mut Document,
    opts: &EditorOptions,
    text: &str,
) -> Result<bool, String> {
    if text.is_empty() {
        return Ok(false);
    }
    let view = view_of(doc);
    let overwrite = doc.input.overwrite;
    let selections = doc.selections.selections().to_vec();
    let mut chars = text.chars();
    let single = match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    };
    let scopes = doc.input.scopes.clone();
    let mut used_scopes: Vec<usize> = Vec::new();
    let edits: Vec<CaretEdit> = selections
        .iter()
        .map(|&sel| match single {
            Some(c) => type_char(&view, opts, overwrite, &scopes, sel, c, &mut used_scopes),
            None => CaretEdit::new(sel.start(), sel.end(), text, text.chars().count()),
        })
        .collect();
    doc.input
        .scopes
        .retain(|scope| !used_scopes.contains(&scope.close));
    doc.input.goal = None;
    apply_caret_edits(doc, UndoKind::Typing, edits)
}

fn type_char(
    view: &View,
    opts: &EditorOptions,
    overwrite: bool,
    scopes: &[super::TabOutScope],
    sel: Selection,
    c: char,
    used: &mut Vec<usize>,
) -> CaretEdit {
    let plain = |sel: Selection| CaretEdit::new(sel.start(), sel.end(), c.to_string(), 1);
    if sel.is_caret() && overwrite {
        return match view.at(sel.head) {
            Some(next) if next != '\n' => CaretEdit::new(sel.head, sel.head + 1, c.to_string(), 1),
            _ => plain(sel),
        };
    }
    if !sel.is_caret() {
        if let Some(edit) = replace_quotes(view, sel, c) {
            return edit;
        }
        if let Some(edit) = surround(view, opts, sel, c) {
            return edit;
        }
        return plain(sel);
    }
    let head = sel.head;
    if !view.lang.is_code() {
        return plain(sel);
    }
    // Overtype of a closing bracket (ED-TYPE-020).
    if opts.pair_brackets
        && view.at(head) == Some(c)
        && view.is_code(head)
        && (matches!(c, ')' | ']' | '}') || c == '>')
    {
        let scope = scopes.iter().find(|s| s.close == head && s.closer == c);
        let opener_exists = match opener_of(c) {
            Some(open) => !view.open_stack(open, c, head).is_empty(),
            None => scope.is_some(),
        };
        if opener_exists || scope.is_some() {
            used.push(head);
            return CaretEdit::move_to(head, head + 1);
        }
    }
    if is_quote(c) && opts.pair_quotes && c != '`' {
        return quote_edit(view, scopes, head, c, used);
    }
    if matches!(c, '(' | '[' | '{') && opts.pair_brackets {
        if view.inside(head).is_none() && pair_allowed_before(view, head) {
            let close = closer_of(c).unwrap_or(c);
            let (opens, closes) = view.bracket_counts(c, close);
            if opens + 1 > closes {
                return pair(head, c, close);
            }
        }
        return plain(sel);
    }
    if c == '<' && opts.pair_brackets && view.inside(head).is_none() {
        if generics_context(view, head) && pair_allowed_before(view, head) {
            return pair(head, '<', '>');
        }
        return plain(sel);
    }
    plain(sel)
}

fn pair(head: usize, open: char, close: char) -> CaretEdit {
    CaretEdit {
        start: head,
        end: head,
        text: format!("{open}{close}"),
        select: (1, 1),
        scope: Some((1, close)),
    }
}

/// Quote typed at a caret outside selections (ED-TYPE-025 to ED-TYPE-027).
fn quote_edit(
    view: &View,
    scopes: &[super::TabOutScope],
    head: usize,
    c: char,
    used: &mut Vec<usize>,
) -> CaretEdit {
    let plain = CaretEdit::new(head, head, c.to_string(), 1);
    // An escaped quote is an ordinary character.
    if backslashes_before(view, head) % 2 == 1 && view.in_literal(head) {
        return plain;
    }
    match view.inside(head) {
        Some(region) if region.is_comment() => plain,
        Some(region) => {
            let closes_here = region.closed && head + 1 == region.end && view.at(head) == Some(c);
            if closes_here {
                used.push(head);
                return CaretEdit::move_to(head, head + 1);
            }
            let _ = scopes;
            plain
        }
        None => {
            if view.before(head).is_some_and(is_word_char) {
                return plain;
            }
            if view.at(head).is_some_and(is_word_char) {
                return plain;
            }
            if quotes_after(view, head, c) % 2 == 1 {
                return plain;
            }
            if c == '\''
                && view.lang.family == super::Family::CLike
                && view.before(head).is_some_and(|p| p == '&' || p == '<')
            {
                return plain;
            }
            CaretEdit {
                start: head,
                end: head,
                text: format!("{c}{c}"),
                select: (1, 1),
                scope: Some((1, c)),
            }
        }
    }
}

/// Typing a quote with a single quote character selected replaces both ends of the literal
/// (ED-TYPE-037).
fn replace_quotes(view: &View, sel: Selection, c: char) -> Option<CaretEdit> {
    if !is_quote(c) || sel.end() - sel.start() != 1 {
        return None;
    }
    let old = view.at(sel.start())?;
    if !is_quote(old) || old == c {
        return None;
    }
    let region = view.region_of_char(sel.start())?;
    if !region.is_literal() || !region.closed {
        return None;
    }
    if sel.start() != region.start && sel.end() != region.end {
        return None;
    }
    let body: String = view.chars[region.start + 1..region.end - 1]
        .iter()
        .collect();
    // Re-escape: the new quote gains a backslash, the old one loses its.
    let mut escaped = String::new();
    let mut chars = body.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&old) {
            continue;
        }
        if ch == c {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    let text = format!("{c}{escaped}{c}");
    Some(CaretEdit::new(region.start, region.end, text, 1))
}

/// Surround on typing (ED-TYPE-035, ED-TYPE-036).
fn surround(view: &View, opts: &EditorOptions, sel: Selection, c: char) -> Option<CaretEdit> {
    if !opts.surround_selection || !matches!(c, '(' | '[' | '{' | '<' | '"' | '\'' | '`') {
        return None;
    }
    let (start, end) = (sel.start(), sel.end());
    let text: String = view.chars[start..end].iter().collect();
    if text.trim().is_empty() {
        return None;
    }
    // A selected comparison operator is replaced, not surrounded.
    if c == '<'
        && view.lang.is_code()
        && !view.in_comment(start)
        && !view.in_literal(start)
        && text.chars().count() <= 3
        && text.chars().all(|ch| matches!(ch, '<' | '>' | '=' | '!'))
    {
        return None;
    }
    let close = closer_of(c)?;
    let forward = sel.head >= sel.anchor;
    let orient = |len: usize| if forward { (1, 1 + len) } else { (1 + len, 1) };
    let chars: Vec<char> = text.chars().collect();
    // Selected delimiters of the same family are replaced, not nested.
    if chars.len() >= 2 {
        let (first, last) = (chars[0], chars[chars.len() - 1]);
        let inner: String = chars[1..chars.len() - 1].iter().collect();
        let quotes = is_quote(c) && is_quote(first) && first == last && !inner.contains(first);
        let brackets = !is_quote(c)
            && closer_of(first).is_some_and(|partner| partner == last && !is_quote(first));
        if quotes || brackets {
            let len = inner.chars().count();
            return Some(CaretEdit {
                start,
                end,
                text: format!("{c}{inner}{close}"),
                select: orient(len),
                scope: None,
            });
        }
    }
    let len = chars.len();
    Some(CaretEdit {
        start,
        end,
        text: format!("{c}{text}{close}"),
        select: orient(len),
        scope: None,
    })
}

/// Backspace (ED-TYPE-040 to ED-TYPE-046).
pub(crate) fn backspace(doc: &mut Document, opts: &EditorOptions) -> Result<bool, String> {
    let view = view_of(doc);
    let edits: Vec<CaretEdit> = doc
        .selections
        .selections()
        .iter()
        .map(|&sel| backspace_edit(&view, opts, sel))
        .collect();
    doc.input.goal = None;
    apply_caret_edits(doc, UndoKind::Backspace, edits)
}

fn is_blank(c: char) -> bool {
    c == ' ' || c == '\t'
}

fn backspace_edit(view: &View, opts: &EditorOptions, sel: Selection) -> CaretEdit {
    if !sel.is_caret() {
        return CaretEdit::new(sel.start(), sel.end(), "", 0);
    }
    let head = sel.head;
    if head == 0 {
        return CaretEdit::move_to(head, head);
    }
    let prev = view.chars[head - 1];
    let line_start = view.line_start(head);
    if is_blank(prev)
        && view.chars[line_start..head].iter().all(|c| is_blank(*c))
        && opts.unindent != UnindentMode::Disabled
        && let Some(edit) = unindent_edit(view, opts, head, line_start)
    {
        return edit;
    }
    let next = view.at(head);
    if view.lang.is_code() {
        // The partner of a deleted opening bracket or quote goes with it (ED-TYPE-043 to 045).
        if opts.pair_brackets
            && let (Some(close), Some(next)) = (closer_of(prev), next)
            && matches!(prev, '(' | '[' | '{')
            && next == close
            && view.is_code(head - 1)
            && view.open_stack(prev, close, head - 1).is_empty()
        {
            return CaretEdit::new(head - 1, head + 1, "", 0);
        }
        if opts.pair_brackets
            && prev == '<'
            && next == Some('>')
            && view.is_code(head - 1)
            && generics_context(view, head - 1)
        {
            return CaretEdit::new(head - 1, head + 1, "", 0);
        }
        if opts.pair_quotes
            && is_quote(prev)
            && next == Some(prev)
            && view.region_of_char(head - 1).is_some_and(|r| {
                r.is_literal() && r.start == head - 1 && r.end == head + 1 && r.closed
            })
        {
            return CaretEdit::new(head - 1, head + 1, "", 0);
        }
    }
    CaretEdit::new(head - 1, head, "", 0)
}

/// Unindent on Backspace (ED-TYPE-046). `None` leaves the plain deletion.
fn unindent_edit(
    view: &View,
    opts: &EditorOptions,
    head: usize,
    line_start: usize,
) -> Option<CaretEdit> {
    let unit = indent::unit(view, opts);
    let width = unit.width.max(1);
    let prefix: String = view.chars[line_start..head].iter().collect();
    let nearest = |view: &View| -> Option<CaretEdit> {
        if prefix.contains('\t') {
            return None;
        }
        let column = prefix.chars().count();
        let target = ((column - 1) / width) * width;
        let _ = view;
        Some(CaretEdit::new(head - (column - target), head, "", 0))
    };
    if opts.unindent == UnindentMode::Nearest || !indent::has_formatter(view) {
        return nearest(view);
    }
    // Proper indent position.
    let first = view.first_non_blank(line_start);
    let full: String = view.chars[line_start..first].iter().collect();
    let proper = indent::proper_for_line(view, opts, line_start);
    let tab_width = opts.indent_size;
    if indent::columns(&full, tab_width) > indent::columns(&proper, tab_width) {
        let caret = proper.chars().count();
        return Some(CaretEdit::new(line_start, first, proper, caret));
    }
    if line_start == 0 {
        return Some(CaretEdit::new(line_start, first, "", 0));
    }
    // Join with the previous line; its trailing blanks go.
    let mut join = line_start - 1;
    let previous_start = view.line_start(join);
    while join > previous_start && is_blank(view.chars[join - 1]) {
        join -= 1;
    }
    Some(CaretEdit::new(join, first, "", 0))
}

/// Delete (ED-TYPE-048).
pub(crate) fn delete_forward(doc: &mut Document) -> Result<bool, String> {
    let len = doc.text.len_chars();
    let edits: Vec<CaretEdit> = doc
        .selections
        .selections()
        .iter()
        .map(|&sel| {
            if !sel.is_caret() {
                CaretEdit::new(sel.start(), sel.end(), "", 0)
            } else if sel.head < len {
                CaretEdit::new(sel.head, sel.head + 1, "", 0)
            } else {
                CaretEdit::move_to(sel.head, sel.head)
            }
        })
        .collect();
    doc.input.goal = None;
    apply_caret_edits(doc, UndoKind::Delete, edits)
}

/// Delete to Word Start / End (ED-TYPE-049).
pub(crate) fn delete_word(
    doc: &mut Document,
    opts: &EditorOptions,
    forward: bool,
) -> Result<bool, String> {
    let view = view_of(doc);
    let edits: Vec<CaretEdit> = doc
        .selections
        .selections()
        .iter()
        .map(|&sel| {
            if !sel.is_caret() {
                return CaretEdit::new(sel.start(), sel.end(), "", 0);
            }
            let target = if forward {
                words::delete_word_end(&view, sel.head, opts.camel_humps)
            } else {
                words::delete_word_start(&view, sel.head, opts.camel_humps)
            };
            let (start, end) = (sel.head.min(target), sel.head.max(target));
            CaretEdit::new(start, end, "", 0)
        })
        .collect();
    doc.input.goal = None;
    apply_caret_edits(
        doc,
        if forward {
            UndoKind::Delete
        } else {
            UndoKind::Backspace
        },
        edits,
    )
}

/// Hungry Backspace (ED-TYPE-047).
pub(crate) fn hungry_backspace(doc: &mut Document, opts: &EditorOptions) -> Result<bool, String> {
    let view = view_of(doc);
    let edits: Vec<CaretEdit> = doc
        .selections
        .selections()
        .iter()
        .map(|&sel| {
            if !sel.is_caret() {
                return CaretEdit::new(sel.start(), sel.end(), "", 0);
            }
            if sel.head > 0 && view.chars[sel.head - 1].is_whitespace() {
                let mut start = sel.head;
                while start > 0 && view.chars[start - 1].is_whitespace() {
                    start -= 1;
                }
                return CaretEdit::new(start, sel.head, "", 0);
            }
            backspace_edit(&view, opts, sel)
        })
        .collect();
    doc.input.goal = None;
    apply_caret_edits(doc, UndoKind::Backspace, edits)
}

/// Tab at the end of a pair typed with its opener leaves it (ED-TYPE-030). Returns whether it
/// moved a caret.
pub(crate) fn tab_out(doc: &mut Document, opts: &EditorOptions) -> bool {
    if !opts.tab_out || doc.input.scopes.is_empty() {
        return false;
    }
    let view = view_of(doc);
    let selections = doc.selections.selections().to_vec();
    let mut moved = false;
    let mut next = Vec::with_capacity(selections.len());
    let mut used = Vec::new();
    for sel in &selections {
        let scope =
            doc.input.scopes.iter().find(|s| {
                sel.is_caret() && s.close == sel.head && view.at(s.close) == Some(s.closer)
            });
        match scope {
            Some(scope) => {
                moved = true;
                used.push(scope.close);
                next.push(Selection::caret(sel.head + 1));
            }
            None => next.push(*sel),
        }
    }
    if !moved {
        // Scopes whose closer is gone are dropped.
        doc.input
            .scopes
            .retain(|s| view.at(s.close) == Some(s.closer));
        return false;
    }
    doc.input.scopes.retain(|s| !used.contains(&s.close));
    doc.selections.set_all(next);
    doc.input.goal = None;
    true
}

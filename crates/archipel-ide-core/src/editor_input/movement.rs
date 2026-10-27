//! Caret movement and selection commands (ED-CARET-001 to ED-CARET-052, ED-CARET-090).

use super::words;
use super::{CloneGroup, EditorOptions, Goal, View, view_of};
use crate::Document;
use archipel_ide_text::Selection;

/// Lines moved by Page Up and Page Down when the window has not reported its height.
const DEFAULT_PAGE_LINES: usize = 30;

/// Collapsed folding regions of the document in characters (ED-CARET-110: carets never rest
/// inside one).
fn collapsed(doc: &Document) -> Vec<(usize, usize)> {
    doc.folds
        .collapsed()
        .iter()
        .map(|fold| {
            (
                doc.text.byte_to_char(fold.start_byte),
                doc.text.byte_to_char(fold.end_byte),
            )
        })
        .collect()
}

/// Moves `target` out of a collapsed region to the edge in the direction of travel.
fn skip_folds(folds: &[(usize, usize)], from: usize, target: usize) -> usize {
    for &(start, end) in folds {
        if start < target && target < end {
            return if target >= from { end } else { start };
        }
    }
    target
}

/// Lines hidden inside collapsed regions (everything after the first line of a region).
fn hidden_lines(doc: &Document, folds: &[(usize, usize)]) -> Vec<(usize, usize)> {
    folds
        .iter()
        .filter_map(|&(start, end)| {
            let first = doc.text.char_to_line(start);
            let last = doc.text.char_to_line(end);
            (last > first).then_some((first + 1, last))
        })
        .collect()
}

fn line_len(view: &View, line_start: usize) -> usize {
    view.line_end(line_start) - line_start
}

/// Runs a caret movement action. `None` when `action` is not one.
pub(crate) fn run(doc: &mut Document, opts: &EditorOptions, action: &str) -> Option<()> {
    let (name, with_selection) = match action.strip_suffix("WithSelection") {
        Some(name) => (name, true),
        None => (action, false),
    };
    let known = [
        "EditorLeft",
        "EditorRight",
        "EditorUp",
        "EditorDown",
        "EditorLineStart",
        "EditorLineEnd",
        "EditorTextStart",
        "EditorTextEnd",
        "EditorPageUp",
        "EditorPageDown",
        "EditorMoveToPageTop",
        "EditorMoveToPageBottom",
        "EditorNextWord",
        "EditorPreviousWord",
        "EditorCodeBlockStart",
        "EditorCodeBlockEnd",
    ];
    if !known.contains(&name) {
        return None;
    }
    let extend = with_selection || doc.input.sticky;
    let view = view_of(doc);
    let folds = collapsed(doc);
    let selections = doc.selections.selections().to_vec();
    let vertical = matches!(
        name,
        "EditorUp" | "EditorDown" | "EditorPageUp" | "EditorPageDown"
    );
    let old_heads: Vec<usize> = selections.iter().map(|s| s.head).collect();
    let goal_columns: Vec<usize> = match &doc.input.goal {
        Some(goal) if vertical && goal.heads == old_heads => goal.columns.clone(),
        _ => old_heads
            .iter()
            .map(|&head| head - view.line_start(head))
            .collect(),
    };
    let page = if doc.input.page_lines == 0 {
        DEFAULT_PAGE_LINES
    } else {
        doc.input.page_lines
    };
    let last_line = view.chars.iter().filter(|c| **c == '\n').count();
    let hidden = hidden_lines(doc, &folds);
    let line_of = |offset: usize| view.chars[..offset].iter().filter(|c| **c == '\n').count();
    let line_start_of = |line: usize| {
        if line == 0 {
            return 0;
        }
        let mut seen = 0;
        for (i, c) in view.chars.iter().enumerate() {
            if *c == '\n' {
                seen += 1;
                if seen == line {
                    return i + 1;
                }
            }
        }
        view.len()
    };
    let mut new_columns = goal_columns.clone();
    let mut next: Vec<Selection> = Vec::with_capacity(selections.len());
    for (index, sel) in selections.iter().enumerate() {
        let head = sel.head;
        let collapse = !extend && !sel.is_caret();
        let target = match name {
            "EditorLeft" => {
                if collapse {
                    sel.start()
                } else {
                    skip_folds(&folds, head, head.saturating_sub(1))
                }
            }
            "EditorRight" => {
                if collapse {
                    sel.end()
                } else {
                    skip_folds(&folds, head, (head + 1).min(view.len()))
                }
            }
            "EditorUp" | "EditorDown" | "EditorPageUp" | "EditorPageDown" => {
                let down = matches!(name, "EditorDown" | "EditorPageDown");
                let base = if collapse && !extend {
                    if down { sel.end() } else { sel.start() }
                } else {
                    head
                };
                let line = line_of(base);
                let steps = if name.starts_with("EditorPage") {
                    page
                } else {
                    1
                };
                let mut target_line = line;
                for _ in 0..steps {
                    let mut candidate = if down {
                        (target_line + 1).min(last_line)
                    } else {
                        target_line.saturating_sub(1)
                    };
                    if let Some(&(first, last)) = hidden
                        .iter()
                        .find(|(first, last)| *first <= candidate && candidate <= *last)
                    {
                        candidate = if down {
                            (last + 1).min(last_line)
                        } else {
                            first - 1
                        };
                        if down && last >= last_line {
                            candidate = target_line;
                        }
                    }
                    if candidate == target_line {
                        break;
                    }
                    target_line = candidate;
                }
                if target_line == line {
                    // Up on the first line and Down on the last do not move (ED-CARET-004).
                    base
                } else {
                    let start = line_start_of(target_line);
                    let column = goal_columns[index].min(line_len(&view, start));
                    new_columns[index] = goal_columns[index];
                    start + column
                }
            }
            "EditorLineStart" => {
                let start = view.line_start(head);
                if !opts.smart_home {
                    start
                } else {
                    let first = view.first_non_blank(start);
                    // Toggles between the first non-blank character and column 0.
                    if head == start || head > first {
                        first
                    } else {
                        start
                    }
                }
            }
            "EditorLineEnd" => {
                let end = view.line_end(head);
                let mut text_end = end;
                while text_end > view.line_start(head)
                    && matches!(view.chars[text_end - 1], ' ' | '\t')
                {
                    text_end -= 1;
                }
                // The first press stops before trailing blanks, the second goes to the real end.
                if head < text_end { text_end } else { end }
            }
            "EditorTextStart" => 0,
            "EditorTextEnd" => view.len(),
            // A collapsed region is one unit: from its edge the word move goes to the other edge
            // (ED-CARET-013).
            "EditorNextWord" => match folds
                .iter()
                .find(|&&(start, end)| start <= head && head < end)
            {
                Some(&(_, end)) => end,
                None => skip_folds(
                    &folds,
                    head,
                    words::next_word(&view, head, opts.camel_humps, opts.caret_stop_windows),
                ),
            },
            "EditorPreviousWord" => {
                match folds
                    .iter()
                    .find(|&&(start, end)| start < head && head <= end)
                {
                    Some(&(start, _)) => start,
                    None => skip_folds(
                        &folds,
                        head,
                        words::previous_word(
                            &view,
                            head,
                            opts.camel_humps,
                            opts.caret_stop_windows,
                        ),
                    ),
                }
            }
            "EditorMoveToPageTop" | "EditorMoveToPageBottom" => {
                let top = doc.input.top_line.min(last_line);
                let line = if name == "EditorMoveToPageTop" {
                    top
                } else {
                    (top + page - 1).min(last_line)
                };
                let start = line_start_of(line);
                start + (head - view.line_start(head)).min(line_len(&view, start))
            }
            "EditorCodeBlockStart" | "EditorCodeBlockEnd" => {
                code_block_target(&view, head, name == "EditorCodeBlockEnd").unwrap_or(head)
            }
            _ => head,
        };
        next.push(Selection {
            anchor: if extend { sel.anchor } else { target },
            head: target,
        });
    }
    let text_edge = matches!(name, "EditorTextStart" | "EditorTextEnd");
    if text_edge && !extend {
        // Move Caret to Text Start / End removes the other carets (ED-CARET-016).
        let primary = next.last().copied().unwrap_or(Selection::caret(0));
        doc.selections.set_single(primary);
    } else {
        doc.selections.set_all(next);
    }
    if vertical {
        let heads: Vec<usize> = doc.selections.selections().iter().map(|s| s.head).collect();
        if heads.len() == new_columns.len() {
            doc.input.goal = Some(Goal {
                heads,
                columns: new_columns,
            });
        }
    } else {
        doc.input.goal = None;
    }
    Some(())
}

/// The caret target of Move Caret to Code Block Start / End (ED-CARET-050): the opening
/// brace of the innermost enclosing block, or the position behind its closing brace.
fn code_block_target(view: &View, head: usize, end: bool) -> Option<usize> {
    let open = view.open_brackets(head);
    let (opener, _) = open.iter().rev().find(|(_, c)| *c == '{').copied()?;
    if !end {
        return Some(opener);
    }
    let mut depth = 0usize;
    for (i, c) in view.chars.iter().enumerate().skip(opener) {
        if !view.is_code(i) {
            continue;
        }
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// Select All (ED-CARET-030).
pub(crate) fn select_all(doc: &mut Document) {
    let len = doc.text.len_chars();
    doc.selections.set_single(Selection {
        anchor: 0,
        head: len,
    });
    doc.input.goal = None;
}

/// Move Caret to Matching Brace (ED-CARET-051).
pub(crate) fn match_brace(doc: &mut Document) {
    let view = view_of(doc);
    let pairs = [('(', ')'), ('[', ']'), ('{', '}')];
    doc.selections.map(|sel| {
        let head = sel.head;
        let find_open = |close_at: usize, open: char, close: char| {
            let mut depth = 0usize;
            for i in (0..=close_at).rev() {
                if !view.is_code(i) {
                    continue;
                }
                let c = view.chars[i];
                if c == close {
                    depth += 1;
                } else if c == open {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
            }
            None
        };
        let find_close = |open_at: usize, open: char, close: char| {
            let mut depth = 0usize;
            for i in open_at..view.len() {
                if !view.is_code(i) {
                    continue;
                }
                let c = view.chars[i];
                if c == open {
                    depth += 1;
                } else if c == close {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
            }
            None
        };
        // The bracket after the caret opens: jump behind its partner.
        if let Some(&(open, close)) = view.at(head).and_then(|c| pairs.iter().find(|p| p.0 == c))
            && view.is_code(head)
            && let Some(partner) = find_close(head, open, close)
        {
            return Selection::caret(partner + 1);
        }
        // The bracket before the caret closes: jump before its partner.
        if let Some(&(open, close)) = head
            .checked_sub(1)
            .and_then(|i| view.at(i))
            .and_then(|c| pairs.iter().find(|p| p.1 == c))
            && view.is_code(head - 1)
            && let Some(partner) = find_open(head - 1, open, close)
        {
            return Selection::caret(partner);
        }
        // Otherwise the nearest unclosed opening bracket before the caret.
        match view.open_brackets(head).last() {
            Some(&(offset, _)) => Selection::caret(offset),
            None => sel,
        }
    });
    doc.input.goal = None;
}

/// Extend Line Selection (ED-CARET-038): whole lines covering the selection; repeating adds
/// one line.
pub(crate) fn select_line(doc: &mut Document) {
    let view = view_of(doc);
    doc.selections.map(|sel| {
        let first_start = view.line_start(sel.start());
        let mut last_end = sel.end();
        // A selection ending at a line start does not cover that line.
        let covers_to_line_start = !sel.is_caret()
            && last_end > first_start
            && view.line_start(last_end) == last_end
            && last_end > 0;
        let last_line_start = if covers_to_line_start {
            view.line_start(last_end - 1)
        } else {
            view.line_start(last_end)
        };
        last_end = (view.line_end(last_line_start) + 1).min(view.len());
        let current = (sel.start(), sel.end());
        let target = (first_start, last_end);
        if current == target && last_end < view.len() {
            let next_end = (view.line_end(last_end) + 1).min(view.len());
            return Selection {
                anchor: first_start,
                head: next_end,
            };
        }
        Selection {
            anchor: first_start,
            head: last_end,
        }
    });
    doc.input.goal = None;
}

/// Escape in the editor (ED-CARET-090): turns off sticky selection, keeps the oldest caret and
/// removes the selection. Returns whether the editor used the key.
pub(crate) fn escape(doc: &mut Document) -> bool {
    let sticky = std::mem::take(&mut doc.input.sticky);
    let multiple = doc.selections.selections().len() > 1;
    let selected = doc.selections.selections().iter().any(|s| !s.is_caret());
    if !multiple && !selected {
        return sticky;
    }
    let keep = if multiple {
        doc.selections.oldest()
    } else {
        doc.selections.primary()
    };
    doc.selections.set_single(Selection::caret(keep.head));
    doc.input.goal = None;
    true
}

/// Shrink Selection (ED-CARET-036): the largest syntactic range inside the selection that
/// contains the caret (or ends at it); else the previous step of Extend Selection while it
/// still lies inside the selection; else the caret.
pub(crate) fn shrink_selection(doc: &mut Document) -> bool {
    let source = doc.text.text();
    let before = doc.selections.selections().to_vec();
    let next: Vec<Selection> = before
        .iter()
        .map(|sel| {
            if sel.is_caret() {
                return *sel;
            }
            let caret = sel.head;
            let mut best: Option<(usize, usize)> = None;
            let mut range = {
                let byte = doc.text.char_to_byte(caret);
                byte..byte
            };
            for _ in 0..64 {
                let Some(next_range) = doc.analysis.expand_selection(&source, range.clone()) else {
                    break;
                };
                let (start, end) = (
                    doc.text.byte_to_char(next_range.start),
                    doc.text.byte_to_char(next_range.end),
                );
                let inside = start >= sel.start() && end <= sel.end();
                if !inside || (start, end) == (sel.start(), sel.end()) {
                    break;
                }
                // Punctuation alone is not a step.
                let has_word = doc
                    .text
                    .slice(start, end)
                    .is_ok_and(|text| text.chars().any(char::is_alphanumeric));
                if has_word {
                    best = Some((start, end));
                }
                range = next_range;
            }
            match best {
                Some((start, end)) => {
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
                None => Selection::caret(caret),
            }
        })
        .collect();
    // Nothing smaller around the caret: step back through Extend Selection.
    let collapsed = next.iter().all(|s| s.is_caret()) && before.iter().any(|s| !s.is_caret());
    if collapsed
        && let Some(previous) = doc.selection_history.last().cloned()
        && previous.len() == before.len()
        && previous.iter().zip(&before).all(|(inner, outer)| {
            outer.start() <= inner.start() && inner.end() <= outer.end() && inner != outer
        })
    {
        doc.selection_history.pop();
        doc.selections.set_all(previous);
        return true;
    }
    let changed = next != before;
    doc.selections.set_all(next);
    doc.selection_history.clear();
    changed
}

/// Clone Caret Above / Below (ED-CARET-073): a caret on the next line in the remembered column;
/// the opposite direction removes the farthest clone first.
pub(crate) fn clone_caret(doc: &mut Document, above: bool) -> bool {
    let view = view_of(doc);
    let selections = doc.selections.selections().to_vec();
    let heads: Vec<usize> = selections.iter().map(|s| s.head).collect();
    let line_of = |offset: usize| view.chars[..offset].iter().filter(|c| **c == '\n').count();
    let last_line = view.chars.iter().filter(|c| **c == '\n').count();
    let line_start_of = |line: usize| {
        let mut seen = 0;
        if line == 0 {
            return 0;
        }
        for (i, c) in view.chars.iter().enumerate() {
            if *c == '\n' {
                seen += 1;
                if seen == line {
                    return i + 1;
                }
            }
        }
        view.len()
    };
    let mut group = match doc.input.clone_group.take() {
        Some(group) if group.heads == heads => group,
        _ => CloneGroup {
            heads: heads.clone(),
            column: selections
                .last()
                .map(|s| s.head - view.line_start(s.head))
                .unwrap_or(0),
            below: 0,
            above: 0,
        },
    };
    let mut carets: Vec<Selection> = selections.clone();
    let lines: Vec<usize> = heads.iter().map(|&h| line_of(h)).collect();
    let changed;
    if above {
        if group.below > 0 && carets.len() > 1 {
            // Remove the lowest clone.
            let lowest = lines
                .iter()
                .enumerate()
                .max_by_key(|(_, l)| **l)
                .map(|(i, _)| i);
            if let Some(i) = lowest {
                carets.remove(i);
            }
            group.below -= 1;
            changed = true;
        } else {
            let top = lines.iter().copied().min().unwrap_or(0);
            if top == 0 {
                changed = false;
            } else {
                let start = line_start_of(top - 1);
                let column = group.column.min(line_len(&view, start));
                carets.push(Selection::caret(start + column));
                group.above += 1;
                changed = true;
            }
        }
    } else if group.above > 0 && carets.len() > 1 {
        let highest = lines
            .iter()
            .enumerate()
            .min_by_key(|(_, l)| **l)
            .map(|(i, _)| i);
        if let Some(i) = highest {
            carets.remove(i);
        }
        group.above -= 1;
        changed = true;
    } else {
        let bottom = lines.iter().copied().max().unwrap_or(0);
        if bottom >= last_line {
            changed = false;
        } else {
            let start = line_start_of(bottom + 1);
            let column = group.column.min(line_len(&view, start));
            carets.push(Selection::caret(start + column));
            group.below += 1;
            changed = true;
        }
    }
    if changed {
        doc.selections.set_all(carets);
        group.heads = doc.selections.selections().iter().map(|s| s.head).collect();
        doc.input.clone_group = Some(group);
    } else {
        doc.input.clone_group = Some(group);
    }
    changed
}

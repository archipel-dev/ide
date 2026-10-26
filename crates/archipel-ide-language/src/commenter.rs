//! The commenter: line and block comment decisions from the syntax tree
//! (a `//` inside a string literal is never a comment) and the language at the
//! caret (JavaScript inside HTML comments with JavaScript's prefix).

use crate::edit::{Edit, EditSet};
use crate::lines::{LineIndex, indent_bytes};
use crate::node::SyntaxSnapshot;
use crate::syntax_data::{LanguageData, PLAIN_TEXT, language_data};
use std::ops::Range;

/// The snapshot of the innermost injected fragment at `offset`, or `snap`.
pub fn innermost(snap: &SyntaxSnapshot, offset: usize) -> &SyntaxSnapshot {
    u32::try_from(offset)
        .ok()
        .and_then(|offset| snap.fragment_at(offset))
        .map_or(snap, |fragment| &fragment.snapshot)
}

/// Editing data of the language at `offset` (the injected language inside a
/// fragment).
pub fn data_at(snap: &SyntaxSnapshot, offset: usize) -> &'static LanguageData {
    language_data(innermost(snap, offset).lang().id).unwrap_or(&PLAIN_TEXT)
}

/// The line comment prefix at `offset`, if the language has one.
pub fn line_prefix_at(snap: &SyntaxSnapshot, offset: usize) -> Option<&'static str> {
    data_at(snap, offset).line_prefix()
}

fn is_comment(snap: &SyntaxSnapshot, kind_id: u16) -> bool {
    snap.lang()
        .token_set_table()
        .comments
        .contains(crate::ElementType(kind_id))
}

/// Is the comment token containing `offset` (strictly inside, or at its start)?
pub fn comment_at(snap: &SyntaxSnapshot, offset: usize) -> Option<Range<usize>> {
    let inner = innermost(snap, offset);
    let leaf = inner.root().find_leaf_at(u32::try_from(offset).ok()?)?;
    std::iter::once(leaf)
        .chain(leaf.ancestors())
        .find(|node| is_comment(inner, node.raw().kind_id()))
        .map(|node| node.byte_range())
}

/// True when the line's first token is a line comment starting with `prefix`.
/// The decision uses the tree, so text inside string literals never counts.
pub fn is_line_commented(
    snap: &SyntaxSnapshot,
    lines: &LineIndex,
    line: usize,
    prefix: &str,
) -> bool {
    let text = snap.text();
    let range = lines.line_range(line, text);
    let raw = &text[range.clone()];
    let first = range.start + indent_bytes(raw);
    if !text[first..].starts_with(prefix) {
        return false;
    }
    match comment_at(snap, first) {
        Some(comment) => comment.start == first,
        None => false,
    }
}

/// Comment with Block Comment: wraps the selection (or the caret's line) in
/// the language's block delimiters, or unwraps it when it already is one.
pub fn toggle_block_comment(snap: &SyntaxSnapshot, range: Range<usize>) -> Option<EditSet> {
    let text = snap.text();
    let data = data_at(snap, range.start);
    let (open, close) = data.block_comment?;
    // Uncomment: the range lies inside a block comment token.
    if let Some(comment) = comment_at(snap, range.start)
        && comment.start <= range.start
        && range.end <= comment.end
    {
        let body = &text[comment.clone()];
        if body.starts_with(open) && body.ends_with(close) && body.len() >= open.len() + close.len()
        {
            let after_open = comment.start + open.len();
            let open_end = after_open + usize::from(text[after_open..].starts_with(' '));
            let before_close = comment.end - close.len();
            let close_start = before_close
                - usize::from(before_close > open_end && text[..before_close].ends_with(' '));
            let removed_before = open_end - comment.start;
            let caret = range
                .start
                .saturating_sub(removed_before)
                .max(comment.start);
            return Some(EditSet {
                edits: vec![
                    Edit::delete(comment.start..open_end),
                    Edit::delete(close_start..comment.end),
                ],
                caret: range.is_empty().then_some(caret),
                selection: (!range.is_empty()).then(|| {
                    let start = range.start.max(open_end) - removed_before;
                    let end = range.end.min(close_start) - removed_before;
                    start..end.max(start)
                }),
            });
        }
    }
    let (start, end) = if range.is_empty() {
        let lines = LineIndex::new(text);
        let line = lines.line_of(range.start);
        let line_range = lines.line_range(line, text);
        let raw = &text[line_range.clone()];
        let trimmed_start = line_range.start + indent_bytes(raw);
        let trimmed_end = line_range.start + raw.trim_end().len();
        if trimmed_start >= trimmed_end {
            // Blank line: an empty comment with the caret inside.
            let edit = Edit::insert(range.start, format!("{open}  {close}"));
            return Some(EditSet {
                edits: vec![edit],
                caret: Some(range.start + open.len() + 1),
                selection: None,
            });
        }
        (trimmed_start, trimmed_end)
    } else {
        (range.start, range.end)
    };
    if !data.nested_block && text[start..end].contains(close) {
        return None;
    }
    let prefix = format!("{open} ");
    let suffix = format!(" {close}");
    let shift = prefix.len();
    Some(EditSet {
        edits: vec![Edit::insert(start, prefix), Edit::insert(end, suffix)],
        caret: range.is_empty().then(|| range.start.max(start) + shift),
        selection: (!range.is_empty()).then(|| start + shift..end + shift),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(id: &str, source: &str) -> SyntaxSnapshot {
        let spec = crate::registry::by_id(id).expect("language");
        SyntaxSnapshot::parse(Default::default(), Default::default(), spec, source.into())
            .expect("parse")
    }

    #[test]
    fn block_comments_wrap_and_unwrap() {
        let source = "int a = 1; int b = 2;\n";
        let snapshot = snap("java", source);
        let range = source.find("a = 1").unwrap()..source.find("; int b").unwrap();
        let set = toggle_block_comment(&snapshot, range).expect("wrap");
        let wrapped = set.apply(source);
        assert_eq!(wrapped, "int /* a = 1 */; int b = 2;\n");
        assert_eq!(&wrapped[set.selection.clone().unwrap()], "a = 1");
        let snapshot = snap("java", &wrapped);
        let at = wrapped.find("a = 1").unwrap();
        let back = toggle_block_comment(&snapshot, at..at + 5).expect("unwrap");
        assert_eq!(back.apply(&wrapped), source);
    }

    #[test]
    fn a_caret_comments_its_line_and_prefixes_ignore_strings() {
        let source = "    foo();\n";
        let snapshot = snap("java", source);
        let set = toggle_block_comment(&snapshot, 6..6).expect("line");
        assert_eq!(set.apply(source), "    /* foo(); */\n");
        let source = "String s = \"\"\"\n// not a comment\n\"\"\";\n";
        let snapshot = snap("java", source);
        let lines = LineIndex::new(source);
        assert!(!is_line_commented(&snapshot, &lines, 1, "//"));
        let source = "x(); // c\n// real\n";
        let snapshot = snap("java", source);
        let lines = LineIndex::new(source);
        assert!(is_line_commented(&snapshot, &lines, 1, "//"));
        assert!(!is_line_commented(&snapshot, &lines, 0, "//"));
    }

    #[test]
    fn injected_languages_choose_their_own_prefix() {
        let source = "<p>x</p>\n<script>\nlet a;\n</script>\n";
        let snapshot = snap("html", source);
        assert_eq!(line_prefix_at(&snapshot, 1), None);
        assert_eq!(
            line_prefix_at(&snapshot, source.find("let").unwrap()),
            Some("//")
        );
    }
}

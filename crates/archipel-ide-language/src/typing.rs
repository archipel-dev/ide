//! Typed editing (IntelliJ `TypedHandlerDelegate`, `EnterHandlerDelegate`):
//! decisions made from the syntax tree instead of character heuristics.
//!
//! - [`before_typed`]: overtype a closer, auto-pair brackets and quotes only
//!   in code (never inside strings and comments), wrap a selection.
//! - [`after_typed`]: re-indent a typed closing bracket, Python's `else:`,
//!   close an HTML/XML start tag, close a triple-quoted string.
//! - [`on_enter`]: indentation from `indents.scm`, `{|}` splitting, comment
//!   and doc-comment continuation.
//!
//! Every function is pure: it takes a snapshot of the document (for
//! [`after_typed`] the snapshot after the character was inserted) and returns
//! edit descriptions in byte offsets.

use crate::commenter::{comment_at, data_at, innermost};
use crate::edit::{Edit, EditSet, TypedDecision};
use crate::indent::{IndentEngine, IndentOptions};
use crate::lines::{LineIndex, indent_bytes, indent_columns};
use crate::node::SyntaxSnapshot;
use crate::syntax_data::LanguageData;
use std::ops::Range;

/// Where a caret sits syntactically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    Code,
    /// Strictly inside a string literal (between its delimiters).
    String,
    /// Inside a comment (after the comment opener).
    Comment,
}

/// The syntactic context at `caret`. Positions at a literal's boundaries
/// (just before the opening or after the closing quote) are code; the end of
/// a line comment still belongs to the comment.
pub fn context_at(snap: &SyntaxSnapshot, caret: usize) -> Context {
    let inner = innermost(snap, caret);
    let table = inner.lang().token_set_table();
    let root = inner.root();
    let probes = [caret.checked_sub(1), Some(caret)];
    for offset in probes.into_iter().flatten() {
        let Some(leaf) = u32::try_from(offset)
            .ok()
            .and_then(|o| root.find_leaf_at(o))
        else {
            continue;
        };
        for node in std::iter::once(leaf).chain(leaf.ancestors()) {
            let ty = crate::ElementType(node.raw().kind_id());
            let range = node.byte_range();
            if table.comments.contains(ty) {
                let text = node.text();
                let line_comment = !text.starts_with("/*") && !text.starts_with("<!--");
                let end_ok = if line_comment {
                    caret <= range.end
                } else {
                    caret < range.end && !(text.ends_with("*/") && caret + 2 > range.end)
                };
                if range.start < caret && end_ok {
                    return Context::Comment;
                }
            }
            if table.string_literals.contains(ty) && range.start < caret && caret < range.end {
                return Context::String;
            }
        }
    }
    Context::Code
}

fn is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// May a pair be inserted in front of `next`? Not before an identifier or
/// another quote.
fn next_allows_pair(next: Option<char>, typed: char) -> bool {
    match next {
        None => true,
        Some(ch) => !is_word(ch) && ch != typed && ch != '\\',
    }
}

fn string_end(snap: &SyntaxSnapshot, caret: usize) -> Option<usize> {
    let inner = innermost(snap, caret);
    let table = inner.lang().token_set_table();
    let leaf = inner.root().find_leaf_at(u32::try_from(caret).ok()?)?;
    std::iter::once(leaf)
        .chain(leaf.ancestors())
        .find(|node| {
            let range = node.byte_range();
            table
                .string_literals
                .contains(crate::ElementType(node.raw().kind_id()))
                && range.start < caret
                && caret < range.end
        })
        .map(|node| node.byte_range().end)
}

/// Language-specific quote prefixes (`r"..."`, `b'...'`, `f"..."`).
fn quote_prefix_allowed(lang: &str, before: &str) -> bool {
    if !matches!(lang, "python" | "starlark" | "rust" | "kotlin") {
        return false;
    }
    let letters: String = before
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_alphabetic())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    let preceded_by_word = before
        .strip_suffix(letters.as_str())
        .and_then(|rest| rest.chars().next_back())
        .is_some_and(is_word);
    !letters.is_empty()
        && letters.len() <= 2
        && !preceded_by_word
        && letters
            .chars()
            .all(|ch| matches!(ch.to_ascii_lowercase(), 'r' | 'b' | 'f' | 'u'))
}

/// Decision before `typed` is inserted at `caret`. `selection` is the
/// selected byte range when there is one (surround).
pub fn before_typed(
    snap: &SyntaxSnapshot,
    caret: usize,
    selection: Option<Range<usize>>,
    typed: char,
) -> TypedDecision {
    let data = data_at(snap, caret);
    let text = snap.text();
    let caret = caret.min(text.len());
    let opens = data.closing_bracket(typed);
    let is_quote = data.is_quote(typed);
    if selection.as_ref().is_some_and(|range| !range.is_empty()) {
        if let Some(close) = opens {
            return TypedDecision::Wrap { open: typed, close };
        }
        if is_quote {
            return TypedDecision::Wrap {
                open: typed,
                close: typed,
            };
        }
        return TypedDecision::Default;
    }
    let before = &text[..caret];
    let prev = before.chars().next_back();
    let next = text[caret..].chars().next();
    let context = context_at(snap, caret);
    if data.is_closing_bracket(typed) && next == Some(typed) && context != Context::Comment {
        return TypedDecision::Overtype;
    }
    if is_quote && next == Some(typed) {
        // Step over the closing quote of the string the caret is in.
        let closes_string =
            string_end(snap, caret).is_some_and(|end| end == caret + typed.len_utf8());
        let empty_pair = context == Context::String && prev == Some(typed);
        if closes_string || empty_pair {
            return TypedDecision::Overtype;
        }
    }
    if let Some(close) = opens {
        if context == Context::Code && next_allows_pair(next, typed) {
            return TypedDecision::InsertPair(close);
        }
        // `${` of a string template pairs inside the string.
        if context == Context::String
            && typed == '{'
            && prev == Some('$')
            && matches!(
                innermost(snap, caret).lang().id,
                "kotlin" | "groovy" | "javascript" | "typescript" | "tsx" | "bash"
            )
        {
            return TypedDecision::InsertPair(close);
        }
        return TypedDecision::Default;
    }
    if is_quote && context == Context::Code && next_allows_pair(next, typed) {
        let lang = innermost(snap, caret).lang().id;
        let apostrophe_like = match prev {
            None => false,
            Some(ch) if is_word(ch) => !quote_prefix_allowed(lang, before),
            // A lifetime (`&'a`, `<'a`) or an escaped quote.
            Some(ch) => {
                ch == '\\' || (lang == "rust" && typed == '\'' && matches!(ch, '&' | '<' | '\''))
            }
        };
        // The third quote of a triple-quoted string is typed plainly; the
        // closing triple follows in `after_typed`.
        let third = data.triple_quotes && before.ends_with(&format!("{typed}{typed}"));
        if !apostrophe_like && !third {
            return TypedDecision::InsertPair(typed);
        }
    }
    TypedDecision::Default
}

/// Words that dedent to the construct they continue (Python/Starlark).
const DEDENT_OWNERS: &[(&str, &[&str])] = &[
    ("else", &["if", "elif", "for", "while", "try", "except"]),
    ("elif", &["if", "elif"]),
    ("except", &["try", "except"]),
    ("finally", &["try", "except", "else"]),
];

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Edits after `typed` was inserted just before `caret`; `snap` already
/// holds the new text.
pub fn after_typed(
    snap: &SyntaxSnapshot,
    caret: usize,
    typed: char,
    opts: IndentOptions,
) -> Option<EditSet> {
    let text = snap.text();
    let data = data_at(snap, caret.saturating_sub(1));
    let typed_at = caret.checked_sub(typed.len_utf8())?;
    match typed {
        '}' | ')' | ']' if data.is_closing_bracket(typed) => {
            reindent_line(snap, typed_at, caret, opts)
        }
        ':' => dedent_python(snap, caret, opts),
        '>' => close_tag(snap, caret),
        '"' | '\'' if data.triple_quotes => {
            let triple: String = std::iter::repeat_n(typed, 3).collect();
            let before = &text[..caret];
            let exactly_three =
                before.ends_with(&triple) && !before[..before.len() - 3].ends_with(typed);
            (exactly_three
                && !text[caret..].starts_with(&triple)
                && !text[caret..].starts_with(typed))
            .then(|| EditSet {
                edits: vec![Edit::insert(caret, triple)],
                caret: Some(caret),
                selection: None,
            })
        }
        _ => None,
    }
}

/// Re-indents the line when `typed_at` is its first non-blank character.
fn reindent_line(
    snap: &SyntaxSnapshot,
    typed_at: usize,
    caret: usize,
    opts: IndentOptions,
) -> Option<EditSet> {
    let text = snap.text();
    let lines = LineIndex::new(text);
    let line = lines.line_of(typed_at);
    let range = lines.line_range(line, text);
    let first = range.start + indent_bytes(&text[range.clone()]);
    if first != typed_at || context_at(snap, typed_at) != Context::Code {
        return None;
    }
    let engine = IndentEngine::new(snap, opts);
    let target = engine.for_line(line, &|row| engine.indent_of(row))?;
    let current = engine.indent_of(line);
    if target == current {
        return None;
    }
    let replacement = opts.indent_string(target);
    let delta = replacement.len() as isize - (first - range.start) as isize;
    Some(EditSet {
        edits: vec![Edit::replace(range.start..first, replacement)],
        caret: Some(caret.saturating_add_signed(delta)),
        selection: None,
    })
}

/// `else:` typed at the start of a line aligns with its `if`.
fn dedent_python(snap: &SyntaxSnapshot, caret: usize, opts: IndentOptions) -> Option<EditSet> {
    if !matches!(snap.lang().id, "python" | "starlark") {
        return None;
    }
    let text = snap.text();
    let lines = LineIndex::new(text);
    let line = lines.line_of(caret.saturating_sub(1));
    let range = lines.line_range(line, text);
    let raw = &text[range.clone()];
    let word = raw.trim_start().split(|ch: char| !is_word(ch)).next()?;
    let (_, owners) = DEDENT_OWNERS.iter().find(|(name, _)| *name == word)?;
    if !raw.trim_end().ends_with(':')
        || context_at(snap, range.start + indent_bytes(raw)) != Context::Code
    {
        return None;
    }
    let current = indent_columns(raw, opts.tab_size);
    let target = (0..line).rev().find_map(|row| {
        let other = &text[lines.line_range(row, text)];
        if other.trim().is_empty() {
            return None;
        }
        let columns = indent_columns(other, opts.tab_size);
        let first = other.trim_start().split(|ch: char| !is_word(ch)).next()?;
        (columns <= current && owners.contains(&first)).then_some(columns)
    })?;
    if target == current {
        return None;
    }
    let first = range.start + indent_bytes(raw);
    let replacement = opts.indent_string(target);
    let delta = replacement.len() as isize - (first - range.start) as isize;
    Some(EditSet {
        edits: vec![Edit::replace(range.start..first, replacement)],
        caret: Some(caret.saturating_add_signed(delta)),
        selection: None,
    })
}

/// Typing `>` after `<name attr="x"` inserts `</name>`.
fn close_tag(snap: &SyntaxSnapshot, caret: usize) -> Option<EditSet> {
    let lang = innermost(snap, caret.saturating_sub(1)).lang().id;
    if !matches!(lang, "html" | "xml") {
        return None;
    }
    let text = snap.text();
    let gt = caret - 1;
    if context_at(snap, gt) != Context::Code {
        return None;
    }
    let open = text[..gt].rfind(['<', '>'])?;
    if text.as_bytes()[open] != b'<' {
        return None;
    }
    let tag = &text[open + 1..gt];
    if tag.ends_with('/') || tag.starts_with(['/', '!', '?']) {
        return None;
    }
    let name: String = tag
        .chars()
        .take_while(|ch| is_word(*ch) || matches!(ch, '-' | ':' | '.'))
        .collect();
    if name.is_empty()
        || (lang == "html" && VOID_ELEMENTS.contains(&name.to_ascii_lowercase().as_str()))
    {
        return None;
    }
    if context_at(snap, open + 1) != Context::Code {
        return None;
    }
    let closing = format!("</{name}>");
    if text[caret..].trim_start().starts_with(&closing) {
        return None;
    }
    Some(EditSet {
        edits: vec![Edit::insert(caret, closing)],
        caret: Some(caret),
        selection: None,
    })
}

fn leading_whitespace(line: &str) -> &str {
    &line[..indent_bytes(line)]
}

/// Enter at `range` (the selection, or an empty range at the caret):
/// the new line's indentation, `{|}` splitting and comment continuation.
/// `None` leaves Enter to the host's plain behavior (no `indents.scm`).
pub fn on_enter(
    snap: &SyntaxSnapshot,
    range: Range<usize>,
    opts: IndentOptions,
) -> Option<EditSet> {
    let text = snap.text();
    let start = range.start.min(text.len());
    let end = range.end.clamp(start, text.len());
    let data = data_at(snap, start);
    let lines = LineIndex::new(text);
    let line = lines.line_of(start);
    let line_range = lines.line_range(line, text);
    let line_text = &text[line_range.clone()];
    let ws = leading_whitespace(line_text);
    let context = context_at(snap, start);
    let finish = |inserted: String, caret_offset: usize| EditSet {
        edits: vec![Edit::replace(start..end, inserted)],
        caret: Some(start + caret_offset),
        selection: None,
    };
    if context == Context::Comment
        && let Some(set) = continue_comment(snap, data, start, end, ws, line_text, &line_range)
    {
        return Some(set);
    }
    if context == Context::Code
        && let Some(set) = open_block_comment(data, text, start, end, ws, line_text, &line_range)
    {
        return Some(set);
    }
    let engine = IndentEngine::new(snap, opts);
    if !engine.supported() {
        return None;
    }
    let current = engine.indent_of(line);
    // Between a bracket and its closer: split onto three lines.
    let prev = text[..start].chars().next_back();
    let next = text[end..].chars().next();
    if context == Context::Code
        && let (Some(open), Some(close)) = (prev, next)
        && data.closing_bracket(open) == Some(close)
    {
        let inner = engine.for_enter(start).max(current);
        let inner_ws = opts.indent_string(inner);
        let outer_ws = opts.indent_string(current);
        let middle = format!("\n{inner_ws}");
        let caret_offset = middle.len();
        return Some(finish(format!("{middle}\n{outer_ws}"), caret_offset));
    }
    // Between a start tag and its end tag.
    if matches!(data.id, "html" | "xml")
        && text[..start].ends_with('>')
        && text[end..].starts_with("</")
        && !text[..start].ends_with("/>")
    {
        let inner_ws = opts.indent_string(current + opts.indent_size);
        let outer_ws = opts.indent_string(current);
        let middle = format!("\n{inner_ws}");
        let caret_offset = middle.len();
        return Some(finish(format!("{middle}\n{outer_ws}"), caret_offset));
    }
    let columns = if context == Context::String {
        current
    } else {
        engine.for_enter(start)
    };
    let inserted = format!("\n{}", opts.indent_string(columns));
    let caret_offset = inserted.len();
    Some(finish(inserted, caret_offset))
}

/// Enter right after a freshly typed `/**` (the parser sees no comment yet):
/// continue with ` * ` and close the comment when nothing closes it later.
fn open_block_comment(
    data: &LanguageData,
    text: &str,
    start: usize,
    end: usize,
    ws: &str,
    line_text: &str,
    line_range: &Range<usize>,
) -> Option<EditSet> {
    let (open, close) = data.block_comment?;
    let trimmed = line_text.trim_start();
    if !trimmed.starts_with(open)
        || !text[..start].trim_end().ends_with(trimmed.trim_end())
        || !text[end..line_range.end].trim().is_empty()
        || trimmed.contains(close)
    {
        return None;
    }
    let base = format!("{ws} ");
    let lead = format!("\n{base}* ");
    let caret_offset = lead.len();
    let next_open = text[end..].find(open).unwrap_or(usize::MAX);
    let next_close = text[end..].find(close).unwrap_or(usize::MAX);
    let closed_later = next_close != usize::MAX && next_close < next_open;
    let inserted = if trimmed.starts_with("/**") && !closed_later {
        format!("{lead}\n{base}{close}")
    } else {
        lead
    };
    Some(EditSet {
        edits: vec![Edit::replace(start..end, inserted)],
        caret: Some(start + caret_offset),
        selection: None,
    })
}

/// Enter inside a comment: `//` and `///` continue, `/** */` continues with
/// ` * ` and an unterminated Javadoc gets its closing line.
fn continue_comment(
    snap: &SyntaxSnapshot,
    data: &LanguageData,
    start: usize,
    end: usize,
    ws: &str,
    line_text: &str,
    line_range: &Range<usize>,
) -> Option<EditSet> {
    let text = snap.text();
    let comment = comment_at(snap, start.saturating_sub(1))?;
    let body = &text[comment.clone()];
    let make = |inserted: String, caret_offset: usize| EditSet {
        edits: vec![Edit::replace(start..end, inserted)],
        caret: Some(start + caret_offset),
        selection: None,
    };
    if let Some((open, close)) = data.block_comment
        && body.starts_with(open)
    {
        if start <= comment.start + open.len().saturating_sub(1) {
            return None;
        }
        let trimmed = line_text.trim_start();
        let on_open_line = trimmed.starts_with(open);
        let star_line = trimmed.starts_with('*');
        if !(on_open_line || star_line) {
            return None;
        }
        let base = if on_open_line {
            format!("{ws} ")
        } else {
            ws.to_owned()
        };
        let lead = format!("\n{base}* ");
        let unterminated = !body.trim_end().ends_with(close);
        let at_line_end = text[end..line_range.end].trim().is_empty();
        if on_open_line && unterminated && at_line_end && body.starts_with("/**") {
            let caret_offset = lead.len();
            let inserted = format!("{lead}\n{base}{close}");
            return Some(make(inserted, caret_offset));
        }
        let caret_offset = lead.len();
        return Some(make(lead, caret_offset));
    }
    // Line comments.
    let line_start = line_range.start;
    let trimmed = line_text.trim_start();
    let prefix = data
        .doc_line_prefix
        .filter(|prefix| trimmed.starts_with(prefix))
        .or_else(|| {
            data.line_comments
                .iter()
                .copied()
                .filter(|prefix| trimmed.starts_with(prefix))
                .max_by_key(|prefix| prefix.len())
        })?;
    let prefix_end = line_start + indent_bytes(line_text) + prefix.len();
    if start < prefix_end {
        return None;
    }
    let is_doc = data.doc_line_prefix == Some(prefix);
    let rest_after_caret = !text[end..line_range.end].trim().is_empty();
    if !(is_doc || rest_after_caret) {
        return None;
    }
    let lead = format!("\n{ws}{prefix} ");
    let caret_offset = lead.len();
    Some(make(lead, caret_offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(id: &str, source: &str) -> SyntaxSnapshot {
        let spec = crate::registry::by_id(id).expect("language");
        SyntaxSnapshot::parse(Default::default(), Default::default(), spec, source.into())
            .expect("parse")
    }

    /// `|` marks the caret in `source`.
    fn caret(source: &str) -> (String, usize) {
        let at = source.find('|').expect("caret marker");
        (source.replacen('|', "", 1), at)
    }

    #[test]
    fn contexts_distinguish_code_strings_and_comments() {
        let (source, at) = caret("String s = \"ab|c\"; // note\n");
        let snapshot = snap("java", &source);
        assert_eq!(context_at(&snapshot, at), Context::String);
        let open = source.find('"').unwrap();
        assert_eq!(context_at(&snapshot, open), Context::Code);
        assert_eq!(context_at(&snapshot, open + 1), Context::String);
        let end_quote = source.rfind('"').unwrap();
        assert_eq!(context_at(&snapshot, end_quote + 1), Context::Code);
        let comment = source.find("note").unwrap();
        assert_eq!(context_at(&snapshot, comment), Context::Comment);
        assert_eq!(context_at(&snapshot, source.len() - 1), Context::Comment);
    }

    #[test]
    fn pairing_is_syntax_aware() {
        for (lang, source, typed, expected) in [
            ("java", "int x = |;\n", '(', TypedDecision::InsertPair(')')),
            ("java", "foo(|);\n", ')', TypedDecision::Overtype),
            ("java", "String s = \"a|b\";\n", '(', TypedDecision::Default),
            ("java", "// c |\n", '"', TypedDecision::Default),
            ("java", "int x = |foo;\n", '(', TypedDecision::Default),
            (
                "java",
                "String s = |;\n",
                '"',
                TypedDecision::InsertPair('"'),
            ),
            (
                "java",
                "String s = \"abc|\";\n",
                '"',
                TypedDecision::Overtype,
            ),
            ("java", "String s = \"a|b\";\n", '"', TypedDecision::Default),
            ("rust", "fn f<|>() {}\n", '\'', TypedDecision::Default),
            (
                "rust",
                "let c = |;\n",
                '\'',
                TypedDecision::InsertPair('\''),
            ),
            ("rust", "fn f(a: &|) {}\n", '\'', TypedDecision::Default),
            (
                "kotlin",
                "val s = \"$|\"\n",
                '{',
                TypedDecision::InsertPair('}'),
            ),
            ("python", "x = r|\n", '"', TypedDecision::InsertPair('"')),
            ("python", "x = it|\n", '\'', TypedDecision::Default),
            ("python", "x = \"\"|\n", '"', TypedDecision::Default),
        ] {
            let (text, at) = caret(source);
            let snapshot = snap(lang, &text);
            assert_eq!(
                before_typed(&snapshot, at, None, typed),
                expected,
                "{lang}: {source:?} typing {typed:?}"
            );
        }
        let (text, at) = caret("x = [|abc|];\n");
        let _ = at;
        let snapshot = snap("java", &text);
        let range = text.find("abc").unwrap()..text.find("abc").unwrap() + 3;
        assert_eq!(
            before_typed(&snapshot, range.end, Some(range), '('),
            TypedDecision::Wrap {
                open: '(',
                close: ')'
            }
        );
    }

    fn enter(lang: &str, source: &str) -> String {
        let (text, at) = caret(source);
        let snapshot = snap(lang, &text);
        let set = on_enter(&snapshot, at..at, IndentOptions::default()).expect("enter");
        let mut out = set.apply(&text);
        let caret = set.caret.expect("caret");
        out.insert(caret, '|');
        out
    }

    #[test]
    fn enter_indents_splits_pairs_and_continues_comments() {
        assert_eq!(
            enter("java", "class A {\n    void f() {|}\n}\n"),
            "class A {\n    void f() {\n        |\n    }\n}\n"
        );
        assert_eq!(
            enter(
                "java",
                "class A {\n    void f() {\n        g();|\n    }\n}\n"
            ),
            "class A {\n    void f() {\n        g();\n        |\n    }\n}\n"
        );
        assert_eq!(
            enter("java", "class A {\n    // note| more\n}\n"),
            "class A {\n    // note\n    // | more\n}\n"
        );
        assert_eq!(
            enter("java", "class A {\n    /**|\n}\n"),
            "class A {\n    /**\n     * |\n     */\n}\n"
        );
        assert_eq!(
            enter("java", "class A {\n    /**\n     * text|\n     */\n}\n"),
            "class A {\n    /**\n     * text\n     * |\n     */\n}\n"
        );
        assert_eq!(
            enter("rust", "/// doc|\nfn f() {}\n"),
            "/// doc\n/// |\nfn f() {}\n"
        );
        assert_eq!(
            enter("python", "def f():|\n    pass\n"),
            "def f():\n    |\n    pass\n"
        );
        assert_eq!(enter("html", "<div>|</div>\n"), "<div>\n    |\n</div>\n");
    }

    fn typed_after(lang: &str, source: &str, typed: char) -> String {
        let (text, at) = caret(source);
        let snapshot = snap(lang, &text);
        match after_typed(&snapshot, at, typed, IndentOptions::default()) {
            Some(set) => {
                let mut out = set.apply(&text);
                out.insert(set.caret.expect("caret"), '|');
                out
            }
            None => source.to_owned(),
        }
    }

    #[test]
    fn closing_characters_reindent_and_tags_close() {
        assert_eq!(
            typed_after(
                "java",
                "class A {\n    void f() {\n        g();\n        }|\n}\n",
                '}'
            ),
            "class A {\n    void f() {\n        g();\n    }|\n}\n"
        );
        assert_eq!(
            typed_after("python", "if a:\n    b\n    else:|\n", ':'),
            "if a:\n    b\nelse:|\n"
        );
        assert_eq!(
            typed_after("html", "<div class=\"a\">|\n", '>'),
            "<div class=\"a\">|</div>\n"
        );
        assert_eq!(typed_after("html", "<br>|\n", '>'), "<br>|\n");
        assert_eq!(
            typed_after("python", "x = \"\"\"|\n", '"'),
            "x = \"\"\"|\"\"\"\n"
        );
    }
}

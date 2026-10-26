//! Complete Current Statement (IntelliJ `SmartEnterProcessor`, Ctrl+Shift+Enter).
//!
//! Generic completion from the syntax tree and the language's
//! [`StatementStyle`]: close the brackets the parser reports MISSING (or that
//! have no partner), add the statement terminator, open the body of a control
//! statement or declaration header, and put the caret on the next line.

use crate::commenter::{comment_at, data_at};
use crate::edit::{Edit, EditSet};
use crate::indent::{IndentEngine, IndentOptions};
use crate::lines::{LineIndex, indent_bytes};
use crate::node::SyntaxSnapshot;
use crate::syntax_data::StatementStyle;
use crate::typing::{Context, context_at};

const BRACE_HEADERS: &[&str] = &[
    "if",
    "for",
    "while",
    "switch",
    "else",
    "try",
    "catch",
    "finally",
    "do",
    "synchronized",
    "foreach",
    "when",
    "match",
    "loop",
    "unsafe",
    "class",
    "interface",
    "enum",
    "struct",
    "impl",
    "trait",
    "mod",
    "fn",
    "fun",
    "func",
    "function",
    "namespace",
    "object",
    "async",
    "select",
    "case",
    "default",
];
/// Headers that never take a parenthesized condition (they open a block directly).
const BARE_HEADERS: &[&str] = &["else", "try", "finally", "do", "loop", "unsafe", "default"];
const INDENT_HEADERS: &[&str] = &[
    "if", "elif", "else", "for", "while", "try", "except", "finally", "with", "def", "class",
    "async", "match", "case", "lambda",
];

fn first_word(line: &str) -> &str {
    let trimmed = line.trim_start().trim_start_matches('}').trim_start();
    trimmed
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .next()
        .unwrap_or("")
}

/// Brackets the line leaves open, innermost last: MISSING closers reported by
/// the parser, else the line's `(`/`[` tokens without a partner.
fn missing_closers(
    snap: &SyntaxSnapshot,
    line_start: usize,
    line_end: usize,
    next_line: usize,
) -> String {
    let mut missing: Vec<(usize, char)> = snap
        .errors()
        .iter()
        .filter(|error| error.missing)
        .filter_map(|error| {
            let start = error.range.start as usize;
            let kind = error.message.strip_prefix("expected ")?;
            let ch = match kind {
                ")" => ')',
                "]" => ']',
                _ => return None,
            };
            (start >= line_start && start <= next_line).then_some((start, ch))
        })
        .collect();
    missing.sort_unstable();
    if !missing.is_empty() {
        return missing.into_iter().map(|(_, ch)| ch).collect();
    }
    // Fallback: unmatched openers on the line, closed in reverse order.
    let mut open: Vec<char> = Vec::new();
    for node in snap.root().preorder() {
        let range = node.byte_range();
        if range.start < line_start || range.start >= line_end || node.raw().child_count() > 0 {
            continue;
        }
        match node.kind() {
            "(" | "[" if crate::brackets::pair_at(snap, range.start).is_none() => {
                open.push(if node.kind() == "(" { ')' } else { ']' });
            }
            _ => {}
        }
    }
    open.into_iter().rev().collect()
}

/// Completes the statement on the line of `caret`; `None` when the language
/// has no statement structure or there is nothing to complete (the host falls
/// back to Start New Line).
pub fn smart_enter(snap: &SyntaxSnapshot, caret: usize, opts: IndentOptions) -> Option<EditSet> {
    let text = snap.text();
    let caret = caret.min(text.len());
    let data = data_at(snap, caret);
    let style = data.statements;
    if style == StatementStyle::None {
        return None;
    }
    let lines = LineIndex::new(text);
    let line = lines.line_of(caret);
    let range = lines.line_range(line, text);
    let raw = &text[range.clone()];
    if raw.trim().is_empty() || context_at(snap, range.start + indent_bytes(raw)) == Context::String
    {
        return None;
    }
    // The insertion point: the end of the code, before a trailing comment.
    let mut code_end = range.start + raw.trim_end().len();
    if let Some(comment) = comment_at(snap, code_end.saturating_sub(1))
        && comment.start >= range.start
        && comment.start > range.start + indent_bytes(raw)
    {
        code_end = range.start + raw[..comment.start - range.start].trim_end().len();
    }
    let code = &text[range.start..code_end];
    let indent_cols = crate::lines::indent_columns(raw, opts.tab_size);
    let engine = IndentEngine::new(snap, opts);
    let outer = opts.indent_string(indent_cols);
    let inner = opts.indent_string(indent_cols + opts.indent_size);
    let closers = missing_closers(snap, range.start, range.end, lines.line_start(line + 1));
    let word = first_word(code);
    let _ = &engine;
    let done = |inserted: String, caret_offset: usize| EditSet {
        edits: vec![Edit::insert(code_end, inserted)],
        caret: Some(code_end + caret_offset),
        selection: None,
    };
    match style {
        StatementStyle::Braces { semicolon } => {
            let header = BRACE_HEADERS.contains(&word);
            let ends_with_open = code.trim_end().ends_with('{');
            let has_body = ends_with_open || code.contains('{') && code.trim_end().ends_with('}');
            if header && !has_body {
                // `if (x)` -> `if (x) {` + body + `}`; a bare `else` too.
                let needs_condition = !BARE_HEADERS.contains(&word);
                let _ = needs_condition;
                let open = format!("{closers} {{");
                let middle = format!("\n{inner}");
                let caret_offset = open.len() + middle.len();
                return Some(done(format!("{open}{middle}\n{outer}}}"), caret_offset));
            }
            if ends_with_open && !has_closer_for_open(snap, code_end - 1) {
                let middle = format!("\n{inner}");
                let caret_offset = closers.len() + middle.len();
                return Some(done(format!("{closers}{middle}\n{outer}}}"), caret_offset));
            }
            let last = code.trim_end().chars().next_back()?;
            let terminated = matches!(last, ';' | '{' | '}' | ',' | ':');
            let add_semicolon = semicolon && !terminated;
            if closers.is_empty() && !add_semicolon {
                // Already complete: open a line below.
                let next = format!("\n{}", opts.indent_string(engine.for_enter(code_end)));
                let caret_offset = next.len();
                return Some(done(next, caret_offset));
            }
            let terminator = if add_semicolon { ";" } else { "" };
            let next = format!("\n{outer}");
            let inserted = format!("{closers}{terminator}{next}");
            let caret_offset = inserted.len();
            Some(done(inserted, caret_offset))
        }
        StatementStyle::Indent => {
            let header = INDENT_HEADERS.contains(&word);
            let ends_with_colon = code.trim_end().ends_with(':');
            if header && !ends_with_colon {
                let inserted = format!("{closers}:\n{inner}");
                let caret_offset = inserted.len();
                return Some(done(inserted, caret_offset));
            }
            let next = if ends_with_colon {
                format!("\n{inner}")
            } else {
                format!("\n{outer}")
            };
            let inserted = format!("{closers}{next}");
            let caret_offset = inserted.len();
            Some(done(inserted, caret_offset))
        }
        StatementStyle::Shell => {
            let (opener, closer) = match word {
                "if" => ("; then", Some("fi")),
                "elif" => ("; then", None),
                "for" | "while" | "until" | "select" => ("; do", Some("done")),
                "case" => (" in", Some("esac")),
                _ => ("", None),
            };
            if opener.is_empty() {
                let next = format!("\n{outer}");
                let inserted = format!("{closers}{next}");
                let caret_offset = inserted.len();
                return Some(done(inserted, caret_offset));
            }
            let already = code.contains(" then") || code.contains(" do") || code.ends_with(" in");
            let head = if already {
                String::new()
            } else {
                opener.to_owned()
            };
            let middle = format!("\n{inner}");
            let tail = closer
                .map(|closer| format!("\n{outer}{closer}"))
                .unwrap_or_default();
            let caret_offset = closers.len() + head.len() + middle.len();
            Some(done(format!("{closers}{head}{middle}{tail}"), caret_offset))
        }
        StatementStyle::None => None,
    }
}

/// Does the `{` at `open` already have a closing `}`?
fn has_closer_for_open(snap: &SyntaxSnapshot, open: usize) -> bool {
    crate::brackets::pair_at(snap, open).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(id: &str, source: &str) -> SyntaxSnapshot {
        let spec = crate::registry::by_id(id).expect("language");
        SyntaxSnapshot::parse(Default::default(), Default::default(), spec, source.into())
            .expect("parse")
    }

    fn complete(lang: &str, source: &str) -> String {
        let at = source.find('|').expect("caret");
        let text = source.replacen('|', "", 1);
        let snapshot = snap(lang, &text);
        let options = IndentOptions::detect(&text);
        let set = smart_enter(&snapshot, at, options).expect("completion");
        let mut out = set.apply(&text);
        out.insert(set.caret.expect("caret"), '|');
        out
    }

    #[test]
    fn statements_get_closers_semicolons_and_blocks() {
        assert_eq!(
            complete(
                "java",
                "class A {\n    void f() {\n        foo(bar(1, 2|\n    }\n}\n"
            ),
            "class A {\n    void f() {\n        foo(bar(1, 2));\n        |\n    }\n}\n"
        );
        assert_eq!(
            complete(
                "java",
                "class A {\n    void f() {\n        if (x|)\n    }\n}\n"
            ),
            "class A {\n    void f() {\n        if (x) {\n            |\n        }\n    }\n}\n"
        );
        assert_eq!(
            complete("rust", "fn f() {\n    let x = g(1|\n}\n"),
            "fn f() {\n    let x = g(1);\n    |\n}\n"
        );
        assert_eq!(
            complete("python", "def f(a|\n    pass\n"),
            "def f(a):\n    |\n    pass\n"
        );
        assert_eq!(
            complete("go", "func main() {\n\tfor i := 0; i < 3; i++|\n}\n"),
            "func main() {\n\tfor i := 0; i < 3; i++ {\n\t\t|\n\t}\n}\n"
        );
        assert_eq!(
            complete("bash", "if [ -z \"$a\" ]|\n"),
            "if [ -z \"$a\" ]; then\n    |\nfi\n"
        );
    }

    #[test]
    fn data_languages_have_no_completion() {
        let snapshot = snap("json", "{\"a\": 1}\n");
        assert!(smart_enter(&snapshot, 3, IndentOptions::default()).is_none());
    }
}

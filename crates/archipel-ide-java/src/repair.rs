//! Error-tolerant analysis input (plan 2.6, JAVA-ERR-002 "per-reference tolerance").
//!
//! A buffer that is being typed is syntactically broken: a statement without its `;`, a call
//! whose `)` is not there yet. The tree-sitter parser then often swallows the whole class into
//! an error node, and nothing of the file can be resolved. Before analysis the text is therefore
//! *repaired*: at the end of a line where the next line cannot continue the statement, the
//! missing closing tokens are inserted (`)`, `;`, a placeholder after a trailing `.`). The
//! repaired text is parsed and analyzed instead; positions before an insertion are the same in
//! both texts, later ones are shifted, and a problem that touches inserted text is dropped.
//!
//! The repair is only used when it makes the parse strictly better (less error text); otherwise
//! the original tree stays. It never reports anything by itself: it only lets the resolver see
//! the intact parts of the file.

use crate::source;
use std::ops::Range;
use tree_sitter::{Node, Tree};

/// The text and tree an analysis runs on, with the map back to the buffer.
pub struct Prepared {
    pub text: String,
    pub tree: Tree,
    /// `(offset in the buffer, inserted length)`, ascending.
    inserts: Vec<(usize, usize)>,
}

impl Prepared {
    pub fn repaired(&self) -> bool {
        !self.inserts.is_empty()
    }

    /// Maps an offset of the repaired text to the buffer.
    pub fn original_offset(&self, offset: usize) -> usize {
        let mut shift = 0;
        for &(at, len) in &self.inserts {
            let at_repaired = at + shift;
            if offset >= at_repaired + len {
                shift += len;
            } else if offset > at_repaired {
                return at;
            } else {
                break;
            }
        }
        offset - shift
    }

    /// Whether `range` (of the repaired text) overlaps text that was inserted.
    pub fn touches_insert(&self, range: &Range<u32>) -> bool {
        let mut shift = 0;
        for &(at, len) in &self.inserts {
            let start = at + shift;
            let end = start + len;
            if (range.start as usize) < end && (range.end as usize) > start {
                return true;
            }
            // An empty range inside the inserted text.
            if range.start == range.end
                && (range.start as usize) > start
                && (range.start as usize) < end
            {
                return true;
            }
            shift += len;
        }
        false
    }

    pub fn original_range(&self, range: &Range<u32>) -> Range<u32> {
        let s = self.original_offset(range.start as usize) as u32;
        let e = (self.original_offset(range.end as usize) as u32).max(s);
        s..e
    }
}

/// Parses `text` (or takes `tree` when the host already did) and repairs it when the parse has
/// errors and a repair improves it.
pub fn prepare(text: &str, tree: Option<&Tree>) -> Option<Prepared> {
    let own;
    let tree = match tree {
        Some(t) => t,
        None => {
            own = source::parse(text)?;
            &own
        }
    };
    if tree.root_node().has_error()
        && let Some(p) = try_repair(text, tree)
    {
        return Some(p);
    }
    Some(Prepared {
        text: text.to_string(),
        tree: tree.clone(),
        inserts: Vec::new(),
    })
}

/// Error text and missing tokens of a tree (lower is better).
fn error_score(root: Node<'_>) -> usize {
    let mut score = 0;
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if n.is_missing() {
            score += 50;
        } else if n.is_error() {
            score += n.byte_range().len().max(1);
            continue;
        }
        if n.has_error() {
            let mut c = n.walk();
            stack.extend(n.children(&mut c));
        }
    }
    score
}

fn try_repair(text: &str, tree: &Tree) -> Option<Prepared> {
    let inserts = candidates(text);
    if inserts.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(text.len() + 16);
    let mut last = 0;
    let mut recorded = Vec::new();
    for (at, add) in &inserts {
        out.push_str(&text[last..*at]);
        out.push_str(add);
        last = *at;
        recorded.push((*at, add.len()));
    }
    out.push_str(&text[last..]);
    let repaired = source::parse(&out)?;
    if error_score(repaired.root_node()) >= error_score(tree.root_node()) {
        return None;
    }
    Some(Prepared {
        text: out,
        tree: repaired,
        inserts: recorded,
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum K {
    Ident,
    Keyword,
    Literal,
    Punct(u8),
    Arrow,
}

#[derive(Clone, Debug)]
struct Tok<'a> {
    kind: K,
    text: &'a str,
    start: usize,
    end: usize,
    line: usize,
}

const KEYWORDS: &[&str] = &[
    "abstract",
    "assert",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extends",
    "final",
    "finally",
    "float",
    "for",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "native",
    "new",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "strictfp",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "try",
    "void",
    "volatile",
    "while",
    "null",
    "true",
    "false",
];

fn lex(text: &str) -> Vec<Tok<'_>> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 0;
    while i < b.len() {
        let c = b[i];
        match c {
            b'\n' => {
                line += 1;
                i += 1;
            }
            c if c.is_ascii_whitespace() => i += 1,
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                    if b[i] == b'\n' {
                        line += 1;
                    }
                    i += 1;
                }
                i = (i + 2).min(b.len());
            }
            b'"' => {
                let start = i;
                let start_line = line;
                if text[i..].starts_with("\"\"\"") {
                    i += 3;
                    while i < b.len() && !text[i..].starts_with("\"\"\"") {
                        if b[i] == b'\\' {
                            i += 1;
                        }
                        if i < b.len() && b[i] == b'\n' {
                            line += 1;
                        }
                        i += 1;
                    }
                    i = (i + 3).min(b.len());
                } else {
                    i += 1;
                    while i < b.len() && b[i] != b'"' && b[i] != b'\n' {
                        if b[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                    if i < b.len() && b[i] == b'"' {
                        i += 1;
                    }
                }
                let i2 = i.min(b.len());
                out.push(Tok {
                    kind: K::Literal,
                    text: &text[start..i2],
                    start,
                    end: i2,
                    line: start_line,
                });
                i = i2;
            }
            b'\'' => {
                let start = i;
                i += 1;
                while i < b.len() && b[i] != b'\'' && b[i] != b'\n' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                if i < b.len() && b[i] == b'\'' {
                    i += 1;
                }
                let i2 = i.min(b.len());
                out.push(Tok {
                    kind: K::Literal,
                    text: &text[start..i2],
                    start,
                    end: i2,
                    line,
                });
                i = i2;
            }
            c if c.is_ascii_alphabetic() || c == b'_' || c == b'$' || c >= 0x80 => {
                let start = i;
                while i < b.len()
                    && (b[i].is_ascii_alphanumeric()
                        || b[i] == b'_'
                        || b[i] == b'$'
                        || b[i] >= 0x80)
                {
                    i += 1;
                }
                let t = &text[start..i];
                let kind = if KEYWORDS.contains(&t) {
                    if matches!(t, "null" | "true" | "false") {
                        K::Literal
                    } else {
                        K::Keyword
                    }
                } else {
                    K::Ident
                };
                out.push(Tok {
                    kind,
                    text: t,
                    start,
                    end: i,
                    line,
                });
            }
            c if c.is_ascii_digit() => {
                let start = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'.')
                {
                    // `1.foo` is not a thing; a dot followed by a non-digit ends the number.
                    if b[i] == b'.' && !b.get(i + 1).is_some_and(u8::is_ascii_digit) {
                        break;
                    }
                    i += 1;
                }
                out.push(Tok {
                    kind: K::Literal,
                    text: &text[start..i],
                    start,
                    end: i,
                    line,
                });
            }
            b'-' if b.get(i + 1) == Some(&b'>') => {
                out.push(Tok {
                    kind: K::Arrow,
                    text: "->",
                    start: i,
                    end: i + 2,
                    line,
                });
                i += 2;
            }
            _ => {
                out.push(Tok {
                    kind: K::Punct(c),
                    text: &text[i..i + 1],
                    start: i,
                    end: i + 1,
                    line,
                });
                i += 1;
            }
        }
    }
    out
}

/// `new Foo` / `new a.B<String>` at the end of `toks`: the argument list is still to come.
fn new_without_arguments(toks: &[Tok<'_>]) -> bool {
    let Some(last) = toks.last() else {
        return false;
    };
    if !matches!(last.kind, K::Ident | K::Punct(b'>')) {
        return false;
    }
    for t in toks.iter().rev() {
        match t.kind {
            K::Ident => {}
            K::Punct(b'.' | b'<' | b'>' | b',' | b'?') => {}
            K::Keyword if t.text == "new" => return true,
            K::Keyword if matches!(t.text, "extends" | "super") => {}
            _ => return false,
        }
    }
    false
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Brace {
    /// Class, method, block, lambda body.
    Code,
    /// Array initializer or annotation array value: elements, no statements.
    Init,
    /// Enum constants come first: lines end without `;` until the first `;`.
    Enum,
}

/// Where to insert what (ascending offsets).
fn candidates(text: &str) -> Vec<(usize, String)> {
    let toks = lex(text);
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut parens: Vec<Option<&str>> = Vec::new(); // owner token text of each open `(`
    let mut brackets = 0usize;
    let mut braces: Vec<Brace> = Vec::new();
    let mut enum_seen_semicolon: Vec<bool> = Vec::new();
    let mut last_close_owner: Option<&str> = None;
    let mut pending_enum = false;
    let mut line_start_tok = 0usize;
    for (i, t) in toks.iter().enumerate() {
        // Track nesting.
        match t.kind {
            K::Punct(b'(') => {
                let owner = i.checked_sub(1).map(|p| toks[p].text);
                parens.push(owner);
            }
            K::Punct(b')') => {
                last_close_owner = parens.pop().flatten();
            }
            K::Punct(b'[') => brackets += 1,
            K::Punct(b']') => brackets = brackets.saturating_sub(1),
            K::Punct(b'{') => {
                let prev = i.checked_sub(1).map(|p| &toks[p]);
                let kind = if pending_enum {
                    pending_enum = false;
                    Brace::Enum
                } else if prev.is_some_and(|p| {
                    matches!(
                        p.kind,
                        K::Punct(b'=') | K::Punct(b',') | K::Punct(b'{') | K::Punct(b']')
                    ) || (p.kind == K::Punct(b'(') && !parens.is_empty())
                }) {
                    Brace::Init
                } else {
                    Brace::Code
                };
                braces.push(kind);
                enum_seen_semicolon.push(false);
            }
            K::Punct(b'}') => {
                braces.pop();
                enum_seen_semicolon.pop();
            }
            K::Punct(b';') => {
                if let (Some(Brace::Enum), Some(s)) =
                    (braces.last(), enum_seen_semicolon.last_mut())
                {
                    *s = true;
                }
            }
            K::Keyword if t.text == "enum" => pending_enum = true,
            _ => {}
        }
        let next = toks.get(i + 1);
        let ends_line = next.is_none_or(|n| n.line != t.line);
        if !ends_line {
            continue;
        }
        let first = &toks[line_start_tok..=i];
        line_start_tok = i + 1;
        let first = &first[0];
        let in_init = matches!(braces.last(), Some(Brace::Init));
        let in_enum_constants = matches!(braces.last(), Some(Brace::Enum))
            && !enum_seen_semicolon.last().copied().unwrap_or(false);
        if in_init || in_enum_constants || braces.is_empty() && t.kind != K::Punct(b'.') {
            // Top level: only a missing `;` after package/import statements can be fixed.
            if !(braces.is_empty() && matches!(first.text, "package" | "import")) {
                continue;
            }
        }
        if first.kind == K::Punct(b'@') {
            continue;
        }
        let next_first = next;
        // What the next line starts with.
        let next_ok = match next_first {
            None => true,
            Some(n) => match n.kind {
                K::Punct(b'}') => true,
                K::Ident => true,
                K::Keyword => !matches!(
                    n.text,
                    "else"
                        | "catch"
                        | "finally"
                        | "instanceof"
                        | "extends"
                        | "implements"
                        | "throws"
                ),
                K::Punct(b'@') => true,
                _ => false,
            },
        };
        if !next_ok {
            continue;
        }
        let depth = parens.len();
        let last_is_eligible_end = match t.kind {
            K::Ident | K::Literal => true,
            // `return` and friends may continue on the next line: only before a closing brace.
            K::Keyword => {
                matches!(t.text, "this" | "super" | "break" | "continue" | "return")
                    && matches!(next_first.map(|n| n.kind), Some(K::Punct(b'}')) | None)
            }
            K::Punct(b')') | K::Punct(b']') => true,
            K::Punct(b'+') | K::Punct(b'-') => {
                // `i++` / `i--`
                i > 0 && toks[i - 1].kind == t.kind && toks[i - 1].end == t.start
            }
            _ => false,
        };
        if t.kind == K::Punct(b'.') && depth == 0 && brackets == 0 {
            if matches!(next_first.map(|n| n.kind), Some(K::Punct(b'}')) | None) {
                out.push((t.end, "_;".to_string()));
            }
            continue;
        }
        if depth > 0 {
            // `foo(` then the block ends: close the call.
            if brackets == 0
                && matches!(next_first.map(|n| n.kind), Some(K::Punct(b'}')))
                && (last_is_eligible_end || t.kind == K::Punct(b'('))
            {
                out.push((t.end, format!("{};", ")".repeat(depth))));
            }
            continue;
        }
        if brackets > 0 {
            continue;
        }
        if new_without_arguments(&toks[..=i]) {
            out.push((t.end, "();".to_string()));
            continue;
        }
        if !last_is_eligible_end {
            continue;
        }
        // `if (x)` / `for (...)` / `while (..)` head: the statement is on the next line.
        if t.kind == K::Punct(b')')
            && matches!(
                last_close_owner,
                Some("if" | "for" | "while" | "switch" | "catch" | "synchronized" | "try")
            )
        {
            continue;
        }
        // A switch label is not a statement.
        if matches!(first.text, "case" | "default") {
            continue;
        }
        out.push((t.end, ";".to_string()));
    }
    out.sort_by_key(|(at, _)| *at);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repaired(text: &str) -> Option<String> {
        let p = prepare(text, None)?;
        p.repaired().then_some(p.text)
    }

    #[test]
    fn missing_semicolon_before_the_closing_brace() {
        let t = "class M {\n  void run() {\n    List<String> more = new ArrayLi\n  }\n}\n";
        let r = repaired(t).expect("repaired");
        assert!(r.contains("new ArrayLi();\n"), "{r}");
        let p = prepare(t, None).unwrap();
        assert!(!p.tree.root_node().has_error());
        let t = "class M {\n  void run() {\n    int x = y\n  }\n}\n";
        assert!(repaired(t).expect("repaired").contains("int x = y;\n"));
    }

    #[test]
    fn open_call_is_closed() {
        let t = "class M {\n  void run() {\n    Object o = new ArrayLi(\n  }\n}\n";
        let r = repaired(t).expect("repaired");
        assert!(r.contains("new ArrayLi();\n"), "{r}");
    }

    #[test]
    fn missing_semicolon_between_statements() {
        let t = "class M {\n  void run() {\n    foo()\n    bar();\n  }\n}\n";
        let r = repaired(t).expect("repaired");
        assert!(r.contains("foo();\n"), "{r}");
    }

    #[test]
    fn braceless_if_and_array_initializers_stay() {
        let t = "class M {\n  int[] a = {\n    1,\n    2\n  };\n  void run(boolean b) {\n    if (b)\n      foo();\n    else\n      bar();\n  }\n}\n";
        assert!(repaired(t).is_none());
        let p = prepare(t, None).unwrap();
        assert!(!p.repaired());
    }

    #[test]
    fn enum_constants_stay() {
        let t = "enum E {\n  A,\n  B\n}\n";
        assert!(!prepare(t, None).unwrap().repaired());
    }

    #[test]
    fn positions_map_back() {
        let t = "class M {\n  void run() {\n    a()\n    bar();\n  }\n}\n";
        let p = prepare(t, None).unwrap();
        assert!(p.repaired());
        let at = p.text.find("bar").unwrap() as u32;
        let orig = t.find("bar").unwrap() as u32;
        assert_eq!(p.original_range(&(at..at + 3)), orig..orig + 3);
        let semi = p.text.find(";\n    bar").unwrap() as u32;
        assert!(p.touches_insert(&(semi..semi + 1)));
        assert!(!p.touches_insert(&(at..at + 3)));
    }
}

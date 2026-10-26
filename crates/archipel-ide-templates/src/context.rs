//! Source context at a caret: language, statement/expression/declaration
//! position and the enclosing class and method. A small brace scanner, not a
//! parser: it skips comments and string literals and tracks `{}` and `()`.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Where a template may be expanded (the "Context" column in the reference IDE).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Context {
    JavaStatement,
    JavaExpression,
    JavaDeclaration,
    /// Anywhere in Kotlin code (outside comments and strings).
    Kotlin,
    /// Anywhere in Rust code (outside comments and strings).
    Rust,
    /// Every file, including comments and strings.
    Other,
}

impl Context {
    pub const ALL: [Self; 6] = [
        Self::JavaStatement,
        Self::JavaExpression,
        Self::JavaDeclaration,
        Self::Kotlin,
        Self::Rust,
        Self::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::JavaStatement => "Java: statement",
            Self::JavaExpression => "Java: expression",
            Self::JavaDeclaration => "Java: declaration",
            Self::Kotlin => "Kotlin",
            Self::Rust => "Rust",
            Self::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Java,
    Kotlin,
    Rust,
    Other,
}

impl Language {
    pub fn of(path: &Path) -> Self {
        match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("java") => Self::Java,
            Some("kt" | "kts") => Self::Kotlin,
            Some("rs") => Self::Rust,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameKind {
    Type,
    Block,
}

#[derive(Debug, Clone)]
struct Frame {
    kind: FrameKind,
    header: String,
    parens: usize,
}

/// What the scanner learned about the text before the caret.
#[derive(Debug, Clone, Default)]
pub struct Scan {
    frames: Vec<Frame>,
    /// Text since the last `;`, `{` or `}` at parenthesis depth zero.
    segment: String,
    parens: usize,
    /// The caret is inside a comment or string literal.
    pub in_literal: bool,
}

const TYPE_KEYWORDS: [&str; 6] = ["class", "interface", "enum", "record", "object", "trait"];
const NOT_METHODS: [&str; 9] = [
    "if",
    "for",
    "while",
    "switch",
    "catch",
    "synchronized",
    "try",
    "when",
    "return",
];

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '@'))
        .filter(|word| !word.is_empty())
}

/// Scans `text` (everything before the caret).
pub fn scan(text: &str) -> Scan {
    #[derive(PartialEq)]
    enum State {
        Code,
        Line,
        Block,
        Str(char),
        TextBlock,
    }
    let chars: Vec<char> = text.chars().collect();
    let mut scan = Scan::default();
    let mut state = State::Code;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match state {
            State::Line => {
                if c == '\n' {
                    state = State::Code;
                }
            }
            State::Block => {
                if c == '*' && next == Some('/') {
                    state = State::Code;
                    i += 1;
                }
            }
            State::Str(quote) => {
                if c == '\\' {
                    i += 1;
                } else if c == quote || c == '\n' {
                    state = State::Code;
                }
            }
            State::TextBlock => {
                if c == '"' && next == Some('"') && chars.get(i + 2) == Some(&'"') {
                    state = State::Code;
                    i += 2;
                }
            }
            State::Code => match c {
                '/' if next == Some('/') => state = State::Line,
                '/' if next == Some('*') => {
                    state = State::Block;
                    i += 1;
                }
                '"' if next == Some('"') && chars.get(i + 2) == Some(&'"') => {
                    state = State::TextBlock;
                    i += 2;
                    scan.segment.push_str("\"\"");
                }
                '"' => {
                    state = State::Str('"');
                    scan.segment.push('"');
                }
                // Character literal, not a Rust lifetime (`'a`).
                '\'' if next == Some('\\') || chars.get(i + 2) == Some(&'\'') => {
                    state = State::Str('\'');
                    scan.segment.push('\'');
                }
                '(' | '[' => {
                    scan.parens += 1;
                    scan.segment.push(c);
                }
                ')' | ']' => {
                    scan.parens = scan.parens.saturating_sub(1);
                    scan.segment.push(c);
                }
                '{' => {
                    let header = std::mem::take(&mut scan.segment).trim().to_owned();
                    let kind = if words(&header).any(|word| TYPE_KEYWORDS.contains(&word)) {
                        FrameKind::Type
                    } else {
                        FrameKind::Block
                    };
                    scan.frames.push(Frame {
                        kind,
                        header,
                        parens: scan.parens,
                    });
                }
                '}' => {
                    if let Some(frame) = scan.frames.pop() {
                        scan.parens = frame.parens;
                    }
                    scan.segment.clear();
                }
                ';' if scan.parens == 0 => scan.segment.clear(),
                _ => scan.segment.push(c),
            },
        }
        i += 1;
    }
    scan.in_literal = state != State::Code;
    scan
}

impl Scan {
    /// Java position class at the caret.
    fn java_context(&self) -> Option<Context> {
        if self.in_literal {
            return None;
        }
        let segment = self.segment.trim();
        let expression_start = segment.ends_with([
            '=', '(', ',', '+', '-', '*', '/', '?', '&', '|', '!', '<', '>',
        ]) || segment.ends_with("return")
            || self.parens > self.frames.last().map_or(0, |frame| frame.parens);
        match self.frames.last() {
            None => Some(Context::JavaDeclaration),
            Some(frame) if frame.kind == FrameKind::Type => {
                if expression_start || segment.contains('=') {
                    Some(Context::JavaExpression)
                } else {
                    Some(Context::JavaDeclaration)
                }
            }
            Some(_) => {
                if expression_start {
                    Some(Context::JavaExpression)
                } else {
                    Some(Context::JavaStatement)
                }
            }
        }
    }

    /// Name of the innermost enclosing type.
    pub fn class_name(&self) -> Option<String> {
        self.frames
            .iter()
            .rev()
            .filter(|frame| frame.kind == FrameKind::Type)
            .find_map(|frame| {
                let mut words = words(&frame.header);
                words.find(|word| TYPE_KEYWORDS.contains(word))?;
                words.next().map(str::to_owned)
            })
    }

    /// Name and parameter names of the innermost enclosing method or function.
    pub fn method(&self) -> Option<(String, Vec<String>)> {
        self.frames.iter().rev().find_map(|frame| {
            if frame.kind == FrameKind::Type {
                return None;
            }
            let header = &frame.header;
            let close = header.rfind(')')?;
            let mut depth = 0usize;
            let mut open = None;
            for (index, c) in header[..=close].char_indices().rev() {
                match c {
                    ')' => depth += 1,
                    '(' => {
                        depth -= 1;
                        if depth == 0 {
                            open = Some(index);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let open = open?;
            let name = words(&header[..open]).last()?;
            if NOT_METHODS.contains(&name) || name == "new" {
                return None;
            }
            // `if`/`for` bodies are blocks too: require a declaration-like header.
            let before = header[..open].trim_end().strip_suffix(name)?.trim_end();
            let declaration = before.is_empty()
                || before
                    .ends_with(|c: char| c.is_alphanumeric() || matches!(c, '_' | '>' | ']' | '?'));
            if !declaration || before.ends_with("new") || header[close..].contains("->") {
                return None;
            }
            let mut params = Vec::new();
            let mut depth = 0usize;
            let mut current = String::new();
            for c in header[open + 1..close].chars().chain([',']) {
                match c {
                    '<' | '(' | '[' => {
                        depth += 1;
                        current.push(c);
                    }
                    '>' | ')' | ']' => {
                        depth = depth.saturating_sub(1);
                        current.push(c);
                    }
                    ',' if depth == 0 => {
                        let param = std::mem::take(&mut current);
                        // `String name` (Java) or `name: String` (Kotlin, Rust).
                        let name = match param.split_once(':') {
                            Some((name, _)) => name.trim().trim_start_matches("mut ").trim(),
                            None => words(&param).last().unwrap_or_default(),
                        };
                        if !name.is_empty() && name != "self" {
                            params.push(name.to_owned());
                        }
                    }
                    _ => current.push(c),
                }
            }
            Some((name.to_owned(), params))
        })
    }
}

/// The contexts that hold at `offset` (chars) of `text` for a file at `path`.
/// `Other` always holds.
pub fn contexts_at(path: &Path, text: &str, offset: usize) -> Vec<Context> {
    let byte = text
        .char_indices()
        .nth(offset)
        .map_or(text.len(), |(byte, _)| byte);
    let before = &text[..byte];
    let mut contexts = Vec::new();
    match Language::of(path) {
        Language::Java => contexts.extend(scan(before).java_context()),
        Language::Kotlin => {
            if !scan(before).in_literal {
                contexts.push(Context::Kotlin);
            }
        }
        Language::Rust => {
            if !scan(before).in_literal {
                contexts.push(Context::Rust);
            }
        }
        Language::Other => {}
    }
    contexts.push(Context::Other);
    contexts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(path: &str, text: &str) -> Vec<Context> {
        contexts_at(Path::new(path), text, text.chars().count())
    }

    #[test]
    fn java_positions() {
        use Context::*;
        assert_eq!(at("A.java", ""), [JavaDeclaration, Other]);
        assert_eq!(at("A.java", "class A {\n    "), [JavaDeclaration, Other]);
        assert_eq!(
            at("A.java", "class A {\n    int x = "),
            [JavaExpression, Other]
        );
        assert_eq!(
            at("A.java", "class A {\n    void f() {\n        "),
            [JavaStatement, Other]
        );
        assert_eq!(
            at(
                "A.java",
                "class A {\n    void f() {\n        int x = 1;\n        foo("
            ),
            [JavaExpression, Other]
        );
        assert_eq!(
            at(
                "A.java",
                "class A {\n    void f() {\n        if (x) {}\n        "
            ),
            [JavaStatement, Other]
        );
        assert_eq!(
            at("A.java", "class A {\n    void f() {\n        // todo "),
            [Other]
        );
        assert_eq!(
            at("A.java", "class A { String s = \"{ "),
            [Other],
            "string literal"
        );
        assert_eq!(
            at(
                "A.java",
                "class A { void f() { for (int i = 0; i < 3; i++) { "
            ),
            [JavaStatement, Other],
            "semicolons inside parentheses do not end the statement"
        );
    }

    #[test]
    fn other_languages() {
        assert_eq!(
            at("a.kt", "fun f() {\n  "),
            [Context::Kotlin, Context::Other]
        );
        assert_eq!(at("a.rs", "fn f() {\n  "), [Context::Rust, Context::Other]);
        assert_eq!(at("a.rs", "let x: &'a str = \"x"), [Context::Other]);
        assert_eq!(at("a.txt", "x"), [Context::Other]);
    }

    #[test]
    fn enclosing_class_and_method() {
        let scan = scan(
            "public class Foo<T> {\n  static int add(int a, Map<String, List<Integer>> b) throws X {\n    if (a) {\n  ",
        );
        assert_eq!(scan.class_name().as_deref(), Some("Foo"));
        assert_eq!(
            scan.method(),
            Some(("add".to_owned(), vec!["a".to_owned(), "b".to_owned()]))
        );
        let kotlin = super::scan("class K {\n  fun go(name: String, n: Int) {\n  ");
        assert_eq!(
            kotlin.method(),
            Some(("go".to_owned(), vec!["name".to_owned(), "n".to_owned()]))
        );
        assert_eq!(super::scan("class A {\n  int x;\n  ").method(), None);
        assert_eq!(
            super::scan("void f() { while (x) {\n")
                .method()
                .map(|m| m.0)
                .as_deref(),
            Some("f")
        );
    }
}

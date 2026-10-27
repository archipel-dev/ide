//! What the typed handlers need to know about the text around a caret: the language family of
//! the file, where comments and literals are, and bracket balance. It works on the characters
//! (not on a syntax tree), so it also answers for unfinished code such as `foo(a` while typing
//! (ED-TYPE-002: the handlers never fail, they fall back to the plain rule).

use archipel_ide_text::TextDocument;
use std::path::Path;

/// Language family of a file; decides which typed handlers apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Plain text: no pairing, no smart indent.
    Plain,
    Java,
    Kotlin,
    /// Other brace languages with `//` and `/* */` comments.
    CLike,
    /// Scripting languages with `#` comments.
    Script,
    /// Anything else with a known line comment prefix.
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lang {
    pub family: Family,
    pub line_comment: Option<&'static str>,
    pub block_comment: Option<(&'static str, &'static str)>,
}

impl Lang {
    pub const PLAIN: Lang = Lang {
        family: Family::Plain,
        line_comment: None,
        block_comment: None,
    };

    /// The language of a file by its extension.
    pub fn of(path: &Path) -> Lang {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase);
        let c_like = |family| Lang {
            family,
            line_comment: Some("//"),
            block_comment: Some(("/*", "*/")),
        };
        match extension.as_deref() {
            Some("java") => c_like(Family::Java),
            Some("kt" | "kts") => c_like(Family::Kotlin),
            Some(
                "rs" | "js" | "mjs" | "ts" | "tsx" | "jsx" | "c" | "h" | "cpp" | "cc" | "hpp"
                | "cs" | "go" | "swift" | "scala" | "groovy" | "gradle" | "dart" | "php" | "css"
                | "scss",
            ) => c_like(Family::CLike),
            Some("py" | "sh" | "bash" | "zsh" | "toml" | "yaml" | "yml" | "rb" | "pl" | "r") => {
                Lang {
                    family: Family::Script,
                    line_comment: Some("#"),
                    block_comment: None,
                }
            }
            Some("sql" | "lua" | "hs") => Lang {
                family: Family::Other,
                line_comment: Some("--"),
                block_comment: None,
            },
            Some("lisp" | "clj" | "el") => Lang {
                family: Family::Other,
                line_comment: Some(";"),
                block_comment: None,
            },
            Some("bat" | "cmd") => Lang {
                family: Family::Other,
                line_comment: Some("REM"),
                block_comment: None,
            },
            None | Some("txt" | "text" | "log" | "md" | "markdown" | "csv" | "json") => Lang::PLAIN,
            Some(_) => Lang {
                family: Family::Other,
                line_comment: Some("//"),
                block_comment: None,
            },
        }
    }

    /// Whether the language has typed handlers (pairing, smart Enter).
    pub fn is_code(self) -> bool {
        !matches!(self.family, Family::Plain)
    }

    /// Java and Kotlin: `<` after a class name opens generics (ED-TYPE-012).
    pub fn has_generics(self) -> bool {
        matches!(self.family, Family::Java | Family::Kotlin)
    }

    /// Brace languages: smart indent follows `{ }`.
    pub fn braces(self) -> bool {
        matches!(
            self.family,
            Family::Java | Family::Kotlin | Family::CLike | Family::Script
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    LineComment,
    BlockComment,
    DocComment,
    Str,
    Char,
    TextBlock,
}

/// A comment or literal. `end` is exclusive; `closed` is false for a literal without closing
/// quote or a block comment without `*/` (they run to the end of the line or the file).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub kind: RegionKind,
    pub start: usize,
    pub end: usize,
    pub closed: bool,
}

impl Region {
    pub fn is_comment(self) -> bool {
        matches!(
            self.kind,
            RegionKind::LineComment | RegionKind::BlockComment | RegionKind::DocComment
        )
    }

    pub fn is_literal(self) -> bool {
        !self.is_comment()
    }
}

/// Texts beyond this size are not scanned (the handlers fall back to the plain rules).
const SCAN_LIMIT: usize = 2_000_000;

/// The characters of a document with the comments and literals located.
#[derive(Debug, Clone)]
pub struct View {
    pub chars: Vec<char>,
    pub lang: Lang,
    regions: Vec<Region>,
}

impl View {
    pub fn new(text: &TextDocument, lang: Lang) -> Self {
        Self::of_text(&text.text(), lang)
    }

    pub fn of_text(text: &str, lang: Lang) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let regions = if lang.is_code() && chars.len() <= SCAN_LIMIT {
            scan(&chars, lang)
        } else {
            Vec::new()
        };
        Self {
            chars,
            lang,
            regions,
        }
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn at(&self, index: usize) -> Option<char> {
        self.chars.get(index).copied()
    }

    pub fn before(&self, caret: usize) -> Option<char> {
        caret.checked_sub(1).and_then(|i| self.at(i))
    }

    /// Start of the line containing `offset`.
    pub fn line_start(&self, offset: usize) -> usize {
        let mut i = offset.min(self.chars.len());
        while i > 0 && self.chars[i - 1] != '\n' {
            i -= 1;
        }
        i
    }

    /// End of the line content (before the line break) containing `offset`.
    pub fn line_end(&self, offset: usize) -> usize {
        let mut i = offset.min(self.chars.len());
        while i < self.chars.len() && self.chars[i] != '\n' {
            i += 1;
        }
        i
    }

    /// The leading blanks of the line starting at `start`.
    pub fn indent_at(&self, start: usize) -> String {
        self.chars[start.min(self.chars.len())..]
            .iter()
            .take_while(|c| **c == ' ' || **c == '\t')
            .collect()
    }

    /// Offset of the first non-blank character of the line starting at `start` (or its end).
    pub fn first_non_blank(&self, start: usize) -> usize {
        let mut i = start;
        while i < self.chars.len() && matches!(self.chars[i], ' ' | '\t') {
            i += 1;
        }
        i
    }

    /// Whether the line has no characters but blanks.
    pub fn is_blank_line(&self, start: usize) -> bool {
        let at = self.first_non_blank(start);
        at >= self.chars.len() || self.chars[at] == '\n'
    }

    pub fn text(&self, start: usize, end: usize) -> String {
        self.chars[start.min(self.chars.len())..end.min(self.chars.len())]
            .iter()
            .collect()
    }

    /// The comment or literal whose characters include `index`.
    pub fn region_of_char(&self, index: usize) -> Option<Region> {
        let at = self.regions.partition_point(|r| r.end <= index);
        self.regions.get(at).copied().filter(|r| r.start <= index)
    }

    /// Whether the character at `index` is code (not inside a comment or literal).
    pub fn is_code(&self, index: usize) -> bool {
        self.region_of_char(index).is_none()
    }

    /// The comment or literal the caret between two characters is inside of: after its opening
    /// delimiter and not after its closing one. A caret right after `"` of `"abc"` is inside;
    /// right after the closing quote it is not.
    pub fn inside(&self, caret: usize) -> Option<Region> {
        // The candidates are the regions around the caret.
        let at = self.regions.partition_point(|r| r.end < caret);
        self.regions
            .iter()
            .skip(at)
            .take(2)
            .copied()
            .find(|r| match r.kind {
                RegionKind::LineComment => caret >= r.start + 2 && caret <= r.end,
                RegionKind::BlockComment | RegionKind::DocComment => {
                    let inner_end = if r.closed { r.end - 2 } else { r.end };
                    let open = if r.kind == RegionKind::DocComment {
                        3
                    } else {
                        2
                    };
                    caret >= r.start + open && caret <= inner_end
                }
                RegionKind::TextBlock => {
                    let inner_end = if r.closed { r.end - 3 } else { r.end };
                    caret >= r.start + 3 && caret <= inner_end
                }
                RegionKind::Str | RegionKind::Char => {
                    let last = if r.closed { r.end - 1 } else { r.end };
                    caret > r.start && caret <= last
                }
            })
    }

    pub fn in_comment(&self, caret: usize) -> bool {
        self.inside(caret).is_some_and(Region::is_comment)
    }

    pub fn in_literal(&self, caret: usize) -> bool {
        self.inside(caret).is_some_and(Region::is_literal)
    }

    /// Number of unmatched-so-far openers and closers of `open`/`close` in code. The structure
    /// is balanced when the counts are equal.
    pub fn bracket_counts(&self, open: char, close: char) -> (usize, usize) {
        let (mut opens, mut closes) = (0, 0);
        for (i, c) in self.chars.iter().enumerate() {
            if *c == open && self.is_code(i) {
                opens += 1;
            } else if *c == close && self.is_code(i) {
                closes += 1;
            }
        }
        (opens, closes)
    }

    /// Offsets of the openers of `open`/`close` that have no closer before `limit`, outermost
    /// first (code only).
    pub fn open_stack(&self, open: char, close: char, limit: usize) -> Vec<usize> {
        let mut stack = Vec::new();
        for (i, c) in self.chars.iter().enumerate().take(limit) {
            if !self.is_code(i) {
                continue;
            }
            if *c == open {
                stack.push(i);
            } else if *c == close {
                stack.pop();
            }
        }
        stack
    }

    /// All unclosed brackets of every kind before `limit`, outermost first: `(offset, char)`.
    pub fn open_brackets(&self, limit: usize) -> Vec<(usize, char)> {
        let mut stack: Vec<(usize, char)> = Vec::new();
        for (i, c) in self.chars.iter().enumerate().take(limit) {
            if !self.is_code(i) {
                continue;
            }
            match c {
                '(' | '[' | '{' => stack.push((i, *c)),
                ')' | ']' | '}' => {
                    let open = match c {
                        ')' => '(',
                        ']' => '[',
                        _ => '{',
                    };
                    if let Some(position) = stack.iter().rposition(|(_, o)| *o == open) {
                        stack.truncate(position);
                    }
                }
                _ => {}
            }
        }
        stack
    }
}

fn scan(chars: &[char], lang: Lang) -> Vec<Region> {
    let mut regions = Vec::new();
    let n = chars.len();
    let c_like = lang.block_comment.is_some();
    let hash = lang.line_comment == Some("#");
    let line_prefix: Vec<char> = lang.line_comment.unwrap_or("").chars().collect();
    let starts_with = |i: usize, pattern: &[char]| {
        !pattern.is_empty() && chars.get(i..i + pattern.len()) == Some(pattern)
    };
    let mut i = 0;
    while i < n {
        let c = chars[i];
        if starts_with(i, &line_prefix) {
            let mut end = i;
            while end < n && chars[end] != '\n' {
                end += 1;
            }
            regions.push(Region {
                kind: RegionKind::LineComment,
                start: i,
                end,
                closed: true,
            });
            i = end;
            continue;
        }
        if c_like && c == '/' && chars.get(i + 1) == Some(&'*') {
            let doc = chars.get(i + 2) == Some(&'*') && chars.get(i + 3) != Some(&'/');
            let mut end = i + 2;
            let mut closed = false;
            while end < n {
                if chars[end] == '*' && chars.get(end + 1) == Some(&'/') {
                    end += 2;
                    closed = true;
                    break;
                }
                end += 1;
            }
            regions.push(Region {
                kind: if doc {
                    RegionKind::DocComment
                } else {
                    RegionKind::BlockComment
                },
                start: i,
                end,
                closed,
            });
            i = end;
            continue;
        }
        if c == '"' || (c == '\'' && !c_like_char_is_code(chars, i, lang)) {
            let triple = chars.get(i + 1) == Some(&c) && chars.get(i + 2) == Some(&c);
            if triple && (c == '"' || hash) {
                let mut end = i + 3;
                let mut closed = false;
                while end < n {
                    if chars[end] == '\\' {
                        end += 2;
                        continue;
                    }
                    if chars[end] == c
                        && chars.get(end + 1) == Some(&c)
                        && chars.get(end + 2) == Some(&c)
                    {
                        end += 3;
                        closed = true;
                        break;
                    }
                    end += 1;
                }
                let end = end.min(n);
                regions.push(Region {
                    kind: RegionKind::TextBlock,
                    start: i,
                    end,
                    closed,
                });
                i = end;
                continue;
            }
            let mut end = i + 1;
            let mut closed = false;
            while end < n && chars[end] != '\n' {
                if chars[end] == '\\' {
                    end += 2;
                    continue;
                }
                if chars[end] == c {
                    end += 1;
                    closed = true;
                    break;
                }
                end += 1;
            }
            let end = end.min(n);
            // A line break ends an unterminated literal before it.
            regions.push(Region {
                kind: if c == '"' {
                    RegionKind::Str
                } else {
                    RegionKind::Char
                },
                start: i,
                end,
                closed,
            });
            i = end;
            continue;
        }
        i += 1;
    }
    regions
}

/// In Rust and similar languages `'a` is a lifetime, not a character literal: a quote that has
/// no partner within a few characters on its line is code there.
fn c_like_char_is_code(chars: &[char], start: usize, lang: Lang) -> bool {
    if matches!(
        lang.family,
        Family::Java | Family::Kotlin | Family::Script | Family::Other
    ) {
        return false;
    }
    let mut i = start + 1;
    let limit = (start + 12).min(chars.len());
    while i < limit {
        match chars[i] {
            '\\' => i += 2,
            '\'' => return false,
            '\n' => return true,
            _ => i += 1,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn java(text: &str) -> View {
        View::of_text(text, Lang::of(Path::new("A.java")))
    }

    #[test]
    fn finds_comments_and_literals() {
        let view = java("a // c\n\"s\" /* b */ 'x'");
        assert!(view.in_comment(5));
        assert!(!view.in_comment(1));
        assert!(view.in_literal(8));
        assert!(!view.in_literal(10), "after the closing quote");
        assert!(view.in_comment(15));
        assert!(view.in_literal(20));
    }

    #[test]
    fn unterminated_literal_ends_at_the_line_break() {
        let view = java("\"ab\nfoo(");
        assert!(view.in_literal(3));
        assert!(!view.in_literal(5));
        assert_eq!(view.open_brackets(view.len()).len(), 1);
    }

    #[test]
    fn counts_brackets_in_code_only() {
        let view = java("f(\"(\") // )\n(");
        assert_eq!(view.bracket_counts('(', ')'), (2, 1));
    }
}

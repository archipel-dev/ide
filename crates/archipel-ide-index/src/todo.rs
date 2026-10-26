//! The `Todo` index: TODO-style comments per file (spec 2.20).
//!
//! Patterns come from settings (`todo.patterns`); a pattern is a regex run
//! over each line of every comment (comment nodes of the parsed tree, so a
//! `TODO` inside a string literal is ignored). Files without a grammar are
//! scanned as plain text. Changing the patterns changes the index's version
//! salt, which rebuilds only this index.

use crate::codec::{Reader, Writer};
use crate::indexes::ids;
use archipel_ide_analysis::index::{FileIndex, IndexInput, IndexPriority, IndexValue};
use archipel_ide_analysis::vfs::MAX_FILE_BYTES;
use archipel_ide_analysis::{Cancelled, FileMeta, IndexId};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

/// One TODO pattern (Settings > Editor > TODO).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoPattern {
    /// Regular expression matched against one line of a comment; the match
    /// runs to the end of the line when it ends with `.*`.
    pub pattern: String,
    #[serde(default)]
    pub case_sensitive: bool,
    /// Theme key of the highlight color (`todo`, `fixme`, ...).
    #[serde(default)]
    pub color_key: String,
}

impl TodoPattern {
    pub fn new(pattern: &str, case_sensitive: bool, color_key: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            case_sensitive,
            color_key: color_key.to_string(),
        }
    }

    pub fn regex(&self) -> Result<Regex, regex::Error> {
        RegexBuilder::new(&self.pattern)
            .case_insensitive(!self.case_sensitive)
            .build()
    }
}

/// `todo` and `fixme`, case-insensitive, word-bounded, to the end of the
/// line (the documented IDE defaults).
pub fn default_patterns() -> Vec<TodoPattern> {
    vec![
        TodoPattern::new(r"\btodo\b.*", false, "todo"),
        TodoPattern::new(r"\bfixme\b.*", false, "fixme"),
    ]
}

/// A named subset of the patterns for the tool window ("Filters" table).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoFilter {
    pub name: String,
    /// Indexes into the pattern list.
    pub patterns: Vec<u32>,
}

/// Everything the TODO settings page configures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoSettings {
    pub patterns: Vec<TodoPattern>,
    pub filters: Vec<TodoFilter>,
    /// Scan every line of block comments (off: only the first line).
    pub multiline: bool,
}

impl Default for TodoSettings {
    fn default() -> Self {
        Self {
            patterns: default_patterns(),
            filters: Vec::new(),
            multiline: true,
        }
    }
}

impl TodoSettings {
    /// Index-relevant part (patterns and multiline), filters excluded.
    pub fn same_index_config(&self, other: &Self) -> bool {
        self.patterns == other.patterns && self.multiline == other.multiline
    }
}

/// `i todo \btodo\b.*` per line: `i` (ignore case) or `s` (case
/// sensitive), the color key, then the regular expression (rest of line).
pub fn parse_patterns(lines: &[String]) -> Vec<TodoPattern> {
    lines
        .iter()
        .filter_map(|line| {
            let mut parts = line.trim().splitn(3, ' ');
            let case = parts.next()?;
            let color = parts.next()?;
            let pattern = parts.next()?.trim();
            if pattern.is_empty() || !matches!(case, "i" | "s") {
                return None;
            }
            let parsed = TodoPattern::new(pattern, case == "s", color);
            parsed.regex().ok()?;
            Some(parsed)
        })
        .collect()
}

pub fn format_patterns(patterns: &[TodoPattern]) -> Vec<String> {
    patterns
        .iter()
        .map(|p| {
            format!(
                "{} {} {}",
                if p.case_sensitive { "s" } else { "i" },
                if p.color_key.is_empty() {
                    "todo"
                } else {
                    &p.color_key
                },
                p.pattern
            )
        })
        .collect()
}

/// `Name: 0,1` per line.
pub fn parse_filters(lines: &[String]) -> Vec<TodoFilter> {
    lines
        .iter()
        .filter_map(|line| {
            let (name, patterns) = line.split_once(':')?;
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            Some(TodoFilter {
                name: name.to_string(),
                patterns: patterns
                    .split(',')
                    .filter_map(|n| n.trim().parse().ok())
                    .collect(),
            })
        })
        .collect()
}

pub fn format_filters(filters: &[TodoFilter]) -> Vec<String> {
    filters
        .iter()
        .map(|f| {
            format!(
                "{}: {}",
                f.name,
                f.patterns
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .collect()
}

/// One occurrence of a pattern in a comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoItem {
    /// Byte range of the match in the file.
    pub start: u32,
    pub end: u32,
    /// Zero-based line.
    pub line: u32,
    /// The matched text (trimmed, without a trailing comment terminator).
    pub text: String,
}

/// The occurrences of one pattern in one file (the index value).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TodoValue(pub Vec<TodoItem>);

impl IndexValue for TodoValue {
    fn encode(&self, out: &mut Vec<u8>) {
        let mut w = Writer::new(out);
        w.varint(self.0.len() as u64);
        for item in &self.0 {
            w.varint(u64::from(item.start));
            w.varint(u64::from(item.end));
            w.varint(u64::from(item.line));
            w.str(&item.text);
        }
    }
    fn decode(bytes: &[u8]) -> Option<Self> {
        let mut r = Reader::new(bytes);
        let count = r.varint()? as usize;
        let mut items = Vec::with_capacity(count.min(1024));
        for _ in 0..count {
            items.push(TodoItem {
                start: r.u32()?,
                end: r.u32()?,
                line: r.u32()?,
                text: r.str()?.to_string(),
            });
        }
        Some(Self(items))
    }
}

/// The `Todo` index: pattern index -> occurrences of that pattern in the file.
pub struct TodoIndex {
    patterns: Vec<TodoPattern>,
    regexes: Vec<Option<Regex>>,
    multiline: bool,
}

impl TodoIndex {
    pub fn new(settings: &TodoSettings) -> Self {
        let regexes = settings.patterns.iter().map(|p| p.regex().ok()).collect();
        Self {
            patterns: settings.patterns.clone(),
            regexes,
            multiline: settings.multiline,
        }
    }

    pub fn patterns(&self) -> &[TodoPattern] {
        &self.patterns
    }

    /// Version salt of the settings (changes when any pattern changes).
    pub fn salt(settings: &TodoSettings) -> String {
        let mut hasher = archipel_ide_analysis::interface::Fnv64::default();
        for pattern in &settings.patterns {
            hasher.write(pattern.pattern.as_bytes());
            hasher.write(&[u8::from(pattern.case_sensitive), 0]);
        }
        hasher.write(&[u8::from(settings.multiline)]);
        format!("{:016x}", hasher.finish())
    }

    /// Occurrences per pattern index in `text`.
    pub fn scan(
        &self,
        text: &str,
        comments: Option<&[(usize, usize)]>,
        heuristic_comments: bool,
    ) -> Vec<(u32, Vec<TodoItem>)> {
        let mut found: Vec<Vec<TodoItem>> = vec![Vec::new(); self.patterns.len()];
        let mut scan_span = |start: usize, end: usize| {
            let Some(span) = text.get(start..end) else {
                return;
            };
            let mut offset = start;
            let mut line = text[..start].bytes().filter(|b| *b == b'\n').count() as u32;
            let lines = if self.multiline { usize::MAX } else { 1 };
            for piece in span.split_inclusive('\n').take(lines) {
                let content = piece.trim_end_matches(['\n', '\r']);
                for (index, regex) in self.regexes.iter().enumerate() {
                    let Some(regex) = regex else { continue };
                    if let Some(m) = regex.find(content) {
                        let mut matched = m.as_str().trim_end();
                        for terminator in ["*/", "-->", "\"\"\"", "'''", "#}", "--}"] {
                            if let Some(stripped) = matched.strip_suffix(terminator) {
                                matched = stripped.trim_end();
                            }
                        }
                        found[index].push(TodoItem {
                            start: (offset + m.start()) as u32,
                            end: (offset + m.start() + matched.len()) as u32,
                            line,
                            text: matched.to_string(),
                        });
                    }
                }
                offset += piece.len();
                line += 1;
            }
        };
        match comments {
            Some(ranges) => {
                for (start, end) in ranges {
                    scan_span(*start, *end);
                }
            }
            None if heuristic_comments => {
                // A grammar exists but the file was not parsed (too large):
                // only text after a comment marker on the line counts.
                let mut offset = 0;
                for piece in text.split_inclusive('\n') {
                    if let Some(at) = ["//", "/*", "#", "--", "<!--", ";", "%"]
                        .iter()
                        .filter_map(|marker| piece.find(marker))
                        .min()
                    {
                        scan_span(offset + at, offset + piece.len());
                    }
                    offset += piece.len();
                }
            }
            None => scan_span(0, text.len()),
        }
        found
            .into_iter()
            .enumerate()
            .filter(|(_, items)| !items.is_empty())
            .map(|(index, items)| (index as u32, items))
            .collect()
    }
}

/// Byte ranges of the comment nodes of `syntax`.
pub fn comment_ranges(syntax: &archipel_ide_language::SyntaxSnapshot) -> Vec<(usize, usize)> {
    let sets = syntax.lang().token_set_table();
    let mut out = Vec::new();
    let mut cursor = syntax.tree().walk();
    loop {
        let node = cursor.node();
        if sets
            .comments
            .contains(archipel_ide_language::ElementType(node.kind_id()))
        {
            out.push((node.start_byte(), node.end_byte()));
        } else if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return out;
            }
        }
    }
}

impl FileIndex for TodoIndex {
    const ID: IndexId = ids::TODO;
    const VERSION: u32 = 1;
    type Key = u32;
    type Value = TodoValue;
    fn accepts(&self, f: &FileMeta) -> bool {
        f.len <= MAX_FILE_BYTES
    }
    fn index(
        &self,
        input: &IndexInput<'_>,
        out: &mut dyn FnMut(u32, TodoValue),
    ) -> Result<(), Cancelled> {
        let ranges = input.syntax.map(comment_ranges);
        let heuristic = ranges.is_none() && input.file.language.is_some();
        for (pattern, items) in self.scan(input.text, ranges.as_deref(), heuristic) {
            out(pattern, TodoValue(items));
        }
        Ok(())
    }
    fn priority(&self) -> IndexPriority {
        IndexPriority::Todo
    }
}

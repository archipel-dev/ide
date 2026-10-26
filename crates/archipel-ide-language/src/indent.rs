//! Indentation from `indents.scm` (IntelliJ `LineIndentProvider`).
//!
//! Captures:
//! - `@indent`: a container; lines inside it that start after its first line
//!   are indented one level from the container's first line.
//! - `@outdent`: a closing token; when it starts a line it aligns with the
//!   first line of its container (a parent captured `@indent`).
//! - `@indent.branch`: a node (`else`, `case`, `except`) that, when it starts a
//!   line, aligns with the first line of its parent.
//! - `@indent.after`: a token or node that, when it ends a line, indents the
//!   next line (`{`, `(`, a Python `:`); used by Enter in broken code.
//! - `@indent.stop`: a statement after which the next line dedents one level
//!   (`return`, `break` in Python).
//!
//! Continuation lines (the second line of a wrapped expression) keep the
//! indentation the user gave them: [`IndentEngine::for_line`] answers `None`.

use crate::lines::{LineIndex, indent_bytes, indent_columns};
use crate::node::SyntaxSnapshot;
use crate::queryset::{self, Captured, QueryKind};
use crate::textobjects;
use std::ops::Range;
use tree_sitter::Node;

/// Indentation settings in effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndentOptions {
    pub indent_size: usize,
    pub tab_size: usize,
    pub use_tabs: bool,
}

impl Default for IndentOptions {
    fn default() -> Self {
        Self {
            indent_size: 4,
            tab_size: 4,
            use_tabs: false,
        }
    }
}

impl IndentOptions {
    /// The whitespace string for `columns` columns of indentation.
    pub fn indent_string(&self, columns: usize) -> String {
        if self.use_tabs {
            let tab = self.tab_size.max(1);
            format!(
                "{}{}",
                "\t".repeat(columns / tab),
                " ".repeat(columns % tab)
            )
        } else {
            " ".repeat(columns)
        }
    }

    /// Guesses the style of `source` (tabs versus 2/3/4/8 spaces) from its
    /// indented lines; defaults to four spaces.
    pub fn detect(source: &str) -> Self {
        let mut tabs = 0usize;
        let mut spaces = 0usize;
        let mut width_votes = [0usize; 9];
        let mut previous = 0usize;
        for line in source.lines().take(2000) {
            if line.trim().is_empty() {
                continue;
            }
            if line.starts_with('\t') {
                tabs += 1;
            } else if line.starts_with(' ') {
                spaces += 1;
                let width = line.len() - line.trim_start_matches(' ').len();
                let delta = width.abs_diff(previous);
                if (1..=8).contains(&delta) {
                    width_votes[delta] += 1;
                }
                previous = width;
                continue;
            }
            previous = 0;
        }
        if tabs > spaces {
            return Self {
                use_tabs: true,
                ..Self::default()
            };
        }
        let best = [2usize, 4, 3, 8]
            .into_iter()
            .max_by_key(|width| (width_votes[*width], usize::from(*width == 4)))
            .filter(|width| width_votes[*width] > 0)
            .unwrap_or(4);
        Self {
            indent_size: best,
            tab_size: best,
            use_tabs: false,
        }
    }
}

/// Indentation queries over one snapshot.
pub struct IndentEngine<'a> {
    snap: &'a SyntaxSnapshot,
    lines: LineIndex,
    opts: IndentOptions,
}

impl<'a> IndentEngine<'a> {
    pub fn new(snap: &'a SyntaxSnapshot, opts: IndentOptions) -> Self {
        Self {
            snap,
            lines: LineIndex::new(snap.text()),
            opts,
        }
    }

    pub fn options(&self) -> IndentOptions {
        self.opts
    }

    pub fn lines(&self) -> &LineIndex {
        &self.lines
    }

    /// True when the language has an `indents.scm`.
    pub fn supported(&self) -> bool {
        queryset::query(self.snap.lang(), QueryKind::Indents).is_some()
    }

    fn text(&self) -> &str {
        self.snap.text()
    }

    fn line_text(&self, line: usize) -> &str {
        &self.text()[self.lines.line_range(line, self.text())]
    }

    /// Indentation of `line` in columns, as written.
    pub fn indent_of(&self, line: usize) -> usize {
        indent_columns(self.line_text(line), self.opts.tab_size)
    }

    fn captures(&self, range: Range<usize>) -> Vec<Captured<'a>> {
        match queryset::query(self.snap.lang(), QueryKind::Indents) {
            Some(query) => queryset::captures_in(query, self.snap.tree(), self.text(), range),
            None => Vec::new(),
        }
    }

    /// The indentation `line` should have, `None` to leave it alone (blank
    /// lines, lines inside strings, continuation lines, languages without
    /// `indents.scm`). `existing(row)` gives the indentation of a reference row
    /// (callers reindenting a range pass the rows already updated).
    pub fn for_line(&self, line: usize, existing: &dyn Fn(usize) -> usize) -> Option<usize> {
        if !self.supported() {
            return None;
        }
        let range = self.lines.line_range(line, self.text());
        let line_text = &self.text()[range.clone()];
        if line_text.trim().is_empty() {
            return None;
        }
        let first = range.start + indent_bytes(line_text);
        let leaf = self.snap.root().find_leaf_at(u32::try_from(first).ok()?)?;
        let leaf = leaf.raw();
        if leaf.start_byte() < first {
            return self.comment_continuation(leaf, first, line);
        }
        let captures = self.captures(leaf.byte_range());
        let has = |node: Node<'_>, name: &str| {
            captures
                .iter()
                .any(|captured| captured.node.id() == node.id() && captured.name == name)
        };
        // A closing token (or end tag) aligns with the line that opened its container.
        let mut closer = Some(leaf);
        while let Some(candidate) = closer.filter(|n| n.start_byte() == leaf.start_byte()) {
            if has(candidate, "outdent") {
                let mut container = candidate.parent();
                while let Some(parent) = container {
                    if has(parent, "indent") && parent.start_position().row < line {
                        return Some(existing(parent.start_position().row));
                    }
                    if parent.start_byte() < candidate.start_byte() {
                        break;
                    }
                    container = parent.parent();
                }
                break;
            }
            closer = candidate.parent();
        }
        // Branch keywords align with the construct they continue.
        let mut node = Some(leaf);
        while let Some(current) = node.filter(|n| n.start_byte() == leaf.start_byte()) {
            if has(current, "indent.branch")
                && let Some(parent) = current.parent()
                && parent.start_position().row < line
            {
                return Some(existing(parent.start_position().row));
            }
            node = current.parent();
        }
        if !self.starts_element(first) {
            return None;
        }
        let mut node = leaf.parent();
        while let Some(current) = node {
            if has(current, "indent") && current.start_position().row < line {
                return Some(existing(current.start_position().row) + self.opts.indent_size);
            }
            node = current.parent();
        }
        // Languages with structural indentation (YAML) predict nothing here.
        let predicts = queryset::query(self.snap.lang(), QueryKind::Indents)
            .is_some_and(|query| query.capture_names().contains(&"indent"));
        predicts.then_some(0)
    }

    /// Lines inside a block comment: continuation lines (`* text`) align one
    /// column right of the comment's start.
    fn comment_continuation(&self, leaf: Node<'_>, first: usize, line: usize) -> Option<usize> {
        let comments = &self.snap.lang().token_set_table().comments;
        if !comments.contains(crate::ElementType(leaf.kind_id())) {
            return None;
        }
        let body = self.text().get(leaf.byte_range())?;
        if !body.starts_with("/*") || !self.text()[first..].starts_with('*') {
            return None;
        }
        let start_row = leaf.start_position().row;
        (start_row < line).then(|| self.indent_of(start_row) + 1)
    }

    /// Does the token at `offset` start an element of a statement list or
    /// argument list (as opposed to continuing the previous one)? Languages
    /// without selection groups answer yes.
    fn starts_element(&self, offset: usize) -> bool {
        let groups = textobjects::groups_around(self.snap, &(offset..offset));
        if groups.is_empty() {
            return true;
        }
        groups
            .iter()
            .any(|group| group.elements.iter().any(|element| element.start == offset))
            || {
                // A token inside an element that starts on this very line.
                let line = self.lines.line_of(offset);
                groups.iter().any(|group| {
                    group.elements.iter().any(|e| {
                        e.start <= offset && offset < e.end && self.lines.line_of(e.start) == line
                    })
                })
            }
    }

    /// Indentation of the line created by pressing Enter at `caret`.
    pub fn for_enter(&self, caret: usize) -> usize {
        let caret = caret.min(self.text().len());
        let current = self.indent_of(self.lines.line_of(caret));
        if !self.supported() {
            return current;
        }
        self.enter_indent(caret, current).unwrap_or(current)
    }

    fn enter_indent(&self, caret: usize, current: usize) -> Option<usize> {
        let text = self.text();
        let line = self.lines.line_of(caret);
        let line_start = self.lines.line_start(line);
        let trimmed = text[line_start..caret].trim_end();
        if trimmed.trim().is_empty() {
            return Some(current);
        }
        let last_end = line_start + trimmed.len();
        let last_leaf = self
            .snap
            .root()
            .find_leaf_at(u32::try_from(last_end - 1).ok()?)?
            .raw();
        let captures = self.captures(last_leaf.byte_range());
        // A trailing opener (or `:`) indents the next line.
        let mut node = Some(last_leaf);
        while let Some(candidate) = node.filter(|n| n.end_byte() == last_end) {
            let hit = |name: &str| {
                captures
                    .iter()
                    .any(|c| c.node.id() == candidate.id() && c.name == name)
            };
            if hit("indent.after") {
                return Some(current + self.opts.indent_size);
            }
            if hit("indent.stop") {
                return Some(current.saturating_sub(self.opts.indent_size));
            }
            node = candidate.parent();
        }
        if !self.starts_element_before(last_end - 1, line) {
            return Some(current);
        }
        // Inside a container that began on an earlier line.
        let probe = last_end - 1;
        let around = self.captures(probe..probe + 1);
        let mut node = last_leaf.parent();
        while let Some(ancestor) = node {
            let unclosed = ancestor
                .child(ancestor.child_count().saturating_sub(1))
                .is_some_and(|last| last.is_missing());
            let open = ancestor.start_byte() < caret
                && (caret < ancestor.end_byte() || (ancestor.end_byte() == caret && unclosed));
            if open
                && around
                    .iter()
                    .any(|c| c.node.id() == ancestor.id() && c.name == "indent")
            {
                return Some(self.indent_of(ancestor.start_position().row) + self.opts.indent_size);
            }
            node = ancestor.parent();
        }
        Some(current)
    }

    /// Enter in the middle of a wrapped statement keeps the continuation
    /// indentation: true when the element at `offset` begins on this line.
    fn starts_element_before(&self, offset: usize, line: usize) -> bool {
        let groups = textobjects::groups_around(self.snap, &(offset..offset));
        let Some(group) = groups.first() else {
            return true;
        };
        match group
            .elements
            .iter()
            .find(|e| e.start <= offset && offset < e.end)
        {
            Some(element) => self.lines.line_of(element.start) >= line,
            None => true,
        }
    }

    /// Target indentation per line of `lines` (Auto-Indent Lines), top-down so
    /// each line sees its reference lines already corrected; lines that keep
    /// their indentation are omitted.
    pub fn reindent(&self, lines: Range<usize>) -> Vec<(usize, usize)> {
        let mut updated: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        let mut out = Vec::new();
        for line in lines {
            if line >= self.lines.line_count() {
                break;
            }
            let existing = |row: usize| {
                updated
                    .get(&row)
                    .copied()
                    .unwrap_or_else(|| self.indent_of(row))
            };
            let Some(target) = self.for_line(line, &existing) else {
                continue;
            };
            if target != self.indent_of(line) {
                out.push((line, target));
            }
            updated.insert(line, target);
        }
        out
    }
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
    fn detects_tabs_and_space_widths() {
        assert!(IndentOptions::detect("a\n\tb\n\t\tc\n").use_tabs);
        assert_eq!(IndentOptions::detect("a\n  b\n    c\n  d\n").indent_size, 2);
        assert_eq!(
            IndentOptions::detect("a\n    b\n        c\n").indent_size,
            4
        );
        assert_eq!(IndentOptions::detect("plain\n").indent_size, 4);
        assert_eq!(IndentOptions::default().indent_string(6), "      ");
        let tabs = IndentOptions {
            use_tabs: true,
            ..IndentOptions::default()
        };
        assert_eq!(tabs.indent_string(6), "\t  ");
    }

    #[test]
    fn closing_brace_and_body_lines_follow_the_container() {
        let source = "class A {\nvoid f() {\nint x;\n}\n}\n";
        let snapshot = snap("java", source);
        let engine = IndentEngine::new(&snapshot, IndentOptions::default());
        let fixed = engine.reindent(0..5);
        assert_eq!(fixed, vec![(1, 4), (2, 8), (3, 4)]);
    }

    #[test]
    fn enter_indents_after_an_opener_and_inside_blocks() {
        let source = "class A {\n    void f() {\n        g();\n";
        let snapshot = snap("java", source);
        let engine = IndentEngine::new(&snapshot, IndentOptions::default());
        let after_brace = source.find("f() {").unwrap() + 5;
        assert_eq!(engine.for_enter(after_brace), 8);
        let after_call = source.find("g();").unwrap() + 4;
        assert_eq!(engine.for_enter(after_call), 8);
    }
}

//! Tree-sitter language platform: a data-driven registry of grammar crates and
//! their bundled queries, a generic highlight engine, and IntelliJ color
//! schemes. No language has its own code path here.

pub mod brackets;
mod captures;
pub mod folding;
pub mod highlight;
mod injection;
pub mod registry;
pub mod scheme;
pub mod selection;
pub mod structure;
pub mod syntax;

pub use archipel_ide_text::{ByteEdit, BytePoint};
pub use captures::capture_key;
pub use folding::{FoldKind, FoldRange};
pub use highlight::{HighlightLayer, HighlightSpan, QueryError};
pub use registry::{LANGUAGES, LanguageSpec};
pub use scheme::{ColorScheme, EffectType, FontStyle, Rgba, TextAttributes};
pub use structure::{StructureItem, SymbolKind};
pub use syntax::SyntaxTree;

use std::path::Path;
use tree_sitter::Node;

/// Maximum number of syntax diagnostics reported per file.
const MAX_DIAGNOSTICS: usize = 100;

/// Bytes highlighted when a file is analyzed lazily; the rest is highlighted on
/// demand as the editor scrolls ([`LanguageAnalysis::ensure_highlighted`]).
pub const INITIAL_HIGHLIGHT_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub row: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Default)]
pub struct LanguageAnalysis {
    /// Display name of the detected language.
    pub language: Option<&'static str>,
    /// Syntax highlighting from the grammar's highlight query.
    pub highlights: HighlightLayer,
    pub diagnostics: Vec<Diagnostic>,
    /// The incrementally maintained syntax tree, when the language is known.
    pub syntax: Option<SyntaxTree>,
    /// The text this analysis describes (shared, for range queries such as
    /// folding regions without re-reading the buffer).
    pub source: std::sync::Arc<str>,
    /// Byte ranges whose highlights are computed (sorted, disjoint).
    highlighted: Vec<std::ops::Range<usize>>,
}

impl LanguageAnalysis {
    /// Matching-bracket pair for the bracket token starting at `byte`, from the
    /// syntax tree (`None` for non-brackets, brackets in strings/comments and
    /// unmatched brackets). Callers fall back to text matching when the
    /// language has no grammar ([`LanguageAnalysis::syntax`] is `None`).
    pub fn matching_bracket(&self, byte: usize) -> Option<(usize, usize)> {
        brackets::matching_bracket(self.syntax.as_ref()?.tree(), byte)
    }

    /// Declarations of the file, nested (empty without a tags query).
    pub fn structure(&self) -> Vec<StructureItem> {
        match &self.syntax {
            Some(syntax) => structure::structure(syntax.language(), syntax.tree(), &self.source),
            None => Vec::new(),
        }
    }

    /// Folding regions intersecting the byte `range` (see [`folding::fold_ranges`]).
    pub fn folds_in(&self, range: std::ops::Range<usize>) -> Vec<FoldRange> {
        match &self.syntax {
            Some(syntax) => {
                folding::fold_ranges(syntax.language(), syntax.tree(), &self.source, range)
            }
            None => Vec::new(),
        }
    }

    /// Extend Selection: the next larger syntactic byte range around `range`.
    pub fn expand_selection(
        &self,
        source: &str,
        range: std::ops::Range<usize>,
    ) -> Option<std::ops::Range<usize>> {
        selection::expand(self.syntax.as_ref()?.tree(), source, range)
    }

    /// Brings the analysis up to date with `source` after `edits` (as
    /// journaled by the text buffer): applies them to the syntax tree,
    /// reparses incrementally and re-highlights only the affected ranges.
    pub fn apply_edits(&mut self, path: &Path, source: &str, edits: &[ByteEdit]) {
        if edits.is_empty() {
            return;
        }
        let Some(syntax) = self.syntax.as_mut() else {
            // Plain text may become a known language (e.g. a new shebang line).
            if registry::detect(path, source).is_some() {
                *self = analyze(path, source);
            }
            return;
        };
        let mut stale: Vec<std::ops::Range<usize>> = Vec::new();
        for edit in edits {
            for range in stale.iter_mut().chain(self.highlighted.iter_mut()) {
                *range = syntax::map_range(range.clone(), edit);
            }
            stale.extend(self.highlights.apply_edit(edit));
        }
        self.highlighted = syntax::merge_ranges(std::mem::take(&mut self.highlighted));
        let Some(mut ranges) = syntax.apply_edits(source, edits) else {
            *self = analyze(path, source);
            return;
        };
        ranges.extend(
            stale
                .into_iter()
                .map(|range| range.start.min(source.len())..range.end.min(source.len())),
        );
        self.source = source.into();
        for range in syntax::merge_ranges(ranges) {
            // Text never highlighted stays lazy; it is computed when shown.
            if self
                .highlighted
                .iter()
                .any(|covered| covered.start <= range.end && range.start <= covered.end)
            {
                self.highlight_region(range);
            }
        }
        let Some(syntax) = self.syntax.as_ref() else {
            return;
        };
        self.diagnostics.clear();
        collect_diagnostics(syntax.tree().root_node(), &mut self.diagnostics);
    }

    /// Highlights `range` (grown until it covers whole new captures and whole
    /// old spans, so no clipped remainder keeps a stale key) and records it
    /// as computed.
    fn highlight_region(&mut self, mut range: std::ops::Range<usize>) {
        let Some(syntax) = self.syntax.as_ref() else {
            return;
        };
        loop {
            let (covered, replacement) = syntax.highlight_covering(&self.source, range.clone());
            let grown = self.highlights.covering(covered.clone());
            if grown == covered {
                self.highlights.splice(covered.clone(), replacement);
                self.highlighted.push(covered);
                self.highlighted = syntax::merge_ranges(std::mem::take(&mut self.highlighted));
                return;
            }
            range = grown;
        }
    }

    /// Computes highlights for any part of the byte `range` not highlighted
    /// yet (the editor calls this with its viewport).
    pub fn ensure_highlighted(&mut self, range: std::ops::Range<usize>) {
        if self.syntax.is_none() {
            return;
        }
        let len = self.source.len();
        let range = range.start.min(len)..range.end.min(len);
        let mut gaps = Vec::new();
        let mut cursor = range.start;
        for covered in &self.highlighted {
            if covered.end <= cursor {
                continue;
            }
            if covered.start >= range.end {
                break;
            }
            if covered.start > cursor {
                gaps.push(cursor..covered.start);
            }
            cursor = cursor.max(covered.end);
        }
        if cursor < range.end {
            gaps.push(cursor..range.end);
        }
        for gap in gaps {
            let gap = syntax::line_range(&self.source, gap);
            self.highlight_region(gap);
        }
    }

    /// True when no injected region (Markdown fence, `<script>`, …) has syntax
    /// errors. Injected error recovery can depend on position, so highlights
    /// of erroneous injected regions may lag until they are edited.
    #[doc(hidden)]
    pub fn injections_error_free(&self) -> bool {
        self.syntax.as_ref().is_none_or(|syntax| {
            injection::error_free(syntax.language(), syntax.tree(), &self.source)
        })
    }

    /// True when every byte of the file has highlights computed.
    pub fn fully_highlighted(&self) -> bool {
        self.syntax.is_none()
            || self.highlighted.first() == Some(&(0..self.source.len()))
            || self.source.is_empty()
    }
}

/// Parses and fully highlights `source`, detecting the language from `path`.
/// Unknown languages and grammar failures yield plain-text analysis. Editor
/// buffers use [`analyze_lazy`] instead.
pub fn analyze(path: &Path, source: &str) -> LanguageAnalysis {
    analyze_with_budget(path, source, usize::MAX)
}

/// Like [`analyze`], but highlights only the first [`INITIAL_HIGHLIGHT_BYTES`];
/// the rest is computed on demand by [`LanguageAnalysis::ensure_highlighted`].
pub fn analyze_lazy(path: &Path, source: &str) -> LanguageAnalysis {
    analyze_with_budget(path, source, INITIAL_HIGHLIGHT_BYTES)
}

/// Like [`analyze`], highlighting only about the first `budget` bytes (whole
/// lines); the rest follows through [`LanguageAnalysis::ensure_highlighted`].
pub fn analyze_with_budget(path: &Path, source: &str, budget: usize) -> LanguageAnalysis {
    let Some(spec) = registry::detect(path, source) else {
        return LanguageAnalysis::default();
    };
    let mut result = LanguageAnalysis {
        language: Some(spec.name),
        ..Default::default()
    };
    let Some(syntax) = SyntaxTree::parse(spec, source) else {
        return result;
    };
    collect_diagnostics(syntax.tree().root_node(), &mut result.diagnostics);
    result.source = source.into();
    result.syntax = Some(syntax);
    let first = syntax::line_range(source, 0..budget.min(source.len()));
    result.highlight_region(first);
    result
}

fn collect_diagnostics(node: Node<'_>, diagnostics: &mut Vec<Diagnostic>) {
    if diagnostics.len() >= MAX_DIAGNOSTICS || !(node.has_error() || node.is_missing()) {
        return;
    }
    let push = |diagnostics: &mut Vec<Diagnostic>, point: tree_sitter::Point, message: String| {
        diagnostics.push(Diagnostic {
            message,
            row: point.row + 1,
            column: point.column + 1,
        });
    };
    if node.is_missing() {
        push(
            diagnostics,
            node.start_position(),
            format!("expected {}", node.kind()),
        );
        return;
    }
    if node.is_error() {
        push(diagnostics, node.start_position(), "syntax error".into());
    }
    let mut cursor = node.walk();
    let mut child_error = false;
    for child in node.children(&mut cursor) {
        child_error |= child.has_error() || child.is_missing();
        collect_diagnostics(child, diagnostics);
    }
    // Some grammars flag an error on a node without exposing an ERROR or
    // MISSING child (e.g. a hidden missing token); report it on the node.
    if !child_error && !node.is_error() {
        let point = if node.child_count() == 0 {
            node.start_position()
        } else {
            node.end_position()
        };
        push(
            diagnostics,
            point,
            format!("syntax error in {}", node.kind()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the attribute key highlighting the first occurrence of `needle`.
    fn key_of(analysis: &LanguageAnalysis, source: &str, needle: &str) -> Option<&'static str> {
        let start = source.find(needle)?;
        analysis
            .highlights
            .spans()
            .iter()
            .find(|span| span.start_byte <= start && start < span.end_byte)
            .map(|span| span.key)
    }

    #[test]
    fn rust_analysis_returns_highlights_without_diagnostics() {
        let source = "/// Docs\npub fn answer() -> u32 { 42 }\n";
        let analysis = analyze(Path::new("lib.rs"), source);
        assert_eq!(analysis.language, Some("Rust"));
        assert!(analysis.diagnostics.is_empty());
        assert_eq!(key_of(&analysis, source, "pub"), Some("DEFAULT_KEYWORD"));
        assert_eq!(key_of(&analysis, source, "42"), Some("DEFAULT_NUMBER"));
        assert_eq!(
            key_of(&analysis, source, "answer"),
            Some("DEFAULT_FUNCTION_DECLARATION")
        );
        assert_eq!(
            key_of(&analysis, source, "/// Docs"),
            Some("DEFAULT_DOC_COMMENT")
        );
    }

    #[test]
    fn rust_analysis_reports_parse_errors() {
        assert!(
            !analyze(Path::new("broken.rs"), "fn broken( {")
                .diagnostics
                .is_empty()
        );
    }

    #[test]
    fn unknown_files_remain_plain_text() {
        let analysis = analyze(Path::new("notes.txt"), "hello");
        assert!(analysis.language.is_none());
        assert!(analysis.highlights.is_empty());
    }

    #[test]
    fn java_tokens_resolve_to_islands_dark_colors() {
        let source = "package com.example;\n\n/** App. */\npublic class App {\n    // greet\n    public String greeting(String name) { return \"Hello, \" + name + 1; }\n    void run() { greeting(\"x\"); }\n}\n";
        let analysis = analyze(Path::new("App.java"), source);
        assert_eq!(analysis.language, Some("Java"));
        let scheme = ColorScheme::islands_dark();
        let color = |needle: &str| {
            key_of(&analysis, source, needle).map(|key| scheme.attributes(key).foreground)
        };
        assert_eq!(color("package"), Some(Rgba::parse("cf8e6d")));
        assert_eq!(color("public class"), Some(Rgba::parse("cf8e6d")));
        assert_eq!(color("\"Hello"), Some(Rgba::parse("6aab73")));
        assert_eq!(color("// greet"), Some(Rgba::parse("7a7e85")));
        assert_eq!(color("/** App"), Some(Rgba::parse("5f826b")));
        assert_eq!(color("greeting(String"), Some(Rgba::parse("56a8f5")));
        assert_eq!(color("1;"), Some(Rgba::parse("2aacb8")));
        // Calls and type references use the plain identifier color, as in IntelliJ.
        assert_eq!(color("greeting(\"x"), Some(Rgba::parse("bcbec4")));
        assert_ne!(key_of(&analysis, source, "String"), Some("DEFAULT_KEYWORD"));
        // Class declarations and type uses are not function declarations.
        assert_eq!(color("App {"), Some(Rgba::parse("bcbec4")));
        assert_eq!(color("String name)"), Some(Rgba::parse("bcbec4")));
        let errors = analyze(Path::new("Broken.java"), "class { void }");
        assert!(!errors.diagnostics.is_empty());
        assert_eq!(
            scheme.attributes("ERRORS_ATTRIBUTES").effect,
            Some((EffectType::WaveUnderline, Rgba::parse("fa6675")))
        );
    }

    #[test]
    fn scene_04_languages_highlight() {
        let cases: &[(&str, &str, &str, &str)] = &[
            (
                "Main.kt",
                "fun main() { val x = \"hi\" }\n",
                "fun",
                "DEFAULT_KEYWORD",
            ),
            (
                "Main.kt",
                "fun main() { val x = \"hi\" }\n",
                "\"hi\"",
                "DEFAULT_STRING",
            ),
            (
                "Main.kt",
                "fun main() { val x = \"hi\" }\n",
                "main",
                "DEFAULT_FUNCTION_DECLARATION",
            ),
            (
                "AndroidManifest.xml",
                "<manifest package=\"a\"/>\n",
                "manifest",
                "XML_TAG_NAME",
            ),
            (
                "AndroidManifest.xml",
                "<manifest package=\"a\"/>\n",
                "\"a\"",
                "DEFAULT_STRING",
            ),
            (
                "BUILD.bazel",
                "# c\njava_binary(name = \"app\", srcs = [\"A.java\"])\n",
                "\"app\"",
                "DEFAULT_STRING",
            ),
            (
                "BUILD.bazel",
                "# c\njava_binary(name = \"app\")\n",
                "# c",
                "DEFAULT_LINE_COMMENT",
            ),
            (
                "defs.bzl",
                "def rule_impl(ctx):\n    return None\n",
                "def",
                "DEFAULT_KEYWORD",
            ),
            (
                "Cargo.toml",
                "[package]\nname = \"x\"\n",
                "\"x\"",
                "DEFAULT_STRING",
            ),
            ("a.json", "{\"k\": 1}", "1", "DEFAULT_NUMBER"),
            ("a.yaml", "k: true\n", "true", "DEFAULT_KEYWORD"),
            ("a.py", "import os\n", "import", "DEFAULT_KEYWORD"),
            (
                "build.gradle",
                "plugins {\n    id 'java'\n}\ndef x = 1\n",
                "'java'",
                "DEFAULT_STRING",
            ),
            (
                "build.gradle",
                "plugins {\n    id 'java'\n}\ndef x = 1\n",
                "def",
                "DEFAULT_KEYWORD",
            ),
            ("a.py", "x = None\n", "None", "DEFAULT_KEYWORD"),
            ("lib.rs", "fn f(x: u32) {}\n", "u32", "DEFAULT_KEYWORD"),
            ("A.java", "class A { int x; }\n", "int", "DEFAULT_KEYWORD"),
            (
                "Main.kt",
                "package a\nfun f(s: String) {}\n",
                "String",
                "DEFAULT_CLASS_REFERENCE",
            ),
            (
                "Main.kt",
                "package a\nfun f(s: String) {}\n",
                "package",
                "DEFAULT_KEYWORD",
            ),
            (
                "Main.kt",
                "class A { override fun g() {} }\n",
                "g()",
                "DEFAULT_FUNCTION_DECLARATION",
            ),
            ("a.go", "package main\n", "package", "DEFAULT_KEYWORD"),
            ("a.ts", "const x: number = 1;\n", "const", "DEFAULT_KEYWORD"),
            (
                "a.cpp",
                "int main() { return 0; }\n",
                "return",
                "DEFAULT_KEYWORD",
            ),
            (
                "a.sh",
                "if true; then echo hi; fi\n",
                "if",
                "DEFAULT_KEYWORD",
            ),
            (
                "a.html",
                "<div class=\"x\"></div>\n",
                "div",
                "HTML_TAG_NAME",
            ),
            (
                "a.css",
                "a { color: red; }\n",
                "color",
                "DEFAULT_INSTANCE_FIELD",
            ),
        ];
        for (path, source, needle, expected) in cases {
            let analysis = analyze(Path::new(path), source);
            assert!(analysis.language.is_some(), "{path} not detected");
            assert_eq!(
                key_of(&analysis, source, needle),
                Some(*expected),
                "{path}: `{needle}` in {:?}",
                analysis.highlights
            );
        }
    }

    #[test]
    fn lazy_highlighting_fills_viewports_and_survives_edits() {
        let mut source = String::new();
        for i in 0..300 {
            source.push_str(&format!(
                "/** d{i} */\nclass C{i} {{ int f() {{ return {i}; }} }}\n"
            ));
        }
        let path = Path::new("Big.java");
        let mut lazy = analyze_with_budget(path, &source, 2_000);
        assert!(!lazy.fully_highlighted());
        assert!(
            lazy.highlights
                .spans()
                .iter()
                .all(|span| span.end_byte <= 2_100)
        );
        let mid = source.len() / 2;
        lazy.ensure_highlighted(mid..mid + 3_000);
        // Edit once inside highlighted text and once in never-shown text.
        let mut document = archipel_ide_text::TextDocument::new(&source);
        document.take_edits();
        document.edit(10, 10, "x").unwrap();
        let far = document.len_chars() - 50;
        document.edit(far, far, "/* c */").unwrap();
        let text = document.text();
        let archipel_ide_text::EditJournal::Edits(edits) = document.take_edits() else {
            panic!("journal");
        };
        lazy.apply_edits(path, &text, &edits);
        lazy.ensure_highlighted(0..text.len());
        assert!(lazy.fully_highlighted());
        assert_eq!(
            lazy.highlights,
            analyze_with_budget(path, &text, usize::MAX).highlights
        );
    }

    #[test]
    fn injected_languages_are_highlighted() {
        let cases: &[(&str, &str, &str, &str)] = &[
            (
                "README.md",
                "# T\n\n```kotlin\nfun main() {}\n```\n",
                "fun",
                "DEFAULT_KEYWORD",
            ),
            (
                "README.md",
                "# T\n\n```java\nclass A {}\n```\n",
                "class",
                "DEFAULT_KEYWORD",
            ),
            (
                "README.md",
                "Use `code` and [link](http://x).\n",
                "`code`",
                "DEFAULT_STRING",
            ),
            (
                "a.html",
                "<script>let x = 1;</script>\n",
                "let",
                "DEFAULT_KEYWORD",
            ),
            (
                "a.html",
                "<style>a { color: red; }</style>\n",
                "color",
                "DEFAULT_INSTANCE_FIELD",
            ),
            (
                "main.rs",
                "fn main() { println!(\"hi {}\", 1); }\n",
                "\"hi",
                "DEFAULT_STRING",
            ),
        ];
        for (path, source, needle, expected) in cases {
            let analysis = analyze(Path::new(path), source);
            assert_eq!(
                key_of(&analysis, source, needle),
                Some(*expected),
                "{path}: `{needle}` in {:?}",
                analysis.highlights
            );
        }
    }

    #[test]
    fn highlights_never_split_or_overlap() {
        for spec in LANGUAGES {
            let path = format!("x.{}", spec.extensions.first().copied().unwrap_or("txt"));
            let source = "fn x() { \"ä\" } // ü\n<a b=\"c\">ä</a> # 1.5e3\n";
            let analysis = analyze(Path::new(&path), source);
            let mut last = 0;
            for span in analysis.highlights.spans() {
                assert!(
                    span.start_byte >= last && span.start_byte < span.end_byte,
                    "{path}"
                );
                assert!(
                    source.is_char_boundary(span.start_byte)
                        && source.is_char_boundary(span.end_byte)
                );
                last = span.end_byte;
            }
        }
    }
}

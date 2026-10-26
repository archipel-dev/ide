//! Incremental reparse + re-highlight must equal a full analysis after any
//! sequence of buffer edits.

use archipel_ide_language::{EditMap, LanguageAnalysis, analyze, analyze_with_budget};
use archipel_ide_text::{EditJournal, TextDocument};
use proptest::prelude::*;
use std::path::Path;

const SAMPLES: &[(&str, &str)] = &[
    (
        "App.java",
        "package com.example;\n\n/** Doc. */\npublic class App {\n    public String greeting(String name) { return \"Hello, \" + name + \"!\"; }\n    // comment\n    static final int MAX = 42;\n}\n",
    ),
    (
        "Main.kt",
        "package a.b\n\nclass Main : Base() {\n    override fun run(x: Int): String = \"v=$x\"\n}\n",
    ),
    (
        "lib.rs",
        "/// Docs\npub fn answer(x: u32) -> u32 {\n    let s = \"ä\\n\";\n    x + 42 // ü\n}\n",
    ),
    (
        "BUILD.bazel",
        "load(\"@r//:defs.bzl\", \"x\")\n\n# c\njava_binary(\n    name = \"app\",\n    srcs = [\"A.java\"],\n)\n",
    ),
    (
        "README.md",
        "# Title\n\nSome *text* with `code`.\n\n```kotlin\nfun main() { println(\"x\") }\n```\n\n- item\n",
    ),
    (
        "index.html",
        "<html>\n<script>let x = `t`;</script>\n<style>a { color: red; }</style>\n<p class=\"c\">hi</p>\n</html>\n",
    ),
    (
        "layout.xml",
        "<?xml version=\"1.0\"?>\n<a b=\"c\">\n  <!-- c -->\n  <d e='f'/>text &amp;\n</a>\n",
    ),
];

const FRAGMENTS: &[&str] = &[
    "",
    "x",
    " ",
    "\n",
    "{",
    "}",
    "(",
    ")",
    "\"",
    "'",
    "/*",
    "*/",
    "//",
    "<",
    ">",
    "fun ",
    "class ",
    "return ",
    "42",
    "```",
    "*",
    "`",
    "</script>",
    "ä",
    "🦀",
    ";",
    "=",
    "\"s\"",
    "/** d */",
    "#",
    "    ",
];

#[derive(Debug, Clone)]
enum Op {
    Edit {
        at: f64,
        delete: usize,
        insert: usize,
    },
    Undo,
    Redo,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        8 => (0.0f64..1.0, 0usize..6, 0..FRAGMENTS.len())
            .prop_map(|(at, delete, insert)| Op::Edit { at, delete, insert }),
        1 => Just(Op::Undo),
        1 => Just(Op::Redo),
    ]
}

/// Incremental highlights equal a full highlight of the same tree (whenever
/// injected regions parse without errors).
/// The tree itself equals a fresh parse whenever the text is error-free
/// (Tree-sitter's error recovery may legitimately differ for broken text).
fn assert_same(path: &str, incremental: &LanguageAnalysis, source: &str) {
    let full = analyze(Path::new(path), source);
    let Some(syntax) = incremental.syntax.as_ref() else {
        assert!(full.syntax.is_none());
        return;
    };
    // Injected trees with errors may recover differently at another position
    // (a Tree-sitter property), so they are only compared when error-free.
    if incremental.injections_error_free() {
        assert_eq!(
            incremental.highlights,
            syntax.highlight(source, 0..source.len()),
            "{path}: highlights for {source:?}"
        );
    }
    assert_eq!(
        incremental.diagnostics.is_empty(),
        !syntax.tree().root_node().has_error(),
        "{path}: diagnostics for {source:?}"
    );
    if full.diagnostics.is_empty() {
        let sexp = |analysis: &LanguageAnalysis| {
            analysis
                .syntax
                .as_ref()
                .map(|syntax| syntax.tree().root_node().to_sexp())
        };
        assert_eq!(
            sexp(incremental),
            sexp(&full),
            "{path}: tree for {source:?}"
        );
        if incremental.injections_error_free() {
            assert_eq!(
                incremental.highlights, full.highlights,
                "{path}: {source:?}"
            );
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn incremental_analysis_matches_full(
        sample in 0..SAMPLES.len(),
        lazy in any::<bool>(),
        viewport in 0.0f64..1.0,
        ops in prop::collection::vec(op(), 1..24),
    ) {
        let (path, initial) = SAMPLES[sample];
        let mut document = TextDocument::new(initial);
        // Lazy analyses highlight only a few bytes up front and fill in
        // viewports on demand, like the editor does for large files.
        let budget = if lazy { 24 } else { usize::MAX };
        let mut analysis = analyze_with_budget(Path::new(path), initial, budget);
        assert!(matches!(document.take_edits(), EditJournal::Reset));
        for op in ops {
            match op {
                Op::Edit { at, delete, insert } => {
                    let len = document.len_chars();
                    let start = ((len as f64) * at) as usize;
                    let end = (start + delete).min(len);
                    document.edit(start, end, FRAGMENTS[insert]).unwrap();
                }
                Op::Undo => {
                    document.undo();
                }
                Op::Redo => {
                    document.redo();
                }
            }
            let source = document.text();
            match document.take_edits() {
                EditJournal::Edits(edits) => analysis.apply_edits(Path::new(path), &source, &edits),
                EditJournal::Reset => analysis = analyze(Path::new(path), &source),
            }
            let at = ((source.len() as f64) * viewport) as usize;
            analysis.ensure_highlighted(at..at + 40);
            let mut complete = analysis.clone();
            complete.ensure_highlighted(0..source.len());
            assert!(complete.fully_highlighted());
            assert_same(path, &complete, &source);
        }
    }
}

/// Kind and byte range of every node of a tree.
fn nodes(tree: &tree_sitter::Tree) -> Vec<(u16, usize, usize)> {
    let mut out = Vec::new();
    let mut cursor = tree.walk();
    loop {
        let node = cursor.node();
        out.push((node.kind_id(), node.start_byte(), node.end_byte()));
        if cursor.goto_first_child() {
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

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// The changed ranges an incremental reparse reports (the host's
    /// `SyntaxChanged` payload) cover every node that is new: each node of
    /// the new tree either existed before (same kind, same range mapped
    /// through the edits, no edit inside) or intersects a reported range.
    #[test]
    fn changed_ranges_cover_every_structural_change(
        sample in 0..SAMPLES.len(),
        ops in prop::collection::vec(op(), 1..16),
    ) {
        let (path, initial) = SAMPLES[sample];
        let mut document = TextDocument::new(initial);
        let mut analysis = analyze(Path::new(path), initial);
        document.take_edits();
        analysis.take_syntax_changes();
        for op in ops {
            let before: std::collections::HashSet<_> = analysis
                .syntax
                .as_ref()
                .map(|syntax| nodes(syntax.tree()))
                .unwrap_or_default()
                .into_iter()
                .collect();
            match op {
                Op::Edit { at, delete, insert } => {
                    let len = document.len_chars();
                    let start = ((len as f64) * at) as usize;
                    let end = (start + delete).min(len);
                    document.edit(start, end, FRAGMENTS[insert]).unwrap();
                }
                Op::Undo => {
                    document.undo();
                }
                Op::Redo => {
                    document.redo();
                }
            }
            let source = document.text();
            let edits = match document.take_edits() {
                EditJournal::Edits(edits) => edits,
                EditJournal::Reset => {
                    analysis = analyze(Path::new(path), &source);
                    analysis.take_syntax_changes();
                    continue;
                }
            };
            if edits.is_empty() {
                continue;
            }
            let map = EditMap::from_edits(&edits);
            analysis.apply_edits(Path::new(path), &source, &edits);
            let changes = analysis.take_syntax_changes();
            prop_assert!(!changes.is_empty());
            let survived: std::collections::HashSet<_> = before
                .iter()
                .filter(|(_, start, end)| map.untouched(*start..*end))
                .map(|(kind, start, end)| (*kind, map.map(*start), map.map(*end)))
                .collect();
            let Some(syntax) = analysis.syntax.as_ref() else {
                continue;
            };
            for (kind, start, end) in nodes(syntax.tree()) {
                if survived.contains(&(kind, start, end)) {
                    continue;
                }
                prop_assert!(
                    changes.iter().any(|range| range.start <= end && start <= range.end),
                    "{path}: new node {kind}@{start}..{end} outside {changes:?} in {source:?}"
                );
            }
        }
    }
}

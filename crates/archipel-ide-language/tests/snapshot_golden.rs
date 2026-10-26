//! Golden test: the generic features (highlight, structure, folds,
//! selection, brackets, diagnostics) give identical answers through the
//! `LanguageAnalysis` facade, the new `SyntaxSnapshot` API and the feature
//! modules called directly on the tree, for every registered language.

use archipel_ide_language::{
    FileId, FileRev, LANGUAGES, SyntaxSnapshot, analyze, brackets, folding, selection, structure,
};
use std::path::Path;

/// One sample per language family (the same texts the language tests use).
const SAMPLES: &[(&str, &str)] = &[
    (
        "App.java",
        "package com.example;\n\n/** Doc. */\npublic class App {\n    public String greeting(String name) {\n        return \"Hello, \" + name;\n    }\n    // c\n    static final int MAX = 42;\n}\n",
    ),
    (
        "lib.rs",
        "/// Docs\npub fn answer(x: u32) -> u32 {\n    let s = \"ä\\n\";\n    x + 42 // ü\n}\n\nmod inner {\n    pub struct P { a: i32 }\n}\n",
    ),
    (
        "Main.kt",
        "package a.b\n\nclass Main : Base() {\n    override fun run(x: Int): String = \"v=$x\"\n}\n",
    ),
    (
        "a.py",
        "import os\n\nclass A:\n    def f(self, x):\n        return [x, (1, 2)]\n",
    ),
    (
        "a.go",
        "package main\n\nfunc main() {\n\tx := []int{1, 2}\n\t_ = x\n}\n",
    ),
    (
        "a.ts",
        "interface I { a: number }\nfunction f(x: I): number {\n  return x.a;\n}\n",
    ),
    ("a.json", "{\"k\": [1, 2, {\"n\": null}]}\n"),
    (
        "README.md",
        "# Title\n\nText *em*.\n\n```kotlin\nfun main() {}\n```\n",
    ),
    (
        "index.html",
        "<html>\n<script>let x = 1;</script>\n<p class=\"c\">hi</p>\n</html>\n",
    ),
    (
        "BUILD.bazel",
        "java_binary(\n    name = \"app\",\n    srcs = [\"A.java\"],\n)\n",
    ),
    ("broken.rs", "fn broken( {\n    let = ;\n"),
];

#[test]
fn snapshot_api_matches_the_facade_and_the_feature_modules() {
    for (path, source) in SAMPLES {
        let analysis = analyze(Path::new(path), source);
        let Some(syntax) = analysis.syntax.as_ref() else {
            panic!("{path} not parsed");
        };
        let lang = syntax.language();
        let snap =
            SyntaxSnapshot::parse(FileId(3), FileRev(7), lang, (*source).into()).expect("parse");
        let via_facade = analysis.snapshot(FileId(3), FileRev(7)).expect("snapshot");
        assert_eq!(
            snap.root().raw().to_sexp(),
            via_facade.root().raw().to_sexp(),
            "{path}"
        );
        // Highlights: the facade's lazily filled layer equals the snapshot's.
        assert_eq!(
            analysis.highlights,
            snap.highlight(0..source.len()),
            "{path} highlights"
        );
        // Structure.
        let direct = structure::structure(lang, syntax.tree(), source);
        assert_eq!(analysis.structure(), direct, "{path} structure");
        assert_eq!(snap.structure(), direct, "{path} structure");
        // Folds.
        let direct = folding::fold_ranges(lang, syntax.tree(), source, 0..source.len());
        assert_eq!(analysis.folds_in(0..source.len()), direct, "{path} folds");
        assert_eq!(snap.folds_in(0..source.len()), direct, "{path} folds");
        // Selection and brackets at every char boundary.
        for offset in (0..=source.len()).filter(|i| source.is_char_boundary(*i)) {
            let direct = selection::expand(syntax.tree(), source, offset..offset);
            assert_eq!(
                analysis.expand_selection(source, offset..offset),
                direct,
                "{path}@{offset}"
            );
            assert_eq!(
                snap.expand_selection(offset..offset),
                direct,
                "{path}@{offset}"
            );
            let direct = brackets::matching_bracket(syntax.tree(), offset);
            assert_eq!(analysis.matching_bracket(offset), direct, "{path}@{offset}");
            assert_eq!(snap.matching_bracket(offset), direct, "{path}@{offset}");
        }
        // Diagnostics.
        assert_eq!(
            analysis.diagnostics,
            snap.diagnostics(),
            "{path} diagnostics"
        );
        assert_eq!(
            snap.errors().is_empty(),
            analysis.diagnostics.is_empty(),
            "{path} errors"
        );
    }
}

#[test]
fn every_language_builds_tables_and_snapshots() {
    for lang in LANGUAGES {
        let table = lang.element_types();
        assert!(!table.is_empty(), "{}", lang.id);
        let snap = SyntaxSnapshot::parse(FileId(1), FileRev(1), lang, "x".into()).expect("parse");
        let root = snap.root();
        assert_eq!(table.name(root.element_type()), root.kind(), "{}", lang.id);
        assert!(root.preorder().count() >= 1);
    }
}

//! `docs/analysis/language-features.md` is generated from the query files and
//! data rows; this test fails when it is stale (`UPDATE_SNAPSHOTS=1` rewrites
//! it) and checks that every query-backed feature has a golden test.

use archipel_ide_language::LANGUAGES;
use archipel_ide_language::matrix;
use archipel_ide_language::queryset::{QueryKind, query};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn golden(lang: &str, feature: &str) -> bool {
    root()
        .join("tests/golden")
        .join(lang)
        .join(format!("{feature}.expected"))
        .exists()
}

#[test]
fn the_feature_matrix_document_is_current() {
    let rendered = matrix::render(&golden);
    let path = root().join("../../docs/analysis/language-features.md");
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, &rendered).expect("write matrix");
        return;
    }
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        current, rendered,
        "docs/analysis/language-features.md is stale: run `UPDATE_SNAPSHOTS=1 cargo test -p archipel-ide-language --test feature_matrix`"
    );
}

#[test]
fn every_query_backed_feature_has_a_golden_test() {
    let wanted = [
        (QueryKind::Folds, "folds"),
        (QueryKind::Tags, "structure"),
        (QueryKind::Brackets, "brackets"),
        (QueryKind::Indents, "indents"),
        (QueryKind::Sticky, "sticky"),
        (QueryKind::TextObjects, "textobjects"),
        (QueryKind::Locals, "locals"),
    ];
    let mut missing = Vec::new();
    for spec in LANGUAGES.iter().filter(|s| s.id != "markdown_inline") {
        if !golden(spec.id, "selection") {
            missing.push(format!("{}/selection", spec.id));
        }
        for (kind, feature) in wanted {
            if query(spec, kind).is_some() && !golden(spec.id, feature) {
                missing.push(format!("{}/{feature}", spec.id));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "no golden test for: {}",
        missing.join(", ")
    );
}

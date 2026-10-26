//! Every query file of every language compiles against its grammar and uses
//! only the capture names the engines know.

use archipel_ide_language::LANGUAGES;
use archipel_ide_language::queryset::{QueryKind, compile_error, query, source};

/// Capture-name prefixes each engine reads (anything else is a typo).
fn known(kind: QueryKind, capture: &str) -> bool {
    let prefixes: &[&str] = match kind {
        QueryKind::Folds => &["fold"],
        QueryKind::Indents => &["indent", "outdent"],
        QueryKind::Brackets => &["open", "close"],
        QueryKind::TextObjects => &[
            "select",
            "function",
            "class",
            "parameter",
            "comment",
            "block",
            "call",
        ],
        QueryKind::Sticky => &["sticky"],
        QueryKind::Doc => &["doc"],
        QueryKind::Locals => &["local"],
        QueryKind::Tags => &["name", "definition", "modifier", "detail", "reference"],
    };
    prefixes
        .iter()
        .any(|prefix| capture == *prefix || capture.starts_with(&format!("{prefix}.")))
        || capture.starts_with('_')
}

#[test]
fn all_queries_compile_and_use_known_captures() {
    let mut failures = Vec::new();
    for spec in LANGUAGES {
        for kind in QueryKind::ALL {
            if source(spec, kind).is_none() {
                continue;
            }
            if let Some(error) = compile_error(spec, kind) {
                failures.push(format!("{} {}: {error}", spec.id, kind.file_name()));
                continue;
            }
            let Some(compiled) = query(spec, kind) else {
                continue;
            };
            // Upstream highlight-style tags (`@reference.call`) are tolerated in tags.
            for capture in compiled.capture_names() {
                if !known(kind, capture) {
                    failures.push(format!(
                        "{} {}: unknown capture @{capture}",
                        spec.id,
                        kind.file_name()
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

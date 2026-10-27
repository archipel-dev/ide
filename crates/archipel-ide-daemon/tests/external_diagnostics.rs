//! LSP diagnostics as an external pass: the feed, `diagnostics_published`,
//! and the deduplication against native highlights (spec 1.10).

mod common;
use archipel_ide_analysis::{FileRev, HighlightKind};
use archipel_ide_daemon::markup::Coexistence;
use archipel_ide_daemon::{ExternalDiagnostic, ExternalFile, FileMarkup};
use common::daemon_harness::DaemonHarness;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

const JAVA: &str = "class A {\n  void f() {\n    try { g(); } catch (Exception e) { }\n  }\n}\n";

fn diagnostic(range: Range<u32>, severity: u8, message: &str) -> ExternalDiagnostic {
    ExternalDiagnostic {
        range,
        start: (0, 0),
        end: (0, 0),
        severity,
        message: message.into(),
        code: Some("67108964".into()),
        source: Some("Java".into()),
        unnecessary: false,
        deprecated: false,
    }
}

fn publish(h: &DaemonHarness, path: &str, rev: u64, diagnostics: Vec<ExternalDiagnostic>) {
    h.svc.shared().external.publish(
        Path::new(path),
        ExternalFile {
            server: "jdtls".into(),
            rev: Some(FileRev(rev)),
            diagnostics: Arc::new(diagnostics),
        },
    );
}

/// `(owner, message)` of every visible problem.
fn problems(h: &DaemonHarness, file: archipel_ide_analysis::FileId) -> Vec<(String, String)> {
    h.problems(file)
}

#[test]
fn nothing_is_shown_until_the_provider_publishes_and_the_daemon_is_told() {
    let mut h = DaemonHarness::new();
    let file = h.open("A.java", "class A { void f() { g(); } }\n");
    h.settle(Duration::from_millis(100));
    publish(
        &h,
        "A.java",
        1,
        vec![diagnostic(
            21..22,
            1,
            "The method g() is undefined for the type A",
        )],
    );
    // The feed alone changes nothing: the daemon is the only producer.
    h.settle(Duration::from_millis(100));
    assert!(
        !problems(&h, file)
            .iter()
            .any(|(tool, _)| tool == "lsp:jdtls"),
        "{:?}",
        problems(&h, file)
    );
    h.svc.lock().diagnostics_published(file);
    assert!(!h.svc.lock().all_idle(), "the pass is scheduled");
    h.settle(Duration::from_millis(100));
    let found = problems(&h, file);
    assert!(
        found.contains(&(
            "lsp:jdtls".to_owned(),
            "The method g() is undefined for the type A".to_owned()
        )),
        "{found:?}"
    );
    let svc = h.svc.lock();
    let markup = svc.markup(file).unwrap();
    let info = markup
        .iter()
        .find(|i| i.tool.as_str() == "lsp:jdtls")
        .unwrap();
    assert_eq!(info.range, 21..22);
    assert_eq!(info.kind, HighlightKind::Error);
    assert_eq!(info.code.as_deref(), Some("67108964"));
    assert_eq!(info.source.as_deref(), Some("Java"));
    assert_eq!(markup.counts().errors, 1);
}

#[test]
fn republishing_replaces_and_withdrawing_clears() {
    let mut h = DaemonHarness::new();
    let file = h.open("A.java", "class A { void f() { g(); } }\n");
    h.settle(Duration::from_millis(100));
    publish(&h, "A.java", 1, vec![diagnostic(21..22, 1, "first")]);
    h.svc.lock().diagnostics_published(file);
    h.settle(Duration::from_millis(100));
    publish(&h, "A.java", 1, vec![diagnostic(21..22, 2, "second")]);
    h.svc.lock().diagnostics_published(file);
    h.settle(Duration::from_millis(100));
    let found = problems(&h, file);
    assert!(
        found.iter().all(|(_, message)| message != "first"),
        "{found:?}"
    );
    assert!(found.iter().any(|(_, message)| message == "second"));
    // A clean list is a result too.
    publish(&h, "A.java", 1, Vec::new());
    h.svc.lock().diagnostics_published(file);
    h.settle(Duration::from_millis(100));
    assert!(
        !problems(&h, file)
            .iter()
            .any(|(tool, _)| tool.starts_with("lsp:"))
    );
    // Withdrawn after a restart: whatever the provider left is cleared.
    publish(&h, "A.java", 1, vec![diagnostic(21..22, 1, "again")]);
    h.svc.lock().diagnostics_published(file);
    h.settle(Duration::from_millis(100));
    assert!(problems(&h, file).iter().any(|(_, m)| m == "again"));
    h.svc.shared().external.withdraw(Path::new("A.java"));
    h.svc.lock().diagnostics_published(file);
    h.settle(Duration::from_millis(100));
    assert!(
        !problems(&h, file)
            .iter()
            .any(|(tool, _)| tool.starts_with("lsp:"))
    );
}

#[test]
fn results_for_an_older_revision_are_mapped_through_the_edits() {
    let mut h = DaemonHarness::new();
    let text = "class A { void f() { g(); } }\n";
    let file = h.open("A.java", text);
    h.settle(Duration::from_millis(100));
    // The server answered for revision 1 while the user typed a comment line.
    h.edit(file, text, 0..0, "// hi\n");
    h.tick();
    publish(&h, "A.java", 1, vec![diagnostic(21..22, 1, "undefined")]);
    h.svc.lock().diagnostics_published(file);
    h.settle(Duration::from_millis(100));
    let svc = h.svc.lock();
    let info = svc
        .markup(file)
        .unwrap()
        .iter()
        .find(|i| i.tool.as_str() == "lsp:jdtls")
        .cloned()
        .expect("mapped, not dropped");
    assert_eq!(info.range, 27..28, "moved by the six inserted bytes");
}

#[test]
fn native_syntax_errors_give_way_once_the_server_has_published() {
    let mut h = DaemonHarness::new();
    let file = h.open("A.java", "class A {\n  void f() {\n");
    h.settle(Duration::from_millis(100));
    assert!(
        problems(&h, file)
            .iter()
            .any(|(tool, _)| tool == "native:syntax"),
        "parse errors show until a server has checked the file"
    );
    publish(
        &h,
        "A.java",
        1,
        vec![diagnostic(10..11, 1, "Syntax error, insert \"}\"")],
    );
    h.svc.lock().diagnostics_published(file);
    h.settle(Duration::from_millis(100));
    let found = problems(&h, file);
    assert!(
        !found.iter().any(|(tool, _)| tool == "native:syntax"),
        "{found:?}"
    );
    assert!(found.iter().any(|(tool, _)| tool == "lsp:jdtls"));
    // An empty published list also means "the server checked it".
    publish(&h, "A.java", 1, Vec::new());
    h.svc.lock().diagnostics_published(file);
    h.settle(Duration::from_millis(100));
    assert!(problems(&h, file).is_empty(), "{:?}", problems(&h, file));
    // Not published any more: the parse errors are back.
    h.svc.shared().external.withdraw(Path::new("A.java"));
    h.svc.lock().diagnostics_published(file);
    h.settle(Duration::from_millis(100));
    assert!(
        problems(&h, file)
            .iter()
            .any(|(tool, _)| tool == "native:syntax")
    );
}

#[test]
fn a_native_twin_of_a_server_diagnostic_is_one_problem() {
    let mut h = DaemonHarness::new();
    let file = h.open("A.java", JAVA);
    h.settle(Duration::from_millis(100));
    let (range, kind) = {
        let svc = h.svc.lock();
        let info = svc
            .markup(file)
            .unwrap()
            .iter()
            .find(|i| i.tool.as_str() == "native:EmptyCatchBlock")
            .cloned()
            .expect("native result");
        (info.range, info.kind)
    };
    let severity = match kind {
        HighlightKind::Error => 1,
        HighlightKind::Warning => 2,
        _ => 3,
    };
    publish(
        &h,
        "A.java",
        1,
        vec![diagnostic(range.clone(), severity, "Empty catch block")],
    );
    h.svc.lock().diagnostics_published(file);
    h.settle(Duration::from_millis(100));
    let svc = h.svc.lock();
    let markup = svc.markup(file).unwrap();
    let at_range: Vec<_> = markup
        .query(range.clone())
        .into_iter()
        .filter(|i| i.range == range && i.kind == kind)
        .collect();
    assert_eq!(at_range.len(), 1, "one problem, not two: {at_range:?}");
    // Java is not graduated (G0): the server's result wins over the heuristic.
    assert_eq!(at_range[0].tool.as_str(), "lsp:jdtls");
    assert_eq!(markup.shadowed().count(), 1);
    let counts = markup.counts();
    assert_eq!(counts.errors + counts.warnings + counts.weak_warnings, 1);
}

#[test]
fn a_graduated_language_keeps_the_native_result() {
    use archipel_ide_analysis::{HighlightInfo, Severity, ToolId};
    let mut markup = FileMarkup::new(FileRev(1));
    let native = HighlightInfo::new(
        5..9,
        Severity::ERROR,
        HighlightKind::Error,
        ToolId::native("resolve"),
    )
    .message("Cannot resolve method 'assertThat'");
    let lsp = HighlightInfo::new(
        5..9,
        Severity::ERROR,
        HighlightKind::Error,
        ToolId::lsp("jdtls"),
    )
    .message("The method assertThat(String) is undefined for the type AppTest");
    let other = HighlightInfo::new(
        20..22,
        Severity::ERROR,
        HighlightKind::Error,
        ToolId::lsp("jdtls"),
    )
    .message("only the server knows");
    markup.replace(&ToolId::native("resolve"), 0..u32::MAX, vec![native]);
    markup.replace(&ToolId::lsp("jdtls"), 0..u32::MAX, vec![lsp, other]);
    let graduated = Coexistence {
        graduated: true,
        external_published: true,
    };
    assert!(markup.reconcile(graduated));
    let messages: Vec<_> = markup
        .iter()
        .map(|i| i.message.as_deref().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(
        messages,
        [
            "Cannot resolve method 'assertThat'",
            "only the server knows"
        ]
    );
    assert!(!markup.reconcile(graduated), "stable");
    // The same entries, language not graduated: the server wins.
    assert!(markup.reconcile(Coexistence {
        graduated: false,
        external_published: true
    }));
    let owners: Vec<_> = markup.iter().map(|i| i.tool.as_str().to_owned()).collect();
    assert_eq!(owners, ["lsp:jdtls", "lsp:jdtls"]);
}

#[test]
fn kinds_must_match_for_a_dedupe() {
    use archipel_ide_analysis::{HighlightInfo, Severity, ToolId};
    let mut markup = FileMarkup::new(FileRev(1));
    markup.replace(
        &ToolId::native("x"),
        0..u32::MAX,
        vec![HighlightInfo::new(
            1..3,
            Severity::WARNING,
            HighlightKind::Warning,
            ToolId::native("x"),
        )],
    );
    markup.replace(
        &ToolId::lsp("jdtls"),
        0..u32::MAX,
        vec![
            HighlightInfo::new(
                1..3,
                Severity::ERROR,
                HighlightKind::Error,
                ToolId::lsp("jdtls"),
            ),
            HighlightInfo::new(
                1..4,
                Severity::WARNING,
                HighlightKind::Warning,
                ToolId::lsp("jdtls"),
            ),
        ],
    );
    for graduated in [true, false] {
        markup.reconcile(Coexistence {
            graduated,
            external_published: true,
        });
        assert_eq!(
            markup.iter().count(),
            3,
            "different kind or range: no dedupe"
        );
    }
}

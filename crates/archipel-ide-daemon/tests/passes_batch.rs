//! Annotators, external annotators, line markers, global inspections and the
//! batch runner.

mod common;
use archipel_ide_analysis::daemon::{AnnotateCx, AnnotationHolder, Annotator, ExternalAnnotator};
use archipel_ide_analysis::inspect::{
    AnalysisScope, GlobalCx, GlobalInspection, ProblemDescriptor, ProblemHighlightType, ProblemSink,
};
use archipel_ide_analysis::{
    CancelToken, Cancelled, FileId, FileRev, HighlightKind, HostConfig, Severity, Snapshot,
    SyntaxNode, TokenSet,
};
use archipel_ide_daemon::inspection::batch::{BatchEvent, run_batch, start_batch};
use archipel_ide_daemon::inspection::profile::{
    EffectiveProfile, Origin, ProfileFile, ProfileStore,
};
use archipel_ide_daemon::{LineMarker, LineMarkerProvider};
use archipel_ide_language::{LanguageId, Registration, registry};
use common::daemon_harness::DaemonHarness;
use std::sync::Arc;
use std::time::Duration;

/// Flags `println!`-like call names in Rust source via the annotator API.
struct CommentAnnotator {
    set: TokenSet,
}

impl Annotator for CommentAnnotator {
    fn interesting(&self) -> &TokenSet {
        &self.set
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn annotate(
        &self,
        n: SyntaxNode<'_>,
        h: &mut AnnotationHolder,
        cx: &AnnotateCx<'_>,
    ) -> Result<(), Cancelled> {
        cx.cancel.check()?;
        if n.text().contains("HACK") {
            h.new_annotation(Severity::WARNING, "hack comment")
                .range(n.range())
                .create();
        }
        Ok(())
    }
}

struct FakeExternal;

impl ExternalAnnotator for FakeExternal {
    type Info = Arc<str>;
    type Result = Vec<std::ops::Range<u32>>;
    fn collect(&self, snap: &Snapshot, file: FileId) -> Option<Self::Info> {
        snap.file_text(file).ok()
    }
    #[allow(clippy::single_range_in_vec_init)]
    fn run(&self, info: Self::Info, _cancel: &CancelToken) -> Option<Self::Result> {
        let at = info.find("lint-me")?;
        Some(vec![at as u32..at as u32 + 7])
    }
    fn apply(&self, _file: FileId, _rev: FileRev, r: Self::Result, h: &mut AnnotationHolder) {
        for range in r {
            h.new_annotation(Severity::WARNING, "external lint says no")
                .range(range)
                .create();
        }
    }
}

struct FnMarkers {
    set: TokenSet,
}

impl LineMarkerProvider for FnMarkers {
    fn interesting(&self) -> &TokenSet {
        &self.set
    }
    fn markers(&self, node: SyntaxNode<'_>, out: &mut Vec<LineMarker>) {
        out.push(LineMarker {
            range: node.range(),
            kind: "function",
            tooltip: "a function".into(),
        });
    }
}

#[test]
fn annotators_external_annotators_and_line_markers_feed_the_markup() {
    let rust = registry::by_id("rust").unwrap();
    let comments = rust.token_set_table().comments.clone();
    let functions = rust.element_types().token_set(&["function_item"]);
    let mut h = DaemonHarness::with(HostConfig::testing(), move |registry| {
        registry.register::<dyn Annotator>(
            Registration::new("hack", LanguageId("rust")).dumb_aware(true),
            Arc::new(CommentAnnotator { set: comments }),
        );
        registry.register::<dyn archipel_ide_analysis::daemon::DynExternalAnnotator>(
            Registration::any("ext"),
            Arc::new(FakeExternal),
        );
        registry.register::<dyn LineMarkerProvider>(
            Registration::new("fns", LanguageId("rust")).dumb_aware(true),
            Arc::new(FnMarkers { set: functions }),
        );
    });
    let file = h.open("a.rs", "// HACK around\nfn f() { /* lint-me */ }\n");
    h.settle(Duration::from_millis(100));
    let problems = h.problems(file);
    assert!(
        problems.contains(&("native:general".into(), "hack comment".into())),
        "{problems:?}"
    );
    assert!(
        problems.contains(&("native:external".into(), "external lint says no".into())),
        "{problems:?}"
    );
    let daemon = h.svc.lock();
    let markers: Vec<_> = daemon
        .markup(file)
        .unwrap()
        .iter()
        .filter(|i| matches!(i.kind, HighlightKind::Semantic("LINE_MARKER")))
        .collect();
    assert_eq!(markers.len(), 1);
    assert_eq!(
        markers[0].group.as_ref().unwrap().0.as_ref(),
        "line-marker:function"
    );
    assert!(!markers[0].severity.is_problem());
}

#[test]
fn external_annotator_results_survive_an_edit_by_being_shifted() {
    let mut h = DaemonHarness::with(HostConfig::testing(), |registry| {
        registry.register::<dyn archipel_ide_analysis::daemon::DynExternalAnnotator>(
            Registration::any("ext"),
            Arc::new(FakeExternal),
        );
    });
    let src = "fn f() { /* lint-me */ }\n";
    let file = h.open("a.rs", src);
    h.settle(Duration::from_millis(100));
    let range = |h: &DaemonHarness| {
        h.svc
            .lock()
            .markup(file)
            .unwrap()
            .iter()
            .find(|i| i.tool.as_str() == "native:external")
            .map(|i| i.range.clone())
    };
    let before = range(&h).unwrap();
    h.edit(file, src, 0..0, "// new\n");
    h.tick();
    assert_eq!(range(&h).unwrap(), before.start + 7..before.end + 7);
}

struct TooManyFiles;

impl GlobalInspection for TooManyFiles {
    fn id(&self) -> &'static str {
        "TooManyFiles"
    }
    fn run(
        &self,
        scope: &AnalysisScope,
        g: &GlobalCx<'_>,
        out: &mut dyn ProblemSink,
    ) -> Result<(), Cancelled> {
        g.cancel.check()?;
        let files = archipel_ide_daemon::inspection::batch::files_in_scope(g.snap, scope);
        if files.len() > 2 {
            out.report(
                files[0],
                ProblemDescriptor {
                    range: 0..0,
                    message: format!("{} files", files.len()),
                    highlight: ProblemHighlightType::GenericErrorOrWarning,
                    fixes: vec![],
                    on_the_fly_only: false,
                },
            );
        }
        Ok(())
    }
}

#[test]
fn batch_runs_local_and_global_tools_over_a_scope() {
    let mut registry = archipel_ide_analysis::default_registry();
    archipel_ide_daemon::inspection::tier_a::register(&mut registry);
    registry
        .register::<dyn GlobalInspection>(Registration::any("too-many"), Arc::new(TooManyFiles));
    let vfs = Arc::new(archipel_ide_analysis::vfs::MemVfs::with_files([
        (
            "/p/src/A.java",
            "class A { void f() { try { g(); } catch (Exception e) { } } }",
        ),
        (
            "/p/src/B.java",
            "class B { void f() { int a = 1; a = a; } }",
        ),
        ("/p/data/c.json", "{\"a\":1,\"a\":2}"),
        ("/p/readme.txt", "plain text"),
    ]));
    let mut host = archipel_ide_analysis::AnalysisHost::new(HostConfig::testing(), vfs, registry);
    let mut ids = Vec::new();
    for path in [
        "/p/src/A.java",
        "/p/src/B.java",
        "/p/data/c.json",
        "/p/readme.txt",
    ] {
        ids.push(host.file_id(std::path::Path::new(path)));
    }
    let snap = host.snapshot();
    let mut problems = Vec::new();
    let cancel = CancelToken::new();
    let summary = run_batch(
        &snap,
        &AnalysisScope::Project,
        &EffectiveProfile::default(),
        &cancel,
        &mut |p| problems.push(p),
    )
    .unwrap();
    assert_eq!(summary.files, 3);
    assert_eq!(summary.skipped, 1, "the plain text file has no grammar");
    let tools: Vec<_> = problems.iter().map(|p| p.tool).collect();
    assert!(
        tools.contains(&"EmptyCatchBlock")
            && tools.contains(&"SelfAssignment")
            && tools.contains(&"DuplicateKey")
            && tools.contains(&"TooManyFiles"),
        "{tools:?}"
    );
    // Directory scope.
    let mut dir = Vec::new();
    run_batch(
        &snap,
        &AnalysisScope::Directory("/p/src".into()),
        &EffectiveProfile::default(),
        &cancel,
        &mut |p| dir.push(p),
    )
    .unwrap();
    assert!(dir.iter().all(|p| p.file == ids[0] || p.file == ids[1]));
    // A profile can turn tools off, global ones included.
    let mut store = ProfileStore::new();
    store.insert(
        "P",
        Origin::Project,
        ProfileFile::from_toml(
            "[tools.EmptyCatchBlock]\nenabled = false\n[tools.TooManyFiles]\nenabled = false\n",
        )
        .unwrap(),
    );
    let mut off = Vec::new();
    run_batch(
        &snap,
        &AnalysisScope::Project,
        &store.resolve("P").unwrap(),
        &cancel,
        &mut |p| off.push(p),
    )
    .unwrap();
    assert!(
        off.iter()
            .all(|p| p.tool != "EmptyCatchBlock" && p.tool != "TooManyFiles")
    );
    // Cancellation is honored.
    let stopped = CancelToken::new();
    stopped.cancel();
    assert!(
        run_batch(
            &snap,
            &AnalysisScope::Project,
            &EffectiveProfile::default(),
            &stopped,
            &mut |_| {}
        )
        .is_err()
    );
}

#[test]
fn batch_streams_from_a_background_job() {
    let mut registry = archipel_ide_analysis::default_registry();
    archipel_ide_daemon::inspection::tier_a::register(&mut registry);
    let vfs = Arc::new(archipel_ide_analysis::vfs::MemVfs::with_files([(
        "/p/A.java",
        "class A { void f() { try { g(); } catch (Exception e) { } } }",
    )]));
    let config = HostConfig {
        threads: 2,
        ..HostConfig::default()
    };
    let mut host = archipel_ide_analysis::AnalysisHost::new(config, vfs, registry);
    host.file_id(std::path::Path::new("/p/A.java"));
    let run = start_batch(
        &mut host,
        AnalysisScope::Project,
        Arc::new(EffectiveProfile::default()),
    );
    let mut seen = Vec::new();
    for _ in 0..500 {
        host.poll();
        seen.extend(run.drain());
        if seen.iter().any(|e| matches!(e, BatchEvent::Finished(_))) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        seen.iter()
            .any(|e| matches!(e, BatchEvent::Problem(p) if p.tool == "EmptyCatchBlock")),
        "{seen:?}"
    );
    assert!(
        seen.iter()
            .any(|e| matches!(e, BatchEvent::Finished(s) if s.files == 1))
    );
}

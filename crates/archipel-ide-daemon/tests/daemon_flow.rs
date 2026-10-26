//! End-to-end daemon behavior: passes, shifting, cancel-on-typing, ordering.

mod common;
use archipel_ide_analysis::daemon::{Pass, PassCx, PassId, PassOutput};
use archipel_ide_analysis::{
    Cancelled, HighlightInfo, HighlightKind, HostConfig, Severity, ToolId,
};
use common::daemon_harness::DaemonHarness;
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const JAVA: &str = "class A {\n  void f() {\n    try { g(); } catch (Exception e) { }\n  }\n}\n";

#[test]
fn opening_a_file_highlights_syntax_errors_then_inspections() {
    let mut h = DaemonHarness::new();
    let file = h.open(
        "A.java",
        "class A {\n  void f() {\n    try { g(); } catch (Exception e) { }\n  \n",
    );
    h.tick();
    // The syntax pass is inline: the very first frame has the error.
    let problems = h.problems(file);
    assert!(
        problems.iter().any(|(tool, _)| tool == "native:syntax"),
        "{problems:?}"
    );
    h.settle(Duration::from_millis(100));
    let problems = h.problems(file);
    assert!(
        problems
            .iter()
            .any(|(tool, m)| tool == "native:EmptyCatchBlock" && m.contains("catch")),
        "{problems:?}"
    );
    assert!(
        h.svc
            .lock()
            .problem_files()
            .summary(file)
            .is_some_and(|s| s.counts.errors >= 1)
    );
}

#[test]
fn highlights_follow_edits_and_are_replaced_after_the_debounce() {
    let mut h = DaemonHarness::new();
    let file = h.open("A.java", JAVA);
    h.settle(Duration::from_millis(100));
    let start = |h: &DaemonHarness| {
        h.svc
            .lock()
            .markup(file)
            .unwrap()
            .iter()
            .find(|i| i.tool.as_str() == "native:EmptyCatchBlock")
            .map(|i| i.range.clone())
    };
    let before = start(&h).expect("empty catch found");
    // Type a comment line above: the highlight moves, nothing recomputed yet.
    let text = h.edit(file, JAVA, 0..0, "// hi\n");
    h.tick();
    let shifted = start(&h).expect("kept while stale");
    assert_eq!(shifted, before.start + 6..before.end + 6);
    // Fix the catch body: the old highlight stays until the pass replaces it.
    let at = text.find("{ }").unwrap();
    let text2 = h.edit(file, &text, at + 2..at + 2, "log();");
    h.tick();
    assert!(
        start(&h).is_some(),
        "stale highlight kept during the debounce"
    );
    assert!(!h.svc.lock().all_idle());
    h.settle(Duration::from_millis(100));
    assert!(start(&h).is_none(), "replaced by the fresh result: {text2}");
}

#[test]
fn passes_are_debounced_by_300ms() {
    let mut h = DaemonHarness::new();
    let file = h.open("A.java", JAVA);
    h.settle(Duration::from_millis(100));
    let applied = h.svc.lock().applied_results();
    h.edit(file, JAVA, 0..0, " ");
    h.tick();
    // Only the inline syntax pass ran so far.
    let after_syntax = h.svc.lock().applied_results();
    assert_eq!(after_syntax, applied + 1);
    h.clock.advance(Duration::from_millis(299));
    h.tick();
    assert_eq!(
        h.svc.lock().applied_results(),
        after_syntax,
        "nothing before 300 ms"
    );
    h.clock.advance(Duration::from_millis(2));
    h.settle(Duration::from_millis(10));
    assert!(h.svc.lock().applied_results() > after_syntax);
}

/// Records `(pass, range, visible)` calls.
struct Recorder {
    id: PassId,
    after: &'static [PassId],
    log: Arc<Mutex<Vec<String>>>,
}

impl Pass for Recorder {
    fn id(&self) -> PassId {
        self.id
    }
    fn after(&self) -> &[PassId] {
        self.after
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn visible_range_first(&self) -> bool {
        true
    }
    fn collect(&self, cx: &PassCx<'_>) -> Result<PassOutput, Cancelled> {
        self.log.lock().push(format!(
            "{}:{}-{}:{}",
            self.id.0, cx.range.start, cx.range.end, cx.visible
        ));
        let tool = ToolId::native(self.id.0);
        let info = HighlightInfo::new(
            cx.range.start..cx.range.start,
            Severity::WARNING,
            HighlightKind::Warning,
            tool.clone(),
        )
        .message(self.id.0.to_string());
        Ok(PassOutput {
            file: cx.file,
            rev: cx.syntax.rev(),
            tool,
            range: if cx.visible {
                cx.range.clone()
            } else {
                0..u32::MAX
            },
            infos: vec![info],
        })
    }
}

#[test]
fn passes_run_visible_range_first_and_respect_prerequisites() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let l = log.clone();
    let mut h = DaemonHarness::with(HostConfig::testing(), move |registry| {
        DaemonHarness::add_pass(
            registry,
            "rec-a",
            Arc::new(Recorder {
                id: PassId("rec-a"),
                after: &[],
                log: l.clone(),
            }),
        );
        DaemonHarness::add_pass(
            registry,
            "rec-b",
            Arc::new(Recorder {
                id: PassId("rec-b"),
                after: &[PassId("rec-a")],
                log: l,
            }),
        );
    });
    let text = "x\n".repeat(50);
    let file = h.open("a.toml", &text.replace('x', "a = 1 #"));
    let len = h.host.snapshot().file_text(file).unwrap().len() as u32;
    h.svc.lock().set_visible(file, 10..30);
    h.settle(Duration::from_millis(50));
    let log = log.lock().clone();
    assert_eq!(
        log,
        [
            "rec-a:10-30:true".to_string(),
            format!("rec-a:0-{len}:false"),
            "rec-b:10-30:true".to_string(),
            format!("rec-b:0-{len}:false"),
        ],
        "visible first, rest after, b only once a is complete"
    );
}

/// Blocks until cancelled.
struct Slow {
    started: Arc<AtomicBool>,
    cancelled: Arc<AtomicUsize>,
    finished: Arc<AtomicUsize>,
}

impl Pass for Slow {
    fn id(&self) -> PassId {
        PassId("slow")
    }
    fn after(&self) -> &[PassId] {
        &[]
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn visible_range_first(&self) -> bool {
        false
    }
    fn collect(&self, cx: &PassCx<'_>) -> Result<PassOutput, Cancelled> {
        self.started.store(true, Ordering::SeqCst);
        let begin = Instant::now();
        while begin.elapsed() < Duration::from_secs(10) {
            if cx.cancel.is_cancelled() {
                self.cancelled.fetch_add(1, Ordering::SeqCst);
                return Err(Cancelled);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        self.finished.fetch_add(1, Ordering::SeqCst);
        Ok(PassOutput {
            file: cx.file,
            rev: cx.syntax.rev(),
            tool: ToolId::native("slow"),
            range: 0..u32::MAX,
            infos: vec![],
        })
    }
}

#[test]
fn typing_cancels_in_flight_passes_and_never_blocks_the_edit_thread() {
    let started = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicUsize::new(0));
    let finished = Arc::new(AtomicUsize::new(0));
    let (s, c, f) = (started.clone(), cancelled.clone(), finished.clone());
    let config = HostConfig {
        threads: 2,
        ..HostConfig::default()
    };
    let mut h = DaemonHarness::with(config, move |registry| {
        DaemonHarness::add_pass(
            registry,
            "slow",
            Arc::new(Slow {
                started: s,
                cancelled: c,
                finished: f,
            }),
        );
    });
    let file = h.open("A.java", JAVA);
    // Frames while the slow pass runs in a job: each returns immediately.
    let begin = Instant::now();
    let mut text = JAVA.to_owned();
    while !started.load(Ordering::SeqCst) {
        h.tick();
        assert!(
            begin.elapsed() < Duration::from_secs(5),
            "slow pass never started"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    let frame = Instant::now();
    for i in 0..20 {
        text = h.edit(file, &text.clone(), 0..0, "x");
        h.tick();
        h.clock.advance(Duration::from_millis(10 + i));
    }
    assert!(
        frame.elapsed() < Duration::from_millis(500),
        "edits + ticks took {:?} with a 10 s pass running",
        frame.elapsed()
    );
    // The first edit cancelled the running pass.
    let wait = Instant::now();
    while cancelled.load(Ordering::SeqCst) == 0 {
        assert!(
            wait.elapsed() < Duration::from_secs(5),
            "running pass was not cancelled"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(finished.load(Ordering::SeqCst), 0);
    assert!(h.svc.lock().pass_stats(PassId("slow")).cancelled >= 1);
}

#[test]
fn closing_a_file_drops_its_markup() {
    let mut h = DaemonHarness::new();
    let file = h.open("A.java", JAVA);
    h.settle(Duration::from_millis(100));
    assert!(h.svc.lock().markup(file).is_some_and(|m| !m.is_empty()));
    h.host.apply(archipel_ide_analysis::Change::Close { file });
    h.tick();
    assert!(h.svc.lock().markup(file).is_none());
    assert!(!h.svc.lock().is_tracking(file));
    assert!(h.svc.lock().problem_files().summary(file).is_none());
}

#[test]
fn caret_driven_identifier_highlighting() {
    let mut h = DaemonHarness::new();
    let src = "class A { int count; void f() { count = count + 1; } }\n";
    let file = h.open("A.java", src);
    h.settle(Duration::from_millis(100));
    let usages = |h: &DaemonHarness| {
        h.svc
            .lock()
            .markup(file)
            .unwrap()
            .iter()
            .filter(|i| matches!(i.kind, HighlightKind::Semantic("IDENTIFIER_UNDER_CARET")))
            .count()
    };
    assert_eq!(usages(&h), 0, "no caret yet");
    h.svc
        .lock()
        .set_caret(file, src.find("count").unwrap() as u32 + 2);
    h.settle(Duration::from_millis(30));
    assert_eq!(usages(&h), 3);
    // Caret on a keyword: no highlights.
    h.svc.lock().set_caret(file, 2);
    h.settle(Duration::from_millis(30));
    assert_eq!(usages(&h), 0);
}

#[test]
fn profile_changes_reanalyze_open_files() {
    use archipel_ide_daemon::inspection::profile::{Origin, ProfileFile, ProfileStore};
    let mut h = DaemonHarness::new();
    let file = h.open("A.java", JAVA);
    h.settle(Duration::from_millis(100));
    assert!(
        h.problems(file)
            .iter()
            .any(|(t, _)| t == "native:EmptyCatchBlock")
    );
    let mut store = ProfileStore::new();
    store.insert(
        "P",
        Origin::Project,
        ProfileFile::from_toml("[tools.EmptyCatchBlock]\nenabled = false\n").unwrap(),
    );
    h.svc.set_profile(store.resolve("P").unwrap());
    h.host.apply(archipel_ide_analysis::Change::SettingsChanged);
    h.settle(Duration::from_millis(100));
    assert!(
        !h.problems(file)
            .iter()
            .any(|(t, _)| t == "native:EmptyCatchBlock")
    );
}

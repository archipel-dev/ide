//! Daemon budgets (spec 1.11): edits never block on the daemon and typing
//! cancels in-flight passes.
//!
//! `cargo bench -p archipel-ide-daemon --bench daemon` (release). Exits non-zero
//! when a budget is missed:
//! - frame (apply edit + daemon tick) p95 < 5 ms on a 5k-line file, while a slow
//!   pass runs in a job;
//! - a running pass observes cancellation < 100 ms after the edit;
//! - full-file inspections of 5k lines < 2 s;
//! - visible-range inspections of the same file < 150 ms.

use archipel_ide_analysis::daemon::{Pass, PassCx, PassId, PassOutput};
use archipel_ide_analysis::testing::byte_edit;
use archipel_ide_analysis::vfs::MemVfs;
use archipel_ide_analysis::{
    AnalysisHost, Cancelled, Change, FileRev, HostConfig, ToolId, default_registry,
};
use archipel_ide_daemon::{DaemonService, SystemClock};
use archipel_ide_language::Registration;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

struct Slow {
    started: Arc<AtomicBool>,
    cancelled_at: Arc<AtomicU64>,
    epoch: Instant,
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
        loop {
            if cx.cancel.is_cancelled() {
                self.cancelled_at
                    .store(self.epoch.elapsed().as_micros() as u64, Ordering::SeqCst);
                return Err(Cancelled);
            }
            std::thread::sleep(Duration::from_micros(200));
        }
    }
}

fn source(lines: usize) -> String {
    let mut text = String::from("class Big {\n");
    for i in 0..lines / 5 {
        text.push_str(&format!(
            "  int m{i}(int a) {{\n    if (a > {i}) {{ return a; }} // note\n    try {{ g(); }} catch (Exception e) {{ }}\n    return a;\n  }}\n"
        ));
    }
    text.push_str("}\n");
    text
}

fn make_host(config: HostConfig, slow: Option<Slow>) -> (AnalysisHost, Arc<DaemonService>) {
    let service = DaemonService::new(Arc::new(SystemClock::new()));
    let mut registry = default_registry();
    service.register(&mut registry);
    if let Some(slow) = slow {
        registry.register::<dyn Pass>(Registration::any("slow").dumb_aware(true), Arc::new(slow));
    }
    (
        AnalysisHost::new(config, Arc::new(MemVfs::new()), registry),
        service,
    )
}

fn frame(host: &mut AnalysisHost, service: &DaemonService) {
    host.poll();
    let events = host.take_events();
    service.lock().tick(host, &events);
}

fn p95(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[(samples.len() * 95 / 100).min(samples.len() - 1)]
}

fn main() {
    let failed = std::cell::Cell::new(false);
    let check = |name: &str, value: Duration, budget: Duration| {
        let ok = value <= budget;
        println!(
            "{name}: {value:?} (budget {budget:?}) {}",
            if ok { "ok" } else { "FAILED" }
        );
        failed.set(failed.get() || !ok);
    };
    let text = source(5000);

    // 1. Edits never block while a pass is running; typing cancels it.
    let started = Arc::new(AtomicBool::new(false));
    let cancelled_at = Arc::new(AtomicU64::new(0));
    let epoch = Instant::now();
    let (mut host, service) = make_host(
        HostConfig {
            threads: 3,
            ..HostConfig::default()
        },
        Some(Slow {
            started: started.clone(),
            cancelled_at: cancelled_at.clone(),
            epoch,
        }),
    );
    let file = host.file_id(Path::new("Big.java"));
    host.apply(Change::Open {
        file,
        rev: FileRev(1),
        text: text.as_str().into(),
    });
    while !started.load(Ordering::SeqCst) {
        frame(&mut host, &service);
        std::thread::sleep(Duration::from_millis(1));
    }
    let mut current = text.clone();
    let mut samples = Vec::new();
    let mut edit_at = None;
    for i in 0..200u64 {
        let begin = Instant::now();
        let edit = byte_edit(&current, 20..20, "x");
        current.insert(20, 'x');
        host.apply(Change::Edit {
            file,
            edits: vec![edit],
            new_rev: FileRev(2 + i),
            text: current.as_str().into(),
            syntax: None,
        });
        frame(&mut host, &service);
        samples.push(begin.elapsed());
        edit_at.get_or_insert(epoch.elapsed().as_micros() as u64);
        std::thread::sleep(Duration::from_millis(2));
    }
    check(
        "frame (edit + tick), 5k lines, pass in flight, p95",
        p95(samples),
        Duration::from_millis(5),
    );
    std::thread::sleep(Duration::from_millis(50));
    let cancelled = cancelled_at.load(Ordering::SeqCst);
    let latency = Duration::from_micros(cancelled.saturating_sub(edit_at.unwrap_or(0)));
    println!("slow pass cancelled {latency:?} after the first edit");
    check("cancellation latency", latency, Duration::from_millis(100));
    if cancelled == 0 {
        println!("slow pass was never cancelled: FAILED");
        failed.set(true);
    }

    // 2. Inspection budgets on a quiet file (deterministic, inline jobs).
    let (mut host, service) = make_host(HostConfig::testing(), None);
    let file = host.file_id(Path::new("Big.java"));
    host.apply(Change::Open {
        file,
        rev: FileRev(1),
        text: text.as_str().into(),
    });
    service.lock().set_visible(file, 0..3000);
    let begin = Instant::now();
    let mut visible_done = None;
    for _ in 0..50 {
        frame(&mut host, &service);
        if visible_done.is_none()
            && service.lock().markup(file).is_some_and(|m| {
                m.iter()
                    .any(|i| i.tool.as_str() == "native:local-inspections")
            })
        {
            visible_done = Some(begin.elapsed());
        }
        if service.lock().all_idle() {
            break;
        }
    }
    let total = begin.elapsed();
    check(
        "full-file inspections (5k lines)",
        total,
        Duration::from_secs(2),
    );
    check(
        "visible-range inspections",
        visible_done.unwrap_or(total),
        Duration::from_millis(150),
    );
    let _ = ToolId::native("x");
    if failed.get() {
        std::process::exit(1);
    }
}

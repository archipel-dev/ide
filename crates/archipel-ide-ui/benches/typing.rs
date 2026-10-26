//! Typing latency on a 50k-line file: full workbench frames (input handling,
//! editing, analysis refresh, layout of the visible lines) on a headless egui
//! context. Target from the editor P0 workstream: < 16 ms per keystroke frame.
//!
//! `cargo bench -p archipel-ide-ui --bench typing` (release). Exits non-zero when the
//! 95th percentile exceeds the budget.

use eframe::egui;
use std::time::{Duration, Instant};

const LINES: usize = 50_000;
const KEYSTROKES: usize = 200;
const BUDGET: Duration = Duration::from_millis(16);

fn sample_source() -> String {
    let mut text = String::from("package demo;\n\npublic class Big {\n");
    let mut line = 3;
    let mut method = 0;
    while line < LINES - 1 {
        text.push_str(&format!(
            "    int method{method}(int value) {{\n        int result = value * {method} + 1; // step\n        return result;\n    }}\n"
        ));
        method += 1;
        line += 4;
    }
    text.push_str("}\n");
    text
}

fn input(events: Vec<egui::Event>, time: f64) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1440.0, 900.0),
        )),
        time: Some(time),
        focused: true,
        events,
        ..Default::default()
    }
}

/// Typing latency samples for one file, sorted ascending.
fn run(file: &str) -> Vec<Duration> {
    let root = std::env::temp_dir().join(format!(
        "archipel-ide-typing-bench-{}-{file}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp project");
    std::fs::write(root.join(file), sample_source()).expect("sample file");
    let mut app = archipel_ide_ui::IdeApp::new(Some(root.clone()), vec![root.join(file)]);
    let ctx = egui::Context::default();
    archipel_ide_ui::install_theme(&ctx);
    let mut time = 0.0;
    for _ in 0..5 {
        app.focus_editor(&ctx);
        let _ = ctx.run(input(Vec::new(), time), |ctx| app.frame(ctx));
        time += 0.016;
    }
    // Type below the first screen so edits shift most of the file.
    let mut events = vec![egui::Event::Key {
        key: egui::Key::PageDown,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }];
    let mut samples = Vec::with_capacity(KEYSTROKES);
    for index in 0..KEYSTROKES {
        events.push(egui::Event::Text(
            if index % 10 == 9 { " " } else { "x" }.into(),
        ));
        let start = Instant::now();
        let _ = ctx.run(input(std::mem::take(&mut events), time), |ctx| {
            app.frame(ctx)
        });
        samples.push(start.elapsed());
        time += 0.016;
    }
    let typed = app
        .state()
        .documents
        .first()
        .map(|document| document.text.text().matches("xxxxxxxxx ").count())
        .unwrap_or(0);
    assert_eq!(typed, KEYSTROKES / 10, "keystrokes must reach the editor");
    let _ = std::fs::remove_dir_all(&root);
    samples.sort();
    samples
}

fn main() {
    // SAFETY: single-threaded before any app starts a worker.
    unsafe { std::env::set_var("ARCHIPEL_NO_UPDATE_CHECK", "1") };
    let mut over_budget = false;
    // Plain text isolates the editor path; Java adds tree-sitter highlighting.
    for file in ["Big.txt", "Big.java"] {
        let samples = run(file);
        let percentile = |p: f64| samples[((samples.len() - 1) as f64 * p) as usize];
        let (p50, p95, max) = (
            percentile(0.5),
            percentile(0.95),
            samples[samples.len() - 1],
        );
        println!(
            "{file}: {LINES} lines, {KEYSTROKES} keystrokes: p50 {:.2} ms, p95 {:.2} ms, max {:.2} ms (budget {} ms)",
            p50.as_secs_f64() * 1e3,
            p95.as_secs_f64() * 1e3,
            max.as_secs_f64() * 1e3,
            BUDGET.as_millis()
        );
        over_budget |= p95 > BUDGET;
    }
    if over_budget && !cfg!(debug_assertions) {
        eprintln!("p95 exceeds the typing budget");
        std::process::exit(1);
    }
}

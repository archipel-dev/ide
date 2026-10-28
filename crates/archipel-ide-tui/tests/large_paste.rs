//! QA bug 15 of the TUI list: a bracketed paste of 20 000 lines and its undo take about a second, not minutes.

mod common;

use archipel_kit_tui::Event;
use common::{Project, Tui};
use std::time::{Duration, Instant};

fn budget() -> Duration {
    if cfg!(debug_assertions) {
        Duration::from_secs(15)
    } else {
        Duration::from_secs(2)
    }
}

#[test]
fn pasting_and_undoing_20k_lines_in_a_java_file_stays_in_budget() {
    let project = Project::new(
        "big-paste",
        &[("App.java", "class App {\n    void run() {}\n}\n")],
    );
    let mut tui = Tui::open(&project.root, &["App.java"], 120, 30);
    tui.advance(0.1);
    let text: String = (0..20_000).map(|k| format!("pasted line {k}\n")).collect();
    let start = Instant::now();
    tui.send(Event::Paste(text));
    tui.advance(0.1);
    let paste = start.elapsed();
    assert!(paste < budget(), "paste took {paste:?}");
    assert!(
        tui.app.ide().state().documents()[0].text.len_lines() > 20_000,
        "the text arrived"
    );
    let start = Instant::now();
    tui.ctrl('z');
    tui.advance(0.1);
    let undo = start.elapsed();
    assert!(undo < budget(), "undo took {undo:?}");
    assert_eq!(tui.app.ide().state().documents()[0].text.len_lines(), 4);
}

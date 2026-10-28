//! TUI-BUG-015: pasting and undoing a large text takes time linear in its size, not quadratic.

use archipel_ide_core::{IdeState, built_in_commands};
use serde_json::json;
use std::path::Path;
use std::time::{Duration, Instant};

fn pasted(lines: usize) -> String {
    (0..lines).map(|k| format!("pasted line {k}\n")).collect()
}

fn state(language_file: &str) -> (std::path::PathBuf, IdeState) {
    let root = std::env::temp_dir().join(format!(
        "archipel-large-paste-{}-{}",
        language_file.replace('.', "_"),
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join(language_file), "class App {\n}\n").unwrap();
    let mut state = IdeState::default();
    state.open_project(&root).unwrap();
    state.open_file(Path::new(language_file)).unwrap();
    (root, state)
}

/// Seconds a paste and its undo take for `lines` lines in a file named `file`.
fn timed(file: &str, lines: usize) -> (Duration, Duration) {
    let (root, mut state) = state(file);
    let commands = built_in_commands();
    let text = pasted(lines);
    let start = Instant::now();
    commands
        .execute("editor.paste", &mut state, json!({ "text": text }))
        .unwrap();
    let paste = start.elapsed();
    assert!(state.documents[0].text.len_lines() > lines);
    let start = Instant::now();
    commands
        .execute("editor.undo", &mut state, json!({}))
        .unwrap();
    let undo = start.elapsed();
    assert_eq!(state.documents[0].text.text(), "class App {\n}\n");
    let _ = std::fs::remove_dir_all(root);
    (paste, undo)
}

/// The budget of 20 000 lines: one second in release, ten in debug builds.
fn budget() -> Duration {
    if cfg!(debug_assertions) {
        Duration::from_secs(10)
    } else {
        Duration::from_secs(1)
    }
}

#[test]
fn pasting_and_undoing_20k_lines_stays_in_budget() {
    for file in ["notes.txt", "App.java", "main.rs"] {
        let (paste, undo) = timed(file, 20_000);
        assert!(paste < budget(), "{file}: paste took {paste:?}");
        assert!(undo < budget(), "{file}: undo took {undo:?}");
    }
}

#[test]
fn paste_time_grows_linearly() {
    let (small, _) = timed("main.rs", 2_000);
    let (large, _) = timed("main.rs", 8_000);
    // 4x the text may take about 4x the time; a quadratic step would take 16x.
    assert!(
        large < small * 10 + Duration::from_millis(50),
        "2k lines {small:?}, 8k lines {large:?}"
    );
}

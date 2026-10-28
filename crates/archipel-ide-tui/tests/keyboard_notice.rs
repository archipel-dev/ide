//! TUI-GUI-155, 159, 45: a terminal without the kitty keyboard protocol sees the keyboard
//! notice once; Help > Terminal Keyboard Setup… shows it again; a kitty terminal is not told.
//! Spec: TUI-GUI-045, TUI-GUI-155, TUI-GUI-159 (tests).

mod common;

use archipel_ide_tui::{StartOptions, TuiApp};
use archipel_kit_tui::input::Caps;
use common::{Project, Tui};

fn start(project: &Project, marker: &std::path::Path, caps: Caps) -> Tui {
    let mut app = TuiApp::new(StartOptions {
        project: Some(project.root.clone()),
        files: vec!["a.txt".into()],
        isolated: true,
        ..StartOptions::default()
    });
    app.keyboard_notice_marker(marker.to_path_buf());
    let mut tui = Tui::with_caps(app, 120, 30, caps);
    tui.advance(0.3);
    tui
}

#[test]
fn a_legacy_terminal_is_told_once() {
    let project = Project::new("notice-legacy", &[("a.txt", "x\n")]);
    let marker = project.root.join("notice-marker");
    let mut first = start(&project, &marker, Caps::legacy());
    assert!(first.contains("Terminal keyboard"), "{}", first.text());
    assert!(first.contains("Ctrl+\\"), "the notice names the leader");
    first.press(archipel_kit_tui::input::KeyCode::Enter);
    first.advance(0.2);
    assert!(!first.contains("Terminal keyboard"));
    // The next start does not repeat it.
    let second = start(&project, &marker, Caps::legacy());
    assert!(!second.contains("Terminal keyboard"), "{}", second.text());
}

#[test]
fn a_kitty_terminal_is_not_told() {
    let project = Project::new("notice-kitty", &[("a.txt", "x\n")]);
    let marker = project.root.join("notice-marker");
    let tui = start(&project, &marker, Caps::kitty());
    assert!(!tui.contains("Terminal keyboard"), "{}", tui.text());
    assert!(!marker.exists());
}

#[test]
fn the_help_menu_shows_the_notice_again() {
    let project = Project::new("notice-help", &[("a.txt", "x\n")]);
    let marker = project.root.join("notice-marker");
    let mut tui = start(&project, &marker, Caps::kitty());
    tui.app.menu_control().expand();
    tui.draw();
    let menu = tui.app.built_menu().expect("a frame").clone();
    let help = menu
        .headers
        .iter()
        .position(|h| h.id == "HelpMenu")
        .expect("a Help menu");
    tui.app.menu_control().open_menu(&tui.rt, &menu, help, true);
    tui.advance(0.1);
    let (x, y) = tui
        .find("Terminal Keyboard Setup")
        .expect("the terminal's own entry");
    tui.click(x + 2, y);
    tui.advance(0.2);
    assert!(tui.contains("Terminal keyboard"), "{}", tui.text());
    assert!(tui.contains("reports every key"), "{}", tui.text());
}

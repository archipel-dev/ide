//! Closing a tab or quitting never loses edits silently (QA bugs 14 and 5 of the TUI list), and a
//! disabled action does nothing instead of raising a toast with an internal id
//! (QA bugs 3 and 4 of the TUI list).

mod common;

use archipel_ide_actions::ActionPlace;
use archipel_kit_tui::input::{KeyCode, KeyModifiers};
use common::{Project, Tui};

fn project(name: &str) -> Project {
    Project::new(name, &[("a.txt", "alpha\n"), ("b.txt", "beta\n")])
}

/// Makes saving `path` fail: a directory takes the file's place.
fn make_unsaveable(project: &Project, path: &str) {
    let target = project.root.join(path);
    std::fs::remove_file(&target).unwrap();
    std::fs::create_dir(&target).unwrap();
}

fn close_tab(tui: &mut Tui) {
    tui.press_with(KeyCode::F(4), KeyModifiers::CONTROL);
    tui.advance(0.1);
}

#[test]
fn closing_a_tab_with_edits_saves_them_first_like_the_gui() {
    let project = project("close-saves");
    let mut tui = Tui::open(&project.root, &["a.txt"], 120, 30);
    tui.advance(0.1);
    tui.type_text("edited ");
    assert_eq!(project.read("a.txt"), "alpha\n");
    close_tab(&mut tui);
    assert_eq!(project.read("a.txt"), "edited alpha\n");
    assert!(tui.app.ide().state().documents().is_empty());
    assert!(!tui.contains("Unsaved changes"), "{}", tui.text());
}

#[test]
fn the_close_button_of_a_tab_saves_before_it_closes() {
    let project = project("close-button");
    let mut tui = Tui::open(&project.root, &["a.txt", "b.txt"], 120, 30);
    tui.advance(0.1);
    tui.type_text("edited ");
    let (x, y) = tui.find("b.txt").expect("the tab");
    // The close button sits at the end of the tab; a hover shows it.
    tui.send(archipel_kit_tui::Event::Mouse(
        archipel_kit_tui::input::MouseEvent {
            kind: archipel_kit_tui::input::MouseEventKind::Moved,
            column: x + 1,
            row: y,
            modifiers: KeyModifiers::NONE,
        },
    ));
    tui.advance(0.3);
    let (cx, cy) = tui.find("\u{ea76}").expect("a close button");
    tui.click(cx, cy);
    tui.advance(0.1);
    assert_eq!(project.read("b.txt"), "edited beta\n");
    assert_eq!(tui.app.ide().state().documents().len(), 1);
}

#[test]
fn a_tab_that_cannot_be_saved_asks_before_it_is_closed() {
    let project = project("close-asks");
    let mut tui = Tui::open(&project.root, &["a.txt"], 120, 30);
    tui.advance(0.1);
    tui.type_text("precious ");
    make_unsaveable(&project, "a.txt");
    close_tab(&mut tui);
    assert!(tui.contains("Unsaved changes"), "{}", tui.text());
    assert_eq!(tui.app.ide().state().documents().len(), 1, "still open");
    // Cancel keeps the tab and the edit.
    tui.press(KeyCode::Esc);
    tui.advance(0.1);
    assert!(!tui.contains("Unsaved changes"));
    assert_eq!(tui.app.ide().state().documents().len(), 1);
    assert!(tui.app.ide().state().documents()[0].text.is_dirty());
    // Discard closes it.
    close_tab(&mut tui);
    assert!(tui.contains("Unsaved changes"), "{}", tui.text());
    tui.press(KeyCode::Enter);
    tui.advance(0.1);
    assert!(tui.app.ide().state().documents().is_empty());
}

#[test]
fn exit_with_a_document_that_cannot_be_saved_asks_instead_of_quitting() {
    let project = project("quit-asks");
    let mut tui = Tui::open(&project.root, &["a.txt"], 120, 30);
    tui.advance(0.1);
    tui.type_text("precious ");
    make_unsaveable(&project, "a.txt");
    tui.app
        .run_action(&mut tui.rt, "Exit", ActionPlace::MainMenu);
    tui.draw();
    assert!(!tui.rt.quit, "the IDE must not quit and lose the edit");
    assert!(tui.contains("Unsaved changes"), "{}", tui.text());
    tui.press(KeyCode::Esc);
    tui.advance(0.1);
    assert!(!tui.rt.quit);
    tui.app
        .run_action(&mut tui.rt, "Exit", ActionPlace::MainMenu);
    tui.draw();
    tui.press(KeyCode::Enter);
    assert!(tui.rt.quit, "Discard and Quit quits");
}

#[test]
fn exit_saves_what_can_be_saved_and_quits_without_asking() {
    let project = project("quit-saves");
    let mut tui = Tui::open(&project.root, &["a.txt"], 120, 30);
    tui.advance(0.1);
    tui.type_text("edited ");
    tui.app
        .run_action(&mut tui.rt, "Exit", ActionPlace::MainMenu);
    assert!(tui.rt.quit);
    assert_eq!(project.read("a.txt"), "edited alpha\n");
}

#[test]
fn a_disabled_action_does_nothing_and_shows_no_toast() {
    let project = project("greyed");
    let mut tui = Tui::open(&project.root, &[], 120, 30);
    tui.advance(0.1);
    // No tab is open: Close, Save and Undo are disabled.
    close_tab(&mut tui);
    tui.ctrl('z');
    tui.ctrl('s');
    tui.advance(0.2);
    let screen = tui.text();
    for forbidden in ["disabled", "editor.", "is not available"] {
        assert!(
            !screen.contains(forbidden),
            "{forbidden}\n{}",
            screen
                .lines()
                .filter(|l| l.contains(forbidden))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    assert!(!tui.contains("Command failed"));
}

//! Spec: TUI-GUI-037 (opening the main menu by keys), 040 (keys in an open menu).
//!
//! F10 expands the bar and opens File, Alt+mnemonic opens that menu, `Ctrl+\ M` does the same
//! where Alt does not arrive; Esc closes the menu and a second Esc collapses the bar again;
//! Left and Right move between the titles.

mod common;

use archipel_kit_tui::input::{KeyCode, KeyModifiers};
use common::{Project, Tui};

fn open(name: &str) -> (Project, Tui) {
    let project = Project::new(name, &[("a.txt", "x\n")]);
    let mut tui = Tui::open(&project.root, &["a.txt"], 140, 40);
    tui.advance(0.3);
    (project, tui)
}

fn open_header(tui: &mut Tui) -> Option<usize> {
    tui.app.menu_control().open_header()
}

#[test]
fn f10_opens_file_and_escape_collapses_in_two_steps() {
    let (_p, mut tui) = open("menu-f10");
    assert!(!tui.app.menu_control().is_expanded());
    tui.press(KeyCode::F(10));
    tui.advance(0.1);
    assert!(tui.app.menu_control().is_expanded());
    assert_eq!(open_header(&mut tui), Some(0), "{}", tui.text());
    assert!(tui.contains("Recent Projects"), "{}", tui.text());
    tui.press(KeyCode::Esc);
    tui.advance(0.1);
    assert_eq!(open_header(&mut tui), None);
    assert!(
        tui.app.menu_control().is_expanded(),
        "the titles stay after the first Esc"
    );
    tui.press(KeyCode::Esc);
    tui.advance(0.1);
    assert!(
        !tui.app.menu_control().is_expanded(),
        "the second Esc collapses the bar"
    );
}

#[test]
fn alt_and_the_mnemonic_open_that_menu() {
    let (_p, mut tui) = open("menu-alt");
    tui.press_with(KeyCode::Char('e'), KeyModifiers::ALT);
    tui.advance(0.1);
    assert_eq!(
        open_header(&mut tui),
        Some(1),
        "Edit is the second title\n{}",
        tui.text()
    );
    tui.press(KeyCode::Right);
    tui.advance(0.1);
    assert_eq!(open_header(&mut tui), Some(2), "Right moves to View");
    tui.press(KeyCode::Left);
    tui.advance(0.1);
    assert_eq!(open_header(&mut tui), Some(1));
}

#[test]
fn an_alt_letter_without_a_menu_leaves_the_bar_alone() {
    let (_p, mut tui) = open("menu-alt-none");
    tui.press_with(KeyCode::Char('z'), KeyModifiers::ALT);
    tui.advance(0.1);
    assert!(!tui.app.menu_control().is_expanded());
}

#[test]
fn the_leader_and_m_open_the_menu() {
    let (_p, mut tui) = open("menu-chord");
    tui.ctrl('\\');
    tui.type_text("m");
    tui.advance(0.1);
    assert_eq!(open_header(&mut tui), Some(0), "{}", tui.text());
}

/// TUI-GUI-036: the mnemonic letter of each title is underlined.
#[test]
fn the_mnemonic_letters_are_underlined() {
    use archipel_kit_tui::ratatui::style::Modifier;
    let (_p, mut tui) = open("menu-underline");
    tui.press(KeyCode::F(10));
    tui.advance(0.1);
    tui.press(KeyCode::Esc);
    tui.advance(0.1);
    let (x, y) = tui.find("Run").expect("the Run title");
    // "Run": the mnemonic is the second letter.
    assert!(tui.cell(x + 1, y).modifier.contains(Modifier::UNDERLINED));
    assert!(!tui.cell(x, y).modifier.contains(Modifier::UNDERLINED));
    let (x, y) = tui.find("File").expect("the File title");
    assert!(tui.cell(x, y).modifier.contains(Modifier::UNDERLINED));
}

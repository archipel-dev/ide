//! TUI-GUI-043: the context menus of the project tree, the editor and the tabs are the
//! GUI's (`archipel-ide-view::context_menu`), opened by a right click.
//! Spec: TUI-GUI-043 (tests).

mod common;

use common::{Project, Tui};

fn open(name: &str) -> (Project, Tui) {
    let project = Project::new(
        name,
        &[
            ("src/Main.java", "class Main {}\n"),
            ("notes.txt", "remember\n"),
        ],
    );
    let mut tui = Tui::open(&project.root, &["src/Main.java"], 140, 36);
    tui.advance(0.3);
    (project, tui)
}

#[test]
fn right_click_on_a_tree_node_opens_its_menu() {
    let (_project, mut tui) = open("context-tree");
    let (x, y) = tui.find("notes.txt").expect("a tree row");
    tui.right_click(x + 2, y);
    tui.advance(0.1);
    let screen = tui.text();
    assert!(screen.contains("Delete"), "{screen}");
    assert!(screen.contains("Copy"), "{screen}");
    // Escape closes it.
    tui.press(archipel_kit_tui::input::KeyCode::Esc);
    tui.advance(0.1);
    assert!(!tui.contains("Delete"), "{}", tui.text());
}

#[test]
fn right_click_in_the_editor_opens_the_editor_menu() {
    let (_project, mut tui) = open("context-editor");
    let (x, y) = tui.find("class Main").expect("the editor text");
    tui.right_click(x + 3, y);
    tui.advance(0.1);
    let screen = tui.text();
    assert!(screen.contains("Paste"), "{screen}");
    assert!(screen.contains("Cut"), "{screen}");
}

#[test]
fn right_click_on_a_tab_opens_the_tab_menu() {
    let (_project, mut tui) = open("context-tab");
    let (x, y) = tui.find("Main.java").expect("a tab");
    // The first match is the tree row; the tab is on row 1.
    let (tx, ty) = if y > 1 {
        let screen = tui.screen();
        let col = screen[1].find("Main.java").expect("the tab on row 1");
        (col as u16, 1)
    } else {
        (x, y)
    };
    tui.right_click(tx + 2, ty);
    tui.advance(0.1);
    assert!(tui.contains("Close"), "{}", tui.text());
}

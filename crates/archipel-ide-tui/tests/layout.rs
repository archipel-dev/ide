//! Spec: TUI-GUI-004, TUI-GUI-010, TUI-GUI-106, TUI-GUI-200 (tests).
mod common;

use common::{Project, Tui};

fn fixture() -> Project {
    Project::new(
        "layout",
        &[
            ("README.md", "# Fixture\n\nA small project.\n"),
            (
                "src/Main.java",
                "package demo;\n\npublic class Main {\n    public static void main(String[] args) {\n        System.out.println(\"hi\");\n    }\n}\n",
            ),
            (
                "src/util/Strings.java",
                "package demo.util;\n\npublic class Strings {}\n",
            ),
        ],
    )
}

#[test]
fn the_layout_renders_tree_tabs_editor_and_status_line() {
    let project = fixture();
    let mut tui = Tui::open(&project.root, &["src/Main.java"], 120, 30);
    tui.advance(0.1);
    let screen = tui.text();
    println!("{screen}");
    assert!(screen.contains("Main.java"), "tab line\n{screen}");
    assert!(
        screen.contains("public class Main"),
        "editor text\n{screen}"
    );
    assert!(screen.contains("src"), "project tree\n{screen}");
    assert!(screen.contains("1:1"), "status line\n{screen}");
}

/// TUI-GUI-004, 106 and 200: no modal editing and none of the old status segments.
#[test]
fn the_old_chrome_is_gone() {
    let project = fixture();
    let mut tui = Tui::open(&project.root, &["src/Main.java"], 160, 45);
    tui.advance(0.2);
    let screen = tui.text();
    for old in ["INSERT", "NORMAL", "VISUAL", "Ln ", "Col ", "\u{f0a2d}"] {
        assert!(!screen.contains(old), "{old:?} is drawn\n{screen}");
    }
    // The toolbar is row 0 and the status bar the last row.
    let rows = tui.screen();
    assert!(rows[0].contains("Code"), "{}", rows[0]);
    assert!(rows[44].contains("UTF-8"), "{}", rows[44]);
}

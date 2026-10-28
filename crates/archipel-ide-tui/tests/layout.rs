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

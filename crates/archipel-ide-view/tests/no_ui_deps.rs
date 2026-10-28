//! The view crate must not depend on a UI toolkit (TUI-CMP-002): `cargo tree -p
//! archipel-ide-view` lists neither egui nor ratatui.

#[test]
fn the_dependency_tree_has_no_toolkit() {
    let output = std::process::Command::new(env!("CARGO"))
        .args(["tree", "-p", "archipel-ide-view", "--edges", "normal", "--prefix", "none", "--manifest-path"])
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .output()
        .expect("cargo tree runs");
    let tree = String::from_utf8_lossy(&output.stdout);
    assert!(tree.contains("archipel-ide-view"), "{tree}\n{}", String::from_utf8_lossy(&output.stderr));
    for banned in ["egui ", "eframe ", "ratatui ", "crossterm ", "archipel-kit ", "archipel-kit-tui "] {
        assert!(
            !tree.lines().any(|line| line.starts_with(banned)),
            "{banned}is in the dependency tree of archipel-ide-view:\n{tree}"
        );
    }
}

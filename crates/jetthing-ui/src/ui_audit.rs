//! UX audit of migrated areas with `archipel_kit::testing::audit`: every target named and at
//! least 16×16, no partly overlapping targets, everything reachable with Tab, in both
//! themes. Add a test per area as it moves to the kit (see `docs/ui-migration.md`).

use crate::IdeApp;
use archipel_kit::Theme;
use archipel_kit::testing::{self, Queryable};
use eframe::egui;
use std::path::PathBuf;

const JAVA: &str = "package com.example;\n\npublic class Main {\n    private int counter;\n\n    public void increment() {\n        counter++;\n    }\n\n    interface Listener {\n        void onChange(int value);\n    }\n}\n";

/// A project with sources, tests and docs, Main.java open.
fn app_with_project(name: &str) -> IdeApp {
    let root: PathBuf =
        std::env::temp_dir().join(format!("jetthing-ui-audit-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/main")).unwrap();
    std::fs::create_dir_all(root.join("src/test")).unwrap();
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("src/main/Main.java"), JAVA).unwrap();
    std::fs::write(root.join("src/main/Util.java"), "class Util {}\n").unwrap();
    std::fs::write(root.join("README.md"), "# Demo\n").unwrap();
    std::fs::write(root.join("build.gradle.kts"), "plugins {}\n").unwrap();
    let root = root.canonicalize().unwrap();
    IdeApp::new(Some(root.clone()), vec![root.join("src/main/Main.java")])
}

fn themes() -> [(&'static str, Theme); 2] {
    [("dark", Theme::dark()), ("light", Theme::light())]
}

/// Runs `show` in a 360×420 panel.
fn panel(ui: &mut egui::Ui, show: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui(egui::vec2(360.0, 420.0), |ui| {
        ui.set_min_size(egui::vec2(360.0, 420.0));
        show(ui);
    });
}

#[test]
fn structure_tool_window_passes_the_audit() {
    for (name, theme) in themes() {
        let app = app_with_project(&format!("structure-{name}"));
        let mut harness = testing::harness_state(theme, app, |ui, app| {
            panel(ui, |ui| crate::structure_view::tool_window(app, ui));
        });
        harness.run();
        // Rows are named by their label (and detail), as a screen reader would read them.
        harness.get_by_label_contains("increment");
        harness.get_by_label("Listener");
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn project_tree_passes_the_audit() {
    for (name, theme) in themes() {
        let app = app_with_project(&format!("project-{name}"));
        let mut harness = testing::harness_state(theme, app, |ui, app| {
            panel(ui, |ui| crate::toolwindows::project_tree_for_tests(app, ui));
        });
        harness.run();
        // The open file's folders are revealed; the chevrons fold and unfold them.
        harness.get_by_label("Main.java");
        harness.get_by_label("README.md");
        harness.get_by_label("Collapse main").click();
        harness.run();
        assert!(harness.query_by_label("Main.java").is_none());
        harness.get_by_label("Expand main").click();
        harness.run();
        harness.get_by_label("Main.java");
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn structure_rows_have_a_context_menu() {
    let app = app_with_project("structure-menu");
    let mut harness = testing::harness_state(Theme::dark(), app, |ui, app| {
        panel(ui, |ui| crate::structure_view::tool_window(app, ui));
    });
    harness.run();
    harness.get_by_label_contains("increment").click_secondary();
    harness.run();
    harness.get_by_label("Navigate to Source");
    harness.get_by_label("Copy Name");
}

#[test]
fn project_rows_have_a_context_menu_with_the_model_actions() {
    let app = app_with_project("project-menu");
    let mut harness = testing::harness_state(Theme::dark(), app, |ui, app| {
        panel(ui, |ui| crate::toolwindows::project_tree_for_tests(app, ui));
    });
    harness.run();
    harness.get_by_label("README.md").click_secondary();
    harness.run();
    harness.get_by_label("Cut");
    harness.get_by_label("Copy");
}

#[test]
fn dropping_a_file_on_a_folder_moves_it() {
    let mut app = app_with_project("drop");
    let root = app.state().project.as_ref().unwrap().root().to_owned();
    crate::toolwindows::move_entry_for_tests(
        &mut app,
        std::path::Path::new("src/main/Util.java"),
        std::path::Path::new("src/test"),
    );
    assert!(!root.join("src/main/Util.java").exists());
    assert!(root.join("src/test/Util.java").exists());
    // Dropping a folder into itself or on its own parent changes nothing.
    crate::toolwindows::move_entry_for_tests(
        &mut app,
        std::path::Path::new("src/main"),
        std::path::Path::new("src/main/inner"),
    );
    crate::toolwindows::move_entry_for_tests(
        &mut app,
        std::path::Path::new("src/main/Main.java"),
        std::path::Path::new("src/main"),
    );
    assert!(root.join("src/main/Main.java").exists());
}

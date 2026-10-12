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

/// The window chrome as the workbench draws it: glow, main toolbar, status bar and both
/// stripes around an empty center, in a 1280×720 window.
fn chrome_harness(theme: Theme, app: IdeApp) -> testing::Harness<'static, IdeApp> {
    let mut installed = false;
    let mut harness = testing::builder::<IdeApp>()
        .with_size(egui::vec2(1280.0, 720.0))
        .build_state(
            move |ctx, app| {
                if !installed {
                    archipel_kit::install(ctx, theme.clone());
                    installed = true;
                    ctx.request_repaint();
                    return;
                }
                crate::chrome::project_glow(app, ctx);
                crate::chrome::main_toolbar(app, ctx);
                crate::chrome::status_bar(app, ctx);
                crate::chrome::left_stripe(app, ctx);
                crate::plugin_ui::right_side(app, ctx);
                egui::CentralPanel::default()
                    .frame(egui::Frame::new())
                    .show(ctx, |_| {});
            },
            app,
        );
    harness.run_ok();
    harness
}

#[test]
fn window_chrome_passes_the_audit() {
    for (name, theme) in themes() {
        let app = app_with_project(&format!("chrome-{name}"));
        let mut harness = chrome_harness(theme, app);
        harness.run_ok();
        // Toolbar widgets, stripe buttons and status bar items are named controls.
        // The project widget and the project crumb of the navigation path.
        assert!(
            harness
                .query_all_by_label_contains("jetthing-ui-audit-chrome")
                .count()
                >= 2
        );
        harness.get_by_label("Current File");
        harness.get_by_label("IDE and Project Settings");
        harness.get_by_label("Project");
        harness.get_by_label("Terminal");
        harness.get_by_label("Main.java");
        harness.get_by_label("UTF-8");
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn stripe_buttons_open_and_close_their_tool_windows() {
    let app = app_with_project("chrome-stripe");
    let mut harness = chrome_harness(Theme::dark(), app);
    assert!(
        !harness
            .state()
            .state()
            .tool_windows
            .is_visible(jetthing_core::ToolWindow::Terminal)
    );
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Terminal")
        .click();
    harness.run_ok();
    assert!(
        harness
            .state()
            .state()
            .tool_windows
            .is_visible(jetthing_core::ToolWindow::Terminal)
    );
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Terminal")
        .click();
    harness.run_ok();
    assert!(
        !harness
            .state()
            .state()
            .tool_windows
            .is_visible(jetthing_core::ToolWindow::Terminal)
    );
}

#[test]
fn toolbar_widgets_open_their_menus_on_click() {
    let app = app_with_project("chrome-menus");
    let mut harness = chrome_harness(Theme::dark(), app);
    // The project widget comes first (the toolbar), then the project crumb in the status bar.
    harness
        .query_all_by_label_contains("jetthing-ui-audit-chrome-menus")
        .next()
        .unwrap()
        .click();
    harness.run_ok();
    harness.get_by_label("Open Project…");
    harness.get_by_label("New Project…");
    harness.key_press(egui::Key::Escape);
    harness.run_ok();
    assert!(harness.query_by_label("Open Project…").is_none());
    harness.get_by_label("Current File").click();
    harness.run_ok();
    harness.get_by_label("Edit Configurations…");
    harness.key_press(egui::Key::Escape);
    harness.run_ok();
    harness.get_by_label("IDE and Project Settings").click();
    harness.run_ok();
    harness.get_by_label("Plugins…");
}

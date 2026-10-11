//! UX audit of migrated areas with `archipel_kit::testing::audit`: every target named and at
//! least 16×16, no partly overlapping targets, everything reachable with Tab, in both
//! themes. Add a test per area as it moves to the kit (see `docs/ui-migration.md`).

use crate::IdeApp;
use crate::popups::Dialog;
use archipel_kit::components::{Button, Checkbox, Dropdown, IconButton, TextField};
use archipel_kit::testing::{self, NodeT, Queryable};
use archipel_kit::{Glyph, Theme};
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

// ---- Dialogs (cluster C1) -------------------------------------------------------------

/// A project that is also a Git repository with one commit.
fn app_with_git(name: &str) -> IdeApp {
    let app = app_with_project(name);
    let root = app.state().project.as_ref().unwrap().root().to_owned();
    drop(app);
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .args([
                "-c",
                "user.name=Dev",
                "-c",
                "user.email=dev@example.invalid",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["add", "."]);
    git(&["commit", "-qm", "first"]);
    IdeApp::new(Some(root.clone()), vec![root.join("src/main/Main.java")])
}

/// A 1000×900 window showing the IDE's dialogs (the dialog is a modal layer, so the
/// harness is not shrunk to fit).
fn dialog_harness<'a>(
    theme: Theme,
    app: IdeApp,
    show: impl Fn(&mut IdeApp, &egui::Context) + 'a,
) -> testing::Harness<'a, IdeApp> {
    let mut pending = Some(theme);
    let mut harness = testing::builder::<IdeApp>()
        .with_size(egui::vec2(1000.0, 900.0))
        .build_ui_state(
            move |ui, app| {
                if let Some(theme) = pending.take() {
                    archipel_kit::install(ui.ctx(), theme);
                    ui.ctx().request_repaint();
                    return;
                }
                show(app, ui.ctx());
            },
            app,
        );
    harness.run_ok();
    harness
}

fn popups_harness<'a>(
    theme: Theme,
    mut app: IdeApp,
    dialog: Dialog,
) -> testing::Harness<'a, IdeApp> {
    app.open_dialog(dialog);
    dialog_harness(theme, app, crate::popups::show)
}

#[test]
fn new_project_dialog_passes_the_audit() {
    for (name, theme) in themes() {
        let app = app_with_project(&format!("new-project-{name}"));
        let form = crate::new_project::Form::new(Vec::new());
        let mut harness = popups_harness(theme, app, Dialog::NewProject(Box::new(form)));
        harness.get_by_label("Create");
        harness.get_by_label("Name");
        harness.get_by_label("Location");
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn new_project_generator_list_is_keyboard_navigable() {
    let app = app_with_project("new-project-keys");
    let form = crate::new_project::Form::new(Vec::new());
    let mut harness = popups_harness(Theme::dark(), app, Dialog::NewProject(Box::new(form)));
    let label = |harness: &testing::Harness<'_, IdeApp>| match harness.state().dialog.as_ref() {
        Some(Dialog::NewProject(form)) => form.generator_label(),
        _ => panic!("the dialog closed"),
    };
    harness.get_by_label("Rust Application").click();
    harness.run();
    assert_eq!(label(&harness), "Rust Application");
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(label(&harness), "Rust Library");
    harness.key_press(egui::Key::Home);
    harness.run();
    // Section titles are not generators: Home stops at the first one.
    assert_eq!(label(&harness), "New Project");
    harness.key_press(egui::Key::End);
    harness.run();
    assert_eq!(label(&harness), "Rust Library");
    harness.key_press(egui::Key::ArrowUp);
    harness.run();
    assert_eq!(label(&harness), "Rust Application");
}

#[test]
fn empty_generator_pages_describe_what_they_create() {
    let app = app_with_project("new-project-empty");
    let form = crate::new_project::Form::new(Vec::new());
    let mut harness = popups_harness(Theme::dark(), app, Dialog::NewProject(Box::new(form)));
    harness.get_by_label("Rust Application").click();
    harness.run();
    harness.get_by_label_contains("src/main.rs");
    harness.get_by_label_contains("Cargo.toml");
    harness.get_by_label("Empty Project").click();
    harness.run();
    harness.get_by_label_contains("README.md");
    assert!(harness.query_by_label_contains("Cargo.toml").is_none());
}

#[test]
fn rename_dialog_passes_the_audit() {
    for (name, theme) in themes() {
        let app = app_with_project(&format!("rename-{name}"));
        let dialog = Dialog::Rename {
            path: "README.md".into(),
            name: "README.md".into(),
        };
        let mut harness = popups_harness(theme, app, dialog);
        harness.get_by_label("Rename");
        harness.get_by_label("Cancel");
        harness.get_by_label("Close");
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn rename_by_typing_and_enter_renames_the_file() {
    let app = app_with_project("rename-enter");
    let root = app.state().project.as_ref().unwrap().root().to_owned();
    let dialog = Dialog::Rename {
        path: "README.md".into(),
        name: "README.md".into(),
    };
    let mut harness = popups_harness(Theme::dark(), app, dialog);
    let field = harness.get_by_label("New name");
    field.click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.get_by_label("New name").type_text("NOTES.md");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(root.join("NOTES.md").is_file());
    assert!(!root.join("README.md").exists());
    assert!(harness.state().dialog.is_none(), "the dialog closes");
}

#[test]
fn rename_to_an_existing_name_says_so_under_the_field() {
    let app = app_with_project("rename-clash");
    let dialog = Dialog::Rename {
        path: "README.md".into(),
        name: "build.gradle.kts".into(),
    };
    let mut harness = popups_harness(Theme::dark(), app, dialog);
    harness.get_by_label("Rename").click();
    harness.run();
    harness.get_by_label_contains("New name: “");
    harness.get_by_label_contains("already exists");
    assert!(harness.state().dialog.is_some(), "the dialog stays open");
}

#[test]
fn delete_dialog_passes_the_audit_and_enter_confirms() {
    for (name, theme) in themes() {
        let app = app_with_project(&format!("delete-{name}"));
        let mut harness = popups_harness(
            theme,
            app,
            Dialog::Delete {
                path: "README.md".into(),
            },
        );
        harness.get_by_label("Delete");
        testing::audit(&mut harness).assert_clean();
    }
    let app = app_with_project("delete-enter");
    let root = app.state().project.as_ref().unwrap().root().to_owned();
    let mut harness = popups_harness(
        Theme::dark(),
        app,
        Dialog::Delete {
            path: "README.md".into(),
        },
    );
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(!root.join("README.md").exists());
    assert!(harness.state().dialog.is_none());
}

#[test]
fn escape_cancels_and_the_file_stays() {
    let app = app_with_project("delete-escape");
    let root = app.state().project.as_ref().unwrap().root().to_owned();
    let mut harness = popups_harness(
        Theme::dark(),
        app,
        Dialog::Delete {
            path: "README.md".into(),
        },
    );
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(root.join("README.md").exists());
    assert!(harness.state().dialog.is_none());
}

#[test]
fn clone_dialog_passes_the_audit() {
    for (name, theme) in themes() {
        let app = app_with_project(&format!("clone-{name}"));
        let mut harness = popups_harness(theme, app, Dialog::Clone(Box::default()));
        harness.get_by_label("Clone");
        harness.get_by_label("URL");
        harness.get_by_label("Directory");
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn push_dialog_passes_the_audit() {
    for (name, theme) in themes() {
        let mut app = app_with_git(&format!("push-{name}"));
        // A remote to push to: the plan page lists the commit.
        let root = app.state().project.as_ref().unwrap().root().to_owned();
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["remote", "add", "origin", "https://example.invalid/x.git"])
            .status()
            .unwrap();
        assert!(status.success());
        app.state.refresh_git().unwrap();
        crate::push_dialog::open(&mut app);
        let mut harness = dialog_harness(theme, app, |app, ctx| crate::push_dialog::show(app, ctx));
        harness.get_by_label("Push");
        harness.get_by_label_contains("first");
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn push_without_a_remote_offers_to_define_one() {
    for (name, theme) in themes() {
        let mut app = app_with_git(&format!("push-no-remote-{name}"));
        crate::push_dialog::open(&mut app);
        let mut harness = dialog_harness(theme, app, |app, ctx| crate::push_dialog::show(app, ctx));
        harness.get_by_label("This repository has no remote to push to.");
        assert!(
            harness
                .query_by_label_contains("Git command failed")
                .is_none()
        );
        harness.get_by_label("Define Remote…");
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn defining_a_remote_from_the_push_dialog_leads_to_the_plan() {
    let mut app = app_with_git("push-define");
    crate::push_dialog::open(&mut app);
    let mut harness = dialog_harness(Theme::dark(), app, |app, ctx| {
        crate::push_dialog::show(app, ctx)
    });
    harness.get_by_label("Define Remote…").click();
    harness.run();
    // A bad name is refused under its field; the form stays.
    harness.get_by_label("URL").click();
    harness.run();
    harness
        .get_by_label("URL")
        .type_text("https://example.invalid/x.git");
    harness.run();
    harness.get_by_label("Add").click();
    harness.run();
    harness.get_by_label("Push");
    harness.get_by_label_contains("first");
}

#[test]
fn first_run_about_and_open_project_pass_the_audit() {
    for (name, theme) in themes() {
        let mut app = app_with_project(&format!("first-run-{name}"));
        app.state.first_run_open = true;
        let mut harness = dialog_harness(theme.clone(), app, |app, ctx| {
            crate::first_run::show(app, ctx)
        });
        harness.get_by_label("Start Using Jetthing");
        harness.get_by_label("Skip");
        testing::audit(&mut harness).assert_clean();

        let mut app = app_with_project(&format!("about-{name}"));
        app.state.about_open = true;
        let mut harness =
            dialog_harness(theme.clone(), app, |app, ctx| crate::about::show(app, ctx));
        harness.get_by_label("Copy");
        testing::audit(&mut harness).assert_clean();

        let app = app_with_project(&format!("open-project-{name}"));
        let dialog = Dialog::OpenProject {
            path: "/no/such/dir".into(),
            error: Some("there is no directory at /no/such/dir".into()),
        };
        let mut harness = popups_harness(theme, app, dialog);
        harness.get_by_label("Open");
        harness.get_by_label("Browse…");
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn run_configurations_dialog_passes_the_audit() {
    for (name, theme) in themes() {
        let app = app_with_project(&format!("run-config-{name}"));
        crate::run_config_dialog::open(&app, &egui::Context::default(), None);
        let mut harness = dialog_harness(theme, app, |app, ctx| {
            if !crate::run_config_dialog::is_open(ctx) {
                crate::run_config_dialog::open(app, ctx, None);
            }
            crate::run_config_dialog::show(app, ctx)
        });
        harness.get_by_label("Run/Debug Configurations");
        harness.get_by_label("Add New Configuration");
        testing::audit(&mut harness).assert_clean();
    }
}

/// The controls the dialogs are built from, under the kit's own checklist.
#[test]
fn dialog_controls_conform() {
    testing::conformance("Create", |ui, enabled| {
        ui.add(Button::primary("Create").enabled(enabled))
    })
    .run();
    testing::conformance("Cancel", |ui, enabled| {
        ui.add(Button::secondary("Cancel").enabled(enabled))
    })
    .run();
    testing::conformance("Close", |ui, enabled| {
        ui.add(IconButton::new(Glyph::Close, "Close").enabled(enabled))
    })
    .run();
    testing::conformance("Create Git repository", |ui, enabled| {
        let mut checked = false;
        ui.add(Checkbox::new(&mut checked, "Create Git repository").enabled(enabled))
    })
    .run();
    testing::conformance("Name", |ui, enabled| {
        let mut text = String::from("untitled");
        ui.add(TextField::new(&mut text).label("Name").enabled(enabled))
    })
    .no_click()
    .keys(&[])
    .run();
    testing::conformance("JDK", |ui, enabled| {
        let mut jdk = "21".to_owned();
        ui.add(
            Dropdown::new("jdk", &mut jdk, ["17".to_owned(), "21".to_owned()])
                .label("JDK")
                .enabled(enabled),
        )
    })
    .keys(&[egui::Key::Enter])
    .run();
}

#[test]
fn plugin_manager_page_passes_the_audit() {
    for (name, theme) in themes() {
        let app = app_with_project(&format!("plugins-{name}"));
        let state = (
            app,
            crate::plugin_manager::PluginManager::default(),
            Vec::new(),
        );
        let mut harness = testing::harness_state(theme, state, |ui, (app, manager, requests)| {
            ui.allocate_ui(egui::vec2(900.0, 520.0), |ui| {
                ui.set_min_size(egui::vec2(900.0, 520.0));
                let mut cx = crate::plugin_manager::PageContext {
                    state: &app.state,
                    requests,
                };
                crate::plugin_manager::page(ui, manager, &mut cx);
            });
        });
        harness.run();
        harness.get_by_label_contains("Installed (");
        harness.get_by_label("Manage plugins");
        testing::audit(&mut harness).assert_clean();
    }
}

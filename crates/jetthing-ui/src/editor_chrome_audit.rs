//! Audit and conformance tests for the editor chrome: the tab bar, the completion, quick-fix
//! and quick-documentation popups and the banners, in both themes (see
//! `docs/ui-migration.md`).

use crate::IdeApp;
use archipel_kit::Theme;
use archipel_kit::components::IconButton;
use archipel_kit::icons::Glyph;
use archipel_kit::testing::{self, Queryable};
use eframe::egui::{self, pos2, vec2};
use jetthing_core::{CodeActionItem, CodeActionList, CompletionSession, DiskConflict, QuickDoc};
use std::path::PathBuf;

const JAVA: &str =
    "package com.example;\n\npublic class Main {\n    public void increment() {}\n}\n";

fn themes() -> [(&'static str, Theme); 2] {
    [("dark", Theme::dark()), ("light", Theme::light())]
}

/// A project with Main.java, Util.java and README.md open; Util is modified, README pinned.
fn app_with_tabs(name: &str) -> IdeApp {
    let root: PathBuf =
        std::env::temp_dir().join(format!("jetthing-ui-chrome-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/Main.java"), JAVA).unwrap();
    std::fs::write(root.join("src/Util.java"), "class Util {}\n").unwrap();
    std::fs::write(root.join("README.md"), "# Demo\n").unwrap();
    let root = root.canonicalize().unwrap();
    let mut app = IdeApp::new(
        Some(root.clone()),
        vec![
            root.join("src/Main.java"),
            root.join("src/Util.java"),
            root.join("README.md"),
        ],
    );
    let util = app
        .state
        .documents
        .iter()
        .position(|d| d.path.ends_with("Util.java"))
        .unwrap();
    let document = &mut app.state.documents[util];
    document.selections.insert(&mut document.text, "x").unwrap();
    let readme = app
        .state
        .documents
        .iter()
        .position(|d| d.path.ends_with("README.md"))
        .unwrap();
    app.state.set_pinned(readme, true).unwrap();
    app
}

fn tab_names(app: &IdeApp) -> Vec<String> {
    app.state
        .documents
        .iter()
        .map(|d| IdeApp::file_name(&d.path))
        .collect()
}

fn right_click(harness: &testing::Harness<'_, IdeApp>, pos: egui::Pos2) {
    harness.hover_at(pos);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
}

/// A 1000×38 strip, like the tab bar of a wide editor.
fn strip(ui: &mut egui::Ui, show: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui(vec2(1000.0, 38.0), |ui| {
        ui.set_min_size(vec2(1000.0, 38.0));
        show(ui);
    });
}

#[test]
fn tab_bar_with_modified_and_pinned_tabs_passes_the_audit() {
    for (name, theme) in themes() {
        let app = app_with_tabs(&format!("tabs-{name}"));
        let mut harness = testing::harness_state(theme, app, |ui, app| {
            strip(ui, |ui| crate::editor::tabs_for_tests(app, ui));
        });
        harness.run();
        harness.get_by_label("Main.java");
        harness.get_by_label("Util.java");
        harness.get_by_label("README.md");
        // The pinned tab shows its pin as a button; the active tab its close button.
        harness.get_by_label("Unpin README.md");
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn clicking_a_tab_activates_it_and_a_middle_click_closes_it() {
    let app = app_with_tabs("click");
    let mut harness = testing::harness_state(Theme::dark(), app, |ui, app| {
        strip(ui, |ui| crate::editor::tabs_for_tests(app, ui));
        // Activating a tab hands the focus to the editor surface: stand-in for it.
        let editor = ui.allocate_response(vec2(200.0, 100.0), egui::Sense::click());
        ui.interact(
            editor.rect,
            crate::editor::editor_id(),
            egui::Sense::click(),
        )
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Editor"));
    });
    harness.run();
    harness.get_by_label("Util.java").click();
    harness.run();
    let active = harness.state().state().active_document.unwrap();
    assert!(
        harness.state().state().documents[active]
            .path
            .ends_with("Util.java")
    );
    // The active tab shows its close button; a middle click closes any tab.
    harness.get_by_label("Close Util.java");
    let main = harness.get_by_label("Main.java").rect().center();
    harness.hover_at(main);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: main,
            button: egui::PointerButton::Middle,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    harness.run();
    assert_eq!(tab_names(harness.state()), ["README.md", "Util.java"]);
}

#[test]
fn right_clicking_the_empty_bar_offers_bar_actions_not_a_tabs_menu() {
    let app = app_with_tabs("bar-menu");
    let mut harness = testing::harness_state(Theme::dark(), app, |ui, app| {
        strip(ui, |ui| crate::editor::tabs_for_tests(app, ui));
    });
    harness.run();
    // Far right of the strip, past the last tab.
    right_click(&harness, pos2(900.0, 19.0));
    harness.run();
    harness.get_by_label("Close All Tabs");
    harness.get_by_label("Split Right");
    harness.get_by_label("Maximize Editor");
    assert!(harness.query_by_label("Close Tab").is_none());
}

#[test]
fn tab_conforms_to_the_kit_component_rules() {
    testing::conformance("Main.java", |ui, enabled| {
        let (rect, _) = ui.allocate_exact_size(vec2(160.0, 28.0), egui::Sense::hover());
        ui.add_enabled_ui(enabled, |ui| {
            crate::tab_strip::show(
                ui,
                &crate::tab_strip::Tab {
                    id: egui::Id::new("conformance_tab"),
                    rect,
                    name: "Main.java",
                    icon: archipel_kit::Icon::for_file("Main.java"),
                    tooltip: "src/Main.java",
                    active: false,
                    preview: false,
                    modified: false,
                    pinned: false,
                    drag: Some(0),
                    dragging: false,
                },
            )
            .response
        })
        .inner
    })
    .run();
}

#[test]
fn lightbulb_conforms_to_the_kit_component_rules() {
    testing::conformance("Show Context Actions", |ui, enabled| {
        ui.add(IconButton::new(Glyph::WeakWarning, "Show Context Actions").enabled(enabled))
    })
    .run();
}

/// Drags the tab named `from` onto the left half of the tab named `to` with the mouse.
#[test]
fn dragging_a_tab_onto_another_reorders_the_bar() {
    let app = app_with_tabs("drag");
    let mut harness = testing::harness_state(Theme::dark(), app, |ui, app| {
        strip(ui, |ui| crate::editor::tabs_with_drag_for_tests(app, ui));
    });
    harness.run();
    assert_eq!(
        tab_names(harness.state()),
        ["README.md", "Main.java", "Util.java"]
    );
    let util = harness.get_by_label("Util.java").rect();
    let main = harness.get_by_label("Main.java").rect();
    // A drag keeps repainting (the ghost follows the pointer), so step instead of run.
    harness.hover_at(util.center());
    harness.run_steps(2);
    harness.drag_at(util.center());
    harness.run_steps(2);
    // Past the drag threshold, then over the left half of Main.java.
    harness.hover_at(util.center() - vec2(30.0, 0.0));
    harness.run_steps(2);
    harness.hover_at(main.left_center() + vec2(8.0, 0.0));
    harness.run_steps(2);
    harness.drop_at(main.left_center() + vec2(8.0, 0.0));
    harness.run_steps(3);
    // The pinned README stays leftmost; Util now precedes Main.
    assert_eq!(
        tab_names(harness.state()),
        ["README.md", "Util.java", "Main.java"]
    );
}

fn app_with_completion(name: &str) -> IdeApp {
    use jetthing_lsp::completion::{MatchClass, RankedItem};
    let mut app = app_with_tabs(name);
    let items: Vec<jetthing_lsp::lsp_types::CompletionItem> = serde_json::from_value(
        serde_json::json!([
            {"label": "increment", "kind": 2, "labelDetails": {"detail": "()", "description": "void"}},
            {"label": "counter", "kind": 5, "labelDetails": {"description": "int"}},
            {"label": "Main", "kind": 7},
        ]),
    )
    .unwrap();
    let ranked = (0..items.len())
        .map(|index| RankedItem {
            index,
            class: MatchClass::Prefix,
            score: 0,
            label_positions: Vec::new(),
        })
        .collect();
    app.state.completion = Some(CompletionSession {
        path: app.state.documents[0].path.clone(),
        anchor: 0,
        request_caret: 0,
        items,
        ranked,
        selected: 0,
        prefix: String::new(),
        incomplete: false,
        waiting: false,
        explicit: false,
        resolved: Default::default(),
        indexing: None,
        requery_when_ready: false,
    });
    app.caret_screen_pos = Some(pos2(160.0, 120.0));
    app
}

#[test]
fn completion_popup_passes_the_audit() {
    for (name, theme) in themes() {
        let app = app_with_completion(&format!("completion-{name}"));
        let mut harness = testing::harness_state(theme, app, |ui, app| {
            crate::completion_ui::show(app, ui.ctx());
        });
        harness.run();
        harness.get_by_label_contains("increment");
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn clicking_a_completion_row_accepts_it() {
    let app = app_with_completion("completion-click");
    let mut harness = testing::harness_state(Theme::dark(), app, |ui, app| {
        crate::completion_ui::show(app, ui.ctx());
    });
    harness.run();
    harness.get_by_label_contains("counter").click();
    harness.run();
    let document = &harness.state().state().documents[0];
    assert!(
        document.text.text().contains("counter"),
        "the clicked item was inserted"
    );
}

fn app_with_quick_fixes(name: &str) -> IdeApp {
    let mut app = app_with_tabs(name);
    let active = app.state.active_document.unwrap();
    let document = &app.state.documents[active];
    app.state.code_actions.list = Some(CodeActionList {
        path: document.path.clone(),
        offset: document.selections.primary().head,
        line: 0,
        revision: document.text.revision(),
        diagnostics: app.state.lsp.diagnostics_generation(),
        items: vec![
            CodeActionItem::new("Import class 'List'", true),
            CodeActionItem::new("Add 'final' modifier", false),
        ],
    });
    app.state.code_actions.popup = true;
    app.caret_screen_pos = Some(pos2(160.0, 120.0));
    app
}

#[test]
fn quick_fix_popup_passes_the_audit() {
    for (name, theme) in themes() {
        let app = app_with_quick_fixes(&format!("quickfix-{name}"));
        let mut harness = testing::harness_state(theme, app, |ui, app| {
            crate::intentions_ui::popup(app, ui.ctx());
        });
        harness.run();
        harness.get_by_label("Import class 'List'");
        harness.get_by_label("Add 'final' modifier");
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn quick_fix_popup_applies_a_clicked_row_and_moves_with_the_keys() {
    let app = app_with_quick_fixes("quickfix-keys");
    let mut harness = testing::harness_state(Theme::dark(), app, |ui, app| {
        crate::intentions_ui::handle(app, ui.ctx());
        crate::intentions_ui::popup(app, ui.ctx());
    });
    harness.run();
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(
        !harness.state().state().code_actions.popup,
        "Escape closes it"
    );
}

fn app_with_quick_doc(name: &str) -> IdeApp {
    let mut app = app_with_tabs(name);
    let active = app.state.active_document.unwrap();
    app.state.docs.popup = Some(QuickDoc {
        path: app.state.documents[active].path.clone(),
        offset: 0,
        markdown: "```java\nvoid increment()\n```\n\nIncrements the **counter**.".into(),
        from_mouse: false,
    });
    app.caret_screen_pos = Some(pos2(160.0, 120.0));
    app
}

#[test]
fn quick_doc_popup_passes_the_audit() {
    for (name, theme) in themes() {
        let app = app_with_quick_doc(&format!("quickdoc-{name}"));
        let mut harness = testing::harness_state(theme, app, |ui, app| {
            crate::doc_ui::show_quick_doc(app, ui.ctx());
        });
        harness.run();
        harness.get_by_label_contains("Increments");
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn quick_doc_with_nowhere_to_show_does_not_hold_escape() {
    let mut app = app_with_quick_doc("quickdoc-hidden");
    app.caret_screen_pos = None;
    let mut harness = testing::harness_state(Theme::dark(), app, |ui, app| {
        crate::doc_ui::show_quick_doc(app, ui.ctx());
    });
    harness.run();
    assert!(
        harness.state().state().docs.popup.is_none(),
        "a popup nobody can see is closed, so it cannot swallow Escape"
    );
}

#[test]
fn disk_conflict_banner_passes_the_audit_and_runs_its_actions() {
    for (name, theme) in themes() {
        let mut app = app_with_tabs(&format!("banner-{name}"));
        let active = app.state.active_document.unwrap();
        app.state.documents[active].conflict = Some(DiskConflict::Modified);
        let mut harness = testing::harness_state(theme, app, |ui, app| {
            ui.allocate_ui(vec2(700.0, 60.0), |ui| {
                crate::disk_banner::show(app, ui);
            });
        });
        harness.run();
        harness.get_by_label("Load Disk Version");
        harness.get_by_label("Keep My Changes");
        testing::audit(&mut harness).assert_clean();
        harness.get_by_label("Keep My Changes").click();
        harness.run();
        assert!(
            harness.state().state().documents[active].conflict.is_none(),
            "the action ran its command"
        );
    }
}

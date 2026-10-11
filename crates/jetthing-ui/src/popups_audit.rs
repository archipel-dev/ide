//! Audit and conformance tests of the popup layer: Search Everywhere, the Switcher, the
//! chooser popups, notifications and the popup stack's Escape rule.

use crate::ui_audit::{app_with_project, panel, themes};
use crate::{palette, popup_stack::PopupId};
use archipel_kit::testing::{self, Queryable};
use eframe::egui;
use jetthing_core::NavigationItem;
use jetthing_core::quick_nav::SearchTab;
use std::path::PathBuf;

fn item(path: &str, line: usize) -> NavigationItem {
    NavigationItem {
        path: PathBuf::from(path),
        external: false,
        line,
        column: 4,
        end_column: 8,
        line_text: "    main();".into(),
    }
}

#[test]
fn search_everywhere_passes_the_audit_in_both_themes() {
    for (name, theme) in themes() {
        let mut app = app_with_project(&format!("se-{name}"));
        app.palette.show_tab(SearchTab::All);
        palette::set_query_for_tests(&mut app, "main");
        let mut harness = testing::harness_state(theme, app, |ui, app| {
            // The popup is 670 wide; the tabs and the option share a line.
            ui.allocate_ui(egui::vec2(670.0, 420.0), |ui| {
                ui.set_min_size(egui::vec2(670.0, 420.0));
                palette::body_for_tests(app, ui);
            });
        });
        harness.run();
        // Every tab is there, and the class that matches the query is listed.
        for tab in SearchTab::ALL {
            harness.get_by_label(tab.label());
        }
        assert!(
            !harness
                .query_all_by_label_contains("Main")
                .collect::<Vec<_>>()
                .is_empty()
        );
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn search_everywhere_result_row_conforms() {
    testing::conformance("Main.java", |ui, enabled| {
        palette::result_row_for_tests(ui, enabled, "Main.java")
    })
    .run();
}

#[test]
fn chooser_and_usage_rows_conform() {
    testing::conformance("Main.java 4", |ui, enabled| {
        crate::navigation_ui::choice_row_for_tests(ui, enabled, &item("src/Main.java", 3))
    })
    .run();
    testing::conformance("Usage in Main.java line 4", |ui, enabled| {
        crate::navigation_ui::usage_row_for_tests(ui, enabled, &item("src/Main.java", 3))
    })
    .run();
}

#[test]
fn switcher_passes_the_audit_in_both_themes() {
    for (name, theme) in themes() {
        let mut app = app_with_project(&format!("switcher-{name}"));
        app.state.switcher_advance(false);
        assert!(app.state.quick_nav.switcher.is_some());
        let mut harness = testing::harness_state(theme, app, |ui, app| {
            panel(ui, |ui| crate::switcher::list_for_tests(app, ui));
        });
        harness.run();
        harness.get_by_label_contains("Main.java");
        testing::audit(&mut harness).assert_clean();
    }
}

#[test]
fn a_completed_commit_shows_a_balloon_without_internal_ids() {
    for (name, theme) in themes() {
        let mut app = app_with_project(&format!("balloon-{name}"));
        app.state.notices.push(
            jetthing_core::NoticeKind::Success,
            "Commit done",
            "2 files committed.",
        );
        let mut harness = testing::harness_state(theme, app, |ui, app| {
            crate::notifications::show(app, ui.ctx());
        });
        harness.run();
        harness.get_by_label("Commit done");
        harness.get_by_label("2 files committed.");
        // The balloon is dismissable with the mouse.
        harness.get_by_label("Dismiss").click();
        harness.run();
        assert!(harness.query_by_label("Commit done").is_none());
    }
}

fn escape(ctx: &egui::Context) {
    ctx.input_mut(|input| {
        input.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
    });
}

#[test]
fn escape_closes_only_the_top_most_popup_and_a_hidden_one_cannot_take_it() {
    let mut app = app_with_project("escape");
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput::default());
    app.palette.show_tab(SearchTab::All);
    app.state.docs.popup = None;
    // Search Everywhere opened over the Switcher: Escape closes Search Everywhere first.
    app.state.switcher_advance(false);
    app.handle_shortcuts_for_tests(&ctx);
    assert_eq!(app.popup_stack.top(), Some(PopupId::SearchEverywhere));
    escape(&ctx);
    app.handle_shortcuts_for_tests(&ctx);
    assert!(!app.palette.open);
    assert!(
        app.state.quick_nav.switcher.is_some(),
        "the Switcher is still open"
    );
    escape(&ctx);
    app.handle_shortcuts_for_tests(&ctx);
    assert!(app.state.quick_nav.switcher.is_none());
    assert_eq!(app.popup_stack.top(), None);
    let _ = ctx.end_pass();
}

#[test]
fn main_finds_symbols_and_drops_scattered_action_matches() {
    use jetthing_core::quick_nav::Hit;
    let mut app = app_with_project("main-query");
    app.palette.show_tab(SearchTab::All);
    palette::set_query_for_tests(&mut app, "main");
    // The symbol index is filled over a few frames.
    let mut found = Vec::new();
    for _ in 0..50 {
        found = palette::items_for_tests(&mut app);
        if found.iter().any(|r| matches!(r.item, Hit::Symbol(_))) {
            break;
        }
    }
    let kinds: Vec<&str> = found
        .iter()
        .map(|r| match &r.item {
            Hit::Symbol(_) => "symbol",
            Hit::File { .. } => "file",
            Hit::Action(_) => "action",
            Hit::Text(_) => "text",
        })
        .collect();
    assert!(kinds.contains(&"symbol"), "{kinds:?}");
    // Letters scattered over words (`M-a-in` in "Markdown: Insert Image") do not match.
    assert!(!kinds.contains(&"action"), "{kinds:?}");
    // Within a section, a contiguous match (`Main`) comes before scattered letters.
    for hit in &found {
        if let Hit::Symbol(symbol) = &hit.item {
            let name = symbol.name.to_lowercase();
            if name.contains("main") {
                break;
            }
            panic!("{name} ranks above a contiguous match");
        }
    }
}

//! The chrome for fixture states: composition, order, enabled state, purity.
//! Spec: TUI-GUI-002, TUI-CMP-006, TUI-GUI-190 (tests).

mod common;

use archipel_ide_view::{Caps, Chrome};
use archipel_kit_model::{Notation, ToolbarItem, outline};
use common::Fixture;

fn labels(o: &archipel_kit_model::Outline) -> Vec<String> {
    o.entries.iter().map(|e| e.label.clone()).collect()
}

#[test]
fn no_project_has_a_toolbar_a_menu_and_stripes() {
    let f = Fixture::empty();
    let chrome = Chrome::build(&f.cx(Caps::kitty()));
    let ids: Vec<_> = chrome
        .toolbar
        .items()
        .filter_map(|i| i.id().map(str::to_owned))
        .collect();
    assert_eq!(ids.first().map(String::as_str), Some("main_menu"));
    assert!(ids.contains(&"search".to_owned()) && ids.contains(&"settings".to_owned()));
    let titles: Vec<_> = chrome
        .menu
        .headers
        .iter()
        .map(|h| h.label.as_str())
        .collect();
    assert_eq!(
        titles,
        [
            "File", "Edit", "View", "Navigate", "Code", "Refactor", "Build", "Run", "Tools", "Git",
            "Window", "Help"
        ]
    );
    assert!(chrome.left.top.iter().any(|b| b.id == "project"));
    assert!(chrome.right.top.iter().any(|b| b.id == "aiChat"));
    assert!(chrome.tabs.tabs.is_empty());
}

#[test]
fn a_java_project_shows_documents_branch_and_run_widget() {
    let f = Fixture::java("compose");
    let cx = f.cx(Caps::kitty());
    let chrome = Chrome::build(&cx);
    let tabs: Vec<_> = chrome.tabs.tabs.iter().map(|t| t.title.as_str()).collect();
    assert_eq!(tabs, ["Main.java", "README.md"]);
    assert!(chrome.tabs.tabs.iter().filter(|t| t.selected).count() == 1);
    // Run, Build, Debug, Search, Settings are at the right in that order.
    let trailing: Vec<_> = chrome
        .toolbar
        .trailing
        .iter()
        .filter_map(|i| match i {
            ToolbarItem::Button(b) => Some(b.id.as_str()),
            _ => None,
        })
        .collect();
    let at = |id| {
        trailing
            .iter()
            .position(|t| *t == id)
            .unwrap_or_else(|| panic!("{id} in {trailing:?}"))
    };
    assert!(
        at("run") < at("build")
            && at("build") < at("debug")
            && at("debug") < at("search")
            && at("search") < at("settings")
    );
    // The status bar has the caret and no mode, file, or count slots.
    let status = outline::status_bar(&chrome.status, Notation::Text);
    let text = status.text();
    assert!(status.entries.iter().any(|e| e.id == "caret"), "{text}");
    assert!(
        !text.contains("INSERT") && !text.contains("NORMAL"),
        "{text}"
    );
    // Breadcrumbs start with the project.
    assert!(chrome.status.breadcrumbs.crumbs[0].icon.is_some());
}

#[test]
fn builders_are_pure() {
    let f = Fixture::java("pure");
    let a = Chrome::build(&f.cx(Caps::kitty()));
    let b = Chrome::build(&f.cx(Caps::kitty()));
    assert_eq!(a, b);
}

#[test]
fn menu_items_without_a_project_are_greyed_like_the_gui() {
    let f = Fixture::empty();
    let chrome = Chrome::build(&f.cx(Caps::kitty()));
    let o = outline::menu_bar(&chrome.menu, Notation::Text);
    let find = |id: &str| {
        o.entries
            .iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("{id}"))
    };
    assert!(!find("SaveAll").enabled || !find("CloseProject").enabled);
    assert!(find("OpenFile").enabled);
    assert!(
        !labels(&o).iter().any(|l| l.starts_with("editor.")),
        "no raw ids"
    );
}

#[test]
fn shortcuts_follow_the_keyboard_the_frontend_has() {
    let f = Fixture::empty();
    let kitty = Chrome::build(&f.cx(Caps::kitty()));
    let legacy = Chrome::build(&f.cx(Caps::legacy()));
    let shortcut = |c: &Chrome, id: &str| {
        outline::menu_bar(&c.menu, Notation::Text)
            .entries
            .into_iter()
            .find(|e| e.id == id)
            .and_then(|e| e.shortcut)
    };
    assert_eq!(
        shortcut(&kitty, "GotoFile").as_deref(),
        Some("Ctrl+Shift+N")
    );
    // Legacy: the same, since no deliverable shortcut exists (the chord is documented elsewhere).
    assert!(shortcut(&legacy, "GotoFile").is_some());
}

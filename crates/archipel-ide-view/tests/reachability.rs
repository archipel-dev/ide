//! TUI-GUI-193: every action of the keymap that this IDE implements can be started by clicks
//! from the toolbar, the main menu, a stripe, a header, a context menu or Search Everywhere;
//! the list of unreachable actions is empty.

mod common;

use archipel_ide_keymap::{KeymapSet, Overrides};
use archipel_ide_view::{Caps, Chrome};
use common::Fixture;

#[test]
fn every_implemented_keymap_action_is_reachable_by_clicks() {
    let fixture = Fixture::java("reach");
    let cx = fixture.cx(Caps::kitty());
    let chrome = Chrome::build(&cx);
    let set = KeymapSet::shared().expect("keymaps");
    let keymap = set
        .keymap("$default", &Overrides::default())
        .expect("the default keymap");
    let mut unreachable = Vec::new();
    let mut checked = 0;
    for action in keymap.bindings().keys() {
        if action.starts_with('$')
            || !(archipel_ide_view::actions::is_implemented(action)
                || cx.state.action_registry().contains(action))
        {
            continue;
        }
        checked += 1;
        if chrome.reach(&cx, action).is_none() {
            unreachable.push(action.to_owned());
        }
    }
    assert!(checked > 100, "only {checked} actions checked");
    unreachable.sort();
    assert!(
        unreachable.is_empty(),
        "unreachable by clicks: {unreachable:?}"
    );
}

#[test]
fn the_main_actions_have_a_visible_control() {
    let fixture = Fixture::java("reach-visible");
    let cx = fixture.cx(Caps::kitty());
    let chrome = Chrome::build(&cx);
    for action in [
        "Run",
        "Debug",
        "Stop",
        "SearchEverywhere",
        "ShowSettings",
        "ActivateProjectToolWindow",
        "ActivateTerminalToolWindow",
        "SaveAll",
        "GotoFile",
        "FindInPath",
    ] {
        let path = chrome.reach(&cx, action).unwrap_or_default();
        assert!(
            path.first()
                .is_some_and(|first| first != "Search Everywhere"),
            "{action} has a visible control: {path:?}"
        );
    }
}

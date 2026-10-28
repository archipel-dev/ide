//! TUI-GUI-170: the terminal's chord alternatives never collide with a keymap or each other.
//! Spec: TUI-GUI-156, TUI-GUI-157, TUI-GUI-165, TUI-GUI-170 (tests).

use archipel_ide_keymap::{KeymapSet, Overrides};
use archipel_ide_view::keys::{CHORDS, LEADER, leader_for, leader_is_bound};
use std::collections::BTreeMap;

#[test]
fn the_leader_is_unbound_in_every_shipped_keymap() {
    let set = KeymapSet::shared().expect("keymaps");
    assert_eq!(LEADER, "ctrl \\");
    let ids: Vec<String> = set.ids().map(str::to_owned).collect();
    assert!(ids.len() >= 26, "{ids:?}");
    let mut fallbacks = Vec::new();
    for id in ids {
        let keymap = set
            .keymap(&id, &Overrides::default())
            .expect("keymap loads");
        let leader = leader_for(&keymap);
        assert!(!leader_is_bound(&keymap, leader), "{id}: {leader} is bound");
        if leader != LEADER {
            fallbacks.push(id.clone());
        }
        // The primary keymaps keep the documented leader.
        if archipel_ide_keymap::PRIMARY_KEYMAPS.contains(&id.as_str()) {
            assert_eq!(leader, LEADER, "{id}");
        }
    }
    // The spec claims `Ctrl+\` is free in every keymap; NetBeans, VSCode, Visual Studio 2022 and the macOS system keymap bind it (open question).
    assert_eq!(
        fallbacks,
        [
            "NetBeans 6.5",
            "VSCode",
            "Visual Studio 2022",
            "macOS System Shortcuts"
        ],
        "{fallbacks:?}"
    );
}

#[test]
fn no_two_chord_alternatives_share_a_key() {
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    for (action, key) in CHORDS {
        if let Some(previous) = seen.insert(key, action) {
            panic!("chord {key} is both {previous} and {action}");
        }
    }
    let mut actions: BTreeMap<&str, &str> = BTreeMap::new();
    for (action, key) in CHORDS {
        if let Some(previous) = actions.insert(action, key) {
            panic!("{action} has two chords: {previous} and {key}");
        }
    }
}

#[test]
fn the_old_leaders_ctrl_k_and_ctrl_w_keep_their_gui_meaning() {
    let set = KeymapSet::shared().expect("keymaps");
    let keymap = set
        .keymap("$default", &Overrides::default())
        .expect("keymap");
    let has = |action: &str, text: &str| {
        keymap
            .shortcuts(action)
            .iter()
            .any(|s| s.label(archipel_ide_keymap::Notation::Text) == text)
    };
    assert!(has("CheckinProject", "Ctrl+K"));
    assert!(has("EditorSelectWord", "Ctrl+W"));
    assert!(has("ParameterInfo", "Ctrl+P"));
}

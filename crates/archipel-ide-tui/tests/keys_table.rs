//! TUI-GUI-194: the key table (appendix B). In kitty caps the GUI shortcut performs the
//! action; in legacy caps the chord of the single leader performs it, and `sendable` says
//! which GUI shortcuts a legacy terminal cannot send.

mod common;

use archipel_ide_view::keys::{CHORDS, leader_for};
use archipel_kit::action_system::{Keymap, Os, Shortcut};
use archipel_kit_tui::input::{Caps, Event, KeyCode, KeyModifiers, key_with, sendable};
use archipel_kit_tui::{Look, Runtime};
use common::{Project, Tui};

/// The event for one stroke in the kit's text form (`shift W`, `alt =`, `F12`, `Enter`).
fn stroke_event(text: &str) -> Event {
    let mut mods = KeyModifiers::NONE;
    let mut key = "";
    for part in text.split_whitespace() {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" => mods |= KeyModifiers::CONTROL,
            "alt" => mods |= KeyModifiers::ALT,
            "shift" => mods |= KeyModifiers::SHIFT,
            _ => key = part,
        }
    }
    let code = match key {
        "Enter" => KeyCode::Enter,
        "Tab" if mods.contains(KeyModifiers::SHIFT) => KeyCode::BackTab,
        "Tab" => KeyCode::Tab,
        "Space" => KeyCode::Char(' '),
        "Insert" => KeyCode::Insert,
        "Home" => KeyCode::Home,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        k if k.starts_with('F') && k.len() > 1 => KeyCode::F(k[1..].parse().expect("F key")),
        k if k.chars().count() == 1 => {
            let c = k.chars().next().expect("one char");
            // A shifted letter arrives as its upper case letter in terminals.
            if mods.contains(KeyModifiers::SHIFT) && c.is_ascii_alphabetic() {
                KeyCode::Char(c.to_ascii_uppercase())
            } else {
                KeyCode::Char(c.to_ascii_lowercase())
            }
        }
        other => panic!("unknown key {other:?}"),
    };
    key_with(code, mods)
}

fn runtime(keymap: Keymap, caps: Caps) -> Runtime {
    Runtime::with(Look::dark(), keymap, caps)
}

fn keymap() -> (Keymap, Tui, &'static str) {
    let project = Project::new("keys-table", &[("a.txt", "x\n")]);
    let tui = Tui::open(&project.root, &["a.txt"], 100, 30);
    // The project is removed with `project`; the keymap does not need it.
    let keymap = tui.app.keymap(Os::Linux);
    let leader = {
        let set = archipel_ide_keymap::KeymapSet::shared().expect("keymaps");
        let ide = set
            .keymap("$default", &archipel_ide_keymap::Overrides::default())
            .expect("keymap");
        leader_for(&ide)
    };
    (keymap, tui, leader)
}

#[test]
fn in_legacy_caps_every_chord_performs_its_action() {
    let (keymap, tui, leader) = keymap();
    let mut failures = Vec::new();
    let mut checked = 0;
    for (action, keys) in CHORDS {
        if !(tui.app.ide().knows_action(action)
            || archipel_ide_tui::TuiApp::is_shell_action(action)
            || *action == "ChooseRunConfiguration"
            || action.starts_with('$'))
        {
            failures.push(format!(
                "{action} has a chord but this IDE does not implement it"
            ));
            continue;
        }
        if !keymap.action_ids().iter().any(|id| id == action) {
            failures.push(format!("{action} has no binding in the terminal keymap"));
            continue;
        }
        let mut rt = runtime(keymap.clone(), Caps::legacy());
        rt.preprocess(&stroke_event(leader));
        rt.preprocess(&stroke_event(keys));
        checked += 1;
        if !rt.actions().iter().any(|id| id == action) {
            failures.push(format!(
                "{leader} {keys} should perform {action}: {:?}",
                rt.actions()
            ));
        }
    }
    assert!(checked > 50, "only {checked} chords were checked");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_gui_shortcut_that_a_legacy_terminal_cannot_send_has_a_chord() {
    let (keymap, _tui, _leader) = keymap();
    let legacy = Caps::legacy();
    let chords: std::collections::BTreeSet<&str> = CHORDS.iter().map(|(a, _)| *a).collect();
    let mut missing = Vec::new();
    for action in keymap.action_ids() {
        let shortcuts = keymap.shortcuts(&action);
        let keys: Vec<_> = shortcuts
            .iter()
            .filter_map(|s| match s {
                Shortcut::Keyboard(k) if k.second.is_none() => Some(k.first),
                _ => None,
            })
            .collect();
        if keys.is_empty() || !archipel_ide_view::actions::is_implemented(&action) {
            continue;
        }
        let any_sendable = keys.iter().any(|k| sendable(k, &legacy));
        // Another action on the same stroke (Find Selection in Path is Find in Path's key)
        // is covered by that action's chord.
        let covered = keymap.action_ids().iter().any(|other| {
            *other != action
                && chords.contains(other.as_str())
                && keymap.shortcuts(other).iter().any(|s| {
                    matches!(s, Shortcut::Keyboard(k) if k.second.is_none() && keys.contains(&k.first))
                })
        });
        if !any_sendable && !chords.contains(action.as_str()) && !covered {
            missing.push(action);
        }
    }
    missing.sort();
    // Every action with only unsendable single strokes has a chord (or is listed here with a
    // reason; the list is empty).
    assert!(missing.is_empty(), "no chord for: {missing:?}");
}

#[test]
fn in_kitty_caps_the_gui_shortcut_performs_the_action() {
    let (keymap, _tui, _leader) = keymap();
    for (action, shortcut) in [
        ("SearchEverywhere", "ctrl shift E"),
        ("GotoAction", "ctrl shift A"),
        ("GotoFile", "ctrl shift N"),
        ("FindInPath", "ctrl shift F"),
        ("Back", "ctrl alt Left"),
        ("ActivateTerminalToolWindow", "alt F12"),
    ] {
        let mut rt = runtime(keymap.clone(), Caps::kitty());
        rt.preprocess(&stroke_event(shortcut));
        assert!(
            rt.actions().iter().any(|id| id == action)
                || (action == "SearchEverywhere" && !rt.actions().is_empty()),
            "{shortcut} should perform {action}: {:?}",
            rt.actions()
        );
    }
}

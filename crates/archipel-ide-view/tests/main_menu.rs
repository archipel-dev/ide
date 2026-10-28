//! The composition rules of the main menu (TUI-GUI-035..042), moved from the GUI's
//! `main_menu.rs`: the reference structure, what is hidden or limited, the appended commands
//! and the shortcuts.
//! Spec: TUI-GUI-035, TUI-GUI-041, TUI-GUI-042 (tests).

mod common;

use archipel_ide_actions::MenuItem as RefItem;
use archipel_ide_view::actions::is_implemented;
use archipel_ide_view::main_menu::{extras, main_menu_items};
use std::collections::BTreeSet;

fn collect_actions(items: &[RefItem], out: &mut BTreeSet<String>) {
    for item in items {
        match item {
            RefItem::Action { id, .. } => {
                out.insert(id.clone());
            }
            RefItem::Submenu { items, .. } => collect_actions(items, out),
            RefItem::Separator(_) => {}
        }
    }
}

fn top(menu: &[RefItem], id: &str) -> Vec<RefItem> {
    menu.iter()
        .find_map(|item| match item {
            RefItem::Submenu { id: top, items, .. } if top == id => Some(items.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn labels(items: &[RefItem], out: &mut Vec<(String, String)>) {
    for item in items {
        match item {
            RefItem::Action { id, label } => out.push((id.clone(), label.clone())),
            RefItem::Submenu { id, label, items } => {
                out.push((id.clone(), label.clone()));
                labels(items, out);
            }
            RefItem::Separator(_) => {}
        }
    }
}

#[test]
fn implemented_actions_appear_in_their_reference_menus() {
    let menu = main_menu_items();
    let mut all = BTreeSet::new();
    collect_actions(&menu, &mut all);
    for id in [
        "ShowSettings",
        "SaveAll",
        "OpenFile",
        "Exit",
        "$Undo",
        "Find",
        "FindInPath",
        "GotoFile",
        "GotoLine",
        "RecentFiles",
        "CommentByLineComment",
        "EditorDuplicate",
        "CompileDirty",
        "CheckinProject",
        "CheckForUpdate",
        "About",
    ] {
        assert!(all.contains(id), "{id} missing from the main menu");
        assert!(is_implemented(id), "{id} not implemented");
    }
    let mut code = BTreeSet::new();
    collect_actions(&top(&menu, "CodeMenu"), &mut code);
    assert!(code.contains("CommentByLineComment"));
    assert!(code.contains("CodeCompletion"));
}

#[test]
fn no_menu_item_shows_a_raw_action_id() {
    let mut all = Vec::new();
    labels(&main_menu_items(), &mut all);
    for (id, label) in all {
        assert!(!label.is_empty(), "{id} has an empty label");
        // A raw id has no spaces; single-word ids ("Exit") are legitimate labels.
        let camel = id
            .chars()
            .zip(id.chars().skip(1))
            .any(|(a, b)| a.is_lowercase() && b.is_uppercase());
        assert!(
            !(label == id && (camel || id.contains('.'))),
            "{id} shows as its raw id"
        );
    }
}

#[test]
fn file_menu_has_one_plugins_entry_and_no_unimplemented_items() {
    let menu = main_menu_items();
    let file = top(&menu, "FileMenu");
    let mut actions = BTreeSet::new();
    collect_actions(&file, &mut actions);
    assert!(actions.iter().all(|id| is_implemented(id)));
    assert!(actions.contains("ShowPlugins"));
    let mut all = Vec::new();
    labels(&file, &mut all);
    assert_eq!(
        all.iter().filter(|(_, l)| l.starts_with("Plugins")).count(),
        1
    );
    let setup = file.iter().find_map(|item| match item {
        RefItem::Submenu { id, label, .. } if id == "FileOtherSettingsGroup" => Some(label),
        _ => None,
    });
    assert!(setup.is_none_or(|label| label == "New Projects Setup"));
}

#[test]
fn archipel_only_commands_are_appended() {
    let menu = main_menu_items();
    let mut covered = BTreeSet::new();
    collect_actions(&menu, &mut covered);
    let specs = archipel_ide_core::built_in_commands().specs();
    let ids = |menu: &str| -> Vec<String> {
        extras(menu, &specs, &covered)
            .into_iter()
            .map(|spec| spec.id.clone())
            .collect()
    };
    // Run is registered at runtime in the reference IDE, so it is an extra here.
    assert!(ids("RunMenu").contains(&"run.project".to_owned()));
    assert!(!ids("FileMenu").contains(&"plugin.refresh".to_owned()));
    assert!(!ids("FileMenu").contains(&"plugin.manager.open".to_owned()));
    assert!(ids("VcsGroups").contains(&"git.refresh".to_owned()));
    // Covered by reference entries: not repeated.
    assert!(!ids("BuildMenu").contains(&"build.project".to_owned()));
    assert!(!ids("FileMenu").contains(&"settings.open".to_owned()));
    assert!(!ids("HelpMenu").contains(&"app.checkForUpdates".to_owned()));
    assert!(
        !ids("ViewMenu")
            .iter()
            .any(|id| id.starts_with("toolwindow.activate."))
    );
}

#[test]
fn implemented_menu_shortcuts_are_unique_on_the_primary_keymaps() {
    let menu = main_menu_items();
    let mut actions = BTreeSet::new();
    collect_actions(&menu, &mut actions);
    let set = archipel_ide_keymap::KeymapSet::shared().unwrap();
    for id in archipel_ide_keymap::PRIMARY_KEYMAPS {
        let keymap = set.keymap(id, &Default::default()).unwrap();
        let mut seen = std::collections::HashMap::new();
        for action in actions.iter().filter(|a| is_implemented(a)) {
            for shortcut in keymap.shortcuts(action) {
                // Force touch is a trackpad gesture the reference macOS keymap gives to both
                // Go to Declaration and Run to Cursor.
                if shortcut.to_string() == "Force touch" {
                    continue;
                }
                // Two spellings of one shortcut on the same action are no conflict.
                if let Some(previous) = seen.insert(shortcut.clone(), action.clone())
                    && previous != *action
                {
                    panic!("{id}: {previous} and {action} share {shortcut}");
                }
            }
        }
        // Save All is Ctrl+S / Cmd+S.
        assert!(
            seen.keys()
                .any(|s| matches!(seen.get(s), Some(a) if a == "SaveAll"))
        );
    }
}

/// TUI-GUI-036: the mnemonic of each title is unique and the documented one.
#[test]
fn the_titles_have_the_documented_unique_mnemonics() {
    let fixture = common::Fixture::empty();
    let cx = fixture.cx(archipel_ide_view::Caps::kitty());
    let bar = archipel_ide_view::main_menu::main_menu(&cx);
    let letters: String = bar
        .headers
        .iter()
        .map(|h| h.mnemonic_char().expect("every title has a mnemonic"))
        .collect();
    assert_eq!(letters, "fevncrbutgwh");
    let unique: std::collections::BTreeSet<char> = letters.chars().collect();
    assert_eq!(unique.len(), letters.len());
}

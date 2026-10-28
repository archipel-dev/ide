//! The main menu: the reference structure of `MainMenu` (`Catalog::shared_menu`), minus what
//! the reference hides at runtime, with the File menu limited to what Archipel IDE implements,
//! the commands that have no reference action appended to the matching menu, and plugin menu
//! items (TUI-GUI-035..042). Enabled state comes from the action registry.

use crate::actions::is_implemented;
use crate::cx::ViewCx;
use archipel_ide_actions::{ActionPlace, Catalog, MenuItem as RefItem, filter_menu};
use archipel_ide_command::CommandSpec;
use archipel_ide_core::keymap_commands::ACTION_COMMANDS;
use archipel_kit_model::{MenuBar, MenuEntry, MenuHeader, MenuItem};
use std::collections::BTreeSet;

/// Command categories whose parameterless commands are offered in each top-level menu when
/// no reference action covers them.
pub const EXTRA_CATEGORIES: [(&str, &[&str]); 10] = [
    ("FileMenu", &["File", "Project", "Plugins"]),
    ("EditMenu", &["Editor"]),
    ("ViewMenu", &["Window"]),
    ("GoToMenu", &["Navigate", "Search"]),
    ("CodeMenu", &["Code"]),
    ("BuildMenu", &["Build"]),
    ("RunMenu", &["Run"]),
    ("ToolsMenu", &["Terminal", "Android"]),
    ("VcsGroups", &["Git"]),
    ("HelpMenu", &["Help"]),
];

/// The prefix of the id of a recent project entry (`recent:<path>`).
pub const RECENT_PREFIX: &str = "recent:";

/// The plugin API menu title of a top-level reference menu.
pub fn plugin_menu_title(menu: &str) -> Option<&'static str> {
    Some(match menu {
        "FileMenu" => "File",
        "EditMenu" => "Edit",
        "ViewMenu" => "View",
        "GoToMenu" => "Navigate",
        "CodeMenu" => "Code",
        "BuildMenu" => "Build",
        "RunMenu" => "Run",
        "ToolsMenu" => "Tools",
        "VcsGroups" => "Git",
        "HelpMenu" => "Help",
        _ => return None,
    })
}

/// Commands that never get a menu entry of their own (reached through other items).
pub fn hidden_extra(id: &str) -> bool {
    id.starts_with("toolwindow.activate.") || id == "search.cancel" || id == "plugin.refresh"
}

/// Action ids anywhere in `items`.
pub fn collect_actions(items: &[RefItem], out: &mut BTreeSet<String>) {
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

/// Whether a command can run without parameters (all declared fields optional).
pub fn parameterless(spec: &CommandSpec) -> bool {
    match serde_json::from_str::<serde_json::Value>(&spec.params) {
        Ok(serde_json::Value::Object(fields)) => fields
            .values()
            .all(|value| value.as_str().is_some_and(|kind| kind.ends_with('?'))),
        _ => false,
    }
}

/// Parameterless commands of `menu`'s categories that no reference entry runs.
pub fn extras<'a>(
    menu: &str,
    specs: &'a [CommandSpec],
    covered: &BTreeSet<String>,
) -> Vec<&'a CommandSpec> {
    let Some((_, categories)) = EXTRA_CATEGORIES.iter().find(|(id, _)| *id == menu) else {
        return Vec::new();
    };
    let covered_commands: BTreeSet<&str> = ACTION_COMMANDS
        .iter()
        .filter(|binding| covered.contains(binding.action))
        .map(|binding| binding.command)
        .collect();
    specs
        .iter()
        .filter(|spec| categories.contains(&spec.category.as_str()))
        .filter(|spec| parameterless(spec) && !hidden_extra(&spec.id))
        .filter(|spec| !covered_commands.contains(spec.id.as_str()))
        .collect()
}

/// Entries the reference IDE (Community edition on Linux) hides at runtime although its
/// action groups list them.
pub const HIDDEN_AT_RUNTIME: [&str; 4] = [
    "AttachProject",
    "CloseAllProjects",
    "CloseOtherProjects",
    "ConvertModuleGroupsToQualifiedNames",
];

/// The main menu items without the entries hidden at runtime.
pub fn main_menu_items() -> Vec<RefItem> {
    fn strip(items: Vec<RefItem>) -> Vec<RefItem> {
        items
            .into_iter()
            .filter(|item| {
                !matches!(item, RefItem::Action { id, .. } if HIDDEN_AT_RUNTIME.contains(&id.as_str()))
            })
            .map(|item| match item {
                RefItem::Submenu { id, label, items } => RefItem::Submenu {
                    id,
                    label,
                    items: strip(items),
                },
                other => other,
            })
            .collect()
    }
    // The File menu lists only what Archipel IDE implements: no disabled placeholders and no
    // raw action ids; the other menus keep the full reference structure.
    let menu = strip(Catalog::shared_menu("MainMenu").as_ref().clone());
    menu.into_iter()
        .map(|item| match item {
            RefItem::Submenu { id, label, items } if id == "FileMenu" => RefItem::Submenu {
                items: with_new_project(filter_menu(&items, &|action| is_implemented(action))),
                id,
                label,
            },
            other => other,
        })
        .collect()
}

/// File > New starts with "New Project…" (opens the New Project wizard).
fn with_new_project(mut items: Vec<RefItem>) -> Vec<RefItem> {
    let entry = RefItem::Action {
        id: "NewProject".into(),
        label: "New Project…".into(),
    };
    match items.iter_mut().find_map(|item| match item {
        RefItem::Submenu { id, items, .. } if id == "NewElementInMenuGroup" => Some(items),
        _ => None,
    }) {
        Some(new) => {
            new.insert(0, RefItem::Separator(None));
            new.insert(0, entry);
        }
        None => items.insert(
            0,
            RefItem::Submenu {
                id: "NewElementInMenuGroup".into(),
                label: "New".into(),
                items: vec![entry],
            },
        ),
    }
    items
}

/// The index (in characters of the cleaned text) of the mnemonic marked in a reference menu
/// text (`R_un` gives 1).
pub fn mnemonic_index(raw: &str) -> Option<usize> {
    let mut chars = raw.chars().peekable();
    let mut out = 0usize;
    while let Some(c) = chars.next() {
        match c {
            '_' | '&' if chars.peek() == Some(&c) => {
                chars.next();
                out += 1;
            }
            '_' | '&' => {
                return chars.peek().map(|_| out);
            }
            _ => out += 1,
        }
    }
    None
}

/// The mnemonic of the reference action or group `id`.
pub fn mnemonic_of(id: &str) -> Option<usize> {
    let raw = Catalog::shared().get(id)?.text.clone()?;
    mnemonic_index(&raw)
}

/// The text shown when an action is not implemented.
pub const NOT_IMPLEMENTED: &str = "Not available in Archipel IDE yet";

fn action_entry(cx: &ViewCx, id: &str, label: &str) -> Option<MenuEntry> {
    let implemented = is_implemented(id);
    let presentation = cx.presentation(id, ActionPlace::MainMenu);
    if implemented && !presentation.visible {
        return None;
    }
    let label = presentation
        .text
        .clone()
        .unwrap_or_else(|| label.to_owned());
    let mut item = MenuItem::new(id, label);
    item.mnemonic = mnemonic_of(id).filter(|at| *at < item.label.chars().count());
    item.shortcut = cx.shortcut(id);
    item.enabled = implemented && presentation.enabled;
    if !implemented {
        item.disabled_reason = Some(NOT_IMPLEMENTED.to_owned());
    }
    Some(MenuEntry::Item(item))
}

fn recent_projects(cx: &ViewCx) -> Vec<MenuEntry> {
    let entries: Vec<_> = cx.state.recent_projects().read().entries().to_vec();
    let mut out = Vec::new();
    if entries.is_empty() {
        let mut none = MenuItem::new("recent:none", "No recent projects");
        none.enabled = false;
        out.push(MenuEntry::Item(none));
    }
    for entry in &entries {
        let mut item = MenuItem::new(
            format!("{RECENT_PREFIX}{}", entry.path.display()),
            entry.name.clone(),
        );
        item.disabled_reason = None;
        out.push(MenuEntry::Item(item));
    }
    out.push(MenuEntry::separator());
    out
}

fn convert(cx: &ViewCx, items: &[RefItem], out: &mut Vec<MenuEntry>) {
    for item in items {
        match item {
            RefItem::Action { id, label } => out.extend(action_entry(cx, id, label)),
            RefItem::Submenu { id, label, items } => {
                let mut entries = Vec::new();
                if id == "$LRU" {
                    entries.extend(recent_projects(cx));
                }
                convert(cx, items, &mut entries);
                let mut item = MenuItem::submenu(id.clone(), label.clone(), entries);
                item.mnemonic = mnemonic_of(id).filter(|at| *at < label.chars().count());
                out.push(MenuEntry::Item(item));
            }
            RefItem::Separator(title) => out.push(MenuEntry::Separator {
                title: title.clone(),
            }),
        }
    }
}

/// Contributed plugin items of `menu` (`(command id, label, shortcut)`).
pub fn plugin_items(cx: &ViewCx, menu: &str) -> Vec<(String, String, Option<String>)> {
    cx.state
        .plugin_host_service()
        .registry()
        .entries()
        .iter()
        .filter(|entry| entry.is_active())
        .flat_map(|entry| {
            let specs = entry.contributions.command_specs();
            entry
                .contributions
                .menus
                .iter()
                .filter(|item| item.menu.title() == menu)
                .filter_map(move |item| {
                    specs
                        .iter()
                        .find(|spec| spec.id == item.command)
                        .map(|spec| (spec.id.clone(), spec.label.clone(), spec.shortcut.clone()))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The main menu bar with every menu complete: reference entries, appended commands, plugin
/// items.
pub fn main_menu(cx: &ViewCx) -> MenuBar {
    let menu = main_menu_items();
    let mut covered = BTreeSet::new();
    collect_actions(&menu, &mut covered);
    let specs = cx.commands.specs();
    let mut headers = Vec::new();
    for item in &menu {
        let RefItem::Submenu { id, label, items } = item else {
            continue;
        };
        let mut entries = Vec::new();
        convert(cx, items, &mut entries);
        let extra = extras(id, &specs, &covered);
        let mut appended = Vec::new();
        for spec in extra {
            let presentation = cx.presentation(&spec.id, ActionPlace::MainMenu);
            if !presentation.visible {
                continue;
            }
            let text = presentation
                .text
                .clone()
                .unwrap_or_else(|| spec.label.clone());
            let mut entry = MenuItem::new(spec.id.clone(), text);
            entry.enabled = presentation.enabled;
            entry.shortcut = command_shortcut(cx, spec);
            appended.push(MenuEntry::Item(entry));
        }
        if let Some(title) = plugin_menu_title(id) {
            let plugin = plugin_items(cx, title);
            if !plugin.is_empty() {
                appended.push(MenuEntry::separator());
            }
            for (command, label, shortcut) in plugin {
                let mut entry = MenuItem::new(command, label);
                entry.shortcut = shortcut.as_deref().map(archipel_kit_model::Shortcut::keys);
                appended.push(MenuEntry::Item(entry));
            }
        }
        if !appended.is_empty() {
            if !matches!(appended.first(), Some(MenuEntry::Separator { .. })) {
                entries.push(MenuEntry::separator());
            }
            entries.extend(appended);
        }
        let mut header = MenuHeader::new(id.clone(), label.clone()).entries(entries);
        header.mnemonic = mnemonic_of(id).filter(|at| *at < label.chars().count());
        headers.push(header);
    }
    MenuBar::new(headers)
}

/// Shortcut of a command: from the keymap when an action maps to it.
pub fn command_shortcut(cx: &ViewCx, spec: &CommandSpec) -> Option<archipel_kit_model::Shortcut> {
    archipel_ide_core::keymap_commands::actions_for_command(&spec.id)
        .find_map(|action| cx.shortcut(action))
}

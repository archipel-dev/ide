//! Main menu built from the reference action groups (`docs/spec/data/actions.json`,
//! `MainMenu`), as listed in `docs/spec/menus.md`.
//!
//! Every reference entry is shown in its place. Entries Archipel IDE implements (a mapped
//! command or workbench popup, see `keybindings`) are enabled according to the current
//! state; the rest are shown disabled with a "not available" tooltip. Archipel IDE commands
//! that have no reference action are appended to the matching menu.

use crate::{IdeApp, icons, keybindings, theme};
use archipel_ide_actions::{Catalog, MenuItem, filter_menu};
use archipel_ide_command::CommandSpec;
use archipel_ide_core::keymap_commands::ACTION_COMMANDS;
use eframe::egui::{self, RichText};
use serde_json::Value;
use std::collections::BTreeSet;

/// Command categories whose parameterless commands are offered in each top-level menu
/// when no reference action covers them.
const EXTRA_CATEGORIES: [(&str, &[&str]); 10] = [
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

/// The plugin API menu title (`MenuLocation::title`) of a top-level reference menu.
fn plugin_menu_title(menu: &str) -> Option<&'static str> {
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
fn hidden_extra(id: &str) -> bool {
    id.starts_with("toolwindow.activate.") || id == "search.cancel" || id == "plugin.refresh"
}

/// Action ids anywhere in `items`.
fn collect_actions(items: &[MenuItem], out: &mut BTreeSet<String>) {
    for item in items {
        match item {
            MenuItem::Action { id, .. } => {
                out.insert(id.clone());
            }
            MenuItem::Submenu { items, .. } => collect_actions(items, out),
            MenuItem::Separator(_) => {}
        }
    }
}

/// Parameterless commands of `menu`'s categories that no reference entry runs.
pub(crate) fn extras<'a>(
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
        .filter(|spec| IdeApp::parameterless(spec) && !hidden_extra(&spec.id))
        .filter(|spec| !covered_commands.contains(spec.id.as_str()))
        .collect()
}

/// Entries the reference IDE (Community edition on Linux) hides at runtime although
/// its action groups list them; observed in `reference/*/07-main-menu.png`.
const HIDDEN_AT_RUNTIME: [&str; 4] = [
    "AttachProject",
    "CloseAllProjects",
    "CloseOtherProjects",
    "ConvertModuleGroupsToQualifiedNames",
];

/// The main menu items without the entries hidden at runtime.
pub(crate) fn main_menu_items() -> Vec<MenuItem> {
    fn strip(items: Vec<MenuItem>) -> Vec<MenuItem> {
        items
            .into_iter()
            .filter(|item| {
                !matches!(item, MenuItem::Action { id, .. } if HIDDEN_AT_RUNTIME.contains(&id.as_str()))
            })
            .map(|item| match item {
                MenuItem::Submenu { id, label, items } => MenuItem::Submenu {
                    id,
                    label,
                    items: strip(items),
                },
                other => other,
            })
            .collect()
    }
    // The File menu lists only what Archipel IDE implements: no disabled placeholders and
    // no raw action ids; the other menus keep the full reference structure.
    let menu = strip(Catalog::shared_menu("MainMenu").as_ref().clone());
    menu.into_iter()
        .map(|item| match item {
            MenuItem::Submenu { id, label, items } if id == "FileMenu" => MenuItem::Submenu {
                items: with_new_project(filter_menu(&items, &|action| {
                    keybindings::is_implemented(action)
                })),
                id,
                label,
            },
            other => other,
        })
        .collect()
}

/// File > New starts with "New Project…" (opens the New Project wizard).
fn with_new_project(mut items: Vec<MenuItem>) -> Vec<MenuItem> {
    let entry = MenuItem::Action {
        id: "NewProject".into(),
        label: "New Project…".into(),
    };
    match items.iter_mut().find_map(|item| match item {
        MenuItem::Submenu { id, items, .. } if id == "NewElementInMenuGroup" => Some(items),
        _ => None,
    }) {
        Some(new) => {
            new.insert(0, MenuItem::Separator(None));
            new.insert(0, entry);
        }
        None => items.insert(
            0,
            MenuItem::Submenu {
                id: "NewElementInMenuGroup".into(),
                label: "New".into(),
                items: vec![entry],
            },
        ),
    }
    items
}

/// The hamburger button; when clicked, the menu titles expand inline in the main
/// toolbar (New UI behaviour). Returns whether the menu bar is expanded.
pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) -> bool {
    if !app.menu_bar_open {
        if icons::button(ui, icons::Icon::MainMenu, "Main Menu").clicked() {
            app.menu_bar_open = true;
            app.menu_bar_popup_seen = false;
        }
        return false;
    }
    let menu = main_menu_items();
    let mut covered = BTreeSet::new();
    collect_actions(&menu, &mut covered);
    let specs = app.commands.specs();
    let height = ui.available_height();
    let bar = ui
        .allocate_ui(egui::vec2(ui.available_width().min(720.0), height), |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                let mut titles = Vec::new();
                for item in &menu {
                    let MenuItem::Submenu { id, label, items } = item else {
                        continue;
                    };
                    let extra = extras(id, &specs, &covered);
                    let plugin_menu = plugin_menu_title(id);
                    let title = ui.menu_button(label, |ui| {
                        scrolled(ui, |ui| {
                            render(app, ui, items);
                            if !extra.is_empty() {
                                ui.separator();
                            }
                            for spec in extra {
                                command_item(app, ui, spec);
                            }
                            if let Some(title) = plugin_menu {
                                crate::plugin_ui::menu_items(app, ui, title);
                            }
                        });
                    });
                    titles.push(title.response);
                }
                switch_on_hover(ui.ctx(), &titles);
            })
        })
        .response;
    // Collapse after a menu closes, on a click elsewhere, or on Escape.
    let ctx = ui.ctx().clone();
    let popup_open = egui::Popup::is_any_open(&ctx);
    let clicked_outside = ctx.input(|input| {
        input.pointer.any_click()
            && input
                .pointer
                .interact_pos()
                .is_some_and(|pos| !bar.rect.contains(pos))
    });
    let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
    if popup_open {
        app.menu_bar_popup_seen = true;
    } else if app.menu_bar_popup_seen || clicked_outside || escape {
        app.menu_bar_open = false;
    }
    true
}

/// While one menu of the bar is open, hovering another title opens that one instead.
fn switch_on_hover(ctx: &egui::Context, titles: &[egui::Response]) {
    let popup = |response: &egui::Response| egui::Popup::default_response_id(response);
    let open = titles
        .iter()
        .any(|title| egui::Popup::is_id_open(ctx, popup(title)));
    if !open {
        return;
    }
    if let Some(hovered) = titles
        .iter()
        .find(|title| title.hovered() && !egui::Popup::is_id_open(ctx, popup(title)))
    {
        egui::Popup::close_all(ctx);
        egui::Popup::open_id(ctx, popup(hovered));
    }
}

/// Long menus scroll instead of running off the screen.
fn scrolled(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.set_min_width(260.0);
    let max_height = (ui.ctx().content_rect().height() - 80.0).max(200.0);
    egui::ScrollArea::vertical()
        .max_height(max_height)
        .min_scrolled_height(max_height)
        .show(ui, add);
}

fn render(app: &mut IdeApp, ui: &mut egui::Ui, items: &[MenuItem]) {
    for item in items {
        match item {
            MenuItem::Action { id, label } => action_item(app, ui, id, label),
            MenuItem::Submenu { id, label, items } => {
                ui.menu_button(label, |ui| {
                    scrolled(ui, |ui| {
                        if id == "$LRU" {
                            recent_projects(app, ui);
                        }
                        render(app, ui, items)
                    })
                });
            }
            MenuItem::Separator(title) => {
                ui.separator();
                if let Some(title) = title {
                    ui.label(
                        RichText::new(title)
                            .size(theme::UI_FONT_SIZE - 2.0)
                            .color(theme::TEXT_INFO),
                    );
                }
            }
        }
    }
}

/// The recently opened projects (most recent first) at the top of Recent Projects.
fn recent_projects(app: &mut IdeApp, ui: &mut egui::Ui) {
    let entries: Vec<_> = app.state.recent_projects.entries().to_vec();
    if entries.is_empty() {
        ui.add_enabled(false, egui::Button::new("No recent projects"));
    }
    for entry in &entries {
        let button = egui::Button::new(&entry.name);
        if ui
            .add(button)
            .on_hover_text(entry.path.display().to_string())
            .clicked()
        {
            let ctx = ui.ctx().clone();
            let path = entry.path.display().to_string();
            if let Err(error) = app.open_project_path(&path, &ctx) {
                app.state.status = error;
            }
            ui.close();
        }
    }
    ui.separator();
}

fn action_item(app: &mut IdeApp, ui: &mut egui::Ui, id: &str, label: &str) {
    let implemented = keybindings::is_implemented(id);
    let enabled = implemented && app.menu_action_enabled(id);
    let button =
        egui::Button::new(label).shortcut_text(app.action_shortcut(id).unwrap_or_default());
    let response = ui.add_enabled(enabled, button);
    if !implemented {
        response.on_disabled_hover_text("Not available in Archipel IDE yet");
    } else if response.clicked() {
        let ctx = ui.ctx().clone();
        app.run_menu_action(id, &ctx);
        ui.close();
    }
}

fn command_item(app: &mut IdeApp, ui: &mut egui::Ui, spec: &CommandSpec) {
    let enabled = app.commands.is_enabled(&spec.id, &app.state);
    let button = egui::Button::new(&spec.label).shortcut_text(app.command_shortcut(spec));
    if ui.add_enabled(enabled, button).clicked() {
        // Stash and unstash gather options in the Git dialogs.
        match spec.id.as_str() {
            "git.stash" => {
                app.git_dialog = Some(crate::branches_popup::GitDialog::Stash {
                    message: String::new(),
                    keep_index: false,
                    include_untracked: false,
                    error: None,
                });
                app.dialog_focus = true;
            }
            "git.stash.pop" => {
                app.git_dialog = Some(crate::branches_popup::GitDialog::Unstash {
                    selected: 0,
                    pop: true,
                    error: None,
                });
                app.dialog_focus = true;
            }
            _ => app.dispatch(&spec.id, Value::Null),
        }
        ui.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_core::built_in_commands;

    fn top(menu: &[MenuItem], id: &str) -> Vec<MenuItem> {
        menu.iter()
            .find_map(|item| match item {
                MenuItem::Submenu { id: top, items, .. } if top == id => Some(items.clone()),
                _ => None,
            })
            .unwrap_or_default()
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
            assert!(keybindings::is_implemented(id), "{id} not implemented");
        }
        let mut code = BTreeSet::new();
        collect_actions(&top(&menu, "CodeMenu"), &mut code);
        assert!(code.contains("CommentByLineComment"));
        assert!(code.contains("CodeCompletion"));
    }

    fn labels(items: &[MenuItem], out: &mut Vec<(String, String)>) {
        for item in items {
            match item {
                MenuItem::Action { id, label } => out.push((id.clone(), label.clone())),
                MenuItem::Submenu { id, label, items } => {
                    out.push((id.clone(), label.clone()));
                    labels(items, out);
                }
                MenuItem::Separator(_) => {}
            }
        }
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
        assert!(actions.iter().all(|id| keybindings::is_implemented(id)));
        assert!(actions.contains("ShowPlugins"));
        let mut all = Vec::new();
        labels(&file, &mut all);
        assert_eq!(
            all.iter().filter(|(_, l)| l.starts_with("Plugins")).count(),
            1
        );
        let setup = file.iter().find_map(|item| match item {
            MenuItem::Submenu { id, label, .. } if id == "FileOtherSettingsGroup" => Some(label),
            _ => None,
        });
        assert!(setup.is_none_or(|label| label == "New Projects Setup"));
    }

    #[test]
    fn archipel_only_commands_are_appended() {
        let menu = main_menu_items();
        let mut covered = BTreeSet::new();
        collect_actions(&menu, &mut covered);
        let specs = built_in_commands().specs();
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
            for action in actions.iter().filter(|a| keybindings::is_implemented(a)) {
                for shortcut in keymap.shortcuts(action) {
                    // Force touch is a trackpad gesture the reference macOS keymap
                    // gives to both Go to Declaration and Run to Cursor.
                    if shortcut.to_string() == "Force touch" {
                        continue;
                    }
                    if let Some(previous) = seen.insert(shortcut.clone(), action.clone()) {
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
}

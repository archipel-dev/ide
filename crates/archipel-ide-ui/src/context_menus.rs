//! Context menus built from the reference popup groups (`ProjectViewPopupMenu`,
//! `EditorPopupMenu`, `EditorTabPopupMenu`; see `docs/spec/menus.md`).
//!
//! Like the reference IDE's popups, a context menu shows only the actions available
//! for its target: Archipel IDE's implemented actions, in reference order and grouping.
//! The model is computed before the menu is drawn so the drawing code needs no
//! mutable access to the app; the chosen action runs afterwards.

use crate::{IdeApp, keybindings, popups::Dialog};
use archipel_ide_actions::{Catalog, MenuItem, filter_menu};
use archipel_ide_core::project_view::DirMark;
use eframe::egui;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// What a context menu acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A Project view node (project-relative path; empty for the project root).
    Tree {
        path: PathBuf,
        is_dir: bool,
    },
    Editor,
    /// An editor tab by index.
    Tab(usize),
}

impl Target {
    fn group(&self) -> &'static str {
        match self {
            Target::Tree { .. } => "ProjectViewPopupMenu",
            Target::Editor => "EditorPopupMenu",
            Target::Tab(_) => "EditorTabPopupMenu",
        }
    }
}

/// Actions that only make sense for a target (they act on its path or tab).
const TARGET_ACTIONS: [&str; 12] = [
    "RenameElement",
    "$Delete",
    "CopyAbsolutePath",
    "CopyFileName",
    "CopyContentRootPath",
    "ProjectViewEditSource",
    "$Paste",
    "CloseContent",
    "CloseAllEditorsButActive",
    "CloseAllEditors",
    "CloseAllToTheLeft",
    "CloseAllToTheRight",
];

fn applies(action: &str, target: &Target) -> bool {
    match target {
        Target::Tree { path, is_dir } => {
            let root = path.as_os_str().is_empty();
            match action {
                "NewFile"
                | "NewDir"
                | "NewClass"
                | "Kotlin.NewFile"
                | "CopyAbsolutePath"
                | "CopyContentRootPath"
                | "FindInPath"
                | "RevealIn"
                | "Terminal.OpenInTerminal"
                | "SynchronizeCurrentFile" => true,
                "$Paste" => true,
                "RenameElement" | "$Delete" | "CopyFileName" | "$Cut" | "$Copy" | "SafeDelete"
                | "Move" => !root,
                "RenameFile" => !root && !is_dir,
                "ProjectViewEditSource" | "OpenInRightSplit" => !is_dir,
                // Run on a file, the active editor's refactoring commands once
                // they are registered (they stay hidden until then).
                "ReformatCode" | "OptimizeImports" => {
                    !is_dir && keybindings::is_implemented(action)
                }
                _ => false,
            }
        }
        Target::Editor => {
            action == "$Paste"
                || (!TARGET_ACTIONS.contains(&action) && keybindings::is_implemented(action))
        }
        Target::Tab(_) => matches!(
            action,
            "CloseContent"
                | "CloseAllEditorsButActive"
                | "CloseAllEditors"
                | "CloseAllToTheLeft"
                | "CloseAllToTheRight"
                | "CloseAllUnmodifiedEditors"
                | "PinActiveEditorTab"
                | "PinActiveTabToggle"
                | "ReopenClosedTab"
                | "SplitVertically"
                | "SplitHorizontally"
                | "MoveEditorToOppositeTabGroup"
                | "ChangeSplitOrientation"
                | "Unsplit"
                | "UnsplitAll"
                | "KeepTabOpen"
                | "CopyAbsolutePath"
                | "CopyFileName"
                | "CopyContentRootPath"
        ),
    }
}

/// A context menu ready to draw.
pub struct Model {
    items: Vec<MenuItem>,
    enabled: BTreeSet<String>,
    shortcuts: BTreeMap<String, String>,
}

impl IdeApp {
    pub(crate) fn context_menu_model(&self, target: &Target) -> Model {
        let menu = Catalog::shared_menu(target.group());
        let mut items = filter_menu(&menu, &|action| applies(action, target));
        if let Target::Tree { path, is_dir } = target {
            relabel(&mut items, "$Delete", "Delete…");
            relabel(&mut items, "SafeDelete", "Safe Delete…");
            relabel(&mut items, "RenameFile", "Rename File…");
            relabel(&mut items, "FindInPath", "Find in Files…");
            relabel(&mut items, "$Paste", "Paste");
            self.add_tree_entries(&mut items, path, *is_dir);
        }
        if let Target::Tab(index) = target {
            let label = if self.state.is_pinned(*index) {
                "Unpin Tab"
            } else {
                "Pin Tab"
            };
            relabel(&mut items, "PinActiveEditorTab", label);
            relabel(&mut items, "PinActiveTabToggle", label);
        }
        let mut enabled = BTreeSet::new();
        let mut shortcuts = BTreeMap::new();
        collect(&items, &mut |action| {
            if self.context_action_enabled(action, target) {
                enabled.insert(action.to_owned());
            }
            if let Some(shortcut) = self.action_shortcut(action) {
                shortcuts.insert(action.to_owned(), shortcut);
            }
        });
        Model {
            items,
            enabled,
            shortcuts,
        }
    }

    /// Entries that the reference IDE builds from other groups (Run, Git, Mark
    /// Directory as, Local History); only implemented ones are added.
    fn add_tree_entries(&self, items: &mut Vec<MenuItem>, path: &Path, is_dir: bool) {
        let root = path.as_os_str().is_empty();
        let action = |id: &str, label: &str| MenuItem::Action {
            id: id.to_owned(),
            label: label.to_owned(),
        };
        // `Paste ▸ Paste` is a one-entry popup; draw the entry itself.
        flatten_single(items, "PasteGroup");
        let position = |items: &[MenuItem], id: &str| {
            items.iter().position(|item| match item {
                MenuItem::Action { id: found, .. } | MenuItem::Submenu { id: found, .. } => {
                    found == id
                }
                MenuItem::Separator(_) => false,
            })
        };
        let after = |items: &[MenuItem], ids: &[&str]| {
            ids.iter()
                .filter_map(|id| position(items, id))
                .max()
                .map(|index| index + 1)
        };
        // Run 'App' for files that contain an entry point.
        if !is_dir && is_runnable_file(path) {
            let name = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            let at =
                after(items, &["OptimizeImports", "$Delete", "RenameFile"]).unwrap_or(items.len());
            items.splice(
                at..at,
                [
                    MenuItem::Separator(None),
                    action("jt.runFile", &format!("Run '{name}'")),
                ],
            );
        }
        // Local History needs a change journal Archipel IDE does not keep yet.
        let at = position(items, "RevealGroup").map_or(items.len(), |index| index + 1);
        items.insert(at, action("LocalHistory", "Local History"));
        if self.state.has_git() && !root {
            let mut git = vec![action("jt.git.diff", "Show Diff")];
            if !is_dir {
                git.push(action("jt.git.stage", "Add"));
                git.push(action("jt.git.unstage", "Unstage"));
            }
            items.insert(
                at + 1,
                MenuItem::Submenu {
                    id: "VcsFileGroupPopup".into(),
                    label: "Git".into(),
                    items: git,
                },
            );
        }
        if is_dir && !root {
            let mut marks: Vec<MenuItem> = DirMark::ALL
                .iter()
                .map(|mark| action(&format!("jt.mark.{}", mark.id()), mark.label()))
                .collect();
            if self.state.project_view.marks.contains_key(path) {
                marks.push(MenuItem::Separator(None));
                marks.push(action("jt.mark.none", "Unmark"));
            }
            let at = position(items, "SynchronizeCurrentFile").unwrap_or(items.len());
            items.insert(
                at,
                MenuItem::Submenu {
                    id: "MarkRootGroup".into(),
                    label: "Mark Directory as".into(),
                    items: marks,
                },
            );
        }
        if !is_dir && !root && self.state.has_git() {
            let at = items.len();
            items.insert(at, MenuItem::Separator(None));
            items.insert(at + 1, action("jt.git.diff", "Compare With HEAD…"));
        }
        // Drop separators that ended up leading, trailing or doubled.
        tidy_separators(items);
    }

    fn context_action_enabled(&self, action: &str, target: &Target) -> bool {
        let documents = self.state.documents.len();
        match (action, target) {
            ("$Paste", Target::Tree { .. }) => self.state.project_view.clipboard.is_some(),
            ("LocalHistory", _) => false,
            (id, Target::Tree { .. }) if id.starts_with("jt.") => true,
            ("$Paste", Target::Editor) => self.state.active_document.is_some(),
            ("CloseAllEditorsButActive", Target::Tab(_)) => documents > 1,
            ("CloseAllToTheLeft", Target::Tab(index)) => *index > 0,
            ("CloseAllToTheRight", Target::Tab(index)) => index + 1 < documents,
            (
                "CloseAllEditors"
                | "CloseAllUnmodifiedEditors"
                | "PinActiveEditorTab"
                | "PinActiveTabToggle",
                Target::Tab(_),
            ) => true,
            ("ReopenClosedTab", Target::Tab(_)) => !self.state.tabs.closed.is_empty(),
            (_, Target::Tree { .. } | Target::Tab(_)) if TARGET_ACTIONS.contains(&action) => true,
            ("$Cut" | "$Copy" | "SafeDelete" | "Move" | "RenameFile", Target::Tree { .. }) => true,
            (
                "RevealIn" | "Terminal.OpenInTerminal" | "SynchronizeCurrentFile",
                Target::Tree { .. },
            ) => true,
            ("FindInPath", Target::Tree { .. }) => true,
            ("ReformatCode" | "OptimizeImports", Target::Tree { .. }) => true,
            _ => self.menu_action_enabled(action),
        }
    }

    /// Absolute path of a target, if it has one.
    fn target_path(&self, target: &Target) -> Option<(PathBuf, PathBuf)> {
        let root = self.state.project.as_ref()?.root().to_owned();
        let relative = match target {
            Target::Tree { path, .. } => path.clone(),
            Target::Tab(index) => self.state.documents.get(*index)?.path.clone(),
            Target::Editor => {
                let index = self.state.active_document?;
                self.state.documents.get(index)?.path.clone()
            }
        };
        Some((root.join(&relative), relative))
    }

    /// Runs the action chosen in a context menu.
    pub(crate) fn run_context_action(
        &mut self,
        action: &str,
        target: &Target,
        ctx: &egui::Context,
    ) {
        if let Target::Tree { path, .. } = target {
            // Context actions act on (and select) the clicked node.
            self.tree_selection = (!path.as_os_str().is_empty()).then(|| path.clone());
        }
        match action {
            "CopyAbsolutePath" | "CopyFileName" | "CopyContentRootPath" => {
                if let Some((absolute, relative)) = self.target_path(target) {
                    let text = match action {
                        "CopyAbsolutePath" => absolute.display().to_string(),
                        "CopyFileName" => IdeApp::file_name(&relative),
                        _ => relative.display().to_string(),
                    };
                    self.state.status = format!("Copied {text}");
                    ctx.copy_text(text);
                }
            }
            "$Cut" | "$Copy" if matches!(target, Target::Tree { .. }) => {
                if let Target::Tree { path, .. } = target {
                    let command = if action == "$Cut" {
                        "project.cutPaths"
                    } else {
                        "project.copyPaths"
                    };
                    self.dispatch(command, json!({"paths": [path]}));
                }
            }
            "$Paste" if matches!(target, Target::Tree { .. }) => {
                if let Target::Tree { path, is_dir } = target {
                    let dir = if *is_dir {
                        path.clone()
                    } else {
                        path.parent().map(Path::to_path_buf).unwrap_or_default()
                    };
                    self.dispatch("project.paste", json!({"dir": dir}));
                }
            }
            "FindInPath" if matches!(target, Target::Tree { .. }) => {
                if let Target::Tree { path, .. } = target {
                    // Scoped to the clicked directory (or file); the project root is everything.
                    self.state.project_view.search_scope =
                        (!path.as_os_str().is_empty()).then(|| path.clone());
                    self.state.search_scope_kind = if self.state.project_view.search_scope.is_some()
                    {
                        archipel_ide_core::SearchScopeKind::Directory
                    } else {
                        archipel_ide_core::SearchScopeKind::Project
                    };
                    self.run_menu_action(action, ctx);
                }
            }
            "RevealIn"
            | "Terminal.OpenInTerminal"
            | "jt.runFile"
            | "jt.git.diff"
            | "jt.git.stage"
            | "jt.git.unstage" => {
                if let Target::Tree { path, .. } = target {
                    let command = match action {
                        "RevealIn" => "project.revealInFiles",
                        "Terminal.OpenInTerminal" => "project.openInTerminal",
                        "jt.runFile" => "project.runFile",
                        "jt.git.diff" => "git.showDiff",
                        "jt.git.stage" => "git.stage",
                        _ => "git.unstage",
                    };
                    self.dispatch(command, json!({"path": path}));
                }
            }
            "SynchronizeCurrentFile" => self.dispatch("file.synchronize", serde_json::Value::Null),
            "SafeDelete" => {
                if let Target::Tree { path, .. } = target {
                    let usages = self.state.safe_delete_usages(path).unwrap_or_default();
                    self.open_dialog(Dialog::SafeDelete {
                        path: path.clone(),
                        usages,
                    });
                }
            }
            "Move" => {
                if let Target::Tree { path, .. } = target {
                    self.open_dialog(Dialog::Move {
                        path: path.clone(),
                        destination: path
                            .parent()
                            .map(|parent| parent.display().to_string())
                            .unwrap_or_default(),
                    });
                }
            }
            "ReformatCode" | "OptimizeImports" if matches!(target, Target::Tree { .. }) => {
                // Acts on the editor: open the file first, then run the command.
                if let Target::Tree { path, .. } = target {
                    self.dispatch("editor.openFile", json!({"path": path}));
                    self.run_menu_action(action, ctx);
                }
            }
            id if id.starts_with("jt.mark.") => {
                if let Target::Tree { path, .. } = target {
                    let mark = id.trim_start_matches("jt.mark.");
                    self.dispatch("project.markDirectory", json!({"path": path, "mark": mark}));
                }
            }
            "RenameFile" | "RenameElement" => {
                if let Target::Tree { path, .. } = target {
                    self.open_dialog(Dialog::Rename {
                        path: path.clone(),
                        name: IdeApp::file_name(path),
                    });
                }
            }
            "$Delete" => {
                if let Target::Tree { path, .. } = target {
                    self.open_dialog(Dialog::Delete { path: path.clone() });
                }
            }
            "ProjectViewEditSource" => {
                if let Target::Tree { path, .. } = target {
                    self.dispatch("editor.openFile", json!({"path": path}));
                }
            }
            "$Paste" => ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste),
            "CloseContent" => {
                if let Target::Tab(index) = target {
                    self.dispatch("editor.close", json!({"index": index}));
                }
            }
            "CloseAllEditors"
            | "CloseAllEditorsButActive"
            | "CloseAllUnmodifiedEditors"
            | "CloseAllToTheLeft"
            | "CloseAllToTheRight"
            | "PinActiveEditorTab"
            | "PinActiveTabToggle" => {
                if let Target::Tab(index) = target {
                    // The tab commands skip pinned tabs, like the reference IDE.
                    let command = match action {
                        "CloseAllEditors" => "editor.closeAll",
                        "CloseAllEditorsButActive" => "editor.closeOthers",
                        "CloseAllUnmodifiedEditors" => "editor.closeUnmodified",
                        "CloseAllToTheLeft" => "editor.closeLeft",
                        "CloseAllToTheRight" => "editor.closeRight",
                        _ => "editor.togglePin",
                    };
                    self.dispatch(command, json!({"index": index}));
                }
            }
            "ReopenClosedTab" => self.dispatch("editor.reopenClosed", serde_json::Value::Null),
            "SplitVertically"
            | "SplitHorizontally"
            | "MoveEditorToOppositeTabGroup"
            | "KeepTabOpen"
                if matches!(target, Target::Tab(_)) =>
            {
                if let Target::Tab(index) = target {
                    let command = match action {
                        "SplitVertically" => "editor.splitRight",
                        "SplitHorizontally" => "editor.splitDown",
                        "KeepTabOpen" => "editor.keepTabOpen",
                        _ => "editor.moveToOppositeGroup",
                    };
                    self.dispatch(command, json!({"index": index}));
                }
            }
            "OpenInRightSplit" => {
                if let Target::Tree { path, .. } = target {
                    self.dispatch("editor.openInRightSplit", json!({"path": path}));
                }
            }
            _ => self.run_menu_action(action, ctx),
        }
    }
}

/// Replaces the submenu `id` by its only entry.
fn flatten_single(items: &mut [MenuItem], id: &str) {
    for item in items.iter_mut() {
        if let MenuItem::Submenu {
            id: found,
            items: children,
            ..
        } = item
            && found == id
            && children.len() == 1
            && let Some(only) = children.pop()
        {
            *item = only;
        }
    }
}

/// Removes leading, trailing and repeated separators.
fn tidy_separators(items: &mut Vec<MenuItem>) {
    let mut previous_separator = true;
    items.retain(|item| {
        let separator = matches!(item, MenuItem::Separator(_));
        let keep = !(separator && previous_separator);
        previous_separator = separator;
        keep
    });
    while matches!(items.last(), Some(MenuItem::Separator(_))) {
        items.pop();
    }
}

/// Files whose entry points the gutter run icons can find.
fn is_runnable_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("java" | "kt" | "rs" | "go" | "py")
    )
}

/// Some labels are set at runtime per place (the Project view shows "Delete…").
fn relabel(items: &mut [MenuItem], action: &str, text: &str) {
    for item in items {
        match item {
            MenuItem::Action { id, label } if id == action => *label = text.to_owned(),
            MenuItem::Submenu { items, .. } => relabel(items, action, text),
            _ => {}
        }
    }
}

fn collect(items: &[MenuItem], visit: &mut impl FnMut(&str)) {
    for item in items {
        match item {
            MenuItem::Action { id, .. } => visit(id),
            MenuItem::Submenu { items, .. } => collect(items, visit),
            MenuItem::Separator(_) => {}
        }
    }
}

/// Draws `model` into an open context menu; sets `chosen` when an item is clicked.
pub fn show(ui: &mut egui::Ui, model: &Model, chosen: &mut Option<String>) {
    ui.set_min_width(240.0);
    render(ui, &model.items, model, chosen);
}

fn render(ui: &mut egui::Ui, items: &[MenuItem], model: &Model, chosen: &mut Option<String>) {
    for item in items {
        match item {
            MenuItem::Action { id, label } => {
                let mut item = archipel_kit::action_system::ActionMenuItem::new(label)
                    .enabled(model.enabled.contains(id));
                if let Some(shortcut) = model.shortcuts.get(id).filter(|s| !s.is_empty()) {
                    item = item.shortcut(shortcut);
                }
                if ui.add(item).clicked() {
                    *chosen = Some(id.clone());
                    ui.close();
                }
            }
            MenuItem::Submenu { label, items, .. } => {
                // The kit has no submenu item yet: a kit `MenuItem` with an arrow, opened
                // by egui's submenu logic, so it lines up with the actions around it.
                let entry =
                    ui.add(archipel_kit::action_system::ActionMenuItem::new(label).shortcut("›"));
                egui::containers::menu::SubMenu::new().show(ui, &entry, |ui| {
                    ui.set_min_width(200.0);
                    render(ui, items, model, chosen);
                });
            }
            MenuItem::Separator(_) => {
                ui.add(archipel_kit::ui::popup::ListSeparator::new());
            }
        }
    }
}

/// Labels of the top-level entries (tests and evidence).
#[cfg(test)]
fn labels(items: &[MenuItem]) -> Vec<String> {
    items
        .iter()
        .map(|item| match item {
            MenuItem::Action { label, .. } => label.clone(),
            MenuItem::Submenu { label, .. } => format!("{label} ▸"),
            MenuItem::Separator(_) => "---".into(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(target: Target) -> Vec<String> {
        let menu = Catalog::shared().menu(target.group());
        let mut items = filter_menu(&menu, &|action| applies(action, &target));
        if matches!(target, Target::Tree { .. }) {
            relabel(&mut items, "$Delete", "Delete…");
        }
        labels(&items)
    }

    #[test]
    fn project_view_menu_follows_the_reference_order() {
        let file = model(Target::Tree {
            path: "src/main.rs".into(),
            is_dir: false,
        });
        assert_eq!(file[0], "New ▸");
        let position = |label: &str| file.iter().position(|l| l == label).unwrap();
        assert!(position("Copy Path/Reference… ▸") < position("Rename…"));
        assert!(position("Rename…") < position("Delete…"));
        assert!(file.contains(&"Edit Source".to_owned()));
        // The root has no Rename/Delete/File Name.
        let root = model(Target::Tree {
            path: PathBuf::new(),
            is_dir: true,
        });
        assert!(!root.contains(&"Rename…".to_owned()));
        assert!(!root.contains(&"Delete…".to_owned()));
        assert!(!root.contains(&"Edit Source".to_owned()));
    }

    #[test]
    fn new_submenu_offers_java_kotlin_file_and_directory() {
        let menu = Catalog::shared().menu("ProjectViewPopupMenu");
        let items = filter_menu(&menu, &|action| {
            applies(
                action,
                &Target::Tree {
                    path: "src/main/java".into(),
                    is_dir: true,
                },
            )
        });
        let Some(MenuItem::Submenu { items: new, .. }) = items.first() else {
            panic!("New submenu first");
        };
        let new = labels(new);
        for label in [
            "Java Class",
            "Kotlin Class/File",
            "File",
            "Directory/Package",
        ] {
            assert!(new.contains(&label.to_owned()), "{label} in {new:?}");
        }
        assert!(keybindings::is_implemented("NewClass"));
    }

    #[test]
    fn editor_and_tab_menus_list_implemented_actions() {
        let editor = model(Target::Editor);
        for label in ["Cut", "Copy", "Paste"] {
            assert!(editor.contains(&label.to_owned()), "{label} in {editor:?}");
        }
        let tab = model(Target::Tab(0));
        assert_eq!(
            &tab[..3],
            ["Close Tab", "Close Other Tabs", "Close All Tabs"]
        );
        assert!(tab.contains(&"Close Tabs to the Left".to_owned()));
    }

    #[test]
    fn project_menu_offers_file_operations_and_hides_unimplemented_entries() {
        let file = model(Target::Tree {
            path: "src/App.java".into(),
            is_dir: false,
        });
        for label in ["Cut", "Copy", "Paste ▸", "Find in Files…"] {
            assert!(file.contains(&label.to_owned()), "{label} in {file:?}");
        }
        assert!(file.contains(&"Refactor ▸".to_owned()), "{file:?}");
        assert!(file.contains(&"Open In ▸".to_owned()), "{file:?}");
        // Unimplemented entries are hidden, not dead; Reformat/Optimize Imports
        // follow the keymap's command table (files only).
        for hidden in ["Find Usages", "Analyze ▸"] {
            assert!(!file.contains(&hidden.to_owned()), "{hidden} hidden");
        }
        for shown in ["Reformat Code", "Optimize Imports"] {
            assert_eq!(
                file.contains(&shown.to_owned()),
                keybindings::is_implemented(if shown == "Reformat Code" {
                    "ReformatCode"
                } else {
                    "OptimizeImports"
                }),
                "{shown}"
            );
        }
        let dir = model(Target::Tree {
            path: "src".into(),
            is_dir: true,
        });
        assert!(!dir.contains(&"Rename File…".to_owned()));
        let root = model(Target::Tree {
            path: PathBuf::new(),
            is_dir: true,
        });
        assert!(!root.contains(&"Cut".to_owned()));
        assert!(root.contains(&"Paste ▸".to_owned()));
    }

    #[test]
    fn extra_entries_are_runtime_built() {
        let (app, _ctx) = crate::tests::headless_app("tree-menu", "App.java", "class App {}\n");
        let menu = |path: &str, is_dir: bool| {
            let target = Target::Tree {
                path: path.into(),
                is_dir,
            };
            let model = app.context_menu_model(&target);
            (labels(&model.items), model)
        };
        let (dir, model) = menu("src", true);
        assert!(dir.contains(&"Mark Directory as ▸".to_owned()), "{dir:?}");
        assert!(dir.contains(&"Local History".to_owned()));
        assert!(
            !model.enabled.contains("LocalHistory"),
            "disabled, not hidden"
        );
        assert!(!model.enabled.contains("$Paste"), "nothing copied yet");
        let (file, _) = menu("App.java", false);
        assert!(file.contains(&"Run 'App'".to_owned()), "{file:?}");
        assert!(!file.contains(&"Mark Directory as ▸".to_owned()));
        // No separators at the edges or doubled.
        for items in [&dir, &file] {
            assert_ne!(items.first().map(String::as_str), Some("---"));
            assert_ne!(items.last().map(String::as_str), Some("---"));
            assert!(!items.windows(2).any(|w| w[0] == "---" && w[1] == "---"));
        }
    }
}

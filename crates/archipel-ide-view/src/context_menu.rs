//! Context menus built from the reference popup groups (`ProjectViewPopupMenu`,
//! `EditorPopupMenu`, `EditorTabPopupMenu`): only the actions available for the target, in
//! reference order and grouping (TUI-GUI-043).

use crate::actions::{CONTEXT, ContextFacts, ContextKind, is_implemented, is_target_action};
use crate::cx::ViewCx;
use crate::dispatch::{Effect, Request};
use archipel_ide_actions::{ActionPlace, Catalog, MenuItem as RefItem, filter_menu};
use archipel_ide_core::project_view::DirMark;
use archipel_kit_model::{Glyph, MenuEntry, MenuItem};
use serde_json::json;
use std::path::{Path, PathBuf};

/// What a context menu acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A Project view node (project-relative path; empty for the project root).
    Tree {
        /// The node.
        path: PathBuf,
        /// Whether it is a directory.
        is_dir: bool,
    },
    /// The editor.
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

/// Whether `action` is offered for `target` at all.
pub fn applies(action: &str, target: &Target) -> bool {
    match target {
        Target::Tree { path, is_dir } => {
            let root = path.as_os_str().is_empty();
            match action {
                "NewFile" | "NewDir" | "NewClass" | "Kotlin.NewFile" | "CopyAbsolutePath"
                | "CopyContentRootPath" | "FindInPath" | "RevealIn" | "Terminal.OpenInTerminal"
                | "SynchronizeCurrentFile" => true,
                "$Paste" => true,
                "RenameElement" | "$Delete" | "CopyFileName" | "$Cut" | "$Copy" | "SafeDelete"
                | "Move" => !root,
                "RenameFile" => !root && !is_dir,
                "ProjectViewEditSource" | "OpenInRightSplit" => !is_dir,
                "ReformatCode" | "OptimizeImports" => !is_dir && is_implemented(action),
                _ => false,
            }
        }
        Target::Editor => action == "$Paste" || (!is_target_action(action) && is_implemented(action)),
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

/// The facts the context menu rules need.
pub fn context_facts(cx: &ViewCx, target: &Target) -> ContextFacts {
    ContextFacts {
        kind: match target {
            Target::Tree { .. } => ContextKind::Tree,
            Target::Editor => ContextKind::Editor,
            Target::Tab(index) => ContextKind::Tab(*index),
        },
        open_tabs: cx.state.open_editor_count(),
        has_closed_tabs: cx.state.has_closed_tabs(),
        has_clipboard: cx.state.project_view_clipboard_present(),
        has_document: cx.state.active_file().is_some(),
    }
}

/// Whether the context menu entry `action` is enabled for `target`.
pub fn context_action_enabled(cx: &ViewCx, action: &str, target: &Target) -> bool {
    let facts = context_facts(cx, target);
    cx.with_data(
        |data| data.with_value(&CONTEXT, facts),
        |registry, data| registry.update(action, ActionPlace::ContextMenu, data),
    )
    .is_some_and(|presentation| presentation.enabled)
}

fn icon_of(id: &str) -> Option<Glyph> {
    Some(match id {
        "$Copy" => Glyph::Copy,
        "$Paste" => Glyph::Paste,
        "$Delete" | "SafeDelete" => Glyph::Delete,
        "FindInPath" => Glyph::FindInFiles,
        "RenameElement" => Glyph::Edit,
        "Run" | "RunClass" => Glyph::Run,
        "Debug" | "DebugClass" => Glyph::Debug,
        "Synchronize" => Glyph::Refresh,
        _ => return None,
    })
}

/// Replaces the submenu `id` by its only entry.
fn flatten_single(items: &mut [RefItem], id: &str) {
    for item in items.iter_mut() {
        if let RefItem::Submenu {
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
fn tidy_separators(items: &mut Vec<RefItem>) {
    let mut previous_separator = true;
    items.retain(|item| {
        let separator = matches!(item, RefItem::Separator(_));
        let keep = !(separator && previous_separator);
        previous_separator = separator;
        keep
    });
    while matches!(items.last(), Some(RefItem::Separator(_))) {
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

fn relabel(items: &mut [RefItem], action: &str, text: &str) {
    for item in items {
        match item {
            RefItem::Action { id, label } if id == action => *label = text.to_owned(),
            RefItem::Submenu { items, .. } => relabel(items, action, text),
            _ => {}
        }
    }
}

/// The reference items of the context menu of `target`, after the runtime additions.
pub fn context_items(cx: &ViewCx, target: &Target) -> Vec<RefItem> {
    let menu = Catalog::shared_menu(target.group());
    let mut items = filter_menu(&menu, &|action| applies(action, target));
    if let Target::Tree { path, is_dir } = target {
        relabel(&mut items, "$Delete", "Delete…");
        relabel(&mut items, "SafeDelete", "Safe Delete…");
        relabel(&mut items, "RenameFile", "Rename File…");
        relabel(&mut items, "FindInPath", "Find in Files…");
        relabel(&mut items, "$Paste", "Paste");
        add_tree_entries(cx, &mut items, path, *is_dir);
    }
    if let Target::Tab(index) = target {
        let label = if cx.state.is_pinned(*index) { "Unpin Tab" } else { "Pin Tab" };
        relabel(&mut items, "PinActiveEditorTab", label);
        relabel(&mut items, "PinActiveTabToggle", label);
    }
    items
}

/// Entries the reference IDE builds from other groups (Run, Git, Mark Directory as, Local
/// History); only implemented ones are added.
fn add_tree_entries(cx: &ViewCx, items: &mut Vec<RefItem>, path: &Path, is_dir: bool) {
    let root = path.as_os_str().is_empty();
    let action = |id: &str, label: &str| RefItem::Action {
        id: id.to_owned(),
        label: label.to_owned(),
    };
    flatten_single(items, "PasteGroup");
    let position = |items: &[RefItem], id: &str| {
        items.iter().position(|item| match item {
            RefItem::Action { id: found, .. } | RefItem::Submenu { id: found, .. } => found == id,
            RefItem::Separator(_) => false,
        })
    };
    let after = |items: &[RefItem], ids: &[&str]| {
        ids.iter()
            .filter_map(|id| position(items, id))
            .max()
            .map(|index| index + 1)
    };
    if !is_dir && is_runnable_file(path) {
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let at = after(items, &["OptimizeImports", "$Delete", "RenameFile"]).unwrap_or(items.len());
        items.splice(
            at..at,
            [RefItem::Separator(None), action("jt.runFile", &format!("Run '{name}'"))],
        );
    }
    let at = position(items, "RevealGroup").map_or(items.len(), |index| index + 1);
    items.insert(at, action("LocalHistory", "Local History"));
    if cx.state.has_git() && !root {
        let mut git = vec![action("jt.git.diff", "Show Diff")];
        if !is_dir {
            git.push(action("jt.git.stage", "Add"));
            git.push(action("jt.git.unstage", "Unstage"));
        }
        items.insert(
            at + 1,
            RefItem::Submenu {
                id: "VcsFileGroupPopup".into(),
                label: "Git".into(),
                items: git,
            },
        );
    }
    if is_dir && !root {
        let mut marks: Vec<RefItem> = DirMark::ALL
            .iter()
            .map(|mark| action(&format!("jt.mark.{}", mark.id()), mark.label()))
            .collect();
        if cx.state.project_view().marks.contains_key(path) {
            marks.push(RefItem::Separator(None));
            marks.push(action("jt.mark.none", "Unmark"));
        }
        let at = position(items, "SynchronizeCurrentFile").unwrap_or(items.len());
        items.insert(
            at,
            RefItem::Submenu {
                id: "MarkRootGroup".into(),
                label: "Mark Directory as".into(),
                items: marks,
            },
        );
    }
    if !is_dir && !root && cx.state.has_git() {
        let at = items.len();
        items.insert(at, RefItem::Separator(None));
        items.insert(at + 1, action("jt.git.diff", "Compare With HEAD…"));
    }
    tidy_separators(items);
}

fn convert(cx: &ViewCx, target: &Target, items: &[RefItem], out: &mut Vec<MenuEntry>) {
    for item in items {
        match item {
            RefItem::Action { id, label } => {
                let mut entry = MenuItem::new(id.clone(), label.clone());
                entry.enabled = context_action_enabled(cx, id, target);
                entry.shortcut = cx.shortcut(id);
                entry.icon = icon_of(id).map(Into::into);
                out.push(MenuEntry::Item(entry));
            }
            RefItem::Submenu { id, label, items } => {
                let mut children = Vec::new();
                convert(cx, target, items, &mut children);
                out.push(MenuEntry::Item(MenuItem::submenu(id.clone(), label.clone(), children)));
            }
            RefItem::Separator(title) => out.push(MenuEntry::Separator {
                title: title.clone(),
            }),
        }
    }
}

/// The context menu of `target` as model entries.
pub fn context_menu(cx: &ViewCx, target: &Target) -> Vec<MenuEntry> {
    let items = context_items(cx, target);
    let mut out = Vec::new();
    convert(cx, target, &items, &mut out);
    out
}

/// What choosing `action` in the context menu of `target` asks for.
pub fn context_action(cx: &ViewCx, action: &str, target: &Target) -> Vec<Effect> {
    let mut effects = Vec::new();
    if let Target::Tree { path, .. } = target {
        // Context actions act on (and select) the clicked node.
        effects.push(Effect::SelectTreeNode(
            (!path.as_os_str().is_empty()).then(|| path.clone()),
        ));
    }
    let command = |id: &str, params: serde_json::Value| Effect::Command {
        id: id.to_owned(),
        params,
    };
    let path_of = |target: &Target| -> Option<(PathBuf, PathBuf)> {
        let root = cx.state.project_root()?;
        let relative = match target {
            Target::Tree { path, .. } => path.clone(),
            Target::Tab(index) => cx.state.document(*index)?.path.clone(),
            Target::Editor => cx.state.document(cx.state.active_index()?)?.path.clone(),
        };
        Some((root.join(&relative), relative))
    };
    match action {
        "CopyAbsolutePath" | "CopyFileName" | "CopyContentRootPath" => {
            if let Some((absolute, relative)) = path_of(target) {
                let text = match action {
                    "CopyAbsolutePath" => absolute.display().to_string(),
                    "CopyFileName" => relative
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    _ => relative.display().to_string(),
                };
                effects.push(Effect::Status(format!("Copied {text}")));
                effects.push(Effect::CopyText(text));
            }
        }
        "$Cut" | "$Copy" if matches!(target, Target::Tree { .. }) => {
            if let Target::Tree { path, .. } = target {
                let id = if action == "$Cut" { "project.cutPaths" } else { "project.copyPaths" };
                effects.push(command(id, json!({"paths": [path]})));
            }
        }
        "$Paste" if matches!(target, Target::Tree { .. }) => {
            if let Target::Tree { path, is_dir } = target {
                let dir = if *is_dir {
                    path.clone()
                } else {
                    path.parent().map(Path::to_path_buf).unwrap_or_default()
                };
                effects.push(command("project.paste", json!({"dir": dir})));
            }
        }
        "$Paste" => effects.push(Effect::Request(Request::Paste)),
        "FindInPath" if matches!(target, Target::Tree { .. }) => {
            if let Target::Tree { path, .. } = target {
                effects.push(Effect::Request(Request::FindInPathIn(path.clone())));
            }
        }
        "RevealIn" | "Terminal.OpenInTerminal" | "jt.runFile" | "jt.git.diff" | "jt.git.stage"
        | "jt.git.unstage" => {
            if let Target::Tree { path, .. } = target {
                let id = match action {
                    "RevealIn" => "project.revealInFiles",
                    "Terminal.OpenInTerminal" => "project.openInTerminal",
                    "jt.runFile" => "project.runFile",
                    "jt.git.diff" => "git.showDiff",
                    "jt.git.stage" => "git.stage",
                    _ => "git.unstage",
                };
                effects.push(command(id, json!({"path": path})));
            }
        }
        "SynchronizeCurrentFile" => effects.push(command("file.synchronize", serde_json::Value::Null)),
        "SafeDelete" | "Move" | "RenameFile" | "RenameElement" | "$Delete" => {
            if let Target::Tree { path, .. } = target {
                let path = path.clone();
                effects.push(Effect::Request(match action {
                    "SafeDelete" => Request::SafeDelete(path),
                    "Move" => Request::Move(path),
                    "$Delete" => Request::Delete(path),
                    _ => Request::Rename(path),
                }));
            }
        }
        "ReformatCode" | "OptimizeImports" if matches!(target, Target::Tree { .. }) => {
            if let Target::Tree { path, .. } = target {
                effects.push(command("editor.openFile", json!({"path": path})));
                effects.push(Effect::Action {
                    id: action.to_owned(),
                    place: ActionPlace::ContextMenu,
                });
            }
        }
        id if id.starts_with("jt.mark.") => {
            if let Target::Tree { path, .. } = target {
                let mark = id.trim_start_matches("jt.mark.");
                effects.push(command("project.markDirectory", json!({"path": path, "mark": mark})));
            }
        }
        "ProjectViewEditSource" => {
            if let Target::Tree { path, .. } = target {
                effects.push(command("editor.openFile", json!({"path": path})));
            }
        }
        "CloseContent" => {
            if let Target::Tab(index) = target {
                effects.push(command("editor.close", json!({"index": index})));
            }
        }
        "CloseAllEditors" | "CloseAllEditorsButActive" | "CloseAllUnmodifiedEditors"
        | "CloseAllToTheLeft" | "CloseAllToTheRight" | "PinActiveEditorTab" | "PinActiveTabToggle" => {
            if let Target::Tab(index) = target {
                let id = match action {
                    "CloseAllEditors" => "editor.closeAll",
                    "CloseAllEditorsButActive" => "editor.closeOthers",
                    "CloseAllUnmodifiedEditors" => "editor.closeUnmodified",
                    "CloseAllToTheLeft" => "editor.closeLeft",
                    "CloseAllToTheRight" => "editor.closeRight",
                    _ => "editor.togglePin",
                };
                effects.push(command(id, json!({"index": index})));
            }
        }
        "ReopenClosedTab" => effects.push(command("editor.reopenClosed", serde_json::Value::Null)),
        "SplitVertically" | "SplitHorizontally" | "MoveEditorToOppositeTabGroup" | "KeepTabOpen"
            if matches!(target, Target::Tab(_)) =>
        {
            if let Target::Tab(index) = target {
                let id = match action {
                    "SplitVertically" => "editor.splitRight",
                    "SplitHorizontally" => "editor.splitDown",
                    "KeepTabOpen" => "editor.keepTabOpen",
                    _ => "editor.moveToOppositeGroup",
                };
                effects.push(command(id, json!({"index": index})));
            }
        }
        "OpenInRightSplit" => {
            if let Target::Tree { path, .. } = target {
                effects.push(command("editor.openInRightSplit", json!({"path": path})));
            }
        }
        _ => effects.push(Effect::Action {
            id: action.to_owned(),
            place: ActionPlace::ContextMenu,
        }),
    }
    effects
}

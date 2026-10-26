//! Catalog of the reference IDE's actions and action groups.
//!
//! Loaded from `docs/spec/data/actions.json` (Apache-2.0 derived data, see
//! `docs/spec/data/NOTICE.md`): 3,451 action and group ids with their menu texts,
//! and the children of each group. The keymap page uses it to name actions; the
//! main menu and context menus are built from its groups.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::LazyLock;

const ACTIONS_JSON: &str = include_str!("../../../docs/spec/data/actions.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Action,
    Group,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Action {
    pub id: String,
    pub kind: Kind,
    pub text: Option<String>,
    pub description: Option<String>,
    /// Groups shown as a submenu (instead of inline).
    pub popup: Option<bool>,
    /// Hidden in a normal IDE installation.
    pub internal: bool,
}

impl Action {
    /// Menu text without mnemonic markers (`_File` → `File`).
    pub fn label(&self) -> String {
        clean_label(self.text.as_deref().unwrap_or(&self.id))
    }
}

/// Removes mnemonic markers (`_` and `&`) and trailing whitespace from a menu text.
pub fn clean_label(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            // `__` and `&&` are literal characters.
            '_' | '&' if chars.peek() == Some(&c) => {
                out.push(c);
                chars.next();
            }
            '_' | '&' => {}
            other => out.push(other),
        }
    }
    out.trim().to_owned()
}

/// Labels the reference IDE sets at runtime instead of in its action XML.
const RUNTIME_LABELS: &[(&str, &str)] = &[
    ("ShowSettings", "Settings…"),
    ("TemplateProjectProperties", "Settings for New Projects…"),
    (
        "RunConfigurationTemplatesForNewProjects",
        "Run Configuration Templates for New Projects…",
    ),
    ("Exit", "Exit"),
    ("Back", "Back"),
    ("Forward", "Forward"),
    ("RunInspection", "Run Inspection by Name…"),
    ("XDebugger.AttachToProcess", "Attach to Process…"),
    ("ManageTargets", "Manage Targets…"),
    ("Stop", "Stop"),
    ("tasks.goto", "Open Task…"),
    ("Annotate", "Annotate with Git Blame"),
    ("MaximizeToolWindow", "Maximize Tool Window"),
    ("MaximizeEditorInSplit", "Maximize Editor in Split"),
    ("Unsplit", "Unsplit"),
    ("About", "About"),
    ("ConvertToWindowsLineSeparators", "CRLF - Windows (\\r\\n)"),
    ("ConvertToUnixLineSeparators", "LF - Unix and macOS (\\n)"),
    ("ConvertToMacLineSeparators", "CR - Classic Mac OS (\\r)"),
];

/// `ProblemsView` → `Problems View`.
fn split_camel_case(name: &str) -> String {
    let mut out = String::new();
    let mut previous_lower = false;
    for c in name.chars() {
        if c.is_uppercase() && previous_lower {
            out.push(' ');
        }
        previous_lower = c.is_lowercase();
        out.push(c);
    }
    out
}

/// One entry of a group.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Child {
    Ref {
        r#ref: String,
    },
    Separator {
        separator: bool,
        text: Option<String>,
    },
}

#[derive(Deserialize)]
struct ActionsFile {
    actions: Vec<Action>,
    groups: BTreeMap<String, Vec<Child>>,
}

#[derive(Debug, Clone, Default)]
pub struct Catalog {
    actions: BTreeMap<String, Action>,
    groups: BTreeMap<String, Vec<Child>>,
}

impl Catalog {
    pub fn bundled() -> Result<Self, String> {
        let file: ActionsFile =
            serde_json::from_str(ACTIONS_JSON).map_err(|error| error.to_string())?;
        Ok(Self {
            actions: file
                .actions
                .into_iter()
                .map(|action| (action.id.clone(), action))
                .collect(),
            groups: file.groups,
        })
    }

    /// Shared parsed copy (an empty catalog if the bundled data were invalid; a test
    /// guards against that).
    pub fn shared() -> &'static Catalog {
        static CATALOG: LazyLock<Catalog> =
            LazyLock::new(|| Catalog::bundled().unwrap_or_default());
        &CATALOG
    }

    pub fn get(&self, id: &str) -> Option<&Action> {
        self.actions.get(id)
    }

    /// Display name of an action id: its menu text, a label the reference IDE computes
    /// at runtime (same table as `docs/spec/tools/extract_intellij_data.py`), the
    /// tool window name for `Activate…ToolWindow`, else the id.
    pub fn label(&self, id: &str) -> String {
        if let Some(text) = self.actions.get(id).and_then(|a| a.text.as_deref()) {
            return clean_label(text);
        }
        if let Some((_, label)) = RUNTIME_LABELS.iter().find(|(action, _)| *action == id) {
            return (*label).to_owned();
        }
        if let Some(name) = id
            .strip_prefix("Activate")
            .and_then(|rest| rest.strip_suffix("ToolWindow"))
        {
            return format!("{} (tool window)", split_camel_case(name));
        }
        // No menu text: a title-cased label, never the raw id.
        let tail = id.rsplit('.').next().unwrap_or(id);
        let label = split_camel_case(tail.trim_start_matches('$'));
        if label.is_empty() {
            id.to_owned()
        } else {
            label
        }
    }

    pub fn actions(&self) -> impl Iterator<Item = &Action> {
        self.actions.values()
    }

    pub fn children(&self, group: &str) -> &[Child] {
        let group = RUNTIME_GROUPS
            .iter()
            .find(|(id, _)| *id == group)
            .map_or(group, |(_, source)| source);
        self.groups.get(group).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Menu structure of `group` as the reference IDE renders it: popup groups become
    /// submenus, other groups are inlined, internal entries are hidden, and separators
    /// are collapsed (no leading, trailing or repeated separators, no empty submenus).
    pub fn menu(&self, group: &str) -> Vec<MenuItem> {
        let mut stack = vec![group.to_owned()];
        self.expand(group, &mut stack)
    }

    /// [`Catalog::menu`] of the shared catalog, computed once per group (the editor
    /// context menu is rebuilt every frame).
    pub fn shared_menu(group: &str) -> std::sync::Arc<Vec<MenuItem>> {
        type Cache = std::sync::Mutex<BTreeMap<String, std::sync::Arc<Vec<MenuItem>>>>;
        static MENUS: LazyLock<Cache> = LazyLock::new(Default::default);
        let build = || std::sync::Arc::new(Catalog::shared().menu(group));
        match MENUS.lock() {
            Ok(mut menus) => menus.entry(group.to_owned()).or_insert_with(build).clone(),
            Err(_) => build(),
        }
    }

    fn expand(&self, group: &str, stack: &mut Vec<String>) -> Vec<MenuItem> {
        let mut items = Vec::new();
        for child in self.children(group) {
            match child {
                Child::Separator { text, .. } => items.push(MenuItem::Separator(text.clone())),
                Child::Ref { r#ref } => {
                    // Guard against cycles in the group graph.
                    if stack.contains(r#ref) || stack.len() > 12 {
                        continue;
                    }
                    let action = self.actions.get(r#ref);
                    if action.is_some_and(|a| a.internal)
                        || INTERNAL_GROUPS.contains(&r#ref.as_str())
                    {
                        continue;
                    }
                    let is_group = action.map(|a| a.kind == Kind::Group).unwrap_or(false)
                        || self.groups.contains_key(r#ref);
                    if is_group {
                        stack.push(r#ref.clone());
                        let children = self.expand(r#ref, stack);
                        stack.pop();
                        let popup = action.and_then(|a| a.popup).unwrap_or(false)
                            || RUNTIME_POPUPS.contains(&r#ref.as_str());
                        if popup {
                            if children
                                .iter()
                                .any(|item| !matches!(item, MenuItem::Separator(_)))
                            {
                                items.push(MenuItem::Submenu {
                                    id: r#ref.clone(),
                                    label: self.label(r#ref),
                                    items: children,
                                });
                            }
                        } else {
                            items.extend(children);
                        }
                    } else {
                        items.push(MenuItem::Action {
                            id: r#ref.clone(),
                            label: self.label(r#ref),
                        });
                    }
                }
            }
        }
        collapse_separators(items)
    }
}

/// Groups whose children the reference IDE fills at runtime from another group.
const RUNTIME_GROUPS: [(&str, &str); 2] = [
    ("NewElementInMenuGroup", "NewGroup"),
    ("WeighingNewGroup", "NewGroup"),
];

/// Groups the spec marks inline that the reference IDE shows as submenus
/// (`reference/*/07-main-menu.png`: File > New Projects Setup).
const RUNTIME_POPUPS: [&str; 1] = ["FileOtherSettingsGroup"];

/// Groups only visible in the reference IDE's internal mode.
const INTERNAL_GROUPS: [&str; 5] = [
    "Internal",
    "InternalActions",
    "Internal.UI",
    "Internal.Errors",
    "UiDebugger",
];

/// One entry of a rendered menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItem {
    Action {
        id: String,
        label: String,
    },
    Submenu {
        id: String,
        label: String,
        items: Vec<MenuItem>,
    },
    /// A separator, optionally titled.
    Separator(Option<String>),
}

/// Keeps the actions for which `keep` returns true, dropping submenus that end up
/// empty and collapsing separators (context menus hide unavailable actions).
pub fn filter_menu(items: &[MenuItem], keep: &impl Fn(&str) -> bool) -> Vec<MenuItem> {
    let mut seen = std::collections::BTreeSet::new();
    filter_inner(items, keep, &mut seen)
}

fn filter_inner(
    items: &[MenuItem],
    keep: &impl Fn(&str) -> bool,
    seen: &mut std::collections::BTreeSet<String>,
) -> Vec<MenuItem> {
    let mut out = Vec::new();
    for item in items {
        match item {
            // An action listed twice in one menu (inline and in a submenu) is shown once.
            MenuItem::Action { id, .. } if keep(id) && seen.insert(id.clone()) => {
                out.push(item.clone())
            }
            MenuItem::Action { .. } => {}
            MenuItem::Submenu { id, label, items } => {
                let items = filter_inner(items, keep, seen);
                if items
                    .iter()
                    .any(|item| !matches!(item, MenuItem::Separator(_)))
                {
                    out.push(MenuItem::Submenu {
                        id: id.clone(),
                        label: label.clone(),
                        items,
                    });
                }
            }
            MenuItem::Separator(_) => out.push(item.clone()),
        }
    }
    collapse_separators(out)
}

fn collapse_separators(items: Vec<MenuItem>) -> Vec<MenuItem> {
    let mut out: Vec<MenuItem> = Vec::with_capacity(items.len());
    for item in items {
        if matches!(item, MenuItem::Separator(_))
            && out
                .last()
                .is_none_or(|last| matches!(last, MenuItem::Separator(_)))
        {
            continue;
        }
        out.push(item);
    }
    while matches!(out.last(), Some(MenuItem::Separator(_))) {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalog_loads() {
        let catalog = Catalog::bundled().unwrap();
        assert_eq!(catalog.actions().count(), 3451);
        assert_eq!(catalog.label("GotoAction"), "Find Action…");
        assert_eq!(catalog.label("ShowSettings"), "Settings…");
        assert_eq!(
            catalog.label("ActivateProblemsViewToolWindow"),
            "Problems View (tool window)"
        );
        assert_eq!(catalog.label("NoSuchAction"), "No Such Action");
        assert_eq!(catalog.get("FileMenu").unwrap().kind, Kind::Group);
        let main: Vec<_> = catalog
            .children("MainMenu")
            .iter()
            .filter_map(|child| match child {
                Child::Ref { r#ref } => Some(r#ref.as_str()),
                Child::Separator { .. } => None,
            })
            .collect();
        assert_eq!(main.first(), Some(&"FileMenu"));
        assert!(main.contains(&"CodeMenu"));
    }

    #[test]
    fn main_menu_follows_the_spec() {
        let catalog = Catalog::bundled().unwrap();
        let menu = catalog.menu("MainMenu");
        let titles: Vec<&str> = menu
            .iter()
            .filter_map(|item| match item {
                MenuItem::Submenu { label, .. } => Some(label.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            titles,
            [
                "File", "Edit", "View", "Navigate", "Code", "Refactor", "Build", "Run", "Tools",
                "Git", "Window", "Help"
            ]
        );
        let MenuItem::Submenu { items: file, .. } = &menu[0] else {
            panic!("File is a submenu");
        };
        // Inlined FileOpenGroup entries and a popup submenu.
        assert!(
            file.iter()
                .any(|i| matches!(i, MenuItem::Action { id, .. } if id == "OpenFile"))
        );
        assert!(file.iter().any(|i| matches!(i, MenuItem::Action { id, label } if id == "ShowSettings" && label == "Settings…")));
        assert!(
            file.iter().any(
                |i| matches!(i, MenuItem::Submenu { id, .. } if id == "NewElementInMenuGroup")
            )
        );
        // Separators are never doubled, leading or trailing.
        fn check(items: &[MenuItem]) {
            assert!(!matches!(items.first(), Some(MenuItem::Separator(_))));
            assert!(!matches!(items.last(), Some(MenuItem::Separator(_))));
            for pair in items.windows(2) {
                assert!(
                    !(matches!(pair[0], MenuItem::Separator(_))
                        && matches!(pair[1], MenuItem::Separator(_)))
                );
            }
            for item in items {
                if let MenuItem::Submenu { items, .. } = item {
                    assert!(!items.is_empty());
                    check(items);
                }
            }
        }
        check(&menu);
    }

    #[test]
    fn filtered_menus_drop_empty_submenus_and_duplicates() {
        let catalog = Catalog::bundled().unwrap();
        let menu = catalog.menu("ProjectViewPopupMenu");
        let keep = |id: &str| matches!(id, "NewFile" | "NewDir" | "RenameElement" | "CopyFileName");
        let filtered = filter_menu(&menu, &keep);
        let mut ids = Vec::new();
        fn walk(items: &[MenuItem], ids: &mut Vec<String>) {
            for item in items {
                match item {
                    MenuItem::Action { id, .. } => ids.push(id.clone()),
                    MenuItem::Submenu { id, items, .. } => {
                        ids.push(format!("{id}▸"));
                        walk(items, ids);
                    }
                    MenuItem::Separator(_) => ids.push("---".into()),
                }
            }
        }
        walk(&filtered, &mut ids);
        // New ▸ (File, Directory) comes first; Rename appears once although the
        // Refactor submenu lists it too.
        assert_eq!(ids[0], "WeighingNewGroup▸");
        assert!(ids.contains(&"NewFile".to_owned()) && ids.contains(&"NewDir".to_owned()));
        assert_eq!(ids.iter().filter(|id| *id == "RenameElement").count(), 1);
        assert!(!ids.iter().any(|id| id == "RefactoringMenu▸"));
        assert!(ids.contains(&"CopyReferencePopupGroup▸".to_owned()));
        assert_ne!(ids.last().map(String::as_str), Some("---"));
    }

    #[test]
    fn mnemonics_are_removed() {
        assert_eq!(clean_label("_File"), "File");
        assert_eq!(clean_label("Save &All"), "Save All");
        assert_eq!(clean_label("a__b"), "a_b");
    }
}

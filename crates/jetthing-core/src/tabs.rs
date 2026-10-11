//! Editor tab behaviour: pinned tabs, close variants, Reopen Closed Tab, tab
//! navigation and the tab limit (PLATFORM-UI-001..003).

use crate::{IdeState, spec, usize_param};
use jetthing_command::{Command, CommandError, CommandRegistry};
use jetthing_text::Selection;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Default "Tab limit" of the reference IDE (Settings | Editor | Editor Tabs).
pub const DEFAULT_TAB_LIMIT: usize = 10;
const CLOSED_HISTORY: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedTab {
    pub path: PathBuf,
    /// Primary caret offset when the tab was closed.
    pub caret: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorTabs {
    pub pinned: BTreeSet<PathBuf>,
    /// Closed tabs, most recent last.
    pub closed: Vec<ClosedTab>,
    /// Maximum number of tabs; opening one more closes the least recently used
    /// unpinned tab. Zero disables the limit.
    pub limit: usize,
    /// The italic preview tab: replaced by the next preview, permanent once
    /// edited, double-clicked or pinned.
    pub preview: Option<PathBuf>,
}

impl Default for EditorTabs {
    fn default() -> Self {
        Self {
            pinned: BTreeSet::new(),
            closed: Vec::new(),
            limit: DEFAULT_TAB_LIMIT,
            preview: None,
        }
    }
}

/// Which tabs a close variant removes, relative to a target tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseScope {
    /// Every unpinned tab except the target.
    Others,
    /// Every unpinned tab.
    All,
    /// Every unpinned, unmodified tab.
    Unmodified,
    /// Unpinned tabs left of the target.
    Left,
    /// Unpinned tabs right of the target.
    Right,
}

impl IdeState {
    pub fn is_pinned(&self, index: usize) -> bool {
        self.documents
            .get(index)
            .is_some_and(|document| self.tabs.pinned.contains(&document.path))
    }

    /// Remembers a tab that is about to close for Reopen Closed Tab.
    pub(crate) fn remember_closed(&mut self, index: usize) {
        let Some(document) = self.documents.get(index) else {
            return;
        };
        let entry = ClosedTab {
            path: document.path.clone(),
            caret: document.selections.primary().head,
        };
        self.tabs.closed.retain(|closed| closed.path != entry.path);
        self.tabs.closed.push(entry);
        if self.tabs.closed.len() > CLOSED_HISTORY {
            self.tabs.closed.remove(0);
        }
        let path = document.path.clone();
        self.tabs.pinned.remove(&path);
    }

    /// Closes the least recently used unpinned, inactive tabs over the limit.
    pub(crate) fn enforce_tab_limit(&mut self) -> Result<(), String> {
        let limit = self.tabs.limit;
        while limit > 0 && self.documents.len() > limit {
            let age = |path: &Path| {
                self.recent_files
                    .iter()
                    .position(|recent| recent == path)
                    .unwrap_or(usize::MAX)
            };
            let victim = (0..self.documents.len())
                .filter(|&index| Some(index) != self.active_document && !self.is_pinned(index))
                .max_by_key(|&index| age(&self.documents[index].path));
            let Some(victim) = victim else {
                return Ok(());
            };
            self.close_document(victim)?;
        }
        Ok(())
    }

    /// Closes the tabs selected by `scope` relative to `target`; returns how many
    /// were closed. Pinned tabs are kept.
    pub fn close_tabs(&mut self, target: usize, scope: CloseScope) -> Result<usize, String> {
        if target >= self.documents.len() {
            return Err("document does not exist".into());
        }
        let doomed: Vec<usize> = (0..self.documents.len())
            .filter(|&index| !self.is_pinned(index))
            .filter(|&index| match scope {
                CloseScope::Others => index != target,
                CloseScope::All => true,
                CloseScope::Unmodified => !self.documents[index].text.is_dirty(),
                CloseScope::Left => index < target,
                CloseScope::Right => index > target,
            })
            .collect();
        if scope != CloseScope::All && scope != CloseScope::Unmodified {
            self.active_document = Some(target);
        }
        let target_path = self.documents[target].path.clone();
        for &index in doomed.iter().rev() {
            self.close_document(index)?;
        }
        if let Some(index) = self.documents.iter().position(|d| d.path == target_path) {
            self.active_document = Some(index);
        }
        Ok(doomed.len())
    }

    /// Reopen Closed Tab (Ctrl+Shift+T / ⇧⌘T); restores the caret.
    pub fn reopen_closed_tab(&mut self) -> Result<Option<PathBuf>, String> {
        while let Some(closed) = self.tabs.closed.pop() {
            if self.open_file(&closed.path).is_err() {
                // Deleted or unreadable since closing: try the next one.
                continue;
            }
            if let Some(index) = self.active_document {
                let document = &mut self.documents[index];
                let caret = closed.caret.min(document.text.len_chars());
                document.selections.set_single(Selection::caret(caret));
            }
            return Ok(Some(closed.path));
        }
        Ok(None)
    }

    /// Pins or unpins a tab; pinned tabs stay leftmost in opening order.
    pub fn set_pinned(&mut self, index: usize, pinned: bool) -> Result<(), String> {
        let path = self
            .documents
            .get(index)
            .ok_or("document does not exist")?
            .path
            .clone();
        if pinned {
            if self.tabs.preview.as_ref() == Some(&path) {
                self.tabs.preview = None;
            }
            self.tabs.pinned.insert(path);
        } else {
            self.tabs.pinned.remove(&path);
        }
        self.sort_pinned_first();
        Ok(())
    }

    /// Moves the tab at `from` so it sits before the tab that is at `before` now
    /// (`before == len` appends). Pinned tabs stay leftmost, so a drop across the
    /// pinned boundary lands on its own side of it. The active tab stays active.
    pub fn move_tab(&mut self, from: usize, before: usize) -> Result<(), String> {
        let count = self.documents.len();
        if from >= count || before > count {
            return Err("document does not exist".into());
        }
        let active = self
            .active_document
            .and_then(|index| self.documents.get(index))
            .map(|document| document.path.clone());
        let document = self.documents.remove(from);
        let at = if before > from { before - 1 } else { before };
        self.documents.insert(at, document);
        self.sort_pinned_first();
        self.active_document =
            active.and_then(|path| self.documents.iter().position(|d| d.path == path));
        Ok(())
    }

    pub(crate) fn sort_pinned_first(&mut self) {
        let active = self
            .active_document
            .and_then(|index| self.documents.get(index))
            .map(|document| document.path.clone());
        let pinned = &self.tabs.pinned;
        // Stable: relative order within the pinned and unpinned groups is kept.
        self.documents
            .sort_by_key(|document| !pinned.contains(&document.path));
        self.active_document =
            active.and_then(|path| self.documents.iter().position(|d| d.path == path));
    }

    /// Select Next/Previous Tab, wrapping around.
    pub fn cycle_tab(&mut self, forward: bool) -> Result<(), String> {
        let count = self.documents.len();
        let Some(active) = self.active_document.filter(|_| count > 0) else {
            return Ok(());
        };
        let next = if forward {
            (active + 1) % count
        } else {
            (active + count - 1) % count
        };
        self.activate_document(next)
    }
}

/// `index` parameter or the active tab.
fn target(state: &IdeState, params: &Value, id: &str) -> Result<usize, CommandError> {
    match params.get("index") {
        Some(_) => usize_param(params, "index"),
        None => state
            .active_document
            .ok_or_else(|| CommandError::Disabled(id.into())),
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    for (id, label, shortcut, scope) in [
        (
            "editor.closeOthers",
            "Close Other Tabs",
            None,
            CloseScope::Others,
        ),
        ("editor.closeAll", "Close All Tabs", None, CloseScope::All),
        (
            "editor.closeUnmodified",
            "Close Unmodified Tabs",
            None,
            CloseScope::Unmodified,
        ),
        (
            "editor.closeLeft",
            "Close Tabs to the Left",
            None,
            CloseScope::Left,
        ),
        (
            "editor.closeRight",
            "Close Tabs to the Right",
            None,
            CloseScope::Right,
        ),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Editor", shortcut, r#"{"index":"number?"}"#),
                |state: &IdeState| state.active_document.is_some(),
                move |state: &mut IdeState, params| {
                    let index = target(state, &params, id)?;
                    let closed = state
                        .close_tabs(index, scope)
                        .map_err(CommandError::Failed)?;
                    Ok(json!({"closed": closed}))
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "editor.reopenClosed",
                "Reopen Closed Tab",
                "Editor",
                Some("⇧⌘T"),
                "{}",
            ),
            |state: &IdeState| !state.tabs.closed.is_empty(),
            |state: &mut IdeState, _| {
                let path = state.reopen_closed_tab().map_err(CommandError::Failed)?;
                Ok(json!({"path": path}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.togglePin",
                "Pin Tab",
                "Editor",
                None,
                r#"{"index":"number?","pinned":"bool?"}"#,
            ),
            |state: &IdeState| state.active_document.is_some(),
            |state: &mut IdeState, params| {
                let index = target(state, &params, "editor.togglePin")?;
                let pinned = params
                    .get("pinned")
                    .and_then(Value::as_bool)
                    .unwrap_or(!state.is_pinned(index));
                state
                    .set_pinned(index, pinned)
                    .map_err(CommandError::Failed)?;
                Ok(json!({"pinned": pinned}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.moveTab",
                "Move Tab",
                "Editor",
                None,
                r#"{"from":"number","before":"number"}"#,
            ),
            |state: &IdeState| state.documents.len() > 1,
            |state: &mut IdeState, params| {
                state
                    .move_tab(
                        usize_param(&params, "from")?,
                        usize_param(&params, "before")?,
                    )
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.toggleMaximize",
                "Maximize Editor",
                "Editor",
                None,
                "{}",
            ),
            |_: &IdeState| true,
            |state: &mut IdeState, _| {
                state.tool_windows.toggle_editor_maximized();
                Ok(json!({"maximized": state.tool_windows.is_editor_maximized()}))
            },
        ))
        .expect("unique built-in id");
    for (id, label, shortcut, forward) in [
        ("editor.nextTab", "Select Next Tab", "⇧⌘]", true),
        ("editor.previousTab", "Select Previous Tab", "⇧⌘[", false),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Editor", Some(shortcut), "{}"),
                |state: &IdeState| state.documents.len() > 1,
                move |state: &mut IdeState, _| {
                    state.cycle_tab(forward).map_err(CommandError::Failed)?;
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;

    fn project(files: &[&str], name: &str) -> (IdeState, CommandRegistry<IdeState>, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("jetthing-tabs-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        for file in files {
            std::fs::write(root.join(file), format!("{file}\n")).unwrap();
        }
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        for file in files {
            commands
                .execute("editor.openFile", &mut state, json!({"path": file}))
                .unwrap();
        }
        (state, commands, root)
    }

    fn names(state: &IdeState) -> Vec<String> {
        state
            .documents
            .iter()
            .map(|d| d.path.display().to_string())
            .collect()
    }

    #[test]
    fn move_tab_reorders_and_keeps_pinned_leftmost_and_the_active_tab() {
        let (mut state, commands, root) = project(&["a", "b", "c"], "move");
        let active = state.documents[state.active_document.unwrap()].path.clone();
        commands
            .execute("editor.moveTab", &mut state, json!({"from": 2, "before": 0}))
            .unwrap();
        assert_eq!(names(&state), ["c", "a", "b"]);
        assert_eq!(state.documents[state.active_document.unwrap()].path, active);
        commands
            .execute("editor.moveTab", &mut state, json!({"from": 0, "before": 3}))
            .unwrap();
        assert_eq!(names(&state), ["a", "b", "c"]);
        state.set_pinned(0, true).unwrap();
        commands
            .execute("editor.moveTab", &mut state, json!({"from": 2, "before": 0}))
            .unwrap();
        assert_eq!(names(&state), ["a", "c", "b"]);
        assert!(
            commands
                .execute("editor.moveTab", &mut state, json!({"from": 9, "before": 0}))
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn close_variants_keep_pinned_tabs() {
        let (mut state, commands, root) = project(&["a", "b", "c", "d", "e"], "close");
        let run = |state: &mut IdeState, id: &str, params: Value| {
            commands.execute(id, state, params).unwrap()
        };
        run(&mut state, "editor.togglePin", json!({"index": 3}));
        assert_eq!(names(&state), ["d", "a", "b", "c", "e"], "pinned move left");
        assert_eq!(state.active_document, Some(4), "active tab e is kept");
        run(&mut state, "editor.closeRight", json!({"index": 2}));
        assert_eq!(names(&state), ["d", "a", "b"]);
        assert_eq!(state.active_document, Some(2));
        run(&mut state, "editor.closeLeft", json!({"index": 2}));
        assert_eq!(names(&state), ["d", "b"]);
        let closed = run(&mut state, "editor.closeOthers", json!({"index": 1}));
        assert_eq!(
            closed,
            json!({"closed": 0}),
            "only the pinned tab remains besides b"
        );
        run(&mut state, "editor.closeAll", Value::Null);
        assert_eq!(names(&state), ["d"]);
        assert!(state.is_pinned(0));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reopen_closed_tab_restores_order_and_caret() {
        let (mut state, commands, root) = project(&["a", "b", "c"], "reopen");
        state.documents[1]
            .selections
            .set_single(Selection::caret(1));
        commands
            .execute("editor.close", &mut state, json!({"index": 1}))
            .unwrap();
        commands
            .execute("editor.close", &mut state, json!({"index": 0}))
            .unwrap();
        let reopened = commands
            .execute("editor.reopenClosed", &mut state, Value::Null)
            .unwrap();
        assert_eq!(reopened, json!({"path": "a"}));
        commands
            .execute("editor.reopenClosed", &mut state, Value::Null)
            .unwrap();
        let active = state.active_document.unwrap();
        assert_eq!(state.documents[active].path, PathBuf::from("b"));
        assert_eq!(state.documents[active].selections.primary().head, 1);
        assert!(!commands.is_enabled("editor.reopenClosed", &state));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tab_limit_closes_the_least_recently_used_unpinned_tab() {
        let (mut state, commands, root) = project(&["a", "b", "c", "d"], "limit");
        state.tabs.limit = 3;
        state.set_pinned(0, true).unwrap();
        commands
            .execute("editor.activate", &mut state, json!({"index": 1}))
            .unwrap();
        std::fs::write(root.join("e"), "e\n").unwrap();
        state.files = state.project.as_ref().unwrap().entries(100);
        commands
            .execute("editor.openFile", &mut state, json!({"path": "e"}))
            .unwrap();
        // `a` is pinned, `e` is active and activating `b` made it recent, so
        // the least recently used `c` and then `d` close.
        assert_eq!(names(&state), ["a", "b", "e"]);
        assert_eq!(state.tabs.closed.last().unwrap().path, PathBuf::from("d"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pinned_tabs_survive_a_workspace_round_trip() {
        let (mut state, commands, root) = project(&["a", "b"], "persist");
        state.set_pinned(1, true).unwrap();
        state.save_workspace().unwrap();
        let mut reopened = IdeState::default();
        commands
            .execute("project.open", &mut reopened, json!({"path": root}))
            .unwrap();
        assert_eq!(names(&reopened), ["b", "a"]);
        assert!(reopened.is_pinned(0));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cycles_tabs_with_wraparound() {
        let (mut state, commands, root) = project(&["a", "b"], "cycle");
        commands
            .execute("editor.nextTab", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.active_document, Some(0));
        commands
            .execute("editor.previousTab", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.active_document, Some(1));
        std::fs::remove_dir_all(root).unwrap();
    }
}

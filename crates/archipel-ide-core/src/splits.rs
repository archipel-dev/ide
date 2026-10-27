//! Editor splits and the preview tab: commands over `archipel_ide_split::SplitTree`
//! (PLATFORM-UI-004, PLATFORM-UI-003). The tree decides which tabs each group
//! shows; `documents` stays the list of open buffers.

use crate::{IdeState, spec, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_settings::ids;
use archipel_ide_split::{Edge, GroupId, SplitTree};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

impl IdeState {
    fn document_path(&self, index: usize) -> Option<PathBuf> {
        self.documents.get(index).map(|d| d.path.clone())
    }

    fn active_path(&self) -> Option<PathBuf> {
        self.active_document.and_then(|i| self.document_path(i))
    }

    /// Brings the split tree and the open documents back in line: closed tabs
    /// leave their group, new ones join the focused group, an edited preview
    /// tab becomes permanent, and a tab activated elsewhere focuses its group.
    pub fn sync_splits(&mut self) {
        let paths: Vec<PathBuf> = self.documents.iter().map(|d| d.path.clone()).collect();
        self.splits.retain(|p| paths.contains(p));
        for path in &paths {
            self.splits.add(path.clone());
        }
        if let Some(preview) = &self.tabs.preview {
            let keep = self
                .documents
                .iter()
                .find(|d| &d.path == preview)
                .is_some_and(|d| !d.text.is_dirty());
            if !keep {
                self.tabs.preview = None;
            }
        }
        if self.diff_active || self.plugin_host.active_tab.is_some() {
            return;
        }
        let Some(current) = self.active_path() else {
            return;
        };
        if self.splits_synced.as_ref() == Some(&current) {
            // Nothing activated a tab: the focused group's tab is the truth.
            self.follow_split_active();
        } else {
            self.splits.open(current.clone());
            self.splits_synced = Some(current);
        }
    }

    /// Makes the focused group's active tab the active document.
    pub(crate) fn follow_split_active(&mut self) {
        let active = self.splits.focused().active.clone();
        self.active_document = active
            .as_ref()
            .and_then(|path| self.documents.iter().position(|d| &d.path == path));
        self.splits_synced = active;
    }

    pub(crate) fn close_tab_in_splits(&mut self, path: &Path) {
        let multi = self.splits.group_count() > 1;
        self.splits.remove_tab(&path.to_owned());
        if multi {
            self.follow_split_active();
        }
    }

    fn show_split_active(&mut self) {
        self.follow_split_active();
        self.diff_active = false;
        self.plugin_host.active_tab = None;
        self.close_completion();
    }

    /// Focuses a group and shows its active tab.
    pub fn focus_group(&mut self, group: GroupId) -> Result<(), String> {
        self.sync_splits();
        if !self.splits.focus(group) {
            return Err("editor group does not exist".into());
        }
        self.show_split_active();
        Ok(())
    }

    fn tab_target(&mut self, params: &Value) -> Result<PathBuf, String> {
        self.sync_splits();
        let index = match params.get("index").and_then(Value::as_u64) {
            Some(index) => index as usize,
            None => self.active_document.ok_or("no editor tab is open")?,
        };
        self.document_path(index)
            .ok_or_else(|| "document does not exist".into())
    }

    /// Split Right / Down: the tab moves into a new group next to its own
    /// (an empty one when it is the group's only tab).
    pub fn split_tab(&mut self, path: PathBuf, edge: Edge, group: Option<GroupId>) {
        let group = group
            .or_else(|| self.splits.group_of(&path))
            .unwrap_or_else(|| self.splits.focused_id());
        self.splits.split(group, edge, Some(path));
        self.show_split_active();
    }

    /// Opens `path` in the group opposite the focused one (Open in Right Split).
    pub fn open_in_right_split(&mut self, path: &Path) -> Result<(), String> {
        self.open_file(path)?;
        self.sync_splits();
        if self.documents.iter().any(|d| d.path == path) {
            self.splits.open_in_opposite(path.to_owned());
            self.show_split_active();
        }
        Ok(())
    }

    /// Single-click open: reuses the preview tab unless the file is already open.
    pub fn open_file_preview(&mut self, path: &Path) -> Result<(), String> {
        if !self
            .settings()
            .read()
            .bool(ids::EDITOR_PREVIEW_TAB)
            .unwrap_or(true)
        {
            return self.open_file(path);
        }
        self.sync_splits();
        if let Some(index) = self.documents.iter().position(|d| d.path == path) {
            return self.activate_document(index);
        }
        let old = self.tabs.preview.clone();
        self.open_file(path)?;
        if self.open_in_plugin_editor_active() || !self.documents.iter().any(|d| d.path == path) {
            return Ok(());
        }
        if let Some(old) = old
            && let Some(index) = self.documents.iter().position(|d| d.path == old)
            && !self.documents[index].text.is_dirty()
            && !self.is_pinned(index)
        {
            self.close_document(index)?;
            self.tabs.closed.retain(|closed| closed.path != old);
        }
        self.tabs.preview = Some(path.to_owned());
        self.sync_splits();
        Ok(())
    }

    fn open_in_plugin_editor_active(&self) -> bool {
        self.plugin_host.active_tab.is_some()
    }

    /// Keep Tab Open: the preview tab becomes permanent.
    pub fn keep_tab_open(&mut self, path: &Path) {
        if self.tabs.preview.as_deref() == Some(path) {
            self.tabs.preview = None;
        }
    }

    pub fn is_preview(&self, index: usize) -> bool {
        self.documents
            .get(index)
            .is_some_and(|d| self.tabs.preview.as_ref() == Some(&d.path))
    }

    /// The tree as stored in the project workspace file.
    pub(crate) fn splits_json(&self) -> Option<Value> {
        self.splits
            .is_split()
            .then(|| serde_json::to_value(&self.splits).ok())
            .flatten()
    }

    pub(crate) fn restore_splits(&mut self, value: &Value, known: &dyn Fn(&PathBuf) -> bool) {
        let Ok(mut tree) = serde_json::from_value::<SplitTree<PathBuf>>(value.clone()) else {
            return;
        };
        tree.retain(known);
        tree.normalize();
        self.splits = tree;
        self.splits_synced = None;
    }
}

fn edge_param(params: &Value) -> Result<Edge, CommandError> {
    match params.get("edge").and_then(Value::as_str) {
        Some("left") => Ok(Edge::Left),
        Some("right") => Ok(Edge::Right),
        Some("top") => Ok(Edge::Top),
        Some("bottom") => Ok(Edge::Bottom),
        _ => Err(CommandError::InvalidParams(
            "expected `edge` to be left, right, top or bottom".into(),
        )),
    }
}

fn group_param(params: &Value) -> Result<GroupId, CommandError> {
    usize_param(params, "group").and_then(|g| {
        GroupId::try_from(g).map_err(|_| CommandError::InvalidParams("bad `group`".into()))
    })
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let has_doc = |state: &IdeState| state.active_document.is_some();
    for (id, label, edge) in [
        ("editor.splitRight", "Split Right", Edge::Right),
        ("editor.splitDown", "Split Down", Edge::Bottom),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Editor", None, r#"{"index":"number?"}"#),
                has_doc,
                move |state: &mut IdeState, params| {
                    let path = state.tab_target(&params).map_err(CommandError::Failed)?;
                    state.split_tab(path, edge, None);
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "editor.splitAt",
                "Split at Editor Edge",
                "Editor",
                None,
                r#"{"index":"number?","group":"number","edge":"string"}"#,
            ),
            has_doc,
            |state: &mut IdeState, params| {
                let edge = edge_param(&params)?;
                let group = group_param(&params)?;
                let path = state.tab_target(&params).map_err(CommandError::Failed)?;
                if state.splits.group(group).is_none() {
                    return Err(CommandError::Failed("editor group does not exist".into()));
                }
                state.split_tab(path, edge, Some(group));
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.moveToGroup",
                "Move Tab to Editor Group",
                "Editor",
                None,
                r#"{"index":"number?","group":"number"}"#,
            ),
            has_doc,
            |state: &mut IdeState, params| {
                let group = group_param(&params)?;
                let path = state.tab_target(&params).map_err(CommandError::Failed)?;
                if state.splits.group(group).is_none() {
                    return Err(CommandError::Failed("editor group does not exist".into()));
                }
                state.splits.move_tab(&path, group);
                state.splits.focus(group);
                state.show_split_active();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.moveToOppositeGroup",
                "Move to Opposite Group",
                "Editor",
                None,
                r#"{"index":"number?"}"#,
            ),
            |state: &IdeState| {
                state.active_document.is_some()
                    && (state.splits.is_split() || state.documents.len() > 1)
            },
            |state: &mut IdeState, params| {
                let path = state.tab_target(&params).map_err(CommandError::Failed)?;
                state
                    .splits
                    .move_to_opposite(&path)
                    .ok_or_else(|| CommandError::Failed("nothing to move to a split".into()))?;
                state.show_split_active();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.openInRightSplit",
                "Open in Right Split",
                "Editor",
                None,
                r#"{"path":"string?"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let path = match params.get("path").and_then(Value::as_str) {
                    Some(path) => PathBuf::from(path),
                    None => state
                        .active_path()
                        .ok_or_else(|| CommandError::Disabled("editor.openInRightSplit".into()))?,
                };
                state
                    .open_in_right_split(&path)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.unsplit",
                "Unsplit",
                "Editor",
                None,
                r#"{"group":"number?"}"#,
            ),
            |state: &IdeState| state.splits.is_split(),
            |state: &mut IdeState, params| {
                state.sync_splits();
                let group = match params.get("group") {
                    Some(_) => group_param(&params)?,
                    None => state.splits.focused_id(),
                };
                if !state.splits.unsplit(group) {
                    return Err(CommandError::Failed("the editor is not split".into()));
                }
                state.show_split_active();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("editor.unsplitAll", "Unsplit All", "Editor", None, "{}"),
            |state: &IdeState| state.splits.is_split(),
            |state: &mut IdeState, _| {
                state.sync_splits();
                state.splits.unsplit_all();
                state.show_split_active();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    for (id, label, forward) in [
        ("editor.nextSplitter", "Go to Next Splitter", true),
        ("editor.previousSplitter", "Go to Previous Splitter", false),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Editor", None, "{}"),
                |state: &IdeState| state.splits.is_split(),
                move |state: &mut IdeState, _| {
                    state.sync_splits();
                    state.splits.focus_cycle(forward);
                    state.show_split_active();
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "editor.changeSplitOrientation",
                "Change Splitter Orientation",
                "Editor",
                None,
                "{}",
            ),
            |state: &IdeState| state.splits.is_split(),
            |state: &mut IdeState, _| {
                let focused = state.splits.focused_id();
                state.splits.change_orientation(focused);
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.focusGroup",
                "Focus Editor Group",
                "Editor",
                None,
                r#"{"group":"number"}"#,
            ),
            |state: &IdeState| state.splits.is_split(),
            |state: &mut IdeState, params| {
                let group = group_param(&params)?;
                state.focus_group(group).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.setSplitRatio",
                "Resize Editor Split",
                "Editor",
                None,
                r#"{"path":"bool[]","ratio":"number"}"#,
            ),
            |state: &IdeState| state.splits.is_split(),
            |state: &mut IdeState, params| {
                let path: Vec<bool> = params
                    .get("path")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(Value::as_bool).collect())
                    .unwrap_or_default();
                let ratio = params
                    .get("ratio")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| CommandError::InvalidParams("expected `ratio`".into()))?;
                state.splits.set_ratio(&path, ratio as f32);
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.openPreview",
                "Open File in Preview Tab",
                "Editor",
                None,
                r#"{"path":"string"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let path = crate::string_param(&params, "path")?;
                state
                    .open_file_preview(Path::new(&path))
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.keepTabOpen",
                "Keep Tab Open",
                "Editor",
                None,
                r#"{"index":"number?"}"#,
            ),
            |state: &IdeState| state.tabs.preview.is_some(),
            |state: &mut IdeState, params| {
                let path = state.tab_target(&params).map_err(CommandError::Failed)?;
                state.keep_tab_open(&path);
                Ok(json!({"kept": path}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;

    fn project(files: &[&str], name: &str) -> (IdeState, CommandRegistry<IdeState>, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-splits-{}-{name}", std::process::id()));
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
        (state, commands, root)
    }

    fn open(state: &mut IdeState, commands: &CommandRegistry<IdeState>, files: &[&str]) {
        for file in files {
            commands
                .execute("editor.openFile", state, json!({"path": file}))
                .unwrap();
        }
    }

    fn group_tabs(state: &IdeState) -> Vec<Vec<String>> {
        state
            .splits
            .groups()
            .iter()
            .map(|g| g.tabs.iter().map(|t| t.display().to_string()).collect())
            .collect()
    }

    fn active(state: &IdeState) -> String {
        state.documents[state.active_document.unwrap()]
            .path
            .display()
            .to_string()
    }

    #[test]
    fn split_right_moves_the_tab_and_groups_remember_their_focus() {
        let (mut state, commands, root) = project(&["a", "b", "c"], "split");
        open(&mut state, &commands, &["a", "b", "c"]);
        commands
            .execute("editor.splitRight", &mut state, json!({"index": 2}))
            .unwrap();
        assert_eq!(group_tabs(&state), [vec!["a", "b"], vec!["c"]]);
        assert_eq!(active(&state), "c");
        commands
            .execute("editor.focusGroup", &mut state, json!({"group": 0}))
            .unwrap();
        assert_eq!(active(&state), "b", "the left group remembered its tab");
        commands
            .execute("editor.nextSplitter", &mut state, json!({}))
            .unwrap();
        assert_eq!(active(&state), "c");
        commands
            .execute("editor.moveToOppositeGroup", &mut state, json!({}))
            .unwrap();
        assert_eq!(group_tabs(&state), [vec!["a", "b", "c"]]);
        assert_eq!(active(&state), "c");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn closing_tabs_and_unsplit_keep_the_model_consistent() {
        let (mut state, commands, root) = project(&["a", "b", "c"], "unsplit");
        open(&mut state, &commands, &["a", "b", "c"]);
        commands
            .execute("editor.splitDown", &mut state, json!({}))
            .unwrap();
        assert_eq!(state.splits.group_count(), 2);
        commands
            .execute("editor.close", &mut state, json!({"index": 2}))
            .unwrap();
        state.sync_splits();
        assert_eq!(
            group_tabs(&state),
            [vec!["a", "b"]],
            "empty group collapsed"
        );
        commands
            .execute("editor.splitRight", &mut state, json!({}))
            .unwrap();
        commands
            .execute("editor.unsplitAll", &mut state, json!({}))
            .unwrap();
        assert_eq!(state.splits.group_count(), 1);
        assert_eq!(active(&state), "b");
        assert!(!commands.is_enabled("editor.unsplit", &state));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn open_in_right_split_creates_the_split_and_reuses_it() {
        let (mut state, commands, root) = project(&["a", "b", "c"], "right");
        open(&mut state, &commands, &["a"]);
        commands
            .execute("editor.openInRightSplit", &mut state, json!({"path": "b"}))
            .unwrap();
        assert_eq!(group_tabs(&state), [vec!["a"], vec!["b"]]);
        commands
            .execute("editor.focusGroup", &mut state, json!({"group": 0}))
            .unwrap();
        commands
            .execute("editor.openInRightSplit", &mut state, json!({"path": "c"}))
            .unwrap();
        assert_eq!(group_tabs(&state), [vec!["a"], vec!["b", "c"]]);
        assert_eq!(active(&state), "c");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn drag_and_drop_commands_split_at_an_edge_or_move_between_groups() {
        let (mut state, commands, root) = project(&["a", "b", "c"], "drop");
        open(&mut state, &commands, &["a", "b", "c"]);
        commands
            .execute(
                "editor.splitAt",
                &mut state,
                json!({"index": 2, "group": 0, "edge": "bottom"}),
            )
            .unwrap();
        assert_eq!(group_tabs(&state), [vec!["a", "b"], vec!["c"]]);
        commands
            .execute(
                "editor.splitAt",
                &mut state,
                json!({"index": 0, "group": 0, "edge": "left"}),
            )
            .unwrap();
        assert_eq!(group_tabs(&state), [vec!["a"], vec!["b"], vec!["c"]]);
        let other = state.splits.groups()[2].id;
        commands
            .execute(
                "editor.moveToGroup",
                &mut state,
                json!({"index": 1, "group": other}),
            )
            .unwrap();
        assert_eq!(group_tabs(&state), [vec!["a"], vec!["c", "b"]]);
        assert!(
            commands
                .execute(
                    "editor.splitAt",
                    &mut state,
                    json!({"index": 0, "group": 99, "edge": "left"})
                )
                .is_err()
        );
        assert!(
            commands
                .execute(
                    "editor.splitAt",
                    &mut state,
                    json!({"index": 0, "group": 0, "edge": "middle"})
                )
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn opening_an_open_file_focuses_its_group() {
        let (mut state, commands, root) = project(&["a", "b"], "refocus");
        open(&mut state, &commands, &["a", "b"]);
        commands
            .execute("editor.splitRight", &mut state, json!({}))
            .unwrap();
        open(&mut state, &commands, &["a"]);
        state.sync_splits();
        assert_eq!(state.splits.focused().tabs, [PathBuf::from("a")]);
        assert_eq!(active(&state), "a");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn splits_survive_a_workspace_round_trip() {
        let (mut state, commands, root) = project(&["a", "b", "c"], "persist");
        open(&mut state, &commands, &["a", "b", "c"]);
        commands
            .execute("editor.splitRight", &mut state, json!({"index": 2}))
            .unwrap();
        commands
            .execute("editor.focusGroup", &mut state, json!({"group": 0}))
            .unwrap();
        commands
            .execute("workspace.save", &mut state, Value::Null)
            .unwrap();
        let mut reopened = IdeState::default();
        commands
            .execute("project.open", &mut reopened, json!({"path": root}))
            .unwrap();
        assert_eq!(group_tabs(&reopened), [vec!["a", "b"], vec!["c"]]);
        assert_eq!(reopened.splits.focused_id(), 0);
        assert_eq!(active(&reopened), "b");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preview_tab_is_replaced_and_becomes_permanent_on_edit_double_click_or_pin() {
        let (mut state, commands, root) = project(&["a", "b", "c", "d"], "preview");
        let preview = |state: &mut IdeState, file: &str| {
            commands
                .execute("editor.openPreview", state, json!({"path": file}))
                .unwrap();
        };
        let names = |state: &IdeState| -> Vec<String> {
            state
                .documents
                .iter()
                .map(|d| d.path.display().to_string())
                .collect()
        };
        preview(&mut state, "a");
        assert!(state.is_preview(0));
        preview(&mut state, "b");
        assert_eq!(names(&state), ["b"], "the next preview replaces it");
        // Double-click opens for real.
        commands
            .execute("editor.openFile", &mut state, json!({"path": "b"}))
            .unwrap();
        assert!(state.tabs.preview.is_none());
        preview(&mut state, "c");
        preview(&mut state, "d");
        assert_eq!(names(&state), ["b", "d"]);
        // Editing makes it permanent.
        state.documents[1].text.edit(0, 0, "x").unwrap();
        preview(&mut state, "a");
        assert_eq!(names(&state), ["b", "d", "a"]);
        assert_eq!(state.tabs.preview, Some(PathBuf::from("a")));
        // Pinning makes it permanent too.
        commands
            .execute("editor.togglePin", &mut state, json!({"index": 2}))
            .unwrap();
        assert!(state.tabs.preview.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn replacing_a_preview_in_a_split_keeps_the_new_tab_active() {
        let (mut state, commands, root) = project(&["a", "b", "c", "d"], "splitpreview");
        open(&mut state, &commands, &["a", "b"]);
        commands
            .execute("editor.splitRight", &mut state, json!({}))
            .unwrap();
        for file in ["c", "d"] {
            commands
                .execute("editor.openPreview", &mut state, json!({"path": file}))
                .unwrap();
        }
        assert_eq!(active(&state), "d", "the replaced preview must not win");
        assert_eq!(state.documents.len(), 3);
        assert!(state.is_preview(state.active_document.unwrap()));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preview_can_be_switched_off() {
        let (mut state, commands, root) = project(&["a", "b"], "nopreview");
        state
            .settings()
            .write()
            .set(
                ids::EDITOR_PREVIEW_TAB,
                archipel_ide_settings::SettingValue::Bool(false),
            )
            .unwrap();
        for file in ["a", "b"] {
            commands
                .execute("editor.openPreview", &mut state, json!({"path": file}))
                .unwrap();
        }
        assert_eq!(state.documents.len(), 2);
        assert!(state.tabs.preview.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}

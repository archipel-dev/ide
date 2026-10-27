//! Changed-line tracking for open documents against `HEAD`: the data behind
//! editor gutter change markers, change navigation and hunk rollback.

use crate::vcs::DiffTab;
use crate::{CommandError, IdeState, spec};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_git::{ChangeKind, Hunk, HunkKind};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Cached comparison of one open document with its `HEAD` version.
#[derive(Debug, Clone, Default)]
pub(crate) struct LineChanges {
    /// `HEAD` content; `None` when the file is untracked, ignored or binary
    /// (no markers are shown).
    pub(crate) base: Option<String>,
    /// Document revision the hunks were computed for.
    pub(crate) revision: Option<u64>,
    pub(crate) hunks: Vec<Hunk>,
}

impl LineChanges {
    pub(crate) fn with_base(base: Option<String>) -> Self {
        Self {
            base,
            ..Self::default()
        }
    }
}

/// Hunk that "owns" new-side `line`: the one covering it, or a deletion
/// marker sitting directly above it.
pub fn hunk_at_line(hunks: &[Hunk], line: usize) -> Option<usize> {
    hunks
        .iter()
        .position(|hunk| hunk.new.contains(&line))
        .or_else(|| {
            hunks
                .iter()
                .position(|hunk| hunk.kind() == HunkKind::Deleted && hunk.new.start == line)
        })
}

/// Byte range of `lines` in `text` (line terminators included).
fn byte_range(text: &str, lines: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let mut starts = vec![0];
    starts.extend(text.match_indices('\n').map(|(index, _)| index + 1));
    let at = |line: usize| {
        starts
            .get(line)
            .copied()
            .unwrap_or(text.len())
            .min(text.len())
    };
    at(lines.start)..at(lines.end)
}

impl IdeState {
    fn load_base(&self, path: &Path) -> Option<String> {
        let git = self.git_status()?;
        let project = self.project.as_ref()?;
        let git_root = git.root.canonicalize().unwrap_or_else(|_| git.root.clone());
        let repo_path = project.root().join(path);
        let repo_path = repo_path.strip_prefix(&git_root).ok()?;
        let untracked = git
            .changes
            .iter()
            .any(|change| change.path == repo_path && change.kind == ChangeKind::Untracked);
        if untracked {
            return None;
        }
        match git.head_text(repo_path) {
            Ok(Some(text)) => Some(text),
            // Staged new file: everything is an addition.
            Ok(None) => git
                .changes
                .iter()
                .any(|change| change.path == repo_path && change.kind == ChangeKind::Added)
                .then(String::new),
            Err(_) => None,
        }
    }

    /// Changed hunks of document `index` against `HEAD`, recomputed only when
    /// the document or the repository changed.
    pub fn document_changes(&self, index: usize) -> Vec<Hunk> {
        let (Some(document), Some(tracker)) = (self.documents.get(index), self.line_status())
        else {
            return Vec::new();
        };
        let path = document.path.clone();
        tracker.changes(
            &path,
            document.text.revision(),
            || self.load_base(&path),
            // Copying the rope out is only needed when the document changed.
            || document.text.text(),
        )
    }

    /// `HEAD` text of the old side of `hunk` in document `index`, as cached by
    /// [`IdeState::document_changes`].
    pub fn hunk_base_text(&self, index: usize, hunk: &Hunk) -> Option<String> {
        let path = &self.documents.get(index)?.path;
        let base = self.line_status()?.base_text(path)?;
        Some(base[byte_range(&base, hunk.old.clone())].to_owned())
    }

    fn active_line(&self) -> Result<(usize, usize), String> {
        let index = self.active_document.ok_or("no active document")?;
        let document = &self.documents[index];
        let line = document
            .text
            .char_to_line(document.selections.primary().head);
        Ok((index, line))
    }

    /// Moves the caret to the next or previous changed hunk.
    fn navigate_change(&mut self, forward: bool) -> Result<(), String> {
        let (index, line) = self.active_line()?;
        let hunks = self.document_changes(index);
        let current = hunk_at_line(&hunks, line);
        let target = if forward {
            hunks
                .iter()
                .enumerate()
                .find(|(i, hunk)| hunk.new.start > line && Some(*i) != current)
        } else {
            hunks
                .iter()
                .enumerate()
                .rev()
                .find(|(i, hunk)| hunk.new.start < line && Some(*i) != current)
        };
        let Some((_, hunk)) = target else {
            return Err(if hunks.is_empty() {
                "No changes in this file".into()
            } else if forward {
                "No next change".into()
            } else {
                "No previous change".into()
            });
        };
        let target_line = hunk.new.start;
        self.edit_active(|document| {
            let line = target_line.min(document.text.len_lines().saturating_sub(1));
            let start = document.text.line_to_char(line).unwrap_or(0);
            document
                .selections
                .set_single(archipel_ide_text::Selection::caret(start));
            Ok(())
        })
    }

    /// Restores the `HEAD` version of the hunk at new-side `line`.
    pub fn rollback_change(&mut self, line: Option<usize>) -> Result<(), String> {
        let (index, caret_line) = self.active_line()?;
        let line = line.unwrap_or(caret_line);
        let hunks = self.document_changes(index);
        let hunk = hunk_at_line(&hunks, line)
            .map(|i| hunks[i].clone())
            .ok_or("There is no change at this line")?;
        let path = self.documents[index].path.clone();
        let base = self
            .line_status()
            .and_then(|tracker| tracker.base_text(&path))
            .ok_or("There is no change at this line")?;
        let old = base[byte_range(&base, hunk.old.clone())].to_owned();
        self.edit_active(|document| {
            let len = document.text.len_chars();
            let at = |line: usize| document.text.line_to_char(line).unwrap_or(len).min(len);
            let (start, end) = (at(hunk.new.start), at(hunk.new.end));
            document
                .text
                .edit(start, end, &old)
                .map_err(|error| error.to_string())?;
            document
                .selections
                .set_single(archipel_ide_text::Selection::caret(start));
            Ok(())
        })?;
        self.status = format!(
            "Rolled back {} line{}",
            hunk.new.len().max(hunk.old.len()),
            if hunk.new.len().max(hunk.old.len()) == 1 {
                ""
            } else {
                "s"
            }
        );
        Ok(())
    }

    /// Opens the diff tab for the active document, revealing the hunk at
    /// new-side `line` when given.
    pub fn show_diff_at(&mut self, path: &Path, line: Option<usize>) -> Result<(), String> {
        self.show_diff(path)?;
        if let (Some(line), Some(tab)) = (line, self.diff.as_mut()) {
            reveal_line(tab, line);
        }
        Ok(())
    }
}

fn reveal_line(tab: &mut DiffTab, line: usize) {
    if let Some(index) = hunk_at_line(&tab.diff.hunks, line) {
        tab.current = Some(index);
        tab.reveal = Some(index);
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let tracked = |state: &IdeState| state.active_document.is_some() && state.has_git();
    for (id, label, shortcut, forward) in [
        ("vcs.nextChange", "Next Change", "⌃⌥⇧↓", true),
        ("vcs.previousChange", "Previous Change", "⌃⌥⇧↑", false),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Git", Some(shortcut), "{}"),
                tracked,
                move |state: &mut IdeState, _| {
                    state
                        .navigate_change(forward)
                        .map_err(CommandError::Failed)?;
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "vcs.rollbackChange",
                "Rollback Lines",
                "Git",
                Some("⌥⌘Z"),
                r#"{"line":"number?"}"#,
            ),
            tracked,
            |state: &mut IdeState, params| {
                let line = params
                    .get("line")
                    .and_then(Value::as_u64)
                    .map(|line| line as usize);
                state.rollback_change(line).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "vcs.showChangeDiff",
                "Show Diff for Change",
                "Git",
                None,
                r#"{"line":"number?"}"#,
            ),
            tracked,
            |state: &mut IdeState, params| {
                let (index, caret) = state.active_line().map_err(CommandError::Failed)?;
                let line = params
                    .get("line")
                    .and_then(Value::as_u64)
                    .map_or(caret, |line| line as usize);
                let path: PathBuf = state.documents[index].path.clone();
                state
                    .show_diff_at(&path, Some(line))
                    .map_err(CommandError::Failed)?;
                Ok(json!({"path": path}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;
    use std::fs;

    fn git(root: &Path, args: &[&str]) {
        assert!(
            std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }

    fn open(name: &str, committed: &str, working: &str) -> (PathBuf, IdeState) {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-lines-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "test@example.invalid"]);
        git(&root, &["config", "user.name", "Archipel IDE Test"]);
        fs::write(root.join("a.txt"), committed).unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "initial"]);
        fs::write(root.join("a.txt"), working).unwrap();
        fs::write(root.join("new.txt"), "untracked\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "a.txt"}))
            .unwrap();
        (root, state)
    }

    #[test]
    fn tracks_changes_while_editing_and_ignores_untracked_files() {
        let (root, mut state) = open("track", "a\nb\nc\n", "a\nB\nc\n");
        let kinds: Vec<_> = state.document_changes(0).iter().map(Hunk::kind).collect();
        assert_eq!(kinds, vec![HunkKind::Modified]);
        state.insert_active_text(0, "new\n").unwrap();
        let kinds: Vec<_> = state.document_changes(0).iter().map(Hunk::kind).collect();
        assert_eq!(kinds, vec![HunkKind::Inserted, HunkKind::Modified]);
        let commands = built_in_commands();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "new.txt"}))
            .unwrap();
        assert!(state.document_changes(1).is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn navigates_and_rolls_back_hunks_with_undo() {
        let (root, mut state) = open("rollback", "1\n2\n3\n4\n5\n6\n", "1\nX\n3\n4\n6\nadded\n");
        let commands = built_in_commands();
        commands
            .execute("vcs.nextChange", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.active_line().unwrap().1, 1);
        commands
            .execute("vcs.nextChange", &mut state, Value::Null)
            .unwrap();
        // The deletion of "5" sits above line 4 ("6").
        assert_eq!(state.active_line().unwrap().1, 4);
        commands
            .execute("vcs.previousChange", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.active_line().unwrap().1, 1);
        assert!(
            commands
                .execute("vcs.previousChange", &mut state, Value::Null)
                .is_err()
        );
        commands
            .execute("vcs.rollbackChange", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "1\n2\n3\n4\n6\nadded\n");
        commands
            .execute("vcs.rollbackChange", &mut state, json!({"line": 4}))
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "1\n2\n3\n4\n5\n6\nadded\n");
        commands
            .execute("vcs.rollbackChange", &mut state, json!({"line": 6}))
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "1\n2\n3\n4\n5\n6\n");
        assert!(state.document_changes(0).is_empty());
        assert!(
            commands
                .execute("vcs.rollbackChange", &mut state, json!({"line": 0}))
                .is_err()
        );
        commands
            .execute("editor.undo", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "1\n2\n3\n4\n5\n6\nadded\n");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shows_diff_at_the_hunk_of_a_line() {
        let (root, mut state) = open("showdiff", "a\nb\nc\nd\n", "A\nb\nc\nD\n");
        built_in_commands()
            .execute("vcs.showChangeDiff", &mut state, json!({"line": 3}))
            .unwrap();
        let tab = state.diff.as_ref().unwrap();
        assert_eq!(tab.current, Some(1));
        assert_eq!(tab.reveal, Some(1));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn byte_ranges_cover_whole_lines() {
        assert_eq!(byte_range("a\nbb\nc", 1..2), 2..5);
        assert_eq!(byte_range("a\nbb\nc", 2..3), 5..6);
        assert_eq!(byte_range("a\n", 1..1), 2..2);
    }
}

//! Git integration state beyond plain status: background status refresh and
//! the diff viewer tab opened from the Commit tool window.

use crate::{CommandError, IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_git::{
    ChangeKind, FileDiff, Notify, RepositoryStatus, StatusWatcher, WhitespaceMode,
};
use archipel_ide_language::{LanguageAnalysis, analyze};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// How often Git status is reloaded in the background. External changes also
/// trigger an immediate reload when the IDE window regains focus.
const STATUS_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffViewer {
    #[default]
    SideBySide,
    Unified,
}

impl DiffViewer {
    pub fn id(self) -> &'static str {
        match self {
            Self::SideBySide => "sideBySide",
            Self::Unified => "unified",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::SideBySide => "Side-by-side viewer",
            Self::Unified => "Unified viewer",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        [Self::SideBySide, Self::Unified]
            .into_iter()
            .find(|viewer| viewer.id() == id)
    }
}

/// Word-level highlighting of changed lines (the "Highlight words" combo).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffHighlight {
    #[default]
    Words,
    Lines,
    None,
}

impl DiffHighlight {
    pub const ALL: [DiffHighlight; 3] = [Self::Words, Self::Lines, Self::None];

    pub fn id(self) -> &'static str {
        match self {
            Self::Words => "words",
            Self::Lines => "lines",
            Self::None => "none",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Words => "Highlight words",
            Self::Lines => "Highlight lines",
            Self::None => "Do not highlight",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.id() == id)
    }
}

/// An open "HEAD vs working copy" comparison, shown as an editor tab.
#[derive(Debug, Clone)]
pub struct DiffTab {
    /// Project-relative path of the compared file.
    pub path: PathBuf,
    pub diff: FileDiff,
    pub viewer: DiffViewer,
    /// The hunk last navigated to with Next/Previous Difference.
    pub current: Option<usize>,
    /// Hunk the viewer should scroll into view on its next frame.
    pub reveal: Option<usize>,
    pub old_analysis: LanguageAnalysis,
    pub new_analysis: LanguageAnalysis,
    /// Short hash of the new side when comparing two commits; `None` for the
    /// working copy (which then updates live).
    pub new_revision: Option<String>,
    pub whitespace: WhitespaceMode,
    pub highlight: DiffHighlight,
    /// Folds long unchanged stretches into `N unchanged lines` rows.
    pub collapse_unchanged: bool,
    /// Scrolls both sides together (off: each side scrolls on its own).
    pub sync_scroll: bool,
    /// Chunks ticked for partial staging.
    pub selected: BTreeSet<usize>,
}

impl DiffTab {
    pub(crate) fn new(path: PathBuf, diff: FileDiff, viewer: DiffViewer) -> Self {
        let old_analysis = analyze(&path, &diff.old_text);
        let new_analysis = analyze(&path, &diff.new_text);
        let reveal = (!diff.hunks.is_empty()).then_some(0);
        Self {
            path,
            current: reveal,
            reveal,
            diff,
            viewer,
            old_analysis,
            new_analysis,
            new_revision: None,
            whitespace: WhitespaceMode::default(),
            highlight: DiffHighlight::default(),
            collapse_unchanged: false,
            sync_scroll: true,
            selected: BTreeSet::new(),
        }
    }

    /// Replaces the compared content, keeping viewer and position.
    fn update(&mut self, diff: FileDiff) {
        let diff = diff.with_whitespace(self.whitespace);
        if diff == self.diff {
            return;
        }
        if diff.old_text != self.diff.old_text {
            self.old_analysis = analyze(&self.path, &diff.old_text);
        }
        if diff.new_text != self.diff.new_text {
            self.new_analysis = analyze(&self.path, &diff.new_text);
        }
        self.current = self
            .current
            .filter(|&index| index < diff.hunks.len())
            .or((!diff.hunks.is_empty()).then_some(0));
        self.selected.retain(|&index| index < diff.hunks.len());
        self.diff = diff;
    }
}

/// Wakes the UI when background work finishes. Installed by the UI layer once
/// its event loop exists; core never depends on a UI toolkit.
#[derive(Clone, Default)]
pub struct RepaintHook(Arc<Mutex<Option<Notify>>>);

impl RepaintHook {
    pub fn set(&self, notify: Notify) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = Some(notify);
        }
    }

    pub(crate) fn notifier(&self) -> Notify {
        let slot = self.0.clone();
        Arc::new(move || {
            if let Some(notify) = slot.lock().ok().and_then(|slot| slot.clone()) {
                notify();
            }
        })
    }
}

impl IdeState {
    /// Starts background Git status refresh for the open project.
    pub(crate) fn start_git_watcher(&mut self) {
        self.git_watcher = self.git.as_ref().map(|git| {
            StatusWatcher::spawn(git.root.clone(), STATUS_INTERVAL, self.repaint.notifier())
        });
    }

    /// Asks the background watcher to reload status now.
    pub fn request_git_refresh(&self) {
        if let Some(watcher) = &self.git_watcher {
            watcher.refresh();
        }
    }

    /// Applies status reloaded in the background (external changes).
    pub fn poll_git(&mut self) {
        let Some(status) = self.git_watcher.as_ref().and_then(StatusWatcher::latest) else {
            return;
        };
        if self.git.as_ref() == Some(&status) {
            return;
        }
        // Files created or deleted outside the IDE change the set of
        // untracked/added/deleted paths; rescan so the project tree shows them.
        let rescan = self.git.as_ref().map(structural_changes) != Some(structural_changes(&status));
        self.git = Some(status);
        self.invalidate_line_changes();
        self.invalidate_git_log();
        self.invalidate_annotations();
        self.refresh_pending_operation();
        if rescan && let Some(project) = &self.project {
            self.set_files(project.entries(20_000));
        }
        self.refresh_diff();
    }

    /// Converts a project-relative path to one relative to the Git root.
    fn repo_relative(&self, git: &RepositoryStatus, path: &Path) -> Result<PathBuf, String> {
        let project = self.project.as_ref().ok_or("no project is open")?;
        let git_root = git.root.canonicalize().unwrap_or_else(|_| git.root.clone());
        let absolute = project.root().join(path);
        absolute
            .strip_prefix(&git_root)
            .map(Path::to_path_buf)
            .map_err(|_| format!("{} is outside the Git repository", path.display()))
    }

    fn compute_diff(&self, path: &Path) -> Result<FileDiff, String> {
        let git = self
            .git
            .as_ref()
            .ok_or("the project is not a Git repository")?;
        let repo_path = self.repo_relative(git, path)?;
        // Compare against what the editor holds, unsaved edits included.
        let working = match self.documents.iter().find(|d| d.path == path) {
            Some(document) => Some(document.text.text()),
            None => git.working_text(&repo_path).map_err(|e| e.to_string())?,
        };
        git.diff_against_head(&repo_path, working)
            .map_err(|error| error.to_string())
    }

    /// Opens (or retargets) the diff tab for project-relative `path`.
    pub fn show_diff(&mut self, path: &Path) -> Result<(), String> {
        let diff = self.compute_diff(path)?;
        let viewer = self.diff.as_ref().map(|tab| tab.viewer).unwrap_or_default();
        let (whitespace, highlight) = self
            .diff
            .as_ref()
            .map(|tab| (tab.whitespace, tab.highlight))
            .unwrap_or_default();
        let diff = diff.with_whitespace(whitespace);
        let count = diff.hunks.len();
        let mut tab = DiffTab::new(path.to_owned(), diff, viewer);
        tab.whitespace = whitespace;
        tab.highlight = highlight;
        self.diff = Some(tab);
        self.diff_active = true;
        self.plugin_host.active_tab = None;
        self.status = format!(
            "{}: {count} difference{}",
            path.display(),
            if count == 1 { "" } else { "s" }
        );
        Ok(())
    }

    /// Recomputes the open diff after the file or HEAD changed. The tab stays
    /// open even if the file no longer differs.
    pub fn refresh_diff(&mut self) {
        let Some(path) = self
            .diff
            .as_ref()
            .filter(|tab| tab.new_revision.is_none())
            .map(|tab| tab.path.clone())
        else {
            return;
        };
        if let Ok(diff) = self.compute_diff(&path)
            && let Some(tab) = self.diff.as_mut()
        {
            tab.update(diff);
        }
    }

    /// Opens the diff tab for repository-relative `path` as changed by
    /// commit `hash` (against its first parent).
    pub fn show_commit_diff(&mut self, hash: &str, path: &Path) -> Result<(), String> {
        let git = self
            .git
            .as_ref()
            .ok_or("the project is not a Git repository")?;
        let commit = self
            .git_log
            .commits
            .iter()
            .find(|commit| commit.hash == hash)
            .ok_or_else(|| format!("commit {hash} is not in the loaded log"))?;
        let parent = commit.parents.first().cloned();
        let short = commit.short.clone();
        let diff = git
            .diff_revisions(path, parent.as_deref(), hash)
            .map_err(|error| error.to_string())?;
        let viewer = self.diff.as_ref().map(|tab| tab.viewer).unwrap_or_default();
        let (whitespace, highlight) = self
            .diff
            .as_ref()
            .map(|tab| (tab.whitespace, tab.highlight))
            .unwrap_or_default();
        let mut tab = DiffTab::new(path.to_owned(), diff.with_whitespace(whitespace), viewer);
        tab.whitespace = whitespace;
        tab.highlight = highlight;
        tab.new_revision = Some(short);
        self.diff = Some(tab);
        self.diff_active = true;
        Ok(())
    }

    /// Stages the ticked chunks of the open diff (`hunks`) of project-relative
    /// `path`, leaving the other changes of the file unstaged.
    pub fn stage_hunks(&mut self, path: &Path, hunks: &[usize]) -> Result<(), String> {
        let git = self
            .git
            .as_ref()
            .ok_or("the project is not a Git repository")?;
        let tab = self
            .diff
            .as_ref()
            .filter(|tab| tab.path == path && tab.new_revision.is_none())
            .ok_or("no working-copy diff is open for this file")?;
        if let Some(bad) = hunks.iter().find(|&&index| index >= tab.diff.hunks.len()) {
            return Err(format!("the diff has no chunk {bad}"));
        }
        let repo_path = self.repo_relative(git, path)?;
        git.stage_text(&repo_path, &tab.diff.old_with_hunks(hunks))
            .map_err(|error| error.to_string())?;
        if let Some(tab) = self.diff.as_mut() {
            tab.selected.clear();
        }
        self.status = format!(
            "Staged {} chunk{} of {}",
            hunks.len(),
            if hunks.len() == 1 { "" } else { "s" },
            path.display()
        );
        self.refresh_git()
    }

    /// Copies chunk `index` from the left side into the working copy (the
    /// `>>` arrow of the diff viewer): the editor buffer when the file is open,
    /// else the file on disk.
    pub fn apply_chunk(&mut self, index: usize) -> Result<(), String> {
        let tab = self
            .diff
            .as_ref()
            .filter(|tab| tab.new_revision.is_none())
            .ok_or("only the working copy can be changed")?;
        let text = tab
            .diff
            .new_with_hunk_reverted(index)
            .ok_or_else(|| format!("the diff has no chunk {index}"))?;
        let path = tab.path.clone();
        if let Some(document) = self.documents.iter_mut().find(|d| d.path == path) {
            let len = document.text.len_chars();
            document
                .text
                .edit(0, len, &text)
                .map_err(|error| error.to_string())?;
            let source = document.text.text();
            document.refresh_analysis(&source);
        } else {
            let root = self.project.as_ref().ok_or("no project is open")?.root();
            std::fs::write(root.join(&path), &text).map_err(|error| error.to_string())?;
        }
        self.refresh_diff();
        self.status = format!("Applied chunk {} to the working copy", index + 1);
        Ok(())
    }

    pub fn close_diff(&mut self) {
        self.diff = None;
        self.diff_active = false;
    }

    fn navigate_diff(&mut self, forward: bool) -> Result<(), String> {
        let tab = self.diff.as_mut().ok_or("no diff is open")?;
        let target = if forward {
            tab.diff.next_hunk(tab.current)
        } else {
            tab.diff.previous_hunk(tab.current)
        };
        match target {
            Some(index) => {
                tab.current = Some(index);
                tab.reveal = Some(index);
                Ok(())
            }
            None if tab.diff.hunks.is_empty() => Err("Contents are identical".into()),
            None => Err(if forward {
                "No next difference".into()
            } else {
                "No previous difference".into()
            }),
        }
    }
}

/// Changes that add or remove files (as opposed to editing them).
fn structural_changes(git: &RepositoryStatus) -> Vec<&Path> {
    git.changes
        .iter()
        .filter(|change| change.kind != ChangeKind::Modified)
        .map(|change| change.path.as_path())
        .collect()
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let diff_active = |state: &IdeState| state.diff_active && state.diff.is_some();
    registry
        .register(Command::new(
            spec(
                "git.showDiff",
                "Show Diff",
                "Git",
                Some("⌘D"),
                r#"{"path":"string"}"#,
            ),
            |state: &IdeState| state.git.is_some(),
            |state: &mut IdeState, params| {
                let path = PathBuf::from(string_param(&params, "path")?);
                state.show_diff(&path).map_err(CommandError::Failed)?;
                let count = state.diff.as_ref().map_or(0, |tab| tab.diff.hunks.len());
                Ok(json!({"differences": count}))
            },
        ))
        .expect("unique built-in id");
    for (id, label, shortcut, forward) in [
        ("diff.nextChange", "Next Difference", "F7", true),
        ("diff.previousChange", "Previous Difference", "⇧F7", false),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Diff", Some(shortcut), "{}"),
                diff_active,
                move |state: &mut IdeState, _| {
                    state.navigate_diff(forward).map_err(CommandError::Failed)?;
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "diff.setViewer",
                "Diff Viewer Mode",
                "Diff",
                None,
                r#"{"viewer":"sideBySide|unified"}"#,
            ),
            |state: &IdeState| state.diff.is_some(),
            |state: &mut IdeState, params| {
                let id = string_param(&params, "viewer")?;
                let viewer = DiffViewer::parse(&id).ok_or_else(|| {
                    CommandError::InvalidParams(format!("unknown diff viewer `{id}`"))
                })?;
                if let Some(tab) = state.diff.as_mut() {
                    tab.viewer = viewer;
                    tab.reveal = tab.current;
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "diff.setWhitespace",
                "Diff Whitespace Mode",
                "Diff",
                None,
                r#"{"mode":"none|trim|all"}"#,
            ),
            |state: &IdeState| state.diff.is_some(),
            |state: &mut IdeState, params| {
                let id = string_param(&params, "mode")?;
                let mode = WhitespaceMode::parse(&id).ok_or_else(|| {
                    CommandError::InvalidParams(format!("unknown whitespace mode `{id}`"))
                })?;
                if let Some(tab) = state.diff.as_mut() {
                    tab.whitespace = mode;
                    tab.diff = tab.diff.clone().with_whitespace(mode);
                    tab.selected.clear();
                    tab.current = (!tab.diff.hunks.is_empty()).then_some(0);
                    tab.reveal = tab.current;
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "diff.setHighlight",
                "Diff Highlight Mode",
                "Diff",
                None,
                r#"{"mode":"words|lines|none"}"#,
            ),
            |state: &IdeState| state.diff.is_some(),
            |state: &mut IdeState, params| {
                let id = string_param(&params, "mode")?;
                let mode = DiffHighlight::parse(&id).ok_or_else(|| {
                    CommandError::InvalidParams(format!("unknown highlight mode `{id}`"))
                })?;
                if let Some(tab) = state.diff.as_mut() {
                    tab.highlight = mode;
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "diff.toggleCollapseUnchanged",
                "Collapse Unchanged Fragments",
                "Diff",
                None,
                "{}",
            ),
            |state: &IdeState| state.diff.is_some(),
            |state: &mut IdeState, _params| {
                if let Some(tab) = state.diff.as_mut() {
                    tab.collapse_unchanged = !tab.collapse_unchanged;
                    tab.reveal = tab.current;
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "diff.toggleSyncScroll",
                "Synchronize Scrolling",
                "Diff",
                None,
                "{}",
            ),
            |state: &IdeState| state.diff.is_some(),
            |state: &mut IdeState, _params| {
                if let Some(tab) = state.diff.as_mut() {
                    tab.sync_scroll = !tab.sync_scroll;
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "diff.toggleChunk",
                "Select Chunk for Staging",
                "Diff",
                None,
                r#"{"index":0}"#,
            ),
            |state: &IdeState| state.diff.is_some(),
            |state: &mut IdeState, params| {
                let index = params
                    .get("index")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| CommandError::InvalidParams("`index` is required".into()))?
                    as usize;
                let tab = state.diff.as_mut().expect("enabled only with a diff");
                if index >= tab.diff.hunks.len() {
                    return Err(CommandError::InvalidParams(format!(
                        "the diff has no chunk {index}"
                    )));
                }
                if !tab.selected.remove(&index) {
                    tab.selected.insert(index);
                }
                Ok(json!({"selected": tab.selected.len()}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "diff.applyChunk",
                "Apply Chunk to Working Copy",
                "Diff",
                None,
                r#"{"index":0}"#,
            ),
            |state: &IdeState| {
                state
                    .diff
                    .as_ref()
                    .is_some_and(|t| t.new_revision.is_none())
            },
            |state: &mut IdeState, params| {
                let index = params
                    .get("index")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| CommandError::InvalidParams("`index` is required".into()))?
                    as usize;
                state.apply_chunk(index).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("diff.activate", "Select Diff Tab", "Diff", None, "{}"),
            |state: &IdeState| state.diff.is_some(),
            |state: &mut IdeState, _| {
                state.diff_active = true;
                state.plugin_host.active_tab = None;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("diff.close", "Close Diff", "Diff", None, "{}"),
            |state: &IdeState| state.diff.is_some(),
            |state: &mut IdeState, _| {
                state.close_diff();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::built_in_commands;
    use crate::{DiffViewer, IdeState};
    use serde_json::{Value, json};
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, Instant};

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

    fn repo(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-vcs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "test@example.invalid"]);
        git(&root, &["config", "user.name", "Archipel IDE Test"]);
        fs::write(root.join("App.java"), "a\nb\nc\nd\ne\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "initial"]);
        root
    }

    #[test]
    fn shows_diff_and_navigates_changes() {
        let root = repo("diff");
        fs::write(root.join("App.java"), "A\nb\nc\nd\nE\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        assert!(!commands.is_enabled("diff.nextChange", &state));
        let result = commands
            .execute("git.showDiff", &mut state, json!({"path": "App.java"}))
            .unwrap();
        assert_eq!(result["differences"], 2);
        assert!(state.diff_active);
        assert_eq!(state.diff.as_ref().unwrap().current, Some(0));
        commands
            .execute("diff.nextChange", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.diff.as_ref().unwrap().current, Some(1));
        assert!(
            commands
                .execute("diff.nextChange", &mut state, Value::Null)
                .is_err()
        );
        commands
            .execute("diff.previousChange", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.diff.as_ref().unwrap().reveal, Some(0));
        commands
            .execute("diff.setViewer", &mut state, json!({"viewer": "unified"}))
            .unwrap();
        assert_eq!(state.diff.as_ref().unwrap().viewer, DiffViewer::Unified);
        assert!(
            commands
                .execute("diff.setViewer", &mut state, json!({"viewer": "x"}))
                .is_err()
        );
        // Opening a file switches away from the diff tab, which stays open.
        commands
            .execute("editor.openFile", &mut state, json!({"path": "App.java"}))
            .unwrap();
        assert!(!state.diff_active);
        commands
            .execute("diff.activate", &mut state, Value::Null)
            .unwrap();
        assert!(state.diff_active);
        // Unsaved editor content is what gets compared.
        commands
            .execute(
                "editor.insertText",
                &mut state,
                json!({"position": 0, "text": "z"}),
            )
            .unwrap();
        state.refresh_diff();
        assert!(state.diff.as_ref().unwrap().diff.new_text.starts_with("zA"));
        commands
            .execute("diff.close", &mut state, Value::Null)
            .unwrap();
        assert!(state.diff.is_none() && !state.diff_active);
        assert!(
            commands
                .execute("git.showDiff", &mut state, json!({"path": "missing.txt"}))
                .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn collapse_unchanged_and_sync_scroll_toggle() {
        let root = repo("toggles");
        fs::write(root.join("App.java"), "A\nb\nc\nd\nE\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        assert!(
            commands
                .execute("diff.toggleSyncScroll", &mut state, json!({}))
                .is_err(),
            "needs an open diff"
        );
        commands
            .execute("git.showDiff", &mut state, json!({"path": "App.java"}))
            .unwrap();
        let tab = state.diff.as_ref().unwrap();
        assert!(tab.sync_scroll && !tab.collapse_unchanged);
        commands
            .execute("diff.toggleSyncScroll", &mut state, json!({}))
            .unwrap();
        commands
            .execute("diff.toggleCollapseUnchanged", &mut state, json!({}))
            .unwrap();
        let tab = state.diff.as_ref().unwrap();
        assert!(!tab.sync_scroll && tab.collapse_unchanged);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stages_and_applies_single_chunks() {
        let root = repo("chunks");
        fs::write(root.join("App.java"), "A\nb\nc\nd\nE\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("git.showDiff", &mut state, json!({"path": "App.java"}))
            .unwrap();
        commands
            .execute("diff.toggleChunk", &mut state, json!({"index": 1}))
            .unwrap();
        assert!(state.diff.as_ref().unwrap().selected.contains(&1));
        assert!(
            commands
                .execute("diff.toggleChunk", &mut state, json!({"index": 9}))
                .is_err()
        );
        commands
            .execute(
                "git.stage",
                &mut state,
                json!({"path": "App.java", "hunks": [1]}),
            )
            .unwrap();
        let staged = std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["show", ":App.java"])
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&staged.stdout), "a\nb\nc\nd\nE\n");
        // The working copy is untouched; chunk 0 can be reverted from the left side.
        commands
            .execute("diff.applyChunk", &mut state, json!({"index": 0}))
            .unwrap();
        assert_eq!(
            fs::read_to_string(root.join("App.java")).unwrap(),
            "a\nb\nc\nd\nE\n"
        );
        commands
            .execute("diff.setWhitespace", &mut state, json!({"mode": "all"}))
            .unwrap();
        commands
            .execute("diff.setHighlight", &mut state, json!({"mode": "none"}))
            .unwrap();
        assert!(
            commands
                .execute("diff.setWhitespace", &mut state, json!({"mode": "x"}))
                .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn picks_up_external_changes_in_the_background() {
        let root = repo("watch");
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        assert!(state.git.as_ref().unwrap().changes.is_empty());
        // Changed by another tool, not through the IDE.
        fs::write(root.join("App.java"), "changed\n").unwrap();
        state.request_git_refresh();
        let deadline = Instant::now() + Duration::from_secs(10);
        while state.git.as_ref().unwrap().changes.is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
            state.poll_git();
        }
        assert_eq!(state.git.as_ref().unwrap().changes.len(), 1);
        // A file created outside the IDE shows up in the project tree.
        fs::write(root.join("Notes.txt"), "new\n").unwrap();
        state.request_git_refresh();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !state
            .files
            .iter()
            .any(|f| f.relative_path.ends_with("Notes.txt"))
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(20));
            state.poll_git();
        }
        assert!(
            state
                .files
                .iter()
                .any(|f| f.relative_path.ends_with("Notes.txt"))
        );
        fs::remove_dir_all(root).unwrap();
    }
}

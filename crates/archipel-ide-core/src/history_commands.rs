//! Local History (IJ-ED-028, IJ-TW-034): the host records a revision before
//! it overwrites a file on save and when an external change is applied, and
//! offers Show History, compare, revert, labels and recent changes.
//!
//! The store is `archipel_ide_index::history` (content-addressed blobs and a
//! per-path revision log) under `<cache>/history/<project hash>`; without a
//! cache directory (tests, headless use) history is off.

use crate::{IdeState, spec};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_git::FileDiff;
use archipel_ide_index::history::{Cause, History, Retention, Revision};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Days of history kept unless the setting says otherwise.
pub const DEFAULT_RETENTION_DAYS: i64 = 5;

/// One revision as the dialog lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionRow {
    pub id: u64,
    pub path: PathBuf,
    pub time_ms: u64,
    pub label: String,
    pub len: u64,
    /// The revision has content to compare or restore (not a deletion or label).
    pub has_content: bool,
}

impl RevisionRow {
    fn of(revision: &Revision) -> Self {
        Self {
            id: revision.id,
            path: revision.path.clone(),
            time_ms: revision.time_ms,
            label: revision.cause.label(),
            len: revision.len,
            has_content: revision.blob.is_some(),
        }
    }
}

/// One line of the diff preview of the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewLine {
    /// `' '` context, `'+'` only in the current text, `'-'` only in the revision.
    pub mark: char,
    pub text: String,
}

/// The Show History dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryDialog {
    /// `None` shows the whole project (recent changes).
    pub path: Option<PathBuf>,
    pub revisions: Vec<RevisionRow>,
    pub selected: Option<usize>,
    pub preview: Vec<PreviewLine>,
    /// Why the selected revision has no preview.
    pub note: Option<String>,
}

/// Local History state of the app.
#[derive(Default)]
pub struct HistoryState {
    store: Option<Arc<History>>,
    root: Option<PathBuf>,
    pub dialog: Option<HistoryDialog>,
    /// Why history is off or failed.
    pub error: Option<String>,
}

impl std::fmt::Debug for HistoryState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoryState")
            .field("enabled", &self.store.is_some())
            .field("dialog", &self.dialog)
            .finish()
    }
}

impl HistoryState {
    pub fn is_enabled(&self) -> bool {
        self.store.is_some()
    }

    pub fn store(&self) -> Option<&Arc<History>> {
        self.store.as_ref()
    }

    /// Opens a history rooted at `dir` directly (tests and embedding).
    pub fn open_at(&mut self, dir: &Path) -> Result<(), String> {
        let store = History::open(dir).map_err(|error| error.to_string())?;
        self.store = Some(Arc::new(store));
        Ok(())
    }
}

fn utf8(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes)
        .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

impl IdeState {
    /// Opens the history of the open project (and closes it with the
    /// project). Called every frame from `sync_index_project`.
    pub(crate) fn sync_history_project(&mut self) {
        let root = self.project.as_ref().map(|p| p.root().to_path_buf());
        if root == self.analysis.history.root {
            return;
        }
        self.analysis.history.root = root.clone();
        self.analysis.history.dialog = None;
        let Some(root) = root else {
            self.analysis.history.store = None;
            return;
        };
        // An explicitly opened history (tests, embedding) stays.
        if self.analysis.history.store.is_some() {
            return;
        }
        let Some(cache) = self.analysis.index_cache_dir.clone() else {
            return;
        };
        let dir = cache.join("history").join(format!(
            "{:016x}",
            archipel_ide_lsp::registry::stable_hash(&root.to_string_lossy())
        ));
        match History::open(&dir) {
            Ok(store) => {
                let days = self
                    .settings
                    .int(archipel_ide_settings::ids::LOCAL_HISTORY_DAYS)
                    .unwrap_or(DEFAULT_RETENTION_DAYS)
                    .clamp(1, 365);
                let _ = store.purge(Retention {
                    max_age_ms: (days as u64) * 24 * 3600 * 1000,
                    ..Retention::default()
                });
                self.analysis.history.store = Some(Arc::new(store));
                self.analysis.history.error = None;
            }
            Err(error) => self.analysis.history.error = Some(error.to_string()),
        }
    }

    /// The disk text to keep before the first save of `path` (history of a
    /// file starts with its content before the editor first overwrites it).
    pub(crate) fn history_previous(&self, path: &Path) -> Option<String> {
        let store = self.analysis.history.store.as_ref()?;
        if store.latest(path).is_some() {
            return None;
        }
        self.project.as_ref()?.read_text(path).ok()
    }

    /// A save wrote `text` to `path`; `previous` is what `history_previous` returned.
    pub(crate) fn history_saved(&mut self, path: &Path, previous: Option<&str>, text: &str) {
        let Some(store) = self.analysis.history.store.clone() else {
            return;
        };
        if let Some(previous) = previous {
            let _ = store.record(path, previous.as_bytes(), Cause::Initial);
        }
        if let Err(error) = store.record(path, text.as_bytes(), Cause::Save) {
            self.analysis.history.error = Some(error.to_string());
        }
    }

    /// The disk text of `path` replaced the editor's (an outside change, or
    /// a reload from disk). `buffer` is what the editor held.
    pub(crate) fn history_external(
        &mut self,
        path: &Path,
        buffer: &str,
        was_dirty: bool,
        disk: &str,
    ) {
        let Some(store) = self.analysis.history.store.clone() else {
            return;
        };
        if store.latest(path).is_none() {
            let cause = if was_dirty {
                Cause::Revert
            } else {
                Cause::Initial
            };
            let _ = store.record(path, buffer.as_bytes(), cause);
        } else if was_dirty {
            let _ = store.record(path, buffer.as_bytes(), Cause::Revert);
        }
        if let Err(error) = store.record(path, disk.as_bytes(), Cause::External) {
            self.analysis.history.error = Some(error.to_string());
        }
    }

    /// Records the content of every open document before a refactoring, VCS
    /// operation or other bulk change (`cause` names it).
    pub fn history_checkpoint(&mut self, cause: &Cause) {
        let Some(store) = self.analysis.history.store.clone() else {
            return;
        };
        for document in &self.documents {
            let text = document.text.text();
            let _ = store.record(&document.path, text.as_bytes(), cause.clone());
        }
    }

    fn history_current_text(&self, path: &Path) -> Option<String> {
        if let Some(document) = self.documents.iter().find(|d| d.path == path) {
            return Some(document.text.text());
        }
        self.project.as_ref()?.read_text(path).ok()
    }

    /// Opens the Show History dialog for `path` (the project when `None`).
    pub fn history_show(&mut self, path: Option<&Path>) -> Result<(), String> {
        let store = self
            .analysis
            .history
            .store
            .clone()
            .ok_or("Local History is not available (no cache directory)")?;
        let revisions = match path {
            Some(path) => store.revisions(path),
            None => store.recent(200),
        };
        let revisions: Vec<RevisionRow> = revisions.iter().map(RevisionRow::of).collect();
        self.analysis.history.dialog = Some(HistoryDialog {
            path: path.map(Path::to_path_buf),
            selected: None,
            preview: Vec::new(),
            note: None,
            revisions,
        });
        if !self
            .analysis
            .history
            .dialog
            .as_ref()
            .is_some_and(|d| d.revisions.is_empty())
        {
            self.history_select(0)?;
        }
        Ok(())
    }

    fn revision_by_id(&self, id: u64, path: &Path) -> Result<(Revision, Arc<History>), String> {
        let store = self
            .analysis
            .history
            .store
            .clone()
            .ok_or("Local History is off")?;
        let revision = store
            .revisions(path)
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| format!("revision {id} of {} no longer exists", path.display()))?;
        Ok((revision, store))
    }

    /// Selects row `index` of the dialog and computes its diff against the
    /// current text.
    pub fn history_select(&mut self, index: usize) -> Result<(), String> {
        let (row, path) = {
            let dialog = self
                .analysis
                .history
                .dialog
                .as_ref()
                .ok_or("no history dialog")?;
            let row = dialog
                .revisions
                .get(index)
                .ok_or_else(|| format!("no revision at {index}"))?
                .clone();
            (row.clone(), row.path)
        };
        let mut preview = Vec::new();
        let mut note = None;
        if !row.has_content {
            note = Some(if path.as_os_str().is_empty() {
                "A label has no content.".to_string()
            } else {
                "The file was deleted at this point.".to_string()
            });
        } else {
            let (revision, store) = self.revision_by_id(row.id, &path)?;
            match store.content(&revision) {
                Ok(Some(old)) => {
                    let old = utf8(old);
                    match self.history_current_text(&path) {
                        Some(current) => preview = preview_lines(&path, &old, &current),
                        None => {
                            note = Some("The file no longer exists; reverting recreates it.".into())
                        }
                    }
                }
                Ok(None) => note = Some("No content.".into()),
                Err(error) => note = Some(format!("Cannot read this revision: {error}")),
            }
        }
        if let Some(dialog) = self.analysis.history.dialog.as_mut() {
            dialog.selected = Some(index);
            dialog.preview = preview;
            dialog.note = note;
        }
        Ok(())
    }

    /// Opens the diff viewer: the revision against the current text.
    pub fn history_compare(&mut self, id: u64, path: &Path) -> Result<(), String> {
        let (revision, store) = self.revision_by_id(id, path)?;
        let old = utf8(
            store
                .content(&revision)
                .map_err(|e| e.to_string())?
                .ok_or("this revision has no content")?,
        );
        let current = self.history_current_text(path).unwrap_or_default();
        let diff = FileDiff::compute(path.to_path_buf(), Some("history".into()), old, current);
        let mut tab = crate::DiffTab::new(path.to_path_buf(), diff, Default::default());
        // Not a Git comparison: never refreshed against HEAD.
        tab.new_revision = Some("current".into());
        self.diff = Some(tab);
        self.diff_active = true;
        self.plugin_host.active_tab = None;
        self.analysis.history.dialog = None;
        Ok(())
    }

    /// Restores `path` to a revision. The current content is kept as a
    /// revision first, so a revert can be reverted.
    pub fn history_revert(&mut self, id: u64, path: &Path) -> Result<(), String> {
        let (revision, store) = self.revision_by_id(id, path)?;
        let text = utf8(
            store
                .content(&revision)
                .map_err(|e| e.to_string())?
                .ok_or("this revision has no content")?,
        );
        if let Some(current) = self.history_current_text(path) {
            let _ = store.record(path, current.as_bytes(), Cause::Revert);
        }
        match self.documents.iter().position(|d| d.path == path) {
            Some(index) => {
                let document = &mut self.documents[index];
                document
                    .text
                    .replace_from_snapshot(&text)
                    .map_err(|e| e.to_string())?;
                let len = document.text.len_chars();
                document.selections.clamp(len);
                document.refresh_analysis(&text);
                self.save(index)?;
            }
            None => {
                self.project
                    .as_ref()
                    .ok_or("no project is open")?
                    .write_text(path, &text)
                    .map_err(|e| e.to_string())?;
                let _ = store.record(path, text.as_bytes(), Cause::Save);
            }
        }
        self.status = format!("Reverted {} to {}", path.display(), revision.cause.label());
        // Refresh the dialog list (the revert is a new revision).
        if self.analysis.history.dialog.is_some() {
            self.history_show(Some(path))?;
        }
        Ok(())
    }
}

/// A unified-style preview: changed hunks of `old` -> `new` with context.
fn preview_lines(path: &Path, old: &str, new: &str) -> Vec<PreviewLine> {
    const CONTEXT: usize = 2;
    const MAX_LINES: usize = 4000;
    let diff = FileDiff::compute(path.to_path_buf(), None, old.to_string(), new.to_string());
    let old_lines = archipel_ide_git::split_lines(old);
    let new_lines = archipel_ide_git::split_lines(new);
    let line =
        |lines: &[&str], at: usize| lines.get(at).map(|l| (*l).to_string()).unwrap_or_default();
    let mut out = Vec::new();
    let mut old_at = 0usize;
    for hunk in &diff.hunks {
        let start = hunk.old.start.saturating_sub(CONTEXT).max(old_at);
        if !out.is_empty() && start > old_at {
            out.push(PreviewLine {
                mark: ' ',
                text: "…".into(),
            });
        }
        for at in start..hunk.old.start {
            out.push(PreviewLine {
                mark: ' ',
                text: line(&old_lines, at),
            });
        }
        for at in hunk.old.clone() {
            out.push(PreviewLine {
                mark: '-',
                text: line(&old_lines, at),
            });
        }
        for at in hunk.new.clone() {
            out.push(PreviewLine {
                mark: '+',
                text: line(&new_lines, at),
            });
        }
        let after_end = (hunk.old.end + CONTEXT).min(old_lines.len());
        for at in hunk.old.end..after_end {
            out.push(PreviewLine {
                mark: ' ',
                text: line(&old_lines, at),
            });
        }
        old_at = after_end;
        if out.len() >= MAX_LINES {
            out.truncate(MAX_LINES);
            out.push(PreviewLine {
                mark: ' ',
                text: "…".into(),
            });
            break;
        }
    }
    out
}

fn path_param(state: &IdeState, params: &Value) -> Result<PathBuf, CommandError> {
    if let Some(path) = params.get("path").and_then(Value::as_str) {
        return Ok(PathBuf::from(path));
    }
    state
        .active_document
        .and_then(|index| state.documents.get(index))
        .map(|document| document.path.clone())
        .ok_or_else(|| CommandError::Failed("no file is open".into()))
}

fn id_param(params: &Value) -> Result<u64, CommandError> {
    params
        .get("revision")
        .and_then(Value::as_u64)
        .ok_or_else(|| CommandError::InvalidParams("expected `revision`".into()))
}

fn rows_json(rows: &[RevisionRow]) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| {
                json!({
                    "id": row.id,
                    "path": row.path,
                    "timeMs": row.time_ms,
                    "label": row.label,
                    "bytes": row.len,
                    "hasContent": row.has_content,
                })
            })
            .collect(),
    )
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let enabled = |state: &IdeState| state.analysis.history.is_enabled();
    registry
        .register(Command::new(
            spec(
                "history.show",
                "Show History",
                "File",
                None,
                r#"{"path":"project-relative path, the active file when omitted"}"#,
            ),
            enabled,
            |state: &mut IdeState, params| {
                let path = path_param(state, &params)?;
                state
                    .history_show(Some(&path))
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "history.project",
                "Show Project History",
                "File",
                None,
                "{}",
            ),
            enabled,
            |state: &mut IdeState, _| {
                state.history_show(None).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("history.recent", "Recent Changes", "File", None, "{}"),
            enabled,
            |state: &mut IdeState, _| {
                state.history_show(None).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "history.select",
                "Select History Revision",
                "File",
                None,
                r#"{"index":0}"#,
            ),
            |state: &IdeState| state.analysis.history.dialog.is_some(),
            |state: &mut IdeState, params| {
                let index = params
                    .get("index")
                    .and_then(Value::as_u64)
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or_else(|| CommandError::InvalidParams("expected `index`".into()))?;
                state.history_select(index).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "history.compare",
                "Compare with Current",
                "File",
                None,
                r#"{"revision":0,"path":"relative path"}"#,
            ),
            enabled,
            |state: &mut IdeState, params| {
                let path = path_param(state, &params)?;
                state
                    .history_compare(id_param(&params)?, &path)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "history.revert",
                "Revert to Revision",
                "File",
                None,
                r#"{"revision":0,"path":"relative path"}"#,
            ),
            enabled,
            |state: &mut IdeState, params| {
                let path = path_param(state, &params)?;
                state
                    .history_revert(id_param(&params)?, &path)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "history.label",
                "Put Label",
                "File",
                None,
                r#"{"text":"label text, \"Label\" when omitted"}"#,
            ),
            enabled,
            |state: &mut IdeState, params| {
                // The menu entry has no prompt: an unnamed label is "Label".
                let text = params
                    .get("text")
                    .and_then(Value::as_str)
                    .map_or_else(|| "Label".to_string(), str::to_string);
                if text.trim().is_empty() {
                    return Err(CommandError::InvalidParams("the label is empty".into()));
                }
                let store = state
                    .analysis
                    .history
                    .store()
                    .cloned()
                    .ok_or_else(|| CommandError::Failed("Local History is off".into()))?;
                state.history_checkpoint(&Cause::Label(text.trim().to_string()));
                store
                    .label(text.trim())
                    .map_err(|e| CommandError::Failed(e.to_string()))?;
                state.status = format!("Label `{}` put", text.trim());
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("history.close", "Close History", "File", None, "{}"),
            |state: &IdeState| state.analysis.history.dialog.is_some(),
            |state: &mut IdeState, _| {
                state.analysis.history.dialog = None;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "history.list",
                "List History Revisions",
                "File",
                None,
                r#"{"path":"project-relative path; the whole project when omitted"}"#,
            ),
            enabled,
            |state: &mut IdeState, params| {
                let store = state
                    .analysis
                    .history
                    .store()
                    .cloned()
                    .ok_or_else(|| CommandError::Failed("Local History is off".into()))?;
                let revisions = match params.get("path").and_then(Value::as_str) {
                    Some(path) => store.revisions(Path::new(path)),
                    None => store.recent(200),
                };
                let rows: Vec<RevisionRow> = revisions.iter().map(RevisionRow::of).collect();
                Ok(rows_json(&rows))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("history.purge", "Purge Local History", "File", None, "{}"),
            enabled,
            |state: &mut IdeState, _| {
                let store = state
                    .analysis
                    .history
                    .store()
                    .cloned()
                    .ok_or_else(|| CommandError::Failed("Local History is off".into()))?;
                let days = state
                    .settings
                    .int(archipel_ide_settings::ids::LOCAL_HISTORY_DAYS)
                    .unwrap_or(DEFAULT_RETENTION_DAYS)
                    .clamp(1, 365);
                let stats = store
                    .purge(Retention {
                        max_age_ms: (days as u64) * 24 * 3600 * 1000,
                        ..Retention::default()
                    })
                    .map_err(|e| CommandError::Failed(e.to_string()))?;
                Ok(json!({
                    "revisionsRemoved": stats.revisions_removed,
                    "blobsRemoved": stats.blobs_removed,
                }))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IdeState;
    use std::time::Duration;

    fn setup() -> (tempfile::TempDir, tempfile::TempDir, IdeState) {
        let project = tempfile::tempdir().expect("project");
        let store = tempfile::tempdir().expect("store");
        std::fs::create_dir_all(project.path().join("src")).expect("dir");
        std::fs::write(project.path().join("src/A.java"), "class A {}\n").expect("write");
        let mut state = IdeState {
            analysis: crate::AnalysisServices::new(archipel_ide_analysis::HostConfig::testing()),
            ..IdeState::default()
        };
        state
            .analysis
            .history
            .open_at(store.path())
            .expect("history");
        state.open_project(project.path()).expect("open");
        state.open_file(Path::new("src/A.java")).expect("open file");
        (project, store, state)
    }

    fn type_text(state: &mut IdeState, text: &str) {
        let document = &mut state.documents[0];
        let len = document.text.len_chars();
        document.text.edit(len, len, text).expect("edit");
        let source = document.text.text();
        document.refresh_analysis(&source);
    }

    fn labels(state: &IdeState, path: &Path) -> Vec<String> {
        state
            .analysis
            .history
            .store()
            .expect("store")
            .revisions(path)
            .iter()
            .map(|r| r.cause.label())
            .collect()
    }

    #[test]
    fn saves_record_revisions_starting_with_the_content_before_the_first_save() {
        let (_project, _store, mut state) = setup();
        let path = Path::new("src/A.java");
        type_text(&mut state, "// one\n");
        state.save(0).expect("save");
        assert_eq!(labels(&state, path), ["Saved", "Initial"]);
        // Saving unchanged content records nothing.
        state.save(0).expect("save again");
        assert_eq!(labels(&state, path).len(), 2);
        type_text(&mut state, "// two\n");
        state.save(0).expect("save");
        assert_eq!(labels(&state, path), ["Saved", "Saved", "Initial"]);
        let store = state.analysis.history.store().expect("store").clone();
        let revisions = store.revisions(path);
        assert_eq!(
            store.content(&revisions[2]).expect("c"),
            Some(b"class A {}\n".to_vec()),
            "the oldest revision is what the disk held before"
        );
        assert_eq!(
            store.content(&revisions[0]).expect("c"),
            Some(b"class A {}\n// one\n// two\n".to_vec())
        );
    }

    #[test]
    fn outside_changes_become_revisions_and_discarded_edits_are_kept() {
        let (project, _store, mut state) = setup();
        let path = Path::new("src/A.java");
        std::fs::write(project.path().join(path), "class A { int outside; }\n").expect("write");
        state.reload_from_disk(0).expect("reload");
        assert_eq!(labels(&state, path), ["External change", "Initial"]);
        // An unsaved edit discarded by a reload is kept as a revision.
        type_text(&mut state, "// unsaved work\n");
        std::fs::write(project.path().join(path), "class A { int again; }\n").expect("write");
        state.reload_from_disk(0).expect("reload");
        assert_eq!(
            labels(&state, path),
            [
                "External change",
                "Before revert",
                "External change",
                "Initial"
            ]
        );
        let store = state.analysis.history.store().expect("store").clone();
        let revisions = store.revisions(path);
        assert_eq!(
            store.content(&revisions[1]).expect("c"),
            Some(b"class A { int outside; }\n// unsaved work\n".to_vec())
        );
    }

    #[test]
    fn the_dialog_compares_reverts_and_labels() {
        let (project, _store, mut state) = setup();
        let commands = crate::built_in_commands();
        let path = Path::new("src/A.java");
        type_text(&mut state, "// edit\n");
        state.save(0).expect("save");
        commands
            .execute("history.show", &mut state, serde_json::json!({}))
            .expect("show");
        let dialog = state.analysis.history.dialog.clone().expect("dialog");
        assert_eq!(dialog.revisions.len(), 2);
        assert_eq!(dialog.selected, Some(0));
        assert!(
            dialog.preview.is_empty(),
            "the newest revision equals the current text"
        );
        commands
            .execute(
                "history.select",
                &mut state,
                serde_json::json!({"index": 1}),
            )
            .expect("select");
        let dialog = state.analysis.history.dialog.clone().expect("dialog");
        assert_eq!(
            dialog.preview,
            vec![
                PreviewLine {
                    mark: ' ',
                    text: "class A {}".into()
                },
                PreviewLine {
                    mark: '+',
                    text: "// edit".into()
                },
            ]
        );
        let initial = dialog.revisions[1].id;

        // Compare opens the diff viewer and closes the dialog.
        commands
            .execute(
                "history.compare",
                &mut state,
                serde_json::json!({"revision": initial, "path": path}),
            )
            .expect("compare");
        assert!(state.analysis.history.dialog.is_none());
        let diff = state.diff.as_ref().expect("diff tab");
        assert_eq!(diff.diff.old_text, "class A {}\n");
        assert_eq!(diff.diff.new_text, "class A {}\n// edit\n");

        // Revert restores the old text in the editor and on disk; unsaved
        // work it replaces is kept as a revision first.
        type_text(&mut state, "// unsaved\n");
        commands
            .execute(
                "history.revert",
                &mut state,
                serde_json::json!({"revision": initial, "path": path}),
            )
            .expect("revert");
        assert_eq!(state.documents[0].text.text(), "class A {}\n");
        assert_eq!(
            std::fs::read_to_string(project.path().join(path)).expect("read"),
            "class A {}\n"
        );
        let store = state.analysis.history.store().expect("store").clone();
        let kept = store
            .revisions(path)
            .into_iter()
            .find(|r| r.cause == Cause::Revert)
            .expect("the unsaved text was kept");
        assert_eq!(
            store.content(&kept).expect("c"),
            Some(b"class A {}\n// edit\n// unsaved\n".to_vec())
        );
        // ... which makes the revert itself revertable.
        commands
            .execute(
                "history.revert",
                &mut state,
                serde_json::json!({"revision": kept.id, "path": path}),
            )
            .expect("revert the revert");
        assert_eq!(
            state.documents[0].text.text(),
            "class A {}\n// edit\n// unsaved\n"
        );

        // Labels and the project-wide list.
        commands
            .execute(
                "history.label",
                &mut state,
                serde_json::json!({"text": "before release"}),
            )
            .expect("label");
        let listed = commands
            .execute("history.list", &mut state, serde_json::json!({}))
            .expect("list");
        assert_eq!(listed[0]["label"], "Label: before release");
        assert!(listed.as_array().expect("array").len() >= 5);
        assert!(
            commands
                .execute(
                    "history.label",
                    &mut state,
                    serde_json::json!({"text": "  "})
                )
                .is_err()
        );
        let purged = commands
            .execute("history.purge", &mut state, serde_json::json!({}))
            .expect("purge");
        assert_eq!(
            purged["revisionsRemoved"], 0,
            "nothing is older than the retention"
        );
        // Unknown revisions are errors, not panics.
        assert!(
            commands
                .execute(
                    "history.revert",
                    &mut state,
                    serde_json::json!({"revision": 9999, "path": path})
                )
                .is_err()
        );
    }

    #[test]
    fn history_is_off_without_a_store_and_checkpoints_cover_open_documents() {
        let project = tempfile::tempdir().expect("project");
        std::fs::write(project.path().join("a.txt"), "a\n").expect("write");
        let mut state = IdeState::default();
        state.open_project(project.path()).expect("open");
        state.open_file(Path::new("a.txt")).expect("open file");
        let commands = crate::built_in_commands();
        assert!(
            commands
                .execute("history.show", &mut state, serde_json::json!({}))
                .is_err()
        );
        state.save(0).expect("save works without history");
        // Switching it on later: a checkpoint records every open buffer.
        let store = tempfile::tempdir().expect("store");
        state
            .analysis
            .history
            .open_at(store.path())
            .expect("history");
        state.history_checkpoint(&Cause::Refactoring("Rename a".into()));
        assert_eq!(
            labels(&state, Path::new("a.txt")),
            ["Refactoring: Rename a"]
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

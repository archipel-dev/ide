//! Keeps open editors consistent with the file system.
//!
//! A [`ProjectWatcher`] reports changed paths. For each open document the disk
//! content is compared with what the editor last loaded or saved (a content
//! hash, so the IDE's own saves are not mistaken for outside edits):
//! unmodified buffers reload silently (undoably); modified buffers get a
//! [`DiskConflict`] that blocks saving until the user picks a side, so autosave
//! can never overwrite an outside change.

use crate::{IdeState, spec, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_project::ProjectWatcher;
use serde_json::{Value, json};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

type WakeFn = Box<dyn Fn() + Send + Sync>;

/// Wakes the UI from background threads (file watcher). The UI installs the
/// callback once it has a window; until then waking is a no-op.
#[derive(Clone, Default)]
pub struct Waker(Arc<Mutex<Option<WakeFn>>>);

impl Waker {
    pub fn set(&self, wake: impl Fn() + Send + Sync + 'static) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = Some(Box::new(wake));
        }
    }

    pub fn wake(&self) {
        if let Ok(slot) = self.0.lock()
            && let Some(wake) = slot.as_ref()
        {
            wake();
        }
    }
}

/// An outside change that collides with unsaved edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskConflict {
    /// The file changed on disk while the buffer has unsaved changes.
    Modified,
    /// The file was deleted on disk while the buffer has unsaved changes.
    Deleted,
}

/// Which side a line of [`IdeState::conflict_diff`] belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffTag {
    Same,
    /// Only in the file system content.
    Disk,
    /// Only in the memory content.
    Memory,
    /// Unchanged lines left out between two hunks.
    Gap,
}

/// One line of the conflict diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub tag: DiffTag,
    pub text: String,
}

/// The `disk_hash` of a document whose file was deleted and whose user chose to keep the
/// editor version: the next save recreates the file (PLAT-SAVE-079).
const RECREATE_ON_SAVE: u64 = 0;

pub fn content_hash(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// What one synchronization did to a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyncOutcome {
    Unchanged,
    Reloaded,
    Closed,
    Conflict,
}

impl IdeState {
    /// Starts watching the open project; failures leave the IDE usable and
    /// `file.synchronize` still works.
    pub(crate) fn start_watcher(&mut self) {
        self.watcher = None;
        let Some(project) = &self.project else {
            return;
        };
        let waker = self.waker.clone();
        match ProjectWatcher::new(project.root(), move || waker.wake()) {
            Ok(watcher) => self.watcher = Some(watcher),
            Err(error) => {
                self.status =
                    format!("File watching unavailable ({error}); use File → Reload All from Disk")
            }
        }
    }

    /// Applies outside file changes; call once per frame.
    pub fn poll_files(&mut self) {
        let Some(changes) = self.watcher.as_ref().map(ProjectWatcher::poll) else {
            return;
        };
        if changes.is_empty() {
            return;
        }
        self.mark_vcs_stale();
        self.request_git_refresh();
        self.index_paths_changed(&changes.paths);
        self.project_model_files_changed(&changes.paths);
        if self.is_gradle_project()
            && changes
                .paths
                .iter()
                .any(|path| archipel_ide_gradle::is_build_input(path))
        {
            self.gradle_changes_pending = true;
        }
        if changes.structure_changed
            && let Some(project) = &self.project
        {
            self.set_files(project.entries(20_000));
        }
        let affected: Vec<_> = self
            .documents
            .iter()
            .filter(|document| changes.paths.contains(&document.path))
            .map(|document| document.path.clone())
            .collect();
        self.sync_paths(&affected);
    }

    /// Checks every open document against the disk and refreshes the tree
    /// (Reload All from Disk).
    pub fn synchronize_all(&mut self) {
        if let Some(project) = &self.project {
            self.set_files(project.entries(20_000));
        }
        self.mark_vcs_stale();
        self.request_git_refresh();
        let paths: Vec<_> = self.documents.iter().map(|d| d.path.clone()).collect();
        let changed = self.sync_paths(&paths);
        if changed == 0 && !self.documents.iter().any(|d| d.conflict.is_some()) {
            self.status = "All files are up to date".into();
        }
    }

    /// Synchronizes the documents at `paths`; returns how many changed.
    fn sync_paths(&mut self, paths: &[std::path::PathBuf]) -> usize {
        let mut changed = 0;
        for path in paths {
            let Some(index) = self.documents.iter().position(|d| &d.path == path) else {
                continue;
            };
            let name = path.display().to_string();
            match self.sync_document(index) {
                SyncOutcome::Unchanged => {}
                SyncOutcome::Reloaded => {
                    changed += 1;
                    self.status = format!("Reloaded {name} (changed on disk)");
                }
                SyncOutcome::Closed => {
                    changed += 1;
                    self.status = format!("Closed {name} (deleted on disk)");
                }
                SyncOutcome::Conflict => {
                    changed += 1;
                    self.status = format!(
                        "{name} changed on disk and has unsaved changes: load the disk version or keep yours"
                    );
                }
            }
        }
        changed
    }

    pub(crate) fn sync_document(&mut self, index: usize) -> SyncOutcome {
        let Some(project) = &self.project else {
            return SyncOutcome::Unchanged;
        };
        let document = &self.documents[index];
        let dirty = document.text.is_dirty();
        let exists = project.root().join(&document.path).exists();
        if !exists {
            if dirty && document.disk_hash == RECREATE_ON_SAVE {
                // "Keep and recreate on save" was chosen: the next save writes the file.
                return SyncOutcome::Unchanged;
            }
            if dirty {
                self.documents[index].conflict = Some(DiskConflict::Deleted);
                return SyncOutcome::Conflict;
            }
            self.remove_document(index);
            return SyncOutcome::Closed;
        }
        let Ok(disk) = project.read_text(&document.path) else {
            // Unreadable or no longer UTF-8: keep the buffer untouched.
            return SyncOutcome::Unchanged;
        };
        let hash = content_hash(&disk);
        if hash == document.disk_hash {
            if document.conflict == Some(DiskConflict::Deleted) {
                self.documents[index].conflict = None;
            }
            return SyncOutcome::Unchanged;
        }
        if dirty {
            self.documents[index].conflict = Some(DiskConflict::Modified);
            return SyncOutcome::Conflict;
        }
        self.load_disk_text(index, &disk);
        SyncOutcome::Reloaded
    }

    /// Replaces the buffer with `disk` as one undoable edit and marks it saved.
    fn load_disk_text(&mut self, index: usize, disk: &str) {
        if self.analysis.history.is_enabled() {
            // Local History: the outside version becomes a revision.
            let document = &self.documents[index];
            let (path, buffer, dirty) = (
                document.path.clone(),
                document.text.text(),
                document.text.is_dirty(),
            );
            self.history_external(&path, &buffer, dirty, disk);
        }
        let document = &mut self.documents[index];
        if document.text.replace_from_snapshot(disk).is_err() {
            document.text = archipel_ide_text::TextDocument::new(disk);
        }
        document.text.mark_saved();
        document.disk_hash = content_hash(disk);
        document.separator = archipel_ide_text::format::detect_line_separator_str(disk);
        document.conflict = None;
        let len = document.text.len_chars();
        document.selections.clamp(len);
        document.refresh_analysis(disk);
        let path = document.path.clone();
        let version = document.text.revision().min(i32::MAX as u64) as i32;
        self.sync_document_lsp(&path, disk, version);
    }

    /// The first document with an unresolved outside change (PLAT-VFS-101): the frontends ask
    /// the user about it.
    pub fn conflicted_document(&self) -> Option<usize> {
        self.documents.iter().position(|d| d.conflict.is_some())
    }

    /// Every document with an unresolved outside change: (index, path, kind).
    pub fn conflicts(&self) -> Vec<(usize, std::path::PathBuf, DiskConflict)> {
        self.documents
            .iter()
            .enumerate()
            .filter_map(|(index, d)| Some((index, d.path.clone(), d.conflict?)))
            .collect()
    }

    /// "Show Difference" of the conflict dialog: the disk text against the memory text as
    /// unified-diff lines with two lines of context, capped at `limit` lines.
    pub fn conflict_diff(&self, index: usize, limit: usize) -> Result<Vec<DiffLine>, String> {
        let project = self.project.as_ref().ok_or("no project is open")?;
        let document = self.documents.get(index).ok_or("document does not exist")?;
        let disk = project.read_text(&document.path).unwrap_or_default();
        let memory = document.text.text();
        let diff = similar::TextDiff::from_lines(&disk, &memory);
        let mut lines = Vec::new();
        for (group_index, group) in diff.grouped_ops(2).iter().enumerate() {
            if group_index > 0 {
                lines.push(DiffLine {
                    tag: DiffTag::Gap,
                    text: String::new(),
                });
            }
            for op in group {
                for change in diff.iter_changes(op) {
                    let tag = match change.tag() {
                        similar::ChangeTag::Equal => DiffTag::Same,
                        similar::ChangeTag::Delete => DiffTag::Disk,
                        similar::ChangeTag::Insert => DiffTag::Memory,
                    };
                    lines.push(DiffLine {
                        tag,
                        text: change.value().trim_end_matches(['\n', '\r']).to_owned(),
                    });
                }
            }
            if lines.len() >= limit {
                lines.truncate(limit);
                break;
            }
        }
        Ok(lines)
    }

    /// Resolves a conflict in favour of the disk (or closes a tab whose file
    /// was deleted). Without a conflict it simply reloads.
    pub fn reload_from_disk(&mut self, index: usize) -> Result<(), String> {
        let project = self.project.as_ref().ok_or("no project is open")?;
        let document = self.documents.get(index).ok_or("document does not exist")?;
        let name = document.path.display().to_string();
        if !project.root().join(&document.path).exists() {
            self.remove_document(index);
            self.status = format!("Closed {name} (deleted on disk)");
            return Ok(());
        }
        let disk = project
            .read_text(&document.path)
            .map_err(|error| error.to_string())?;
        self.load_disk_text(index, &disk);
        self.status = format!("Loaded {name} from disk");
        Ok(())
    }

    /// Resolves a conflict in favour of the editor: the buffer stays modified
    /// and the next save deliberately replaces (or recreates) the disk file.
    pub fn keep_memory_version(&mut self, index: usize) -> Result<(), String> {
        let project = self.project.as_ref().ok_or("no project is open")?;
        let document = self.documents.get(index).ok_or("document does not exist")?;
        let disk_hash = project
            .read_text(&document.path)
            .map(|text| content_hash(&text))
            .unwrap_or(RECREATE_ON_SAVE);
        let document = &mut self.documents[index];
        document.disk_hash = disk_hash;
        document.conflict = None;
        self.status = format!(
            "Kept your version of {}; saving replaces the disk file",
            document.path.display()
        );
        Ok(())
    }

    fn remove_document(&mut self, index: usize) {
        self.documents.remove(index);
        self.active_document = match self.active_document {
            Some(_) if self.documents.is_empty() => None,
            Some(active) if active > index => Some(active - 1),
            Some(active) if active == index => Some(index.min(self.documents.len() - 1)),
            other => other,
        };
    }
}

fn active_has_conflict(state: &IdeState) -> bool {
    state.conflicted_document().is_some()
}

fn document_index(state: &IdeState, params: &Value, id: &str) -> Result<usize, CommandError> {
    match params.get("index") {
        Some(_) => usize_param(params, "index"),
        None => state
            .active_document
            .ok_or_else(|| CommandError::Disabled(id.into())),
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "editor.reloadFromDisk",
                "Load Disk Version",
                "Editor",
                None,
                r#"{"index":"number?"}"#,
            ),
            active_has_conflict,
            |state: &mut IdeState, params| {
                let index = document_index(state, &params, "editor.reloadFromDisk")?;
                state
                    .reload_from_disk(index)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.keepMemoryVersion",
                "Keep Editor Version",
                "Editor",
                None,
                r#"{"index":"number?"}"#,
            ),
            active_has_conflict,
            |state: &mut IdeState, params| {
                let index = document_index(state, &params, "editor.keepMemoryVersion")?;
                state
                    .keep_memory_version(index)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "file.synchronize",
                "Reload All from Disk",
                "Project",
                Some("⌥⌘Y"),
                "{}",
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, _| {
                state.synchronize_all();
                let conflicts: Vec<_> = state
                    .documents
                    .iter()
                    .filter(|document| document.conflict.is_some())
                    .map(|document| document.path.clone())
                    .collect();
                Ok(json!({ "conflicts": conflicts }))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    fn project(name: &str) -> (PathBuf, IdeState) {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-disk-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/App.java"), "class App {}\n").unwrap();
        let mut state = IdeState::default();
        state.open_project(&root).unwrap();
        state.open_file(Path::new("src/App.java")).unwrap();
        (root.canonicalize().unwrap(), state)
    }

    fn text(state: &IdeState) -> String {
        state.documents[0].text.text()
    }

    #[test]
    fn unmodified_buffer_reloads_outside_change() {
        let (root, mut state) = project("reload");
        std::fs::write(root.join("src/App.java"), "class App { int x; }\n").unwrap();
        state.synchronize_all();
        assert_eq!(text(&state), "class App { int x; }\n");
        assert!(!state.documents[0].text.is_dirty());
        assert!(state.status.contains("Reloaded"), "{}", state.status);
        // The reload is undoable, like an edit.
        assert!(state.documents[0].text.can_undo());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn modified_buffer_conflicts_and_is_never_silently_overwritten() {
        let (root, mut state) = project("conflict");
        let registry = built_in_commands();
        registry
            .execute(
                "editor.insertText",
                &mut state,
                json!({"position": 0, "text": "// mine\n"}),
            )
            .unwrap();
        std::fs::write(root.join("src/App.java"), "class Theirs {}\n").unwrap();
        state.synchronize_all();
        assert_eq!(state.documents[0].conflict, Some(DiskConflict::Modified));
        assert!(text(&state).starts_with("// mine"), "buffer kept");
        // Autosave (save all) and explicit save both refuse to overwrite.
        assert!(
            registry
                .execute("editor.saveAll", &mut state, Value::Null)
                .is_err()
        );
        assert!(
            registry
                .execute("editor.save", &mut state, Value::Null)
                .is_err()
        );
        assert_eq!(
            std::fs::read_to_string(root.join("src/App.java")).unwrap(),
            "class Theirs {}\n"
        );
        // Keeping the editor version makes the next save deliberate.
        registry
            .execute("editor.keepMemoryVersion", &mut state, Value::Null)
            .unwrap();
        registry
            .execute("editor.saveAll", &mut state, Value::Null)
            .unwrap();
        assert!(
            std::fs::read_to_string(root.join("src/App.java"))
                .unwrap()
                .starts_with("// mine")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn plat_vfs_101_conflict_is_found_and_its_difference_listed() {
        let (root, mut state) = project("diff");
        assert_eq!(state.conflicted_document(), None);
        state.documents[0].text.edit(0, 0, "// mine\n").unwrap();
        std::fs::write(root.join("src/App.java"), "class Theirs {}\n").unwrap();
        state.synchronize_all();
        assert_eq!(state.conflicted_document(), Some(0));
        let diff = state.conflict_diff(0, 50).unwrap();
        assert!(diff.contains(&DiffLine {
            tag: DiffTag::Disk,
            text: "class Theirs {}".into()
        }));
        assert!(diff.contains(&DiffLine {
            tag: DiffTag::Memory,
            text: "// mine".into()
        }));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn loading_the_disk_version_resolves_a_conflict() {
        let (root, mut state) = project("load");
        let registry = built_in_commands();
        registry
            .execute(
                "editor.insertText",
                &mut state,
                json!({"position": 0, "text": "x"}),
            )
            .unwrap();
        std::fs::write(root.join("src/App.java"), "class Theirs {}\n").unwrap();
        state.synchronize_all();
        registry
            .execute("editor.reloadFromDisk", &mut state, Value::Null)
            .unwrap();
        assert_eq!(text(&state), "class Theirs {}\n");
        assert_eq!(state.documents[0].conflict, None);
        assert!(!state.documents[0].text.is_dirty());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn own_saves_are_not_outside_changes_and_deletions_close_clean_tabs() {
        let (root, mut state) = project("own");
        let registry = built_in_commands();
        registry
            .execute(
                "editor.insertText",
                &mut state,
                json!({"position": 0, "text": "x"}),
            )
            .unwrap();
        registry
            .execute("editor.save", &mut state, Value::Null)
            .unwrap();
        state.synchronize_all();
        assert_eq!(state.status, "All files are up to date");
        assert!(state.documents[0].text.can_undo(), "no reload happened");
        std::fs::remove_file(root.join("src/App.java")).unwrap();
        state.synchronize_all();
        assert!(state.documents.is_empty());
        assert_eq!(state.active_document, None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn deleted_file_with_unsaved_changes_conflicts() {
        let (root, mut state) = project("deleted");
        let registry = built_in_commands();
        registry
            .execute(
                "editor.insertText",
                &mut state,
                json!({"position": 0, "text": "x"}),
            )
            .unwrap();
        std::fs::remove_file(root.join("src/App.java")).unwrap();
        state.synchronize_all();
        assert_eq!(state.documents[0].conflict, Some(DiskConflict::Deleted));
        assert!(
            registry
                .execute("editor.save", &mut state, Value::Null)
                .is_err()
        );
        assert!(!root.join("src/App.java").exists());
        registry
            .execute("editor.keepMemoryVersion", &mut state, Value::Null)
            .unwrap();
        registry
            .execute("editor.save", &mut state, Value::Null)
            .unwrap();
        assert!(
            root.join("src/App.java").exists(),
            "keeping recreates the file"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn watcher_delivers_outside_changes_without_a_manual_refresh() {
        let (root, mut state) = project("watch");
        std::fs::write(root.join("src/App.java"), "class Watched {}\n").unwrap();
        std::fs::write(root.join("src/New.java"), "class New {}\n").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline
            && (text(&state) != "class Watched {}\n"
                || !state
                    .files
                    .iter()
                    .any(|f| f.relative_path == Path::new("src/New.java")))
        {
            state.poll_files();
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(text(&state), "class Watched {}\n");
        assert!(
            state
                .files
                .iter()
                .any(|f| f.relative_path == Path::new("src/New.java"))
        );
        assert!(state.git().unwrap().is_stale());
        let _ = std::fs::remove_dir_all(root);
    }
}

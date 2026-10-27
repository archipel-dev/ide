//! Saving documents and autosave (spec `docs/specs/platform/09-saving-and-autosave.md`).
//!
//! One path writes unsaved text to disk: [`IdeState::save_all_at`] for the set,
//! [`IdeState::save_one`] for a single document. Autosave triggers (frame deactivation, idle
//! timeout) and the save points before a run, build, external-system task or VCS operation all
//! go through it, so the rules hold everywhere: skip what is not tracked or already equal to
//! the disk, refresh before writing, ask the vetoers, run the cleanup (trailing spaces, final
//! newline), write safely, and report failures once per call.
//!
//! Design notes:
//!
//! - The unsaved set is the open documents whose text differs from the last save, plus the
//!   documents of closed tabs ([`DocumentSaving::park`]): closing a tab does not save
//!   (PLAT-SAVE-052), the text waits for the next trigger.
//! - A veto leaves the document unsaved without an error; an unresolved disk conflict is the
//!   veto that callers surface, because a VCS operation or a project switch must not go on
//!   while text exists only in memory.
//! - Nothing here makes analysis depend on a save: the daemon reads the document text
//!   (PLAT-SAVE-110, 111).

use crate::services::{DocumentSaveListener, DocumentSaving, SaveFailureReport};
use crate::{DiskConflict, Document, IdeState, content_hash};
use archipel_ide_platform::ComponentManager;
use archipel_ide_project::{Project, ProjectError, WriteMode};
use archipel_ide_settings::{AUTOSAVE_IDLE_DEFAULT, AUTOSAVE_IDLE_MAX, AUTOSAVE_IDLE_MIN, ids};
use archipel_ide_text::{Selection, TextChange};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// The exact user-visible texts of the feature (PLAT-SAVE-013, 077).
pub mod texts {
    pub const FAILURE_TITLE: &str = "Cannot Save Files";
    pub const FAILURE_MESSAGE: &str = "Following errors occurred on attempt to save files:";
    pub const REVERT_CHANGES: &str = "Revert Changes";
    pub const CLOSE: &str = "Close";
    pub const IGNORE_CHANGES: &str = "Ignore Changes";
    pub const AUTOSAVE_COMMENT: &str = "Autosave cannot be disabled completely.";
    pub const HOW_IT_WORKS: &str = "How it works";
    pub const HELP_TOPIC: &str = "autosave";
}

/// A file larger than this is never written from a document (PLAT-VFS-035, PLAT-SAVE-073 (b)).
pub const CONTENT_LOADING_LIMIT: usize = 20 * 1024 * 1024;
/// Documents up to this many characters are compared with the disk before writing
/// (PLAT-SAVE-075).
pub const CONTENT_COMPARE_LIMIT: usize = 1_000_000;

/// Where a save happens. Everything but [`SavePoint::Deactivation`], [`SavePoint::Idle`] and
/// [`SavePoint::BeforeLaunchBuild`] is explicit (PLAT-SAVE-073: the flag only changes the veto
/// outcome).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SavePoint {
    /// The Save All action and Save Document.
    SaveAll,
    /// The window lost focus to another application (PLAT-SAVE-020).
    Deactivation,
    /// No user input for the idle timeout (PLAT-SAVE-021).
    Idle,
    /// Run, tests and every other task started from a configuration (PLAT-SAVE-030, 031).
    Run,
    /// Debug, just before the debuggee starts (PLAT-SAVE-030).
    Debug,
    /// Build, make, rebuild, compile (PLAT-SAVE-033).
    Build,
    /// The "Build" step before a launch (PLAT-SAVE-032): non-explicit.
    BeforeLaunchBuild,
    /// Gradle, Maven and other external-system tasks and reloads (PLAT-SAVE-034).
    ExternalSystem,
    /// Tools menu entries (PLAT-SAVE-035).
    ExternalTool,
    /// Commit (PLAT-SAVE-040).
    Commit,
    /// Update, pull, checkout, merge, reset, worktree, rollback, stash, ... (PLAT-SAVE-041).
    Vcs,
    /// Project close and switching projects (PLAT-SAVE-050).
    ProjectClose,
    /// Application exit (PLAT-SAVE-051).
    Exit,
}

impl SavePoint {
    pub fn explicit(self) -> bool {
        !matches!(
            self,
            Self::Deactivation | Self::Idle | Self::BeforeLaunchBuild
        )
    }
}

/// What one save call did, per document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaveOutcome {
    /// Written to disk.
    pub written: Vec<PathBuf>,
    /// Left the unsaved set without a write (nothing to write, file gone).
    pub dropped: Vec<PathBuf>,
    /// Vetoed (completion popup, size, extension): still unsaved, no error.
    pub vetoed: Vec<PathBuf>,
    /// Vetoed because the file changed on disk while it has unsaved edits.
    pub conflicts: Vec<PathBuf>,
    /// An I/O failure: path and message.
    pub failed: Vec<(PathBuf, String)>,
}

impl SaveOutcome {
    /// No document failed (vetoes are not failures).
    pub fn is_success(&self) -> bool {
        self.failed.is_empty()
    }

    /// `Err` when a document failed or waits for a conflict resolution: callers that must not
    /// go on with text only in memory (VCS operations, project switch) propagate it.
    pub fn into_result(self) -> Result<(), String> {
        if !self.failed.is_empty() {
            return Err(self
                .failed
                .iter()
                .map(|(_, message)| message.as_str())
                .collect::<Vec<_>>()
                .join("; "));
        }
        if !self.conflicts.is_empty() {
            return Err(format!(
                "Not saved, changed on disk: {} (load the disk version or keep yours)",
                self.conflicts
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        Ok(())
    }
}

enum DocOutcome {
    Written,
    Dropped,
    Vetoed,
    Conflict,
    Failed(String),
}

impl DocOutcome {
    fn record(self, path: &Path, outcome: &mut SaveOutcome) {
        let path = path.to_path_buf();
        match self {
            Self::Written => outcome.written.push(path),
            Self::Dropped => outcome.dropped.push(path),
            Self::Vetoed => outcome.vetoed.push(path),
            Self::Conflict => outcome.conflicts.push(path),
            Self::Failed(message) => outcome.failed.push((path, message)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StripMode {
    None,
    Modified,
    All,
}

/// The editor settings the save path applies (PLAT-SAVE-090).
#[derive(Clone, Copy)]
struct Cleanup {
    strip: StripMode,
    keep_on_caret_line: bool,
    ensure_final_newline: bool,
}

struct WriteContext<'a> {
    as_is: bool,
    mode: WriteMode,
    cleanup: Cleanup,
    /// The document has a focused editor: its caret lines keep trailing spaces.
    editor_focused: bool,
    listeners: &'a [Arc<dyn DocumentSaveListener>],
}

enum Written {
    /// Written; `edited` when the cleanup or a listener changed the document.
    Text { text: String, edited: bool },
    /// Equal to the disk text: nothing written (PLAT-SAVE-075).
    Unchanged,
}

impl IdeState {
    /// The document saving service (application scope).
    pub fn saving(&self) -> Arc<DocumentSaving> {
        self.application().service()
    }

    fn setting_bool(&self, id: &str, default: bool) -> bool {
        self.settings().read().bool(id).unwrap_or(default)
    }

    /// The idle timeout in seconds, clamped into 1 to 300 (PLAT-SAVE-010).
    pub fn autosave_idle_seconds(&self) -> u64 {
        self.settings()
            .read()
            .int(ids::AUTOSAVE_IDLE_SECONDS)
            .unwrap_or(AUTOSAVE_IDLE_DEFAULT)
            .clamp(AUTOSAVE_IDLE_MIN, AUTOSAVE_IDLE_MAX) as u64
    }

    fn cleanup_settings(&self) -> Cleanup {
        let settings = self.settings();
        let settings = settings.read();
        let strip = match settings
            .text(ids::STRIP_TRAILING_SPACES)
            .as_deref()
            .unwrap_or("modified")
        {
            "none" => StripMode::None,
            "all" => StripMode::All,
            _ => StripMode::Modified,
        };
        Cleanup {
            strip,
            keep_on_caret_line: settings.bool(ids::KEEP_TRAILING_ON_CARET).unwrap_or(true),
            ensure_final_newline: settings.bool(ids::ENSURE_NEWLINE).unwrap_or(false),
        }
    }

    fn write_mode(&self) -> WriteMode {
        if self.setting_bool(ids::SAFE_WRITE, true) {
            WriteMode::Safe
        } else {
            WriteMode::InPlace
        }
    }

    // ---- PLAT-SAVE-001: the queries ------------------------------------------------------

    /// Paths of the documents with unsaved text: open tabs and closed tabs that were not
    /// written yet.
    pub fn unsaved_documents(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = self
            .documents
            .iter()
            .filter(|document| document.is_unsaved())
            .map(|document| document.path.clone())
            .collect();
        paths.extend(self.saving().parked_paths());
        paths
    }

    /// Whether the document of `path` has unsaved text (never for a virtual document,
    /// PLAT-SAVE-003).
    pub fn is_unsaved(&self, path: &Path) -> bool {
        self.documents
            .iter()
            .find(|document| document.path == path)
            .map_or_else(
                || self.saving().parked_paths().iter().any(|p| p == path),
                Document::is_unsaved,
            )
    }

    /// True only when the file has an unsaved document whose text differs from what was last
    /// loaded or saved (PLAT-SAVE-001).
    pub fn is_file_modified(&self, path: &Path) -> bool {
        self.documents
            .iter()
            .find(|document| document.path == path)
            .is_some_and(|document| {
                document.is_unsaved() && content_hash(&document.text.text()) != document.disk_hash
            })
    }

    // ---- PLAT-SAVE-001: the saves --------------------------------------------------------

    /// Explicit save of every unsaved document. `Err` when one failed or waits for a conflict
    /// resolution.
    pub fn save_all(&mut self) -> Result<(), String> {
        self.save_all_at(SavePoint::SaveAll).into_result()
    }

    /// Saves every unsaved document at `point` (the explicit flag follows the point).
    pub fn save_all_at(&mut self, point: SavePoint) -> SaveOutcome {
        self.save_documents(point.explicit(), point, None)
    }

    /// Explicit save of the documents `filter` accepts.
    pub fn save_filtered(&mut self, filter: &dyn Fn(&Path) -> bool) -> SaveOutcome {
        self.save_documents(true, SavePoint::SaveAll, Some(filter))
    }

    /// Saves the document of `path` (PLAT-SAVE-001 `save_one`).
    pub fn save_one(&mut self, path: &Path, explicit: bool) -> SaveOutcome {
        self.save_single(path, explicit, false)
    }

    /// Saves without stripping trailing spaces or adding a final newline, for this call only
    /// (PLAT-SAVE-001 `save_as_is`, PLAT-SAVE-090).
    pub fn save_as_is(&mut self, path: &Path) -> SaveOutcome {
        self.save_single(path, true, true)
    }

    /// Explicit save of the document at `index` (Save Document, closing a project).
    pub fn save(&mut self, index: usize) -> Result<(), String> {
        let path = self
            .documents
            .get(index)
            .ok_or("document does not exist")?
            .path
            .clone();
        let outcome = self.save_single(&path, true, false);
        if outcome.conflicts.is_empty() {
            outcome.into_result()
        } else {
            // The text kept its place; say why in the words of the banner.
            Err(disk_conflict_message(
                self.documents
                    .iter()
                    .find(|d| d.path == path)
                    .and_then(|d| d.conflict),
                &path,
            ))
        }
    }

    fn save_single(&mut self, path: &Path, explicit: bool, as_is: bool) -> SaveOutcome {
        let saving = self.saving();
        for listener in saving.listeners() {
            listener.before_any_document_saving(path, explicit);
        }
        let mut outcome = SaveOutcome::default();
        let result = self.save_open_document(path, explicit, as_is);
        result.record(path, &mut outcome);
        self.report_save_failures(&outcome, false);
        outcome
    }

    fn report_save_failures(&mut self, outcome: &SaveOutcome, closing: bool) {
        if outcome.failed.is_empty() {
            return;
        }
        let report = SaveFailureReport {
            paths: outcome
                .failed
                .iter()
                .map(|(path, _)| path.clone())
                .collect(),
            messages: outcome.failed.iter().map(|(_, m)| m.clone()).collect(),
            closing,
        };
        self.status = format!("Cannot save: {}", report.text().replace('\n', "; "));
        self.saving().report_failure(report);
    }

    /// PLAT-SAVE-070: the algorithm of `save_all` (steps 1 to 4 of the spec).
    fn save_documents(
        &mut self,
        explicit: bool,
        point: SavePoint,
        filter: Option<&dyn Fn(&Path) -> bool>,
    ) -> SaveOutcome {
        let saving = self.saving();
        let listeners = saving.listeners();
        for listener in &listeners {
            listener.before_all_documents_saving();
        }
        let mut outcome = SaveOutcome::default();
        // Documents already vetoed or failed in this call are not retried.
        let mut skipped: HashSet<PathBuf> = HashSet::new();
        // Documents a listener changes during the pass are picked up by another pass.
        for _ in 0..8 {
            let accepted =
                |path: &Path| !skipped.contains(path) && filter.is_none_or(|accept| accept(path));
            let open: Vec<PathBuf> = self
                .documents
                .iter()
                .filter(|document| document.is_unsaved() && accepted(&document.path))
                .map(|document| document.path.clone())
                .collect();
            let parked: Vec<PathBuf> = saving
                .parked_paths()
                .into_iter()
                .filter(|path| accepted(path))
                .collect();
            if open.is_empty() && parked.is_empty() {
                break;
            }
            for path in open {
                let result = self.save_open_document(&path, explicit, false);
                if !matches!(result, DocOutcome::Written | DocOutcome::Dropped) {
                    skipped.insert(path.clone());
                }
                result.record(&path, &mut outcome);
            }
            for path in parked {
                let Some(document) = saving.take_parked(&path) else {
                    continue;
                };
                let (kept, result) = self.save_parked_document(document, explicit);
                if let Some(document) = kept {
                    saving.restore_parked(vec![document]);
                }
                if !matches!(result, DocOutcome::Written | DocOutcome::Dropped) {
                    skipped.insert(path.clone());
                }
                result.record(&path, &mut outcome);
            }
        }
        self.report_save_failures(&outcome, point == SavePoint::ProjectClose);
        if explicit && outcome.is_success() && !outcome.written.is_empty() {
            self.status = match outcome.written.as_slice() {
                [only] => format!("Saved {}", only.display()),
                many => format!("Saved {} files", many.len()),
            };
        }
        outcome
    }

    /// PLAT-SAVE-071 to 076 for the document of an open tab.
    fn save_open_document(&mut self, path: &Path, explicit: bool, as_is: bool) -> DocOutcome {
        let saving = self.saving();
        let listeners = saving.listeners();
        let Some(index) = self.documents.iter().position(|d| d.path == path) else {
            return DocOutcome::Dropped;
        };
        // 071: not tracked, or nothing differs from the last save: drop without a write.
        if !self.documents[index].is_tracked() {
            return DocOutcome::Dropped;
        }
        if !self.documents[index].text.is_dirty() {
            // Already clean: it was not in the unsaved set.
            return DocOutcome::Dropped;
        }
        // 072: when the disk changed behind the editor's back, refresh before anything else: a
        // conflict makes the document wait for the user's choice, never a silent overwrite.
        if self.documents[index].conflict.is_none() {
            self.sync_document(index);
        }
        if self.documents[index].conflict.is_some() {
            // 073a, 079
            return DocOutcome::Conflict;
        }
        // 073: the vetoers.
        if self.save_vetoed(index, explicit) {
            return DocOutcome::Vetoed;
        }
        let editor_focused = saving.focused_document().as_deref() == Some(path);
        let context = WriteContext {
            as_is,
            mode: self.write_mode(),
            cleanup: self.cleanup_settings(),
            editor_focused,
            listeners: &listeners,
        };
        let previous = self.history_previous(path);
        let Some(project) = self.project.as_ref() else {
            return DocOutcome::Failed("no project is open".into());
        };
        let revision = self.documents[index].text.revision();
        let written = write_document(project, &mut self.documents[index], &context);
        match written {
            Ok(Written::Unchanged) => {
                for listener in &listeners {
                    listener.unsaved_document_dropped(path);
                }
                DocOutcome::Dropped
            }
            Ok(Written::Text { text, edited }) => {
                let document = &self.documents[index];
                let version = document.text.revision().min(i32::MAX as u64) as i32;
                let absolute = project.root().join(&document.path);
                let _ = self.lsp.save_document(&absolute);
                if edited || document.text.revision() != revision {
                    // The cleanup is an ordinary document change: the language service sees it.
                    self.sync_document_lsp(path, &text, version);
                }
                self.history_saved(path, previous.as_deref(), &text);
                self.status = format!("Saved {}", path.display());
                for listener in &listeners {
                    listener.after_document_saved(path);
                }
                DocOutcome::Written
            }
            Err(message) => DocOutcome::Failed(message),
        }
    }

    /// The built-in and extension vetoes (PLAT-SAVE-073 (b) to e; the conflict is handled by the
    /// caller).
    fn save_vetoed(&self, index: usize, explicit: bool) -> bool {
        let document = &self.documents[index];
        let too_large =
            document.text.char_to_byte(document.text.len_chars()) > CONTENT_LOADING_LIMIT;
        // c: a completion popup for this document, non-explicit saves only.
        let lookup_open = !explicit
            && self
                .completion
                .as_ref()
                .is_some_and(|session| session.path == document.path);
        too_large || lookup_open || self.saving().vetoed_by_extension(&document.path, explicit)
    }

    /// A closed tab's text that was not written yet. A file deleted since is dropped
    /// (PLAT-SAVE-074); a file changed on disk since brings the tab back with its conflict
    /// banner, because nobody would see a dialog for a closed tab.
    fn save_parked_document(
        &mut self,
        mut document: Document,
        explicit: bool,
    ) -> (Option<Document>, DocOutcome) {
        let path = document.path.clone();
        let saving = self.saving();
        let listeners = saving.listeners();
        if !document.is_tracked() || !document.text.is_dirty() {
            return (None, DocOutcome::Dropped);
        }
        let Some(project) = self.project.as_ref() else {
            return (
                Some(document),
                DocOutcome::Failed("no project is open".into()),
            );
        };
        if !project.root().join(&path).exists() {
            for listener in &listeners {
                listener.unsaved_document_dropped(&path);
            }
            return (None, DocOutcome::Dropped);
        }
        let changed = match project.read_text(&path) {
            Ok(disk) => content_hash(&disk) != document.disk_hash,
            Err(_) => false,
        };
        if changed {
            document.conflict = Some(DiskConflict::Modified);
            self.status = format!(
                "{} changed on disk and has unsaved changes: load the disk version or keep yours",
                path.display()
            );
            self.documents.push(document);
            return (None, DocOutcome::Conflict);
        }
        let too_large =
            document.text.char_to_byte(document.text.len_chars()) > CONTENT_LOADING_LIMIT;
        if too_large || saving.vetoed_by_extension(&path, explicit) {
            return (Some(document), DocOutcome::Vetoed);
        }
        let context = WriteContext {
            as_is: false,
            mode: self.write_mode(),
            cleanup: self.cleanup_settings(),
            editor_focused: false,
            listeners: &listeners,
        };
        let previous = self.history_previous(&path);
        let Some(project) = self.project.as_ref() else {
            return (
                Some(document),
                DocOutcome::Failed("no project is open".into()),
            );
        };
        match write_document(project, &mut document, &context) {
            Ok(Written::Unchanged) => {
                for listener in &listeners {
                    listener.unsaved_document_dropped(&path);
                }
                (None, DocOutcome::Dropped)
            }
            Ok(Written::Text { text, .. }) => {
                self.history_saved(&path, previous.as_deref(), &text);
                for listener in &listeners {
                    listener.after_document_saved(&path);
                }
                (None, DocOutcome::Written)
            }
            Err(message) => (Some(document), DocOutcome::Failed(message)),
        }
    }

    // ---- PLAT-SAVE-052: closing a tab does not save --------------------------------------

    /// The text of a tab that closes stays in the unsaved set until a trigger writes it.
    pub(crate) fn park_closed_document(&mut self, document: Document) {
        if document.is_unsaved() {
            self.saving().park(document);
        }
    }

    /// Reopening a tab takes back the text that was waiting for a save; the disk may have
    /// changed meanwhile, which shows as the usual conflict.
    pub(crate) fn take_parked_document(&mut self, path: &Path) -> Option<Document> {
        self.saving().take_parked(path)
    }

    // ---- PLAT-SAVE-077: failures ---------------------------------------------------------

    /// "Revert Changes" / "Ignore Changes": reloads each failed document from disk,
    /// discarding its memory text.
    pub fn revert_documents(&mut self, paths: &[PathBuf]) {
        let saving = self.saving();
        for path in paths {
            if saving.take_parked(path).is_some() {
                continue;
            }
            if let Some(index) = self.documents.iter().position(|d| &d.path == path) {
                let _ = self.reload_from_disk(index);
            }
        }
    }

    // ---- PLAT-SAVE-020, 021: autosave triggers -------------------------------------------

    /// The application lost focus. Saves all unsaved documents when the option is on and no
    /// modal dialog is open (PLAT-SAVE-020). Non-explicit.
    pub fn autosave_on_deactivation(&mut self, modal: bool) -> Option<SaveOutcome> {
        if modal || !self.setting_bool(ids::AUTOSAVE_FOCUS, true) {
            return None;
        }
        Some(self.save_all_at(SavePoint::Deactivation))
    }

    /// The idle timer (PLAT-SAVE-021). Call once per frame with the current time; the window
    /// reports input with [`DocumentSaving::note_input`]. Settings are read each call, so a
    /// change applies at once (PLAT-SAVE-015).
    pub fn autosave_idle_tick(&mut self, now: Instant, modal: bool) -> Option<SaveOutcome> {
        let enabled = self.setting_bool(ids::AUTOSAVE_IDLE, false);
        let seconds = self.autosave_idle_seconds();
        let saving = self.saving();
        if !saving.idle_due(now, enabled, seconds) || modal {
            return None;
        }
        saving.mark_idle_saved();
        Some(self.save_all_at(SavePoint::Idle))
    }

    /// How long until the idle save is due, for scheduling the next repaint.
    pub fn autosave_idle_remaining(&self, now: Instant) -> Option<std::time::Duration> {
        let enabled = self.setting_bool(ids::AUTOSAVE_IDLE, false);
        if !enabled || self.unsaved_documents().is_empty() {
            return None;
        }
        self.saving()
            .idle_remaining(now, enabled, self.autosave_idle_seconds())
    }

    /// The save point before a task starts (PLAT-SAVE-030 to 035): saves all unsaved
    /// documents. A failure is reported to the window; the task still starts, like a run
    /// that cannot save one read-only file.
    ///
    /// No setting disables these save points (PLAT-SAVE-004), and they never rely on autosave
    /// (PLAT-SAVE-113). Editing features (typing, undo, completion, focus changes inside the IDE)
    /// do not save; the only triggers are the ones in this module (PLAT-SAVE-053).
    pub fn save_before(&mut self, point: SavePoint) {
        let _ = self.save_all_at(point);
    }
}

fn disk_conflict_message(conflict: Option<DiskConflict>, path: &Path) -> String {
    match conflict {
        Some(DiskConflict::Deleted) => format!(
            "{} was deleted on disk; close it or keep your version before saving",
            path.display()
        ),
        _ => format!(
            "{} changed on disk; load the disk version or keep yours before saving",
            path.display()
        ),
    }
}

/// PLAT-SAVE-075, 076, 078, 090: compare with the disk, clean up, write.
fn write_document(
    project: &Project,
    document: &mut Document,
    context: &WriteContext<'_>,
) -> Result<Written, String> {
    let name = document.path.display().to_string();
    let disk = match project.read_text(&document.path) {
        Ok(text) => Some(text),
        // A deleted file is recreated by the save the user chose ("keep my version").
        Err(ProjectError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => None,
        // Not text: always written.
        Err(ProjectError::NonUtf8(_)) => None,
        Err(error) => return Err(format!("{name}: {error}")),
    };
    if !document.force_save
        && document.text.len_chars() <= CONTENT_COMPARE_LIMIT
        && disk.as_deref() == Some(document.text.text().as_str())
    {
        document.text.mark_saved();
        document.disk_hash = content_hash(&document.text.text());
        return Ok(Written::Unchanged);
    }
    let revision = document.text.revision();
    let read_only = std::fs::metadata(project.root().join(&document.path))
        .is_ok_and(|metadata| metadata.permissions().readonly());
    if !context.as_is && !read_only {
        clean_up(document, disk.as_deref(), context);
    }
    for listener in context.listeners {
        listener.before_document_saving(document);
    }
    let edited = document.text.revision() != revision;
    let text = document.text.text();
    project
        .write_document_text(&document.path, &text, context.mode)
        .map_err(|error| format!("{name}: {error}"))?;
    if edited {
        document.refresh_analysis(&text);
        document.selections.clamp(document.text.len_chars());
    }
    document.text.mark_saved();
    document.disk_hash = content_hash(&text);
    document.force_save = false;
    Ok(Written::Text { text, edited })
}

/// PLAT-SAVE-090, 091: strips trailing spaces (on modified lines or all), ensures the final
/// newline, with the caret-line rule, as one document change.
fn clean_up(document: &mut Document, disk: Option<&str>, context: &WriteContext<'_>) {
    let cleanup = context.cleanup;
    if cleanup.strip == StripMode::None && !cleanup.ensure_final_newline {
        document.kept_trailing.clear();
        return;
    }
    let text = document.text.text();
    // Lines holding a caret of the focused editor (PLAT-SAVE-092: none without focus).
    let caret_lines: HashSet<usize> = if context.editor_focused {
        document
            .selections
            .selections()
            .iter()
            .map(|selection| document.text.char_to_line(selection.head))
            .collect()
    } else {
        HashSet::new()
    };
    let modified = modified_lines(disk, &text);
    let last_line = document.text.len_lines().saturating_sub(1);
    let mut changes: Vec<TextChange> = Vec::new();
    let mut kept: Vec<String> = Vec::new();
    for line in 0..document.text.len_lines() {
        let Some(raw) = document.text.line(line) else {
            continue;
        };
        let content = raw.trim_end_matches(['\n', '\r']);
        let trimmed = content.trim_end_matches([' ', '\t']);
        let trailing = content.len() - trimmed.len();
        if trailing == 0 {
            continue;
        }
        let whitespace_only_last =
            line == last_line && trimmed.is_empty() && cleanup.ensure_final_newline;
        let strip = whitespace_only_last
            || match cleanup.strip {
                StripMode::None => false,
                StripMode::All => true,
                StripMode::Modified => {
                    modified.contains(&line) || document.kept_trailing.iter().any(|k| k == content)
                }
            };
        if !strip {
            continue;
        }
        if caret_lines.contains(&line) && cleanup.keep_on_caret_line {
            // The spaces stay; the line stays flagged so the next save strips it.
            kept.push(content.to_owned());
            continue;
        }
        let start = document.text.line_to_char(line).unwrap_or(0);
        let content_chars = content.chars().count();
        changes.push(TextChange {
            start_char: start + content_chars - trailing,
            end_char: start + content_chars,
            inserted: String::new(),
        });
    }
    if cleanup.ensure_final_newline {
        let len = document.text.len_chars();
        let last = document.text.line(last_line).unwrap_or_default();
        let ends_with_newline = last.is_empty();
        let whitespace_only = !last.is_empty() && last.trim_matches([' ', '\t']).is_empty();
        let kept_on_caret =
            whitespace_only && caret_lines.contains(&last_line) && cleanup.keep_on_caret_line;
        if !ends_with_newline && (!whitespace_only || kept_on_caret) {
            let separator = archipel_ide_text::format::detect_line_separator(&document.text);
            changes.push(TextChange {
                start_char: len,
                end_char: len,
                inserted: separator.as_str().to_owned(),
            });
        }
    }
    document.kept_trailing = kept;
    if changes.is_empty() {
        return;
    }
    let mut carets: Vec<Selection> = document.selections.selections().to_vec();
    for selection in &mut carets {
        selection.anchor = shifted(selection.anchor, &changes);
        selection.head = shifted(selection.head, &changes);
    }
    if document.text.edit_many(changes).is_ok() {
        document.selections.set_all(carets);
        document.selections.clamp(document.text.len_chars());
    }
}

/// Where a character offset ends up after `changes` (offsets refer to the old text).
fn shifted(offset: usize, changes: &[TextChange]) -> usize {
    let mut moved = offset as isize;
    for change in changes {
        let inserted = change.inserted.chars().count() as isize;
        let removed = (change.end_char - change.start_char) as isize;
        if change.start_char == change.end_char {
            // An insertion at the caret leaves the caret before it.
            if offset > change.start_char {
                moved += inserted;
            }
        } else if offset >= change.end_char {
            moved += inserted - removed;
        } else if offset > change.start_char {
            moved -= (offset - change.start_char) as isize;
        }
    }
    moved.max(0) as usize
}

/// The lines of `text` that differ from the last saved `disk` text (all of them without one).
fn modified_lines(disk: Option<&str>, text: &str) -> HashSet<usize> {
    let Some(disk) = disk else {
        return (0..text.lines().count().max(1)).collect();
    };
    let diff = similar::TextDiff::from_lines(disk, text);
    diff.iter_all_changes()
        .filter(|change| change.tag() == similar::ChangeTag::Insert)
        .filter_map(|change| change.new_index())
        .collect()
}

#[cfg(test)]
mod tests;

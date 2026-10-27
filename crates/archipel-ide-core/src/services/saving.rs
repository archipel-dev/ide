//! Document saving as an application service (IntelliJ: `FileDocumentManager` plus the
//! `SaveAndSyncHandler`; spec `docs/specs/platform/09-saving-and-autosave.md`).
//!
//! The service owns what is not a document: the save listeners and vetoers extensions add, the
//! documents of closed tabs that still hold unsaved text (PLAT-SAVE-052), the idle timer
//! (PLAT-SAVE-021), the failure report the window shows (PLAT-SAVE-077) and which document the
//! focused editor shows (PLAT-SAVE-091). The documents themselves still live on `IdeState`, so
//! the writing is [`IdeState::save_all_at`] and friends in `saving.rs`.

use crate::Document;
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Observes saves (PLAT-SAVE-002). All methods default to nothing. `before_document_saving` may
/// edit the document; the edit is part of what is written.
pub trait DocumentSaveListener: Send + Sync {
    /// Once at the start of a `save_all`, before anything is written.
    fn before_all_documents_saving(&self) {}
    /// First thing of a single-document save, even when the document turns out unchanged.
    fn before_any_document_saving(&self, _path: &Path, _explicit: bool) {}
    /// Only when a write will happen.
    fn before_document_saving(&self, _document: &mut Document) {}
    fn after_document_saved(&self, _path: &Path) {}
    /// The document left the unsaved set without a write.
    fn unsaved_document_dropped(&self, _path: &Path) {}
}

/// A policy that makes one document skip the current save without an error (PLAT-SAVE-073).
/// An extension that panics is isolated and counts as no veto.
pub trait SavingVetoer: Send + Sync {
    fn may_save(&self, path: &Path, explicit: bool) -> bool;
}

/// What the window shows after a save call failed (PLAT-SAVE-077).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFailureReport {
    /// Project-relative paths of the documents that failed.
    pub paths: Vec<PathBuf>,
    /// One message per failed document.
    pub messages: Vec<String>,
    /// Raised by a project close: the confirm button reads "Ignore Changes".
    pub closing: bool,
}

impl SaveFailureReport {
    /// The read-only text of the dialog.
    pub fn text(&self) -> String {
        self.messages.join("\n")
    }
}

#[derive(Default)]
struct IdleTimer {
    /// The last user input event.
    last_input: Option<Instant>,
    /// A save already ran for the current quiet period.
    saved: bool,
    /// The (enabled, seconds) the timer was armed with; a change re-arms it (PLAT-SAVE-015).
    armed: Option<(bool, u64)>,
}

#[derive(Default)]
struct SavingState {
    listeners: Vec<Arc<dyn DocumentSaveListener>>,
    vetoers: Vec<Arc<dyn SavingVetoer>>,
    /// Closed tabs whose text was not written yet.
    parked: Vec<Document>,
    idle: IdleTimer,
    focused: Option<PathBuf>,
    failure: Option<SaveFailureReport>,
}

/// The application service: `application.service::<DocumentSaving>()`.
#[derive(Default)]
pub struct DocumentSaving {
    state: Mutex<SavingState>,
}

impl DocumentSaving {
    pub fn add_listener(&self, listener: Arc<dyn DocumentSaveListener>) {
        self.state.lock().listeners.push(listener);
    }

    pub fn add_vetoer(&self, vetoer: Arc<dyn SavingVetoer>) {
        self.state.lock().vetoers.push(vetoer);
    }

    pub(crate) fn listeners(&self) -> Vec<Arc<dyn DocumentSaveListener>> {
        self.state.lock().listeners.clone()
    }

    /// Whether an extension vetoes saving `path`.
    pub(crate) fn vetoed_by_extension(&self, path: &Path, explicit: bool) -> bool {
        let vetoers = self.state.lock().vetoers.clone();
        vetoers.iter().any(|vetoer| {
            // A vetoer that throws is isolated and treated as no veto.
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                !vetoer.may_save(path, explicit)
            }))
            .unwrap_or(false)
        })
    }

    /// Keeps the unsaved document of a closed tab (PLAT-SAVE-052).
    pub(crate) fn park(&self, document: Document) {
        let mut state = self.state.lock();
        state.parked.retain(|parked| parked.path != document.path);
        state.parked.push(document);
    }

    /// The parked document of `path`, taken out (reopening its tab).
    pub(crate) fn take_parked(&self, path: &Path) -> Option<Document> {
        let mut state = self.state.lock();
        let index = state.parked.iter().position(|d| d.path == path)?;
        Some(state.parked.remove(index))
    }

    pub(crate) fn restore_parked(&self, documents: Vec<Document>) {
        self.state.lock().parked.extend(documents);
    }

    /// Project-relative paths of parked documents with unsaved text.
    pub fn parked_paths(&self) -> Vec<PathBuf> {
        self.state
            .lock()
            .parked
            .iter()
            .map(|document| document.path.clone())
            .collect()
    }

    /// Drops parked documents (their project is gone).
    pub(crate) fn clear_parked(&self) {
        self.state.lock().parked.clear();
    }

    /// The window tells which document its focused editor shows, `None` when no editor has focus
    /// (also while the window is deactivated). Only that document has caret lines to keep
    /// (PLAT-SAVE-091, 092).
    pub fn set_focused_document(&self, path: Option<&Path>) {
        let mut state = self.state.lock();
        if state.focused.as_deref() != path {
            state.focused = path.map(Path::to_path_buf);
        }
    }

    pub fn focused_document(&self) -> Option<PathBuf> {
        self.state.lock().focused.clone()
    }

    /// Records a failed save for the window to show; an identical pending report is not queued
    /// twice.
    pub(crate) fn report_failure(&self, report: SaveFailureReport) {
        let mut state = self.state.lock();
        match &state.failure {
            Some(pending) if pending.paths == report.paths && pending.closing == report.closing => {
            }
            _ => state.failure = Some(report),
        }
    }

    /// The pending failure report, taken by the window once per failed call.
    pub fn take_failure(&self) -> Option<SaveFailureReport> {
        self.state.lock().failure.take()
    }

    pub fn has_failure(&self) -> bool {
        self.state.lock().failure.is_some()
    }

    /// A user input event happened (key, mouse). Background work does not count (PLAT-SAVE-021).
    pub fn note_input(&self, now: Instant) {
        let mut state = self.state.lock();
        state.idle.last_input = Some(now);
        state.idle.saved = false;
    }

    /// Whether the idle save is due at `now`. Enabling the option or changing the timeout re-arms
    /// the timer; disabling cancels it (PLAT-SAVE-015). A save runs once per quiet period.
    pub fn idle_due(&self, now: Instant, enabled: bool, seconds: u64) -> bool {
        let mut state = self.state.lock();
        let idle = &mut state.idle;
        let config = Some((enabled, seconds));
        if idle.armed != config {
            idle.armed = config;
            idle.saved = false;
        }
        if !enabled || idle.saved {
            return false;
        }
        let last = *idle.last_input.get_or_insert(now);
        now.saturating_duration_since(last) >= Duration::from_secs(seconds)
    }

    pub(crate) fn mark_idle_saved(&self) {
        self.state.lock().idle.saved = true;
    }

    /// How long until the idle save is due, for the next repaint; `None` when nothing waits.
    pub fn idle_remaining(&self, now: Instant, enabled: bool, seconds: u64) -> Option<Duration> {
        let state = self.state.lock();
        if !enabled || state.idle.saved {
            return None;
        }
        let last = state.idle.last_input?;
        Some((last + Duration::from_secs(seconds)).saturating_duration_since(now))
    }
}

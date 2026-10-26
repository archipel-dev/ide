//! The analysis world of the app: one [`AnalysisServices`] in `IdeState`,
//! kept in sync with the open documents by [`IdeState::poll_analysis`]
//! (called once per frame). Editor buffers already reparse incrementally, so
//! their syntax snapshots are handed to the host as is (no second parse).
//! Plain-text buffers (no grammar) are synced after a typing pause, which
//! keeps typing in large plain files free of full-text copies.

use crate::{IdeState, spec};
use archipel_ide_analysis::{
    AnalysisEvent, AnalysisHost, Change, FileId, FileRev, HostConfig, SyntaxUpdate,
};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_language::ByteEdit;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Quiet time before a plain-text buffer's text is copied to the host.
const PLAIN_TEXT_SYNC_DELAY: Duration = Duration::from_millis(300);

/// Edits of a document not yet seen by the analysis host (recorded by
/// `Document::refresh_analysis`).
#[derive(Debug, Clone, Default)]
pub struct PendingAnalysisSync {
    /// Buffer revision the edits lead to.
    pub(crate) rev: u64,
    pub(crate) edits: Vec<ByteEdit>,
    /// The journal was reset (reload): send the whole text.
    pub(crate) reset: bool,
    pub(crate) dirty: bool,
}

impl PendingAnalysisSync {
    pub(crate) fn record(&mut self, rev: u64, edits: Option<&[ByteEdit]>) {
        self.rev = rev;
        self.dirty = true;
        match edits {
            Some(edits) if !self.reset => self.edits.extend_from_slice(edits),
            _ => {
                self.reset = true;
                self.edits.clear();
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Synced {
    rev: u64,
    /// Plain-text buffers: when the pending change was first seen.
    waiting_since: Option<Instant>,
}

type Subscriber = Box<dyn FnMut(&AnalysisEvent) + Send>;

/// The analysis host plus its synchronization with `IdeState::documents`.
pub struct AnalysisServices {
    pub host: AnalysisHost,
    /// Indexes, dumb mode and the TODO index (WP2); driven by `poll_analysis`.
    pub index: archipel_ide_index::IndexService,
    /// Directory the persistent indexes live in (`None`: memory only); set by
    /// the app at startup through `IdeState::enable_index_persistence`.
    pub(crate) index_cache_dir: Option<std::path::PathBuf>,
    /// TODO tool window state (`todo_commands.rs`).
    pub todo: crate::todo_commands::TodoState,
    /// Local History state (`history_commands.rs`).
    pub history: crate::history_commands::HistoryState,
    synced: HashMap<FileId, Synced>,
    subscribers: Vec<Subscriber>,
    /// Events dispatched by the last poll (diagnostics).
    pub last_events: usize,
}

impl Default for AnalysisServices {
    fn default() -> Self {
        Self::new(HostConfig::default())
    }
}

impl AnalysisServices {
    pub fn new(config: HostConfig) -> Self {
        Self {
            host: AnalysisHost::new(
                config,
                Arc::new(archipel_ide_analysis::vfs::DiskVfs),
                crate::analysis_modules::service_registry(),
            ),
            synced: HashMap::new(),
            index: archipel_ide_index::IndexService::new(archipel_ide_index::IndexConfig::default()),
            index_cache_dir: None,
            todo: Default::default(),
            history: Default::default(),
            subscribers: Vec::new(),
            last_events: 0,
        }
    }

    /// Calls `f` for every analysis event (daemon, indexes, code vision).
    pub fn subscribe(&mut self, f: impl FnMut(&AnalysisEvent) + Send + 'static) {
        self.subscribers.push(Box::new(f));
    }

    /// The host's id of an open document's file.
    pub fn file_of(&self, path: &std::path::Path) -> Option<FileId> {
        self.host.lookup(path)
    }
}

impl IdeState {
    /// Feeds document edits, opens and closes to the analysis host, runs its
    /// housekeeping and dispatches its events. Returns true when anything
    /// changed. Cheap when nothing was typed.
    pub fn poll_analysis(&mut self) -> bool {
        let now = Instant::now();
        self.sync_index_project();
        let services = &mut self.analysis;
        let mut open: HashSet<FileId> = HashSet::with_capacity(self.documents.len());
        let mut changed = false;
        for document in &mut self.documents {
            let file = services.host.file_id(&document.path);
            open.insert(file);
            let known = services.synced.get(&file).cloned();
            let pending = &mut document.analysis_sync;
            if known.is_some() && !pending.dirty {
                continue;
            }
            let rev = if pending.dirty {
                pending.rev
            } else {
                document.text.revision()
            };
            let has_syntax = document.analysis.syntax.is_some();
            if !has_syntax && known.is_some() {
                // Plain text: copy the buffer once typing pauses.
                let since = known.as_ref().and_then(|s| s.waiting_since).unwrap_or(now);
                if now.duration_since(since) < PLAIN_TEXT_SYNC_DELAY {
                    services.synced.insert(
                        file,
                        Synced {
                            rev: known.map_or(0, |s| s.rev),
                            waiting_since: Some(since),
                        },
                    );
                    continue;
                }
            }
            let text: Arc<str> = if has_syntax {
                document.analysis.source.clone()
            } else {
                document.text.text().into()
            };
            let snapshot = document.analysis.snapshot(file, FileRev(rev));
            let changes = document.analysis.take_syntax_changes();
            let change = match (&known, pending.reset || pending.edits.is_empty()) {
                (Some(_), false) => Change::Edit {
                    file,
                    edits: std::mem::take(&mut pending.edits),
                    new_rev: FileRev(rev),
                    text,
                    syntax: snapshot.map(|snapshot| SyntaxUpdate {
                        snapshot,
                        changed: changes,
                    }),
                },
                (Some(_), true) => Change::Reset {
                    file,
                    new_rev: FileRev(rev),
                    text,
                    syntax: snapshot,
                },
                (None, _) => Change::Reset {
                    file,
                    new_rev: FileRev(rev),
                    text,
                    syntax: snapshot,
                },
            };
            *pending = PendingAnalysisSync::default();
            services.host.apply(change);
            services.synced.insert(
                file,
                Synced {
                    rev,
                    waiting_since: None,
                },
            );
            changed = true;
        }
        let closed: Vec<FileId> = services
            .synced
            .keys()
            .filter(|file| !open.contains(file))
            .copied()
            .collect();
        for file in closed {
            services.synced.remove(&file);
            services.host.apply(Change::Close { file });
            changed = true;
        }
        changed |= services.host.poll();
        let events = services.host.take_events();
        services.index.on_events(&mut services.host, &events);
        changed |= services.index.poll(&mut services.host);
        let events: Vec<_> = events
            .into_iter()
            .chain(services.host.take_events())
            .collect();
        services.last_events = events.len();
        for event in &events {
            for subscriber in &mut services.subscribers {
                subscriber(event);
            }
        }
        // WP3 seam: the daemon runs after the host's housekeeping.
        changed |= self.poll_daemon(&events);
        changed
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec("analysis.status", "Analysis Status", "Analysis", None, "{}"),
            |_| true,
            |state: &mut IdeState, _| {
                state.poll_analysis();
                let snapshot = state.analysis.host.snapshot();
                let stats = state.analysis.host.cache().stats();
                let open: Vec<_> = archipel_ide_analysis::host::describe_open(&snapshot)
                    .into_iter()
                    .map(|(path, rev): (PathBuf, FileRev)| json!({"path": path, "rev": rev.0}))
                    .collect();
                Ok(json!({
                    "generation": snapshot.generation(),
                    "files": snapshot.files().len(),
                    "open": open,
                    "dumb": snapshot.is_dumb(),
                    "liveJobs": state.analysis.host.live_jobs(),
                    "cache": {"entries": stats.entries, "bytes": stats.bytes, "hits": stats.hits, "misses": stats.misses},
                    "modules": crate::analysis_modules::MODULES.iter().map(|m| m.id).collect::<Vec<_>>(),
                }))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::IdeState;
    use archipel_ide_analysis::{AnalysisEvent, FileRev};
    use std::sync::{Arc, Mutex};

    #[test]
    fn documents_are_mirrored_into_the_analysis_host() {
        let mut state = IdeState {
            analysis: super::AnalysisServices::new(archipel_ide_analysis::HostConfig::testing()),
            ..IdeState::default()
        };
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        state
            .analysis
            .subscribe(move |event| sink.lock().expect("lock").push(event.clone()));
        let path = std::path::PathBuf::from("/virtual/Main.java");
        state.documents.push(crate::Document::new(
            path.clone(),
            "class Main { void run() {} }\n",
        ));
        assert!(state.poll_analysis());
        let file = state.analysis.file_of(&path).expect("file id");
        let snap = state.analysis.host.snapshot();
        assert!(snap.is_open(file));
        assert_eq!(snap.syntax(file).map(|s| s.root().kind()), Some("program"));
        // Type: the host receives the edit and the editor's own snapshot.
        let document = &mut state.documents[0];
        document.text.edit(0, 0, "final ").expect("edit");
        let source = document.text.text();
        document.refresh_analysis(&source);
        let rev = document.text.revision();
        state.poll_analysis();
        let snap = state.analysis.host.snapshot();
        assert_eq!(snap.file_rev(file), Some(FileRev(rev)));
        assert!(
            snap.file_text(file)
                .expect("text")
                .starts_with("final class")
        );
        assert!(
            snap.edits_since(file, FileRev(0)).is_some()
                || snap.edits_since(file, FileRev(1)).is_some()
        );
        assert!(seen.lock().expect("lock").iter().any(
            |event| matches!(event, AnalysisEvent::SyntaxChanged { file: f, .. } if *f == file)
        ));
        // Closing the document closes the file.
        state.documents.clear();
        state.poll_analysis();
        assert!(!state.analysis.host.snapshot().is_open(file));
        let status = crate::built_in_commands()
            .execute("analysis.status", &mut state, serde_json::json!({}))
            .expect("status");
        assert!(status["modules"].as_array().is_some_and(|m| m.len() > 5));
    }
}

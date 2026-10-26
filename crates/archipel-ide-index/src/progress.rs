//! Progress and status model of the indexing scheduler (status-bar widget,
//! `index.status`, banners in popups).

use crate::store::StoreKind;
use archipel_ide_analysis::index::IndexPriority;
use archipel_ide_analysis::{IndexId, IndexState};
use serde_json::{Value, json};
use std::path::PathBuf;

/// What the scheduler is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// No project attached.
    Detached,
    /// Listing the project's files.
    Scanning,
    /// Building name indexes or indexing file contents.
    Indexing,
    Idle,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Detached => "detached",
            Phase::Scanning => "scanning",
            Phase::Indexing => "indexing",
            Phase::Idle => "idle",
        }
    }
}

/// One index and its readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexInfo {
    pub id: IndexId,
    pub state: IndexState,
    pub version: String,
    pub dumb_aware: bool,
    pub priority: IndexPriority,
}

/// A file the indexer did not index, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedFile {
    pub path: PathBuf,
    pub reason: String,
}

/// Snapshot of the scheduler for the UI and `index.status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexStatus {
    pub attached: bool,
    pub root: Option<PathBuf>,
    pub phase: Phase,
    /// Some index a feature may need is not ready.
    pub dumb: bool,
    /// Files of the running sweep that are done / total.
    pub done: u32,
    pub total: u32,
    /// Files waiting in the queues.
    pub queued: u32,
    pub running_jobs: u32,
    /// Files known to the index.
    pub files: u32,
    pub indexes: Vec<IndexInfo>,
    pub skipped: Vec<SkippedFile>,
    pub skipped_total: u32,
    pub store: StoreKind,
    /// Why the persistent store is not in use, when it is not.
    pub store_error: Option<String>,
    /// Milliseconds the last full scan took.
    pub scan_ms: Option<u64>,
}

impl IndexStatus {
    pub fn detached() -> Self {
        Self {
            attached: false,
            root: None,
            phase: Phase::Detached,
            dumb: false,
            done: 0,
            total: 0,
            queued: 0,
            running_jobs: 0,
            files: 0,
            indexes: Vec::new(),
            skipped: Vec::new(),
            skipped_total: 0,
            store: StoreKind::Memory,
            store_error: None,
            scan_ms: None,
        }
    }

    pub fn is_busy(&self) -> bool {
        matches!(self.phase, Phase::Scanning | Phase::Indexing)
    }

    /// Text of the status-bar item ("Indexing 1,204 / 20,000 files").
    pub fn summary(&self) -> Option<String> {
        match self.phase {
            Phase::Detached | Phase::Idle => None,
            Phase::Scanning => Some("Scanning files…".into()),
            Phase::Indexing => Some(format!(
                "Indexing {} / {} files",
                group(self.done),
                group(self.total.max(self.done))
            )),
        }
    }

    /// Fraction of the sweep done, 0..=1.
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        (self.done as f32 / self.total as f32).clamp(0.0, 1.0)
    }

    pub fn state_of(&self, id: IndexId) -> IndexState {
        self.indexes
            .iter()
            .find(|info| info.id == id)
            .map_or(IndexState::Absent, |info| info.state)
    }

    /// The JSON of the `index.status` command.
    pub fn to_json(&self) -> Value {
        json!({
            "attached": self.attached,
            "root": self.root,
            "phase": self.phase.label(),
            "dumb": self.dumb,
            "done": self.done,
            "total": self.total,
            "queued": self.queued,
            "runningJobs": self.running_jobs,
            "files": self.files,
            "store": match self.store { StoreKind::Memory => "memory", StoreKind::Persistent => "persistent" },
            "storeError": self.store_error,
            "scanMs": self.scan_ms,
            "skippedTotal": self.skipped_total,
            "skipped": self.skipped.iter().map(|s| json!({"path": s.path, "reason": s.reason})).collect::<Vec<_>>(),
            "indexes": self.indexes.iter().map(|info| json!({
                "id": info.id.0,
                "state": state_label(info.state),
                "done": match info.state { IndexState::Building { done, .. } => Some(done), _ => None },
                "total": match info.state { IndexState::Building { total, .. } => Some(total), _ => None },
                "version": info.version,
                "dumbAware": info.dumb_aware,
                "priority": format!("{:?}", info.priority),
            })).collect::<Vec<_>>(),
        })
    }
}

pub fn state_label(state: IndexState) -> &'static str {
    match state {
        IndexState::Absent => "absent",
        IndexState::Building { .. } => "building",
        IndexState::Ready => "ready",
        IndexState::Failed => "failed",
    }
}

fn group(value: u32) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (at, c) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_groups_digits() {
        let mut status = IndexStatus::detached();
        status.phase = Phase::Indexing;
        status.done = 1204;
        status.total = 20000;
        assert_eq!(
            status.summary().as_deref(),
            Some("Indexing 1,204 / 20,000 files")
        );
        assert!((status.fraction() - 0.0602).abs() < 1e-3);
        status.phase = Phase::Idle;
        assert_eq!(status.summary(), None);
    }
}

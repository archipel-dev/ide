//! Session persistence: metadata and transcripts as JSON under the data directory.
//!
//! `<data_dir>/archipel/agents/<repo-dir>/sessions/<id>/meta.json` holds the session's
//! metadata (never environment variables or credentials) and `transcript.jsonl` one
//! [`Update`] per line, passed through the [`Redactor`] before it is written. Restoring a
//! session replays the log through the same reducer a live session uses.

use crate::redact::Redactor;
use crate::update::Update;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: u32,
    pub task: u32,
    pub task_title: String,
    pub task_prompt: String,
    pub parent: Option<u32>,
    #[serde(default)]
    pub depth: u32,
    /// `claude` or `codex`.
    pub kind: String,
    pub name: String,
    pub branch: String,
    pub worktree: PathBuf,
    pub base_branch: String,
    pub provider_session: Option<String>,
    pub model: Option<String>,
    /// Permission mode id.
    pub mode: String,
    /// `working`, `needs_input`, `done`, `failed` or `stopped`.
    pub state: String,
    /// `squash`, `merge` or `discard` once handled.
    pub outcome: Option<String>,
    pub tokens: u32,
    pub cost_usd: f64,
    pub elapsed_ms: u64,
    pub unread: bool,
    /// Whether the worktree exists (a launch may have failed before creating it).
    #[serde(default)]
    pub worktree_ready: bool,
}

/// `<data_dir>/archipel/agents/<repo-dir>`.
pub fn repo_root_dir(data_dir: &Path, repo_dir_name: &str) -> PathBuf {
    data_dir.join("archipel").join("agents").join(repo_dir_name)
}

pub struct Store {
    sessions: PathBuf,
}

impl Store {
    pub fn new(repo_dir: &Path) -> Self {
        Self {
            sessions: repo_dir.join("sessions"),
        }
    }

    fn dir(&self, id: u32) -> PathBuf {
        self.sessions.join(id.to_string())
    }

    /// Writes the metadata atomically (temporary file, then rename).
    pub fn save_meta(&self, meta: &SessionMeta) -> io::Result<()> {
        let dir = self.dir(meta.id);
        fs::create_dir_all(&dir)?;
        let tmp = dir.join("meta.json.tmp");
        let json = serde_json::to_vec_pretty(meta).map_err(io::Error::other)?;
        fs::write(&tmp, json)?;
        fs::rename(&tmp, dir.join("meta.json"))
    }

    pub fn append(&self, id: u32, update: &Update, redactor: &Redactor) -> io::Result<()> {
        let dir = self.dir(id);
        fs::create_dir_all(&dir)?;
        let line = serde_json::to_string(update).map_err(io::Error::other)?;
        let line = redactor.redact(&line);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("transcript.jsonl"))?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")
    }

    /// Every stored session with its transcript, by id. Unreadable sessions and lines are
    /// skipped.
    pub fn load_all(&self) -> Vec<(SessionMeta, Vec<Update>)> {
        let Ok(entries) = fs::read_dir(&self.sessions) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let dir = entry.path();
            let Some(meta) = fs::read(dir.join("meta.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<SessionMeta>(&bytes).ok())
            else {
                continue;
            };
            let updates = fs::File::open(dir.join("transcript.jsonl"))
                .map(|file| {
                    io::BufReader::new(file)
                        .lines()
                        .map_while(Result::ok)
                        .filter_map(|line| serde_json::from_str::<Update>(&line).ok())
                        .collect()
                })
                .unwrap_or_default();
            out.push((meta, updates));
        }
        out.sort_by_key(|(meta, _)| meta.id);
        out
    }

    /// Removes a session's files.
    pub fn delete(&self, id: u32) {
        let _ = fs::remove_dir_all(self.dir(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::AgentEvent;

    fn meta(id: u32) -> SessionMeta {
        SessionMeta {
            id,
            task: 1,
            task_title: "t".into(),
            task_prompt: "p".into(),
            parent: None,
            depth: 0,
            kind: "claude".into(),
            name: "n".into(),
            branch: "archipel/n".into(),
            worktree: "/w".into(),
            base_branch: "main".into(),
            provider_session: Some("abc".into()),
            model: None,
            mode: "ask".into(),
            state: "working".into(),
            outcome: None,
            tokens: 3,
            cost_usd: 0.0,
            elapsed_ms: 10,
            unread: false,
            worktree_ready: true,
        }
    }

    #[test]
    fn meta_and_transcript_round_trip_and_secrets_stay_out() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let redactor = Redactor::new(["hunter2-hunter2".to_string()]);
        store.save_meta(&meta(2)).unwrap();
        store.save_meta(&meta(1)).unwrap();
        for text in ["hello", "the key hunter2-hunter2 leaked"] {
            store
                .append(1, &Update::Push { event: AgentEvent::AssistantText(text.into()) }, &redactor)
                .unwrap();
        }
        let loaded = store.load_all();
        assert_eq!(loaded.iter().map(|(m, _)| m.id).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(loaded[0].1.len(), 2);
        let raw = fs::read_to_string(dir.path().join("sessions/1/transcript.jsonl")).unwrap();
        assert!(!raw.contains("hunter2"));
        store.delete(1);
        assert_eq!(store.load_all().len(), 1);
    }

    #[test]
    fn damaged_files_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        store.save_meta(&meta(1)).unwrap();
        fs::write(dir.path().join("sessions/1/transcript.jsonl"), "{broken\n\n{\"kind\":\"nope\"}\n").unwrap();
        fs::create_dir_all(dir.path().join("sessions/9")).unwrap();
        fs::write(dir.path().join("sessions/9/meta.json"), "not json").unwrap();
        let loaded = store.load_all();
        assert_eq!(loaded.len(), 1);
        assert!(loaded[0].1.is_empty());
    }
}

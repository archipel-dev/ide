//! Completion statistics: items the user accepts rank higher next time, as
//! in the reference IDE. Kept per user (not per project) in
//! `<config dir>/archipel-ide/completion-stats.json`, loaded on first use and
//! written with the workspace.

use archipel_ide_lsp::lsp_types::CompletionItem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// Entries kept; the least recently used are dropped beyond this.
const MAX_ENTRIES: usize = 5_000;
/// Uses counted per item; older habits fade as newer ones are added.
const MAX_COUNT: u32 = 20;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
struct Entry {
    count: u32,
    /// Monotonic use sequence (recency).
    last: u64,
}

#[derive(Debug, Default)]
struct Inner {
    path: Option<PathBuf>,
    loaded: bool,
    dirty: bool,
    sequence: u64,
    entries: HashMap<String, Entry>,
}

/// Shared, internally synchronized statistics (read while ranking, written
/// when an item is accepted and when the workspace is saved).
#[derive(Debug, Default)]
pub struct CompletionStats {
    inner: Mutex<Inner>,
}

#[derive(Serialize, Deserialize)]
struct File {
    sequence: u64,
    entries: HashMap<String, Entry>,
}

/// Statistics key: language, identifier and kind.
fn key(language: &str, item: &CompletionItem) -> String {
    let kind = item
        .kind
        .and_then(|kind| serde_json::to_value(kind).ok())
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    format!(
        "{language}:{}:{kind}",
        archipel_ide_lsp::completion::filter_text(item)
    )
}

impl CompletionStats {
    /// Statistics stored at `path` (`None`: memory only).
    pub fn at(path: Option<PathBuf>) -> Self {
        Self {
            inner: Mutex::new(Inner {
                path,
                ..Default::default()
            }),
        }
    }

    /// The per-user statistics file.
    pub fn user() -> Self {
        Self::at(
            archipel_ide_lsp::registry::config_dir().map(|dir| dir.join("completion-stats.json")),
        )
    }

    fn with<T>(&self, f: impl FnOnce(&mut Inner) -> T) -> T {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.load();
        f(&mut inner)
    }

    /// Records that `item` was accepted in a `language` document.
    pub fn record(&self, language: &str, item: &CompletionItem) {
        self.with(|inner| inner.record(language, item));
    }

    /// Ranking weight of `item`: uses, with a bonus for the most recent picks.
    pub fn weight(&self, language: &str, item: &CompletionItem) -> i32 {
        self.with(|inner| inner.weight(language, item))
    }

    /// Writes the statistics if they changed.
    pub fn save(&self) -> Result<(), String> {
        self.with(Inner::save)
    }
}

impl Inner {
    fn load(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        let Some(text) = self
            .path
            .as_ref()
            .and_then(|path| std::fs::read_to_string(path).ok())
        else {
            return;
        };
        if let Ok(file) = serde_json::from_str::<File>(&text) {
            self.sequence = self.sequence.max(file.sequence);
            for (key, entry) in file.entries {
                self.entries.entry(key).or_insert(entry);
            }
        }
    }

    fn record(&mut self, language: &str, item: &CompletionItem) {
        self.sequence += 1;
        let entry = self.entries.entry(key(language, item)).or_default();
        entry.count = (entry.count + 1).min(MAX_COUNT);
        entry.last = self.sequence;
        self.dirty = true;
        if self.entries.len() > MAX_ENTRIES {
            let mut by_age: Vec<(u64, String)> = self
                .entries
                .iter()
                .map(|(key, entry)| (entry.last, key.clone()))
                .collect();
            by_age.sort_unstable();
            for (_, key) in by_age.into_iter().take(self.entries.len() - MAX_ENTRIES) {
                self.entries.remove(&key);
            }
        }
    }

    fn weight(&self, language: &str, item: &CompletionItem) -> i32 {
        let Some(entry) = self.entries.get(&key(language, item)) else {
            return 0;
        };
        let recent = self.sequence.saturating_sub(entry.last) < 10;
        entry.count as i32 * 2 + i32::from(recent)
    }

    fn save(&mut self) -> Result<(), String> {
        if !self.dirty {
            return Ok(());
        }
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let file = File {
            sequence: self.sequence,
            entries: self.entries.clone(),
        };
        let json = serde_json::to_string(&file).map_err(|error| error.to_string())?;
        // Write-then-rename so a crash never leaves a truncated file.
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, json).map_err(|error| error.to_string())?;
        std::fs::rename(&temp, path).map_err(|error| error.to_string())?;
        self.dirty = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str) -> CompletionItem {
        CompletionItem {
            label: label.into(),
            ..Default::default()
        }
    }

    #[test]
    fn accepted_items_gain_weight_and_persist() {
        let path =
            std::env::temp_dir().join(format!("archipel-ide-stats-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let stats = CompletionStats::at(Some(path.clone()));
        assert_eq!(stats.weight("java", &item("toString")), 0);
        stats.record("java", &item("toString"));
        stats.record("java", &item("toString"));
        assert!(stats.weight("java", &item("toString")) > 0);
        assert_eq!(stats.weight("rust", &item("toString")), 0, "per language");
        stats.save().unwrap();
        let reloaded = CompletionStats::at(Some(path.clone()));
        assert_eq!(
            reloaded.weight("java", &item("toString")),
            stats.weight("java", &item("toString"))
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn unchanged_or_memory_only_statistics_are_not_written() {
        let memory = CompletionStats::at(None);
        memory.record("java", &item("a"));
        memory.save().unwrap();
        let path = std::env::temp_dir().join(format!(
            "archipel-ide-stats-clean-{}.json",
            std::process::id()
        ));
        let clean = CompletionStats::at(Some(path.clone()));
        clean.save().unwrap();
        assert!(!path.exists());
    }
}

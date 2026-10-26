//! Recently opened projects, shown on the welcome screen. Stored as a small JSON file.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const LIMIT: usize = 30;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecentProject {
    pub path: PathBuf,
    pub name: String,
}

#[derive(Debug, Default, Clone)]
pub struct RecentProjects {
    store: Option<PathBuf>,
    entries: Vec<RecentProject>,
}

impl RecentProjects {
    /// Loads `store`; a missing or unreadable file yields an empty list.
    pub fn load(store: Option<PathBuf>) -> Self {
        let entries = store
            .as_deref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self { store, entries }
    }

    pub fn entries(&self) -> &[RecentProject] {
        &self.entries
    }

    /// Moves `path` to the front.
    pub fn record(&mut self, path: &Path) {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into(),
        );
        self.entries.retain(|entry| entry.path != path);
        self.entries.insert(
            0,
            RecentProject {
                path: path.to_owned(),
                name,
            },
        );
        self.entries.truncate(LIMIT);
        self.save();
    }

    pub fn remove(&mut self, path: &Path) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.path != path);
        let changed = self.entries.len() != before;
        if changed {
            self.save();
        }
        changed
    }

    /// Entries whose name or path contains `query` (case-insensitive).
    pub fn search(&self, query: &str) -> Vec<&RecentProject> {
        let query = query.trim().to_lowercase();
        self.entries
            .iter()
            .filter(|entry| {
                query.is_empty()
                    || entry.name.to_lowercase().contains(&query)
                    || entry.path.to_string_lossy().to_lowercase().contains(&query)
            })
            .collect()
    }

    fn save(&self) {
        let Some(path) = &self.store else { return };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(&self.entries) {
            let _ = std::fs::write(path, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_dedupes_searches_and_persists() {
        let dir = std::env::temp_dir().join(format!("archipel-ide-recent-{}", std::process::id()));
        let store = dir.join("recent.json");
        let mut recent = RecentProjects::load(Some(store.clone()));
        recent.record(Path::new("/w/alpha"));
        recent.record(Path::new("/w/beta"));
        recent.record(Path::new("/w/alpha"));
        let names: Vec<_> = recent.entries().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["alpha", "beta"]);
        assert_eq!(recent.search("BET").len(), 1);
        assert_eq!(recent.search("/w/").len(), 2);
        let mut reloaded = RecentProjects::load(Some(store));
        assert_eq!(reloaded.entries().len(), 2);
        assert!(reloaded.remove(Path::new("/w/beta")));
        assert!(!reloaded.remove(Path::new("/w/beta")));
        let _ = std::fs::remove_dir_all(dir);
    }
}

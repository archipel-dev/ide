//! Recursive file system watcher for a project root.
//!
//! Events arrive on a background thread and are batched until the IDE polls,
//! so a burst (formatter, `git checkout`) becomes one refresh. Build output and
//! workspace metadata are ignored; changes inside `.git` are only flagged, for
//! VCS refreshes.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use notify::{EventKind, RecursiveMode, Watcher, event::ModifyKind};

/// Directories whose contents never matter to open editors or the tree.
/// `.jetthing` is the data directory of versions before the rename to Archipel IDE.
const IGNORED: [&str; 5] = [
    ".archipel",
    ".jetthing",
    "target",
    "node_modules",
    ".gradle",
];

/// Changes collected since the last poll.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FileChanges {
    /// Project-relative paths whose content or existence changed.
    pub paths: BTreeSet<PathBuf>,
    /// A file or directory was created, removed or renamed.
    pub structure_changed: bool,
    /// Something under `.git` changed (index, HEAD, refs).
    pub git_changed: bool,
}

impl FileChanges {
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty() && !self.structure_changed && !self.git_changed
    }

    /// Folds one raw event (absolute paths) into the batch.
    pub fn record(&mut self, root: &Path, kind: &EventKind, paths: &[PathBuf]) {
        let structural = matches!(
            kind,
            EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(_))
        );
        if matches!(kind, EventKind::Access(_)) {
            return;
        }
        for path in paths {
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let mut components = relative.components();
            let first = components
                .next()
                .and_then(|component| component.as_os_str().to_str());
            match first {
                Some(".git") => self.git_changed = true,
                Some(name) if IGNORED.contains(&name) => {}
                None => {}
                Some(_) => {
                    self.paths.insert(relative.to_owned());
                    self.structure_changed |= structural;
                }
            }
        }
    }
}

pub struct ProjectWatcher {
    root: PathBuf,
    events: Receiver<notify::Result<notify::Event>>,
    // Dropping the watcher stops the background thread.
    _watcher: notify::RecommendedWatcher,
}

impl ProjectWatcher {
    /// Watches `root` recursively. `wake` runs on the watcher thread after
    /// each event so an idle UI can poll.
    pub fn new(root: &Path, wake: impl Fn() + Send + 'static) -> Result<Self, notify::Error> {
        let (sender, events) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |event| {
            if sender.send(event).is_ok() {
                wake();
            }
        })?;
        watcher.watch(root, RecursiveMode::Recursive)?;
        Ok(Self {
            root: root.to_owned(),
            events,
            _watcher: watcher,
        })
    }

    /// Drains pending events; never blocks. A watcher error (e.g. dropped
    /// events on overflow) marks everything as possibly changed.
    pub fn poll(&self) -> FileChanges {
        let mut changes = FileChanges::default();
        while let Ok(event) = self.events.try_recv() {
            match event {
                Ok(event) => changes.record(&self.root, &event.kind, &event.paths),
                Err(_) => {
                    changes.structure_changed = true;
                    changes.git_changed = true;
                }
            }
        }
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, DataChange};
    use std::time::{Duration, Instant};

    #[test]
    fn events_are_filtered_and_classified() {
        let root = Path::new("/p");
        let mut changes = FileChanges::default();
        changes.record(
            root,
            &EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            &[PathBuf::from("/p/src/App.java")],
        );
        assert!(!changes.structure_changed);
        changes.record(
            root,
            &EventKind::Create(CreateKind::File),
            &[
                PathBuf::from("/p/target/debug/x"),
                PathBuf::from("/p/.git/index"),
                PathBuf::from("/elsewhere/a"),
            ],
        );
        assert!(changes.git_changed);
        assert!(!changes.structure_changed, "ignored dirs are not structure");
        assert_eq!(
            changes.paths.iter().collect::<Vec<_>>(),
            [&PathBuf::from("src/App.java")]
        );
        changes.record(
            root,
            &EventKind::Create(CreateKind::File),
            &[PathBuf::from("/p/notes.txt")],
        );
        assert!(changes.structure_changed);
    }

    #[test]
    fn real_writes_are_reported() {
        let root = std::env::temp_dir().join(format!("archipel-ide-watch-{}", std::process::id()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        let root = root.canonicalize().unwrap();
        std::fs::write(root.join("src/a.txt"), "one").unwrap();
        let watcher = ProjectWatcher::new(&root, || {}).unwrap();
        std::fs::write(root.join("src/a.txt"), "two").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut changes = FileChanges::default();
        while Instant::now() < deadline && !changes.paths.contains(Path::new("src/a.txt")) {
            let next = watcher.poll();
            changes.paths.extend(next.paths);
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            changes.paths.contains(Path::new("src/a.txt")),
            "{changes:?}"
        );
        let _ = std::fs::remove_dir_all(root);
    }
}

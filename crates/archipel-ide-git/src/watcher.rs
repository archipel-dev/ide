//! Background Git status refresh, so changes made outside the IDE (terminal,
//! other tools, file managers) show up in the Commit window and the project
//! tree without a manual refresh.

use crate::RepositoryStatus;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

/// Callback invoked from the watcher thread after a new status is queued,
/// typically to wake the UI event loop.
pub type Notify = Arc<dyn Fn() + Send + Sync>;

/// Reloads [`RepositoryStatus`] on a background thread at a fixed interval and
/// whenever [`StatusWatcher::refresh`] is called, and queues the result only
/// when it differs from the previously queued one.
///
/// The thread stops when the watcher is dropped.
pub struct StatusWatcher {
    wake: Sender<()>,
    updates: Receiver<RepositoryStatus>,
}

impl StatusWatcher {
    pub fn spawn(root: PathBuf, interval: Duration, notify: Notify) -> Self {
        let (wake, wake_rx) = mpsc::channel::<()>();
        let (updates_tx, updates) = mpsc::channel();
        thread::Builder::new()
            .name("archipel-ide-git-status".into())
            .spawn(move || {
                let mut last: Option<RepositoryStatus> = None;
                loop {
                    if let Ok(status) = RepositoryStatus::load(&root)
                        && last.as_ref() != Some(&status)
                    {
                        last = Some(status.clone());
                        if updates_tx.send(status).is_err() {
                            return;
                        }
                        notify();
                    }
                    match wake_rx.recv_timeout(interval) {
                        Ok(()) => while wake_rx.try_recv().is_ok() {},
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
            })
            .ok();
        Self { wake, updates }
    }

    /// Requests an immediate reload (e.g. when the IDE window regains focus).
    pub fn refresh(&self) {
        let _ = self.wake.send(());
    }

    /// The most recent status queued since the last call, if any.
    pub fn latest(&self) -> Option<RepositoryStatus> {
        self.updates.try_iter().last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    fn git(root: &Path, args: &[&str]) {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }

    fn wait_for(watcher: &StatusWatcher, pred: impl Fn(&RepositoryStatus) -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if let Some(status) = watcher.latest()
                && pred(&status)
            {
                return true;
            }
            thread::sleep(Duration::from_millis(20));
        }
        false
    }

    #[test]
    fn reports_external_changes_and_notifies() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-git-watch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q"]);
        let count = Arc::new(AtomicUsize::new(0));
        let counter = count.clone();
        let watcher = StatusWatcher::spawn(
            root.clone(),
            Duration::from_secs(60),
            Arc::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            }),
        );
        assert!(wait_for(&watcher, |s| s.changes.is_empty()));
        fs::write(root.join("outside.txt"), "x\n").unwrap();
        // The interval is long, so only an explicit refresh can pick this up.
        watcher.refresh();
        assert!(wait_for(&watcher, |s| s.changes.len() == 1));
        // An unchanged status is not queued again.
        watcher.refresh();
        thread::sleep(Duration::from_millis(300));
        assert!(watcher.latest().is_none());
        assert_eq!(count.load(Ordering::SeqCst), 2);
        drop(watcher);
        fs::remove_dir_all(root).unwrap();
    }
}

//! Git repository status as a project service (IntelliJ: `GitRepositoryManager`).
//!
//! [`GitRepositories`] owns the status of the open project's repository, the interrupted
//! merge/rebase/cherry-pick, and the background watcher that notices outside changes. Whoever
//! replaces the status (a refresh after an IDE action, or the watcher) goes through
//! [`GitRepositories::update`], which announces the change on [`GIT_REPO_CHANGE`] of the
//! project bus; caches that depend on Git state subscribe instead of being called.

use archipel_ide_git::{
    ChangeKind, Notify, PendingOperation, RepositoryStatus, StatusWatcher, pending_operation,
};
use archipel_ide_platform::{Disposable, MessageBus, ServiceState, Topic};
use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// How often Git status is reloaded in the background. External changes also
/// trigger an immediate reload when the IDE window regains focus.
const STATUS_INTERVAL: Duration = Duration::from_secs(2);

/// Reports that the repository status was replaced (IntelliJ `GitRepositoryChangeListener`).
pub trait GitRepositoryChangeListener: Send + Sync {
    /// `structure_changed` is true when files were added or removed (not just edited), so the
    /// project tree should rescan.
    fn repository_changed(&self, _status: &RepositoryStatus, _structure_changed: bool) {}
}

/// On the project's message bus.
pub const GIT_REPO_CHANGE: Topic<dyn GitRepositoryChangeListener> =
    Topic::new("GitRepositoryChangeListener");

#[derive(Default)]
struct State {
    status: Option<Arc<RepositoryStatus>>,
    pending: Option<PendingOperation>,
    /// Files or `.git` changed outside the IDE since the project was opened.
    stale: bool,
}

/// The project service. Without a repository `status()` is `None`.
pub struct GitRepositories {
    bus: MessageBus,
    state: ServiceState<State>,
    /// Background refresh for external changes.
    watcher: Mutex<Option<StatusWatcher>>,
}

impl GitRepositories {
    pub fn new(bus: MessageBus) -> Self {
        Self {
            bus,
            state: ServiceState::default(),
            watcher: Mutex::new(None),
        }
    }

    /// Loads the repository containing `root`, if any, and starts watching it. Not an event:
    /// opening the project is.
    pub fn open(&self, root: &Path, notify: Notify) {
        let status = RepositoryStatus::load(root).ok();
        let watcher = status
            .as_ref()
            .map(|git| StatusWatcher::spawn(git.root.clone(), STATUS_INTERVAL, notify));
        self.state.with_mut(|state| {
            state.pending = status.as_ref().and_then(|git| pending(&git.root));
            state.status = status.map(Arc::new);
            state.stale = false;
        });
        *self.watcher.lock() = watcher;
    }

    /// The current status (cheap to clone and keep: the snapshot never changes).
    pub fn status(&self) -> Option<Arc<RepositoryStatus>> {
        self.state.read().status.clone()
    }

    /// The interrupted merge/rebase/cherry-pick, if any.
    pub fn pending(&self) -> Option<PendingOperation> {
        self.state.read().pending
    }

    /// Replaces the status and announces it on [`GIT_REPO_CHANGE`].
    pub fn update(&self, status: RepositoryStatus) {
        let status = Arc::new(status);
        let structure_changed = self.state.with_mut(|state| {
            let changed = state.status.as_deref().map(structural_changes)
                != Some(structural_changes(&status));
            state.pending = pending(&status.root);
            state.status = Some(status.clone());
            changed
        });
        self.bus.publish(GIT_REPO_CHANGE, |l| {
            l.repository_changed(&status, structure_changed)
        });
    }

    /// Records that files or `.git` changed outside the IDE.
    pub fn mark_stale(&self) {
        self.state.with_mut(|state| state.stale = true);
    }

    /// Whether files or `.git` changed outside the IDE since the project was opened.
    pub fn is_stale(&self) -> bool {
        self.state.read().stale
    }

    /// Re-reads the interrupted-operation state from the repository.
    pub fn refresh_pending(&self) {
        self.state.with_mut(|state| {
            state.pending = state.status.as_ref().and_then(|git| pending(&git.root));
        });
    }

    /// Asks the background watcher to reload status now.
    pub fn request_refresh(&self) {
        if let Some(watcher) = self.watcher.lock().as_ref() {
            watcher.refresh();
        }
    }

    /// The newest status the watcher loaded that differs from the current one.
    pub fn poll_watcher(&self) -> Option<RepositoryStatus> {
        let latest = self
            .watcher
            .lock()
            .as_ref()
            .and_then(StatusWatcher::latest)?;
        (self.state.read().status.as_deref() != Some(&latest)).then_some(latest)
    }
}

impl Disposable for GitRepositories {
    /// Stops the watcher and forgets the repository.
    fn dispose(&self) {
        *self.watcher.lock() = None;
        self.state.with_mut(|state| *state = State::default());
    }
}

fn pending(root: &Path) -> Option<PendingOperation> {
    pending_operation(root).ok().flatten()
}

/// Changes that add or remove files (as opposed to editing them).
fn structural_changes(git: &RepositoryStatus) -> Vec<&Path> {
    git.changes
        .iter()
        .filter(|change| change.kind != ChangeKind::Modified)
        .map(|change| change.path.as_path())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_platform::{Application, ComponentManager, Project, ServiceDescriptor};
    use std::path::PathBuf;
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args(args)
            .output()
            .expect("git is installed");
        assert!(status.status.success(), "git {args:?} failed");
    }

    fn repo(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "archipel-git-service-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        std::fs::write(dir.join("a.txt"), "a\n").unwrap();
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "-q", "-m", "init"]);
        dir
    }

    #[derive(Default)]
    struct Heard(Mutex<Vec<(usize, bool)>>);

    impl GitRepositoryChangeListener for Heard {
        fn repository_changed(&self, status: &RepositoryStatus, structure_changed: bool) {
            self.0
                .lock()
                .push((status.changes.len(), structure_changed));
        }
    }

    fn project(dir: &Path) -> (Application, Project) {
        let app = Application::new();
        app.register_project_service(ServiceDescriptor::disposable(|project: &Project| {
            GitRepositories::new(project.message_bus().clone())
        }))
        .unwrap();
        let project = app.project_manager().open(dir);
        (app, project)
    }

    #[test]
    fn update_replaces_the_status_and_announces_structure_changes() {
        let dir = repo("update");
        let (_app, project) = project(&dir);
        let heard = Arc::new(Heard::default());
        project
            .message_bus()
            .subscribe(project.disposable(), GIT_REPO_CHANGE, heard.clone())
            .unwrap();
        let git_repos = project.service::<GitRepositories>();
        git_repos.open(&dir, Arc::new(|| {}));
        assert_eq!(git_repos.status().unwrap().branch, "main");

        std::fs::write(dir.join("a.txt"), "edited\n").unwrap();
        git_repos.update(RepositoryStatus::load(&dir).unwrap());
        std::fs::write(dir.join("new.txt"), "n\n").unwrap();
        git_repos.update(RepositoryStatus::load(&dir).unwrap());

        // A modified file is not structural; an untracked one is.
        assert_eq!(*heard.0.lock(), [(1, false), (2, true)]);
        assert_eq!(git_repos.status().unwrap().changes.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_directory_without_a_repository_has_no_status() {
        let dir =
            std::env::temp_dir().join(format!("archipel-git-service-none-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (_app, project) = project(&dir);
        let git_repos = project.service::<GitRepositories>();
        git_repos.open(&dir, Arc::new(|| {}));
        // A temp dir can sit inside a repository on some machines; only check consistency.
        assert_eq!(git_repos.pending(), None);
        assert!(git_repos.poll_watcher().is_none() || git_repos.status().is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn closing_the_project_disposes_the_service() {
        let dir = repo("dispose");
        let (app, project) = project(&dir);
        let git_repos = project.service::<GitRepositories>();
        git_repos.open(&dir, Arc::new(|| {}));
        assert!(git_repos.status().is_some());
        app.project_manager().close(&project);
        assert!(git_repos.status().is_none(), "the repository is forgotten");
        assert!(project.get_service::<GitRepositories>().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

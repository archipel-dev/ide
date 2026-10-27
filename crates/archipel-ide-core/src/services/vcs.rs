//! The VCS project services that sit next to [`GitRepositories`](super::GitRepositories):
//! the Git Log tab ([`GitLogService`]), blame annotations ([`BlameAnnotations`]) and the gutter
//! change markers ([`LineStatusTracker`]).
//!
//! Each caches something derived from the repository, so each subscribes to
//! [`GIT_REPO_CHANGE`] on the project bus and drops what the new status invalidated. Nobody
//! calls them after a commit, checkout or pull: the repository announces it.

use super::git::{GIT_REPO_CHANGE, GitRepositoryChangeListener};
use crate::GitLogState;
use crate::annotations::Annotation;
use crate::line_changes::LineChanges;
use archipel_ide_git::{FileDiff, Hunk, RepositoryStatus};
use archipel_ide_platform::{ComponentManager, MessageBus, Project, ServiceState, Topic};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

// ------------------------------------------------------------------------------ Git Log

/// The Git tool window's Log tab (IntelliJ: `VcsLogData`). Readers take an [`Arc`] snapshot.
pub struct GitLogService {
    state: Arc<ServiceState<Arc<GitLogState>>>,
}

struct InvalidateLog(Arc<ServiceState<Arc<GitLogState>>>);

impl GitRepositoryChangeListener for InvalidateLog {
    fn repository_changed(&self, _status: &RepositoryStatus, _structure_changed: bool) {
        self.0.with_mut(|log| {
            if log.loaded {
                Arc::make_mut(log).loaded = false;
            }
        });
    }
}

impl GitLogService {
    /// Creates the service of `project` and follows its repository.
    pub fn connect(project: &Project) -> Self {
        let state = Arc::new(ServiceState::new(Arc::new(GitLogState::default())));
        let _ = project.message_bus().subscribe(
            project.disposable(),
            GIT_REPO_CHANGE,
            Arc::new(InvalidateLog(state.clone())),
        );
        Self { state }
    }

    /// The log as of now (cheap: the snapshot never changes).
    pub fn snapshot(&self) -> Arc<GitLogState> {
        self.state.read().clone()
    }

    /// Changes the log. Snapshots taken earlier are unaffected.
    pub fn with_mut<R>(&self, f: impl FnOnce(&mut GitLogState) -> R) -> R {
        self.state.with_mut(|log| f(Arc::make_mut(log)))
    }

    /// Marks the log stale; it reloads the next time it is shown.
    pub fn invalidate(&self) {
        self.with_mut(|log| log.loaded = false);
    }
}

// ------------------------------------------------------------------------------ Annotations

/// Minimum time between blame runs while a document is being edited.
pub(crate) const REBLAME_INTERVAL: Duration = Duration::from_millis(800);

type Annotations = ServiceState<HashMap<PathBuf, Annotation>>;

/// Blame annotations of the documents that have them switched on.
pub struct BlameAnnotations {
    files: Arc<Annotations>,
}

struct Reblame(Arc<Annotations>);

impl GitRepositoryChangeListener for Reblame {
    fn repository_changed(&self, _status: &RepositoryStatus, _structure_changed: bool) {
        BlameAnnotations::invalidate_all(&self.0);
    }
}

impl BlameAnnotations {
    /// Creates the service of `project` and follows its repository.
    pub fn connect(project: &Project) -> Self {
        let files = Arc::new(Annotations::default());
        let _ = project.message_bus().subscribe(
            project.disposable(),
            GIT_REPO_CHANGE,
            Arc::new(Reblame(files.clone())),
        );
        Self { files }
    }

    fn invalidate_all(files: &Annotations) {
        files.with_mut(|files| {
            for annotation in files.values_mut() {
                annotation.invalidate(REBLAME_INTERVAL);
            }
        });
    }

    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, HashMap<PathBuf, Annotation>> {
        self.files.read()
    }

    pub fn write(&self) -> parking_lot::RwLockWriteGuard<'_, HashMap<PathBuf, Annotation>> {
        self.files.write()
    }
}

// ------------------------------------------------------------------------------ Line status

/// Reports that cached line status was dropped (IntelliJ: `LineStatusTrackerListener`).
pub trait LineStatusListener: Send + Sync {
    /// The `HEAD` contents the markers compare with changed (commit, checkout, pull): every
    /// editor gutter must recompute.
    fn line_status_invalidated(&self) {}
}

/// On the project's message bus.
pub const LINE_STATUS_TOPIC: Topic<dyn LineStatusListener> = Topic::new("LineStatusListener");

type Files = ServiceState<HashMap<PathBuf, LineChanges>>;

/// Changed lines of the open documents against `HEAD` (IntelliJ: `LineStatusTrackerManager`).
pub struct LineStatusTracker {
    bus: MessageBus,
    files: Arc<Files>,
}

struct DropBases {
    bus: MessageBus,
    files: Arc<Files>,
}

impl GitRepositoryChangeListener for DropBases {
    fn repository_changed(&self, _status: &RepositoryStatus, _structure_changed: bool) {
        self.files.with_mut(HashMap::clear);
        self.bus
            .publish(LINE_STATUS_TOPIC, |l| l.line_status_invalidated());
    }
}

impl LineStatusTracker {
    /// Creates the service of `project`; a new repository status drops the cached `HEAD` texts.
    pub fn connect(project: &Project) -> Self {
        let files = Arc::new(Files::default());
        let bus = project.message_bus().clone();
        let _ = bus.subscribe(
            project.disposable(),
            GIT_REPO_CHANGE,
            Arc::new(DropBases {
                bus: bus.clone(),
                files: files.clone(),
            }),
        );
        Self { bus, files }
    }

    /// Drops the cached `HEAD` texts, e.g. after a commit or checkout.
    pub fn invalidate(&self) {
        self.files.with_mut(HashMap::clear);
        self.bus
            .publish(LINE_STATUS_TOPIC, |l| l.line_status_invalidated());
    }

    /// Changed hunks of the document at `path` (project-relative), recomputed only when the
    /// document (`revision`) or the repository changed. `load_base` reads the `HEAD` text and
    /// `text` the document's current text; each runs only when needed.
    pub fn changes(
        &self,
        path: &Path,
        revision: u64,
        load_base: impl FnOnce() -> Option<String>,
        text: impl FnOnce() -> String,
    ) -> Vec<Hunk> {
        let mut files = self.files.write();
        let entry = files
            .entry(path.to_owned())
            .or_insert_with(|| LineChanges::with_base(load_base()));
        if entry.revision != Some(revision) {
            entry.hunks = match &entry.base {
                Some(base) => FileDiff::compute(path.to_owned(), None, base.clone(), text()).hunks,
                None => Vec::new(),
            };
            entry.revision = Some(revision);
        }
        entry.hunks.clone()
    }

    /// The cached `HEAD` text of `path`, as loaded by [`LineStatusTracker::changes`].
    pub fn base_text(&self, path: &Path) -> Option<String> {
        self.files.read().get(path)?.base.clone()
    }

    /// Whether `path` has a cached comparison.
    pub fn is_tracked(&self, path: &Path) -> bool {
        self.files.read().contains_key(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::GitRepositories;
    use archipel_ide_platform::{Application, ServiceDescriptor};
    use parking_lot::Mutex;
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args(args)
            .output()
            .expect("git is installed");
        assert!(output.status.success(), "git {args:?} failed");
    }

    fn open(name: &str) -> (PathBuf, Application, Project) {
        let dir = std::env::temp_dir().join(format!(
            "archipel-vcs-services-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        std::fs::write(dir.join("a.txt"), "a\n").unwrap();
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "-q", "-m", "init"]);
        let app = Application::new();
        app.register_project_service(ServiceDescriptor::disposable(|project: &Project| {
            GitRepositories::new(project.message_bus().clone())
        }))
        .unwrap();
        app.register_project_service(ServiceDescriptor::new(|project: &Project| {
            GitLogService::connect(project)
        }))
        .unwrap();
        app.register_project_service(ServiceDescriptor::new(|project: &Project| {
            LineStatusTracker::connect(project)
        }))
        .unwrap();
        app.register_project_service(ServiceDescriptor::new(|project: &Project| {
            BlameAnnotations::connect(project)
        }))
        .unwrap();
        let project = app.project_manager().open(&dir);
        (dir, app, project)
    }

    #[derive(Default)]
    struct Heard(Mutex<usize>);

    impl LineStatusListener for Heard {
        fn line_status_invalidated(&self) {
            *self.0.lock() += 1;
        }
    }

    #[test]
    fn a_repository_change_drops_the_cached_line_status_and_announces_it() {
        let (dir, _app, project) = open("line-status");
        let heard = Arc::new(Heard::default());
        project
            .message_bus()
            .subscribe(project.disposable(), LINE_STATUS_TOPIC, heard.clone())
            .unwrap();
        let tracker = project.service::<LineStatusTracker>();
        let path = Path::new("a.txt");
        std::fs::write(dir.join("a.txt"), "a\nb\n").unwrap();
        let hunks = tracker.changes(path, 1, || Some("a\n".into()), || "a\nb\n".into());
        assert_eq!(hunks.len(), 1);
        assert_eq!(tracker.base_text(path).as_deref(), Some("a\n"));
        // Same revision: neither the base nor the text is read again.
        let again = tracker.changes(
            path,
            1,
            || unreachable!("cached"),
            || unreachable!("cached"),
        );
        assert_eq!(again, hunks);

        let git_repos = project.service::<GitRepositories>();
        git_repos.update(RepositoryStatus::load(&dir).unwrap());
        assert_eq!(
            *heard.0.lock(),
            1,
            "the repository topic drives the tracker"
        );
        assert!(
            !tracker.is_tracked(path),
            "the cached HEAD text was dropped"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_repository_change_marks_the_log_stale_and_reblames() {
        let (dir, _app, project) = open("log");
        let log = project.service::<GitLogService>();
        log.with_mut(|log| log.loaded = true);
        let before = log.snapshot();
        let annotations = project.service::<BlameAnnotations>();
        annotations.write().insert(
            "a.txt".into(),
            Annotation {
                revision: Some(3),
                lines: Vec::new(),
                computed_at: std::time::Instant::now(),
            },
        );
        project
            .service::<GitRepositories>()
            .update(RepositoryStatus::load(&dir).unwrap());
        assert!(!log.snapshot().loaded);
        assert!(before.loaded, "a snapshot taken earlier is unchanged");
        assert_eq!(annotations.read()[Path::new("a.txt")].revision, None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

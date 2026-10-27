//! Fetch, pull, push and Get from VCS as background operations.
//!
//! [`GitRemoteOperations`] (project service) runs one fetch, pull or push at a time and
//! announces its end on [`REMOTE_TOPIC`] of the project bus. [`CloneManager`] (application
//! service: a clone runs before any project exists) runs one `git clone` and announces its end
//! on [`CLONE_TOPIC`] of the application bus.

use archipel_ide_git::{Notify, RemoteEvent, RemoteJob, RemoteOperation};
use archipel_ide_platform::{Disposable, MessageBus, Topic};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------------------ fetch/pull/push

/// How a fetch, pull or push ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteOutcome {
    /// "Fetching", "Pulling" or "Pushing" (the operation's progress label).
    pub label: &'static str,
    /// The operation rewrote the working tree: editors must reload.
    pub pull: bool,
    /// The one-line summary, or the error text.
    pub result: Result<String, String>,
}

/// Reports the end of a fetch, pull or push.
pub trait RemoteOperationListener: Send + Sync {
    fn remote_operation_finished(&self, _outcome: &RemoteOutcome) {}
}

/// On the project's message bus.
pub const REMOTE_TOPIC: Topic<dyn RemoteOperationListener> = Topic::new("RemoteOperationListener");

#[derive(Default)]
struct RemoteState {
    job: Option<RemoteJob>,
    /// Latest progress line of the running operation.
    progress: Option<String>,
    /// Message of the last failed operation.
    error: Option<String>,
}

/// The project service. A job that is still running when the project closes is abandoned.
pub struct GitRemoteOperations {
    bus: MessageBus,
    state: Mutex<RemoteState>,
}

impl GitRemoteOperations {
    pub fn new(bus: MessageBus) -> Self {
        Self {
            bus,
            state: Mutex::new(RemoteState::default()),
        }
    }

    pub fn is_busy(&self) -> bool {
        self.state.lock().job.is_some()
    }

    /// Latest progress line of the running operation.
    pub fn progress(&self) -> Option<String> {
        self.state.lock().progress.clone()
    }

    /// Message of the last failed operation.
    pub fn error(&self) -> Option<String> {
        self.state.lock().error.clone()
    }

    pub fn set_error(&self, error: Option<String>) {
        self.state.lock().error = error;
    }

    /// Starts `operation` in the repository at `root`.
    pub fn start(
        &self,
        root: &Path,
        operation: RemoteOperation,
        notify: Notify,
    ) -> Result<(), String> {
        let mut state = self.state.lock();
        if state.job.is_some() {
            return Err("Another fetch, pull or push is still running".into());
        }
        let label = operation.label();
        let job = RemoteJob::start(root, operation, notify).map_err(|error| error.to_string())?;
        state.job = Some(job);
        state.progress = Some(format!("{label}…"));
        Ok(())
    }

    /// Applies what the worker reported. Returns the newest progress line, if any; when the
    /// operation ended, announces it on [`REMOTE_TOPIC`].
    pub fn poll(&self) -> Option<String> {
        let mut latest = None;
        let mut finished = None;
        {
            let mut state = self.state.lock();
            let job = state.job.as_ref()?;
            let label = job.operation.label();
            let pull = matches!(job.operation, RemoteOperation::Pull { .. });
            let mut events = Vec::new();
            for event in job.poll() {
                let done = matches!(event, RemoteEvent::Finished(_));
                events.push(event);
                if done {
                    break;
                }
            }
            for event in events {
                match event {
                    RemoteEvent::Progress(line) => {
                        let progress = format!("{label}: {line}");
                        state.progress = Some(progress.clone());
                        latest = Some(progress);
                    }
                    RemoteEvent::Finished(result) => {
                        state.job = None;
                        state.progress = None;
                        finished = Some(RemoteOutcome {
                            label,
                            pull,
                            result,
                        });
                    }
                }
            }
        }
        if let Some(outcome) = &finished {
            self.bus
                .publish(REMOTE_TOPIC, |l| l.remote_operation_finished(outcome));
        }
        latest
    }
}

impl Disposable for GitRemoteOperations {
    fn dispose(&self) {
        *self.state.lock() = RemoteState::default();
    }
}

// ------------------------------------------------------------------------------ clone

/// A running `git clone` started from the welcome screen.
pub struct CloneJob {
    pub(crate) job: RemoteJob,
    pub url: String,
    pub directory: PathBuf,
}

/// How a clone ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneOutcome {
    pub url: String,
    pub directory: PathBuf,
    pub result: Result<String, String>,
}

/// Reports the end of a clone.
pub trait CloneListener: Send + Sync {
    fn clone_finished(&self, _outcome: &CloneOutcome) {}
}

/// On the application's message bus.
pub const CLONE_TOPIC: Topic<dyn CloneListener> = Topic::new("CloneListener");

#[derive(Default)]
struct CloneState {
    job: Option<CloneJob>,
    progress: Option<String>,
    error: Option<String>,
}

/// The application service: Get from Version Control.
pub struct CloneManager {
    bus: MessageBus,
    state: Mutex<CloneState>,
}

impl CloneManager {
    pub fn new(bus: MessageBus) -> Self {
        Self {
            bus,
            state: Mutex::new(CloneState::default()),
        }
    }

    pub fn is_busy(&self) -> bool {
        self.state.lock().job.is_some()
    }

    pub fn progress(&self) -> Option<String> {
        self.state.lock().progress.clone()
    }

    /// The message of the last failed clone, left for the dialog to show.
    pub fn error(&self) -> Option<String> {
        self.state.lock().error.clone()
    }

    /// Takes the message of the last failed clone.
    pub fn take_error(&self) -> Option<String> {
        self.state.lock().error.take()
    }

    pub fn set_error(&self, error: Option<String>) {
        self.state.lock().error = error;
    }

    /// Starts cloning `url` into `directory` (whose parent must exist).
    pub fn start(&self, url: &str, directory: &Path, notify: Notify) -> Result<(), String> {
        let mut state = self.state.lock();
        if state.job.is_some() {
            return Err("Another clone is still running".into());
        }
        let url = url.trim();
        if url.is_empty() {
            return Err("Enter the repository URL".into());
        }
        if directory.exists() {
            return Err(format!("{} already exists", directory.display()));
        }
        let parent = directory
            .parent()
            .filter(|parent| parent.is_dir())
            .ok_or_else(|| format!("The parent of {} does not exist", directory.display()))?;
        let operation = RemoteOperation::Clone {
            url: url.to_owned(),
            directory: directory.display().to_string(),
        };
        let job = RemoteJob::start(parent, operation, notify).map_err(|error| error.to_string())?;
        state.progress = Some("Cloning…".into());
        state.error = None;
        state.job = Some(CloneJob {
            job,
            url: url.to_owned(),
            directory: directory.to_owned(),
        });
        Ok(())
    }

    /// Applies what the worker reported. Returns the newest progress line and, when the clone
    /// ended, how (also announced on [`CLONE_TOPIC`]). A failed clone is recorded as the
    /// error; a successful one is for the caller to open.
    pub fn poll(&self) -> (Option<String>, Option<CloneOutcome>) {
        let mut latest = None;
        let mut finished = None;
        {
            let mut state = self.state.lock();
            let Some(clone) = state.job.as_ref() else {
                return (None, None);
            };
            let mut result = None;
            for event in clone.job.poll() {
                match event {
                    RemoteEvent::Progress(line) => latest = Some(line),
                    RemoteEvent::Finished(done) => result = Some(done),
                }
            }
            if let Some(line) = &latest {
                state.progress = Some(line.clone());
            }
            if let Some(result) = result {
                let clone = state.job.take().expect("checked above");
                state.progress = None;
                if let Err(error) = &result {
                    state.error = Some(error.clone());
                }
                finished = Some(CloneOutcome {
                    url: clone.url,
                    directory: clone.directory,
                    result,
                });
            }
        }
        if let Some(outcome) = &finished {
            self.bus.publish(CLONE_TOPIC, |l| l.clone_finished(outcome));
        }
        (latest, finished)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_platform::{Application, ComponentManager, ServiceDescriptor};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    #[derive(Default)]
    struct Heard(Mutex<Vec<CloneOutcome>>);

    impl CloneListener for Heard {
        fn clone_finished(&self, outcome: &CloneOutcome) {
            self.0.lock().push(outcome.clone());
        }
    }

    #[test]
    fn a_failed_clone_is_announced_and_kept_as_the_error() {
        let app = Application::new();
        app.register_service(ServiceDescriptor::new(|app: &Application| {
            CloneManager::new(app.message_bus().clone())
        }))
        .unwrap();
        let heard = Arc::new(Heard::default());
        app.message_bus()
            .subscribe(app.disposable(), CLONE_TOPIC, heard.clone())
            .unwrap();
        let base =
            std::env::temp_dir().join(format!("archipel-clone-service-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let clone = app.service::<CloneManager>();
        // Validation errors never start a job.
        assert!(clone.start(" ", &base.join("x"), Arc::new(|| {})).is_err());
        assert!(
            clone
                .start("u", &base, Arc::new(|| {}))
                .unwrap_err()
                .contains("already exists")
        );
        assert!(!clone.is_busy());

        clone
            .start(
                base.join("missing-origin").to_str().unwrap(),
                &base.join("copy"),
                Arc::new(|| {}),
            )
            .unwrap();
        assert!(clone.is_busy());
        assert_eq!(clone.progress().as_deref(), Some("Cloning…"));
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut outcome = None;
        while outcome.is_none() && Instant::now() < deadline {
            outcome = clone.poll().1;
            std::thread::sleep(Duration::from_millis(20));
        }
        let outcome = outcome.expect("the clone ended");
        assert!(outcome.result.is_err());
        assert!(!clone.is_busy() && clone.progress().is_none());
        assert_eq!(clone.error(), outcome.result.clone().err());
        assert_eq!(*heard.0.lock(), [outcome]);
        let _ = std::fs::remove_dir_all(&base);
    }
}

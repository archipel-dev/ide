//! Background installs with progress and cancellation. Poll from the UI loop.

use crate::download::HttpClient;
use crate::store::InstallPhase;
use crate::{Installed, ToolVersion, Toolchains, VersionRequest};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

pub type JobId = u64;

#[derive(Debug, Clone)]
pub enum ToolchainEvent {
    /// The release to install has been chosen.
    Resolved {
        job: JobId,
        component: String,
        version: ToolVersion,
    },
    Progress {
        job: JobId,
        component: String,
        phase: InstallPhase,
    },
    Finished {
        job: JobId,
        component: String,
        result: Result<Installed, String>,
    },
}

/// A running install.
#[derive(Debug, Clone)]
pub struct Job {
    pub id: JobId,
    pub component: String,
    pub version: Option<ToolVersion>,
    pub phase: InstallPhase,
    cancel: Arc<AtomicBool>,
}

impl Job {
    /// `Downloading JDK 21.0.4… 45%` style status text.
    pub fn describe(&self, name: &str) -> String {
        let version = self
            .version
            .as_ref()
            .map(|v| format!(" {v}"))
            .unwrap_or_default();
        match self.phase {
            InstallPhase::Resolving => format!("Resolving {name}{version}…"),
            InstallPhase::Downloading(progress) => match progress.total {
                Some(total) if total > 0 => format!(
                    "Downloading {name}{version}… {}%",
                    progress.received.saturating_mul(100) / total
                ),
                _ => format!(
                    "Downloading {name}{version}… {:.1} MB",
                    progress.received as f64 / 1_048_576.0
                ),
            },
            InstallPhase::Verifying => format!("Verifying {name}{version}…"),
            InstallPhase::Extracting => format!("Installing {name}{version}…"),
        }
    }

    /// Download fraction in `0.0..=1.0`, when known.
    pub fn fraction(&self) -> Option<f32> {
        match self.phase {
            InstallPhase::Downloading(progress) => progress
                .total
                .filter(|total| *total > 0)
                .map(|total| (progress.received as f64 / total as f64).min(1.0) as f32),
            InstallPhase::Verifying | InstallPhase::Extracting => Some(1.0),
            InstallPhase::Resolving => None,
        }
    }
}

pub struct ToolchainService {
    sender: Sender<ToolchainEvent>,
    /// Behind a mutex only so the service is `Sync` (it is a platform service); the
    /// receiving methods take `&mut self`, so the lock is never contended.
    receiver: Mutex<Receiver<ToolchainEvent>>,
    jobs: Vec<Job>,
    next_id: JobId,
    http: HttpClient,
}

impl Default for ToolchainService {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver: Mutex::new(receiver),
            jobs: Vec::new(),
            next_id: 1,
            http: HttpClient::new(),
        }
    }
}

impl ToolchainService {
    fn receiver_mut(&mut self) -> &mut Receiver<ToolchainEvent> {
        self.receiver
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }

    pub fn is_busy(&self) -> bool {
        !self.jobs.is_empty()
    }

    /// Starts installing `component` (resolving `request`, else the project
    /// pin) on a background thread. Returns the running job when the component
    /// is already being installed. Call only in response to a user action.
    pub fn install(
        &mut self,
        toolchains: &Toolchains,
        component: &str,
        request: Option<VersionRequest>,
    ) -> JobId {
        if let Some(job) = self.jobs.iter().find(|job| job.component == component) {
            return job.id;
        }
        let id = self.next_id;
        self.next_id += 1;
        let cancel = Arc::new(AtomicBool::new(false));
        self.jobs.push(Job {
            id,
            component: component.to_owned(),
            version: None,
            phase: InstallPhase::Resolving,
            cancel: cancel.clone(),
        });
        let sender = self.sender.clone();
        let toolchains = toolchains.clone();
        let http = self.http.clone();
        let name = component.to_owned();
        let component = component.to_owned();
        let spawned = std::thread::Builder::new()
            .name(format!("toolchain-install-{component}"))
            .spawn(move || {
                let result = (|| {
                    let release = toolchains.resolve(&component, request.as_ref(), &http)?;
                    let _ = sender.send(ToolchainEvent::Resolved {
                        job: id,
                        component: component.clone(),
                        version: release.version.clone(),
                    });
                    // Throttle progress events to keep the UI channel small.
                    let mut last = Instant::now() - Duration::from_secs(1);
                    toolchains.store().install(
                        &release,
                        &http,
                        &mut |phase| {
                            let downloading = matches!(phase, InstallPhase::Downloading(_));
                            if !downloading || last.elapsed() >= Duration::from_millis(100) {
                                last = Instant::now();
                                let _ = sender.send(ToolchainEvent::Progress {
                                    job: id,
                                    component: component.clone(),
                                    phase,
                                });
                            }
                        },
                        &cancel,
                    )
                })();
                let _ = sender.send(ToolchainEvent::Finished {
                    job: id,
                    component,
                    result: result.map_err(|error| error.to_string()),
                });
            });
        if let Err(error) = spawned {
            let _ = self.sender.send(ToolchainEvent::Finished {
                job: id,
                component: name,
                result: Err(format!("cannot start install thread: {error}")),
            });
        }
        id
    }

    /// Requests cancellation; the partial download is kept for resuming.
    pub fn cancel(&self, job: JobId) -> bool {
        self.jobs
            .iter()
            .find(|candidate| candidate.id == job)
            .map(|job| job.cancel.store(true, Ordering::Relaxed))
            .is_some()
    }

    pub fn cancel_all(&self) {
        for job in &self.jobs {
            job.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Drains pending events, updating [`ToolchainService::jobs`].
    pub fn poll(&mut self) -> Vec<ToolchainEvent> {
        let events: Vec<ToolchainEvent> = self.receiver_mut().try_iter().collect();
        for event in &events {
            self.apply(event);
        }
        events
    }

    fn apply(&mut self, event: &ToolchainEvent) {
        match event {
            ToolchainEvent::Resolved { job, version, .. } => {
                if let Some(job) = self.jobs.iter_mut().find(|j| j.id == *job) {
                    job.version = Some(version.clone());
                }
            }
            ToolchainEvent::Progress { job, phase, .. } => {
                if let Some(job) = self.jobs.iter_mut().find(|j| j.id == *job) {
                    job.phase = *phase;
                }
            }
            ToolchainEvent::Finished { job, .. } => self.jobs.retain(|j| j.id != *job),
        }
    }

    /// Blocks until `job` finishes or `timeout` passes (headless CLI use).
    /// Events for other jobs are applied and returned in order.
    pub fn wait(
        &mut self,
        job: JobId,
        timeout: Duration,
    ) -> (Vec<ToolchainEvent>, Option<Result<Installed, String>>) {
        let deadline = Instant::now() + timeout;
        let mut seen = Vec::new();
        while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            let Ok(event) = self.receiver_mut().recv_timeout(remaining) else {
                break;
            };
            self.apply(&event);
            if let ToolchainEvent::Finished {
                job: done, result, ..
            } = &event
                && *done == job
            {
                let result = result.clone();
                seen.push(event);
                return (seen, Some(result));
            }
            seen.push(event);
        }
        (seen, None)
    }
}

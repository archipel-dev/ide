//! Update checking and installation as an application service (IntelliJ: `UpdateChecker`).
//!
//! [`UpdateChecker`] owns the background worker, the [`UpdateState`] machine, and what the
//! installation left behind (the original executable path, a downloaded installer to run on
//! quit). Every transition that the user should hear about is announced on [`UPDATE_TOPIC`]
//! with the message for the status bar; the notification balloon and the status line follow
//! from that event.

use crate::update_commands::{
    CURRENT_VERSION, UpdateState, current_version, installable_executable,
};
use archipel_ide_platform::{MessageBus, Topic};
use archipel_ide_update::{
    CheckOutcome, InstallKind, Installed, InstalledUpdate, UpdateEvent, UpdateService,
    UpdateSource, Version,
};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};

/// Reports update progress (IntelliJ: `UpdateChecker` notifications).
pub trait UpdateListener: Send + Sync {
    /// The state changed. `message` is what the user should be told (status bar and balloon);
    /// it is `None` for silent background checks that found nothing.
    fn update_state_changed(&self, _state: &UpdateState, _message: Option<&str>) {}
}

/// On the application's message bus.
pub const UPDATE_TOPIC: Topic<dyn UpdateListener> = Topic::new("UpdateListener");

#[derive(Default)]
struct Inner {
    worker: UpdateService,
    state: UpdateState,
    /// The running check was started by the IDE itself, not by the user.
    silent: bool,
    /// Original install path, retained when the running binary is renamed.
    executable: Option<PathBuf>,
    /// Verified installer to run on quit (MSI/DMG/.deb installations).
    installer: Option<(PathBuf, InstallKind)>,
}

/// The application service: `application.service::<UpdateChecker>()`.
pub struct UpdateChecker {
    bus: MessageBus,
    inner: Mutex<Inner>,
}

impl UpdateChecker {
    pub fn new(bus: MessageBus) -> Self {
        Self {
            bus,
            inner: Mutex::new(Inner::default()),
        }
    }

    pub fn is_busy(&self) -> bool {
        self.inner.lock().worker.is_busy()
    }

    pub fn state(&self) -> UpdateState {
        self.inner.lock().state.clone()
    }

    pub fn is_available(&self) -> bool {
        matches!(self.inner.lock().state, UpdateState::Available { .. })
    }

    /// Starts a check; ignored while another operation runs.
    pub fn check(&self, silent: bool) {
        let mut inner = self.inner.lock();
        if inner.worker.is_busy() {
            return;
        }
        inner.silent = silent;
        inner.state = UpdateState::Checking;
        inner
            .worker
            .check(UpdateSource::from_env(), current_version());
    }

    /// Downloads and installs the available update. Returns the version being installed.
    pub fn install(&self) -> Result<Version, String> {
        let mut inner = self.inner.lock();
        let UpdateState::Available { version, release } = &inner.state else {
            return Err("no update is available".into());
        };
        let exe = inner
            .executable
            .clone()
            .map_or_else(installable_executable, Ok)?;
        let version = version.clone();
        let release = (**release).clone();
        inner.executable = Some(exe.clone());
        inner.worker.install(UpdateSource::from_env(), release, exe);
        inner.state = UpdateState::Installing(version.clone());
        Ok(version)
    }

    /// Applies the worker's result, if it has one, and announces it on [`UPDATE_TOPIC`].
    /// Returns whether the state changed.
    pub fn poll(&self) -> bool {
        let (state, message) = {
            let mut inner = self.inner.lock();
            let Some(event) = inner.worker.poll() else {
                return false;
            };
            let silent = inner.silent;
            let (state, message) = match event {
                UpdateEvent::Checked(Ok(CheckOutcome::Available { version, release })) => (
                    UpdateState::Available {
                        release: Box::new(release),
                        version: version.clone(),
                    },
                    Some(format!("Archipel IDE {version} is available")),
                ),
                UpdateEvent::Checked(Ok(CheckOutcome::UpToDate { latest })) => (
                    UpdateState::UpToDate(latest),
                    (!silent).then(|| format!("Archipel IDE {CURRENT_VERSION} is up to date")),
                ),
                UpdateEvent::Checked(Err(error)) => {
                    let message = (!silent).then(|| format!("Update check failed: {error}"));
                    (UpdateState::Failed(error), message)
                }
                UpdateEvent::Installed(Ok(InstalledUpdate { version, installed })) => {
                    inner.installer = match installed {
                        Installed::InPlace => None,
                        Installed::Installer { path, kind } => Some((path, kind)),
                    };
                    let message = if inner.installer.is_some() {
                        format!("Archipel IDE {version} downloaded; quit to install")
                    } else {
                        format!("Archipel IDE {version} installed; restart to finish")
                    };
                    (UpdateState::Installed(version), Some(message))
                }
                UpdateEvent::Installed(Err(error)) => (
                    UpdateState::Failed(error.clone()),
                    Some(format!("Update failed: {error}")),
                ),
            };
            inner.state = state.clone();
            (state, message)
        };
        self.bus.publish(UPDATE_TOPIC, |l| {
            l.update_state_changed(&state, message.as_deref())
        });
        true
    }

    /// On Linux, current_exe follows the running inode into the `.old` backup. The installation
    /// target captured before replacement is what a restart must start.
    pub fn restart_target(&self) -> Option<PathBuf> {
        self.inner.lock().executable.clone()
    }

    /// The downloaded installer to run on quit, with its kind.
    pub fn installer(&self) -> Option<(PathBuf, InstallKind)> {
        self.inner.lock().installer.clone()
    }

    pub fn installer_kind(&self) -> Option<InstallKind> {
        self.inner.lock().installer.as_ref().map(|(_, kind)| *kind)
    }

    /// Replaces the background worker (tests, embedders).
    pub fn set_worker(&self, worker: UpdateService) {
        self.inner.lock().worker = worker;
    }

    /// Sets the state directly (tests, embedders).
    pub fn set_state(&self, state: UpdateState) {
        self.inner.lock().state = state;
    }

    /// Sets the downloaded installer directly (tests, embedders).
    pub fn set_installer(&self, installer: Option<(PathBuf, InstallKind)>) {
        self.inner.lock().installer = installer;
    }

    /// Sets the install target directly (tests, embedders).
    pub fn set_executable(&self, executable: Option<&Path>) {
        self.inner.lock().executable = executable.map(Path::to_owned);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_platform::{Application, ComponentManager, ServiceDescriptor};
    use std::sync::Arc;

    #[derive(Default)]
    struct Heard(Mutex<Vec<(String, Option<String>)>>);

    impl UpdateListener for Heard {
        fn update_state_changed(&self, state: &UpdateState, message: Option<&str>) {
            self.0
                .lock()
                .push((format!("{state:?}"), message.map(str::to_owned)));
        }
    }

    fn checker() -> (Application, Arc<UpdateChecker>, Arc<Heard>) {
        let app = Application::new();
        app.register_service(ServiceDescriptor::new(|app: &Application| {
            UpdateChecker::new(app.message_bus().clone())
        }))
        .unwrap();
        let heard = Arc::new(Heard::default());
        app.message_bus()
            .subscribe(app.disposable(), UPDATE_TOPIC, heard.clone())
            .unwrap();
        let checker = app.service::<UpdateChecker>();
        (app, checker, heard)
    }

    fn feed(checker: &UpdateChecker, event: UpdateEvent) {
        let (sender, receiver) = std::sync::mpsc::channel();
        sender.send(event).unwrap();
        checker.set_worker(UpdateService::with_receiver(receiver));
    }

    #[test]
    fn a_finished_check_is_announced_with_its_message() {
        let (_app, checker, heard) = checker();
        let latest = Version::parse("9.9.9").unwrap();
        feed(
            &checker,
            UpdateEvent::Checked(Ok(CheckOutcome::UpToDate { latest })),
        );
        assert!(checker.poll());
        assert!(matches!(checker.state(), UpdateState::UpToDate(_)));
        let heard = heard.0.lock();
        assert_eq!(heard.len(), 1);
        assert_eq!(
            heard[0].1.as_deref(),
            Some(format!("Archipel IDE {CURRENT_VERSION} is up to date").as_str())
        );
    }

    #[test]
    fn silent_checks_that_find_nothing_announce_no_message() {
        let (_app, checker, heard) = checker();
        checker.inner.lock().silent = true;
        feed(&checker, UpdateEvent::Checked(Err("offline".into())));
        assert!(checker.poll());
        assert_eq!(checker.state(), UpdateState::Failed("offline".into()));
        assert_eq!(heard.0.lock()[0].1, None);
        assert!(!checker.poll(), "nothing pending");
    }

    #[test]
    fn installing_needs_an_available_update() {
        let (_app, checker, _) = checker();
        assert!(checker.install().is_err());
    }
}

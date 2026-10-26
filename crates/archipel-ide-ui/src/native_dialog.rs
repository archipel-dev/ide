//! Native OS file dialogs behind a small trait.
//!
//! The dialog runs on a background thread and its result is polled each frame, so the UI
//! never blocks. When the platform has no dialog (no display or no desktop portal, e.g.
//! headless Xvfb CI) the caller falls back to the in-app path field.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

/// What the user is asked to choose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogRequest {
    pub title: String,
    pub start_dir: Option<PathBuf>,
    pub folder: bool,
}

/// How a dialog ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogOutcome {
    Picked(PathBuf),
    Cancelled,
    /// No native dialog can be shown here; use the in-app picker.
    Unavailable,
}

/// Shows a dialog and blocks until it closes. Called on a background thread.
pub trait FileDialogs: Send + Sync {
    fn pick(&self, request: &DialogRequest) -> DialogOutcome;
}

/// The operating system's own dialog (xdg-desktop-portal, NSOpenPanel, IFileDialog).
pub struct SystemDialogs;

impl SystemDialogs {
    #[cfg(target_os = "linux")]
    fn available() -> bool {
        let display = ["DISPLAY", "WAYLAND_DISPLAY"]
            .iter()
            .any(|name| std::env::var_os(name).is_some_and(|v| !v.is_empty()));
        let bus = std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
            || std::env::var_os("XDG_RUNTIME_DIR")
                .is_some_and(|dir| std::path::Path::new(&dir).join("bus").exists());
        display && bus
    }

    #[cfg(not(target_os = "linux"))]
    fn available() -> bool {
        true
    }
}

impl FileDialogs for SystemDialogs {
    fn pick(&self, request: &DialogRequest) -> DialogOutcome {
        if !Self::available() {
            return DialogOutcome::Unavailable;
        }
        let mut dialog = rfd::AsyncFileDialog::new().set_title(&request.title);
        if let Some(dir) = request.start_dir.as_ref().filter(|dir| dir.is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        let picked = pollster::block_on(async {
            if request.folder {
                dialog.pick_folder().await
            } else {
                dialog.pick_file().await
            }
        });
        match picked {
            Some(handle) => DialogOutcome::Picked(handle.path().to_owned()),
            None => DialogOutcome::Cancelled,
        }
    }
}

/// Answers with a path queued through `dialog.injectPath` (control socket, scripted runs).
pub struct InjectedDialog(pub PathBuf);

impl FileDialogs for InjectedDialog {
    fn pick(&self, _: &DialogRequest) -> DialogOutcome {
        DialogOutcome::Picked(self.0.clone())
    }
}

/// What a finished dialog was for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    OpenProject,
    NewProjectLocation,
    CloneLocation,
}

/// A dialog running on a background thread.
pub struct PendingDialog {
    pub purpose: Purpose,
    receiver: Receiver<DialogOutcome>,
}

impl PendingDialog {
    pub fn spawn(
        dialogs: std::sync::Arc<dyn FileDialogs>,
        purpose: Purpose,
        request: DialogRequest,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(dialogs.pick(&request));
        });
        Self { purpose, receiver }
    }

    /// The outcome once the dialog closed; `Err` when still open.
    pub fn poll(&self) -> Result<DialogOutcome, ()> {
        match self.receiver.try_recv() {
            Ok(outcome) => Ok(outcome),
            Err(mpsc::TryRecvError::Empty) => Err(()),
            // The thread died without answering: treat as unavailable.
            Err(mpsc::TryRecvError::Disconnected) => Ok(DialogOutcome::Unavailable),
        }
    }
}

/// Test double: answers with a fixed outcome and records the requests.
#[cfg(test)]
pub struct FakeDialogs {
    pub outcome: DialogOutcome,
    pub requests: std::sync::Mutex<Vec<DialogRequest>>,
}

#[cfg(test)]
impl FakeDialogs {
    pub fn new(outcome: DialogOutcome) -> Self {
        Self {
            outcome,
            requests: Default::default(),
        }
    }
}

#[cfg(test)]
impl FileDialogs for FakeDialogs {
    fn pick(&self, request: &DialogRequest) -> DialogOutcome {
        self.requests.lock().unwrap().push(request.clone());
        self.outcome.clone()
    }
}

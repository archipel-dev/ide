//! "Check for Updates" and in-place installation.

use crate::{IdeState, spec};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_update::{
    CheckOutcome, InstallKind, Installed, InstalledUpdate, Release, UpdateEvent, UpdateSource,
    Version,
};
use serde_json::{Value, json};

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum UpdateState {
    #[default]
    Idle,
    Checking,
    UpToDate(Version),
    Available {
        version: Version,
        release: Box<Release>,
    },
    Installing(Version),
    /// Installed; takes effect after restarting the IDE.
    Installed(Version),
    Failed(String),
}

pub fn current_version() -> Version {
    Version::parse(CURRENT_VERSION).unwrap_or(Version {
        major: 0,
        minor: 0,
        patch: 0,
        pre: None,
    })
}

/// Only release layouts (`archipel-ide` executable from a distribution archive) can
/// update themselves; development builds (`target/*/archipel-ide`) cannot.
pub fn installable_executable() -> Result<std::path::PathBuf, String> {
    // Inside an AppImage the binary runs from a temporary mount; the runtime
    // names the real file in `$APPIMAGE`, which is what updates replace.
    if let Some(appimage) = std::env::var_os("APPIMAGE").map(std::path::PathBuf::from)
        && appimage.is_file()
    {
        return Ok(appimage);
    }
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    if exe.file_stem().and_then(|s| s.to_str()) == Some("archipel-ide") {
        Ok(exe)
    } else {
        Err("This is a development build; download the update from the release page.".into())
    }
}

impl IdeState {
    pub fn check_for_updates(&mut self, silent: bool) {
        if self.updates.is_busy() {
            return;
        }
        self.update_silent = silent;
        self.update_state = UpdateState::Checking;
        self.updates
            .check(UpdateSource::from_env(), current_version());
    }

    pub fn install_update(&mut self) -> Result<(), String> {
        let UpdateState::Available { version, release } = &self.update_state else {
            return Err("no update is available".into());
        };
        let exe = self
            .update_executable
            .clone()
            .map_or_else(installable_executable, Ok)?;
        self.update_executable = Some(exe.clone());
        let version = version.clone();
        self.updates
            .install(UpdateSource::from_env(), (**release).clone(), exe);
        self.status = format!("Downloading Archipel IDE {version}…");
        self.update_state = UpdateState::Installing(version);
        Ok(())
    }

    pub fn poll_updates(&mut self) {
        let Some(event) = self.updates.poll() else {
            return;
        };
        self.update_state = match event {
            UpdateEvent::Checked(Ok(CheckOutcome::Available { version, release })) => {
                self.status = format!("Archipel IDE {version} is available");
                self.update_notification = true;
                UpdateState::Available {
                    version,
                    release: Box::new(release),
                }
            }
            UpdateEvent::Checked(Ok(CheckOutcome::UpToDate { latest })) => {
                if !self.update_silent {
                    self.status = format!("Archipel IDE {CURRENT_VERSION} is up to date");
                    self.update_notification = true;
                }
                UpdateState::UpToDate(latest)
            }
            UpdateEvent::Checked(Err(error)) => {
                if !self.update_silent {
                    self.status = format!("Update check failed: {error}");
                    self.update_notification = true;
                }
                UpdateState::Failed(error)
            }
            UpdateEvent::Installed(Ok(InstalledUpdate { version, installed })) => {
                self.update_installer = match installed {
                    Installed::InPlace => None,
                    Installed::Installer { path, kind } => Some((path, kind)),
                };
                self.status = if self.update_installer.is_some() {
                    format!("Archipel IDE {version} downloaded; quit to install")
                } else {
                    format!("Archipel IDE {version} installed; restart to finish")
                };
                self.update_notification = true;
                UpdateState::Installed(version)
            }
            UpdateEvent::Installed(Err(error)) => {
                self.status = format!("Update failed: {error}");
                self.update_notification = true;
                UpdateState::Failed(error)
            }
        };
    }

    /// On Linux, current_exe follows the running inode into the `.old` backup.
    /// Use the installation target captured before replacement instead.
    pub fn update_restart_target(&self) -> Option<&std::path::Path> {
        self.update_executable.as_deref()
    }

    /// How the downloaded update is applied: `None` for an in-place update
    /// (restart), otherwise the installer kind to run on quit.
    pub fn update_installer_kind(&self) -> Option<InstallKind> {
        self.update_installer.as_ref().map(|(_, kind)| *kind)
    }

    /// Saves the session and opens the downloaded installer. The caller quits
    /// the IDE afterwards so the installer can replace its files.
    pub fn run_update_installer(&mut self) -> Result<(), String> {
        let Some((path, _)) = self.update_installer.clone() else {
            return Err("no downloaded installer; check for updates first".into());
        };
        if self.project.is_some() {
            self.save_all()?;
            self.save_workspace()?;
        }
        archipel_ide_update::launch_installer(&path).map_err(|error| error.to_string())
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "app.prepareRestart",
                "Save Session for Restart",
                "Help",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                if state.project.is_some() {
                    state.save_all().map_err(CommandError::Failed)?;
                    state.save_workspace().map_err(CommandError::Failed)?;
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "app.checkForUpdates",
                "Check for Updates…",
                "Help",
                None,
                r#"{"silent":"bool?"}"#,
            ),
            |state: &IdeState| !state.updates.is_busy(),
            |state: &mut IdeState, params| {
                let silent = params
                    .get("silent")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                state.check_for_updates(silent);
                Ok(json!({"current": CURRENT_VERSION}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "app.installUpdate",
                "Update and Restart",
                "Help",
                None,
                "{}",
            ),
            |state: &IdeState| matches!(state.update_state, UpdateState::Available { .. }),
            |state: &mut IdeState, _| {
                state.install_update().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "app.runUpdateInstaller",
                "Quit and Install Update",
                "Help",
                None,
                "{}",
            ),
            |state: &IdeState| state.update_installer.is_some(),
            |state: &mut IdeState, _| {
                state.run_update_installer().map_err(CommandError::Failed)?;
                Ok(json!({"quit": true}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "app.dismissUpdate",
                "Dismiss Update Notification",
                "Help",
                None,
                "{}",
            ),
            |state: &IdeState| state.update_notification,
            |state: &mut IdeState, _| {
                state.update_notification = false;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("app.about", "About Archipel IDE", "Help", None, "{}"),
            |_| true,
            |state: &mut IdeState, _| {
                let build = crate::build_info::BUILD;
                state.status = build.version_line("Archipel IDE");
                state.about_open = true;
                Ok(build.to_json())
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("app.closeAbout", "Close About", "Help", None, "{}"),
            |state: &IdeState| state.about_open,
            |state: &mut IdeState, _| {
                state.about_open = false;
                // The version line was only there for the dialog; later messages stay.
                if state.status == crate::build_info::BUILD.version_line("Archipel IDE") {
                    state.status.clear();
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_update::parse_release;

    #[test]
    fn restart_preparation_saves_dirty_files_and_propagates_save_errors() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-restart-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("notes.txt");
        std::fs::write(&path, "old").unwrap();
        let commands = crate::built_in_commands();
        let mut state = IdeState::default();
        state.open_project(&root).unwrap();
        state.open_file(std::path::Path::new("notes.txt")).unwrap();
        state.documents[0].text.edit(0, 3, "saved").unwrap();
        commands
            .execute("app.prepareRestart", &mut state, Value::Null)
            .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "saved");
        assert!(root.join(".archipel/workspace.json").is_file());
        assert!(!state.documents[0].text.is_dirty());
        state.documents[0].text.edit(0, 5, "keep this").unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(
            commands
                .execute("app.prepareRestart", &mut state, Value::Null)
                .is_err()
        );
        assert!(state.documents[0].text.is_dirty());
        assert_eq!(state.documents[0].text.text(), "keep this");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn welcome_session_can_prepare_for_restart() {
        crate::built_in_commands()
            .execute("app.prepareRestart", &mut IdeState::default(), Value::Null)
            .unwrap();
    }

    #[test]
    fn about_opens_the_dialog_and_reports_the_build() {
        let mut state = IdeState::default();
        let info = crate::built_in_commands()
            .execute("app.about", &mut state, Value::Null)
            .unwrap();
        assert!(state.about_open);
        assert_eq!(info["version"], CURRENT_VERSION);
        assert_eq!(info["commit"], crate::build_info::BUILD.commit);
        assert!(
            state
                .status
                .starts_with(&format!("Archipel IDE {CURRENT_VERSION} ("))
        );
    }

    #[test]
    fn closing_about_clears_its_status_line() {
        let commands = crate::built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("app.about", &mut state, Value::Null)
            .unwrap();
        assert!(!state.status.is_empty());
        commands
            .execute("app.closeAbout", &mut state, Value::Null)
            .unwrap();
        assert!(!state.about_open);
        assert_eq!(state.status, "");
        // A message set while the dialog was open is kept.
        commands
            .execute("app.about", &mut state, Value::Null)
            .unwrap();
        state.status = "Build details copied".into();
        commands
            .execute("app.closeAbout", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.status, "Build details copied");
        assert!(!commands.is_enabled("app.closeAbout", &state));
    }

    #[test]
    fn version_comes_from_the_workspace() {
        assert_eq!(current_version().to_string(), CURRENT_VERSION);
    }

    #[test]
    fn installer_downloads_switch_restart_to_quit_and_install() {
        let mut state = IdeState::default();
        let commands = crate::built_in_commands();
        assert!(
            commands
                .execute("app.runUpdateInstaller", &mut state, Value::Null)
                .is_err()
        );
        state.update_installer = Some((
            std::path::PathBuf::from("archipel-ide-9.0.0-windows-x86_64.msi"),
            InstallKind::Msi,
        ));
        assert_eq!(state.update_installer_kind(), Some(InstallKind::Msi));
        let in_place = InstalledUpdate {
            version: Version::parse("9.0.0").unwrap(),
            installed: Installed::InPlace,
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        sender.send(UpdateEvent::Installed(Ok(in_place))).unwrap();
        state.updates = archipel_ide_update::UpdateService::with_receiver(receiver);
        state.poll_updates();
        assert_eq!(state.update_installer_kind(), None);
        assert!(matches!(state.update_state, UpdateState::Installed(_)));
        assert!(state.status.contains("restart"));
    }

    #[test]
    fn install_requires_an_available_update_and_a_release_build() {
        let mut state = IdeState::default();
        assert!(state.install_update().is_err());
        let release = parse_release(
            r#"{"tag_name":"v99.0.0","html_url":"https://example.invalid","assets":[]}"#,
        )
        .unwrap();
        state.update_state = UpdateState::Available {
            version: release.version().unwrap(),
            release: Box::new(release),
        };
        // Test binaries are not named `archipel-ide`, so they never replace themselves.
        let error = state.install_update().unwrap_err();
        assert!(error.contains("development build"));
        assert!(matches!(state.update_state, UpdateState::Available { .. }));
    }
}

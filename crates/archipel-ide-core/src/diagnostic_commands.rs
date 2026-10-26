//! Help → Show Log in Files and the "closed unexpectedly" crash notice.

use crate::{IdeState, diagnostics, spec};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};
use std::path::PathBuf;

impl IdeState {
    /// Looks for crash reports the user has not dismissed yet (startup).
    pub fn check_crash_reports(&mut self, log_dir: &std::path::Path) {
        self.crash_report = diagnostics::unseen_crash_reports(log_dir)
            .into_iter()
            .next();
    }

    fn log_directory() -> PathBuf {
        diagnostics::active_log_dir().unwrap_or_else(diagnostics::log_dir)
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec("app.showLog", "Show Log in Files", "Help", None, "{}"),
            |_| true,
            |state: &mut IdeState, _| {
                let dir = IdeState::log_directory();
                std::fs::create_dir_all(&dir)
                    .and_then(|_| diagnostics::open_path(&dir))
                    .map_err(|error| {
                        CommandError::Failed(format!("cannot open {}: {error}", dir.display()))
                    })?;
                state.status = format!("Logs: {}", dir.display());
                Ok(json!({"dir": dir, "log": dir.join("archipel.log")}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "app.openCrashReport",
                "Open Last Crash Report",
                "Help",
                None,
                "{}",
            ),
            |state: &IdeState| state.crash_report.is_some(),
            |state: &mut IdeState, _| {
                let report = state
                    .crash_report
                    .clone()
                    .ok_or_else(|| CommandError::Failed("no crash report to open".into()))?;
                diagnostics::open_path(&report).map_err(|error| {
                    CommandError::Failed(format!("cannot open {}: {error}", report.display()))
                })?;
                Ok(json!({"report": report}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "app.dismissCrashReport",
                "Dismiss Crash Report Notice",
                "Help",
                None,
                "{}",
            ),
            |state: &IdeState| state.crash_report.is_some(),
            |state: &mut IdeState, _| {
                if let Some(report) = state.crash_report.take() {
                    let dir = report.parent().map(PathBuf::from).unwrap_or_default();
                    diagnostics::mark_crash_reports_seen(&dir, &report)
                        .map_err(|error| CommandError::Failed(error.to_string()))?;
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_notice_shows_newest_report_until_dismissed() {
        let dir =
            std::env::temp_dir().join(format!("archipel-ide-crash-cmd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("crash-20261010T100000-1.txt"), "report").unwrap();
        let commands = crate::built_in_commands();
        let mut state = IdeState::default();
        assert!(!commands.is_enabled("app.dismissCrashReport", &state));
        state.check_crash_reports(&dir);
        assert_eq!(
            state.crash_report.as_deref(),
            Some(dir.join("crash-20261010T100000-1.txt").as_path())
        );
        commands
            .execute("app.dismissCrashReport", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.crash_report, None);
        state.check_crash_reports(&dir);
        assert_eq!(state.crash_report, None, "dismissed reports stay dismissed");
        assert!(
            commands
                .execute("app.openCrashReport", &mut state, Value::Null)
                .is_err()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}

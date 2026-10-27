//! Remote SDK repository state and `android.sdk.*` install commands.

use crate::{IdeState, spec};
use archipel_ide_android::sdk_repo::{JobEvent, RemotePackage, SdkJob, parse_list};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JobKind {
    List,
    Apply,
}

/// Packages of the remote repository merged with the installed inventory.
#[derive(Default)]
pub struct SdkRepoState {
    pub packages: Vec<RemotePackage>,
    /// Whether the repository listing has been fetched at least once.
    pub loaded: bool,
    job: Option<(JobKind, SdkJob)>,
    queue: VecDeque<Vec<String>>,
    /// Last progress line of the running job.
    pub progress: Option<String>,
    /// Outcome of the last job: `Err` carries the failure message.
    pub result: Option<Result<String, String>>,
}

impl SdkRepoState {
    pub fn is_busy(&self) -> bool {
        self.job.is_some() || !self.queue.is_empty()
    }
}

impl IdeState {
    fn sdk_job(&self, args: &[String]) -> Result<SdkJob, String> {
        let sdk = self
            .android_tooling()
            .sdk()
            .and_then(|d| d.sdk)
            .ok_or("No Android SDK is configured.")?;
        let tool = sdk.tool("sdkmanager").ok_or(
            "sdkmanager not found. Install the Command-line Tools from \
             https://developer.android.com/studio#command-line-tools-only",
        )?;
        Ok(SdkJob::spawn(&tool, &sdk.root, args))
    }

    /// Fetches the repository listing (`sdkmanager --list`).
    pub fn refresh_remote_packages(&mut self) -> Result<(), String> {
        if self.android_tooling().read().sdk_repo.is_busy() {
            return Err("sdkmanager is already running.".into());
        }
        let job = self.sdk_job(&["--list".into()])?;
        let tooling = self.android_tooling();
        let mut guard = tooling.write();
        let repo = &mut guard.sdk_repo;
        repo.job = Some((JobKind::List, job));
        repo.progress = Some("Fetching the package list…".into());
        repo.result = None;
        Ok(())
    }

    /// Uninstalls then installs the given `sdkmanager` package paths.
    pub fn apply_sdk_changes(
        &mut self,
        install: Vec<String>,
        uninstall: Vec<String>,
    ) -> Result<(), String> {
        if self.android_tooling().read().sdk_repo.is_busy() {
            return Err("sdkmanager is already running.".into());
        }
        if install.is_empty() && uninstall.is_empty() {
            return Err("Nothing to install or uninstall.".into());
        }
        let valid = |p: &String| {
            !p.is_empty()
                && !p.starts_with('-')
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, ';' | '.' | '-' | '_'))
        };
        if let Some(bad) = install.iter().chain(&uninstall).find(|p| !valid(p)) {
            return Err(format!("{bad:?} is not an SDK package path"));
        }
        let mut queue = VecDeque::new();
        if !uninstall.is_empty() {
            let mut args = vec!["--uninstall".to_owned()];
            args.extend(uninstall);
            queue.push_back(args);
        }
        if !install.is_empty() {
            let mut args = vec!["--install".to_owned()];
            args.extend(install);
            queue.push_back(args);
        }
        let first = queue.pop_front().expect("one step queued");
        let job = self.sdk_job(&first)?;
        let tooling = self.android_tooling();
        let mut guard = tooling.write();
        let repo = &mut guard.sdk_repo;
        repo.queue = queue;
        repo.job = Some((JobKind::Apply, job));
        repo.progress = Some("Starting sdkmanager…".into());
        repo.result = None;
        Ok(())
    }

    pub(crate) fn poll_sdk_repo(&mut self) -> bool {
        // Drain under the read guard, then release it: finishing a job writes the state again.
        let (kind, events) = {
            let tooling = self.android_tooling();
            let guard = tooling.read();
            let Some((kind, job)) = &guard.sdk_repo.job else {
                return false;
            };
            (*kind, job.poll())
        };
        if events.is_empty() {
            return false;
        }
        for event in events {
            match event {
                JobEvent::Line(line) => {
                    self.android_tooling().write().sdk_repo.progress = Some(line)
                }
                JobEvent::Done(result) => self.finish_sdk_job(kind, result),
            }
        }
        true
    }

    fn finish_sdk_job(&mut self, kind: JobKind, result: Result<String, String>) {
        self.android_tooling().write().sdk_repo.job = None;
        match (kind, result) {
            (JobKind::List, Ok(text)) => {
                let tooling = self.android_tooling();
                let mut guard = tooling.write();
                let repo = &mut guard.sdk_repo;
                repo.packages = parse_list(&text);
                repo.loaded = true;
                repo.progress = None;
                repo.result = Some(Ok(format!("{} packages", repo.packages.len())));
            }
            (JobKind::Apply, Ok(_)) => {
                let next = self.android_tooling().write().sdk_repo.queue.pop_front();
                if let Some(args) = next {
                    match self.sdk_job(&args) {
                        Ok(job) => {
                            self.android_tooling().write().sdk_repo.job =
                                Some((JobKind::Apply, job));
                            return;
                        }
                        Err(error) => {
                            self.android_tooling().write().sdk_repo.result = Some(Err(error))
                        }
                    }
                } else {
                    self.android_tooling().write().sdk_repo.result =
                        Some(Ok("SDK packages updated.".into()));
                }
                self.android_tooling().write().sdk_repo.progress = None;
                let root = self.android_tooling().read().root.clone();
                self.android_tooling().detect_sdk(root.as_deref());
                let _ = self.refresh_remote_packages();
            }
            (_, Err(error)) => {
                let tooling = self.android_tooling();
                let mut guard = tooling.write();
                let repo = &mut guard.sdk_repo;
                repo.queue.clear();
                repo.progress = None;
                repo.result = Some(Err(if error.to_lowercase().contains("licen") {
                    format!("{error}\nAccept the SDK licenses with `sdkmanager --licenses` first.")
                } else {
                    error
                }));
            }
        }
    }
}

fn string_list(params: &Value, key: &str) -> Vec<String> {
    params
        .get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "android.sdk.refreshRemote",
                "Refresh SDK Package List",
                "Android",
                None,
                "{}",
            ),
            |state: &IdeState| state.android_tooling().read().sdk.is_some(),
            |state: &mut IdeState, _| {
                state
                    .refresh_remote_packages()
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.sdk.apply",
                "Install or Uninstall SDK Packages",
                "Android",
                None,
                r#"{"install":"string[]","uninstall":"string[]"}"#,
            ),
            |state: &IdeState| state.android_tooling().read().sdk.is_some(),
            |state: &mut IdeState, params| {
                state
                    .apply_sdk_changes(
                        string_list(&params, "install"),
                        string_list(&params, "uninstall"),
                    )
                    .map_err(CommandError::InvalidParams)?;
                Ok(json!({"started": true}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(all(test, unix))]
mod tests {
    use crate::{IdeState, built_in_commands};
    use archipel_ide_android::{SdkDetection, SystemEnvironment};
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};

    #[test]
    fn lists_then_installs_through_a_fake_sdkmanager() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-core-sdkrepo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let bin = root.join("cmdline-tools/latest/bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(root.join("platform-tools")).unwrap();
        let log = root.join("calls.log");
        let script = bin.join("sdkmanager");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\necho \"$@\" >> {}\nif [ \"$2\" = --list ]; then printf 'Installed packages:\\n  Path | Version | Description | Location\\n  platform-tools | 35 | Platform-Tools | platform-tools\\nAvailable Packages:\\n  Path | Version | Description\\n  platforms;android-36 | 2 | Android SDK Platform 36\\n'; fi\nexit 0\n",
                log.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut state = IdeState::default();
        state.android_tooling().write().sdk_setting = Some(root.clone());
        state.android_tooling().write().sdk =
            Some(SdkDetection::detect(Some(&root), None, &SystemEnvironment));
        let commands = built_in_commands();
        let wait = |state: &mut IdeState| {
            let until = Instant::now() + Duration::from_secs(10);
            while state.android_tooling().read().sdk_repo.is_busy() {
                state.poll_android();
                assert!(Instant::now() < until, "sdkmanager job hung");
                std::thread::sleep(Duration::from_millis(10));
            }
        };
        commands
            .execute("android.sdk.refreshRemote", &mut state, json!({}))
            .unwrap();
        wait(&mut state);
        assert!(state.android_tooling().read().sdk_repo.loaded);
        assert_eq!(state.android_tooling().read().sdk_repo.packages.len(), 2);
        let error = commands
            .execute(
                "android.sdk.apply",
                &mut state,
                json!({"install": ["--evil"], "uninstall": []}),
            )
            .unwrap_err();
        assert!(error.to_string().contains("not an SDK package"));
        commands
            .execute(
                "android.sdk.apply",
                &mut state,
                json!({"install": ["platforms;android-36"], "uninstall": ["platform-tools"]}),
            )
            .unwrap();
        wait(&mut state);
        let calls = std::fs::read_to_string(&log).unwrap();
        assert!(calls.contains("--uninstall platform-tools"), "{calls}");
        assert!(calls.contains("--install platforms;android-36"), "{calls}");
        assert!(matches!(
            state.android_tooling().read().sdk_repo.result,
            Some(Ok(_))
        ));
        std::fs::remove_dir_all(root).unwrap();
    }
}

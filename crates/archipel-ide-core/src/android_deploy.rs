//! Running an Android app on a device: Gradle builds the APK in the runner
//! (Build tool window), then a background job installs it with adb and starts
//! the launcher activity (`android.app.*`, `android.apk.install`).

use crate::{IdeState, ToolWindow, spec, string_param};
use archipel_ide_android::deploy::{DeployPlan, find_apks, install, launch};
use archipel_ide_android::{Adb, Device};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_runner::{TaskPlan, TaskState};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployPhase {
    Idle,
    Building,
    Installing,
    Launching,
    Done(String),
    Failed(String),
}

enum DeployEvent {
    Phase(DeployPhase),
    Line(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Steps {
    Install,
    Launch,
    InstallAndLaunch,
}

pub struct DeployState {
    pub phase: DeployPhase,
    /// Deploy waiting for the Gradle build to finish.
    pending: Option<(DeployPlan, PathBuf)>,
    events: Option<Receiver<DeployEvent>>,
    /// adb output and progress of the last deploy.
    pub log: Vec<String>,
}

impl Default for DeployState {
    fn default() -> Self {
        Self {
            phase: DeployPhase::Idle,
            pending: None,
            events: None,
            log: Vec::new(),
        }
    }
}

impl DeployState {
    pub fn is_busy(&self) -> bool {
        matches!(
            self.phase,
            DeployPhase::Building | DeployPhase::Installing | DeployPhase::Launching
        )
    }
}

fn device_name(devices: &[Device], serial: &str) -> String {
    devices
        .iter()
        .find(|d| d.serial == serial)
        .map(Device::display_name)
        .unwrap_or_else(|| serial.to_owned())
}

impl IdeState {
    fn deploy_target(&self, serial: Option<String>) -> Result<String, String> {
        let serial = serial
            .or_else(|| self.android.devices.selected.clone())
            .ok_or(
                "No target device. Connect a device or start an emulator from the Device Manager.",
            )?;
        match self
            .android
            .devices
            .devices
            .iter()
            .find(|d| d.serial == serial)
        {
            Some(device) if !device.is_online() => Err(format!(
                "{} is {}; it must be online to install apps.",
                device.display_name(),
                device.state.label().to_lowercase()
            )),
            _ => Ok(serial),
        }
    }

    fn deploy_plan(&self, serial: &str) -> Result<(DeployPlan, PathBuf), String> {
        let project = self
            .android
            .project
            .as_ref()
            .ok_or("The open project is not an Android project.")?;
        let variant = project
            .app_module()
            .and_then(|m| self.android.active_variants.get(&m.gradle_path))
            .map(String::as_str);
        Ok((
            DeployPlan::for_project(project, serial, variant)?,
            project.root.clone(),
        ))
    }

    /// Run 'app': builds the debug APK with Gradle, then installs and launches it.
    pub fn run_android_app(&mut self, serial: Option<String>) -> Result<(), String> {
        if self.execution().is_running() || self.android.deploy.is_busy() {
            return Err("A build or deploy is already running.".into());
        }
        let serial = self.deploy_target(serial)?;
        let (plan, root) = self.deploy_plan(&serial)?;
        let program = if root.join("gradlew").is_file() {
            if cfg!(windows) {
                "gradlew.bat"
            } else {
                "./gradlew"
            }
        } else {
            "gradle"
        };
        let task = TaskPlan {
            name: format!("Gradle {}", plan.gradle_task),
            program: program.into(),
            args: vec![plan.gradle_task.clone()],
            cwd: root.clone(),
            env: Vec::new(),
            kind: archipel_ide_runner::TaskKind::Build,
        };
        self.execution().start(task).map_err(|e| e.to_string())?;
        self.tool_windows.show(ToolWindow::Build);
        let deploy = &mut self.android.deploy;
        deploy.log = vec![format!(
            "Building {} for {}",
            plan.gradle_task,
            device_name(&self.android.devices.devices, &serial)
        )];
        deploy.phase = DeployPhase::Building;
        deploy.pending = Some((plan, root));
        self.status = format!(
            "Building 'app' for {}",
            device_name(&self.android.devices.devices, &serial)
        );
        Ok(())
    }

    fn start_deploy_job(
        &mut self,
        plan: DeployPlan,
        root: &Path,
        apk: Option<PathBuf>,
        steps: Steps,
    ) -> Result<(), String> {
        let adb: Adb = self.android.adb()?;
        let apk = match (steps, apk) {
            (Steps::Launch, _) => None,
            (_, Some(apk)) => Some(apk),
            (_, None) => Some(
                find_apks(&root.join(&plan.module_dir).join(&plan.apk_dir))
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        format!(
                            "No {} APK in {}. Build the app first ({}).",
                            plan.variant,
                            root.join(&plan.module_dir)
                                .join("build/outputs/apk")
                                .display(),
                            plan.gradle_task
                        )
                    })?,
            ),
        };
        let component = plan.component();
        if steps != Steps::Install && component.is_none() {
            return Err(format!(
                "{} declares no launcher activity (MAIN/LAUNCHER intent filter) in its manifest.",
                plan.package
            ));
        }
        let name = device_name(&self.android.devices.devices, &plan.serial);
        let (sender, receiver) = mpsc::channel();
        let serial = plan.serial.clone();
        std::thread::Builder::new()
            .name("archipel-ide-android-deploy".into())
            .spawn(move || {
                let send = |event| {
                    let _ = sender.send(event);
                };
                if let Some(apk) = apk {
                    send(DeployEvent::Phase(DeployPhase::Installing));
                    send(DeployEvent::Line(format!(
                        "$ adb -s {serial} install -r -t {}",
                        apk.display()
                    )));
                    match install(&adb, &serial, &apk) {
                        Ok(output) => {
                            for line in output.lines().filter(|l| !l.trim().is_empty()) {
                                send(DeployEvent::Line(line.to_owned()));
                            }
                        }
                        Err(error) => {
                            send(DeployEvent::Line(error.clone()));
                            send(DeployEvent::Phase(DeployPhase::Failed(error)));
                            return;
                        }
                    }
                }
                let Some(component) = component.filter(|_| steps != Steps::Install) else {
                    send(DeployEvent::Phase(DeployPhase::Done(format!("Installed on {name}"))));
                    return;
                };
                send(DeployEvent::Phase(DeployPhase::Launching));
                send(DeployEvent::Line(format!(
                    "$ adb -s {serial} shell am start -n {component} -a android.intent.action.MAIN -c android.intent.category.LAUNCHER"
                )));
                match launch(&adb, &serial, &component) {
                    Ok(output) => {
                        for line in output.lines().filter(|l| !l.trim().is_empty()) {
                            send(DeployEvent::Line(line.to_owned()));
                        }
                        send(DeployEvent::Phase(DeployPhase::Done(format!(
                            "Launched {component} on {name}"
                        ))));
                    }
                    Err(error) => {
                        send(DeployEvent::Line(error.clone()));
                        send(DeployEvent::Phase(DeployPhase::Failed(format!(
                            "Could not start {component}: {error}"
                        ))));
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        self.android.deploy.events = Some(receiver);
        self.android.deploy.phase = DeployPhase::Installing;
        Ok(())
    }

    /// Advances a pending Run after its Gradle build and drains deploy events.
    pub(crate) fn poll_deploy(&mut self) -> bool {
        let mut changed = false;
        if self.android.deploy.pending.is_some() && !self.execution().is_running() {
            changed = true;
            let (plan, root) = self.android.deploy.pending.take().expect("pending deploy");
            let task_state = self.execution().runner().state().clone();
            match task_state {
                TaskState::Exited(0) => {
                    if let Err(error) =
                        self.start_deploy_job(plan, &root, None, Steps::InstallAndLaunch)
                    {
                        self.android.deploy.phase = DeployPhase::Failed(error);
                    }
                }
                state => {
                    let reason = match state {
                        TaskState::Exited(code) => {
                            format!("Gradle build failed with exit code {code}")
                        }
                        TaskState::Cancelled => "Gradle build cancelled".into(),
                        _ => "Gradle build did not run".into(),
                    };
                    self.android.deploy.phase = DeployPhase::Failed(reason);
                }
            }
        }
        if let Some(events) = &self.android.deploy.events {
            let events: Vec<_> = events.try_iter().collect();
            changed |= !events.is_empty();
            for event in events {
                match event {
                    DeployEvent::Line(line) => self.android.deploy.log.push(line),
                    DeployEvent::Phase(phase) => {
                        if matches!(phase, DeployPhase::Done(_) | DeployPhase::Failed(_)) {
                            self.android.deploy.events = None;
                        }
                        self.android.deploy.phase = phase;
                    }
                }
            }
        }
        if changed {
            match &self.android.deploy.phase {
                DeployPhase::Installing => self.status = "Installing APK…".into(),
                DeployPhase::Launching => self.status = "Launching activity…".into(),
                DeployPhase::Done(message) | DeployPhase::Failed(message) => {
                    self.status = message.clone()
                }
                _ => {}
            }
        }
        changed
    }
}

fn serial_param(params: &Value) -> Option<String> {
    params
        .get("serial")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "android.app.run",
                "Run 'app' on Device",
                "Android",
                None,
                r#"{"serial":"string?"}"#,
            ),
            |state: &IdeState| state.android.project.is_some() && !state.execution().is_running(),
            |state: &mut IdeState, params| {
                state
                    .run_android_app(serial_param(&params))
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    for (id, label, steps) in [
        (
            "android.app.deploy",
            "Install and Launch 'app'",
            Steps::InstallAndLaunch,
        ),
        ("android.app.launch", "Launch 'app'", Steps::Launch),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Android", None, r#"{"serial":"string?"}"#),
                |state: &IdeState| {
                    state.android.project.is_some() && !state.android.deploy.is_busy()
                },
                move |state: &mut IdeState, params| {
                    let serial = state
                        .deploy_target(serial_param(&params))
                        .map_err(CommandError::Failed)?;
                    let (plan, root) = state.deploy_plan(&serial).map_err(CommandError::Failed)?;
                    state.android.deploy.log.clear();
                    state
                        .start_deploy_job(plan, &root, None, steps)
                        .map_err(CommandError::Failed)?;
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "android.apk.install",
                "Install APK",
                "Android",
                None,
                r#"{"path":"string","serial":"string?"}"#,
            ),
            |state: &IdeState| !state.android.deploy.is_busy(),
            |state: &mut IdeState, params| {
                let path = PathBuf::from(string_param(&params, "path")?);
                let root = state.project.as_ref().map(|p| p.root().to_owned());
                let apk = match (&root, path.is_absolute()) {
                    (Some(root), false) => root.join(&path),
                    _ => path,
                };
                if !apk.is_file() {
                    return Err(CommandError::InvalidParams(format!(
                        "{} is not a file",
                        apk.display()
                    )));
                }
                let serial = state
                    .deploy_target(serial_param(&params))
                    .map_err(CommandError::Failed)?;
                let plan = DeployPlan {
                    serial,
                    gradle_task: String::new(),
                    module_dir: PathBuf::new(),
                    variant: String::new(),
                    apk_dir: PathBuf::new(),
                    package: String::new(),
                    activity: None,
                };
                state.android.deploy.log.clear();
                state
                    .start_deploy_job(plan, Path::new(""), Some(apk), Steps::Install)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "android.app.status",
                "Android Deploy Status",
                "Android",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                let deploy = &state.android.deploy;
                let (phase, message) = match &deploy.phase {
                    DeployPhase::Idle => ("idle", None),
                    DeployPhase::Building => ("building", None),
                    DeployPhase::Installing => ("installing", None),
                    DeployPhase::Launching => ("launching", None),
                    DeployPhase::Done(m) => ("done", Some(m.clone())),
                    DeployPhase::Failed(m) => ("failed", Some(m.clone())),
                };
                Ok(json!({"phase": phase, "message": message, "log": deploy.log}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(all(test, unix))]
mod tests {
    use crate::{IdeState, built_in_commands};
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "archipel-ide-core-deploy-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    fn write(path: &Path, text: &str, executable: bool) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
        if executable {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    /// Android project whose `gradlew` writes the APK, plus an SDK whose adb
    /// logs its arguments and reports one online emulator.
    fn fixture(base: &Path, install_reply: &str) -> (PathBuf, PathBuf) {
        let project = base.join("project");
        write(
            &project.join("settings.gradle.kts"),
            "include(\":app\")\n",
            false,
        );
        write(
            &project.join("app/build.gradle.kts"),
            "plugins { id(\"com.android.application\") }\nandroid { namespace = \"com.example.app\" }\n",
            false,
        );
        write(
            &project.join("app/src/main/AndroidManifest.xml"),
            r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android"><application><activity android:name=".MainActivity"><intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter></activity></application></manifest>"#,
            false,
        );
        write(
            &project.join("gradlew"),
            "#!/bin/sh\necho \"> Task $1\"\nmkdir -p app/build/outputs/apk/debug\necho apk > app/build/outputs/apk/debug/app-debug.apk\necho BUILD SUCCESSFUL\n",
            true,
        );
        let sdk = base.join("sdk");
        let payload = "emulator-5554\tdevice\n";
        write(
            &sdk.join("platform-tools/adb"),
            &format!(
                "#!/bin/sh\necho \"$@\" >> \"{log}\"\ncase \"$*\" in\n  \"track-devices -l\") printf '%s' '{:04x}{payload}'; exec sleep 30 ;;\n  *\" install -r -t \"*) {install_reply} ;;\n  *\"shell am start\"*) echo 'Starting: Intent {{ cmp=com.example.app/.MainActivity }}' ;;\n  *) exit 1 ;;\nesac\n",
                payload.len(),
                log = base.join("adb.log").display(),
            ),
            true,
        );
        (project, sdk)
    }

    fn wait_until(state: &mut IdeState, condition: impl Fn(&IdeState) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !condition(state) {
            assert!(
                Instant::now() < deadline,
                "timed out: {:?}",
                state.android.deploy.phase
            );
            state.poll_runner();
            state.poll_android();
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn open(project: &Path, sdk: &Path) -> IdeState {
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": project}))
            .unwrap();
        state.set_android_sdk(sdk).unwrap();
        // Opening the project may have started tracking with a host SDK; restart.
        commands
            .execute("android.devices.track", &mut state, json!({}))
            .unwrap();
        wait_until(&mut state, |s| s.android.devices.selected.is_some());
        state
    }

    #[test]
    fn run_builds_installs_and_launches() {
        let base = temp("run");
        let (project, sdk) = fixture(&base, "echo 'Performing Streamed Install'; echo Success");
        let mut state = open(&project, &sdk);
        let commands = built_in_commands();
        commands
            .execute("android.app.run", &mut state, json!({}))
            .unwrap();
        wait_until(&mut state, |s| !s.android.deploy.is_busy());
        let status = commands
            .execute("android.app.status", &mut state, json!({}))
            .unwrap();
        assert_eq!(status["phase"], "done", "{status}");
        assert_eq!(
            status["message"],
            "Launched com.example.app/.MainActivity on emulator-5554"
        );
        let log = std::fs::read_to_string(base.join("adb.log")).unwrap();
        assert!(log.contains("-s emulator-5554 install -r -t "), "{log}");
        assert!(log.contains("app-debug.apk"), "{log}");
        assert!(
            log.contains("-s emulator-5554 shell am start -n com.example.app/.MainActivity -a android.intent.action.MAIN -c android.intent.category.LAUNCHER"),
            "{log}"
        );
        let output: String = state
            .execution()
            .runner()
            .output()
            .iter()
            .map(|c| c.text.clone())
            .collect();
        assert!(output.contains("> Task :app:assembleDebug"), "{output}");
        drop(state);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn install_failures_are_reported_with_hints() {
        let base = temp("fail");
        let (project, sdk) = fixture(
            &base,
            "echo 'adb: failed to install app-debug.apk: Failure [INSTALL_FAILED_OLDER_SDK: Requires newer sdk version #34 (current version is #30)]' >&2; exit 1",
        );
        let mut state = open(&project, &sdk);
        let commands = built_in_commands();
        let error = commands
            .execute("android.app.deploy", &mut state, json!({}))
            .unwrap_err();
        assert!(error.to_string().contains("No debug APK"), "{error}");
        commands
            .execute("android.app.run", &mut state, json!({}))
            .unwrap();
        wait_until(&mut state, |s| !s.android.deploy.is_busy());
        let status = commands
            .execute("android.app.status", &mut state, json!({}))
            .unwrap();
        assert_eq!(status["phase"], "failed");
        let message = status["message"].as_str().unwrap();
        assert!(message.contains("INSTALL_FAILED_OLDER_SDK"), "{message}");
        assert!(message.contains("minSdk"), "{message}");
        assert!(
            state.status.contains("Installation failed"),
            "{}",
            state.status
        );

        let missing = commands
            .execute(
                "android.apk.install",
                &mut state,
                json!({"path": "nope.apk"}),
            )
            .unwrap_err();
        assert!(missing.to_string().contains("is not a file"), "{missing}");
        drop(state);
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn run_requires_a_device() {
        let mut state = IdeState::default();
        let error = state.run_android_app(None).unwrap_err();
        assert!(error.contains("No target device"), "{error}");
    }
}

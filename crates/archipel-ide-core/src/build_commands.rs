//! Build, run and test actions backed by the build-system registry.

use crate::{IdeState, ToolWindow, spec, string_array_param, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_runner::{
    BuildSystem, BuildSystemKind, BuildSystemRegistry, RunnerError, TaskKind, TaskPlan, TaskState,
};
use serde_json::{Value, json};
use std::path::Path;

/// Registered build-system providers. Integrations (e.g. Android) may add providers
/// with precedence via [`BuildSystems::registry_mut`].
pub struct BuildSystems(BuildSystemRegistry);

impl Default for BuildSystems {
    fn default() -> Self {
        let mut registry = BuildSystemRegistry::default();
        registry.register_before(
            BuildSystemKind::Maven,
            Box::new(archipel_ide_gradle::Gradle),
        );
        Self(registry)
    }
}

impl BuildSystems {
    pub fn registry(&self) -> &BuildSystemRegistry {
        &self.0
    }

    pub fn registry_mut(&mut self) -> &mut BuildSystemRegistry {
        &mut self.0
    }

    pub fn detect(&self, root: &Path) -> Option<&dyn BuildSystem> {
        self.0.detect(root)
    }
}

/// What the run widget shows for the open project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedBuild {
    pub kind: BuildSystemKind,
    /// Label of the default run target (`Gradle Run`, `Cargo Run`).
    pub run_label: String,
    /// Why Run is unavailable, if it is.
    pub run_unavailable: Option<String>,
    /// Why Build is unavailable (missing tool), if it is.
    pub build_unavailable: Option<String>,
}

impl IdeState {
    /// Re-detects the build system of the open project.
    pub fn detect_build(&mut self) {
        self.detected_build = self.project.as_ref().and_then(|project| {
            let provider = self.build_systems.detect(project.root())?;
            let root = project.root();
            let run = provider.run_plan(root);
            let build = provider.build_plan(root);
            let run_label = match (&run, &build) {
                (Ok(plan), _) | (Err(_), Ok(plan)) => plan.name.clone(),
                (Err(_), Err(_)) => provider.kind().display_name().to_owned(),
            };
            Some(DetectedBuild {
                kind: provider.kind(),
                run_label,
                run_unavailable: run.err().map(|error| error.to_string()),
                build_unavailable: build.err().map(|error| error.to_string()),
            })
        });
    }

    fn plan(&self, kind: TaskKind) -> Result<TaskPlan, RunnerError> {
        let root = self
            .project
            .as_ref()
            .ok_or_else(|| RunnerError::NotRunnable("no project is open".into()))?
            .root();
        let provider = self
            .build_systems
            .detect(root)
            .ok_or_else(|| RunnerError::NoBuildSystem(root.to_owned()))?;
        match kind {
            TaskKind::Run => provider.run_plan(root),
            TaskKind::Test => provider.test_plan(root),
            _ => provider.build_plan(root),
        }
    }

    /// Starts `plan` in the Build tool window; failures are shown there as well.
    pub fn start_task(
        &mut self,
        plan: Result<TaskPlan, RunnerError>,
        label: &str,
        kind: TaskKind,
    ) -> Result<(), String> {
        if self.execution().is_running() {
            return Err("A task is already running; stop it first.".into());
        }
        self.tool_windows.show(ToolWindow::Build);
        match plan {
            Ok(plan) => {
                let name = plan.name.clone();
                self.execution().start(plan).map_err(|error| {
                    let message = error.to_string();
                    self.status = format!("{name}: {message}");
                    message
                })?;
                self.status = format!("Running {name}");
                Ok(())
            }
            Err(error) => {
                let message = error.to_string();
                let cwd = self
                    .project
                    .as_ref()
                    .map(|project| project.root().to_owned())
                    .unwrap_or_default();
                self.execution().fail(
                    Some(TaskPlan::new(label, "", Vec::<String>::new(), cwd).with_kind(kind)),
                    message.clone(),
                );
                self.status = format!("{label}: {message}");
                Err(message)
            }
        }
    }

    pub fn start_build(&mut self) -> Result<(), String> {
        let plan = self.plan(TaskKind::Build);
        self.start_task(plan, "Build", TaskKind::Build)
    }

    /// Runs the selected run configuration, else the runnable at the caret ("Current File"),
    /// else the detected default target.
    pub fn start_run(&mut self) -> Result<(), String> {
        if self.execution().configurations().selected().is_some() {
            return self.run_configuration(None);
        }
        // "Current File": the entry point or test at the caret, resolved like the gutter.
        if let Some(line) = self.caret_runnable_line() {
            return self.run_gutter(line, false, false);
        }
        let plan = self.plan(TaskKind::Run);
        self.start_task(plan, "Run", TaskKind::Run)
    }

    pub fn start_tests(&mut self) -> Result<(), String> {
        let plan = self.plan(TaskKind::Test);
        self.start_task(plan, "Test", TaskKind::Test)
    }

    /// Pumps the running task. A task that exited is handled through the manager's
    /// `process_terminated` event, see [`IdeState::run_exited`].
    pub fn poll_runner(&mut self) {
        if let Err(error) = self.execution().poll() {
            self.status = error.to_string();
        }
        self.dispatch_reactions();
    }

    /// What follows a task that exited: the debug session waiting for its build, the end of a
    /// sync, and the status bar text.
    pub(crate) fn run_exited(&mut self) {
        self.continue_pending_debug();
        let (kind, name, state) = {
            let execution = self.execution();
            let runner = execution.runner();
            (
                runner.plan().map(|plan| plan.kind),
                runner.plan().map(|plan| plan.name.clone()),
                runner.state().clone(),
            )
        };
        if kind == Some(TaskKind::Sync) {
            if self.is_bazel_project() {
                self.finish_bazel_sync();
            } else {
                self.finish_gradle_sync();
            }
            if matches!(state, TaskState::Exited(0)) {
                return;
            }
        }
        let name = name.unwrap_or_else(|| "Task".into());
        self.status = match state {
            TaskState::Exited(0) => format!("{name} finished successfully"),
            TaskState::Exited(code) => format!("{name} failed with exit code {code}"),
            TaskState::Cancelled => format!("{name} cancelled"),
            _ => self.status.clone(),
        };
    }

    /// Re-runs only the tests that failed in the last test task.
    pub fn rerun_failed_tests(&mut self) -> Result<(), String> {
        let last = self
            .execution()
            .runner()
            .plan()
            .cloned()
            .ok_or("nothing has run yet")?;
        let failed = self.execution().runner().tests().failed_tests();
        let root = self
            .project
            .as_ref()
            .ok_or("no project is open")?
            .root()
            .to_owned();
        let plan = self
            .build_systems
            .detect(&root)
            .ok_or_else(|| RunnerError::NoBuildSystem(root.clone()))
            .and_then(|provider| provider.rerun_failed_plan(&root, &last, &failed));
        self.start_task(plan, "Rerun Failed Tests", TaskKind::Test)
    }

    fn can_start(&self) -> bool {
        self.project.is_some() && !self.execution().is_running() && self.detected_build.is_some()
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec("build.project", "Build Project", "Build", Some("⌘F9"), "{}"),
            IdeState::can_start,
            |state: &mut IdeState, _| {
                state.start_build().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("run.project", "Run", "Run", Some("⌃R"), "{}"),
            |state: &IdeState| {
                let configured = state.execution().configurations().selected().is_some();
                state.project.is_some()
                    && !state.execution().is_running()
                    && (configured
                        || state
                            .detected_build
                            .as_ref()
                            .is_some_and(|build| build.run_unavailable.is_none()))
            },
            |state: &mut IdeState, _| {
                state.start_run().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("test.project", "Run All Tests", "Run", None, "{}"),
            IdeState::can_start,
            |state: &mut IdeState, _| {
                state.start_tests().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("test.rerunFailed", "Rerun Failed Tests", "Run", None, "{}"),
            |state: &IdeState| {
                state.project.is_some()
                    && !state.execution().is_running()
                    && state.execution().runner().tests().counts().failed > 0
            },
            |state: &mut IdeState, _| {
                state.rerun_failed_tests().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("build.detect", "Reload Build System", "Build", None, "{}"),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, _| {
                state.detect_build();
                Ok(match &state.detected_build {
                    Some(build) => json!({
                        "kind": build.kind.id(),
                        "runLabel": build.run_label,
                        "runUnavailable": build.run_unavailable,
                        "buildUnavailable": build.build_unavailable,
                    }),
                    None => Value::Null,
                })
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "run.task",
                "Run Task",
                "Run",
                None,
                r#"{"program":"string","args":["string"]}"#,
            ),
            |state: &IdeState| state.project.is_some() && !state.execution().is_running(),
            |state: &mut IdeState, params| {
                let program = string_param(&params, "program")?;
                let args = string_array_param(&params, "args")?;
                let cwd = state
                    .project
                    .as_ref()
                    .ok_or_else(|| CommandError::Disabled("run.task".into()))?
                    .root()
                    .to_owned();
                let plan = TaskPlan::new(program.clone(), program, args, cwd);
                state
                    .start_task(Ok(plan), "Run Task", TaskKind::Custom)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("run.rerun", "Rerun", "Run", None, "{}"),
            |state: &IdeState| {
                state.project.is_some()
                    && !state.execution().is_running()
                    && state
                        .execution()
                        .runner()
                        .plan()
                        .is_some_and(|plan| !plan.program.is_empty())
            },
            |state: &mut IdeState, _| {
                let plan = state
                    .execution()
                    .runner()
                    .plan()
                    .cloned()
                    .ok_or_else(|| CommandError::Disabled("run.rerun".into()))?;
                let (label, kind) = (plan.name.clone(), plan.kind);
                state
                    .start_task(Ok(plan), &label, kind)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("run.cancel", "Stop Running Task", "Run", None, "{}"),
            |state: &IdeState| state.execution().is_running(),
            |state: &mut IdeState, _| {
                state
                    .execution()
                    .cancel()
                    .map_err(|error| CommandError::Failed(error.to_string()))?;
                state.status = "Task cancelled".into();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(all(test, unix))]
mod tests {
    use crate::{IdeState, ToolWindow, built_in_commands};
    use archipel_ide_runner::{BuildSystemKind, TaskState};
    use serde_json::{Value, json};
    use std::path::PathBuf;

    fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-build-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn android_project_disables_run_with_reason_and_never_starts_gradle_run() {
        let root = project(
            "android",
            &[
                ("settings.gradle.kts", "include(\":app\")\n"),
                (
                    "app/build.gradle.kts",
                    "plugins { id(\"com.android.application\") }\n",
                ),
                ("gradlew", "#!/bin/sh\necho should-not-run\n"),
            ],
        );
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        let build = state.detected_build.clone().unwrap();
        assert_eq!(build.kind, BuildSystemKind::Gradle);
        assert!(
            build
                .run_unavailable
                .unwrap()
                .contains("Android application")
        );
        assert!(!commands.is_enabled("run.project", &state));
        assert!(commands.is_enabled("build.project", &state));
        // Even when invoked directly, the failure is shown in the Build window.
        assert!(state.start_run().is_err());
        assert!(
            matches!(state.execution().runner().state(), TaskState::Failed(reason) if reason.contains("Android"))
        );
        assert_eq!(
            state.tool_windows.visible(crate::Anchor::Bottom),
            Some(ToolWindow::Build)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_gradle_tests_are_rerun_with_filters() {
        let root = project(
            "rerun",
            &[
                ("settings.gradle.kts", ""),
                ("build.gradle.kts", "plugins { java }\n"),
                (
                    "gradlew",
                    "#!/bin/sh\necho \"args: $*\"\nmkdir -p build/test-results/test\nprintf '<testsuite name=\"T\"><testcase classname=\"com.T\" name=\"bad()\"><failure message=\"x\"/></testcase><testcase classname=\"com.T\" name=\"good()\"/></testsuite>' > build/test-results/test/TEST-T.xml\nexit 1\n",
                ),
            ],
        );
        std::process::Command::new("chmod")
            .args(["+x", "gradlew"])
            .current_dir(&root)
            .status()
            .unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        assert!(!commands.is_enabled("test.rerunFailed", &state));
        commands
            .execute("test.project", &mut state, Value::Null)
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while state.execution().is_running() && std::time::Instant::now() < deadline {
            state.poll_runner();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        state.poll_runner();
        assert_eq!(state.execution().runner().tests().counts().failed, 1);
        assert!(commands.is_enabled("test.rerunFailed", &state));
        commands
            .execute("test.rerunFailed", &mut state, Value::Null)
            .unwrap();
        assert_eq!(
            state.execution().runner().plan().unwrap().args,
            ["test", "--console=plain", "--tests", "com.T.bad"]
        );
        let _ = state.execution().cancel();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unknown_project_disables_build_actions() {
        let root = project("unknown", &[("README.md", "hi")]);
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        assert!(state.detected_build.is_none());
        for id in ["build.project", "run.project", "test.project"] {
            assert!(!commands.is_enabled(id, &state), "{id}");
        }
        assert!(state.start_build().is_err());
        assert!(
            matches!(state.execution().runner().state(), TaskState::Failed(reason) if reason.contains("no supported build system"))
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bazel_workspace_is_detected_and_run_explains_targets() {
        let root = project("bazel", &[("MODULE.bazel", ""), ("BUILD.bazel", "")]);
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        let detected = commands
            .execute("build.detect", &mut state, Value::Null)
            .unwrap();
        assert_eq!(detected["kind"], "bazel");
        assert!(
            detected["runUnavailable"]
                .as_str()
                .unwrap()
                .contains("target")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

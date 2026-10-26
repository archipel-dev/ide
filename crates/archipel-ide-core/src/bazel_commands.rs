//! Bazel sync (target query) and per-target build/run/test (Bazel tool window).

use crate::{IdeState, spec, string_param};
use archipel_ide_bazel::{BazelTargets, BazelWorkspace};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_runner::{BuildSystemKind, OutputStream, RunnerError, TaskKind, TaskState};
use serde_json::{Value, json};

impl IdeState {
    pub fn is_bazel_project(&self) -> bool {
        self.detected_build
            .as_ref()
            .is_some_and(|build| build.kind == BuildSystemKind::Bazel)
    }

    pub(crate) fn load_bazel_targets(&mut self) {
        self.bazel_targets = None;
        self.bazel_error = None;
        let Some(project) = &self.project else {
            return;
        };
        match BazelTargets::load_cached(project.root()) {
            Some(Ok(targets)) => self.bazel_targets = Some(targets),
            Some(Err(error)) => self.bazel_error = Some(error),
            None => {}
        }
    }

    pub fn start_bazel_sync(&mut self) -> Result<(), String> {
        let plan = self
            .project
            .as_ref()
            .ok_or_else(|| RunnerError::NotRunnable("no project is open".into()))
            .and_then(|project| {
                BazelWorkspace::detect(project.root())
                    .ok_or_else(|| RunnerError::NoBuildSystem(project.root().to_owned()))
            })
            .and_then(|workspace| workspace.query_plan());
        self.start_task(plan, "Bazel Sync", TaskKind::Sync)
    }

    /// Applies the target list of a finished `bazel query`.
    pub(crate) fn finish_bazel_sync(&mut self) {
        // `--keep_going` exits with 3 when some packages failed but results exist.
        if !matches!(self.runner.state(), TaskState::Exited(0 | 3)) {
            self.bazel_error = Some("Bazel sync failed; see the Build tool window".into());
            return;
        }
        let targets = BazelTargets::parse_label_kind(
            self.runner
                .console()
                .lines()
                .iter()
                .filter(|line| line.stream == OutputStream::Stdout)
                .map(|line| line.text.as_str()),
        );
        if let Some(project) = &self.project
            && let Err(error) = targets.save(project.root())
        {
            self.bazel_error = Some(format!("could not cache Bazel targets: {error}"));
        } else {
            self.bazel_error = None;
        }
        self.status = format!(
            "Bazel sync finished: {} target{}",
            targets.targets.len(),
            if targets.targets.len() == 1 { "" } else { "s" }
        );
        self.bazel_targets = Some(targets);
    }

    /// Builds, runs or tests one target through a temporary run configuration.
    pub fn run_bazel_target(&mut self, label: &str, command: Option<&str>) -> Result<(), String> {
        let target = self
            .bazel_targets
            .as_ref()
            .and_then(|targets| targets.get(label))
            .cloned()
            .ok_or_else(|| {
                format!("unknown Bazel target `{label}`; sync the Bazel project first")
            })?;
        let command = command.unwrap_or(target.role().default_command());
        if !matches!(command, "build" | "run" | "test") {
            return Err(format!("unsupported Bazel command `{command}`"));
        }
        self.run_temporary(target.configuration(command))
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec("bazel.sync", "Sync Bazel Project", "Bazel", None, "{}"),
            |state: &IdeState| state.is_bazel_project() && !state.runner.is_running(),
            |state: &mut IdeState, _| {
                state.start_bazel_sync().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "bazel.runTarget",
                "Build/Run/Test Bazel Target",
                "Bazel",
                None,
                r#"{"label":"string","command":"build|run|test?"}"#,
            ),
            |state: &IdeState| state.bazel_targets.is_some() && !state.runner.is_running(),
            |state: &mut IdeState, params| {
                let label = string_param(&params, "label")?;
                let command = params
                    .get("command")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                state
                    .run_bazel_target(&label, command.as_deref())
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("bazel.targets", "List Bazel Targets", "Bazel", None, "{}"),
            |state: &IdeState| state.is_bazel_project(),
            |state: &mut IdeState, _| {
                Ok(json!({
                    "targets": state.bazel_targets.as_ref().map(|targets| {
                        targets.targets.iter().map(|t| json!({"label": t.label, "kind": t.kind})).collect::<Vec<_>>()
                    }),
                    "error": state.bazel_error,
                }))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(all(test, unix))]
mod tests {
    use crate::{IdeState, built_in_commands};
    use serde_json::{Value, json};
    use std::time::{Duration, Instant};

    fn wait(state: &mut IdeState) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while state.runner.is_running() && Instant::now() < deadline {
            state.poll_runner();
            std::thread::sleep(Duration::from_millis(10));
        }
        state.poll_runner();
    }

    #[test]
    fn sync_with_fake_bazel_lists_targets_and_runs_them() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-bazel-cmd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(root.join("MODULE.bazel"), "").unwrap();
        // A fake `bazel` on PATH: answers the query and echoes everything else.
        std::fs::write(
            bin.join("bazel"),
            "#!/bin/sh\nif [ \"$1\" = query ]; then\n  echo 'java_binary rule //:app'\n  echo 'sh_test rule //lib:t_test'\n  echo 'Loading: done' >&2\nelse\n  echo \"bazel $*\"\nfi\n",
        )
        .unwrap();
        std::process::Command::new("chmod")
            .args(["+x", "bin/bazel"])
            .current_dir(&root)
            .status()
            .unwrap();
        // Discovery prefers `bazelisk`; shadow a real one (CI runners ship it).
        std::fs::copy(bin.join("bazel"), bin.join("bazelisk")).unwrap();
        let path = format!(
            "{}:{}",
            bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        // SAFETY: tests in this module run the fake tool through PATH; other tests
        // do not depend on `bazel`/`bazelisk` resolution.
        unsafe { std::env::set_var("PATH", &path) };
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("bazel.sync", &mut state, Value::Null)
            .unwrap();
        wait(&mut state);
        let listed = commands
            .execute("bazel.targets", &mut state, Value::Null)
            .unwrap();
        assert_eq!(listed["targets"][0]["label"], "//:app");
        assert_eq!(listed["targets"][1]["kind"], "sh_test");
        commands
            .execute(
                "bazel.runTarget",
                &mut state,
                json!({"label": "//lib:t_test"}),
            )
            .unwrap();
        wait(&mut state);
        assert_eq!(
            state.runner.console().lines()[0].text,
            "bazel test //lib:t_test"
        );
        assert_eq!(
            state.run_widget_label().as_deref(),
            Some("test //lib:t_test")
        );
        assert!(
            commands
                .execute(
                    "bazel.runTarget",
                    &mut state,
                    json!({"label": "//:app", "command": "clean"})
                )
                .is_err()
        );
        let mut reopened = IdeState::default();
        commands
            .execute("project.open", &mut reopened, json!({"path": root}))
            .unwrap();
        assert_eq!(reopened.bazel_targets, state.bazel_targets);
        std::fs::remove_dir_all(root).unwrap();
    }
}

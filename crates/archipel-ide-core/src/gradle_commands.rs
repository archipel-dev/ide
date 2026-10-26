//! Gradle sync and task execution (Gradle tool window actions).

use crate::{IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_gradle::{GradleBuild, GradleModel};
use archipel_ide_runner::{
    BuildSystemKind, RunConfiguration, RunTarget, RunnerError, TaskKind, TaskState,
};
use serde_json::{Value, json};

impl IdeState {
    pub fn is_gradle_project(&self) -> bool {
        self.detected_build
            .as_ref()
            .is_some_and(|build| build.kind == BuildSystemKind::Gradle)
    }

    fn gradle_build(&self) -> Result<GradleBuild, RunnerError> {
        let root = self
            .project
            .as_ref()
            .ok_or_else(|| RunnerError::NotRunnable("no project is open".into()))?
            .root();
        GradleBuild::detect(root).ok_or_else(|| RunnerError::NoBuildSystem(root.to_owned()))
    }

    /// Loads the model cached by the last sync (no Gradle process is started).
    pub fn load_gradle_model(&mut self) {
        self.gradle_model = None;
        self.model_generation += 1;
        self.gradle_error = None;
        let Some(project) = &self.project else {
            return;
        };
        match GradleModel::load_cached(project.root()) {
            Some(Ok(model)) => {
                self.gradle_model = Some(model);
                self.model_generation += 1;
            }
            Some(Err(error)) => self.gradle_error = Some(error),
            None => {}
        }
    }

    pub fn start_gradle_sync(&mut self) -> Result<(), String> {
        self.gradle_changes_pending = false;
        let plan = self.gradle_build().and_then(|build| build.sync_plan());
        self.start_task(plan, "Gradle Sync", TaskKind::Sync)
    }

    /// Runs task paths and options as typed in Execute Gradle Task
    /// (`:app:run`, `test --tests 'Foo*'` is passed token by token).
    pub fn run_gradle_task(&mut self, command: &str) -> Result<(), String> {
        let tokens: Vec<String> = command.split_whitespace().map(str::to_owned).collect();
        if tokens.iter().all(|token| token.starts_with('-')) {
            return Err("enter at least one Gradle task, e.g. :app:test".into());
        }
        // Like the reference, running a task creates a temporary run configuration.
        // Tokens keep their order so options such as `--tests Foo` stay together.
        let name = format!("{} [{}]", self.project_name_for_config(), tokens.join(" "));
        self.run_temporary(RunConfiguration::new(
            name,
            RunTarget::Gradle {
                tasks: tokens,
                options: Vec::new(),
                program_arguments: Vec::new(),
            },
        ))
    }

    fn project_name_for_config(&self) -> String {
        self.project
            .as_ref()
            .and_then(|project| project.root().file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Gradle".into())
    }

    /// After the first sync, offers the run configurations the reference IDE
    /// suggests: `run` for application projects and `test` for Java projects.
    /// Nothing is created when the project already has saved configurations.
    pub(crate) fn create_gradle_run_configurations(&mut self) {
        let (Some(model), Some(root)) = (&self.gradle_model, self.project_root()) else {
            return;
        };
        if self
            .run_configurations
            .configurations()
            .iter()
            .any(|config| !config.temporary)
        {
            return;
        }
        let mut configs = Vec::new();
        for project in &model.projects {
            for (wanted, applies) in [
                ("run", project.plugins.application),
                ("test", project.plugins.java),
            ] {
                let Some(task) = project
                    .tasks
                    .iter()
                    .find(|task| task.name == wanted)
                    .filter(|_| applies)
                else {
                    continue;
                };
                configs.push(RunConfiguration::new(
                    format!("{} [{wanted}]", project.name),
                    RunTarget::Gradle {
                        tasks: vec![task.path.clone()],
                        options: Vec::new(),
                        program_arguments: Vec::new(),
                    },
                ));
            }
        }
        // Run configurations first, so the run widget selects one of them.
        configs.sort_by_key(|config| !config.name.ends_with("[run]"));
        let first = configs.first().map(|config| config.name.clone());
        for config in configs {
            if let Err(error) = self.run_configurations.save(&root, None, config) {
                self.status = format!("Could not create run configuration: {error}");
                return;
            }
        }
        if let Some(name) = first {
            let _ = self.run_configurations.select(&name);
        }
    }

    /// Applies the result of a finished sync.
    pub(crate) fn finish_gradle_sync(&mut self) {
        match self.runner.state() {
            TaskState::Exited(0) => {
                self.load_gradle_model();
                self.create_gradle_run_configurations();
                if let Some(model) = &self.gradle_model {
                    let plural = |count: usize, word: &str| {
                        format!("{count} {word}{}", if count == 1 { "" } else { "s" })
                    };
                    self.status = format!(
                        "Gradle sync finished: {}, {} (Gradle {})",
                        plural(model.projects.len(), "project"),
                        plural(model.task_count(), "task"),
                        model.gradle_version
                    );
                } else {
                    self.gradle_error =
                        Some("Gradle finished but did not write the project model".into());
                    self.status = "Gradle sync did not produce a model".into();
                }
            }
            TaskState::Exited(_) => {
                self.gradle_error = Some("Gradle sync failed; see the Build tool window".into());
            }
            _ => {}
        }
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "gradle.sync",
                "Sync All Gradle Projects",
                "Gradle",
                None,
                "{}",
            ),
            |state: &IdeState| state.is_gradle_project() && !state.runner.is_running(),
            |state: &mut IdeState, _| {
                state.start_gradle_sync().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "gradle.loadChanges",
                "Load Gradle Changes",
                "Gradle",
                None,
                "{}",
            ),
            |state: &IdeState| {
                state.gradle_changes_pending
                    && state.is_gradle_project()
                    && !state.runner.is_running()
            },
            |state: &mut IdeState, _| {
                state.start_gradle_sync().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "gradle.dismissChanges",
                "Ignore Gradle Changes",
                "Gradle",
                None,
                "{}",
            ),
            |state: &IdeState| state.gradle_changes_pending,
            |state: &mut IdeState, _| {
                state.gradle_changes_pending = false;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "gradle.runTask",
                "Execute Gradle Task",
                "Gradle",
                None,
                r#"{"task":"string"}"#,
            ),
            |state: &IdeState| state.is_gradle_project() && !state.runner.is_running(),
            |state: &mut IdeState, params| {
                let task = string_param(&params, "task")?;
                state.run_gradle_task(&task).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "gradle.refresh",
                "Reload Gradle Model from Cache",
                "Gradle",
                None,
                "{}",
            ),
            |state: &IdeState| state.is_gradle_project(),
            |state: &mut IdeState, _| {
                state.load_gradle_model();
                if let Some(error) = &state.gradle_error {
                    return Err(CommandError::Failed(error.clone()));
                }
                Ok(match &state.gradle_model {
                    Some(model) => json!({
                        "gradleVersion": model.gradle_version,
                        "projects": model.projects.iter().map(|p| &p.path).collect::<Vec<_>>(),
                        "tasks": model.task_count(),
                    }),
                    None => Value::Null,
                })
            },
        ))
        .expect("unique built-in id");
}

#[cfg(all(test, unix))]
mod tests {
    use crate::{IdeState, built_in_commands};
    use archipel_ide_runner::TaskState;
    use serde_json::{Value, json};
    use std::time::{Duration, Instant};

    fn fixture(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-gradle-cmd-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("gradle")).unwrap();
        std::fs::write(root.join("settings.gradle.kts"), "include(\":app\")\n").unwrap();
        root
    }

    fn wait(state: &mut IdeState) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while state.runner.is_running() && Instant::now() < deadline {
            state.poll_runner();
            std::thread::sleep(Duration::from_millis(10));
        }
        state.poll_runner();
    }

    #[test]
    fn sync_with_fake_wrapper_loads_model_and_tasks_run_through_wrapper() {
        let root = fixture("sync");
        // A fake wrapper that writes a model where Gradle would, and echoes task runs.
        std::fs::write(
            root.join("gradlew"),
            r#"#!/bin/sh
for arg in "$@"; do
  case "$arg" in -Parchipel.modelFile=*) out="${arg#-Parchipel.modelFile=}";; esac
done
if [ -n "$out" ]; then
  printf '{"gradleVersion":"8.14.3","rootDir":"%s","projects":[{"path":":","name":"r","dir":"%s"},{"path":":app","name":"app","dir":"%s/app","plugins":{"java":true,"application":true},"tasks":[{"name":"run","path":":app:run","group":"application"},{"name":"test","path":":app:test","group":"verification"}]}]}' "$PWD" "$PWD" "$PWD" > "$out"
  echo "> Task :archipelModel"
else
  echo "ran $*"
fi
"#,
        )
        .unwrap();
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
        assert!(state.gradle_model.is_none());
        assert!(commands.is_enabled("gradle.sync", &state));
        commands
            .execute("gradle.sync", &mut state, Value::Null)
            .unwrap();
        wait(&mut state);
        assert_eq!(state.runner.state(), &TaskState::Exited(0));
        let model = state
            .gradle_model
            .as_ref()
            .expect("model loaded after sync");
        assert_eq!(model.project(":app").unwrap().tasks[0].path, ":app:run");
        // The first sync creates run and test configurations and selects run.
        let names: Vec<&str> = state
            .run_configurations
            .configurations()
            .iter()
            .map(|config| config.name.as_str())
            .collect();
        assert_eq!(names, ["app [run]", "app [test]"]);
        assert_eq!(
            state.run_configurations.selected().map(|c| c.name.as_str()),
            Some("app [run]")
        );
        assert!(root.join(".archipel/runConfigurations").is_dir());
        assert!(
            state.status.contains("2 projects, 2 tasks "),
            "{}",
            state.status
        );

        commands
            .execute("gradle.runTask", &mut state, json!({"task": ":app:run"}))
            .unwrap();
        wait(&mut state);
        assert_eq!(
            state.runner.console().lines()[0].text,
            "ran :app:run --console=plain"
        );
        assert!(
            commands
                .execute(
                    "gradle.runTask",
                    &mut state,
                    json!({"task": "  --offline "})
                )
                .is_err()
        );
        commands
            .execute(
                "gradle.runTask",
                &mut state,
                json!({"task": "clean :app:test --offline"}),
            )
            .unwrap();
        wait(&mut state);
        assert_eq!(
            state.runner.console().lines()[0].text,
            "ran clean :app:test --offline --console=plain"
        );

        // Reopening restores the cached model without running Gradle.
        let mut reopened = IdeState::default();
        commands
            .execute("project.open", &mut reopened, json!({"path": root}))
            .unwrap();
        assert_eq!(reopened.gradle_model, state.gradle_model);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn build_script_changes_offer_load_gradle_changes_and_sync_clears_them() {
        let root = fixture("autosync");
        std::fs::write(root.join("gradlew"), "#!/bin/sh\necho ran\n").unwrap();
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
        assert!(!commands.is_enabled("gradle.loadChanges", &state));
        std::fs::write(root.join("gradle.properties"), "org.gradle.daemon=false\n").unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !state.gradle_changes_pending && Instant::now() < deadline {
            state.poll_files();
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            state.gradle_changes_pending,
            "watcher saw gradle.properties"
        );
        assert!(commands.is_enabled("gradle.loadChanges", &state));
        commands
            .execute("gradle.loadChanges", &mut state, Value::Null)
            .unwrap();
        assert!(!state.gradle_changes_pending);
        wait(&mut state);
        state.gradle_changes_pending = true;
        commands
            .execute("gradle.dismissChanges", &mut state, Value::Null)
            .unwrap();
        assert!(!state.gradle_changes_pending);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn gradle_commands_are_disabled_for_other_projects() {
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-gradle-cmd-none-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("go.mod"), "module x\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        for id in ["gradle.sync", "gradle.runTask", "gradle.refresh"] {
            assert!(!commands.is_enabled(id, &state), "{id}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

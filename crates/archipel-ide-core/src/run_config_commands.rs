//! Run/Debug Configurations: list, select, edit, save, delete and run.

use crate::{IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_runner::{RunConfiguration, RunManager, RunTarget};
use serde_json::{Value, json};

/// Run widget label while no configuration is selected.
pub const CURRENT_FILE_LABEL: &str = "Current File";

impl IdeState {
    pub(crate) fn load_run_configurations(&mut self) {
        let loaded = match &self.project {
            Some(project) => RunManager::load(project.root()),
            None => RunManager::default(),
        };
        *self.execution().configurations_mut() = loaded;
    }

    /// Runs the named configuration (the selected one when `name` is `None`).
    pub fn run_configuration(&mut self, name: Option<&str>) -> Result<(), String> {
        let config = {
            let execution = self.execution();
            let configurations = execution.configurations();
            match name {
                Some(name) => configurations.get(name),
                None => configurations.selected(),
            }
            .cloned()
            .ok_or_else(|| "no run configuration is selected".to_owned())?
        };
        let root = self
            .project
            .as_ref()
            .ok_or("no project is open")?
            .root()
            .to_owned();
        if name.is_some() {
            self.execution().configurations_mut().select(&config.name)?;
        }
        let plan = self
            .build_systems
            .registry()
            .plan_configuration(&root, &config);
        let layout = (!config.open_tool_window).then(|| self.tool_windows.clone());
        let started = self.start_task(plan, &config.name, config.target.task_kind());
        if let Some(layout) = layout {
            self.tool_windows = layout;
        }
        started
    }

    /// Creates (or refreshes) a temporary configuration and runs it, like running a
    /// Gradle task or gutter target that has no saved configuration yet.
    pub fn run_temporary(&mut self, config: RunConfiguration) -> Result<(), String> {
        let name = config.name.clone();
        if self
            .execution()
            .configurations()
            .get(&name)
            .is_none_or(|existing| existing.temporary)
        {
            self.execution().configurations_mut().add_temporary(config);
        }
        self.run_configuration(Some(&name))
    }

    /// Label of the run widget: the selected configuration, else `Current File`.
    pub fn run_widget_label(&self) -> Option<String> {
        self.execution()
            .configurations()
            .selected()
            .map(|config| config.name.clone())
            .or_else(|| {
                self.detected_build
                    .as_ref()
                    .map(|_| CURRENT_FILE_LABEL.to_owned())
            })
    }
}

fn configuration_json(config: &RunConfiguration) -> Value {
    let mut value = serde_json::to_value(config).unwrap_or(Value::Null);
    if let Value::Object(map) = &mut value {
        map.insert("temporary".into(), Value::Bool(config.temporary));
        map.insert("typeName".into(), json!(config.target.type_name()));
    }
    value
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "runConfig.edit",
                "Edit Configurations…",
                "Run",
                None,
                r#"{"name":"string?"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                // The UI shell opens the Run/Debug Configurations dialog on its next frame.
                state.execution().request_edit_configurations(
                    params
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                );
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("runConfig.list", "List Run Configurations", "Run", None, "{}"),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, _| {
                let execution = state.execution();
                let manager = execution.configurations();
                Ok(json!({
                    "configurations": manager.configurations().iter().map(configuration_json).collect::<Vec<_>>(),
                    "selected": manager.selected().map(|config| config.name.clone()),
                    "errors": manager.errors.iter().map(|(path, error)| json!({"path": path, "error": error})).collect::<Vec<_>>(),
                    "types": RunTarget::TYPE_IDS,
                }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "runConfig.select",
                "Select Run Configuration",
                "Run",
                None,
                r#"{"name":"string"}"#,
            ),
            |state: &IdeState| {
                !state
                    .execution()
                    .configurations()
                    .configurations()
                    .is_empty()
            },
            |state: &mut IdeState, params| {
                let name = string_param(&params, "name")?;
                state
                    .execution()
                    .configurations_mut()
                    .select(&name)
                    .map_err(CommandError::InvalidParams)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "runConfig.selectCurrentFile",
                "Select Current File",
                "Run",
                None,
                "{}",
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, _| {
                state.execution().configurations_mut().clear_selection();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "runConfig.run",
                "Run Configuration",
                "Run",
                None,
                r#"{"name":"string?"}"#,
            ),
            |state: &IdeState| {
                state.project.is_some()
                    && !state.execution().is_running()
                    && !state
                        .execution()
                        .configurations()
                        .configurations()
                        .is_empty()
            },
            |state: &mut IdeState, params| {
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                state
                    .run_configuration(name.as_deref())
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "runConfig.save",
                "Save Run Configuration",
                "Run",
                None,
                r#"{"previousName":"string?","configuration":{"name":"string","type":"gradle|cargo|bazel|maven|npm|go|shell"}}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let config: RunConfiguration = serde_json::from_value(
                    params.get("configuration").cloned().unwrap_or(Value::Null),
                )
                .map_err(|error| CommandError::InvalidParams(format!("configuration: {error}")))?;
                let previous = params.get("previousName").and_then(Value::as_str);
                let root = state
                    .project
                    .as_ref()
                    .ok_or_else(|| CommandError::Disabled("runConfig.save".into()))?
                    .root()
                    .to_owned();
                state
                    .execution()
                    .configurations_mut()
                    .save(&root, previous, config)
                    .map_err(CommandError::InvalidParams)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "runConfig.delete",
                "Delete Run Configuration",
                "Run",
                None,
                r#"{"name":"string"}"#,
            ),
            |state: &IdeState| {
                !state
                    .execution()
                    .configurations()
                    .configurations()
                    .is_empty()
            },
            |state: &mut IdeState, params| {
                let name = string_param(&params, "name")?;
                let root = state
                    .project
                    .as_ref()
                    .ok_or_else(|| CommandError::Disabled("runConfig.delete".into()))?
                    .root()
                    .to_owned();
                state
                    .execution()
                    .configurations_mut()
                    .delete(&root, &name)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(all(test, unix))]
mod tests {
    use super::CURRENT_FILE_LABEL;
    use crate::{IdeState, built_in_commands};
    use archipel_ide_runner::TaskState;
    use serde_json::{Value, json};
    use std::time::{Duration, Instant};

    fn wait(state: &mut IdeState) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while state.execution().is_running() && Instant::now() < deadline {
            state.poll_runner();
            std::thread::sleep(Duration::from_millis(10));
        }
        state.poll_runner();
    }

    #[test]
    fn saved_configuration_runs_from_run_action_and_survives_reopen() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-runcfg-cmd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("go.mod"), "module x\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        let detected = CURRENT_FILE_LABEL.to_owned();
        assert_eq!(state.run_widget_label(), Some(detected.clone()));
        commands
            .execute(
                "runConfig.save",
                &mut state,
                json!({"configuration": {
                    "name": "Where am I",
                    "type": "shell",
                    "program": "/bin/sh",
                    "arguments": ["-c", "pwd; printf \"$GREETING\""],
                    "working_directory": "sub",
                    "env": {"GREETING": "hello"}
                }}),
            )
            .unwrap();
        assert_eq!(state.run_widget_label().as_deref(), Some("Where am I"));
        // The Run action runs the selected configuration.
        commands
            .execute("run.project", &mut state, Value::Null)
            .unwrap();
        wait(&mut state);
        assert_eq!(state.execution().runner().state(), &TaskState::Exited(0));
        let lines: Vec<_> = state
            .execution()
            .runner()
            .console()
            .lines()
            .iter()
            .map(|line| line.text.clone())
            .collect();
        assert!(lines[0].ends_with("/sub"), "{lines:?}");
        assert_eq!(lines[1], "hello");

        let error = commands
            .execute(
                "runConfig.save",
                &mut state,
                json!({"configuration": {"name": "Bad", "type": "bazel", "command": "run", "target": "app"}}),
            )
            .unwrap_err();
        assert!(error.to_string().contains("//:app"), "{error}");

        let mut reopened = IdeState::default();
        commands
            .execute("project.open", &mut reopened, json!({"path": root}))
            .unwrap();
        let listed = commands
            .execute("runConfig.list", &mut reopened, Value::Null)
            .unwrap();
        assert_eq!(listed["selected"], "Where am I");
        assert_eq!(listed["configurations"][0]["typeName"], "Shell Command");
        commands
            .execute(
                "runConfig.delete",
                &mut reopened,
                json!({"name": "Where am I"}),
            )
            .unwrap();
        assert_eq!(reopened.run_widget_label(), Some(detected));
        std::fs::remove_dir_all(root).unwrap();
    }
}

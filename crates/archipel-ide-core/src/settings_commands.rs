//! Settings state and `settings.*` commands.

use crate::services::Settings;
use crate::{IdeState, spec, string_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_settings::{SettingValue, ids};
use serde_json::{Value, json};
use std::path::Path;

impl IdeState {
    /// Loads the per-user settings from `dir` (normally
    /// [`archipel_ide_settings::default_config_dir`]).
    pub fn load_user_settings(&mut self, dir: &Path) {
        let loaded = self.settings().write().store.load_application(dir);
        if let Err(error) = loaded {
            self.status = format!("{error}. Using defaults; the file is left unchanged.");
        }
        self.refresh_keymap();
        self.sync_gradle_options();
    }

    pub(crate) fn load_project_settings(&mut self, root: &Path) {
        let loaded = self.settings().write().store.load_project(root);
        if let Err(error) = loaded {
            self.status = format!("{error}. Using defaults; the file is left unchanged.");
        }
        self.sync_gradle_options();
    }

    /// Hands Settings | Build Tools | Gradle to the Gradle integration.
    pub(crate) fn sync_gradle_options(&self) {
        use archipel_ide_gradle::{BuildRunner, Distribution, GradleOptions};
        let settings = self.settings();
        let settings = settings.read();
        let path = |id| settings.text(id).filter(|v| !v.trim().is_empty());
        archipel_ide_gradle::set_options(GradleOptions {
            build_run: BuildRunner::from_id(
                &settings.text(ids::GRADLE_BUILD_RUN).unwrap_or_default(),
            ),
            distribution: Distribution::from_id(
                &settings.text(ids::GRADLE_DISTRIBUTION).unwrap_or_default(),
            ),
            offline: settings.bool(ids::GRADLE_OFFLINE).unwrap_or(false),
            user_home: path(ids::GRADLE_USER_HOME),
            jvm: path(ids::GRADLE_JVM),
        });
    }

    /// Saves the settings and updates state derived from them (the keymap).
    pub(crate) fn persist_settings(&mut self) -> Result<(), CommandError> {
        self.refresh_keymap();
        self.sync_gradle_options();
        self.settings()
            .read()
            .store
            .save()
            .map_err(|error| CommandError::Failed(error.to_string()))
    }
}

fn setting_id(state: &IdeState, params: &Value) -> Result<String, CommandError> {
    let id = string_param(params, "id")?;
    if state.settings().read().registry.setting(&id).is_none() {
        return Err(CommandError::InvalidParams(format!(
            "unknown setting `{id}`"
        )));
    }
    Ok(id)
}

fn value_param(params: &Value, name: &str) -> Result<SettingValue, CommandError> {
    params
        .get(name)
        .and_then(SettingValue::from_json)
        .ok_or_else(|| {
            CommandError::InvalidParams(format!(
                "expected `{name}` to be a boolean, number, string or list of strings"
            ))
        })
}

fn invalid(error: archipel_ide_settings::SettingsError) -> CommandError {
    CommandError::InvalidParams(error.to_string())
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "settings.open",
                "Settings…",
                "File",
                Some("⌘,"),
                r#"{"page":"string?"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let page = match params.get("page").and_then(Value::as_str) {
                    Some(page) if state.settings().read().registry.page(page).is_none() => {
                        return Err(CommandError::InvalidParams(format!(
                            "unknown settings page `{page}`"
                        )));
                    }
                    Some(page) => page.to_owned(),
                    None => "preferences.lookFeel".to_owned(),
                };
                state.settings_request = Some(page.clone());
                Ok(json!({"page": page}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "settings.get",
                "Get Setting",
                "Settings",
                None,
                r#"{"id":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = setting_id(state, &params)?;
                let settings = state.settings();
                let settings = settings.read();
                let value = settings
                    .store
                    .get(&settings.registry, &id)
                    .map_err(invalid)?;
                Ok(json!({"id": id, "value": value.to_json()}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "settings.set",
                "Set Setting",
                "Settings",
                None,
                r#"{"id":"string","value":"any"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = setting_id(state, &params)?;
                let value = value_param(&params, "value")?;
                {
                    let settings = state.settings();
                    let mut settings = settings.write();
                    let Settings { registry, store } = &mut *settings;
                    store.set(registry, &id, value).map_err(invalid)?;
                }
                state.persist_settings()?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "settings.reset",
                "Reset Setting to Default",
                "Settings",
                None,
                r#"{"id":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let id = setting_id(state, &params)?;
                {
                    let settings = state.settings();
                    let mut settings = settings.write();
                    let Settings { registry, store } = &mut *settings;
                    store.reset(registry, &id).map_err(invalid)?;
                }
                state.persist_settings()?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "settings.apply",
                "Apply Settings",
                "Settings",
                None,
                r#"{"changes":"object"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let changes = params
                    .get("changes")
                    .and_then(Value::as_object)
                    .ok_or_else(|| {
                        CommandError::InvalidParams("expected object field `changes`".into())
                    })?;
                // Validate everything before touching the store (all-or-nothing).
                {
                    let settings = state.settings();
                    let mut settings = settings.write();
                    let Settings { registry, store } = &mut *settings;
                    let mut next = store.clone();
                    for (id, value) in changes {
                        let value = SettingValue::from_json(value).ok_or_else(|| {
                            CommandError::InvalidParams(format!("unsupported value for `{id}`"))
                        })?;
                        next.set(registry, id, value).map_err(invalid)?;
                    }
                    *store = next;
                }
                state.persist_settings()?;
                state.status = match changes.len() {
                    1 => "1 setting changed".into(),
                    count => format!("{count} settings changed"),
                };
                Ok(json!({"changed": changes.keys().collect::<Vec<_>>()}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "settings.search",
                "Search Settings",
                "Settings",
                None,
                r#"{"query":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let query = string_param(&params, "query")?;
                let settings = state.settings();
                let settings = settings.read();
                let registry = &settings.registry;
                let result = registry.search(&query);
                let pages: Vec<_> = result
                    .matching_pages
                    .iter()
                    .map(|id| json!({"id": id, "path": registry.breadcrumbs(id)}))
                    .collect();
                Ok(json!({"pages": pages, "settings": result.matching_settings}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::built_in_commands;
    use archipel_ide_settings::ids;
    use serde_json::json;

    #[test]
    fn settings_commands_validate_apply_and_persist() {
        let commands = built_in_commands();
        let mut state = crate::IdeState::default();
        let config =
            std::env::temp_dir().join(format!("archipel-ide-core-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&config);
        state.load_user_settings(&config);

        let result = commands
            .execute(
                "settings.open",
                &mut state,
                json!({"page": "AppEditorFontConfigurable"}),
            )
            .unwrap();
        assert_eq!(result["page"], "AppEditorFontConfigurable");
        assert_eq!(
            state.settings_request.as_deref(),
            Some("AppEditorFontConfigurable")
        );
        assert!(
            commands
                .execute("settings.open", &mut state, json!({"page": "nope"}))
                .is_err()
        );

        // All-or-nothing: one invalid value rejects the whole batch.
        assert!(
            commands
                .execute(
                    "settings.apply",
                    &mut state,
                    json!({"changes": {ids::EDITOR_FONT_SIZE: 16, ids::THEME: "Bogus"}}),
                )
                .is_err()
        );
        assert_eq!(state.settings().read().int(ids::EDITOR_FONT_SIZE), Some(13));
        commands
            .execute(
                "settings.apply",
                &mut state,
                json!({"changes": {ids::EDITOR_FONT_SIZE: 16, ids::SMOOTH_SCROLLING: false}}),
            )
            .unwrap();
        assert_eq!(state.settings().read().int(ids::EDITOR_FONT_SIZE), Some(16));
        assert_eq!(state.status, "2 settings changed");
        let saved = std::fs::read_to_string(config.join("settings.toml")).unwrap();
        assert!(saved.contains("editor.font.size"));

        let got = commands
            .execute(
                "settings.get",
                &mut state,
                json!({"id": ids::EDITOR_FONT_SIZE}),
            )
            .unwrap();
        assert_eq!(got["value"], 16);
        commands
            .execute(
                "settings.reset",
                &mut state,
                json!({"id": ids::EDITOR_FONT_SIZE}),
            )
            .unwrap();
        assert_eq!(state.settings().read().int(ids::EDITOR_FONT_SIZE), Some(13));
        assert!(
            commands
                .execute(
                    "settings.set",
                    &mut state,
                    json!({"id": "x.y", "value": true})
                )
                .is_err()
        );
        assert!(
            commands
                .execute(
                    "settings.set",
                    &mut state,
                    json!({"id": ids::CHECK_UPDATES, "value": {"nested": 1}})
                )
                .is_err()
        );

        let found = commands
            .execute("settings.search", &mut state, json!({"query": "gradle"}))
            .unwrap();
        assert!(
            found["pages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|page| page["id"] == "reference.settingsdialog.project.gradle")
        );
        let _ = std::fs::remove_dir_all(config);
    }
}

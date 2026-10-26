//! First start: "Customize Archipel IDE" with theme and keymap choices and an
//! optional settings import, like the reference IDE's import/customize step.
//!
//! The wizard opens when the config directory has neither `settings.toml` nor
//! the `first-run.done` marker, so existing users are not asked again. Help →
//! Customize Archipel IDE… reopens it.

use crate::{IdeState, spec};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_settings::{SettingKind, SettingValue, SettingsStore, ids};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub const FIRST_RUN_MARKER: &str = "first-run.done";

/// Result of importing a `settings.toml` from another installation.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub imported: Vec<String>,
    /// Unknown ids or invalid values, with the reason.
    pub skipped: Vec<String>,
}

impl IdeState {
    /// Opens the wizard when `config_dir` shows no previous use (startup).
    pub fn detect_first_run(&mut self, config_dir: &Path) {
        self.first_run_dir = Some(config_dir.to_owned());
        self.first_run_open = !config_dir.join(FIRST_RUN_MARKER).exists()
            && !config_dir.join("settings.toml").exists();
    }

    fn finish_first_run(&mut self) -> Result<(), CommandError> {
        self.first_run_open = false;
        let Some(dir) = &self.first_run_dir else {
            return Ok(());
        };
        std::fs::create_dir_all(dir)
            .and_then(|_| std::fs::write(dir.join(FIRST_RUN_MARKER), "done\n"))
            .map_err(|error| {
                CommandError::Failed(format!("cannot write {}: {error}", dir.display()))
            })
    }

    /// Copies every known, valid setting from another `settings.toml` into the
    /// application layer (in memory; the caller persists).
    pub fn import_settings(&mut self, path: &Path) -> Result<ImportReport, CommandError> {
        let text = std::fs::read_to_string(path).map_err(|error| {
            CommandError::Failed(format!("cannot read {}: {error}", path.display()))
        })?;
        let layer = SettingsStore::parse_layer(&text).map_err(|reason| {
            CommandError::Failed(format!(
                "{} is not a settings file: {reason}",
                path.display()
            ))
        })?;
        let mut report = ImportReport::default();
        let settings = &mut self.settings;
        for (id, value) in layer {
            match settings.store.set(&settings.registry, &id, value) {
                Ok(()) => report.imported.push(id),
                Err(error) => report.skipped.push(format!("{id}: {error}")),
            }
        }
        Ok(report)
    }

    /// Option ids and labels of an enum setting (theme, keymap) for the wizard.
    pub fn setting_options(&self, id: &str) -> Vec<(String, String)> {
        match self.settings.registry.setting(id).map(|def| &def.kind) {
            Some(SettingKind::Enum { options }) => options
                .iter()
                .map(|option| (option.id.clone(), option.label.clone()))
                .collect(),
            _ => Vec::new(),
        }
    }
}

fn optional_text(params: &Value, name: &str) -> Result<Option<String>, CommandError> {
    match params.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(CommandError::InvalidParams(format!(
            "`{name}` must be a string"
        ))),
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "app.customize",
                "Customize Archipel IDE…",
                "Help",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                state.first_run_open = true;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "app.completeCustomize",
                "Apply Customization",
                "Help",
                None,
                r#"{"theme":"string?","keymap":"string?","importFrom":"string?"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let theme = optional_text(&params, "theme")?;
                let keymap = optional_text(&params, "keymap")?;
                let import = optional_text(&params, "importFrom")?
                    .filter(|path| !path.trim().is_empty())
                    .map(|path| PathBuf::from(path.trim()));
                // Validate the choices before importing or writing anything.
                let mut choices = Vec::new();
                for (id, value) in [(ids::THEME, theme), (ids::KEYMAP, keymap)] {
                    let Some(value) = value else { continue };
                    let def = state.settings.registry.setting(id).ok_or_else(|| {
                        CommandError::Failed(format!("setting `{id}` is not registered"))
                    })?;
                    let value = def
                        .validate(&SettingValue::Text(value))
                        .map_err(|error| CommandError::InvalidParams(error.to_string()))?;
                    choices.push((id, value));
                }
                let report = match &import {
                    Some(path) => state.import_settings(path)?,
                    None => ImportReport::default(),
                };
                // Explicit choices win over imported values.
                let settings = &mut state.settings;
                for (id, value) in choices {
                    settings
                        .store
                        .set(&settings.registry, id, value)
                        .map_err(|error| CommandError::InvalidParams(error.to_string()))?;
                }
                // Saves and rebuilds the keymap engine from the chosen keymap.
                state.persist_settings()?;
                state.finish_first_run()?;
                state.status = match (import.is_some(), report.skipped.len()) {
                    (false, _) => "Archipel IDE is set up".into(),
                    (true, 0) => format!("Imported {} settings", report.imported.len()),
                    (true, skipped) => format!(
                        "Imported {} settings, skipped {skipped}",
                        report.imported.len()
                    ),
                };
                Ok(json!({"imported": report.imported, "skipped": report.skipped}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "app.skipCustomize",
                "Skip Customization",
                "Help",
                None,
                "{}",
            ),
            |state: &IdeState| state.first_run_open,
            |state: &mut IdeState, _| {
                state.finish_first_run()?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;

    fn config(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "archipel-ide-first-run-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn opens_only_without_previous_settings_or_marker() {
        let dir = config("detect");
        let mut state = IdeState::default();
        state.detect_first_run(&dir);
        assert!(state.first_run_open);
        std::fs::write(dir.join("settings.toml"), "").unwrap();
        state.detect_first_run(&dir);
        assert!(!state.first_run_open, "existing users are not asked");
        std::fs::remove_file(dir.join("settings.toml")).unwrap();
        std::fs::write(dir.join(FIRST_RUN_MARKER), "done").unwrap();
        state.detect_first_run(&dir);
        assert!(!state.first_run_open);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn completing_applies_theme_and_keymap_and_persists() {
        let dir = config("complete");
        let commands = built_in_commands();
        let mut state = IdeState::default();
        state.load_user_settings(&dir);
        state.detect_first_run(&dir);
        assert!(
            commands
                .execute(
                    "app.completeCustomize",
                    &mut state,
                    json!({"theme": "Bogus"}),
                )
                .is_err()
        );
        assert!(state.first_run_open, "invalid choices keep the wizard open");
        commands
            .execute(
                "app.completeCustomize",
                &mut state,
                json!({"theme": "Islands Light", "keymap": "Default for KDE"}),
            )
            .unwrap();
        assert!(!state.first_run_open);
        assert_eq!(
            state.settings.text(ids::THEME).as_deref(),
            Some("Islands Light")
        );
        assert_eq!(
            state.settings.text(ids::KEYMAP).as_deref(),
            Some("Default for KDE")
        );
        assert_eq!(state.keymap.keymap.id, "Default for KDE");
        assert!(dir.join(FIRST_RUN_MARKER).is_file());
        let saved = std::fs::read_to_string(dir.join("settings.toml")).unwrap();
        assert!(saved.contains("Islands Light"), "{saved}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn import_takes_valid_settings_and_reports_the_rest() {
        let dir = config("import");
        let source = config("import-source");
        let mut exported = IdeState::default();
        exported.load_user_settings(&source);
        let commands = built_in_commands();
        commands
            .execute(
                "settings.apply",
                &mut exported,
                json!({"changes": {ids::EDITOR_FONT_SIZE: 17, ids::THEME: "Islands Light"}}),
            )
            .unwrap();
        let mut text = std::fs::read_to_string(source.join("settings.toml")).unwrap();
        text = text.replace("[settings]", "[settings]\n\"no.such.setting\" = true");
        std::fs::write(source.join("settings.toml"), text).unwrap();

        let mut state = IdeState::default();
        state.load_user_settings(&dir);
        state.detect_first_run(&dir);
        let result = commands
            .execute(
                "app.completeCustomize",
                &mut state,
                json!({
                    "importFrom": source.join("settings.toml"),
                    "theme": "Islands Dark",
                }),
            )
            .unwrap();
        assert_eq!(state.settings.int(ids::EDITOR_FONT_SIZE), Some(17));
        assert_eq!(
            state.settings.text(ids::THEME).as_deref(),
            Some("Islands Dark"),
            "the explicit choice wins over the imported theme"
        );
        assert_eq!(result["skipped"].as_array().unwrap().len(), 1, "{result}");
        assert!(
            state.status.starts_with("Imported 2 settings, skipped 1"),
            "{}",
            state.status
        );
        assert!(
            commands
                .execute(
                    "app.completeCustomize",
                    &mut state,
                    json!({"importFrom": dir.join("missing.toml")}),
                )
                .is_err()
        );
        std::fs::remove_dir_all(dir).unwrap();
        std::fs::remove_dir_all(source).unwrap();
    }

    #[test]
    fn skip_writes_the_marker_and_customize_reopens() {
        let dir = config("skip");
        let commands = built_in_commands();
        let mut state = IdeState::default();
        state.detect_first_run(&dir);
        commands
            .execute("app.skipCustomize", &mut state, Value::Null)
            .unwrap();
        assert!(!state.first_run_open);
        assert!(dir.join(FIRST_RUN_MARKER).is_file());
        commands
            .execute("app.customize", &mut state, Value::Null)
            .unwrap();
        assert!(state.first_run_open);
        assert!(!state.setting_options(ids::KEYMAP).is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }
}

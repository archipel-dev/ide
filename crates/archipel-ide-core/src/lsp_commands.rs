//! Language server commands: completion, server status, restart and notices.
use crate::{IdeState, spec, string_param, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_lsp::CompletionTrigger;
use serde_json::{Value, json};
use std::path::PathBuf;

impl IdeState {
    /// Absolute path of the active document.
    pub fn active_absolute_path(&self) -> Option<PathBuf> {
        let document = self.documents.get(self.active_document?)?;
        Some(self.project.as_ref()?.root().join(&document.path))
    }

    /// Whether a language server is registered (and enabled) for the active document.
    pub fn active_has_language_server(&self) -> bool {
        self.active_absolute_path()
            .is_some_and(|path| self.lsp.handles(&path))
    }

    /// Restarts every language server and re-opens the open documents.
    pub fn restart_language_servers(&mut self) {
        self.lsp.restart_all();
        if let Some(root) = self.project.as_ref().map(|p| p.root().to_owned()) {
            self.lsp.set_project(Some(&root));
        }
        let documents: Vec<(PathBuf, String, i32)> = self
            .documents
            .iter()
            .map(|document| {
                (
                    document.path.clone(),
                    document.text.text(),
                    document.text.revision().min(i32::MAX as u64) as i32,
                )
            })
            .collect();
        for (path, text, version) in documents {
            self.sync_document_lsp(&path, &text, version);
        }
        self.status = "Restarted language servers".into();
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "language.parameterInfo",
                "Parameter Info",
                "Code",
                Some("⌘P"),
                "{}",
            ),
            |state: &IdeState| state.active_has_language_server(),
            |state: &mut IdeState, _| {
                state
                    .request_parameter_info(None, true)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "language.closeParameterInfo",
                "Close Parameter Info",
                "Code",
                None,
                "{}",
            ),
            |state: &IdeState| state.signature.popup.is_some(),
            |state: &mut IdeState, _| {
                state.close_parameter_info();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "language.quickDoc",
                "Quick Documentation",
                "Code",
                Some("F1"),
                "{}",
            ),
            |state: &IdeState| state.active_has_language_server(),
            |state: &mut IdeState, _| {
                state.toggle_quick_doc().map_err(CommandError::Failed)?;
                Ok(json!({"panel": state.docs.completion_panel}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "language.hoverAt",
                "Show Documentation at Offset",
                "Code",
                None,
                r#"{"offset":0}"#,
            ),
            |state: &IdeState| state.active_has_language_server(),
            |state: &mut IdeState, params| {
                let offset = usize_param(&params, "offset")?;
                state
                    .request_quick_doc(offset, true)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "language.closeQuickDoc",
                "Close Documentation",
                "Code",
                None,
                "{}",
            ),
            |state: &IdeState| state.docs.popup.is_some(),
            |state: &mut IdeState, _| {
                state.close_quick_doc();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    // Tab, Shift+Tab and Esc drive these only while a snippet is active, so
    // they carry no menu shortcut.
    for (id, label) in [
        ("snippet.next", "Next Snippet Stop"),
        ("snippet.previous", "Previous Snippet Stop"),
        ("snippet.exit", "Exit Snippet"),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Code", None, "{}"),
                |state: &IdeState| state.snippet.is_some(),
                move |state: &mut IdeState, _| {
                    match id {
                        "snippet.next" => {
                            state.snippet_next();
                        }
                        "snippet.previous" => {
                            state.snippet_previous();
                        }
                        _ => state.exit_snippet(),
                    }
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "completion.accept",
                "Accept Completion",
                "Code",
                Some("⏎"),
                r#"{"replace":"bool?","row":"number?"}"#,
            ),
            |state: &IdeState| state.completion.as_ref().is_some_and(|s| s.visible()),
            |state: &mut IdeState, params| {
                if let Some(row) = params.get("row") {
                    let row = row
                        .as_u64()
                        .and_then(|row| usize::try_from(row).ok())
                        .ok_or_else(|| {
                            CommandError::InvalidParams("row must be a non-negative integer".into())
                        })?;
                    state.select_completion(row);
                }
                let replace = params
                    .get("replace")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                state
                    .accept_completion(replace)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "completion.move",
                "Move Completion Selection",
                "Code",
                None,
                r#"{"delta":"number"}"#,
            ),
            |state: &IdeState| state.completion.as_ref().is_some_and(|s| s.visible()),
            |state: &mut IdeState, params| {
                let delta = params
                    .get("delta")
                    .and_then(Value::as_i64)
                    .and_then(|delta| isize::try_from(delta).ok())
                    .ok_or_else(|| {
                        CommandError::InvalidParams("delta must be an integer".into())
                    })?;
                state.move_completion_selection(delta);
                Ok(json!({"selected": state.completion.as_ref().map(|s| s.selected)}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "completion.close",
                "Close Completion",
                "Code",
                Some("⎋"),
                "{}",
            ),
            |state: &IdeState| state.completion.is_some(),
            |state: &mut IdeState, _| {
                state.close_completion();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "language.complete",
                "Code Completion",
                "Code",
                Some("⌃Space"),
                "{}",
            ),
            |state: &IdeState| {
                state.active_has_language_server() || state.active_has_completion_source()
            },
            |state: &mut IdeState, _| {
                let id = state
                    .start_completion(CompletionTrigger::Invoked, true)
                    .map_err(CommandError::Failed)?;
                Ok(json!({"requestId": id}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "language.completion",
                "Request Completion",
                "Code",
                None,
                r#"{"path":"string","line":0,"character":0}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let relative = string_param(&params, "path")?;
                let line = usize_param(&params, "line")?;
                let character = usize_param(&params, "character")?;
                let root = state
                    .project
                    .as_ref()
                    .ok_or_else(|| CommandError::Disabled("language.completion".into()))?
                    .root()
                    .to_owned();
                let id = state
                    .lsp
                    .request_completion(
                        &root.join(relative),
                        line,
                        character,
                        CompletionTrigger::Invoked,
                    )
                    .map_err(|error| CommandError::Failed(error.to_string()))?;
                Ok(json!({"requestId": id}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "language.restartServers",
                "Restart Language Servers",
                "Code",
                None,
                "{}",
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, _| {
                state.restart_language_servers();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "language.servers",
                "Language Server Status",
                "Code",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                Ok(json!({
                    "servers": state.lsp.servers(),
                    "missing": state.lsp.missing_servers().collect::<Vec<_>>(),
                    "notices": state.lsp.notices(),
                }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "language.dismissNotice",
                "Dismiss Language Server Notice",
                "Code",
                None,
                r#"{"id":"string"}"#,
            ),
            |state: &IdeState| !state.lsp.notices().is_empty(),
            |state: &mut IdeState, params| {
                state.lsp.dismiss_notice(&string_param(&params, "id")?);
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use serde_json::json;

    #[test]
    fn completion_is_enabled_for_java_files_and_explains_a_missing_server() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-lsp-cmd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".archipel")).unwrap();
        std::fs::write(root.join("App.java"), "class App {}\n").unwrap();
        std::fs::write(root.join("notes.txt"), "plain\n").unwrap();
        std::fs::write(
            root.join(".archipel/lsp.toml"),
            "[servers.jdtls]\ncommand = \"/nonexistent/jdtls\"\n",
        )
        .unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "notes.txt"}))
            .unwrap();
        assert!(!commands.is_enabled("language.complete", &state));
        commands
            .execute("editor.openFile", &mut state, json!({"path": "App.java"}))
            .unwrap();
        assert!(commands.is_enabled("language.complete", &state));
        let error = commands
            .execute("language.complete", &mut state, json!({}))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("Java language server `jdtls` was not found"),
            "{error}"
        );
        let status = commands
            .execute("language.servers", &mut state, json!({}))
            .unwrap();
        assert_eq!(status["missing"], json!(["jdtls"]));
        assert_eq!(status["notices"][0]["id"], "missing:jdtls");
        commands
            .execute(
                "language.dismissNotice",
                &mut state,
                json!({"id": "missing:jdtls"}),
            )
            .unwrap();
        assert!(state.lsp.notices().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}

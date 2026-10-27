//! PTY actions shared by terminal controls and command clients.
use crate::{IdeState, spec, string_param, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    for (id, label) in [
        ("terminal.resize", "Resize Terminal"),
        ("terminal.scroll", "Scroll Terminal"),
        ("terminal.paste", "Paste into Terminal"),
        ("terminal.copy", "Copy Terminal Selection"),
        ("terminal.clear", "Clear Terminal"),
    ] {
        let schema = match id {
            "terminal.resize" => r#"{"rows":"number","cols":"number"}"#,
            "terminal.scroll" => r#"{"lines":"number"}"#,
            "terminal.paste" => r#"{"text":"string"}"#,
            "terminal.copy" => {
                r#"{"startRow":"number","startCol":"number","endRow":"number","endCol":"number"}"#
            }
            _ => "{}",
        };
        registry.register(Command::new(spec(id, label, "Terminal", None, schema),
            |state: &IdeState| state.terminal().has_session(), move |state: &mut IdeState, params| {
                let service = state.terminal();
                let mut session = service.session_mut();
                let terminal = session.as_mut().ok_or_else(|| CommandError::Disabled(id.into()))?;
                match id {
                    "terminal.resize" => {
                        let rows = usize_param(&params, "rows")?;
                        let cols = usize_param(&params, "cols")?;
                        if !(1..=200).contains(&rows) || !(1..=500).contains(&cols) {
                            return Err(CommandError::InvalidParams("terminal dimensions must be 1..200 rows and 1..500 columns".into()));
                        }
                        terminal.resize(rows as u16, cols as u16).map_err(|e| CommandError::Failed(e.to_string()))?;
                    },
                    "terminal.scroll" => {
                        let lines = params.get("lines").and_then(Value::as_i64).ok_or_else(|| CommandError::InvalidParams("lines must be an integer".into()))?;
                        terminal.scroll(lines);
                    },
                    "terminal.paste" => terminal.paste(&string_param(&params, "text")?).map_err(|e| CommandError::Failed(e.to_string()))?,
                    "terminal.clear" => terminal.send("\u{c}").map_err(|e| CommandError::Failed(e.to_string()))?,
                    "terminal.copy" => {
                        let sr = usize_param(&params, "startRow")?;
                        let sc = usize_param(&params, "startCol")?;
                        let er = usize_param(&params, "endRow")?;
                        let ec = usize_param(&params, "endCol")?;
                        let (rows, cols) = terminal.screen().size();
                        if sr >= rows as usize || er >= rows as usize || sc > cols as usize || ec > cols as usize {
                            return Err(CommandError::InvalidParams("selection is outside the terminal screen".into()));
                        }
                        let (start, end) = if (sr, sc) <= (er, ec) { ((sr, sc), (er, ec)) } else { ((er, ec), (sr, sc)) };
                        return Ok(json!({"text":terminal.screen().contents_between(start.0 as u16,start.1 as u16,end.0 as u16,end.1 as u16)}));
                    },
                    _ => {},
                }
                Ok(Value::Null)
            })).expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "terminal.restart",
                "Restart Terminal",
                "Terminal",
                None,
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                let cwd = state
                    .project
                    .as_ref()
                    .map(|p| p.root().to_owned())
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                let terminal = archipel_ide_terminal::TerminalSession::spawn_shell(&cwd)
                    .map_err(|e| CommandError::Failed(e.to_string()))?;
                state.terminal().state_mut().set_session(terminal);
                state.tool_windows.show(crate::ToolWindow::Terminal);
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn terminal_commands_validate_dimensions_and_selection_boundaries() {
        let mut state = IdeState::default();
        state.terminal().state_mut().set_session(
            archipel_ide_terminal::TerminalSession::spawn(
                "/bin/sh",
                &["-c", "sleep 30"],
                std::path::Path::new("/tmp"),
            )
            .unwrap(),
        );
        let commands = crate::built_in_commands();
        assert!(
            commands
                .execute("terminal.resize", &mut state, json!({"rows":0,"cols":80}))
                .is_err()
        );
        commands
            .execute("terminal.resize", &mut state, json!({"rows":10,"cols":40}))
            .unwrap();
        assert!(
            commands
                .execute(
                    "terminal.copy",
                    &mut state,
                    json!({"startRow":0,"startCol":999,"endRow":0,"endCol":999})
                )
                .is_err()
        );
        assert!(
            commands
                .execute(
                    "terminal.copy",
                    &mut state,
                    json!({"startRow":0,"startCol":0,"endRow":0,"endCol":40})
                )
                .is_ok()
        );
    }
}

//! Multiple Terminal sessions (`Local ×` tabs with `+` and a session list). The sessions and
//! tabs live in the [`TerminalService`](crate::services::TerminalService); these are the
//! commands and the `IdeState` entry points on top of it.

use crate::{CommandError, IdeState, ToolWindow, spec, usize_param};
use archipel_ide_command::{Command, CommandRegistry};
use serde_json::{Value, json};

impl IdeState {
    /// Tab titles with the active one flagged; a lone shell shows as `Local`.
    pub fn terminal_tab_titles(&self) -> Vec<(String, bool)> {
        self.terminal().state().tab_titles()
    }

    /// Polls all sessions so background tabs keep receiving output; announces shells that
    /// exited. Returns whether a session exists.
    pub fn poll_terminals(&mut self) -> bool {
        self.terminal().poll()
    }

    pub fn new_terminal_tab(&mut self) -> Result<(), String> {
        let session = self.spawn_terminal_session(None)?;
        self.terminal().state_mut().push_tab(session);
        self.tool_windows.show(ToolWindow::Terminal);
        Ok(())
    }

    pub fn select_terminal_tab(&mut self, index: usize) -> Result<(), String> {
        self.terminal().state_mut().select_tab(index)
    }

    pub fn close_terminal_tab(&mut self, index: usize) -> Result<(), String> {
        self.terminal().state_mut().close_tab(index)
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "terminal.newTab",
                "New Terminal Session",
                "Terminal",
                None,
                "{}",
            ),
            |_: &IdeState| true,
            |state: &mut IdeState, _| {
                state.new_terminal_tab().map_err(CommandError::Failed)?;
                Ok(json!({ "tabs": state.terminal_tab_titles().len() }))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "terminal.selectTab",
                "Select Terminal Session",
                "Terminal",
                None,
                r#"{"index":"number"}"#,
            ),
            |state: &IdeState| state.terminal().has_session(),
            |state: &mut IdeState, params: Value| {
                let index = usize_param(&params, "index")?;
                state
                    .select_terminal_tab(index)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "terminal.closeTab",
                "Close Terminal Session",
                "Terminal",
                None,
                r#"{"index":"number"}"#,
            ),
            |state: &IdeState| state.terminal_tab_titles().len() > 1,
            |state: &mut IdeState, params: Value| {
                let index = usize_param(&params, "index")?;
                state
                    .close_terminal_tab(index)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

//! Multiple Terminal sessions (`Local ×` tabs with `+` and a session list).
//! The active session lives in [`IdeState::terminal`], so everything that
//! types into or reads the terminal keeps working; the others wait here and
//! are swapped in by [`IdeState::select_terminal_tab`].

use crate::{CommandError, IdeState, ToolWindow, spec, usize_param};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_terminal::TerminalSession;
use serde_json::{Value, json};

struct Tab {
    title: String,
    /// `None` for the active tab (its session is `IdeState::terminal`).
    session: Option<TerminalSession>,
}

#[derive(Default)]
pub struct TerminalTabs {
    tabs: Vec<Tab>,
    active: usize,
    created: usize,
}

impl TerminalTabs {
    fn title(&mut self) -> String {
        self.created += 1;
        match self.created {
            1 => "Local".to_owned(),
            n => format!("Local ({n})"),
        }
    }

    /// The first session of a Terminal opened without tabs.
    fn adopt(&mut self, has_terminal: bool) {
        if self.tabs.is_empty() && has_terminal {
            let title = self.title();
            self.tabs.push(Tab {
                title,
                session: None,
            });
            self.active = 0;
        }
    }
}

impl IdeState {
    /// Tab titles with the active one flagged; a lone shell shows as `Local`.
    pub fn terminal_tab_titles(&self) -> Vec<(String, bool)> {
        if self.terminal_tabs.tabs.is_empty() {
            return if self.terminal.is_some() {
                vec![("Local".into(), true)]
            } else {
                Vec::new()
            };
        }
        self.terminal_tabs
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| (tab.title.clone(), index == self.terminal_tabs.active))
            .collect()
    }

    /// Polls the sessions in background tabs so their output keeps arriving.
    pub fn poll_background_terminals(&mut self) {
        for tab in &mut self.terminal_tabs.tabs {
            if let Some(session) = tab.session.as_mut() {
                session.poll();
            }
        }
    }

    pub fn new_terminal_tab(&mut self) -> Result<(), String> {
        let session = self.spawn_terminal_session(None)?;
        self.terminal_tabs.adopt(self.terminal.is_some());
        let title = self.terminal_tabs.title();
        let tabs = &mut self.terminal_tabs;
        if let Some(previous) = tabs.tabs.get_mut(tabs.active) {
            previous.session = self.terminal.take();
        }
        tabs.tabs.push(Tab {
            title,
            session: None,
        });
        tabs.active = tabs.tabs.len() - 1;
        self.terminal = Some(session);
        self.tool_windows.show(ToolWindow::Terminal);
        Ok(())
    }

    pub fn select_terminal_tab(&mut self, index: usize) -> Result<(), String> {
        self.terminal_tabs.adopt(self.terminal.is_some());
        let tabs = &mut self.terminal_tabs;
        if index >= tabs.tabs.len() {
            return Err(format!("there is no terminal tab {index}"));
        }
        if index != tabs.active {
            let next = tabs.tabs[index].session.take();
            if let Some(previous) = tabs.tabs.get_mut(tabs.active) {
                previous.session = self.terminal.take();
            }
            self.terminal = next;
            tabs.active = index;
        }
        Ok(())
    }

    pub fn close_terminal_tab(&mut self, index: usize) -> Result<(), String> {
        self.terminal_tabs.adopt(self.terminal.is_some());
        let len = self.terminal_tabs.tabs.len();
        if index >= len {
            return Err(format!("there is no terminal tab {index}"));
        }
        if len < 2 {
            return Err("the last terminal tab cannot be closed".into());
        }
        let active = self.terminal_tabs.active;
        if index == active {
            // Show the neighbour, then drop the closed (now parked) shell.
            let neighbour = if index + 1 < len {
                index + 1
            } else {
                index - 1
            };
            self.select_terminal_tab(neighbour)?;
        }
        let tabs = &mut self.terminal_tabs;
        tabs.tabs.remove(index);
        if tabs.active > index {
            tabs.active -= 1;
        }
        Ok(())
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
            |state: &IdeState| state.terminal.is_some(),
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn shell(name: &str) -> TerminalSession {
        TerminalSession::spawn(
            "/bin/sh",
            &["-c", &format!("echo {name}; sleep 30")],
            std::path::Path::new("/tmp"),
        )
        .unwrap()
    }

    #[test]
    fn sessions_swap_in_and_close() {
        let mut state = IdeState {
            terminal: Some(shell("first")),
            ..Default::default()
        };
        assert_eq!(
            state.terminal_tab_titles(),
            vec![("Local".to_owned(), true)]
        );
        assert!(state.close_terminal_tab(0).is_err(), "the last tab stays");

        // `new_terminal_tab` spawns the user's shell; park a scripted one directly.
        state.terminal_tabs.adopt(true);
        state.terminal_tabs.tabs[0].session = state.terminal.take();
        state.terminal_tabs.tabs.push(Tab {
            title: "Local (2)".into(),
            session: None,
        });
        state.terminal = Some(shell("second"));
        state.terminal_tabs.active = 1;
        assert_eq!(state.terminal_tab_titles().len(), 2);

        state.select_terminal_tab(0).unwrap();
        assert!(state.terminal_tab_titles()[0].1);
        assert!(state.terminal.is_some());
        assert!(state.select_terminal_tab(5).is_err());

        state.close_terminal_tab(0).unwrap();
        assert_eq!(
            state.terminal_tab_titles(),
            vec![("Local (2)".to_owned(), true)]
        );
        assert!(state.terminal.is_some());
    }
}

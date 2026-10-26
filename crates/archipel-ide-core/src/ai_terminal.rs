//! Terminal natural language → command. A suggested command is only typed into
//! the shell, and only run, after the user confirms it.

use crate::{IdeState, spec, string_param};
use archipel_ide_ai::shell::{ShellContext, parse_command, request, risk};
use archipel_ide_ai::{AiError, AiUpdate};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};

/// Visible terminal lines sent as context (when file sharing is allowed).
const SCREEN_LINES: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellSuggestionState {
    Thinking,
    Ready {
        command: String,
        warning: Option<String>,
    },
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSuggestion {
    pub(crate) request: u64,
    pub query: String,
    pub state: ShellSuggestionState,
}

impl IdeState {
    pub fn ai_terminal_suggest(&mut self, query: &str) -> Result<u64, String> {
        if !self.ai.is_enabled() {
            return Err("AI is off: configure a provider first".into());
        }
        if query.trim().is_empty() {
            return Err("describe the command first".into());
        }
        let terminal = self.terminal.as_ref().ok_or("open the terminal first")?;
        let screen = self.ai.policy.share_file_contents.then(|| {
            let contents = terminal.screen().contents();
            let lines: Vec<&str> = contents.lines().collect();
            lines[lines.len().saturating_sub(SCREEN_LINES)..].join("\n")
        });
        let cwd = self
            .project
            .as_ref()
            .map(|p| p.root().display().to_string())
            .unwrap_or_default();
        let request = request(query.trim(), &ShellContext::detect(&cwd, screen));
        self.ai_terminal_dismiss();
        let id = self.ai.start(request)?;
        self.ai.shell = Some(ShellSuggestion {
            request: id,
            query: query.trim().to_owned(),
            state: ShellSuggestionState::Thinking,
        });
        Ok(id)
    }

    pub fn ai_terminal_dismiss(&mut self) {
        if let Some(suggestion) = self.ai.shell.take()
            && suggestion.state == ShellSuggestionState::Thinking
        {
            self.ai.service.cancel(suggestion.request);
        }
    }

    pub(crate) fn ai_terminal_update(&mut self, id: u64, update: &AiUpdate) -> bool {
        let Some(suggestion) = self.ai.shell.as_mut().filter(|s| s.request == id) else {
            return false;
        };
        if let AiUpdate::Finished(result) = update {
            suggestion.state = match result {
                Ok(response) => match parse_command(&response.text()) {
                    Ok(command) => ShellSuggestionState::Ready {
                        warning: risk(&command).map(str::to_owned),
                        command,
                    },
                    Err(reason) => ShellSuggestionState::Failed(reason),
                },
                Err(AiError::Cancelled) => {
                    self.ai.shell = None;
                    return true;
                }
                Err(error) => ShellSuggestionState::Failed(error.to_string()),
            };
        }
        true
    }

    /// Types the confirmed command into the shell, pressing Enter when `run`.
    pub fn ai_terminal_accept(&mut self, run: bool) -> Result<(), String> {
        let Some(ShellSuggestion {
            state: ShellSuggestionState::Ready { command, .. },
            ..
        }) = self.ai.shell.clone()
        else {
            return Err("no suggested command".into());
        };
        let terminal = self.terminal.as_mut().ok_or("open the terminal first")?;
        // Typed like keystrokes: `parse_command` guarantees a single line without
        // control characters.
        let enter = if run { "\r" } else { "" };
        terminal
            .send(&format!("{command}{enter}"))
            .map_err(|e| e.to_string())?;
        self.ai.shell = None;
        Ok(())
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let ready = |state: &IdeState| {
        matches!(
            state.ai.shell,
            Some(ShellSuggestion {
                state: ShellSuggestionState::Ready { .. },
                ..
            })
        ) && state.terminal.is_some()
    };
    let commands = vec![
        Command::new(
            spec(
                "ai.terminal.suggest",
                "AI: Suggest Terminal Command",
                "AI",
                None,
                r#"{"text":"string"}"#,
            ),
            |state: &IdeState| state.ai.is_enabled() && state.terminal.is_some(),
            |state: &mut IdeState, params| {
                let text = string_param(&params, "text")?;
                let id = state
                    .ai_terminal_suggest(&text)
                    .map_err(CommandError::Failed)?;
                Ok(json!({"requestId": id}))
            },
        ),
        Command::new(
            spec(
                "ai.terminal.run",
                "AI: Run Suggested Command",
                "AI",
                None,
                "{}",
            ),
            ready,
            |state: &mut IdeState, _| {
                state
                    .ai_terminal_accept(true)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec(
                "ai.terminal.insert",
                "AI: Insert Suggested Command",
                "AI",
                None,
                "{}",
            ),
            ready,
            |state: &mut IdeState, _| {
                state
                    .ai_terminal_accept(false)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec(
                "ai.terminal.dismiss",
                "AI: Dismiss Suggested Command",
                "AI",
                None,
                "{}",
            ),
            |state: &IdeState| state.ai.shell.is_some(),
            |state: &mut IdeState, _| {
                state.ai_terminal_dismiss();
                Ok(Value::Null)
            },
        ),
    ];
    for command in commands {
        registry.register(command).expect("unique built-in id");
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::ShellSuggestionState;
    use crate::{IdeState, built_in_commands};
    use archipel_ide_ai::{MockProvider, MockReply};
    use serde_json::{Value, json};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn open(name: &str, replies: Vec<MockReply>) -> (IdeState, Arc<MockProvider>) {
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-ai-term-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut state = IdeState::default();
        state.init_ai(Some(root.join(".config")));
        state.open_project(&root).unwrap();
        state.open_terminal().unwrap();
        // Let the shell draw its prompt (and enable bracketed paste, as in the IDE).
        let deadline = Instant::now() + Duration::from_secs(5);
        while state
            .terminal
            .as_mut()
            .is_some_and(|t| t.poll().trim().is_empty())
        {
            assert!(Instant::now() < deadline, "shell did not start");
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_millis(200));
        let provider = Arc::new(MockProvider::new(replies));
        state.ai.use_provider(provider.clone());
        (state, provider)
    }

    fn settle(state: &mut IdeState) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while matches!(
            state.ai.shell.as_ref().map(|s| &s.state),
            Some(ShellSuggestionState::Thinking)
        ) {
            assert!(Instant::now() < deadline);
            state.poll_ai();
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn screen_contains(state: &mut IdeState, needle: &str, wait: Duration) -> bool {
        let deadline = Instant::now() + wait;
        while Instant::now() < deadline {
            if let Some(terminal) = state.terminal.as_mut() {
                terminal.poll();
                if terminal.screen().contents().contains(needle) {
                    return true;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    #[test]
    fn suggests_then_runs_only_after_confirmation() {
        let (mut state, provider) = open(
            "run",
            vec![MockReply::text("```sh\necho archipel-ide-$((40+2))\n```")],
        );
        let commands = built_in_commands();
        commands
            .execute(
                "ai.terminal.suggest",
                &mut state,
                json!({"text": "print the answer"}),
            )
            .unwrap();
        settle(&mut state);
        let sent = provider.requests()[0].messages[0].text();
        assert!(sent.contains("Request: print the answer"));
        assert!(sent.contains("Shell: "));
        assert!(matches!(
            &state.ai.shell.as_ref().unwrap().state,
            ShellSuggestionState::Ready { command, warning: None } if command == "echo archipel-ide-$((40+2))"
        ));
        assert!(
            !screen_contains(&mut state, "archipel-ide-42", Duration::from_millis(300)),
            "nothing runs before confirmation"
        );
        commands
            .execute("ai.terminal.run", &mut state, Value::Null)
            .unwrap();
        assert!(screen_contains(
            &mut state,
            "archipel-ide-42",
            Duration::from_secs(5)
        ));
        assert!(state.ai.shell.is_none());
    }

    #[test]
    fn risky_and_unsure_answers_are_reported() {
        let (mut state, _) = open(
            "risk",
            vec![
                MockReply::text("rm -rf build"),
                MockReply::text("UNSURE: which files?"),
            ],
        );
        let commands = built_in_commands();
        commands
            .execute("ai.terminal.suggest", &mut state, json!({"text": "clean"}))
            .unwrap();
        settle(&mut state);
        assert!(matches!(
            &state.ai.shell.as_ref().unwrap().state,
            ShellSuggestionState::Ready { warning: Some(w), .. } if w == "deletes files"
        ));
        commands
            .execute(
                "ai.terminal.suggest",
                &mut state,
                json!({"text": "delete stuff"}),
            )
            .unwrap();
        settle(&mut state);
        assert_eq!(
            state.ai.shell.as_ref().unwrap().state,
            ShellSuggestionState::Failed("which files?".into())
        );
        assert!(!commands.is_enabled("ai.terminal.run", &state));
        commands
            .execute("ai.terminal.dismiss", &mut state, Value::Null)
            .unwrap();
        assert!(state.ai.shell.is_none());
    }
}

//! Inline AI code completion (ghost text): request, validation against the
//! document as the user keeps typing, and acceptance through the edit pipeline.

use crate::ai_commands::language_of;
use crate::{IdeState, spec};
use archipel_ide_ai::completion::{CompletionContext, clean, next_word};
use archipel_ide_ai::{AiError, ChatResponse};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};
use std::path::PathBuf;

/// A suggestion shown as ghost text at `position` in `path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSuggestion {
    pub path: PathBuf,
    /// Char offset where the not-yet-typed suggestion text starts.
    pub position: usize,
    pub text: String,
}

/// A completion request in flight, tied to the document state it was made for.
#[derive(Debug, Clone)]
pub(crate) struct InlineRequest {
    pub id: u64,
    path: PathBuf,
    revision: u64,
    caret: usize,
    context: CompletionContext,
}

impl IdeState {
    /// Active document path, revision and single caret, if any.
    fn inline_target(&self) -> Option<(PathBuf, u64, usize)> {
        let document = self.documents.get(self.active_document?)?;
        let selections = document.selections.selections();
        let primary = document.selections.primary();
        (selections.len() == 1 && primary.is_caret()).then(|| {
            (
                document.path.clone(),
                document.text.revision(),
                primary.head,
            )
        })
    }

    /// Whether inline completion may run now (AI on, setting on, file sharing
    /// allowed, no LSP completion popup).
    pub fn ai_inline_available(&self) -> bool {
        self.ai.is_enabled()
            && self.ai.settings.inline_completion
            && self.ai.policy.share_file_contents
            && !self
                .completion
                .as_ref()
                .is_some_and(|session| session.visible())
            && self.inline_target().is_some()
    }

    /// Requests a suggestion for the caret; `Ok(None)` when the position is not
    /// eligible (e.g. the caret is in the middle of a line).
    pub fn ai_inline_request(&mut self) -> Result<Option<u64>, String> {
        if !self.ai_inline_available() {
            return Err("inline completion is not available here".into());
        }
        let (path, revision, caret) = self.inline_target().ok_or("no caret")?;
        let document = &self.documents[self.active_document.ok_or("no active document")?];
        let context = CompletionContext::new(
            &path.display().to_string(),
            &language_of(&path),
            &document.text.text(),
            caret,
        );
        self.ai_inline_cancel();
        if !context.is_eligible() {
            return Ok(None);
        }
        let id = self.ai.start(context.request())?;
        self.ai.inline_request = Some(InlineRequest {
            id,
            path,
            revision,
            caret,
            context,
        });
        Ok(Some(id))
    }

    pub(crate) fn ai_inline_cancel(&mut self) {
        if let Some(request) = self.ai.inline_request.take() {
            self.ai.service.cancel(request.id);
        }
    }

    pub(crate) fn ai_inline_finished(&mut self, result: Result<ChatResponse, AiError>) {
        let Some(request) = self.ai.inline_request.take() else {
            return;
        };
        let current = self.inline_target();
        if current != Some((request.path.clone(), request.revision, request.caret)) {
            return;
        }
        match result {
            Ok(response) => {
                self.ai.inline =
                    clean(&response.text(), &request.context).map(|text| InlineSuggestion {
                        path: request.path,
                        position: request.caret,
                        text,
                    });
            }
            Err(AiError::Cancelled) => {}
            Err(error) => self.status = format!("AI completion: {error}"),
        }
    }

    /// Keeps the suggestion consistent with the document: typing characters that
    /// match it consumes them; anything else dismisses it. Stale requests are
    /// cancelled. Call after every edit or caret move.
    pub fn ai_inline_sync(&mut self) {
        let target = self.inline_target();
        if let Some(request) = &self.ai.inline_request
            && target != Some((request.path.clone(), request.revision, request.caret))
        {
            self.ai_inline_cancel();
        }
        let Some(suggestion) = self.ai.inline.as_ref() else {
            return;
        };
        let keep = (|| {
            let (path, _, caret) = target?;
            if path != suggestion.path
                || caret < suggestion.position
                || self
                    .completion
                    .as_ref()
                    .is_some_and(|session| session.visible())
            {
                return None;
            }
            let document = self.documents.get(self.active_document?)?;
            let typed = document.text.slice(suggestion.position, caret).ok()?;
            let rest = suggestion.text.strip_prefix(typed.as_str())?;
            Some((caret, rest.to_owned()))
        })();
        self.ai.inline = match keep {
            Some((_, rest)) if rest.is_empty() => None,
            Some((caret, rest)) => Some(InlineSuggestion {
                path: suggestion.path.clone(),
                position: caret,
                text: rest,
            }),
            None => None,
        };
    }

    /// Inserts the whole suggestion, or only its next word.
    pub fn ai_inline_accept(&mut self, word_only: bool) -> Result<(), String> {
        self.ai_inline_sync();
        let suggestion = self.ai.inline.clone().ok_or("no inline suggestion")?;
        let text = if word_only {
            next_word(&suggestion.text).0.to_owned()
        } else {
            suggestion.text.clone()
        };
        self.insert_active_text(suggestion.position, &text)?;
        // The inserted text matches the suggestion, so syncing keeps the remainder.
        self.ai_inline_sync();
        Ok(())
    }

    pub fn ai_inline_dismiss(&mut self) {
        self.ai_inline_cancel();
        self.ai.inline = None;
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let failed = |error: String| CommandError::Failed(error);
    let has = |state: &IdeState| state.ai.inline.is_some();
    let commands = vec![
        Command::new(
            spec(
                "ai.inline.request",
                "AI: Suggest Inline Completion",
                "AI",
                None,
                "{}",
            ),
            |state: &IdeState| state.ai_inline_available(),
            move |state: &mut IdeState, _| {
                let id = state.ai_inline_request().map_err(failed)?;
                Ok(json!({"requestId": id}))
            },
        ),
        Command::new(
            spec(
                "ai.inline.accept",
                "AI: Accept Inline Completion",
                "AI",
                Some("⇥"),
                "{}",
            ),
            has,
            move |state: &mut IdeState, _| {
                state.ai_inline_accept(false).map_err(failed)?;
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec(
                "ai.inline.acceptWord",
                "AI: Accept Next Word of Inline Completion",
                "AI",
                Some("⌥→"),
                "{}",
            ),
            has,
            move |state: &mut IdeState, _| {
                state.ai_inline_accept(true).map_err(failed)?;
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec(
                "ai.inline.dismiss",
                "AI: Dismiss Inline Completion",
                "AI",
                Some("⎋"),
                "{}",
            ),
            |state: &IdeState| state.ai.inline.is_some() || state.ai.inline_request.is_some(),
            |state: &mut IdeState, _| {
                state.ai_inline_dismiss();
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec(
                "ai.inline.setEnabled",
                "AI: Enable or Disable Inline Completion",
                "AI",
                None,
                r#"{"enabled":"bool"}"#,
            ),
            |_| true,
            move |state: &mut IdeState, params| {
                let enabled = params
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| {
                        CommandError::InvalidParams("expected bool field `enabled`".into())
                    })?;
                state.ai.settings.inline_completion = enabled;
                if !enabled {
                    state.ai_inline_dismiss();
                }
                state.save_ai_settings().map_err(failed)?;
                Ok(json!({"enabled": enabled}))
            },
        ),
    ];
    for command in commands {
        registry.register(command).expect("unique built-in id");
    }
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use archipel_ide_ai::{MockProvider, MockReply};
    use archipel_ide_command::CommandRegistry;
    use archipel_ide_text::Selection;
    use serde_json::{Value, json};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    const SOURCE: &str = "fn total(items: &[u32]) -> u32 {\n    let sum = \n}\n";
    /// Caret after `let sum = ` (char offset).
    const CARET: usize = 47;

    fn open(
        name: &str,
        replies: Vec<MockReply>,
    ) -> (IdeState, CommandRegistry<IdeState>, Arc<MockProvider>) {
        let root: PathBuf =
            std::env::temp_dir().join(format!("archipel-ide-inline-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/lib.rs"), SOURCE).unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        state.init_ai(Some(root.join("config")));
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "src/lib.rs"}))
            .unwrap();
        let provider = Arc::new(MockProvider::new(replies));
        state.ai.use_provider(provider.clone());
        state.documents[0]
            .selections
            .set_single(Selection::caret(CARET));
        (state, commands, provider)
    }

    fn wait(state: &mut IdeState) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while state.ai.inline_request.is_some() {
            assert!(Instant::now() < deadline, "completion did not finish");
            state.poll_ai();
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn text(state: &IdeState) -> String {
        state.documents[0].text.text()
    }

    #[test]
    fn suggests_accepts_and_undoes_in_one_step() {
        let (mut state, commands, provider) = open(
            "accept",
            vec![MockReply::text("```rust\nitems.iter().sum();\n```")],
        );
        assert_eq!(&SOURCE[CARET - 10..CARET], "let sum = ");
        commands
            .execute("ai.inline.request", &mut state, Value::Null)
            .unwrap();
        wait(&mut state);
        let request = &provider.requests()[0];
        assert_eq!(request.role, archipel_ide_ai::ModelRole::Completion);
        assert!(
            request.messages[0]
                .text()
                .contains("let sum = <cursor/>\n}")
        );
        assert_eq!(
            state.ai.inline.as_ref().unwrap().text,
            "items.iter().sum();"
        );
        commands
            .execute("ai.inline.accept", &mut state, Value::Null)
            .unwrap();
        assert!(text(&state).contains("let sum = items.iter().sum();\n}"));
        assert!(state.ai.inline.is_none());
        commands
            .execute("editor.undo", &mut state, Value::Null)
            .unwrap();
        assert_eq!(text(&state), SOURCE);
    }

    #[test]
    fn typing_matching_text_consumes_and_other_text_dismisses() {
        let (mut state, commands, _) = open("typing", vec![MockReply::text("items.len()")]);
        commands
            .execute("ai.inline.request", &mut state, Value::Null)
            .unwrap();
        wait(&mut state);
        state.insert_active_text(CARET, "it").unwrap();
        state.ai_inline_sync();
        let suggestion = state.ai.inline.clone().unwrap();
        assert_eq!(
            (suggestion.position, suggestion.text.as_str()),
            (CARET + 2, "ems.len()")
        );
        commands
            .execute("ai.inline.acceptWord", &mut state, Value::Null)
            .unwrap();
        assert!(text(&state).contains("let sum = items\n"));
        assert_eq!(state.ai.inline.as_ref().unwrap().text, ".len()");
        state.insert_active_text(CARET + 5, "x").unwrap();
        state.ai_inline_sync();
        assert!(state.ai.inline.is_none());
    }

    #[test]
    fn stale_requests_are_dropped_and_ineligible_positions_skipped() {
        let mut slow = MockReply::text("items.len()");
        slow.delay = Duration::from_millis(20);
        let (mut state, commands, provider) = open("stale", vec![slow]);
        commands
            .execute("ai.inline.request", &mut state, Value::Null)
            .unwrap();
        state.insert_active_text(CARET, "x").unwrap();
        state.ai_inline_sync();
        assert!(
            state.ai.inline_request.is_none(),
            "typing cancels the request"
        );
        std::thread::sleep(Duration::from_millis(100));
        state.poll_ai();
        assert!(state.ai.inline.is_none());
        // Mid-line carets are not eligible and send nothing.
        state.documents[0]
            .selections
            .set_single(Selection::caret(3));
        let result = commands
            .execute("ai.inline.request", &mut state, Value::Null)
            .unwrap();
        assert_eq!(result["requestId"], Value::Null);
        assert_eq!(provider.requests().len(), 1);
        commands
            .execute(
                "ai.inline.setEnabled",
                &mut state,
                json!({"enabled": false}),
            )
            .unwrap();
        assert!(!commands.is_enabled("ai.inline.request", &state));
    }

    #[test]
    fn dismiss_clears_the_suggestion() {
        let (mut state, commands, _) = open("dismiss", vec![MockReply::text("0")]);
        commands
            .execute("ai.inline.request", &mut state, Value::Null)
            .unwrap();
        wait(&mut state);
        assert!(state.ai.inline.is_some());
        commands
            .execute("ai.inline.dismiss", &mut state, Value::Null)
            .unwrap();
        assert!(state.ai.inline.is_none());
        assert!(
            commands
                .execute("ai.inline.accept", &mut state, Value::Null)
                .is_err()
        );
    }
}

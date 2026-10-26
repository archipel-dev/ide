//! AI for version control: commit message generation from the changes selected
//! in the Commit tool window.

use crate::{IdeState, spec, string_array_param};
use archipel_ide_ai::commit::{clean_message, request};
use archipel_ide_ai::{AiError, AiUpdate, ChatResponse, StreamEvent};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use serde_json::{Value, json};
use std::path::PathBuf;

/// Recent subjects sent so the message follows the repository's style.
const RECENT_SUBJECTS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftState {
    Streaming,
    Done,
}

/// A commit message being generated (streamed into the Commit window).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitDraft {
    pub(crate) request: u64,
    pub text: String,
    pub state: DraftState,
}

impl IdeState {
    pub fn ai_generate_commit_message(&mut self, paths: &[PathBuf]) -> Result<u64, String> {
        if !self.ai.is_enabled() {
            return Err("AI is off: configure a provider first".into());
        }
        if !self.ai.policy.share_file_contents {
            return Err("this project does not share file contents with AI".into());
        }
        if paths.is_empty() {
            return Err("select changes to commit first".into());
        }
        let git = self.git.as_ref().ok_or("project is not a Git repository")?;
        let diff = git.diff(paths).map_err(|e| e.to_string())?;
        if diff.trim().is_empty() {
            return Err("the selected files have no changes to describe".into());
        }
        let subjects = git.recent_subjects(RECENT_SUBJECTS).unwrap_or_default();
        let request = request(&git.branch, &subjects, &diff);
        self.ai_stop_commit_message();
        let id = self.ai.start(request)?;
        self.ai.commit_draft = Some(CommitDraft {
            request: id,
            text: String::new(),
            state: DraftState::Streaming,
        });
        self.status = "AI: generating commit message…".into();
        Ok(id)
    }

    pub fn ai_stop_commit_message(&mut self) {
        if let Some(draft) = self.ai.commit_draft.take()
            && draft.state == DraftState::Streaming
        {
            self.ai.service.cancel(draft.request);
        }
    }

    /// Routes a provider update; returns false when it belongs to another request.
    pub(crate) fn ai_commit_update(&mut self, id: u64, update: &AiUpdate) -> bool {
        let Some(draft) = self.ai.commit_draft.as_mut().filter(|d| d.request == id) else {
            return false;
        };
        match update {
            AiUpdate::Event(StreamEvent::TextDelta(text)) => draft.text.push_str(text),
            AiUpdate::Event(_) => {}
            AiUpdate::Finished(result) => self.ai_commit_finished(result.clone()),
        }
        true
    }

    fn ai_commit_finished(&mut self, result: Result<ChatResponse, AiError>) {
        match result {
            Ok(response) => {
                let text = clean_message(&response.text());
                if let Some(draft) = self.ai.commit_draft.as_mut() {
                    draft.text = text;
                    draft.state = DraftState::Done;
                }
                self.status = "AI: commit message ready".into();
            }
            Err(AiError::Cancelled) => self.ai.commit_draft = None,
            Err(error) => {
                self.ai.commit_draft = None;
                self.status = format!("AI: {error}");
            }
        }
    }

    /// Text for the commit message field while generating; a finished draft is
    /// handed over once and cleared.
    pub fn take_commit_draft(&mut self) -> Option<(String, bool)> {
        let draft = self.ai.commit_draft.as_ref()?;
        let streaming = draft.state == DraftState::Streaming;
        let text = draft.text.clone();
        if !streaming {
            self.ai.commit_draft = None;
        }
        Some((text, streaming))
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "ai.vcs.generateCommitMessage",
                "AI: Generate Commit Message",
                "AI",
                None,
                r#"{"paths":["string"]}"#,
            ),
            |state: &IdeState| state.ai.is_enabled() && state.git.is_some(),
            |state: &mut IdeState, params| {
                let paths: Vec<PathBuf> = string_array_param(&params, "paths")?
                    .into_iter()
                    .map(PathBuf::from)
                    .collect();
                let id = state
                    .ai_generate_commit_message(&paths)
                    .map_err(CommandError::Failed)?;
                Ok(json!({"requestId": id}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "ai.vcs.stopCommitMessage",
                "AI: Stop Generating Commit Message",
                "AI",
                None,
                "{}",
            ),
            |state: &IdeState| state.ai.commit_draft.is_some(),
            |state: &mut IdeState, _| {
                state.ai_stop_commit_message();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use archipel_ide_ai::{AiError, MockProvider, MockReply};
    use serde_json::{Value, json};
    use std::path::Path;
    use std::process::Command;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn git(root: &Path, args: &[&str]) {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }

    fn repository(name: &str, replies: Vec<MockReply>) -> (IdeState, Arc<MockProvider>) {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-ai-vcs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "test@example.invalid"]);
        git(&root, &["config", "user.name", "Archipel IDE Test"]);
        std::fs::write(root.join("lib.rs"), "fn a() {}\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "feat: initial"]);
        std::fs::write(root.join("lib.rs"), "fn a() -> u8 { 1 }\n").unwrap();
        let mut state = IdeState::default();
        state.init_ai(Some(root.join(".config")));
        state.open_project(&root).unwrap();
        let provider = Arc::new(MockProvider::new(replies));
        state.ai.use_provider(provider.clone());
        (state, provider)
    }

    fn finish(state: &mut IdeState) -> Option<(String, bool)> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            state.poll_ai();
            match state.take_commit_draft() {
                Some((text, false)) => return Some((text, false)),
                None => return None,
                Some(_) => {}
            }
            assert!(Instant::now() < deadline, "generation did not finish");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn generates_a_cleaned_message_from_the_selected_diff() {
        let (mut state, provider) = repository(
            "generate",
            vec![MockReply::text("```\nfeat: return a value from a\n```")],
        );
        let commands = built_in_commands();
        commands
            .execute(
                "ai.vcs.generateCommitMessage",
                &mut state,
                json!({"paths": ["lib.rs"]}),
            )
            .unwrap();
        assert_eq!(
            finish(&mut state),
            Some(("feat: return a value from a".into(), false))
        );
        assert!(state.take_commit_draft().is_none(), "handed over once");
        let sent = provider.requests()[0].messages[0].text();
        assert!(sent.contains("-fn a() {}\n+fn a() -> u8 { 1 }"));
        assert!(sent.contains("Recent commit subjects:\nfeat: initial"));
        assert_eq!(
            provider.requests()[0].role,
            archipel_ide_ai::ModelRole::Completion
        );
    }

    #[test]
    fn refuses_empty_selections_and_reports_errors() {
        let (mut state, _) = repository(
            "errors",
            vec![MockReply::error(AiError::Overloaded("busy".into()))],
        );
        let commands = built_in_commands();
        assert!(
            commands
                .execute(
                    "ai.vcs.generateCommitMessage",
                    &mut state,
                    json!({"paths": []})
                )
                .is_err()
        );
        commands
            .execute(
                "ai.vcs.generateCommitMessage",
                &mut state,
                json!({"paths": ["lib.rs"]}),
            )
            .unwrap();
        assert_eq!(finish(&mut state), None);
        assert!(state.status.contains("overloaded"));
        assert!(!commands.is_enabled("ai.vcs.stopCommitMessage", &state));
        let _ = commands.execute("ai.vcs.stopCommitMessage", &mut state, Value::Null);
    }

    #[test]
    fn stop_discards_the_draft() {
        let mut slow = MockReply::text("feat: one two three four");
        slow.delay = Duration::from_millis(30);
        let (mut state, _) = repository("stop", vec![slow]);
        let commands = built_in_commands();
        commands
            .execute(
                "ai.vcs.generateCommitMessage",
                &mut state,
                json!({"paths": ["lib.rs"]}),
            )
            .unwrap();
        commands
            .execute("ai.vcs.stopCommitMessage", &mut state, Value::Null)
            .unwrap();
        state.poll_ai();
        assert!(state.take_commit_draft().is_none());
    }
}

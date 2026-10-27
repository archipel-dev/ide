//! AI actions on the editor and on failed builds: Explain, Refactor, Generate
//! tests/docs, Find problems and Fix with AI. Answers appear in AI Chat; actions
//! that rewrite code open the diff preview as soon as the answer completes.

use crate::{IdeState, ToolWindow, spec, usize_param};
use archipel_ide_ai::actions::{AiAction, first_error_location, output_tail};
use archipel_ide_ai::chat::{ContextItem, ContextKind};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_runner::TaskState;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Build output lines attached to "Fix Build with AI".
const OUTPUT_LINES: usize = 200;

impl IdeState {
    /// Problems in the active file as `(1-based line, message)`, optionally only
    /// those on `line`. Syntax errors and language-server diagnostics are merged.
    pub fn ai_problems(&self, line: Option<usize>) -> Vec<(usize, String)> {
        let Some(document) = self.active_document.and_then(|i| self.documents.get(i)) else {
            return Vec::new();
        };
        let mut problems: Vec<(usize, String)> = document
            .analysis
            .diagnostics
            .iter()
            .map(|d| (d.row, d.message.clone()))
            .chain(
                self.active_lsp_diagnostics()
                    .into_iter()
                    .map(|d| (d.start_line + 1, d.message)),
            )
            .filter(|(row, _)| line.is_none_or(|line| *row == line))
            .collect();
        problems.sort();
        problems.dedup();
        problems
    }

    fn ai_action_ready(&self) -> bool {
        self.project.is_some() && self.ai.is_enabled() && !self.ai.chat.is_streaming()
    }

    /// The failed build/run command, if the last task failed.
    fn failed_task(&self) -> Option<String> {
        match self.execution().runner().state() {
            TaskState::Exited(code) if *code != 0 => self.execution().runner().plan().map(|plan| {
                format!("{} {}", plan.program, plan.args.join(" "))
                    .trim()
                    .to_owned()
            }),
            _ => None,
        }
    }

    /// Resolves a path reported by a compiler to a project-relative file.
    fn project_file(&self, reported: &str) -> Option<PathBuf> {
        let root = self.project.as_ref()?.root();
        let path = Path::new(reported);
        let relative = if path.is_absolute() {
            path.strip_prefix(root).ok()?.to_owned()
        } else {
            path.to_owned()
        };
        root.join(&relative).is_file().then_some(relative)
    }

    pub fn ai_run_action(&mut self, action: AiAction) -> Result<u64, String> {
        if !self.ai_action_ready() {
            return Err("AI is off or busy".into());
        }
        let context = match &action {
            AiAction::FixBuild { command } => {
                // Output can trail the exit status; collect what has arrived.
                self.poll_runner();
                if !self.ai.policy.share_file_contents {
                    return Err("this project does not share file contents with AI".into());
                }
                let output: String = self
                    .execution()
                    .runner()
                    .output()
                    .iter()
                    .map(|c| c.text.as_str())
                    .collect();
                let mut items = vec![ContextItem {
                    kind: ContextKind::Output,
                    path: PathBuf::from(command),
                    language: "text".into(),
                    text: output_tail(&output, OUTPUT_LINES),
                    range: None,
                    first_line: 1,
                }];
                // Attach (and open) the file of the first error so a fix can be applied.
                if let Some((path, _)) = first_error_location(&output)
                    && let Some(relative) = self.project_file(&path)
                    && self.open_file(&relative).is_ok()
                {
                    items.extend(self.ai_context(true, false)?);
                }
                items
            }
            _ => {
                if self.active_document.is_none() {
                    return Err("open a file first".into());
                }
                self.ai_context(true, true)?
            }
        };
        let request = self.ai.chat.begin(&action.prompt(), context)?;
        let id = match self.ai.start(request) {
            Ok(id) => id,
            Err(error) => {
                self.ai
                    .chat
                    .finish(Err(archipel_ide_ai::AiError::NotConfigured(error.clone())));
                return Err(error);
            }
        };
        self.ai.chat.request = Some(id);
        self.ai.auto_preview = action.edits_code().then(|| self.ai.chat.turns.len() - 1);
        self.tool_windows.show(ToolWindow::AiChat);
        self.status = format!("AI: {}", action.label());
        Ok(id)
    }

    /// Opens the diff preview for an edit action whose answer just completed.
    pub(crate) fn ai_action_finished(&mut self) {
        let Some(turn) = self.ai.auto_preview.take() else {
            return;
        };
        if self
            .ai
            .chat
            .turns
            .get(turn)
            .is_some_and(|t| t.state == archipel_ide_ai::chat::TurnState::Done)
        {
            // An answer without code (e.g. "nothing to fix") simply stays in the chat.
            let _ = self.ai_preview_apply(turn, 0);
        }
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let editor_ready =
        |state: &IdeState| state.ai_action_ready() && state.active_document.is_some();
    let simple = [
        AiAction::Explain,
        AiAction::Refactor,
        AiAction::GenerateTests,
        AiAction::GenerateDocs,
        AiAction::FindProblems,
    ];
    for action in simple {
        let id = format!("ai.action.{}", action.id());
        registry
            .register(Command::new(
                spec(&id, &format!("AI: {}", action.label()), "AI", None, "{}"),
                editor_ready,
                move |state: &mut IdeState, _| {
                    let id = state
                        .ai_run_action(action.clone())
                        .map_err(CommandError::Failed)?;
                    Ok(json!({"requestId": id}))
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "ai.action.fixProblems",
                "AI: Fix with AI",
                "AI",
                None,
                r#"{"line":"number?"}"#,
            ),
            move |state: &IdeState| editor_ready(state) && !state.ai_problems(None).is_empty(),
            |state: &mut IdeState, params| {
                let line = match params.get("line") {
                    None | Some(Value::Null) => None,
                    Some(_) => Some(usize_param(&params, "line")?),
                };
                // Problems on the requested line, else every problem in the file.
                let mut problems = state.ai_problems(line);
                if problems.is_empty() {
                    problems = state.ai_problems(None);
                }
                if problems.is_empty() {
                    return Err(CommandError::Failed("no problems to fix".into()));
                }
                let listed = problems
                    .into_iter()
                    .map(|(line, message)| format!("line {line}: {message}"))
                    .collect();
                let id = state
                    .ai_run_action(AiAction::FixProblems(listed))
                    .map_err(CommandError::Failed)?;
                Ok(json!({"requestId": id}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "ai.action.fixBuild",
                "AI: Fix Build with AI",
                "AI",
                None,
                "{}",
            ),
            |state: &IdeState| state.ai_action_ready() && state.failed_task().is_some(),
            |state: &mut IdeState, _| {
                let command = state
                    .failed_task()
                    .ok_or_else(|| CommandError::Disabled("ai.action.fixBuild".into()))?;
                let id = state
                    .ai_run_action(AiAction::FixBuild { command })
                    .map_err(CommandError::Failed)?;
                Ok(json!({"requestId": id}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, ToolWindow, built_in_commands};
    use archipel_ide_ai::{MockProvider, MockReply};
    use archipel_ide_command::CommandRegistry;
    use archipel_ide_runner::TaskPlan;
    use archipel_ide_text::Selection;
    use serde_json::{Value, json};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn open(
        name: &str,
        source: &str,
        replies: Vec<MockReply>,
    ) -> (
        IdeState,
        CommandRegistry<IdeState>,
        Arc<MockProvider>,
        PathBuf,
    ) {
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-actions-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/lib.rs"), source).unwrap();
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
        (state, commands, provider, root)
    }

    fn wait(state: &mut IdeState) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while state.ai.chat.is_streaming() {
            assert!(Instant::now() < deadline, "answer did not finish");
            state.poll_ai();
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn explain_uses_the_selection_and_shows_the_chat() {
        let (mut state, commands, provider, _) = open(
            "explain",
            "fn a() {}\nfn b() {}\n",
            vec![MockReply::text("It is empty.")],
        );
        state.tool_windows.hide_all();
        state.documents[0].selections.set_single(Selection {
            anchor: 10,
            head: 19,
        });
        commands
            .execute("ai.action.explain", &mut state, Value::Null)
            .unwrap();
        assert!(state.tool_windows.is_visible(ToolWindow::AiChat));
        wait(&mut state);
        let sent = provider.requests()[0].messages[0].text();
        assert!(sent.contains("lines=\"2-2\">\nfn b() {}"));
        assert!(sent.ends_with("Explain what the attached code does, step by step."));
        assert!(state.ai.pending_apply.is_none(), "explain does not edit");
    }

    #[test]
    fn refactor_opens_the_diff_preview_when_done() {
        let reply = MockReply::text("```rust\nfn a() -> () {}\n```");
        let (mut state, commands, _, _) = open("refactor", "fn a() {}\n", vec![reply]);
        commands
            .execute("ai.action.refactor", &mut state, Value::Null)
            .unwrap();
        wait(&mut state);
        let preview = state.ai.pending_apply.clone().expect("preview opened");
        assert_eq!(preview.replacement, "fn a() -> () {}");
        commands
            .execute("ai.chat.applyConfirm", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "fn a() -> () {}");
    }

    #[test]
    fn fix_with_ai_lists_problems_of_the_line_or_file() {
        let source = "fn a() {}\nfn b( {}\n";
        let (mut state, commands, provider, _) = open(
            "fix",
            source,
            vec![MockReply::text("```rust\nfn a() {}\nfn b() {}\n```")],
        );
        let problems = state.ai_problems(None);
        assert!(!problems.is_empty(), "the parser reports the syntax error");
        assert!(commands.is_enabled("ai.action.fixProblems", &state));
        commands
            .execute("ai.action.fixProblems", &mut state, json!({"line": 1}))
            .unwrap();
        wait(&mut state);
        let sent = provider.requests()[0].messages[0].text();
        assert!(sent.contains("<file path=\"src/lib.rs\""));
        assert!(sent.contains(&format!("- line {}: ", problems[0].0)));
        assert!(state.ai.pending_apply.is_some());
    }

    #[cfg(unix)]
    #[test]
    fn fix_build_attaches_output_and_the_failing_file() {
        let (mut state, commands, provider, root) = open(
            "build",
            "fn a() {}\n",
            vec![MockReply::text("Add a semicolon.")],
        );
        std::fs::write(root.join("src/main.rs"), "fn main() { 1 }\n").unwrap();
        assert!(!commands.is_enabled("ai.action.fixBuild", &state));
        let script =
            "echo 'error[E0308]: mismatched types' >&2; echo ' --> src/main.rs:1:13' >&2; exit 101";
        state
            .execution()
            .start(TaskPlan {
                name: "Cargo Build".into(),
                program: "sh".into(),
                args: vec!["-c".into(), script.into()],
                cwd: root.clone(),
                env: Vec::new(),
                kind: archipel_ide_runner::TaskKind::Build,
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while state.execution().is_running() {
            assert!(Instant::now() < deadline);
            state.poll_runner();
            std::thread::sleep(Duration::from_millis(5));
        }
        // Reader threads may deliver output after the exit status.
        while !state
            .execution()
            .runner()
            .output()
            .iter()
            .any(|c| c.text.contains("-->"))
        {
            assert!(Instant::now() < deadline, "output did not arrive");
            state.poll_runner();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(commands.is_enabled("ai.action.fixBuild", &state));
        commands
            .execute("ai.action.fixBuild", &mut state, Value::Null)
            .unwrap();
        wait(&mut state);
        let sent = provider.requests()[0].messages[0].text();
        assert!(sent.contains("<output command=\"sh -c"));
        assert!(sent.contains("error[E0308]: mismatched types"));
        assert!(sent.contains("<file path=\"src/main.rs\" language=\"rust\">\nfn main() { 1 }"));
        assert!(sent.contains("failed. Explain the cause"));
        assert_eq!(state.ai.chat.turns[0].context[0].label(), "Build output");
        let active = state.active_document.unwrap();
        assert_eq!(state.documents[active].path, PathBuf::from("src/main.rs"));
    }
}

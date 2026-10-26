//! AI Assistant state and commands: provider configuration, AI Chat with attached
//! context, and applying suggested code through the undoable edit pipeline.

use crate::{IdeState, ToolWindow, spec, string_param, usize_param};
use archipel_ide_ai::chat::{ChatSession, ContextItem, ContextKind, TurnState};
use archipel_ide_ai::diff::{DiffLine, line_diff};
use archipel_ide_ai::markdown::code_blocks;
use archipel_ide_ai::settings::{config_dir, secret_name};
use archipel_ide_ai::{
    AiService, AiSettings, AiUpdate, ChatRequest, FileSecretStore, MockProvider, MockReply,
    ProjectPolicy, Provider, ProviderKind, Role, SecretStore,
};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_text::Selection;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Conversation history per project.
pub const CHAT_FILE: &str = ".archipel/ai-chat.json";
/// Set to use the offline demo provider (screenshots, demos); never sends data.
pub const MOCK_ENV: &str = "ARCHIPEL_AI_MOCK";

/// A suggested edit awaiting confirmation in the diff preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyPreview {
    pub path: PathBuf,
    /// Char range replaced in the document.
    pub start: usize,
    pub end: usize,
    pub replacement: String,
    pub diff: Vec<DiffLine>,
    /// Document revision the preview was computed against.
    revision: u64,
}

pub struct AiState {
    pub settings: AiSettings,
    config_dir: Option<PathBuf>,
    provider: Option<Arc<dyn Provider>>,
    /// Provider injected by tests or the demo mode; survives reconfiguration.
    fixed_provider: Option<Arc<dyn Provider>>,
    pub service: AiService,
    pub chat: ChatSession,
    pub policy: ProjectPolicy,
    /// Context chip toggles in the chat input.
    pub attach_file: bool,
    pub attach_selection: bool,
    pub pending_apply: Option<ApplyPreview>,
    /// Last configuration or request error, shown in the chat window.
    pub error: Option<String>,
    /// Inline completion ghost text, and the request producing the next one.
    pub inline: Option<crate::ai_inline::InlineSuggestion>,
    pub(crate) inline_request: Option<crate::ai_inline::InlineRequest>,
    /// Assistant turn of an editing AI action whose diff preview opens when done.
    pub(crate) auto_preview: Option<usize>,
    /// Commit message being generated for the Commit tool window.
    pub commit_draft: Option<crate::ai_vcs::CommitDraft>,
    /// Natural-language terminal command suggestion awaiting confirmation.
    pub shell: Option<crate::ai_terminal::ShellSuggestion>,
    /// Agent mode task.
    pub agent: Option<crate::ai_agent::AgentRun>,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            settings: AiSettings::default(),
            config_dir: None,
            provider: None,
            fixed_provider: None,
            service: AiService::default(),
            chat: ChatSession::default(),
            policy: ProjectPolicy::default(),
            attach_file: true,
            attach_selection: true,
            pending_apply: None,
            error: None,
            inline: None,
            inline_request: None,
            auto_preview: None,
            commit_draft: None,
            shell: None,
            agent: None,
        }
    }
}

impl AiState {
    /// Whether AI features may be used in the current project.
    pub fn is_enabled(&self) -> bool {
        self.settings.enabled && self.policy.allow_ai
    }

    /// Provider label for the status bar widget.
    pub fn summary(&self) -> String {
        match &self.fixed_provider {
            Some(provider) => format!(
                "{} · {}",
                provider.id(),
                provider.model(archipel_ide_ai::ModelRole::Chat)
            ),
            None => self.settings.summary(),
        }
    }

    pub fn is_demo(&self) -> bool {
        self.fixed_provider
            .as_ref()
            .is_some_and(|p| p.id() == "mock")
    }

    /// Uses `provider` instead of the configured one and enables AI.
    pub fn use_provider(&mut self, provider: Arc<dyn Provider>) {
        self.settings.enabled = true;
        self.fixed_provider = Some(provider.clone());
        self.provider = Some(provider);
    }

    fn store(&self) -> Option<FileSecretStore> {
        self.config_dir.as_deref().map(FileSecretStore::new)
    }

    fn provider(&mut self) -> Result<Arc<dyn Provider>, String> {
        if let Some(provider) = &self.provider {
            return Ok(provider.clone());
        }
        struct NoSecrets;
        impl SecretStore for NoSecrets {
            fn get(&self, _: &str) -> Option<String> {
                None
            }
            fn set(&self, _: &str, _: &str) -> Result<(), String> {
                Err("no configuration directory".into())
            }
            fn delete(&self, _: &str) -> Result<(), String> {
                Ok(())
            }
        }
        let env = |name: &str| std::env::var(name).ok();
        let provider = match self.store() {
            Some(store) => archipel_ide_ai::build_provider(&self.settings, &store, &env),
            None => archipel_ide_ai::build_provider(&self.settings, &NoSecrets, &env),
        }
        .map_err(|error| error.to_string())?;
        self.provider = Some(provider.clone());
        Ok(provider)
    }

    pub fn start(&mut self, request: ChatRequest) -> Result<u64, String> {
        let provider = self
            .provider()
            .inspect_err(|error| self.error = Some(error.clone()))?;
        self.error = None;
        Ok(self.service.start(provider, request))
    }
}

pub(crate) fn language_of(path: &Path) -> String {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    match extension {
        "rs" => "rust",
        "kt" | "kts" => "kotlin",
        "java" => "java",
        "py" => "python",
        "js" | "mjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "go" => "go",
        "toml" => "toml",
        "json" => "json",
        "md" => "markdown",
        "sh" | "bash" => "bash",
        "xml" => "xml",
        "gradle" => "groovy",
        other => other,
    }
    .to_owned()
}

impl IdeState {
    /// Loads user AI settings from `dir` (default: the platform config directory).
    /// `ARCHIPEL_AI_MOCK` switches to the offline demo provider.
    pub fn init_ai(&mut self, dir: Option<PathBuf>) {
        self.ai.config_dir = dir.or_else(config_dir);
        if let Some(dir) = &self.ai.config_dir {
            match AiSettings::load(dir) {
                Ok(settings) => self.ai.settings = settings,
                Err(error) => self.ai.error = Some(error),
            }
        }
        if archipel_ide_compat::env_var_os(MOCK_ENV).is_some() {
            self.ai
                .use_provider(Arc::new(MockProvider::responding(demo_reply)));
        }
    }

    /// Loads the project's AI policy and chat history; stops requests of the
    /// previous project.
    pub(crate) fn load_ai_project(&mut self) {
        self.ai.service.cancel_all();
        self.ai.pending_apply = None;
        self.ai.inline = None;
        self.ai.inline_request = None;
        self.ai.auto_preview = None;
        self.ai.commit_draft = None;
        self.ai.shell = None;
        self.ai.agent = None;
        self.ai.chat = ChatSession::default();
        let Some(root) = self.project.as_ref().map(|p| p.root().to_owned()) else {
            return;
        };
        self.ai.policy = ProjectPolicy::load(&root).unwrap_or_else(|error| {
            self.ai.error = Some(error);
            // An unreadable policy must not widen what is shared.
            ProjectPolicy {
                allow_ai: false,
                share_file_contents: false,
            }
        });
        if let Ok(text) =
            std::fs::read_to_string(archipel_ide_compat::project_read_path(root.join(CHAT_FILE)))
        {
            match serde_json::from_str::<ChatSession>(&text) {
                Ok(mut chat) => {
                    // An answer interrupted by quitting is not resumable.
                    for turn in &mut chat.turns {
                        if turn.state == TurnState::Streaming {
                            turn.state = TurnState::Cancelled;
                        }
                    }
                    self.ai.chat = chat;
                }
                Err(_) => self.status = format!("Ignored unreadable {CHAT_FILE}"),
            }
        }
    }

    pub(crate) fn save_ai_chat(&self) -> Result<(), String> {
        let project = self.project.as_ref().ok_or("no project is open")?;
        let path = archipel_ide_compat::project_write_path(project.root().join(CHAT_FILE));
        if self.ai.chat.turns.is_empty() {
            if path.exists() {
                std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(&self.ai.chat).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Context attached to a chat message from the active editor.
    pub fn ai_context(&self, file: bool, selection: bool) -> Result<Vec<ContextItem>, String> {
        let Some(document) = self.active_document.and_then(|i| self.documents.get(i)) else {
            return Ok(Vec::new());
        };
        if (file || selection) && !self.ai.policy.share_file_contents {
            return Err(format!(
                "this project does not share file contents with AI (share_file_contents in {})",
                archipel_ide_ai::settings::PROJECT_POLICY_FILE
            ));
        }
        let language = language_of(&document.path);
        let primary = document.selections.primary();
        let mut items = Vec::new();
        if selection && !primary.is_caret() {
            items.push(ContextItem {
                kind: ContextKind::Selection,
                path: document.path.clone(),
                language: language.clone(),
                text: document
                    .text
                    .slice(primary.start(), primary.end())
                    .map_err(|e| e.to_string())?,
                range: Some((primary.start(), primary.end())),
                first_line: document.text.char_to_line(primary.start()) + 1,
            });
        } else if file {
            items.push(ContextItem {
                kind: ContextKind::File,
                path: document.path.clone(),
                language,
                text: document.text.text(),
                range: None,
                first_line: 1,
            });
        }
        Ok(items)
    }

    pub fn ai_send(&mut self, text: &str, file: bool, selection: bool) -> Result<u64, String> {
        if !self.ai.is_enabled() {
            return Err("AI is off: configure a provider first".into());
        }
        let context = self.ai_context(file, selection)?;
        let request = self.ai.chat.begin(text, context)?;
        match self.ai.start(request) {
            Ok(id) => {
                self.ai.chat.request = Some(id);
                self.tool_windows.show(ToolWindow::AiChat);
                Ok(id)
            }
            Err(error) => {
                self.ai
                    .chat
                    .finish(Err(archipel_ide_ai::AiError::NotConfigured(error.clone())));
                Err(error)
            }
        }
    }

    pub fn ai_stop(&mut self) {
        if let Some(id) = self.ai.chat.request {
            self.ai.service.cancel(id);
        }
    }

    pub fn poll_ai(&mut self) {
        for (id, update) in self.ai.service.poll() {
            if self.ai_commit_update(id, &update) || self.ai_terminal_update(id, &update) {
                continue;
            }
            let Some(update) = self.ai_agent_update(id, update) else {
                continue;
            };
            if self.ai.inline_request.as_ref().is_some_and(|r| r.id == id) {
                if let AiUpdate::Finished(result) = update {
                    self.ai_inline_finished(result);
                }
                continue;
            }
            if self.ai.chat.request != Some(id) {
                continue;
            }
            match update {
                AiUpdate::Event(event) => self.ai.chat.apply_event(event),
                AiUpdate::Finished(result) => {
                    if let Err(error) = &result {
                        self.status = format!("AI: {error}");
                    }
                    self.ai.chat.finish(result);
                    self.ai_action_finished();
                }
            }
        }
        self.ai_agent_poll();
    }

    fn code_block(&self, turn: usize, block: usize) -> Result<String, String> {
        let turn = self
            .ai
            .chat
            .turns
            .get(turn)
            .filter(|t| t.role == Role::Assistant)
            .ok_or("no such answer")?;
        code_blocks(&turn.text)
            .into_iter()
            .nth(block)
            .map(|code| code.code)
            .ok_or_else(|| "no such code block".into())
    }

    /// Computes the diff preview for applying a code block of answer `turn`.
    ///
    /// Target: the selection attached to the question (if unchanged), else the
    /// attached file, else the active editor's selection or caret.
    pub fn ai_preview_apply(&mut self, turn: usize, block: usize) -> Result<(), String> {
        let replacement = self.code_block(turn, block)?;
        let context = turn
            .checked_sub(1)
            .and_then(|q| self.ai.chat.turns.get(q))
            .map(|q| q.context.clone())
            .unwrap_or_default();
        let mut target = None;
        for item in &context {
            let Some(index) = self.documents.iter().position(|d| d.path == item.path) else {
                continue;
            };
            let document = &self.documents[index];
            match (item.kind, item.range) {
                (ContextKind::Selection, Some((start, end)))
                    if document.text.slice(start, end).ok().as_deref() == Some(&item.text) =>
                {
                    target = Some((index, start, end));
                }
                (ContextKind::File, _) => target = Some((index, 0, document.text.len_chars())),
                _ => {}
            }
        }
        let (index, start, end) = match target {
            Some(target) => target,
            None => {
                let index = self
                    .active_document
                    .ok_or("open the file to apply the code to")?;
                let primary = self.documents[index].selections.primary();
                (index, primary.start(), primary.end())
            }
        };
        let document = &self.documents[index];
        let before = document.text.text();
        let mut after: String = before.chars().take(start).collect();
        after.push_str(&replacement);
        after.extend(before.chars().skip(end));
        self.ai.pending_apply = Some(ApplyPreview {
            path: document.path.clone(),
            start,
            end,
            diff: line_diff(&before, &after, 3),
            replacement,
            revision: document.text.revision(),
        });
        Ok(())
    }

    /// Applies the previewed edit as one undoable transaction.
    pub fn ai_confirm_apply(&mut self) -> Result<(), String> {
        let preview = self.ai.pending_apply.take().ok_or("nothing to apply")?;
        let index = self
            .documents
            .iter()
            .position(|d| d.path == preview.path)
            .ok_or("the file was closed; apply again")?;
        if self.documents[index].text.revision() != preview.revision {
            return Err("the file changed since the preview; apply again".into());
        }
        self.activate_document(index)?;
        self.edit_active(|document| {
            document
                .text
                .edit(preview.start, preview.end, &preview.replacement)
                .map_err(|e| e.to_string())?;
            document.selections.set_single(Selection::caret(
                preview.start + preview.replacement.chars().count(),
            ));
            Ok(())
        })?;
        self.status = format!("Applied AI suggestion to {}", preview.path.display());
        Ok(())
    }

    /// Persists provider settings (and the API key in the secret store) and enables AI.
    pub fn ai_configure(
        &mut self,
        provider: ProviderKind,
        chat_model: Option<String>,
        completion_model: Option<String>,
        base_url: Option<String>,
        api_key: Option<String>,
    ) -> Result<(), String> {
        let mut settings = self.ai.settings.clone();
        if settings.provider != provider {
            let (chat, completion) = provider.default_models().unwrap_or(("", ""));
            settings.chat_model = chat.into();
            settings.completion_model = completion.into();
            settings.base_url = None;
        }
        settings.provider = provider;
        if let Some(model) = chat_model.filter(|m| !m.trim().is_empty()) {
            settings.chat_model = model.trim().into();
        }
        if let Some(model) = completion_model.filter(|m| !m.trim().is_empty()) {
            settings.completion_model = model.trim().into();
        }
        if settings.completion_model.is_empty() {
            settings.completion_model = settings.chat_model.clone();
        }
        if let Some(url) = base_url {
            settings.base_url = Some(url.trim().to_owned()).filter(|u| !u.is_empty());
        }
        settings.enabled = true;
        if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
            self.ai
                .store()
                .ok_or("no configuration directory for the secret store")?
                .set(&secret_name(provider), key.trim())?;
        }
        // Validate before committing so a missing key or model leaves AI off.
        let previous = std::mem::replace(&mut self.ai.settings, settings);
        self.ai.provider = self.ai.fixed_provider.clone();
        if let Err(error) = self.ai.provider() {
            self.ai.settings = previous;
            self.ai.provider = None;
            return Err(error);
        }
        if let Some(dir) = &self.ai.config_dir {
            self.ai.settings.save(dir)?;
        }
        self.ai.error = None;
        self.status = format!("AI enabled: {}", self.ai.summary());
        Ok(())
    }

    pub(crate) fn save_ai_settings(&self) -> Result<(), String> {
        match &self.ai.config_dir {
            Some(dir) => self.ai.settings.save(dir),
            None => Ok(()),
        }
    }

    pub fn ai_disable(&mut self) -> Result<(), String> {
        self.ai_inline_dismiss();
        self.ai.service.cancel_all();
        self.poll_ai();
        self.ai.settings.enabled = false;
        self.ai.provider = None;
        self.ai.fixed_provider = None;
        if let Some(dir) = &self.ai.config_dir {
            self.ai.settings.save(dir)?;
        }
        self.status = "AI disabled".into();
        Ok(())
    }
}

/// Offline demo answers for `ARCHIPEL_AI_MOCK` (clearly labelled `mock-chat`).
fn demo_reply(request: &ChatRequest) -> MockReply {
    let question = request
        .messages
        .last()
        .map(|m| m.text())
        .unwrap_or_default();
    if request.system == archipel_ide_ai::agent::SYSTEM_PROMPT {
        return demo_agent_reply(request);
    }
    if request.system == archipel_ide_ai::shell::SYSTEM_PROMPT {
        // A fixed, read-only command: demo mode does not interpret the request.
        return MockReply::text("ls -la");
    }
    if request.system == archipel_ide_ai::commit::SYSTEM_PROMPT {
        let files = question.matches("diff --git").count();
        return MockReply::text(&format!(
            "chore: demo commit message\n\nDemo mode describes {files} changed file(s) without \
             analysing them; configure a provider for real messages."
        ));
    }
    if request.role == archipel_ide_ai::ModelRole::Completion {
        // Generic placeholder code; demo mode never analyses the file.
        let before = question.split("<cursor/>").next().unwrap_or_default();
        let line = before.rsplit('\n').next().unwrap_or_default();
        let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
        let text = if line.trim_end().ends_with('{') {
            format!("\n{indent}    // demo completion\n{indent}    todo!()")
        } else {
            "todo!() // demo completion".to_owned()
        };
        return MockReply::text(&text);
    }
    let attached = ["<selection", "<file"].iter().find_map(|tag| {
        let start = question.find(tag)?;
        let body = &question[start..];
        let open_end = body.find(">\n")? + 2;
        let close = body.find(&format!("\n</{}>", &tag[1..]))?;
        let header = &body[..open_end];
        let language = header
            .split("language=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or_default()
            .to_owned();
        Some((language, body[open_end..close].to_owned()))
    });
    let text = match attached {
        Some((language, code)) => {
            let comment = if matches!(language.as_str(), "python" | "bash" | "toml") {
                "#"
            } else {
                "//"
            };
            format!(
                "**Summary.** The attached code has {} lines. Demo mode does not analyse it; \
                 configure a provider for real answers.\n\n\
                 Suggested change:\n\n1. Keep the behaviour unchanged.\n2. Add a short note \
                 explaining the intent.\n\n```{language}\n{comment} Reviewed by the AI assistant (demo).\n{code}\n```\n",
                code.lines().count()
            )
        }
        None => "This is **demo mode** (`ARCHIPEL_AI_MOCK`): answers are canned and nothing \
                 leaves your machine. Attach a file or selection and try `/explain`."
            .to_owned(),
    };
    let mut reply = MockReply::text(&text);
    reply.delay = std::time::Duration::from_millis(15);
    reply
}

/// Scripted demo agent: list files, read the first Rust (or any) file, propose a
/// one-line comment edit, then summarize. Never derived from real analysis.
fn demo_agent_reply(request: &ChatRequest) -> MockReply {
    let last_result = || {
        request
            .messages
            .last()
            .and_then(|message| {
                message.content.iter().find_map(|block| match block {
                    archipel_ide_ai::ContentBlock::ToolResult {
                        content, is_error, ..
                    } => Some((content.clone(), *is_error)),
                    _ => None,
                })
            })
            .unwrap_or_default()
    };
    let mut reply = match request.messages.len() {
        1 => MockReply {
            chunks: vec![
                "Plan (demo mode, no real analysis):\n\n1. List the project files.\n2. Read \
                 the main source file.\n3. Propose a small edit for your approval.\n4. Summarize.\n"
                    .into(),
            ],
            tool_calls: vec![("list_files".into(), serde_json::json!({}))],
            ..MockReply::default()
        },
        3 => {
            let (listing, _) = last_result();
            let path = listing
                .lines()
                .find(|line| line.ends_with(".rs"))
                .or_else(|| listing.lines().next())
                .unwrap_or("README.md")
                .to_owned();
            MockReply::tool("read_file", serde_json::json!({"path": path}))
        }
        5 => {
            let (content, is_error) = last_result();
            let first = content
                .lines()
                .next()
                .and_then(|line| line.split_once("| "))
                .map(|(_, text)| text.to_owned())
                .filter(|text| !text.trim().is_empty() && !is_error);
            let path = request.messages[3]
                .content
                .iter()
                .find_map(|block| match block {
                    archipel_ide_ai::ContentBlock::ToolUse { input, .. } => {
                        input["path"].as_str().map(str::to_owned)
                    }
                    _ => None,
                })
                .unwrap_or_default();
            match first {
                Some(line) => MockReply::tool(
                    "edit_file",
                    serde_json::json!({
                        "path": path,
                        "old_text": line,
                        "new_text": format!("// Reviewed by the Archipel IDE agent (demo).\n{line}"),
                    }),
                ),
                None => MockReply::text("The file is empty; nothing to change (demo)."),
            }
        }
        _ => {
            let (result, is_error) = last_result();
            MockReply::text(&if is_error {
                format!("The change was not applied: {result} (demo summary).")
            } else {
                format!(
                    "{result}. In demo mode the agent only adds a review comment; configure a provider for real tasks."
                )
            })
        }
    };
    reply.delay = std::time::Duration::from_millis(15);
    reply
}

fn bool_param(params: &Value, name: &str, default: bool) -> Result<bool, CommandError> {
    match params.get(name) {
        None | Some(Value::Null) => Ok(default),
        Some(value) => value
            .as_bool()
            .ok_or_else(|| CommandError::InvalidParams(format!("`{name}` must be a boolean"))),
    }
}

fn optional_string(params: &Value, name: &str) -> Result<Option<String>, CommandError> {
    match params.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_str()
            .map(|s| Some(s.to_owned()))
            .ok_or_else(|| CommandError::InvalidParams(format!("`{name}` must be a string"))),
    }
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    let ready = |state: &IdeState| state.project.is_some() && state.ai.is_enabled();
    let failed = |error: String| CommandError::Failed(error);
    let commands: Vec<Command<IdeState>> = vec![
        Command::new(
            spec(
                "ai.configure",
                "AI: Configure Provider",
                "AI",
                None,
                r#"{"provider":"anthropic|openai|ollama","chatModel":"string?","completionModel":"string?","baseUrl":"string?","apiKey":"string?"}"#,
            ),
            |_| true,
            move |state: &mut IdeState, params| {
                let id = string_param(&params, "provider")?;
                let provider = ProviderKind::parse(&id).ok_or_else(|| {
                    CommandError::InvalidParams(format!(
                        "unknown provider `{id}` (expected anthropic, openai or ollama)"
                    ))
                })?;
                state
                    .ai_configure(
                        provider,
                        optional_string(&params, "chatModel")?,
                        optional_string(&params, "completionModel")?,
                        optional_string(&params, "baseUrl")?,
                        optional_string(&params, "apiKey")?,
                    )
                    .map_err(failed)?;
                Ok(json!({"provider": provider.id(), "chatModel": state.ai.settings.chat_model}))
            },
        ),
        Command::new(
            spec("ai.disable", "AI: Disable", "AI", None, "{}"),
            |state: &IdeState| state.ai.settings.enabled,
            move |state: &mut IdeState, _| {
                state.ai_disable().map_err(failed)?;
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec(
                "ai.chat.send",
                "AI Chat: Send Message",
                "AI",
                None,
                r#"{"text":"string","attachFile":"bool?","attachSelection":"bool?"}"#,
            ),
            move |state: &IdeState| ready(state) && !state.ai.chat.is_streaming(),
            move |state: &mut IdeState, params| {
                let text = string_param(&params, "text")?;
                let file = bool_param(&params, "attachFile", state.ai.attach_file)?;
                let selection = bool_param(&params, "attachSelection", state.ai.attach_selection)?;
                let id = state.ai_send(&text, file, selection).map_err(failed)?;
                Ok(json!({"requestId": id}))
            },
        ),
        Command::new(
            spec("ai.chat.stop", "AI Chat: Stop Generating", "AI", None, "{}"),
            |state: &IdeState| state.ai.chat.is_streaming(),
            |state: &mut IdeState, _| {
                state.ai_stop();
                state.poll_ai();
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec("ai.chat.new", "AI Chat: New Chat", "AI", None, "{}"),
            |state: &IdeState| !state.ai.chat.turns.is_empty(),
            |state: &mut IdeState, _| {
                state.ai_stop();
                state.poll_ai();
                state.ai.chat.clear();
                state.ai.pending_apply = None;
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec(
                "ai.chat.copyCode",
                "AI Chat: Copy Code Block",
                "AI",
                None,
                r#"{"turn":0,"block":0}"#,
            ),
            |_| true,
            move |state: &mut IdeState, params| {
                let text = state
                    .code_block(
                        usize_param(&params, "turn")?,
                        usize_param(&params, "block")?,
                    )
                    .map_err(failed)?;
                Ok(json!({"text": text}))
            },
        ),
        Command::new(
            spec(
                "ai.chat.insertCode",
                "AI Chat: Insert Code at Caret",
                "AI",
                None,
                r#"{"turn":0,"block":0}"#,
            ),
            |state: &IdeState| state.active_document.is_some(),
            move |state: &mut IdeState, params| {
                let text = state
                    .code_block(
                        usize_param(&params, "turn")?,
                        usize_param(&params, "block")?,
                    )
                    .map_err(failed)?;
                let caret = state
                    .active_caret()
                    .ok_or_else(|| failed("no active editor".into()))?;
                state.insert_active_text(caret, &text).map_err(failed)?;
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec(
                "ai.chat.previewApply",
                "AI Chat: Apply Code (Preview Diff)",
                "AI",
                None,
                r#"{"turn":0,"block":0}"#,
            ),
            |state: &IdeState| !state.documents.is_empty(),
            move |state: &mut IdeState, params| {
                state
                    .ai_preview_apply(
                        usize_param(&params, "turn")?,
                        usize_param(&params, "block")?,
                    )
                    .map_err(failed)?;
                let preview = state
                    .ai
                    .pending_apply
                    .as_ref()
                    .expect("preview was just set");
                let (added, removed) = archipel_ide_ai::diff::stats(&preview.diff);
                Ok(json!({"path": preview.path, "added": added, "removed": removed}))
            },
        ),
        Command::new(
            spec(
                "ai.chat.applyConfirm",
                "AI Chat: Accept Suggested Change",
                "AI",
                None,
                "{}",
            ),
            |state: &IdeState| state.ai.pending_apply.is_some(),
            move |state: &mut IdeState, _| {
                state.ai_confirm_apply().map_err(failed)?;
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec(
                "ai.chat.applyCancel",
                "AI Chat: Discard Suggested Change",
                "AI",
                None,
                "{}",
            ),
            |state: &IdeState| state.ai.pending_apply.is_some(),
            |state: &mut IdeState, _| {
                state.ai.pending_apply = None;
                Ok(Value::Null)
            },
        ),
        Command::new(
            spec(
                "ai.chat.setContext",
                "AI Chat: Set Attached Context",
                "AI",
                None,
                r#"{"file":"bool?","selection":"bool?"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                state.ai.attach_file = bool_param(&params, "file", state.ai.attach_file)?;
                state.ai.attach_selection =
                    bool_param(&params, "selection", state.ai.attach_selection)?;
                Ok(json!({"file": state.ai.attach_file, "selection": state.ai.attach_selection}))
            },
        ),
    ];
    for command in commands {
        registry.register(command).expect("unique built-in id");
    }
    crate::ai_inline::register(registry);
    crate::ai_actions::register(registry);
    crate::ai_vcs::register(registry);
    crate::ai_terminal::register(registry);
    crate::ai_agent::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;
    use archipel_ide_ai::{AiError, MockReply};
    use std::time::{Duration, Instant};

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "archipel-ide-core-ai-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/lib.rs"), "fn a() {}\nfn b() {}\nfn c() {}\n").unwrap();
        dir
    }

    fn wait(state: &mut IdeState) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while state.ai.chat.is_streaming() {
            assert!(Instant::now() < deadline, "answer did not finish");
            state.poll_ai();
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn open(
        name: &str,
        provider: Arc<MockProvider>,
    ) -> (IdeState, CommandRegistry<IdeState>, PathBuf) {
        let root = temp(name);
        let commands = built_in_commands();
        let mut state = IdeState::default();
        state.init_ai(Some(root.join("config")));
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "src/lib.rs"}))
            .unwrap();
        state.ai.use_provider(provider);
        (state, commands, root)
    }

    #[test]
    fn ai_is_off_until_configured_and_configuration_persists() {
        let root = temp("configure");
        let commands = built_in_commands();
        let mut state = IdeState::default();
        state.init_ai(Some(root.join("config")));
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        assert!(!commands.is_enabled("ai.chat.send", &state));
        assert!(matches!(
            commands.execute("ai.configure", &mut state, json!({"provider": "nope"})),
            Err(CommandError::InvalidParams(_))
        ));
        // Anthropic without a stored or environment key stays off.
        if std::env::var_os("ANTHROPIC_API_KEY").is_none() {
            let error = commands
                .execute("ai.configure", &mut state, json!({"provider": "anthropic"}))
                .unwrap_err();
            assert!(error.to_string().contains("ANTHROPIC_API_KEY"));
            assert!(!state.ai.settings.enabled);
            assert!(!root.join("config/ai.toml").exists());
        }
        // Ollama needs no key but has no default model.
        assert!(
            commands
                .execute("ai.configure", &mut state, json!({"provider": "ollama"}))
                .is_err()
        );
        let result = commands
            .execute(
                "ai.configure",
                &mut state,
                json!({"provider": "ollama", "chatModel": "qwen2.5-coder"}),
            )
            .unwrap();
        assert_eq!(result["chatModel"], "qwen2.5-coder");
        assert!(commands.is_enabled("ai.chat.send", &state));
        let mut reloaded = IdeState::default();
        reloaded.init_ai(Some(root.join("config")));
        assert!(reloaded.ai.settings.enabled);
        assert_eq!(reloaded.ai.settings.provider, ProviderKind::Ollama);
        assert_eq!(reloaded.ai.settings.completion_model, "qwen2.5-coder");
        commands
            .execute(
                "ai.configure",
                &mut state,
                json!({"provider": "anthropic", "apiKey": "sk-stored"}),
            )
            .unwrap();
        assert_eq!(state.ai.settings.chat_model, "claude-opus-5-5");
        assert_eq!(
            FileSecretStore::new(&root.join("config"))
                .get("anthropic.api_key")
                .as_deref(),
            Some("sk-stored")
        );
        commands
            .execute("ai.disable", &mut state, Value::Null)
            .unwrap();
        assert!(!commands.is_enabled("ai.chat.send", &state));
    }

    #[test]
    fn chat_streams_with_selection_context_and_persists() {
        let provider = Arc::new(MockProvider::new([MockReply::text("It defines **b**.")]));
        let (mut state, commands, root) = open("chat", provider.clone());
        state.documents[0].selections.set_single(Selection {
            anchor: 10,
            head: 19,
        });
        commands
            .execute("ai.chat.send", &mut state, json!({"text": "/explain"}))
            .unwrap();
        assert!(state.tool_windows.is_visible(ToolWindow::AiChat));
        assert!(!commands.is_enabled("ai.chat.send", &state));
        wait(&mut state);
        assert_eq!(state.ai.chat.turns[1].text, "It defines **b**.");
        assert_eq!(state.ai.chat.turns[1].model, "mock-chat");
        let sent = provider.requests()[0].messages[0].text();
        assert!(sent.contains(
            "<selection path=\"src/lib.rs\" language=\"rust\" lines=\"2-2\">\nfn b() {}"
        ));
        // Selection wins over the whole file.
        assert!(!sent.contains("fn a()"));
        commands
            .execute("workspace.save", &mut state, Value::Null)
            .unwrap();
        let mut reopened = IdeState::default();
        reopened.open_project(&root).unwrap();
        assert_eq!(reopened.ai.chat.turns.len(), 2);
        commands
            .execute("ai.chat.new", &mut state, Value::Null)
            .unwrap();
        assert!(state.ai.chat.turns.is_empty());
    }

    #[test]
    fn stop_cancels_the_streaming_answer() {
        let mut slow = MockReply::text("one two three four five six");
        slow.delay = Duration::from_millis(40);
        let (mut state, commands, _) = open("stop", Arc::new(MockProvider::new([slow])));
        commands
            .execute(
                "ai.chat.send",
                &mut state,
                json!({"text": "hi", "attachFile": false}),
            )
            .unwrap();
        commands
            .execute("ai.chat.stop", &mut state, Value::Null)
            .unwrap();
        assert_eq!(state.ai.chat.turns[1].state, TurnState::Cancelled);
        assert!(commands.is_enabled("ai.chat.send", &state));
    }

    #[test]
    fn provider_errors_are_shown_on_the_answer() {
        let provider = Arc::new(MockProvider::new([MockReply::error(AiError::Auth(
            "bad key".into(),
        ))]));
        let (mut state, commands, _) = open("error", provider);
        commands
            .execute("ai.chat.send", &mut state, json!({"text": "hi"}))
            .unwrap();
        wait(&mut state);
        assert!(
            matches!(&state.ai.chat.turns[1].state, TurnState::Failed(m) if m.contains("bad key"))
        );
        assert!(state.status.contains("authentication failed"));
    }

    #[test]
    fn apply_previews_a_diff_and_edits_undoably() {
        let reply = MockReply::text(
            "Better:\n\n```rust\nfn b() -> u8 { 1 }\n```\n\n```rust\nfn x() {}\n```",
        );
        let (mut state, commands, _) = open("apply", Arc::new(MockProvider::new([reply])));
        state.documents[0].selections.set_single(Selection {
            anchor: 10,
            head: 19,
        });
        commands
            .execute("ai.chat.send", &mut state, json!({"text": "/fix"}))
            .unwrap();
        wait(&mut state);
        assert_eq!(
            commands
                .execute(
                    "ai.chat.copyCode",
                    &mut state,
                    json!({"turn": 1, "block": 1})
                )
                .unwrap()["text"],
            "fn x() {}"
        );
        // The caret moved away; the attached selection is still the target.
        state.documents[0]
            .selections
            .set_single(Selection::caret(0));
        let preview = commands
            .execute(
                "ai.chat.previewApply",
                &mut state,
                json!({"turn": 1, "block": 0}),
            )
            .unwrap();
        assert_eq!(
            (preview["added"].as_u64(), preview["removed"].as_u64()),
            (Some(1), Some(1))
        );
        assert_eq!(
            state.documents[0].text.text(),
            "fn a() {}\nfn b() {}\nfn c() {}\n"
        );
        commands
            .execute("ai.chat.applyConfirm", &mut state, Value::Null)
            .unwrap();
        assert_eq!(
            state.documents[0].text.text(),
            "fn a() {}\nfn b() -> u8 { 1 }\nfn c() {}\n"
        );
        commands
            .execute("editor.undo", &mut state, Value::Null)
            .unwrap();
        assert_eq!(
            state.documents[0].text.text(),
            "fn a() {}\nfn b() {}\nfn c() {}\n"
        );
        // A stale preview is refused instead of corrupting the file.
        commands
            .execute(
                "ai.chat.previewApply",
                &mut state,
                json!({"turn": 1, "block": 1}),
            )
            .unwrap();
        state.insert_active_text(0, "// edit\n").unwrap();
        assert!(
            commands
                .execute("ai.chat.applyConfirm", &mut state, Value::Null)
                .is_err()
        );
        assert!(
            commands
                .execute(
                    "ai.chat.previewApply",
                    &mut state,
                    json!({"turn": 1, "block": 9})
                )
                .is_err()
        );
        commands
            .execute(
                "ai.chat.insertCode",
                &mut state,
                json!({"turn": 1, "block": 1}),
            )
            .unwrap();
        assert!(
            state.documents[0]
                .text
                .text()
                .starts_with("// edit\nfn x() {}")
        );
    }

    #[test]
    fn project_policy_blocks_file_sharing() {
        let provider = Arc::new(MockProvider::default());
        let (mut state, commands, root) = open("policy", provider.clone());
        std::fs::create_dir_all(root.join(".archipel")).unwrap();
        std::fs::write(
            root.join(".archipel/ai.toml"),
            "share_file_contents = false\n",
        )
        .unwrap();
        state.open_project(&root).unwrap();
        state.ai.use_provider(provider.clone());
        state.open_file(Path::new("src/lib.rs")).unwrap();
        assert!(
            commands
                .execute("ai.chat.send", &mut state, json!({"text": "hi"}))
                .unwrap_err()
                .to_string()
                .contains("does not share file contents")
        );
        commands
            .execute(
                "ai.chat.send",
                &mut state,
                json!({"text": "hi", "attachFile": false, "attachSelection": false}),
            )
            .unwrap();
        wait(&mut state);
        assert_eq!(provider.requests()[0].messages[0].text(), "hi");
    }

    #[test]
    fn demo_agent_proposes_an_approvable_edit_and_summarizes() {
        let (mut state, commands, _) = open("demo-agent", Arc::new(MockProvider::default()));
        state
            .ai
            .use_provider(Arc::new(MockProvider::responding(demo_reply)));
        commands
            .execute("ai.agent.start", &mut state, json!({"goal": "review"}))
            .unwrap();
        let settle = |state: &mut IdeState| {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                state.poll_ai();
                let phase = state.ai.agent.as_ref().unwrap().phase.clone();
                if !matches!(
                    phase,
                    crate::AgentPhase::Thinking | crate::AgentPhase::RunningCommand
                ) {
                    return phase;
                }
                assert!(Instant::now() < deadline, "demo agent stuck");
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        assert_eq!(settle(&mut state), crate::AgentPhase::AwaitingApproval);
        commands
            .execute("ai.agent.approve", &mut state, Value::Null)
            .unwrap();
        assert_eq!(settle(&mut state), crate::AgentPhase::Done);
        let text = state
            .documents
            .iter()
            .find(|d| d.path == Path::new("src/lib.rs"))
            .unwrap()
            .text
            .text();
        assert!(text.starts_with("// Reviewed by the Archipel IDE agent (demo).\nfn a() {}"));
    }

    #[test]
    fn demo_reply_echoes_attached_code_in_a_block() {
        let request = ChatRequest::chat(
            "",
            vec![archipel_ide_ai::ChatMessage::user(
                "<selection path=\"a.py\" language=\"python\" lines=\"1-1\">\nx = 1\n</selection>\n\nExplain",
            )],
        );
        let text = demo_reply(&request).chunks.concat();
        assert!(text.contains("```python\n# Reviewed by the AI assistant (demo).\nx = 1\n```"));
        let mut completion = ChatRequest::chat(
            "",
            vec![archipel_ide_ai::ChatMessage::user("  fn a() {<cursor/>\n")],
        );
        completion.role = archipel_ide_ai::ModelRole::Completion;
        assert_eq!(
            demo_reply(&completion).chunks.concat(),
            "\n      // demo completion\n      todo!()"
        );
    }
}

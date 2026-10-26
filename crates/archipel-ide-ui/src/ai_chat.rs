//! AI Chat tool window (right stripe), apply-with-diff preview and status widget.
//!
//! Every action dispatches an `ai.*` command; this module only renders state.

use crate::icons::{self, Icon};
use crate::widgets::{self, island_frame, tool_window_header};
use crate::{IdeApp, theme};
use archipel_ide_ai::ProviderKind;
use archipel_ide_ai::chat::{ContextKind, SLASH_COMMANDS, TurnState};
use archipel_ide_ai::diff::{DiffKind, DiffLine, stats};
use archipel_ide_ai::markdown::{MdBlock, Span, parse};
use archipel_ide_ai::{Role, settings::PROJECT_POLICY_FILE};
use archipel_ide_core::{Anchor, ToolWindow};
use eframe::egui::{self, Color32, RichText, text::LayoutJob, vec2};
use serde_json::{Value, json};

pub fn input_id() -> egui::Id {
    egui::Id::new("ai_chat_input")
}

/// Provider setup form shown while AI is off.
pub struct SetupForm {
    provider: ProviderKind,
    chat_model: String,
    completion_model: String,
    base_url: String,
    api_key: String,
    error: Option<String>,
}

impl Default for SetupForm {
    fn default() -> Self {
        Self {
            provider: ProviderKind::Anthropic,
            chat_model: archipel_ide_ai::settings::ANTHROPIC_CHAT_MODEL.into(),
            completion_model: archipel_ide_ai::settings::ANTHROPIC_COMPLETION_MODEL.into(),
            base_url: String::new(),
            api_key: String::new(),
            error: None,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    #[default]
    Chat,
    Agent,
}

#[derive(Default)]
pub struct AiChatUi {
    pub(crate) mode: Mode,
    pub(crate) agent_input: String,
    pub(crate) inline: crate::ai_inline::InlineUi,
    pub(crate) terminal_open: bool,
    pub(crate) terminal_input: String,
    pub(crate) terminal_focus: bool,
    input: String,
    setup: SetupForm,
    last_visible: bool,
}

enum Action {
    Send,
    Stop,
    NewChat,
    Copy(usize, usize),
    Insert(usize, usize),
    Apply(usize, usize),
    Context { file: bool, selection: bool },
    Configure,
    Hide,
}

/// Right-anchored tool window; shown before the bottom panel so it spans the height.
pub fn right(app: &mut IdeApp, ctx: &egui::Context) {
    let visible = app.state.tool_windows.visible(Anchor::Right) == Some(ToolWindow::AiChat);
    if visible && !app.ai_ui.last_visible && app.state.ai.is_enabled() {
        ctx.memory_mut(|memory| memory.request_focus(input_id()));
    }
    app.ai_ui.last_visible = visible;
    if !visible {
        return;
    }
    let mut actions = Vec::new();
    egui::SidePanel::right("right_tool_window")
        .default_width(400.0)
        .min_width(280.0)
        .resizable(true)
        .show_separator_line(false)
        .frame(island_frame(ctx))
        .show(ctx, |ui| {
            let enabled = app.state.ai.is_enabled();
            let has_turns = !app.state.ai.chat.turns.is_empty();
            if tool_window_header(ui, ToolWindow::AiChat.title(), |ui| {
                if enabled && has_turns && icons::button(ui, Icon::Plus, "New Chat").clicked() {
                    actions.push(Action::NewChat);
                }
            }) {
                actions.push(Action::Hide);
            }
            if !app.state.ai.policy.allow_ai {
                empty_state(
                    ui,
                    "AI is disabled for this project",
                    &format!(
                        "Set allow_ai = true in {PROJECT_POLICY_FILE} to use AI features here."
                    ),
                );
            } else if !enabled {
                setup(app, ui, &mut actions);
            } else {
                mode_tabs(app, ui);
                if app.ai_ui.mode == Mode::Agent {
                    crate::ai_agent::panel(app, ui);
                    return;
                }
                egui::TopBottomPanel::bottom("ai_chat_input_panel")
                    .show_separator_line(false)
                    .frame(egui::Frame::new().inner_margin(egui::Margin {
                        left: 10,
                        right: 10,
                        top: 6,
                        bottom: 10,
                    }))
                    .show_inside(ui, |ui| input_area(app, ui, &mut actions));
                egui::CentralPanel::default()
                    .frame(egui::Frame::new())
                    .show_inside(ui, |ui| conversation(app, ui, &mut actions));
            }
        });
    for action in actions {
        perform(app, ctx, action);
    }
}

/// Chat | Agent switch below the header.
fn mode_tabs(app: &mut IdeApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        for (mode, label) in [(Mode::Chat, "Chat"), (Mode::Agent, "Agent")] {
            let mut text = RichText::new(label);
            if app.ai_ui.mode == mode {
                text = text.color(theme::TEXT_BRIGHT).strong();
            }
            if ui.selectable_label(app.ai_ui.mode == mode, text).clicked() {
                app.ai_ui.mode = mode;
            }
        }
        if app
            .state
            .ai
            .agent
            .as_ref()
            .is_some_and(|a| a.phase.is_active())
            && app.ai_ui.mode == Mode::Chat
        {
            ui.label(RichText::new("agent running…").color(theme::TEXT_INFO));
        }
    });
    ui.add_space(2.0);
}

fn perform(app: &mut IdeApp, ctx: &egui::Context, action: Action) {
    let block = |turn, block| json!({"turn": turn, "block": block});
    match action {
        Action::Send => {
            let text = app.ai_ui.input.trim().to_owned();
            if text.is_empty() {
                return;
            }
            match app
                .commands
                .execute("ai.chat.send", &mut app.state, json!({"text": text}))
            {
                Ok(_) => app.ai_ui.input.clear(),
                Err(error) => app.state.status = error.to_string(),
            }
        }
        Action::Stop => app.dispatch("ai.chat.stop", Value::Null),
        Action::NewChat => app.dispatch("ai.chat.new", Value::Null),
        Action::Copy(turn, index) => {
            match app
                .commands
                .execute("ai.chat.copyCode", &mut app.state, block(turn, index))
            {
                Ok(result) => {
                    ctx.copy_text(result["text"].as_str().unwrap_or_default().to_owned());
                    app.state.status = "Code copied to the clipboard".into();
                }
                Err(error) => app.state.status = error.to_string(),
            }
        }
        Action::Insert(turn, index) => {
            app.dispatch("ai.chat.insertCode", block(turn, index));
            ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
        }
        Action::Apply(turn, index) => app.dispatch("ai.chat.previewApply", block(turn, index)),
        Action::Context { file, selection } => app.dispatch(
            "ai.chat.setContext",
            json!({"file": file, "selection": selection}),
        ),
        Action::Configure => {
            let form = &app.ai_ui.setup;
            let params = json!({
                "provider": form.provider.id(),
                "chatModel": form.chat_model,
                "completionModel": form.completion_model,
                "baseUrl": form.base_url,
                "apiKey": form.api_key,
            });
            match app.commands.execute("ai.configure", &mut app.state, params) {
                Ok(_) => {
                    app.ai_ui.setup.api_key.clear();
                    app.ai_ui.setup.error = None;
                    ctx.memory_mut(|memory| memory.request_focus(input_id()));
                }
                Err(error) => app.ai_ui.setup.error = Some(error.to_string()),
            }
        }
        Action::Hide => app.toggle_tool_window(ToolWindow::AiChat),
    }
}

fn empty_state(ui: &mut egui::Ui, title: &str, detail: &str) {
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        icons::show(ui, Icon::Ai, 28.0, theme::TEXT_SECONDARY);
        ui.add_space(8.0);
        ui.label(RichText::new(title).color(theme::TEXT_BRIGHT).strong());
        ui.add_space(4.0);
        ui.label(RichText::new(detail).color(theme::TEXT_SECONDARY));
    });
}

fn setup(app: &mut IdeApp, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    let form = &mut app.ai_ui.setup;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(14, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        icons::show(ui, Icon::Ai, 18.0, theme::TEXT_BRIGHT);
                        ui.label(
                            RichText::new("AI Assistant is off")
                                .font(theme::semibold(theme::UI_FONT_SIZE + 1.0))
                                .color(theme::TEXT_BRIGHT),
                        );
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "Choose a provider to enable chat. Nothing is sent anywhere until \
                             you enable it; attached files and selections are sent only with \
                             your messages.",
                        )
                        .color(theme::TEXT_SECONDARY),
                    );
                    ui.add_space(10.0);
                    let previous = form.provider;
                    egui::Grid::new("ai_setup")
                        .num_columns(2)
                        .spacing([10.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Provider");
                            egui::ComboBox::from_id_salt("ai_provider")
                                .selected_text(form.provider.label())
                                .show_ui(ui, |ui| {
                                    for kind in [
                                        ProviderKind::Anthropic,
                                        ProviderKind::OpenAi,
                                        ProviderKind::Ollama,
                                    ] {
                                        ui.selectable_value(&mut form.provider, kind, kind.label());
                                    }
                                });
                            ui.end_row();
                            ui.label("Chat model");
                            ui.add(
                                egui::TextEdit::singleline(&mut form.chat_model)
                                    .hint_text("required")
                                    .desired_width(f32::INFINITY),
                            );
                            ui.end_row();
                            ui.label("Completion model");
                            ui.add(
                                egui::TextEdit::singleline(&mut form.completion_model)
                                    .hint_text("same as chat model")
                                    .desired_width(f32::INFINITY),
                            );
                            ui.end_row();
                            ui.label("Endpoint");
                            ui.add(
                                egui::TextEdit::singleline(&mut form.base_url)
                                    .hint_text(form.provider.default_url())
                                    .desired_width(f32::INFINITY),
                            );
                            ui.end_row();
                            if let Some(env) = form.provider.key_env() {
                                ui.label("API key");
                                let hint = if std::env::var_os(env).is_some() {
                                    format!("using {env} from the environment")
                                } else {
                                    format!("or set {env}")
                                };
                                ui.add(
                                    egui::TextEdit::singleline(&mut form.api_key)
                                        .password(true)
                                        .hint_text(hint)
                                        .desired_width(f32::INFINITY),
                                );
                                ui.end_row();
                            }
                        });
                    if form.provider != previous {
                        let (chat, completion) = form.provider.default_models().unwrap_or(("", ""));
                        form.chat_model = chat.into();
                        form.completion_model = completion.into();
                        form.base_url.clear();
                        form.error = None;
                    }
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "Keys are kept in the Archipel IDE config directory (secrets.toml, \
                             readable only by you).",
                        )
                        .color(theme::TEXT_INFO)
                        .size(theme::UI_FONT_SIZE - 1.0),
                    );
                    if let Some(error) = &form.error {
                        ui.add_space(6.0);
                        ui.label(RichText::new(error).color(theme::ERROR));
                    }
                    ui.add_space(10.0);
                    if ui
                        .add_enabled(
                            !form.chat_model.trim().is_empty(),
                            egui::Button::new(RichText::new("Enable AI").color(Color32::WHITE))
                                .fill(theme::ACCENT),
                        )
                        .clicked()
                    {
                        actions.push(Action::Configure);
                    }
                });
        });
}

fn conversation(app: &mut IdeApp, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    let chat = &app.state.ai.chat;
    if chat.turns.is_empty() {
        empty_state(
            ui,
            "Ask about your code",
            "The current file or selection is attached automatically. Try /explain, /tests, /docs or /fix.",
        );
        return;
    }
    let demo = app.state.ai.is_demo();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    for (index, turn) in chat.turns.iter().enumerate() {
                        match turn.role {
                            Role::User => {
                                egui::Frame::new()
                                    .fill(*theme::FRAME)
                                    .corner_radius(8)
                                    .inner_margin(egui::Margin::symmetric(10, 8))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.label(
                                            RichText::new("You")
                                                .color(theme::TEXT_SECONDARY)
                                                .size(theme::UI_FONT_SIZE - 1.0),
                                        );
                                        ui.label(
                                            RichText::new(&turn.text).color(theme::TEXT_BRIGHT),
                                        );
                                        if !turn.context.is_empty() {
                                            ui.horizontal_wrapped(|ui| {
                                                for item in &turn.context {
                                                    chip(ui, &item.label(), item.kind, true, false);
                                                }
                                            });
                                        }
                                    });
                            }
                            Role::Assistant => {
                                ui.horizontal(|ui| {
                                    icons::show(ui, Icon::Ai, 14.0, theme::MODIFIED);
                                    let mut label = "AI Assistant".to_owned();
                                    if !turn.model.is_empty() {
                                        label.push_str(&format!(" · {}", turn.model));
                                    }
                                    if demo {
                                        label.push_str(" (demo)");
                                    }
                                    ui.label(
                                        RichText::new(label)
                                            .color(theme::TEXT_SECONDARY)
                                            .size(theme::UI_FONT_SIZE - 1.0),
                                    );
                                });
                                markdown(
                                    ui,
                                    index,
                                    &turn.text,
                                    turn.state == TurnState::Done,
                                    actions,
                                );
                                match &turn.state {
                                    TurnState::Streaming => {
                                        ui.horizontal(|ui| {
                                            ui.add(egui::Spinner::new().size(12.0));
                                            let label = if turn.thinking || turn.text.is_empty() {
                                                "Thinking…"
                                            } else {
                                                "Generating…"
                                            };
                                            ui.label(RichText::new(label).color(theme::TEXT_INFO));
                                        });
                                    }
                                    TurnState::Cancelled => {
                                        ui.label(
                                            RichText::new("Stopped")
                                                .color(theme::TEXT_INFO)
                                                .italics(),
                                        );
                                    }
                                    TurnState::Failed(error) => {
                                        ui.horizontal_wrapped(|ui| {
                                            icons::show(ui, Icon::Error, 14.0, theme::ERROR);
                                            ui.label(RichText::new(error).color(theme::ERROR));
                                        });
                                    }
                                    TurnState::Done => {}
                                }
                            }
                        }
                        ui.add_space(10.0);
                    }
                });
        });
}

fn chip(
    ui: &mut egui::Ui,
    label: &str,
    kind: ContextKind,
    on: bool,
    enabled: bool,
) -> egui::Response {
    let icon = match kind {
        ContextKind::File => Icon::for_file(label.split(':').next().unwrap_or(label)),
        ContextKind::Selection => Icon::File,
        ContextKind::Output => Icon::Build,
    };
    let text_color = if on {
        theme::TEXT
    } else {
        theme::TEXT_DISABLED
    };
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0),
        *text_color,
    );
    let size = vec2(galley.size().x + 32.0, 22.0);
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let stroke = if on {
        theme::BORDER
    } else {
        theme::TOOLBAR_SEPARATOR
    };
    let fill = if response.hovered() && enabled {
        *theme::HOVER
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect(
        rect,
        11.0,
        fill,
        egui::Stroke::new(1.0_f32, stroke),
        egui::StrokeKind::Inside,
    );
    icons::paint(
        ui.painter(),
        egui::Rect::from_min_size(rect.left_center() + vec2(8.0, -7.0), vec2(14.0, 14.0)),
        icon,
        if on {
            icons::ICON
        } else {
            theme::TEXT_DISABLED
        },
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 26.0, rect.center().y - galley.size().y / 2.0),
        galley,
        *text_color,
    );
    response
}

fn input_area(app: &mut IdeApp, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    let ai = &app.state.ai;
    let streaming = ai.chat.is_streaming();
    // Context chips for what will be attached from the active editor.
    let context = app.state.ai_context(true, true);
    let sharing = ai.policy.share_file_contents;
    if let Some(document) = app
        .state
        .active_document
        .and_then(|i| app.state.documents.get(i))
    {
        let name = IdeApp::file_name(&document.path);
        let selection = document.selections.primary();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
            let has_selection = !selection.is_caret();
            if has_selection {
                let label = context
                    .as_ref()
                    .ok()
                    .and_then(|items| items.first())
                    .map(|item| item.label())
                    .unwrap_or_else(|| name.clone());
                let on = ai.attach_selection && sharing;
                if chip(ui, &label, ContextKind::Selection, on, sharing)
                    .on_hover_text(if on {
                        "Selection attached — click to detach"
                    } else {
                        "Click to attach the selection"
                    })
                    .clicked()
                {
                    actions.push(Action::Context {
                        file: ai.attach_file,
                        selection: !ai.attach_selection,
                    });
                }
            }
            if !has_selection || !ai.attach_selection {
                let on = ai.attach_file && sharing;
                if chip(ui, &name, ContextKind::File, on, sharing)
                    .on_hover_text(if on {
                        "Current file attached — click to detach"
                    } else {
                        "Click to attach the current file"
                    })
                    .clicked()
                {
                    actions.push(Action::Context {
                        file: !ai.attach_file,
                        selection: ai.attach_selection,
                    });
                }
            }
            if !sharing {
                ui.label(
                    RichText::new("File sharing is off for this project").color(theme::TEXT_INFO),
                );
            }
        });
        ui.add_space(6.0);
    }
    // Slash command suggestions.
    let input = app.ai_ui.input.clone();
    if input.starts_with('/') && !input.contains(char::is_whitespace) {
        for (command, prompt) in SLASH_COMMANDS
            .iter()
            .filter(|(c, _)| c.starts_with(input.as_str()))
        {
            let (rect, response) = widgets::list_row(ui, false);
            let x = widgets::row_text(
                ui,
                rect,
                rect.left() + 8.0,
                command,
                egui::FontId::monospace(theme::UI_FONT_SIZE),
                theme::TEXT_BRIGHT,
            );
            widgets::row_text(
                ui,
                rect,
                x + 10.0,
                prompt,
                egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0),
                theme::TEXT_INFO,
            );
            if response.clicked() {
                app.ai_ui.input = format!("{command} ");
                ui.memory_mut(|memory| memory.request_focus(input_id()));
            }
        }
    }
    let focused = ui.memory(|memory| memory.has_focus(input_id()));
    // Enter sends, Shift+Enter inserts a newline.
    if focused
        && !streaming
        && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
    {
        actions.push(Action::Send);
    }
    egui::Frame::new()
        .fill(*theme::FIELD)
        .stroke(egui::Stroke::new(
            1.0_f32,
            if focused {
                theme::ACCENT
            } else {
                theme::BORDER
            },
        ))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut app.ai_ui.input)
                    .id(input_id())
                    .frame(false)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .hint_text("Ask AI Assistant, use / for commands"),
            );
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(app.state.ai.summary())
                        .color(theme::TEXT_INFO)
                        .size(theme::UI_FONT_SIZE - 2.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if streaming {
                        if icons::button_colored(
                            ui,
                            Icon::Stop,
                            theme::STOP_RED,
                            "Stop generating",
                            false,
                        )
                        .clicked()
                        {
                            actions.push(Action::Stop);
                        }
                    } else {
                        let ready = !app.ai_ui.input.trim().is_empty();
                        let color = if ready {
                            Color32::WHITE
                        } else {
                            *theme::TEXT_DISABLED
                        };
                        if icons::button_colored(ui, Icon::Send, color, "Send (Enter)", ready)
                            .clicked()
                            && ready
                        {
                            actions.push(Action::Send);
                        }
                    }
                });
            });
        });
    if let Some(error) = &app.state.ai.error {
        ui.label(RichText::new(error).color(theme::ERROR));
    }
}

fn span_job(spans: &[Span], size: f32, base: Color32, wrap: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    for span in spans {
        let mut format = egui::TextFormat {
            font_id: if span.style.code {
                egui::FontId::monospace(size - 0.5)
            } else if span.style.bold {
                theme::semibold(size)
            } else {
                egui::FontId::proportional(size)
            },
            color: base,
            italics: span.style.italic,
            ..Default::default()
        };
        if span.style.code {
            format.background = *theme::FRAME;
            format.color = *theme::TEXT_BRIGHT;
        }
        if span.style.link {
            format.color = *theme::MODIFIED;
            format.underline = egui::Stroke::new(1.0_f32, theme::MODIFIED);
        }
        job.append(&span.text, 0.0, format);
    }
    job.wrap.max_width = wrap;
    job
}

/// Renders an answer; code blocks get Copy / Insert / Apply once the answer is done.
fn markdown(ui: &mut egui::Ui, turn: usize, text: &str, done: bool, actions: &mut Vec<Action>) {
    let width = ui.available_width();
    let mut code_index = 0;
    for block in parse(text) {
        match block {
            MdBlock::Heading { level, spans } => {
                let size = theme::UI_FONT_SIZE
                    + match level {
                        1 => 3.0,
                        2 => 2.0,
                        _ => 1.0,
                    };
                let mut bold = spans.clone();
                for span in &mut bold {
                    span.style.bold = true;
                }
                ui.add_space(4.0);
                ui.label(span_job(&bold, size, *theme::TEXT_BRIGHT, width));
            }
            MdBlock::Paragraph(spans) => {
                ui.label(span_job(&spans, theme::UI_FONT_SIZE, *theme::TEXT, width));
            }
            MdBlock::Quote(spans) => {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(3.0, 18.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 1.0, theme::BORDER);
                    ui.label(span_job(
                        &spans,
                        theme::UI_FONT_SIZE,
                        *theme::TEXT_SECONDARY,
                        width - 12.0,
                    ));
                });
            }
            MdBlock::ListItem {
                depth,
                marker,
                spans,
            } => {
                ui.horizontal_top(|ui| {
                    ui.add_space(depth as f32 * 16.0 + 4.0);
                    ui.label(RichText::new(&marker).color(theme::TEXT_SECONDARY));
                    let indent = depth as f32 * 16.0 + 24.0;
                    ui.label(span_job(
                        &spans,
                        theme::UI_FONT_SIZE,
                        *theme::TEXT,
                        width - indent,
                    ));
                });
            }
            MdBlock::Rule => {
                ui.separator();
            }
            MdBlock::Code(code) => {
                let index = code_index;
                code_index += 1;
                egui::Frame::new()
                    .fill(*theme::editor::BACKGROUND_PANEL)
                    .stroke(egui::Stroke::new(1.0_f32, theme::POPUP_BORDER))
                    .corner_radius(6)
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(if code.language.is_empty() {
                                    "code"
                                } else {
                                    &code.language
                                })
                                .color(theme::TEXT_INFO)
                                .size(theme::UI_FONT_SIZE - 2.0),
                            );
                            if done {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.spacing_mut().item_spacing.x = 2.0;
                                        if ui
                                            .small_button("Apply…")
                                            .on_hover_text(
                                                "Preview the change as a diff, then apply it",
                                            )
                                            .clicked()
                                        {
                                            actions.push(Action::Apply(turn, index));
                                        }
                                        if ui
                                            .small_button("Insert")
                                            .on_hover_text("Insert at the caret")
                                            .clicked()
                                        {
                                            actions.push(Action::Insert(turn, index));
                                        }
                                        if icons::button(ui, Icon::Copy, "Copy").clicked() {
                                            actions.push(Action::Copy(turn, index));
                                        }
                                    },
                                );
                            }
                        });
                        egui::ScrollArea::horizontal()
                            .id_salt(("ai_code", turn, index))
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(highlighted(&code.language, &code.code))
                                        .extend(),
                                );
                            });
                    });
            }
        }
        ui.add_space(4.0);
    }
}

/// Code block text with the editor's syntax colors when the fence names a known language.
fn highlighted(language: &str, code: &str) -> LayoutJob {
    let base = egui::TextFormat {
        font_id: egui::FontId::monospace(theme::EDITOR_FONT_SIZE - 1.0),
        color: *theme::editor::TEXT,
        ..Default::default()
    };
    let analysis = archipel_ide_language::registry::by_id(language)
        .and_then(|spec| spec.extensions.first())
        .map(|extension| {
            archipel_ide_language::analyze(
                std::path::Path::new(&format!("snippet.{extension}")),
                code,
            )
        })
        .unwrap_or_default();
    let mut job = LayoutJob::default();
    let mut cursor = 0;
    for span in analysis.highlights.spans() {
        let (start, end) = (span.start_byte, span.end_byte.min(code.len()));
        if start < cursor
            || start >= end
            || !code.is_char_boundary(start)
            || !code.is_char_boundary(end)
        {
            continue;
        }
        job.append(&code[cursor..start], 0.0, base.clone());
        job.append(
            &code[start..end],
            0.0,
            crate::syntax::text_format(span.key, &base),
        );
        cursor = end;
    }
    job.append(&code[cursor..], 0.0, base);
    job
}

/// Line diff with old/new line numbers and added/removed tinting.
pub(crate) fn diff_view(ui: &mut egui::Ui, diff: &[DiffLine], max_height: f32, id: &str) {
    let row_height = 18.0;
    egui::ScrollArea::both()
        .id_salt(id)
        .max_height(max_height)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            if diff.is_empty() {
                ui.label(RichText::new("No textual change.").color(theme::TEXT_INFO));
            }
            for line in diff {
                let (fill, sign, color) = match line.kind {
                    DiffKind::Added => (
                        Color32::from_rgba_unmultiplied(0x4E, 0x9D, 0x6C, 0x40),
                        "+",
                        theme::editor::TEXT,
                    ),
                    DiffKind::Removed => (
                        Color32::from_rgba_unmultiplied(0xC5, 0x4E, 0x58, 0x40),
                        "−",
                        theme::editor::TEXT,
                    ),
                    DiffKind::Skipped => (Color32::TRANSPARENT, " ", theme::TEXT_INFO),
                    DiffKind::Context => (Color32::TRANSPARENT, " ", theme::TEXT_SECONDARY),
                };
                let number = |n: Option<usize>| {
                    n.map(|n| format!("{n:>4}"))
                        .unwrap_or_else(|| "    ".into())
                };
                let text = format!(
                    "{} {} {} {}",
                    number(line.old_line),
                    number(line.new_line),
                    sign,
                    line.text
                );
                let font = egui::FontId::monospace(theme::EDITOR_FONT_SIZE - 1.0);
                let galley = ui.painter().layout_no_wrap(text, font, *color);
                let (rect, _) = ui.allocate_exact_size(
                    vec2(ui.available_width().max(galley.size().x + 8.0), row_height),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(rect, 0.0, fill);
                ui.painter().galley(
                    egui::pos2(rect.left() + 4.0, rect.center().y - galley.size().y / 2.0),
                    galley,
                    *color,
                );
            }
        });
}

/// Renders Markdown without code-block actions (agent messages).
pub(crate) fn markdown_readonly(ui: &mut egui::Ui, text: &str) {
    markdown(ui, usize::MAX, text, false, &mut Vec::new());
}

/// Modal diff preview for "Apply…".
pub fn diff_preview(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(preview) = app.state.ai.pending_apply.as_ref() else {
        return;
    };
    let (added, removed) = stats(&preview.diff);
    let path = preview.path.display().to_string();
    let diff = preview.diff.clone();
    let mut accept = false;
    let mut cancel = false;
    let screen = ctx.content_rect();
    let width = (screen.width() - 80.0).clamp(360.0, 820.0);
    egui::Area::new(egui::Id::new("ai_apply_preview"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(
            screen.center().x - width / 2.0,
            screen.top() + screen.height() * 0.12,
        ))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .corner_radius(10)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(width - 24.0);
                    ui.horizontal(|ui| {
                        icons::show(ui, Icon::Ai, 16.0, theme::MODIFIED);
                        ui.label(
                            RichText::new(format!("Apply AI Suggestion to {path}"))
                                .font(theme::semibold(theme::UI_FONT_SIZE))
                                .color(theme::TEXT_BRIGHT),
                        );
                        ui.label(RichText::new(format!("+{added}")).color(theme::ADDED));
                        ui.label(RichText::new(format!("−{removed}")).color(theme::UNTRACKED));
                    });
                    ui.add_space(8.0);
                    diff_view(
                        ui,
                        &diff,
                        (screen.height() * 0.55).max(160.0),
                        "ai_apply_diff",
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Accept").color(Color32::WHITE))
                                    .fill(theme::ACCENT),
                            )
                            .clicked()
                        {
                            accept = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                        ui.label(
                            RichText::new("The change is applied as one undoable edit.")
                                .color(theme::TEXT_INFO),
                        );
                    });
                });
        });
    if accept {
        app.dispatch("ai.chat.applyConfirm", Value::Null);
        ctx.memory_mut(|memory| memory.request_focus(crate::editor::editor_id()));
    } else if cancel {
        app.dispatch("ai.chat.applyCancel", Value::Null);
    }
}

/// Status bar AI widget: provider/model or "off"; toggles the AI Chat window.
pub fn status_widget(app: &mut IdeApp, ui: &mut egui::Ui, font: egui::FontId) {
    let ai = &app.state.ai;
    let (text, color, tooltip) = if !ai.policy.allow_ai {
        (
            String::new(),
            theme::TEXT_INFO,
            format!("AI is disabled for this project ({PROJECT_POLICY_FILE})"),
        )
    } else if !ai.is_enabled() {
        (
            String::new(),
            theme::TEXT_INFO,
            "AI Assistant is off — click to set up a provider".to_owned(),
        )
    } else if ai.chat.is_streaming() {
        (
            "AI: generating…".to_owned(),
            theme::TEXT_SECONDARY,
            ai.summary(),
        )
    } else {
        let model = ai.summary();
        (
            String::new(),
            theme::TEXT_SECONDARY,
            format!("AI Assistant: {model}"),
        )
    };
    let icon_color = if ai.is_enabled() {
        theme::MODIFIED
    } else {
        theme::TEXT_INFO
    };
    let response = widgets::flat_widget(
        ui,
        Some((Icon::Ai, *icon_color)),
        &text,
        font,
        color,
        false,
        22.0,
    );
    if response.on_hover_text(tooltip).clicked() {
        app.dispatch("toolwindow.activate.aiChat", Value::Null);
    }
}

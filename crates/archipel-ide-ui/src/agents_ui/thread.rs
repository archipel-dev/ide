//! Thread tabs, the native transcript and the inspector.
//!
//! One tab per open agent thread (the kit's editor-style `JBTabs`); the active tab shows
//! the transcript at a comfortable reading width with a reply field at the bottom. The
//! inspector on the right always reflects the active tab.

use super::composer;
use super::*;
use archipel_ide_core::agent_events::{
    AgentEvent, DiffLine, PERMISSION_OPTIONS, ToolKind, ToolStatus,
};
use archipel_kit::parts::Surface;
use archipel_kit::tokens::radius;
use archipel_kit::ui::tabs::{JBTabs, TabInfo};
use archipel_kit::ui::{AsyncProcessIcon, PropertyGrid, TitledSeparator};

const READING_WIDTH: f32 = 760.0;
/// Lines of tool output shown before the rest is folded.
const OUTPUT_LINES: usize = 14;

/// What kind of row an event becomes in the transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowKind {
    User,
    Text,
    Thinking,
    Tool,
    /// A tool call with an inline mini diff.
    Edit,
    /// A permission card that waits for the user.
    Question,
    /// A permission request that was answered (one compact row).
    Answered,
    Subagent,
    Result,
    Error,
}

pub(super) fn row_kind(event: &AgentEvent) -> RowKind {
    match event {
        AgentEvent::UserPrompt(_) => RowKind::User,
        AgentEvent::AssistantText(_) => RowKind::Text,
        AgentEvent::Thinking(_) => RowKind::Thinking,
        AgentEvent::ToolCall { edit: Some(_), .. } => RowKind::Edit,
        AgentEvent::ToolCall { .. } => RowKind::Tool,
        AgentEvent::PermissionRequest { answer: None, .. } => RowKind::Question,
        AgentEvent::PermissionRequest { .. } => RowKind::Answered,
        AgentEvent::SubagentSpawned { .. } => RowKind::Subagent,
        AgentEvent::TurnResult { .. } => RowKind::Result,
        AgentEvent::Error(_) => RowKind::Error,
    }
}

pub(super) fn tool_concept(kind: ToolKind) -> Concept {
    match kind {
        ToolKind::Read => Concept::ToolRead,
        ToolKind::Edit => Concept::ToolEdit,
        ToolKind::Write => Concept::ToolWrite,
        ToolKind::Bash => Concept::ToolBash,
        ToolKind::Search => Concept::ToolSearch,
        ToolKind::Spawn => Concept::ToolSpawn,
        ToolKind::Other => Concept::ToolOther,
    }
}

/// `4.1k` style token count.
pub(super) fn tokens_text(tokens: u32) -> String {
    if tokens >= 1_000 {
        format!("{:.1}k", f64::from(tokens) / 1_000.0)
    } else {
        tokens.to_string()
    }
}

fn seconds_text(duration: std::time::Duration) -> String {
    let s = duration.as_secs_f32();
    if s >= 60.0 {
        format!("{}m {:02}s", s as u32 / 60, s as u32 % 60)
    } else {
        format!("{s:.1}s")
    }
}

// ---------------------------------------------------------------------------
// Tabs and transcript
// ---------------------------------------------------------------------------

pub(super) fn threads(app: &mut IdeApp, ui: &mut egui::Ui) {
    let sessions = app.state.agents.sessions();
    let open: Vec<&SessionInfo> = app
        .state
        .agents
        .threads
        .iter()
        .filter_map(|id| sessions.iter().find(|s| s.id == *id))
        .collect();
    let draft = app.state.agents.new_thread;
    let draft_active = draft && app.state.agents.draft_active;
    if open.is_empty() && !draft {
        let shown = StatusText::new("No thread open")
            .hint("Pick an agent in the list, or start a new thread.")
            .icon(glyph(Concept::Agent))
            .action("New thread")
            .show(ui);
        if shown.clicked() {
            app.dispatch("agents.newThread", Value::Null);
        }
        return;
    }
    let active = if draft_active {
        open.len()
    } else {
        app.state
            .agents
            .selected
            .and_then(|id| open.iter().position(|s| s.id == id))
            .unwrap_or(0)
    };
    let mut tabs: Vec<TabInfo> = open
        .iter()
        .map(|s| {
            TabInfo::new(s.name.clone())
                .icon(look(s.state).0)
                .closable(true)
                .modified(s.unread)
        })
        .collect();
    if draft {
        tabs.push(TabInfo::new("New thread").icon(Glyph::Plus).closable(true));
    }
    let mut actions = Vec::new();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let out = JBTabs::new(&tabs, active).show(ui);
        if let Some(index) = out.closed {
            actions.push(match open.get(index) {
                Some(s) => Action::CloseThread(s.id),
                None => Action::CloseDraft,
            });
        } else if let Some(index) = out.selected {
            actions.push(match open.get(index) {
                Some(s) => Action::Select(s.id),
                None => Action::NewThread,
            });
        }
        if ui
            .add(ActionButton::new(Glyph::Plus, "New agent thread"))
            .clicked()
        {
            actions.push(Action::NewThread);
        }
    });
    // The composer of a tab that just opened takes the keyboard.
    if draft_active && !app.agents_ui.was_draft {
        app.agents_ui.focus_composer = true;
    }
    app.agents_ui.was_draft = draft_active;
    if draft_active {
        draft_view(app, ui, &mut actions);
    } else {
        let info = open[active].clone();
        thread_view(app, ui, &info, &mut actions);
    }
    apply(app, actions);
}

/// The transcript column: centered, at reading width.
fn column(ui: &mut egui::Ui, id: impl std::hash::Hash) -> egui::Ui {
    let area = ui.available_rect_before_wrap();
    ui.allocate_rect(area, Sense::hover());
    let width = area.width().min(READING_WIDTH);
    let column = Rect::from_min_max(
        egui::pos2(area.center().x - width / 2.0, area.top()),
        egui::pos2(area.center().x + width / 2.0, area.bottom()),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(id)
            .max_rect(column)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(column.intersect(ui.clip_rect()));
    child
}

fn composer_panel(
    app: &mut IdeApp,
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    target: Option<&SessionInfo>,
    actions: &mut Vec<Action>,
) {
    egui::TopBottomPanel::bottom(egui::Id::new(("agents_composer_panel", id)))
        .show_separator_line(false)
        .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(12, 10)))
        .show_inside(ui, |ui| composer::composer(app, ui, target, actions));
}

fn thread_view(app: &mut IdeApp, ui: &mut egui::Ui, info: &SessionInfo, actions: &mut Vec<Action>) {
    let id = info.id;
    let mut child = column(ui, ("agents_thread", id));
    let events: Vec<AgentEvent> = app.state.agents.backend.events(id).to_vec();
    composer_panel(app, &mut child, id, Some(info), actions);
    let expanded = app.agents_ui.expanded.clone();
    egui::ScrollArea::vertical()
        .id_salt(("agents_transcript", id))
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 8.0;
                    transcript(ui, info, &events, &expanded, actions);
                });
        });
}

/// Suggestions drawn from the project: a verb and a file.
pub(super) fn suggestions(app: &IdeApp) -> Vec<String> {
    let names: Vec<String> = app
        .state
        .files
        .iter()
        .filter(|entry| !entry.is_dir)
        .map(|entry| IdeApp::file_name(&entry.relative_path))
        .filter(|name| {
            [".java", ".kt", ".rs", ".ts", ".py", ".go", ".js"]
                .iter()
                .any(|ext| name.ends_with(ext))
        })
        .take(2)
        .collect();
    match names.as_slice() {
        [first, second] => vec![
            format!("Explain how {first} works"),
            format!("Add tests for {second}"),
            "Find and fix bugs".to_owned(),
        ],
        [first] => vec![
            format!("Explain how {first} works"),
            format!("Add tests for {first}"),
        ],
        _ => vec![
            "Explain the project structure".to_owned(),
            "Find and fix bugs".to_owned(),
        ],
    }
}

/// The empty new-thread tab: a heading, suggestion chips and the composer.
fn draft_view(app: &mut IdeApp, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    let mut child = column(ui, "agents_draft");
    composer_panel(app, &mut child, "draft", None, actions);
    let suggestions = suggestions(app);
    let space = (child.available_height() * 0.3 - 40.0).max(20.0);
    child.add_space(space);
    let mut picked = None;
    child.vertical_centered(|ui| {
        let colors = Theme::colors(ui.ctx());
        icons::show(
            ui,
            Icon::Kit(glyph(Concept::Agent).into()),
            28.0,
            colors.text_secondary,
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("What should the agents work on?")
                .font(theme::semibold(theme::UI_FONT_SIZE + 5.0))
                .color(colors.text_strong),
        );
        ui.label(
            egui::RichText::new(
                "Describe a task below. Pick several agents to compare their results.",
            )
            .color(colors.text_secondary),
        );
        ui.add_space(14.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            for text in &suggestions {
                if ui
                    .add(Button::secondary(text.as_str()).small(true).min_width(0.0))
                    .clicked()
                {
                    picked = Some(text.clone());
                }
            }
        });
    });
    if let Some(text) = picked {
        let composer = app.agents_ui.composers.entry(None).or_default();
        composer.cursor = text.chars().count();
        composer.text = text;
        app.agents_ui.focus_composer = true;
    }
    let _ = actions;
}

fn transcript(
    ui: &mut egui::Ui,
    info: &SessionInfo,
    events: &[AgentEvent],
    expanded: &HashSet<(SessionId, usize)>,
    actions: &mut Vec<Action>,
) {
    let colors = Theme::colors(ui.ctx());
    let id = info.id;
    for (index, event) in events.iter().enumerate() {
        let open = expanded.contains(&(id, index));
        if matches!(row_kind(event), RowKind::User | RowKind::Result) {
            ui.add_space(4.0);
        }
        match event {
            AgentEvent::UserPrompt(text) => {
                ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                    let max = ui.available_width() * 0.85;
                    Surface::Field
                        .frame(ui.ctx())
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.set_max_width(max);
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(text).color(colors.text_strong),
                                )
                                .wrap(),
                            );
                        });
                });
            }
            AgentEvent::AssistantText(text) => {
                crate::ai_chat::render_markdown(ui, id as usize * 10_000 + index, text);
            }
            AgentEvent::Thinking(text) => {
                let row = ColoredListCellRenderer::new("Thinking")
                    .icon(glyph(Concept::Thinking))
                    .label_tone(Tone::Muted)
                    .expanded(Some(open))
                    .show(ui);
                if row.toggled || row.response.clicked() {
                    actions.push(Action::ToggleRow(id, index));
                }
                if open {
                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(text)
                                    .italics()
                                    .color(colors.text_secondary),
                            )
                            .wrap(),
                        );
                    });
                }
            }
            AgentEvent::ToolCall {
                kind,
                title,
                args,
                status,
                result,
                output,
                edit,
                ..
            } => {
                tool_row(
                    ui,
                    id,
                    index,
                    (*kind, title, args, *status, result.as_deref()),
                    output.as_deref(),
                    open,
                    actions,
                );
                if let Some(edit) = edit {
                    mini_diff(ui, id, edit, actions);
                }
            }
            AgentEvent::PermissionRequest {
                question,
                options,
                answer,
                ..
            } => match answer {
                None => permission_card(ui, id, question, options, actions),
                Some(choice) => {
                    let deny = choice == PERMISSION_OPTIONS[2];
                    let width = ui.available_width();
                    let text = elide(ui, &format!("{choice}: {question}"), width - 60.0);
                    ColoredListCellRenderer::new(text)
                        .icon(if deny {
                            glyph(Concept::Failed)
                        } else {
                            glyph(Concept::Permission)
                        })
                        .icon_tone(if deny { Tone::Error } else { Tone::Success })
                        .label_tone(Tone::Secondary)
                        .show(ui);
                }
            },
            AgentEvent::SubagentSpawned { child, name } => {
                let row = ColoredListCellRenderer::new(format!("Started agent {name}"))
                    .icon(glyph(Concept::Subagent))
                    .icon_tone(Tone::Accent)
                    .trailing(Slot::text_tone("Open thread", Tone::Link))
                    .show(ui);
                if row.response.clicked() {
                    actions.push(Action::Show(*child));
                }
            }
            AgentEvent::TurnResult {
                files,
                added,
                removed,
                duration,
                tokens,
            } => {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    icons::show(
                        ui,
                        Icon::Kit(glyph(Concept::Result).into()),
                        14.0,
                        Tone::Success.color(&colors),
                    );
                    ui.add(Tag::new("Done").tone(Tone::Success));
                    ui.add(Tag::new(format!(
                        "{} {}",
                        widgets::pluralize(*files, "file", "files"),
                        stats_text(*added, *removed)
                    )));
                    ui.add(Tag::new(seconds_text(*duration)));
                    ui.add(Tag::new(format!("{} tokens", tokens_text(*tokens))));
                });
            }
            AgentEvent::Error(message) => {
                InlineBanner::new(Status::Error, message.as_str()).show(ui);
            }
        }
    }
    if info.state == AgentState::Working {
        ui.horizontal(|ui| {
            ui.add(AsyncProcessIcon::new().size(14.0).label("Working"));
            ui.label(egui::RichText::new("Working").color(colors.text_secondary));
        });
    }
}

/// A one-line tool row: icon, verb and target, outcome; expands to the output.
fn tool_row(
    ui: &mut egui::Ui,
    session: SessionId,
    index: usize,
    call: (ToolKind, &str, &str, ToolStatus, Option<&str>),
    output: Option<&str>,
    open: bool,
    actions: &mut Vec<Action>,
) {
    let (kind, title, args, status, result) = call;
    let colors = Theme::colors(ui.ctx());
    let tone = match status {
        ToolStatus::Running => Tone::Accent,
        ToolStatus::Ok => Tone::Icon,
        ToolStatus::Error => Tone::Error,
    };
    let result_tone = if status == ToolStatus::Error {
        Tone::Error
    } else {
        Tone::Success
    };
    let width = ui.available_width();
    let trailing = match (status, result) {
        (ToolStatus::Running, _) => 24.0,
        (_, Some(text)) => text_width(ui, text) + 10.0,
        _ => 0.0,
    };
    let room = width - 76.0 - trailing;
    let label = elide(ui, title, room);
    let left = room - text_width(ui, &label) - 14.0;
    let mut row = ColoredListCellRenderer::new(label)
        .icon(glyph(tool_concept(kind)))
        .icon_tone(tone)
        .expanded(output.map(|_| open));
    if left > 70.0 {
        row = row.secondary(elide(ui, args, left));
    }
    row = match (status, result) {
        (ToolStatus::Running, _) => row.trailing(Slot::ui_width(24.0, |ui| {
            ui.add(AsyncProcessIcon::new().size(12.0).label("Running"));
        })),
        (_, Some(text)) => row.trailing(Slot::text_tone(text, result_tone)),
        _ => row,
    };
    let shown = row.show(ui);
    if output.is_some() && (shown.toggled || shown.response.clicked()) {
        actions.push(Action::ToggleRow(session, index));
    }
    if let (true, Some(output)) = (open, output) {
        Surface::Field
            .frame(ui.ctx())
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let total = output.lines().count();
                for line in output.lines().take(OUTPUT_LINES) {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(line)
                                .monospace()
                                .color(colors.text_secondary),
                        )
                        .truncate(),
                    );
                }
                if total > OUTPUT_LINES {
                    ui.label(
                        egui::RichText::new(format!("{} more lines", total - OUTPUT_LINES))
                            .color(colors.text_placeholder),
                    );
                }
            });
    }
}

/// The edit's inline diff with +/- counts and an "Open diff" button.
fn mini_diff(
    ui: &mut egui::Ui,
    session: SessionId,
    edit: &archipel_ide_core::agent_events::EditSummary,
    actions: &mut Vec<Action>,
) {
    let colors = Theme::colors(ui.ctx());
    Surface::Field
        .frame(ui.ctx())
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 1.0;
            for DiffLine { sign, text } in &edit.snippet {
                let color = match sign {
                    '+' => Tone::Added.color(&colors),
                    '-' => Tone::Error.color(&colors),
                    _ => colors.text_secondary,
                };
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(format!("{sign} {text}"))
                            .monospace()
                            .color(color),
                    )
                    .truncate(),
                );
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} {}",
                        IdeApp::file_name(&edit.path),
                        stats_text(edit.added, edit.removed)
                    ))
                    .color(colors.text_secondary),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(Button::ghost("Open diff").small(true).min_width(0.0))
                        .clicked()
                    {
                        actions.push(Action::Review(session, Some(edit.path.clone())));
                    }
                });
            });
        });
}

/// The highlighted card of a permission request, with real buttons.
fn permission_card(
    ui: &mut egui::Ui,
    session: SessionId,
    question: &str,
    options: &[String],
    actions: &mut Vec<Action>,
) {
    let colors = Theme::colors(ui.ctx());
    let tint = colors.banner_warning;
    egui::Frame::new()
        .fill(tint.background)
        .stroke(Stroke::new(1.0_f32, tint.border))
        .corner_radius(radius::BANNER)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.horizontal(|ui| {
                icons::show(
                    ui,
                    Icon::Kit(glyph(Concept::Permission).into()),
                    16.0,
                    Tone::Warning.color(&colors),
                );
                ui.label(
                    egui::RichText::new("Permission needed")
                        .font(theme::semibold(theme::UI_FONT_SIZE))
                        .color(colors.text_strong),
                );
            });
            ui.add(egui::Label::new(egui::RichText::new(question).color(colors.text)).wrap());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for (index, option) in options.iter().enumerate() {
                    let button = match index {
                        0 => Button::primary(option.as_str()),
                        1 => Button::secondary(option.as_str()),
                        _ => Button::ghost(option.as_str()),
                    };
                    if ui.add(button.small(true).min_width(0.0)).clicked() {
                        actions.push(Action::Permission(session, index));
                    }
                }
            });
        });
}

// ---------------------------------------------------------------------------
// Inspector
// ---------------------------------------------------------------------------

/// The right panel: always the active thread tab (status, worktree, changes, sub-threads,
/// open questions, actions).
pub(super) fn inspector(app: &mut IdeApp, ui: &mut egui::Ui) {
    panel_header(ui, "Inspector", |_| {});
    if app.state.agents.new_thread && app.state.agents.draft_active {
        StatusText::new("New thread")
            .hint("Choose the agents and describe the task. The inspector follows the thread once it starts.")
            .show(ui);
        return;
    }
    let Some(info) = app
        .state
        .agents
        .selected
        .and_then(|id| app.state.agents.session(id))
    else {
        StatusText::new("No thread selected")
            .hint("The inspector shows the active thread.")
            .show(ui);
        return;
    };
    let colors = Theme::colors(ui.ctx());
    let changes = app.state.agents.backend.changes(info.id);
    let sessions = app.state.agents.sessions();
    let children: Vec<&SessionInfo> = sessions
        .iter()
        .filter(|s| s.parent == Some(info.id))
        .collect();
    let question = app
        .state
        .agents
        .backend
        .events(info.id)
        .iter()
        .find_map(|event| match event {
            AgentEvent::PermissionRequest {
                question,
                answer: None,
                ..
            } => Some(question.clone()),
            _ => None,
        });
    let mut actions = Vec::new();

    egui::TopBottomPanel::bottom("agents_inspector_actions")
        .show_separator_line(false)
        .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(
            theme::layout::CONTENT_INSET as i8,
            8,
        )))
        .show_inside(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let width = ui.available_width();
            let has = !changes.is_empty();
            let path = app
                .agents_ui
                .changes_selected
                .and_then(|i| changes.get(i))
                .map(|c| c.path.clone());
            if ui
                .add(
                    if has {
                        Button::primary("Review Changes")
                    } else {
                        Button::secondary("Review Changes").enabled(false)
                    }
                    .icon(glyph(Concept::Review))
                    .min_width(width),
                )
                .clicked()
            {
                actions.push(Action::Review(info.id, path.clone()));
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let half = ((width - 6.0) / 2.0).max(0.0);
                if ui
                    .add(
                        Button::secondary("Open in Code")
                            .enabled(has)
                            .min_width(half),
                    )
                    .clicked()
                {
                    actions.push(Action::OpenInCode(info.id, path.clone()));
                }
                let finished = !info.state.is_active() && info.outcome.is_none() && has;
                if ui
                    .add(
                        Button::secondary("Merge\u{2026}")
                            .icon(glyph(Concept::Merge))
                            .enabled(finished)
                            .min_width(half),
                    )
                    .clicked()
                {
                    actions.push(Action::Merge(info.id, MergeStrategy::Squash));
                }
            });
        });

    egui::ScrollArea::vertical()
        .id_salt("agents_inspector_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(
                    theme::layout::CONTENT_INSET as i8,
                    0,
                ))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.add(state_tag(&info));
                        ui.label(
                            egui::RichText::new(info.profile.id())
                                .font(theme::semibold(theme::UI_FONT_SIZE))
                                .color(colors.text_strong),
                        );
                    });
                    ui.add_space(6.0);
                    PropertyGrid::new(("agents_props", info.id))
                        .row("Agent", info.profile.product())
                        .mono_row("Worktree", info.worktree.clone())
                        .mono_row("Branch", info.branch.clone())
                        .row("Elapsed", elapsed(&info))
                        .row("Tokens", tokens_text(info.tokens))
                        .show(ui);
                    ui.label(
                        egui::RichText::new(format!("via {}", info.profile.protocol()))
                            .color(colors.text_placeholder),
                    );
                    if let Some(question) = &question {
                        ui.add(TitledSeparator::new("Waiting for your answer"));
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(question).color(Tone::Warning.color(&colors)),
                            )
                            .wrap(),
                        );
                    }
                    ui.add(TitledSeparator::new(format!(
                        "Changed files ({})",
                        changes.len()
                    )));
                    if changes.is_empty() {
                        ui.label(
                            egui::RichText::new("No changes yet").color(colors.text_placeholder),
                        );
                    }
                    let labels = change_labels(ui, &changes, ui.available_width());
                    let queued: RefCell<Vec<Action>> = RefCell::new(Vec::new());
                    let mut make = change_rows(&changes, &labels, &queued, info.id);
                    #[allow(clippy::needless_range_loop)]
                    for index in 0..changes.len() {
                        let selected = app.agents_ui.changes_selected == Some(index);
                        let row = make(index).selected(selected).show(ui);
                        if row.response.clicked() {
                            app.agents_ui.changes_selected = Some(index);
                        }
                        if row.response.double_clicked() {
                            actions
                                .push(Action::Review(info.id, Some(changes[index].path.clone())));
                        }
                    }
                    drop(make);
                    actions.extend(queued.into_inner());
                    if !children.is_empty() {
                        ui.add(TitledSeparator::new(format!(
                            "Sub-threads ({})",
                            children.len()
                        )));
                        for child in &children {
                            let (glyph_, tone) = look(child.state);
                            let row = ColoredListCellRenderer::new(child.name.clone())
                                .flat(true)
                                .icon(glyph_)
                                .icon_tone(tone)
                                .secondary(child.state.label())
                                .show(ui);
                            if row.response.clicked() {
                                actions.push(Action::Show(child.id));
                            }
                        }
                    }
                });
        });
    apply(app, actions);
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_core::agent_events::EditSummary;

    #[test]
    fn events_map_to_rows() {
        let edit = EditSummary {
            path: "a.rs".into(),
            added: 1,
            removed: 0,
            snippet: vec![],
        };
        let call = |edit| AgentEvent::ToolCall {
            id: 1,
            kind: ToolKind::Edit,
            title: "Edit a.rs".into(),
            args: "a.rs".into(),
            status: ToolStatus::Ok,
            result: None,
            output: None,
            edit,
        };
        assert_eq!(row_kind(&call(None)), RowKind::Tool);
        assert_eq!(row_kind(&call(Some(edit))), RowKind::Edit);
        let question = |answer: Option<&str>| AgentEvent::PermissionRequest {
            id: 1,
            question: "?".into(),
            options: vec![],
            answer: answer.map(str::to_owned),
        };
        assert_eq!(row_kind(&question(None)), RowKind::Question);
        assert_eq!(row_kind(&question(Some("Deny"))), RowKind::Answered);
        assert_eq!(row_kind(&AgentEvent::Error("x".into())), RowKind::Error);
        assert_eq!(
            row_kind(&AgentEvent::SubagentSpawned {
                child: 2,
                name: "x".into()
            }),
            RowKind::Subagent
        );
        assert_eq!(tool_concept(ToolKind::Bash), Concept::ToolBash);
        assert_eq!(tokens_text(4_130), "4.1k");
        assert_eq!(tokens_text(900), "900");
    }
}

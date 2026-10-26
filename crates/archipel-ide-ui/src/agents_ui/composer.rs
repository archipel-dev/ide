//! The composer: the one input of Agents mode.
//!
//! It sits at the bottom of the active thread tab (inside the transcript column) and talks
//! to that thread: reply, answer a question, follow up. On the "new thread" tab it starts
//! agents instead, and there (only) it chooses which agents, so fan-out opens one tab per
//! agent. Enter sends, Shift+Enter inserts a newline, Up on an empty box recalls the last
//! prompt, Esc leaves the box, `@` completes files, `/` completes slash commands.

use super::*;
use archipel_kit::parts::Surface;

/// Agent and model choices of the selector chip.
pub(super) const ENTRIES: [(AgentProfile, &str); 5] = [
    (AgentProfile::Claude, "sonnet"),
    (AgentProfile::Claude, "opus"),
    (AgentProfile::Codex, "gpt-5-codex"),
    (AgentProfile::Gemini, "gemini-2.5-pro"),
    (AgentProfile::Opencode, "default model"),
];

/// Mock slash commands.
pub(super) const SLASH: [(&str, &str); 5] = [
    ("/review", "Review the changes so far"),
    ("/test", "Run the test suite and fix failures"),
    ("/plan", "Write a plan before editing anything"),
    ("/compact", "Summarize the thread to free context"),
    ("/clear", "Start over in this thread"),
];

/// Mock issues for the `#` button.
pub(super) const ISSUES: [(&str, &str); 4] = [
    ("#128", "Tabs reorder when the active tab closes"),
    ("#131", "Add OAuth login with a device code"),
    ("#142", "Storage layer leaks connections"),
    ("#150", "Flaky retry test on CI"),
];

/// Longest the text area grows before it scrolls.
const MAX_LINES: f32 = 8.0;
const COMPLETION_ROWS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PermMode {
    Ask,
    AutoEdit,
    Plan,
}

impl PermMode {
    const ALL: [PermMode; 3] = [PermMode::Ask, PermMode::AutoEdit, PermMode::Plan];

    fn label(self) -> &'static str {
        match self {
            PermMode::Ask => "Ask",
            PermMode::AutoEdit => "Auto-edit",
            PermMode::Plan => "Plan",
        }
    }
}

pub(super) struct Composer {
    pub text: String,
    /// Attached files (`@path`) and issues (`#128`).
    pub chips: Vec<String>,
    /// New thread only: which of [`ENTRIES`] to start.
    pub selected: [bool; 5],
    /// Existing thread: the model picked in the selector (display only in the mock).
    pub model: Option<usize>,
    pub mode: PermMode,
    /// Highlighted row of the completion popup.
    pub completion: usize,
    /// Esc closed the popup for the text as it is now.
    pub dismissed: bool,
    /// Caret as a character index, from the last frame.
    pub cursor: usize,
    /// Position in the prompt history while recalling with Up.
    pub recall: Option<usize>,
    pub attach_filter: String,
}

impl Default for Composer {
    fn default() -> Self {
        Self {
            text: String::new(),
            chips: Vec::new(),
            selected: [true, false, false, false, false],
            model: None,
            mode: PermMode::Ask,
            completion: 0,
            dismissed: false,
            cursor: 0,
            recall: None,
            attach_filter: String::new(),
        }
    }
}

impl Composer {
    /// Agents to start for a new thread, each once, in [`AgentProfile::ALL`] order.
    pub fn profiles(&self) -> Vec<AgentProfile> {
        AgentProfile::ALL
            .into_iter()
            .filter(|profile| {
                ENTRIES
                    .iter()
                    .zip(self.selected)
                    .any(|((p, _), on)| on && p == profile)
            })
            .collect()
    }

    /// Text plus attachments, as the prompt sent to the agent.
    pub fn prompt(&self) -> String {
        let mut prompt = self.text.trim().to_owned();
        for chip in &self.chips {
            if !prompt.is_empty() {
                prompt.push(' ');
            }
            prompt.push_str(chip);
        }
        prompt
    }

    fn clear(&mut self) {
        self.text.clear();
        self.chips.clear();
        self.recall = None;
        self.cursor = 0;
    }
}

// ---------------------------------------------------------------------------
// Completion
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CompletionKind {
    File,
    Slash,
}

/// The `@file` or `/command` token the caret is in: kind, start (character index) and the
/// text typed after the trigger character.
pub(super) fn token_at(text: &str, cursor: usize) -> Option<(CompletionKind, usize, String)> {
    let chars: Vec<char> = text.chars().collect();
    let cursor = cursor.min(chars.len());
    let mut start = cursor;
    while start > 0 && !chars[start - 1].is_whitespace() {
        start -= 1;
    }
    let token: String = chars[start..cursor].iter().collect();
    match token.chars().next()? {
        '@' => Some((CompletionKind::File, start, token[1..].to_owned())),
        // Slash commands only at the very start of the message.
        '/' if chars[..start].iter().all(|c| c.is_whitespace()) => {
            Some((CompletionKind::Slash, start, token[1..].to_owned()))
        }
        _ => None,
    }
}

pub(super) fn slash_matches(query: &str) -> Vec<(&'static str, &'static str)> {
    SLASH
        .iter()
        .copied()
        .filter(|(name, _)| name[1..].starts_with(&query.to_lowercase()))
        .collect()
}

/// Project files matching `query` (file name first, then path), at most `limit`.
pub(super) fn file_matches(files: &[PathBuf], query: &str, limit: usize) -> Vec<PathBuf> {
    let query = query.to_lowercase();
    let name = |path: &PathBuf| {
        path.file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    };
    let mut hits: Vec<(u8, &PathBuf)> = files
        .iter()
        .filter_map(|path| {
            if query.is_empty() {
                return Some((2, path));
            }
            if name(path).starts_with(&query) {
                Some((0, path))
            } else if name(path).contains(&query) {
                Some((1, path))
            } else if path.to_string_lossy().to_lowercase().contains(&query) {
                Some((2, path))
            } else {
                None
            }
        })
        .collect();
    hits.sort_by_key(|(rank, path)| (*rank, path.components().count()));
    hits.into_iter()
        .take(limit)
        .map(|(_, path)| path.clone())
        .collect()
}

/// `text` with the characters `start..end` replaced by `with`; the new caret index.
fn splice(text: &mut String, start: usize, end: usize, with: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let (start, end) = (start.min(chars.len()), end.min(chars.len()));
    let mut out: String = chars[..start].iter().collect();
    out.push_str(with);
    out.extend(chars[end..].iter());
    *text = out;
    start + with.chars().count()
}

/// Whether the composer has keyboard focus (shell chords stay out of its way).
pub(crate) fn focused(ctx: &egui::Context) -> bool {
    ctx.memory(|memory| memory.has_focus(field_id()))
}

fn field_id() -> egui::Id {
    egui::Id::new("agents_composer_field")
}

/// Removes an unmodified (no Shift) Enter press and reports whether there was one.
fn take_enter(ui: &mut egui::Ui) -> bool {
    ui.input_mut(|input| {
        let before = input.events.len();
        input.events.retain(|event| {
            !matches!(
                event,
                egui::Event::Key { key: egui::Key::Enter, pressed: true, modifiers, .. }
                    if modifiers.matches_exact(egui::Modifiers::NONE)
            )
        });
        input.events.len() != before
    })
}

fn take_key(ui: &mut egui::Ui, key: egui::Key) -> bool {
    ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key))
}

/// A round icon button (send / stop). The kit has no round button yet.
fn round_button(
    ui: &mut egui::Ui,
    glyph: Glyph,
    name: &str,
    fill: egui::Color32,
    icon: egui::Color32,
    enabled: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        vec2(32.0, 32.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name));
    if enabled {
        archipel_kit::cursor::on_hover_action(&response);
    }
    let fill = if enabled && response.hovered() {
        fill.gamma_multiply(0.85)
    } else {
        fill
    };
    ui.painter().circle_filled(rect.center(), 16.0, fill);
    icons::paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), vec2(16.0, 16.0)),
        Icon::Kit(glyph.into()),
        icon,
    );
    response
}

// ---------------------------------------------------------------------------
// The composer widget
// ---------------------------------------------------------------------------

/// Draws the composer for the active thread (`target`) or, with `None`, for the new thread
/// tab. What the user sends or stops is queued in `actions`.
pub(super) fn composer(
    app: &mut IdeApp,
    ui: &mut egui::Ui,
    target: Option<&SessionInfo>,
    actions: &mut Vec<Action>,
) {
    let ctx = ui.ctx().clone();
    let colors = Theme::colors(&ctx);
    let key = target.map(|t| t.id);
    let mut c = app.agents_ui.composers.remove(&key).unwrap_or_default();
    let working = target.is_some_and(|t| t.state.is_active());
    let asking = target.is_some_and(|t| t.state == AgentState::NeedsInput);
    let focus = std::mem::take(&mut app.agents_ui.focus_composer);
    let files: Vec<PathBuf> = app
        .state
        .files
        .iter()
        .filter(|entry| !entry.is_dir)
        .map(|entry| entry.relative_path.clone())
        .collect();

    // What the popup would offer for the caret position of the last frame.
    let had_focus = ctx.memory(|m| m.has_focus(field_id()));
    let token = token_at(&c.text, c.cursor).filter(|_| !c.dismissed);
    let (kind, start, items): (Option<CompletionKind>, usize, Vec<(String, String)>) = match &token
    {
        Some((CompletionKind::File, start, query)) => (
            Some(CompletionKind::File),
            *start,
            file_matches(&files, query, COMPLETION_ROWS)
                .into_iter()
                .map(|p| (p.display().to_string(), String::new()))
                .collect(),
        ),
        Some((CompletionKind::Slash, start, query)) => (
            Some(CompletionKind::Slash),
            *start,
            slash_matches(query)
                .into_iter()
                .map(|(n, d)| (n.to_owned(), d.to_owned()))
                .collect(),
        ),
        None => (None, 0, Vec::new()),
    };
    let popup_open = had_focus && !items.is_empty();
    if c.completion >= items.len() {
        c.completion = 0;
    }

    // Keys that belong to the composer, taken before the text edit sees them.
    let mut send = false;
    let mut accept = None;
    if had_focus {
        if popup_open {
            if take_key(ui, egui::Key::ArrowDown) {
                c.completion = (c.completion + 1) % items.len();
            }
            if take_key(ui, egui::Key::ArrowUp) {
                c.completion = (c.completion + items.len() - 1) % items.len();
            }
            if take_enter(ui) || take_key(ui, egui::Key::Tab) {
                accept = Some(c.completion);
            }
            if take_key(ui, egui::Key::Escape) {
                c.dismissed = true;
            }
        } else {
            if take_enter(ui) {
                send = true;
            }
            if c.text.is_empty()
                && c.chips.is_empty()
                && take_key(ui, egui::Key::ArrowUp)
                && !app.agents_ui.history.is_empty()
            {
                let last = app.agents_ui.history.len() - 1;
                let at = c.recall.map_or(last, |i| i.saturating_sub(1));
                c.recall = Some(at);
                c.text = app.agents_ui.history[at].clone();
                c.cursor = c.text.chars().count();
            }
            if take_key(ui, egui::Key::Escape) {
                ui.memory_mut(|m| m.surrender_focus(field_id()));
            }
        }
    }

    let placeholder = match target {
        _ if asking => "Answer above or type a reply".to_owned(),
        Some(t) => format!("Reply to {}\u{2026}", t.name),
        None => match c.profiles().as_slice() {
            [one] => format!("Ask {} to\u{2026}", one.id()),
            _ => "Ask the selected agents to\u{2026}".to_owned(),
        },
    };

    let mut text_changed = false;
    let mut toolbar = Toolbar::default();
    let frame = Surface::Popup
        .frame(&ctx)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 6.0;
            // Attachments.
            if !c.chips.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let mut remove = None;
                    for (index, chip) in c.chips.iter().enumerate() {
                        ui.add(Tag::new(chip.as_str()).tone(Tone::Link));
                        if ui
                            .add(
                                IconButton::new(Glyph::Close, format!("Remove {chip}")).small(true),
                            )
                            .clicked()
                        {
                            remove = Some(index);
                        }
                    }
                    if let Some(index) = remove {
                        c.chips.remove(index);
                    }
                });
            }
            // The text area: grows to MAX_LINES, then scrolls.
            let row = ui.fonts_mut(|f| f.row_height(&egui::TextStyle::Body.resolve(ui.style())));
            let out = egui::ScrollArea::vertical()
                .id_salt("agents_composer_scroll")
                .max_height(row * MAX_LINES)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    egui::TextEdit::multiline(&mut c.text)
                        .id(field_id())
                        .frame(false)
                        .desired_rows(1)
                        .desired_width(f32::INFINITY)
                        .margin(vec2(0.0, 2.0))
                        .hint_text(egui::RichText::new(&placeholder).color(colors.text_placeholder))
                        .show(ui)
                })
                .inner;
            ui.ctx().accesskit_node_builder(out.response.id, |node| {
                node.set_label(placeholder.clone());
            });
            if focus {
                out.response.request_focus();
            }
            text_changed = out.response.changed();
            if let Some(range) = out.cursor_range {
                c.cursor = range.primary.index;
            }
            // Bottom toolbar: selectors on the left, send / stop on the right.
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                toolbar = toolbar_row(app, ui, &mut c, target, &files, working, &colors);
            });
        });

    if text_changed {
        c.dismissed = false;
        c.recall = None;
    }

    // Accept a completion (keyboard, or a click in the popup below).
    let mut clicked = None;
    if let (Some(kind), true) = (kind, popup_open) {
        let rect = frame.response.rect;
        egui::Area::new(egui::Id::new("agents_completion"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::LEFT_BOTTOM)
            .fixed_pos(rect.left_top() - vec2(0.0, 6.0))
            .show(&ctx, |ui| {
                Surface::Popup.frame(&ctx).show(ui, |ui| {
                    ui.set_width(rect.width().min(440.0));
                    for (index, (label, detail)) in items.iter().enumerate() {
                        let mut row = ListRow::new(label.as_str())
                            .flat(true)
                            .selected(index == c.completion);
                        row = match kind {
                            CompletionKind::File => row.icon(Icon::for_file(label)),
                            CompletionKind::Slash => row.secondary(detail.as_str()),
                        };
                        if row.show(ui).response.clicked() {
                            clicked = Some(index);
                        }
                    }
                });
            });
    }
    if let Some(index) = accept.or(clicked) {
        let (label, _) = &items[index];
        let end = c.cursor;
        let caret = match kind {
            Some(CompletionKind::File) => {
                let chip = format!("@{label}");
                if !c.chips.contains(&chip) {
                    c.chips.push(chip);
                }
                splice(&mut c.text, start, end, "")
            }
            _ => splice(&mut c.text, start, end, &format!("{label} ")),
        };
        c.cursor = caret;
        let mut state = egui::TextEdit::load_state(&ctx, field_id()).unwrap_or_default();
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(caret),
            )));
        state.store(&ctx, field_id());
        ctx.memory_mut(|m| m.request_focus(field_id()));
    }

    // Send / stop.
    if toolbar.stop {
        if let Some(t) = target {
            actions.push(Action::Stop(t.id));
        }
    } else if (send || toolbar.send) && !c.prompt().is_empty() {
        let prompt = c.prompt();
        match target {
            None => {
                let profiles: Vec<&'static str> = c.profiles().iter().map(|p| p.id()).collect();
                if profiles.is_empty() {
                    app.state.status = "Choose at least one agent".into();
                } else {
                    actions.push(Action::Launch(prompt.clone(), profiles));
                    app.agents_ui.history.push(prompt);
                    c.clear();
                }
            }
            Some(t) => {
                if asking {
                    // A typed reply to an open question: yes / no, or a "no, instead ...".
                    match prompt.to_lowercase().as_str() {
                        "y" | "yes" | "allow" => actions.push(Action::Permission(t.id, 0)),
                        "a" | "always" => actions.push(Action::Permission(t.id, 1)),
                        "n" | "no" | "deny" => actions.push(Action::Permission(t.id, 2)),
                        _ => {
                            actions.push(Action::Permission(t.id, 2));
                            actions.push(Action::Reply(t.id, prompt.clone()));
                        }
                    }
                } else {
                    actions.push(Action::Reply(t.id, prompt.clone()));
                }
                app.agents_ui.history.push(prompt);
                c.clear();
            }
        }
        ctx.memory_mut(|m| m.request_focus(field_id()));
    }
    app.agents_ui.composers.insert(key, c);
}

#[derive(Default)]
struct Toolbar {
    send: bool,
    stop: bool,
}

fn toolbar_row(
    app: &mut IdeApp,
    ui: &mut egui::Ui,
    c: &mut Composer,
    target: Option<&SessionInfo>,
    files: &[PathBuf],
    working: bool,
    colors: &archipel_kit::theme::Colors,
) -> Toolbar {
    let mut out = Toolbar::default();
    // Agent / model selector.
    let label = match target {
        Some(t) => {
            let model = c.model.map_or_else(
                || {
                    ENTRIES
                        .iter()
                        .find(|(p, _)| *p == t.profile)
                        .map_or("", |e| e.1)
                },
                |i| ENTRIES[i].1,
            );
            format!("{} \u{b7} {model}", t.profile.id())
        }
        None => match c.profiles().as_slice() {
            [one] => {
                let model = ENTRIES
                    .iter()
                    .zip(c.selected)
                    .find(|((p, _), on)| *on && p == one)
                    .map_or("", |((_, m), _)| *m);
                format!("{} \u{b7} {model}", one.id())
            }
            many => format!("{} agents", many.len()),
        },
    };
    let chip = ui
        .add(Button::ghost(label).small(true).min_width(0.0))
        .on_hover_text(match target {
            Some(_) => "Model of this thread",
            None => "Agents to start (pick several to fan out)",
        });
    egui::Popup::menu(&chip)
        .close_behavior(if target.is_none() {
            egui::PopupCloseBehavior::CloseOnClickOutside
        } else {
            egui::PopupCloseBehavior::CloseOnClick
        })
        .frame(Surface::Popup.frame(ui.ctx()))
        .show(|ui| {
            ui.set_width(250.0);
            for (index, (profile, model)) in ENTRIES.iter().enumerate() {
                let checked = match target {
                    None => c.selected[index],
                    Some(t) => {
                        profile == &t.profile
                            && c.model.map_or(
                                ENTRIES
                                    .iter()
                                    .position(|(p, _)| p == profile)
                                    .is_some_and(|first| first == index),
                                |m| m == index,
                            )
                    }
                };
                let item = MenuItem::new(format!("{} \u{b7} {model}", profile.product_name()))
                    .checked(checked)
                    .enabled(target.is_none_or(|t| t.profile == *profile));
                if ui.add(item).clicked() {
                    match target {
                        None => {
                            c.selected[index] = !c.selected[index];
                            if !c.selected.iter().any(|on| *on) {
                                c.selected[index] = true;
                            }
                        }
                        Some(_) => c.model = Some(index),
                    }
                }
            }
        });
    // Permission mode.
    let mode = ui
        .add(Button::ghost(c.mode.label()).small(true).min_width(0.0))
        .on_hover_text("Permission mode");
    egui::Popup::menu(&mode)
        .frame(Surface::Popup.frame(ui.ctx()))
        .show(|ui| {
            ui.set_width(140.0);
            for option in PermMode::ALL {
                if ui
                    .add(MenuItem::new(option.label()).checked(c.mode == option))
                    .clicked()
                {
                    c.mode = option;
                }
            }
        });
    // Attach a file.
    let attach = ui.add(IconButton::new(glyph(Concept::Attach), "Attach file"));
    egui::Popup::menu(&attach)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .frame(Surface::Popup.frame(ui.ctx()))
        .show(|ui| {
            ui.set_width(300.0);
            ui.add(
                TextField::search(&mut c.attach_filter)
                    .placeholder("Find a file")
                    .width(280.0),
            );
            for path in file_matches(files, &c.attach_filter, 8) {
                let name = path.display().to_string();
                if ui
                    .add_sized(
                        [ui.available_width(), 24.0],
                        egui::Button::new(&name)
                            .frame(false)
                            .wrap_mode(egui::TextWrapMode::Truncate),
                    )
                    .clicked()
                {
                    let chip = format!("@{name}");
                    if !c.chips.contains(&chip) {
                        c.chips.push(chip);
                    }
                    ui.close();
                }
            }
        });
    // Attach an issue.
    let issue = ui
        .add(Button::ghost("#").small(true).min_width(0.0))
        .on_hover_text("Attach an issue");
    egui::Popup::menu(&issue)
        .frame(Surface::Popup.frame(ui.ctx()))
        .show(|ui| {
            ui.set_width(320.0);
            for (number, title) in ISSUES {
                let row = ListRow::new(number)
                    .flat(true)
                    .secondary(title)
                    .show(ui)
                    .response;
                if row.clicked() {
                    let chip = number.to_owned();
                    if !c.chips.contains(&chip) {
                        c.chips.push(chip);
                    }
                    ui.close();
                }
            }
        });
    let _ = app;
    // Send, or stop while the agent works.
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let ready = !c.prompt().is_empty();
        if working && !ready {
            let stop = round_button(
                ui,
                Glyph::Stop,
                "Stop agent",
                colors.border,
                colors.text,
                true,
            );
            out.stop = stop.clicked();
        } else {
            let send = round_button(
                ui,
                glyph(Concept::Send),
                "Send message",
                if ready { colors.accent } else { colors.border },
                if ready {
                    colors.text_on_accent
                } else {
                    colors.text_disabled
                },
                ready,
            );
            out.send = send.clicked();
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn tokens_trigger_completion_only_where_they_should() {
        assert_eq!(
            token_at("look at @App", 12),
            Some((CompletionKind::File, 8, "App".into()))
        );
        assert_eq!(
            token_at("/re", 3),
            Some((CompletionKind::Slash, 0, "re".into()))
        );
        // A slash in the middle of a message is not a command.
        assert_eq!(token_at("run /re", 7), None);
        assert_eq!(token_at("hello", 5), None);
        assert_eq!(
            token_at("@", 1),
            Some((CompletionKind::File, 0, String::new()))
        );
        // The caret after a space leaves the token.
        assert_eq!(token_at("@App ", 5), None);
    }

    #[test]
    fn completion_candidates_are_ranked() {
        let files: Vec<PathBuf> = [
            "src/main/App.java",
            "src/test/AppTest.java",
            "README.md",
            "docs/Apple.md",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect();
        let hits = file_matches(&files, "app", 3);
        assert_eq!(hits[0], Path::new("docs/Apple.md"));
        assert!(
            hits.iter()
                .all(|p| p.to_string_lossy().to_lowercase().contains("app"))
        );
        assert_eq!(file_matches(&files, "", 2).len(), 2);
        assert_eq!(slash_matches("pl")[0].0, "/plan");
        assert_eq!(slash_matches("").len(), SLASH.len());
        assert!(slash_matches("zz").is_empty());
    }

    #[test]
    fn splice_replaces_by_character() {
        let mut text = "fix \u{e4} @Ap now".to_owned();
        let caret = splice(&mut text, 6, 9, "");
        assert_eq!(text, "fix \u{e4}  now");
        assert_eq!(caret, 6);
    }

    #[test]
    fn new_thread_profiles_follow_the_selected_entries() {
        let mut composer = Composer::default();
        assert_eq!(composer.profiles(), [AgentProfile::Claude]);
        composer.selected = [true, true, true, false, true];
        assert_eq!(
            composer.profiles(),
            [
                AgentProfile::Claude,
                AgentProfile::Codex,
                AgentProfile::Opencode
            ]
        );
        composer.text = "  fix it ".into();
        composer.chips = vec!["@App.java".into(), "#128".into()];
        assert_eq!(composer.prompt(), "fix it @App.java #128");
    }
}

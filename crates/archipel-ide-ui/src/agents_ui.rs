//! Agents mode: the second arrangement of the workbench.
//!
//! Left: tasks and agents. Center: a terminal grid (the editor-split model driving one
//! pane per agent) or the review of one agent's diff. Right: the Changes of the selected
//! agent. Bottom: the prompt bar. In Code mode the same agents show up as a compact tool
//! window, a status bar widget with a popup list and balloon notifications.
//!
//! Everything here is built from kit components and the IDE's existing renderers
//! (`terminal_ui` for the cells, `diff_ui` for the diff). All state lives in
//! `IdeState::agents` behind the `AgentsBackend` trait; every action is a typed command.

use crate::icons::{self, Icon};
use crate::widgets::{self, island_frame, panel_header};
use crate::{IdeApp, theme};
use archipel_ide_core::DiffTab;
use archipel_ide_core::agent_events::permission_input;

mod thread;
use archipel_ide_core::DiffViewer;
use archipel_ide_core::agents::{
    AgentNotice, AgentProfile, AgentState, AgentsScreen, ChangeKind, ChangedFile, MergeStrategy,
    NoticeKind, SessionId, SessionInfo, TaskId, TaskInfo, WorkbenchMode,
};
use archipel_kit::behaviour::ContextMenu;
use archipel_kit::components::{
    Banner, Button, Checkbox, Counter, Dialog, DialogAction, EmptyState, IconButton, ListRow,
    MenuItem, MenuSeparator, Radio, SegmentedControl, Severity, Tag, TextField, Toast, Tooltip,
    VirtualList,
};
use archipel_kit::parts::Slot;
use archipel_kit::{Glyph, Theme, Tone};
use eframe::egui::{self, Rect, Sense, Stroke, vec2};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;

const SIDEBAR_WIDTH: f32 = 290.0;
const INSPECTOR_WIDTH: f32 = 300.0;
/// Mock issues the `# Issue` chip cycles through.
const ISSUES: [&str; 3] = ["#128", "#131", "#142"];

/// Rows of the agents tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Task(TaskId),
    Session(SessionId),
}

struct Entry {
    key: Key,
    depth: usize,
    parent: bool,
}

#[derive(Default)]
struct RowText {
    label: String,
    secondary: Option<String>,
    /// Unread items (a counter is shown when above zero).
    trailing: usize,
    show_stats: bool,
}

/// What a pane or row asked for; applied after drawing (commands need `&mut IdeApp`).
enum Action {
    Select(SessionId),
    Show(SessionId),
    CloseThread(SessionId),
    /// Answers a permission card with the option at the index.
    Permission(SessionId, usize),
    /// A follow-up message to an agent.
    Reply(SessionId, String),
    ToggleRow(SessionId, usize),
    Stop(SessionId),
    Review(SessionId, Option<PathBuf>),
    OpenInCode(SessionId, Option<PathBuf>),
    Toggle(Key),
    Merge(SessionId, MergeStrategy),
}

/// The merge / discard dialog (mock).
struct MergeDialog {
    session: SessionId,
    strategy: MergeStrategy,
}

/// The diff of one file, rebuilt when the agent's work changes.
struct ReviewCache {
    key: (SessionId, PathBuf, usize, usize, usize),
    tab: DiffTab,
}

pub struct State {
    prompt: String,
    chips: Vec<String>,
    profiles: [bool; 4],
    collapsed: HashSet<Key>,
    /// The file picked in the Changes panel.
    changes_selected: Option<usize>,
    merge: Option<MergeDialog>,
    review: Option<ReviewCache>,
    viewer: DiffViewer,
    focus_prompt: bool,
    /// Reply field that should take keyboard focus (the "Reply" balloon action).
    focus_reply: Option<SessionId>,
    /// Expanded transcript rows (thinking, tool output): (thread, event index).
    expanded: HashSet<(SessionId, usize)>,
    /// Unsent follow-up text per thread.
    replies: std::collections::HashMap<SessionId, String>,
    issue: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            chips: Vec::new(),
            profiles: [true, false, false, false],
            collapsed: HashSet::new(),
            changes_selected: None,
            merge: None,
            review: None,
            viewer: DiffViewer::SideBySide,
            focus_prompt: false,
            focus_reply: None,
            expanded: HashSet::new(),
            replies: Default::default(),
            issue: 0,
        }
    }
}

impl State {
    fn chosen_profiles(&self) -> Vec<&'static str> {
        AgentProfile::ALL
            .iter()
            .zip(self.profiles)
            .filter(|(_, on)| *on)
            .map(|(profile, _)| profile.id())
            .collect()
    }
}

/// Every concept the Agents UI draws an icon for. Each one maps to a glyph in [`glyph`], the
/// only place that chooses icons: when the kit gets dedicated glyphs, change one line each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Concept {
    /// An agent / the Agents tool window (stand-in: Lightning).
    Agent,
    Task,
    Working,
    NeedsInput,
    Done,
    Failed,
    Stopped,
    Thinking,
    ToolRead,
    ToolEdit,
    ToolWrite,
    ToolBash,
    ToolSearch,
    ToolSpawn,
    ToolOther,
    Permission,
    Subagent,
    Result,
    Send,
    Review,
    Merge,
}

pub(crate) fn glyph(concept: Concept) -> Glyph {
    match concept {
        Concept::Agent => Glyph::Lightning,
        Concept::Task => Glyph::Folder,
        Concept::Working => Glyph::Run,
        Concept::NeedsInput => Glyph::Help,
        Concept::Done => Glyph::Success,
        Concept::Failed => Glyph::Error,
        Concept::Stopped => Glyph::Pause,
        Concept::Thinking => Glyph::Sparkle,
        Concept::ToolRead => Glyph::FileText,
        Concept::ToolEdit => Glyph::Edit,
        Concept::ToolWrite => Glyph::NewFile,
        Concept::ToolBash => Glyph::Terminal,
        Concept::ToolSearch => Glyph::Search,
        Concept::ToolSpawn => Glyph::Branch,
        Concept::ToolOther => Glyph::Plugin,
        Concept::Permission => Glyph::Lock,
        Concept::Subagent => Glyph::Branch,
        Concept::Result => Glyph::Check,
        Concept::Send => Glyph::Send,
        Concept::Review => Glyph::Diff,
        Concept::Merge => Glyph::Merge,
    }
}

/// Icon and meaning of an agent state.
fn look(state: AgentState) -> (Glyph, Tone) {
    match state {
        AgentState::Working => (glyph(Concept::Working), Tone::Run),
        AgentState::NeedsInput => (glyph(Concept::NeedsInput), Tone::Warning),
        AgentState::Done => (glyph(Concept::Done), Tone::Success),
        AgentState::Error => (glyph(Concept::Failed), Tone::Error),
        AgentState::Stopped => (glyph(Concept::Stopped), Tone::Muted),
    }
}

fn state_tag(info: &SessionInfo) -> Tag {
    let label = match info.outcome {
        Some(MergeStrategy::Discard) => "discarded",
        Some(_) => "merged",
        None => info.state.label(),
    };
    let tone = match (info.outcome, info.state) {
        (Some(_), _) => Tone::Muted,
        (None, AgentState::Working) => Tone::Accent,
        (None, state) => look(state).1,
    };
    Tag::new(label).tone(tone)
}

fn elapsed(info: &SessionInfo) -> String {
    let seconds = info.elapsed.as_secs();
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

/// `text` shortened with an ellipsis to fit `max` points (rows do not clip their label).
fn elide(ui: &egui::Ui, text: &str, max: f32) -> String {
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    let color = ui.visuals().text_color();
    let width = |s: &str| {
        ui.painter()
            .layout_no_wrap(s.to_owned(), font.clone(), color)
            .size()
            .x
    };
    if width(text) <= max {
        return text.to_owned();
    }
    let mut cut = text.to_owned();
    while !cut.is_empty() && width(&format!("{cut}\u{2026}")) > max {
        cut.pop();
    }
    format!("{}\u{2026}", cut.trim_end())
}

fn text_width(ui: &egui::Ui, text: &str) -> f32 {
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    ui.painter()
        .layout_no_wrap(text.to_owned(), font, ui.visuals().text_color())
        .size()
        .x
}

/// Room a row's chevron, icon and insets take at depth 0, plus what each depth adds.
const ROW_LEFT: f32 = 66.0;
const ROW_DEPTH: f32 = 18.0;
const ROW_RIGHT: f32 = 14.0;
const COUNTER_SLOT: f32 = 28.0;

fn stats_text(added: usize, removed: usize) -> String {
    format!("+{added} \u{2212}{removed}")
}

// ---------------------------------------------------------------------------
// Mode switch, status widget, balloons, banner
// ---------------------------------------------------------------------------

/// The `Code | Agents` switch of the main toolbar.
pub(crate) fn mode_switch(app: &mut IdeApp, ui: &mut egui::Ui) {
    let current = app.state.workbench_mode;
    let mut mode = current;
    ui.add(SegmentedControl::new(
        &mut mode,
        [
            (WorkbenchMode::Code, WorkbenchMode::Code.label()),
            (WorkbenchMode::Agents, WorkbenchMode::Agents.label()),
        ],
    ));
    if mode != current {
        app.dispatch("workbench.setMode", json!({"mode": mode.id()}));
    }
    let attention = app.state.agents.counts().attention();
    if attention > 0 && current == WorkbenchMode::Code {
        ui.add(Counter::new(attention).accent(true));
    }
}

/// Status bar text of the agents widget (`2 running · ? 1 input`), empty without agents.
pub(crate) fn status_text(app: &IdeApp) -> String {
    let counts = app.state.agents.counts();
    counts.status_text()
}

/// `◐ 2 running · ? 1 input`: opens a popup list of the agents.
pub(crate) fn status_widget(app: &mut IdeApp, ui: &mut egui::Ui, font: egui::FontId) {
    if app.state.agents.sessions().is_empty() {
        return;
    }
    let counts = app.state.agents.counts();
    let text = match counts.status_text() {
        text if text.is_empty() => "Agents".to_owned(),
        text => text,
    };
    let tone = if counts.needs_input > 0 {
        Tone::Warning
    } else if counts.failed > 0 {
        Tone::Error
    } else if counts.working > 0 {
        Tone::Run
    } else {
        Tone::Secondary
    };
    let color = tone.resolve(ui.ctx());
    let response = widgets::flat_widget(
        ui,
        Some((Icon::Kit(glyph(Concept::Agent).into()), color)),
        &text,
        font,
        color,
        false,
        22.0,
    )
    .on_hover_text("Agents");
    egui::Popup::menu(&response)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .frame(archipel_kit::parts::Surface::Popup.frame(ui.ctx()))
        .show(|ui| popup_list(app, ui));
}

fn popup_list(app: &mut IdeApp, ui: &mut egui::Ui) {
    ui.set_min_width(320.0);
    let sessions = app.state.agents.sessions();
    let mut shown = None;
    for info in &sessions {
        let (glyph, tone) = look(info.state);
        let row = ListRow::new(info.name.as_str())
            .flat(true)
            .icon(glyph)
            .icon_tone(tone)
            .secondary(format!(
                "{} \u{b7} {}",
                info.profile.id(),
                info.state.label()
            ))
            .label_tone(if info.unread {
                Tone::Strong
            } else {
                Tone::Default
            })
            .show(ui);
        if row.response.clicked() {
            shown = Some(info.id);
        }
    }
    ui.add(MenuSeparator);
    if ui
        .add(MenuItem::new("Open Agents Mode").shortcut(shortcut_label(app)))
        .clicked()
    {
        app.dispatch("workbench.setMode", json!({"mode": "agents"}));
        ui.close();
    }
    if let Some(id) = shown {
        app.dispatch("agents.show", json!({"session": id}));
        ui.close();
    }
}

fn shortcut_label(app: &IdeApp) -> String {
    app.action_shortcut(archipel_ide_core::agents::TOGGLE_MODE_ACTION)
        .unwrap_or_else(|| crate::keys::display("\u{2325}\u{21e7}\u{2318}X"))
}

/// Balloons about agents (Code mode only: in Agents mode the agents are in view).
pub(crate) fn notices(app: &mut IdeApp, ctx: &egui::Context) {
    if app.state.workbench_mode != WorkbenchMode::Code || app.state.agents.notices.is_empty() {
        return;
    }
    let notices: Vec<AgentNotice> = app
        .state
        .agents
        .notices
        .iter()
        .rev()
        .take(3)
        .rev()
        .cloned()
        .collect();
    let mut dispatches: Vec<(&'static str, Value)> = Vec::new();
    egui::Area::new(egui::Id::new("agents_notices"))
        .order(crate::notifications::BALLOON_ORDER)
        .anchor(
            egui::Align2::RIGHT_BOTTOM,
            vec2(-16.0, -(crate::chrome::STATUS_BAR_HEIGHT + 14.0)),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            for notice in &notices {
                let (severity, actions): (Severity, &[&str]) = match notice.kind {
                    NoticeKind::NeedsInput => (Severity::Warning, &["Reply", "Review"]),
                    NoticeKind::Done => (Severity::Success, &["Review"]),
                    NoticeKind::Failed => (Severity::Error, &["Show"]),
                    NoticeKind::Spawned => (Severity::Info, &["Show"]),
                };
                let mut toast = Toast::new(severity, notice.title.as_str());
                if !notice.body.is_empty() {
                    toast = toast.body(notice.body.as_str());
                }
                for label in actions {
                    toast = toast.action(*label);
                }
                let out = toast.show(ui);
                if let Some(index) = out.action {
                    let session = json!({"session": notice.session});
                    dispatches.push(match actions[index] {
                        "Review" => ("agents.review", session),
                        _ => ("agents.show", session),
                    });
                    if actions[index] == "Reply" {
                        app.agents_ui.focus_reply = Some(notice.session);
                    }
                }
                if out.dismissed {
                    dispatches.push(("agents.dismissNotice", json!({"id": notice.id})));
                }
            }
        });
    for (command, params) in dispatches {
        app.dispatch(command, params);
    }
}

/// "Back to agent fix-tab": shown above the editor after "Open in Code mode".
pub(crate) fn back_banner(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(session) = app.state.agents.return_to else {
        return;
    };
    let Some(info) = app.state.agents.session(session) else {
        return;
    };
    let out = egui::Frame::new()
        .outer_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            Banner::new(
                Severity::Info,
                format!(
                    "Viewing {}'s change in the editor. The agent is {}.",
                    info.name,
                    info.state.label()
                ),
            )
            .action(format!("Back to agent {}", info.name))
            .show(ui)
        })
        .inner;
    if out.action == Some(0) {
        app.dispatch("agents.backToAgent", Value::Null);
    }
}

// ---------------------------------------------------------------------------
// Tree of tasks and agents
// ---------------------------------------------------------------------------

fn flatten(tasks: &[TaskInfo], sessions: &[SessionInfo], collapsed: &HashSet<Key>) -> Vec<Entry> {
    fn children(
        out: &mut Vec<Entry>,
        mine: &[&SessionInfo],
        parent: Option<SessionId>,
        depth: usize,
        collapsed: &HashSet<Key>,
    ) {
        for session in mine.iter().filter(|s| s.parent == parent) {
            let has_children = mine.iter().any(|s| s.parent == Some(session.id));
            out.push(Entry {
                key: Key::Session(session.id),
                depth,
                parent: has_children,
            });
            if has_children && !collapsed.contains(&Key::Session(session.id)) {
                children(out, mine, Some(session.id), depth + 1, collapsed);
            }
        }
    }
    let mut out = Vec::new();
    for task in tasks {
        out.push(Entry {
            key: Key::Task(task.id),
            depth: 0,
            parent: true,
        });
        if collapsed.contains(&Key::Task(task.id)) {
            continue;
        }
        let mine: Vec<&SessionInfo> = sessions.iter().filter(|s| s.task == task.id).collect();
        children(&mut out, &mine, None, 1, collapsed);
    }
    out
}

/// The tree of tasks and agents: the Agents mode sidebar, or (`compact`) the Code mode tool
/// window.
pub(crate) fn tree(app: &mut IdeApp, ui: &mut egui::Ui, compact: bool) {
    let tasks = app.state.agents.backend.tasks();
    let sessions = app.state.agents.sessions();
    if sessions.is_empty() {
        EmptyState::new("No agents yet")
            .hint("Describe a task in Agents mode and press Enter.")
            .icon(glyph(Concept::Agent))
            .show(ui);
        return;
    }
    let entries = flatten(&tasks, &sessions, &app.agents_ui.collapsed);
    let selected_session = app.state.agents.selected;
    let mut selected =
        selected_session.and_then(|id| entries.iter().position(|e| e.key == Key::Session(id)));
    // Rows do not clip their text: fit every label to its row first.
    let width = ui.available_width();
    let texts: Vec<RowText> = entries
        .iter()
        .map(|entry| match entry.key {
            Key::Task(id) => {
                let title = tasks
                    .iter()
                    .find(|t| t.id == id)
                    .map_or("Task".to_owned(), |t| t.title.clone());
                let unread = sessions.iter().filter(|s| s.task == id && s.unread).count();
                let trailing = if unread > 0 { COUNTER_SLOT } else { 0.0 };
                RowText {
                    label: elide(ui, &title, width - ROW_LEFT - trailing - ROW_RIGHT),
                    secondary: None,
                    trailing: unread,
                    show_stats: false,
                }
            }
            Key::Session(id) => {
                let Some(info) = sessions.iter().find(|s| s.id == id) else {
                    return RowText::default();
                };
                let secondary = info.profile.id().to_owned();
                let stats = (!info.unread && info.files > 0 && !compact)
                    .then(|| stats_text(info.added, info.removed));
                let base = width
                    - ROW_LEFT
                    - ROW_DEPTH * entry.depth as f32
                    - text_width(ui, &secondary)
                    - 8.0
                    - ROW_RIGHT;
                // The name has priority: the diff stats only stay when both fit.
                let stats_width = stats.as_ref().map_or(0.0, |t| text_width(ui, t) + 8.0);
                let show_stats =
                    stats.is_some() && text_width(ui, &info.name) <= base - stats_width;
                let trailing = if info.unread {
                    COUNTER_SLOT
                } else if show_stats {
                    stats_width
                } else {
                    0.0
                };
                RowText {
                    label: elide(ui, &info.name, (base - trailing).max(48.0)),
                    secondary: Some(secondary),
                    trailing: usize::from(info.unread),
                    show_stats,
                }
            }
        })
        .collect();
    let actions: RefCell<Vec<Action>> = RefCell::new(Vec::new());
    let collapsed = app.agents_ui.collapsed.clone();
    let height = ui.available_height();
    let out = VirtualList::new(("agents_tree", compact), entries.len(), &mut selected)
        .name("Agents")
        .height(height)
        .show(ui, |index| {
            let entry = &entries[index];
            let text = &texts[index];
            let expanded = entry.parent.then(|| !collapsed.contains(&entry.key));
            match entry.key {
                Key::Task(_) => {
                    let mut row = ListRow::new(text.label.clone())
                        .icon(glyph(Concept::Task))
                        .depth(0)
                        .expanded(expanded)
                        .label_tone(if text.trailing > 0 {
                            Tone::Strong
                        } else {
                            Tone::Default
                        });
                    if text.trailing > 0 {
                        let unread = text.trailing;
                        row = row.trailing(Slot::ui_width(COUNTER_SLOT, move |ui| {
                            ui.add(Counter::new(unread).accent(true));
                        }));
                    }
                    row
                }
                Key::Session(id) => {
                    let Some(info) = sessions.iter().find(|s| s.id == id) else {
                        return ListRow::new("");
                    };
                    session_row(info, entry, expanded, text, &actions)
                }
            }
        });
    let mut queued = actions.into_inner();
    if let Some(index) = out.toggled {
        queued.push(Action::Toggle(entries[index].key));
    }
    if out.selection_changed
        && let Some(index) = selected
    {
        match entries[index].key {
            Key::Session(id) => queued.push(Action::Select(id)),
            Key::Task(_) => queued.push(Action::Toggle(entries[index].key)),
        }
    }
    if let Some(index) = out.activated
        && let Key::Session(id) = entries[index].key
    {
        queued.push(Action::Show(id));
    }
    apply(app, queued);
}

fn session_row<'a>(
    info: &SessionInfo,
    entry: &Entry,
    expanded: Option<bool>,
    text: &RowText,
    actions: &'a RefCell<Vec<Action>>,
) -> ListRow<'a> {
    let (glyph, tone) = look(info.state);
    let id = info.id;
    let running = info.state.is_active();
    let has_changes = info.files > 0;
    let mut row = ListRow::new(text.label.clone())
        .icon(glyph)
        .icon_tone(tone)
        .depth(entry.depth)
        .expanded(expanded)
        .secondary(text.secondary.clone().unwrap_or_default())
        .label_tone(if info.unread {
            Tone::Strong
        } else {
            Tone::Default
        });
    if info.unread {
        row = row.trailing(Slot::ui_width(COUNTER_SLOT, |ui| {
            ui.add(Counter::new(1).accent(true));
        }));
    } else if text.show_stats {
        row = row.trailing(Slot::text(stats_text(info.added, info.removed)));
    }
    row.with(Tooltip::new(info.name.clone()))
        .with(ContextMenu::new(move |ui| {
            let queue = |action| actions.borrow_mut().push(action);
            if ui.add(MenuItem::new("Stop").enabled(running)).clicked() {
                queue(Action::Stop(id));
            }
            if ui.add(MenuItem::new("Open Terminal")).clicked() {
                queue(Action::Show(id));
            }
            ui.add(MenuSeparator);
            if ui
                .add(MenuItem::new("Review Changes").enabled(has_changes))
                .clicked()
            {
                queue(Action::Review(id, None));
            }
            if ui
                .add(MenuItem::new("Open in Code Mode").enabled(has_changes))
                .clicked()
            {
                queue(Action::OpenInCode(id, None));
            }
        }))
}

// ---------------------------------------------------------------------------
// Agents mode screens
// ---------------------------------------------------------------------------

/// Draws the whole Agents mode body (everything between toolbar and status bar).
pub(crate) fn show(app: &mut IdeApp, ctx: &egui::Context) {
    egui::SidePanel::left("agents_sidebar")
        .default_width(SIDEBAR_WIDTH)
        .min_width(200.0)
        .resizable(true)
        .show_separator_line(false)
        .frame(island_frame(ctx))
        .show(ctx, |ui| sidebar(app, ui));
    match app.state.agents.screen {
        AgentsScreen::Main => {
            egui::SidePanel::right("agents_inspector")
                .default_width(INSPECTOR_WIDTH)
                .min_width(220.0)
                .resizable(true)
                .show_separator_line(false)
                .frame(island_frame(ctx))
                .show(ctx, |ui| thread::inspector(app, ui));
            egui::TopBottomPanel::bottom("agents_prompt")
                .show_separator_line(false)
                .frame(island_frame(ctx))
                .show(ctx, |ui| prompt_bar(app, ui));
            egui::CentralPanel::default()
                .frame(island_frame(ctx))
                .show(ctx, |ui| thread::threads(app, ui));
        }
        AgentsScreen::Review => {
            egui::CentralPanel::default()
                .frame(island_frame(ctx))
                .show(ctx, |ui| review(app, ui));
        }
    }
    merge_dialog(app, ctx);
}

fn sidebar(app: &mut IdeApp, ui: &mut egui::Ui) {
    let mut new_task = false;
    panel_header(ui, "Agents", |ui| {
        new_task = ui.add(IconButton::new(Glyph::Plus, "New task")).clicked();
    });
    if new_task {
        app.agents_ui.focus_prompt = true;
        if app.state.agents.screen == AgentsScreen::Review {
            app.dispatch("agents.closeReview", Value::Null);
        }
    }
    tree(app, ui, false);
}

// --- Prompt bar ---------------------------------------------------------------

fn prompt_bar(app: &mut IdeApp, ui: &mut egui::Ui) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| prompt_bar_rows(app, ui));
}

fn prompt_bar_rows(app: &mut IdeApp, ui: &mut egui::Ui) {
    ui.spacing_mut().item_spacing.y = 8.0;
    let mut launch = false;
    let mut add_file = false;
    let mut add_issue = false;
    let mut remove_chip = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let width = (ui.available_width() - 96.0).max(120.0);
        let response = ui.add(
            TextField::new(&mut app.agents_ui.prompt)
                .placeholder("Describe a task for the agents (@file, #issue), Enter to launch")
                .width(width),
        );
        if app.agents_ui.focus_prompt {
            app.agents_ui.focus_prompt = false;
            response.request_focus();
        }
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            launch = true;
        }
        if ui
            .add(
                // The kit's disabled primary button has no readable label: fall back to secondary.
                if app.agents_ui.prompt.trim().is_empty() {
                    Button::secondary("Launch").enabled(false)
                } else {
                    Button::primary("Launch")
                }
                .icon(glyph(Concept::Send)),
            )
            .clicked()
        {
            launch = true;
        }
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        add_file = ui
            .add(Button::ghost("@ File").small(true).min_width(0.0))
            .clicked();
        add_issue = ui
            .add(Button::ghost("# Issue").small(true).min_width(0.0))
            .clicked();
        for (index, chip) in app.agents_ui.chips.iter().enumerate() {
            ui.add(Tag::new(chip.as_str()).tone(Tone::Link));
            if ui
                .add(IconButton::new(Glyph::Close, format!("Remove {chip}")).small(true))
                .clicked()
            {
                remove_chip = Some(index);
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            for (index, profile) in AgentProfile::ALL.iter().enumerate().rev() {
                ui.add(Checkbox::new(
                    &mut app.agents_ui.profiles[index],
                    profile.id(),
                ))
                .on_hover_text(format!(
                    "{} ({})",
                    profile.id(),
                    profile.protocol()
                ));
            }
            ui.label(egui::RichText::new("Agents").color(Theme::colors(ui.ctx()).text_secondary));
        });
    });
    if add_file {
        let name = app
            .state
            .active_document
            .and_then(|index| app.state.documents.get(index))
            .map(|document| IdeApp::file_name(&document.path))
            .or_else(|| {
                app.state
                    .files
                    .iter()
                    .find(|entry| !entry.is_dir)
                    .map(|entry| IdeApp::file_name(&entry.relative_path))
            });
        if let Some(name) = name {
            let chip = format!("@{name}");
            if !app.agents_ui.chips.contains(&chip) {
                app.agents_ui.chips.push(chip);
            }
        }
    }
    if add_issue {
        let chip = ISSUES[app.agents_ui.issue % ISSUES.len()].to_owned();
        app.agents_ui.issue += 1;
        if !app.agents_ui.chips.contains(&chip) {
            app.agents_ui.chips.push(chip);
        }
    }
    if let Some(index) = remove_chip {
        app.agents_ui.chips.remove(index);
    }
    if launch {
        launch_prompt(app);
    }
}

/// Launches the typed task on the chosen agents (one agent per checked profile).
fn launch_prompt(app: &mut IdeApp) {
    let text = app.agents_ui.prompt.trim().to_owned();
    if text.is_empty() {
        app.state.status = "Describe the task for the agents first".into();
        return;
    }
    let mut prompt = text;
    for chip in &app.agents_ui.chips {
        prompt.push(' ');
        prompt.push_str(chip);
    }
    let profiles = app.agents_ui.chosen_profiles();
    if profiles.is_empty() {
        app.state.status = "Choose at least one agent to launch".into();
        return;
    }
    app.dispatch(
        "agents.launch",
        json!({"prompt": prompt, "profiles": profiles}),
    );
    app.agents_ui.prompt.clear();
    app.agents_ui.chips.clear();
}

// --- Changes panel --------------------------------------------------------------

/// File names fitted to a list `width` wide (rows do not clip their label).
fn change_labels(ui: &egui::Ui, changes: &[ChangedFile], width: f32) -> Vec<String> {
    changes
        .iter()
        .map(|change| {
            let stats = stats_text(change.added, change.removed);
            let room = width - 52.0 - text_width(ui, &stats) - ROW_RIGHT;
            elide(ui, &IdeApp::file_name(&change.path), room.max(48.0))
        })
        .collect()
}

fn change_rows<'a>(
    changes: &'a [ChangedFile],
    labels: &'a [String],
    actions: &'a RefCell<Vec<Action>>,
    session: SessionId,
) -> impl FnMut(usize) -> ListRow<'a> {
    move |index| {
        let change = &changes[index];
        let name = IdeApp::file_name(&change.path);
        let path = change.path.clone();
        let tone = match change.kind {
            ChangeKind::Modified => Tone::Modified,
            ChangeKind::Added => Tone::Added,
        };
        ListRow::new(labels[index].clone())
            .flat(true)
            .icon(Icon::for_file(&name))
            .label_tone(tone)
            .trailing(Slot::text(stats_text(change.added, change.removed)))
            .with(ContextMenu::new(move |ui| {
                if ui.add(MenuItem::new("Review Changes")).clicked() {
                    actions
                        .borrow_mut()
                        .push(Action::Review(session, Some(path.clone())));
                }
                if ui.add(MenuItem::new("Open in Code Mode")).clicked() {
                    actions
                        .borrow_mut()
                        .push(Action::OpenInCode(session, Some(path.clone())));
                }
            }))
    }
}

// --- Review screen ----------------------------------------------------------------

fn review(app: &mut IdeApp, ui: &mut egui::Ui) {
    let colors = Theme::colors(ui.ctx());
    let Some(info) = app
        .state
        .agents
        .selected
        .and_then(|id| app.state.agents.session(id))
    else {
        app.dispatch("agents.closeReview", Value::Null);
        return;
    };
    let changes = app.state.agents.backend.changes(info.id);
    let mut actions = Vec::new();
    let mut close = false;
    panel_header(ui, &format!("Review: {}", info.name), |ui| {
        close = ui
            .add(IconButton::new(Glyph::Close, "Back to agents"))
            .clicked();
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_space(theme::layout::CONTENT_INSET - 4.0);
        ui.add(state_tag(&info));
        ui.label(
            egui::RichText::new(format!(
                "base: main@{} \u{b7} {} \u{b7} {}",
                "a1b2c3d",
                widgets::pluralize(changes.len(), "file", "files"),
                stats_text(info.added, info.removed)
            ))
            .color(colors.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.add_space(theme::layout::CONTENT_INSET - 4.0);
            let handled = info.outcome.is_some();
            let can_merge = !info.state.is_active() && !handled && !changes.is_empty();
            if ui
                .add(Button::danger("Discard").enabled(!info.state.is_active() && !handled))
                .clicked()
            {
                app.agents_ui.merge = Some(MergeDialog {
                    session: info.id,
                    strategy: MergeStrategy::Discard,
                });
            }
            if ui
                .add(Button::primary("Merge\u{2026}").enabled(can_merge))
                .clicked()
            {
                app.agents_ui.merge = Some(MergeDialog {
                    session: info.id,
                    strategy: MergeStrategy::Squash,
                });
            }
            let selected_path = app.state.agents.review_file.clone();
            if ui
                .add(Button::secondary("Open in Code mode").enabled(!changes.is_empty()))
                .clicked()
            {
                actions.push(Action::OpenInCode(info.id, selected_path));
            }
            let mut viewer = app.agents_ui.viewer;
            ui.add(SegmentedControl::new(
                &mut viewer,
                [
                    (DiffViewer::SideBySide, "Side-by-side"),
                    (DiffViewer::Unified, "Unified"),
                ],
            ));
            app.agents_ui.viewer = viewer;
        });
    });
    ui.add_space(4.0);
    if changes.is_empty() {
        EmptyState::new("Nothing to review")
            .hint("This agent has not changed any file yet.")
            .show(ui);
        if close {
            app.dispatch("agents.closeReview", Value::Null);
        }
        apply(app, actions);
        return;
    }
    // The file shown: the one asked for, else the first.
    let current = app
        .state
        .agents
        .review_file
        .clone()
        .filter(|path| changes.iter().any(|c| &c.path == path))
        .unwrap_or_else(|| changes[0].path.clone());
    app.state.agents.review_file = Some(current.clone());
    let mut selected = changes.iter().position(|c| c.path == current);
    egui::SidePanel::left("agents_review_files")
        .default_width(240.0)
        .min_width(160.0)
        .resizable(true)
        .show_separator_line(true)
        .frame(egui::Frame::new())
        .show_inside(ui, |ui| {
            let queued: RefCell<Vec<Action>> = RefCell::new(Vec::new());
            let height = ui.available_height();
            let labels = change_labels(ui, &changes, ui.available_width());
            let out = VirtualList::new(
                ("agents_review_files", info.id),
                changes.len(),
                &mut selected,
            )
            .name("Changed files")
            .height(height)
            .show(ui, change_rows(&changes, &labels, &queued, info.id));
            actions.extend(queued.into_inner());
            if out.selection_changed
                && let Some(index) = selected
            {
                app.state.agents.review_file = Some(changes[index].path.clone());
            }
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::new())
        .show_inside(ui, |ui| {
            let change = changes.iter().find(|c| c.path == current);
            let key = (
                info.id,
                current.clone(),
                info.files,
                info.added,
                info.removed,
            );
            if app.agents_ui.review.as_ref().map(|r| &r.key) != Some(&key) {
                app.agents_ui.review =
                    app.state
                        .agents
                        .backend
                        .diff(info.id, &current)
                        .map(|diff| ReviewCache {
                            key,
                            tab: DiffTab::new(current.clone(), diff, app.agents_ui.viewer),
                        });
            }
            let viewer = app.agents_ui.viewer;
            match app.agents_ui.review.as_mut() {
                Some(cache) => {
                    cache.tab.viewer = viewer;
                    if change.is_some() {
                        crate::diff_ui::show_review(ui, &mut cache.tab);
                    }
                }
                None => {
                    EmptyState::new("No diff available").show(ui);
                }
            }
        });
    if close {
        app.dispatch("agents.closeReview", Value::Null);
    }
    apply(app, actions);
}

fn merge_dialog(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(dialog) = app.agents_ui.merge.as_mut() else {
        return;
    };
    let Some(info) = app.state.agents.session(dialog.session) else {
        app.agents_ui.merge = None;
        return;
    };
    let mut strategy = dialog.strategy;
    let discard = strategy == MergeStrategy::Discard;
    let summary = format!(
        "{}: {}, {}",
        info.name,
        widgets::pluralize(info.files, "changed file", "changed files"),
        stats_text(info.added, info.removed)
    );
    let primary = match strategy {
        MergeStrategy::Squash => "Squash and Merge",
        MergeStrategy::Merge => "Merge",
        MergeStrategy::Discard => "Discard Changes",
    };
    let action = Dialog::new(if discard {
        "Discard agent work"
    } else {
        "Merge agent work"
    })
    .message(summary)
    .icon(if discard {
        Glyph::Warning
    } else {
        glyph(Concept::Merge)
    })
    .primary(primary)
    .danger(discard)
    .secondary("Cancel")
    .content(|ui| {
        ui.add(Radio::new(
            &mut strategy,
            MergeStrategy::Squash,
            "Squash into main as one commit",
        ));
        ui.add(Radio::new(
            &mut strategy,
            MergeStrategy::Merge,
            "Merge with a merge commit",
        ));
        ui.add(Radio::new(
            &mut strategy,
            MergeStrategy::Discard,
            "Discard the changes and the worktree",
        ));
    })
    .show(ctx);
    if let Some(dialog) = app.agents_ui.merge.as_mut() {
        dialog.strategy = strategy;
    }
    match action {
        Some(DialogAction::Primary) => {
            let session = info.id;
            app.agents_ui.merge = None;
            app.dispatch(
                "agents.merge",
                json!({"session": session, "strategy": strategy.id()}),
            );
        }
        Some(_) => app.agents_ui.merge = None,
        None => {}
    }
}

// ---------------------------------------------------------------------------

fn apply(app: &mut IdeApp, actions: Vec<Action>) {
    for action in actions {
        match action {
            Action::Select(id) => app.dispatch("agents.select", json!({"session": id})),
            Action::Show(id) => app.dispatch("agents.show", json!({"session": id})),
            Action::CloseThread(id) => app.dispatch("agents.closeThread", json!({"session": id})),
            Action::Permission(id, option) => app.dispatch(
                "agents.send",
                json!({"session": id, "input": permission_input(option)}),
            ),
            Action::Reply(id, text) => app.dispatch(
                "agents.send",
                json!({"session": id, "input": format!("{text}\r")}),
            ),
            Action::ToggleRow(id, index) => {
                if !app.agents_ui.expanded.remove(&(id, index)) {
                    app.agents_ui.expanded.insert((id, index));
                }
            }
            Action::Stop(id) => app.dispatch("agents.stop", json!({"session": id})),
            Action::Review(id, path) => {
                let mut params = json!({"session": id});
                if let Some(path) = path {
                    params["path"] = json!(path);
                }
                app.dispatch("agents.review", params)
            }
            Action::OpenInCode(id, path) => {
                let mut params = json!({"session": id});
                if let Some(path) = path {
                    params["path"] = json!(path);
                }
                app.dispatch("agents.openInCode", params)
            }
            Action::Toggle(key) => {
                if !app.agents_ui.collapsed.remove(&key) {
                    app.agents_ui.collapsed.insert(key);
                }
            }
            Action::Merge(id, strategy) => {
                app.agents_ui.merge = Some(MergeDialog {
                    session: id,
                    strategy,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{frame_input, headless_app};
    use archipel_kit::testing::{self, Queryable};
    use std::time::Duration;

    fn chord() -> egui::Event {
        egui::Event::Key {
            key: egui::Key::X,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND | egui::Modifiers::ALT | egui::Modifiers::SHIFT,
        }
    }

    #[test]
    fn the_mode_switch_keeps_editor_state() {
        let (mut app, ctx) = headless_app("agents-mode-switch", "a.txt", "hello world\n");
        for _ in 0..2 {
            let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        }
        app.state.documents[0]
            .selections
            .set_single(archipel_ide_text::Selection::caret(6));
        let before = (
            app.state.documents[0].text.text(),
            app.state.documents[0].selections.primary(),
            app.state.active_document,
            app.state.documents.len(),
        );
        // Ctrl+Alt+Shift+X is Archipel.ToggleAgentsMode.
        let _ = ctx.run(frame_input(vec![chord()]), |ctx| app.frame(ctx));
        assert_eq!(app.state.workbench_mode, WorkbenchMode::Agents);
        for _ in 0..3 {
            let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        }
        let _ = ctx.run(frame_input(vec![chord()]), |ctx| app.frame(ctx));
        assert_eq!(app.state.workbench_mode, WorkbenchMode::Code);
        let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        let after = (
            app.state.documents[0].text.text(),
            app.state.documents[0].selections.primary(),
            app.state.active_document,
            app.state.documents.len(),
        );
        assert_eq!(before, after, "documents, tabs and the caret are shared");
    }

    #[test]
    fn the_status_widget_counts_agents_by_state() {
        let (mut app, ctx) = headless_app("agents-status", "a.txt", "x\n");
        assert_eq!(status_text(&app), "");
        app.dispatch("agents.demo", Value::Null);
        // Pre-rolled 4 s: claude asks; codex, its child, gemini and the spike still work.
        let counts = app.state.agents.counts();
        assert_eq!(counts.needs_input, 1);
        assert_eq!(counts.working, 4);
        assert_eq!(counts.failed, 0);
        assert_eq!(status_text(&app), "4 running · ? 1 input");
        for _ in 0..2 {
            let _ = ctx.run(frame_input(Vec::new()), |ctx| app.frame(ctx));
        }
        // Later everything but the question is done.
        for _ in 0..200 {
            app.state
                .agents
                .advance(Duration::from_millis(100), WorkbenchMode::Code);
        }
        assert_eq!(status_text(&app), "? 1 input · 3 done · 1 failed");
    }

    #[test]
    fn permission_card_buttons_answer_the_agent() {
        for (label, answer) in [
            ("Allow once", "Allow once"),
            ("Always allow", "Always allow"),
            ("Deny", "Deny"),
        ] {
            let (mut app, _ctx) = headless_app("agents-permission", "a.txt", "x\n");
            app.dispatch("agents.demo", Value::Null);
            app.dispatch("workbench.setMode", json!({"mode": "agents"}));
            app.dispatch("agents.select", json!({"session": 1}));
            let mut harness = testing::harness_state(Theme::dark(), app, |ui, app| {
                ui.allocate_ui(egui::vec2(900.0, 700.0), |ui| {
                    ui.set_min_size(egui::vec2(900.0, 700.0));
                    thread::threads(app, ui);
                });
            });
            harness.run_steps(4);
            harness.get_by_label("Permission needed");
            harness.get_all_by_label(label).next().unwrap().click();
            harness.run_steps(4);
            let app = harness.state();
            let events = app.state.agents.backend.events(1);
            assert!(
                events.iter().any(|e| matches!(e, archipel_ide_core::agent_events::AgentEvent::PermissionRequest { answer: Some(a), .. } if a == answer)),
                "{label}"
            );
            assert_ne!(
                app.state.agents.session(1).unwrap().state,
                AgentState::NeedsInput
            );
        }
    }

    #[test]
    fn thread_tabs_and_inspector_pass_the_audit_and_follow_the_active_tab() {
        for theme in [Theme::dark(), Theme::light()] {
            let (mut app, _ctx) = headless_app("agents-audit", "a.txt", "x\n");
            app.dispatch("agents.demo", Value::Null);
            app.dispatch("workbench.setMode", json!({"mode": "agents"}));
            app.dispatch("agents.select", json!({"session": 2}));
            let mut harness = testing::harness_state(theme, app, |ui, app| {
                ui.allocate_ui(egui::vec2(1100.0, 700.0), |ui| {
                    ui.set_min_size(egui::vec2(1100.0, 700.0));
                    ui.horizontal_top(|ui| {
                        ui.allocate_ui(egui::vec2(760.0, 700.0), |ui| thread::threads(app, ui));
                        ui.allocate_ui(egui::vec2(300.0, 700.0), |ui| thread::inspector(app, ui));
                    });
                });
            });
            harness.run_steps(4);
            // The inspector shows the active thread: codex, with its sub-thread.
            harness.get_by_label("Sub-threads (1)");
            harness.get_by_label("Started agent add-oauth-tests");
            harness.get_by_label("add-oauth");
            harness.get_by_label("fix-tab").click();
            harness.run_steps(4);
            assert_eq!(harness.state().state.agents.selected, Some(1));
            harness.get_by_label("Needs your answer");
            testing::audit(&mut harness).assert_clean();
        }
    }
}

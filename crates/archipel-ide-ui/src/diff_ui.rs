//! Diff viewer editor tab: side-by-side and unified presentations of a
//! [`DiffTab`], with syntax highlighting, word-level change highlights and
//! next/previous difference navigation.

use crate::icons::{self, Icon};
use crate::{IdeApp, editor, theme};
use archipel_ide_core::{DiffHighlight, DiffTab, DiffViewer};
use archipel_ide_git::{Hunk, HunkKind, LineRange, WhitespaceMode};
use archipel_ide_language::LanguageAnalysis;
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, pos2, vec2};
use serde_json::{Value, json};

const ROW_HEIGHT: f32 = 22.0;
const TOOLBAR_HEIGHT: f32 = 34.0;
const HEADER_HEIGHT: f32 = 22.0;
const NUMBER_WIDTH: f32 = 44.0;
const CONNECTOR_WIDTH: f32 = 28.0;
const TEXT_PADDING: f32 = 10.0;

/// Observed dark-theme diff colours (reference scene 13).
mod colors {
    use eframe::egui::Color32;
    pub const INSERTED: Color32 = Color32::from_rgb(0x29, 0x43, 0x35);
    pub const INSERTED_WORD: Color32 = Color32::from_rgb(0x36, 0x63, 0x48);
    pub const INSERTED_STRIPE: Color32 = Color32::from_rgb(0x48, 0x6C, 0x50);
    pub const MODIFIED: Color32 = Color32::from_rgb(0x2B, 0x3B, 0x57);
    pub const MODIFIED_WORD: Color32 = Color32::from_rgb(0x3A, 0x55, 0x82);
    pub const MODIFIED_STRIPE: Color32 = Color32::from_rgb(0x43, 0x69, 0xA6);
    pub const DELETED: Color32 = Color32::from_rgb(0x3E, 0x3F, 0x42);
    pub const DELETED_WORD: Color32 = Color32::from_rgb(0x58, 0x5A, 0x5E);
    pub const DELETED_STRIPE: Color32 = Color32::from_rgb(0x6F, 0x73, 0x7A);
    pub const DIVIDER: Color32 = Color32::from_rgb(0x2B, 0x2D, 0x30);
}

fn hunk_fill(kind: HunkKind) -> Color32 {
    match kind {
        HunkKind::Inserted => colors::INSERTED,
        HunkKind::Modified => colors::MODIFIED,
        HunkKind::Deleted => colors::DELETED,
    }
}

fn hunk_stripe(kind: HunkKind) -> Color32 {
    match kind {
        HunkKind::Inserted => colors::INSERTED_STRIPE,
        HunkKind::Modified => colors::MODIFIED_STRIPE,
        HunkKind::Deleted => colors::DELETED_STRIPE,
    }
}

fn word_fill(kind: HunkKind) -> Color32 {
    match kind {
        HunkKind::Inserted => colors::INSERTED_WORD,
        HunkKind::Modified => colors::MODIFIED_WORD,
        HunkKind::Deleted => colors::DELETED_WORD,
    }
}

pub fn tab_title(tab: &DiffTab) -> String {
    format!("{} (Diff)", IdeApp::file_name(&tab.path))
}

/// Draws the selected diff tab below the editor tab strip.
pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(tab) = app.state.diff.as_ref() else {
        return;
    };
    let mut actions: Vec<(&'static str, Value)> = Vec::new();
    toolbar(ui, tab, &mut actions);
    let reveal = tab.reveal;
    let scroll_id = egui::Id::new(("diff_scroll", &tab.path, tab.viewer.id()));
    match tab.viewer {
        DiffViewer::SideBySide => side_by_side(ui, tab, scroll_id, reveal, &mut actions),
        DiffViewer::Unified => unified(ui, tab, scroll_id, reveal),
    }
    if let Some(tab) = app.state.diff.as_mut() {
        tab.reveal = None;
    }
    for (id, params) in actions {
        app.dispatch(id, params);
    }
}

/// The diff body of a [`DiffTab`] without the editor tab toolbar (Agents mode review).
/// Chunk actions of the viewer are dropped: a review only reads.
pub(crate) fn show_review(ui: &mut egui::Ui, tab: &mut DiffTab) {
    let reveal = tab.reveal.take();
    let scroll_id = egui::Id::new(("review_scroll", &tab.path, tab.viewer.id()));
    match tab.viewer {
        DiffViewer::SideBySide => side_by_side(ui, tab, scroll_id, reveal, &mut Vec::new()),
        DiffViewer::Unified => unified(ui, tab, scroll_id, reveal),
    }
}

fn icon_button(ui: &mut egui::Ui, rect: Rect, icon: Icon, enabled: bool, tooltip: &str) -> bool {
    let id = ui.id().with(("diff_toolbar", tooltip));
    let response = ui.interact(rect, id, Sense::click());
    if enabled && response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme::HOVER);
    }
    let color = if enabled {
        *theme::TEXT_SECONDARY
    } else {
        *theme::TEXT_DISABLED
    };
    icons::paint(ui.painter(), rect.shrink(4.0), icon, color);
    enabled && response.on_hover_text(tooltip).clicked()
}

fn toolbar(ui: &mut egui::Ui, tab: &DiffTab, actions: &mut Vec<(&'static str, Value)>) {
    let (bar, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), TOOLBAR_HEIGHT), Sense::hover());
    let painter = ui.painter().clone();
    let mut x = bar.left() + 8.0;
    let button = |x: f32| Rect::from_min_size(pos2(x, bar.center().y - 12.0), vec2(24.0, 24.0));
    let diff = &tab.diff;
    let has_previous = diff.previous_hunk(tab.current).is_some();
    let has_next = diff.next_hunk(tab.current).is_some();
    if icon_button(
        ui,
        button(x),
        Icon::ChevronUp,
        has_previous,
        &format!("Previous Difference ({})", crate::keys::display("⇧F7")),
    ) {
        actions.push(("diff.previousChange", Value::Null));
    }
    x += 26.0;
    if icon_button(
        ui,
        button(x),
        Icon::ChevronDown,
        has_next,
        &format!("Next Difference ({})", crate::keys::display("F7")),
    ) {
        actions.push(("diff.nextChange", Value::Null));
    }
    x += 34.0;
    // Viewer mode dropdown.
    let combo_rect = Rect::from_min_size(pos2(x, bar.center().y - 12.0), vec2(150.0, 24.0));
    let mut combo_ui = ui.new_child(egui::UiBuilder::new().max_rect(combo_rect));
    let mut viewer = tab.viewer;
    egui::ComboBox::from_id_salt("diff_viewer_mode")
        .selected_text(viewer.label())
        .width(146.0)
        .show_ui(&mut combo_ui, |ui| {
            for option in [DiffViewer::SideBySide, DiffViewer::Unified] {
                ui.selectable_value(&mut viewer, option, option.label());
            }
        });
    if viewer != tab.viewer {
        actions.push(("diff.setViewer", json!({"viewer": viewer.id()})));
    }
    x += 154.0;
    let whitespace = combo(
        ui,
        bar,
        &mut x,
        150.0,
        "diff_whitespace",
        tab.whitespace.label(),
        WhitespaceMode::ALL.map(|mode| (mode.id(), mode.label())),
        tab.whitespace.id(),
    );
    if let Some(mode) = whitespace {
        actions.push(("diff.setWhitespace", json!({ "mode": mode })));
    }
    let highlight = combo(
        ui,
        bar,
        &mut x,
        130.0,
        "diff_highlight",
        tab.highlight.label(),
        DiffHighlight::ALL.map(|mode| (mode.id(), mode.label())),
        tab.highlight.id(),
    );
    if let Some(mode) = highlight {
        actions.push(("diff.setHighlight", json!({ "mode": mode })));
    }
    for (label, hint, on, id) in [
        (
            "Collapse unchanged",
            "Fold long unchanged stretches",
            tab.collapse_unchanged,
            "diff.toggleCollapseUnchanged",
        ),
        (
            "Sync scroll",
            "Scroll both sides together",
            tab.sync_scroll,
            "diff.toggleSyncScroll",
        ),
    ] {
        let width = 24.0 + label.len() as f32 * 6.5;
        let rect = Rect::from_min_size(pos2(x, bar.center().y - 11.0), vec2(width, 22.0));
        x += width + 6.0;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        let mut value = on;
        if child
            .checkbox(&mut value, label)
            .on_hover_text(hint)
            .clicked()
        {
            actions.push((id, Value::Null));
        }
    }
    if tab.new_revision.is_none() && !tab.selected.is_empty() {
        let rect = Rect::from_min_size(pos2(x, bar.center().y - 11.0), vec2(150.0, 22.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        let label = format!(
            "Stage {} chunk{}",
            tab.selected.len(),
            if tab.selected.len() == 1 { "" } else { "s" }
        );
        if child
            .button(label)
            .on_hover_text("Stage the ticked chunks only")
            .clicked()
        {
            let hunks: Vec<usize> = tab.selected.iter().copied().collect();
            actions.push(("git.stage", json!({ "path": tab.path, "hunks": hunks })));
        }
    }
    let count = diff.hunks.len();
    let summary = match count {
        0 => "No differences".to_owned(),
        1 => "1 difference".to_owned(),
        n => format!("{n} differences"),
    };
    painter.text(
        pos2(bar.right() - 12.0, bar.center().y),
        egui::Align2::RIGHT_CENTER,
        summary,
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        *theme::TEXT_SECONDARY,
    );
    painter.hline(
        bar.x_range(),
        bar.bottom() - 0.5,
        Stroke::new(1.0_f32, colors::DIVIDER),
    );
}

/// A toolbar dropdown at `*x`; returns the id of a newly chosen option.
#[allow(clippy::too_many_arguments)]
fn combo(
    ui: &mut egui::Ui,
    bar: Rect,
    x: &mut f32,
    width: f32,
    salt: &str,
    selected: &str,
    options: impl IntoIterator<Item = (&'static str, &'static str)>,
    current: &str,
) -> Option<String> {
    let rect = Rect::from_min_size(pos2(*x, bar.center().y - 12.0), vec2(width, 24.0));
    *x += width + 6.0;
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    let mut chosen = None;
    egui::ComboBox::from_id_salt(salt)
        .selected_text(selected)
        .width(width - 4.0)
        .show_ui(&mut child, |ui| {
            for (id, label) in options {
                if ui.selectable_label(id == current, label).clicked() {
                    chosen = Some(id.to_owned());
                }
            }
        });
    chosen
}

/// A small padlock: the read-only marker of the left header.
fn paint_lock(painter: &egui::Painter, center: Pos2, color: Color32) {
    let body = Rect::from_center_size(center + vec2(0.0, 2.0), vec2(9.0, 7.0));
    painter.rect_filled(body, 1.5, color);
    let shackle = [
        pos2(center.x - 3.0, body.top()),
        pos2(center.x - 3.0, body.top() - 3.0),
        pos2(center.x + 3.0, body.top() - 3.0),
        pos2(center.x + 3.0, body.top()),
    ];
    painter.add(egui::Shape::line(
        shackle.to_vec(),
        Stroke::new(1.3_f32, color),
    ));
}

/// Byte offset at which each line starts.
fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(text.match_indices('\n').map(|(index, _)| index + 1));
    starts
}

struct Side<'a> {
    text: &'a str,
    lines: Vec<&'a str>,
    starts: Vec<usize>,
    analysis: &'a LanguageAnalysis,
}

impl<'a> Side<'a> {
    fn new(text: &'a str, analysis: &'a LanguageAnalysis) -> Self {
        Self {
            text,
            lines: archipel_ide_git::split_lines(text),
            starts: line_starts(text),
            analysis,
        }
    }
}

/// Paints one line of code with syntax colours and changed-word backgrounds.
fn paint_line(
    painter: &egui::Painter,
    side: &Side,
    line: usize,
    origin: Pos2,
    words: &[LineRange],
    word_color: Color32,
) {
    let Some(text) = side.lines.get(line) else {
        return;
    };
    let start = side.starts.get(line).copied().unwrap_or(0);
    debug_assert!(start <= side.text.len());
    let job = editor::highlighted_line_job(
        text,
        start,
        side.analysis,
        &egui::FontId::monospace(theme::EDITOR_FONT_SIZE),
    );
    let galley = painter.layout_job(job);
    let y = origin.y + (ROW_HEIGHT - galley.size().y) / 2.0;
    for range in words.iter().filter(|range| range.line == line) {
        let (Some(from), Some(to)) = (
            text.get(..range.columns.start),
            text.get(..range.columns.end),
        ) else {
            continue;
        };
        let x1 = galley
            .pos_from_cursor(egui::text::CCursor::new(from.chars().count()))
            .left();
        let x2 = galley
            .pos_from_cursor(egui::text::CCursor::new(to.chars().count()))
            .left();
        painter.rect_filled(
            Rect::from_min_max(
                pos2(origin.x + x1, origin.y + 2.0),
                pos2(origin.x + x2, origin.y + ROW_HEIGHT - 2.0),
            ),
            2.0,
            word_color,
        );
    }
    painter.galley(pos2(origin.x, y), galley, *theme::editor::TEXT);
}

fn line_number(painter: &egui::Painter, right: f32, y: f32, number: usize, current: bool) {
    painter.text(
        pos2(right, y + ROW_HEIGHT / 2.0),
        egui::Align2::RIGHT_CENTER,
        number.to_string(),
        egui::FontId::monospace(theme::EDITOR_FONT_SIZE - 1.0),
        if current {
            *theme::editor::LINE_NUMBER_CARET
        } else {
            *theme::editor::LINE_NUMBER
        },
    );
}

/// Reads, scrolls and stores the first visible row; returns it.
fn scroll_position(
    ui: &egui::Ui,
    body: Rect,
    id: egui::Id,
    total_rows: usize,
    reveal_row: Option<usize>,
) -> f32 {
    let visible = body.height() / ROW_HEIGHT;
    let max = (total_rows as f32 - visible + 3.0).max(0.0);
    let mut top: f32 = ui.data(|data| data.get_temp(id)).unwrap_or(0.0);
    if let Some(row) = reveal_row {
        top = row as f32 - visible / 3.0;
    }
    let hovered = ui.rect_contains_pointer(body);
    if hovered {
        let delta = ui.input(|input| input.smooth_scroll_delta.y);
        top -= delta / ROW_HEIGHT;
    }
    if hovered && ui.input(|input| input.key_pressed(egui::Key::PageDown)) {
        top += visible;
    }
    top = top.clamp(0.0, max);
    ui.data_mut(|data| data.insert_temp(id, top));
    top
}

fn error_stripe(
    painter: &egui::Painter,
    rect: Rect,
    total_rows: usize,
    marks: &[(usize, usize, HunkKind)],
) {
    if total_rows == 0 {
        return;
    }
    // Short files map rows to their real position; long ones are squashed.
    let scale = (rect.height() / total_rows.max(1) as f32).min(ROW_HEIGHT);
    for &(start, len, kind) in marks {
        let y = rect.top() + start as f32 * scale;
        let height = (len as f32 * scale).max(3.0);
        painter.rect_filled(
            Rect::from_min_size(pos2(rect.left(), y), vec2(rect.width(), height)),
            1.0,
            hunk_stripe(kind),
        );
    }
}

fn header(painter: &egui::Painter, rect: Rect, text: &str, icon: Option<Icon>) {
    let mut x = rect.left() + 8.0;
    if let Some(icon) = icon {
        icons::paint(
            painter,
            Rect::from_min_size(pos2(x, rect.center().y - 7.0), vec2(14.0, 14.0)),
            icon,
            *theme::TEXT_SECONDARY,
        );
        x += 18.0;
    }
    painter.text(
        pos2(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        *theme::TEXT_SECONDARY,
    );
}

fn paint_checkbox(painter: &egui::Painter, rect: Rect, checked: bool) {
    painter.rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0_f32, *theme::TEXT_SECONDARY),
        egui::StrokeKind::Inside,
    );
    if checked {
        painter.rect_filled(rect.shrink(2.5), 1.0, *theme::TEXT_BRIGHT);
    }
}

fn base_label(tab: &DiffTab) -> String {
    let path = tab.diff.path.display();
    match &tab.diff.base_revision {
        Some(revision) => format!("{revision}  {path}"),
        None => format!("(new file)  {path}"),
    }
}

/// Label of the new side: a commit hash, or the working copy.
fn new_label(tab: &DiffTab) -> String {
    tab.new_revision
        .clone()
        .unwrap_or_else(|| "Current version".to_owned())
}

fn side_by_side(
    ui: &mut egui::Ui,
    tab: &DiffTab,
    scroll_id: egui::Id,
    reveal: Option<usize>,
    actions: &mut Vec<(&'static str, Value)>,
) {
    let diff = &tab.diff;
    let old = Side::new(&diff.old_text, &tab.old_analysis);
    let new = Side::new(&diff.new_text, &tab.new_analysis);
    let area = ui.available_rect_before_wrap();
    ui.allocate_rect(area, Sense::hover());
    let middle = NUMBER_WIDTH * 2.0 + CONNECTOR_WIDTH;
    let pane_width = ((area.width() - middle - 10.0) / 2.0).max(40.0);
    let left_pane = Rect::from_min_size(
        pos2(area.left(), area.top() + HEADER_HEIGHT),
        vec2(pane_width, area.height() - HEADER_HEIGHT),
    );
    let gutter_left = left_pane.right();
    let connector_left = gutter_left + NUMBER_WIDTH;
    let connector_right = connector_left + CONNECTOR_WIDTH;
    let right_pane = Rect::from_min_max(
        pos2(connector_right + NUMBER_WIDTH, left_pane.top()),
        pos2(area.right() - 10.0, left_pane.bottom()),
    );
    let stripe = Rect::from_min_max(
        pos2(area.right() - 8.0, left_pane.top()),
        pos2(area.right() - 2.0, left_pane.bottom()),
    );
    let painter = ui.painter_at(area);
    header(
        &painter,
        Rect::from_min_max(area.min, pos2(gutter_left, left_pane.top())),
        &base_label(tab),
        Some(Icon::Commit),
    );
    // The base side is read-only.
    let left_header = Rect::from_min_max(area.min, pos2(gutter_left, left_pane.top()));
    paint_lock(
        &painter,
        pos2(left_header.right() - 12.0, left_header.center().y),
        *theme::TEXT_DISABLED,
    );
    let right_header = Rect::from_min_max(
        pos2(right_pane.left(), area.top()),
        pos2(area.right(), left_pane.top()),
    );
    if tab.new_revision.is_none() && !diff.hunks.is_empty() {
        // "Current version" ticks every chunk for partial staging.
        let all = tab.selected.len() == diff.hunks.len();
        let box_rect = Rect::from_center_size(
            pos2(right_header.left() + 8.0, right_header.center().y),
            vec2(12.0, 12.0),
        );
        let response = ui
            .interact(
                Rect::from_min_max(
                    pos2(box_rect.left() - 2.0, right_header.top()),
                    pos2(box_rect.left() + 130.0, right_header.bottom()),
                ),
                ui.id().with("diff_select_all"),
                Sense::click(),
            )
            .on_hover_text("Tick every chunk for staging");
        paint_checkbox(&painter, box_rect, all);
        painter.text(
            pos2(box_rect.right() + 6.0, right_header.center().y),
            egui::Align2::LEFT_CENTER,
            new_label(tab),
            egui::FontId::proportional(theme::UI_FONT_SIZE),
            *theme::TEXT_SECONDARY,
        );
        if response.clicked() {
            for index in 0..diff.hunks.len() {
                if tab.selected.contains(&index) == all {
                    actions.push(("diff.toggleChunk", json!({ "index": index })));
                }
            }
        }
    } else {
        header(&painter, right_header, &new_label(tab), None);
    }
    painter.hline(
        area.x_range(),
        left_pane.top() - 0.5,
        Stroke::new(1.0_f32, colors::DIVIDER),
    );
    painter.vline(
        gutter_left,
        left_pane.y_range(),
        Stroke::new(1.0_f32, colors::DIVIDER),
    );
    painter.vline(
        connector_right + NUMBER_WIDTH,
        left_pane.y_range(),
        Stroke::new(1.0_f32, colors::DIVIDER),
    );

    let total = new.lines.len().max(old.lines.len());
    let fold_list = if tab.collapse_unchanged {
        folds(&diff.hunks, old.lines.len(), new.lines.len())
    } else {
        Vec::new()
    };
    let new_map = FoldMap {
        folds: &fold_list,
        is_new: true,
    };
    let old_map = FoldMap {
        folds: &fold_list,
        is_new: false,
    };
    let reveal_row = reveal
        .and_then(|index| diff.hunks.get(index))
        .map(|h| new_map.display(h.new.start as f32).max(0.0) as usize);
    let rows_total = (new_map.display(new.lines.len() as f32))
        .max(old_map.display(old.lines.len() as f32))
        .ceil() as usize;
    let visible = left_pane.height() / ROW_HEIGHT;
    let (top_new, top_old) = if tab.sync_scroll {
        let top_new = scroll_position(
            ui,
            left_pane.union(right_pane),
            scroll_id,
            rows_total,
            reveal_row,
        );
        // Both panes keep the same content at their vertical centre.
        let centre = diff.new_to_old(new_map.line(top_new + visible / 2.0));
        (top_new, (old_map.display(centre) - visible / 2.0).max(0.0))
    } else {
        let top_new = scroll_position(ui, right_pane, scroll_id, rows_total, reveal_row);
        let old_id = scroll_id.with("old");
        let reveal_old = reveal
            .and_then(|index| diff.hunks.get(index))
            .map(|h| old_map.display(h.old.start as f32).max(0.0) as usize);
        let top_old = scroll_position(ui, left_pane, old_id, rows_total, reveal_old);
        (top_new, top_old)
    };
    let y_new = |line: f32| left_pane.top() + (new_map.display(line) - top_new) * ROW_HEIGHT;
    let y_old = |line: f32| left_pane.top() + (old_map.display(line) - top_old) * ROW_HEIGHT;

    let body = Rect::from_min_max(pos2(area.left(), left_pane.top()), area.max);
    let body_painter = ui.painter_at(body);
    // Hunk backgrounds and connectors.
    for (index, hunk) in diff.hunks.iter().enumerate() {
        let kind = hunk.kind();
        let fill = if tab.highlight == DiffHighlight::None {
            Color32::TRANSPARENT
        } else {
            hunk_fill(kind)
        };
        let (o1, o2) = (y_old(hunk.old.start as f32), y_old(hunk.old.end as f32));
        let (n1, n2) = (y_new(hunk.new.start as f32), y_new(hunk.new.end as f32));
        if o2 < body.top() && n2 < body.top() || o1 > body.bottom() && n1 > body.bottom() {
            continue;
        }
        let current = tab.current == Some(index);
        paint_band(
            &body_painter,
            left_pane.left(),
            connector_left,
            o1,
            o2,
            fill,
            current,
        );
        paint_band(
            &body_painter,
            connector_right,
            right_pane.right(),
            n1,
            n2,
            fill,
            current,
        );
        let points = vec![
            pos2(connector_left, o1),
            pos2(connector_right, n1),
            pos2(connector_right, n2.max(n1 + 1.0)),
            pos2(connector_left, o2.max(o1 + 1.0)),
        ];
        body_painter.add(egui::Shape::convex_polygon(points, fill, Stroke::NONE));
    }
    // Per-chunk controls: `>>` copies the chunk into the working copy and the
    // checkbox ticks it for partial staging.
    if tab.new_revision.is_none() {
        for (index, hunk) in diff.hunks.iter().enumerate() {
            let y = y_new(hunk.new.start as f32);
            if y < body.top() - ROW_HEIGHT || y > body.bottom() {
                continue;
            }
            let apply = Rect::from_center_size(
                pos2(connector_right - 9.0, y + ROW_HEIGHT / 2.0),
                vec2(16.0, 16.0),
            );
            let response = ui
                .interact(apply, ui.id().with(("diff_apply", index)), Sense::click())
                .on_hover_text("Apply this chunk to the current version");
            if response.hovered() {
                body_painter.rect_filled(apply, 3.0, theme::HOVER);
            }
            body_painter.text(
                apply.center(),
                egui::Align2::CENTER_CENTER,
                "»",
                egui::FontId::proportional(theme::UI_FONT_SIZE + 2.0),
                *theme::TEXT_SECONDARY,
            );
            if response.clicked() {
                actions.push(("diff.applyChunk", json!({ "index": index })));
            }
            let check = Rect::from_center_size(
                pos2(connector_right + 10.0, y + ROW_HEIGHT / 2.0),
                vec2(12.0, 12.0),
            );
            let response = ui
                .interact(
                    check.expand(3.0),
                    ui.id().with(("diff_check", index)),
                    Sense::click(),
                )
                .on_hover_text("Include this chunk when staging");
            paint_checkbox(&body_painter, check, tab.selected.contains(&index));
            if response.clicked() {
                actions.push(("diff.toggleChunk", json!({ "index": index })));
            }
        }
    }
    // Text and line numbers.
    let current_hunk = tab.current.and_then(|index| diff.hunks.get(index));
    for (side, top, pane, numbers_right, words, is_new, map) in [
        (
            &old,
            top_old,
            left_pane,
            connector_left - 6.0,
            Side::old_words as fn(&Hunk) -> &[LineRange],
            false,
            &old_map,
        ),
        (
            &new,
            top_new,
            right_pane,
            right_pane.left() - 8.0,
            Side::new_words as fn(&Hunk) -> &[LineRange],
            true,
            &new_map,
        ),
    ] {
        let mut first = map.line(top).floor().max(0.0) as usize;
        if let Some(fold) = map.fold_at(first) {
            first = map.range(fold).start;
        }
        let last = (map.line(top + visible).ceil() as usize + 1).min(side.lines.len());
        let text_painter = ui.painter_at(pane);
        for line in first..last {
            let y = pane.top() + (map.display(line as f32) - top) * ROW_HEIGHT;
            if let Some(fold) = map.fold_at(line) {
                if line == map.range(fold).start {
                    fold_placeholder(
                        &text_painter,
                        Rect::from_min_size(pos2(pane.left(), y), vec2(pane.width(), ROW_HEIGHT)),
                        fold.new.len(),
                    );
                }
                continue;
            }
            let hunk = diff.hunks.iter().find(|hunk| {
                let range = if is_new { &hunk.new } else { &hunk.old };
                range.contains(&line)
            });
            let in_current = current_hunk.is_some_and(|h| {
                if is_new {
                    h.new.contains(&line)
                } else {
                    h.old.contains(&line)
                }
            });
            line_number(&body_painter, numbers_right, y, line + 1, in_current);
            let (ranges, color) = match hunk {
                Some(hunk) if tab.highlight == DiffHighlight::Words => {
                    (words(hunk), word_fill(hunk.kind()))
                }
                Some(hunk) => (&[][..], word_fill(hunk.kind())),
                None => (&[][..], Color32::TRANSPARENT),
            };
            paint_line(
                &text_painter,
                side,
                line,
                pos2(pane.left() + TEXT_PADDING, y),
                ranges,
                color,
            );
        }
    }
    let marks: Vec<_> = diff
        .hunks
        .iter()
        .map(|hunk| (hunk.new.start, hunk.new.len().max(1), hunk.kind()))
        .collect();
    error_stripe(&painter, stripe, total.max(1), &marks);
}

impl Side<'_> {
    fn old_words(hunk: &Hunk) -> &[LineRange] {
        &hunk.old_words
    }
    fn new_words(hunk: &Hunk) -> &[LineRange] {
        &hunk.new_words
    }
}

/// Fills `y1..y2` across `x1..x2`; an empty range draws a divider line where
/// the other side's lines belong.
fn paint_band(
    painter: &egui::Painter,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    fill: Color32,
    current: bool,
) {
    if y2 - y1 < 0.5 {
        painter.hline(x1..=x2, y1, Stroke::new(1.0_f32, fill.gamma_multiply(2.0)));
    } else {
        painter.rect_filled(Rect::from_min_max(pos2(x1, y1), pos2(x2, y2)), 0.0, fill);
    }
    if current {
        painter.vline(
            x1 + 1.0,
            y1..=y2.max(y1 + 1.0),
            Stroke::new(2.0_f32, fill.gamma_multiply(2.2)),
        );
    }
}

/// One row of the unified viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnifiedRow {
    pub old: Option<usize>,
    pub new: Option<usize>,
    pub hunk: Option<usize>,
    /// Number of unchanged lines this row stands for when collapsed (else 0).
    pub folded: usize,
}

/// Unchanged lines kept around each change when unchanged stretches collapse.
const CONTEXT: usize = 3;

/// A stretch of unchanged lines hidden behind one placeholder row; the same
/// length on both sides.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Fold {
    old: std::ops::Range<usize>,
    new: std::ops::Range<usize>,
}

/// The unchanged stretches longer than `2 * CONTEXT + 1` lines, minus the
/// context lines kept next to a change. Files without changes stay open.
fn folds(hunks: &[Hunk], old_len: usize, new_len: usize) -> Vec<Fold> {
    if hunks.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut gap =
        |old: std::ops::Range<usize>, new: std::ops::Range<usize>, top: bool, end: bool| {
            let lead = if top { 0 } else { CONTEXT };
            let trail = if end { 0 } else { CONTEXT };
            let len = new.len().min(old.len());
            if len > lead + trail + 1 {
                out.push(Fold {
                    old: old.start + lead..old.start + len - trail,
                    new: new.start + lead..new.start + len - trail,
                });
            }
        };
    let first = &hunks[0];
    gap(0..first.old.start, 0..first.new.start, true, false);
    for pair in hunks.windows(2) {
        gap(
            pair[0].old.end..pair[1].old.start,
            pair[0].new.end..pair[1].new.start,
            false,
            false,
        );
    }
    let last = &hunks[hunks.len() - 1];
    gap(last.old.end..old_len, last.new.end..new_len, false, true);
    out
}

/// Display-row mapping of one side with `folds` collapsed to one row each.
struct FoldMap<'a> {
    folds: &'a [Fold],
    is_new: bool,
}

impl FoldMap<'_> {
    fn range<'f>(&self, fold: &'f Fold) -> &'f std::ops::Range<usize> {
        if self.is_new { &fold.new } else { &fold.old }
    }

    /// Fold containing `line`.
    fn fold_at(&self, line: usize) -> Option<&Fold> {
        self.folds
            .iter()
            .find(|fold| self.range(fold).contains(&line))
    }

    /// Display row of a (fractional) line.
    fn display(&self, line: f32) -> f32 {
        let mut hidden = 0.0;
        for fold in self.folds {
            let range = self.range(fold);
            if line >= range.end as f32 {
                hidden += (range.len() - 1) as f32;
            } else {
                if line >= range.start as f32 {
                    return range.start as f32 - hidden;
                }
                break;
            }
        }
        line - hidden
    }

    /// Line shown at a display row (a fold's placeholder maps to its start).
    fn line(&self, display: f32) -> f32 {
        let mut hidden = 0.0;
        for fold in self.folds {
            let range = self.range(fold);
            let row = range.start as f32 - hidden;
            if display < row {
                break;
            }
            if display < row + 1.0 {
                return range.start as f32;
            }
            hidden += (range.len() - 1) as f32;
        }
        display + hidden
    }
}

/// Replaces unchanged stretches of the unified rows that `folds` hide with one
/// placeholder row each.
fn fold_rows(rows: Vec<UnifiedRow>, folds: &[Fold]) -> Vec<UnifiedRow> {
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        match row
            .new
            .and_then(|line| folds.iter().find(|f| f.new.contains(&line)))
        {
            Some(fold) if row.hunk.is_none() => {
                if row.new == Some(fold.new.start) {
                    out.push(UnifiedRow {
                        old: None,
                        new: None,
                        hunk: None,
                        folded: fold.new.len(),
                    });
                }
            }
            _ => out.push(row),
        }
    }
    out
}

/// Interleaves unchanged lines with each hunk's removed then added lines.
pub(crate) fn unified_rows(hunks: &[Hunk], old_len: usize, new_len: usize) -> Vec<UnifiedRow> {
    let mut rows = Vec::with_capacity(old_len.max(new_len) + 8);
    let (mut old, mut new) = (0, 0);
    for (index, hunk) in hunks.iter().enumerate() {
        while old < hunk.old.start && new < hunk.new.start {
            rows.push(UnifiedRow {
                old: Some(old),
                new: Some(new),
                hunk: None,
                folded: 0,
            });
            old += 1;
            new += 1;
        }
        for line in hunk.old.clone() {
            rows.push(UnifiedRow {
                old: Some(line),
                new: None,
                hunk: Some(index),
                folded: 0,
            });
        }
        for line in hunk.new.clone() {
            rows.push(UnifiedRow {
                old: None,
                new: Some(line),
                hunk: Some(index),
                folded: 0,
            });
        }
        old = hunk.old.end;
        new = hunk.new.end;
    }
    while old < old_len && new < new_len {
        rows.push(UnifiedRow {
            old: Some(old),
            new: Some(new),
            hunk: None,
            folded: 0,
        });
        old += 1;
        new += 1;
    }
    rows
}

/// The `N unchanged lines` row a collapsed stretch is drawn as.
fn fold_placeholder(painter: &egui::Painter, rect: Rect, lines: usize) {
    painter.rect_filled(rect, 0.0, theme::HOVER);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!(
            "⋯  {lines} unchanged line{}",
            if lines == 1 { "" } else { "s" }
        ),
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        *theme::TEXT_SECONDARY,
    );
}

fn unified(ui: &mut egui::Ui, tab: &DiffTab, scroll_id: egui::Id, reveal: Option<usize>) {
    let diff = &tab.diff;
    let old = Side::new(&diff.old_text, &tab.old_analysis);
    let new = Side::new(&diff.new_text, &tab.new_analysis);
    let rows = unified_rows(&diff.hunks, old.lines.len(), new.lines.len());
    let rows = if tab.collapse_unchanged {
        fold_rows(rows, &folds(&diff.hunks, old.lines.len(), new.lines.len()))
    } else {
        rows
    };
    let area = ui.available_rect_before_wrap();
    ui.allocate_rect(area, Sense::hover());
    let pane = Rect::from_min_max(
        pos2(
            area.left() + NUMBER_WIDTH * 2.0 + 6.0,
            area.top() + HEADER_HEIGHT,
        ),
        pos2(area.right() - 10.0, area.bottom()),
    );
    let painter = ui.painter_at(area);
    header(
        &painter,
        Rect::from_min_max(area.min, pos2(area.right(), pane.top())),
        &format!("{}  vs  {}", base_label(tab), new_label(tab)),
        Some(Icon::Commit),
    );
    painter.hline(
        area.x_range(),
        pane.top() - 0.5,
        Stroke::new(1.0_f32, colors::DIVIDER),
    );
    let reveal_row = reveal.and_then(|index| rows.iter().position(|row| row.hunk == Some(index)));
    let top = scroll_position(
        ui,
        pane.expand2(vec2(NUMBER_WIDTH * 2.0, 0.0)),
        scroll_id,
        rows.len(),
        reveal_row,
    );
    let visible = pane.height() / ROW_HEIGHT;
    let body = Rect::from_min_max(pos2(area.left(), pane.top()), area.max);
    let body_painter = ui.painter_at(body);
    let text_painter = ui.painter_at(pane);
    let first = top.floor().max(0.0) as usize;
    let last = ((top + visible).ceil() as usize + 1).min(rows.len());
    for (row_index, row) in rows.iter().enumerate().take(last).skip(first) {
        let y = pane.top() + (row_index as f32 - top) * ROW_HEIGHT;
        if row.folded > 0 {
            fold_placeholder(
                &body_painter,
                Rect::from_min_max(pos2(area.left(), y), pos2(pane.right(), y + ROW_HEIGHT)),
                row.folded,
            );
            continue;
        }
        let hunk = row.hunk.map(|index| &diff.hunks[index]);
        let current = row.hunk.is_some() && row.hunk == tab.current;
        if let Some(hunk) = hunk {
            let kind = if row.new.is_some() {
                HunkKind::Inserted
            } else {
                HunkKind::Deleted
            };
            let fill = if tab.highlight == DiffHighlight::None {
                Color32::TRANSPARENT
            } else {
                hunk_fill(kind)
            };
            body_painter.rect_filled(
                Rect::from_min_max(pos2(area.left(), y), pos2(pane.right(), y + ROW_HEIGHT)),
                0.0,
                fill,
            );
            if current {
                body_painter.vline(
                    area.left() + 1.0,
                    y..=y + ROW_HEIGHT,
                    Stroke::new(2.0_f32, hunk_stripe(hunk.kind())),
                );
            }
        }
        if let Some(line) = row.old {
            line_number(
                &body_painter,
                area.left() + NUMBER_WIDTH - 4.0,
                y,
                line + 1,
                current,
            );
        }
        if let Some(line) = row.new {
            line_number(
                &body_painter,
                area.left() + NUMBER_WIDTH * 2.0 - 4.0,
                y,
                line + 1,
                current,
            );
        }
        let origin = pos2(pane.left() + TEXT_PADDING, y);
        match (row.old, row.new, hunk) {
            (Some(line), None, Some(hunk)) => paint_line(
                &text_painter,
                &old,
                line,
                origin,
                if tab.highlight == DiffHighlight::Words {
                    &hunk.old_words
                } else {
                    &[]
                },
                colors::DELETED_WORD,
            ),
            (None, Some(line), Some(hunk)) => paint_line(
                &text_painter,
                &new,
                line,
                origin,
                if tab.highlight == DiffHighlight::Words {
                    &hunk.new_words
                } else {
                    &[]
                },
                colors::INSERTED_WORD,
            ),
            (_, Some(line), None) => {
                paint_line(&text_painter, &new, line, origin, &[], Color32::TRANSPARENT)
            }
            _ => {}
        }
    }
    let marks: Vec<_> = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            let hunk = row.hunk?;
            (index == 0 || rows[index - 1].hunk != Some(hunk)).then(|| {
                let len = rows[index..]
                    .iter()
                    .take_while(|r| r.hunk == Some(hunk))
                    .count();
                (index, len, diff.hunks[hunk].kind())
            })
        })
        .collect();
    let stripe = Rect::from_min_max(
        pos2(area.right() - 8.0, pane.top()),
        pos2(area.right() - 2.0, pane.bottom()),
    );
    error_stripe(&painter, stripe, rows.len().max(1), &marks);
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_git::FileDiff;
    use std::path::PathBuf;

    #[test]
    fn unified_rows_interleave_removed_and_added_lines() {
        let diff = FileDiff::compute(
            PathBuf::from("a"),
            None,
            "a\nb\nc\n".into(),
            "a\nB\nc\nd\n".into(),
        );
        let rows = unified_rows(&diff.hunks, 3, 4);
        let shape: Vec<_> = rows.iter().map(|r| (r.old, r.new)).collect();
        assert_eq!(
            shape,
            vec![
                (Some(0), Some(0)),
                (Some(1), None),
                (None, Some(1)),
                (Some(2), Some(2)),
                (None, Some(3)),
            ]
        );
        assert_eq!(rows[1].hunk, Some(0));
        assert_eq!(rows[4].hunk, Some(1));
    }

    fn numbered(lines: &[usize]) -> String {
        lines.iter().map(|n| format!("{n}\n")).collect()
    }

    #[test]
    fn long_unchanged_stretches_fold_but_keep_context() {
        let old: Vec<usize> = (0..30).collect();
        let mut new = old.clone();
        new[15] = 99;
        let diff = FileDiff::compute(PathBuf::from("a"), None, numbered(&old), numbered(&new));
        let folds = folds(&diff.hunks, 30, 30);
        // Lines 0..12 and 19..30 are far from the change at 15.
        assert_eq!(
            folds,
            vec![
                Fold {
                    old: 0..12,
                    new: 0..12
                },
                Fold {
                    old: 19..30,
                    new: 19..30
                },
            ]
        );
        let map = FoldMap {
            folds: &folds,
            is_new: true,
        };
        assert_eq!(map.display(12.0), 1.0);
        assert_eq!(map.display(15.0), 4.0);
        assert_eq!(map.display(30.0), 1.0 + 7.0 + 1.0);
        for line in [0.0, 12.0, 15.0, 18.0, 19.0] {
            assert_eq!(map.line(map.display(line)), line);
        }
        let rows = fold_rows(unified_rows(&diff.hunks, 30, 30), &folds);
        assert_eq!(rows.iter().filter(|r| r.folded > 0).count(), 2);
        assert_eq!(rows.len(), 1 + 3 + 2 + 3 + 1);
    }

    #[test]
    fn short_gaps_and_unchanged_files_do_not_fold() {
        let old: Vec<usize> = (0..8).collect();
        let mut new = old.clone();
        new[4] = 99;
        let diff = FileDiff::compute(PathBuf::from("a"), None, numbered(&old), numbered(&new));
        assert!(folds(&diff.hunks, 8, 8).is_empty());
        assert!(folds(&[], 10, 10).is_empty());
    }

    #[test]
    fn line_starts_follow_newlines() {
        assert_eq!(line_starts("ab\ncd\n"), vec![0, 3, 6]);
    }
}

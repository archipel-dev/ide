//! Editor gutter change markers (lines changed against `HEAD`) and the popup
//! opened by clicking one: previous/next change, rollback, show diff and the
//! `HEAD` version of the changed lines.

use crate::icons::{self, Icon};
use crate::{IdeApp, theme};
use archipel_ide_git::{Hunk, HunkKind};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, pos2, vec2};
use serde_json::{Value, json};

/// Width of the change bar; it sits between the line numbers and the text.
const MARKER_WIDTH: f32 = 4.0;
/// Right edge of the marker relative to the gutter's right edge.
const MARKER_INSET: f32 = 7.0;

/// Observed dark-theme gutter marker colours.
pub const ADDED: Color32 = Color32::from_rgb(0x54, 0x91, 0x59);
pub const MODIFIED: Color32 = Color32::from_rgb(0x37, 0x5F, 0xAD);
pub const DELETED: Color32 = Color32::from_rgb(0x86, 0x8A, 0x91);

pub fn marker_color(kind: HunkKind) -> Color32 {
    match kind {
        HunkKind::Inserted => ADDED,
        HunkKind::Modified => MODIFIED,
        HunkKind::Deleted => DELETED,
    }
}

/// Whether gutter-relative `x` hits the marker column (with some slack).
pub fn in_marker_column(x: f32, gutter_width: f32) -> bool {
    let right = gutter_width - MARKER_INSET;
    (right - MARKER_WIDTH - 4.0..=right + 4.0).contains(&x)
}

/// Paints the marker for `line` (a bar for added/modified lines, a wedge at
/// the top of the line below deleted lines). `gutter_right_top` is the
/// gutter's right edge at the line's top.
pub fn paint_marker(
    painter: &egui::Painter,
    changes: &[Hunk],
    line: usize,
    gutter_right_top: Pos2,
    row_height: f32,
) {
    let right = gutter_right_top.x - MARKER_INSET;
    let y = gutter_right_top.y;
    for hunk in changes {
        let kind = hunk.kind();
        if hunk.new.contains(&line) {
            let rect =
                Rect::from_min_max(pos2(right - MARKER_WIDTH, y), pos2(right, y + row_height));
            painter.rect_filled(rect, 0.0, marker_color(kind));
        } else if kind == HunkKind::Deleted && hunk.new.start == line {
            let points = vec![
                pos2(right - MARKER_WIDTH - 1.0, y - 4.0),
                pos2(right + 3.0, y),
                pos2(right - MARKER_WIDTH - 1.0, y + 4.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                points,
                marker_color(kind),
                Stroke::NONE,
            ));
        }
    }
}

/// An open change popup for the active document.
pub struct ChangePopup {
    pub hunk: usize,
    pub position: Pos2,
    /// Set until the first frame is drawn: the click that opened the popup
    /// must not count as a click outside it.
    fresh: bool,
}

impl ChangePopup {
    pub fn new(hunk: usize, position: Pos2) -> Self {
        Self {
            hunk,
            position,
            fresh: true,
        }
    }
}

fn tool_button(ui: &mut egui::Ui, icon: Icon, enabled: bool, tooltip: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(24.0, 24.0), egui::Sense::click());
    if enabled && response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme::HOVER);
    }
    let color = if enabled {
        theme::TEXT_SECONDARY
    } else {
        theme::TEXT_DISABLED
    };
    icons::paint(ui.painter(), rect.shrink(4.0), icon, color);
    enabled && response.on_hover_text(tooltip).clicked()
}

fn text_button(ui: &mut egui::Ui, label: &str, tooltip: &str) -> bool {
    ui.add(
        egui::Button::new(egui::RichText::new(label).color(theme::TEXT))
            .fill(Color32::TRANSPARENT)
            .min_size(vec2(0.0, 24.0)),
    )
    .on_hover_text(tooltip)
    .clicked()
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(popup) = app.change_popup.as_ref() else {
        return;
    };
    let Some(index) = app.state.active_document.filter(|_| !app.state.diff_active) else {
        app.change_popup = None;
        return;
    };
    let hunks = app.state.document_changes(index).to_vec();
    let Some(hunk) = hunks.get(popup.hunk).cloned() else {
        app.change_popup = None;
        return;
    };
    let old_text = app.state.hunk_base_text(index, &hunk).unwrap_or_default();
    let line_param = json!({"line": hunk.new.start});
    let mut action: Option<(&'static str, Value)> = None;
    let mut select = None;
    let mut close = false;
    let area = egui::Area::new(egui::Id::new("vcs_change_popup"))
        .order(egui::Order::Foreground)
        .fixed_pos(popup.position + vec2(12.0, 6.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(*theme::POPUP)
                .stroke(Stroke::new(1.0_f32, theme::POPUP_BORDER))
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        let previous = popup.hunk.checked_sub(1);
                        let next = (popup.hunk + 1 < hunks.len()).then_some(popup.hunk + 1);
                        if tool_button(
                            ui,
                            Icon::ChevronUp,
                            previous.is_some(),
                            &format!("Previous Change ({})", crate::keys::display("⌃⌥⇧↑")),
                        ) {
                            select = previous;
                        }
                        if tool_button(
                            ui,
                            Icon::ChevronDown,
                            next.is_some(),
                            &format!("Next Change ({})", crate::keys::display("⌃⌥⇧↓")),
                        ) {
                            select = next;
                        }
                        ui.add_space(6.0);
                        if text_button(
                            ui,
                            "Rollback",
                            &format!("Rollback Lines ({})", crate::keys::display("⌥⌘Z")),
                        ) {
                            action = Some(("vcs.rollbackChange", line_param.clone()));
                        }
                        if text_button(ui, "Show Diff", "Show Diff") {
                            action = Some(("vcs.showChangeDiff", line_param.clone()));
                        }
                        ui.add_space(6.0);
                        if tool_button(ui, Icon::Close, true, "Close (Esc)") {
                            close = true;
                        }
                    });
                    if !hunk.old.is_empty() {
                        old_lines(ui, &old_text);
                    }
                });
        });
    if let Some(next) = select {
        // Follow the selected change in the editor too.
        app.dispatch(
            "editor.gotoLine",
            json!({"line": hunks[next].new.start + 1}),
        );
        if let Some(popup) = app.change_popup.as_mut() {
            popup.hunk = next;
        }
        return;
    }
    let fresh = app
        .change_popup
        .as_mut()
        .is_some_and(|popup| std::mem::take(&mut popup.fresh));
    if action.is_some() || close || (!fresh && area.response.clicked_elsewhere()) {
        app.change_popup = None;
    }
    if let Some((id, params)) = action {
        app.dispatch(id, params);
    }
}

/// The `HEAD` version of the changed lines.
fn old_lines(ui: &mut egui::Ui, text: &str) {
    let lines = archipel_ide_git::split_lines(text);
    const MAX_LINES: usize = 20;
    let font = egui::FontId::monospace(theme::EDITOR_FONT_SIZE);
    let width = lines
        .iter()
        .take(MAX_LINES)
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as f32
        * ui.fonts_mut(|fonts| fonts.glyph_width(&font, 'm'))
        + 16.0;
    let height = lines.len().min(MAX_LINES) as f32 * 20.0 + 6.0;
    let (rect, _) = ui.allocate_exact_size(
        vec2(width.clamp(200.0, 900.0), height),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::from_rgb(0x3E, 0x3F, 0x42));
    for (row, line) in lines.iter().take(MAX_LINES).enumerate() {
        painter.text(
            pos2(
                rect.left() + 8.0,
                rect.top() + 3.0 + row as f32 * 20.0 + 10.0,
            ),
            egui::Align2::LEFT_CENTER,
            line.replace('\t', "    "),
            font.clone(),
            *theme::editor::TEXT,
        );
    }
    if lines.len() > MAX_LINES {
        ui.label(
            egui::RichText::new(format!("… {} more lines", lines.len() - MAX_LINES))
                .color(theme::TEXT_INFO),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_column_is_just_left_of_the_text() {
        let gutter = 60.0;
        assert!(in_marker_column(gutter - MARKER_INSET - 1.0, gutter));
        assert!(!in_marker_column(10.0, gutter));
        assert!(!in_marker_column(gutter + 20.0, gutter));
    }
}

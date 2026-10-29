//! Code completion popup ("lookup").
//!
//! Row layout follows the reference IDE: kind icon, label with the matched
//! characters in bold, the signature in grey right after it and the type
//! right-aligned; deprecated items are struck through. The popup is anchored
//! so labels line up with the identifier being completed.

use crate::{IdeApp, theme};
use archipel_ide_view::popups::completion::{self as model, BadgeTone, CompletionRow};
use eframe::egui::{self, Color32, FontId, Rect, Sense, Stroke, pos2, text::LayoutJob, vec2};
use serde_json::json;

const ROW_HEIGHT: f32 = 24.0;
const VISIBLE_ROWS: usize = 11;
const ICON_COLUMN: f32 = 26.0;
const FOOTER_HEIGHT: f32 = 28.0;
/// UI-POP-032: 573 px wide.
const MIN_WIDTH: f32 = 573.0;
const MAX_WIDTH: f32 = 720.0;
/// Matched characters (blue, like the reference lookup's match highlight).
const MATCHED: Color32 = Color32::from_rgb(0x56, 0xA8, 0xF5);

/// Kind icon: an original letter-in-circle glyph with a per-kind color.
fn kind_glyph(kind: &str) -> Option<(char, Color32)> {
    let method = Color32::from_rgb(0xE5, 0x6A, 0x6A);
    let field = Color32::from_rgb(0xD9, 0xA3, 0x43);
    let class = Color32::from_rgb(0x56, 0x8A, 0xF2);
    let interface = Color32::from_rgb(0x5F, 0xB8, 0x65);
    let variable = Color32::from_rgb(0xA8, 0x86, 0xD9);
    let badge = model::kind_badge(kind)?;
    // The popup has no badge for keywords.
    if badge.letter == 'k' {
        return None;
    }
    Some((
        badge.letter,
        match badge.tone {
            BadgeTone::Method => method,
            BadgeTone::Field => field,
            BadgeTone::Class => class,
            BadgeTone::Interface => interface,
            BadgeTone::Variable => variable,
            BadgeTone::Neutral => *theme::TEXT_SECONDARY,
        },
    ))
}

fn paint_kind_icon(painter: &egui::Painter, center: egui::Pos2, kind: &str) {
    let Some((letter, color)) = kind_glyph(kind) else {
        return;
    };
    painter.circle(
        center,
        6.5,
        color.gamma_multiply(0.18),
        Stroke::new(1.2_f32, color),
    );
    painter.text(
        center + vec2(0.0, -0.5),
        egui::Align2::CENTER_CENTER,
        letter,
        theme::mono_bold(9.5),
        color,
    );
}

/// Label with the matched characters in bold and highlighted.
fn label_job(label: &str, matched: &[usize], deprecated: bool, selected: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    let normal = if selected {
        *theme::TEXT_BRIGHT
    } else {
        *theme::editor::TEXT
    };
    let strike = if deprecated {
        Stroke::new(1.0_f32, *theme::TEXT_SECONDARY)
    } else {
        Stroke::NONE
    };
    let mut buffer = String::new();
    let mut buffer_matched = false;
    // Parameter lists (`name(String arg)`) are grey, like the reference lookup.
    let params_start = label.chars().position(|c| c == '(').unwrap_or(usize::MAX);
    let grey = *theme::TEXT_INFO;
    let flush = |job: &mut LayoutJob, text: &mut String, matched: bool, params: bool| {
        if text.is_empty() {
            return;
        }
        let format = egui::TextFormat {
            font_id: if matched {
                theme::mono_bold(theme::EDITOR_FONT_SIZE)
            } else {
                FontId::monospace(theme::EDITOR_FONT_SIZE)
            },
            color: if params {
                grey
            } else if matched {
                MATCHED
            } else {
                normal
            },
            strikethrough: strike,
            ..Default::default()
        };
        job.append(text, 0.0, format);
        text.clear();
    };
    for (index, c) in label.chars().enumerate() {
        let is_matched = matched.contains(&index);
        if is_matched != buffer_matched || index == params_start {
            flush(&mut job, &mut buffer, buffer_matched, index > params_start);
            buffer_matched = is_matched;
        }
        buffer.push(c);
    }
    flush(
        &mut job,
        &mut buffer,
        buffer_matched,
        label.chars().count() > params_start,
    );
    job
}

type Row = CompletionRow;

/// Shows the popup; returns its screen rect for the documentation panel.
pub fn show(app: &mut IdeApp, ctx: &egui::Context) -> Option<Rect> {
    let session = app.state.completion().as_ref()?;
    if !session.visible() && !session.indexing_only() {
        return None;
    }
    let caret = app.caret_screen_pos?;
    // A click that takes keyboard focus away from the editor (project tree,
    // tool window, toolbar) dismisses the popup, as in the reference IDE.
    // Clicks on the popup itself select a row and are handled below.
    if session.visible() && !app.editor_focused(ctx) {
        let popup_rect = ctx.memory(|memory| memory.area_rect(egui::Id::new("completion_popup")));
        let pressed_outside = ctx.input(|input| {
            input.pointer.any_pressed()
                && input
                    .pointer
                    .interact_pos()
                    .is_some_and(|pos| !popup_rect.is_some_and(|rect| rect.contains(pos)))
        });
        if pressed_outside {
            app.dispatch("completion.close", serde_json::Value::Null);
            return None;
        }
    }
    // The server's activity, else why the native answer may be incomplete.
    let indexing = session.indexing.clone().or_else(|| session.note.clone());
    if session.indexing.is_some() {
        // Re-query and refresh the notice when the server finishes.
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
    let rows: Vec<Row> = model::rows(session);
    let selected = session.selected;
    let prefix_chars = session.prefix.chars().count() as f32;

    let mono = FontId::monospace(theme::EDITOR_FONT_SIZE);
    let small = FontId::monospace(theme::EDITOR_FONT_SIZE - 1.0);
    let char_width = ctx.fonts_mut(|fonts| fonts.glyph_width(&mono, 'M'));
    // Width from the widest of the first rows, like the reference popup.
    let content_width = rows
        .iter()
        .take(60)
        .map(|row| {
            let label = row.label.chars().count() as f32 * char_width;
            let detail = row
                .detail
                .as_ref()
                .map_or(0.0, |d| d.chars().count() as f32 * char_width);
            let kind = row.tail.as_ref().map_or(0.0, |t| {
                t.chars().count().min(40) as f32 * char_width + 24.0
            });
            ICON_COLUMN + 8.0 + label + detail + kind + 16.0
        })
        .fold(MIN_WIDTH, f32::max)
        .min(MAX_WIDTH)
        .max(if indexing.is_some() { 360.0 } else { 0.0 });
    let visible = rows.len().min(VISIBLE_ROWS);
    let height = visible as f32 * ROW_HEIGHT + FOOTER_HEIGHT + 2.0;
    let screen = ctx.content_rect();
    // Align the label column with the start of the typed identifier.
    let mut x = caret.x - prefix_chars * char_width - ICON_COLUMN - 6.0;
    x = x
        .min(screen.right() - content_width - 8.0)
        .max(screen.left() + 4.0);
    let mut y = caret.y + 2.0;
    if y + height > screen.bottom() - crate::chrome::STATUS_BAR_HEIGHT {
        y = caret.y - ROW_HEIGHT - height - 2.0;
    }

    let mut clicked = None;
    let area = egui::Area::new(egui::Id::new("completion_popup"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(x, y))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(*theme::POPUP)
                .stroke(Stroke::new(1.0_f32, *theme::POPUP_BORDER))
                .corner_radius(8)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 12,
                    spread: 0,
                    color: Color32::from_black_alpha(90),
                })
                .inner_margin(egui::Margin::same(1))
                .show(ui, |ui| {
                    ui.set_width(content_width);
                    egui::ScrollArea::vertical()
                        .max_height(visible as f32 * ROW_HEIGHT)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            for (index, row) in rows.iter().enumerate() {
                                let (rect, response) = ui.allocate_exact_size(
                                    vec2(ui.available_width(), ROW_HEIGHT),
                                    Sense::click(),
                                );
                                paint_row(ui, rect, row, index == selected, response.hovered());
                                if index == selected {
                                    ui.scroll_to_rect(rect, None);
                                }
                                if response.clicked() {
                                    clicked = Some(index);
                                }
                            }
                        });
                    footer(ui, &small, indexing.as_deref());
                });
        });
    if let Some(row) = clicked {
        app.dispatch("completion.accept", json!({"row": row}));
    }
    Some(area.response.rect)
}

fn paint_row(ui: &egui::Ui, rect: Rect, row: &Row, selected: bool, hovered: bool) {
    let painter = ui.painter().with_clip_rect(rect);
    if selected {
        // UI-POP-031: the lookup list keeps the editor's focus, so its selection is the
        // inactive one (Gray4), not the blue of a focused list.
        painter.rect_filled(
            rect.shrink2(vec2(5.0, 0.0)),
            4.0,
            *theme::SELECTION_INACTIVE,
        );
    } else if hovered {
        painter.rect_filled(rect.shrink2(vec2(4.0, 1.0)), 4.0, *theme::HOVER);
    }
    paint_kind_icon(
        &painter,
        pos2(rect.left() + ICON_COLUMN / 2.0 + 4.0, rect.center().y),
        row.kind,
    );
    let mut x = rect.left() + ICON_COLUMN + 6.0;
    let label = painter.layout_job(label_job(
        &row.label,
        &row.matched,
        row.deprecated,
        selected,
    ));
    let label_width = label.size().x;
    painter.galley(
        pos2(x, rect.center().y - label.size().y / 2.0),
        label,
        *theme::TEXT,
    );
    x += label_width;
    let grey = *theme::TEXT_INFO;
    let right_edge = rect.right() - 12.0;
    let mut type_left = right_edge;
    if let Some(tail) = &row.tail {
        let galley = painter.layout_no_wrap(
            tail.clone(),
            FontId::monospace(theme::EDITOR_FONT_SIZE - 1.0),
            grey,
        );
        type_left = (right_edge - galley.size().x).max(x + 24.0);
        painter.galley(
            pos2(type_left, rect.center().y - galley.size().y / 2.0),
            galley,
            grey,
        );
    }
    if let Some(detail) = &row.detail {
        let clip = Rect::from_min_max(rect.min, pos2(type_left - 12.0, rect.max.y));
        let galley = painter.layout_no_wrap(
            detail.clone(),
            FontId::monospace(theme::EDITOR_FONT_SIZE),
            grey,
        );
        painter.with_clip_rect(clip).galley(
            pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            grey,
        );
    }
}

/// The footer hint, or the indexing notice while the language server is
/// still importing the project (`indexing` is its progress, e.g.
/// "jdtls: Building 0%").
fn footer(ui: &mut egui::Ui, font: &FontId, indexing: Option<&str>) {
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), FOOTER_HEIGHT), Sense::hover());
    let painter = ui.painter();
    painter.hline(
        rect.x_range(),
        rect.top(),
        Stroke::new(1.0_f32, *theme::POPUP_BORDER),
    );
    let (hint, color) = match indexing {
        Some(status) => (indexing_notice(status), *theme::WARNING),
        None => (
            model::footer_hint(cfg!(target_os = "macos")).to_owned(),
            *theme::TEXT_INFO,
        ),
    };
    painter.with_clip_rect(rect).text(
        pos2(rect.left() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        hint,
        FontId::proportional(font.size - 0.5),
        color,
    );
}

/// "Indexing… results may be incomplete (jdtls: Building 0%)".
pub(crate) fn indexing_notice(status: &str) -> String {
    model::indexing_notice(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexing_notice_names_the_server_activity() {
        assert_eq!(
            indexing_notice("jdtls: Building 0%"),
            "Indexing… results may be incomplete (jdtls: Building 0%)"
        );
    }

    #[test]
    fn matched_characters_are_split_into_bold_sections() {
        let job = label_job("getRequiredField", &[0, 3, 11], false, false);
        assert_eq!(job.text, "getRequiredField");
        // g | et | R | equired | F | ield
        assert_eq!(job.sections.len(), 6);
        assert_eq!(
            job.sections[0].format.font_id,
            theme::mono_bold(theme::EDITOR_FONT_SIZE)
        );
    }

    #[test]
    fn parameter_lists_in_labels_are_grey() {
        let job = label_job("main(String[] args)", &[0], false, false);
        assert_eq!(job.text, "main(String[] args)");
        let last = job.sections.last().unwrap();
        assert_eq!(last.format.color, *theme::TEXT_INFO);
        assert_ne!(job.sections[0].format.color, *theme::TEXT_INFO);
    }

    #[test]
    fn kind_badges_follow_the_view_model() {
        assert!(kind_glyph("method").is_some());
        assert!(kind_glyph("text").is_none());
        assert!(kind_glyph("keyword").is_none());
    }
}

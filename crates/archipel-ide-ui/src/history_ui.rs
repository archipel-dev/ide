//! Local History: the Show History dialog (IJ-ED-028, IJ-TW-034). A list of
//! revisions on the left (time, cause, size), the selected revision's diff
//! against the current text on the right, Compare with Current, Revert and
//! Put Label. State and commands live in `archipel_ide_core::history_commands`.

use crate::{IdeApp, theme};
use eframe::egui::{self, Color32, RichText};
use serde_json::json;

const WIDTH: f32 = 920.0;
const HEIGHT: f32 = 520.0;
const LIST_WIDTH: f32 = 320.0;

/// `YYYY-MM-DD HH:MM` in UTC from milliseconds since the epoch.
pub(crate) fn utc_stamp(ms: u64) -> String {
    let secs = ms / 1000;
    let (days, rest) = (secs / 86_400, secs % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}",
        rest / 3600,
        rest % 3600 / 60
    )
}

/// "just now", "5 min ago", "3 h ago", "2 days ago".
pub(crate) fn age(now_ms: u64, time_ms: u64) -> String {
    let secs = now_ms.saturating_sub(time_ms) / 1000;
    match secs {
        0..=44 => "just now".into(),
        45..=3_599 => format!("{} min ago", (secs + 30) / 60),
        3_600..=86_399 => format!("{} h ago", secs / 3600),
        _ => format!("{} days ago", secs / 86_400),
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(dialog) = app.state.analysis.history.dialog.clone() else {
        return;
    };
    let mut select = None;
    let mut close = false;
    let mut compare = None;
    let mut revert = None;
    let mut label = None;
    let now = now_ms();
    crate::popups::popup_frame(ctx, "history_dialog", WIDTH, |ui| {
        let title = match &dialog.path {
            Some(path) => format!("History: {}", path.display()),
            None => "Project History".to_owned(),
        };
        ui.label(
            RichText::new(title)
                .font(theme::semibold(theme::UI_FONT_SIZE + 2.0))
                .color(theme::TEXT_BRIGHT),
        );
        ui.add_space(6.0);
        if dialog.revisions.is_empty() {
            ui.label(
                RichText::new(
                    "No revisions yet: history starts with the first save or outside change.",
                )
                .color(theme::TEXT_SECONDARY),
            );
        }
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(LIST_WIDTH, HEIGHT - 110.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("history_revisions")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            for (index, row) in dialog.revisions.iter().enumerate() {
                                let selected = dialog.selected == Some(index);
                                let (rect, response) = crate::widgets::list_row(ui, selected);
                                let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
                                let end = crate::widgets::row_text(
                                    ui,
                                    rect,
                                    rect.left() + 8.0,
                                    &utc_stamp(row.time_ms),
                                    font.clone(),
                                    theme::TEXT_BRIGHT,
                                );
                                let mut text = row.label.clone();
                                if dialog.path.is_none() && !row.path.as_os_str().is_empty() {
                                    text = format!("{} · {text}", row.path.display());
                                }
                                crate::widgets::row_text(
                                    ui,
                                    rect,
                                    end + 8.0,
                                    &text,
                                    font,
                                    theme::TEXT_SECONDARY,
                                );
                                response.clone().on_hover_text(age(now, row.time_ms));
                                if response.clicked() {
                                    select = Some(index);
                                }
                            }
                        });
                },
            );
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.set_width(WIDTH - LIST_WIDTH - 60.0);
                if let Some(note) = &dialog.note {
                    ui.label(RichText::new(note).color(theme::TEXT_SECONDARY));
                } else if dialog.selected.is_some() && dialog.preview.is_empty() {
                    ui.label(
                        RichText::new("Same as the current text.").color(theme::TEXT_SECONDARY),
                    );
                }
                egui::ScrollArea::both()
                    .id_salt("history_preview")
                    .max_height(HEIGHT - 110.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for line in &dialog.preview {
                            let color: Color32 = match line.mark {
                                '+' => *theme::ADDED,
                                '-' => *theme::ERROR,
                                _ => *theme::TEXT_SECONDARY,
                            };
                            ui.label(
                                RichText::new(format!("{} {}", line.mark, line.text))
                                    .monospace()
                                    .color(color),
                            );
                        }
                    });
            });
        });
        ui.add_space(8.0);
        let selected = dialog
            .selected
            .and_then(|index| dialog.revisions.get(index))
            .filter(|row| row.has_content);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    selected.is_some(),
                    egui::Button::new("Compare with Current"),
                )
                .clicked()
                && let Some(row) = selected
            {
                compare = Some((row.id, row.path.clone()));
            }
            if ui
                .add_enabled(selected.is_some(), egui::Button::new("Revert"))
                .clicked()
                && let Some(row) = selected
            {
                revert = Some((row.id, row.path.clone()));
            }
            ui.separator();
            let key = egui::Id::new("history_label_text");
            let mut text: String = ui.data(|d| d.get_temp(key)).unwrap_or_default();
            let edit = ui.add(
                egui::TextEdit::singleline(&mut text)
                    .hint_text("Label")
                    .desired_width(160.0),
            );
            ui.data_mut(|d| d.insert_temp(key, text.clone()));
            if ui
                .add_enabled(!text.trim().is_empty(), egui::Button::new("Put Label"))
                .clicked()
                || (edit.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && !text.trim().is_empty())
            {
                label = Some(text.clone());
                ui.data_mut(|d| d.insert_temp(key, String::new()));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    close = true;
                }
            });
        });
    });
    if let Some(index) = select {
        app.dispatch("history.select", json!({"index": index}));
    }
    if let Some((revision, path)) = compare {
        app.dispatch(
            "history.compare",
            json!({"revision": revision, "path": path}),
        );
    }
    if let Some((revision, path)) = revert {
        app.dispatch(
            "history.revert",
            json!({"revision": revision, "path": path}),
        );
    }
    if let Some(text) = label {
        app.dispatch("history.label", json!({"text": text}));
        if let Some(path) = dialog.path.clone() {
            app.dispatch("history.show", json!({"path": path}));
        } else {
            app.dispatch("history.project", json!({}));
        }
    }
    if close {
        app.dispatch("history.close", json!({}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_and_ages_format() {
        assert_eq!(utc_stamp(0), "1970-01-01 00:00");
        assert_eq!(utc_stamp(1_700_000_000_000), "2023-11-14 22:13");
        assert_eq!(utc_stamp(951_782_400_000 + 59 * 60_000), "2000-02-29 00:59");
        assert_eq!(age(10_000, 9_000), "just now");
        assert_eq!(age(1_000_000, 700_000), "5 min ago");
        assert_eq!(age(10 * 3_600_000, 3_600_000), "9 h ago");
        assert_eq!(age(3 * 86_400_000, 0), "3 days ago");
    }
}

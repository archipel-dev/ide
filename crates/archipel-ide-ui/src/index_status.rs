//! Indexing progress in the status bar and the "indexing" banner shown by
//! popups and tool windows whose results depend on an index that is not
//! ready yet (dumb mode, per index).

use crate::{IdeApp, icons::Icon, theme, widgets};
use archipel_ide_core::IdeState;
use archipel_ide_index::progress::state_label;
use archipel_ide_index::{IndexStatus, Phase};
use eframe::egui::{self, RichText};
use serde_json::json;

/// Runs shorter than this many files show no status-bar item (single edits).
const MIN_VISIBLE_FILES: u32 = 10;

/// The status text of a busy indexer, or `None` when nothing should show.
pub fn visible_summary(status: &IndexStatus) -> Option<String> {
    if status.phase == Phase::Idle && !status.dumb {
        return None;
    }
    if !status.dumb && status.phase == Phase::Indexing && status.total < MIN_VISIBLE_FILES {
        return None;
    }
    status
        .summary()
        .or_else(|| status.dumb.then(|| "Indexing…".to_owned()))
}

/// Status-bar item: spinner and "Indexing 1,204 / 20,000 files" while the
/// indexer works; hover lists every index, right-click offers a rebuild.
pub fn status_widget(app: &mut IdeApp, ui: &mut egui::Ui, font: egui::FontId) {
    if !app.state.analysis.index.is_attached() {
        return;
    }
    let status = app.state.index_status();
    let Some(text) = visible_summary(&status) else {
        return;
    };
    let response = widgets::flat_widget(ui, None, &text, font, theme::TEXT_INFO, false, 22.0);
    widgets::busy_spinner(ui, 12.0, *theme::TEXT_INFO);
    let response = response.on_hover_ui(|ui| {
        ui.set_max_width(360.0);
        ui.label(RichText::new("Indexing").strong());
        if status.dumb {
            ui.label("Some features are limited until indexing finishes.");
        }
        for info in &status.indexes {
            let state = match info.state {
                archipel_ide_analysis::IndexState::Building { done, total } => {
                    format!("building {done} / {total}")
                }
                other => state_label(other).to_owned(),
            };
            ui.label(format!("{} — {state}", info.id.0));
        }
        if status.skipped_total > 0 {
            ui.label(
                RichText::new(format!("{} files skipped", status.skipped_total))
                    .color(theme::TEXT_SECONDARY),
            );
        }
        ui.label(RichText::new("Right-click for actions").color(theme::TEXT_INFO));
    });
    let mut rebuild = false;
    response.context_menu(|ui| {
        if ui.button("Rebuild Indexes").clicked() {
            rebuild = true;
            ui.close();
        }
    });
    if rebuild {
        app.dispatch("index.rebuild", json!({}));
    }
}

/// A one-line banner "Indexing… results may be incomplete" with a spinner,
/// or nothing when the state has no pending index.
pub fn banner(ui: &mut egui::Ui, state: &IdeState) {
    if !state.analysis.index.is_attached() {
        return;
    }
    let status = state.index_status();
    if !status.dumb && !status.is_busy() {
        return;
    }
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        crate::icons::show(ui, Icon::Warning, 14.0, *theme::WARNING);
        let text = status
            .summary()
            .map(|s| format!("{s}: results may be incomplete"))
            .unwrap_or_else(|| "Indexing: results may be incomplete".into());
        ui.label(RichText::new(text).color(theme::TEXT_SECONDARY));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_index::Phase;

    #[test]
    fn short_runs_stay_invisible_and_dumb_mode_always_shows() {
        let mut status = IndexStatus::detached();
        status.attached = true;
        status.phase = Phase::Indexing;
        status.total = 3;
        assert_eq!(visible_summary(&status), None);
        status.total = 2_000;
        status.done = 5;
        assert_eq!(
            visible_summary(&status).as_deref(),
            Some("Indexing 5 / 2,000 files")
        );
        status.phase = Phase::Idle;
        assert_eq!(visible_summary(&status), None);
        status.dumb = true;
        assert_eq!(visible_summary(&status).as_deref(), Some("Indexing…"));
        status.phase = Phase::Scanning;
        assert_eq!(visible_summary(&status).as_deref(), Some("Scanning files…"));
    }
}

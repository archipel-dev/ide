//! Floating "Load Gradle Changes" notification shown when build scripts changed.

use crate::{IdeApp, theme};
use eframe::egui::{self, RichText};

pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    if !app.state.gradle_changes_pending() || !app.state.is_gradle_project() {
        return;
    }
    let busy = app.state.execution().is_running();
    let mut action = None;
    egui::Frame::new()
        .fill(*theme::BANNER_WARNING)
        .stroke(egui::Stroke::new(1.0_f32, *theme::BANNER_WARNING_BORDER))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(10, 6))
        .outer_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Gradle build scripts have changed").color(theme::TEXT_BRIGHT),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Links underline on hover, like the other banners' actions.
                    let link = |ui: &mut egui::Ui, text: &str, color| {
                        let response = ui.add(
                            egui::Label::new(RichText::new(text).color(color))
                                .sense(egui::Sense::click()),
                        );
                        if response.hovered() {
                            let line = response.rect.shrink2(egui::vec2(0.0, 1.0));
                            ui.painter().hline(
                                line.x_range(),
                                line.bottom(),
                                egui::Stroke::new(1.0_f32, color),
                            );
                        }
                        response
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                    };
                    if link(ui, "Ignore", theme::TEXT_SECONDARY) {
                        action = Some("gradle.dismissChanges");
                    }
                    ui.add_space(12.0);
                    if !busy && link(ui, "Load Gradle Changes", theme::MODIFIED) {
                        action = Some("gradle.loadChanges");
                    }
                });
            });
        });
    if let Some(id) = action {
        app.dispatch(id, serde_json::Value::Null);
    }
}

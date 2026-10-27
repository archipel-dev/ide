//! "Archipel IDE closed unexpectedly" balloon after a crash in a previous session.

use crate::{IdeApp, theme};
use eframe::egui::{self, RichText};
use serde_json::Value;

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    if app.state.crash_report.is_none() {
        return;
    }
    let screen = ctx.content_rect();
    let width = 340.0;
    // Stack above the update balloon when both are showing.
    let lift = if app.state.notifications().update_balloon() {
        150.0
    } else {
        0.0
    };
    let mut action = None;
    egui::Area::new(egui::Id::new("crash_notice"))
        .order(crate::notifications::BALLOON_ORDER)
        .fixed_pos(egui::pos2(
            screen.right() - width - 16.0,
            screen.bottom() - 150.0 - lift - crate::chrome::STATUS_BAR_HEIGHT,
        ))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(*theme::NOTIFICATION)
                .stroke(egui::Stroke::new(1.0_f32, theme::NOTIFICATION))
                .corner_radius(8)
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    ui.set_width(width - 28.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Archipel IDE closed unexpectedly")
                                .font(theme::semibold(theme::UI_FONT_SIZE))
                                .color(theme::TEXT_BRIGHT),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if crate::icons::button(ui, crate::icons::Icon::Close, "Dismiss")
                                .clicked()
                            {
                                action = Some("app.dismissCrashReport");
                            }
                        });
                    });
                    ui.label(
                        RichText::new(
                            "A crash report was saved. Attach it to a bug report so it can be fixed.",
                        )
                        .color(theme::TEXT),
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        for (text, id) in [
                            ("Open Report", "app.openCrashReport"),
                            ("Show Log in Files", "app.showLog"),
                        ] {
                            let clicked = ui
                                .add(
                                    egui::Label::new(RichText::new(text).color(theme::MODIFIED))
                                        .sense(egui::Sense::click()),
                                )
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked();
                            if clicked {
                                action = Some(id);
                            }
                            ui.add_space(12.0);
                        }
                    });
                });
        });
    if let Some(id) = action {
        app.dispatch(id, Value::Null);
    }
}

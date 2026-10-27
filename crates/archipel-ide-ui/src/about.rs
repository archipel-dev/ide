//! Help → About: version and build details with a copy button for bug reports.

use crate::{IdeApp, theme};
use archipel_ide_core::build_info::BUILD;
use eframe::egui::{self, RichText};

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    if !app.state.about_open() {
        return;
    }
    let width = 360.0;
    let screen = ctx.content_rect();
    let mut close = false;
    egui::Area::new(egui::Id::new("about_dialog"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(
            screen.center().x - width / 2.0,
            screen.top() + screen.height() * 0.2,
        ))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .corner_radius(10)
                .inner_margin(egui::Margin::same(16))
                .show(ui, |ui| {
                    ui.set_width(width - 32.0);
                    ui.label(
                        RichText::new("Archipel IDE")
                            .font(theme::semibold(theme::UI_FONT_SIZE + 6.0))
                            .color(theme::TEXT_BRIGHT),
                    );
                    ui.label(
                        RichText::new("Native Rust IDE platform").color(theme::TEXT_SECONDARY),
                    );
                    ui.add_space(10.0);
                    egui::Grid::new("about_rows")
                        .num_columns(2)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            for (label, value) in BUILD.rows() {
                                ui.label(RichText::new(label).color(theme::TEXT_SECONDARY));
                                ui.label(RichText::new(value).color(theme::TEXT));
                                ui.end_row();
                            }
                        });
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(
                            "Apache-2.0 OR MIT. Not affiliated with or endorsed by JetBrains.",
                        )
                        .color(theme::TEXT_INFO),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button("Copy")
                            .on_hover_text("Copy build details")
                            .clicked()
                        {
                            ctx.copy_text(BUILD.report());
                            app.state.set_status("Build details copied".into());
                        }
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                });
        });
    // Escape is handled with the other dismiss keys in `IdeApp`.
    if close {
        app.dispatch("app.closeAbout", serde_json::Value::Null);
    }
}

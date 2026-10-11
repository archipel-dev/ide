//! Help → About: version and build details with a copy button for bug reports.

use crate::IdeApp;
use crate::dialog::{self, Outcome, Shell};
use archipel_kit::components::PropertyGrid;
use eframe::egui;
use jetthing_core::build_info::BUILD;

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    if !app.state.about_open {
        return;
    }
    let outcome = Shell::new("about", "Jetthing")
        .width(400.0)
        .ok("Close")
        .cancel(None)
        .secondary("Copy", true)
        .show(ctx, |ui| {
            dialog::message(ui, "Native Rust IDE platform");
            let mut grid = PropertyGrid::new("about_rows");
            for (label, value) in BUILD.rows() {
                grid = grid.row(label, value);
            }
            grid.show(ui);
            dialog::hint(
                ui,
                "Apache-2.0 OR MIT. Not affiliated with or endorsed by JetBrains.",
            );
        });
    match outcome {
        Some(Outcome::Secondary(_)) => {
            ctx.copy_text(BUILD.report());
            app.state.status = "Build details copied".into();
        }
        // Escape is also handled with the other dismiss keys in `IdeApp`.
        Some(_) => app.dispatch("app.closeAbout", serde_json::Value::Null),
        None => {}
    }
}

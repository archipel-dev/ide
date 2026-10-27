//! Build Variants tool window: one row per Android module with its active
//! variant, chosen from the variants declared in the module's build script.

use crate::{IdeApp, theme};
use eframe::egui::{self, RichText};
use serde_json::json;

pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(project) = app.state.android().project.as_ref() else {
        ui.add_space(12.0);
        ui.label(RichText::new("Not an Android project").color(theme::TEXT_INFO));
        return;
    };
    let modules: Vec<(String, Vec<String>)> = project
        .modules
        .iter()
        .map(|m| {
            (
                m.gradle_path.clone(),
                m.variants.variants.iter().map(|v| v.name.clone()).collect(),
            )
        })
        .collect();
    let mut choice = None;
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(12, 6))
        .show(ui, |ui| {
            egui::Grid::new("build_variants")
                .num_columns(2)
                .spacing([24.0, 8.0])
                .striped(false)
                .show(ui, |ui| {
                    ui.label(RichText::new("Module").color(theme::TEXT_SECONDARY));
                    ui.label(RichText::new("Active Build Variant").color(theme::TEXT_SECONDARY));
                    ui.end_row();
                    for (module, variants) in &modules {
                        ui.label(module.trim_start_matches(':'));
                        let active = app.state.active_variant(module).unwrap_or_default();
                        egui::ComboBox::from_id_salt(("variant", module))
                            .selected_text(&active)
                            .width(180.0)
                            .show_ui(ui, |ui| {
                                for variant in variants {
                                    if ui.selectable_label(*variant == active, variant).clicked() {
                                        choice = Some((module.clone(), variant.clone()));
                                    }
                                }
                            });
                        ui.end_row();
                    }
                });
            ui.add_space(10.0);
            ui.label(
                RichText::new(
                    "Variants are read from buildTypes and productFlavors in the build scripts.",
                )
                .color(theme::TEXT_INFO),
            );
        });
    if let Some((module, variant)) = choice {
        app.dispatch(
            "android.variants.select",
            json!({"module": module, "variant": variant}),
        );
    }
}

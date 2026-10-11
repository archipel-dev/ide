//! "Customize Jetthing" on first start (and Help → Customize Jetthing…):
//! theme, keymap and an optional settings import, applied together through
//! `app.completeCustomize`.

use crate::IdeApp;
use crate::dialog::{self, Outcome, Shell};
use archipel_kit::Theme;
use archipel_kit::components::{Dropdown, SegmentedControl, TextField};
use eframe::egui;
use jetthing_settings::ids;
use serde_json::{Value, json};

/// Choices made in the wizard before they are applied.
#[derive(Debug, Clone, Default)]
pub struct Draft {
    pub theme: String,
    pub keymap: String,
    pub import_path: String,
}

impl Draft {
    fn current(app: &IdeApp) -> Draft {
        let settings = &app.state.settings;
        Draft {
            theme: settings.text(ids::THEME).unwrap_or_default(),
            keymap: settings.text(ids::KEYMAP).unwrap_or_default(),
            import_path: String::new(),
        }
    }
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    if !app.state.first_run_open {
        app.customize = None;
        return;
    }
    let mut draft = app.customize.take().unwrap_or_else(|| Draft::current(app));
    let themes = app.state.setting_options(ids::THEME);
    let keymaps = app.state.setting_options(ids::KEYMAP);
    let outcome = Shell::new("first_run", "Welcome to Jetthing")
        .width(480.0)
        .ok("Start Using Jetthing")
        .cancel(Some("Skip"))
        .show(ctx, |ui| {
            dialog::message(
                ui,
                "Pick a look and a keymap. You can change both later in Settings.",
            );
            section(ui, "Theme");
            ui.add(SegmentedControl::new(
                &mut draft.theme,
                themes.iter().cloned(),
            ));
            section(ui, "Keymap");
            // The dropdown shows labels; the draft keeps the id.
            let labels: Vec<String> = keymaps.iter().map(|(_, label)| label.clone()).collect();
            let mut label = keymaps
                .iter()
                .find(|(id, _)| *id == draft.keymap)
                .map_or_else(|| draft.keymap.clone(), |(_, label)| label.clone());
            let before = label.clone();
            ui.add(
                Dropdown::new("first_run_keymap", &mut label, labels)
                    .label("Keymap")
                    .width(220.0),
            );
            if label != before
                && let Some((id, _)) = keymaps.iter().find(|(_, l)| *l == label)
            {
                draft.keymap = id.clone();
            }
            ui.add(
                TextField::new(&mut draft.import_path)
                    .label("Import settings (optional)")
                    .placeholder("Path to settings.toml from another installation")
                    .monospace(true),
            );
        });
    let action: Option<(&str, Value)> = match outcome {
        Some(Outcome::Ok) => {
            let mut params = json!({
                "theme": draft.theme,
                "keymap": draft.keymap,
            });
            if !draft.import_path.trim().is_empty() {
                params["importFrom"] = json!(draft.import_path.trim());
            }
            Some(("app.completeCustomize", params))
        }
        Some(_) => Some(("app.skipCustomize", Value::Null)),
        None => None,
    };
    app.customize = Some(draft);
    if let Some((id, params)) = action {
        app.dispatch(id, params);
    }
}

fn section(ui: &mut egui::Ui, text: &str) {
    let colors = Theme::colors(ui.ctx());
    ui.label(egui::RichText::new(text).color(colors.text));
}

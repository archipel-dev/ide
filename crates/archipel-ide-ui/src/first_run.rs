//! "Customize Archipel IDE" on first start (and Help → Customize Archipel IDE…):
//! theme, keymap and an optional settings import, applied together through
//! `app.completeCustomize`.

use crate::{IdeApp, theme};
use archipel_ide_settings::ids;
use eframe::egui::{self, RichText};
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
        app.first_run_skip = None;
        return;
    }
    let mut draft = app.customize.take().unwrap_or_else(|| Draft::current(app));
    let themes = app.state.setting_options(ids::THEME);
    let keymaps = app.state.setting_options(ids::KEYMAP);
    let width = 460.0;
    let screen = ctx.content_rect();
    let mut action: Option<(&str, Value)> = None;
    let mut skip_rect = None;
    // Modal: the welcome buttons behind it stay inert until a choice is made,
    // otherwise a click on "New Project" stacks a second dialog on top.
    crate::popups::modal_backdrop(ctx, "first_run_backdrop");
    egui::Area::new(egui::Id::new("first_run_dialog"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(
            screen.center().x - width / 2.0,
            screen.top() + screen.height() * 0.15,
        ))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .corner_radius(10)
                .inner_margin(egui::Margin::same(18))
                .show(ui, |ui| {
                    ui.set_width(width - 36.0);
                    ui.label(
                        RichText::new("Welcome to Archipel IDE")
                            .font(theme::semibold(theme::UI_FONT_SIZE + 6.0))
                            .color(theme::TEXT_BRIGHT),
                    );
                    ui.label(
                        RichText::new(
                            "Pick a look and a keymap. You can change both later in Settings.",
                        )
                        .color(theme::TEXT_SECONDARY),
                    );
                    ui.add_space(12.0);
                    section(ui, "Theme");
                    ui.horizontal(|ui| {
                        for (id, label) in &themes {
                            ui.selectable_value(&mut draft.theme, id.clone(), label);
                        }
                    });
                    ui.add_space(10.0);
                    section(ui, "Keymap");
                    let selected = keymaps
                        .iter()
                        .find(|(id, _)| *id == draft.keymap)
                        .map(|(_, label)| label.clone())
                        .unwrap_or_else(|| draft.keymap.clone());
                    egui::ComboBox::from_id_salt("first_run_keymap")
                        .selected_text(selected)
                        .width(200.0)
                        .show_ui(ui, |ui| {
                            for (id, label) in &keymaps {
                                ui.selectable_value(&mut draft.keymap, id.clone(), label);
                            }
                        });
                    ui.add_space(10.0);
                    section(ui, "Import settings (optional)");
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.import_path)
                            .hint_text("Path to settings.toml from another installation")
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        let start = egui::Button::new(
                            RichText::new("Start Using Archipel IDE").color(egui::Color32::WHITE),
                        )
                        .fill(theme::ACCENT);
                        if ui.add(start).clicked() {
                            let mut params = json!({
                                "theme": draft.theme,
                                "keymap": draft.keymap,
                            });
                            if !draft.import_path.trim().is_empty() {
                                params["importFrom"] = json!(draft.import_path.trim());
                            }
                            action = Some(("app.completeCustomize", params));
                        }
                        let skip = ui.button("Skip");
                        skip_rect = Some(skip.rect);
                        if skip.clicked() {
                            action = Some(("app.skipCustomize", Value::Null));
                        }
                    });
                });
        });
    app.customize = Some(draft);
    app.first_run_skip = skip_rect;
    if let Some((id, params)) = action {
        app.dispatch(id, params);
    }
}

fn section(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme::semibold(theme::UI_FONT_SIZE))
            .color(theme::TEXT),
    );
    ui.add_space(4.0);
}

//! Get from Version Control: URL + directory, then `git clone` with progress.

use crate::native_dialog::Purpose;
use crate::popups::{popup_frame, title};
use crate::{IdeApp, theme};
use eframe::egui::{self, RichText};
use serde_json::json;

pub struct Form {
    url: String,
    pub parent: String,
    directory: String,
    directory_edited: bool,
    error: Option<String>,
}

impl Form {
    pub fn new() -> Self {
        let parent = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".into());
        Self {
            url: String::new(),
            directory: parent.clone(),
            parent,
            directory_edited: false,
            error: None,
        }
    }

    /// Chooses a new parent folder; the directory follows unless typed by hand.
    pub fn set_parent(&mut self, parent: String) {
        self.parent = parent;
        self.directory_edited = false;
        self.follow_url();
    }

    fn follow_url(&mut self) {
        if self.directory_edited {
            return;
        }
        let name = archipel_ide_core::repository_name(&self.url);
        let parent = self.parent.trim_end_matches(['/', '\\']);
        self.directory = if name.is_empty() {
            parent.to_owned()
        } else {
            format!("{parent}/{name}")
        };
    }
}

impl Default for Form {
    fn default() -> Self {
        Self::new()
    }
}

/// Draws the dialog; returns false once it should close.
pub fn show(
    app: &mut IdeApp,
    ctx: &egui::Context,
    form: &mut Form,
    focus: bool,
    browse: &mut Option<Purpose>,
) -> bool {
    let mut keep = true;
    let mut clone = false;
    let busy = app.state.clone_busy();
    if let Some(error) = app.state.clone_error.take() {
        form.error = Some(error);
    }
    let progress = app.state.clone_progress.clone();
    popup_frame(ctx, "dialog_clone", 560.0, |ui| {
        title(ui, "Get from Version Control");
        ui.add_enabled_ui(!busy, |ui| {
            egui::Grid::new("clone_fields")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("URL").color(theme::TEXT_SECONDARY));
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut form.url)
                            .id(egui::Id::new("clone_url"))
                            .hint_text("https://example.com/org/repository.git")
                            .desired_width(400.0),
                    );
                    if focus {
                        response.request_focus();
                    }
                    if response.changed() {
                        form.error = None;
                        form.follow_url();
                    }
                    ui.end_row();
                    ui.label(RichText::new("Directory").color(theme::TEXT_SECONDARY));
                    ui.horizontal(|ui| {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut form.directory)
                                .id(egui::Id::new("clone_directory"))
                                .desired_width(330.0),
                        );
                        if response.changed() {
                            form.directory_edited = true;
                            form.error = None;
                        }
                        if ui.button("Browse…").clicked() {
                            *browse = Some(Purpose::CloneLocation);
                        }
                    });
                    ui.end_row();
                });
        });
        ui.add_space(8.0);
        if busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    RichText::new(progress.as_deref().unwrap_or("Cloning…"))
                        .color(theme::TEXT_SECONDARY),
                );
            });
        }
        if let Some(message) = &form.error {
            ui.label(RichText::new(message).color(theme::ERROR));
        }
        ui.add_space(10.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add_enabled(!busy, egui::Button::new("Cancel")).clicked() {
                keep = false;
            }
            if ui
                .add_enabled(
                    !busy && !form.url.trim().is_empty() && !form.directory.trim().is_empty(),
                    egui::Button::new("Clone"),
                )
                .clicked()
            {
                clone = true;
            }
        });
    });
    if clone {
        form.error = None;
        let params = json!({"url": form.url.trim(), "directory": form.directory.trim()});
        if let Err(error) = app
            .commands
            .execute("project.clone", &mut app.state, params)
        {
            form.error = Some(error.to_string());
        }
    }
    if busy {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    keep
}

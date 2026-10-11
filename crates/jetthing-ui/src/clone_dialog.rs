//! Get from Version Control: URL + directory, then `git clone` with progress.

use crate::IdeApp;
use crate::dialog::{self, Outcome, Select, Shell};
use crate::native_dialog::Purpose;
use archipel_kit::components::{Button, Spinner, TextField};
use eframe::egui;
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
        let name = jetthing_core::repository_name(&self.url);
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
    let busy = app.state.clone_busy();
    if let Some(error) = app.state.clone_error.take() {
        form.error = Some(error);
    }
    let progress = app.state.clone_progress.clone();
    let url_error = form.url_error();
    let directory_error = form.directory_error();
    let valid = !form.url.trim().is_empty() && !form.directory.trim().is_empty();
    let outcome = Shell::new("clone", "Get from Version Control")
        .width(560.0)
        .ok("Clone")
        .ok_enabled(!busy && valid)
        .cancel_enabled(!busy)
        .show(ctx, |ui| {
            let response = ui.add(
                TextField::new(&mut form.url)
                    .label("URL")
                    .placeholder("https://example.com/org/repository.git")
                    .error(url_error.as_deref())
                    .enabled(!busy),
            );
            if focus {
                dialog::focus_first(ctx, &response, Select::All);
            }
            if response.changed() {
                form.error = None;
                form.follow_url();
            }
            let width = ui.available_width();
            ui.horizontal_top(|ui| {
                let response = ui.add(
                    TextField::new(&mut form.directory)
                        .label("Directory")
                        .error(directory_error.as_deref())
                        .enabled(!busy)
                        .width(width - 100.0 - ui.spacing().item_spacing.x),
                );
                if response.changed() {
                    form.directory_edited = true;
                    form.error = None;
                }
                // Level with the field, below its label.
                ui.vertical(|ui| {
                    ui.add_space(ui.text_style_height(&egui::TextStyle::Body) + 4.0);
                    if ui
                        .add(Button::secondary("Browse…").enabled(!busy))
                        .clicked()
                    {
                        *browse = Some(Purpose::CloneLocation);
                    }
                });
            });
            if busy {
                ui.horizontal(|ui| {
                    ui.add(Spinner::new().label("Cloning"));
                    dialog::hint(ui, progress.as_deref().unwrap_or("Cloning…"));
                });
            }
        });
    match outcome {
        Some(Outcome::Cancel) if !busy => keep = false,
        Some(Outcome::Ok) if !busy && valid => {
            form.error = None;
            let params = json!({"url": form.url.trim(), "directory": form.directory.trim()});
            if let Err(error) = app
                .commands
                .execute("project.clone", &mut app.state, params)
            {
                form.error = Some(error.to_string());
            }
        }
        _ => {}
    }
    if busy {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    keep
}

impl Form {
    /// Whether the failure the last clone reported is about the directory.
    fn error_is_about_directory(&self) -> bool {
        self.error.as_ref().is_some_and(|error| {
            let error = error.to_lowercase();
            error.contains("directory")
                || error.contains("destination")
                || error.contains("already exists")
        })
    }

    fn url_error(&self) -> Option<String> {
        let error = self
            .error
            .as_ref()
            .filter(|_| !self.error_is_about_directory())?;
        Some(dialog::friendly("URL", &git_message(error)))
    }

    fn directory_error(&self) -> Option<String> {
        let error = self
            .error
            .as_ref()
            .filter(|_| self.error_is_about_directory())?;
        Some(dialog::friendly("Directory", &git_message(error)))
    }
}

/// Drops git's own prefixes (`fatal:`, `Git command failed:`) from a failure message.
fn git_message(error: &str) -> String {
    let error = error
        .trim()
        .strip_prefix("Git command failed:")
        .unwrap_or(error)
        .trim();
    let error = error.strip_prefix("fatal:").unwrap_or(error).trim();
    error.lines().next().unwrap_or(error).to_owned()
}

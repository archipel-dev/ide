//! Notification balloon (bottom right), used for IDE updates.

use crate::{IdeApp, theme};
use archipel_ide_core::{CURRENT_VERSION, UpdateState};
use eframe::egui::{self, RichText};
use serde_json::Value;

/// Save the current session before the replacement process can read it.
pub fn restart(app: &mut IdeApp, ctx: &egui::Context) -> Result<(), String> {
    app.commands
        .execute("app.prepareRestart", &mut app.state, Value::Null)
        .map_err(|error| error.to_string())?;
    let exe = app
        .state
        .update_restart_target()
        .ok_or("The updated executable path is unavailable; restart Archipel IDE manually.")?;
    let mut command = std::process::Command::new(exe);
    if let Some(project) = &app.state.project {
        // Workspace restore reopens the current tabs; stale startup argv may
        // refer to a project the user has since left.
        command.arg(project.root());
    }
    command.spawn().map_err(|error| error.to_string())?;
    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    Ok(())
}

/// Layer of every notification balloon: above the workbench and tool windows,
/// below dialogs, popups and menus (all `Order::Foreground`), so a balloon
/// never covers a modal.
pub const BALLOON_ORDER: egui::Order = egui::Order::Middle;

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    if !app.state.update_notification {
        return;
    }
    let screen = ctx.content_rect();
    let width = 340.0;
    let mut action: Option<&'static str> = None;
    let mut open_url = None;
    let mut do_restart = false;
    let mut do_install = false;
    egui::Area::new(egui::Id::new("notification_balloon"))
        .order(crate::notifications::BALLOON_ORDER)
        .fixed_pos(egui::pos2(
            screen.right() - width - 16.0,
            screen.bottom() - 150.0 - crate::chrome::STATUS_BAR_HEIGHT,
        ))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(*theme::NOTIFICATION)
                .stroke(egui::Stroke::new(1.0_f32, theme::NOTIFICATION))
                .corner_radius(8)
                .inner_margin(egui::Margin::same(theme::layout::DIALOG_PADDING as i8))
                .show(ui, |ui| {
                    ui.set_width(width - 28.0);
                    let (title, body) = match &app.state.update_state {
                        UpdateState::Available { version, .. } => (
                            format!("Archipel IDE {version} is available"),
                            format!("You are using {CURRENT_VERSION}."),
                        ),
                        UpdateState::Installing(version) => (
                            format!("Updating to {version}…"),
                            "Downloading and verifying the release.".into(),
                        ),
                        UpdateState::Installed(version) => {
                            match app.state.update_installer_kind() {
                                Some(kind) => (
                                    format!("Archipel IDE {version} downloaded"),
                                    kind.installer_hint().into(),
                                ),
                                None => (
                                    format!("Archipel IDE {version} installed"),
                                    "Restart the IDE to finish the update.".into(),
                                ),
                            }
                        }
                        UpdateState::UpToDate(_) => (
                            "You have the latest version".into(),
                            format!("Archipel IDE {CURRENT_VERSION} is up to date."),
                        ),
                        UpdateState::Failed(error) => ("Update check failed".into(), error.clone()),
                        UpdateState::Checking => ("Checking for updates…".into(), String::new()),
                        UpdateState::Idle => (String::new(), String::new()),
                    };
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(title)
                                .font(theme::semibold(theme::UI_FONT_SIZE))
                                .color(theme::TEXT_BRIGHT),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if crate::icons::button(ui, crate::icons::Icon::Close, "Dismiss")
                                .clicked()
                            {
                                action = Some("app.dismissUpdate");
                            }
                        });
                    });
                    if !body.is_empty() {
                        ui.label(RichText::new(body).color(theme::TEXT));
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let link = |ui: &mut egui::Ui, text: &str| {
                            ui.add(
                                egui::Label::new(RichText::new(text).color(theme::MODIFIED))
                                    .sense(egui::Sense::click()),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        };
                        match &app.state.update_state {
                            UpdateState::Available { release, .. } => {
                                let installable = archipel_ide_core::update_installable().is_ok();
                                if installable {
                                    if link(ui, "Update and Restart") {
                                        action = Some("app.installUpdate");
                                    }
                                } else if link(ui, "Download") {
                                    open_url = Some(release.html_url.clone());
                                }
                                ui.add_space(12.0);
                                if link(ui, "Release Notes") {
                                    open_url = Some(release.html_url.clone());
                                }
                            }
                            UpdateState::Installing(_) | UpdateState::Checking => {
                                ui.add(egui::Spinner::new().size(14.0).color(theme::ACCENT));
                            }
                            UpdateState::Installed(_)
                                if app.state.update_installer_kind().is_some() =>
                            {
                                if link(ui, "Quit and Install") {
                                    do_install = true;
                                }
                            }
                            UpdateState::Installed(_) if link(ui, "Restart") => {
                                do_restart = true;
                            }
                            _ => {}
                        }
                    });
                });
        });
    if let Some(url) = open_url {
        ctx.open_url(egui::OpenUrl::new_tab(url));
    }
    if do_restart && let Err(error) = restart(app, ctx) {
        app.state.status = format!("Could not restart Archipel IDE: {error}");
    }
    if do_install {
        // The installer replaces the IDE's files, so quit once it has started.
        match app
            .commands
            .execute("app.runUpdateInstaller", &mut app.state, Value::Null)
        {
            Ok(_) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Err(error) => app.state.status = format!("Could not start the installer: {error}"),
        }
    }
    if let Some(id) = action {
        app.dispatch(id, Value::Null);
        // Keep the balloon for progress after starting an install.
        if id == "app.installUpdate" {
            app.state.update_notification = true;
        }
    }
}

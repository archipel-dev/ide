//! Notification balloons (kit `Toast`s): the IDE update, and completed operations such as
//! a finished commit or an installed toolchain.

use crate::IdeApp;
use archipel_kit::components::{Severity, Spinner, Toast};
use eframe::egui;
use jetthing_core::{CURRENT_VERSION, Notice, NoticeKind, UpdateState};
use serde_json::Value;

/// Save the current session before the replacement process can read it.
pub fn restart(app: &mut IdeApp, ctx: &egui::Context) -> Result<(), String> {
    app.commands
        .execute("app.prepareRestart", &mut app.state, Value::Null)
        .map_err(|error| error.to_string())?;
    let exe = app
        .state
        .update_restart_target()
        .ok_or("The updated executable path is unavailable; restart Jetthing manually.")?;
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

/// How long a success or info balloon stays before it fades out.
const TOAST_SECONDS: f64 = 8.0;
/// Below the main toolbar, so the balloons never cover the bottom-right ones (update,
/// language server, crash notices) that sit above the status bar.
const TOAST_TOP: f32 = 56.0;

/// A balloon on screen: the notice and the time (egui seconds) it appeared.
#[derive(Clone)]
struct Shown {
    id: u64,
    notice: Notice,
    since: f64,
}

fn toasts_id() -> egui::Id {
    egui::Id::new("completed_operation_toasts")
}

fn severity(kind: NoticeKind) -> Severity {
    match kind {
        NoticeKind::Success => Severity::Success,
        NoticeKind::Info => Severity::Info,
        NoticeKind::Error => Severity::Error,
    }
}

/// Balloons for completed operations: new notices from the state, shown top right,
/// each dismissable. Errors stay until dismissed; the rest time out.
fn show_notices(app: &mut IdeApp, ctx: &egui::Context) {
    let now = ctx.input(|input| input.time);
    let mut shown: Vec<Shown> = ctx
        .data(|data| data.get_temp(toasts_id()))
        .unwrap_or_default();
    for notice in app.state.notices.take() {
        let id = shown.last().map_or(0, |s| s.id + 1);
        shown.push(Shown {
            id,
            notice,
            since: now,
        });
    }
    shown.retain(|s| s.notice.kind == NoticeKind::Error || now - s.since < TOAST_SECONDS);
    if shown.is_empty() {
        ctx.data_mut(|data| data.remove::<Vec<Shown>>(toasts_id()));
        return;
    }
    let mut dismissed = Vec::new();
    egui::Area::new(egui::Id::new("completed_operation_balloons"))
        .order(BALLOON_ORDER)
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, TOAST_TOP))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            for item in &shown {
                let out = Toast::new(severity(item.notice.kind), item.notice.title.clone())
                    .body(item.notice.body.clone())
                    .show(ui);
                if out.dismissed {
                    dismissed.push(item.id);
                }
            }
        });
    shown.retain(|s| !dismissed.contains(&s.id));
    if shown.iter().any(|s| s.notice.kind != NoticeKind::Error) {
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
    ctx.data_mut(|data| data.insert_temp(toasts_id(), shown));
}

/// What a link in the update balloon does.
#[derive(Clone, PartialEq)]
enum UpdateAction {
    Command(&'static str),
    OpenUrl(String),
    RunInstaller,
    Restart,
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    show_notices(app, ctx);
    if !app.state.update_notification {
        return;
    }
    let screen = ctx.content_rect();
    let width = 340.0;
    let (title, body) = match &app.state.update_state {
        UpdateState::Available { version, .. } => (
            format!("Jetthing {version} is available"),
            format!("You are using {CURRENT_VERSION}."),
        ),
        UpdateState::Installing(version) => (
            format!("Updating to {version}…"),
            "Downloading and verifying the release.".into(),
        ),
        UpdateState::Installed(version) => match app.state.update_installer_kind() {
            Some(kind) => (
                format!("Jetthing {version} downloaded"),
                kind.installer_hint().into(),
            ),
            None => (
                format!("Jetthing {version} installed"),
                "Restart the IDE to finish the update.".into(),
            ),
        },
        UpdateState::UpToDate(_) => (
            "You have the latest version".into(),
            format!("Jetthing {CURRENT_VERSION} is up to date."),
        ),
        UpdateState::Failed(error) => ("Update check failed".into(), error.clone()),
        UpdateState::Checking => ("Checking for updates…".into(), String::new()),
        UpdateState::Idle => (String::new(), String::new()),
    };
    // Link-style actions of the balloon, by state.
    let mut actions: Vec<(&str, UpdateAction)> = Vec::new();
    match &app.state.update_state {
        UpdateState::Available { release, .. } => {
            if jetthing_core::update_installable().is_ok() {
                actions.push((
                    "Update and Restart",
                    UpdateAction::Command("app.installUpdate"),
                ));
            } else {
                actions.push(("Download", UpdateAction::OpenUrl(release.html_url.clone())));
            }
            actions.push((
                "Release Notes",
                UpdateAction::OpenUrl(release.html_url.clone()),
            ));
        }
        UpdateState::Installed(_) if app.state.update_installer_kind().is_some() => {
            actions.push(("Quit and Install", UpdateAction::RunInstaller));
        }
        UpdateState::Installed(_) => actions.push(("Restart", UpdateAction::Restart)),
        _ => {}
    }
    let busy = matches!(
        app.state.update_state,
        UpdateState::Installing(_) | UpdateState::Checking
    );
    let mut toast = Toast::new(Severity::Info, title).body(body);
    for (label, _) in &actions {
        toast = toast.action(*label);
    }
    let mut chosen = None;
    let mut dismissed = false;
    egui::Area::new(egui::Id::new("notification_balloon"))
        .order(BALLOON_ORDER)
        .fixed_pos(egui::pos2(
            screen.right() - width - 16.0,
            screen.bottom() - 150.0 - crate::chrome::STATUS_BAR_HEIGHT,
        ))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                let out = toast.show(ui);
                dismissed = out.dismissed;
                chosen = out
                    .action
                    .and_then(|index| actions.get(index))
                    .map(|a| a.1.clone());
                if busy {
                    ui.add(Spinner::new());
                }
            });
        });
    if dismissed {
        app.dispatch("app.dismissUpdate", Value::Null);
    }
    match chosen {
        Some(UpdateAction::OpenUrl(url)) => ctx.open_url(egui::OpenUrl::new_tab(url)),
        Some(UpdateAction::Restart) => {
            if let Err(error) = restart(app, ctx) {
                app.state.status = format!("Could not restart Jetthing: {error}");
            }
        }
        Some(UpdateAction::RunInstaller) => {
            // The installer replaces the IDE's files, so quit once it has started.
            match app
                .commands
                .execute("app.runUpdateInstaller", &mut app.state, Value::Null)
            {
                Ok(_) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Err(error) => app.state.status = format!("Could not start the installer: {error}"),
            }
        }
        Some(UpdateAction::Command(id)) => {
            app.dispatch(id, Value::Null);
            // Keep the balloon for progress after starting an install.
            if id == "app.installUpdate" {
                app.state.update_notification = true;
            }
        }
        None => {}
    }
}

/// User-facing text for a failed command: a disabled command is "not available", not
/// an internal id.
pub fn error_message(id: &str, error: &jetthing_command::CommandError) -> String {
    match error {
        jetthing_command::CommandError::Disabled(_) => {
            format!(
                "Not available right now: {}",
                crate::notifications::action_label(id)
            )
        }
        other => other.to_string(),
    }
}

/// A readable name for a command id (`editor.undo` becomes "Undo"): the last segment,
/// split into words.
fn action_label(id: &str) -> String {
    let last = id.rsplit('.').next().unwrap_or(id);
    let mut label = String::new();
    for (index, ch) in last.chars().enumerate() {
        if ch.is_uppercase() && index > 0 {
            label.push(' ');
            label.extend(ch.to_lowercase());
        } else if index == 0 {
            label.extend(ch.to_uppercase());
        } else {
            label.push(ch);
        }
    }
    label
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disabled_command_reads_as_unavailable_without_its_id() {
        let error = jetthing_command::CommandError::Disabled("editor.undo".into());
        let text = error_message("editor.undo", &error);
        assert_eq!(text, "Not available right now: Undo");
        assert!(!text.contains("editor."));
        assert_eq!(action_label("git.pushForce"), "Push force");
    }
}

//! Push dialog (Ctrl+Shift+K): outgoing commits of the current branch, the
//! remote target, and a force-with-lease option.

use crate::{IdeApp, theme};
use archipel_ide_git::PushPlan;
use eframe::egui::{self, RichText, vec2};
use serde_json::json;

pub struct PushDialog {
    plan: Result<PushPlan, String>,
    remote: String,
    target: String,
    force: bool,
}

/// Opens the dialog for the current branch (errors are shown inside it).
pub fn open(app: &mut IdeApp) {
    let plan = app.state.push_plan();
    let (remote, target) = plan
        .as_ref()
        .map(|plan| (plan.remote.clone(), plan.target.clone()))
        .unwrap_or_default();
    app.push_dialog = Some(PushDialog {
        plan,
        remote,
        target,
        force: false,
    });
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(dialog) = app.push_dialog.as_mut() else {
        return;
    };
    let mut push = false;
    let mut close = false;
    let screen = ctx.content_rect();
    let width = 560.0;
    let project = app
        .state
        .project
        .as_ref()
        .and_then(|project| project.root().file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    egui::Area::new(egui::Id::new("push_dialog"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(
            screen.center().x - width / 2.0,
            screen.top() + screen.height() * 0.15,
        ))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .corner_radius(10)
                .inner_margin(egui::Margin::same(theme::layout::DIALOG_PADDING as i8))
                .show(ui, |ui| {
                    ui.set_width(width - 28.0);
                    ui.label(
                        RichText::new(format!("Push Commits to {project}"))
                            .font(theme::semibold(theme::UI_FONT_SIZE))
                            .color(theme::TEXT_BRIGHT),
                    );
                    ui.add_space(8.0);
                    match &dialog.plan {
                        Err(error) => {
                            ui.label(RichText::new(error).color(theme::ERROR));
                            ui.add_space(8.0);
                            if ui.button("Close").clicked() {
                                close = true;
                            }
                        }
                        Ok(plan) => {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&plan.branch).color(theme::TEXT_BRIGHT));
                                ui.label(RichText::new("→").color(theme::TEXT_SECONDARY));
                                egui::ComboBox::from_id_salt("push_remote")
                                    .selected_text(&dialog.remote)
                                    .show_ui(ui, |ui| {
                                        for remote in &plan.remotes {
                                            ui.selectable_value(
                                                &mut dialog.remote,
                                                remote.clone(),
                                                remote,
                                            );
                                        }
                                    });
                                ui.label(RichText::new(":").color(theme::TEXT_SECONDARY));
                                ui.add(
                                    egui::TextEdit::singleline(&mut dialog.target)
                                        .desired_width(180.0),
                                );
                                if plan.new_branch {
                                    ui.label(RichText::new("New").color(theme::SUCCESS));
                                }
                            });
                            ui.add_space(8.0);
                            let count = plan.commits.len();
                            ui.label(
                                RichText::new(crate::widgets::pluralize(
                                    count, "commit", "commits",
                                ))
                                .color(theme::TEXT_SECONDARY),
                            );
                            egui::Frame::new()
                                .fill(*theme::FIELD)
                                .corner_radius(6)
                                .inner_margin(egui::Margin::same(6))
                                .show(ui, |ui| {
                                    ui.set_min_height(120.0);
                                    ui.set_width(ui.available_width());
                                    egui::ScrollArea::vertical()
                                        .max_height(220.0)
                                        .show(ui, |ui| {
                                            if plan.commits.is_empty() {
                                                ui.label(
                                                    RichText::new("Nothing to push")
                                                        .color(theme::TEXT_SECONDARY),
                                                );
                                            }
                                            for commit in &plan.commits {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        RichText::new(&commit.short)
                                                            .monospace()
                                                            .color(theme::TEXT_INFO),
                                                    );
                                                    ui.label(
                                                        RichText::new(&commit.subject)
                                                            .color(theme::TEXT_BRIGHT),
                                                    );
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            ui.label(
                                                                RichText::new(&commit.author)
                                                                    .color(theme::TEXT_SECONDARY),
                                                            );
                                                        },
                                                    );
                                                });
                                            }
                                        });
                                });
                            ui.add_space(8.0);
                            ui.checkbox(&mut dialog.force, "Force push (with lease)");
                            ui.add_space(8.0);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let enabled = !dialog.target.trim().is_empty()
                                        && (!plan.commits.is_empty()
                                            || plan.new_branch
                                            || dialog.force);
                                    let button = egui::Button::new(
                                        RichText::new("Push").color(egui::Color32::WHITE),
                                    )
                                    .fill(if enabled {
                                        theme::ACCENT
                                    } else {
                                        theme::SELECTION_DISABLED
                                    })
                                    .min_size(vec2(72.0, 26.0));
                                    push = ui.add_enabled(enabled, button).clicked();
                                    close = ui.button("Cancel").clicked();
                                },
                            );
                        }
                    }
                });
        });
    if push {
        let params = json!({
            "remote": dialog.remote,
            "target": dialog.target.trim(),
            "force": dialog.force,
        });
        app.push_dialog = None;
        app.dispatch("git.push", params);
    } else if close {
        app.push_dialog = None;
    }
}

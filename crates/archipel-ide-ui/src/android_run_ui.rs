//! Balloon showing the progress and result of running an Android app on a
//! device (Gradle build, `adb install`, `am start`).

use crate::{IdeApp, icons, theme};
use archipel_ide_core::DeployPhase;
use eframe::egui::{self, RichText};
use icons::Icon;

#[derive(Default)]
pub struct DeployBalloon {
    /// Phase the user dismissed; the balloon reappears when the phase changes.
    dismissed: Option<DeployPhase>,
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    let deploy = &app.state.android().deploy;
    if deploy.phase == DeployPhase::Idle
        || app.deploy_balloon.dismissed.as_ref() == Some(&deploy.phase)
    {
        return;
    }
    let (title, color) = match &deploy.phase {
        DeployPhase::Building => ("Building 'app'…".to_owned(), theme::TEXT_BRIGHT),
        DeployPhase::Installing => ("Installing APK…".to_owned(), theme::TEXT_BRIGHT),
        DeployPhase::Launching => ("Launching activity…".to_owned(), theme::TEXT_BRIGHT),
        DeployPhase::Done(message) => (message.clone(), theme::SUCCESS),
        DeployPhase::Failed(message) => (message.clone(), theme::ERROR),
        DeployPhase::Idle => return,
    };
    let screen = ctx.content_rect();
    let width = 460.0;
    let mut dismiss = false;
    egui::Area::new(egui::Id::new("android_deploy_balloon"))
        .order(crate::notifications::BALLOON_ORDER)
        .pivot(egui::Align2::RIGHT_BOTTOM)
        .fixed_pos(egui::pos2(
            screen.right() - 16.0,
            screen.bottom() - crate::chrome::STATUS_BAR_HEIGHT - 12.0,
        ))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(*theme::NOTIFICATION)
                .stroke(egui::Stroke::new(1.0_f32, theme::NOTIFICATION))
                .corner_radius(8)
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    ui.set_width(width - 28.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Run 'app'").font(theme::semibold(theme::UI_FONT_SIZE)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if icons::button(ui, Icon::Close, "Dismiss").clicked() {
                                dismiss = true;
                            }
                        });
                    });
                    ui.add(egui::Label::new(RichText::new(title).color(color)).wrap());
                    let tail: Vec<&String> = deploy.log.iter().rev().take(6).collect();
                    if !tail.is_empty() {
                        ui.add_space(6.0);
                        for line in tail.into_iter().rev() {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(line)
                                        .font(egui::FontId::monospace(theme::UI_FONT_SIZE - 1.0))
                                        .color(theme::TEXT_SECONDARY),
                                )
                                .truncate(),
                            );
                        }
                    }
                });
        });
    if dismiss {
        app.deploy_balloon.dismissed = Some(app.state.android().deploy.phase.clone());
    }
}

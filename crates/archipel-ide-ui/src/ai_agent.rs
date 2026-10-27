//! Agent mode panel inside the AI Chat tool window: goal input, a visible step
//! list (plan text, tool calls with status), approval cards with diffs, Stop.

use crate::icons::{self, Icon};
use crate::{IdeApp, theme};
use archipel_ide_core::{AgentPhase, AgentStepKind, AgentStepStatus};
use eframe::egui::{self, Color32, RichText};
use serde_json::{Value, json};

fn input_id() -> egui::Id {
    egui::Id::new("ai_agent_input")
}

enum Action {
    Start,
    Approve,
    Reject,
    Stop,
    Clear,
}

pub fn panel(app: &mut IdeApp, ui: &mut egui::Ui) {
    let mut action = None;
    let active = app
        .state
        .ai
        .agent
        .as_ref()
        .is_some_and(|a| a.phase.is_active());
    egui::TopBottomPanel::bottom("ai_agent_footer")
        .show_separator_line(false)
        .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(10, 8)))
        .show_inside(ui, |ui| {
            if active {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(12.0));
                    let phase = app.state.ai().agent.as_ref().map(|a| a.phase.clone());
                    let label = match phase {
                        Some(AgentPhase::AwaitingApproval) => "Waiting for your approval",
                        Some(AgentPhase::RunningCommand) => "Running command…",
                        _ => "Working…",
                    };
                    ui.label(RichText::new(label).color(theme::TEXT_SECONDARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(RichText::new("Stop").color(Color32::WHITE)).fill(theme::STOP_RED))
                            .clicked()
                        {
                            action = Some(Action::Stop);
                        }
                    });
                });
                return;
            }
            let focused = ui.memory(|m| m.has_focus(input_id()));
            if focused && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                action = Some(Action::Start);
            }
            egui::Frame::new()
                .fill(*theme::FIELD)
                .stroke(egui::Stroke::new(1.0_f32, if focused { theme::ACCENT } else { theme::BORDER }))
                .corner_radius(8)
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut app.ai_ui.agent_input)
                            .id(input_id())
                            .frame(false)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .hint_text("Describe a task, e.g. “add a unit test for word_counts and run the tests”"),
                    );
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Edits and commands need your approval")
                                .color(theme::TEXT_INFO)
                                .size(theme::UI_FONT_SIZE - 2.0),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let ready = !app.ai_ui.agent_input.trim().is_empty();
                            if icons::button_colored(
                                ui,
                                Icon::Send,
                                if ready { Color32::WHITE } else { *theme::TEXT_DISABLED },
                                "Start task (Enter)",
                                ready,
                            )
                            .clicked()
                                && ready
                            {
                                action = Some(Action::Start);
                            }
                        });
                    });
                });
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::new())
        .show_inside(ui, |ui| match app.state.ai().agent.clone() {
            None => {
                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    icons::show(ui, Icon::Ai, 28.0, theme::TEXT_SECONDARY);
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Agent mode")
                            .color(theme::TEXT_BRIGHT)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "The agent plans the task, reads and searches files on its own, \
                             and asks before every edit and command.",
                        )
                        .color(theme::TEXT_SECONDARY),
                    );
                });
            }
            Some(agent) => {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        egui::Frame::new()
                            .inner_margin(egui::Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                egui::Frame::new()
                                    .fill(*theme::FRAME)
                                    .corner_radius(8)
                                    .inner_margin(egui::Margin::symmetric(10, 8))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.label(
                                            RichText::new("Task")
                                                .color(theme::TEXT_SECONDARY)
                                                .size(theme::UI_FONT_SIZE - 1.0),
                                        );
                                        ui.label(
                                            RichText::new(&agent.goal).color(theme::TEXT_BRIGHT),
                                        );
                                    });
                                ui.add_space(8.0);
                                for step in &agent.steps {
                                    match &step.kind {
                                        AgentStepKind::Message(text) => {
                                            ui.horizontal(|ui| {
                                                icons::show(ui, Icon::Ai, 14.0, theme::MODIFIED);
                                                ui.label(
                                                    RichText::new("Agent")
                                                        .color(theme::TEXT_SECONDARY)
                                                        .size(theme::UI_FONT_SIZE - 1.0),
                                                );
                                            });
                                            crate::ai_chat::markdown_readonly(ui, text);
                                        }
                                        AgentStepKind::Tool {
                                            summary,
                                            diff,
                                            command,
                                        } => {
                                            tool_step(ui, summary, &step.status);
                                            if step.status == AgentStepStatus::AwaitingApproval {
                                                approval_card(
                                                    ui,
                                                    diff,
                                                    command.as_deref(),
                                                    &mut action,
                                                );
                                            }
                                        }
                                    }
                                    ui.add_space(4.0);
                                }
                                match &agent.phase {
                                    AgentPhase::Done => {
                                        ui.label(
                                            RichText::new("✓ Task complete").color(theme::SUCCESS),
                                        );
                                    }
                                    AgentPhase::Stopped => {
                                        ui.label(
                                            RichText::new("Stopped")
                                                .color(theme::TEXT_INFO)
                                                .italics(),
                                        );
                                    }
                                    AgentPhase::Failed(error) => {
                                        ui.horizontal_wrapped(|ui| {
                                            icons::show(ui, Icon::Error, 14.0, theme::ERROR);
                                            ui.label(RichText::new(error).color(theme::ERROR));
                                        });
                                    }
                                    _ => {}
                                }
                                if !agent.phase.is_active() && ui.button("New Task").clicked() {
                                    action = Some(Action::Clear);
                                }
                            });
                    });
            }
        });
    match action {
        Some(Action::Start) => {
            let goal = app.ai_ui.agent_input.trim().to_owned();
            if goal.is_empty() {
                return;
            }
            match app
                .commands
                .execute("ai.agent.start", &mut app.state, json!({"goal": goal}))
            {
                Ok(_) => app.ai_ui.agent_input.clear(),
                Err(error) => app.state.set_status(error.to_string()),
            }
        }
        Some(Action::Approve) => app.dispatch("ai.agent.approve", Value::Null),
        Some(Action::Reject) => app.dispatch("ai.agent.reject", json!({})),
        Some(Action::Stop) => app.dispatch("ai.agent.stop", Value::Null),
        Some(Action::Clear) => app.dispatch("ai.agent.clear", Value::Null),
        None => {}
    }
}

fn tool_step(ui: &mut egui::Ui, summary: &str, status: &AgentStepStatus) {
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        match status {
            AgentStepStatus::Running => {
                ui.add(egui::Spinner::new().size(12.0));
            }
            AgentStepStatus::AwaitingApproval => {
                icons::show(ui, Icon::Warning, 14.0, theme::WARNING);
            }
            AgentStepStatus::Done => {
                ui.label(RichText::new("✓").color(theme::SUCCESS));
            }
            AgentStepStatus::Rejected => {
                ui.label(RichText::new("⊘").color(theme::TEXT_INFO));
            }
            AgentStepStatus::Failed(_) => {
                icons::show(ui, Icon::Error, 14.0, theme::ERROR);
            }
        }
        let color = match status {
            AgentStepStatus::Rejected => theme::TEXT_INFO,
            _ => theme::TEXT,
        };
        ui.add(egui::Label::new(RichText::new(summary).color(color)).truncate());
    });
    let note = match status {
        AgentStepStatus::Failed(reason) if !reason.is_empty() => {
            Some((reason.as_str(), theme::ERROR))
        }
        AgentStepStatus::Rejected => Some(("rejected", theme::TEXT_INFO)),
        _ => None,
    };
    if let Some((note, color)) = note {
        ui.horizontal(|ui| {
            ui.add_space(26.0);
            ui.add(
                egui::Label::new(
                    RichText::new(note)
                        .color(color)
                        .size(theme::UI_FONT_SIZE - 1.0),
                )
                .truncate(),
            );
        });
    }
}

fn approval_card(
    ui: &mut egui::Ui,
    diff: &[archipel_ide_ai::diff::DiffLine],
    command: Option<&str>,
    action: &mut Option<Action>,
) {
    egui::Frame::new()
        .fill(*theme::editor::BACKGROUND_PANEL)
        .stroke(egui::Stroke::new(1.0_f32, theme::WARNING))
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            match command {
                Some(command) => {
                    ui.label(
                        RichText::new("Run this command in the project root?")
                            .color(theme::TEXT_SECONDARY),
                    );
                    ui.label(
                        RichText::new(format!("$ {command}"))
                            .monospace()
                            .color(theme::TEXT_BRIGHT),
                    );
                    if let Some(warning) = archipel_ide_ai::shell::risk(command) {
                        ui.horizontal(|ui| {
                            icons::show(ui, Icon::Warning, 14.0, theme::WARNING);
                            ui.label(
                                RichText::new(format!("This command {warning}."))
                                    .color(theme::WARNING),
                            );
                        });
                    }
                }
                None => {
                    let (added, removed) = archipel_ide_ai::diff::stats(diff);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Apply this change?").color(theme::TEXT_SECONDARY));
                        ui.label(RichText::new(format!("+{added}")).color(theme::ADDED));
                        ui.label(RichText::new(format!("−{removed}")).color(theme::UNTRACKED));
                    });
                    crate::ai_chat::diff_view(ui, diff, 260.0, "ai_agent_diff");
                }
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("Approve").color(Color32::WHITE))
                            .fill(theme::ACCENT),
                    )
                    .clicked()
                {
                    *action = Some(Action::Approve);
                }
                if ui.button("Reject").clicked() {
                    *action = Some(Action::Reject);
                }
            });
        });
}

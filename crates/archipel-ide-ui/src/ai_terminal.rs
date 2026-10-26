//! Terminal "describe a command" bar: natural language → suggested command,
//! which runs only after the user clicks Run (or presses ⌘⏎).

use crate::icons::{self, Icon};
use crate::{IdeApp, theme};
use archipel_ide_core::ShellSuggestionState;
use eframe::egui::{self, Color32, RichText};
use serde_json::{Value, json};

fn input_id() -> egui::Id {
    egui::Id::new("ai_terminal_input")
}

/// Header toggle; returns true when clicked.
pub fn header_button(ui: &mut egui::Ui, available: bool, open: bool) -> bool {
    available
        && icons::button_colored(
            ui,
            Icon::Ai,
            theme::MODIFIED,
            "Describe a command (AI)",
            open,
        )
        .clicked()
}

pub fn toggle(app: &mut IdeApp) {
    app.ai_ui.terminal_open = !app.ai_ui.terminal_open;
    if app.ai_ui.terminal_open {
        app.ai_ui.terminal_focus = true;
    } else {
        app.dispatch("ai.terminal.dismiss", Value::Null);
    }
}

/// Height of the bar: an input row and a status/actions row. It never changes
/// while the bar is open, so showing or running a suggestion does not resize the
/// terminal (a resize would push fresh output into scrollback).
const BAR_HEIGHT: f32 = 64.0;

/// The bar above the terminal grid.
pub fn bar(app: &mut IdeApp, ui: &mut egui::Ui) {
    if !app.ai_ui.terminal_open && app.state.ai.shell.is_none() {
        return;
    }
    let mut suggest = false;
    let mut run = false;
    let mut insert = false;
    let mut cancel = false;
    let suggestion = app.state.ai.shell.clone();
    egui::Frame::new()
        .fill(*theme::FRAME)
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .outer_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.set_height(BAR_HEIGHT - 12.0);
            ui.horizontal(|ui| {
                icons::show(ui, Icon::Ai, 16.0, theme::MODIFIED);
                let focused = ui.memory(|m| m.has_focus(input_id()));
                if focused
                    && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter))
                {
                    run = true;
                } else if focused
                    && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
                {
                    suggest = true;
                }
                let response = ui.add(
                    egui::TextEdit::singleline(&mut app.ai_ui.terminal_input)
                        .id(input_id())
                        .desired_width(f32::INFINITY)
                        .hint_text("Describe a command, e.g. “show the 5 largest files here”"),
                );
                if std::mem::take(&mut app.ai_ui.terminal_focus) {
                    response.request_focus();
                }
            });
            ui.horizontal(|ui| {
                ui.add_space(22.0);
                match suggestion.map(|s| s.state) {
                    None => {
                        ui.label(
                            RichText::new(format!(
                                "Enter to suggest · {} to run · nothing runs without confirmation",
                                crate::keys::display("⌘↩")
                            ))
                            .color(theme::TEXT_INFO),
                        );
                    }
                    Some(ShellSuggestionState::Thinking) => {
                        ui.add(egui::Spinner::new().size(12.0));
                        ui.label(RichText::new("Thinking…").color(theme::TEXT_INFO));
                        cancel = ui.small_button("Cancel").clicked();
                    }
                    Some(ShellSuggestionState::Ready { command, warning }) => {
                        run |= ui
                            .add(
                                egui::Button::new(RichText::new("Run").color(Color32::WHITE))
                                    .fill(theme::ACCENT),
                            )
                            .on_hover_text(format!(
                                "Run in the terminal ({})",
                                crate::keys::display("⌘↩")
                            ))
                            .clicked();
                        insert = ui
                            .button("Insert")
                            .on_hover_text("Type into the terminal without running")
                            .clicked();
                        cancel = ui.button("Cancel").clicked();
                        if let Some(warning) = warning {
                            icons::show(ui, Icon::Warning, 14.0, theme::WARNING)
                                .on_hover_text(format!("This command {warning}."));
                            ui.label(RichText::new(format!("{warning}:")).color(theme::WARNING));
                        }
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("$ {command}"))
                                    .monospace()
                                    .color(theme::TEXT_BRIGHT),
                            )
                            .truncate(),
                        )
                        .on_hover_text(&command);
                    }
                    Some(ShellSuggestionState::Failed(reason)) => {
                        icons::show(ui, Icon::Error, 14.0, theme::ERROR);
                        ui.add(
                            egui::Label::new(RichText::new(reason).color(theme::ERROR)).truncate(),
                        );
                        cancel = ui.small_button("Dismiss").clicked();
                    }
                }
            });
        });
    let ready = matches!(
        app.state.ai.shell.as_ref().map(|s| &s.state),
        Some(ShellSuggestionState::Ready { .. })
    );
    if suggest {
        let text = app.ai_ui.terminal_input.trim().to_owned();
        if !text.is_empty() {
            app.dispatch("ai.terminal.suggest", json!({"text": text}));
        }
    } else if run && ready {
        app.dispatch("ai.terminal.run", Value::Null);
        app.ai_ui.terminal_input.clear();
        ui.memory_mut(|m| m.request_focus(crate::toolwindows::terminal_input_id()));
    } else if insert {
        app.dispatch("ai.terminal.insert", Value::Null);
        app.ai_ui.terminal_input.clear();
        ui.memory_mut(|m| m.request_focus(crate::toolwindows::terminal_input_id()));
    } else if cancel {
        app.dispatch("ai.terminal.dismiss", Value::Null);
    }
}

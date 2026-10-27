//! In-editor Find / Replace bar (⌘F / ⌘R).

use crate::icons::{self, Icon};
use crate::{IdeApp, theme};
use archipel_ide_text::ops::{self, FindOptions};
use eframe::egui::{self, RichText, vec2};
use serde_json::{Value, json};

#[derive(Default)]
pub struct FindBar {
    pub open: bool,
    pub replace: bool,
    pub query: String,
    pub replacement: String,
    pub match_case: bool,
    pub whole_words: bool,
    pub regex: bool,
    focus_requested: bool,
    /// Matches for (path, revision, query, options); searching a large file
    /// every frame would dominate the frame time.
    cache: Option<(MatchKey, Vec<(usize, usize)>)>,
}

#[derive(PartialEq)]
struct MatchKey {
    path: std::path::PathBuf,
    revision: u64,
    query: String,
    options: FindOptions,
}

pub fn query_id() -> egui::Id {
    egui::Id::new("findbar_query")
}

impl FindBar {
    /// Opens the bar, seeding the query from a single-line selection like the reference IDE.
    pub fn show(&mut self, replace: bool, selection: Option<String>) {
        self.open = true;
        self.replace = replace;
        self.focus_requested = true;
        if let Some(text) = selection.filter(|t| !t.is_empty() && !t.contains('\n')) {
            self.query = text;
        }
    }

    pub fn options(&self) -> FindOptions {
        FindOptions {
            match_case: self.match_case,
            whole_words: self.whole_words,
            regex: self.regex,
        }
    }

    /// Why the query cannot run (an invalid regular expression), if it can't.
    pub fn error(&self) -> Option<String> {
        ops::validate_query(&self.query, self.options())
            .err()
            .map(|error| error.to_string())
    }

    /// Matches of the query in `document`, recomputed only when the document,
    /// its revision, the query or the options change.
    pub fn matches(&mut self, document: &archipel_ide_core::Document) -> Vec<(usize, usize)> {
        let key = MatchKey {
            path: document.path.clone(),
            revision: document.text.revision(),
            query: self.query.clone(),
            options: self.options(),
        };
        if let Some((cached, matches)) = &self.cache
            && *cached == key
        {
            return matches.clone();
        }
        let matches = ops::find_all(&document.text.text(), &self.query, self.options());
        self.cache = Some((key, matches.clone()));
        matches
    }

    pub fn params(&self) -> Value {
        json!({
            "query": self.query,
            "replacement": self.replacement,
            "matchCase": self.match_case,
            "wholeWords": self.whole_words,
            "regex": self.regex,
        })
    }
}

fn toggle(ui: &mut egui::Ui, label: &str, on: &mut bool, tooltip: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(24.0, 22.0), egui::Sense::click());
    let fill = if *on {
        *theme::SELECTION
    } else if response.hovered() {
        *theme::HOVER
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        theme::semibold(theme::UI_FONT_SIZE - 1.0),
        if *on {
            *theme::TEXT_BRIGHT
        } else {
            *theme::TEXT_SECONDARY
        },
    );
    let clicked = response.on_hover_text(tooltip).clicked();
    if clicked {
        *on = !*on;
    }
    clicked
}

pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    let Some(index) = app.state.active_index() else {
        return;
    };
    let matches = app.find.matches(&app.state.documents()[index]);
    let primary = app.state.documents()[index].selections.primary();
    let current = matches
        .iter()
        .position(|&(s, e)| s == primary.start() && e == primary.end());
    let mut action: Option<(&'static str, Value)> = None;
    egui::Frame::new()
        .fill(*theme::editor::BACKGROUND_PANEL)
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                icons::show(ui, Icon::Search, 14.0, *theme::TEXT_SECONDARY);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut app.find.query)
                        .id(query_id())
                        .frame(false)
                        .desired_width(260.0)
                        .hint_text("Find"),
                );
                if app.find.focus_requested {
                    response.request_focus();
                    app.find.focus_requested = false;
                }
                let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if enter {
                    let backwards = ui.input(|i| i.modifiers.shift);
                    action = Some((
                        if backwards {
                            "editor.findPrevious"
                        } else {
                            "editor.findNext"
                        },
                        app.find.params(),
                    ));
                    response.request_focus();
                }
                toggle(ui, "Cc", &mut app.find.match_case, "Match Case");
                toggle(ui, "W", &mut app.find.whole_words, "Words");
                toggle(ui, ".*", &mut app.find.regex, "Regex");
                let error = app.find.error();
                let summary = if let Some(error) = &error {
                    error.clone()
                } else if app.find.query.is_empty() {
                    String::new()
                } else if matches.is_empty() {
                    "0 results".into()
                } else if let Some(current) = current {
                    format!("{}/{}", current + 1, matches.len())
                } else {
                    format!("{} results", matches.len())
                };
                let color = if !app.find.query.is_empty() && matches.is_empty() {
                    *theme::ERROR
                } else {
                    *theme::TEXT_SECONDARY
                };
                ui.label(RichText::new(summary).color(color));
                if icons::button(ui, Icon::ChevronUp, "Previous Occurrence (⇧Enter)").clicked() {
                    action = Some(("editor.findPrevious", app.find.params()));
                }
                if icons::button(ui, Icon::ChevronDown, "Next Occurrence (Enter)").clicked() {
                    action = Some(("editor.findNext", app.find.params()));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::button(ui, Icon::Close, "Close (Escape)").clicked() {
                        app.find.open = false;
                    }
                });
            });
            if app.find.replace {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.add_space(18.0);
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut app.find.replacement)
                            .frame(false)
                            .desired_width(260.0)
                            .hint_text("Replace"),
                    );
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        action = Some(("editor.replace", app.find.params()));
                        response.request_focus();
                    }
                    let enabled = !matches.is_empty();
                    if ui
                        .add_enabled(enabled, egui::Button::new("Replace"))
                        .clicked()
                    {
                        action = Some(("editor.replace", app.find.params()));
                    }
                    if ui
                        .add_enabled(enabled, egui::Button::new("Replace All"))
                        .clicked()
                    {
                        action = Some(("editor.replaceAll", app.find.params()));
                    }
                });
            }
        });
    let rect = ui.min_rect();
    ui.painter().hline(
        rect.x_range(),
        ui.cursor().top(),
        egui::Stroke::new(1.0_f32, theme::POPUP_BORDER),
    );
    if let Some((id, params)) = action {
        app.dispatch(id, params);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_query_from_single_line_selection_only() {
        let mut bar = FindBar::default();
        bar.show(false, Some("needle".into()));
        assert_eq!(bar.query, "needle");
        bar.show(true, Some("two\nlines".into()));
        assert_eq!(bar.query, "needle");
        assert!(bar.replace);
        let path = std::path::PathBuf::from("n.txt");
        let mut document = archipel_ide_core::Document::new(path, "a needle, Needle");
        assert_eq!(bar.matches(&document).len(), 2);
        bar.match_case = true;
        assert_eq!(
            bar.matches(&document).len(),
            1,
            "options invalidate the cache"
        );
        document.text.edit(0, 0, "needle ").unwrap();
        assert_eq!(
            bar.matches(&document).len(),
            2,
            "edits invalidate the cache"
        );
    }

    #[test]
    fn regex_toggle_reaches_matches_params_and_errors() {
        let mut bar = FindBar {
            query: r"ne+dle\b".into(),
            regex: true,
            ..Default::default()
        };
        let document = archipel_ide_core::Document::new("n.txt".into(), "needle neeedles neeedle");
        assert_eq!(bar.matches(&document).len(), 2);
        assert_eq!(bar.params()["regex"], true);
        assert_eq!(bar.error(), None);
        bar.query = "(".into();
        assert!(bar.error().unwrap().contains("invalid regular expression"));
        bar.regex = false;
        assert_eq!(bar.error(), None, "plain text is never invalid");
    }
}

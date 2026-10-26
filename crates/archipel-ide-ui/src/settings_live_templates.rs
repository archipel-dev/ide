//! Settings › Editor › Live Templates: template groups with enable toggles
//! and an editor for the selected template's abbreviation, description and
//! text. Edits go through the `template.save` / `template.delete` /
//! `template.setEnabled` commands, which persist user templates in settings.

use crate::icons::{self, Icon};
use crate::theme;
use archipel_ide_templates::{Template, TemplateRegistry, template_key};
use eframe::egui::{self, RichText};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Default)]
pub struct LiveTemplatesPage {
    query: String,
    collapsed: BTreeSet<String>,
    selected: Option<(String, String, bool)>,
    draft: Option<Draft>,
}

/// The template being edited: a copy of the selected one, or a new one.
struct Draft {
    group: String,
    postfix: bool,
    original: Option<(String, String, bool)>,
    template: Template,
}

impl Draft {
    fn new(group: &str) -> Self {
        Self {
            group: group.to_owned(),
            postfix: false,
            original: None,
            template: Template::default(),
        }
    }
}

pub fn show(
    ui: &mut egui::Ui,
    page: &mut LiveTemplatesPage,
    registry: &TemplateRegistry,
    requests: &mut Vec<(String, Value)>,
) {
    let mut request = |id: &str, params: Value| requests.push((id.to_owned(), params));
    ui.horizontal(|ui| {
        crate::widgets::search_field(
            ui,
            egui::Id::new("live_templates_search"),
            &mut page.query,
            "Search templates",
            260.0,
        );
        let group = page
            .selected
            .as_ref()
            .map(|(group, _, _)| group.clone())
            .or_else(|| registry.sets.first().map(|set| set.group.clone()))
            .unwrap_or_else(|| "user".into());
        if ui
            .button("Add")
            .on_hover_text("New live template")
            .clicked()
        {
            page.selected = None;
            page.draft = Some(Draft::new(&group));
        }
        let can_remove = page.selected.is_some();
        if ui
            .add_enabled(can_remove, egui::Button::new("Remove"))
            .on_hover_text("Remove a user template, or switch a bundled one off")
            .clicked()
            && let Some((group, abbreviation, postfix)) = page.selected.take()
        {
            page.draft = None;
            request(
                "template.delete",
                json!({"group": group, "abbreviation": abbreviation, "postfix": postfix}),
            );
        }
    });
    ui.add_space(4.0);
    let query = page.query.trim().to_lowercase();
    let list_height = (ui.available_height() - 250.0).max(120.0);
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0_f32, theme::BORDER))
        .corner_radius(4)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("live_templates_list")
                .max_height(list_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for set in &registry.sets {
                        let rows: Vec<(&Template, bool)> = set
                            .templates
                            .iter()
                            .map(|t| (t, false))
                            .chain(set.postfix.iter().map(|t| (t, true)))
                            .filter(|(t, _)| {
                                query.is_empty()
                                    || t.abbreviation.to_lowercase().contains(&query)
                                    || t.description.to_lowercase().contains(&query)
                            })
                            .collect();
                        if rows.is_empty() && !query.is_empty() {
                            continue;
                        }
                        group_row(ui, page, registry, set, &mut request);
                        if page.collapsed.contains(&set.group) && query.is_empty() {
                            continue;
                        }
                        for (template, postfix) in rows {
                            template_row(
                                ui,
                                page,
                                registry,
                                &set.group,
                                template,
                                postfix,
                                &mut request,
                            );
                        }
                    }
                });
        });
    ui.add_space(8.0);
    editor(ui, page, &mut request);
}

fn group_row(
    ui: &mut egui::Ui,
    page: &mut LiveTemplatesPage,
    registry: &TemplateRegistry,
    set: &archipel_ide_templates::TemplateSet,
    request: &mut impl FnMut(&str, Value),
) {
    ui.horizontal(|ui| {
        let collapsed = page.collapsed.contains(&set.group);
        let arrow = if collapsed {
            Icon::ChevronRight
        } else {
            Icon::ChevronDown
        };
        if icons::button(ui, arrow, if collapsed { "Expand" } else { "Collapse" }).clicked()
            && !page.collapsed.remove(&set.group)
        {
            page.collapsed.insert(set.group.clone());
        }
        let mut enabled = !registry.disabled.contains(&set.group);
        if ui.checkbox(&mut enabled, "").changed() {
            request(
                "template.setEnabled",
                json!({"key": set.group, "enabled": enabled}),
            );
        }
        ui.label(
            RichText::new(&set.group)
                .font(theme::semibold(theme::UI_FONT_SIZE))
                .color(theme::TEXT_BRIGHT),
        );
    });
}

fn template_row(
    ui: &mut egui::Ui,
    page: &mut LiveTemplatesPage,
    registry: &TemplateRegistry,
    group: &str,
    template: &Template,
    postfix: bool,
    request: &mut impl FnMut(&str, Value),
) {
    ui.horizontal(|ui| {
        ui.add_space(26.0);
        let set = registry.sets.iter().find(|set| set.group == group);
        let mut enabled = set.is_none_or(|set| registry.is_enabled(set, template, postfix));
        if ui.checkbox(&mut enabled, "").changed() {
            request(
                "template.setEnabled",
                json!({"key": template_key(group, template, postfix), "enabled": enabled}),
            );
        }
        let key = (group.to_owned(), template.abbreviation.clone(), postfix);
        let selected = page.selected.as_ref() == Some(&key);
        let label = if postfix {
            format!(".{}", template.abbreviation)
        } else {
            template.abbreviation.clone()
        };
        let response = ui.selectable_label(
            selected,
            RichText::new(label).monospace().color(if enabled {
                *theme::TEXT
            } else {
                *theme::TEXT_DISABLED
            }),
        );
        ui.label(RichText::new(&template.description).color(theme::TEXT_INFO));
        if response.clicked() {
            page.draft = Some(Draft {
                group: group.to_owned(),
                postfix,
                original: Some(key.clone()),
                template: template.clone(),
            });
            page.selected = Some(key);
        }
    });
}

fn editor(ui: &mut egui::Ui, page: &mut LiveTemplatesPage, request: &mut impl FnMut(&str, Value)) {
    let Some(draft) = page.draft.as_mut() else {
        ui.label(
            RichText::new("Select a template to edit it, or Add to create one.")
                .color(theme::TEXT_INFO),
        );
        return;
    };
    egui::Grid::new("live_template_editor")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Abbreviation:");
            ui.add(
                egui::TextEdit::singleline(&mut draft.template.abbreviation).desired_width(200.0),
            );
            ui.end_row();
            ui.label("Description:");
            ui.add(
                egui::TextEdit::singleline(&mut draft.template.description).desired_width(420.0),
            );
            ui.end_row();
            ui.label("Group:");
            ui.horizontal(|ui| {
                ui.add_enabled(
                    draft.original.is_none(),
                    egui::TextEdit::singleline(&mut draft.group).desired_width(160.0),
                );
                ui.add_enabled(
                    draft.original.is_none(),
                    egui::Checkbox::new(&mut draft.postfix, "Postfix (expr.key)"),
                );
            });
            ui.end_row();
            ui.label("Template text:");
            ui.add(
                egui::TextEdit::multiline(&mut draft.template.text)
                    .code_editor()
                    .desired_rows(6)
                    .desired_width(420.0),
            );
            ui.end_row();
        });
    ui.label(
        RichText::new(if draft.postfix {
            "Use $EXPR$ for the expression before the dot, $VAR$ for variables and $END$ for the final caret."
        } else {
            "Use $VAR$ for variables, $END$ for the final caret and $SELECTION$ to surround code."
        })
        .color(theme::TEXT_INFO),
    );
    ui.label(
        RichText::new(format!(
            "Applicable in: {}",
            archipel_ide_core::context_labels(&draft.template)
        ))
        .color(theme::TEXT_INFO),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let valid =
            !draft.template.abbreviation.trim().is_empty() && !draft.group.trim().is_empty();
        if ui
            .add_enabled(valid, egui::Button::new("Save template"))
            .clicked()
        {
            request(
                "template.save",
                json!({
                    "group": draft.group,
                    "postfix": draft.postfix,
                    "abbreviation": draft.template.abbreviation,
                    "description": draft.template.description,
                    "text": draft.template.text,
                    "contexts": draft.template.contexts,
                    "variables": draft.template.variables,
                }),
            );
            page.selected = Some((
                draft.group.trim().to_owned(),
                draft.template.abbreviation.trim().to_owned(),
                draft.postfix,
            ));
        }
    });
}

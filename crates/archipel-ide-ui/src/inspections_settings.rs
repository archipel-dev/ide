//! Settings > Editor > Inspections: the profile's inspections grouped by
//! category with an enable checkbox, the highlight level and the options of
//! each tool. Edits go through `inspection.profile.edit`, which writes the
//! profile file and re-analyzes the open files.

use archipel_ide_core::IdeState;
use archipel_ide_core::intentions::Level;
use archipel_ide_core::{InspectionRow, level_id};
use archipel_kit::components::{Checkbox, SectionHeader, SegmentedControl, TextField};
use eframe::egui;
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Default)]
pub struct InspectionsPage {
    query: String,
    selected: Option<String>,
    collapsed: BTreeSet<String>,
}

const LEVELS: [(Level, &str); 4] = [
    (Level::Error, "Error"),
    (Level::Warning, "Warning"),
    (Level::WeakWarning, "Weak warning"),
    (Level::Information, "Information"),
];

fn matches(row: &InspectionRow, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty()
        || row.name.to_lowercase().contains(&query)
        || row.id.to_lowercase().contains(&query)
        || row.group.iter().any(|g| g.to_lowercase().contains(&query))
}

pub fn show(
    ui: &mut egui::Ui,
    page: &mut InspectionsPage,
    state: &IdeState,
    requests: &mut Vec<(String, Value)>,
) {
    let rows = state.inspection_rows();
    ui.horizontal(|ui| {
        ui.add(
            TextField::search(&mut page.query)
                .placeholder("Search inspections")
                .width(260.0),
        );
        let profile = state
            .daemon_service()
            .map(|s| s.profiles().selected.clone())
            .unwrap_or_default();
        ui.label(format!("Profile: {profile}"));
    });
    ui.add_space(6.0);
    let mut groups: Vec<String> = rows.iter().map(|r| r.group.join(" / ")).collect();
    groups.sort();
    groups.dedup();
    egui::ScrollArea::vertical()
        .id_salt("inspections_page")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for group in &groups {
                let members: Vec<&InspectionRow> = rows
                    .iter()
                    .filter(|r| &r.group.join(" / ") == group && matches(r, &page.query))
                    .collect();
                if members.is_empty() {
                    continue;
                }
                let open = !page.collapsed.contains(group);
                let header = ui.add(SectionHeader::new(format!(
                    "{} {group} ({})",
                    if open { "v" } else { ">" },
                    members.len()
                )));
                if header.interact(egui::Sense::click()).clicked() {
                    if open {
                        page.collapsed.insert(group.clone());
                    } else {
                        page.collapsed.remove(group);
                    }
                }
                if open {
                    for row in members {
                        tool_row(ui, page, row, requests);
                    }
                }
            }
        });
}

fn tool_row(
    ui: &mut egui::Ui,
    page: &mut InspectionsPage,
    row: &InspectionRow,
    requests: &mut Vec<(String, Value)>,
) {
    let mut enabled = row.enabled;
    ui.horizontal(|ui| {
        if ui.add(Checkbox::new(&mut enabled, row.name)).changed() {
            requests.push((
                "inspection.profile.edit".into(),
                json!({"tool": row.id, "enabled": enabled}),
            ));
        }
        if ui.small_button("Details").clicked() {
            page.selected = (page.selected.as_deref() != Some(row.id)).then(|| row.id.to_owned());
        }
    });
    if page.selected.as_deref() != Some(row.id) {
        return;
    }
    ui.indent(("inspection_details", row.id), |ui| {
        ui.label(row.description);
        ui.label(format!("Languages: {}", row.languages.join(", ")));
        let mut level = row.level;
        let options: Vec<(Level, &str)> = LEVELS.to_vec();
        if ui.add(SegmentedControl::new(&mut level, options)).changed() {
            requests.push((
                "inspection.profile.edit".into(),
                json!({"tool": row.id, "level": level_id(level)}),
            ));
        }
        for (id, label, value) in &row.options {
            match value {
                Value::Bool(current) => {
                    let mut flag = *current;
                    if ui.add(Checkbox::new(&mut flag, *label)).changed() {
                        requests.push((
                            "inspection.profile.edit".into(),
                            json!({"tool": row.id, "option": id, "value": flag}),
                        ));
                    }
                }
                Value::Number(number) => {
                    let current = number.as_i64().unwrap_or(0);
                    let mut text = current.to_string();
                    ui.horizontal(|ui| {
                        ui.label(*label);
                        let response = ui.add(TextField::new(&mut text).width(80.0));
                        if response.lost_focus()
                            && let Ok(value) = text.trim().parse::<i64>()
                            && value != current
                        {
                            requests.push((
                                "inspection.profile.edit".into(),
                                json!({"tool": row.id, "option": id, "value": value}),
                            ));
                        }
                    });
                }
                Value::String(current) => {
                    let mut text = current.clone();
                    ui.horizontal(|ui| {
                        ui.label(*label);
                        let response = ui.text_edit_singleline(&mut text);
                        if response.lost_focus() && text != *current {
                            requests.push((
                                "inspection.profile.edit".into(),
                                json!({"tool": row.id, "option": id, "value": text}),
                            ));
                        }
                    });
                }
                _ => {}
            }
        }
    });
}

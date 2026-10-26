//! Settings pages with custom bodies that read existing IDE state:
//! Editor › File and Code Templates (the built-in New Java/Kotlin templates) and
//! Version Control (the VCS root mappings of the open project).

use crate::theme;
use archipel_ide_core::IdeState;
use eframe::egui::{self, RichText};
use std::path::Path;

pub const FILE_TEMPLATES_PAGE: &str = "fileTemplates";
pub const VCS_PAGE: &str = "project.propVCSSupport.Mappings";

/// File and Code Templates: the template list and a read-only body view.
/// Editing templates is not supported yet; the page says so.
pub fn file_templates(ui: &mut egui::Ui) {
    let templates = archipel_ide_core::file_templates();
    let id = egui::Id::new("file_template_selected");
    let mut selected: usize = ui.data(|data| data.get_temp(id)).unwrap_or(0);
    selected = selected.min(templates.len().saturating_sub(1));
    ui.label(
        RichText::new(
            "These templates create the files of New | Java Class and New | Kotlin Class/File. \
             They are shown read-only; customizing them is not supported yet.",
        )
        .color(theme::TEXT_SECONDARY),
    );
    ui.add_space(8.0);
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(200.0, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                let mut group = "";
                for (index, template) in templates.iter().enumerate() {
                    if template.group != group {
                        group = template.group;
                        ui.add_space(4.0);
                        ui.label(RichText::new(group).color(theme::TEXT_BRIGHT));
                    }
                    let label = format!("{}.{}", template.name, template.extension);
                    if ui.selectable_label(index == selected, label).clicked() {
                        selected = index;
                    }
                }
            },
        );
        ui.separator();
        if let Some(template) = templates.get(selected) {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(format!("Name: {}", template.name))
                        .font(theme::semibold(theme::UI_FONT_SIZE)),
                );
                ui.label(
                    RichText::new(format!("Extension: {}", template.extension))
                        .color(theme::TEXT_SECONDARY),
                );
                ui.add_space(6.0);
                let mut text = template.text.as_str();
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(10),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(
                        "${PACKAGE_NAME} and ${NAME} are filled in when the file is created.",
                    )
                    .small()
                    .color(theme::TEXT_INFO),
                );
            });
        }
    });
    ui.data_mut(|data| data.insert_temp(id, selected));
}

/// Cache entry: capture time, project root, rows.
type VcsCache = (f64, std::path::PathBuf, Vec<(String, &'static str)>);

/// Directory → VCS rows for the project root and its direct children.
fn vcs_mappings(root: &Path) -> Vec<(String, &'static str)> {
    let mut rows = Vec::new();
    let mut candidates = vec![root.to_path_buf()];
    if let Ok(read) = std::fs::read_dir(root) {
        let mut children: Vec<_> = read
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        children.sort();
        candidates.extend(children);
    }
    for dir in candidates {
        if dir.join(".git").exists() {
            let shown = match dir.strip_prefix(root) {
                Ok(relative) if relative.as_os_str().is_empty() => "<Project>".to_owned(),
                Ok(relative) => relative.display().to_string(),
                Err(_) => dir.display().to_string(),
            };
            rows.push((shown, "Git"));
        }
    }
    rows
}

/// Version Control: the directory/VCS mapping table (read-only).
pub fn version_control(ui: &mut egui::Ui, state: &IdeState) {
    let Some(project) = state.project.as_ref() else {
        ui.label(
            RichText::new("Open a project to see its VCS mappings.").color(theme::TEXT_SECONDARY),
        );
        return;
    };
    let id = egui::Id::new("vcs_mappings_cache");
    let now = ui.input(|input| input.time);
    let cached: Option<VcsCache> = ui.data(|data| data.get_temp(id));
    let rows = match cached {
        Some((at, root, rows)) if root == project.root() && now - at < 2.0 => rows,
        _ => {
            let rows = vcs_mappings(project.root());
            let entry = (now, project.root().to_path_buf(), rows.clone());
            ui.data_mut(|data| data.insert_temp(id, entry));
            rows
        }
    };
    ui.label(RichText::new("Directory mappings").color(theme::TEXT_BRIGHT));
    ui.add_space(4.0);
    egui::Grid::new("vcs_mappings")
        .num_columns(2)
        .striped(true)
        .min_col_width(260.0)
        .show(ui, |ui| {
            ui.label(RichText::new("Directory").color(theme::TEXT_SECONDARY));
            ui.label(RichText::new("VCS").color(theme::TEXT_SECONDARY));
            ui.end_row();
            for (directory, vcs) in &rows {
                ui.label(directory);
                ui.label(*vcs);
                ui.end_row();
            }
        });
    if rows.is_empty() {
        ui.add_space(4.0);
        ui.label(
            RichText::new("No version control roots found in the project.")
                .color(theme::TEXT_SECONDARY),
        );
    }
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Mappings are detected from .git directories; editing them is not supported yet.",
        )
        .small()
        .color(theme::TEXT_INFO),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_git_roots_in_project_and_children() {
        let root = std::env::temp_dir().join(format!("jt-vcs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::create_dir_all(root.join("sub/.git")).unwrap();
        std::fs::create_dir_all(root.join("plain")).unwrap();
        let rows = vcs_mappings(&root);
        assert_eq!(
            rows,
            vec![("<Project>".to_owned(), "Git"), ("sub".to_owned(), "Git")]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn templates_use_placeholders() {
        let templates = archipel_ide_core::file_templates();
        assert!(templates.iter().any(|t| t.text.contains("${PACKAGE_NAME}")));
    }
}

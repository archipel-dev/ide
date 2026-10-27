//! The Spring Boot page of the New Project wizard: project settings on the first page and
//! a start.spring.io style dependency picker on the second.

use super::{Form, field_label};
use crate::native_dialog::Purpose;
use crate::{IdeApp, theme};
use archipel_ide_core::spring_commands::RefreshStatus;
use archipel_ide_project::spring::{SpringBuild, SpringSpec, unsupported_combination};
use archipel_ide_project::spring_catalog::{Catalog, Dependency};
use archipel_kit::action_system::ActionButton;
use archipel_kit::icons::Glyph;
use archipel_kit::openapi::ui::ComboBox;
use archipel_kit::ui::{
    Button, ColoredListCellRenderer, JBCheckBox, JBTextField, SegmentedButton, TitledSeparator,
};
use eframe::egui::{self, RichText, vec2};
use serde_json::{Value, json};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Page {
    Settings,
    Dependencies,
}

/// Wizard state of the Spring Boot generator.
#[derive(Clone)]
pub struct SpringForm {
    pub(super) page: Page,
    pub(super) language: String,
    pub(super) build: String,
    pub(super) group: String,
    pub(super) artifact: String,
    pub(super) package: String,
    artifact_edited: bool,
    package_edited: bool,
    pub(super) java_version: u32,
    pub(super) boot_version: String,
    pub(super) packaging: String,
    pub(super) query: String,
    /// Category filter of the picker; `None` shows every category.
    pub(super) category: Option<String>,
    /// Selected starter ids in the order they were picked.
    pub(super) selected: Vec<String>,
}

impl Default for SpringForm {
    fn default() -> Self {
        Self {
            page: Page::Settings,
            language: "java".into(),
            build: "gradle-groovy".into(),
            group: "com.example".into(),
            artifact: String::new(),
            package: String::new(),
            artifact_edited: false,
            package_edited: false,
            java_version: 21,
            boot_version: String::new(),
            packaging: "jar".into(),
            query: String::new(),
            category: None,
            selected: Vec::new(),
        }
    }
}

impl SpringForm {
    /// Keeps artifact and package derived from the project name and group until edited.
    pub(super) fn sync_names(&mut self, project_name: &str) {
        if !self.artifact_edited {
            self.artifact = project_name.trim().to_owned();
        }
        if !self.package_edited {
            self.package = SpringSpec::derive_package(&self.group, &self.artifact);
        }
    }

    /// Falls back to the newest version when the chosen one is not offered (any more).
    pub(super) fn sync_catalog(&mut self, catalog: &Catalog) {
        if !catalog.boot_versions.contains(&self.boot_version) {
            self.boot_version = catalog.boot_versions.first().cloned().unwrap_or_default();
        }
        if !catalog.java_versions.contains(&self.java_version) {
            self.java_version = catalog.java_versions.first().copied().unwrap_or(21);
        }
    }

    /// Dependencies shown in the picker: matching the search words, in the chosen category.
    pub(super) fn visible<'a>(&self, catalog: &'a Catalog) -> Vec<&'a Dependency> {
        catalog.search(&self.query, self.category.as_deref())
    }

    /// Adds or removes a starter, keeping selection order.
    pub(super) fn set_selected(&mut self, id: &str, on: bool) {
        let present = self.selected.iter().any(|selected| selected == id);
        if on && !present {
            self.selected.push(id.to_owned());
        } else if !on {
            self.selected.retain(|selected| selected != id);
        }
    }

    pub(super) fn is_selected(&self, id: &str) -> bool {
        self.selected.iter().any(|selected| selected == id)
    }

    fn build(&self) -> SpringBuild {
        SpringBuild::parse(&self.build).unwrap_or(SpringBuild::GradleGroovy)
    }

    /// Why the chosen language, build system and packaging cannot be combined.
    pub(super) fn problem(&self) -> Option<&'static str> {
        use archipel_ide_project::generator::Language;
        let language = match self.language.as_str() {
            "kotlin" => Language::Kotlin,
            "groovy" => Language::Groovy,
            _ => Language::Java,
        };
        unsupported_combination(language, self.build(), self.packaging == "war")
    }

    /// Parameters of `project.create` (merged into the shared name, location and Git ones).
    pub(super) fn params(&self) -> Value {
        json!({
            "kind": "spring-boot",
            "language": self.language,
            "build": self.build,
            "group": self.group.trim(),
            "artifact": self.artifact.trim(),
            "package": self.package.trim(),
            "javaVersion": self.java_version,
            "bootVersion": self.boot_version,
            "packaging": self.packaging,
            "dependencies": self.selected,
        })
    }
}

const LANGUAGES: [(&str, &str); 3] = [("java", "Java"), ("kotlin", "Kotlin"), ("groovy", "Groovy")];
const BUILDS: [(&str, &str); 4] = [
    ("gradle-groovy", "Gradle - Groovy DSL"),
    ("gradle-kotlin", "Gradle - Kotlin DSL"),
    ("maven", "Maven"),
    ("bazel", "Bazel"),
];

fn segmented(ui: &mut egui::Ui, value: &mut String, options: &[(&str, &str)]) {
    ui.add(SegmentedButton::new(
        value,
        options.iter().map(|(id, label)| ((*id).to_owned(), *label)),
    ));
}

fn refresh_row(ui: &mut egui::Ui, app: &mut IdeApp) {
    let status = app.state.spring.refresh_status.clone();
    let running = status == Some(RefreshStatus::Running);
    ui.horizontal(|ui| {
        if ui
            .add(
                Button::secondary("Refresh from start.spring.io")
                    .icon(Glyph::Refresh)
                    .small(true)
                    .enabled(!running),
            )
            .clicked()
        {
            app.dispatch("spring.catalog.refresh", json!({}));
        }
        let (text, error) = match &status {
            None => ("Built-in catalog, works offline".to_owned(), false),
            Some(RefreshStatus::Running) => ("Contacting start.spring.io…".to_owned(), false),
            Some(RefreshStatus::Done { added }) => (
                format!(
                    "Updated: {added} new starter{}",
                    if *added == 1 { "" } else { "s" }
                ),
                false,
            ),
            Some(RefreshStatus::Failed(message)) => (
                format!("Could not refresh, using the built-in catalog ({message})"),
                true,
            ),
        };
        ui.add(
            egui::Label::new(RichText::new(text).color(if error {
                theme::ERROR
            } else {
                theme::TEXT_SECONDARY
            }))
            .truncate(),
        );
    });
}

/// The settings page (name, location, language, build system and versions).
pub(super) fn settings_page(
    ui: &mut egui::Ui,
    form: &mut Form,
    app: &mut IdeApp,
    focus: bool,
    browse: &mut Option<Purpose>,
    download: &mut Option<u32>,
) -> bool {
    let mut submit = false;
    let progress = app.state.toolchains().read().progress();
    let catalog_versions = app.state.spring.catalog.boot_versions.clone();
    let java_versions = app.state.spring.catalog.java_versions.clone();
    egui::Grid::new("new_project_spring_fields")
        .num_columns(2)
        .spacing([16.0, 10.0])
        .show(ui, |ui| {
            field_label(ui, "Name");
            let response = ui.add(JBTextField::new(&mut form.name).width(380.0));
            submit |= response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if focus {
                response.request_focus();
            }
            ui.end_row();
            field_label(ui, "Location");
            ui.horizontal(|ui| {
                ui.add(JBTextField::new(&mut form.location).width(300.0));
                if ui.add(Button::secondary("Browse…")).clicked() {
                    *browse = Some(Purpose::NewProjectLocation);
                }
            });
            ui.end_row();
            ui.label("");
            ui.add(
                egui::Label::new(
                    RichText::new(format!(
                        "Project will be created in: {}/{}",
                        form.location.trim_end_matches(['/', '\\']),
                        form.name
                    ))
                    .color(theme::TEXT_INFO),
                )
                .truncate(),
            );
            ui.end_row();
            ui.label("");
            ui.add(JBCheckBox::new(&mut form.git, "Create Git repository"));
            ui.end_row();
            field_label(ui, "Language");
            segmented(ui, &mut form.spring.language, &LANGUAGES);
            ui.end_row();
            field_label(ui, "Build system");
            segmented(ui, &mut form.spring.build, &BUILDS);
            ui.end_row();
            field_label(ui, "Group");
            ui.add(JBTextField::new(&mut form.spring.group).width(380.0));
            ui.end_row();
            field_label(ui, "Artifact");
            if ui
                .add(JBTextField::new(&mut form.spring.artifact).width(380.0))
                .changed()
            {
                form.spring.artifact_edited = form.spring.artifact.trim() != form.name.trim();
            }
            ui.end_row();
            field_label(ui, "Package name");
            if ui
                .add(JBTextField::new(&mut form.spring.package).width(380.0))
                .changed()
            {
                form.spring.package_edited = form.spring.package
                    != SpringSpec::derive_package(&form.spring.group, &form.spring.artifact);
            }
            ui.end_row();
            field_label(ui, "JDK");
            let mut labels: Vec<String> = form
                .jdks
                .iter()
                .map(|choice| choice.label.clone())
                .collect();
            if labels.is_empty() {
                labels.push(format!("{} (not installed)", form.jdk));
            }
            let mut selected = form
                .jdks
                .iter()
                .find(|choice| choice.major == form.jdk)
                .map_or_else(
                    || format!("{} (not installed)", form.jdk),
                    |choice| choice.label.clone(),
                );
            if !labels.contains(&selected) {
                labels.push(selected.clone());
            }
            let downloads = [17_u32, 21, 25].map(|major| format!("Download JDK {major}…"));
            labels.extend(downloads.iter().cloned());
            let before = selected.clone();
            ui.add(
                ComboBox::new("new_project_spring_jdk", &mut selected, labels)
                    .width(300.0)
                    .label("JDK"),
            );
            if selected != before {
                if let Some(index) = downloads.iter().position(|label| *label == selected) {
                    let major = [17_u32, 21, 25][index];
                    *download = Some(major);
                    form.jdk = major;
                } else if let Some(choice) =
                    form.jdks.iter().find(|choice| choice.label == selected)
                {
                    form.jdk = choice.major;
                }
                if java_versions.contains(&form.jdk) {
                    form.spring.java_version = form.jdk;
                }
            }
            ui.end_row();
            if let Some((text, _)) = &progress {
                ui.label("");
                ui.label(RichText::new(text).color(theme::TEXT_SECONDARY));
                ui.end_row();
            }
            field_label(ui, "Java version");
            ui.add(SegmentedButton::new(
                &mut form.spring.java_version,
                java_versions
                    .iter()
                    .map(|version| (*version, version.to_string())),
            ));
            ui.end_row();
            field_label(ui, "Spring Boot");
            let mut boot = form.spring.boot_version.clone();
            ui.add(
                ComboBox::new("new_project_spring_boot", &mut boot, catalog_versions)
                    .width(160.0)
                    .label("Spring Boot version"),
            );
            form.spring.boot_version = boot;
            ui.end_row();
            field_label(ui, "Packaging");
            segmented(
                ui,
                &mut form.spring.packaging,
                &[("jar", "Jar"), ("war", "War")],
            );
            ui.end_row();
        });
    ui.add_space(8.0);
    refresh_row(ui, app);
    if let Some(problem) = form.spring.problem() {
        ui.add_space(6.0);
        ui.label(RichText::new(problem).color(theme::ERROR));
    }
    if form.jdk < form.spring.java_version {
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!(
                "JDK {} cannot compile for Java {}; pick a newer JDK or a lower Java version",
                form.jdk, form.spring.java_version
            ))
            .color(theme::TEXT_SECONDARY),
        );
    }
    form.spring.sync_names(&form.name);
    submit
}

/// The dependency picker: search, categories, checkboxes with descriptions and the
/// list of selected dependencies.
pub(super) fn dependencies_page(ui: &mut egui::Ui, form: &mut Form, app: &mut IdeApp, height: f32) {
    let catalog = app.state.spring.catalog.clone();
    ui.horizontal(|ui| {
        ui.add(
            JBTextField::search(&mut form.spring.query)
                .placeholder("Search dependencies")
                .width(300.0),
        );
        ui.add_space(8.0);
        refresh_row(ui, app);
    });
    ui.add_space(6.0);
    let body = (height - 40.0).max(120.0);
    ui.horizontal_top(|ui| {
        // Categories.
        ui.allocate_ui_with_layout(
            vec2(150.0, body),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(150.0);
                ui.add(TitledSeparator::new("Categories"));
                egui::ScrollArea::vertical()
                    .id_salt("new_project_spring_categories")
                    .max_height(body - 30.0)
                    .show(ui, |ui| {
                        let all = form.spring.category.is_none();
                        if ColoredListCellRenderer::new("All")
                            .selected(all)
                            .show(ui)
                            .response
                            .clicked()
                        {
                            form.spring.category = None;
                        }
                        for category in &catalog.categories {
                            let selected =
                                form.spring.category.as_deref() == Some(category.id.as_str());
                            if ColoredListCellRenderer::new(&category.name)
                                .selected(selected)
                                .show(ui)
                                .response
                                .clicked()
                            {
                                form.spring.category = Some(category.id.clone());
                            }
                        }
                    });
            },
        );
        ui.separator();
        // Starters of the category, with their descriptions.
        ui.allocate_ui_with_layout(
            vec2(350.0, body),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(350.0);
                ui.add(TitledSeparator::new("Dependencies"));
                let visible = form.spring.visible(&catalog);
                egui::ScrollArea::vertical()
                    .id_salt("new_project_spring_dependencies")
                    .max_height(body - 30.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if visible.is_empty() {
                            ui.label(
                                RichText::new("No dependencies match").color(theme::TEXT_SECONDARY),
                            );
                        }
                        let mut category = "";
                        for dependency in visible {
                            if form.spring.category.is_none() && dependency.category != category {
                                category = &dependency.category;
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(catalog.category_name(category))
                                        .strong()
                                        .color(theme::TEXT_SECONDARY),
                                );
                            }
                            let mut on = form.spring.is_selected(&dependency.id);
                            if ui.add(JBCheckBox::new(&mut on, &dependency.name)).changed() {
                                form.spring.set_selected(&dependency.id, on);
                            }
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&dependency.description)
                                            .small()
                                            .color(theme::TEXT_SECONDARY),
                                    )
                                    .wrap(),
                                );
                            });
                        }
                    });
            },
        );
        ui.separator();
        // Selected dependencies.
        ui.allocate_ui_with_layout(
            vec2(200.0, body),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(200.0);
                ui.add(TitledSeparator::new(format!(
                    "Selected dependencies ({})",
                    form.spring.selected.len()
                )));
                egui::ScrollArea::vertical()
                    .id_salt("new_project_spring_selected")
                    .max_height(body - 30.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if form.spring.selected.is_empty() {
                            ui.label(
                                RichText::new("Nothing selected yet").color(theme::TEXT_SECONDARY),
                            );
                        }
                        let mut remove = None;
                        for id in &form.spring.selected {
                            let name = catalog
                                .dependency(id)
                                .map_or(id.as_str(), |dep| dep.name.as_str());
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        ActionButton::new(Glyph::Close, format!("Remove {name}"))
                                            .small(true),
                                    )
                                    .clicked()
                                {
                                    remove = Some(id.clone());
                                }
                                ui.add(egui::Label::new(name).truncate());
                            });
                        }
                        if let Some(id) = remove {
                            form.spring.set_selected(&id, false);
                        }
                    });
            },
        );
    });
}

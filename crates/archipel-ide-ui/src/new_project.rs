//! New Project wizard: generator list on the left, project form on the right.

use crate::native_dialog::Purpose;
use crate::popups::{popup_frame, title};
use crate::{IdeApp, theme, widgets};
use archipel_ide_core::JdkChoice;
use eframe::egui::{self, RichText, vec2};
use serde_json::json;

/// Preferred height of the generator list and form row; the button row sits below it.
const CONTENT_HEIGHT: f32 = 420.0;

/// Room the dialog needs around the content row: title, button row, margins and
/// a gap to the window edges. Taller content would push Create/Cancel off-screen.
const CHROME_HEIGHT: f32 = 150.0;

/// Content row height that still leaves the button row visible in a window of
/// `screen_height` points (the minimum window is 800×500).
fn content_height(screen_height: f32) -> f32 {
    (screen_height - CHROME_HEIGHT).clamp(200.0, CONTENT_HEIGHT)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Preset {
    Jvm {
        language: &'static str,
        build: &'static str,
        dsl: &'static str,
    },
    Empty,
    RustBinary,
    RustLibrary,
    Android(&'static str),
}

struct Generator {
    section: &'static str,
    label: &'static str,
    preset: Preset,
}

const fn jvm(language: &'static str, build: &'static str, dsl: &'static str) -> Preset {
    Preset::Jvm {
        language,
        build,
        dsl,
    }
}

const GENERATORS: &[Generator] = &[
    Generator {
        section: "New Project",
        label: "New Project",
        preset: jvm("java", "gradle", "kotlin"),
    },
    Generator {
        section: "New Project",
        label: "Empty Project",
        preset: Preset::Empty,
    },
    Generator {
        section: "New Project",
        label: "Java",
        preset: jvm("java", "intellij", "kotlin"),
    },
    Generator {
        section: "New Project",
        label: "Kotlin",
        preset: jvm("kotlin", "intellij", "kotlin"),
    },
    Generator {
        section: "Generators",
        label: "Gradle – Java (Kotlin DSL)",
        preset: jvm("java", "gradle", "kotlin"),
    },
    Generator {
        section: "Generators",
        label: "Gradle – Java (Groovy DSL)",
        preset: jvm("java", "gradle", "groovy"),
    },
    Generator {
        section: "Generators",
        label: "Gradle – Kotlin (Kotlin DSL)",
        preset: jvm("kotlin", "gradle", "kotlin"),
    },
    Generator {
        section: "Generators",
        label: "Maven – Java",
        preset: jvm("java", "maven", "kotlin"),
    },
    Generator {
        section: "Generators",
        label: "Android – Empty Activity (Compose)",
        preset: Preset::Android("android-compose"),
    },
    Generator {
        section: "Generators",
        label: "Android – Empty Views Activity",
        preset: Preset::Android("android-views"),
    },
    Generator {
        section: "Generators",
        label: "Android – No Activity",
        preset: Preset::Android("android-no-activity"),
    },
    Generator {
        section: "Generators",
        label: "Rust Application",
        preset: Preset::RustBinary,
    },
    Generator {
        section: "Generators",
        label: "Rust Library",
        preset: Preset::RustLibrary,
    },
];

pub struct Form {
    generator: usize,
    pub name: String,
    pub location: String,
    git: bool,
    language: String,
    build: String,
    dsl: String,
    jdk: u32,
    sample: bool,
    group: String,
    jdks: Vec<JdkChoice>,
    jdk_busy: bool,
    pub error: Option<String>,
}

impl Form {
    pub fn new(jdks: Vec<JdkChoice>) -> Self {
        let mut form = Self {
            generator: 0,
            name: "untitled".into(),
            location: std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".into()),
            git: false,
            language: String::new(),
            build: String::new(),
            dsl: String::new(),
            jdk: 21,
            sample: true,
            group: "org.example".into(),
            jdks,
            jdk_busy: false,
            error: None,
        };
        form.select(0);
        form.pick_default_jdk();
        form
    }

    fn select(&mut self, index: usize) {
        self.generator = index;
        if let Preset::Jvm {
            language,
            build,
            dsl,
        } = GENERATORS[index].preset
        {
            self.language = language.into();
            self.build = build.into();
            self.dsl = dsl.into();
        }
        self.error = None;
    }

    fn pick_default_jdk(&mut self) {
        if let Some(choice) = self.jdks.iter().find(|choice| choice.major == 21) {
            self.jdk = choice.major;
        } else if let Some(choice) = self.jdks.first() {
            self.jdk = choice.major;
        }
    }

    fn preset(&self) -> Preset {
        GENERATORS[self.generator].preset
    }
}

fn segmented(ui: &mut egui::Ui, options: &[(&str, &str)], value: &mut String) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (id, label) in options {
            if ui.selectable_label(value == id, *label).clicked() {
                *value = (*id).into();
            }
        }
    });
}

fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).color(theme::TEXT_SECONDARY));
}

/// Draws the wizard; returns false once it should close.
pub fn show(
    app: &mut IdeApp,
    ctx: &egui::Context,
    form: &mut Form,
    focus: bool,
    browse: &mut Option<Purpose>,
) -> bool {
    let mut keep = true;
    let mut create = false;
    let mut download: Option<u32> = None;
    // Pick up JDKs that finished downloading.
    let busy = app.state.toolchains.service.is_busy();
    if form.jdk_busy && !busy {
        form.jdks = app.state.jdk_choices();
        form.pick_default_jdk();
    }
    form.jdk_busy = busy;
    let progress = app.state.toolchains.progress();
    let fields = [
        "new_project_name",
        "new_project_parent",
        "new_project_group",
    ];
    let enter = ctx.memory(|m| fields.iter().any(|id| m.has_focus(egui::Id::new(id))))
        && !egui::Popup::is_any_open(ctx)
        && ctx.input(|input| input.key_pressed(egui::Key::Enter));
    let content_height = content_height(ctx.content_rect().height());
    popup_frame(ctx, "dialog_new_project", 780.0, |ui| {
        title(ui, "New Project");
        ui.horizontal_top(|ui| {
            // Bound the row: the vertical separator below grows to the available height,
            // which would otherwise push the Create/Cancel row off-screen.
            ui.set_max_height(content_height);
            ui.allocate_ui_with_layout(
                vec2(210.0, content_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(210.0);
                    egui::ScrollArea::vertical()
                        .id_salt("new_project_generators")
                        .show(ui, |ui| {
                            let mut section = "";
                            for (index, generator) in GENERATORS.iter().enumerate() {
                                if generator.section != section {
                                    section = generator.section;
                                    ui.add_space(4.0);
                                    ui.label(RichText::new(section).color(theme::TEXT_SECONDARY));
                                }
                                let selected = form.generator == index;
                                let (rect, response) = widgets::list_row(ui, selected);
                                widgets::row_text(
                                    ui,
                                    rect,
                                    rect.left() + 12.0,
                                    generator.label,
                                    egui::FontId::proportional(theme::UI_FONT_SIZE),
                                    *theme::TEXT,
                                );
                                if response.clicked() {
                                    form.select(index);
                                }
                            }
                        });
                },
            );
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("new_project_form")
                .max_height(content_height)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(520.0);
                        egui::Grid::new("new_project_fields")
                            .num_columns(2)
                            .spacing([16.0, 10.0])
                            .show(ui, |ui| {
                                field_label(ui, "Name");
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut form.name)
                                        .id(egui::Id::new("new_project_name"))
                                        .desired_width(380.0),
                                );
                                if focus {
                                    response.request_focus();
                                }
                                ui.end_row();
                                field_label(ui, "Location");
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut form.location)
                                            .id(egui::Id::new("new_project_parent"))
                                            .desired_width(320.0),
                                    );
                                    if ui.button("Browse…").clicked() {
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
                                ui.checkbox(&mut form.git, "Create Git repository");
                                ui.end_row();
                                if let Preset::Jvm { .. } = form.preset() {
                                    field_label(ui, "Language");
                                    segmented(
                                        ui,
                                        &[("java", "Java"), ("kotlin", "Kotlin")],
                                        &mut form.language,
                                    );
                                    ui.end_row();
                                    field_label(ui, "Build system");
                                    segmented(
                                        ui,
                                        &[
                                            ("intellij", "IntelliJ"),
                                            ("maven", "Maven"),
                                            ("gradle", "Gradle"),
                                        ],
                                        &mut form.build,
                                    );
                                    ui.end_row();
                                    if form.build == "gradle" {
                                        field_label(ui, "Gradle DSL");
                                        segmented(
                                            ui,
                                            &[("kotlin", "Kotlin"), ("groovy", "Groovy")],
                                            &mut form.dsl,
                                        );
                                        ui.end_row();
                                    }
                                    field_label(ui, "JDK");
                                    ui.horizontal(|ui| {
                                        let selected = form
                                            .jdks
                                            .iter()
                                            .find(|choice| choice.major == form.jdk)
                                            .map_or_else(
                                                || format!("{} (not installed)", form.jdk),
                                                |choice| choice.label.clone(),
                                            );
                                        egui::ComboBox::from_id_salt("new_project_jdk")
                                            .selected_text(selected)
                                            .width(300.0)
                                            .show_ui(ui, |ui| {
                                                for choice in &form.jdks {
                                                    ui.selectable_value(
                                                        &mut form.jdk,
                                                        choice.major,
                                                        &choice.label,
                                                    );
                                                }
                                            });
                                        ui.menu_button("Download JDK…", |ui| {
                                            for major in [17_u32, 21, 25] {
                                                if ui.button(format!("JDK {major}")).clicked() {
                                                    download = Some(major);
                                                    form.jdk = major;
                                                    ui.close();
                                                }
                                            }
                                        });
                                    });
                                    ui.end_row();
                                    if let Some((text, _)) = &progress {
                                        ui.label("");
                                        ui.label(RichText::new(text).color(theme::TEXT_SECONDARY));
                                        ui.end_row();
                                    }
                                    ui.label("");
                                    ui.checkbox(&mut form.sample, "Add sample code");
                                    ui.end_row();
                                }
                            });
                        if matches!(form.preset(), Preset::Jvm { .. }) {
                            ui.add_space(8.0);
                            egui::CollapsingHeader::new("Advanced Settings")
                                .id_salt("new_project_advanced")
                                .show(ui, |ui| {
                                    egui::Grid::new("new_project_advanced_fields")
                                        .spacing([16.0, 8.0])
                                        .show(ui, |ui| {
                                            field_label(ui, "GroupId");
                                            ui.add_sized(
                                                vec2(300.0, ui.spacing().interact_size.y),
                                                egui::TextEdit::singleline(&mut form.group)
                                                    .id(egui::Id::new("new_project_group")),
                                            );
                                            ui.end_row();
                                            field_label(ui, "ArtifactId");
                                            ui.label(
                                                RichText::new(&form.name).color(theme::TEXT_INFO),
                                            );
                                            ui.end_row();
                                        });
                                });
                        }
                        if let Some(message) = &form.error {
                            ui.add_space(6.0);
                            ui.label(RichText::new(message).color(theme::ERROR));
                        }
                    })
                });
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    keep = false;
                }
                if ui
                    .add_enabled(
                        !form.name.trim().is_empty() && !form.location.trim().is_empty(),
                        egui::Button::new("Create"),
                    )
                    .clicked()
                {
                    create = true;
                }
            });
        });
    });
    if let Some(major) = download {
        app.dispatch(
            "toolchain.install",
            json!({"component": "jdk", "version": major.to_string()}),
        );
    }
    if (create || enter) && keep && !form.name.trim().is_empty() && !form.location.trim().is_empty()
    {
        let mut params = json!({
            "parent": form.location.trim(),
            "name": form.name.trim(),
            "group": form.group.trim(),
            "git": form.git,
        });
        match form.preset() {
            Preset::Empty => params["kind"] = "empty".into(),
            Preset::RustBinary => params["kind"] = "rust-binary".into(),
            Preset::RustLibrary => params["kind"] = "rust-library".into(),
            Preset::Android(kind) => params["kind"] = kind.into(),
            Preset::Jvm { .. } => {
                params["kind"] = "jvm".into();
                params["language"] = form.language.clone().into();
                params["build"] = form.build.clone().into();
                params["dsl"] = form.dsl.clone().into();
                params["jdk"] = form.jdk.into();
                params["sample"] = form.sample.into();
            }
        }
        match app
            .commands
            .execute("project.create", &mut app.state, params)
        {
            Ok(_) => {
                app.project_opened(ctx);
                keep = false;
            }
            Err(error) => form.error = Some(error.to_string()),
        }
    }
    keep
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_shrinks_so_the_button_row_fits_small_windows() {
        assert_eq!(content_height(900.0), CONTENT_HEIGHT);
        // The minimum window height must still show Create/Cancel.
        assert!(content_height(500.0) + CHROME_HEIGHT <= 500.0);
        assert!(content_height(100.0) >= 200.0);
    }
}

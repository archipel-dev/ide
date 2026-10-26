//! New Project wizard: generator list on the left, project form on the right.

mod spring;

use crate::native_dialog::Purpose;
use crate::popups::{popup_frame, title};
use crate::{IdeApp, theme, widgets};
use archipel_ide_core::JdkChoice;
use archipel_kit::components::Button;
use eframe::egui::{self, RichText, vec2};
use serde_json::json;
use spring::{Page, SpringForm};

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
    /// The Spring Boot (Initializr style) generator with its dependency picker.
    Spring,
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
        label: "Spring Boot",
        preset: Preset::Spring,
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
    spring: SpringForm,
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
            spring: SpringForm::default(),
            error: None,
        };
        form.select(0);
        form.pick_default_jdk();
        form
    }

    /// Preselects a generator, build tool, search and page; used by automation scenes.
    pub(crate) fn preset_for_automation(&mut self, args: &serde_json::Value) {
        let text = |key: &str| args.get(key).and_then(|v| v.as_str());
        if let Some(label) = text("generator")
            && let Some(index) = GENERATORS.iter().position(|g| g.label == label)
        {
            self.select(index);
        }
        if let Some(build) = text("build") {
            self.spring.build = build.into();
        }
        if let Some(name) = text("name") {
            self.name = name.into();
        }
        if let Some(query) = text("query") {
            self.spring.query = query.into();
        }
        for id in args
            .get("select")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
        {
            if let Some(id) = id.as_str() {
                self.spring.set_selected(id, true);
            }
        }
        if text("page") == Some("dependencies") {
            self.spring.page = Page::Dependencies;
        }
    }

    fn select(&mut self, index: usize) {
        self.generator = index;
        self.spring.page = Page::Settings;
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
    app.state.poll_spring_catalog();
    form.spring.sync_catalog(&app.state.spring.catalog);
    let is_spring = matches!(form.preset(), Preset::Spring);
    let deps_page = is_spring && form.spring.page == Page::Dependencies;
    let mut go_next = false;
    let mut go_back = false;
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
            if !deps_page {
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
                                        ui.label(
                                            RichText::new(section).color(theme::TEXT_SECONDARY),
                                        );
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
            }
            if deps_page {
                ui.vertical(|ui| {
                    ui.set_width(700.0);
                    spring::dependencies_page(ui, form, app, content_height);
                });
            } else if is_spring {
                egui::ScrollArea::vertical()
                    .id_salt("new_project_form")
                    .max_height(content_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.set_width(520.0);
                            go_next =
                                spring::settings_page(ui, form, app, focus, browse, &mut download);
                            if let Some(message) = &form.error {
                                ui.add_space(6.0);
                                ui.label(RichText::new(message).color(theme::ERROR));
                            }
                        })
                    });
            } else {
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
                                            &[
                                                ("java", "Java"),
                                                ("kotlin", "Kotlin"),
                                                ("groovy", "Groovy"),
                                            ],
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
                                                ("bazel", "Bazel"),
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
                                            ui.label(
                                                RichText::new(text).color(theme::TEXT_SECONDARY),
                                            );
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
                                                    RichText::new(&form.name)
                                                        .color(theme::TEXT_INFO),
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
            }
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(Button::secondary("Cancel")).clicked() {
                    keep = false;
                }
                let filled = !form.name.trim().is_empty() && !form.location.trim().is_empty();
                if is_spring && !deps_page {
                    if ui
                        .add(
                            Button::primary("Next")
                                .enabled(filled && form.spring.problem().is_none()),
                        )
                        .clicked()
                    {
                        go_next = true;
                    }
                } else {
                    if ui.add(Button::primary("Create").enabled(filled)).clicked() {
                        create = true;
                    }
                    if deps_page && ui.add(Button::secondary("Previous")).clicked() {
                        go_back = true;
                    }
                }
            });
        });
    });
    if go_next
        && form.spring.problem().is_none()
        && !form.name.trim().is_empty()
        && !form.location.trim().is_empty()
    {
        form.spring.page = Page::Dependencies;
        form.error = None;
    }
    if go_back {
        form.spring.page = Page::Settings;
    }
    if let Some(major) = download {
        app.dispatch(
            "toolchain.install",
            json!({"component": "jdk", "version": major.to_string()}),
        );
    }
    let enter = enter && !is_spring;
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
            Preset::Spring => {
                if let (Some(target), Some(extra)) =
                    (params.as_object_mut(), form.spring.params().as_object())
                {
                    target.extend(extra.clone());
                }
                params["group"] = form.spring.group.trim().into();
            }
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

    #[test]
    fn spring_picker_search_add_remove_and_build_switch() {
        let catalog = archipel_ide_project::spring_catalog::Catalog::builtin();
        let mut form = SpringForm::default();
        form.query = "postgres".into();
        let ids: Vec<_> = form
            .visible(&catalog)
            .iter()
            .map(|d| d.id.clone())
            .collect();
        assert!(ids.contains(&"postgresql".to_owned()));
        form.set_selected("postgresql", true);
        form.set_selected("web", true);
        form.set_selected("web", true);
        assert_eq!(form.selected, ["postgresql", "web"]);
        form.set_selected("postgresql", false);
        assert_eq!(form.selected, ["web"]);
        form.category = Some("nosql".into());
        form.query.clear();
        assert!(form.visible(&catalog).iter().all(|d| d.category == "nosql"));
        form.language = "kotlin".into();
        form.build = "bazel".into();
        assert!(form.problem().is_some(), "Bazel is Java only");
        form.build = "maven".into();
        assert!(form.problem().is_none());
        let params = form.params();
        assert_eq!(params["build"], "maven");
        assert_eq!(params["dependencies"], json!(["web"]));
    }

    #[test]
    fn spring_wizard_pages_click_through_to_create() {
        use archipel_kit::testing::{self, Queryable};
        let parent = std::env::temp_dir().join(format!("archipel-wizard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&parent);
        let mut app = IdeApp::new(None, Vec::new());
        let mut form = Form::new(Vec::new());
        form.select(
            GENERATORS
                .iter()
                .position(|g| matches!(g.preset, Preset::Spring))
                .unwrap(),
        );
        form.location = parent.display().to_string();
        form.name = "demo".into();
        app.state.status.clear();
        let mut harness = testing::harness_state(
            archipel_kit::Theme::dark(),
            (app, form),
            |ui, (app, form)| {
                let mut browse = None;
                show(app, ui.ctx(), form, false, &mut browse);
            },
        );
        harness.run();
        harness.get_by_label("Maven").click();
        harness.run();
        harness.get_by_label("Next").click();
        harness.run();
        harness.get_by_label("Spring Web").click();
        harness.run();
        assert_eq!(harness.state().1.spring.selected, ["web"]);
        harness.get_by_label("Remove Spring Web").click();
        harness.run();
        assert!(harness.state().1.spring.selected.is_empty());
        harness.get_by_label("Spring Web").click();
        harness.run();
        harness.get_by_label("Create").click();
        harness.run();
        assert!(parent.join("demo/pom.xml").is_file());
        let _ = std::fs::remove_dir_all(&parent);
    }
}

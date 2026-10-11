//! New Project wizard: generator list on the left, project form on the right.

use crate::IdeApp;
use crate::dialog::{self, Outcome, Select, Shell};
use crate::native_dialog::Purpose;
use archipel_kit::Theme;
use archipel_kit::components::{
    Button, Checkbox, Dropdown, ListRow, MenuItem, PropertyGrid, SectionHeader, SegmentedControl,
    TextField, VirtualList,
};
use archipel_kit::parts::Popover;
use eframe::egui::{self, vec2};
use jetthing_core::JdkChoice;
use serde_json::json;

/// Preferred height of the generator list and form row; the button row sits below it.
const CONTENT_HEIGHT: f32 = 640.0;

/// Room the dialog needs around the content row: window margins, title bar, button row
/// and frame padding. Taller content would push Create/Cancel off-screen.
const CHROME_HEIGHT: f32 = 176.0;

/// Width of the generator list.
const LIST_WIDTH: f32 = 250.0;

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

/// Which field a failed `project.create` is about.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Location,
}

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
    jdk_menu: bool,
    error: Option<(Field, String)>,
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
            jdk_menu: false,
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

    /// The selected generator's label (for tests and the audit).
    #[cfg(test)]
    pub(crate) fn generator_label(&self) -> &'static str {
        GENERATORS[self.generator].label
    }

    /// The message under the Name field.
    fn name_error(&self) -> Option<String> {
        let name = self.name.trim();
        if name.is_empty() {
            Some("Name must not be empty.".into())
        } else if name.contains(['/', '\\']) {
            Some("Name must be a single folder name, without “/” or “\\”.".into())
        } else {
            self.error
                .as_ref()
                .filter(|(field, _)| *field == Field::Name)
                .map(|(_, message)| message.clone())
        }
    }

    /// The message under the Location field.
    fn location_error(&self) -> Option<String> {
        if self.location.trim().is_empty() {
            Some("Location must not be empty.".into())
        } else {
            self.error
                .as_ref()
                .filter(|(field, _)| *field == Field::Location)
                .map(|(_, message)| message.clone())
        }
    }

    fn valid(&self) -> bool {
        !self.name.trim().is_empty()
            && !self.name.contains(['/', '\\'])
            && !self.location.trim().is_empty()
    }
}

/// A row of the generator list: a section title or a generator.
#[derive(Clone, Copy)]
enum Entry {
    Section(&'static str),
    Generator(usize),
}

fn entries() -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut section = "";
    for (index, generator) in GENERATORS.iter().enumerate() {
        if generator.section != section {
            section = generator.section;
            entries.push(Entry::Section(section));
        }
        entries.push(Entry::Generator(index));
    }
    entries
}

/// What a generator creates, for the pages that have no options of their own (UX-024).
fn description(preset: Preset) -> (&'static str, Vec<(&'static str, &'static str)>) {
    match preset {
        Preset::Empty => (
            "An empty project: a folder with a README. Add build files and sources yourself.",
            vec![("README.md", "A heading with the project name")],
        ),
        Preset::RustBinary => (
            "A Cargo binary crate with a Hello, world! entry point.",
            vec![
                ("Cargo.toml", "Package manifest, edition 2024"),
                ("src/main.rs", "The program entry point"),
                ("README.md", "A heading with the project name"),
            ],
        ),
        Preset::RustLibrary => (
            "A Cargo library crate with a function and a unit test.",
            vec![
                ("Cargo.toml", "Package manifest, edition 2024"),
                ("src/lib.rs", "An add function and its test"),
                ("README.md", "A heading with the project name"),
            ],
        ),
        Preset::Android(kind) => (
            match kind {
                "android-compose" => "An Android application with one Jetpack Compose activity.",
                "android-views" => "An Android application with one activity using view layouts.",
                _ => "An Android application module without an activity.",
            },
            vec![
                ("settings.gradle.kts", "Gradle project with the app module"),
                ("app/build.gradle.kts", "Android application module"),
                (
                    "app/src/main/AndroidManifest.xml",
                    "The application manifest",
                ),
            ],
        ),
        Preset::Jvm { .. } => ("", Vec::new()),
    }
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
    let mut download: Option<u32> = None;
    // Pick up JDKs that finished downloading.
    let busy = app.state.toolchains.service.is_busy();
    if form.jdk_busy && !busy {
        form.jdks = app.state.jdk_choices();
        form.pick_default_jdk();
    }
    form.jdk_busy = busy;
    let progress = app.state.toolchains.progress();
    let content_height = content_height(ctx.content_rect().height());
    let entries = entries();
    let position = |generator: usize| {
        entries
            .iter()
            .position(|entry| matches!(entry, Entry::Generator(index) if *index == generator))
    };
    let mut selected = position(form.generator);
    let mut jdk_menu = form.jdk_menu;
    let outcome = Shell::new("new_project", "New Project")
        .width(780.0)
        .ok("Create")
        .ok_enabled(form.valid())
        .keys_held(form.jdk_menu)
        .show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                let total = ui.available_width();
                ui.spacing_mut().item_spacing.x = 16.0;
                ui.allocate_ui_with_layout(
                    vec2(LIST_WIDTH, content_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(LIST_WIDTH);
                        VirtualList::new("new_project_generators", entries.len(), &mut selected)
                            .name("Project generators")
                            .height(content_height)
                            .show(ui, |index| match entries[index] {
                                Entry::Section(title) => ListRow::new(title).enabled(false),
                                Entry::Generator(generator) => {
                                    ListRow::new(GENERATORS[generator].label)
                                }
                            });
                    },
                );
                let form_width = (total - LIST_WIDTH - 16.0).max(200.0);
                egui::ScrollArea::vertical()
                    .id_salt("new_project_form")
                    .max_height(content_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        // The scroll area inherits the row's left-to-right layout.
                        ui.vertical(|ui| {
                            ui.set_width(form_width - 12.0);
                            form_body(
                                ui,
                                ctx,
                                form,
                                focus,
                                browse,
                                &mut download,
                                &mut jdk_menu,
                                progress.as_ref().map(|(text, _)| text.as_str()),
                            );
                        });
                    });
            });
        });
    form.jdk_menu = jdk_menu;
    if let Some(to) = selected
        && let Some(from) = position(form.generator)
        && to != from
    {
        let to = dialog::skip_unselectable(entries.len(), from, to, |index| {
            matches!(entries[index], Entry::Generator(_))
        });
        if let Entry::Generator(index) = entries[to] {
            form.select(index);
        }
    }
    if let Some(major) = download {
        app.dispatch(
            "toolchain.install",
            json!({"component": "jdk", "version": major.to_string()}),
        );
    }
    match outcome {
        Some(Outcome::Cancel) => keep = false,
        Some(Outcome::Ok) if form.valid() => create(app, ctx, form, &mut keep),
        _ => {}
    }
    keep
}

#[allow(clippy::too_many_arguments)]
fn form_body(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    form: &mut Form,
    focus: bool,
    browse: &mut Option<Purpose>,
    download: &mut Option<u32>,
    jdk_menu: &mut bool,
    progress: Option<&str>,
) {
    let width = ui.available_width();
    let name_error = form.name_error();
    let response = ui.add(
        TextField::new(&mut form.name)
            .label("Name")
            .error(name_error.as_deref()),
    );
    if focus {
        dialog::focus_first(ctx, &response, Select::All);
    }
    if response.changed() {
        form.error = None;
    }
    let location_error = form.location_error();
    ui.horizontal_top(|ui| {
        let browse_width = 100.0;
        let response = ui.add(
            TextField::new(&mut form.location)
                .label("Location")
                .error(location_error.as_deref())
                .width(width - browse_width - ui.spacing().item_spacing.x),
        );
        if response.changed() {
            form.error = None;
        }
        // Level with the field, below its label.
        ui.vertical(|ui| {
            ui.add_space(ui.text_style_height(&egui::TextStyle::Body) + 4.0);
            if ui
                .add(Button::secondary("Browse…").min_width(100.0))
                .clicked()
            {
                *browse = Some(Purpose::NewProjectLocation);
            }
        });
    });
    dialog::hint(
        ui,
        format!(
            "Project will be created in: {}/{}",
            form.location.trim_end_matches(['/', '\\']),
            form.name
        ),
    );
    ui.add(Checkbox::new(&mut form.git, "Create Git repository"));
    match form.preset() {
        Preset::Jvm { .. } => jvm_options(ui, ctx, form, download, jdk_menu, progress),
        preset => {
            ui.add_space(8.0);
            ui.add(SectionHeader::new("What you get"));
            let (text, files) = description(preset);
            dialog::message(ui, text);
            let mut grid = PropertyGrid::new("new_project_files");
            for (file, what) in files {
                grid = grid.mono_row(file, what);
            }
            grid.show(ui);
        }
    }
}

fn jvm_options(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    form: &mut Form,
    download: &mut Option<u32>,
    jdk_menu: &mut bool,
    progress: Option<&str>,
) {
    fn labelled(ui: &mut egui::Ui, label: &str) {
        let colors = Theme::colors(ui.ctx());
        ui.label(egui::RichText::new(label).color(colors.text));
    }
    labelled(ui, "Language");
    let language = |id: &str, label: &str| (id.to_owned(), label.to_owned());
    ui.add(SegmentedControl::new(
        &mut form.language,
        [language("java", "Java"), language("kotlin", "Kotlin")],
    ));
    labelled(ui, "Build system");
    ui.add(SegmentedControl::new(
        &mut form.build,
        [
            language("intellij", "IntelliJ"),
            language("maven", "Maven"),
            language("gradle", "Gradle"),
        ],
    ));
    if form.build == "gradle" {
        labelled(ui, "Gradle DSL");
        ui.add(SegmentedControl::new(
            &mut form.dsl,
            [language("kotlin", "Kotlin"), language("groovy", "Groovy")],
        ));
    }
    labelled(ui, "JDK");
    ui.horizontal(|ui| {
        let mut labels: Vec<String> = form
            .jdks
            .iter()
            .map(|choice| choice.label.clone())
            .collect();
        let current = form
            .jdks
            .iter()
            .find(|choice| choice.major == form.jdk)
            .map_or_else(
                || {
                    let missing = format!("{} (not installed)", form.jdk);
                    labels.push(missing.clone());
                    missing
                },
                |choice| choice.label.clone(),
            );
        let mut label = current.clone();
        ui.add(
            Dropdown::new("new_project_jdk", &mut label, labels)
                .label("JDK")
                .width(280.0),
        );
        if label != current
            && let Some(choice) = form.jdks.iter().find(|choice| choice.label == label)
        {
            form.jdk = choice.major;
        }
        let button = ui.add(Button::secondary("Download JDK…"));
        if button.clicked() {
            *jdk_menu = !*jdk_menu;
        }
        let mut menu_open = *jdk_menu;
        let mut picked = false;
        Popover::below("new_project_download_jdk", &button)
            .width(160.0)
            .show(ctx, &mut menu_open, |ui| {
                for major in [17_u32, 21, 25] {
                    if ui.add(MenuItem::new(format!("JDK {major}"))).clicked() {
                        *download = Some(major);
                        form.jdk = major;
                        picked = true;
                    }
                }
            });
        *jdk_menu = menu_open && !picked;
    });
    if let Some(text) = progress {
        dialog::hint(ui, text);
    }
    ui.add(Checkbox::new(&mut form.sample, "Add sample code"));
    ui.add_space(8.0);
    ui.add(SectionHeader::new("Advanced settings"));
    ui.add(TextField::new(&mut form.group).label("GroupId"));
    PropertyGrid::new("new_project_artifact")
        .row("ArtifactId", form.name.clone())
        .show(ui);
}

/// Runs `project.create`; failures stay in the dialog under the field they are about.
fn create(app: &mut IdeApp, ctx: &egui::Context, form: &mut Form, keep: &mut bool) {
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
            *keep = false;
        }
        Err(error) => {
            let message = error.to_string();
            let location = message.contains("location")
                || message.contains("there is no directory")
                || message.contains("not a directory");
            let field = if location {
                (Field::Location, "Location")
            } else {
                (Field::Name, "Name")
            };
            form.error = Some((field.0, dialog::friendly(field.1, &message)));
        }
    }
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
    fn selection_skips_section_titles_in_the_direction_of_travel() {
        let entries = entries();
        // Entry 0 is the "New Project" title: arriving from below it goes to the next row.
        let first = entries
            .iter()
            .position(|e| matches!(e, Entry::Generator(_)))
            .unwrap();
        assert_eq!(
            dialog::skip_unselectable(entries.len(), first, 0, |i| matches!(
                entries[i],
                Entry::Generator(_)
            )),
            first
        );
        // The second title ("Generators") is skipped downwards and upwards.
        let title = entries
            .iter()
            .rposition(|e| matches!(e, Entry::Section(_)))
            .unwrap();
        assert_eq!(
            dialog::skip_unselectable(entries.len(), title - 1, title, |i| matches!(
                entries[i],
                Entry::Generator(_)
            )),
            title + 1
        );
        assert_eq!(
            dialog::skip_unselectable(entries.len(), title + 1, title, |i| matches!(
                entries[i],
                Entry::Generator(_)
            )),
            title - 1
        );
    }
}

//! Run/Debug Configurations dialog: configuration list on the left, form on the right.

use crate::icons::{self, Icon};
use crate::widgets::{list_row, row_text};
use crate::{IdeApp, theme};
use archipel_ide_runner::{RunConfiguration, RunTarget};
use eframe::egui::{self, RichText, vec2};
use serde_json::{Map, Value, json};

/// Editable copy of one configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    /// Name stored on disk (None for a configuration not saved yet).
    pub previous: Option<String>,
    pub name: String,
    pub type_id: &'static str,
    /// Form values keyed by the TOML field name.
    pub fields: Vec<(&'static str, String)>,
    pub working_directory: String,
    /// `KEY=value; KEY2=value2`, as in the reference dialog.
    pub env: String,
    pub temporary: bool,
    pub dirty: bool,
    /// Show the Run tool window when the configuration starts.
    pub open_tool_window: bool,
    /// Optional rows enabled through "Modify options".
    pub show_working_directory: bool,
    pub show_env: bool,
}

#[derive(Clone, Copy)]
struct Field {
    key: &'static str,
    label: &'static str,
    hint: &'static str,
    list: bool,
    optional: bool,
}

const fn field(
    key: &'static str,
    label: &'static str,
    hint: &'static str,
    list: bool,
    optional: bool,
) -> Field {
    Field {
        key,
        label,
        hint,
        list,
        optional,
    }
}

const PROGRAM_ARGS: Field = field(
    "program_arguments",
    "Program arguments",
    "Arguments passed to the program",
    true,
    false,
);
const OPTIONS: Field = field(
    "options",
    "Options",
    "Extra command-line options",
    true,
    false,
);
const GRADLE_FIELDS: &[Field] = &[
    field(
        "tasks",
        "Run",
        ":app:run or several tasks separated by spaces",
        true,
        false,
    ),
    OPTIONS,
    PROGRAM_ARGS,
];
const CARGO_FIELDS: &[Field] = &[
    field(
        "command",
        "Command",
        "run, test, build, bench",
        false,
        false,
    ),
    field("package", "Package", "Workspace member (-p)", false, true),
    field("bin", "Binary", "Binary target (--bin)", false, true),
    OPTIONS,
    PROGRAM_ARGS,
];
const BAZEL_FIELDS: &[Field] = &[
    field("command", "Command", "run, test, build", false, false),
    field("target", "Target", "//:app", false, false),
    OPTIONS,
    PROGRAM_ARGS,
];
const MAVEN_FIELDS: &[Field] = &[
    field("goals", "Goals", "clean verify", true, false),
    OPTIONS,
];
const NPM_FIELDS: &[Field] = &[
    field("script", "Script", "start", false, false),
    PROGRAM_ARGS,
];
const GO_FIELDS: &[Field] = &[
    field("command", "Command", "run, test, build", false, false),
    field("package", "Package", ". or ./cmd/server", false, false),
    PROGRAM_ARGS,
];
const APPLICATION_FIELDS: &[Field] = &[
    field(
        "module",
        "Module classpath (-cp)",
        ":app (empty: the root project)",
        false,
        true,
    ),
    field(
        "jdk",
        "JDK",
        "JAVA_HOME of the JDK (empty: the Gradle JDK)",
        false,
        true,
    ),
    field("main_class", "Main class", "com.example.App", false, false),
    PROGRAM_ARGS,
];
const SPRINGBOOT_FIELDS: &[Field] = &[
    field(
        "build",
        "Build tool",
        "gradle, maven or bazel",
        false,
        false,
    ),
    field(
        "target",
        "Module / target",
        "Gradle :app, Maven module directory or Bazel //:app (empty: root)",
        false,
        true,
    ),
    field("main_class", "Main class", "com.example.App", false, true),
    field(
        "profiles",
        "Active profiles",
        "dev,local (--spring.profiles.active)",
        false,
        true,
    ),
    PROGRAM_ARGS,
];
const SHELL_FIELDS: &[Field] = &[
    field(
        "program",
        "Program",
        "Executable on PATH or a path",
        false,
        false,
    ),
    field(
        "arguments",
        "Arguments",
        "Arguments separated by spaces",
        true,
        false,
    ),
];

fn fields(type_id: &str) -> &'static [Field] {
    match type_id {
        "application" => APPLICATION_FIELDS,
        "springboot" => SPRINGBOOT_FIELDS,
        "gradle" => GRADLE_FIELDS,
        "cargo" => CARGO_FIELDS,
        "bazel" => BAZEL_FIELDS,
        "maven" => MAVEN_FIELDS,
        "npm" => NPM_FIELDS,
        "go" => GO_FIELDS,
        _ => SHELL_FIELDS,
    }
}

impl Draft {
    pub fn from_config(config: &RunConfiguration) -> Self {
        let value = serde_json::to_value(config).unwrap_or(Value::Null);
        let type_id = static_type(config.target.type_id());
        let fields = fields(type_id)
            .iter()
            .map(|field| {
                let text = match &value[field.key] {
                    Value::Array(items) => items
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" "),
                    Value::String(text) => text.clone(),
                    _ => String::new(),
                };
                (field.key, text)
            })
            .collect();
        Self {
            previous: (!config.temporary).then(|| config.name.clone()),
            name: config.name.clone(),
            type_id,
            fields,
            working_directory: config
                .working_directory
                .as_ref()
                .map(|dir| dir.display().to_string())
                .unwrap_or_default(),
            env: config
                .env
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join("; "),
            temporary: config.temporary,
            dirty: false,
            open_tool_window: config.open_tool_window,
            show_working_directory: true,
            show_env: true,
        }
    }

    pub fn new(type_id: &str, name: String) -> Option<Self> {
        let target = RunTarget::template(type_id)?;
        let mut draft = Self::from_config(&RunConfiguration::new(name, target));
        draft.previous = None;
        draft.dirty = true;
        Some(draft)
    }

    /// Converts the form back into a configuration (validation included).
    pub fn to_config(&self) -> Result<RunConfiguration, String> {
        let mut object = Map::new();
        object.insert("name".into(), json!(self.name.trim()));
        object.insert("type".into(), json!(self.type_id));
        for (field, (_, text)) in fields(self.type_id).iter().zip(&self.fields) {
            let text = text.trim();
            if field.list {
                object.insert(
                    field.key.into(),
                    json!(text.split_whitespace().collect::<Vec<_>>()),
                );
            } else if !(field.optional && text.is_empty()) {
                object.insert(field.key.into(), json!(text));
            }
        }
        if !self.working_directory.trim().is_empty() {
            object.insert(
                "working_directory".into(),
                json!(self.working_directory.trim()),
            );
        }
        let mut env = Map::new();
        for pair in self
            .env
            .split(';')
            .map(str::trim)
            .filter(|pair| !pair.is_empty())
        {
            let (key, value) = pair
                .split_once('=')
                .ok_or_else(|| format!("Environment entry `{pair}` must look like KEY=value"))?;
            env.insert(key.trim().into(), json!(value.trim()));
        }
        object.insert("env".into(), Value::Object(env));
        let config: RunConfiguration =
            serde_json::from_value(Value::Object(object)).map_err(|error| error.to_string())?;
        let mut config = config;
        config.open_tool_window = self.open_tool_window;
        config.validate()?;
        Ok(config)
    }
}

fn static_type(type_id: &str) -> &'static str {
    RunTarget::TYPE_IDS
        .into_iter()
        .find(|id| *id == type_id)
        .unwrap_or("shell")
}

#[derive(Clone, Default)]
struct State {
    open: bool,
    drafts: Vec<Draft>,
    selected: usize,
    error: Option<String>,
    add_menu: bool,
    help: bool,
    /// Saved configurations removed in the dialog; deleted on Apply/OK.
    removed: Vec<String>,
}

fn state_id() -> egui::Id {
    egui::Id::new("run_config_dialog")
}

/// Whether the dialog is showing.
pub fn is_open(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp::<State>(state_id()))
        .is_some_and(|state| state.open)
}

/// Opens the dialog with the project's configurations (selecting `focus` if given).
pub fn open(app: &IdeApp, ctx: &egui::Context, focus: Option<&str>) {
    let drafts: Vec<Draft> = app
        .state
        .run_configurations
        .configurations()
        .iter()
        .map(Draft::from_config)
        .collect();
    let focus = focus.or(app
        .state
        .run_configurations
        .selected()
        .map(|c| c.name.as_str()));
    let selected = focus
        .and_then(|name| drafts.iter().position(|draft| draft.name == name))
        .unwrap_or(0);
    let error = app
        .state
        .run_configurations
        .errors
        .first()
        .map(|(path, error)| format!("Skipped {}: {error}", path.display()));
    ctx.data_mut(|data| {
        data.insert_temp(
            state_id(),
            State {
                open: true,
                drafts,
                selected,
                error,
                add_menu: false,
                help: false,
                removed: Vec::new(),
            },
        )
    });
}

enum Outcome {
    Keep,
    Close,
    Run(String),
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(mut state) = ctx.data(|data| data.get_temp::<State>(state_id())) else {
        return;
    };
    if !state.open {
        return;
    }
    let mut outcome = Outcome::Keep;
    let modal = egui::Modal::new(egui::Id::new("run_config_modal")).show(ctx, |ui| {
        ui.set_width(860.0);
        ui.set_height(520.0);
        ui.label(
            RichText::new("Run/Debug Configurations")
                .font(theme::semibold(theme::UI_FONT_SIZE))
                .color(theme::TEXT_BRIGHT),
        );
        ui.add_space(6.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(240.0);
                list(ui, &mut state);
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.set_width(590.0);
                form(ui, &mut state);
            });
        });
        ui.add_space(8.0);
        if let Some(error) = &state.error {
            ui.label(RichText::new(error).color(theme::ERROR));
        }
        if state.help {
            ui.label(
                RichText::new(
                    "Configurations are stored as files in .archipel/runConfigurations. \
                     Use Modify options to show optional fields.",
                )
                .color(theme::TEXT_INFO),
            );
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("?").on_hover_text("Help").clicked() {
                state.help = !state.help;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let dirty =
                    state.drafts.iter().any(|draft| draft.dirty) || !state.removed.is_empty();
                if ui.add_enabled(dirty, egui::Button::new("Apply")).clicked() {
                    apply(app, &mut state);
                }
                if ui.button("Cancel").clicked() {
                    outcome = Outcome::Close;
                }
                if ui
                    .add(
                        egui::Button::new(RichText::new("OK").color(egui::Color32::WHITE))
                            .fill(theme::ACCENT),
                    )
                    .clicked()
                    && apply(app, &mut state)
                {
                    outcome = Outcome::Close;
                }
                let run_enabled = !state.drafts.is_empty() && !app.state.runner.is_running();
                if ui
                    .add_enabled(run_enabled, egui::Button::new("▶ Run"))
                    .on_hover_text("Save and run the selected configuration")
                    .clicked()
                    && apply(app, &mut state)
                    && let Some(draft) = state.drafts.get(state.selected)
                {
                    outcome = Outcome::Run(draft.name.trim().to_owned());
                }
            });
        });
    });
    if modal.should_close() && matches!(outcome, Outcome::Keep) {
        outcome = Outcome::Close;
    }
    match outcome {
        Outcome::Keep => {}
        Outcome::Close => state.open = false,
        Outcome::Run(name) => {
            state.open = false;
            app.dispatch("runConfig.run", json!({"name": name}));
        }
    }
    ctx.data_mut(|data| data.insert_temp(state_id(), state));
}

fn list(ui: &mut egui::Ui, state: &mut State) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        if icons::button(ui, Icon::Add, "Add New Configuration").clicked() {
            state.add_menu = !state.add_menu;
        }
        let can_remove = !state.drafts.is_empty();
        if ui
            .add_enabled_ui(can_remove, |ui| {
                icons::button(ui, Icon::Minimize, "Remove Configuration")
            })
            .inner
            .clicked()
        {
            let draft = state.drafts.remove(state.selected);
            if let Some(name) = draft.previous {
                state.removed.push(name);
            }
            state.error = None;
            state.selected = state.selected.min(state.drafts.len().saturating_sub(1));
        }
        if ui
            .add_enabled_ui(can_remove, |ui| {
                icons::button(ui, Icon::File, "Copy Configuration")
            })
            .inner
            .clicked()
            && let Some(draft) = state.drafts.get(state.selected).cloned()
        {
            let mut copy = draft;
            copy.previous = None;
            copy.temporary = false;
            copy.dirty = true;
            copy.name = unique_name(&state.drafts, &format!("{} (copy)", copy.name));
            state.drafts.push(copy);
            state.selected = state.drafts.len() - 1;
        }
    });
    if state.add_menu {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.label(RichText::new("Add New Configuration").color(theme::TEXT_INFO));
            for type_id in RunTarget::TYPE_IDS {
                let target = RunTarget::template(type_id).expect("known type");
                if ui
                    .add(crate::widgets::menu_item(target.type_name()))
                    .clicked()
                {
                    let name = unique_name(&state.drafts, "Unnamed");
                    if let Some(draft) = Draft::new(type_id, name) {
                        state.drafts.push(draft);
                        state.selected = state.drafts.len() - 1;
                    }
                    state.add_menu = false;
                }
            }
        });
    }
    ui.add_space(4.0);
    ui.add_enabled(
        false,
        egui::Label::new(RichText::new("Edit configuration templates…").color(theme::TEXT_INFO)),
    )
    .on_disabled_hover_text("Configuration templates are not supported yet");
    ui.add_space(4.0);
    egui::ScrollArea::vertical()
        .id_salt("run_config_list")
        .auto_shrink([false, false])
        .max_height(410.0)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut group = "";
            let mut order: Vec<usize> = (0..state.drafts.len()).collect();
            order.sort_by_key(|&index| {
                (
                    state.drafts[index].type_id,
                    state.drafts[index].name.clone(),
                )
            });
            for index in order {
                let draft = &state.drafts[index];
                let type_name = RunTarget::template(draft.type_id)
                    .map(|target| target.type_name())
                    .unwrap_or("Shell Command");
                if type_name != group {
                    group = type_name;
                    let (rect, _) = list_row(ui, false);
                    icons::paint(
                        ui.painter(),
                        egui::Rect::from_min_size(
                            egui::pos2(rect.left() + 8.0, rect.center().y - 7.0),
                            vec2(14.0, 14.0),
                        ),
                        Icon::ChevronDown,
                        theme::TEXT_SECONDARY,
                    );
                    row_text(
                        ui,
                        rect,
                        rect.left() + 28.0,
                        type_name,
                        theme::semibold(theme::UI_FONT_SIZE),
                        theme::TEXT_BRIGHT,
                    );
                }
                let (rect, response) = list_row(ui, index == state.selected);
                icons::paint(
                    ui.painter(),
                    egui::Rect::from_min_size(
                        egui::pos2(rect.left() + 28.0, rect.center().y - 7.0),
                        vec2(14.0, 14.0),
                    ),
                    Icon::Run,
                    if draft.temporary {
                        theme::TEXT_INFO
                    } else {
                        theme::RUN_GREEN
                    },
                );
                let label = if draft.dirty {
                    format!("{} *", draft.name)
                } else {
                    draft.name.clone()
                };
                let color = if draft.temporary {
                    theme::TEXT_SECONDARY
                } else {
                    theme::TEXT
                };
                row_text(
                    ui,
                    rect,
                    rect.left() + 48.0,
                    &label,
                    egui::FontId::proportional(theme::UI_FONT_SIZE),
                    color,
                );
                if response.clicked() {
                    state.selected = index;
                }
            }
            if state.drafts.is_empty() {
                ui.add_space(12.0);
                ui.label(
                    RichText::new("No configurations. Click + to add one.").color(theme::TEXT_INFO),
                );
            }
        });
}

fn unique_name(drafts: &[Draft], base: &str) -> String {
    let mut name = base.to_owned();
    let mut counter = 2;
    while drafts.iter().any(|draft| draft.name == name) {
        name = format!("{base} ({counter})");
        counter += 1;
    }
    name
}

fn form(ui: &mut egui::Ui, state: &mut State) {
    let Some(draft) = state.drafts.get_mut(state.selected) else {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("Select a configuration or add a new one").color(theme::TEXT_INFO),
            );
        });
        return;
    };
    let mut changed = false;
    egui::Grid::new("run_config_form")
        .num_columns(2)
        .spacing(vec2(12.0, 10.0))
        .min_col_width(130.0)
        .show(ui, |ui| {
            ui.label("Name:");
            changed |= text(ui, &mut draft.name, "", 420.0);
            ui.end_row();
            ui.label("Type:");
            let type_name = RunTarget::template(draft.type_id)
                .map(|target| target.type_name())
                .unwrap_or("Shell Command");
            ui.label(RichText::new(type_name).color(theme::TEXT_SECONDARY));
            ui.end_row();
            ui.label("");
            ui.add_enabled(
                false,
                egui::Checkbox::new(&mut true, "Store as project file"),
            )
            .on_disabled_hover_text("Configurations are always stored in .archipel/runConfigurations");
            ui.end_row();
            ui.label("");
            ui.menu_button(
                RichText::new("Modify options").color(theme::ACCENT),
                |ui| {
                    ui.checkbox(&mut draft.show_working_directory, "Working directory");
                    ui.checkbox(&mut draft.show_env, "Environment variables");
                    if ui
                        .checkbox(
                            &mut draft.open_tool_window,
                            "Open run/debug tool window when started",
                        )
                        .changed()
                    {
                        changed = true;
                    }
                },
            );
            ui.end_row();
            if draft.temporary {
                ui.label("");
                ui.label(
                    RichText::new("Temporary configuration; Apply or OK stores it in .archipel/runConfigurations")
                        .color(theme::TEXT_INFO),
                );
                ui.end_row();
            }
            for (field, (_, value)) in fields(draft.type_id).iter().zip(draft.fields.iter_mut()) {
                ui.label(format!("{}:", field.label));
                changed |= text(ui, value, field.hint, 420.0);
                ui.end_row();
            }
            if draft.show_working_directory {
                ui.label("Working directory:");
                changed |= text(ui, &mut draft.working_directory, "Project root (relative path)", 420.0);
                ui.end_row();
            }
            if draft.show_env {
                ui.label("Environment variables:");
                changed |= text(ui, &mut draft.env, "KEY=value; KEY2=value2", 420.0);
                ui.end_row();
                ui.label("");
                ui.label(
                    RichText::new("Separate variables with semicolon: VAR=value; VAR1=value1")
                        .color(theme::TEXT_INFO)
                        .size(theme::UI_FONT_SIZE - 1.0),
                );
                ui.end_row();
            }
        });
    if draft.open_tool_window {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            egui::Frame::new()
                .fill(*theme::SELECTION)
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(8, 2))
                .show(ui, |ui| {
                    ui.label("Open run/debug tool window when started");
                    if ui
                        .small_button("×")
                        .on_hover_text("Remove option")
                        .clicked()
                    {
                        draft.open_tool_window = false;
                        changed = true;
                    }
                });
        });
    }
    ui.add_space(10.0);
    ui.add_enabled_ui(false, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Code Coverage").font(theme::semibold(theme::UI_FONT_SIZE)));
            let _ = ui.button("Modify");
        });
        ui.label(RichText::new("Coverage runners are not available yet.").color(theme::TEXT_INFO));
    });
    if changed {
        draft.dirty = true;
        state.error = draft.to_config().err();
    }
}

fn text(ui: &mut egui::Ui, value: &mut String, hint: &str, width: f32) -> bool {
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(RichText::new(hint).color(theme::TEXT_DISABLED))
            .desired_width(width)
            .font(egui::FontId::monospace(theme::UI_FONT_SIZE - 0.5))
            .margin(egui::Margin::symmetric(6, 4)),
    )
    .changed()
}

/// Deletes removed configurations and saves dirty drafts. Returns false on error.
fn apply(app: &mut IdeApp, state: &mut State) -> bool {
    for name in std::mem::take(&mut state.removed) {
        if let Err(error) =
            app.commands
                .execute("runConfig.delete", &mut app.state, json!({"name": name}))
        {
            state.error = Some(error.to_string());
            return false;
        }
    }
    for index in 0..state.drafts.len() {
        if !state.drafts[index].dirty {
            continue;
        }
        let config = match state.drafts[index].to_config() {
            Ok(config) => config,
            Err(error) => {
                state.selected = index;
                state.error = Some(format!("{}: {error}", state.drafts[index].name));
                return false;
            }
        };
        let params = json!({
            "previousName": state.drafts[index].previous,
            "configuration": serde_json::to_value(&config).unwrap_or(Value::Null),
        });
        if let Err(error) = app
            .commands
            .execute("runConfig.save", &mut app.state, params)
        {
            state.selected = index;
            state.error = Some(error.to_string());
            return false;
        }
        let draft = &mut state.drafts[index];
        draft.previous = Some(config.name.clone());
        draft.name = config.name;
        draft.temporary = false;
        draft.dirty = false;
    }
    if let Some(draft) = state.drafts.get(state.selected) {
        let _ = app.commands.execute(
            "runConfig.select",
            &mut app.state,
            json!({"name": draft.name}),
        );
    }
    state.error = None;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_round_trips_every_type() {
        for type_id in RunTarget::TYPE_IDS {
            let mut target = RunTarget::template(type_id).unwrap();
            match &mut target {
                RunTarget::Shell { program, .. } => *program = "ls".into(),
                RunTarget::Application { main_class, .. } => *main_class = "com.example.App".into(),
                _ => {}
            }
            let mut config = RunConfiguration::new("X", target);
            config.env.insert("A".into(), "1".into());
            config.env.insert("B".into(), "two words".into());
            config.working_directory = Some("sub".into());
            let draft = Draft::from_config(&config);
            assert_eq!(draft.env, "A=1; B=two words");
            assert_eq!(draft.to_config().unwrap(), config, "{type_id}");
        }
    }

    #[test]
    fn form_text_becomes_typed_fields() {
        let mut draft = Draft::new("cargo", "Server".into()).unwrap();
        draft.fields = vec![
            ("command", "run".into()),
            ("package", "".into()),
            ("bin", "server".into()),
            ("options", "--release  --locked".into()),
            ("program_arguments", "--port 8080".into()),
        ];
        let config = draft.to_config().unwrap();
        assert_eq!(
            config.target,
            RunTarget::Cargo {
                command: "run".into(),
                package: None,
                bin: Some("server".into()),
                options: vec!["--release".into(), "--locked".into()],
                program_arguments: vec!["--port".into(), "8080".into()],
            }
        );
        draft.env = "NOVALUE".into();
        assert!(draft.to_config().unwrap_err().contains("KEY=value"));
        draft.env.clear();
        draft.working_directory = "../outside".into();
        assert!(draft.to_config().is_err());
    }

    #[test]
    fn application_drafts_need_a_main_class_and_keep_the_tool_window_option() {
        let mut draft = Draft::new("application", "App".into()).unwrap();
        assert!(draft.to_config().unwrap_err().contains("main class"));
        draft.fields = vec![
            ("module", ":app".into()),
            ("jdk", "/usr/lib/jvm/java-21".into()),
            ("main_class", "com.example.App".into()),
            ("program_arguments", "-v".into()),
        ];
        draft.open_tool_window = false;
        let config = draft.to_config().unwrap();
        assert!(!config.open_tool_window);
        assert_eq!(
            config.target,
            RunTarget::Application {
                module: ":app".into(),
                jdk: "/usr/lib/jvm/java-21".into(),
                main_class: "com.example.App".into(),
                program_arguments: vec!["-v".into()],
            }
        );
    }

    #[test]
    fn unique_names_do_not_clash() {
        let drafts = vec![
            Draft::new("shell", "Unnamed".into()).unwrap(),
            Draft::new("shell", "Unnamed (2)".into()).unwrap(),
        ];
        assert_eq!(unique_name(&drafts, "Unnamed"), "Unnamed (3)");
    }
}

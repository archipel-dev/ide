//! Run/Debug Configurations dialog: configuration list on the left, form on the right.

use crate::IdeApp;
use crate::dialog::{self, Outcome, Shell};
use archipel_kit::components::{
    Banner, Button, Checkbox, EmptyState, IconButton, Link, ListRow, MenuItem, PropertyGrid,
    SectionHeader, Severity, TextField, VirtualList,
};
use archipel_kit::parts::Popover;
use archipel_kit::tokens::size;
use archipel_kit::{Glyph, Tone};
use eframe::egui;
use jetthing_runner::{RunConfiguration, RunTarget};
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
    modify_menu: bool,
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
                modify_menu: false,
                help: false,
                removed: Vec::new(),
            },
        )
    });
}

enum Choice {
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
    let mut choice = Choice::Keep;
    let dirty = state.drafts.iter().any(|draft| draft.dirty) || !state.removed.is_empty();
    let run_enabled = !state.drafts.is_empty() && !app.state.runner.is_running();
    let height = (ctx.content_rect().height() - 220.0).clamp(240.0, 480.0);
    let outcome = Shell::new("run_config", "Run/Debug Configurations")
        .width(880.0)
        .ok("OK")
        .secondary("Help", true)
        .secondary("Apply", dirty)
        .secondary("Run", run_enabled)
        .keys_held(state.add_menu || state.modify_menu)
        .show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                ui.vertical(|ui| {
                    ui.set_width(250.0);
                    list(ui, ctx, &mut state, height);
                });
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    form(ui, ctx, &mut state, height);
                });
            });
            if state.help {
                dialog::hint(
                    ui,
                    "Configurations are stored as files in .jetthing/runConfigurations. \
                     Use Modify options to show optional fields.",
                );
            }
        });
    match outcome {
        Some(Outcome::Cancel) => choice = Choice::Close,
        Some(Outcome::Ok) if apply(app, &mut state) => choice = Choice::Close,
        Some(Outcome::Secondary(0)) => state.help = !state.help,
        Some(Outcome::Secondary(1)) => {
            apply(app, &mut state);
        }
        Some(Outcome::Secondary(_)) => {
            if apply(app, &mut state)
                && let Some(draft) = state.drafts.get(state.selected)
            {
                choice = Choice::Run(draft.name.trim().to_owned());
            }
        }
        _ => {}
    }
    match choice {
        Choice::Keep => {}
        Choice::Close => state.open = false,
        Choice::Run(name) => {
            state.open = false;
            app.dispatch("runConfig.run", json!({"name": name}));
        }
    }
    ctx.data_mut(|data| data.insert_temp(state_id(), state));
}

/// A row of the configuration list: a type heading or a configuration (by draft index).
#[derive(Clone, Copy)]
enum Entry {
    Type(&'static str),
    Config(usize),
}

fn entries(state: &State) -> Vec<Entry> {
    let mut order: Vec<usize> = (0..state.drafts.len()).collect();
    order.sort_by_key(|&index| {
        (
            state.drafts[index].type_id,
            state.drafts[index].name.clone(),
        )
    });
    let mut entries = Vec::new();
    let mut group = "";
    for index in order {
        let type_name = RunTarget::template(state.drafts[index].type_id)
            .map(|target| target.type_name())
            .unwrap_or("Shell Command");
        if type_name != group {
            group = type_name;
            entries.push(Entry::Type(type_name));
        }
        entries.push(Entry::Config(index));
    }
    entries
}

fn list(ui: &mut egui::Ui, ctx: &egui::Context, state: &mut State, height: f32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = size::TOOLBAR_GAP;
        let add = ui.add(IconButton::new(Glyph::Plus, "Add New Configuration"));
        if add.clicked() {
            state.add_menu = !state.add_menu;
        }
        let can_remove = !state.drafts.is_empty();
        if ui
            .add(IconButton::new(Glyph::Minus, "Remove Configuration").enabled(can_remove))
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
            .add(IconButton::new(Glyph::Copy, "Copy Configuration").enabled(can_remove))
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
        let mut open = state.add_menu;
        let mut picked = false;
        Popover::below("run_config_add", &add)
            .width(220.0)
            .show(ctx, &mut open, |ui| {
                for type_id in RunTarget::TYPE_IDS {
                    let target = RunTarget::template(type_id).expect("known type");
                    if ui.add(MenuItem::new(target.type_name())).clicked() {
                        let name = unique_name(&state.drafts, "Unnamed");
                        if let Some(draft) = Draft::new(type_id, name) {
                            state.drafts.push(draft);
                            state.selected = state.drafts.len() - 1;
                        }
                        picked = true;
                    }
                }
            });
        state.add_menu = open && !picked;
    });
    ui.add_enabled(false, Link::new("Edit configuration templates…"))
        .on_disabled_hover_text("Configuration templates are not supported yet");
    let entries = entries(state);
    if entries.is_empty() {
        dialog::hint(ui, "No configurations. Click + to add one.");
        return;
    }
    let position = |draft: usize| {
        entries
            .iter()
            .position(|entry| matches!(entry, Entry::Config(index) if *index == draft))
    };
    let mut selected = position(state.selected);
    VirtualList::new("run_config_list", entries.len(), &mut selected)
        .name("Run configurations")
        .height(height - 60.0)
        .show(ui, |index| match entries[index] {
            Entry::Type(name) => ListRow::new(name).enabled(false),
            Entry::Config(draft_index) => {
                let draft = &state.drafts[draft_index];
                let label = if draft.dirty {
                    format!("{} *", draft.name)
                } else {
                    draft.name.clone()
                };
                ListRow::new(label)
                    .depth(1)
                    .icon(Glyph::Run)
                    .icon_tone(if draft.temporary {
                        Tone::Muted
                    } else {
                        Tone::Run
                    })
                    .label_tone(if draft.temporary {
                        Tone::Secondary
                    } else {
                        Tone::Default
                    })
            }
        });
    if let Some(to) = selected
        && let Some(from) = position(state.selected)
        && to != from
    {
        let to = dialog::skip_unselectable(entries.len(), from, to, |index| {
            matches!(entries[index], Entry::Config(_))
        });
        if let Entry::Config(index) = entries[to] {
            state.selected = index;
        }
    }
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

fn form(ui: &mut egui::Ui, ctx: &egui::Context, state: &mut State, height: f32) {
    if let Some(error) = &state.error {
        Banner::new(Severity::Error, error.clone()).show(ui);
    }
    let Some(draft) = state.drafts.get_mut(state.selected) else {
        EmptyState::new("Select a configuration or add a new one").show(ui);
        return;
    };
    let mut changed = false;
    egui::ScrollArea::vertical()
        .id_salt("run_config_form")
        .max_height(height)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            changed |= ui
                .add(TextField::new(&mut draft.name).label("Name"))
                .changed();
            let type_name = RunTarget::template(draft.type_id)
                .map(|target| target.type_name())
                .unwrap_or("Shell Command");
            PropertyGrid::new("run_config_type")
                .row("Type", type_name)
                .show(ui);
            ui.add(Checkbox::new(&mut true, "Store as project file").enabled(false))
                .on_disabled_hover_text(
                    "Configurations are always stored in .jetthing/runConfigurations",
                );
            let modify = ui.add(Button::ghost("Modify options"));
            if modify.clicked() {
                state.modify_menu = !state.modify_menu;
            }
            let mut open = state.modify_menu;
            Popover::below("run_config_modify", &modify)
                .width(240.0)
                .show(ctx, &mut open, |ui| {
                    if ui
                        .add(
                            MenuItem::new("Working directory")
                                .checked(draft.show_working_directory),
                        )
                        .clicked()
                    {
                        draft.show_working_directory = !draft.show_working_directory;
                    }
                    if ui
                        .add(MenuItem::new("Environment variables").checked(draft.show_env))
                        .clicked()
                    {
                        draft.show_env = !draft.show_env;
                    }
                });
            state.modify_menu = open;
            if draft.temporary {
                dialog::hint(
                    ui,
                    "Temporary configuration; Apply or OK stores it in .jetthing/runConfigurations",
                );
            }
            for (field, (_, value)) in fields(draft.type_id).iter().zip(draft.fields.iter_mut()) {
                changed |= ui
                    .add(
                        TextField::new(value)
                            .label(field.label)
                            .placeholder(field.hint)
                            .monospace(true),
                    )
                    .changed();
            }
            if draft.show_working_directory {
                changed |= ui
                    .add(
                        TextField::new(&mut draft.working_directory)
                            .label("Working directory")
                            .placeholder("Project root (relative path)")
                            .monospace(true),
                    )
                    .changed();
            }
            if draft.show_env {
                changed |= ui
                    .add(
                        TextField::new(&mut draft.env)
                            .label("Environment variables")
                            .placeholder("KEY=value; KEY2=value2")
                            .monospace(true),
                    )
                    .changed();
                dialog::hint(
                    ui,
                    "Separate variables with semicolon: VAR=value; VAR1=value1",
                );
            }
            changed |= ui
                .add(Checkbox::new(
                    &mut draft.open_tool_window,
                    "Open run/debug tool window when started",
                ))
                .changed();
            ui.add(SectionHeader::new("Code Coverage"));
            ui.add(Button::secondary("Modify").enabled(false));
            dialog::hint(ui, "Coverage runners are not available yet.");
        });
    if changed {
        draft.dirty = true;
        state.error = draft.to_config().err();
    }
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

//! The reference IDE's Settings tree and Archipel IDE's initial setting definitions.
//!
//! Pages come from `docs/spec/data/settings.json` (Apache-2.0 derived data, see
//! `docs/spec/data/NOTICE.md`), so page ids equal the reference configurable ids.
//! Setting labels, groups and defaults follow the observable Settings dialog
//! (`docs/parity/evidence/reference/*/08-settings.png`) and the public documentation.

use crate::{EnumOption, PageDef, Scope, SettingDef, SettingKind, SettingValue, SettingsRegistry};
use serde::Deserialize;

const SETTINGS_JSON: &str = include_str!("../../../docs/spec/data/settings.json");

/// Page id of the generated Advanced Settings page.
pub const ADVANCED_SETTINGS_PAGE: &str = "advanced.settings";

/// Top-level order of the Settings tree as the reference dialog shows it.
pub const TOP_LEVEL_ORDER: [&str; 10] = [
    "appearance",
    "preferences.keymap",
    "editor",
    "preferences.pluginManager",
    "project.propVCSSupport.Mappings",
    "build",
    "language",
    "tools",
    "settings.sync",
    ADVANCED_SETTINGS_PAGE,
];

const GROUPS: [(&str, &str); 5] = [
    ("appearance", "Appearance & Behavior"),
    ("editor", "Editor"),
    ("build", "Build, Execution, Deployment"),
    ("language", "Languages & Frameworks"),
    ("tools", "Tools"),
];

/// Ids of settings that Archipel IDE subsystems read.
pub mod ids {
    pub const THEME: &str = "appearance.theme";
    pub const EDITOR_SCHEME: &str = "appearance.editorColorScheme";
    pub const ZOOM: &str = "appearance.zoom";
    pub const CUSTOM_FONT: &str = "appearance.customFont";
    pub const CUSTOM_FONT_SIZE: &str = "appearance.customFontSize";
    pub const SMOOTH_SCROLLING: &str = "appearance.smoothScrolling";
    pub const COMPACT_MODE: &str = "appearance.compactMode";
    /// The UI look: `islands`, `new_ui` or `classic` (the kit's `UiLook` ids).
    pub const UI_LOOK: &str = "appearance.uiLook";
    pub const PROJECT_COLORS: &str = "appearance.projectColors";
    pub const KEYMAP: &str = "keymap.active";
    /// User shortcut changes, one `keymap<TAB>action<TAB>shortcut…` line per action.
    pub const KEYMAP_OVERRIDES: &str = "keymap.overrides";
    /// User live templates, one JSON `UserTemplate` per line.
    pub const LIVE_TEMPLATES_USER: &str = "editor.liveTemplates.user";
    /// Switched-off live template groups and templates (`group` or `group/abbreviation`).
    pub const LIVE_TEMPLATES_DISABLED: &str = "editor.liveTemplates.disabled";
    /// TODO patterns, one `i|s <color key> <regex>` line each (`i`: ignore case).
    /// Days Local History keeps revisions.
    pub const LOCAL_HISTORY_DAYS: &str = "system.localHistory.retentionDays";
    pub const TODO_PATTERNS: &str = "editor.todo.patterns";
    /// TODO tool window filters, one `Name: 0,1` line each (pattern indexes).
    pub const TODO_FILTERS: &str = "editor.todo.filters";
    /// Search for TODO patterns in every line of block comments.
    pub const TODO_MULTILINE: &str = "editor.todo.multiline";
    pub const EDITOR_FONT_FAMILY: &str = "editor.font.family";
    pub const EDITOR_FONT_SIZE: &str = "editor.font.size";
    pub const EDITOR_LINE_HEIGHT: &str = "editor.font.lineHeight";
    pub const EDITOR_LIGATURES: &str = "editor.font.ligatures";
    pub const STRIP_TRAILING_SPACES: &str = "editor.general.stripTrailingSpaces";
    pub const ENSURE_NEWLINE: &str = "editor.general.ensureNewlineAtEof";
    pub const WHEEL_ZOOM: &str = "editor.general.wheelFontChange";
    pub const EDITOR_PREVIEW_TAB: &str = "editor.tabs.preview";
    pub const SHOW_LINE_NUMBERS: &str = "editor.appearance.lineNumbers";
    pub const PAIR_BRACKETS: &str = "editor.codeEditing.pairBrackets";
    pub const PAIR_QUOTES: &str = "editor.codeEditing.pairQuotes";
    pub const GRADLE_DISTRIBUTION: &str = "gradle.distribution";
    pub const GRADLE_BUILD_RUN: &str = "gradle.buildRunUsing";
    pub const GRADLE_OFFLINE: &str = "gradle.offline";
    pub const GRADLE_USER_HOME: &str = "gradle.userHome";
    pub const GRADLE_JVM: &str = "gradle.jvm";
    pub const CHECK_UPDATES: &str = "updates.checkOnStartup";
    pub const TERMINAL_SHELL: &str = "terminal.shellPath";
    pub const AUTOSAVE_FOCUS: &str = "system.saveOnFrameDeactivation";
    pub const REOPEN_PROJECT: &str = "system.reopenLastProject";
}

#[derive(Deserialize)]
struct SpecData {
    configurables: Vec<Configurable>,
    #[serde(rename = "advancedSettings")]
    advanced_settings: Vec<AdvancedSetting>,
}

#[derive(Deserialize)]
struct Configurable {
    id: String,
    #[serde(rename = "parentId")]
    parent_id: Option<String>,
    #[serde(rename = "groupId")]
    group_id: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    level: String,
    #[serde(rename = "groupWeight")]
    group_weight: i32,
}

#[derive(Deserialize)]
struct AdvancedSetting {
    id: String,
    default: String,
    #[serde(rename = "enum")]
    enum_class: Option<String>,
    group: Option<String>,
    title: Option<String>,
    description: Option<String>,
}

pub(crate) fn registry() -> SettingsRegistry {
    let mut registry = SettingsRegistry::default();
    // The spec data is compiled in and covered by tests; a malformed file yields an
    // empty tree rather than a panic.
    let data: SpecData = serde_json::from_str(SETTINGS_JSON).unwrap_or(SpecData {
        configurables: Vec::new(),
        advanced_settings: Vec::new(),
    });
    for (id, label) in GROUPS {
        let _ = registry.register_page(PageDef::new(id, None, label));
    }
    add_configurables(&mut registry, &data.configurables);
    add_advanced_settings(&mut registry, &data.advanced_settings);
    add_definitions(&mut registry);
    registry
}

fn add_configurables(registry: &mut SettingsRegistry, configurables: &[Configurable]) {
    // Parents can appear after their children in the data; insert until stable.
    let mut pending: Vec<&Configurable> = configurables
        .iter()
        .filter(|c| c.display_name.is_some())
        .collect();
    loop {
        let before = pending.len();
        pending.retain(|c| {
            let parent = c.parent_id.as_deref().or(match c.group_id.as_deref() {
                Some("root") | None => None,
                Some(group) => Some(group),
            });
            if parent.is_some_and(|parent| registry.page(parent).is_none()) {
                return true;
            }
            let level = if c.level == "project" {
                Scope::Project
            } else {
                Scope::Application
            };
            let page = PageDef::new(&c.id, parent, c.display_name.clone().unwrap_or_default())
                .level(level)
                .weight(c.group_weight);
            // Duplicate ids (registered by two modules) keep the first definition.
            let _ = registry.register_page(page);
            false
        });
        if pending.len() == before {
            // Remaining pages hang below parents outside the Community data.
            break;
        }
    }
}

fn humanize(constant: &str) -> String {
    let lower = constant.replace('_', " ").to_lowercase();
    let mut chars = lower.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

fn add_advanced_settings(registry: &mut SettingsRegistry, settings: &[AdvancedSetting]) {
    let _ = registry.register_page(PageDef::new(
        ADVANCED_SETTINGS_PAGE,
        None,
        "Advanced Settings",
    ));
    let mut sorted: Vec<_> = settings.iter().collect();
    sorted.sort_by(|a, b| {
        (
            a.group.as_deref().unwrap_or(""),
            a.title.as_deref().unwrap_or(&a.id),
        )
            .cmp(&(
                b.group.as_deref().unwrap_or(""),
                b.title.as_deref().unwrap_or(&b.id),
            ))
    });
    for setting in sorted {
        let (kind, default) = if setting.enum_class.is_some() {
            // The data names the enum class but not its constants; only the default
            // is known, so it is the single option until the values are specified.
            (
                SettingKind::Enum {
                    options: vec![EnumOption::new(
                        &setting.default,
                        humanize(&setting.default),
                    )],
                },
                SettingValue::Text(setting.default.clone()),
            )
        } else if let Ok(value) = setting.default.parse::<bool>() {
            (SettingKind::Bool, SettingValue::Bool(value))
        } else if let Ok(value) = setting.default.parse::<i64>() {
            (
                SettingKind::Int {
                    min: i64::from(i32::MIN),
                    max: i64::from(i32::MAX),
                },
                SettingValue::Int(value),
            )
        } else {
            (
                SettingKind::Text,
                SettingValue::Text(setting.default.clone()),
            )
        };
        let label = setting.title.clone().unwrap_or_else(|| setting.id.clone());
        let mut def = SettingDef::new(&setting.id, ADVANCED_SETTINGS_PAGE, label, kind, default);
        def.group = setting.group.clone();
        def.description = setting.description.clone();
        let _ = registry.register_setting(def);
    }
}

/// UI themes offered in Settings | Appearance: the IntelliJ themes of the UI kit, in its
/// order. A test in `archipel-ide-ui` checks the list against the kit.
pub const THEMES: [&str; 12] = [
    "Islands Dark",
    "Islands Light",
    "Islands Darcula",
    "High Contrast",
    "New UI Dark",
    "New UI Light",
    "Light with Light Header",
    "Darcula",
    "IntelliJ Light",
    "High Contrast (Classic)",
    "macOS Light",
    "Windows 10 Light",
];

/// Editor color schemes of the UI kit; the first option is the theme's own.
pub const SCHEMES: [&str; 18] = [
    "Default",
    "Darcula",
    "Islands Dark",
    "New UI Dark",
    "New UI Light",
    "Darcula Contrast",
    "IntelliJ Light",
    "High Contrast",
    "High Contrast (Classic)",
    "Monokai",
    "Twilight",
    "Blackboard",
    "Cobalt",
    "All Hallows Eve",
    "Warm Neon",
    "GitHub",
    "Vibrant Ink",
    "Rails Casts",
];

fn scheme_options() -> Vec<(&'static str, &'static str)> {
    std::iter::once(("", "Theme default"))
        .chain(SCHEMES.iter().map(|name| (*name, *name)))
        .collect()
}

fn options(pairs: &[(&str, &str)]) -> SettingKind {
    SettingKind::Enum {
        options: pairs
            .iter()
            .map(|(id, label)| EnumOption::new(*id, *label))
            .collect(),
    }
}

fn text(value: &str) -> SettingValue {
    SettingValue::Text(value.into())
}

/// The keymap the reference IDE selects by default on this platform.
/// The keymap the reference IDE selects by default on this desktop
/// (macOS → macOS, GNOME → GNOME, KDE → KDE, other Unix → XWin, else Windows).
pub fn default_keymap() -> &'static str {
    archipel_ide_keymap::Desktop::detect().default_keymap()
}

/// The bundled keymaps: the five primary ones first, then the rest by name.
fn keymap_options() -> Vec<EnumOption> {
    let mut ids: Vec<String> = archipel_ide_keymap::PRIMARY_KEYMAPS
        .iter()
        .map(|id| (*id).to_owned())
        .collect();
    if let Ok(set) = archipel_ide_keymap::KeymapSet::shared() {
        let mut rest: Vec<String> = set
            .ids()
            .filter(|id| !archipel_ide_keymap::PRIMARY_KEYMAPS.contains(id))
            .map(Into::into)
            .collect();
        rest.sort_by_key(|id| id.to_lowercase());
        ids.extend(rest);
    }
    ids.into_iter()
        .map(|id| {
            let label = archipel_ide_keymap::display_name(&id).to_owned();
            EnumOption::new(id, label)
        })
        .collect()
}

const CODE_EDITING: &str = "preferences.editor.code.editing";
const GRADLE: &str = "reference.settingsdialog.project.gradle";

fn add_definitions(registry: &mut SettingsRegistry) {
    use SettingKind::{Bool, Path};
    use SettingValue::{Bool as B, Int as I};
    let look = "preferences.lookFeel";
    let accessibility = "Accessibility";
    let ui_options = "UI Options";
    let zoom: Vec<_> = [
        "50%", "75%", "80%", "90%", "100%", "110%", "125%", "150%", "175%", "200%",
    ]
    .iter()
    .map(|label| (*label, *label))
    .collect();
    let definitions = vec![
        SettingDef::new(
            ids::THEME,
            look,
            "Theme",
            options(&THEMES.map(|name| (name, name))),
            text("Islands Dark"),
        ),
        SettingDef::new(
            ids::UI_LOOK,
            look,
            "UI look",
            options(&[
                ("islands", "Islands"),
                ("new_ui", "New UI"),
                ("classic", "Classic"),
            ]),
            text("islands"),
        )
        .description("Islands, New UI (no islands) or the classic pre-2022 look"),
        SettingDef::new(
            ids::EDITOR_SCHEME,
            look,
            "Editor color scheme",
            options(&scheme_options()),
            text(""),
        ),
        SettingDef::new(ids::ZOOM, look, "Zoom", options(&zoom), text("100%"))
            .group(accessibility)
            .description("Change with Ctrl+Alt+Shift+= or Ctrl+Alt+Shift+Minus"),
        SettingDef::new(ids::CUSTOM_FONT, look, "Use custom font", Bool, B(false))
            .group(accessibility),
        SettingDef::new(
            ids::CUSTOM_FONT_SIZE,
            look,
            "Size",
            SettingKind::Int { min: 6, max: 40 },
            I(13),
        )
        .group(accessibility),
        SettingDef::new(
            "appearance.screenReaders",
            look,
            "Support screen readers",
            Bool,
            B(false),
        )
        .group(accessibility)
        .restart(),
        SettingDef::new(
            "appearance.contrastScrollbars",
            look,
            "Use contrast scrollbars",
            Bool,
            B(false),
        )
        .group(accessibility),
        SettingDef::new(ids::COMPACT_MODE, look, "Compact mode", Bool, B(false))
            .group(ui_options)
            .description("UI elements take up less screen space"),
        SettingDef::new(
            "appearance.fullPathInTitle",
            look,
            "Always show full path in window header",
            Bool,
            B(false),
        )
        .group(ui_options),
        SettingDef::new(
            ids::PROJECT_COLORS,
            look,
            "Use project colors in main toolbar",
            Bool,
            B(true),
        )
        .group(ui_options)
        .description("Distinguish projects with different toolbar colors at a glance."),
        SettingDef::new(
            "appearance.keepPopupsOpen",
            look,
            "Keep popups open for toggle items",
            Bool,
            B(true),
        )
        .group(ui_options),
        SettingDef::new(
            "appearance.dndWithAlt",
            look,
            "Drag-and-drop with Alt pressed only",
            Bool,
            B(false),
        )
        .group(ui_options),
        SettingDef::new(
            ids::SMOOTH_SCROLLING,
            look,
            "Smooth scrolling",
            Bool,
            B(true),
        )
        .group(ui_options),
        SettingDef::new(
            "appearance.mnemonicsControls",
            look,
            "Enable mnemonics in controls",
            Bool,
            B(true),
        )
        .group(ui_options),
        SettingDef::new(
            "appearance.mnemonicsMenu",
            look,
            "Enable mnemonics in menu",
            Bool,
            B(true),
        )
        .group(ui_options),
        SettingDef::new(
            "appearance.menuIcons",
            look,
            "Display icons in menu items",
            Bool,
            B(true),
        )
        .group(ui_options),
        SettingDef::new(
            ids::KEYMAP,
            "preferences.keymap",
            "Keymap",
            SettingKind::Enum {
                options: keymap_options(),
            },
            text(default_keymap()),
        ),
        SettingDef::new(
            ids::KEYMAP_OVERRIDES,
            "preferences.keymap",
            "Modified shortcuts",
            SettingKind::List,
            SettingValue::List(Vec::new()),
        ),
        SettingDef::new(
            ids::LIVE_TEMPLATES_USER,
            "editing.templates",
            "User live templates",
            SettingKind::List,
            SettingValue::List(Vec::new()),
        ),
        SettingDef::new(
            ids::LIVE_TEMPLATES_DISABLED,
            "editing.templates",
            "Disabled live templates",
            SettingKind::List,
            SettingValue::List(Vec::new()),
        ),
        SettingDef::new(
            ids::TODO_PATTERNS,
            "preferences.toDoOptions",
            "Patterns",
            SettingKind::List,
            SettingValue::List(vec![
                r"i todo \btodo\b.*".into(),
                r"i fixme \bfixme\b.*".into(),
            ]),
        )
        .description(
            "One pattern per line: i (ignore case) or s (case sensitive), a color key, then a regular expression",
        ),
        SettingDef::new(
            ids::TODO_FILTERS,
            "preferences.toDoOptions",
            "Filters",
            SettingKind::List,
            SettingValue::List(Vec::new()),
        )
        .description("One filter per line: Name: followed by the pattern numbers, e.g. Urgent: 1"),
        SettingDef::new(
            ids::TODO_MULTILINE,
            "preferences.toDoOptions",
            "Search for TODO patterns in all lines of multiline comments",
            Bool,
            B(true),
        ),
        SettingDef::new(
            ids::EDITOR_FONT_FAMILY,
            "AppEditorFontConfigurable",
            "Font",
            options(&[("JetBrains Mono", "JetBrains Mono")]),
            text("JetBrains Mono"),
        ),
        SettingDef::new(
            ids::EDITOR_FONT_SIZE,
            "AppEditorFontConfigurable",
            "Size",
            SettingKind::Int { min: 4, max: 72 },
            I(13),
        ),
        SettingDef::new(
            ids::EDITOR_LINE_HEIGHT,
            "AppEditorFontConfigurable",
            "Line height",
            SettingKind::Float { min: 0.6, max: 3.0 },
            SettingValue::Float(1.2),
        ),
        SettingDef::new(
            ids::EDITOR_LIGATURES,
            "AppEditorFontConfigurable",
            "Enable ligatures",
            Bool,
            B(false),
        ),
        SettingDef::new(
            ids::WHEEL_ZOOM,
            "preferences.editor",
            "Change font size with Ctrl+Mouse Wheel",
            Bool,
            B(false),
        )
        .group("Mouse Control"),
        SettingDef::new(
            ids::STRIP_TRAILING_SPACES,
            "preferences.editor",
            "Remove trailing spaces on",
            options(&[
                ("none", "None"),
                ("modified", "Modified lines"),
                ("all", "All lines"),
            ]),
            text("modified"),
        )
        .group("On Save"),
        SettingDef::new(
            ids::ENSURE_NEWLINE,
            "preferences.editor",
            "Ensure every saved file ends with a line break",
            Bool,
            B(false),
        )
        .group("On Save"),
        SettingDef::new(
            ids::EDITOR_PREVIEW_TAB,
            "preferences.editor",
            "Open files in a preview tab on single click in the Project tree",
            Bool,
            B(true),
        )
        .group("Editor Tabs"),
        SettingDef::new(
            ids::SHOW_LINE_NUMBERS,
            "preferences.editor",
            "Show line numbers",
            Bool,
            B(true),
        )
        .group("Appearance"),
        SettingDef::new(
            ids::PAIR_BRACKETS,
            CODE_EDITING,
            "Insert pair bracket",
            Bool,
            B(true),
        )
        .group("Smart Keys")
        .description("Typing ( [ or { also inserts the closing bracket"),
        SettingDef::new(
            ids::PAIR_QUOTES,
            CODE_EDITING,
            "Insert pair quote",
            Bool,
            B(true),
        )
        .group("Smart Keys")
        .description("Typing a quote also inserts the closing quote"),
        SettingDef::new(
            ids::GRADLE_DISTRIBUTION,
            GRADLE,
            "Distribution",
            options(&[
                ("wrapper", "Wrapper"),
                ("local", "Local installation (gradle on PATH)"),
            ]),
            text("wrapper"),
        )
        .scope(Scope::Project)
        .description("Wrapper falls back to gradle on PATH when the project has no gradlew"),
        SettingDef::new(
            ids::GRADLE_BUILD_RUN,
            GRADLE,
            "Build and run using",
            options(&[("gradle", "Gradle"), ("archipel-ide", "Archipel IDE")]),
            text("gradle"),
        )
        .scope(Scope::Project)
        .description("Gradle runs the full build task; Archipel IDE only compiles before running"),
        SettingDef::new(ids::GRADLE_OFFLINE, GRADLE, "Offline work", Bool, B(false))
            .scope(Scope::Project)
            .description("Runs Gradle with --offline"),
        SettingDef::new(
            ids::GRADLE_USER_HOME,
            GRADLE,
            "Gradle user home",
            Path,
            text(""),
        )
        .description("Sets GRADLE_USER_HOME; empty uses the default (~/.gradle)"),
        SettingDef::new(ids::GRADLE_JVM, GRADLE, "Gradle JVM", Path, text(""))
            .scope(Scope::Project)
            .description("JDK home passed as JAVA_HOME; empty uses the environment's JDK"),
        SettingDef::new(
            ids::REOPEN_PROJECT,
            "preferences.general",
            "Reopen projects on startup",
            Bool,
            B(true),
        )
        .group("Project"),
        SettingDef::new(
            ids::LOCAL_HISTORY_DAYS,
            "preferences.general",
            "Keep Local History for (days)",
            SettingKind::Int { min: 1, max: 365 },
            I(5),
        )
        .group("Local History"),
        SettingDef::new(
            ids::AUTOSAVE_FOCUS,
            "preferences.general",
            "Save files if the IDE is switched to a different application",
            Bool,
            B(true),
        )
        .group("Autosave"),
        SettingDef::new(
            ids::CHECK_UPDATES,
            "preferences.updates",
            "Check IDE updates",
            Bool,
            B(true),
        ),
        SettingDef::new(
            ids::TERMINAL_SHELL,
            "terminal",
            "Shell path",
            Path,
            text(""),
        )
        .scope(Scope::Project)
        .description("Empty uses the login shell ($SHELL, or cmd.exe on Windows)"),
    ];
    for definition in definitions {
        let _ = registry.register_setting(definition);
    }
}

use super::*;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "archipel-ide-settings-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[derive(serde::Deserialize)]
struct Spec {
    #[serde(rename = "advancedSettings")]
    advanced_settings: Vec<SpecAdvanced>,
}

#[derive(serde::Deserialize)]
struct SpecAdvanced {
    id: String,
    default: String,
    group: Option<String>,
    title: Option<String>,
}

fn spec() -> Spec {
    serde_json::from_str(include_str!("../../../docs/spec/data/settings.json")).unwrap()
}

#[test]
fn tree_follows_the_reference_dialog() {
    let registry = SettingsRegistry::intellij();
    let top: Vec<_> = registry
        .children(None)
        .iter()
        .map(|page| page.label.as_str())
        .collect();
    assert_eq!(
        top,
        [
            "Appearance & Behavior",
            "Keymap",
            "Editor",
            "Plugins",
            "Version Control",
            "Build, Execution, Deployment",
            "Languages & Frameworks",
            "Tools",
            "Backup and Sync",
            "Advanced Settings",
        ]
    );
    let appearance: Vec<_> = registry
        .children(Some("appearance"))
        .iter()
        .map(|page| page.label.as_str())
        .collect();
    // Order of 08-settings.png.
    assert_eq!(
        &appearance[..7],
        [
            "Appearance",
            "Menus and Toolbars",
            "System Settings",
            "File Colors",
            "Scopes",
            "Notifications",
            "Data Editor and Viewer",
        ]
    );
    let editor: Vec<_> = registry
        .children(Some("editor"))
        .iter()
        .map(|page| page.id.as_str())
        .collect();
    assert_eq!(
        &editor[..4],
        [
            "preferences.editor",
            "preferences.editor.code.editing",
            "AppEditorFontConfigurable",
            "reference.settingsdialog.IDE.editor.colors",
        ]
    );
    assert_eq!(
        registry
            .page("reference.settingsdialog.project.gradle")
            .map(|p| p.level),
        Some(Scope::Project)
    );
    assert_eq!(
        registry.breadcrumbs("reference.settingsdialog.project.gradle"),
        ["Build, Execution, Deployment", "Build Tools", "Gradle"]
    );
    assert_eq!(
        registry.breadcrumbs("preferences.lookFeel"),
        ["Appearance & Behavior", "Appearance"]
    );
}

#[test]
fn advanced_settings_defaults_equal_the_spec() {
    let registry = SettingsRegistry::intellij();
    let store = SettingsStore::default();
    let spec = spec();
    assert_eq!(spec.advanced_settings.len(), 124);
    assert_eq!(registry.page_settings(ADVANCED_SETTINGS_PAGE).len(), 124);
    for entry in &spec.advanced_settings {
        let def = registry
            .setting(&entry.id)
            .expect("advanced setting registered");
        assert_eq!(def.page, ADVANCED_SETTINGS_PAGE);
        assert_eq!(def.group, entry.group);
        assert_eq!(Some(&def.label), entry.title.as_ref().or(Some(&entry.id)));
        let value = store.get(&registry, &entry.id).unwrap();
        let rendered = match value {
            SettingValue::Bool(value) => value.to_string(),
            SettingValue::Int(value) => value.to_string(),
            SettingValue::Text(value) => value,
            other => panic!("unexpected default {other:?}"),
        };
        assert_eq!(rendered, entry.default, "default of {}", entry.id);
    }
    // Shown grouped and sorted like the spec table.
    let first = &registry.page_settings(ADVANCED_SETTINGS_PAGE)[0];
    assert_eq!(first.id, "ide.ui.theme.custom.islands");
}

#[test]
fn defaults_and_validation() {
    let registry = SettingsRegistry::intellij();
    let mut store = SettingsStore::default();
    assert_eq!(store.get_int(&registry, ids::EDITOR_FONT_SIZE), Some(13));
    assert_eq!(
        store.get_text(&registry, ids::THEME).as_deref(),
        Some("Islands Dark")
    );
    assert!(matches!(
        store.set(&registry, ids::EDITOR_FONT_SIZE, SettingValue::Int(500)),
        Err(SettingsError::InvalidValue { .. })
    ));
    assert!(matches!(
        store.set(&registry, ids::EDITOR_FONT_SIZE, SettingValue::Bool(true)),
        Err(SettingsError::InvalidValue { .. })
    ));
    assert!(matches!(
        store.set(
            &registry,
            ids::THEME,
            SettingValue::Text("Solarized".into())
        ),
        Err(SettingsError::InvalidValue { .. })
    ));
    assert!(matches!(
        store.set(&registry, "no.such.setting", SettingValue::Bool(true)),
        Err(SettingsError::UnknownSetting(_))
    ));
    // Integers are accepted for floats.
    store
        .set(&registry, ids::EDITOR_LINE_HEIGHT, SettingValue::Int(2))
        .unwrap();
    assert_eq!(
        store.get_float(&registry, ids::EDITOR_LINE_HEIGHT),
        Some(2.0)
    );
    // Setting the default removes the override.
    store
        .set(&registry, ids::EDITOR_FONT_SIZE, SettingValue::Int(13))
        .unwrap();
    assert!(!store.is_overridden(&registry, ids::EDITOR_FONT_SIZE));
}

#[test]
fn registration_is_an_extension_point() {
    let mut registry = SettingsRegistry::intellij();
    registry
        .register_page(PageDef::new("archipel.rust", Some("language"), "Rust"))
        .unwrap();
    registry
        .register_setting(
            SettingDef::new(
                "rust.analyzerPath",
                "archipel.rust",
                "rust-analyzer executable",
                SettingKind::Path,
                SettingValue::Text(String::new()),
            )
            .scope(Scope::Project),
        )
        .unwrap();
    assert!(
        registry
            .children(Some("language"))
            .iter()
            .any(|page| page.id == "archipel.rust")
    );
    assert!(matches!(
        registry.register_page(PageDef::new("archipel.rust", None, "Again")),
        Err(SettingsError::Duplicate { .. })
    ));
    assert!(matches!(
        registry.register_page(PageDef::new("x", Some("missing.parent"), "X")),
        Err(SettingsError::UnknownPage(_))
    ));
    assert!(matches!(
        registry.register_setting(SettingDef::new(
            "bad.default",
            "archipel.rust",
            "Bad",
            SettingKind::Int { min: 0, max: 1 },
            SettingValue::Int(5),
        )),
        Err(SettingsError::InvalidValue { .. })
    ));
}

#[test]
fn project_layer_overrides_application_layer() {
    let registry = SettingsRegistry::intellij();
    let mut store = SettingsStore::default();
    // Without a project, project-scoped settings go to the application layer.
    store
        .set(
            &registry,
            ids::TERMINAL_SHELL,
            SettingValue::Text("/bin/zsh".into()),
        )
        .unwrap();
    let root = temp_dir("layers");
    store.load_project(&root).unwrap();
    assert_eq!(
        store.get_text(&registry, ids::TERMINAL_SHELL).as_deref(),
        Some("/bin/zsh")
    );
    store
        .set(
            &registry,
            ids::TERMINAL_SHELL,
            SettingValue::Text("/bin/fish".into()),
        )
        .unwrap();
    assert_eq!(
        store.get_text(&registry, ids::TERMINAL_SHELL).as_deref(),
        Some("/bin/fish")
    );
    assert!(store.to_toml(Scope::Project).contains("/bin/fish"));
    assert!(store.to_toml(Scope::Application).contains("/bin/zsh"));
    store.close_project();
    assert_eq!(
        store.get_text(&registry, ids::TERMINAL_SHELL).as_deref(),
        Some("/bin/zsh")
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn projects_from_before_the_rename_keep_working_and_migrate_on_save() {
    let registry = SettingsRegistry::intellij();
    let root = temp_dir("legacy-project");
    std::fs::create_dir_all(root.join(".jetthing")).unwrap();
    std::fs::write(
        root.join(".jetthing").join("settings.toml"),
        "[settings]\n\"gradle.buildRunUsing\" = \"jetthing\"\n\"terminal.shellPath\" = \"/bin/fish\"\n",
    )
    .unwrap();
    let mut store = SettingsStore::default();
    store.load_project(&root).unwrap();
    assert_eq!(
        store.get_text(&registry, ids::GRADLE_BUILD_RUN).as_deref(),
        Some("archipel-ide"),
        "the old option id is translated"
    );
    assert_eq!(
        store.get_text(&registry, ids::TERMINAL_SHELL).as_deref(),
        Some("/bin/fish")
    );
    assert!(!root.join(".archipel").exists(), "reading does not migrate");
    store.save().unwrap();
    let migrated = std::fs::read_to_string(root.join(".archipel").join("settings.toml")).unwrap();
    assert!(migrated.contains("/bin/fish") && migrated.contains("archipel-ide"));
    assert!(
        root.join(".jetthing").join("settings.toml").is_file(),
        "old file is left"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn persistence_round_trip_keeps_unknown_keys() {
    let registry = SettingsRegistry::intellij();
    let config = temp_dir("roundtrip");
    std::fs::write(
        config.join("settings.toml"),
        "[settings]\n\"plugin.from.elsewhere\" = \"kept\"\n",
    )
    .unwrap();
    let mut store = SettingsStore::default();
    store.load_application(&config).unwrap();
    store
        .set(&registry, ids::EDITOR_FONT_SIZE, SettingValue::Int(16))
        .unwrap();
    store
        .set(&registry, ids::EDITOR_LINE_HEIGHT, SettingValue::Float(1.5))
        .unwrap();
    store
        .set(
            &registry,
            ids::THEME,
            SettingValue::Text("Islands Light".into()),
        )
        .unwrap();
    store.save().unwrap();
    let mut reloaded = SettingsStore::default();
    reloaded.load_application(&config).unwrap();
    assert_eq!(reloaded.get_int(&registry, ids::EDITOR_FONT_SIZE), Some(16));
    assert_eq!(
        reloaded.get_float(&registry, ids::EDITOR_LINE_HEIGHT),
        Some(1.5)
    );
    assert_eq!(
        reloaded.get_text(&registry, ids::THEME).as_deref(),
        Some("Islands Light")
    );
    assert!(
        reloaded
            .to_toml(Scope::Application)
            .contains("plugin.from.elsewhere")
    );
    let _ = std::fs::remove_dir_all(config);
}

#[test]
fn malformed_files_are_reported_and_never_overwritten() {
    let registry = SettingsRegistry::intellij();
    let config = temp_dir("malformed");
    let path = config.join("settings.toml");
    std::fs::write(&path, "[settings\nbroken").unwrap();
    let mut store = SettingsStore::default();
    assert!(matches!(
        store.load_application(&config),
        Err(SettingsError::Read { .. })
    ));
    store
        .set(&registry, ids::EDITOR_FONT_SIZE, SettingValue::Int(20))
        .unwrap();
    store.save().unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "[settings\nbroken");
    // Values that no longer validate fall back to the default.
    std::fs::write(&path, "[settings]\n\"editor.font.size\" = 9000\n").unwrap();
    store.load_application(&config).unwrap();
    assert_eq!(store.get_int(&registry, ids::EDITOR_FONT_SIZE), Some(13));
    let _ = std::fs::remove_dir_all(config);
}

#[test]
fn search_filters_pages_and_settings() {
    let registry = SettingsRegistry::intellij();
    let result = registry.search("smooth");
    assert!(result.matching_settings.contains(ids::SMOOTH_SCROLLING));
    assert!(result.matching_pages.contains("preferences.lookFeel"));
    assert!(result.visible_pages.contains("appearance"));
    assert!(!result.visible_pages.contains("editor"));

    let gradle = registry.search("gradle");
    assert!(
        gradle
            .matching_pages
            .contains("reference.settingsdialog.project.gradle")
    );
    assert!(gradle.visible_pages.contains("build.tools"));
    // Advanced Settings entries are searchable by group too.
    assert!(gradle.matching_settings.contains("gradle.download.sources"));

    let multi = registry.search("font SIZE");
    assert!(multi.matching_settings.contains(ids::EDITOR_FONT_SIZE));
    assert!(!multi.matching_settings.contains(ids::EDITOR_LIGATURES));

    // Enum option labels are searchable.
    assert!(
        registry
            .search("islands light")
            .matching_settings
            .contains(ids::THEME)
    );

    let empty = registry.search("   ");
    assert!(empty.matching_settings.is_empty());
    assert_eq!(
        empty.visible_pages.len(),
        registry.children(None).len() + {
            let mut count = 0;
            let mut stack: Vec<_> = registry
                .children(None)
                .iter()
                .map(|p| p.id.clone())
                .collect();
            while let Some(id) = stack.pop() {
                for child in registry.children(Some(&id)) {
                    count += 1;
                    stack.push(child.id.clone());
                }
            }
            count
        }
    );
    assert!(registry.search("zzzz-no-match").visible_pages.is_empty());
}

#[test]
fn draft_apply_reset_and_cancel() {
    let registry = SettingsRegistry::intellij();
    let mut store = SettingsStore::default();
    let mut draft = SettingsDraft::default();
    assert!(!draft.is_modified());
    draft
        .set(
            &registry,
            &store,
            ids::EDITOR_FONT_SIZE,
            SettingValue::Int(18),
        )
        .unwrap();
    draft
        .set(
            &registry,
            &store,
            ids::SMOOTH_SCROLLING,
            SettingValue::Bool(false),
        )
        .unwrap();
    assert!(draft.is_modified());
    assert_eq!(
        draft
            .value(&registry, &store, ids::EDITOR_FONT_SIZE)
            .unwrap(),
        SettingValue::Int(18)
    );
    // The store is untouched until Apply.
    assert_eq!(store.get_int(&registry, ids::EDITOR_FONT_SIZE), Some(13));
    let pages = draft.modified_pages(&registry);
    assert!(pages.contains("AppEditorFontConfigurable"));
    assert!(pages.contains("editor"));
    assert!(pages.contains("preferences.lookFeel"));
    // Reset link drops one page's edits.
    draft.reset_page(&registry, "preferences.lookFeel");
    assert!(!draft.is_setting_modified(ids::SMOOTH_SCROLLING));
    assert!(draft.is_setting_modified(ids::EDITOR_FONT_SIZE));
    // Editing back to the stored value clears the edit.
    draft
        .set(
            &registry,
            &store,
            ids::EDITOR_FONT_SIZE,
            SettingValue::Int(13),
        )
        .unwrap();
    assert!(!draft.is_modified());
    // Invalid edits are rejected.
    assert!(
        draft
            .set(
                &registry,
                &store,
                ids::EDITOR_FONT_SIZE,
                SettingValue::Int(1)
            )
            .is_err()
    );
    draft
        .set(
            &registry,
            &store,
            ids::EDITOR_FONT_SIZE,
            SettingValue::Int(15),
        )
        .unwrap();
    let changed = draft.apply(&registry, &mut store).unwrap();
    assert_eq!(changed, [ids::EDITOR_FONT_SIZE]);
    assert!(!draft.is_modified());
    assert_eq!(store.get_int(&registry, ids::EDITOR_FONT_SIZE), Some(15));
    // Restore defaults is a pending edit too.
    draft
        .restore_defaults(&registry, &store, "AppEditorFontConfigurable")
        .unwrap();
    assert!(draft.is_setting_modified(ids::EDITOR_FONT_SIZE));
    assert_eq!(store.get_int(&registry, ids::EDITOR_FONT_SIZE), Some(15));
}

#[test]
fn values_convert_from_json() {
    assert_eq!(
        SettingValue::from_json(&serde_json::json!(13)),
        Some(SettingValue::Int(13))
    );
    assert_eq!(
        SettingValue::from_json(&serde_json::json!(1.5)),
        Some(SettingValue::Float(1.5))
    );
    assert_eq!(
        SettingValue::from_json(&serde_json::json!(["a"])),
        Some(SettingValue::List(vec!["a".into()]))
    );
    assert_eq!(SettingValue::from_json(&serde_json::json!({"a": 1})), None);
    assert_eq!(SettingValue::Int(3).to_json(), serde_json::json!(3));
}

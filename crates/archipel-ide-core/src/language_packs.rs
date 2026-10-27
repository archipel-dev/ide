//! Language packs in the app (`archipel-ide-langpacks`, `docs/architecture/language-packs.md`).
//!
//! The IDE installs the built-in packs when the platform starts. What a pack registers on the
//! extension points reaches the rest of the app through this module: the analysis registry
//! and the language server registry are rebuilt when the extensions change
//! ([`IdeState::sync_language_packs`]), the settings and the per-frame work of every
//! [`LanguageActivity`] are driven generically ([`IdeState::poll_language_activities`]), and the
//! import chooser commands exist per [`ImportFixProvider`]. Nothing here names a language.

use crate::IdeState;
use crate::services::Settings;
use archipel_ide_analysis::langext::{
    self, ChooserPopup, IMPORT_FIX_PROVIDER, ImportFixProvider, on_language_extensions_changed,
};
use archipel_ide_analysis::{Change, ChoiceRow, ModCommand};
use archipel_ide_langpacks::{
    ActivityCx, LANGUAGE_ACTIVITY, LanguageActivity, LanguagePacks, OpenDocuments,
    PackSettingDefault, ProjectInputs, SettingValues, TextSource,
};
use archipel_ide_lsp::LANGUAGE_SERVER_PROVIDER;
use archipel_ide_platform::{Application, ComponentManager, ExtensionRegistry};
use archipel_ide_settings::{SettingDef, SettingKind, SettingValue};
use archipel_ide_toolchains::providers::{
    ProjectRequirement, TOOLCHAIN_REQUIREMENT_PROVIDER, requirements_from,
};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set by the extension registry whenever a language extension point changes (a pack was
/// installed or removed); [`IdeState::sync_language_packs`] consumes it.
#[derive(Clone, Default)]
pub(crate) struct PacksDirty(Arc<AtomicBool>);

impl PacksDirty {
    pub(crate) fn take(&self) -> bool {
        self.0.swap(false, Ordering::SeqCst)
    }
}

/// Installs the built-in packs and watches the extension points for changes.
pub(crate) fn install_language_packs(application: &Application) -> PacksDirty {
    let dirty = PacksDirty::default();
    let extensions = application.extensions();
    let node = application.disposable();
    let flag = dirty.0.clone();
    on_language_extensions_changed(extensions, node, {
        let flag = flag.clone();
        move || flag.store(true, Ordering::SeqCst)
    });
    // The points that are not language bound.
    let _ = extensions.add_change_listener(LANGUAGE_SERVER_PROVIDER, node, {
        let flag = flag.clone();
        move || flag.store(true, Ordering::SeqCst)
    });
    let _ = extensions.add_change_listener(TOOLCHAIN_REQUIREMENT_PROVIDER, node, {
        let flag = flag.clone();
        move || flag.store(true, Ordering::SeqCst)
    });
    let _ = extensions.add_change_listener(LANGUAGE_ACTIVITY, node, {
        let flag = flag.clone();
        move || flag.store(true, Ordering::SeqCst)
    });
    let _ = extensions.add_change_listener(IMPORT_FIX_PROVIDER, node, move || {
        flag.store(true, Ordering::SeqCst)
    });
    let packs = application.service::<LanguagePacks>();
    for error in packs.install_builtin() {
        // A built-in pack that cannot register is a programming error; the IDE still starts.
        debug_assert!(false, "{error}");
        eprintln!("archipel: {error}");
    }
    dirty
}

impl SettingValues for Settings {
    fn bool(&self, key: &str) -> Option<bool> {
        Settings::bool(self, key)
    }

    fn text(&self, key: &str) -> Option<String> {
        Settings::text(self, key)
    }
}

struct Documents<'a>(&'a [crate::Document]);

impl OpenDocuments for Documents<'_> {
    fn for_each(&self, visit: &mut dyn FnMut(&Path, u64, TextSource<'_>)) {
        for document in self.0 {
            visit(&document.path, document.text.revision(), &|| {
                document.text.text().into()
            });
        }
    }
}

/// The requirements of the project at `root` from the installed packs.
pub(crate) fn detect_requirements(
    extensions: &ExtensionRegistry,
    root: &Path,
) -> Vec<ProjectRequirement> {
    requirements_from(
        &extensions.extension_list(TOOLCHAIN_REQUIREMENT_PROVIDER),
        root,
    )
}

impl IdeState {
    /// The installed language packs (application service).
    pub fn language_packs(&self) -> Arc<LanguagePacks> {
        self.application().service()
    }

    /// Applies pack changes to what caches them: the analysis registry (passes, inspections,
    /// folding, ... of every language), the language servers and the toolchain requirements.
    /// Cheap when nothing changed; called at the start of every poll.
    pub(crate) fn sync_language_packs(&mut self) {
        if !self.platform.packs_dirty().take() {
            return;
        }
        let extensions = self.application().extensions().clone();
        self.analysis.apply_extensions(&extensions);
        self.lsp
            .set_contributed_providers(extensions.extension_list(LANGUAGE_SERVER_PROVIDER));
        if let (Some(root), Some(requirements)) = (
            self.project.as_ref().map(|p| p.root().to_path_buf()),
            self.toolchain_requirements(),
        ) {
            let detected = detect_requirements(&extensions, &root);
            requirements
                .write()
                .reload(detected, &self.toolchains().read());
        }
    }

    /// The activities of the installed packs, in loading order.
    pub fn language_activities(&self) -> Vec<Arc<dyn LanguageActivity>> {
        self.application()
            .extensions()
            .extension_list(LANGUAGE_ACTIVITY)
    }

    /// `(pack id, state)` of every installed pack's activity, for status displays.
    pub fn language_status(&self) -> Vec<(&'static str, Option<String>)> {
        self.language_activities()
            .iter()
            .map(|activity| (activity.id(), activity.status()))
            .collect()
    }

    /// Adds the settings of the installed packs to the inspections page.
    fn ensure_pack_settings(&mut self) {
        let activities = self.language_activities();
        if activities.is_empty() {
            return;
        }
        let settings = self.settings();
        let mut settings = settings.write();
        let registry = &mut settings.registry;
        if registry.page(crate::INSPECTIONS_PAGE).is_none() {
            return;
        }
        for activity in activities {
            for setting in activity.settings() {
                if registry.setting(setting.key).is_some() {
                    continue;
                }
                let (kind, default) = match setting.default {
                    PackSettingDefault::Bool(value) => {
                        (SettingKind::Bool, SettingValue::Bool(value))
                    }
                    PackSettingDefault::Text(value) => {
                        (SettingKind::Text, SettingValue::Text(value.to_owned()))
                    }
                };
                let mut def = SettingDef::new(
                    setting.key,
                    crate::INSPECTIONS_PAGE,
                    setting.title,
                    kind,
                    default,
                )
                .group(setting.group);
                if let Some(description) = setting.description {
                    def = def.description(description);
                }
                let _ = registry.register_setting(def);
            }
        }
    }

    /// Per-frame driver of every language activity: settings, project model, buffers. Returns
    /// true when the daemon must run its passes again.
    pub(crate) fn poll_language_activities(&mut self) -> bool {
        self.sync_language_packs();
        self.ensure_pack_settings();
        let activities = self.language_activities();
        if activities.is_empty() {
            return false;
        }
        let project = self.project.as_ref().map(|project| {
            let root = project.root().to_path_buf();
            let (model, model_loading) = match self.project_model() {
                Some(service) => {
                    let state = service.read();
                    (state.model.clone(), state.is_busy())
                }
                None => (None, false),
            };
            let index_cache_dir = self.analysis.index_cache_dir.clone().map(|dir| {
                dir.join("index").join(format!(
                    "{:016x}",
                    archipel_ide_lsp::registry::stable_hash(&root.to_string_lossy())
                ))
            });
            ProjectInputs {
                root,
                model,
                model_loading,
                index_cache_dir,
            }
        });
        let mut changed = false;
        let mut native_wins = Vec::new();
        {
            let settings = self.settings();
            let settings = settings.read();
            let documents = Documents(&self.documents);
            let cx = ActivityCx {
                settings: &*settings,
                project: project.as_ref(),
                documents: &documents,
            };
            for activity in &activities {
                let outcome = activity.poll(&cx);
                changed |= outcome.changed;
                native_wins.extend(outcome.native_wins);
            }
        }
        if !native_wins.is_empty()
            && let Some(daemon) = self.daemon_service()
        {
            let mut shared = daemon.shared().native_wins.write();
            for (language, wins) in native_wins {
                if wins {
                    shared.insert(language.into());
                } else {
                    shared.remove(language);
                }
            }
        }
        if changed {
            self.analysis.host.apply(Change::SettingsChanged);
        }
        changed
    }

    /// The file watcher reported these project-relative paths.
    pub(crate) fn language_paths_changed(&self, paths: &[std::path::PathBuf]) {
        for activity in self.language_activities() {
            activity.paths_changed(paths);
        }
    }

    // --- import fixes -------------------------------------------------------------------

    fn import_providers(&self) -> Vec<Arc<dyn ImportFixProvider>> {
        self.application()
            .extensions()
            .extension_list(IMPORT_FIX_PROVIDER)
    }

    /// The provider that owns the popup for the active editor: the one serving its language,
    /// else the first.
    fn import_provider_for_active_document(&self) -> Option<Arc<dyn ImportFixProvider>> {
        let providers = self.import_providers();
        let language = self
            .active_document
            .and_then(|index| self.documents.get(index))
            .and_then(|document| {
                langext::language_for_path(self.application().extensions(), &document.path)
            });
        language
            .and_then(|language| {
                providers
                    .iter()
                    .find(|p| p.languages().contains(&language.as_str()))
                    .cloned()
            })
            .or_else(|| providers.first().cloned())
    }

    /// Opens the chooser of an import fix (`ModCommand::ChooseFrom`).
    pub(crate) fn open_import_chooser(
        &self,
        title: String,
        rows: Vec<ChoiceRow>,
    ) -> Result<(), String> {
        let provider = self
            .import_provider_for_active_document()
            .ok_or("no chooser available")?;
        provider.open_chooser(title, rows);
        Ok(())
    }

    /// The open import chooser and the id of the provider that owns it.
    pub fn import_chooser(&self) -> Option<(&'static str, ChooserPopup)> {
        self.import_providers()
            .iter()
            .find_map(|provider| Some((provider.id(), provider.chooser()?)))
    }

    fn import_provider(&self, id: &str) -> Option<Arc<dyn ImportFixProvider>> {
        self.application()
            .extensions()
            .find_by_id(IMPORT_FIX_PROVIDER, id)
    }

    /// The command that applies row `index` of the open chooser of provider `id`.
    pub fn choose_import(&self, id: &str, index: usize) -> Option<ModCommand> {
        self.import_provider(id)?.choose(index)
    }

    /// Closes the chooser of provider `id`.
    pub fn close_import_chooser(&self, id: &str) {
        if let Some(provider) = self.import_provider(id) {
            provider.close_chooser();
        }
    }
}

fn chooser_json(chooser: Option<ChooserPopup>) -> Value {
    json!({
        "open": chooser.is_some(),
        "title": chooser.as_ref().map(|c| c.title.clone()),
        "rows": chooser.map(|c| c.rows.iter().map(|r| match &r.detail {
            Some(d) => format!("{} {d}", r.label),
            None => r.label.clone(),
        }).collect::<Vec<_>>()),
    })
}

/// Registers `<prefix>.chooser`, `<prefix>.chooseImport` and `<prefix>.cancelChooser` for every
/// pack with an import fix provider.
pub(crate) fn register_commands(registry: &mut archipel_ide_command::CommandRegistry<IdeState>) {
    use archipel_ide_command::Command;
    for prefix in archipel_ide_langpacks::builtin_import_command_prefixes() {
        registry
            .register(Command::new(
                crate::spec(
                    &format!("{prefix}.chooser"),
                    "Import Chooser",
                    "Code",
                    None,
                    "{}",
                ),
                |_| true,
                move |state: &mut IdeState, _| {
                    let chooser = state
                        .import_provider(prefix)
                        .and_then(|provider| provider.chooser());
                    Ok(chooser_json(chooser))
                },
            ))
            .expect("unique built-in id");
        registry
            .register(Command::new(
                crate::spec(
                    &format!("{prefix}.chooseImport"),
                    "Choose Import",
                    "Code",
                    None,
                    r#"{"index":0}"#,
                ),
                move |state: &IdeState| {
                    state
                        .import_provider(prefix)
                        .is_some_and(|provider| provider.chooser().is_some())
                },
                move |state: &mut IdeState, args: Value| {
                    let index = crate::usize_param(&args, "index").unwrap_or(0);
                    let command = state.choose_import(prefix, index).ok_or_else(|| {
                        archipel_ide_command::CommandError::Failed("no such choice".into())
                    })?;
                    state
                        .execute_mod_command(&command)
                        .map(|m| json!({"message": m}))
                        .map_err(archipel_ide_command::CommandError::Failed)
                },
            ))
            .expect("unique built-in id");
        registry
            .register(Command::new(
                crate::spec(
                    &format!("{prefix}.cancelChooser"),
                    "Cancel Import Chooser",
                    "Code",
                    None,
                    "{}",
                ),
                |_| true,
                move |state: &mut IdeState, _| {
                    state.close_import_chooser(prefix);
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
}

//! What a language pack does while a project is open: settings it adds, the per-frame sync
//! with the project model and the open buffers, file watcher notifications and a status line.
//!
//! IntelliJ counterpart: a `ProjectActivity` plus the `Configurable` of the language. The
//! host (`archipel-ide-core`) drives every registered [`LanguageActivity`] once per frame
//! without knowing which language it serves (PLAT-LANG-012, PLAT-LANG-013).

use archipel_ide_platform::ExtensionPointName;
use archipel_ide_projectmodel::ProjectModel;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Read access to the stored settings (the host implements it over its settings service).
pub trait SettingValues {
    fn bool(&self, key: &str) -> Option<bool>;
    fn text(&self, key: &str) -> Option<String>;
}

/// The default of a [`PackSetting`] (and so its kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackSettingDefault {
    Bool(bool),
    Text(&'static str),
}

/// A setting a pack adds to Settings | Editor | Inspections.
#[derive(Debug, Clone, Copy)]
pub struct PackSetting {
    pub key: &'static str,
    pub title: &'static str,
    /// The group heading on the page (the language name).
    pub group: &'static str,
    pub description: Option<&'static str>,
    pub default: PackSettingDefault,
}

/// The text of an open buffer, read only when the pack needs it.
pub type TextSource<'a> = &'a dyn Fn() -> Arc<str>;

/// The open buffers, by path, revision and lazily read text.
pub trait OpenDocuments {
    fn for_each(&self, visit: &mut dyn FnMut(&Path, u64, TextSource<'_>));
}

/// The open project as far as language support needs to know.
pub struct ProjectInputs {
    pub root: PathBuf,
    /// The imported project model, once there is one.
    pub model: Option<Arc<ProjectModel>>,
    /// The model is still being (re)imported.
    pub model_loading: bool,
    /// Warnings and errors of the last import (unresolved dependencies, failed resolvers): why
    /// the classpath may be incomplete.
    pub import_notes: Vec<String>,
    /// Directory language indexes may persist into (`None`: memory only), already specific
    /// to this project.
    pub index_cache_dir: Option<PathBuf>,
}

/// What one frame's poll sees.
pub struct ActivityCx<'a> {
    pub settings: &'a dyn SettingValues,
    pub project: Option<&'a ProjectInputs>,
    pub documents: &'a dyn OpenDocuments,
}

/// What a poll asks the host to do.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ActivityOutcome {
    /// Answers changed: the daemon must analyze again.
    pub changed: bool,
    /// `(language id, wins)`: the native analysis of that language does (not) take precedence
    /// over the language server's duplicates.
    pub native_wins: Option<(&'static str, bool)>,
}

/// Per-project work of a language pack, called by the host once per frame.
pub trait LanguageActivity: Send + Sync {
    /// The pack id (shown in the status data under `languages.<id>`).
    fn id(&self) -> &'static str;

    /// Settings to add to the inspections page. Called whenever the page is missing them.
    fn settings(&self) -> Vec<PackSetting> {
        Vec::new()
    }

    /// Brings the pack's state in line with settings, project model and buffers.
    fn poll(&self, cx: &ActivityCx<'_>) -> ActivityOutcome;

    /// Files changed on disk (project-relative paths from the watcher).
    fn paths_changed(&self, _paths: &[PathBuf]) {}

    /// A short state name for status displays (`"Ready"`), `None` if the pack has no state.
    fn status(&self) -> Option<String> {
        None
    }

    /// A warning for the status bar (`"Classpath incomplete: ..."`), `None` when all is well.
    fn notice(&self) -> Option<String> {
        None
    }
}

/// Where packs contribute their [`LanguageActivity`] (`PLAT-LANG-012`).
pub const LANGUAGE_ACTIVITY: ExtensionPointName<dyn LanguageActivity> =
    ExtensionPointName::new("dev.archipel.language.activity");

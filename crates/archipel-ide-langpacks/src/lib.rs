//! Language packs: a language is a bundle of extensions registered at startup.
//!
//! A [`LanguagePack`] registers its parts on the extension points of the platform (see
//! `docs/architecture/language-packs.md` and spec `PLAT-LANG-*`): the language and its file
//! types, the parser definition, daemon passes, suppressors, intentions, the language server,
//! the toolchain requirements, the import fix provider and the per-project activity. The host
//! (`archipel-ide-core`, daemon, UI) only reads the extension points; it never names a
//! language. Removing a pack (`LanguagePacks::uninstall`) disposes its node in the disposer
//! tree and with it every contribution (PLAT-LANG-005).
//!
//! Registration is static: [`LanguagePacks::install_builtin`] runs at application start.

#![forbid(unsafe_code)]

pub mod activity;
pub mod packs;

pub use activity::{
    ActivityCx, ActivityOutcome, LANGUAGE_ACTIVITY, LanguageActivity, OpenDocuments, PackSetting,
    PackSettingDefault, ProjectInputs, SettingValues, TextSource,
};

use archipel_ide_analysis::langext::{
    self, ALL_POINTS, FILE_TYPE, FileType, IMPORT_FIX_PROVIDER, ImportFixProvider, LANGUAGE,
    Language, LanguagePoint, PARSER_DEFINITION, TreeSitterParser,
};
use archipel_ide_language::{LanguageId, Registration};
use archipel_ide_lsp::{LANGUAGE_SERVER_PROVIDER, LanguageServerProvider};
use archipel_ide_platform::{
    Application, ComponentManager, DisposableId, Disposer, ExtensionError, ExtensionPointName,
    ExtensionRegistry, LoadingOrder,
};
use archipel_ide_toolchains::providers::{
    FnRequirement, ProjectRequirement, TOOLCHAIN_REQUIREMENT_PROVIDER,
};
use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;

/// A bundle of language extensions (IntelliJ: the `plugin.xml` of a language plugin).
pub trait LanguagePack: Send + Sync {
    /// Stable id of the pack (`"java"`); also the id of its [`LanguageActivity`] status.
    fn id(&self) -> &'static str;

    fn display_name(&self) -> &'static str;

    /// Registers every extension of the pack. Called once per install.
    fn register(&self, cx: &mut PackContext<'_>);

    /// Command id prefixes of the import choosers of this pack: the host registers
    /// `<prefix>.chooser`, `<prefix>.chooseImport` and `<prefix>.cancelChooser` for them.
    fn import_command_prefixes(&self) -> &'static [&'static str] {
        &[]
    }
}

/// Why a pack could not be installed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PackError {
    #[error("language pack `{0}` is already installed")]
    AlreadyInstalled(&'static str),
    #[error("language pack `{pack}`: {source}")]
    Extension {
        pack: &'static str,
        source: ExtensionError,
    },
    #[error("language pack `{pack}` names the unknown tree-sitter language `{language}`")]
    UnknownLanguage {
        pack: &'static str,
        language: &'static str,
    },
}

/// What a pack registers its extensions through. Every extension is registered under the
/// pack's node of the disposer tree, so it leaves with the pack.
pub struct PackContext<'a> {
    extensions: &'a ExtensionRegistry,
    parent: DisposableId,
    pack: &'static str,
    error: Option<PackError>,
}

impl PackContext<'_> {
    fn record(&mut self, result: Result<(), ExtensionError>) {
        if let Err(source) = result
            && self.error.is_none()
        {
            self.error = Some(PackError::Extension {
                pack: self.pack,
                source,
            });
        }
    }

    /// Declares a language (PLAT-LANG-001).
    pub fn language(&mut self, language: Language) {
        let result = self.extensions.register(
            LANGUAGE,
            language.id,
            LoadingOrder::Anywhere,
            Arc::new(language),
            Some(self.parent),
        );
        self.record(result);
    }

    /// Declares a file type (PLAT-LANG-020/021) under the id of its language, or its name.
    pub fn file_type(&mut self, file_type: FileType) {
        let id = file_type.language.unwrap_or(file_type.name);
        let result = self.extensions.register(
            FILE_TYPE,
            id,
            LoadingOrder::Anywhere,
            Arc::new(file_type),
            Some(self.parent),
        );
        self.record(result);
    }

    /// A tree-sitter language of the grammar table in one step: the language, its file type (if
    /// it has extensions or file names) and its parser definition (PLAT-LANG-001, -020, -030).
    /// The grammar table itself (`archipel_ide_language::LANGUAGES`) stays the data source.
    pub fn tree_sitter_language(
        &mut self,
        id: &'static str,
        configure: impl FnOnce(Language) -> Language,
    ) {
        let Some(def) = archipel_ide_language::LANGUAGES.iter().find(|l| l.id == id) else {
            if self.error.is_none() {
                self.error = Some(PackError::UnknownLanguage {
                    pack: self.pack,
                    language: id,
                });
            }
            return;
        };
        self.language(configure(Language::of(def)));
        if !def.extensions.is_empty() || !def.file_names.is_empty() {
            self.file_type(FileType::of(def));
        }
        self.language_extension(
            PARSER_DEFINITION,
            Registration::new("tree-sitter", LanguageId(def.id)).dumb_aware(true),
            Arc::new(TreeSitterParser(def)),
        );
    }

    /// Registers a language-bound extension (PLAT-LANG-010..014).
    pub fn language_extension<T: ?Sized + Send + Sync + 'static>(
        &mut self,
        point: LanguagePoint<T>,
        registration: Registration,
        extension: Arc<T>,
    ) {
        let result = langext::register_language_extension(
            self.extensions,
            point,
            registration,
            extension,
            Some(self.parent),
        );
        self.record(result);
    }

    /// Registers an extension on a point that is not language bound.
    pub fn extension<T: ?Sized + Send + Sync + 'static>(
        &mut self,
        point: ExtensionPointName<T>,
        id: &'static str,
        order: LoadingOrder,
        extension: Arc<T>,
    ) {
        let result = self
            .extensions
            .register(point, id, order, extension, Some(self.parent));
        self.record(result);
    }

    /// The language server of a language (`LanguageServerProvider`, mapping files to a server).
    pub fn language_server(&mut self, provider: Arc<dyn LanguageServerProvider>) {
        let id = provider.spec().id;
        self.extension(
            LANGUAGE_SERVER_PROVIDER,
            id,
            LoadingOrder::Anywhere,
            provider,
        );
    }

    /// What projects of the language need from the toolchains. `listed_after` names the
    /// language whose requirement this one follows in the setup order.
    pub fn toolchain_requirement(
        &mut self,
        id: &'static str,
        detect: fn(&Path) -> Option<ProjectRequirement>,
        listed_after: Option<&'static str>,
    ) {
        self.extension(
            TOOLCHAIN_REQUIREMENT_PROVIDER,
            id,
            LoadingOrder::Anywhere,
            Arc::new(FnRequirement {
                detect,
                listed_after,
            }),
        );
    }

    /// The import fix provider (chooser state) of the pack.
    pub fn import_fix_provider(&mut self, provider: Arc<dyn ImportFixProvider>) {
        let id = provider.id();
        self.extension(IMPORT_FIX_PROVIDER, id, LoadingOrder::Anywhere, provider);
    }

    /// The native completion source of the pack (class names with import on insert).
    pub fn completion_source(
        &mut self,
        source: Arc<dyn archipel_ide_analysis::langext::CompletionSource>,
    ) {
        let id = source.id();
        self.extension(
            archipel_ide_analysis::langext::COMPLETION_SOURCE,
            id,
            LoadingOrder::Anywhere,
            source,
        );
    }

    /// The per-project activity of the pack.
    pub fn activity(&mut self, activity: Arc<dyn LanguageActivity>) {
        let id = activity.id();
        self.extension(LANGUAGE_ACTIVITY, id, LoadingOrder::Anywhere, activity);
    }
}

struct Installed {
    id: &'static str,
    node: DisposableId,
    import_prefixes: &'static [&'static str],
}

/// The installed language packs of an application (an application service).
pub struct LanguagePacks {
    extensions: ExtensionRegistry,
    disposer: Disposer,
    app_node: DisposableId,
    installed: Mutex<Vec<Installed>>,
}

impl LanguagePacks {
    pub fn new(application: &Application) -> Self {
        Self {
            extensions: application.extensions().clone(),
            disposer: application.disposer().clone(),
            app_node: application.disposable(),
            installed: Mutex::new(Vec::new()),
        }
    }

    pub fn extensions(&self) -> &ExtensionRegistry {
        &self.extensions
    }

    /// Installs a pack: its extensions are registered under a new node of the disposer tree.
    /// On an error nothing of the pack stays registered.
    pub fn install(&self, pack: &dyn LanguagePack) -> Result<(), PackError> {
        let id = pack.id();
        let mut installed = self.installed.lock();
        if installed.iter().any(|existing| existing.id == id) {
            return Err(PackError::AlreadyInstalled(id));
        }
        let node = self
            .disposer
            .new_child(self.app_node, &format!("language pack {id}"))
            .map_err(|source| PackError::Extension {
                pack: id,
                source: ExtensionError::ParentDisposed(source),
            })?;
        let mut cx = PackContext {
            extensions: &self.extensions,
            parent: node,
            pack: id,
            error: None,
        };
        pack.register(&mut cx);
        if let Some(error) = cx.error {
            self.disposer.dispose(node);
            return Err(error);
        }
        installed.push(Installed {
            id,
            node,
            import_prefixes: pack.import_command_prefixes(),
        });
        Ok(())
    }

    /// Removes a pack and every extension it registered. Returns whether it was installed.
    pub fn uninstall(&self, id: &str) -> bool {
        let node = {
            let mut installed = self.installed.lock();
            let Some(index) = installed.iter().position(|existing| existing.id == id) else {
                return false;
            };
            installed.remove(index).node
        };
        self.disposer.dispose(node);
        true
    }

    pub fn is_installed(&self, id: &str) -> bool {
        self.installed.lock().iter().any(|p| p.id == id)
    }

    /// Ids of the installed packs, in install order.
    pub fn installed(&self) -> Vec<&'static str> {
        self.installed.lock().iter().map(|p| p.id).collect()
    }

    /// Installs [`packs::builtin_packs`]; returns the errors (none in a healthy build).
    pub fn install_builtin(&self) -> Vec<PackError> {
        packs::builtin_packs()
            .iter()
            .filter_map(|pack| self.install(pack.as_ref()).err())
            .collect()
    }

    /// The registrations of `language` on every language-bound point, with the point's name and
    /// spec ids: "which services does language X have" as data (spec 08 section 6; the
    /// conformance tests of PLAT-LANG-104 read it).
    pub fn services_of(&self, language: LanguageId) -> Vec<ServiceInfo> {
        let mut out = Vec::new();
        for point in ALL_POINTS {
            for registration in point.registrations(&self.extensions) {
                if registration.language == language {
                    out.push(ServiceInfo {
                        point: point.name(),
                        spec: point.spec(),
                        id: registration.id,
                        dumb_aware: registration.dumb_aware,
                    });
                }
            }
        }
        out
    }
}

/// One registration of a language, see [`LanguagePacks::services_of`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInfo {
    pub point: &'static str,
    pub spec: &'static str,
    pub id: &'static str,
    pub dumb_aware: bool,
}

/// The import command prefixes (`java`) of the packs installed in `packs`.
pub fn import_command_prefixes(packs: &LanguagePacks) -> Vec<&'static str> {
    packs
        .installed
        .lock()
        .iter()
        .flat_map(|p| p.import_prefixes.iter().copied())
        .collect()
}

/// The import command prefixes of the built-in packs, for registering their commands before
/// the packs are installed (the command registry is built once at start).
pub fn builtin_import_command_prefixes() -> Vec<&'static str> {
    packs::builtin_packs()
        .iter()
        .flat_map(|pack| pack.import_command_prefixes().iter().copied())
        .collect()
}

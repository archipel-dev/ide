//! Toolchain provisioning as services (IntelliJ: the SDK table plus a project's SDK needs).
//!
//! - [`ToolchainState`] is an **application service** ([`ToolchainManager`]): discovery, the
//!   user-level pins and the background installs, shared by every project.
//! - [`ToolchainRequirements`] is a **project service**: what the open project needs, which of
//!   that is missing, and which "missing toolchain" bars the user dismissed.
//!
//! Installs announce themselves on [`TOOLCHAIN_TOPIC`]; nothing in the toolchain code calls
//! the language servers or the build integration directly.

use archipel_ide_platform::{ServiceState, Topic};
use archipel_ide_toolchains::providers::ProjectRequirement;
use archipel_ide_toolchains::service::ToolchainService;
use archipel_ide_toolchains::{ToolchainSettings, ToolchainStore, Toolchains};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The application service that owns discovery, pins and installs.
pub type ToolchainManager = ServiceState<ToolchainState>;

/// The project service with the open project's toolchain needs.
pub type ToolchainRequirementsService = ServiceState<ToolchainRequirements>;

/// Events of the toolchain installs, on the application bus.
pub trait ToolchainListener: Send + Sync {
    /// A component finished installing.
    fn component_installed(&self, _component: &str, _version: &str) {}
    /// A component's install failed.
    fn component_failed(&self, _component: &str, _error: &str) {}
}

pub const TOOLCHAIN_TOPIC: Topic<dyn ToolchainListener> = Topic::new("ToolchainListener");

/// Provisioning state shared by all projects.
pub struct ToolchainState {
    pub toolchains: Toolchains,
    pub service: ToolchainService,
    /// `toolchains.toml` that `toolchain.usePath` writes (user level).
    pub user_settings: Option<PathBuf>,
    /// Problem loading `toolchains.toml`, shown instead of failing project open.
    pub settings_error: Option<String>,
    /// Install what an opened project is missing without asking. The app turns this on;
    /// it stays off in tests and headless runs so they never download.
    pub auto_setup: bool,
}

/// The "missing toolchain" editor notification for one language.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ToolchainBanner {
    pub language: String,
    /// Missing component ids (requirements included), in setup order.
    pub components: Vec<String>,
    /// "Java project: JDK, Gradle and the Java language server are missing".
    pub text: String,
    /// Whether a setup of these components is running.
    pub busy: bool,
}

impl Default for ToolchainState {
    fn default() -> Self {
        let store = ToolchainStore::new(archipel_ide_toolchains::default_root());
        store.clean();
        Self::new(
            Toolchains::new(archipel_ide_toolchains::providers::builtin_catalog(), store),
            ToolchainSettings::user_file(),
        )
    }
}

impl ToolchainState {
    pub fn new(toolchains: Toolchains, user_settings: Option<PathBuf>) -> Self {
        Self {
            toolchains,
            service: ToolchainService::default(),
            user_settings,
            settings_error: None,
            auto_setup: false,
        }
    }

    /// Reloads user and project `toolchains.toml` and the project's pins.
    pub fn reload(&mut self, project_root: Option<&Path>) {
        let project_file = project_root.map(ToolchainSettings::project_file);
        let settings =
            ToolchainSettings::load_from(self.user_settings.as_deref(), project_file.as_deref());
        self.settings_error = settings.as_ref().err().map(ToString::to_string);
        self.toolchains
            .apply_project(project_root, settings.unwrap_or_default());
    }

    /// Status-bar text and fraction for the first running install.
    pub fn progress(&self) -> Option<(String, Option<f32>)> {
        let job = self.service.jobs().first()?;
        // Component ids keep the status bar entry short (`jdtls`, `jdk`).
        let mut text = job.describe(&job.component);
        let more = self.service.jobs().len() - 1;
        if more > 0 {
            text.push_str(&format!(" (+{more})"));
        }
        Some((text, job.fraction()))
    }

    /// Component name in the bar: language servers read "the Java language server".
    fn banner_name(&self, id: &str, language: &str) -> String {
        let server = self.toolchains.catalog().get(id).is_some_and(|provider| {
            provider.spec().kind == archipel_ide_toolchains::ComponentKind::LanguageServer
        });
        if server {
            format!("the {language} language server")
        } else {
            self.component_name(id)
        }
    }

    pub fn component_name(&self, id: &str) -> String {
        self.toolchains
            .catalog()
            .get(id)
            .map_or(id.to_owned(), |p| p.spec().name.clone())
    }
}

/// What the open project needs from the toolchains.
#[derive(Default)]
pub struct ToolchainRequirements {
    /// What the project needs, by language (from marker files).
    pub requirements: Vec<ProjectRequirement>,
    /// Requirements with components that cannot be found, cached for the UI.
    missing: Vec<(ProjectRequirement, Vec<String>)>,
    /// Languages whose "missing toolchain" bar was dismissed this session.
    dismissed: BTreeSet<String>,
}

impl ToolchainRequirements {
    /// Sets the detected requirements of the project (from the language packs'
    /// `TOOLCHAIN_REQUIREMENT_PROVIDER` extensions) and recomputes which components are missing.
    pub fn reload(&mut self, requirements: Vec<ProjectRequirement>, tools: &ToolchainState) {
        self.requirements = requirements;
        self.refresh_missing(tools);
    }

    /// Recomputes which required components cannot be found.
    pub fn refresh_missing(&mut self, tools: &ToolchainState) {
        self.missing = self
            .requirements
            .iter()
            .filter_map(|requirement| {
                let missing = tools.toolchains.missing(requirement.components);
                // Setup order: dependencies (JDK) before what needs them (jdtls).
                let mut ordered: Vec<String> = requirement
                    .components
                    .iter()
                    .filter(|id| missing.iter().any(|m| m == *id))
                    .map(|id| (*id).to_owned())
                    .collect();
                let extra: Vec<String> = missing
                    .into_iter()
                    .filter(|id| !ordered.contains(id))
                    .collect();
                ordered.extend(extra);
                (!ordered.is_empty()).then(|| (requirement.clone(), ordered))
            })
            .collect();
    }

    /// The missing component ids of every language, in order, without duplicates.
    pub fn missing_components(&self) -> Vec<&str> {
        let mut components: Vec<&str> = Vec::new();
        for (_, missing) in &self.missing {
            for id in missing {
                if !components.contains(&id.as_str()) {
                    components.push(id);
                }
            }
        }
        components
    }

    /// Hides the bar of `language` until the next session.
    pub fn dismiss(&mut self, language: &str) {
        self.dismissed.insert(language.to_owned());
    }

    /// The bar to show above an editor for `path`, if its language lacks tools.
    pub fn banner_for(&self, tools: &ToolchainState, path: &Path) -> Option<ToolchainBanner> {
        let found = self.missing.iter().find(|(requirement, _)| {
            requirement.applies_to(path) && !self.dismissed.contains(requirement.language)
        })?;
        Self::banner(tools, found)
    }

    /// The bar shown while no editor is open: the first language of the project
    /// that lacks tools, so setup is offered as soon as the project opens.
    pub fn project_banner(&self, tools: &ToolchainState) -> Option<ToolchainBanner> {
        let found = self
            .missing
            .iter()
            .find(|(requirement, _)| !self.dismissed.contains(requirement.language))?;
        Self::banner(tools, found)
    }

    fn banner(
        tools: &ToolchainState,
        (requirement, components): &(ProjectRequirement, Vec<String>),
    ) -> Option<ToolchainBanner> {
        let names: Vec<String> = components
            .iter()
            .map(|id| tools.banner_name(id, requirement.language))
            .collect();
        let list = match names.as_slice() {
            [one] => one.clone(),
            [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
            [] => return None,
        };
        let verb = if names.len() == 1 { "is" } else { "are" };
        Some(ToolchainBanner {
            language: requirement.language.to_owned(),
            text: format!("{} project: {list} {verb} missing", requirement.language),
            busy: tools
                .service
                .jobs()
                .iter()
                .any(|job| components.contains(&job.component)),
            components: components.clone(),
        })
    }
}

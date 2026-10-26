//! Finding components and the resolver API other subsystems call.
//!
//! Lookup order for a component:
//!
//! 1. a path configured in `toolchains.toml` ("use existing installation");
//! 2. a managed install matching the project pin, when the project pins a version;
//! 3. a home variable such as `JAVA_HOME` or `ANDROID_HOME`;
//! 4. `PATH`;
//! 5. the newest managed install.
//!
//! System installs therefore win over managed ones unless the project pins a
//! version that a managed install provides (the version of a `PATH` binary is
//! not probed yet).

use crate::store::find_executable;
use crate::{
    Catalog, ComponentSpec, Installed, Pin, Platform, ProjectPins, Release, ToolVersion,
    ToolchainError, ToolchainSettings, ToolchainStore, VersionRequest, download::HttpClient,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "name")]
pub enum LocateSource {
    /// `path` in `toolchains.toml`.
    Configured,
    /// A home variable (`JAVA_HOME`).
    HomeEnv(String),
    /// Found on `PATH`.
    Path,
    /// Installed by Archipel IDE.
    Managed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Located {
    pub component: String,
    pub executable: PathBuf,
    /// Installation home (`JAVA_HOME`-style): the directory above `bin/`.
    pub home: PathBuf,
    /// Known for managed installs; `PATH` binaries are not probed.
    pub version: Option<ToolVersion>,
    pub source: LocateSource,
}

/// What Settings and notifications show for one component.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentStatus {
    pub spec: ComponentSpec,
    pub located: Option<Located>,
    pub installed: Vec<Installed>,
    pub pin: Option<Pin>,
}

/// Resolver facade: catalog + store + settings + project pins.
///
/// Build/run providers, the LSP registry, Android and debugger integrations ask
/// it for executables ([`Toolchains::executable`]) and process environments
/// ([`Toolchains::environment`]) instead of searching `PATH` themselves.
#[derive(Clone)]
pub struct Toolchains {
    catalog: Catalog,
    store: ToolchainStore,
    platform: Platform,
    settings: ToolchainSettings,
    pins: ProjectPins,
    project_root: Option<PathBuf>,
    path_var: Option<OsString>,
    env: BTreeMap<String, OsString>,
}

impl Toolchains {
    pub fn new(catalog: Catalog, store: ToolchainStore) -> Self {
        let platform = Platform::current().unwrap_or(Platform {
            os: if cfg!(windows) {
                crate::Os::Windows
            } else if cfg!(target_os = "macos") {
                crate::Os::Macos
            } else {
                crate::Os::Linux
            },
            arch: crate::Arch::X86_64,
        });
        Self {
            catalog,
            store,
            platform,
            settings: ToolchainSettings::default(),
            pins: ProjectPins::default(),
            project_root: None,
            path_var: None,
            env: BTreeMap::new(),
        }
    }

    /// Uses `path` instead of the process `PATH` and `env` instead of the
    /// process environment for home variables (tests, sandboxed lookups).
    pub fn with_environment(mut self, path: OsString, env: BTreeMap<String, OsString>) -> Self {
        self.path_var = Some(path);
        self.env = env;
        self
    }

    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = platform;
        self
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub fn store(&self) -> &ToolchainStore {
        &self.store
    }

    pub fn platform(&self) -> Platform {
        self.platform
    }

    pub fn settings(&self) -> &ToolchainSettings {
        &self.settings
    }

    pub fn pins(&self) -> &ProjectPins {
        &self.pins
    }

    pub fn project_root(&self) -> Option<&Path> {
        self.project_root.as_deref()
    }

    /// Loads user + project settings and the project's pins.
    pub fn set_project(&mut self, root: Option<&Path>) -> Result<(), ToolchainError> {
        let settings = ToolchainSettings::load(root);
        self.apply_project(root, settings.as_ref().cloned().unwrap_or_default());
        settings.map(|_| ())
    }

    /// Like [`Toolchains::set_project`] with already-loaded settings.
    pub fn apply_project(&mut self, root: Option<&Path>, settings: ToolchainSettings) {
        self.pins = root.map_or_else(ProjectPins::default, |root| {
            ProjectPins::detect(root, &settings)
        });
        self.settings = settings;
        self.project_root = root.map(Path::to_owned);
    }

    /// The version requested for `component` by the project, else latest.
    pub fn request(&self, component: &str) -> VersionRequest {
        self.pins.request(component)
    }

    pub fn locate(&self, component: &str) -> Option<Located> {
        let spec = self.catalog.get(component)?.spec().clone();
        self.locate_spec(&spec)
    }

    /// Locates a component that may not be in the catalog.
    pub fn locate_spec(&self, spec: &ComponentSpec) -> Option<Located> {
        let located = |executable: PathBuf, version, source| Located {
            component: spec.id.clone(),
            home: home_of(&executable),
            executable,
            version,
            source,
        };
        if let Some(path) = self
            .settings
            .component(&spec.id)
            .and_then(|c| c.path.as_ref())
        {
            let executable = if path.is_file() {
                Some(path.clone())
            } else {
                find_executable(path, spec, &self.platform)
            };
            if let Some(executable) = executable {
                return Some(located(executable, None, LocateSource::Configured));
            }
        }
        let installed = self.store.installed(&spec.id);
        let managed = |request: &VersionRequest| {
            installed
                .iter()
                .filter(|install| request.matches(&install.version))
                .find_map(|install| {
                    let executable = install.executable(spec, &self.platform)?;
                    Some(located(
                        executable,
                        Some(install.version.clone()),
                        LocateSource::Managed,
                    ))
                })
        };
        if let Some(pin) = self.pins.get(&spec.id)
            && matches!(pin.request, VersionRequest::Version(_))
            && let Some(found) = managed(&pin.request)
        {
            return Some(found);
        }
        for variable in &spec.home_env {
            let home = self
                .env
                .get(variable)
                .cloned()
                .or_else(|| {
                    if self.path_var.is_none() {
                        std::env::var_os(variable)
                    } else {
                        None
                    }
                })
                .filter(|value| !value.is_empty());
            if let Some(executable) =
                home.and_then(|home| find_executable(Path::new(&home), spec, &self.platform))
            {
                return Some(located(
                    executable,
                    None,
                    LocateSource::HomeEnv(variable.clone()),
                ));
            }
        }
        if let Some(executable) = self.on_path(spec.primary_executable()) {
            return Some(located(executable, None, LocateSource::Path));
        }
        // Newest managed install (the list is sorted newest first).
        installed.iter().find_map(|install| {
            let executable = install.executable(spec, &self.platform)?;
            Some(located(
                executable,
                Some(install.version.clone()),
                LocateSource::Managed,
            ))
        })
    }

    fn on_path(&self, name: &str) -> Option<PathBuf> {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        match &self.path_var {
            Some(path) => which::which_in(name, Some(path), cwd).ok(),
            None => which::which(name).ok(),
        }
    }

    /// Path to `component`'s primary executable, if it can be found.
    pub fn executable(&self, component: &str) -> Option<PathBuf> {
        self.locate(component).map(|located| located.executable)
    }

    pub fn status(&self, component: &str) -> Option<ComponentStatus> {
        let spec = self.catalog.get(component)?.spec().clone();
        Some(ComponentStatus {
            located: self.locate_spec(&spec),
            installed: self.store.installed(&spec.id),
            pin: self.pins.get(&spec.id).cloned(),
            spec,
        })
    }

    /// Of `components` (and what they require), the ones that cannot be found.
    pub fn missing(&self, components: &[&str]) -> Vec<String> {
        let mut wanted: Vec<String> = Vec::new();
        let mut queue: Vec<String> = components.iter().map(|id| (*id).to_owned()).collect();
        while let Some(id) = queue.pop() {
            if wanted.contains(&id) {
                continue;
            }
            if let Some(provider) = self.catalog.get(&id) {
                queue.extend(provider.spec().requires.iter().cloned());
            }
            wanted.push(id);
        }
        wanted
            .into_iter()
            .filter(|id| self.locate(id).is_none())
            .collect()
    }

    /// Resolves the release to install for `component`: `request` if given,
    /// else the project pin, else the provider's default (latest, LTS for JDKs).
    pub fn resolve(
        &self,
        component: &str,
        request: Option<&VersionRequest>,
        http: &HttpClient,
    ) -> Result<Release, ToolchainError> {
        let provider = self
            .catalog
            .get(component)
            .ok_or_else(|| ToolchainError::UnknownComponent(component.into()))?;
        let request = request
            .cloned()
            .or_else(|| self.pins.get(component).map(|pin| pin.request.clone()))
            .unwrap_or_else(|| provider.default_request());
        provider.resolve(
            &request,
            &self.platform,
            http,
            self.settings.component(component),
        )
    }

    /// Environment for processes that need `components`: their bin
    /// directories prepended to `PATH` and home variables (`JAVA_HOME`) set.
    pub fn environment(&self, components: &[&str]) -> BTreeMap<String, OsString> {
        let mut env = BTreeMap::new();
        let mut bins = Vec::new();
        for id in components {
            let Some(provider) = self.catalog.get(id) else {
                continue;
            };
            let Some(located) = self.locate_spec(provider.spec()) else {
                continue;
            };
            if let Some(bin) = located.executable.parent() {
                bins.push(bin.to_owned());
            }
            for variable in &provider.spec().home_env {
                env.entry(variable.clone())
                    .or_insert_with(|| located.home.clone().into_os_string());
            }
        }
        if !bins.is_empty() {
            let current = self
                .path_var
                .clone()
                .or_else(|| std::env::var_os("PATH"))
                .unwrap_or_default();
            // Empty entries would mean the current directory.
            let paths = bins
                .into_iter()
                .chain(std::env::split_paths(&current))
                .filter(|path| !path.as_os_str().is_empty());
            if let Ok(joined) = std::env::join_paths(paths) {
                env.insert("PATH".into(), joined);
            }
        }
        env
    }
}

/// The directory above `bin/` (`<jdk>/bin/java` → `<jdk>`), else the
/// executable's directory.
fn home_of(executable: &Path) -> PathBuf {
    let parent = executable.parent().unwrap_or(executable);
    if parent.file_name().is_some_and(|name| name == "bin") {
        parent.parent().unwrap_or(parent).to_owned()
    } else {
        parent.to_owned()
    }
}

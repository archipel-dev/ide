//! Language server registry.
//!
//! Each [`LanguageServerProvider`] maps file types to a server, knows how to
//! find the server's executable and how to pick its workspace root. The
//! built-in providers only *describe* well-known servers; nothing is bundled.
//! Executables are discovered on `PATH` or configured in `lsp.toml`
//! (see [`ServerSettings`]).

use crate::{LspError, client::LaunchConfig};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

/// One way to launch a server: a program name or path plus arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub program: &'static str,
    pub args: &'static [&'static str],
}

/// Builds a [`Candidate`] in a promotable constant context.
macro_rules! candidate {
    ($program:expr, $args:expr) => {
        Candidate {
            program: $program,
            args: $args,
        }
    };
}

/// Static description of a language server.
#[derive(Debug, Clone)]
pub struct ServerSpec {
    /// Stable id used in settings and commands (`rust-analyzer`, `jdtls`, ...).
    pub id: &'static str,
    /// Human-readable language name for notifications ("Java", "Kotlin").
    pub language: &'static str,
    /// File extensions (without dot) mapped to LSP language ids.
    pub extensions: &'static [(&'static str, &'static str)],
    /// Exact file names mapped to LSP language ids (`BUILD.bazel`).
    pub file_names: &'static [(&'static str, &'static str)],
    /// Executables tried in order.
    pub candidates: &'static [Candidate],
    /// Files or directories that mark a workspace root for this server.
    pub root_markers: &'static [&'static str],
    /// How to install the server, shown when it is missing.
    pub install_hint: &'static str,
    /// Whether a missing server deserves a notification balloon. Servers for
    /// configuration formats only show in the status bar.
    pub notify_missing: bool,
    /// Default `initializationOptions` (JSON), unless set in `lsp.toml`.
    pub initialization_options: Option<&'static str>,
}

impl ServerSpec {
    /// The LSP language id for `path`, if this server handles it.
    pub fn language_id(&self, path: &Path) -> Option<&'static str> {
        let file_name = path.file_name()?.to_str()?;
        if let Some((_, id)) = self.file_names.iter().find(|(name, _)| *name == file_name) {
            return Some(id);
        }
        let extension = path.extension()?.to_str()?;
        self.extensions
            .iter()
            .find(|(ext, _)| ext.eq_ignore_ascii_case(extension))
            .map(|(_, id)| *id)
    }
}

/// Context for building a launch configuration.
#[derive(Clone)]
pub struct LaunchContext<'a> {
    /// Workspace root the server will serve.
    pub root: &'a Path,
    /// Per-user cache directory for server state (`~/.cache/archipel`).
    pub cache_dir: PathBuf,
    /// User settings for this server, if any.
    pub settings: Option<&'a ServerSettings>,
    /// Finds servers installed outside `PATH` (managed toolchains).
    pub resolver: Option<&'a dyn ProgramResolver>,
}

/// A server program found by a [`ProgramResolver`]: the full command line
/// (without the per-workspace arguments a provider adds) and its environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProgram {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// Hook through which the toolchain service supplies server executables.
///
/// `lsp.toml` commands still win; otherwise a resolved program is used before
/// the spec's `PATH` candidates.
pub trait ProgramResolver: Send + Sync {
    /// How to start `spec`'s server, if the resolver knows it.
    fn resolve(&self, spec: &ServerSpec) -> Option<ResolvedProgram>;

    /// Toolchain component that provides `spec`'s server, for "Set up" actions
    /// on missing-server notices.
    fn setup_component(&self, spec: &ServerSpec) -> Option<String>;
}

/// A language integration that can provide a language server.
///
/// Implement this to add a server from a plugin; the built-ins live in
/// [`builtin_providers`].
pub trait LanguageServerProvider: Send + Sync {
    fn spec(&self) -> &ServerSpec;

    /// Resolves the executable and arguments. The default implementation uses
    /// the configured command, else the first candidate found on `PATH`.
    fn launch(&self, context: &LaunchContext<'_>) -> Result<LaunchConfig, LspError> {
        default_launch(self.spec(), context)
    }

    /// Relevance adjustment for a completion item; higher sorts first among
    /// items that match equally well. Used where a server's `sortText` does
    /// not distinguish items the reference IDE ranks differently.
    fn completion_relevance(&self, _item: &lsp_types::CompletionItem) -> i32 {
        0
    }

    /// The toolchain component that provides this server (`rust-analyzer`, `jdtls`), for the
    /// "Set up" action of a missing-server notice. `None`: not provisioned by the toolchains.
    /// The default is the catalog entry of the server id ([`server_component`]).
    fn toolchain_component(&self) -> Option<&'static str> {
        server_component(self.spec().id)
    }
}

/// The toolchain component that provides the well-known server `id` (the catalog of servers
/// the toolchain service can provision).
pub fn server_component(id: &str) -> Option<&'static str> {
    match id {
        "rust-analyzer" => Some("rust-analyzer"),
        "jdtls" => Some("jdtls"),
        "kotlin" => Some("kotlin-lsp"),
        "taplo" => Some("taplo"),
        _ => None,
    }
}

/// The extension point language packs contribute their language server to (the mapping
/// "language -> server", `PLAT-LANG-012`; see `docs/architecture/language-packs.md`).
/// [`ServerRegistry::from_extensions`] reads it in loading order: an earlier extension wins
/// for the same file types.
pub const LANGUAGE_SERVER_PROVIDER: archipel_ide_platform::ExtensionPointName<
    dyn LanguageServerProvider,
> = archipel_ide_platform::ExtensionPointName::new("dev.archipel.language.languageServerProvider");

/// Resolves `spec`'s executable from settings or `PATH`.
pub fn default_launch(
    spec: &ServerSpec,
    context: &LaunchContext<'_>,
) -> Result<LaunchConfig, LspError> {
    let mut config = if let Some(command) = context.settings.and_then(|s| s.command.as_ref()) {
        let program = resolve_program(command).ok_or_else(|| LspError::ServerNotFound {
            server: spec.id.into(),
            language: spec.language.into(),
            hint: format!(
                "The configured command `{command}` was not found. Fix `servers.{}.command` in lsp.toml.",
                spec.id
            ),
        })?;
        let args = context
            .settings
            .and_then(|s| s.args.clone())
            .unwrap_or_default();
        LaunchConfig::new(spec.id, program).args(args)
    } else if let Some(resolved) = context.resolver.and_then(|resolver| resolver.resolve(spec)) {
        let mut config = LaunchConfig::new(spec.id, resolved.program).args(resolved.args);
        config.env = resolved.env;
        config
    } else {
        spec.candidates
            .iter()
            .find_map(|candidate| {
                let program = resolve_program(candidate.program)?;
                Some(LaunchConfig::new(spec.id, program).args(candidate.args.iter().copied()))
            })
            .ok_or_else(|| LspError::ServerNotFound {
                server: spec.id.into(),
                language: spec.language.into(),
                hint: spec.install_hint.into(),
            })?
    };
    config.initialization_options = spec
        .initialization_options
        .and_then(|options| serde_json::from_str(options).ok());
    if let Some(settings) = context.settings {
        if let Some(args) = &settings.args
            && settings.command.is_none()
        {
            config.args = args.clone();
        }
        config.env.extend(
            settings
                .env
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        if settings.initialization_options.is_some() {
            config.initialization_options = settings.initialization_options.clone();
        }
    }
    Ok(config)
}

/// Finds an executable by absolute/relative path or on `PATH`.
pub fn resolve_program(program: &str) -> Option<PathBuf> {
    let path = Path::new(program);
    if path.components().count() > 1 || path.is_absolute() {
        return path.is_file().then(|| path.to_owned());
    }
    which::which(program).ok()
}

/// User overrides for one server (`[servers.<id>]` in `lsp.toml`).
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ServerSettings {
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    #[serde(rename = "initialization_options")]
    pub initialization_options: Option<Value>,
}

fn enabled_default() -> bool {
    true
}

/// Contents of `lsp.toml`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LspSettings {
    #[serde(default)]
    pub servers: BTreeMap<String, ServerSettings>,
}

impl LspSettings {
    pub fn parse(input: &str) -> Result<Self, LspError> {
        toml::from_str(input).map_err(|error| LspError::Settings(error.to_string()))
    }

    /// Reads `<config>/archipel/lsp.toml`, then `<project>/.archipel/lsp.toml`,
    /// with project entries overriding user entries per server.
    pub fn load(project_root: Option<&Path>) -> Result<Self, LspError> {
        let mut settings = Self::default();
        // Projects from before the rename keep the file in `.jetthing/` until the first save.
        let files = config_dir()
            .map(|dir| dir.join("lsp.toml"))
            .into_iter()
            .chain(project_root.map(|root| {
                archipel_ide_compat::project_read_path(root.join(".archipel").join("lsp.toml"))
            }));
        for file in files {
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            let parsed = Self::parse(&text)
                .map_err(|error| LspError::Settings(format!("{}: {error}", file.display())))?;
            settings.servers.extend(parsed.servers);
        }
        Ok(settings)
    }
}

/// Registry of language server providers.
#[derive(Clone)]
pub struct ServerRegistry {
    /// Registered by hand ([`ServerRegistry::register`]); these win over contributed ones.
    explicit: Vec<Arc<dyn LanguageServerProvider>>,
    /// The providers language packs contribute to [`LANGUAGE_SERVER_PROVIDER`], in loading
    /// order ([`ServerRegistry::set_contributed`]).
    contributed: Vec<Arc<dyn LanguageServerProvider>>,
    /// Whether [`ServerRegistry::set_contributed`] has any effect (not for [`Self::empty`]).
    accepts_contributions: bool,
}

impl Default for ServerRegistry {
    /// The well-known servers of [`builtin_providers`]: what a stand-alone use of the LSP
    /// crate (tests, tools) gets. The IDE replaces them with what its language packs
    /// contribute ([`ServerRegistry::set_contributed`]).
    fn default() -> Self {
        Self {
            explicit: Vec::new(),
            contributed: builtin_providers(),
            accepts_contributions: true,
        }
    }
}

impl ServerRegistry {
    /// No providers, and none are accepted from language packs either.
    pub fn empty() -> Self {
        Self {
            explicit: Vec::new(),
            contributed: Vec::new(),
            accepts_contributions: false,
        }
    }

    /// A registry of exactly what the extension registry holds (loading order).
    pub fn from_extensions(extensions: &archipel_ide_platform::ExtensionRegistry) -> Self {
        Self {
            explicit: Vec::new(),
            contributed: extensions.extension_list(LANGUAGE_SERVER_PROVIDER),
            accepts_contributions: true,
        }
    }

    /// Adds a provider; later registrations take precedence for the same file types.
    pub fn register(&mut self, provider: Arc<dyn LanguageServerProvider>) {
        self.explicit.insert(0, provider);
    }

    /// Replaces the contributed providers (a language pack was installed or removed).
    /// Returns whether anything changed. Servers that are already running are not stopped.
    pub fn set_contributed(&mut self, providers: Vec<Arc<dyn LanguageServerProvider>>) -> bool {
        let same = self.contributed.len() == providers.len()
            && self
                .contributed
                .iter()
                .zip(&providers)
                .all(|(a, b)| Arc::ptr_eq(a, b));
        if !self.accepts_contributions || same {
            return false;
        }
        self.contributed = providers;
        true
    }

    /// Every provider: registered by hand first, then contributed ones in loading order.
    pub fn providers(&self) -> impl Iterator<Item = &Arc<dyn LanguageServerProvider>> {
        self.explicit.iter().chain(&self.contributed)
    }

    pub fn get(&self, id: &str) -> Option<&Arc<dyn LanguageServerProvider>> {
        self.providers().find(|p| p.spec().id == id)
    }

    /// The provider and LSP language id for `path`.
    pub fn for_path(
        &self,
        path: &Path,
    ) -> Option<(&Arc<dyn LanguageServerProvider>, &'static str)> {
        self.providers()
            .find_map(|provider| Some((provider, provider.spec().language_id(path)?)))
    }
}

/// Picks the workspace root for `file`: the outermost directory between the
/// file and `project_root` that contains one of `markers`, else `project_root`.
/// Files outside the project use their nearest marker directory, else their
/// parent directory.
pub fn workspace_root(file: &Path, project_root: &Path, markers: &[&str]) -> PathBuf {
    let has_marker = |dir: &Path| markers.iter().any(|marker| dir.join(marker).exists());
    let inside = file.starts_with(project_root);
    let mut chosen = None;
    let mut dir = file.parent();
    while let Some(current) = dir {
        if inside && !current.starts_with(project_root) {
            break;
        }
        if has_marker(current) {
            chosen = Some(current.to_owned());
            if !inside {
                break;
            }
        }
        dir = current.parent();
    }
    chosen.unwrap_or_else(|| {
        if inside {
            project_root.to_owned()
        } else {
            file.parent().unwrap_or(project_root).to_owned()
        }
    })
}

/// Per-user cache directory (`$XDG_CACHE_HOME/archipel`, `~/Library/Caches/archipel`,
/// `%LOCALAPPDATA%\archipel`). A cache left by the Jetthing versions moves to the new name.
pub fn cache_dir() -> PathBuf {
    let env = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    let base = if cfg!(windows) {
        env("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|home| PathBuf::from(home).join("Library/Caches"))
    } else {
        env("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".cache")))
    };
    archipel_ide_compat::adopt_legacy_dir(base.unwrap_or_else(std::env::temp_dir).join("archipel"))
}

/// Per-user configuration directory (`$XDG_CONFIG_HOME/archipel`, ...).
pub fn config_dir() -> Option<PathBuf> {
    let env = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    let base = if cfg!(windows) {
        env("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else {
        env("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".config")))
    };
    base.map(|base| archipel_ide_compat::adopt_legacy_dir(base.join("archipel")))
}

/// Stable 64-bit FNV-1a hash, used to name per-root cache directories.
pub fn stable_hash(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Provider backed only by a [`ServerSpec`].
pub struct SpecProvider(pub ServerSpec);

impl LanguageServerProvider for SpecProvider {
    fn spec(&self) -> &ServerSpec {
        &self.0
    }
}

/// Environment variable through which a resolver passes the java-debug plugin jar.
pub const JAVA_DEBUG_PLUGIN_ENV: &str = "ARCHIPEL_JAVA_DEBUG_PLUGIN";

/// Classes whose static members jdtls offers as "import static" fixes for
/// unresolved methods (`java.completion.favoriteStaticMembers`). jdtls only
/// proposes static imports from this list, and its default one lacks AssertJ,
/// Hamcrest's `assertThat` and Truth; the reference IDE offers the import from
/// any class on the classpath, so the list names the common test libraries.
pub const JDTLS_FAVORITE_STATIC_MEMBERS: &[&str] = &[
    "org.junit.Assert.*",
    "org.junit.Assume.*",
    "org.junit.jupiter.api.Assertions.*",
    "org.junit.jupiter.api.Assumptions.*",
    "org.junit.jupiter.api.DynamicContainer.*",
    "org.junit.jupiter.api.DynamicTest.*",
    "org.assertj.core.api.Assertions.*",
    "org.assertj.core.api.Assumptions.*",
    "org.hamcrest.MatcherAssert.assertThat",
    "org.hamcrest.Matchers.*",
    "org.hamcrest.CoreMatchers.*",
    "org.mockito.Mockito.*",
    "org.mockito.ArgumentMatchers.*",
    "org.mockito.BDDMockito.*",
    "org.mockito.Answers.*",
    "com.google.common.truth.Truth.*",
    "org.awaitility.Awaitility.*",
    "org.springframework.test.web.servlet.request.MockMvcRequestBuilders.*",
    "org.springframework.test.web.servlet.result.MockMvcResultMatchers.*",
    "org.springframework.test.web.servlet.result.MockMvcResultHandlers.*",
];

/// Eclipse JDT LS needs a writable workspace data directory per project.
pub struct JdtlsProvider(pub ServerSpec);

impl LanguageServerProvider for JdtlsProvider {
    fn spec(&self) -> &ServerSpec {
        &self.0
    }

    fn launch(&self, context: &LaunchContext<'_>) -> Result<LaunchConfig, LspError> {
        let mut config = default_launch(&self.0, context)?;
        // jdtls' file system plugin reads this JVM property; without it the
        // Gradle import writes .project/.classpath/.settings into the user's tree.
        let launcher = config
            .program
            .file_stem()
            .is_some_and(|stem| stem == "jdtls");
        const NO_METADATA: &str = "-Djava.import.generatesMetadataFilesAtProjectRoot=false";
        if !config.args.iter().any(|arg| arg.contains(NO_METADATA)) {
            if launcher {
                config.args.insert(0, format!("--jvm-arg={NO_METADATA}"));
            } else if let Some(jar) = config.args.iter().position(|arg| arg == "-jar") {
                // The Equinox launcher run on a JDK directly (managed installs).
                config.args.insert(jar, NO_METADATA.to_owned());
            }
        }
        if !config.args.iter().any(|arg| arg == "-data") {
            let name = context
                .root
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("workspace");
            let data = context.cache_dir.join("jdtls").join(format!(
                "{name}-{:016x}",
                stable_hash(&context.root.display().to_string())
            ));
            config.args.push("-data".into());
            config.args.push(data.display().to_string());
        }
        if config.initialization_options.is_none() {
            config.initialization_options = Some(serde_json::json!({
                "settings": {"java": {
                    "import": {"generatesMetadataFilesAtProjectRoot": false},
                    "completion": {
                        "guessMethodArguments": "insertParameterNames",
                        "favoriteStaticMembers": JDTLS_FAVORITE_STATIC_MEMBERS
                    },
                    "signatureHelp": {"enabled": true}
                }},
                "extendedClientCapabilities": {"progressReportProvider": false}
            }));
        }
        // The java-debug plugin jar (found by the toolchain resolver) is loaded as an
        // OSGi bundle; it provides the `vscode.java.*` commands the debugger uses.
        let bundle = config
            .env
            .iter()
            .position(|(key, _)| key == JAVA_DEBUG_PLUGIN_ENV)
            .map(|index| config.env.remove(index).1);
        if let (Some(bundle), Some(Value::Object(options))) =
            (bundle, config.initialization_options.as_mut())
        {
            options
                .entry("bundles")
                .or_insert_with(|| serde_json::json!([bundle]));
        }
        Ok(config)
    }

    /// jdtls gives members of the receiver's class and those inherited from
    /// `java.lang.Object` the same `sortText`; the reference IDE lists the
    /// class's own members first.
    fn completion_relevance(&self, item: &lsp_types::CompletionItem) -> i32 {
        if item
            .detail
            .as_deref()
            .is_some_and(|detail| detail.starts_with("Object."))
        {
            -1
        } else {
            0
        }
    }
}

const GRADLE_MAVEN: &[&str] = &[
    "settings.gradle",
    "settings.gradle.kts",
    "build.gradle",
    "build.gradle.kts",
    "pom.xml",
];

/// Specs for the servers Archipel IDE knows about out of the box.
pub fn builtin_specs() -> Vec<ServerSpec> {
    vec![
        ServerSpec {
            id: "rust-analyzer",
            language: "Rust",
            extensions: &[("rs", "rust")],
            file_names: &[],
            candidates: &[candidate!("rust-analyzer", &[])],
            root_markers: &["Cargo.toml"],
            install_hint: "Install it with `rustup component add rust-analyzer` and make sure it is on PATH.",
            notify_missing: true,
            // Like the reference IDE: insert `name()` and put the caret inside,
            // without argument placeholders.
            initialization_options: Some(
                r#"{"completion":{"callable":{"snippets":"add_parentheses"}}}"#,
            ),
        },
        ServerSpec {
            id: "jdtls",
            language: "Java",
            extensions: &[("java", "java")],
            file_names: &[],
            candidates: &[candidate!("jdtls", &[])],
            root_markers: GRADLE_MAVEN,
            install_hint: "Install Eclipse JDT Language Server (https://download.eclipse.org/jdtls/) and put its `bin/jdtls` launcher on PATH. It needs JDK 21 or newer.",
            notify_missing: true,
            initialization_options: None,
        },
        ServerSpec {
            id: "kotlin",
            language: "Kotlin",
            extensions: &[("kt", "kotlin"), ("kts", "kotlin")],
            file_names: &[],
            candidates: &[
                candidate!("kotlin-lsp", &["--stdio"]),
                candidate!("kotlin-language-server", &[]),
            ],
            root_markers: GRADLE_MAVEN,
            install_hint: "Install JetBrains kotlin-lsp (https://github.com/Kotlin/kotlin-lsp; its server archive provides bin/intellij-server, which can be linked as `kotlin-lsp`) or kotlin-language-server, and put it on PATH.",
            notify_missing: true,
            initialization_options: None,
        },
        ServerSpec {
            id: "starlark",
            language: "Starlark",
            extensions: &[
                ("bzl", "starlark"),
                ("bazel", "starlark"),
                ("star", "starlark"),
            ],
            file_names: &[
                ("BUILD", "starlark"),
                ("WORKSPACE", "starlark"),
                ("MODULE.bazel", "starlark"),
            ],
            candidates: &[
                candidate!("starpls", &["server"]),
                candidate!("bazel-lsp", &[]),
            ],
            root_markers: &["MODULE.bazel", "WORKSPACE", "WORKSPACE.bazel"],
            install_hint: "Install starpls (https://github.com/withered-magic/starpls) or bazel-lsp and put it on PATH.",
            notify_missing: true,
            initialization_options: None,
        },
        ServerSpec {
            id: "pyright",
            language: "Python",
            extensions: &[("py", "python"), ("pyi", "python")],
            file_names: &[],
            candidates: &[
                candidate!("basedpyright-langserver", &["--stdio"]),
                candidate!("pyright-langserver", &["--stdio"]),
            ],
            root_markers: &[
                "pyproject.toml",
                "setup.py",
                "setup.cfg",
                "requirements.txt",
            ],
            install_hint: "Install it with `pip install basedpyright` or `npm install -g pyright`.",
            notify_missing: true,
            initialization_options: None,
        },
        ServerSpec {
            id: "gopls",
            language: "Go",
            extensions: &[("go", "go")],
            file_names: &[],
            candidates: &[candidate!("gopls", &[])],
            root_markers: &["go.work", "go.mod"],
            install_hint: "Install it with `go install golang.org/x/tools/gopls@latest`.",
            notify_missing: true,
            initialization_options: None,
        },
        ServerSpec {
            id: "typescript",
            language: "TypeScript/JavaScript",
            extensions: &[
                ("ts", "typescript"),
                ("mts", "typescript"),
                ("cts", "typescript"),
                ("tsx", "typescriptreact"),
                ("js", "javascript"),
                ("mjs", "javascript"),
                ("cjs", "javascript"),
                ("jsx", "javascriptreact"),
            ],
            file_names: &[],
            candidates: &[candidate!("typescript-language-server", &["--stdio"])],
            root_markers: &["tsconfig.json", "jsconfig.json", "package.json"],
            install_hint: "Install it with `npm install -g typescript typescript-language-server`.",
            notify_missing: true,
            initialization_options: None,
        },
        ServerSpec {
            id: "clangd",
            language: "C/C++",
            extensions: &[
                ("c", "c"),
                ("h", "c"),
                ("cc", "cpp"),
                ("cpp", "cpp"),
                ("cxx", "cpp"),
                ("hpp", "cpp"),
                ("hh", "cpp"),
                ("hxx", "cpp"),
            ],
            file_names: &[],
            candidates: &[candidate!("clangd", &[])],
            root_markers: &["compile_commands.json", "compile_flags.txt", ".clangd"],
            install_hint: "Install clangd from your LLVM distribution or package manager.",
            notify_missing: true,
            initialization_options: None,
        },
        ServerSpec {
            id: "lemminx",
            language: "XML",
            extensions: &[("xml", "xml"), ("xsd", "xml"), ("xsl", "xml")],
            file_names: &[],
            candidates: &[candidate!("lemminx", &[])],
            root_markers: &[],
            install_hint: "Install Eclipse LemMinX (https://github.com/eclipse/lemminx) and put `lemminx` on PATH.",
            notify_missing: false,
            initialization_options: None,
        },
        ServerSpec {
            id: "taplo",
            language: "TOML",
            extensions: &[("toml", "toml")],
            file_names: &[],
            candidates: &[candidate!("taplo", &["lsp", "stdio"])],
            root_markers: &[],
            install_hint: "Install it with `cargo install taplo-cli --features lsp`.",
            notify_missing: false,
            initialization_options: None,
        },
        ServerSpec {
            id: "yaml",
            language: "YAML",
            extensions: &[("yaml", "yaml"), ("yml", "yaml")],
            file_names: &[],
            candidates: &[candidate!("yaml-language-server", &["--stdio"])],
            root_markers: &[],
            install_hint: "Install it with `npm install -g yaml-language-server`.",
            notify_missing: false,
            initialization_options: None,
        },
        ServerSpec {
            id: "json",
            language: "JSON",
            extensions: &[("json", "json"), ("jsonc", "jsonc")],
            file_names: &[],
            candidates: &[
                candidate!("vscode-json-language-server", &["--stdio"]),
                candidate!("vscode-json-languageserver", &["--stdio"]),
            ],
            root_markers: &[],
            install_hint: "Install it with `npm install -g vscode-langservers-extracted`.",
            notify_missing: false,
            initialization_options: None,
        },
    ]
}

/// The built-in provider of the server `id` (`"jdtls"`, `"rust-analyzer"`, ...). Language packs
/// pick the servers of their language from this catalog.
pub fn builtin_provider(id: &str) -> Option<Arc<dyn LanguageServerProvider>> {
    builtin_specs()
        .into_iter()
        .find(|spec| spec.id == id)
        .map(provider_for)
}

fn provider_for(spec: ServerSpec) -> Arc<dyn LanguageServerProvider> {
    if spec.id == "jdtls" {
        Arc::new(JdtlsProvider(spec))
    } else {
        Arc::new(SpecProvider(spec))
    }
}

pub fn builtin_providers() -> Vec<Arc<dyn LanguageServerProvider>> {
    builtin_specs().into_iter().map(provider_for).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context<'a>(root: &'a Path, settings: Option<&'a ServerSettings>) -> LaunchContext<'a> {
        LaunchContext {
            root,
            cache_dir: PathBuf::from("/cache/archipel"),
            settings,
            resolver: None,
        }
    }

    #[test]
    fn maps_files_to_servers_and_language_ids() {
        let registry = ServerRegistry::default();
        let id = |path: &str| {
            registry
                .for_path(Path::new(path))
                .map(|(provider, language)| (provider.spec().id, language))
        };
        assert_eq!(id("src/main.rs"), Some(("rust-analyzer", "rust")));
        assert_eq!(id("App.java"), Some(("jdtls", "java")));
        assert_eq!(id("MainActivity.kt"), Some(("kotlin", "kotlin")));
        assert_eq!(id("build.gradle.kts"), Some(("kotlin", "kotlin")));
        assert_eq!(id("pkg/BUILD"), Some(("starlark", "starlark")));
        assert_eq!(id("MODULE.bazel"), Some(("starlark", "starlark")));
        assert_eq!(id("defs.bzl"), Some(("starlark", "starlark")));
        assert_eq!(id("app.tsx"), Some(("typescript", "typescriptreact")));
        assert_eq!(id("AndroidManifest.xml"), Some(("lemminx", "xml")));
        assert_eq!(id("Cargo.toml"), Some(("taplo", "toml")));
        assert_eq!(id("main.c"), Some(("clangd", "c")));
        assert_eq!(id("notes.txt"), None);
        assert_eq!(id("Makefile"), None);
    }

    #[test]
    fn every_builtin_has_an_install_hint_and_candidates() {
        for spec in builtin_specs() {
            assert!(!spec.candidates.is_empty(), "{}", spec.id);
            assert!(!spec.install_hint.is_empty(), "{}", spec.id);
        }
    }

    #[test]
    fn missing_server_is_an_actionable_error() {
        let spec = ServerSpec {
            candidates: &[candidate!("archipel-ide-definitely-missing-server", &[])],
            ..builtin_specs().remove(1)
        };
        let error = default_launch(&spec, &context(Path::new("/p"), None)).unwrap_err();
        let text = error.to_string();
        assert!(text.contains("Java"), "{text}");
        assert!(text.contains("download.eclipse.org/jdtls"), "{text}");
    }

    #[cfg(unix)]
    #[test]
    fn jdtls_on_a_jdk_keeps_metadata_out_of_the_project_and_favours_test_libraries() {
        // A managed install runs the Equinox launcher on the JDK directly: the
        // JVM property goes in front of `-jar`, not after it.
        let settings = ServerSettings {
            command: Some("/bin/sh".into()),
            args: Some(vec!["-Xms1G".into(), "-jar".into(), "launcher.jar".into()]),
            enabled: true,
            ..Default::default()
        };
        let provider = JdtlsProvider(builtin_specs().remove(1));
        let config = provider
            .launch(&context(Path::new("/p/app"), Some(&settings)))
            .unwrap();
        let jar = config.args.iter().position(|arg| arg == "-jar").unwrap();
        assert_eq!(
            config.args[jar - 1],
            "-Djava.import.generatesMetadataFilesAtProjectRoot=false"
        );
        let favorites = &config.initialization_options.unwrap()["settings"]["java"]["completion"]["favoriteStaticMembers"];
        let favorites: Vec<&str> = favorites
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|value| value.as_str())
            .collect();
        for expected in [
            "org.assertj.core.api.Assertions.*",
            "org.hamcrest.MatcherAssert.assertThat",
            "org.junit.jupiter.api.Assertions.*",
        ] {
            assert!(favorites.contains(&expected), "{expected} in {favorites:?}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn jdtls_gets_a_stable_per_root_data_directory() {
        let settings = ServerSettings {
            command: Some("/bin/sh".into()),
            enabled: true,
            ..Default::default()
        };
        let provider = JdtlsProvider(builtin_specs().remove(1));
        let root = Path::new("/home/user/samples/java-gradle");
        let config = provider.launch(&context(root, Some(&settings))).unwrap();
        assert_eq!(config.program, PathBuf::from("/bin/sh"));
        let data = config.args.iter().position(|arg| arg == "-data").unwrap();
        let dir = &config.args[data + 1];
        assert!(
            dir.starts_with("/cache/archipel/jdtls/java-gradle-"),
            "{dir}"
        );
        let again = provider.launch(&context(root, Some(&settings))).unwrap();
        assert_eq!(config.args, again.args);
        assert!(config.initialization_options.is_some());
        assert!(!config.args.iter().any(|arg| arg.starts_with("--jvm-arg")));

        // The Python launcher gets the JVM property that keeps Eclipse
        // metadata out of the project.
        let bin =
            std::env::temp_dir().join(format!("archipel-ide-jdtls-bin-{}", std::process::id()));
        std::fs::create_dir_all(&bin).unwrap();
        let launcher = bin.join("jdtls");
        std::fs::write(&launcher, "#!/bin/sh\n").unwrap();
        let settings = ServerSettings {
            command: Some(launcher.display().to_string()),
            enabled: true,
            ..Default::default()
        };
        let config = provider.launch(&context(root, Some(&settings))).unwrap();
        assert_eq!(
            config.args[0],
            "--jvm-arg=-Djava.import.generatesMetadataFilesAtProjectRoot=false"
        );
        std::fs::remove_dir_all(bin).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn settings_override_command_args_and_env() {
        let settings = LspSettings::parse(
            "[servers.rust-analyzer]\ncommand = \"/bin/sh\"\nargs = [\"-c\", \"true\"]\nenv = { RA_LOG = \"info\" }\n",
        )
        .unwrap();
        let server = &settings.servers["rust-analyzer"];
        assert!(server.enabled);
        let spec = builtin_specs().remove(0);
        let config = default_launch(&spec, &context(Path::new("/p"), Some(server))).unwrap();
        assert_eq!(config.args, ["-c", "true"]);
        assert_eq!(config.env, [("RA_LOG".to_string(), "info".to_string())]);
        assert!(LspSettings::parse("[servers.x]\nunknown = 1\n").is_err());
    }

    #[test]
    fn workspace_root_prefers_outermost_marker_inside_the_project() {
        let base = std::env::temp_dir().join(format!("archipel-ide-root-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let module = base.join("app/src/main/java");
        std::fs::create_dir_all(&module).unwrap();
        std::fs::write(base.join("settings.gradle.kts"), "").unwrap();
        std::fs::write(base.join("app/build.gradle.kts"), "").unwrap();
        let file = module.join("App.java");
        assert_eq!(workspace_root(&file, &base, GRADLE_MAVEN), base);
        assert_eq!(workspace_root(&file, &base, &["Cargo.toml"]), base);
        let outside = std::env::temp_dir().join("archipel-ide-outside/x.rs");
        assert_eq!(
            workspace_root(&outside, &base, &["Cargo.toml"]),
            std::env::temp_dir().join("archipel-ide-outside")
        );
        std::fs::remove_dir_all(base).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rust_analyzer_inserts_parentheses_without_argument_placeholders() {
        let settings = ServerSettings {
            command: Some("/bin/sh".into()),
            enabled: true,
            ..Default::default()
        };
        let spec = builtin_specs().remove(0);
        let config = default_launch(&spec, &context(Path::new("/p"), Some(&settings))).unwrap();
        assert_eq!(
            config.initialization_options.unwrap()["completion"]["callable"]["snippets"],
            "add_parentheses"
        );
    }

    #[test]
    fn contributed_providers_follow_the_extension_registry() {
        use archipel_ide_platform::{Application, ComponentManager, LoadingOrder};
        let app = Application::new();
        let pack = app.disposable();
        let ext = app.extensions();
        let rust = builtin_provider("rust-analyzer").unwrap();
        ext.register(
            LANGUAGE_SERVER_PROVIDER,
            "rust-analyzer",
            LoadingOrder::Anywhere,
            rust,
            Some(pack),
        )
        .unwrap();
        let mut registry = ServerRegistry::from_extensions(ext);
        let (provider, language) = registry.for_path(Path::new("a.rs")).unwrap();
        assert_eq!(language, "rust");
        assert_eq!(provider.toolchain_component(), Some("rust-analyzer"));
        assert!(registry.for_path(Path::new("A.java")).is_none());
        // Hand-registered providers win; `empty()` takes no contributions.
        assert!(!ServerRegistry::empty().set_contributed(vec![]));
        assert!(registry.set_contributed(vec![builtin_provider("jdtls").unwrap()]));
        assert!(registry.for_path(Path::new("a.rs")).is_none());
        assert!(registry.for_path(Path::new("A.java")).is_some());
        app.dispose();
        assert!(
            ServerRegistry::from_extensions(ext)
                .for_path(Path::new("a.rs"))
                .is_none()
        );
    }

    #[test]
    fn stable_hash_is_deterministic() {
        assert_eq!(stable_hash(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(stable_hash("a"), stable_hash("a"));
        assert_ne!(stable_hash("a"), stable_hash("b"));
    }

    struct FixedResolver;

    impl ProgramResolver for FixedResolver {
        fn resolve(&self, spec: &ServerSpec) -> Option<ResolvedProgram> {
            (spec.id == "jdtls").then(|| ResolvedProgram {
                program: PathBuf::from("/managed/jdk/bin/java"),
                args: vec!["-jar".into(), "launcher.jar".into()],
                env: vec![("JAVA_HOME".into(), "/managed/jdk".into())],
            })
        }

        fn setup_component(&self, spec: &ServerSpec) -> Option<String> {
            Some(spec.id.to_owned())
        }
    }

    #[test]
    fn resolved_programs_come_after_configured_commands_and_before_path() {
        let registry = ServerRegistry::default();
        let jdtls = registry.get("jdtls").unwrap();
        let root = Path::new("/work/demo");
        let mut context = context(root, None);
        context.resolver = Some(&FixedResolver);
        let config = jdtls.launch(&context).unwrap();
        assert_eq!(config.program, PathBuf::from("/managed/jdk/bin/java"));
        assert_eq!(
            &config.args[..3],
            [
                "-Djava.import.generatesMetadataFilesAtProjectRoot=false",
                "-jar",
                "launcher.jar"
            ]
        );
        // The jdtls provider still adds its per-workspace data directory.
        assert!(config.args.contains(&"-data".to_string()));
        assert_eq!(
            config.env,
            vec![("JAVA_HOME".into(), "/managed/jdk".into())]
        );

        // An lsp.toml command wins over the resolver.
        let settings = ServerSettings {
            command: Some("/definitely/missing/jdtls".into()),
            ..ServerSettings::default()
        };
        let mut context = super::tests::context(root, Some(&settings));
        context.resolver = Some(&FixedResolver);
        assert!(matches!(
            jdtls.launch(&context),
            Err(LspError::ServerNotFound { .. })
        ));
    }
}

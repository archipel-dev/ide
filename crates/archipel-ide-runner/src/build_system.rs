//! Build-system service trait and the built-in providers.
//!
//! Every build tool Archipel IDE understands (Cargo, Gradle, Maven, Bazel, npm, Go)
//! implements [`BuildSystem`]. The platform asks the first provider that detects a
//! project root for build/run/test plans; providers that cannot run a project
//! return an actionable [`RunnerError`] instead of a plan that is known to fail.
//! Richer integrations (the Gradle model, Bazel targets, Android deploy) live in
//! their own crates and register additional or replacement providers.

use crate::{RunConfiguration, RunTarget, RunnerError, TaskKind, TaskPlan, TestId};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildSystemKind {
    Cargo,
    Gradle,
    Maven,
    Bazel,
    Npm,
    Go,
}

impl BuildSystemKind {
    pub fn id(self) -> &'static str {
        match self {
            BuildSystemKind::Cargo => "cargo",
            BuildSystemKind::Gradle => "gradle",
            BuildSystemKind::Maven => "maven",
            BuildSystemKind::Bazel => "bazel",
            BuildSystemKind::Npm => "npm",
            BuildSystemKind::Go => "go",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            BuildSystemKind::Cargo => "Cargo",
            BuildSystemKind::Gradle => "Gradle",
            BuildSystemKind::Maven => "Maven",
            BuildSystemKind::Bazel => "Bazel",
            BuildSystemKind::Npm => "npm",
            BuildSystemKind::Go => "Go",
        }
    }
}

/// A build tool integration. Implementations must not panic on malformed projects.
pub trait BuildSystem: Send + Sync {
    fn kind(&self) -> BuildSystemKind;

    /// Whether `root` is a project of this build system.
    fn detect(&self, root: &Path) -> bool;

    /// Plan for Build | Build Project.
    fn build_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError>;

    /// Plan for the default Run action.
    fn run_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError>;

    /// Plan that runs all tests.
    fn test_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError>;

    /// Plan for a run configuration whose type belongs to this provider.
    fn configuration_plan(
        &self,
        _root: &Path,
        config: &RunConfiguration,
    ) -> Result<TaskPlan, RunnerError> {
        Err(RunnerError::NotRunnable(format!(
            "{} cannot run {} configurations",
            self.kind().display_name(),
            config.target.type_name()
        )))
    }

    /// Re-runs only `failed` tests of a finished test `plan`.
    fn rerun_failed_plan(
        &self,
        _root: &Path,
        _plan: &TaskPlan,
        _failed: &[TestId],
    ) -> Result<TaskPlan, RunnerError> {
        Err(RunnerError::NotRunnable(format!(
            "re-running failed tests is not supported for {}",
            self.kind().display_name()
        )))
    }
}

/// `plan` with its arguments replaced, keeping name, kind, cwd and environment.
pub fn derive_plan(plan: &TaskPlan, args: Vec<String>) -> TaskPlan {
    TaskPlan {
        args,
        kind: TaskKind::Test,
        ..plan.clone()
    }
}

/// Splits arguments at the first `--` (tool options vs. program arguments).
fn split_at_separator(args: &[String]) -> (&[String], &[String]) {
    match args.iter().position(|arg| arg == "--") {
        Some(index) => (&args[..index], &args[index + 1..]),
        None => (args, &[]),
    }
}

/// Ordered provider list; the first provider that detects a root wins.
pub struct BuildSystemRegistry {
    providers: Vec<Box<dyn BuildSystem>>,
}

impl Default for BuildSystemRegistry {
    /// The built-in providers of this crate (Gradle and others are added by their crates).
    fn default() -> Self {
        Self {
            providers: vec![
                Box::new(Cargo),
                Box::new(Maven),
                Box::new(Bazel),
                Box::new(Npm),
                Box::new(Go),
            ],
        }
    }
}

impl BuildSystemRegistry {
    pub fn empty() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Adds a provider with precedence over the existing ones.
    pub fn register_first(&mut self, provider: Box<dyn BuildSystem>) {
        self.providers.insert(0, provider);
    }

    /// Adds a provider before the first provider of `kind` (or last if absent).
    pub fn register_before(&mut self, kind: BuildSystemKind, provider: Box<dyn BuildSystem>) {
        let index = self
            .providers
            .iter()
            .position(|existing| existing.kind() == kind)
            .unwrap_or(self.providers.len());
        self.providers.insert(index, provider);
    }

    pub fn detect(&self, root: &Path) -> Option<&dyn BuildSystem> {
        self.providers
            .iter()
            .find(|provider| provider.detect(root))
            .map(Box::as_ref)
    }

    /// Plans `config` with the provider registered for its type (shell commands are
    /// planned directly). Works even when the project root is of another type.
    pub fn plan_configuration(
        &self,
        root: &Path,
        config: &RunConfiguration,
    ) -> Result<TaskPlan, RunnerError> {
        config
            .validate()
            .map_err(|error| RunnerError::NotRunnable(format!("{}: {error}", config.name)))?;
        let Some(kind) = config.target.build_system() else {
            return config.shell_plan(root);
        };
        let provider = self
            .providers
            .iter()
            .find(|provider| provider.kind() == kind)
            .ok_or_else(|| {
                RunnerError::NotRunnable(format!(
                    "no {} integration is registered",
                    kind.display_name()
                ))
            })?;
        // Spring Boot configurations run as the Gradle, Maven or Bazel target they resolve to.
        let resolved;
        let config = match config.target.resolve_spring() {
            Some(target) => {
                resolved = RunConfiguration {
                    target,
                    ..config.clone()
                };
                &resolved
            }
            None => config,
        };
        let plan = provider.configuration_plan(root, config)?;
        Ok(config.finish_plan(plan, root))
    }

    pub fn kinds(&self) -> Vec<BuildSystemKind> {
        self.providers
            .iter()
            .map(|provider| provider.kind())
            .collect()
    }
}

/// Searches `PATH` for an executable, honouring `PATHEXT` on Windows.
pub fn find_program(name: &str) -> Option<PathBuf> {
    let candidate = Path::new(name);
    if candidate.components().count() > 1 {
        return candidate.is_file().then(|| candidate.to_owned());
    }
    // Managed tool directories come before the system PATH.
    let paths = crate::tool_env::tool_environment().joined_path(std::env::var_os("PATH"))?;
    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.BAT;.CMD".into())
            .split(';')
            .map(str::to_owned)
            .chain(std::iter::once(String::new()))
            .collect()
    } else {
        vec![String::new()]
    };
    std::env::split_paths(&paths).find_map(|dir| {
        extensions.iter().find_map(|extension| {
            let path = dir.join(format!("{name}{extension}"));
            is_executable(&path).then_some(path)
        })
    })
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// Resolves a tool on `PATH` or returns an actionable error.
pub fn require_program(name: &str, hint: &str) -> Result<String, RunnerError> {
    find_program(name)
        .map(|found| {
            // Managed installs are not on the system PATH: run them by full path.
            if crate::tool_env::tool_environment()
                .path_dirs
                .iter()
                .any(|dir| found.starts_with(dir))
            {
                found.display().to_string()
            } else {
                name.to_owned()
            }
        })
        .ok_or_else(|| RunnerError::ToolNotFound {
            program: name.into(),
            hint: hint.into(),
        })
}

fn plan(
    kind: TaskKind,
    name: &str,
    program: String,
    args: &[&str],
    root: &Path,
) -> Result<TaskPlan, RunnerError> {
    Ok(TaskPlan::new(name, program, args.iter().copied(), root).with_kind(kind))
}

/// `base`, then `options`, then `--` and `program_arguments` when there are any.
fn with_program_arguments(
    mut base: Vec<String>,
    options: &[String],
    program_arguments: &[String],
) -> Vec<String> {
    base.extend(options.iter().cloned());
    if !program_arguments.is_empty() {
        base.push("--".into());
        base.extend(program_arguments.iter().cloned());
    }
    base
}

pub struct Cargo;

impl BuildSystem for Cargo {
    fn kind(&self) -> BuildSystemKind {
        BuildSystemKind::Cargo
    }
    fn detect(&self, root: &Path) -> bool {
        root.join("Cargo.toml").is_file()
    }
    fn build_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        let cargo = require_program("cargo", "Install Rust with rustup (https://rustup.rs).")?;
        plan(TaskKind::Build, "Cargo Build", cargo, &["build"], root)
    }
    fn run_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        if !cargo_has_binary(root) {
            // Libraries have nothing to launch; running their tests is the useful default.
            return self.test_plan(root);
        }
        let cargo = require_program("cargo", "Install Rust with rustup (https://rustup.rs).")?;
        plan(TaskKind::Run, "Cargo Run", cargo, &["run"], root)
    }
    fn test_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        let cargo = require_program("cargo", "Install Rust with rustup (https://rustup.rs).")?;
        plan(TaskKind::Test, "Cargo Test", cargo, &["test"], root)
    }
    fn configuration_plan(
        &self,
        root: &Path,
        config: &RunConfiguration,
    ) -> Result<TaskPlan, RunnerError> {
        let RunTarget::Cargo {
            command,
            package,
            bin,
            options,
            program_arguments,
        } = &config.target
        else {
            return Err(RunnerError::NotRunnable("not a Cargo configuration".into()));
        };
        let cargo = require_program("cargo", "Install Rust with rustup (https://rustup.rs).")?;
        let mut base = vec![command.clone()];
        if let Some(package) = package.as_ref().filter(|p| !p.is_empty()) {
            base.extend(["-p".into(), package.clone()]);
        }
        if let Some(bin) = bin.as_ref().filter(|b| !b.is_empty()) {
            base.extend(["--bin".into(), bin.clone()]);
        }
        let args = with_program_arguments(base, options, program_arguments);
        Ok(TaskPlan::new(&config.name, cargo, args, root))
    }
    fn rerun_failed_plan(
        &self,
        _root: &Path,
        plan: &TaskPlan,
        failed: &[TestId],
    ) -> Result<TaskPlan, RunnerError> {
        if failed.is_empty() {
            return Err(RunnerError::NotRunnable("no failed tests".into()));
        }
        let (cargo_args, test_args) = split_at_separator(&plan.args);
        let mut args = cargo_args.to_vec();
        args.push("--".into());
        // Keep harness options (e.g. --nocapture) but replace name filters.
        args.extend(test_args.iter().filter(|arg| arg.starts_with('-')).cloned());
        if !args.iter().any(|arg| arg == "--exact") {
            args.push("--exact".into());
        }
        args.extend(failed.iter().map(|test| match test.suite.as_str() {
            "(top level)" => test.name.clone(),
            suite => format!("{suite}::{}", test.name),
        }));
        Ok(derive_plan(plan, args))
    }
}

fn cargo_has_binary(root: &Path) -> bool {
    let manifest = std::fs::read_to_string(root.join("Cargo.toml"))
        .ok()
        .and_then(|source| source.parse::<toml::Value>().ok());
    root.join("src/main.rs").is_file()
        || root.join("src/bin").is_dir()
        || manifest
            .as_ref()
            .and_then(|manifest| manifest.get("bin"))
            .and_then(toml::Value::as_array)
            .is_some_and(|bins| !bins.is_empty())
}

pub struct Maven;

impl Maven {
    pub fn program(root: &Path) -> Result<String, RunnerError> {
        let wrapper = if cfg!(windows) { "mvnw.cmd" } else { "mvnw" };
        if root.join(wrapper).is_file() {
            return Ok(if cfg!(windows) {
                root.join(wrapper).display().to_string()
            } else {
                format!("./{wrapper}")
            });
        }
        require_program(
            "mvn",
            "Install Apache Maven and add it to PATH, or add the Maven wrapper (mvnw).",
        )
    }
}

impl BuildSystem for Maven {
    fn kind(&self) -> BuildSystemKind {
        BuildSystemKind::Maven
    }
    fn detect(&self, root: &Path) -> bool {
        root.join("pom.xml").is_file()
    }
    fn build_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        plan(
            TaskKind::Build,
            "Maven Compile",
            Self::program(root)?,
            &["compile"],
            root,
        )
    }
    fn run_plan(&self, _root: &Path) -> Result<TaskPlan, RunnerError> {
        Err(RunnerError::NotRunnable(
            "Maven projects have no standard run goal; create a run configuration".into(),
        ))
    }
    fn test_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        plan(
            TaskKind::Test,
            "Maven Test",
            Self::program(root)?,
            &["test"],
            root,
        )
    }
    fn configuration_plan(
        &self,
        root: &Path,
        config: &RunConfiguration,
    ) -> Result<TaskPlan, RunnerError> {
        let RunTarget::Maven { goals, options } = &config.target else {
            return Err(RunnerError::NotRunnable("not a Maven configuration".into()));
        };
        let args = goals.iter().chain(options).cloned();
        Ok(TaskPlan::new(
            &config.name,
            Self::program(root)?,
            args,
            root,
        ))
    }
    fn rerun_failed_plan(
        &self,
        _root: &Path,
        plan: &TaskPlan,
        failed: &[TestId],
    ) -> Result<TaskPlan, RunnerError> {
        if failed.is_empty() {
            return Err(RunnerError::NotRunnable("no failed tests".into()));
        }
        let filter = failed
            .iter()
            .map(|test| format!("{}#{}", test.suite, java_method_name(&test.name)))
            .collect::<Vec<_>>()
            .join(",");
        let mut args: Vec<String> = plan
            .args
            .iter()
            .filter(|arg| !arg.starts_with("-Dtest="))
            .cloned()
            .collect();
        args.push(format!("-Dtest={filter}"));
        args.push("-Dsurefire.failIfNoSpecifiedTests=false".into());
        Ok(derive_plan(plan, args))
    }
}

/// JUnit 5 reports `greets()` or `adds(int)[1]`; filters need the bare method name.
pub fn java_method_name(name: &str) -> &str {
    name.split(['(', '[']).next().unwrap_or(name).trim()
}

pub struct Bazel;

impl Bazel {
    pub fn is_workspace(root: &Path) -> bool {
        ["MODULE.bazel", "WORKSPACE", "WORKSPACE.bazel"]
            .iter()
            .any(|name| root.join(name).is_file())
    }

    /// Prefers `bazelisk`, which honours `.bazelversion`, then `bazel`.
    pub fn program() -> Result<String, RunnerError> {
        if find_program("bazelisk").is_some() {
            return Ok("bazelisk".into());
        }
        require_program(
            "bazel",
            "Install Bazelisk (https://github.com/bazelbuild/bazelisk) or Bazel and add it to PATH.",
        )
    }
}

impl BuildSystem for Bazel {
    fn kind(&self) -> BuildSystemKind {
        BuildSystemKind::Bazel
    }
    fn detect(&self, root: &Path) -> bool {
        Self::is_workspace(root)
    }
    fn build_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        plan(
            TaskKind::Build,
            "Bazel Build",
            Self::program()?,
            &["build", "//..."],
            root,
        )
    }
    fn run_plan(&self, _root: &Path) -> Result<TaskPlan, RunnerError> {
        Err(RunnerError::NotRunnable(
            "Bazel runs a specific target: pick one in the Bazel tool window or create a run configuration with a target such as //:app"
                .into(),
        ))
    }
    fn test_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        plan(
            TaskKind::Test,
            "Bazel Test",
            Self::program()?,
            // Keep going so one broken package does not hide every other result.
            &["test", "--keep_going", "//..."],
            root,
        )
    }
    fn configuration_plan(
        &self,
        root: &Path,
        config: &RunConfiguration,
    ) -> Result<TaskPlan, RunnerError> {
        let RunTarget::Bazel {
            command,
            target,
            options,
            program_arguments,
        } = &config.target
        else {
            return Err(RunnerError::NotRunnable("not a Bazel configuration".into()));
        };
        let base = vec![command.clone(), target.clone()];
        let args = with_program_arguments(base, options, program_arguments);
        Ok(TaskPlan::new(&config.name, Self::program()?, args, root))
    }
    fn rerun_failed_plan(
        &self,
        _root: &Path,
        plan: &TaskPlan,
        failed: &[TestId],
    ) -> Result<TaskPlan, RunnerError> {
        // Bazel reports one suite per test target; re-test those targets.
        let mut targets: Vec<String> = failed
            .iter()
            .map(|test| test.suite.clone())
            .filter(|label| label.starts_with("//") || label.starts_with('@'))
            .collect();
        targets.dedup();
        if targets.is_empty() {
            return Err(RunnerError::NotRunnable(
                "no failed Bazel test targets".into(),
            ));
        }
        let options = plan
            .args
            .iter()
            .skip(1)
            .filter(|arg| arg.starts_with('-'))
            .cloned();
        let args = std::iter::once("test".to_owned())
            .chain(options)
            .chain(targets)
            .collect();
        Ok(derive_plan(plan, args))
    }
}

pub struct Npm;

impl BuildSystem for Npm {
    fn kind(&self) -> BuildSystemKind {
        BuildSystemKind::Npm
    }
    fn detect(&self, root: &Path) -> bool {
        root.join("package.json").is_file()
    }
    fn build_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        let npm = require_program("npm", "Install Node.js (https://nodejs.org) and npm.")?;
        if npm_script(root, "build") {
            plan(TaskKind::Build, "npm Build", npm, &["run", "build"], root)
        } else {
            plan(TaskKind::Build, "npm Install", npm, &["install"], root)
        }
    }
    fn run_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        let npm = require_program("npm", "Install Node.js (https://nodejs.org) and npm.")?;
        plan(TaskKind::Run, "npm Start", npm, &["start"], root)
    }
    fn test_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        let npm = require_program("npm", "Install Node.js (https://nodejs.org) and npm.")?;
        plan(TaskKind::Test, "npm Test", npm, &["test"], root)
    }
    fn configuration_plan(
        &self,
        root: &Path,
        config: &RunConfiguration,
    ) -> Result<TaskPlan, RunnerError> {
        let RunTarget::Npm {
            script,
            program_arguments,
        } = &config.target
        else {
            return Err(RunnerError::NotRunnable("not an npm configuration".into()));
        };
        let npm = require_program("npm", "Install Node.js (https://nodejs.org) and npm.")?;
        let base = vec!["run".into(), script.clone()];
        let args = with_program_arguments(base, &[], program_arguments);
        Ok(TaskPlan::new(&config.name, npm, args, root))
    }
}

fn npm_script(root: &Path, name: &str) -> bool {
    std::fs::read_to_string(root.join("package.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .is_some_and(|package| package["scripts"][name].is_string())
}

pub struct Go;

impl BuildSystem for Go {
    fn kind(&self) -> BuildSystemKind {
        BuildSystemKind::Go
    }
    fn detect(&self, root: &Path) -> bool {
        root.join("go.mod").is_file()
    }
    fn build_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        let go = require_program("go", "Install Go (https://go.dev/dl) and add it to PATH.")?;
        plan(TaskKind::Build, "Go Build", go, &["build", "./..."], root)
    }
    fn run_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        let go = require_program("go", "Install Go (https://go.dev/dl) and add it to PATH.")?;
        plan(TaskKind::Run, "Go Run", go, &["run", "."], root)
    }
    fn test_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        let go = require_program("go", "Install Go (https://go.dev/dl) and add it to PATH.")?;
        plan(TaskKind::Test, "Go Test", go, &["test", "./..."], root)
    }
    fn configuration_plan(
        &self,
        root: &Path,
        config: &RunConfiguration,
    ) -> Result<TaskPlan, RunnerError> {
        let RunTarget::Go {
            command,
            package,
            program_arguments,
        } = &config.target
        else {
            return Err(RunnerError::NotRunnable("not a Go configuration".into()));
        };
        let go = require_program("go", "Install Go (https://go.dev/dl) and add it to PATH.")?;
        let base = vec![command.clone(), package.clone()];
        // `go run pkg args…` takes program arguments directly.
        let args = base.into_iter().chain(program_arguments.iter().cloned());
        Ok(TaskPlan::new(&config.name, go, args, root))
    }
    fn rerun_failed_plan(
        &self,
        _root: &Path,
        plan: &TaskPlan,
        failed: &[TestId],
    ) -> Result<TaskPlan, RunnerError> {
        if failed.is_empty() {
            return Err(RunnerError::NotRunnable("no failed tests".into()));
        }
        let names: Vec<&str> = failed.iter().map(|test| test.name.as_str()).collect();
        let mut packages: Vec<String> = failed.iter().map(|test| test.suite.clone()).collect();
        packages.dedup();
        let args = ["test".to_owned(), "-v".to_owned(), "-run".to_owned()]
            .into_iter()
            .chain([format!("^({})$", names.join("|"))])
            .chain(packages)
            .collect();
        Ok(derive_plan(plan, args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-build-system-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn registry_detects_in_order_and_accepts_overrides() {
        let root = temp("order");
        std::fs::write(root.join("MODULE.bazel"), "").unwrap();
        std::fs::write(root.join("package.json"), "{}").unwrap();
        let mut registry = BuildSystemRegistry::default();
        assert_eq!(
            registry.detect(&root).unwrap().kind(),
            BuildSystemKind::Bazel
        );
        registry.register_first(Box::new(Npm));
        assert_eq!(registry.detect(&root).unwrap().kind(), BuildSystemKind::Npm);
        assert!(BuildSystemRegistry::empty().detect(&root).is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bazel_and_maven_refuse_to_guess_a_run_target() {
        let root = temp("bazel");
        std::fs::write(root.join("WORKSPACE"), "").unwrap();
        assert!(Bazel.detect(&root));
        let error = Bazel.run_plan(&root).unwrap_err().to_string();
        assert!(error.contains("//:app"), "{error}");
        assert!(matches!(
            Maven.run_plan(&root),
            Err(RunnerError::NotRunnable(_))
        ));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn configurations_are_planned_by_their_provider() {
        let registry = BuildSystemRegistry::default();
        let root = Path::new("/w");
        let shell = RunConfiguration::new(
            "Echo",
            RunTarget::Shell {
                program: "echo".into(),
                arguments: vec!["hi".into()],
            },
        );
        let plan = registry.plan_configuration(root, &shell).unwrap();
        assert_eq!((plan.program.as_str(), plan.kind), ("echo", TaskKind::Run));
        if find_program("cargo").is_some() {
            let cargo = RunConfiguration::new(
                "Server",
                RunTarget::Cargo {
                    command: "run".into(),
                    package: Some("server".into()),
                    bin: None,
                    options: vec!["--release".into()],
                    program_arguments: vec!["--port".into(), "80".into()],
                },
            );
            let plan = registry.plan_configuration(root, &cargo).unwrap();
            assert_eq!(
                plan.args,
                ["run", "-p", "server", "--release", "--", "--port", "80"]
            );
            assert_eq!(plan.name, "Server");
        }
        let gradle = RunConfiguration::new("G", RunTarget::template("gradle").unwrap());
        let error = registry.plan_configuration(root, &gradle).unwrap_err();
        assert!(
            error.to_string().contains("no Gradle integration"),
            "{error}"
        );
        let maven = RunConfiguration::new(
            "Verify",
            RunTarget::Maven {
                goals: vec!["clean".into(), "verify".into()],
                options: vec!["-q".into()],
            },
        );
        match registry.plan_configuration(root, &maven) {
            Ok(plan) => assert_eq!(plan.args, ["clean", "verify", "-q"]),
            Err(error) => assert!(matches!(error, RunnerError::ToolNotFound { .. }), "{error}"),
        }
        let bazel = RunConfiguration::new(
            "App",
            RunTarget::Bazel {
                command: "run".into(),
                target: "//:app".into(),
                options: Vec::new(),
                program_arguments: vec!["x".into()],
            },
        );
        match registry.plan_configuration(root, &bazel) {
            Ok(plan) => assert_eq!(plan.args, ["run", "//:app", "--", "x"]),
            Err(error) => assert!(error.to_string().contains("Bazel"), "{error}"),
        }
        let invalid = RunConfiguration::new("", RunTarget::template("shell").unwrap());
        assert!(registry.plan_configuration(root, &invalid).is_err());
    }

    #[test]
    fn rerun_failed_plans_filter_to_failed_tests() {
        let failed = [
            TestId {
                suite: "tests::math".into(),
                name: "subtracts".into(),
            },
            TestId {
                suite: "(top level)".into(),
                name: "smoke".into(),
            },
        ];
        let plan = TaskPlan::new(
            "Cargo Test",
            "cargo",
            ["test", "-p", "core", "--", "--nocapture", "old"],
            "/w",
        )
        .with_kind(TaskKind::Test);
        let rerun = Cargo
            .rerun_failed_plan(Path::new("/w"), &plan, &failed)
            .unwrap();
        assert_eq!(
            rerun.args,
            [
                "test",
                "-p",
                "core",
                "--",
                "--nocapture",
                "--exact",
                "tests::math::subtracts",
                "smoke"
            ]
        );
        let go = TaskPlan::new("Go Test", "go", ["test", "./..."], "/w");
        let go_failed = [TestId {
            suite: "example.com/calc".into(),
            name: "TestSub".into(),
        }];
        assert_eq!(
            Go.rerun_failed_plan(Path::new("/w"), &go, &go_failed)
                .unwrap()
                .args,
            ["test", "-v", "-run", "^(TestSub)$", "example.com/calc"]
        );
        let mvn = TaskPlan::new("Maven Test", "mvn", ["test"], "/w");
        let java_failed = [TestId {
            suite: "com.example.AppTest".into(),
            name: "adds(int)[1]".into(),
        }];
        assert_eq!(
            Maven
                .rerun_failed_plan(Path::new("/w"), &mvn, &java_failed)
                .unwrap()
                .args,
            [
                "test",
                "-Dtest=com.example.AppTest#adds",
                "-Dsurefire.failIfNoSpecifiedTests=false"
            ]
        );
        let bazel = TaskPlan::new(
            "Bazel Test",
            "bazel",
            ["test", "--config=ci", "//..."],
            "/w",
        );
        let bazel_failed = [TestId {
            suite: "//:app_test".into(),
            name: "//:app_test".into(),
        }];
        assert_eq!(
            Bazel
                .rerun_failed_plan(Path::new("/w"), &bazel, &bazel_failed)
                .unwrap()
                .args,
            ["test", "--config=ci", "//:app_test"]
        );
        assert!(
            Npm.rerun_failed_plan(Path::new("/w"), &plan, &failed)
                .is_err()
        );
    }

    #[test]
    fn missing_tools_produce_actionable_errors() {
        let error =
            require_program("archipel-ide-definitely-missing-tool", "Install it.").unwrap_err();
        assert_eq!(
            error.to_string(),
            "`archipel-ide-definitely-missing-tool` was not found on PATH. Install it."
        );
    }

    #[test]
    fn npm_build_uses_build_script_when_present() {
        if find_program("npm").is_none() {
            eprintln!("skipping: npm is not installed");
            return;
        }
        let root = temp("npm");
        std::fs::write(root.join("package.json"), r#"{"scripts":{"build":"tsc"}}"#).unwrap();
        assert_eq!(Npm.build_plan(&root).unwrap().args, ["run", "build"]);
        std::fs::write(root.join("package.json"), "{}").unwrap();
        assert_eq!(Npm.build_plan(&root).unwrap().args, ["install"]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cargo_library_runs_tests() {
        if find_program("cargo").is_none() {
            eprintln!("skipping: cargo is not installed");
            return;
        }
        let root = temp("cargo");
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='x'\nversion='0.1.0'\n",
        )
        .unwrap();
        let run = Cargo.run_plan(&root).unwrap();
        assert_eq!(
            (run.name.as_str(), run.kind),
            ("Cargo Test", TaskKind::Test)
        );
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        assert_eq!(Cargo.run_plan(&root).unwrap().args, ["run"]);
        std::fs::remove_dir_all(root).unwrap();
    }
}

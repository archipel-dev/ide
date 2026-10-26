//! Run configurations: named, persistent launch descriptions.
//!
//! Each configuration is stored as one TOML file in `.archipel/runConfigurations/`
//! (shareable through VCS like the reference IDE's "Store as project file"). The
//! build-system provider of the configuration's type turns it into a [`TaskPlan`].

use crate::{BuildSystemKind, RunnerError, TaskKind, TaskPlan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const RUN_CONFIGURATIONS_DIR: &str = ".archipel/runConfigurations";

/// What a configuration launches. The `type` key selects the variant in TOML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RunTarget {
    /// Gradle tasks (`:app:run`), extra command-line options, and program arguments
    /// (passed with `--args` to `run`/`JavaExec` tasks).
    Gradle {
        tasks: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        options: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        program_arguments: Vec<String>,
    },
    /// `cargo <command> [-p package] [--bin bin] [options] [-- program arguments]`.
    Cargo {
        command: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        package: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bin: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        options: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        program_arguments: Vec<String>,
    },
    /// `bazel <command> <target> [options] [-- program arguments]`.
    Bazel {
        command: String,
        target: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        options: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        program_arguments: Vec<String>,
    },
    /// `mvn <goals> [options]`.
    Maven {
        goals: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        options: Vec<String>,
    },
    /// `npm run <script> [-- arguments]`.
    Npm {
        script: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        program_arguments: Vec<String>,
    },
    /// `go <command> <package> [-- arguments]`.
    Go {
        command: String,
        package: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        program_arguments: Vec<String>,
    },
    /// A JVM main class run on a module's classpath (planned through the Gradle
    /// integration, like the gutter's `Run 'App.main()'`).
    Application {
        /// Gradle project path of the module (`:app`); the root project when empty.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        module: String,
        /// JDK home the program runs on (`JAVA_HOME`); the configured Gradle JDK when empty.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        jdk: String,
        main_class: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        program_arguments: Vec<String>,
    },
    /// A Spring Boot application, run through the project's build tool: `bootRun`
    /// (Gradle), `spring-boot:run` (Maven) or `bazel run` (see [`RunTarget::resolve_spring`]).
    SpringBoot {
        /// `gradle`, `maven` or `bazel`.
        build: String,
        /// Gradle project path (`:app`; the root project when empty), Maven module
        /// directory relative to the project (the root when empty) or Bazel label
        /// (`//:app`).
        #[serde(default, skip_serializing_if = "String::is_empty")]
        target: String,
        /// Main class to start when the project has several; the build tool decides when empty.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        main_class: String,
        /// Comma separated `spring.profiles.active`.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        profiles: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        program_arguments: Vec<String>,
    },
    /// Any executable.
    Shell {
        program: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        arguments: Vec<String>,
    },
}

impl RunTarget {
    /// Provider that plans this target; `None` for shell commands.
    pub fn build_system(&self) -> Option<BuildSystemKind> {
        match self {
            RunTarget::Gradle { .. } | RunTarget::Application { .. } => {
                Some(BuildSystemKind::Gradle)
            }
            RunTarget::SpringBoot { build, .. } => match build.as_str() {
                "gradle" => Some(BuildSystemKind::Gradle),
                "maven" => Some(BuildSystemKind::Maven),
                "bazel" => Some(BuildSystemKind::Bazel),
                _ => None,
            },
            RunTarget::Cargo { .. } => Some(BuildSystemKind::Cargo),
            RunTarget::Bazel { .. } => Some(BuildSystemKind::Bazel),
            RunTarget::Maven { .. } => Some(BuildSystemKind::Maven),
            RunTarget::Npm { .. } => Some(BuildSystemKind::Npm),
            RunTarget::Go { .. } => Some(BuildSystemKind::Go),
            RunTarget::Shell { .. } => None,
        }
    }

    pub fn type_id(&self) -> &'static str {
        match self {
            RunTarget::Gradle { .. } => "gradle",
            RunTarget::Application { .. } => "application",
            RunTarget::SpringBoot { .. } => "springboot",
            RunTarget::Cargo { .. } => "cargo",
            RunTarget::Bazel { .. } => "bazel",
            RunTarget::Maven { .. } => "maven",
            RunTarget::Npm { .. } => "npm",
            RunTarget::Go { .. } => "go",
            RunTarget::Shell { .. } => "shell",
        }
    }

    /// Display name of the configuration type (the dialog's group label).
    pub fn type_name(&self) -> &'static str {
        match self {
            RunTarget::Shell { .. } => "Shell Command",
            RunTarget::Application { .. } => "Application",
            RunTarget::SpringBoot { .. } => "Spring Boot",
            other => other
                .build_system()
                .map(BuildSystemKind::display_name)
                .unwrap_or("Shell Command"),
        }
    }

    /// A blank target of the given type id, used by the "+" menu.
    pub fn template(type_id: &str) -> Option<Self> {
        Some(match type_id {
            "gradle" => RunTarget::Gradle {
                tasks: vec!["run".into()],
                options: Vec::new(),
                program_arguments: Vec::new(),
            },
            "cargo" => RunTarget::Cargo {
                command: "run".into(),
                package: None,
                bin: None,
                options: Vec::new(),
                program_arguments: Vec::new(),
            },
            "bazel" => RunTarget::Bazel {
                command: "run".into(),
                target: "//:app".into(),
                options: Vec::new(),
                program_arguments: Vec::new(),
            },
            "maven" => RunTarget::Maven {
                goals: vec!["verify".into()],
                options: Vec::new(),
            },
            "npm" => RunTarget::Npm {
                script: "start".into(),
                program_arguments: Vec::new(),
            },
            "go" => RunTarget::Go {
                command: "run".into(),
                package: ".".into(),
                program_arguments: Vec::new(),
            },
            "application" => RunTarget::Application {
                module: String::new(),
                jdk: String::new(),
                main_class: String::new(),
                program_arguments: Vec::new(),
            },
            "springboot" => RunTarget::SpringBoot {
                build: "gradle".into(),
                target: String::new(),
                main_class: String::new(),
                profiles: String::new(),
                program_arguments: Vec::new(),
            },
            "shell" => RunTarget::Shell {
                program: String::new(),
                arguments: Vec::new(),
            },
            _ => return None,
        })
    }

    /// The concrete Gradle, Maven or Bazel target a Spring Boot configuration runs as;
    /// `None` for every other type.
    ///
    /// Gradle runs `bootRun` of the project, Maven `spring-boot:run` (`-f` selects a
    /// module, `-Dspring-boot.run.profiles` the profiles), Bazel `run` of the label.
    /// Profiles become `--spring.profiles.active=…` where arguments are passed to the
    /// application.
    pub fn resolve_spring(&self) -> Option<RunTarget> {
        let RunTarget::SpringBoot {
            build,
            target,
            main_class,
            profiles,
            program_arguments,
        } = self
        else {
            return None;
        };
        let profiles = profiles.trim();
        let mut application_arguments = Vec::new();
        if !profiles.is_empty() {
            application_arguments.push(format!("--spring.profiles.active={profiles}"));
        }
        application_arguments.extend(program_arguments.iter().cloned());
        Some(match build.as_str() {
            "maven" => {
                let mut options = Vec::new();
                if !target.trim().is_empty() {
                    options.push("-f".to_owned());
                    options.push(format!("{}/pom.xml", target.trim().trim_end_matches('/')));
                }
                if !main_class.trim().is_empty() {
                    options.push(format!(
                        "-Dspring-boot.run.main-class={}",
                        main_class.trim()
                    ));
                }
                if !profiles.is_empty() {
                    options.push(format!("-Dspring-boot.run.profiles={profiles}"));
                }
                if !program_arguments.is_empty() {
                    options.push(format!(
                        "-Dspring-boot.run.arguments={}",
                        program_arguments.join(" ")
                    ));
                }
                RunTarget::Maven {
                    goals: vec!["spring-boot:run".into()],
                    options,
                }
            }
            "bazel" => RunTarget::Bazel {
                command: "run".into(),
                target: if target.trim().is_empty() {
                    "//:app".into()
                } else {
                    target.trim().to_owned()
                },
                options: Vec::new(),
                program_arguments: application_arguments,
            },
            _ => {
                let project = target.trim().trim_end_matches(':');
                RunTarget::Gradle {
                    tasks: vec![if project.is_empty() {
                        "bootRun".into()
                    } else {
                        format!("{project}:bootRun")
                    }],
                    options: Vec::new(),
                    program_arguments: application_arguments,
                }
            }
        })
    }

    pub const TYPE_IDS: [&'static str; 9] = [
        "application",
        "springboot",
        "gradle",
        "cargo",
        "bazel",
        "maven",
        "npm",
        "go",
        "shell",
    ];

    /// Kind of task this target runs (decides Run vs Test vs Build window title).
    pub fn task_kind(&self) -> TaskKind {
        let verb = match self {
            RunTarget::Gradle { tasks, .. } => tasks
                .first()
                .map(|task| task.rsplit(':').next().unwrap_or(task).to_owned())
                .unwrap_or_default(),
            RunTarget::Cargo { command, .. }
            | RunTarget::Bazel { command, .. }
            | RunTarget::Go { command, .. } => command.clone(),
            RunTarget::Maven { goals, .. } => goals.last().cloned().unwrap_or_default(),
            RunTarget::Npm { script, .. } => script.clone(),
            RunTarget::Application { .. }
            | RunTarget::SpringBoot { .. }
            | RunTarget::Shell { .. } => "run".into(),
        };
        match verb.as_str() {
            "test" | "check" | "verify" => TaskKind::Test,
            "build" | "assemble" | "compile" | "package" | "install" => TaskKind::Build,
            _ => TaskKind::Run,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RunConfiguration {
    pub name: String,
    #[serde(flatten)]
    pub target: RunTarget,
    /// Relative to the project root; the root when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Temporary configurations (from a Gradle task double-click or a gutter icon)
    /// are listed but not written to disk until saved.
    #[serde(skip)]
    pub temporary: bool,
    /// Show the Run tool window when the configuration starts (default).
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub open_tool_window: bool,
}

fn yes() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

impl RunConfiguration {
    pub fn new(name: impl Into<String>, target: RunTarget) -> Self {
        Self {
            name: name.into(),
            target,
            working_directory: None,
            env: BTreeMap::new(),
            temporary: false,
            open_tool_window: true,
        }
    }

    /// Validates user-editable fields; errors are phrased for the dialog.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Name must not be empty".into());
        }
        if let Some(dir) = &self.working_directory
            && (dir.is_absolute()
                || dir
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir)))
        {
            return Err("Working directory must be inside the project".into());
        }
        for key in self.env.keys() {
            if key.is_empty() || key.contains('=') || key.contains('\0') {
                return Err(format!("Invalid environment variable name `{key}`"));
            }
        }
        match &self.target {
            RunTarget::Gradle { tasks, .. } if tasks.iter().all(|t| t.trim().is_empty()) => {
                Err("Enter at least one Gradle task".into())
            }
            RunTarget::Maven { goals, .. } if goals.iter().all(|g| g.trim().is_empty()) => {
                Err("Enter at least one Maven goal".into())
            }
            RunTarget::Bazel { target, .. }
                if !target.starts_with("//")
                    && !target.starts_with('@')
                    && !target.starts_with(':') =>
            {
                Err("Bazel target must be a label such as //:app".into())
            }
            RunTarget::Cargo { command, .. }
            | RunTarget::Bazel { command, .. }
            | RunTarget::Go { command, .. }
                if command.trim().is_empty() || command.contains(char::is_whitespace) =>
            {
                Err("Command must be a single word such as run or test".into())
            }
            RunTarget::Npm { script, .. } if script.trim().is_empty() => {
                Err("Enter the npm script to run".into())
            }
            RunTarget::Application { main_class, .. } if main_class.trim().is_empty() => {
                Err("Enter the main class to run".into())
            }
            RunTarget::SpringBoot { build, .. }
                if !matches!(build.as_str(), "gradle" | "maven" | "bazel") =>
            {
                Err("Build tool must be gradle, maven or bazel".into())
            }
            RunTarget::SpringBoot { profiles, .. }
                if !profiles
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, ',' | '-' | '_' | '.' | '!')) =>
            {
                Err("Profiles are names separated by commas, such as dev,local".into())
            }
            RunTarget::SpringBoot { build, target, .. }
                if build == "bazel"
                    && !target.is_empty()
                    && !target.starts_with("//")
                    && !target.starts_with('@')
                    && !target.starts_with(':') =>
            {
                Err("Bazel target must be a label such as //:app".into())
            }
            RunTarget::Shell { program, .. } if program.trim().is_empty() => {
                Err("Enter the program to run".into())
            }
            _ => Ok(()),
        }
    }

    /// Directory the task runs in.
    pub fn cwd(&self, root: &Path) -> PathBuf {
        match &self.working_directory {
            Some(dir) => root.join(dir),
            None => root.to_owned(),
        }
    }

    /// Applies name, kind, cwd and environment to a plan produced by a provider.
    pub fn finish_plan(&self, plan: TaskPlan, root: &Path) -> TaskPlan {
        let mut plan = plan.with_kind(self.target.task_kind());
        plan.name = self.name.clone();
        plan.cwd = self.cwd(root);
        plan.env
            .extend(self.env.iter().map(|(k, v)| (k.clone(), v.clone())));
        plan
    }

    /// Plan for a shell configuration (other types are planned by their provider).
    pub fn shell_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        match &self.target {
            RunTarget::Shell { program, arguments } => Ok(self.finish_plan(
                TaskPlan::new(&self.name, program, arguments.iter().cloned(), root),
                root,
            )),
            _ => Err(RunnerError::NotRunnable(format!(
                "{} configurations are planned by the {} integration",
                self.target.type_name(),
                self.target.type_name()
            ))),
        }
    }

    pub fn file_name(&self) -> String {
        let stem: String = self
            .name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let stem = stem.trim_matches('_');
        if stem.is_empty() {
            "configuration.toml".into()
        } else {
            format!("{stem}.toml")
        }
    }
}

/// Loaded configurations of a project plus the selection of the run widget.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunManager {
    configurations: Vec<RunConfiguration>,
    selected: Option<String>,
    /// Files that failed to parse, with the reason.
    pub errors: Vec<(PathBuf, String)>,
}

impl RunManager {
    /// Loads `*.toml` from the project's configuration directory. Unreadable files
    /// are reported in [`RunManager::errors`] and skipped.
    pub fn load(root: &Path) -> Self {
        let mut manager = Self::default();
        let dir = archipel_ide_compat::project_read_path(root.join(RUN_CONFIGURATIONS_DIR));
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return manager;
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
            .collect();
        paths.sort();
        for path in paths {
            let parsed = std::fs::read_to_string(&path)
                .map_err(|error| error.to_string())
                .and_then(|text| {
                    toml::from_str::<RunConfiguration>(&text).map_err(|error| error.to_string())
                })
                .and_then(|config| config.validate().map(|()| config));
            match parsed {
                Ok(config) if manager.get(&config.name).is_none() => {
                    manager.configurations.push(config)
                }
                Ok(config) => manager.errors.push((
                    path,
                    format!("duplicate configuration name `{}`", config.name),
                )),
                Err(error) => manager.errors.push((path, error)),
            }
        }
        manager.selected = manager.configurations.first().map(|c| c.name.clone());
        manager
    }

    pub fn configurations(&self) -> &[RunConfiguration] {
        &self.configurations
    }

    pub fn get(&self, name: &str) -> Option<&RunConfiguration> {
        self.configurations
            .iter()
            .find(|config| config.name == name)
    }

    pub fn selected(&self) -> Option<&RunConfiguration> {
        self.selected.as_deref().and_then(|name| self.get(name))
    }

    pub fn select(&mut self, name: &str) -> Result<(), String> {
        if self.get(name).is_none() {
            return Err(format!("no run configuration named `{name}`"));
        }
        self.selected = Some(name.to_owned());
        Ok(())
    }

    /// Deselects the configuration (the run widget falls back to `Current File`).
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// Adds or replaces a temporary configuration and selects it (reference behaviour
    /// when running something without an existing configuration).
    pub fn add_temporary(&mut self, mut config: RunConfiguration) {
        config.temporary = true;
        let name = config.name.clone();
        match self.configurations.iter_mut().find(|c| c.name == name) {
            Some(existing) if existing.temporary => *existing = config,
            Some(_) => {}
            None => self.configurations.push(config),
        }
        self.selected = Some(name);
    }

    /// Saves `config` to disk, replacing the configuration called `previous_name`
    /// (renames delete the old file). The saved configuration becomes selected.
    pub fn save(
        &mut self,
        root: &Path,
        previous_name: Option<&str>,
        mut config: RunConfiguration,
    ) -> Result<(), String> {
        config.validate()?;
        config.temporary = false;
        if let Some(clash) = self.get(&config.name)
            && Some(clash.name.as_str()) != previous_name
            && !clash.temporary
        {
            return Err(format!(
                "a configuration named `{}` already exists",
                config.name
            ));
        }
        // The first save in a project from before the rename copies `.jetthing/` to `.archipel/`.
        let dir = archipel_ide_compat::project_write_path(root.join(RUN_CONFIGURATIONS_DIR));
        std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
        let text = toml::to_string_pretty(&config).map_err(|error| error.to_string())?;
        // Drop the file of the configuration being replaced (also on rename).
        if let Some(old) = self.path_of(root, previous_name.unwrap_or(&config.name)) {
            std::fs::remove_file(old).map_err(|error| error.to_string())?;
        }
        // Distinct names can sanitise to the same file name; keep both.
        let stem = config.file_name().trim_end_matches(".toml").to_owned();
        let mut file_name = format!("{stem}.toml");
        let mut counter = 2;
        while dir.join(&file_name).exists() {
            file_name = format!("{stem}-{counter}.toml");
            counter += 1;
        }
        std::fs::write(dir.join(&file_name), text).map_err(|error| error.to_string())?;
        self.configurations.retain(|existing| {
            existing.name != config.name && Some(existing.name.as_str()) != previous_name
        });
        self.selected = Some(config.name.clone());
        self.configurations.push(config);
        self.sort();
        Ok(())
    }

    pub fn delete(&mut self, root: &Path, name: &str) -> Result<(), String> {
        let config = self
            .get(name)
            .cloned()
            .ok_or_else(|| format!("no run configuration named `{name}`"))?;
        if !config.temporary
            && let Some(path) = self.path_of(root, name)
        {
            std::fs::remove_file(path).map_err(|error| error.to_string())?;
        }
        self.configurations.retain(|existing| existing.name != name);
        if self.selected.as_deref() == Some(name) {
            self.selected = self.configurations.first().map(|c| c.name.clone());
        }
        Ok(())
    }

    fn sort(&mut self) {
        self.configurations
            .sort_by(|a, b| (a.target.type_id(), &a.name).cmp(&(b.target.type_id(), &b.name)));
    }

    /// The file currently holding the configuration `name`, if any.
    fn path_of(&self, root: &Path, name: &str) -> Option<PathBuf> {
        let dir = archipel_ide_compat::project_write_path(root.join(RUN_CONFIGURATIONS_DIR));
        std::fs::read_dir(&dir).ok()?.find_map(|entry| {
            let path = entry.ok()?.path();
            let text = std::fs::read_to_string(&path).ok()?;
            let config = toml::from_str::<RunConfiguration>(&text).ok()?;
            (config.name == name).then_some(path)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spring(build: &str, target: &str) -> RunTarget {
        RunTarget::SpringBoot {
            build: build.into(),
            target: target.into(),
            main_class: String::new(),
            profiles: "dev,local".into(),
            program_arguments: vec!["--debug".into()],
        }
    }

    #[test]
    fn spring_boot_resolves_to_the_command_of_each_build_tool() {
        assert_eq!(
            spring("gradle", "").resolve_spring(),
            Some(RunTarget::Gradle {
                tasks: vec!["bootRun".into()],
                options: Vec::new(),
                program_arguments: vec![
                    "--spring.profiles.active=dev,local".into(),
                    "--debug".into()
                ],
            })
        );
        match spring("gradle", ":app").resolve_spring() {
            Some(RunTarget::Gradle { tasks, .. }) => assert_eq!(tasks, [":app:bootRun"]),
            other => panic!("{other:?}"),
        }
        let maven = RunTarget::SpringBoot {
            build: "maven".into(),
            target: "web".into(),
            main_class: "com.example.App".into(),
            profiles: "dev,local".into(),
            program_arguments: vec!["--debug".into()],
        };
        assert_eq!(
            maven.resolve_spring(),
            Some(RunTarget::Maven {
                goals: vec!["spring-boot:run".into()],
                options: vec![
                    "-f".into(),
                    "web/pom.xml".into(),
                    "-Dspring-boot.run.main-class=com.example.App".into(),
                    "-Dspring-boot.run.profiles=dev,local".into(),
                    "-Dspring-boot.run.arguments=--debug".into(),
                ],
            })
        );
        assert_eq!(
            spring("bazel", "").resolve_spring(),
            Some(RunTarget::Bazel {
                command: "run".into(),
                target: "//:app".into(),
                options: Vec::new(),
                program_arguments: vec![
                    "--spring.profiles.active=dev,local".into(),
                    "--debug".into()
                ],
            })
        );
        assert_eq!(RunTarget::template("cargo").unwrap().resolve_spring(), None);
        assert_eq!(spring("bazel", "").task_kind(), TaskKind::Run);
    }

    #[test]
    fn spring_boot_configurations_validate_and_round_trip_through_toml() {
        let config = RunConfiguration::new("DemoApplication", spring("maven", ""));
        config.validate().unwrap();
        let text = toml::to_string(&config).unwrap();
        assert!(text.contains("type = \"springboot\""), "{text}");
        let parsed: RunConfiguration = toml::from_str(&text).unwrap();
        assert_eq!(parsed, config);
        assert_eq!(config.target.build_system(), Some(BuildSystemKind::Maven));
        assert_eq!(config.target.type_name(), "Spring Boot");
        for bad in [
            RunTarget::SpringBoot {
                build: "ant".into(),
                target: String::new(),
                main_class: String::new(),
                profiles: String::new(),
                program_arguments: Vec::new(),
            },
            RunTarget::SpringBoot {
                build: "gradle".into(),
                target: String::new(),
                main_class: String::new(),
                profiles: "dev local".into(),
                program_arguments: Vec::new(),
            },
            RunTarget::SpringBoot {
                build: "bazel".into(),
                target: "app".into(),
                main_class: String::new(),
                profiles: String::new(),
                program_arguments: Vec::new(),
            },
        ] {
            assert!(RunConfiguration::new("X", bad).validate().is_err());
        }
    }

    fn temp(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-runconfig-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn gradle(name: &str, task: &str) -> RunConfiguration {
        RunConfiguration::new(
            name,
            RunTarget::Gradle {
                tasks: vec![task.into()],
                options: Vec::new(),
                program_arguments: vec!["--verbose".into()],
            },
        )
    }

    #[test]
    fn toml_round_trip_uses_type_tag() {
        let mut config = gradle("App", ":app:run");
        config.env.insert("JAVA_OPTS".into(), "-Xmx1g".into());
        config.working_directory = Some("app".into());
        let text = toml::to_string_pretty(&config).unwrap();
        assert!(text.contains("type = \"gradle\""), "{text}");
        assert!(text.contains("tasks = [\":app:run\"]"), "{text}");
        let parsed: RunConfiguration = toml::from_str(&text).unwrap();
        assert_eq!(parsed, config);
        let cargo: RunConfiguration = toml::from_str(
            "name = \"Tests\"\ntype = \"cargo\"\ncommand = \"test\"\npackage = \"core\"\n",
        )
        .unwrap();
        assert_eq!(cargo.target.task_kind(), TaskKind::Test);
    }

    #[test]
    fn save_load_rename_and_delete_persist_files() {
        let root = temp("persist");
        let mut manager = RunManager::load(&root);
        assert!(manager.configurations().is_empty());
        manager
            .save(&root, None, gradle("App", ":app:run"))
            .unwrap();
        manager
            .save(&root, None, gradle("Tests", ":app:test"))
            .unwrap();
        let dir = root.join(RUN_CONFIGURATIONS_DIR);
        assert!(dir.join("App.toml").is_file());
        let reloaded = RunManager::load(&root);
        assert_eq!(reloaded.configurations().len(), 2);
        assert_eq!(
            reloaded.get("Tests").unwrap(),
            manager.get("Tests").unwrap()
        );

        manager
            .save(&root, Some("App"), gradle("Main App", ":app:run"))
            .unwrap();
        assert!(!dir.join("App.toml").exists());
        assert!(dir.join("Main_App.toml").is_file());
        assert_eq!(manager.selected().unwrap().name, "Main App");
        assert!(
            manager.save(&root, None, gradle("Tests", ":x")).is_err(),
            "duplicate name"
        );

        manager.delete(&root, "Tests").unwrap();
        assert!(!dir.join("Tests.toml").exists());
        assert_eq!(RunManager::load(&root).configurations().len(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_files_are_reported_not_fatal() {
        let root = temp("invalid");
        let dir = root.join(RUN_CONFIGURATIONS_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("broken.toml"), "name = ").unwrap();
        std::fs::write(
            dir.join("unknown.toml"),
            "name = \"x\"\ntype = \"rocket\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("escape.toml"),
            "name = \"x\"\ntype = \"shell\"\nprogram = \"ls\"\nworking_directory = \"../..\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("ok.toml"),
            "name = \"ok\"\ntype = \"shell\"\nprogram = \"ls\"\n",
        )
        .unwrap();
        let manager = RunManager::load(&root);
        assert_eq!(manager.configurations().len(), 1);
        assert_eq!(manager.errors.len(), 3);
        assert_eq!(manager.selected().unwrap().name, "ok");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn temporary_configurations_are_selected_but_not_saved() {
        let root = temp("temporary");
        let mut manager = RunManager::load(&root);
        manager.add_temporary(gradle(":app:test", ":app:test"));
        assert_eq!(manager.selected().unwrap().name, ":app:test");
        assert!(manager.selected().unwrap().temporary);
        assert!(!root.join(RUN_CONFIGURATIONS_DIR).exists());
        manager
            .save(&root, Some(":app:test"), gradle(":app:test", ":app:test"))
            .unwrap();
        assert!(!manager.selected().unwrap().temporary);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn validation_messages() {
        assert!(
            RunConfiguration::new(" ", RunTarget::template("shell").unwrap())
                .validate()
                .is_err()
        );
        let bazel = RunConfiguration::new(
            "b",
            RunTarget::Bazel {
                command: "run".into(),
                target: "app".into(),
                options: Vec::new(),
                program_arguments: Vec::new(),
            },
        );
        assert!(bazel.validate().unwrap_err().contains("//:app"));
        for id in RunTarget::TYPE_IDS {
            let target = RunTarget::template(id).unwrap();
            assert_eq!(target.type_id(), id);
        }
    }

    #[test]
    fn shell_plan_applies_cwd_and_env() {
        let mut config = RunConfiguration::new(
            "List",
            RunTarget::Shell {
                program: "ls".into(),
                arguments: vec!["-la".into()],
            },
        );
        config.working_directory = Some("src".into());
        config.env.insert("A".into(), "1".into());
        let plan = config.shell_plan(Path::new("/w")).unwrap();
        assert_eq!(plan.cwd, PathBuf::from("/w/src"));
        assert_eq!(plan.env, [("A".to_owned(), "1".to_owned())]);
        assert_eq!(plan.name, "List");
        assert_eq!(config.file_name(), "List.toml");
    }
}

//! Gradle integration: build detection, wrapper/distribution discovery and task plans.
//!
//! Detection is static (it reads `settings.gradle(.kts)` and build scripts as text),
//! so it never executes build logic of an untrusted project. The precise project
//! model comes from a Gradle sync, which runs the user's Gradle.

pub mod model;
mod options;

pub use options::{BuildRunner, Distribution, GradleOptions, options, set_options};

pub use model::{
    DependencyModel, GradleModel, JdkModel, LibraryModel, ProjectModel, SourceSetModel, TaskModel,
};

use archipel_ide_runner::{
    BuildSystem, BuildSystemKind, RunConfiguration, RunTarget, RunnerError, TaskKind, TaskPlan,
    TestId,
    build_system::{derive_plan, java_method_name, require_program},
};
use std::path::{Path, PathBuf};

pub const SETTINGS_FILES: [&str; 2] = ["settings.gradle.kts", "settings.gradle"];
pub const BUILD_FILES: [&str; 2] = ["build.gradle.kts", "build.gradle"];

/// Plugins detected in a module's build script.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModulePlugins {
    pub android_application: bool,
    pub android_library: bool,
    pub application: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradleModule {
    /// Gradle path, `:` for the root project, `:app` for a sub-project.
    pub path: String,
    pub dir: PathBuf,
    pub build_file: Option<PathBuf>,
    pub plugins: ModulePlugins,
}

impl GradleModule {
    /// Task path for `task` in this module (`run` or `:app:run`).
    pub fn task(&self, task: &str) -> String {
        if self.path == ":" {
            task.to_owned()
        } else {
            format!("{}:{task}", self.path)
        }
    }
}

/// Statically detected Gradle build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradleBuild {
    pub root: PathBuf,
    pub settings_file: Option<PathBuf>,
    /// Wrapper script for this platform, when present.
    pub wrapper: Option<PathBuf>,
    /// Version from `gradle/wrapper/gradle-wrapper.properties`.
    pub wrapper_version: Option<String>,
    pub modules: Vec<GradleModule>,
    /// `buildSrc` or convention-plugin builds may apply plugins we cannot see.
    pub has_convention_plugins: bool,
}

impl GradleBuild {
    pub fn is_gradle_root(root: &Path) -> bool {
        SETTINGS_FILES
            .iter()
            .chain(BUILD_FILES.iter())
            .any(|name| root.join(name).is_file())
    }

    pub fn detect(root: &Path) -> Option<Self> {
        if !Self::is_gradle_root(root) {
            return None;
        }
        let settings_file = SETTINGS_FILES
            .iter()
            .map(|name| root.join(name))
            .find(|path| path.is_file());
        let mut paths = vec![":".to_owned()];
        if let Some(settings) = &settings_file
            && let Ok(text) = std::fs::read_to_string(settings)
        {
            for path in included_projects(&text) {
                if !paths.contains(&path) {
                    paths.push(path);
                }
            }
        }
        let modules = paths
            .into_iter()
            .map(|path| {
                let dir = path
                    .split(':')
                    .filter(|segment| !segment.is_empty())
                    .fold(root.to_owned(), |dir, segment| dir.join(segment));
                let build_file = BUILD_FILES
                    .iter()
                    .map(|name| dir.join(name))
                    .find(|file| file.is_file());
                let plugins = build_file
                    .as_ref()
                    .and_then(|file| std::fs::read_to_string(file).ok())
                    .map(|text| detect_plugins(&text))
                    .unwrap_or_default();
                GradleModule {
                    path,
                    dir,
                    build_file,
                    plugins,
                }
            })
            .collect();
        let wrapper_name = if cfg!(windows) {
            "gradlew.bat"
        } else {
            "gradlew"
        };
        let wrapper = Some(root.join(wrapper_name)).filter(|path| path.is_file());
        let wrapper_version =
            std::fs::read_to_string(root.join("gradle/wrapper/gradle-wrapper.properties"))
                .ok()
                .and_then(|text| wrapper_version(&text));
        let has_convention_plugins = ["buildSrc", "build-logic"]
            .iter()
            .any(|dir| root.join(dir).is_dir());
        Some(Self {
            root: root.to_owned(),
            settings_file,
            wrapper,
            wrapper_version,
            modules,
            has_convention_plugins,
        })
    }

    /// Program used to invoke Gradle: the wrapper when present, else `gradle` on PATH.
    pub fn program(&self) -> Result<String, RunnerError> {
        let wrapper = self
            .wrapper
            .as_ref()
            .filter(|_| options().distribution == Distribution::Wrapper);
        match wrapper {
            Some(wrapper) if cfg!(windows) => Ok(wrapper.display().to_string()),
            Some(_) => Ok("./gradlew".into()),
            None => require_program(
                "gradle",
                "Add the Gradle wrapper (gradlew) to the project or install Gradle and add it to PATH.",
            ),
        }
    }

    /// Modules that apply the Android application plugin (`:app`). The Android
    /// integration uses these to offer install-and-launch run configurations.
    pub fn android_application_modules(&self) -> Vec<&GradleModule> {
        self.modules
            .iter()
            .filter(|module| module.plugins.android_application)
            .collect()
    }

    pub fn is_android(&self) -> bool {
        self.modules
            .iter()
            .any(|module| module.plugins.android_application || module.plugins.android_library)
    }

    /// A plan running `tasks` with the console settings Archipel IDE parses.
    pub fn task_plan(
        &self,
        kind: TaskKind,
        name: impl Into<String>,
        tasks: &[String],
    ) -> Result<TaskPlan, RunnerError> {
        let args = tasks.iter().cloned().chain(["--console=plain".to_owned()]);
        Ok(self.plan(name, args)?.with_kind(kind))
    }

    /// A plan invoking Gradle with `args`, honouring the user's [`GradleOptions`]
    /// (distribution, `--offline`, Gradle user home and JDK).
    pub(crate) fn plan(
        &self,
        name: impl Into<String>,
        args: impl IntoIterator<Item = String>,
    ) -> Result<TaskPlan, RunnerError> {
        let options = options();
        let args = args
            .into_iter()
            .chain(options.offline.then(|| "--offline".to_owned()));
        let mut plan = TaskPlan::new(name, self.program()?, args, &self.root);
        if let Some(home) = &options.user_home {
            plan = plan.with_env("GRADLE_USER_HOME", home);
        }
        if let Some(jvm) = &options.jvm {
            plan = plan.with_env("JAVA_HOME", jvm);
        }
        Ok(plan)
    }
}

/// Whether the project-relative `path` is a file whose change makes the imported Gradle
/// model stale (build and settings scripts, `gradle.properties`, version catalogs).
pub fn is_build_input(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    matches!(
        name,
        "build.gradle"
            | "build.gradle.kts"
            | "settings.gradle"
            | "settings.gradle.kts"
            | "gradle.properties"
    ) || name.ends_with(".versions.toml")
}

/// Extracts project paths from `include(...)` statements in a settings script.
pub fn included_projects(settings: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in settings.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("include") else {
            continue;
        };
        if !rest.starts_with(['(', ' ', '\t']) {
            continue; // includeBuild, includeFlat …
        }
        let rest = rest.split("//").next().unwrap_or(rest);
        for literal in quoted(rest) {
            let path = if literal.starts_with(':') {
                literal
            } else {
                format!(":{literal}")
            };
            if path.len() > 1 && !path.contains(['/', '\\', ' ']) {
                paths.push(path);
            }
        }
    }
    paths
}

fn quoted(text: &str) -> Vec<String> {
    let mut literals = Vec::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '"' || ch == '\'' {
            let literal: String = chars.by_ref().take_while(|&c| c != ch).collect();
            literals.push(literal);
        }
    }
    literals
}

/// Recognises plugin applications by id or version-catalog alias.
pub fn detect_plugins(script: &str) -> ModulePlugins {
    let code: String = script
        .lines()
        .map(|line| line.split("//").next().unwrap_or(line))
        // `alias(...) apply false` only declares the plugin version for sub-projects.
        .filter(|line| !line.contains("apply false"))
        .collect::<Vec<_>>()
        .join("\n");
    let android_application =
        code.contains("com.android.application") || code.contains("plugins.android.application");
    let android_library =
        code.contains("com.android.library") || code.contains("plugins.android.library");
    let application = code.lines().any(|line| {
        let line = line.trim();
        line == "application"
            || line.starts_with("application ")
            || line.contains("id(\"application\")")
            || line.contains("id 'application'")
            || line.contains("id \"application\"")
            || line.contains("apply plugin: 'application'")
            || line.contains("apply plugin: \"application\"")
            || line.contains("`application`")
    }) || code.contains("application {");
    ModulePlugins {
        android_application,
        android_library,
        application,
    }
}

/// Reads the Gradle version from a wrapper `distributionUrl`.
pub fn wrapper_version(properties: &str) -> Option<String> {
    let url = properties.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "distributionUrl").then(|| value.trim().replace("\\:", ":"))
    })?;
    let file = url.rsplit('/').next()?;
    let version = file.strip_prefix("gradle-")?;
    let version = version
        .strip_suffix("-bin.zip")
        .or_else(|| version.strip_suffix("-all.zip"))?;
    Some(version.to_owned())
}

/// The Gradle provider for the platform build-system registry.
#[derive(Debug, Default)]
pub struct Gradle;

impl BuildSystem for Gradle {
    fn kind(&self) -> BuildSystemKind {
        BuildSystemKind::Gradle
    }

    fn detect(&self, root: &Path) -> bool {
        GradleBuild::is_gradle_root(root)
    }

    fn build_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        let build = detected(root)?;
        if build.is_android() {
            // Android Studio's Make Project assembles the selected (debug) variant.
            return build.task_plan(
                TaskKind::Build,
                "Gradle Assemble",
                &["assembleDebug".into()],
            );
        }
        if options().build_run == BuildRunner::Archipel {
            let tasks = ["classes".to_owned(), "testClasses".to_owned()];
            return build.task_plan(TaskKind::Build, "Gradle Compile", &tasks);
        }
        build.task_plan(TaskKind::Build, "Gradle Build", &["build".into()])
    }

    fn run_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        let build = detected(root)?;
        if let Some(module) = build.android_application_modules().first() {
            return Err(RunnerError::NotRunnable(format!(
                "{} is an Android application; Gradle has no `run` task for it. \
                 Use an Android run configuration to install and launch it on a device.",
                module.path
            )));
        }
        let runnable: Vec<&GradleModule> = build
            .modules
            .iter()
            .filter(|module| module.plugins.application)
            .collect();
        match runnable.as_slice() {
            [module] => build.task_plan(TaskKind::Run, "Gradle Run", &[module.task("run")]),
            [] if build.has_convention_plugins || build.is_android() => {
                Err(RunnerError::NotRunnable(
                    "no module applies the Gradle `application` plugin; run a task from the \
                     Gradle tool window instead"
                        .into(),
                ))
            }
            [] => Err(RunnerError::NotRunnable(
                "no module applies the Gradle `application` plugin, so there is no `run` task; \
                 apply it or run a task from the Gradle tool window"
                    .into(),
            )),
            // Several applications: `run` executes it in every module that has it.
            _ => build.task_plan(TaskKind::Run, "Gradle Run", &["run".into()]),
        }
    }

    fn test_plan(&self, root: &Path) -> Result<TaskPlan, RunnerError> {
        let build = detected(root)?;
        let task = if build.is_android() {
            "testDebugUnitTest"
        } else {
            "test"
        };
        build.task_plan(TaskKind::Test, "Gradle Test", &[task.into()])
    }

    fn configuration_plan(
        &self,
        root: &Path,
        config: &RunConfiguration,
    ) -> Result<TaskPlan, RunnerError> {
        if let RunTarget::Application {
            module,
            jdk,
            main_class,
            program_arguments,
        } = &config.target
        {
            let build = detected(root)?;
            let module = match module.trim() {
                "" => build.modules.first(),
                path => build.modules.iter().find(|m| m.path == path),
            }
            .ok_or_else(|| {
                RunnerError::NotRunnable(format!("no Gradle module `{}`", module.trim()))
            })?;
            let script = build.init_script()?;
            let mut args = vec![
                module.task(model::RUN_MAIN_TASK),
                "--init-script".to_owned(),
                script.display().to_string(),
                format!("-Parchipel.mainClass={}", main_class.trim()),
            ];
            if !program_arguments.is_empty() {
                args.push(format!("--args={}", program_arguments.join(" ")));
            }
            let mut plan = build.task_plan(TaskKind::Run, &config.name, &args)?;
            if !jdk.trim().is_empty() {
                plan = plan.with_env("JAVA_HOME", jdk.trim());
            }
            return Ok(plan);
        }
        let RunTarget::Gradle {
            tasks,
            options,
            program_arguments,
        } = &config.target
        else {
            return Err(RunnerError::NotRunnable(
                "not a Gradle configuration".into(),
            ));
        };
        let build = detected(root)?;
        let mut args: Vec<String> = tasks
            .iter()
            .flat_map(|task| task.split_whitespace())
            .map(str::to_owned)
            .chain(options.iter().cloned())
            .collect();
        if !program_arguments.is_empty() {
            // `run` and other JavaExec tasks receive program arguments via --args.
            args.push(format!("--args={}", program_arguments.join(" ")));
        }
        build.task_plan(config.target.task_kind(), &config.name, &args)
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
        // Keep tasks and options, drop earlier filters, add one --tests per failure.
        let mut args = Vec::new();
        let mut previous_filter = false;
        for arg in &plan.args {
            if previous_filter {
                previous_filter = false;
                continue;
            }
            if arg == "--tests" {
                previous_filter = true;
                continue;
            }
            args.push(arg.clone());
        }
        for test in failed {
            args.push("--tests".into());
            args.push(format!("{}.{}", test.suite, java_method_name(&test.name)));
        }
        Ok(derive_plan(plan, args))
    }
}

fn detected(root: &Path) -> Result<GradleBuild, RunnerError> {
    GradleBuild::detect(root).ok_or_else(|| RunnerError::NoBuildSystem(root.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir()
                .join(format!("archipel-ide-gradle-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        fn write(&self, path: &str, text: &str) -> &Self {
            let path = self.0.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
            self
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn parses_include_forms() {
        let settings = r#"
            rootProject.name = "demo"
            include(":app", ":lib:core") // comment ":ignored"
            include 'tools'
            includeBuild("build-logic")
            include(":app")
        "#;
        assert_eq!(
            included_projects(settings),
            [":app", ":lib:core", ":tools", ":app"]
        );
    }

    #[test]
    fn reads_wrapper_version() {
        let properties = "distributionBase=GRADLE_USER_HOME\n\
            distributionUrl=https\\://services.gradle.org/distributions/gradle-8.14.3-bin.zip\n";
        assert_eq!(wrapper_version(properties).as_deref(), Some("8.14.3"));
        assert_eq!(wrapper_version("x=y"), None);
    }

    #[test]
    fn detects_plugins_by_id_and_catalog_alias() {
        let android = detect_plugins("plugins {\n    alias(libs.plugins.android.application)\n}");
        assert!(android.android_application && !android.application);
        let library = detect_plugins("plugins { id(\"com.android.library\") }");
        assert!(library.android_library);
        let app =
            detect_plugins("plugins {\n    application\n}\napplication { mainClass = \"A\" }");
        assert!(app.application);
        let groovy = detect_plugins("plugins {\n    id 'application'\n}");
        assert!(groovy.application);
        let commented =
            detect_plugins("plugins {\n    // id(\"com.android.application\")\n    java\n}");
        assert_eq!(commented, ModulePlugins::default());
    }

    #[test]
    fn android_projects_never_plan_gradle_run() {
        let fixture = Fixture::new("android");
        fixture
            .write("settings.gradle.kts", "include(\":app\")\n")
            .write(
                "build.gradle.kts",
                "plugins { alias(libs.plugins.android.application) apply false }",
            )
            .write(
                "app/build.gradle.kts",
                "plugins {\n alias(libs.plugins.android.application)\n}",
            )
            .write("gradlew", "#!/bin/sh\n");
        let build = GradleBuild::detect(&fixture.0).unwrap();
        assert_eq!(
            build
                .android_application_modules()
                .iter()
                .map(|module| module.path.as_str())
                .collect::<Vec<_>>(),
            [":app"]
        );
        let error = Gradle.run_plan(&fixture.0).unwrap_err().to_string();
        assert!(error.contains("Android application"), "{error}");
        let build_plan = Gradle.build_plan(&fixture.0).unwrap();
        assert_eq!(build_plan.args, ["assembleDebug", "--console=plain"]);
        #[cfg(unix)]
        assert_eq!(build_plan.program, "./gradlew");
        assert_eq!(
            Gradle.test_plan(&fixture.0).unwrap().args[0],
            "testDebugUnitTest"
        );
    }

    #[test]
    fn jvm_application_runs_in_its_module() {
        let fixture = Fixture::new("jvm");
        fixture
            .write("settings.gradle", "include 'app', 'lib'\n")
            .write("app/build.gradle", "plugins {\n    id 'application'\n}\n")
            .write("lib/build.gradle", "plugins { id 'java-library' }\n")
            .write("gradlew", "")
            .write("gradlew.bat", "");
        let build = GradleBuild::detect(&fixture.0).unwrap();
        assert_eq!(build.modules.len(), 3);
        assert_eq!(build.modules[1].dir, fixture.0.join("app"));
        let run = Gradle.run_plan(&fixture.0).unwrap();
        assert_eq!(run.args, [":app:run", "--console=plain"]);
        assert_eq!(run.kind, TaskKind::Run);
        assert_eq!(Gradle.build_plan(&fixture.0).unwrap().args[0], "build");
    }

    #[test]
    fn gradle_configuration_plan_passes_options_and_program_arguments() {
        let fixture = Fixture::new("config");
        fixture
            .write("settings.gradle.kts", "")
            .write("gradlew", "");
        let config = RunConfiguration::new(
            "App",
            RunTarget::Gradle {
                tasks: vec![":app:run".into()],
                options: vec!["--offline".into()],
                program_arguments: vec!["a".into(), "b".into()],
            },
        );
        let plan = Gradle.configuration_plan(&fixture.0, &config).unwrap();
        assert_eq!(
            plan.args,
            [":app:run", "--offline", "--args=a b", "--console=plain"]
        );
        assert_eq!(plan.kind, TaskKind::Run);
    }

    #[test]
    fn rerun_failed_adds_test_filters() {
        let plan = TaskPlan::new(
            "Gradle Test",
            "./gradlew",
            [":app:test", "--tests", "Old", "--console=plain"],
            "/w",
        );
        let failed = [TestId {
            suite: "com.example.AppTest".into(),
            name: "fails()".into(),
        }];
        let rerun = Gradle
            .rerun_failed_plan(Path::new("/w"), &plan, &failed)
            .unwrap();
        assert_eq!(
            rerun.args,
            [
                ":app:test",
                "--console=plain",
                "--tests",
                "com.example.AppTest.fails"
            ]
        );
        assert_eq!(rerun.kind, TaskKind::Test);
    }

    #[test]
    fn library_only_build_explains_missing_run_task() {
        let fixture = Fixture::new("lib");
        fixture
            .write("build.gradle.kts", "plugins { `java-library` }\n")
            .write("gradlew", "");
        let error = Gradle.run_plan(&fixture.0).unwrap_err();
        assert!(matches!(error, RunnerError::NotRunnable(_)));
        assert!(error.to_string().contains("application"));
    }
}

#[cfg(test)]
mod build_input_tests {
    use super::is_build_input;
    use std::path::Path;

    #[test]
    fn recognises_gradle_build_inputs() {
        for yes in [
            "build.gradle",
            "app/build.gradle.kts",
            "settings.gradle.kts",
            "gradle.properties",
            "gradle/libs.versions.toml",
        ] {
            assert!(is_build_input(Path::new(yes)), "{yes}");
        }
        for no in ["src/Main.java", "build.rs", "Cargo.toml"] {
            assert!(!is_build_input(Path::new(no)), "{no}");
        }
    }
}

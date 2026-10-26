//! Gutter run icons: runnables of the active document and their run configurations.

use crate::{IdeState, spec, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_gradle::{GradleBuild, model::RUN_MAIN_TASK};
use archipel_ide_runnables::{Runnable, RunnableKind};
use archipel_ide_runner::{BuildSystemKind, RunConfiguration, RunTarget};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// One gutter icon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GutterMark {
    pub line: usize,
    pub kind: RunnableKind,
    /// Menu label, e.g. `Run 'AppTest.greets()'`.
    pub label: String,
}

/// Cache key: document path and text revision.
#[derive(Debug, Clone, Default)]
pub struct RunnableCache {
    key: Option<(PathBuf, u64)>,
    marks: Vec<(GutterMark, RunConfiguration)>,
}

impl IdeState {
    /// Gutter icons of the active document (cached per text revision).
    pub fn gutter_marks(&mut self) -> Vec<GutterMark> {
        self.refresh_runnables();
        self.runnable_cache
            .marks
            .iter()
            .map(|(mark, _)| mark.clone())
            .collect()
    }

    /// The last computed icons when they belong to the active document, even if
    /// its text has changed since (`None` otherwise). The editor shows these
    /// while the user types and refreshes once typing pauses, because finding
    /// runnables walks the whole syntax tree.
    pub fn cached_gutter_marks(&self) -> Option<Vec<GutterMark>> {
        let path = &self.documents.get(self.active_document?)?.path;
        let (cached_path, _) = self.runnable_cache.key.as_ref()?;
        (cached_path == path).then(|| {
            self.runnable_cache
                .marks
                .iter()
                .map(|(mark, _)| mark.clone())
                .collect()
        })
    }

    fn refresh_runnables(&mut self) {
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get(index))
        else {
            self.runnable_cache = RunnableCache::default();
            return;
        };
        let key = (document.path.clone(), document.text.revision());
        if self.runnable_cache.key.as_ref() == Some(&key) {
            return;
        }
        let path = document.path.clone();
        let source = document.text.text();
        let marks = self.compute_marks(&path, &source, document);
        self.runnable_cache = RunnableCache {
            key: Some(key),
            marks,
        };
    }

    fn compute_marks(
        &self,
        relative: &Path,
        source: &str,
        document: &crate::Document,
    ) -> Vec<(GutterMark, RunConfiguration)> {
        let Some(root) = self
            .project
            .as_ref()
            .map(|project| project.root().to_owned())
        else {
            return Vec::new();
        };
        let absolute = root.join(relative);
        let file_name = relative
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        // Bazel BUILD files: one icon per rule that the last sync knows.
        if matches!(file_name.as_deref(), Some("BUILD" | "BUILD.bazel"))
            && let Some(targets) = &self.bazel_targets
        {
            let rules = archipel_ide_bazel::rule_lines(source);
            let declared = targets.in_build_file(relative);
            return rules
                .into_iter()
                .filter_map(|(line, name)| {
                    let target = declared.iter().find(|target| target.name() == name)?;
                    let command = target.role().default_command();
                    let kind = match target.role() {
                        archipel_ide_bazel::TargetRole::Test => RunnableKind::Test,
                        _ => RunnableKind::Main,
                    };
                    let verb = match command {
                        "run" => "Run",
                        "test" => "Test",
                        _ => "Build",
                    };
                    Some((
                        GutterMark {
                            line,
                            kind,
                            label: format!("{verb} '{}'", target.label),
                        },
                        target.configuration(command),
                    ))
                })
                .collect();
        }
        let cargo_root = nearest_ancestor_with(&absolute, &root, "Cargo.toml");
        let module = cargo_root
            .as_ref()
            .map(|crate_root| archipel_ide_runnables::rust_module_path(crate_root, &absolute))
            .unwrap_or_default();
        // Reuse the editor's incremental syntax tree when it describes this text;
        // reparsing a large file on every keystroke costs hundreds of ms.
        let runnables = match &document.analysis.syntax {
            Some(syntax) if &*document.analysis.source == source => {
                archipel_ide_runnables::find_in_tree(
                    syntax.language().id,
                    syntax.tree(),
                    relative,
                    source,
                    &module,
                )
            }
            _ => archipel_ide_runnables::find(relative, source, &module),
        };
        runnables
            .into_iter()
            .filter_map(|runnable| {
                let config = self.configuration_for(&root, &absolute, &runnable).ok()?;
                Some((
                    GutterMark {
                        line: runnable.line,
                        kind: runnable.kind,
                        label: format!("Run '{}'", runnable.label),
                    },
                    config,
                ))
            })
            .collect()
    }

    /// Maps a runnable to a configuration for the project's build system.
    fn configuration_for(
        &self,
        root: &Path,
        file: &Path,
        runnable: &Runnable,
    ) -> Result<RunConfiguration, String> {
        let kind = self
            .detected_build
            .as_ref()
            .map(|build| build.kind)
            .ok_or("no build system detected")?;
        let name = match runnable.kind {
            RunnableKind::Main => runnable
                .symbol
                .rsplit(['.', '$'])
                .next()
                .unwrap_or(&runnable.symbol)
                .to_owned(),
            _ => runnable.label.trim_end_matches("()").to_owned(),
        };
        let target = match (kind, runnable.language) {
            (BuildSystemKind::Gradle, "java" | "kotlin") => {
                let build = GradleBuild::detect(root).ok_or("not a Gradle build")?;
                let module = build
                    .modules
                    .iter()
                    .filter(|module| file.starts_with(&module.dir))
                    .max_by_key(|module| module.dir.components().count())
                    .ok_or("file is outside every Gradle project")?;
                match runnable.kind {
                    RunnableKind::Main => {
                        let script = build.init_script().map_err(|error| error.to_string())?;
                        RunTarget::Gradle {
                            tasks: vec![module.task(RUN_MAIN_TASK)],
                            options: vec![
                                "--init-script".into(),
                                script.display().to_string(),
                                format!("-Parchipel.mainClass={}", runnable.symbol),
                            ],
                            program_arguments: Vec::new(),
                        }
                    }
                    _ => {
                        let task = if build.is_android() {
                            "testDebugUnitTest"
                        } else {
                            "test"
                        };
                        RunTarget::Gradle {
                            tasks: vec![module.task(task)],
                            options: vec!["--tests".into(), runnable.symbol.replace('$', ".")],
                            program_arguments: Vec::new(),
                        }
                    }
                }
            }
            (BuildSystemKind::Maven, "java" | "kotlin") if runnable.kind != RunnableKind::Main => {
                let filter = match runnable.kind {
                    RunnableKind::Test => {
                        let (class, method) = runnable
                            .symbol
                            .rsplit_once('.')
                            .ok_or("unexpected test name")?;
                        format!("{class}#{method}")
                    }
                    _ => runnable.symbol.clone(),
                };
                RunTarget::Maven {
                    goals: vec!["test".into()],
                    options: vec![
                        format!("-Dtest={filter}"),
                        "-Dsurefire.failIfNoSpecifiedTests=false".into(),
                    ],
                }
            }
            (BuildSystemKind::Cargo, "rust") => {
                let crate_root = nearest_ancestor_with(file, root, "Cargo.toml")
                    .ok_or("file is outside every Cargo package")?;
                let relative = file.strip_prefix(&crate_root).unwrap_or(file);
                let stem = file
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let target_dir = relative
                    .components()
                    .next()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut options = Vec::new();
                let in_bin = relative.starts_with("src/bin");
                match (runnable.kind, target_dir.as_str()) {
                    (_, "tests") => options.extend(["--test".into(), stem.clone()]),
                    (_, "examples") => options.extend(["--example".into(), stem.clone()]),
                    (_, _) if in_bin => options.extend(["--bin".into(), stem.clone()]),
                    _ => {}
                }
                let (command, program_arguments) = match runnable.kind {
                    RunnableKind::Main => ("run", Vec::new()),
                    RunnableKind::Test => ("test", vec![runnable.symbol.clone(), "--exact".into()]),
                    RunnableKind::TestContainer => ("test", vec![format!("{}::", runnable.symbol)]),
                };
                let name = match runnable.kind {
                    // Name binaries after their target, like `cargo run --bin <name>`.
                    RunnableKind::Main if !options.is_empty() => stem,
                    RunnableKind::Main => crate_root
                        .file_name()
                        .map(|dir| dir.to_string_lossy().into_owned())
                        .unwrap_or(name),
                    _ => name,
                };
                let mut config = RunConfiguration::new(
                    name,
                    RunTarget::Cargo {
                        command: command.into(),
                        package: None,
                        bin: None,
                        options,
                        program_arguments,
                    },
                );
                let package_dir = crate_root.strip_prefix(root).unwrap_or(Path::new(""));
                if !package_dir.as_os_str().is_empty() {
                    config.working_directory = Some(package_dir.to_owned());
                }
                return Ok(config);
            }
            (BuildSystemKind::Go, "go") => {
                let dir = file
                    .parent()
                    .and_then(|dir| dir.strip_prefix(root).ok())
                    .map(|dir| format!("./{}", dir.display()))
                    .unwrap_or_else(|| ".".into());
                let package = if dir == "./" { ".".into() } else { dir };
                match runnable.kind {
                    RunnableKind::Main => RunTarget::Go {
                        command: "run".into(),
                        package,
                        program_arguments: Vec::new(),
                    },
                    _ => RunTarget::Go {
                        command: "test".into(),
                        package,
                        program_arguments: vec![
                            "-v".into(),
                            "-run".into(),
                            format!("^{}$", runnable.symbol),
                        ],
                    },
                }
            }
            _ => return Err("no run configuration for this entry point and build system".into()),
        };
        Ok(RunConfiguration::new(name, target))
    }

    /// Runs (or opens for editing) the gutter target at `line` of the active document.
    pub fn run_gutter(&mut self, line: usize, edit: bool, debug: bool) -> Result<(), String> {
        self.refresh_runnables();
        let config = self
            .runnable_cache
            .marks
            .iter()
            .find(|(mark, _)| mark.line == line)
            .map(|(_, config)| config.clone())
            .ok_or_else(|| format!("nothing to run at line {}", line + 1))?;
        if edit {
            let name = config.name.clone();
            if self
                .run_configurations
                .get(&name)
                .is_none_or(|existing| existing.temporary)
            {
                self.run_configurations.add_temporary(config);
            }
            self.edit_run_configurations = Some(name);
            return Ok(());
        }
        if debug {
            if self.is_debugging() {
                return Err("A debug session is already running; stop it first.".into());
            }
            let name = config.name.clone();
            if self
                .run_configurations
                .get(&name)
                .is_none_or(|existing| existing.temporary)
            {
                self.run_configurations.add_temporary(config);
            }
            self.run_configurations.select(&name)?;
            return self.start_debugging();
        }
        self.run_temporary(config)
    }
}

/// Closest directory from `file` up to `root` (inclusive) that contains `marker`.
fn nearest_ancestor_with(file: &Path, root: &Path, marker: &str) -> Option<PathBuf> {
    file.ancestors()
        .skip(1)
        .take_while(|dir| dir.starts_with(root))
        .find(|dir| dir.join(marker).is_file())
        .map(Path::to_owned)
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "run.gutter",
                "Run Gutter Target",
                "Run",
                Some("⌃⇧R"),
                r#"{"line":0,"edit":false,"debug":false}"#,
            ),
            |state: &IdeState| state.active_document.is_some() && !state.runner.is_running(),
            |state: &mut IdeState, params| {
                let line = match params.get("line") {
                    Some(_) => usize_param(&params, "line")?,
                    // Without a line: the target at or above the caret (Ctrl+Shift+F10).
                    None => state
                        .caret_runnable_line()
                        .ok_or_else(|| CommandError::Failed("no runnable at the caret".into()))?,
                };
                let edit = params.get("edit").and_then(Value::as_bool).unwrap_or(false);
                let debug = params
                    .get("debug")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                state
                    .run_gutter(line, edit, debug)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "project.runFile",
                "Run File",
                "Run",
                None,
                r#"{"path":"string"}"#,
            ),
            |state: &IdeState| state.project.is_some() && !state.runner.is_running(),
            |state: &mut IdeState, params| {
                let path = PathBuf::from(crate::string_param(&params, "path")?);
                let label = state.run_file(&path).map_err(CommandError::Failed)?;
                Ok(json!({"label": label}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("run.gutterTargets", "List Gutter Targets", "Run", None, "{}"),
            |state: &IdeState| state.active_document.is_some(),
            |state: &mut IdeState, _| {
                Ok(json!(
                    state
                        .gutter_marks()
                        .into_iter()
                        .map(|mark| json!({"line": mark.line, "label": mark.label, "kind": format!("{:?}", mark.kind)}))
                        .collect::<Vec<_>>()
                ))
            },
        ))
        .expect("unique built-in id");
}

impl IdeState {
    /// Opens `path` and runs its entry point (`Main`), else its test container.
    pub fn run_file(&mut self, path: &Path) -> Result<String, String> {
        self.open_file(path)?;
        self.refresh_runnables();
        let pick = |kind: RunnableKind| {
            self.runnable_cache
                .marks
                .iter()
                .find(|(mark, _)| mark.kind == kind)
                .cloned()
        };
        let (mark, config) = pick(RunnableKind::Main)
            .or_else(|| pick(RunnableKind::TestContainer))
            .ok_or_else(|| format!("nothing to run in {}", path.display()))?;
        self.run_temporary(config)?;
        Ok(mark.label)
    }

    /// The closest runnable at or above the caret line (context run action).
    pub(crate) fn caret_runnable_line(&mut self) -> Option<usize> {
        let caret = self.active_caret()?;
        let document = self.documents.get(self.active_document?)?;
        let line = document.text.char_to_line(caret);
        self.refresh_runnables();
        self.runnable_cache
            .marks
            .iter()
            .map(|(mark, _)| mark.line)
            .filter(|mark| *mark <= line)
            .max()
    }
}

#[cfg(all(test, unix))]
mod tests {
    use crate::{IdeState, built_in_commands};
    use archipel_ide_runner::RunTarget;
    use serde_json::{Value, json};
    use std::path::{Path, PathBuf};

    fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-gutter-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        root
    }

    fn open(
        root: &Path,
        file: &str,
    ) -> (IdeState, archipel_ide_command::CommandRegistry<IdeState>) {
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": file}))
            .unwrap();
        (state, commands)
    }

    #[test]
    fn gradle_java_main_and_test_icons_map_to_gradle_tasks() {
        let root = project(
            "gradle",
            &[
                ("settings.gradle.kts", "include(\":app\")\n"),
                ("app/build.gradle.kts", "plugins { application }\n"),
                (
                    "app/src/main/java/App.java",
                    "public class App { public static void main(String[] a) {} }\n",
                ),
                (
                    "app/src/test/java/com/example/AppTest.java",
                    "package com.example;\nclass AppTest {\n  @Test\n  void greets() {}\n}\n",
                ),
            ],
        );
        let (mut state, commands) = open(&root, "app/src/test/java/com/example/AppTest.java");
        let marks = state.gutter_marks();
        assert_eq!(marks.len(), 2, "{marks:?}");
        assert_eq!(marks[1].label, "Run 'AppTest.greets()'");
        // Edit opens the dialog on a temporary configuration instead of running.
        commands
            .execute("run.gutter", &mut state, json!({"line": 3, "edit": true}))
            .unwrap();
        assert_eq!(
            state.edit_run_configurations.as_deref(),
            Some("AppTest.greets")
        );
        let config = state.run_configurations.get("AppTest.greets").unwrap();
        assert_eq!(
            config.target,
            RunTarget::Gradle {
                tasks: vec![":app:test".into()],
                options: vec!["--tests".into(), "com.example.AppTest.greets".into()],
                program_arguments: Vec::new(),
            }
        );
        commands
            .execute(
                "editor.openFile",
                &mut state,
                json!({"path": "app/src/main/java/App.java"}),
            )
            .unwrap();
        let listed = commands
            .execute("run.gutterTargets", &mut state, Value::Null)
            .unwrap();
        assert_eq!(listed[0]["label"], "Run 'App.main()'");
        commands
            .execute("run.gutter", &mut state, json!({"line": 0, "edit": true}))
            .unwrap();
        let RunTarget::Gradle { tasks, options, .. } =
            &state.run_configurations.get("App").unwrap().target
        else {
            panic!("expected a Gradle configuration");
        };
        assert_eq!(tasks, &[":app:archipelRunMain".to_owned()]);
        assert_eq!(options.last().unwrap(), "-Parchipel.mainClass=App");
        assert!(
            root.join(".archipel/gradle/archipel-model.init.gradle")
                .is_file()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn class_with_main_gets_its_own_icon_and_the_widget_reads_current_file() {
        let root = project(
            "class-icon",
            &[
                ("settings.gradle.kts", "include(\":app\")\n"),
                ("app/build.gradle.kts", "plugins { application }\n"),
                (
                    "app/src/main/java/App.java",
                    "public class App {\n  public static void main(String[] a) {}\n}\n",
                ),
            ],
        );
        let (mut state, _commands) = open(&root, "app/src/main/java/App.java");
        let lines: Vec<usize> = state.gutter_marks().iter().map(|mark| mark.line).collect();
        assert_eq!(lines, [0, 1]);
        assert_eq!(state.run_widget_label().as_deref(), Some("Current File"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cargo_tests_run_exactly_and_bin_targets_are_selected() {
        let root = project(
            "cargo",
            &[
                (
                    "Cargo.toml",
                    "[package]\nname = \"calc\"\nversion = \"0.1.0\"\n",
                ),
                ("src/lib.rs", "pub mod ops;\n"),
                (
                    "src/ops.rs",
                    "#[cfg(test)]\nmod tests {\n    #[test]\n    fn adds() {}\n}\n",
                ),
                ("src/bin/tool.rs", "fn main() {}\n"),
            ],
        );
        let (mut state, commands) = open(&root, "src/ops.rs");
        commands
            .execute("run.gutter", &mut state, json!({"line": 3, "edit": true}))
            .unwrap();
        let config = state.run_configurations.get("ops::tests::adds").unwrap();
        let RunTarget::Cargo {
            command,
            program_arguments,
            ..
        } = &config.target
        else {
            panic!("expected a Cargo configuration");
        };
        assert_eq!(command, "test");
        assert_eq!(
            program_arguments,
            &["ops::tests::adds".to_owned(), "--exact".to_owned()]
        );
        commands
            .execute(
                "editor.openFile",
                &mut state,
                json!({"path": "src/bin/tool.rs"}),
            )
            .unwrap();
        commands
            .execute("run.gutter", &mut state, json!({"line": 0, "edit": true}))
            .unwrap();
        let RunTarget::Cargo {
            command, options, ..
        } = &state.run_configurations.get("tool").unwrap().target
        else {
            panic!("expected a Cargo configuration");
        };
        assert_eq!(
            (command.as_str(), options.as_slice()),
            ("run", &["--bin".to_owned(), "tool".to_owned()][..])
        );
        assert!(
            commands
                .execute("run.gutter", &mut state, json!({"line": 5}))
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

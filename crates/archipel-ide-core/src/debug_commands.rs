//! Debugger: line breakpoints and Debug Adapter Protocol sessions.

use crate::java_debug::JavaStage;
use crate::{IdeState, SavePoint, ToolWindow, spec, string_param, usize_param};
use archipel_ide_command::{Command, CommandError, CommandRegistry};
use archipel_ide_dap::{Adapter, BreakpointOptions, DebugClient, SessionState, Step};
use archipel_ide_runner::{RunConfiguration, RunTarget, TaskKind, TaskPlan, TaskState};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const BREAKPOINTS_FILE: &str = ".archipel/breakpoints.json";
/// Breakpoint conditions/logpoints and watch expressions.
pub const DEBUG_STATE_FILE: &str = ".archipel/debug-state.json";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct StoredDebugState {
    #[serde(default)]
    breakpoints: Vec<StoredBreakpointOptions>,
    #[serde(default)]
    watches: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct StoredBreakpointOptions {
    path: PathBuf,
    line: usize,
    #[serde(flatten)]
    options: BreakpointOptions,
}

/// What to debug once its build has finished.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDebug {
    pub name: String,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
}

/// The editor's view of the debugger: where execution stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPoint {
    /// Project-relative path.
    pub path: PathBuf,
    /// Zero-based line.
    pub line: usize,
}

/// A Java debug session being prepared through jdtls (see `java_debug`).
pub(crate) struct PendingJava {
    pub name: String,
    /// Open Java document whose language server runs the plugin commands.
    pub anchor: PathBuf,
    pub main_class: Option<String>,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    pub stage: JavaStage,
    pub started: Instant,
    /// Attach to a JVM started by a Gradle `--debug-jvm` test task instead of launching.
    pub attach: bool,
}

#[derive(Default)]
pub struct DebuggerState {
    /// Project-relative path → zero-based breakpoint lines.
    pub breakpoints: BTreeMap<PathBuf, BTreeSet<usize>>,
    pub client: Option<DebugClient>,
    pub pending: Option<PendingDebug>,
    pub(crate) java: Option<PendingJava>,
    /// Project-relative path and zero-based line → condition / logpoint options.
    pub breakpoint_options: BTreeMap<(PathBuf, usize), BreakpointOptions>,
    /// Watch expressions, kept across sessions.
    pub watches: Vec<String>,
    /// Breakpoint whose properties popup is open (project-relative path, zero-based line).
    pub editing_breakpoint: Option<(PathBuf, usize)>,
    /// Text of the Evaluate Expression dialog while it is open.
    pub evaluate_dialog: Option<String>,
    /// Name of the configuration being debugged.
    pub name: String,
    /// Frame already announced as a stop (to announce each stop only once).
    revealed: Option<(i64, i64)>,
    pub error: Option<String>,
    /// What the session's listeners were told last (see `XDebuggerManager::poll_session`).
    pub(crate) announced: crate::services::Announced,
}

impl DebuggerState {
    pub(crate) fn reset_revealed(&mut self) {
        self.revealed = None;
    }

    pub(crate) fn revealed(&self) -> Option<(i64, i64)> {
        self.revealed
    }

    pub(crate) fn set_revealed(&mut self, frame: Option<(i64, i64)>) {
        self.revealed = frame;
    }
}

impl IdeState {
    pub(crate) fn absolute_breakpoints(
        &self,
    ) -> Result<BTreeMap<PathBuf, BTreeSet<usize>>, String> {
        let root = self.project.as_ref().ok_or("no project is open")?.root();
        Ok(self
            .debugger()
            .state()
            .breakpoints
            .iter()
            .map(|(path, lines)| (root.join(path), lines.clone()))
            .collect())
    }

    pub(crate) fn load_breakpoints(&mut self) {
        let debugger = self.debugger();
        debugger.reset();
        let Some(project) = &self.project else {
            return;
        };
        let mut state = debugger.state_mut();
        if let Ok(text) = std::fs::read_to_string(archipel_ide_compat::project_read_path(
            project.root().join(BREAKPOINTS_FILE),
        )) && let Ok(map) = serde_json::from_str::<BTreeMap<PathBuf, BTreeSet<usize>>>(&text)
        {
            state.breakpoints = map;
        }
        if let Ok(text) = std::fs::read_to_string(archipel_ide_compat::project_read_path(
            project.root().join(DEBUG_STATE_FILE),
        )) && let Ok(stored) = serde_json::from_str::<StoredDebugState>(&text)
        {
            state.breakpoint_options = stored
                .breakpoints
                .into_iter()
                .map(|entry| ((entry.path, entry.line), entry.options))
                .collect();
            state.watches = stored.watches;
        }
    }

    fn save_debug_state(&self) -> Result<(), String> {
        let project = self.project.as_ref().ok_or("no project is open")?;
        let path = archipel_ide_compat::project_write_path(project.root().join(DEBUG_STATE_FILE));
        let stored = {
            let debugger = self.debugger();
            let state = debugger.state();
            if state.breakpoint_options.is_empty() && state.watches.is_empty() {
                let _ = std::fs::remove_file(path);
                return Ok(());
            }
            StoredDebugState {
                breakpoints: state
                    .breakpoint_options
                    .iter()
                    .map(|((path, line), options)| StoredBreakpointOptions {
                        path: path.clone(),
                        line: *line,
                        options: options.clone(),
                    })
                    .collect(),
                watches: state.watches.clone(),
            }
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let json = serde_json::to_string_pretty(&stored).map_err(|error| error.to_string())?;
        std::fs::write(path, json).map_err(|error| error.to_string())
    }

    /// Gives a new session the project's breakpoint options and watches.
    pub(crate) fn seed_session(&self, client: &mut DebugClient) {
        let Some(project) = &self.project else {
            return;
        };
        let root = project.root().to_owned();
        let (options, watches) = {
            let debugger = self.debugger();
            let state = debugger.state();
            let options = state
                .breakpoint_options
                .iter()
                .map(|((path, line), options)| ((root.join(path), *line), options.clone()))
                .collect();
            (options, state.watches.clone())
        };
        client.with_session(|session| {
            session.load_options(options);
            for watch in &watches {
                session.add_watch(watch);
            }
        });
    }

    /// Conditions/logpoint options of a breakpoint line.
    pub fn breakpoint_options(&self, path: &Path, line: usize) -> Option<BreakpointOptions> {
        self.debugger()
            .state()
            .breakpoint_options
            .get(&(path.to_owned(), line))
            .cloned()
    }

    /// Sets the options of an existing breakpoint, creating the breakpoint when the line
    /// has none (like the properties popup of the reference IDE).
    pub fn set_breakpoint_options(
        &mut self,
        path: &Path,
        line: usize,
        options: BreakpointOptions,
    ) -> Result<(), String> {
        let options = options.normalized();
        if !self.breakpoint_lines(path).contains(&line) {
            self.toggle_breakpoint(path, line)?;
        }
        let key = (path.to_owned(), line);
        {
            let debugger = self.debugger();
            let mut state = debugger.state_mut();
            if options.is_empty() {
                state.breakpoint_options.remove(&key);
            } else {
                state.breakpoint_options.insert(key, options.clone());
            }
            if let (Some(client), Some(project)) = (state.client.as_mut(), &self.project) {
                let absolute = project.root().join(path);
                client.with_session(|session| {
                    session.set_breakpoint_options(absolute, line, options)
                });
            }
        }
        self.save_debug_state()
    }

    pub fn add_watch(&mut self, expression: &str) -> Result<(), String> {
        let expression = expression.trim();
        if expression.is_empty() {
            return Err("a watch needs an expression".into());
        }
        {
            let debugger = self.debugger();
            let mut state = debugger.state_mut();
            if !state.watches.iter().any(|w| w == expression) {
                state.watches.push(expression.to_owned());
            }
            if let Some(client) = state.client.as_mut() {
                client.with_session(|session| session.add_watch(expression));
            }
        }
        self.save_debug_state()
    }

    pub fn remove_watch(&mut self, index: usize) -> Result<(), String> {
        {
            let debugger = self.debugger();
            let mut state = debugger.state_mut();
            if index >= state.watches.len() {
                return Err(format!("no watch at index {index}"));
            }
            state.watches.remove(index);
            if let Some(client) = state.client.as_mut() {
                client.with_session(|session| session.remove_watch(index));
            }
        }
        self.save_debug_state()
    }

    /// Replaces a watch; an empty expression removes it.
    pub fn edit_watch(&mut self, index: usize, expression: &str) -> Result<(), String> {
        let expression = expression.trim();
        if index >= self.debugger().state().watches.len() {
            return Err(format!("no watch at index {index}"));
        }
        if expression.is_empty() {
            return self.remove_watch(index);
        }
        {
            let debugger = self.debugger();
            let mut state = debugger.state_mut();
            if state
                .watches
                .iter()
                .enumerate()
                .any(|(i, w)| i != index && w == expression)
            {
                return Err(format!("`{expression}` is already watched"));
            }
            state.watches[index] = expression.to_owned();
            if let Some(client) = state.client.as_mut() {
                client.with_session(|session| session.edit_watch(index, expression));
            }
        }
        self.save_debug_state()
    }

    /// Runs to the caret line of the active document (Alt+F9).
    pub fn run_to_cursor(
        &mut self,
        path: Option<&Path>,
        line: Option<usize>,
    ) -> Result<(), String> {
        let (path, line) = match (path, line) {
            (Some(path), Some(line)) => (path.to_owned(), line),
            _ => {
                let document = self
                    .active_document
                    .and_then(|index| self.documents.get(index))
                    .ok_or("no editor is active")?;
                (
                    document.path.clone(),
                    document
                        .text
                        .char_to_line(document.selections.primary().head),
                )
            }
        };
        let root = self.project.as_ref().ok_or("no project is open")?.root();
        let absolute = root.join(path);
        self.with_debug_client(|client| {
            client.with_session(|session| session.run_to_line(absolute, line))
        })
    }

    fn save_breakpoints(&self) -> Result<(), String> {
        let project = self.project.as_ref().ok_or("no project is open")?;
        let path = archipel_ide_compat::project_write_path(project.root().join(BREAKPOINTS_FILE));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let json = serde_json::to_string_pretty(&self.debugger().state().breakpoints)
            .map_err(|error| error.to_string())?;
        std::fs::write(path, json).map_err(|error| error.to_string())
    }

    /// Toggles a line breakpoint; returns whether the line now has one.
    pub fn toggle_breakpoint(&mut self, path: &Path, line: usize) -> Result<bool, String> {
        let debugger = self.debugger();
        let (added, lines, had_options) = {
            let mut state = debugger.state_mut();
            let lines = state.breakpoints.entry(path.to_owned()).or_default();
            let added = lines.insert(line) || {
                lines.remove(&line);
                false
            };
            let lines = lines.clone();
            if lines.is_empty() {
                state.breakpoints.remove(path);
            }
            let had_options = !added
                && state
                    .breakpoint_options
                    .remove(&(path.to_owned(), line))
                    .is_some();
            (added, lines, had_options)
        };
        if had_options {
            self.save_debug_state()?;
        }
        if let (Some(client), Some(project)) = (debugger.state_mut().client.as_mut(), &self.project)
        {
            let absolute = project.root().join(path);
            client.with_session(|session| session.update_breakpoints(absolute, lines));
        }
        self.save_breakpoints()?;
        Ok(added)
    }

    pub fn breakpoint_lines(&self, path: &Path) -> Vec<usize> {
        self.debugger()
            .state()
            .breakpoints
            .get(path)
            .map(|lines| lines.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn is_debugging(&self) -> bool {
        self.debugger().is_debugging()
    }

    /// Starts debugging the selected run configuration (or the detected Cargo binary).
    pub fn start_debugging(&mut self) -> Result<(), String> {
        if self.is_debugging() {
            return Err("A debug session is already running; stop it first.".into());
        }
        let root = self
            .project
            .as_ref()
            .ok_or("no project is open")?
            .root()
            .to_owned();
        let config = match self.execution().configurations().selected() {
            Some(config) => config.clone(),
            None => default_debug_configuration(&root)?,
        };
        self.debugger().state_mut().error = None;
        self.debugger().state_mut().name = config.name.clone();
        match &config.target {
            RunTarget::Cargo {
                command,
                package,
                bin,
                options,
                program_arguments,
            } if command == "run" => {
                let cwd = config.cwd(&root);
                let crate_dir = cwd.clone();
                let binary = bin
                    .clone()
                    .or_else(|| option_value(options, "--bin"))
                    .or_else(|| package.clone())
                    .or_else(|| cargo_package_name(&crate_dir))
                    .ok_or("cannot tell which binary to debug; set `bin` in the configuration")?;
                let example = option_value(options, "--example");
                let mut build_args = vec!["build".to_owned()];
                build_args.extend(
                    options
                        .iter()
                        .filter(|option| *option != "--release")
                        .cloned(),
                );
                if let Some(package) = package {
                    build_args.extend(["-p".into(), package.clone()]);
                }
                if let Some(bin) = bin {
                    build_args.extend(["--bin".into(), bin.clone()]);
                }
                let name = example.clone().unwrap_or(binary);
                let target = cargo_target_dir(&crate_dir, &root);
                let program = match example {
                    Some(_) => target.join("debug/examples").join(&name),
                    None => target.join("debug").join(&name),
                };
                self.debugger().state_mut().pending = Some(PendingDebug {
                    name: config.name.clone(),
                    program,
                    args: program_arguments.clone(),
                    cwd: cwd.clone(),
                    env: config
                        .env
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                });
                let plan =
                    TaskPlan::new(format!("Build {}", config.name), "cargo", build_args, cwd)
                        .with_kind(TaskKind::Build);
                // PLAT-SAVE-032: the before-launch build saves non-explicitly.
                self.start_task_at(
                    Ok(plan),
                    "Build",
                    TaskKind::Build,
                    SavePoint::BeforeLaunchBuild,
                )
                .inspect_err(|_| self.debugger().state_mut().pending = None)
            }
            RunTarget::Shell { program, arguments } => {
                let cwd = config.cwd(&root);
                let program = if Path::new(program).is_absolute() {
                    PathBuf::from(program)
                } else if program.contains('/') {
                    cwd.join(program)
                } else {
                    archipel_ide_runner::find_program(program)
                        .ok_or_else(|| format!("`{program}` was not found on PATH"))?
                };
                self.launch_debugger(PendingDebug {
                    name: config.name.clone(),
                    program,
                    args: arguments.clone(),
                    cwd,
                    env: config
                        .env
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                })
            }
            RunTarget::Gradle { options, .. } if is_gradle_test(&config.target) => {
                let _ = options;
                self.start_test_debugging(&config, &root)
            }
            RunTarget::Gradle {
                options,
                program_arguments,
                ..
            } => {
                let main_class = option_value_prefixed(options, "-Parchipel.mainClass=");
                self.start_java_debugging(&config, &root, main_class, program_arguments.clone())
            }
            other => Err(format!(
                "Debugging {} configurations is not supported yet; native programs (Cargo, shell) can be debugged with lldb-dap or GDB, Java (Gradle) with the java-debug plugin",
                other.type_name()
            )),
        }
    }

    /// Debugs a Gradle test run (the JUnit gutter's Debug entry): the test task runs with
    /// `--debug-jvm`, so the forked test JVM suspends and listens on 5005, and java-debug
    /// attaches to it once it reports that.
    fn start_test_debugging(
        &mut self,
        config: &RunConfiguration,
        root: &Path,
    ) -> Result<(), String> {
        let anchor = self.java_anchor(root)?;
        let mut debug_config = config.clone();
        if let RunTarget::Gradle { options, .. } = &mut debug_config.target
            && !options.iter().any(|option| option == "--debug-jvm")
        {
            options.push("--debug-jvm".into());
        }
        let plan = self
            .build_systems
            .registry()
            .plan_configuration(root, &debug_config);
        self.start_task(plan, &config.name, TaskKind::Test)?;
        self.debugger().state_mut().java = Some(PendingJava {
            name: config.name.clone(),
            anchor,
            main_class: None,
            args: Vec::new(),
            cwd: config.cwd(root),
            env: Vec::new(),
            stage: JavaStage::Build,
            started: Instant::now(),
            attach: true,
        });
        self.tool_windows.show(ToolWindow::Debug);
        self.status = format!("Starting Java test debugger for {}", config.name);
        Ok(())
    }

    /// Starts a Java debug session through jdtls and the java-debug plugin.
    fn start_java_debugging(
        &mut self,
        config: &RunConfiguration,
        root: &Path,
        main_class: Option<String>,
        args: Vec<String>,
    ) -> Result<(), String> {
        self.save_before(SavePoint::Debug);
        let anchor = self.java_anchor(root)?;
        self.debugger().state_mut().java = Some(PendingJava {
            name: config.name.clone(),
            anchor,
            main_class,
            args,
            cwd: config.cwd(root),
            env: config
                .env
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            stage: JavaStage::Build,
            started: Instant::now(),
            attach: false,
        });
        self.tool_windows.show(ToolWindow::Debug);
        self.status = format!("Starting Java debugger for {}", config.name);
        Ok(())
    }

    /// An open Java document: the active one, else the first source with a `main` method
    /// (opened so the language server knows it).
    fn java_anchor(&mut self, root: &Path) -> Result<PathBuf, String> {
        let is_java = |path: &Path| path.extension().is_some_and(|ext| ext == "java");
        let active = self
            .active_document
            .and_then(|index| self.documents.get(index))
            .map(|document| document.path.clone())
            .filter(|path| is_java(path));
        let relative = match active {
            Some(path) => path,
            None => {
                let found = find_java_main(root)
                    .ok_or("No Java source with a main method was found to debug")?;
                self.open_file(&found)?;
                found
            }
        };
        Ok(root.join(relative))
    }

    /// Starts the adapter for a built program.
    fn launch_debugger(&mut self, pending: PendingDebug) -> Result<(), String> {
        // PLAT-SAVE-030: everything is on disk just before the debuggee starts.
        self.save_before(SavePoint::Debug);
        if !pending.program.is_file() {
            return Err(format!(
                "{} does not exist; build it first",
                pending.program.display()
            ));
        }
        let adapter = Adapter::discover_native().map_err(|error| error.to_string())?;
        let launch =
            adapter.launch_request(&pending.program, &pending.args, &pending.cwd, &pending.env);
        let breakpoints = self.absolute_breakpoints()?;
        let mut client = DebugClient::start(adapter, launch, breakpoints, &pending.cwd)
            .map_err(|error| error.to_string())?;
        self.seed_session(&mut client);
        self.debugger().attach_session(&pending.name, client);
        self.tool_windows.show(ToolWindow::Debug);
        self.status = format!("Debugging {}", pending.name);
        Ok(())
    }

    /// Called when a task finishes: launches a debug session waiting for its build.
    pub(crate) fn continue_pending_debug(&mut self) {
        let pending = self.debugger().state_mut().pending.take();
        let Some(pending) = pending else {
            return;
        };
        let exited_cleanly = matches!(self.execution().runner().state(), TaskState::Exited(0));
        if exited_cleanly {
            if let Err(error) = self.launch_debugger(pending) {
                self.debugger().state_mut().error = Some(error.clone());
                self.status = error;
            }
        } else {
            self.status = format!("Build failed; not debugging {}", pending.name);
        }
    }

    /// Pumps the adapter. A new execution point is announced as `session_paused`; the editor
    /// reveals it in [`IdeState::debug_paused`].
    pub fn poll_debug(&mut self) {
        self.poll_java_debug();
        self.debugger().poll_session();
        self.dispatch_reactions();
    }

    /// Reveals the execution point in the editor and shows the Debug tool window.
    pub(crate) fn debug_paused(&mut self, frame: &archipel_ide_dap::StackFrame) {
        self.reveal_frame(frame);
        self.tool_windows.show(ToolWindow::Debug);
    }

    fn reveal_frame(&mut self, frame: &archipel_ide_dap::StackFrame) {
        let (Some(path), Some(project)) = (&frame.path, &self.project) else {
            return;
        };
        let Ok(relative) = path.strip_prefix(project.root()).map(Path::to_path_buf) else {
            return;
        };
        if self.open_file(&relative).is_err() {
            return;
        }
        if let Some(index) = self.active_document {
            let document = &mut self.documents[index];
            let line = usize::try_from(frame.line - 1).unwrap_or(0);
            if let Some(start) = document.text.line_to_char(line) {
                document
                    .selections
                    .set_single(archipel_ide_text::Selection::caret(start));
            }
        }
    }

    /// Where the editor should draw the execution-point marker.
    pub fn execution_point(&self) -> Option<ExecutionPoint> {
        let debugger = self.debugger();
        let state = debugger.state();
        let session = state.client.as_ref()?.session();
        if !matches!(session.state(), SessionState::Stopped { .. }) {
            return None;
        }
        let frame = session.selected_frame()?;
        let path = frame
            .path
            .as_ref()?
            .strip_prefix(self.project.as_ref()?.root())
            .ok()?
            .to_owned();
        Some(ExecutionPoint {
            path,
            line: usize::try_from(frame.line - 1).ok()?,
        })
    }

    fn with_debug_client(&mut self, action: impl FnOnce(&mut DebugClient)) -> Result<(), String> {
        let debugger = self.debugger();
        let mut state = debugger.state_mut();
        let client = state
            .client
            .as_mut()
            .filter(|client| !client.session().is_terminated())
            .ok_or("no debug session is running")?;
        action(client);
        Ok(())
    }
}

/// A Gradle configuration that runs tests (`--tests` filter or a `test` task).
fn is_gradle_test(target: &RunTarget) -> bool {
    let RunTarget::Gradle { tasks, options, .. } = target else {
        return false;
    };
    options.iter().any(|option| option == "--tests")
        || tasks.iter().any(|task| {
            let name = task.rsplit(':').next().unwrap_or(task);
            name == "test" || name.ends_with("UnitTest")
        })
}

fn option_value(options: &[String], name: &str) -> Option<String> {
    options
        .iter()
        .position(|option| option == name)
        .and_then(|index| options.get(index + 1))
        .cloned()
}

fn option_value_prefixed(options: &[String], prefix: &str) -> Option<String> {
    options
        .iter()
        .find_map(|option| option.strip_prefix(prefix))
        .map(str::to_owned)
}

/// Project-relative path of a Java file declaring `static void main`, searching
/// `src/` trees (bounded, skipping build output).
fn find_java_main(root: &Path) -> Option<PathBuf> {
    let mut stack = vec![root.to_owned()];
    let mut visited = 0;
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).ok()?.flatten() {
            visited += 1;
            if visited > 20_000 {
                return None;
            }
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if !name.starts_with('.') && !matches!(&*name, "build" | "out" | "target") {
                    stack.push(path);
                }
            } else if name.ends_with(".java")
                && std::fs::read_to_string(&path)
                    .is_ok_and(|text| text.contains("static void main"))
            {
                return path.strip_prefix(root).ok().map(Path::to_path_buf);
            }
        }
    }
    None
}

fn cargo_package_name(crate_dir: &Path) -> Option<String> {
    let manifest = std::fs::read_to_string(crate_dir.join("Cargo.toml")).ok()?;
    let value: toml::Table = toml::from_str(&manifest).ok()?;
    value
        .get("package")?
        .get("name")?
        .as_str()
        .map(str::to_owned)
}

/// `target/` of the workspace that contains `crate_dir` (closest existing, else local).
fn cargo_target_dir(crate_dir: &Path, root: &Path) -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir);
    }
    crate_dir
        .ancestors()
        .take_while(|dir| dir.starts_with(root))
        .filter(|dir| dir.join("Cargo.toml").is_file())
        .last()
        .unwrap_or(crate_dir)
        .join("target")
}

fn default_debug_configuration(root: &Path) -> Result<RunConfiguration, String> {
    if root.join("Cargo.toml").is_file() {
        return Ok(RunConfiguration::new(
            "Debug",
            RunTarget::Cargo {
                command: "run".into(),
                package: None,
                bin: None,
                options: Vec::new(),
                program_arguments: Vec::new(),
            },
        ));
    }
    Err("select a run configuration to debug (Cargo or shell configurations are supported)".into())
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            spec(
                "debug.toggleBreakpoint",
                "Toggle Line Breakpoint",
                "Run",
                Some("⌘F8"),
                r#"{"path":"string?","line":"number?"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let path = match params.get("path").and_then(Value::as_str) {
                    Some(path) => PathBuf::from(path),
                    None => state
                        .active_document
                        .and_then(|index| state.documents.get(index))
                        .map(|document| document.path.clone())
                        .ok_or_else(|| CommandError::Disabled("debug.toggleBreakpoint".into()))?,
                };
                let line = match params.get("line") {
                    Some(_) => usize_param(&params, "line")?,
                    None => {
                        let index = state.active_document.ok_or_else(|| {
                            CommandError::Disabled("debug.toggleBreakpoint".into())
                        })?;
                        let document = &state.documents[index];
                        document
                            .text
                            .char_to_line(document.selections.primary().head)
                    }
                };
                let added = state
                    .toggle_breakpoint(&path, line)
                    .map_err(CommandError::Failed)?;
                Ok(json!({"added": added}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("debug.start", "Debug", "Run", Some("⌃D"), "{}"),
            |state: &IdeState| {
                state.project.is_some() && !state.execution().is_running() && !state.is_debugging()
            },
            |state: &mut IdeState, _| {
                state.start_debugging().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    for (id, label, shortcut, step) in [
        (
            "debug.resume",
            "Resume Program",
            Some("⌥⌘R"),
            Some(Step::Continue),
        ),
        ("debug.pause", "Pause Program", None, Some(Step::Pause)),
        ("debug.stepOver", "Step Over", Some("F8"), Some(Step::Next)),
        (
            "debug.stepInto",
            "Step Into",
            Some("F7"),
            Some(Step::StepIn),
        ),
        (
            "debug.stepOut",
            "Step Out",
            Some("⇧F8"),
            Some(Step::StepOut),
        ),
        (
            "debug.forceStepInto",
            "Force Step Into",
            Some("⌥⇧F7"),
            Some(Step::ForceStepIn),
        ),
        ("debug.stop", "Stop Debugging", None, None),
    ] {
        registry
            .register(Command::new(
                spec(id, label, "Run", shortcut, "{}"),
                move |state: &IdeState| {
                    let debugger = state.debugger();
                    let debugger = debugger.state();
                    let Some(session) = debugger.client.as_ref().map(|c| c.session()) else {
                        return false;
                    };
                    match (step, session.state()) {
                        (_, SessionState::Terminated { .. }) => false,
                        (None, _) => true,
                        (Some(Step::Pause), SessionState::Running) => true,
                        (Some(Step::Pause), _) => false,
                        (Some(_), SessionState::Stopped { .. }) => true,
                        _ => false,
                    }
                },
                move |state: &mut IdeState, _| {
                    state
                        .with_debug_client(|client| match step {
                            Some(step) => client.with_session(|session| session.step(step)),
                            None => client.stop(),
                        })
                        .map_err(CommandError::Failed)?;
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "debug.selectFrame",
                "Select Stack Frame",
                "Run",
                None,
                r#"{"frame":0}"#,
            ),
            |state: &IdeState| state.debugger().state().client.is_some(),
            |state: &mut IdeState, params| {
                let frame = params["frame"].as_i64().ok_or_else(|| {
                    CommandError::InvalidParams("frame must be an integer".into())
                })?;
                state
                    .with_debug_client(|client| client.with_session(|s| s.select_frame(frame)))
                    .map_err(CommandError::Failed)?;
                let selected = state
                    .debugger()
                    .state()
                    .client
                    .as_ref()
                    .and_then(|client| client.session().selected_frame().cloned());
                if let Some(frame) = selected {
                    state.reveal_frame(&frame);
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "debug.loadVariables",
                "Expand Variable",
                "Run",
                None,
                r#"{"reference":0}"#,
            ),
            |state: &IdeState| state.debugger().state().client.is_some(),
            |state: &mut IdeState, params| {
                let reference = params["reference"].as_i64().ok_or_else(|| {
                    CommandError::InvalidParams("reference must be an integer".into())
                })?;
                state
                    .with_debug_client(|client| {
                        client.with_session(|session| session.request_variables(reference))
                    })
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "debug.evaluate",
                "Evaluate Expression",
                "Run",
                None,
                r#"{"expression":"string"}"#,
            ),
            |state: &IdeState| {
                state
                    .debugger()
                    .state()
                    .client
                    .as_ref()
                    .is_some_and(|client| {
                        matches!(client.session().state(), SessionState::Stopped { .. })
                    })
            },
            |state: &mut IdeState, params| {
                let expression = string_param(&params, "expression")?;
                state
                    .with_debug_client(|client| client.with_session(|s| s.evaluate(&expression)))
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "debug.runToCursor",
                "Run to Cursor",
                "Run",
                Some("⌥F9"),
                r#"{"path":"string?","line":"number?"}"#,
            ),
            |state: &IdeState| {
                state.active_document.is_some()
                    && state
                        .debugger()
                        .state()
                        .client
                        .as_ref()
                        .is_some_and(|client| {
                            matches!(client.session().state(), SessionState::Stopped { .. })
                        })
            },
            |state: &mut IdeState, params| {
                let path = params
                    .get("path")
                    .and_then(Value::as_str)
                    .map(PathBuf::from);
                let line = match params.get("line") {
                    Some(_) => Some(usize_param(&params, "line")?),
                    None => None,
                };
                state
                    .run_to_cursor(path.as_deref(), line)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "debug.editBreakpoint",
                "Edit Breakpoint",
                "Run",
                Some("⌘⇧F8"),
                r#"{"path":"string?","line":"number?"}"#,
            ),
            |state: &IdeState| state.project.is_some() && state.active_document.is_some(),
            |state: &mut IdeState, params| {
                let (path, line) = breakpoint_target(state, &params)?;
                if !state.breakpoint_lines(&path).contains(&line) {
                    state
                        .toggle_breakpoint(&path, line)
                        .map_err(CommandError::Failed)?;
                }
                state.debugger().state_mut().editing_breakpoint = Some((path, line));
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "debug.closeBreakpointEditor",
                "Close Breakpoint Properties",
                "Run",
                None,
                "{}",
            ),
            |state: &IdeState| state.debugger().state().editing_breakpoint.is_some(),
            |state: &mut IdeState, _| {
                state.debugger().state_mut().editing_breakpoint = None;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "debug.setBreakpointOptions",
                "Set Breakpoint Condition or Logpoint",
                "Run",
                None,
                r#"{"path":"string?","line":"number?","condition":"string?","logMessage":"string?","hitCondition":"string?"}"#,
            ),
            |state: &IdeState| state.project.is_some() && state.active_document.is_some(),
            |state: &mut IdeState, params| {
                let (path, line) = breakpoint_target(state, &params)?;
                let text = |key: &str| {
                    params
                        .get(key)
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                };
                let options = BreakpointOptions {
                    condition: text("condition"),
                    log_message: text("logMessage"),
                    hit_condition: text("hitCondition"),
                };
                state
                    .set_breakpoint_options(&path, line, options)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "debug.evaluateDialog",
                "Evaluate Expression…",
                "Run",
                Some("⌥F8"),
                "{}",
            ),
            |state: &IdeState| {
                state
                    .debugger()
                    .state()
                    .client
                    .as_ref()
                    .is_some_and(|client| {
                        matches!(client.session().state(), SessionState::Stopped { .. })
                    })
            },
            |state: &mut IdeState, _| {
                let seed = state
                    .active_document
                    .and_then(|index| state.documents.get(index))
                    .and_then(|document| {
                        let selection = document.selections.primary();
                        (selection.anchor != selection.head)
                            .then(|| {
                                let (from, to) = (
                                    selection.anchor.min(selection.head),
                                    selection.anchor.max(selection.head),
                                );
                                document.text.slice(from, to).ok()
                            })
                            .flatten()
                    })
                    .unwrap_or_default();
                state.debugger().state_mut().evaluate_dialog = Some(seed);
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "debug.closeEvaluateDialog",
                "Close Evaluate Expression",
                "Run",
                None,
                "{}",
            ),
            |state: &IdeState| state.debugger().state().evaluate_dialog.is_some(),
            |state: &mut IdeState, _| {
                state.debugger().state_mut().evaluate_dialog = None;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "debug.addWatch",
                "Add Watch",
                "Run",
                None,
                r#"{"expression":"string"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let expression = string_param(&params, "expression")?;
                state.add_watch(&expression).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "debug.removeWatch",
                "Remove Watch",
                "Run",
                None,
                r#"{"index":0}"#,
            ),
            |state: &IdeState| !state.debugger().state().watches.is_empty(),
            |state: &mut IdeState, params| {
                let index = usize_param(&params, "index")?;
                state.remove_watch(index).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "debug.editWatch",
                "Edit Watch",
                "Run",
                None,
                r#"{"index":0,"expression":"string"}"#,
            ),
            |state: &IdeState| !state.debugger().state().watches.is_empty(),
            |state: &mut IdeState, params| {
                let index = usize_param(&params, "index")?;
                let expression = string_param(&params, "expression")?;
                state
                    .edit_watch(index, &expression)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
}

/// The `path`/`line` params of a breakpoint command, defaulting to the caret.
fn breakpoint_target(state: &IdeState, params: &Value) -> Result<(PathBuf, usize), CommandError> {
    let document = state
        .active_document
        .and_then(|index| state.documents.get(index));
    let path = match params.get("path").and_then(Value::as_str) {
        Some(path) => PathBuf::from(path),
        None => document
            .map(|document| document.path.clone())
            .ok_or_else(|| CommandError::Failed("no editor is active".into()))?,
    };
    let line = match params.get("line") {
        Some(_) => usize_param(params, "line")?,
        None => {
            let document =
                document.ok_or_else(|| CommandError::Failed("no editor is active".into()))?;
            document
                .text
                .char_to_line(document.selections.primary().head)
        }
    };
    Ok((path, line))
}

#[cfg(all(test, unix))]
mod tests {
    use crate::{IdeState, built_in_commands};
    use archipel_ide_dap::SessionState;
    use serde_json::{Value, json};

    #[test]
    fn reads_the_package_name_from_a_manifest() {
        let dir = std::env::temp_dir().join(format!("jt-pkg-name-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        assert_eq!(super::cargo_package_name(&dir).as_deref(), Some("demo"));
        std::fs::remove_dir_all(dir).unwrap();
    }
    use std::time::{Duration, Instant};

    #[test]
    fn breakpoints_toggle_and_persist() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-debug-bp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("main.c"), "int main(void) {\n  return 0;\n}\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "main.c"}))
            .unwrap();
        let added = commands
            .execute("debug.toggleBreakpoint", &mut state, json!({"line": 1}))
            .unwrap();
        assert_eq!(added["added"], true);
        assert_eq!(state.breakpoint_lines(std::path::Path::new("main.c")), [1]);
        let mut reopened = IdeState::default();
        commands
            .execute("project.open", &mut reopened, json!({"path": root}))
            .unwrap();
        assert_eq!(
            reopened.breakpoint_lines(std::path::Path::new("main.c")),
            [1]
        );
        commands
            .execute(
                "debug.toggleBreakpoint",
                &mut reopened,
                json!({"path": "main.c", "line": 1}),
            )
            .unwrap();
        assert!(
            reopened
                .breakpoint_lines(std::path::Path::new("main.c"))
                .is_empty()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn breakpoint_options_and_watches_persist_per_project() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-debug-opts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("main.c"), "int main(void) {\n  return 0;\n}\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "main.c"}))
            .unwrap();
        // Setting a condition on a line without a breakpoint creates one.
        commands
            .execute(
                "debug.setBreakpointOptions",
                &mut state,
                json!({"path": "main.c", "line": 1, "condition": " x > 1 ", "logMessage": "  "}),
            )
            .unwrap();
        assert_eq!(state.breakpoint_lines(std::path::Path::new("main.c")), [1]);
        let options = state
            .breakpoint_options(std::path::Path::new("main.c"), 1)
            .unwrap();
        assert_eq!(options.condition.as_deref(), Some("x > 1"));
        assert_eq!(options.log_message, None);
        commands
            .execute("debug.addWatch", &mut state, json!({"expression": "x + 1"}))
            .unwrap();
        commands
            .execute("debug.addWatch", &mut state, json!({"expression": "y"}))
            .unwrap();
        commands
            .execute(
                "debug.editWatch",
                &mut state,
                json!({"index": 1, "expression": "z"}),
            )
            .unwrap();
        assert!(
            commands
                .execute(
                    "debug.editWatch",
                    &mut state,
                    json!({"index": 1, "expression": "x + 1"})
                )
                .is_err()
        );
        assert!(
            commands
                .execute("debug.removeWatch", &mut state, json!({"index": 5}))
                .is_err()
        );

        let mut reopened = IdeState::default();
        commands
            .execute("project.open", &mut reopened, json!({"path": root}))
            .unwrap();
        assert_eq!(reopened.debugger().state().watches, ["x + 1", "z"]);
        assert_eq!(
            reopened
                .breakpoint_options(std::path::Path::new("main.c"), 1)
                .and_then(|o| o.condition),
            Some("x > 1".to_owned())
        );
        // Removing the breakpoint drops its options; removing every watch deletes the file.
        commands
            .execute("editor.openFile", &mut reopened, json!({"path": "main.c"}))
            .unwrap();
        commands
            .execute("debug.toggleBreakpoint", &mut reopened, json!({"line": 1}))
            .unwrap();
        assert!(
            reopened
                .breakpoint_options(std::path::Path::new("main.c"), 1)
                .is_none()
        );
        commands
            .execute(
                "debug.editWatch",
                &mut reopened,
                json!({"index": 0, "expression": ""}),
            )
            .unwrap();
        commands
            .execute("debug.removeWatch", &mut reopened, json!({"index": 0}))
            .unwrap();
        assert!(!root.join(super::DEBUG_STATE_FILE).exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn edit_breakpoint_opens_the_properties_popup_and_debug_dialogs_need_a_stop() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-debug-edit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("main.c"), "int main(void) {\n  return 0;\n}\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "main.c"}))
            .unwrap();
        commands
            .execute("debug.editBreakpoint", &mut state, json!({"line": 0}))
            .unwrap();
        assert_eq!(
            state.debugger().state().editing_breakpoint,
            Some((std::path::PathBuf::from("main.c"), 0))
        );
        assert_eq!(state.breakpoint_lines(std::path::Path::new("main.c")), [0]);
        commands
            .execute("debug.closeBreakpointEditor", &mut state, Value::Null)
            .unwrap();
        assert!(state.debugger().state().editing_breakpoint.is_none());
        for id in [
            "debug.evaluateDialog",
            "debug.runToCursor",
            "debug.forceStepInto",
        ] {
            assert!(!commands.is_enabled(id, &state), "{id} needs a stop");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn debugs_a_shell_configuration_with_the_native_adapter() {
        let Some(compiler) = ["cc", "gcc", "clang"]
            .into_iter()
            .find(|c| archipel_ide_runner::find_program(c).is_some())
        else {
            eprintln!("skipping: no C compiler");
            return;
        };
        if archipel_ide_dap::Adapter::discover_native().is_err() {
            eprintln!("skipping: no lldb-dap or gdb");
            return;
        }
        let root =
            std::env::temp_dir().join(format!("archipel-ide-debug-run-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("main.c"),
            "#include <stdio.h>\nint main(void) {\n  int x = 1;\n  printf(\"x=%d\\n\", x);\n  return 0;\n}\n",
        )
        .unwrap();
        assert!(
            std::process::Command::new(compiler)
                .args(["-g", "-O0", "-o", "app", "main.c"])
                .current_dir(&root)
                .status()
                .unwrap()
                .success()
        );
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute(
                "runConfig.save",
                &mut state,
                json!({"configuration": {"name": "app", "type": "shell", "program": "./app"}}),
            )
            .unwrap();
        commands
            .execute(
                "debug.toggleBreakpoint",
                &mut state,
                json!({"path": "main.c", "line": 3}),
            )
            .unwrap();
        commands
            .execute("debug.start", &mut state, Value::Null)
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(60);
        while state.execution_point().is_none() && Instant::now() < deadline {
            state.poll_debug();
            std::thread::sleep(Duration::from_millis(20));
        }
        let point = state.execution_point().expect("stopped at the breakpoint");
        assert_eq!((point.path.to_str(), point.line), (Some("main.c"), 3));
        assert_eq!(
            state.documents[state.active_document.unwrap()].path,
            std::path::Path::new("main.c")
        );
        assert!(commands.is_enabled("debug.stepOver", &state));
        commands
            .execute("debug.resume", &mut state, Value::Null)
            .unwrap();
        while !state
            .debugger()
            .state()
            .client
            .as_ref()
            .unwrap()
            .session()
            .is_terminated()
            && Instant::now() < deadline
        {
            state.poll_debug();
            std::thread::sleep(Duration::from_millis(20));
        }
        {
            let debugger = state.debugger();
            let debugger = debugger.state();
            let session = debugger.client.as_ref().unwrap().session();
            assert!(matches!(session.state(), SessionState::Terminated { .. }));
        }
        assert!(!state.is_debugging());
        std::fs::remove_dir_all(root).unwrap();
    }
}

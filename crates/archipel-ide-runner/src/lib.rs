//! Process execution for builds, runs and tests.
//!
//! [`TaskService`] runs one child process at a time, streams its output into a
//! line-oriented [`ConsoleBuffer`] and recognises build steps and compiler messages
//! ([`BuildEvents`]). Build tools are described by the [`BuildSystem`] trait.

pub mod build_system;
pub mod console;
pub mod events;
pub mod run_config;
pub mod test_results;
pub mod tool_env;

pub use build_system::{BuildSystem, BuildSystemKind, BuildSystemRegistry, find_program};
pub use console::{ConsoleBuffer, ConsoleLine};
pub use events::{BuildEvents, BuildMessage, BuildStep, MessageKind, StepOutcome};
pub use run_config::{RunConfiguration, RunManager, RunTarget};
pub use test_results::{
    ConsoleTestParser, TestCase, TestCounts, TestId, TestOutcome, TestResults, TestSuite,
};
pub use tool_env::{ToolEnvironment, set_tool_environment, tool_environment};

use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("no supported build system found in {0}")]
    NoBuildSystem(PathBuf),
    #[error("`{program}` was not found on PATH. {hint}")]
    ToolNotFound { program: String, hint: String },
    #[error("{0}")]
    NotRunnable(String),
    #[error("failed to start task: {0}")]
    Start(#[from] std::io::Error),
    #[error("task process lock is unavailable")]
    Lock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputChunk {
    pub stream: OutputStream,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    Idle,
    Running,
    Exited(i32),
    Cancelled,
    /// The task could not be started; the message says why and what to do.
    Failed(String),
}

/// What a task does; decides where its output is shown and how it is labelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskKind {
    Build,
    Run,
    Test,
    Sync,
    #[default]
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskPlan {
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    pub kind: TaskKind,
}

impl TaskPlan {
    pub fn new(
        name: impl Into<String>,
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
        cwd: impl Into<PathBuf>,
    ) -> Self {
        Self {
            name: name.into(),
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
            cwd: cwd.into(),
            env: Vec::new(),
            kind: TaskKind::Custom,
        }
    }

    pub fn with_kind(mut self, kind: TaskKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Shell-like rendering for display (`./gradlew build`).
    pub fn command_line(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .map(|part| {
                if part.is_empty() || part.contains(char::is_whitespace) {
                    format!("\"{part}\"")
                } else {
                    part.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Resolves a message path printed by the tool against the task directory.
    pub fn resolve(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_owned()
        } else {
            self.cwd.join(path)
        }
    }
}

struct ActiveTask {
    child: Arc<Mutex<Child>>,
    /// Behind a mutex only so the task service is `Sync` (it can live in a shared service);
    /// it is read from one thread at a time.
    output: Mutex<Receiver<OutputChunk>>,
}

pub struct TaskService {
    active: Option<ActiveTask>,
    state: TaskState,
    output: Vec<OutputChunk>,
    console: ConsoleBuffer,
    events: BuildEvents,
    plan: Option<TaskPlan>,
    started: Option<Instant>,
    started_at: Option<std::time::SystemTime>,
    elapsed: Option<Duration>,
    /// Number of tasks started so far; identifies the current run.
    runs: u64,
    tests: TestResults,
    test_parser: ConsoleTestParser,
    /// JUnit reports that could not be read after the last test task.
    pub test_report_errors: Vec<(PathBuf, String)>,
}

impl Default for TaskService {
    fn default() -> Self {
        Self {
            active: None,
            state: TaskState::Idle,
            output: Vec::new(),
            console: ConsoleBuffer::default(),
            events: BuildEvents::default(),
            plan: None,
            started: None,
            started_at: None,
            elapsed: None,
            runs: 0,
            tests: TestResults::default(),
            test_parser: ConsoleTestParser::default(),
            test_report_errors: Vec::new(),
        }
    }
}

/// Raw chunks retained for consumers that need exact bytes; bounded like the console.
const MAX_RAW_CHUNKS: usize = 10_000;

impl TaskService {
    pub fn start(&mut self, plan: TaskPlan) -> Result<(), RunnerError> {
        self.cancel()?;
        match self.spawn(&plan) {
            Ok(active) => {
                self.reset(plan);
                self.state = TaskState::Running;
                self.active = Some(active);
                Ok(())
            }
            Err(error) => {
                let error = match error {
                    RunnerError::Start(io) if io.kind() == std::io::ErrorKind::NotFound => {
                        RunnerError::ToolNotFound {
                            program: plan.program.clone(),
                            hint: "Install it or fix the run configuration.".into(),
                        }
                    }
                    other => other,
                };
                self.reset(plan);
                self.state = TaskState::Failed(error.to_string());
                Err(error)
            }
        }
    }

    /// Records a task that could not even be planned, so the failure is visible
    /// in the tool window instead of only in the status bar.
    pub fn fail(&mut self, plan: Option<TaskPlan>, message: impl Into<String>) {
        if self.is_running() {
            return;
        }
        self.active = None;
        self.reset(plan.unwrap_or_else(|| TaskPlan::new("Build", "", Vec::<String>::new(), "")));
        self.state = TaskState::Failed(message.into());
    }

    fn reset(&mut self, plan: TaskPlan) {
        self.runs += 1;
        self.output.clear();
        self.console.clear();
        self.events = BuildEvents::default();
        self.plan = Some(plan);
        self.started = Some(Instant::now());
        // Report files may carry coarse timestamps; allow a little slack.
        self.started_at = std::time::SystemTime::now().checked_sub(Duration::from_secs(2));
        self.elapsed = None;
        self.tests = TestResults::default();
        self.test_parser = ConsoleTestParser::default();
        self.test_report_errors.clear();
    }

    fn spawn(&self, plan: &TaskPlan) -> Result<ActiveTask, RunnerError> {
        let mut command = Command::new(&plan.program);
        // Managed JDK/Gradle first on PATH and JAVA_HOME, unless the plan sets them.
        tool_env::tool_environment().apply(&mut command, &plan.env);
        command
            .args(&plan.args)
            .current_dir(&plan.cwd)
            .envs(plan.env.iter().map(|(key, value)| (key, value)))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let child = Arc::new(Mutex::new(child));
        let (sender, output) = mpsc::channel();
        for (reader, stream) in [
            (
                stdout.map(|r| Box::new(r) as Box<dyn Read + Send>),
                OutputStream::Stdout,
            ),
            (
                stderr.map(|r| Box::new(r) as Box<dyn Read + Send>),
                OutputStream::Stderr,
            ),
        ] {
            if let Some(reader) = reader
                && let Err(error) = spawn_reader(reader, stream, sender.clone())
            {
                if let Ok(mut process) = child.lock() {
                    let _ = process.kill();
                }
                return Err(error.into());
            }
        }
        Ok(ActiveTask {
            child,
            output: Mutex::new(output),
        })
    }

    pub fn poll(&mut self) -> Result<(), RunnerError> {
        let Some(active) = &self.active else {
            return Ok(());
        };
        let now = self.now();
        let chunks: Vec<OutputChunk> = active
            .output
            .lock()
            .map_err(|_| RunnerError::Lock)?
            .try_iter()
            .collect();
        let mut exited = None;
        if self.state == TaskState::Running {
            let mut child = active.child.lock().map_err(|_| RunnerError::Lock)?;
            if let Some(status) = child.try_wait()? {
                exited = Some(status.code().unwrap_or(-1));
            }
        }
        for chunk in chunks {
            self.ingest(chunk, now);
        }
        if let Some(code) = exited {
            // The readers may still hold the last bytes; wait briefly for their EOF.
            // Bounded, because a daemon that inherited the pipes can keep them open.
            if let Some(active) = &self.active {
                let deadline = Instant::now() + Duration::from_millis(300);
                let mut rest = Vec::new();
                loop {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    let received = match active.output.lock() {
                        Ok(output) => output.recv_timeout(remaining),
                        Err(_) => break,
                    };
                    match received {
                        Ok(chunk) => rest.push(chunk),
                        Err(_) => break,
                    }
                }
                for chunk in rest {
                    self.ingest(chunk, now);
                }
            }
            self.complete(now, code == 0);
            self.state = TaskState::Exited(code);
        }
        Ok(())
    }

    fn ingest(&mut self, chunk: OutputChunk, now: Duration) {
        let completed = self.console.push(chunk.stream, &chunk.text);
        let lines = self.console.lines();
        let start = lines.len() - completed.min(lines.len());
        let base = self.console.dropped();
        for (offset, line) in lines[start..].iter().enumerate() {
            self.events.on_line(&line.text, now, base + start + offset);
            self.test_parser.on_line(&line.text, &mut self.tests);
        }
        if self.output.len() >= MAX_RAW_CHUNKS {
            self.output.drain(..MAX_RAW_CHUNKS / 2);
        }
        self.output.push(chunk);
    }

    fn complete(&mut self, now: Duration, success: bool) {
        let before = self.console.lines().len();
        self.console.finish();
        let base = self.console.dropped();
        let lines = self.console.lines();
        for (index, line) in lines.iter().enumerate().skip(before) {
            self.events.on_line(&line.text, now, base + index);
            self.test_parser.on_line(&line.text, &mut self.tests);
        }
        self.events.finish(now, success);
        self.test_parser.finish(&mut self.tests);
        if let Some(plan) = &self.plan
            && plan.kind == TaskKind::Test
            && let Some(since) = self.started_at
        {
            let (reports, errors) = TestResults::collect_reports(&plan.cwd, since);
            self.tests.merge(reports);
            self.test_report_errors = errors;
        }
        self.elapsed = Some(now);
    }

    fn now(&self) -> Duration {
        self.started
            .map(|started| started.elapsed())
            .unwrap_or_default()
    }

    pub fn cancel(&mut self) -> Result<(), RunnerError> {
        if let Some(active) = &self.active
            && self.state == TaskState::Running
        {
            active.child.lock().map_err(|_| RunnerError::Lock)?.kill()?;
            let now = self.now();
            self.complete(now, false);
            self.state = TaskState::Cancelled;
        }
        Ok(())
    }

    pub fn state(&self) -> &TaskState {
        &self.state
    }
    pub fn is_running(&self) -> bool {
        self.state == TaskState::Running
    }
    /// Raw output chunks exactly as received (bounded).
    pub fn output(&self) -> &[OutputChunk] {
        &self.output
    }
    /// Cleaned, complete output lines.
    pub fn console(&self) -> &ConsoleBuffer {
        &self.console
    }
    /// Steps and messages recognised in the output.
    pub fn events(&self) -> &BuildEvents {
        &self.events
    }
    /// Identifies the current run; changes whenever a task is started.
    pub fn run_id(&self) -> u64 {
        self.runs
    }
    pub fn plan(&self) -> Option<&TaskPlan> {
        self.plan.as_ref()
    }
    /// Test results of the current/last task (live for libtest and Go, from JUnit
    /// reports once a test task has finished).
    pub fn tests(&self) -> &TestResults {
        &self.tests
    }
    /// Time since start while running, total duration once finished.
    pub fn elapsed(&self) -> Option<Duration> {
        match self.state {
            TaskState::Running => Some(self.now()),
            _ => self.elapsed,
        }
    }
}

impl Drop for TaskService {
    fn drop(&mut self) {
        let _ = self.cancel();
    }
}

fn spawn_reader(
    mut reader: impl Read + Send + 'static,
    stream: OutputStream,
    sender: mpsc::Sender<OutputChunk>,
) -> Result<(), std::io::Error> {
    thread::Builder::new()
        .name(format!("archipel-ide-task-{stream:?}"))
        .spawn(move || {
            let mut buffer = [0_u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => {
                        if sender
                            .send(OutputChunk {
                                stream,
                                text: String::from_utf8_lossy(&buffer[..read]).into_owned(),
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
        })?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn shell(script: &str) -> TaskPlan {
        TaskPlan::new("test", "/bin/sh", ["-c", script], "/tmp")
    }

    fn wait(service: &mut TaskService) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while service.is_running() && Instant::now() < deadline {
            service.poll().unwrap();
            thread::sleep(Duration::from_millis(5));
        }
        service.poll().unwrap();
    }

    #[test]
    fn streams_real_process_output_and_exit_code() {
        let mut service = TaskService::default();
        service
            .start(shell("printf out; printf err >&2; exit 7"))
            .unwrap();
        wait(&mut service);
        assert_eq!(service.state(), &TaskState::Exited(7));
        assert!(
            service
                .output()
                .iter()
                .any(|chunk| chunk.stream == OutputStream::Stdout && chunk.text.contains("out"))
        );
        let lines: Vec<_> = service
            .console()
            .lines()
            .iter()
            .map(|line| (line.stream, line.text.as_str()))
            .collect();
        assert!(lines.contains(&(OutputStream::Stdout, "out")), "{lines:?}");
        assert!(lines.contains(&(OutputStream::Stderr, "err")), "{lines:?}");
        assert!(service.elapsed().is_some());
    }

    #[test]
    fn recognises_build_steps_from_live_output() {
        let mut service = TaskService::default();
        service
            .start(shell(
                "echo '> Task :compileJava'; sleep 0.05; \
                 echo 'src/A.java:3: error: boom' >&2; echo '> Task :test'; echo 'BUILD FAILED in 1s'; exit 1",
            ))
            .unwrap();
        wait(&mut service);
        let events = service.events();
        assert_eq!(events.steps().len(), 2, "{:?}", events.steps());
        assert_eq!(events.count(MessageKind::Error), 1);
        assert_eq!(events.summary(), Some("BUILD FAILED in 1s"));
        let message = &events.messages()[0];
        assert_eq!(
            service
                .plan()
                .unwrap()
                .resolve(message.file.as_ref().unwrap()),
            PathBuf::from("/tmp/src/A.java")
        );
    }

    #[test]
    fn passes_environment_and_reports_missing_programs() {
        let mut service = TaskService::default();
        service
            .start(shell("printf \"$ARCHIPEL_TEST_VALUE\"").with_env("ARCHIPEL_TEST_VALUE", "42"))
            .unwrap();
        wait(&mut service);
        assert_eq!(service.console().lines()[0].text, "42");
        let error = service
            .start(TaskPlan::new(
                "x",
                "archipel-ide-missing-program",
                ["a"],
                "/tmp",
            ))
            .unwrap_err();
        assert!(matches!(error, RunnerError::ToolNotFound { .. }));
        assert!(
            matches!(service.state(), TaskState::Failed(message) if message.contains("archipel-ide-missing-program"))
        );
        service.fail(None, "no supported build system");
        assert_eq!(
            service.state(),
            &TaskState::Failed("no supported build system".into())
        );
    }

    #[test]
    fn cancels_running_process() {
        let mut service = TaskService::default();
        service.start(shell("sleep 30")).unwrap();
        service.cancel().unwrap();
        assert_eq!(service.state(), &TaskState::Cancelled);
    }

    #[test]
    fn test_tasks_collect_console_and_report_results() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-task-tests-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let script = "echo 'test a::b ... ok'; mkdir -p build/test-results/test; \
            printf '<testsuite name=\"T\"><testcase classname=\"T\" name=\"x()\" time=\"0.1\"><failure message=\"boom\"/></testcase></testsuite>' \
            > build/test-results/test/TEST-T.xml";
        let mut service = TaskService::default();
        service
            .start(TaskPlan::new("t", "/bin/sh", ["-c", script], &root).with_kind(TaskKind::Test))
            .unwrap();
        wait(&mut service);
        let counts = service.tests().counts();
        assert_eq!(
            (counts.passed, counts.failed),
            (1, 1),
            "{:?}",
            service.tests()
        );
        assert_eq!(service.tests().failed_tests()[0].name, "x()");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn command_line_quotes_arguments_with_spaces() {
        let plan = TaskPlan::new("x", "./gradlew", [":app:test", "--tests", "A B"], "/w");
        assert_eq!(plan.command_line(), "./gradlew :app:test --tests \"A B\"");
    }
}

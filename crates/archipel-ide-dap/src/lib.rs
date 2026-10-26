//! Debug Adapter Protocol client.
//!
//! - [`transport`]: framing and the adapter child process.
//! - [`session`]: the protocol state machine (no I/O).
//! - [`DebugClient`]: a session bound to a running adapter.
//! - [`Adapter`]: discovery of installed adapters (`lldb-dap`, `gdb -i dap`) and their
//!   launch arguments. Adapters are never bundled; missing ones produce actionable errors.

pub mod session;
pub mod transport;

pub use session::{
    BreakpointOptions, BreakpointStatus, LaunchRequest, OutputCategory, OutputLine, Scope, Session,
    SessionState, StackFrame, Step, ThreadInfo, Variable, Watch,
};

use archipel_ide_runner::find_program;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use thiserror::Error;
use transport::AdapterProcess;

#[derive(Debug, Error)]
pub enum DapError {
    #[error("could not start debug adapter `{program}`: {error}")]
    Spawn { program: String, error: String },
    #[error("no debug adapter found: {0}")]
    NoAdapter(String),
    #[error("debug adapter protocol error: {0}")]
    Protocol(String),
    #[error("debug adapter I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("debug adapter closed the connection")]
    Closed,
}

/// Native-code adapters Archipel IDE can drive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Adapter {
    /// LLVM's `lldb-dap` (formerly `lldb-vscode`).
    LldbDap { program: String },
    /// GDB 14+ built-in DAP interpreter (`gdb -i dap`).
    Gdb { program: String },
    /// microsoft/java-debug running inside jdtls; `port` is the DAP server that the
    /// `vscode.java.startDebugSession` workspace command opened on localhost.
    Java { port: u16 },
}

impl Adapter {
    /// Finds an installed native adapter: `lldb-dap` (versioned names too), then GDB.
    pub fn discover_native() -> Result<Self, DapError> {
        let lldb = ["lldb-dap", "lldb-vscode"]
            .into_iter()
            .map(str::to_owned)
            .chain((14..=22).rev().map(|version| format!("lldb-dap-{version}")))
            .find(|name| find_program(name).is_some());
        if let Some(program) = lldb {
            return Ok(Adapter::LldbDap { program });
        }
        if find_program("gdb").is_some() {
            return Ok(Adapter::Gdb {
                program: "gdb".into(),
            });
        }
        Err(DapError::NoAdapter(
            "install LLDB (lldb-dap) or GDB 14+ and add it to PATH to debug native programs".into(),
        ))
    }

    pub fn id(&self) -> &'static str {
        match self {
            Adapter::LldbDap { .. } => "lldb-dap",
            Adapter::Gdb { .. } => "gdb",
            Adapter::Java { .. } => "java-debug",
        }
    }

    fn command(&self) -> (String, Vec<String>) {
        match self {
            Adapter::Java { .. } => (String::new(), Vec::new()),
            Adapter::LldbDap { program } => (program.clone(), Vec::new()),
            Adapter::Gdb { program } => (program.clone(), vec!["-i".into(), "dap".into()]),
        }
    }

    /// `launch` arguments for a native program.
    pub fn launch_request(
        &self,
        program: &Path,
        args: &[String],
        cwd: &Path,
        env: &[(String, String)],
    ) -> LaunchRequest {
        let arguments = match self {
            Adapter::LldbDap { .. } => json!({
                "program": program,
                "args": args,
                "cwd": cwd,
                "env": env.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>(),
                "stopOnEntry": false,
            }),
            Adapter::Java { .. } => json!({"cwd": cwd, "args": args}),
            Adapter::Gdb { .. } => {
                let mut arguments = json!({
                    "program": program,
                    "args": args,
                    "cwd": cwd,
                    "stopAtBeginningOfMainSubprogram": false,
                });
                if !env.is_empty() {
                    arguments["env"] = Value::Object(
                        env.iter()
                            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                            .collect(),
                    );
                }
                arguments
            }
        };
        LaunchRequest {
            adapter_id: self.id().into(),
            attach: false,
            arguments,
        }
    }
}

/// `launch` arguments for a Java main class (java-debug's `launch` request).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaLaunch {
    pub main_class: String,
    pub project_name: Option<String>,
    pub class_paths: Vec<String>,
    pub module_paths: Vec<String>,
    pub args: Vec<String>,
    pub vm_args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
}

impl JavaLaunch {
    pub fn request(&self) -> LaunchRequest {
        let mut arguments = json!({
            "mainClass": self.main_class,
            "classPaths": self.class_paths,
            "modulePaths": self.module_paths,
            "args": self.args.join(" "),
            "vmArgs": self.vm_args.join(" "),
            "cwd": self.cwd,
            "env": self.env.iter().cloned().collect::<BTreeMap<_, _>>(),
            "console": "internalConsole",
            "stopOnEntry": false,
            "noDebug": false,
            "shortenCommandLine": "auto",
        });
        if let Some(project) = &self.project_name {
            arguments["projectName"] = json!(project);
        }
        LaunchRequest {
            adapter_id: "java-debug".into(),
            attach: false,
            arguments,
        }
    }
}

/// `attach` arguments for a JVM that listens for a debugger (e.g. `gradle test --debug-jvm`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaAttach {
    pub host: String,
    pub port: u16,
    pub project_name: Option<String>,
}

impl JavaAttach {
    pub fn request(&self) -> LaunchRequest {
        let mut arguments = json!({
            "hostName": self.host,
            "port": self.port,
            "timeout": 30_000,
        });
        if let Some(project) = &self.project_name {
            arguments["projectName"] = json!(project);
        }
        LaunchRequest {
            adapter_id: "java-debug".into(),
            attach: true,
            arguments,
        }
    }
}

/// A [`Session`] connected to an adapter process.
pub struct DebugClient {
    process: AdapterProcess,
    session: Session,
    pub adapter: Adapter,
}

impl DebugClient {
    pub fn start(
        adapter: Adapter,
        launch: LaunchRequest,
        breakpoints: BTreeMap<PathBuf, BTreeSet<usize>>,
        cwd: &Path,
    ) -> Result<Self, DapError> {
        let process = match &adapter {
            Adapter::Java { port } => AdapterProcess::connect(*port)?,
            _ => {
                let (program, args) = adapter.command();
                AdapterProcess::spawn(&program, &args, cwd)?
            }
        };
        let mut client = Self {
            process,
            session: Session::new(launch, breakpoints),
            adapter,
        };
        client.flush()?;
        Ok(client)
    }

    fn flush(&mut self) -> Result<(), DapError> {
        for message in self.session.take_outgoing() {
            self.process.send(&message)?;
        }
        Ok(())
    }

    /// Exchanges pending messages; call regularly (every UI frame).
    pub fn poll(&mut self) {
        for message in self.process.receive() {
            match message {
                Ok(message) => self.session.handle(&message),
                Err(error) => self.session.adapter_exited(&format!("{error}\n")),
            }
        }
        if let Err(error) = self.flush() {
            let stderr = self.process.stderr();
            self.session.adapter_exited(&format!("{error}\n{stderr}"));
        }
        if self.process.has_exited() {
            let stderr = self.process.stderr();
            self.session.adapter_exited(&stderr);
        }
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Applies an action to the session and sends the resulting requests.
    pub fn with_session(&mut self, action: impl FnOnce(&mut Session)) {
        action(&mut self.session);
        if let Err(error) = self.flush() {
            self.session.adapter_exited(&format!("{error}\n"));
        }
    }

    /// Ends the debuggee and the adapter.
    pub fn stop(&mut self) {
        self.with_session(Session::disconnect);
    }

    pub fn kill(&mut self) {
        self.process.kill();
        self.session.adapter_exited("");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_requests_per_adapter() {
        let env = [("RUST_BACKTRACE".to_owned(), "1".to_owned())];
        let lldb = Adapter::LldbDap {
            program: "lldb-dap".into(),
        }
        .launch_request(Path::new("/w/app"), &["x".into()], Path::new("/w"), &env);
        assert_eq!(lldb.arguments["env"], json!(["RUST_BACKTRACE=1"]));
        assert_eq!(lldb.adapter_id, "lldb-dap");
        let gdb = Adapter::Gdb {
            program: "gdb".into(),
        };
        assert_eq!(gdb.command().1, ["-i", "dap"]);
        let launch = gdb.launch_request(Path::new("/w/app"), &[], Path::new("/w"), &env);
        assert_eq!(launch.arguments["env"]["RUST_BACKTRACE"], "1");
    }

    #[test]
    fn java_attach_sends_host_and_port() {
        let request = JavaAttach {
            host: "localhost".into(),
            port: 5005,
            project_name: None,
        }
        .request();
        assert!(request.attach);
        assert_eq!(request.arguments["port"], 5005);
        assert_eq!(request.arguments["hostName"], "localhost");
        assert!(request.arguments.get("projectName").is_none());
    }

    #[test]
    fn java_launch_carries_class_and_paths() {
        let request = JavaLaunch {
            main_class: "com.example.App".into(),
            project_name: Some("app".into()),
            class_paths: vec!["/w/build/classes/java/main".into()],
            module_paths: Vec::new(),
            args: vec!["a".into(), "b".into()],
            vm_args: Vec::new(),
            cwd: "/w".into(),
            env: vec![("K".into(), "V".into())],
        }
        .request();
        assert_eq!(request.adapter_id, "java-debug");
        assert_eq!(request.arguments["mainClass"], "com.example.App");
        assert_eq!(request.arguments["projectName"], "app");
        assert_eq!(request.arguments["args"], "a b");
        assert_eq!(request.arguments["env"]["K"], "V");
    }
}

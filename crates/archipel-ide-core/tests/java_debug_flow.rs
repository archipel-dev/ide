//! Debugging a Java project through jdtls' java-debug plugin: `debug.start` on a Gradle
//! configuration drives the `vscode.java.*` commands (answered by `archipel-fake-lsp`),
//! connects to the announced DAP port (a scripted server) and reaches the breakpoint.

use archipel_ide_core::{IdeState, built_in_commands};
use archipel_ide_dap::transport::{read_message, write_message};
use archipel_ide_lsp::{
    LanguageServerProvider, LaunchConfig, LaunchContext, LspError, LspManager, ServerRegistry,
    ServerSpec,
};
use archipel_ide_runner::{RunConfiguration, RunTarget};
use serde_json::{Value, json};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Fake(PathBuf, ServerSpec, Vec<String>);

impl LanguageServerProvider for Fake {
    fn spec(&self) -> &ServerSpec {
        &self.1
    }
    fn launch(&self, _: &LaunchContext<'_>) -> Result<LaunchConfig, LspError> {
        Ok(LaunchConfig::new("fake", &self.0).args(self.2.clone()))
    }
}

fn fake_server() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let path = exe
        .parent()?
        .parent()?
        .join(format!("archipel-fake-lsp{}", std::env::consts::EXE_SUFFIX));
    path.is_file().then_some(path)
}

fn fake_dap(listener: TcpListener, source: PathBuf) {
    let (stream, _) = listener.accept().unwrap();
    let mut writer = stream.try_clone().unwrap();
    let mut reader = std::io::BufReader::new(stream);
    let mut seq = 100;
    while let Ok(Some(request)) = read_message(&mut reader) {
        let command = request["command"].as_str().unwrap_or_default().to_owned();
        let body = match command.as_str() {
            "initialize" => json!({"supportsConfigurationDoneRequest": true}),
            "threads" => json!({"threads": [{"id": 1, "name": "main"}]}),
            "stackTrace" => json!({"stackFrames": [{
                "id": 7, "name": "demo.App.main(String[])", "line": 1, "column": 1,
                "source": {"path": source}
            }]}),
            "scopes" => json!({"scopes": [{"name": "Local", "variablesReference": 3}]}),
            "variables" => json!({"variables": [
                {"name": "args", "value": "String[0]@21", "variablesReference": 0}]}),
            _ => json!({}),
        };
        let mut send = |mut message: Value| {
            seq += 1;
            message["seq"] = json!(seq);
            write_message(&mut writer, &message).unwrap();
        };
        send(
            json!({"type": "response", "request_seq": request["seq"], "success": true,
                    "command": command, "body": body}),
        );
        if command == "launch" {
            send(json!({"type": "event", "event": "initialized"}));
        }
        if command == "configurationDone" {
            send(json!({"type": "event", "event": "stopped", "body":
                {"reason": "breakpoint", "threadId": 1, "allThreadsStopped": true}}));
        }
    }
}

#[test]
fn debug_start_resolves_through_jdtls_and_stops_at_a_breakpoint() {
    let Some(server) = fake_server() else {
        eprintln!("archipel-fake-lsp not built; run `cargo test --workspace`");
        return;
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let root = std::env::temp_dir().join(format!("archipel-ide-java-debug-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("App.java"),
        "public class App { public static void main(String[] args) {} }\n",
    )
    .unwrap();
    let source = root.join("App.java");
    std::thread::spawn(move || fake_dap(listener, source));
    let mut registry = ServerRegistry::empty();
    registry.register(Arc::new(Fake(
        server,
        ServerSpec {
            id: "fake",
            language: "Java",
            extensions: &[("java", "java")],
            file_names: &[],
            candidates: &[],
            root_markers: &[],
            install_hint: "",
            notify_missing: false,
            initialization_options: None,
        },
        vec!["--java-debug".into(), port.to_string()],
    )));
    let commands = built_in_commands();
    let mut state = IdeState::default();
    state.lsp = LspManager::new(registry);
    commands
        .execute("project.open", &mut state, json!({"path": root}))
        .unwrap();
    commands
        .execute("editor.openFile", &mut state, json!({"path": "App.java"}))
        .unwrap();
    commands
        .execute(
            "debug.toggleBreakpoint",
            &mut state,
            json!({"path": "App.java", "line": 0}),
        )
        .unwrap();
    state
        .execution()
        .configurations_mut()
        .add_temporary(RunConfiguration::new(
            "App",
            RunTarget::Gradle {
                tasks: vec!["run".into()],
                options: Vec::new(),
                program_arguments: Vec::new(),
            },
        ));
    commands
        .execute("debug.start", &mut state, Value::Null)
        .unwrap();
    assert!(state.is_debugging());
    let deadline = Instant::now() + Duration::from_secs(20);
    while state.execution_point().is_none() {
        assert!(
            Instant::now() < deadline,
            "no stop: {} / {:?}",
            state.status,
            state.debugger().state().error
        );
        state.poll_lsp();
        state.poll_debug();
        std::thread::sleep(Duration::from_millis(10));
    }
    let point = state.execution_point().unwrap();
    assert_eq!(point.line, 0);
    if let Some(client) = state.debugger().state_mut().client.as_mut() {
        client.kill();
    }
    let _ = std::fs::remove_dir_all(root);
}

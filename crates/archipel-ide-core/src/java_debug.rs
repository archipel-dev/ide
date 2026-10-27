//! Java debugging through jdtls: the java-debug plugin (loaded as a jdtls bundle) exposes
//! `vscode.java.*` workspace commands. A session is started in four asynchronous steps:
//! build the workspace, resolve the main class, resolve its class path, then ask the plugin
//! to open a DAP server whose port `archipel-ide-dap` connects to.

use crate::IdeState;
use crate::debug_commands::PendingJava;
use archipel_ide_dap::{Adapter, DebugClient, JavaAttach, JavaLaunch, LaunchRequest};
use archipel_ide_lsp::{Feature, LspError, RequestHandle};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::Duration;

/// How long to wait for the language server to become ready.
const READY_TIMEOUT: Duration = Duration::from_secs(180);

pub(crate) enum JavaStage {
    /// Waiting for jdtls; the build request is sent as soon as it is ready.
    Build,
    Building(RequestHandle),
    Resolving(RequestHandle),
    Classpath {
        main_class: String,
        project: String,
        handle: RequestHandle,
    },
    /// A Gradle test JVM started with `--debug-jvm`: wait until it prints that it listens
    /// (probing the port would consume the single JDWP connection).
    AwaitListening,
    StartSession {
        launch: LaunchRequest,
        handle: RequestHandle,
    },
}

/// The port from the JVM banner `Listening for transport dt_socket at address: 5005`.
pub(crate) fn parse_listening_port(text: &str) -> Option<u16> {
    let (_, rest) = text.split_once("Listening for transport dt_socket at address:")?;
    rest.split_whitespace().next()?.parse().ok()
}

/// One entry of `vscode.java.resolveMainClass`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MainClass {
    pub main_class: String,
    pub project: String,
    pub file: Option<PathBuf>,
}

pub(crate) fn parse_main_classes(value: &Value) -> Vec<MainClass> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            Some(MainClass {
                main_class: entry["mainClass"].as_str()?.to_owned(),
                project: entry["projectName"].as_str().unwrap_or_default().to_owned(),
                file: entry["filePath"].as_str().map(PathBuf::from),
            })
        })
        .collect()
}

/// The class to debug: the requested one, else the one declared in `active`, else the first.
pub(crate) fn pick_main_class<'a>(
    classes: &'a [MainClass],
    requested: Option<&str>,
    active: Option<&std::path::Path>,
) -> Option<&'a MainClass> {
    requested
        .and_then(|name| classes.iter().find(|class| class.main_class == name))
        .or_else(|| {
            active.and_then(|path| classes.iter().find(|c| c.file.as_deref() == Some(path)))
        })
        .or_else(|| classes.first())
}

/// `[modulePaths, classPaths]` from `vscode.java.resolveClasspath`.
pub(crate) fn parse_classpath(value: &Value) -> (Vec<String>, Vec<String>) {
    let list = |index: usize| -> Vec<String> {
        value[index]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| item.as_str().map(str::to_owned))
            .collect()
    };
    (list(0), list(1))
}

/// The port from `vscode.java.startDebugSession` (a bare number).
pub(crate) fn parse_port(value: &Value) -> Option<u16> {
    value.as_u64().and_then(|port| u16::try_from(port).ok())
}

fn plugin_hint(error: &LspError) -> String {
    let text = error.to_string();
    if text.contains("delegateCommandHandler") || text.contains("vscode.java") {
        format!(
            "{text}. The Java debugger plugin (java-debug) is not loaded by the language server; set it up in Settings > Toolchains and restart the language server."
        )
    } else {
        text
    }
}

impl IdeState {
    fn java_command(
        &mut self,
        anchor: &std::path::Path,
        command: &str,
        arguments: Value,
    ) -> Result<RequestHandle, LspError> {
        self.lsp.request_with_params(
            anchor,
            Feature::ExecuteCommand,
            json!({"command": command, "arguments": arguments}),
        )
    }

    /// Advances an in-flight Java debug start; called every frame by `poll_debug`.
    pub(crate) fn poll_java_debug(&mut self) {
        let Some(mut pending) = self.debugger().state_mut().java.take() else {
            return;
        };
        match self.step_java_debug(&mut pending) {
            Ok(true) => {}
            Ok(false) => self.debugger().state_mut().java = Some(pending),
            Err(error) => {
                self.status = error.clone();
                self.debugger().state_mut().error = Some(error);
            }
        }
    }

    /// `Ok(true)` when finished (session started), `Ok(false)` to keep polling.
    fn step_java_debug(&mut self, pending: &mut PendingJava) -> Result<bool, String> {
        let anchor = pending.anchor.clone();
        match &pending.stage {
            JavaStage::Build if pending.attach => {
                if !self.execution().is_running() {
                    return Err(
                        "The Gradle test task ended before the test JVM was listening; see the Build window".into(),
                    );
                }
                pending.stage = JavaStage::AwaitListening;
            }
            JavaStage::AwaitListening => {
                let port = self
                    .execution()
                    .runner()
                    .output()
                    .iter()
                    .find_map(|chunk| parse_listening_port(&chunk.text));
                match port {
                    Some(port) => {
                        let launch = JavaAttach {
                            host: "localhost".into(),
                            port,
                            project_name: None,
                        }
                        .request();
                        match self.java_command(&anchor, "vscode.java.startDebugSession", json!([]))
                        {
                            Ok(handle) => {
                                pending.stage = JavaStage::StartSession { launch, handle }
                            }
                            Err(LspError::NotReady(_) | LspError::Starting { .. })
                                if pending.started.elapsed() < READY_TIMEOUT =>
                            {
                                self.status =
                                    "Waiting for the Java language server to debug…".into();
                            }
                            Err(error) => return Err(plugin_hint(&error)),
                        }
                    }
                    None if !self.execution().is_running() => {
                        return Err(
                            "The Gradle test task ended before the test JVM was listening".into(),
                        );
                    }
                    None => {}
                }
            }
            JavaStage::Build => {
                match self.java_command(&anchor, "vscode.java.buildWorkspace", json!([false])) {
                    Ok(handle) => pending.stage = JavaStage::Building(handle),
                    Err(LspError::NotReady(_) | LspError::Starting { .. })
                        if pending.started.elapsed() < READY_TIMEOUT =>
                    {
                        self.status = "Waiting for the Java language server to debug…".into();
                    }
                    Err(error) => return Err(plugin_hint(&error)),
                }
            }
            JavaStage::Building(handle) => {
                // A failed or partial build still lets the debugger try the existing classes.
                if self.lsp.take_response(handle).is_some() {
                    let root = self
                        .project
                        .as_ref()
                        .ok_or("no project is open")?
                        .root()
                        .to_owned();
                    let root = archipel_ide_lsp::client::file_uri(&root)
                        .map_err(|error| error.to_string())?
                        .to_string();
                    let handle = self
                        .java_command(&anchor, "vscode.java.resolveMainClass", json!([root]))
                        .map_err(|error| plugin_hint(&error))?;
                    pending.stage = JavaStage::Resolving(handle);
                }
            }
            JavaStage::Resolving(handle) => {
                if let Some(response) = self.lsp.take_response(handle) {
                    let classes = parse_main_classes(&response.map_err(|e| plugin_hint(&e))?);
                    let class =
                        pick_main_class(&classes, pending.main_class.as_deref(), Some(&anchor))
                            .ok_or("No Java main class was found in this project")?
                            .clone();
                    let handle = self
                        .java_command(
                            &anchor,
                            "vscode.java.resolveClasspath",
                            json!([class.main_class, class.project]),
                        )
                        .map_err(|error| plugin_hint(&error))?;
                    pending.stage = JavaStage::Classpath {
                        main_class: class.main_class,
                        project: class.project,
                        handle,
                    };
                }
            }
            JavaStage::Classpath {
                main_class,
                project,
                handle,
            } => {
                if let Some(response) = self.lsp.take_response(handle) {
                    let (module_paths, class_paths) =
                        parse_classpath(&response.map_err(|e| plugin_hint(&e))?);
                    let launch = JavaLaunch {
                        main_class: main_class.clone(),
                        project_name: Some(project.clone()).filter(|p| !p.is_empty()),
                        class_paths,
                        module_paths,
                        args: pending.args.clone(),
                        vm_args: Vec::new(),
                        cwd: pending.cwd.clone(),
                        env: pending.env.clone(),
                    };
                    let handle = self
                        .java_command(&anchor, "vscode.java.startDebugSession", json!([]))
                        .map_err(|error| plugin_hint(&error))?;
                    pending.stage = JavaStage::StartSession {
                        launch: launch.request(),
                        handle,
                    };
                }
            }
            JavaStage::StartSession { launch, handle } => {
                if let Some(response) = self.lsp.take_response(handle) {
                    let response = response.map_err(|e| plugin_hint(&e))?;
                    let port = parse_port(&response)
                        .ok_or_else(|| format!("java-debug returned no debug port ({response})"))?;
                    let breakpoints = self.absolute_breakpoints()?;
                    let mut client = DebugClient::start(
                        Adapter::Java { port },
                        launch.clone(),
                        breakpoints,
                        &pending.cwd,
                    )
                    .map_err(|error| error.to_string())?;
                    self.seed_session(&mut client);
                    self.debugger().attach_session(&pending.name, client);
                    self.tool_windows.show(crate::ToolWindow::Debug);
                    self.status = format!("Debugging {}", pending.name);
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_listening_port_from_the_jvm_banner() {
        assert_eq!(
            parse_listening_port("Listening for transport dt_socket at address: 5005\n"),
            Some(5005)
        );
        assert_eq!(parse_listening_port("> Task :test"), None);
    }

    #[test]
    fn parses_plugin_results() {
        let classes = parse_main_classes(&json!([
            {"mainClass": "demo.Other", "projectName": "app", "filePath": "/w/Other.java"},
            {"mainClass": "demo.App", "projectName": "app", "filePath": "/w/App.java"},
            {"projectName": "bad"}
        ]));
        assert_eq!(classes.len(), 2);
        let active = std::path::Path::new("/w/App.java");
        assert_eq!(
            pick_main_class(&classes, None, Some(active))
                .unwrap()
                .main_class,
            "demo.App"
        );
        assert_eq!(
            pick_main_class(&classes, Some("demo.Other"), Some(active))
                .unwrap()
                .main_class,
            "demo.Other"
        );
        assert_eq!(
            pick_main_class(&classes, None, None).unwrap().main_class,
            "demo.Other"
        );
        assert!(pick_main_class(&[], None, None).is_none());
        let (modules, classpath) = parse_classpath(&json!([[], ["/w/build/classes", "/lib.jar"]]));
        assert!(modules.is_empty());
        assert_eq!(classpath, ["/w/build/classes", "/lib.jar"]);
        assert_eq!(parse_port(&json!(5005)), Some(5005));
        assert_eq!(parse_port(&json!("x")), None);
    }

    #[test]
    fn missing_plugin_error_names_the_fix() {
        let error = LspError::Server {
            code: -32001,
            message: "No delegateCommandHandler for vscode.java.resolveMainClass".into(),
        };
        assert!(plugin_hint(&error).contains("java-debug"));
    }
}

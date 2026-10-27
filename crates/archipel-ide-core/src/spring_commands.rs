//! Spring Boot support of the workbench: the starter catalog of the New Project wizard
//! (built in, optionally refreshed from start.spring.io), project creation parameters,
//! and the commands around them.

use crate::{CommandError, IdeState, string_array_param, string_param};
use archipel_ide_command::{Command, CommandRegistry};
use archipel_ide_project::generator::{Language, ProjectKind};
use archipel_ide_project::spring::{SpringBuild, SpringSpec};
use archipel_ide_project::spring_catalog::{self, Catalog, Remote};
use serde_json::{Value, json};
use std::sync::mpsc;

/// Catalog and refresh state; the wizard reads and the commands change it.
#[derive(Default)]
pub struct SpringState {
    /// Built in until a refresh succeeds.
    pub catalog: Catalog,
    refresh: Option<mpsc::Receiver<Result<Remote, String>>>,
    /// Outcome of the last refresh, for the wizard's status line.
    pub refresh_status: Option<RefreshStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshStatus {
    Running,
    Done { added: usize },
    Failed(String),
}

impl IdeState {
    /// Starts fetching Boot versions and starters from `url` on a worker thread.
    pub fn refresh_spring_catalog(&mut self, url: &str) -> Result<(), String> {
        if self.spring().state().refresh.is_some() {
            return Err("a refresh is already running".into());
        }
        let (sender, receiver) = mpsc::channel();
        let url = url.to_owned();
        std::thread::Builder::new()
            .name("spring-catalog-refresh".into())
            .spawn(move || {
                let _ = sender.send(spring_catalog::fetch_remote(&url));
            })
            .map_err(|error| format!("cannot start the refresh: {error}"))?;
        let spring = self.spring();
        let mut state = spring.state();
        state.refresh = Some(receiver);
        state.refresh_status = Some(RefreshStatus::Running);
        Ok(())
    }

    /// Installs a finished refresh. Returns whether the catalog or status changed.
    pub fn poll_spring_catalog(&mut self) -> bool {
        let spring = self.spring();
        let mut state = spring.state();
        let Some(receiver) = &state.refresh else {
            return false;
        };
        let outcome = match receiver.try_recv() {
            Ok(outcome) => outcome,
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => Err("the refresh was interrupted".into()),
        };
        state.refresh = None;
        state.refresh_status = Some(match outcome {
            Ok(remote) => RefreshStatus::Done {
                added: state.catalog.merge_remote(&remote),
            },
            Err(error) => RefreshStatus::Failed(error),
        });
        true
    }
}

fn text(params: &Value, key: &str, default: &str) -> String {
    params
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .trim()
        .to_owned()
}

/// Project kind for `project.create` with `kind = "spring-boot"`.
pub(crate) fn project_kind(state: &IdeState, params: &Value) -> Result<ProjectKind, CommandError> {
    let invalid = |message: String| CommandError::InvalidParams(message);
    let spring = state.spring();
    let spring = spring.state();
    let catalog = &spring.catalog;
    let language = match text(params, "language", "java").as_str() {
        "java" => Language::Java,
        "kotlin" => Language::Kotlin,
        "groovy" => Language::Groovy,
        other => return Err(invalid(format!("unknown language `{other}`"))),
    };
    let build_id = text(params, "build", "gradle-kotlin");
    let build = SpringBuild::parse(&build_id)
        .ok_or_else(|| invalid(format!("unknown build system `{build_id}`")))?;
    let group = text(params, "group", "com.example");
    let artifact = match text(params, "artifact", "").as_str() {
        "" => string_param(params, "name")?,
        artifact => artifact.to_owned(),
    };
    let package = match text(params, "package", "").as_str() {
        "" => SpringSpec::derive_package(&group, &artifact),
        package => package.to_owned(),
    };
    let java_version = params
        .get("javaVersion")
        .and_then(Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .unwrap_or(21);
    let boot_version = match text(params, "bootVersion", "").as_str() {
        "" => catalog.boot_versions.first().cloned().unwrap_or_default(),
        version => version.to_owned(),
    };
    let war = match text(params, "packaging", "jar").as_str() {
        "jar" => false,
        "war" => true,
        other => return Err(invalid(format!("unknown packaging `{other}`"))),
    };
    let dependencies = if params.get("dependencies").is_some() {
        string_array_param(params, "dependencies")?
    } else {
        Vec::new()
    };
    let spec = SpringSpec {
        language,
        build,
        group,
        artifact,
        package,
        description: text(params, "description", "Demo project for Spring Boot"),
        java_version,
        boot_version,
        war,
        dependencies,
    };
    spec.validate(catalog)
        .map_err(CommandError::InvalidParams)?;
    Ok(ProjectKind::Spring {
        spec: Box::new(spec),
        catalog: Box::new(catalog.clone()),
    })
}

pub(crate) fn register(registry: &mut CommandRegistry<IdeState>) {
    registry
        .register(Command::new(
            crate::spec(
                "spring.catalog.refresh",
                "Refresh Spring Boot Catalog from start.spring.io",
                "Project",
                None,
                r#"{"url":"string?"}"#,
            ),
            |state: &IdeState| state.spring().state().refresh.is_none(),
            |state: &mut IdeState, params| {
                let url = text(&params, "url", spring_catalog::INITIALIZR_URL);
                state
                    .refresh_spring_catalog(&url)
                    .map_err(CommandError::Failed)?;
                Ok(json!({"url": url}))
            },
        ))
        .expect("unique built-in id");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_in_commands;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    const METADATA: &str = r#"{"bootVersion":{"values":[{"id":"4.2.0","name":"4.2.0"},{"id":"4.1.2","name":"4.1.2"},{"id":"4.3.0-SNAPSHOT","name":"s"}]},
        "dependencies":{"values":[{"name":"Web","values":[{"id":"fresh-starter","name":"Fresh Starter","description":"A starter added by the service."}]}]}}"#;
    const COORDINATES: &str = r#"{"dependencies":{"fresh-starter":{"groupId":"com.example","artifactId":"fresh-starter","version":"1.0.0"}}}"#;

    /// A one-shot Initializr stand-in on localhost.
    fn serve() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(2) {
                let mut stream = stream.unwrap();
                let mut buffer = [0u8; 2048];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                assert!(
                    request
                        .to_lowercase()
                        .contains("accept: application/vnd.initializr.v2.2+json"),
                    "{request}"
                );
                let body = if request.starts_with("GET /dependencies") {
                    COORDINATES
                } else {
                    METADATA
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        format!("http://{address}")
    }

    fn wait(state: &mut IdeState) {
        for _ in 0..200 {
            if state.poll_spring_catalog() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        panic!("the refresh did not finish");
    }

    #[test]
    fn refresh_merges_versions_and_new_starters_from_the_service() {
        let commands = built_in_commands();
        let mut state = IdeState::default();
        let before = state.spring().state().catalog.dependencies.len();
        commands
            .execute(
                "spring.catalog.refresh",
                &mut state,
                json!({"url": serve()}),
            )
            .unwrap();
        assert!(!commands.is_enabled("spring.catalog.refresh", &state));
        wait(&mut state);
        assert_eq!(
            state.spring().state().refresh_status,
            Some(RefreshStatus::Done { added: 1 })
        );
        assert_eq!(
            state.spring().state().catalog.dependencies.len(),
            before + 1
        );
        assert_eq!(
            state.spring().state().catalog.boot_versions,
            ["4.2.0", "4.1.2"]
        );
        assert!(commands.is_enabled("spring.catalog.refresh", &state));
    }

    #[test]
    fn an_unreachable_service_keeps_the_built_in_catalog() {
        let commands = built_in_commands();
        let mut state = IdeState::default();
        let before = state.spring().state().catalog.clone();
        commands
            .execute(
                "spring.catalog.refresh",
                &mut state,
                json!({"url": "http://127.0.0.1:1"}),
            )
            .unwrap();
        wait(&mut state);
        assert!(matches!(
            state.spring().state().refresh_status,
            Some(RefreshStatus::Failed(_))
        ));
        assert_eq!(state.spring().state().catalog, before);
    }

    #[test]
    fn project_create_generates_a_spring_project_for_every_build_system() {
        let parent =
            std::env::temp_dir().join(format!("archipel-core-spring-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&parent);
        let commands = built_in_commands();
        for (build, file) in [
            ("gradle-groovy", "build.gradle"),
            ("gradle-kotlin", "build.gradle.kts"),
            ("maven", "pom.xml"),
            ("bazel", "MODULE.bazel"),
        ] {
            let mut state = IdeState::default();
            let name = format!("app-{build}");
            commands
                .execute(
                    "project.create",
                    &mut state,
                    json!({"parent": parent, "name": name, "kind": "spring-boot", "build": build,
                           "language": "java", "group": "com.example", "dependencies": ["web", "h2"]}),
                )
                .unwrap();
            let root = state.project.as_ref().unwrap().root().to_owned();
            assert!(root.join(file).is_file(), "{build}");
            assert!(
                state
                    .files
                    .iter()
                    .any(|entry| entry.relative_path.ends_with(file)),
                "the project opens with its files"
            );
            let detected = state.detected_build.as_ref().map(|build| build.kind);
            assert!(detected.is_some(), "{build}: build tool auto-detected");
        }
        for bad in [
            json!({"build": "ant"}),
            json!({"language": "cobol"}),
            json!({"dependencies": ["nope"]}),
            json!({"packaging": "ear"}),
            json!({"build": "bazel", "language": "kotlin"}),
            json!({"group": "com.class"}),
        ] {
            let mut params = json!({"parent": parent, "name": "bad", "kind": "spring-boot"});
            for (key, value) in bad.as_object().unwrap() {
                params[key] = value.clone();
            }
            let mut state = IdeState::default();
            assert!(
                commands
                    .execute("project.create", &mut state, params)
                    .is_err()
            );
            assert!(!parent.join("bad").exists());
        }
        let _ = std::fs::remove_dir_all(&parent);
    }
}

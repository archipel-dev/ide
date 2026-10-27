//! End-to-end: a Java Gradle project on a machine without a JDK or jdtls.
//! `toolchain.setup` installs both, then jdtls (started from the managed
//! install through the LSP resolver hook) answers completion.
//!
//! Run with `cargo test -p archipel-ide-core --test toolchain_e2e -- --ignored --nocapture`.
//! Set `ARCHIPEL_E2E_TOOLCHAINS` to reuse an install root between runs.

use archipel_ide_core::{IdeState, ToolchainState, built_in_commands};
use archipel_ide_lsp::CompletionTrigger;
use archipel_ide_toolchains::{ToolchainStore, Toolchains, providers::builtin_catalog};
use serde_json::json;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[test]
#[ignore = "downloads a JDK and jdtls (~380 MB) and runs jdtls"]
fn java_project_without_jdk_sets_up_jdk_and_jdtls_and_completes() {
    let scratch =
        std::env::temp_dir().join(format!("archipel-ide-e2e-java-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let root = archipel_ide_compat::env_var_os("ARCHIPEL_E2E_TOOLCHAINS")
        .map(Into::into)
        .unwrap_or_else(|| scratch.join("toolchains"));
    let project = scratch.join("demo");
    std::fs::create_dir_all(project.join("src/main/java/demo")).unwrap();
    std::fs::write(
        project.join("settings.gradle"),
        "rootProject.name = 'demo'\n",
    )
    .unwrap();
    std::fs::write(project.join("build.gradle"), "plugins { id 'java' }\n").unwrap();
    let source = "package demo;\n\npublic class App {\n    public String greeting(String name) { return \"Hello, \" + name; }\n\n    public static void main(String[] args) {\n        App app = new App();\n        app.\n    }\n}\n";
    std::fs::write(project.join("src/main/java/demo/App.java"), source).unwrap();

    // A "clean machine": no PATH, no JAVA_HOME; only managed installs count.
    let toolchains = Toolchains::new(builtin_catalog(), ToolchainStore::new(&root))
        .with_environment(Default::default(), BTreeMap::new());
    let mut state = IdeState::default();
    state.toolchains().replace(ToolchainState::new(
        toolchains,
        Some(scratch.join("user/toolchains.toml")),
    ));
    let commands = built_in_commands();
    commands
        .execute("project.open", &mut state, json!({"path": project}))
        .unwrap();

    let started = Instant::now();
    let setup = commands
        .execute(
            "toolchain.setup",
            &mut state,
            json!({"components": ["jdtls"], "wait": true}),
        )
        .unwrap();
    println!("setup in {:.1?}: {setup}", started.elapsed());
    let missing = commands
        .execute(
            "toolchain.missing",
            &mut state,
            json!({"components": ["jdtls"]}),
        )
        .unwrap();
    assert_eq!(missing, json!([]));
    let jdk = commands
        .execute("toolchain.locate", &mut state, json!({"component": "jdk"}))
        .unwrap();
    assert_eq!(jdk["source"]["kind"], "managed", "{jdk}");

    commands
        .execute(
            "editor.openFile",
            &mut state,
            json!({"path": "src/main/java/demo/App.java"}),
        )
        .unwrap();
    let file = project
        .canonicalize()
        .unwrap()
        .join("src/main/java/demo/App.java");
    let started = Instant::now();
    let deadline = started + Duration::from_secs(240);
    let mut items = Vec::new();
    while items.is_empty() && Instant::now() < deadline {
        state.poll_lsp();
        if state.lsp.ready_server_for(&file).is_ok() {
            let _ = state
                .lsp
                .request_completion(&file, 7, 12, CompletionTrigger::Invoked);
            while state.lsp.completion_pending() && Instant::now() < deadline {
                state.poll_lsp();
                std::thread::sleep(Duration::from_millis(50));
            }
            if let Some(result) = state.lsp.take_completion() {
                items = result.items;
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    println!(
        "completion after {:.1?}: {:?}",
        started.elapsed(),
        &labels[..labels.len().min(8)]
    );
    let servers = state.lsp.servers();
    println!(
        "servers: {:?}",
        servers
            .iter()
            .map(|s| (s.id.as_str(), format!("{:?}", s.status)))
            .collect::<Vec<_>>()
    );
    assert!(
        labels.iter().any(|label| label.starts_with("greeting")),
        "{labels:?}"
    );
    state.lsp.shutdown();
    let _ = std::fs::remove_dir_all(&scratch);
}

//! Integration checks against installed language servers. Ignored by default;
//! run with `cargo test -p archipel-ide-lsp --test real_servers -- --ignored`.

use archipel_ide_lsp::{CompletionTrigger, LspManager, lsp_types::CompletionItem};
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

/// Requests completion until the server returns items (servers answer with
/// an empty list while they are still loading the workspace).
fn complete(
    manager: &mut LspManager,
    file: &Path,
    line: usize,
    character: usize,
    deadline: Instant,
) -> Vec<CompletionItem> {
    while Instant::now() < deadline {
        manager.poll();
        if manager.ready_server_for(file).is_ok() {
            manager
                .request_completion(file, line, character, CompletionTrigger::Invoked)
                .unwrap();
            while Instant::now() < deadline {
                manager.poll();
                if let Some(result) = manager.take_completion() {
                    if !result.items.is_empty() {
                        return result.items;
                    }
                    break;
                }
                thread::sleep(Duration::from_millis(25));
            }
        }
        thread::sleep(Duration::from_millis(200));
    }
    Vec::new()
}

fn installed(program: &str) -> bool {
    archipel_ide_lsp::registry::resolve_program(program).is_some()
}

#[test]
#[ignore = "requires an installed rust-analyzer"]
fn rust_analyzer_publishes_diagnostics_and_completes() {
    if !installed("rust-analyzer") {
        return;
    }
    let root = std::env::temp_dir().join(format!("archipel-ide-lsp-real-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='archipel-ide-lsp-smoke'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    let source =
        "pub fn broken() { let value: u32 = \"not a number\"; let s = String::new(); s.\n}\n";
    let file = root.join("src/lib.rs");
    std::fs::write(&file, source).unwrap();
    let file = file.canonicalize().unwrap();
    let mut manager = LspManager::default();
    manager.set_project(Some(&root));
    manager.sync_document(&file, source, 0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    while manager.diagnostics_for(&file).is_empty() && Instant::now() < deadline {
        manager.poll();
        thread::sleep(Duration::from_millis(25));
    }
    assert!(!manager.diagnostics_for(&file).is_empty());
    let items = complete(&mut manager, &file, 0, 76, deadline);
    let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.iter().any(|label| label.starts_with("push_str")),
        "{labels:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires an installed jdtls and JDK 21+"]
fn jdtls_completes_java_members() {
    if !installed("jdtls") {
        return;
    }
    let root = std::env::temp_dir().join(format!("archipel-ide-jdtls-real-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    let source = "public class App {\n    public String greeting(String name) { return \"Hello, \" + name; }\n    public static void main(String[] args) {\n        App app = new App();\n        app.\n    }\n}\n";
    let file = root.join("src/App.java");
    std::fs::write(&file, source).unwrap();
    let file = file.canonicalize().unwrap();
    let mut manager = LspManager::default();
    manager.set_project(Some(&root));
    manager.sync_document(&file, source, 0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(180);
    let items = complete(&mut manager, &file, 4, 12, deadline);
    let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.iter().any(|label| label.starts_with("greeting")),
        "{labels:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires kotlin-lsp on PATH and /home/user/samples/kotlin-app"]
fn kotlin_lsp_completes_kotlin_members() {
    let root = Path::new("/home/user/samples/kotlin-app");
    if !installed("kotlin-lsp") || !root.is_dir() {
        return;
    }
    let file = root.join("src/main/kotlin/Main.kt").canonicalize().unwrap();
    let source = std::fs::read_to_string(&file)
        .unwrap()
        .replace("    \n}", "    greeter.\n}");
    let mut manager = LspManager::default();
    manager.set_project(Some(root));
    manager.sync_document(&file, &source, 0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(300);
    let items = complete(&mut manager, &file, 6, 12, deadline);
    let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.iter().any(|label| label.starts_with("greeting")),
        "{labels:?}"
    );
}

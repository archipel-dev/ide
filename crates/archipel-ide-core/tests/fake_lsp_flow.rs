//! Completion → resolve → auto-import and quick documentation through the
//! command registry, against the scriptable `archipel-fake-lsp` server.
//!
//! The server binary is built by `cargo test --workspace`; when this crate's
//! tests run alone and it is missing, the test is skipped with a note.

use archipel_ide_core::{IdeState, built_in_commands};
use archipel_ide_lsp::{
    LanguageServerProvider, LaunchConfig, LaunchContext, LspError, LspManager, ServerRegistry,
    ServerSpec,
};
use serde_json::json;
use std::{
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

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
    let dir = exe.parent()?.parent()?;
    let path = dir.join(format!("archipel-fake-lsp{}", std::env::consts::EXE_SUFFIX));
    path.is_file().then_some(path)
}

fn wait(state: &mut IdeState, mut done: impl FnMut(&IdeState) -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        state.poll_lsp();
        if done(state) {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

/// Opens a project with `App.fake` ("app.") served by the fake server and
/// waits until the server is ready, caret after the dot.
fn open_fake_project(server: PathBuf, args: &[&str], name: &str) -> (PathBuf, IdeState) {
    let root = std::env::temp_dir().join(format!(
        "archipel-ide-core-fake-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("App.fake"), "app.").unwrap();
    let mut registry = ServerRegistry::empty();
    registry.register(Arc::new(Fake(
        server,
        ServerSpec {
            id: "fake",
            language: "Fake",
            extensions: &[("fake", "fake")],
            file_names: &[],
            candidates: &[],
            root_markers: &[],
            install_hint: "",
            notify_missing: false,
            initialization_options: None,
        },
        args.iter().map(|arg| arg.to_string()).collect(),
    )));
    let commands = built_in_commands();
    let mut state = IdeState::default();
    state.lsp = LspManager::new(registry);
    commands
        .execute("project.open", &mut state, json!({"path": root}))
        .unwrap();
    commands
        .execute("editor.openFile", &mut state, json!({"path": "App.fake"}))
        .unwrap();
    state.documents[0]
        .selections
        .set_single(archipel_ide_text::Selection::caret(4));
    assert!(wait(&mut state, |state| {
        state
            .active_absolute_path()
            .is_some_and(|path| !state.lsp.trigger_characters(&path).is_empty())
    }));
    (root, state)
}

fn type_text(state: &mut IdeState, text: &str) {
    let caret = state.documents[0].selections.primary().head;
    state.documents[0].text.edit(caret, caret, text).unwrap();
    state.documents[0]
        .selections
        .set_single(archipel_ide_text::Selection::caret(
            caret + text.chars().count(),
        ));
    state.sync_active_lsp();
    state.completion_after_typing(text);
}

#[test]
fn incomplete_lists_are_requeried_while_typing() {
    let Some(server) = fake_server() else {
        eprintln!("archipel-fake-lsp not built; run `cargo test --workspace`");
        return;
    };
    let (root, mut state) = open_fake_project(server, &["--incomplete"], "incomplete");
    // Typing an identifier start opens the popup without Ctrl+Space.
    type_text(&mut state, "g");
    assert!(wait(&mut state, |state| {
        state
            .completion
            .as_ref()
            .is_some_and(|s| s.visible() && s.incomplete)
    }));
    let labels = |state: &IdeState| -> Vec<String> {
        let session = state.completion.as_ref().unwrap();
        session
            .ranked
            .iter()
            .map(|row| session.items[row.index].label.clone())
            .collect()
    };
    assert!(labels(&state).contains(&"getRequiredField".to_string()));
    // The next character re-queries with TriggerForIncompleteCompletions (3).
    type_text(&mut state, "r");
    assert!(wait(&mut state, |state| {
        state
            .completion
            .as_ref()
            .is_some_and(|s| !s.waiting && s.items.iter().any(|i| i.label == "trigger3"))
    }));
    // `gr`: the prefix match first, then the camel-hump match g+R.
    assert_eq!(labels(&state), ["greeting", "getRequiredField"]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn commit_characters_accept_the_selected_item_and_stay_typed() {
    let Some(server) = fake_server() else {
        eprintln!("archipel-fake-lsp not built; run `cargo test --workspace`");
        return;
    };
    let (root, mut state) = open_fake_project(server, &[], "commit");
    type_text(&mut state, "g");
    assert!(wait(&mut state, |state| {
        state.completion.as_ref().is_some_and(|s| {
            s.visible() && s.selected_item().is_some_and(|i| i.label == "greeting")
        })
    }));
    type_text(&mut state, ";");
    let text = state.documents[0].text.text();
    assert!(text.starts_with("app.greeting"), "{text}");
    assert!(text.ends_with(';'), "{text}");
    assert!(state.completion.is_none());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn completion_resolves_docs_applies_imports_and_shows_quick_doc() {
    let Some(server) = fake_server() else {
        eprintln!("archipel-fake-lsp not built; run `cargo test --workspace`");
        return;
    };
    let (root, mut state) = open_fake_project(server, &[], "flow");
    let commands = built_in_commands();

    // Ctrl+Space, then the docs panel (Ctrl+Q while the popup is open).
    commands
        .execute("language.complete", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| {
        state.completion.as_ref().is_some_and(|s| s.visible())
    }));
    let toggled = commands
        .execute("language.quickDoc", &mut state, json!({}))
        .unwrap();
    assert_eq!(toggled["panel"], true);
    assert!(wait(&mut state, |state| {
        state
            .completion_documentation()
            .is_some_and(|doc| doc.contains("**greeting**"))
    }));

    // Accepting an unresolved row resolves it and applies its import edit.
    commands
        .execute("completion.move", &mut state, json!({"delta": 1}))
        .unwrap();
    commands
        .execute("completion.accept", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| {
        state.documents[0]
            .text
            .text()
            .starts_with("import fake.Greeting;\n")
    }));
    let document = &state.documents[0];
    assert_eq!(
        document.text.text(),
        "import fake.Greeting;\napp.getRequiredField"
    );
    assert_eq!(
        document.selections.primary().head,
        document.text.len_chars()
    );

    // Ctrl+Q without the popup shows hover documentation at the caret.
    commands
        .execute("language.quickDoc", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| state.docs.popup.is_some()));
    let popup = state.docs.popup.clone().unwrap();
    assert!(popup.markdown.contains("Greets *name*"));
    // Moving the caret closes it.
    state.documents[0]
        .selections
        .set_single(archipel_ide_text::Selection::caret(0));
    state.poll_lsp();
    assert!(state.docs.popup.is_none());

    // Typing `(` (a server trigger character) opens parameter info with the
    // active parameter located in the label; closing and Ctrl+P work too.
    let end = state.documents[0].text.len_chars();
    state.documents[0].text.edit(end, end, "(").unwrap();
    state.documents[0]
        .selections
        .set_single(archipel_ide_text::Selection::caret(end + 1));
    state.signature_after_typing("(");
    assert!(wait(&mut state, |state| state.signature.popup.is_some()));
    let info = state.signature.popup.clone().unwrap();
    assert_eq!(info.signatures[0].label, "greeting(String name, int times)");
    assert_eq!(info.signatures[0].active, Some((22, 31)));
    commands
        .execute("language.closeParameterInfo", &mut state, json!({}))
        .unwrap();
    assert!(state.signature.popup.is_none());
    commands
        .execute("language.parameterInfo", &mut state, json!({}))
        .unwrap();
    assert!(wait(&mut state, |state| state.signature.popup.is_some()));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn completion_during_import_says_indexing_and_requeries_when_ready() {
    let Some(server) = fake_server() else {
        eprintln!("archipel-fake-lsp not built; run `cargo test --workspace`");
        return;
    };
    let (root, mut state) = open_fake_project(server, &["--importing", "1"], "importing");
    let commands = built_in_commands();
    commands
        .execute("language.complete", &mut state, json!({}))
        .unwrap();
    // The empty answer during "Building 0%" keeps the popup with a notice.
    assert!(wait(&mut state, |state| {
        state
            .completion
            .as_ref()
            .is_some_and(|s| !s.waiting && s.indexing_only() && s.requery_when_ready)
    }));
    let notice = state.completion.as_ref().unwrap().indexing.clone().unwrap();
    assert!(notice.contains("Building"), "{notice}");
    assert_ne!(state.status, "No suggestions");
    // Once the import ends, the popup asks again and shows the real items.
    assert!(wait(&mut state, |state| {
        state.completion.as_ref().is_some_and(|s| {
            s.visible() && s.indexing.is_none() && s.items.iter().any(|i| i.label == "greeting")
        })
    }));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn code_vision_shows_usages_authors_and_parameter_hints() {
    let Some(server) = fake_server() else {
        eprintln!("archipel-fake-lsp is not built; skipping");
        return;
    };
    let root =
        std::env::temp_dir().join(format!("archipel-ide-core-vision-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let source = "class App {\n    String greeting(String name) {\n        return name;\n    }\n    void main() {\n        greeting(\"x\");\n    }\n}\n";
    std::fs::write(root.join("App.java"), source).unwrap();
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
        Vec::new(),
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
    assert!(wait(&mut state, |state| {
        let (lenses, inlays) = state.code_vision(0);
        lenses.iter().any(|lens| lens.usages.is_some()) && !inlays.is_empty()
    }));
    let (lenses, inlays) = state.code_vision(0);
    let greeting = lenses
        .iter()
        .find(|lens| lens.line == 1)
        .expect("method lens");
    // The fake server lists the declaration among the references.
    assert_eq!(greeting.usages, Some(2));
    assert_eq!(greeting.column, 11);
    assert_eq!(inlays.len(), 1);
    assert_eq!((inlays[0].line, inlays[0].column), (5, 17));
    assert_eq!(inlays[0].text, "name:");

    commands
        .execute("view.codeVision", &mut state, json!({}))
        .unwrap();
    assert!(state.code_vision(0).0.is_empty() && state.code_vision(0).1.is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

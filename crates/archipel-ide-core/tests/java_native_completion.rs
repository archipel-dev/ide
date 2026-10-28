//! Native Java class-name completion through the whole editor, with no language server
//! registered (plan M3 slice): the popup opens by itself after `@` and after `new `
//! (CI-CMP-030), lists classes from the Java indexes in the resolve scope of the file
//! (JAVA-CMP-018, JAVA-RES-033) and accepting a row writes the name and the import as one
//! undo step (JAVA-CMP-019, 020). Libraries are our own hermetic fixture jars
//! (`crates/archipel-ide-java/tests/fixtures`, built by `tools/java-fixtures/build.sh`).

use archipel_ide_core::{IdeState, built_in_commands};
use archipel_ide_lsp::{LspManager, ServerRegistry};
use archipel_ide_projectmodel::{
    DependencyScope, JpsModuleSourceRootType, Library, Module, OrderEntry, ProjectModel, Sdk,
    SourceFolder,
};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Project {
    root: PathBuf,
    state: IdeState,
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn fixture_jar(name: &str) -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest
        .join("../archipel-ide-java/tests/fixtures")
        .join(name);
    let out = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("java-fixtures")
        .join(format!("core-{name}.jar"));
    if !out.exists() {
        let status = std::process::Command::new("bash")
            .arg(manifest.join("../../tools/java-fixtures/build.sh"))
            .arg(&src)
            .arg(&out)
            .status()
            .expect("bash");
        assert!(
            status.success(),
            "building the {name} fixture jar needs a JDK"
        );
    }
    out
}

fn pump(state: &mut IdeState) {
    state.poll_project_model();
    state.poll_lsp();
    state.poll_analysis();
}

fn wait(
    state: &mut IdeState,
    seconds: u64,
    what: &str,
    mut done: impl FnMut(&mut IdeState) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        pump(state);
        if done(state) {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for {what}; status {}", state.status);
}

fn java_ready(state: &IdeState) -> bool {
    state.language_status().iter().any(|(id, status)| {
        *id == "java" && status.as_deref().is_some_and(|s| s.starts_with("Ready"))
    })
}

/// Opens a project made of `files` without a language server; `libraries` are fixture jars put
/// on the test classpath of module `app` (an imported model replaces the ad-hoc module).
fn open(name: &str, files: &[(&str, &str)], libraries: &[&str], file: &str) -> Project {
    let root = std::env::temp_dir().join(format!(
        "archipel-java-complete-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for (path, text) in files {
        let target = root.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, text).unwrap();
    }
    let root = std::fs::canonicalize(&root).unwrap();
    let commands = built_in_commands();
    let mut state = IdeState::default();
    state.lsp = LspManager::new(ServerRegistry::empty());
    commands
        .execute("project.open", &mut state, json!({"path": root}))
        .unwrap();
    if !libraries.is_empty() {
        let mut model = ProjectModel::new("demo", &root);
        let mut app = Module::new("app");
        app.content_entry_mut(&root)
            .add_source_folder(SourceFolder::new(
                root.join("src/main/java"),
                JpsModuleSourceRootType::Source,
            ))
            .add_source_folder(SourceFolder::new(
                root.join("src/test/java"),
                JpsModuleSourceRootType::TestSource,
            ));
        for lib in libraries {
            model.put_library(Library::new(*lib).with_classes(fixture_jar(lib)));
            app.add_order_entry(OrderEntry::library(*lib, DependencyScope::Test));
        }
        model.modules.push(app);
        let jdk = std::env::var_os("JAVA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.join("lib/modules").exists())
            .unwrap_or_else(|| PathBuf::from("/usr/lib/jvm/java-21-openjdk-amd64"));
        model.put_sdk(Sdk::jdk_at(jdk, Some("21")));
        model.project_sdk = Some("JDK 21".into());
        state
            .project_model()
            .expect("project model service")
            .write()
            .model = Some(Arc::new(model));
    }
    commands
        .execute("editor.openFile", &mut state, json!({"path": file}))
        .unwrap();
    wait(&mut state, 120, "the Java indexes", |state| {
        java_ready(state)
    });
    Project { root, state }
}

fn caret_at(state: &mut IdeState, marker: &str) {
    let document = &mut state.documents[state.active_document.unwrap()];
    let text = document.text.text();
    let at = text
        .find(marker)
        .unwrap_or_else(|| panic!("no `{marker}` in the text"))
        + marker.len();
    let at = document.text.byte_to_char(at);
    document
        .selections
        .set_single(archipel_ide_text::Selection::caret(at));
}

fn type_text(state: &mut IdeState, text: &str) {
    // What the window does after typed text: the completion list follows or opens.
    for c in text.chars() {
        state.editor_type_live(&c.to_string()).unwrap();
        pump(state);
    }
}

fn rows(state: &IdeState) -> Vec<String> {
    let session = state.completion.as_ref().expect("a completion popup");
    session
        .ranked
        .iter()
        .map(|row| {
            let item = &session.items[row.index];
            match item
                .label_details
                .as_ref()
                .and_then(|d| d.description.as_deref())
            {
                Some(tail) => format!("{} ({tail})", item.label),
                None => item.label.clone(),
            }
        })
        .collect()
}

fn text(state: &IdeState) -> String {
    state.documents[state.active_document.unwrap()].text.text()
}

const MAIN: &str = "src/main/java/p/Main.java";
const MAIN_TEXT: &str =
    "package p;\n\nclass Main {\n    void run() {\n        Object list = \n    }\n}\n";

/// JAVA-CMP-018, 019, CI-CMP-030: `new ` opens the popup on its own, `ArrayLi` leaves
/// `ArrayList (java.util)` first, accepting adds the import in the same undo step. The project
/// has no build file: the ad-hoc module (JDK only) serves it.
#[test]
fn typing_new_array_li_offers_array_list_and_adds_the_import() {
    let mut p = open("new", &[(MAIN, MAIN_TEXT)], &[], MAIN);
    caret_at(&mut p.state, "Object list = ");
    type_text(&mut p.state, "new ");
    assert!(
        p.state.completion.is_some(),
        "the popup opens after `new ` by itself"
    );
    type_text(&mut p.state, "ArrayLi");
    let rows = rows(&p.state);
    assert_eq!(
        rows.first().map(String::as_str),
        Some("ArrayList (java.util)"),
        "{rows:?}"
    );
    assert!(!rows.iter().any(|r| r.starts_with("List (")), "{rows:?}");
    p.state.accept_completion(false).unwrap();
    let after = text(&p.state);
    assert!(after.contains("import java.util.ArrayList;"), "{after}");
    assert!(after.contains("Object list = new ArrayList"), "{after}");
    // One undo step takes the name and the import back together.
    built_in_commands()
        .execute("editor.undo", &mut p.state, json!({}))
        .unwrap();
    let undone = text(&p.state);
    assert!(!undone.contains("import java.util.ArrayList;"), "{undone}");
    assert!(undone.contains("new ArrayLi"), "{undone}");
}

const TEST: &str = "src/test/java/p/AppTest.java";
const TEST_TEXT: &str = "package p;\n\nclass AppTest {\n    \n    void greets() {}\n}\n";

/// The owner's case 1: `@Te` in a JUnit 5 test offers `Test (org.junit.jupiter.api)` and no
/// `String` operations; accepting writes `@Test` and its import.
#[test]
fn typing_at_te_in_a_junit_test_offers_test_with_its_import() {
    let mut p = open("junit", &[(TEST, TEST_TEXT)], &["junit5"], TEST);
    caret_at(&mut p.state, "class AppTest {\n    ");
    type_text(&mut p.state, "@");
    assert!(
        p.state.completion.is_some(),
        "the popup opens after `@` by itself"
    );
    type_text(&mut p.state, "Te");
    let rows = rows(&p.state);
    assert_eq!(
        rows.first().map(String::as_str),
        Some("Test (org.junit.jupiter.api)"),
        "{rows:?}"
    );
    assert!(!rows.iter().any(|r| r.starts_with("String")), "{rows:?}");
    p.state.accept_completion(false).unwrap();
    let after = text(&p.state);
    assert!(
        after.contains("import org.junit.jupiter.api.Test;"),
        "{after}"
    );
    assert!(after.contains("    @Test\n    void greets() {}"), "{after}");
}

/// JAVA-RES-029: a production file does not see the test library.
#[test]
fn production_files_do_not_see_test_libraries() {
    let main = "src/main/java/p/App.java";
    let mut p = open(
        "prod",
        &[
            (
                main,
                "package p;\n\nclass App {\n    \n    void greets() {}\n}\n",
            ),
            (TEST, TEST_TEXT),
        ],
        &["junit5"],
        main,
    );
    caret_at(&mut p.state, "class App {\n    ");
    type_text(&mut p.state, "@Te");
    // Nothing matches (the popup closes by itself) or the rows are other annotation types.
    if p.state.completion.is_some() {
        let rows = rows(&p.state);
        assert!(!rows.iter().any(|r| r.starts_with("Test (")), "{rows:?}");
    }
}

/// JAVA-CMP-021: a lone non-imported candidate shows in the popup; nothing is inserted until
/// the user confirms.
#[test]
fn a_lone_non_imported_candidate_is_not_inserted_without_confirmation() {
    let mut p = open("lone", &[(MAIN, MAIN_TEXT)], &[], MAIN);
    caret_at(&mut p.state, "Object list = ");
    type_text(&mut p.state, "new StringJoi");
    let rows = rows(&p.state);
    assert_eq!(rows, ["StringJoiner (java.util)"]);
    let after = text(&p.state);
    assert!(!after.contains("import java.util"), "{after}");
    assert!(after.contains("new StringJoi"), "{after}");
    p.state.accept_completion(false).unwrap();
    assert!(text(&p.state).contains("import java.util.StringJoiner;"));
}

const LIST_TEXT: &str =
    "package p;\n\nimport java.util.List;\n\nclass Main {\n    void run() {\n        \n    }\n}\n";

fn messages(state: &IdeState) -> Vec<String> {
    state
        .active_problems()
        .iter()
        .map(|p| p.message.clone())
        .collect()
}

/// The owner's bug on main: `List<String> more = new ArrayLi`, accept `ArrayList` (import added),
/// type `();`: nothing stale may stay in Problems or underlined. The unresolved name is marked
/// while typing and is gone, with the import in place, once the debounce has run.
#[test]
fn typing_a_class_name_accepting_it_and_finishing_the_statement_leaves_no_error() {
    let mut p = open("stale", &[(MAIN, LIST_TEXT)], &[], MAIN);
    caret_at(&mut p.state, "void run() {\n        ");
    type_text(&mut p.state, "List<String> more = new ArrayLi");
    wait(&mut p.state, 20, "the error while typing", |s| {
        messages(s)
            .iter()
            .any(|m| m == "Cannot resolve symbol 'ArrayLi'")
    });
    p.state.accept_completion(false).unwrap();
    assert!(text(&p.state).contains("import java.util.ArrayList;"));
    type_text(&mut p.state, "();");
    wait(&mut p.state, 20, "no error after the debounce", |s| {
        s.active_problems()
            .iter()
            .all(|p| p.severity != archipel_ide_core::Severity::Error)
    });
    let left = messages(&p.state);
    assert!(
        left.iter().all(|m| !m.contains("ArrayLi")),
        "stale problems: {left:?}"
    );
    assert!(
        !left.iter().any(|m| m.starts_with("Syntax error")),
        "stale problems: {left:?}"
    );
}

/// The owner's case `foo();` in a class without `foo`, unsaved, character by character: the
/// call is red within the budget after the last keystroke (100 ms Java debounce plus the pass,
/// plan 3.5), with the catalog message and on exactly the name.
#[test]
fn an_unknown_method_call_is_red_shortly_after_the_last_keystroke() {
    let mut p = open("foo", &[(MAIN, MAIN_TEXT)], &[], MAIN);
    caret_at(&mut p.state, "Object list = ");
    for c in "foo();".chars() {
        p.state.editor_type_live(&c.to_string()).unwrap();
        pump(&mut p.state);
    }
    let typed = Instant::now();
    let mut seen = None;
    while typed.elapsed() < Duration::from_secs(5) {
        pump(&mut p.state);
        if let Some(problem) = p
            .state
            .active_problems()
            .into_iter()
            .find(|p| p.message == "Cannot resolve method 'foo()'")
        {
            seen = Some((typed.elapsed(), problem));
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let (after, problem) = seen.expect("the unknown call is marked");
    eprintln!(
        "unknown call marked {} ms after the last keystroke",
        after.as_millis()
    );
    assert_eq!(
        (problem.line, problem.end_line),
        (problem.line, problem.line)
    );
    assert_eq!(problem.end_column - problem.column, "foo".len());
    assert!(
        after < Duration::from_millis(1000),
        "marked only after {after:?}"
    );
}

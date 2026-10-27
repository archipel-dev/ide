//! The headline scenario through the whole app stack, without a language server: a Gradle project
//! is imported into the project model, the Java indexes follow it, the daemon's `java-resolve`
//! pass marks the unresolved `assertThat` natively, Alt+Enter lists `Import static method...`
//! first, the chooser (`Method to Import`) lists AssertJ first and applying the first row writes
//! the import. Needs Gradle and warm caches, so it is ignored by default:
//! `cargo test -p archipel-ide-core --test java_native_flow -- --ignored --nocapture`.

use archipel_ide_core::{IdeState, built_in_commands};
use archipel_ide_lsp::{LspManager, ServerRegistry};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const TEST_FILE: &str = "src/test/java/com/example/AppTest.java";

fn fixture(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "archipel-java-native-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let files: [(&str, &str); 4] = [
        (
            "settings.gradle",
            "rootProject.name = 'java-gradle-assertj'\n",
        ),
        (
            "build.gradle",
            "plugins {\n    id 'java'\n}\n\nrepositories {\n    mavenCentral()\n}\n\ndependencies {\n    testImplementation 'org.junit.jupiter:junit-jupiter:5.11.0'\n    testRuntimeOnly 'org.junit.platform:junit-platform-launcher:1.11.0'\n    testImplementation 'org.assertj:assertj-core:3.26.3'\n}\n\ntest {\n    useJUnitPlatform()\n}\n",
        ),
        (
            "src/main/java/com/example/App.java",
            "package com.example;\n\npublic class App {\n    public static String greeting(String name) {\n        return \"Hello, \" + name + \"!\";\n    }\n}\n",
        ),
        (
            TEST_FILE,
            "package com.example;\n\nimport org.junit.jupiter.api.Test;\n\nclass AppTest {\n    @Test\n    void greets() {\n        assertThat(App.greeting(\"Java\")).isEqualTo(\"Hello, Java!\");\n    }\n}\n",
        ),
    ];
    for (path, text) in files {
        let target = root.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, text).unwrap();
    }
    std::fs::canonicalize(&root).unwrap()
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
    panic!(
        "timed out waiting for {what}; status {}; problems {:?}",
        state.status,
        state.active_problems()
    );
}

#[test]
#[ignore = "needs Gradle and warm dependency caches"]
fn unimported_assert_that_gets_the_native_error_and_the_static_import() {
    let root = fixture("headline");
    let commands = built_in_commands();
    let mut state = IdeState::default();
    state.lsp = LspManager::new(ServerRegistry::empty());
    commands
        .execute("project.open", &mut state, json!({"path": root}))
        .unwrap();
    commands
        .execute("editor.openFile", &mut state, json!({"path": TEST_FILE}))
        .unwrap();
    // While the model imports, the unresolved reference is pending (JAVA-ERR-003), then it is an error.
    wait(&mut state, 120, "the native error", |state| {
        state
            .active_problems()
            .iter()
            .any(|p| p.message == "Cannot resolve method 'assertThat(String)'")
    });
    let problems = state.active_problems();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!((problems[0].line, problems[0].column), (7, 8));
    // Caret on the name, Alt+Enter.
    {
        let document = &mut state.documents[state.active_document.unwrap()];
        let at = document.text.text().find("assertThat").unwrap() + 3;
        let at = document.text.byte_to_char(at);
        document
            .selections
            .set_single(archipel_ide_text::Selection::caret(at));
    }
    state.request_code_actions(true).unwrap();
    wait(&mut state, 30, "the Alt+Enter popup", |state| {
        state.code_actions.popup && state.current_alerts().is_some()
    });
    let set = state.current_alerts().expect("alerts");
    assert_eq!(
        set.actions[0].text,
        "Import static method...",
        "{:?}",
        set.texts()
    );
    assert_eq!(format!("{:?}", set.actions[0].group), "Error");
    commands
        .execute("code.applyAlert", &mut state, json!({"index": 0}))
        .unwrap();
    // Several candidates: the chooser.
    let (provider, chooser) = state.import_chooser().expect("the chooser opened");
    assert_eq!(provider, "java");
    assert_eq!(chooser.title, "Method to Import");
    assert_eq!(chooser.rows[0].label, "Assertions.assertThat");
    assert_eq!(
        chooser.rows[0].detail.as_deref(),
        Some("(org.assertj.core.api)")
    );
    commands
        .execute("java.chooseImport", &mut state, json!({"index": 0}))
        .unwrap();
    let text = state.documents[state.active_document.unwrap()].text.text();
    assert!(text.contains("\nimport org.junit.jupiter.api.Test;\n\nimport static org.assertj.core.api.Assertions.assertThat;\n\nclass AppTest"), "{text}");
    wait(&mut state, 30, "the error to go away", |state| {
        state.active_problems().is_empty()
    });
    let _ = Path::new("");
    let _ = std::fs::remove_dir_all(&root);
}

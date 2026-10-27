//! End-to-end checks against a real Eclipse JDT LS (ignored by default).
//!
//! Needs jdtls installed through the toolchain store (`archipel-cli run
//! toolchain.install '{"component":"jdtls","wait":true}'`), a JDK it runs on
//! (21 or newer, matching the jdtls release) and network access for the
//! build tool's dependencies. `ARCHIPEL_REAL_JDTLS_PROJECT` names a Gradle
//! or Maven project with AssertJ as a test dependency and
//! `src/test/java/com/example/AppTest.java` calling `assertThat(...)`
//! without the static import. Run with
//! `cargo test -p archipel-ide-core --test real_jdtls -- --ignored --nocapture`.

use archipel_ide_core::{IdeState, built_in_commands};
use serde_json::json;
use std::time::{Duration, Instant};

const TEST_FILE: &str = "src/test/java/com/example/AppTest.java";

fn pump(state: &mut IdeState) {
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
        std::thread::sleep(Duration::from_millis(50));
    }
    let servers: Vec<_> = state
        .lsp
        .servers()
        .iter()
        .map(|s| format!("{:?}", s))
        .collect();
    if let Some(service) = state.daemon_service() {
        let feed = service.shared().external.all();
        eprintln!("feed: {feed:?}");
        let file = state.analysis.host.lookup(std::path::Path::new(TEST_FILE));
        if let Some(markup) = file.and_then(|f| service.lock().markup(f).cloned()) {
            eprintln!(
                "markup rev {:?}: {:?}",
                markup.rev(),
                markup
                    .iter()
                    .map(|i| (i.tool.as_str().to_owned(), i.range.clone()))
                    .collect::<Vec<_>>()
            );
        }
        eprintln!("doc rev {}", state.documents[0].text.revision());
    }
    panic!(
        "timed out waiting for {what}; status: {}; problems: {:?}; servers: {servers:?}",
        state.status,
        state.active_problems()
    );
}

#[test]
#[ignore = "requires jdtls, a JDK and network access"]
fn unimported_assert_that_is_marked_and_offers_the_static_import() {
    let Ok(project) = std::env::var("ARCHIPEL_REAL_JDTLS_PROJECT") else {
        eprintln!("ARCHIPEL_REAL_JDTLS_PROJECT is not set");
        return;
    };
    let commands = built_in_commands();
    let mut state = IdeState::default();
    commands
        .execute("project.open", &mut state, json!({"path": project}))
        .unwrap();
    commands
        .execute("editor.openFile", &mut state, json!({"path": TEST_FILE}))
        .unwrap();
    // The import is shown as progress, diagnostics wait for it.
    let mut saw_import = false;
    wait(&mut state, 300, "the error marker", |state| {
        saw_import |= state.lsp.servers().iter().any(|s| s.importing);
        state
            .active_problems()
            .iter()
            .any(|p| p.message.contains("assertThat") && p.source.as_deref().is_some())
    });
    eprintln!("importing was visible: {saw_import}");
    let problems = state.active_problems();
    eprintln!("problems: {problems:?}");
    // The caret on the unresolved method, wait for the server to settle.
    {
        let document = &mut state.documents[state.active_document.unwrap()];
        let at = document.text.text().find("assertThat").unwrap() + 3;
        document
            .selections
            .set_single(archipel_ide_text::Selection::caret(at));
    }
    wait(&mut state, 60, "an idle server", |state| {
        state.lsp.servers().iter().all(|s| s.progress.is_empty())
    });
    state.request_code_actions(true).unwrap();
    wait(&mut state, 60, "the Alt+Enter popup", |state| {
        state.code_actions.popup
    });
    let set = state.current_alerts().expect("alerts");
    eprintln!("Alt+Enter: {:?}", set.texts());
    let row = set
        .actions
        .iter()
        .position(|row| row.text.contains("static import") && row.text.contains("assertThat"))
        .unwrap_or_else(|| panic!("static import offered: {:?}", set.texts()));
    assert_eq!(
        format!("{:?}", set.actions[row].group),
        "Error",
        "the fix of the error under the caret comes first"
    );
    assert_eq!(row, 0, "{:?}", set.texts());
    commands
        .execute("code.applyAlert", &mut state, json!({"index": row}))
        .unwrap();
    // jdtls defers the edit (`codeAction/resolve`): it arrives a moment later.
    wait(&mut state, 30, "the resolved edit", |state| {
        state.documents[state.active_document.unwrap()]
            .text
            .text()
            .contains("import static")
    });
    let text = state.documents[state.active_document.unwrap()].text.text();
    eprintln!("{text}");
    assert!(
        text.contains("import static org.assertj.core.api.Assertions.assertThat;"),
        "{text}"
    );
    // One undo restores the file, and the error goes away once jdtls
    // revalidated the new text.
    wait(&mut state, 60, "the error to disappear", |state| {
        !state
            .active_problems()
            .iter()
            .any(|p| p.message.contains("assertThat"))
    });
}

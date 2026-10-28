//! PLAT-SAVE-110 and 111 (docs/specs/platform/09-saving-and-autosave.md): the highlighting
//! daemon analyzes the document text, so errors appear while typing without a save, and a
//! save does not restart analysis. Language server diagnostics are part of it: a server that
//! serves them on request (rust-analyzer's native ones) is pulled after the edit, not only
//! after the save. Against the scriptable `archipel-fake-lsp` standing in for a Java server
//! (`--pull-diagnostics`, `--log`), with the native Java analysis running next to it.
//!
//! The server binary is built by `cargo test --workspace`; when this crate's tests run alone
//! and it is missing, the tests are skipped with a note.

use archipel_ide_core::{IdeState, built_in_commands};
use archipel_ide_daemon::passes;
use archipel_ide_lsp::{
    LanguageServerProvider, LaunchConfig, LaunchContext, LspError, LspManager, ServerRegistry,
    ServerSpec,
};
use serde_json::json;
use std::{
    path::{Path, PathBuf},
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

struct Fixture {
    root: PathBuf,
    state: IdeState,
    log: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.state.lsp.shutdown();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

const SAVED: &str = "class A {\n}\n";

/// A project with `A.java`; with a server it is the fake one, without the language has no
/// server and only the native analysis runs.
fn open(server: Option<PathBuf>, args: &[&str], name: &str) -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "archipel-ide-core-unsaved-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("A.java"), SAVED).unwrap();
    let log = root.join("server.log");
    let mut all: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
    all.push("--log".into());
    all.push(log.display().to_string());
    let mut registry = ServerRegistry::empty();
    if let Some(server) = server.clone() {
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
            all,
        )));
    }
    let commands = built_in_commands();
    let mut state = IdeState::default();
    state.lsp = LspManager::new(registry);
    commands
        .execute("project.open", &mut state, json!({"path": root}))
        .unwrap();
    commands
        .execute("editor.openFile", &mut state, json!({"path": "A.java"}))
        .unwrap();
    let mut fixture = Fixture { root, state, log };
    // The server is up when it has announced its trigger characters.
    if server.is_some() {
        assert!(
            wait(&mut fixture.state, |state| {
                state
                    .active_absolute_path()
                    .is_some_and(|path| !state.lsp.trigger_characters(&path).is_empty())
            }),
            "the server did not start"
        );
    }
    fixture
}

fn pump(state: &mut IdeState) {
    state.poll_lsp();
    state.poll_analysis();
}

fn wait(state: &mut IdeState, mut done: impl FnMut(&mut IdeState) -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        pump(state);
        if done(state) {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

fn run_for(state: &mut IdeState, duration: Duration) {
    let until = Instant::now() + duration;
    while Instant::now() < until {
        pump(state);
        thread::sleep(Duration::from_millis(10));
    }
}

fn type_text(state: &mut IdeState, text: &str) {
    let commands = built_in_commands();
    for c in text.chars() {
        commands
            .execute("editor.type", state, json!({"text": c.to_string()}))
            .unwrap();
    }
}

fn received(log: &Path, prefix: &str) -> usize {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.starts_with(prefix))
        .count()
}

fn problems(state: &mut IdeState) -> Vec<String> {
    state
        .cached_active_problems()
        .iter()
        .map(|problem| problem.message.clone())
        .collect()
}

/// Runs of the passes that analyze the text (the external diagnostics pass shows what the
/// server reported and may legitimately run again when a server reports after a save).
fn native_runs(state: &IdeState) -> u64 {
    let service = state.daemon_service().expect("the daemon is running");
    let daemon = service.lock();
    [
        passes::SYNTAX,
        passes::GENERAL,
        passes::LOCAL_INSPECTIONS,
        passes::IDENTIFIER_HIGHLIGHTER,
        passes::LINE_MARKERS,
    ]
    .into_iter()
    .map(|pass| daemon.pass_stats(pass).runs)
    .sum()
}

/// PLAT-SAVE-110: type a marker the server reports as an error; it shows with the file unsaved,
/// and the server never got a `didSave`.
#[test]
fn a_server_error_shows_while_typing_without_saving() {
    let Some(server) = fake_server() else {
        eprintln!("archipel-fake-lsp not built; run `cargo test --workspace`");
        return;
    };
    let mut fixture = open(Some(server), &["--pull-diagnostics"], "typing");
    type_text(&mut fixture.state, "// @error");
    assert!(
        wait(&mut fixture.state, |state| problems(state)
            .iter()
            .any(|message| message == "marker @error")),
        "{:?}",
        problems(&mut fixture.state)
    );
    let document = &fixture.state.documents[0];
    assert!(document.text.is_dirty(), "still unsaved");
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("A.java")).unwrap(),
        SAVED,
        "nothing was written"
    );
    assert_eq!(received(&fixture.log, "textDocument/didSave"), 0);
    assert!(received(&fixture.log, "textDocument/diagnostic") >= 1);
    // Nine typed characters, at most a few changes: the burst is coalesced.
    let changes = received(&fixture.log, "textDocument/didChange");
    assert!((1..=3).contains(&changes), "{changes} changes");
}

/// PLAT-SAVE-110, natively: a syntax error typed into the document shows without a save.
#[test]
fn a_syntax_error_shows_while_typing_without_saving() {
    let mut fixture = open(None, &[], "syntax");
    run_for(&mut fixture.state, Duration::from_millis(400));
    let errors = |state: &mut IdeState| {
        state
            .cached_active_problems()
            .iter()
            .filter(|problem| problem.severity == archipel_ide_core::Severity::Error)
            .count()
    };
    assert_eq!(errors(&mut fixture.state), 0);
    // Into the class body, then a broken statement.
    fixture.state.documents[0]
        .selections
        .set_single(archipel_ide_text::Selection::caret(10));
    type_text(&mut fixture.state, "int x = ;");
    assert!(
        wait(&mut fixture.state, |state| errors(state) > 0),
        "{:?}",
        problems(&mut fixture.state)
    );
    assert!(fixture.state.documents[0].text.is_dirty());
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("A.java")).unwrap(),
        SAVED
    );
}

/// PLAT-SAVE-111: a save does not restart analysis. After the text was analyzed, a save adds no
/// run of a text-analyzing pass, sends the server no change and no pull (only `didSave`).
#[test]
fn saving_does_not_restart_analysis() {
    let Some(server) = fake_server() else {
        eprintln!("archipel-fake-lsp not built; run `cargo test --workspace`");
        return;
    };
    let mut fixture = open(Some(server), &["--pull-diagnostics"], "save");
    type_text(&mut fixture.state, "// @error");
    assert!(wait(&mut fixture.state, |state| problems(state)
        .iter()
        .any(|message| message == "marker @error")));
    // Everything settles: the daemon is idle and the exchange with the server is over.
    run_for(&mut fixture.state, Duration::from_millis(1200));
    let service = fixture.state.daemon_service().unwrap();
    assert!(service.lock().all_idle(), "the daemon should be idle");
    let runs = native_runs(&fixture.state);
    let changes = received(&fixture.log, "textDocument/didChange");
    let pulls = received(&fixture.log, "textDocument/diagnostic");
    assert!(runs > 0 && changes > 0 && pulls > 0);

    fixture.state.save_all().unwrap();
    run_for(&mut fixture.state, Duration::from_millis(1500));

    assert!(!fixture.state.documents[0].text.is_dirty(), "saved");
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("A.java")).unwrap(),
        fixture.state.documents[0].text.text()
    );
    assert_eq!(received(&fixture.log, "textDocument/didSave"), 1);
    assert_eq!(
        native_runs(&fixture.state),
        runs,
        "the save restarted the analysis"
    );
    assert_eq!(received(&fixture.log, "textDocument/didChange"), changes);
    assert_eq!(received(&fixture.log, "textDocument/diagnostic"), pulls);
}

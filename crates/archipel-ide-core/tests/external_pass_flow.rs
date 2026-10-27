//! The language server's diagnostics and code actions through the daemon:
//! one markup for the Problems view, F2 and the squiggles, one Alt+Enter
//! list in IntelliJ's order, WorkspaceEdits and commands with undo. Runs
//! against the scriptable `archipel-fake-lsp` (served as the Java server).

use archipel_ide_core::intentions::IntentionGroup;
use archipel_ide_core::{IdeState, built_in_commands};
use archipel_ide_lsp::{
    LanguageServerProvider, LaunchConfig, LaunchContext, LspError, LspManager, ServerRegistry,
    ServerSpec,
};
use archipel_ide_text::Selection;
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
    let path = exe
        .parent()?
        .parent()?
        .join(format!("archipel-fake-lsp{}", std::env::consts::EXE_SUFFIX));
    path.is_file().then_some(path)
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

const SOURCE: &str = "class A {\n  void f() { } // @error\n  void g() { } // @warning\n}\n";

/// Opens `A.java` served by the fake server (as the Java language server).
fn open(server: PathBuf, args: &[&str], name: &str) -> (PathBuf, IdeState) {
    let root = std::env::temp_dir().join(format!("archipel-ide-ext-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("A.java"), SOURCE).unwrap();
    let mut registry = ServerRegistry::empty();
    registry.register(Arc::new(Fake(
        server,
        ServerSpec {
            id: "jdtls",
            language: "Java",
            extensions: &[("java", "java")],
            file_names: &[],
            candidates: &[],
            root_markers: &[],
            install_hint: "",
            notify_missing: false,
            initialization_options: None,
        },
        args.iter().map(|a| a.to_string()).collect(),
    )));
    let mut state = IdeState::default();
    state.lsp = LspManager::new(registry);
    let commands = built_in_commands();
    commands
        .execute("project.open", &mut state, json!({"path": root}))
        .unwrap();
    commands
        .execute("editor.openFile", &mut state, json!({"path": "A.java"}))
        .unwrap();
    (root, state)
}

fn caret_at(state: &mut IdeState, needle: &str, plus: usize) {
    let document = &mut state.documents[0];
    let at = document.text.text().find(needle).unwrap() + plus;
    let at = document.text.byte_to_char(at);
    document.selections.set_single(Selection::caret(at));
}

fn lsp_tools(state: &IdeState) -> Vec<String> {
    let service = state.daemon_service().unwrap();
    let file = state.analysis.host.lookup(Path::new("A.java")).unwrap();
    let daemon = service.lock();
    daemon
        .markup(file)
        .map(|m| m.iter().map(|i| i.tool.as_str().to_owned()).collect())
        .unwrap_or_default()
}

#[test]
fn diagnostics_reach_the_problems_view_f2_and_the_markup_through_the_daemon() {
    let Some(server) = fake_server() else {
        eprintln!("archipel-fake-lsp not built; run `cargo test --workspace`");
        return;
    };
    let (root, mut state) = open(server, &[], "diag");
    assert!(
        wait(&mut state, |state| state.active_problems().len() == 2),
        "{:?}",
        state.active_problems()
    );
    let problems = state.active_problems();
    assert_eq!(problems[0].message, "marker @error");
    assert_eq!(problems[0].source.as_deref(), Some("fake"));
    assert_eq!(problems[0].code.as_deref(), Some("F1"));
    assert_eq!((problems[0].line, problems[0].column), (1, 18));
    // The same highlights are in the daemon's markup, owned by `lsp:jdtls`.
    assert_eq!(lsp_tools(&state), ["lsp:jdtls", "lsp:jdtls"]);
    // The feed was read through the pass: the daemon applied results.
    // The Problems view and the tree's error propagation read the same data.
    state.refresh_problems();
    assert_eq!(state.problems.counts().errors, 1);
    assert_eq!(state.problems.counts().warnings, 1);
    assert!(state.problems.has_errors(Path::new("A.java")));
    // F2 lands on the error (errors first), from the markup.
    let target = state.goto_problem(true).unwrap();
    assert_eq!(target.message, "marker @error");
    let document = &state.documents[0];
    assert_eq!(
        document
            .text
            .char_to_line(document.selections.primary().head),
        1
    );
    // The server cleared it: the daemon's markup follows.
    let fixed = state.documents[0].text.text().replace("@error", "ok");
    let length = state.documents[0].text.len_chars();
    state.documents[0].text.edit(0, length, &fixed).unwrap();
    let source = state.documents[0].text.text();
    state.documents[0].refresh_analysis(&source);
    state.sync_active_lsp();
    assert!(wait(&mut state, |state| state.active_problems().len() == 1));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn alt_enter_is_one_list_fixes_of_the_error_first_then_warnings_then_intentions() {
    let Some(server) = fake_server() else {
        return;
    };
    let (root, mut state) = open(server, &[], "order");
    assert!(wait(&mut state, |state| state.active_problems().len() == 2));
    // Caret on the error; the warning is elsewhere on its own line.
    caret_at(&mut state, "@error", 2);
    state.request_code_actions(true).unwrap();
    assert!(wait(&mut state, |state| state.code_actions.popup));
    let set = state.current_alerts().expect("alerts");
    let rows: Vec<(String, IntentionGroup)> = set
        .actions
        .iter()
        .filter(|row| row.submenu.is_none())
        .map(|row| (row.text.clone(), row.group))
        .collect();
    assert_eq!(
        rows[0],
        ("Fix marker @error".into(), IntentionGroup::Error),
        "{rows:?}"
    );
    // Quick fixes sort before every intention.
    let first_other = rows
        .iter()
        .position(|(_, group)| *group == IntentionGroup::Other)
        .expect("intentions present");
    assert!(
        rows[..first_other]
            .iter()
            .all(|(_, group)| *group != IntentionGroup::Other),
        "{rows:?}"
    );
    assert!(
        rows[first_other..]
            .iter()
            .all(|(_, group)| *group == IntentionGroup::Other),
        "no fix after an intention: {rows:?}"
    );
    // Refactor and source actions of the server are intentions.
    assert!(rows.contains(&("Add header comment".into(), IntentionGroup::Other)));
    // The fix is attached to the server's highlight at the caret.
    let fix = set
        .actions
        .iter()
        .find(|a| a.text == "Fix marker @error")
        .unwrap();
    let anchor = fix.anchor.clone().expect("attached to its diagnostic");
    let text = state.documents[0].text.text();
    assert_eq!(&text[anchor.start as usize..anchor.end as usize], "@error");
    // The bulb is red: a fix of an error.
    assert_eq!(format!("{:?}", set.bulb()), "ErrorFix", "{:?}", set.texts());
    // Caret on the warning's line but not on the warning, error fixes first.
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn a_fix_elsewhere_on_the_line_is_a_remote_error_fix_after_the_one_at_the_caret() {
    let Some(server) = fake_server() else {
        return;
    };
    let (root, mut state) = open(server, &[], "remote");
    let text = "class A {\n  void f() { } // @warning and @error\n}\n";
    let length = state.documents[0].text.len_chars();
    state.documents[0].text.edit(0, length, text).unwrap();
    let source = state.documents[0].text.text();
    state.documents[0].refresh_analysis(&source);
    state.sync_active_lsp();
    assert!(wait(&mut state, |state| state.active_problems().len() == 2));
    caret_at(&mut state, "@warning", 2);
    state.request_code_actions(true).unwrap();
    assert!(wait(&mut state, |state| state.code_actions.popup));
    let set = state.current_alerts().unwrap();
    let groups: Vec<(String, IntentionGroup)> = set
        .actions
        .iter()
        .filter(|a| a.text.starts_with("Fix marker"))
        .map(|a| (a.text.clone(), a.group))
        .collect();
    assert_eq!(
        groups,
        [
            ("Fix marker @warning".to_owned(), IntentionGroup::Inspection),
            ("Fix marker @error".to_owned(), IntentionGroup::RemoteError),
        ]
    );
    std::fs::remove_dir_all(root).ok();
}

fn apply_row(state: &mut IdeState, text: &str) {
    let set = state.current_alerts().expect("alerts");
    let index = set
        .actions
        .iter()
        .position(|row| row.text == text)
        .unwrap_or_else(|| panic!("{text} in {:?}", set.texts()));
    built_in_commands()
        .execute("code.applyAlert", state, json!({"index": index}))
        .unwrap();
}

#[test]
fn applying_a_workspace_edit_fix_is_one_undo_step() {
    let Some(server) = fake_server() else {
        return;
    };
    let (root, mut state) = open(server, &[], "edit");
    assert!(wait(&mut state, |state| state.active_problems().len() == 2));
    caret_at(&mut state, "@error", 2);
    state.request_code_actions(true).unwrap();
    assert!(wait(&mut state, |state| state.code_actions.popup));
    apply_row(&mut state, "Fix marker @error");
    assert_eq!(
        state.documents[0].text.text(),
        SOURCE.replace("@error", "fixed")
    );
    assert_eq!(state.status, "Applied: Fix marker @error");
    assert!(state.documents[0].text.undo());
    assert_eq!(state.documents[0].text.text(), SOURCE);
    assert!(!state.documents[0].text.can_undo(), "one step");
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn a_java_apply_workspace_edit_command_is_applied_by_the_editor() {
    let Some(server) = fake_server() else {
        return;
    };
    let (root, mut state) = open(server, &["--java-commands"], "command");
    assert!(wait(&mut state, |state| state.active_problems().len() == 2));
    caret_at(&mut state, "@error", 2);
    state.request_code_actions(true).unwrap();
    assert!(wait(&mut state, |state| state.code_actions.popup));
    apply_row(&mut state, "Fix marker @error");
    assert_eq!(
        state.documents[0].text.text(),
        SOURCE.replace("@error", "fixed")
    );
    assert!(state.documents[0].text.undo());
    assert_eq!(state.documents[0].text.text(), SOURCE);
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn a_server_command_is_executed_and_its_edit_applied_with_undo() {
    let Some(server) = fake_server() else {
        return;
    };
    let (root, mut state) = open(server, &[], "servercommand");
    assert!(wait(&mut state, |state| state.active_problems().len() == 2));
    caret_at(&mut state, "@error", 2);
    state.request_code_actions(true).unwrap();
    assert!(wait(&mut state, |state| state.code_actions.popup));
    apply_row(&mut state, "Run fake command");
    assert!(wait(&mut state, |state| state.documents[0]
        .text
        .text()
        .starts_with("// applied\n")));
    assert!(state.documents[0].text.undo());
    assert_eq!(state.documents[0].text.text(), SOURCE);
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn diagnostics_wait_for_the_project_import_and_appear_when_it_is_done() {
    let Some(server) = fake_server() else {
        return;
    };
    let (root, mut state) = open(server, &["--importing", "1"], "import");
    // The server publishes while it still imports ("Starting"): held back.
    assert!(wait(&mut state, |state| state
        .lsp
        .servers()
        .first()
        .is_some_and(|s| s.importing)));
    for _ in 0..60 {
        pump(&mut state);
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        state.active_problems().is_empty(),
        "{:?}",
        state.active_problems()
    );
    assert!(lsp_tools(&state).is_empty());
    // The import ends (the fake ends it after the first completion request).
    let path = state.active_absolute_path().unwrap();
    state
        .lsp
        .request_completion(&path, 0, 0, archipel_ide_lsp::CompletionTrigger::Invoked)
        .unwrap();
    assert!(wait(&mut state, |state| state.active_problems().len() == 2));
    assert!(!state.lsp.servers()[0].importing);
    std::fs::remove_dir_all(root).ok();
}

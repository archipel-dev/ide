//! Diagnostics of unsaved text: edits reach the server promptly (and in bursts), a server with
//! `diagnosticProvider` is pulled after every change, and nothing needs a save. Against the
//! scriptable `archipel-fake-lsp` (`--pull-diagnostics`, `--log`).

use archipel_ide_lsp::{LaunchConfig, LspService, ServerState};
use std::{
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

const FAKE: &str = env!("CARGO_BIN_EXE_archipel-fake-lsp");

struct Setup {
    root: PathBuf,
    file: PathBuf,
    log: PathBuf,
}

fn setup(name: &str) -> Setup {
    let root = std::env::temp_dir().join(format!(
        "archipel-ide-lsp-pull-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    std::fs::write(root.join("App.fake"), "ok\n").unwrap();
    Setup {
        file: root.join("App.fake"),
        log: root.join("server.log"),
        root,
    }
}

fn launch(setup: &Setup, args: &[&str]) -> LspService {
    let mut all: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
    all.push("--log".into());
    all.push(setup.log.display().to_string());
    LspService::launch(&LaunchConfig::new("fake", FAKE).args(all), &setup.root).unwrap()
}

fn received(log: &Path, prefix: &str) -> usize {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.starts_with(prefix))
        .count()
}

fn wait(service: &mut LspService, mut done: impl FnMut(&mut LspService) -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        service.poll().unwrap();
        if done(service) {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

fn messages(service: &LspService, path: &Path) -> Vec<String> {
    let mut messages: Vec<String> = service
        .diagnostics()
        .get(path)
        .map(|list| list.iter().map(|d| d.message.clone()).collect())
        .unwrap_or_default();
    messages.sort();
    messages
}

fn ready(service: &mut LspService) {
    assert!(wait(service, |s| s.state() == ServerState::Ready));
}

/// The server that serves diagnostics only on request (rust-analyzer's native ones) shows the
/// errors of the text as it is edited, with nothing saved.
#[test]
fn pull_diagnostics_follow_the_unsaved_text() {
    let setup = setup("follow");
    let mut service = launch(&setup, &["--pull-diagnostics"]);
    service
        .sync_document(&setup.file, "fake", "ok\n", 1)
        .unwrap();
    ready(&mut service);
    assert!(service.supports_pull_diagnostics());
    // Opening is a pull; the server asked for another with workspace/diagnostic/refresh.
    assert!(wait(&mut service, |s| messages(s, &setup.file)
        == ["fake diagnostic (pull)"]));
    // An edit that adds an error marker: the pull after the change shows it.
    service
        .sync_document(&setup.file, "fake", "ok @error\n", 2)
        .unwrap();
    assert!(
        wait(&mut service, |s| messages(s, &setup.file)
            == ["marker @error"]),
        "{:?}",
        messages(&service, &setup.file)
    );
    // And an edit that removes it clears it again.
    service
        .sync_document(&setup.file, "fake", "ok\n", 3)
        .unwrap();
    assert!(wait(&mut service, |s| messages(s, &setup.file)
        == ["fake diagnostic (pull)"]));
    assert_eq!(
        received(&setup.log, "textDocument/didSave"),
        0,
        "never saved"
    );
    assert_eq!(
        received(&setup.log, "textDocument/didChange v=3"),
        1,
        "the text reached the server"
    );
    let _ = std::fs::remove_dir_all(&setup.root);
}

/// Each change is followed by one pull, and the server's refresh request after it is answered
/// by one more; the exchange ends there (no pull loop).
#[test]
fn a_refresh_request_pulls_again_and_the_exchange_ends() {
    let setup = setup("refresh");
    let mut service = launch(&setup, &["--pull-diagnostics"]);
    service
        .sync_document(&setup.file, "fake", "ok\n", 1)
        .unwrap();
    // Open: a pull, then the pull the refresh asks for.
    assert!(wait(&mut service, |_| received(
        &setup.log,
        "textDocument/diagnostic"
    ) >= 2));
    service
        .sync_document(&setup.file, "fake", "ok @warning\n", 2)
        .unwrap();
    assert!(wait(&mut service, |_| received(
        &setup.log,
        "textDocument/diagnostic"
    ) >= 4));
    let until = Instant::now() + Duration::from_millis(500);
    while Instant::now() < until {
        service.poll().unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(received(&setup.log, "textDocument/diagnostic"), 4);
    let _ = std::fs::remove_dir_all(&setup.root);
}

/// A burst of edits is sent as one `didChange` with the last text; a request in between sends
/// it at once so the server never answers for stale text.
#[test]
fn a_burst_of_edits_is_one_change_and_a_request_sends_it_first() {
    let setup = setup("burst");
    let mut service = launch(&setup, &[]);
    service
        .sync_document(&setup.file, "fake", "ok\n", 1)
        .unwrap();
    ready(&mut service);
    for version in 2..=6 {
        service
            .sync_document(&setup.file, "fake", &format!("ok{version}\n"), version)
            .unwrap();
    }
    assert_eq!(
        received(&setup.log, "textDocument/didChange"),
        0,
        "waiting for the quiet time"
    );
    assert!(service.has_pending_work());
    assert!(wait(&mut service, |_| received(
        &setup.log,
        "textDocument/didChange"
    ) == 1));
    assert_eq!(received(&setup.log, "textDocument/didChange v=6"), 1);
    // A request sends what is waiting.
    service
        .sync_document(&setup.file, "fake", "ok7\n", 7)
        .unwrap();
    service
        .request_raw("textDocument/hover", serde_json::json!({}))
        .unwrap();
    assert!(wait(&mut service, |_| received(
        &setup.log,
        "textDocument/didChange v=7"
    ) == 1));
    let _ = std::fs::remove_dir_all(&setup.root);
}

/// With no delay every edit is sent at once (what tests and `--change-delay 0` callers get).
#[test]
fn without_a_delay_every_edit_is_sent_at_once() {
    let setup = setup("nodelay");
    let mut service = launch(&setup, &[]);
    service.set_change_delay(Duration::ZERO);
    service
        .sync_document(&setup.file, "fake", "ok\n", 1)
        .unwrap();
    ready(&mut service);
    for version in 2..=4 {
        service
            .sync_document(&setup.file, "fake", "ok\n", version)
            .unwrap();
    }
    assert!(wait(&mut service, |_| received(
        &setup.log,
        "textDocument/didChange"
    ) == 3));
    let _ = std::fs::remove_dir_all(&setup.root);
}

/// Pushed and pulled diagnostics of one file are shown together without repeats (rust-analyzer
/// pulls its native ones and pushes the `cargo check` results after a save).
#[test]
fn pushed_and_pulled_diagnostics_are_merged() {
    let setup = setup("merge");
    let mut service = launch(&setup, &["--pull-diagnostics"]);
    service
        .sync_document(&setup.file, "fake", "ok @error\n", 1)
        .unwrap();
    ready(&mut service);
    assert!(wait(&mut service, |s| messages(s, &setup.file)
        == ["marker @error"]));
    service.save_document(&setup.file).unwrap();
    assert!(wait(&mut service, |s| messages(s, &setup.file)
        == ["fake check warning", "marker @error"]));
    // The next change pulls again; the pushed check result stays until the server replaces it.
    service
        .sync_document(&setup.file, "fake", "ok @hint\n", 2)
        .unwrap();
    assert!(wait(&mut service, |s| messages(s, &setup.file)
        == ["fake check warning", "marker @hint"]));
    let _ = std::fs::remove_dir_all(&setup.root);
}

/// A server without `diagnosticProvider` is never pulled; its pushes are shown as before.
#[test]
fn a_pushing_server_is_not_pulled() {
    let setup = setup("push");
    let mut service = launch(&setup, &[]);
    service
        .sync_document(&setup.file, "fake", "ok @error\n", 1)
        .unwrap();
    assert!(wait(&mut service, |s| messages(s, &setup.file)
        == ["marker @error"]));
    assert!(!service.supports_pull_diagnostics());
    assert_eq!(received(&setup.log, "textDocument/diagnostic"), 0);
    let _ = std::fs::remove_dir_all(&setup.root);
}

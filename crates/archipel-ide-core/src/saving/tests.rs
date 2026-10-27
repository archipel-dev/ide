//! Tests of the saving spec (`docs/specs/platform/09-saving-and-autosave.md`); every test name
//! carries the requirement IDs it covers.

use super::*;
use crate::DocumentBacking;
use crate::built_in_commands;
use crate::completion::CompletionSession;
use crate::services::SavingVetoer;
use archipel_ide_runner::{RunConfiguration, RunTarget, TaskKind, TaskPlan, TaskState};
use archipel_ide_settings::SettingValue;
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::time::Duration;

fn project(name: &str, files: &[(&str, &str)]) -> (PathBuf, IdeState) {
    let root = std::env::temp_dir().join(format!("archipel-save-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    for (file, text) in files {
        if let Some(parent) = Path::new(file).parent() {
            std::fs::create_dir_all(root.join(parent)).unwrap();
        }
        std::fs::write(root.join(file), text).unwrap();
    }
    let mut state = IdeState::default();
    state.open_project(&root).unwrap();
    for (file, _) in files {
        state.open_file(Path::new(file)).unwrap();
    }
    (root.canonicalize().unwrap(), state)
}

fn disk(root: &Path, file: &str) -> String {
    std::fs::read_to_string(root.join(file)).unwrap()
}

fn set(state: &IdeState, id: &str, value: SettingValue) {
    state.settings().write().set(id, value).unwrap();
}

/// Appends `text` to the document of `file` (a plain edit; it saves nothing).
fn append(state: &mut IdeState, file: &str, text: &str) {
    let index = state
        .documents
        .iter()
        .position(|d| d.path == Path::new(file))
        .unwrap();
    let end = state.documents[index].text.len_chars();
    state.documents[index].text.edit(end, end, text).unwrap();
}

fn doc<'a>(state: &'a IdeState, file: &str) -> &'a Document {
    state
        .documents
        .iter()
        .find(|d| d.path == Path::new(file))
        .unwrap()
}

fn cleanup(root: PathBuf) {
    let _ = std::fs::remove_dir_all(root);
}

fn make_read_only(path: &Path, read_only: bool) {
    let mut permissions = std::fs::metadata(path).unwrap().permissions();
    permissions.set_readonly(read_only);
    std::fs::set_permissions(path, permissions).unwrap();
}

/// PLAT-SAVE-001: the queries, `save_all`, `save_one` and `save_as_is`.
#[test]
fn plat_save_001_queries_and_save_as_is_keep_trailing_spaces() {
    let (root, mut state) = project("001", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    assert!(state.unsaved_documents().is_empty());
    append(&mut state, "a.txt", "x  \n");
    assert_eq!(state.unsaved_documents(), vec![PathBuf::from("a.txt")]);
    assert!(state.is_unsaved(Path::new("a.txt")));
    assert!(state.is_file_modified(Path::new("a.txt")));
    assert!(!state.is_unsaved(Path::new("b.txt")));
    let outcome = state.save_as_is(Path::new("a.txt"));
    assert_eq!(outcome.written, vec![PathBuf::from("a.txt")]);
    assert_eq!(disk(&root, "a.txt"), "a\nx  \n", "spaces stay in the file");
    assert_eq!(
        doc(&state, "a.txt").text.text(),
        "a\nx  \n",
        "and in the document"
    );
    assert!(!state.is_unsaved(Path::new("a.txt")));
    assert!(!state.is_file_modified(Path::new("a.txt")));
    // save_one and save_filtered.
    append(&mut state, "a.txt", "1\n");
    append(&mut state, "b.txt", "2\n");
    let outcome = state.save_filtered(&|path| path == Path::new("b.txt"));
    assert_eq!(outcome.written, vec![PathBuf::from("b.txt")]);
    assert_eq!(disk(&root, "a.txt"), "a\nx  \n");
    assert!(state.save_one(Path::new("a.txt"), true).failed.is_empty());
    assert_eq!(disk(&root, "a.txt"), "a\nx  \n1\n");
    cleanup(root);
}

#[derive(Default)]
struct Recorder {
    events: Mutex<Vec<String>>,
}

impl DocumentSaveListener for Recorder {
    fn before_all_documents_saving(&self) {
        self.events.lock().push("before_all".into());
    }
    fn before_any_document_saving(&self, path: &Path, explicit: bool) {
        self.events
            .lock()
            .push(format!("before_any {} {explicit}", path.display()));
    }
    fn before_document_saving(&self, document: &mut Document) {
        self.events
            .lock()
            .push(format!("before {}", document.path.display()));
    }
    fn after_document_saved(&self, path: &Path) {
        self.events.lock().push(format!("after {}", path.display()));
    }
    fn unsaved_document_dropped(&self, path: &Path) {
        self.events
            .lock()
            .push(format!("dropped {}", path.display()));
    }
}

struct Signing;

impl DocumentSaveListener for Signing {
    fn before_document_saving(&self, document: &mut Document) {
        let end = document.text.len_chars();
        document.text.edit(end, end, "// signed\n").unwrap();
    }
}

/// PLAT-SAVE-002: listener events in order; an editing listener's change is written.
#[test]
fn plat_save_002_listener_sees_the_sequence_and_may_edit() {
    let (root, mut state) = project("002", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    let recorder = Arc::new(Recorder::default());
    state.saving().add_listener(recorder.clone());
    append(&mut state, "a.txt", "1\n");
    append(&mut state, "b.txt", "2\n");
    state.save_all().unwrap();
    assert_eq!(
        *recorder.events.lock(),
        [
            "before_all",
            "before a.txt",
            "after a.txt",
            "before b.txt",
            "after b.txt"
        ]
    );
    // A single save fires before_any first, even when the document turns out unchanged.
    recorder.events.lock().clear();
    state.save_one(Path::new("a.txt"), true);
    assert_eq!(*recorder.events.lock(), ["before_any a.txt true"]);
    // The editing listener's change is part of what is written.
    state.saving().add_listener(Arc::new(Signing));
    append(&mut state, "a.txt", "3\n");
    state.save_all().unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\n1\n3\n// signed\n");
    cleanup(root);
}

/// PLAT-SAVE-003: a non-physical document is never unsaved, unless it opted in.
#[test]
fn plat_save_003_virtual_documents_are_not_tracked() {
    let (root, mut state) = project("003", &[("a.txt", "a\n")]);
    state.documents[0].backing = DocumentBacking::Virtual;
    append(&mut state, "a.txt", "edit\n");
    assert!(!state.is_unsaved(Path::new("a.txt")));
    assert!(state.unsaved_documents().is_empty());
    state.save_all().unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\n", "saving a fragment is a no-op");
    state.documents[0].backing = DocumentBacking::TrackedVirtual;
    assert!(state.is_unsaved(Path::new("a.txt")));
    state.save_all().unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\nedit\n");
    cleanup(root);
}

fn wait_until_finished(state: &mut IdeState) {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while std::time::Instant::now() < deadline {
        state.poll_runner();
        if !state.execution().is_running() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the task did not finish");
}

fn copy_config(name: &str) -> RunConfiguration {
    RunConfiguration {
        name: name.into(),
        target: RunTarget::Shell {
            program: "sh".into(),
            arguments: vec!["-c".into(), "cp src.txt out.txt".into()],
        },
        working_directory: None,
        env: Default::default(),
        temporary: true,
        open_tool_window: true,
    }
}

/// PLAT-SAVE-030, 004, 113: Run after an edit runs the edited content, with both autosave
/// settings off.
#[cfg(unix)]
#[test]
fn plat_save_030_004_113_run_after_an_edit_runs_the_edited_content() {
    let (root, mut state) = project("030", &[("src.txt", "old\n")]);
    set(&state, ids::AUTOSAVE_FOCUS, SettingValue::Bool(false));
    set(&state, ids::AUTOSAVE_IDLE, SettingValue::Bool(false));
    append(&mut state, "src.txt", "edited\n");
    assert_eq!(disk(&root, "src.txt"), "old\n");
    state.run_temporary(copy_config("copy")).unwrap();
    wait_until_finished(&mut state);
    assert_eq!(disk(&root, "out.txt"), "old\nedited\n");
    assert!(matches!(
        state.execution().runner().state(),
        TaskState::Exited(0)
    ));
    cleanup(root);
}

/// PLAT-SAVE-031, 033: a test task and a build task save first.
#[cfg(unix)]
#[test]
fn plat_save_031_033_test_and_build_tasks_save_first() {
    for (name, kind) in [("031", TaskKind::Test), ("033", TaskKind::Build)] {
        let (root, mut state) = project(name, &[("src.txt", "old\n")]);
        append(&mut state, "src.txt", "edited\n");
        let plan = TaskPlan::new("job", "sh", ["-c", "cp src.txt out.txt"], &root).with_kind(kind);
        state.start_task(Ok(plan), "job", kind).unwrap();
        wait_until_finished(&mut state);
        assert_eq!(disk(&root, "out.txt"), "old\nedited\n", "{kind:?}");
        cleanup(root);
    }
}

/// PLAT-SAVE-032: the before-launch build saves non-explicitly, so a document with an open
/// completion popup is skipped; the others are written.
#[cfg(unix)]
#[test]
fn plat_save_032_before_launch_build_is_non_explicit() {
    let (root, mut state) = project("032", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    append(&mut state, "a.txt", "1\n");
    append(&mut state, "b.txt", "2\n");
    open_lookup(&mut state, "a.txt");
    let plan = TaskPlan::new("job", "sh", ["-c", "true"], &root).with_kind(TaskKind::Build);
    state
        .start_task_at(
            Ok(plan),
            "Build",
            TaskKind::Build,
            SavePoint::BeforeLaunchBuild,
        )
        .unwrap();
    wait_until_finished(&mut state);
    assert_eq!(disk(&root, "a.txt"), "a\n", "lookup open: skipped");
    assert_eq!(disk(&root, "b.txt"), "b\n2\n");
    cleanup(root);
}

/// PLAT-SAVE-034: reloading the external build model saves first.
#[test]
fn plat_save_034_reload_saves_before_reading_the_build_files() {
    let (root, mut state) = project("034", &[("notes.txt", "n\n")]);
    append(&mut state, "notes.txt", "edit\n");
    // No build system in the project: the import refuses, after the save.
    let _ = state.start_external_import(crate::project_model::ImportReason::Refresh);
    assert_eq!(disk(&root, "notes.txt"), "n\nedit\n");
    cleanup(root);
}

/// PLAT-SAVE-035: the external tool save point saves everything first.
#[test]
fn plat_save_035_external_tool_save_point_saves_all() {
    let (root, mut state) = project("035", &[("a.txt", "a\n")]);
    append(&mut state, "a.txt", "tool\n");
    state.save_before(SavePoint::ExternalTool);
    assert_eq!(disk(&root, "a.txt"), "a\ntool\n");
    cleanup(root);
}

/// PLAT-SAVE-020: deactivation saves everything, unless the option is off or a modal dialog is
/// open.
#[test]
fn plat_save_020_deactivation_saves_all_unless_modal_or_off() {
    let (root, mut state) = project("020", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    append(&mut state, "a.txt", "1\n");
    append(&mut state, "b.txt", "2\n");
    assert!(
        state.autosave_on_deactivation(true).is_none(),
        "modal dialog open"
    );
    assert_eq!(disk(&root, "a.txt"), "a\n");
    set(&state, ids::AUTOSAVE_FOCUS, SettingValue::Bool(false));
    assert!(state.autosave_on_deactivation(false).is_none());
    assert_eq!(disk(&root, "a.txt"), "a\n");
    set(&state, ids::AUTOSAVE_FOCUS, SettingValue::Bool(true));
    let outcome = state.autosave_on_deactivation(false).unwrap();
    assert_eq!(outcome.written.len(), 2);
    assert_eq!(disk(&root, "a.txt"), "a\n1\n");
    assert_eq!(disk(&root, "b.txt"), "b\n2\n");
    cleanup(root);
}

/// PLAT-SAVE-021, 015: the idle save fires after N quiet seconds, not while typing, once per
/// quiet period; a changed timeout re-arms the timer.
#[test]
fn plat_save_021_015_idle_save_fires_after_quiet_time_and_applies_live() {
    let (root, mut state) = project("021", &[("a.txt", "a\n")]);
    let t0 = Instant::now();
    let at = |seconds: f64| t0 + Duration::from_secs_f64(seconds);
    // Off by default: nothing happens however long it is quiet.
    append(&mut state, "a.txt", "1\n");
    state.saving().note_input(t0);
    assert!(state.autosave_idle_tick(at(500.0), false).is_none());
    // Enable with 2 seconds (applies at once, no restart).
    set(&state, ids::AUTOSAVE_IDLE, SettingValue::Bool(true));
    set(&state, ids::AUTOSAVE_IDLE_SECONDS, SettingValue::Int(2));
    state.saving().note_input(at(1000.0));
    assert!(state.autosave_idle_tick(at(1001.0), false).is_none());
    // Typing every 1.5 s never saves.
    for step in 1..6 {
        state.saving().note_input(at(1000.0 + 1.5 * step as f64));
        assert!(
            state
                .autosave_idle_tick(at(1000.0 + 1.5 * step as f64 + 1.4), false)
                .is_none()
        );
    }
    assert_eq!(disk(&root, "a.txt"), "a\n");
    // Stop for 2 seconds: saved once.
    let quiet_since = 1000.0 + 1.5 * 5.0;
    assert!(
        state
            .autosave_idle_tick(at(quiet_since + 1.9), false)
            .is_none()
    );
    let outcome = state
        .autosave_idle_tick(at(quiet_since + 2.0), false)
        .unwrap();
    assert_eq!(outcome.written, vec![PathBuf::from("a.txt")]);
    assert_eq!(disk(&root, "a.txt"), "a\n1\n");
    // Once per quiet period; a no-op with nothing unsaved.
    assert!(
        state
            .autosave_idle_tick(at(quiet_since + 30.0), false)
            .is_none()
    );
    // The next input starts a new period.
    append(&mut state, "a.txt", "2\n");
    state.saving().note_input(at(quiet_since + 40.0));
    assert!(
        state
            .autosave_idle_tick(at(quiet_since + 41.0), false)
            .is_none()
    );
    // Changing the timeout re-arms with the new value: 60 s, no save at 2 s.
    set(&state, ids::AUTOSAVE_IDLE_SECONDS, SettingValue::Int(60));
    assert!(
        state
            .autosave_idle_tick(at(quiet_since + 42.5), false)
            .is_none()
    );
    assert!(
        state
            .autosave_idle_tick(at(quiet_since + 90.0), false)
            .is_none()
    );
    assert!(
        state
            .autosave_idle_tick(at(quiet_since + 101.0), true)
            .is_none(),
        "modal"
    );
    assert!(
        state
            .autosave_idle_tick(at(quiet_since + 101.0), false)
            .is_some()
    );
    assert_eq!(disk(&root, "a.txt"), "a\n1\n2\n");
    // Switching it off cancels.
    append(&mut state, "a.txt", "3\n");
    state.saving().note_input(at(quiet_since + 200.0));
    set(&state, ids::AUTOSAVE_IDLE, SettingValue::Bool(false));
    assert!(
        state
            .autosave_idle_tick(at(quiet_since + 900.0), false)
            .is_none()
    );
    cleanup(root);
}

/// PLAT-SAVE-010: the stored timeout is clamped into 1 to 300; defaults of 010 to 012.
#[test]
fn plat_save_010_011_012_defaults_and_clamping() {
    let (root, state) = project("010", &[("a.txt", "a\n")]);
    {
        let settings = state.settings();
        let settings = settings.read();
        assert_eq!(settings.bool(ids::AUTOSAVE_IDLE), Some(false));
        assert_eq!(settings.int(ids::AUTOSAVE_IDLE_SECONDS), Some(15));
        assert_eq!(settings.bool(ids::AUTOSAVE_FOCUS), Some(true));
        assert_eq!(settings.bool(ids::SAFE_WRITE), Some(true));
        let focus = settings.registry.setting(ids::AUTOSAVE_FOCUS).unwrap();
        assert_eq!(
            focus.label,
            "Save files when switching to a different application or a built-in terminal"
        );
    }
    set(&state, ids::AUTOSAVE_IDLE_SECONDS, SettingValue::Int(999));
    assert_eq!(state.autosave_idle_seconds(), 300);
    set(&state, ids::AUTOSAVE_IDLE_SECONDS, SettingValue::Int(0));
    assert_eq!(state.autosave_idle_seconds(), 1);
    cleanup(root);
}

/// PLAT-SAVE-052, 053: closing a tab does not save and does not prompt; the text is written by
/// the next trigger; typing does not save.
#[test]
fn plat_save_052_053_tab_close_does_not_save_and_typing_does_not_save() {
    let (root, mut state) = project("052", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    set(&state, ids::AUTOSAVE_IDLE, SettingValue::Bool(true));
    state.saving().note_input(Instant::now());
    append(&mut state, "a.txt", "typed\n");
    // 053: stay in the IDE, wait less than the idle timeout: the disk is unchanged.
    assert!(
        state
            .autosave_idle_tick(Instant::now() + Duration::from_secs(5), false)
            .is_none()
    );
    assert_eq!(disk(&root, "a.txt"), "a\n");
    // 052: close the tab.
    let index = state
        .documents
        .iter()
        .position(|d| d.path == Path::new("a.txt"))
        .unwrap();
    state.close_document(index).unwrap();
    assert_eq!(state.documents.len(), 1);
    assert_eq!(disk(&root, "a.txt"), "a\n", "closing does not save");
    assert_eq!(state.unsaved_documents(), vec![PathBuf::from("a.txt")]);
    // Reopening brings the pending text back.
    state.open_file(Path::new("a.txt")).unwrap();
    assert_eq!(doc(&state, "a.txt").text.text(), "a\ntyped\n");
    assert!(state.is_unsaved(Path::new("a.txt")));
    let index = state
        .documents
        .iter()
        .position(|d| d.path == Path::new("a.txt"))
        .unwrap();
    state.close_document(index).unwrap();
    // The next trigger (deactivation) writes the closed tab's text.
    state.autosave_on_deactivation(false).unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\ntyped\n");
    assert!(state.unsaved_documents().is_empty());
    cleanup(root);
}

/// PLAT-SAVE-071, 075: undo to the original and retyped text write nothing.
#[test]
fn plat_save_071_075_nothing_is_written_when_the_text_equals_the_disk() {
    let (root, mut state) = project("071", &[("a.txt", "a\n")]);
    let recorder = Arc::new(Recorder::default());
    state.saving().add_listener(recorder.clone());
    let modified = |root: &Path| {
        std::fs::metadata(root.join("a.txt"))
            .unwrap()
            .modified()
            .unwrap()
    };
    let before = modified(&root);
    // Edit, undo to the original: the document is not dirty, no write.
    append(&mut state, "a.txt", "x");
    assert!(state.documents[0].text.undo());
    assert!(!state.is_unsaved(Path::new("a.txt")));
    assert!(state.save_all_at(SavePoint::SaveAll).written.is_empty());
    // Edit, then restore the text by typing: still dirty, but equal to the disk.
    append(&mut state, "a.txt", "x");
    let end = state.documents[0].text.len_chars();
    state.documents[0].text.edit(end - 1, end, "").unwrap();
    assert!(state.documents[0].text.is_dirty());
    std::thread::sleep(Duration::from_millis(20));
    let outcome = state.save_all_at(SavePoint::SaveAll);
    assert!(outcome.written.is_empty());
    assert_eq!(outcome.dropped, vec![PathBuf::from("a.txt")]);
    assert!(!state.documents[0].text.is_dirty(), "the marker clears");
    assert_eq!(modified(&root), before, "no write happened");
    assert!(
        recorder
            .events
            .lock()
            .iter()
            .all(|e| !e.starts_with("before a.txt")),
        "{:?}",
        recorder.events.lock()
    );
    cleanup(root);
}

/// PLAT-SAVE-072, 079: a file changed on disk is refreshed before the save; no silent overwrite;
/// keeping the memory text makes the next save overwrite, loading the disk text reloads.
#[test]
fn plat_save_072_079_changed_on_disk_is_never_overwritten_silently() {
    let (root, mut state) = project("072", &[("a.txt", "a\n")]);
    append(&mut state, "a.txt", "mine\n");
    // No watcher event reached the editor: the save itself refreshes first.
    std::fs::write(root.join("a.txt"), "theirs\n").unwrap();
    let outcome = state.save_all_at(SavePoint::Deactivation);
    assert_eq!(outcome.conflicts, vec![PathBuf::from("a.txt")]);
    assert_eq!(disk(&root, "a.txt"), "theirs\n");
    assert!(
        state.is_unsaved(Path::new("a.txt")),
        "the text stays unsaved"
    );
    assert!(outcome.into_result().is_err());
    // "Keep Memory Changes", then the next save overwrites the disk content.
    state.keep_memory_version(0).unwrap();
    state.save_all().unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\nmine\n");
    // "Load File System Changes": the document is reloaded and leaves the unsaved set.
    append(&mut state, "a.txt", "again\n");
    std::fs::write(root.join("a.txt"), "other\n").unwrap();
    assert!(state.save_all().is_err());
    state.reload_from_disk(0).unwrap();
    assert_eq!(doc(&state, "a.txt").text.text(), "other\n");
    assert!(state.unsaved_documents().is_empty());
    cleanup(root);
}

struct Veto(PathBuf);

impl SavingVetoer for Veto {
    fn may_save(&self, path: &Path, _explicit: bool) -> bool {
        path != self.0
    }
}

struct Throwing;

impl SavingVetoer for Throwing {
    fn may_save(&self, _path: &Path, _explicit: bool) -> bool {
        panic!("a misbehaving extension");
    }
}

fn open_lookup(state: &mut IdeState, file: &str) {
    state.completion = Some(CompletionSession {
        path: PathBuf::from(file),
        anchor: 0,
        request_caret: 0,
        items: Vec::new(),
        ranked: Vec::new(),
        selected: 0,
        prefix: String::new(),
        incomplete: false,
        waiting: false,
        explicit: false,
        resolved: Default::default(),
        indexing: None,
        requery_when_ready: false,
    });
}

/// PLAT-SAVE-073: a lookup vetoes non-explicit saves only; extension vetoes leave the document
/// unsaved without an error; a throwing vetoer counts as no veto.
#[test]
fn plat_save_073_vetoes_leave_the_document_unsaved_without_an_error() {
    let (root, mut state) = project("073", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    append(&mut state, "a.txt", "1\n");
    open_lookup(&mut state, "a.txt");
    let outcome = state.save_all_at(SavePoint::Deactivation);
    assert_eq!(outcome.vetoed, vec![PathBuf::from("a.txt")]);
    assert!(outcome.is_success() && outcome.clone().into_result().is_ok());
    assert_eq!(disk(&root, "a.txt"), "a\n", "document skipped");
    assert!(!state.saving().has_failure(), "no dialog");
    state.save_all().unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\n1\n", "Save All writes it");
    // Extension vetoes.
    state.saving().add_vetoer(Arc::new(Throwing));
    state
        .saving()
        .add_vetoer(Arc::new(Veto(PathBuf::from("b.txt"))));
    append(&mut state, "b.txt", "2\n");
    let outcome = state.save_all_at(SavePoint::SaveAll);
    assert_eq!(outcome.vetoed, vec![PathBuf::from("b.txt")]);
    assert_eq!(disk(&root, "b.txt"), "b\n");
    cleanup(root);
}

/// PLAT-SAVE-070, 077: one failing, one vetoed, one good document: the good one is saved, the
/// others stay unsaved, one failure report; "Revert Changes" reloads the failed document.
#[test]
fn plat_save_070_077_078_trace_with_failing_vetoed_and_good_documents() {
    let (root, mut state) = project(
        "070",
        &[("good.txt", "g\n"), ("veto.txt", "v\n"), ("ro.txt", "r\n")],
    );
    state
        .saving()
        .add_vetoer(Arc::new(Veto(PathBuf::from("veto.txt"))));
    make_read_only(&root.join("ro.txt"), true);
    // Trailing spaces on a read-only document are not stripped (078).
    append(&mut state, "ro.txt", "kept  \n");
    append(&mut state, "good.txt", "1\n");
    append(&mut state, "veto.txt", "2\n");
    let outcome = state.save_all_at(SavePoint::Deactivation);
    assert_eq!(outcome.written, vec![PathBuf::from("good.txt")]);
    assert_eq!(outcome.vetoed, vec![PathBuf::from("veto.txt")]);
    assert_eq!(outcome.failed.len(), 1);
    assert!(
        outcome.failed[0].1.contains("read-only"),
        "{:?}",
        outcome.failed
    );
    assert_eq!(disk(&root, "good.txt"), "g\n1\n");
    assert_eq!(disk(&root, "veto.txt"), "v\n");
    assert_eq!(disk(&root, "ro.txt"), "r\n");
    assert_eq!(
        doc(&state, "ro.txt").text.text(),
        "r\nkept  \n",
        "no strip when read-only"
    );
    assert!(state.is_unsaved(Path::new("ro.txt")));
    assert!(state.is_unsaved(Path::new("veto.txt")));
    // One dialog for the call.
    let report = state.saving().take_failure().unwrap();
    assert_eq!(report.paths, vec![PathBuf::from("ro.txt")]);
    assert!(!report.closing);
    assert!(state.saving().take_failure().is_none());
    assert_eq!(
        (texts::FAILURE_TITLE, texts::FAILURE_MESSAGE),
        (
            "Cannot Save Files",
            "Following errors occurred on attempt to save files:"
        )
    );
    // "Close" keeps the text unsaved, a later trigger retries and reports again.
    let again = state.save_all_at(SavePoint::Idle);
    assert_eq!(again.failed.len(), 1);
    assert!(state.saving().take_failure().is_some());
    // "Revert Changes" restores the disk text.
    state.revert_documents(&report.paths);
    assert_eq!(doc(&state, "ro.txt").text.text(), "r\n");
    assert!(!state.is_unsaved(Path::new("ro.txt")));
    make_read_only(&root.join("ro.txt"), false);
    cleanup(root);
}

/// PLAT-SAVE-076, 090: a CRLF file keeps its separators and only modified lines lose their
/// trailing spaces; undo is not involved.
#[test]
fn plat_save_076_090_crlf_is_kept_and_modified_lines_are_stripped() {
    let (root, mut state) = project("076", &[("a.txt", "one  \r\ntwo\r\n")]);
    // Line 0 keeps its old trailing spaces (not modified); line 1 gets new ones.
    let end = state.documents[0].text.len_chars();
    state.documents[0]
        .text
        .edit(end - 2, end - 2, "  ")
        .unwrap();
    // Autosave on deactivation strips like Save All does.
    state.autosave_on_deactivation(false).unwrap();
    assert_eq!(disk(&root, "a.txt"), "one  \r\ntwo\r\n");
    assert_eq!(doc(&state, "a.txt").text.text(), "one  \r\ntwo\r\n");
    // "All lines" strips line 0 as well.
    set(
        &state,
        ids::STRIP_TRAILING_SPACES,
        SettingValue::Text("all".into()),
    );
    append(&mut state, "a.txt", "three\r\n");
    state.save_all().unwrap();
    assert_eq!(disk(&root, "a.txt"), "one\r\ntwo\r\nthree\r\n");
    // "None" leaves everything.
    set(
        &state,
        ids::STRIP_TRAILING_SPACES,
        SettingValue::Text("none".into()),
    );
    append(&mut state, "a.txt", "x  \r\n");
    state.save_all().unwrap();
    assert_eq!(disk(&root, "a.txt"), "one\r\ntwo\r\nthree\r\nx  \r\n");
    cleanup(root);
}

/// PLAT-SAVE-090: the strip runs on every save path and is skipped for `save_as_is`; the
/// final newline is added with the file's separator.
#[test]
fn plat_save_090_final_newline_and_every_path() {
    let (root, mut state) = project("090", &[("a.txt", "a\r\n"), ("b.txt", "b\n")]);
    set(&state, ids::ENSURE_NEWLINE, SettingValue::Bool(true));
    append(&mut state, "a.txt", "tail");
    append(&mut state, "b.txt", "tail  ");
    state.save_before(SavePoint::Run);
    assert_eq!(disk(&root, "a.txt"), "a\r\ntail\r\n");
    assert_eq!(disk(&root, "b.txt"), "b\ntail\n");
    // The edits are ordinary document changes: document and file agree.
    assert_eq!(doc(&state, "b.txt").text.text(), "b\ntail\n");
    assert!(!state.is_unsaved(Path::new("b.txt")));
    cleanup(root);
}

/// PLAT-SAVE-091, 092: trailing spaces on the caret line of the focused editor stay and are
/// stripped by a later save once the caret left; without a focused editor they are stripped.
#[test]
fn plat_save_091_092_caret_line_keeps_trailing_spaces_until_the_caret_leaves() {
    let (root, mut state) = project("091", &[("a.txt", "a\nb\n")]);
    let after_b = 3;
    state.documents[0].text.edit(after_b, after_b, " ").unwrap();
    state.documents[0]
        .selections
        .set_single(Selection::caret(after_b + 1));
    state
        .saving()
        .set_focused_document(Some(Path::new("a.txt")));
    state.save_all().unwrap();
    assert_eq!(
        disk(&root, "a.txt"),
        "a\nb \n",
        "the space stays under the caret"
    );
    assert_eq!(doc(&state, "a.txt").text.text(), "a\nb \n");
    // The line stays flagged: another edit elsewhere, caret still there.
    append(&mut state, "a.txt", "c\n");
    state.save_all().unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\nb \nc\n");
    // The caret moves away: the next save strips the line once.
    state.documents[0]
        .selections
        .set_single(Selection::caret(0));
    append(&mut state, "a.txt", "d\n");
    state.save_all().unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\nb\nc\nd\n");
    // PLAT-SAVE-092: no focused editor (deactivated window): nothing to keep.
    let end = state.documents[0].text.len_chars();
    state.documents[0].text.edit(end, end, "e \n").unwrap();
    state.documents[0]
        .selections
        .set_single(Selection::caret(end + 2));
    state.saving().set_focused_document(None);
    state.autosave_on_deactivation(false).unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\nb\nc\nd\ne\n");
    // With "keep on caret line" off, even the focused editor's caret line is stripped (093).
    set(
        &state,
        ids::KEEP_TRAILING_ON_CARET,
        SettingValue::Bool(false),
    );
    let end = state.documents[0].text.len_chars();
    state.documents[0].text.edit(end, end, "f \n").unwrap();
    state.documents[0]
        .selections
        .set_single(Selection::caret(end + 2));
    state
        .saving()
        .set_focused_document(Some(Path::new("a.txt")));
    state.save_all().unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\nb\nc\nd\ne\nf\n");
    assert_eq!(
        doc(&state, "a.txt").selections.primary().head,
        end + 1,
        "the caret follows the stripped text"
    );
    cleanup(root);
}

/// PLAT-SAVE-074: the closed tab of a file deleted since is dropped, not recreated.
#[test]
fn plat_save_074_a_deleted_file_of_a_closed_tab_is_dropped() {
    let (root, mut state) = project("074", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    append(&mut state, "a.txt", "1\n");
    state.close_document(0).unwrap();
    std::fs::remove_file(root.join("a.txt")).unwrap();
    let outcome = state.save_all_at(SavePoint::Exit);
    assert_eq!(outcome.dropped, vec![PathBuf::from("a.txt")]);
    assert!(!root.join("a.txt").exists());
    assert!(state.unsaved_documents().is_empty());
    cleanup(root);
}

/// PLAT-SAVE-072: a closed tab whose file changed meanwhile comes back with a conflict.
#[test]
fn plat_save_072_a_closed_tab_with_an_outside_change_comes_back_as_a_conflict() {
    let (root, mut state) = project("072b", &[("a.txt", "a\n")]);
    append(&mut state, "a.txt", "1\n");
    state.close_document(0).unwrap();
    std::fs::write(root.join("a.txt"), "theirs\n").unwrap();
    let outcome = state.save_all_at(SavePoint::Deactivation);
    assert_eq!(outcome.conflicts, vec![PathBuf::from("a.txt")]);
    assert_eq!(disk(&root, "a.txt"), "theirs\n");
    assert_eq!(state.documents.len(), 1, "the tab is back");
    assert_eq!(state.documents[0].conflict, Some(DiskConflict::Modified));
    cleanup(root);
}

/// PLAT-SAVE-050: a document that cannot be saved cancels the project close; the dialog says
/// "Ignore Changes".
#[test]
fn plat_save_050_a_failing_save_cancels_the_project_close() {
    let (root, mut state) = project("050", &[("a.txt", "a\n")]);
    append(&mut state, "a.txt", "1\n");
    make_read_only(&root.join("a.txt"), true);
    assert!(state.close_project().is_err());
    assert!(state.project.is_some(), "the project stays open");
    assert!(
        state.is_unsaved(Path::new("a.txt")),
        "the document stays unsaved"
    );
    let report = state.saving().take_failure().unwrap();
    assert!(report.closing, "confirm button reads Ignore Changes");
    make_read_only(&root.join("a.txt"), false);
    state.close_project().unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\n1\n");
    cleanup(root);
}

/// PLAT-SAVE-051: exit saves open and closed documents.
#[test]
fn plat_save_051_exit_saves_everything() {
    let (root, mut state) = project("051", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    append(&mut state, "a.txt", "1\n");
    append(&mut state, "b.txt", "2\n");
    state.close_document(0).unwrap();
    let outcome = state.save_all_at(SavePoint::Exit);
    assert_eq!(outcome.written.len(), 2);
    assert_eq!(disk(&root, "a.txt"), "a\n1\n");
    assert_eq!(disk(&root, "b.txt"), "b\n2\n");
    cleanup(root);
}

fn git(root: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

/// PLAT-SAVE-040, 041: commit and Git working-tree operations save first.
#[test]
fn plat_save_040_041_commit_and_vcs_operations_save_first() {
    let (root, mut state) = project("040", &[("a.txt", "a\n")]);
    git(&root, &["init", "-q", "-b", "main"]);
    // The IDE's own commit runs git without `-c`, so the repository needs an identity.
    git(&root, &["config", "user.name", "t"]);
    git(&root, &["config", "user.email", "t@example.invalid"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-q", "-m", "init"]);
    state.refresh_git().unwrap();
    append(&mut state, "a.txt", "committed\n");
    state
        .commit_paths("edit", &[PathBuf::from("a.txt")])
        .unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\ncommitted\n");
    let shown = std::process::Command::new("git")
        .args(["show", "HEAD:a.txt"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&shown.stdout), "a\ncommitted\n");
    // A working-tree operation (the path of checkout, merge, stash, rollback).
    append(&mut state, "a.txt", "dirty\n");
    state.run_git_operation(|_| Ok("done".into())).unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\ncommitted\ndirty\n");
    // A change-list refresh.
    append(&mut state, "a.txt", "refresh\n");
    built_in_commands()
        .execute("git.refresh", &mut state, Value::Null)
        .unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\ncommitted\ndirty\nrefresh\n");
    cleanup(root);
}

/// PLAT-SAVE-094 (VFS): a save is not an outside change; the stamps agree afterwards and a
/// synchronize neither reloads nor flags a conflict.
#[test]
fn plat_save_076_own_save_is_not_reloaded_as_an_outside_change() {
    let (root, mut state) = project("094", &[("a.txt", "a\n")]);
    append(&mut state, "a.txt", "1\n");
    let revision = state.documents[0].text.revision();
    state.save_all().unwrap();
    state.synchronize_all();
    assert_eq!(state.documents[0].text.revision(), revision);
    assert_eq!(state.documents[0].conflict, None);
    assert!(!state.is_file_modified(Path::new("a.txt")));
    cleanup(root);
}

/// PLAT-SAVE-111: saving changes no text unless a cleanup applies, so analysis (which reads the
/// document, PLAT-SAVE-110) has nothing to redo.
#[test]
fn plat_save_110_111_save_does_not_touch_the_document_text() {
    let (root, mut state) = project("111", &[("A.java", "class A {}\n")]);
    append(&mut state, "A.java", "// edit\n");
    let analysis_text = state.documents[0].text.text();
    let revision = state.documents[0].text.revision();
    state.save_all().unwrap();
    assert_eq!(state.documents[0].text.revision(), revision);
    assert_eq!(state.documents[0].text.text(), analysis_text);
    cleanup(root);
}

/// The Save All command is explicit and reports a conflict instead of overwriting.
#[test]
fn plat_save_024_save_all_command_saves_everything() {
    let (root, mut state) = project("024", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    set(&state, ids::AUTOSAVE_FOCUS, SettingValue::Bool(false));
    append(&mut state, "a.txt", "1\n");
    append(&mut state, "b.txt", "2\n");
    built_in_commands()
        .execute("editor.saveAll", &mut state, json!({}))
        .unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\n1\n");
    assert_eq!(disk(&root, "b.txt"), "b\n2\n");
    cleanup(root);
}

/// PLAT-SAVE-004: with both autosave options off, the save points still save and Save All works.
#[test]
fn plat_save_004_autosave_cannot_be_disabled_completely() {
    let (root, mut state) = project("004", &[("a.txt", "a\n")]);
    set(&state, ids::AUTOSAVE_FOCUS, SettingValue::Bool(false));
    set(&state, ids::AUTOSAVE_IDLE, SettingValue::Bool(false));
    append(&mut state, "a.txt", "1\n");
    assert!(state.autosave_on_deactivation(false).is_none());
    state.save_before(SavePoint::Run);
    assert_eq!(disk(&root, "a.txt"), "a\n1\n");
    append(&mut state, "a.txt", "2\n");
    state.save_all().unwrap();
    assert_eq!(disk(&root, "a.txt"), "a\n1\n2\n");
    cleanup(root);
}

/// PLAT-SAVE-053: editing, undo and redo, and moving focus inside the IDE do not save.
#[test]
fn plat_save_053_editing_features_do_not_save() {
    let (root, mut state) = project("053", &[("a.txt", "a\n")]);
    set(&state, ids::AUTOSAVE_IDLE, SettingValue::Bool(true));
    append(&mut state, "a.txt", "typed\n");
    state.documents[0].text.undo();
    state.documents[0].text.redo();
    state.saving().set_focused_document(None);
    state
        .saving()
        .set_focused_document(Some(Path::new("a.txt")));
    assert_eq!(disk(&root, "a.txt"), "a\n");
    cleanup(root);
}

/// PLAT-SAVE-113: a run with both autosave options off reads the edited disk content.
#[cfg(unix)]
#[test]
fn plat_save_113_save_points_do_not_rely_on_autosave() {
    let (root, mut state) = project("113", &[("src.txt", "old\n")]);
    set(&state, ids::AUTOSAVE_FOCUS, SettingValue::Bool(false));
    append(&mut state, "src.txt", "edited\n");
    state.run_temporary(copy_config("copy113")).unwrap();
    wait_until_finished(&mut state);
    assert_eq!(disk(&root, "out.txt"), "old\nedited\n");
    cleanup(root);
}

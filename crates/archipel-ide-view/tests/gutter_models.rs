//! The gutter models both frontends draw (M1, M2, M4): the run icon popup, the run icons'
//! debounce, the breakpoint variants and form, the change popup.
//! Spec: EXE-CTX-050..053, TUI-GUI-090, TUI-GUI-091, TUI-GUI-140.

mod common;

use archipel_ide_view::Caps;
use archipel_ide_view::dispatch::Effect;
use archipel_ide_view::popups::breakpoint::{self, BreakpointForm};
use archipel_ide_view::popups::change::{self, Tool};
use archipel_ide_view::popups::gutter::{self, BreakpointKind, RunMarks};
use archipel_kit_model::MenuEntry;
use common::Fixture;
use serde_json::json;
use std::path::PathBuf;

const MAIN: &str = "src/main/java/com/example/Main.java";

fn labels(entries: &[MenuEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| match entry {
            MenuEntry::Item(item) => item.label.clone(),
            MenuEntry::Separator { .. } => "---".into(),
        })
        .collect()
}

fn item(entries: &[MenuEntry], label: &str) -> archipel_kit_model::MenuItem {
    entries
        .iter()
        .find_map(|entry| match entry {
            MenuEntry::Item(item) if item.label == label => Some(item.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no row {label}: {:?}", labels(entries)))
}

fn main_line(fixture: &mut Fixture) -> usize {
    fixture.run("editor.openFile", json!({ "path": MAIN }));
    let marks = fixture.state.gutter_marks();
    marks
        .iter()
        .find(|mark| mark.label.contains("main"))
        .unwrap_or_else(|| panic!("no run icon on main: {marks:?}"))
        .line
}

#[test]
fn the_run_popup_lists_the_executor_actions_then_modify() {
    let mut fixture = Fixture::java("gutter-popup");
    let line = main_line(&mut fixture);
    let entries = gutter::run_popup(&fixture.cx(Caps::kitty()), line);
    let names = labels(&entries);
    assert_eq!(names.len(), 5, "{names:?}");
    assert!(names[0].starts_with("Run '"), "{names:?}");
    assert!(names[1].starts_with("Debug '"), "{names:?}");
    assert!(
        names[2].starts_with("Run '") && names[2].ends_with("with Coverage"),
        "{names:?}"
    );
    assert_eq!(names[3], "---");
    assert_eq!(names[4], "Modify Run Configuration…");
    // Coverage is disabled and says why; the others are enabled.
    let coverage = item(&entries, &names[2]);
    assert!(!coverage.enabled && coverage.disabled_reason.is_some());
    assert!(item(&entries, &names[0]).enabled && item(&entries, &names[1]).enabled);
    // Run shows the keymap's shortcut for RunClass.
    let run = item(&entries, &names[0]);
    assert_eq!(
        run.shortcut.map(|s| s.label(Caps::kitty().notation())),
        Some("Ctrl+Shift+F10".to_owned())
    );
    // A line without an icon has no popup.
    assert!(gutter::run_popup(&fixture.cx(Caps::kitty()), 0).is_empty());
}

#[test]
fn the_rows_of_the_run_popup_are_the_run_gutter_command() {
    let mut fixture = Fixture::java("gutter-rows");
    let line = main_line(&mut fixture);
    let entries = gutter::run_popup(&fixture.cx(Caps::gui()), line);
    let cx = fixture.cx(Caps::gui());
    let expect = |index: usize, want: serde_json::Value| {
        let MenuEntry::Item(row) = &entries[index] else {
            panic!("row {index} is a separator")
        };
        let effects = archipel_ide_view::dispatch::menu_row(&cx, &row.id);
        assert_eq!(
            effects,
            vec![Effect::Command {
                id: "run.gutter".into(),
                params: want
            }]
        );
    };
    expect(0, json!({"line": line, "edit": false}));
    expect(1, json!({"line": line, "edit": false, "debug": true}));
    expect(4, json!({"line": line, "edit": true}));
}

#[test]
fn run_icons_keep_the_previous_ones_while_typing_and_refresh_after_the_pause() {
    let mut fixture = Fixture::java("gutter-debounce");
    fixture.run("editor.openFile", json!({ "path": MAIN }));
    let mut marks = RunMarks::default();
    let (first, kept) = marks.marks(&mut fixture.state, 100.0);
    assert!(!kept, "the first call computes");
    assert!(!first.is_empty());
    // An edit: the icons of the old text are kept for a quarter second.
    fixture.run("editor.type", json!({"text": "x", "live": true}));
    let (_, kept) = marks.marks(&mut fixture.state, 100.1);
    assert!(kept);
    let (_, kept) = marks.marks(
        &mut fixture.state,
        100.1 + gutter::RUN_ICONS_STALE_FOR + 0.01,
    );
    assert!(!kept);
}

#[test]
fn breakpoints_show_their_kind_and_the_form_edits_the_options() {
    let mut fixture = Fixture::java("gutter-breakpoints");
    fixture.run("editor.openFile", json!({ "path": MAIN }));
    let path = PathBuf::from(MAIN);
    for line in [1, 2, 3] {
        fixture.run(
            "debug.toggleBreakpoint",
            json!({"path": path, "line": line}),
        );
    }
    let form = BreakpointForm {
        condition: "i == 3".into(),
        ..BreakpointForm::load(&fixture.state, &path, 2)
    };
    for effect in form.done() {
        let Effect::Command { id, params } = effect else {
            panic!("a command")
        };
        fixture.run(&id, params);
    }
    let log = BreakpointForm {
        log_message: "hit {i}".into(),
        ..BreakpointForm::load(&fixture.state, &path, 3)
    };
    for effect in log.done() {
        let Effect::Command { id, params } = effect else {
            panic!("a command")
        };
        fixture.run(&id, params);
    }
    let kinds: Vec<_> = gutter::breakpoint_marks(&fixture.state, &path)
        .into_iter()
        .map(|mark| (mark.line, mark.kind))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (1, BreakpointKind::Line),
            (2, BreakpointKind::Conditional),
            (3, BreakpointKind::Logpoint)
        ]
    );
    // The form loads what was stored and Remove toggles the breakpoint off with its options.
    let loaded = BreakpointForm::load(&fixture.state, &path, 2);
    assert_eq!(loaded.condition, "i == 3");
    assert_eq!(loaded.title(), "Line Breakpoint — Main.java:3");
    assert_eq!(loaded.value("condition"), "i == 3");
    for effect in loaded.remove() {
        let Effect::Command { id, params } = effect else {
            panic!("a command")
        };
        fixture.run(&id, params);
    }
    assert!(!fixture.state.breakpoint_lines(&path).contains(&2));
    assert_eq!(fixture.state.breakpoint_options(&path, 2), None);
    assert_eq!(
        breakpoint::FIELDS.map(|f| f.id),
        ["condition", "logMessage", "hitCondition"]
    );
}

#[test]
fn the_form_opens_through_the_edit_command_and_closes_again() {
    let mut fixture = Fixture::java("gutter-form");
    fixture.run("editor.openFile", json!({ "path": MAIN }));
    let path = PathBuf::from(MAIN);
    assert_eq!(breakpoint::editing(&fixture.state), None);
    for effect in breakpoint::open(&path, 2) {
        let Effect::Command { id, params } = effect else {
            panic!("a command")
        };
        fixture.run(&id, params);
    }
    assert_eq!(breakpoint::editing(&fixture.state), Some((path.clone(), 2)));
    assert!(
        fixture.state.breakpoint_lines(&path).contains(&2),
        "created"
    );
    let Effect::Command { id, params } = breakpoint::close() else {
        panic!("a command")
    };
    fixture.run(&id, params);
    assert_eq!(breakpoint::editing(&fixture.state), None);
}

fn git(root: &std::path::Path, args: &[&str]) {
    assert!(
        std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[test]
fn the_change_popup_shows_the_old_lines_and_steps_between_changes() {
    let root = std::env::temp_dir().join(format!("archipel-view-change-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "test@example.invalid"]);
    git(&root, &["config", "user.name", "Test"]);
    std::fs::write(root.join("a.txt"), "1\n2\n3\n4\n5\n6\n").unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "initial"]);
    std::fs::write(root.join("a.txt"), "1\nX\n3\n4\n6\nadded\n").unwrap();
    let mut fixture = Fixture::empty();
    fixture.run("project.open", json!({ "path": root }));
    fixture.run("editor.openFile", json!({"path": "a.txt"}));
    let cx = fixture.cx(Caps::kitty());
    let first = change::hunk_at(&cx, 1).expect("the modified line owns a hunk");
    let popup = change::change_popup(&cx, first).expect("popup");
    assert_eq!(popup.old_lines, ["2"], "the HEAD text of the changed line");
    assert_eq!(popup.more, 0);
    let flags: Vec<_> = popup.tools.iter().map(|t| (t.tool, t.enabled)).collect();
    assert_eq!(
        flags,
        vec![
            (Tool::Previous, false),
            (Tool::Next, true),
            (Tool::Rollback, true),
            // The terminal has no diff viewer yet.
            (Tool::ShowDiff, false),
            (Tool::Close, true),
        ]
    );
    assert!(
        popup
            .tool(Tool::Rollback)
            .unwrap()
            .tooltip
            .contains("Ctrl+Alt+Z"),
        "{:?}",
        popup.tool(Tool::Rollback)
    );
    // The GUI's popup can show the diff.
    let gui = change::change_popup(&fixture.cx(Caps::gui()), first).unwrap();
    assert!(gui.tool(Tool::ShowDiff).unwrap().enabled);
    // Next selects the next hunk and moves the caret to its first line.
    let step = popup.step(&cx, Tool::Next);
    assert_eq!(step.select, Some(first + 1));
    assert!(!step.close);
    assert!(matches!(&step.effects[0], Effect::Command { id, .. } if id == "editor.gotoLine"));
    // Previous on the first change does nothing.
    assert_eq!(popup.step(&cx, Tool::Previous).effects, Vec::new());
    // Rollback acts on the change and closes.
    let step = popup.step(&cx, Tool::Rollback);
    assert!(step.close);
    assert_eq!(
        step.effects,
        vec![Effect::Command {
            id: "vcs.rollbackChange".into(),
            params: json!({"line": 1})
        }]
    );
    // A pure insertion has no old lines.
    let last = change::hunk_at(&cx, 5).unwrap();
    assert!(
        change::change_popup(&cx, last)
            .unwrap()
            .old_lines
            .is_empty()
    );
    assert!(change::change_popup(&cx, 99).is_none());
    drop(fixture);
    let _ = std::fs::remove_dir_all(root);
}

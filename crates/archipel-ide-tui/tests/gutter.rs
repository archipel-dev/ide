//! Wave 1: the editor gutter of the terminal (change bars, run icons, breakpoints, the run
//! popup, the breakpoint dialog, the change popup).
//! Spec: TUI-GUI-090, TUI-GUI-091, TUI-GUI-140, EXE-CTX-050..053, EXE-DBG-010.

mod common;

use archipel_kit_tui::input::{KeyCode, KeyModifiers};
use common::{Project, Tui};
use std::path::Path;

const RUN: &str = "\u{eb2c}";
const BREAKPOINT: &str = "\u{ea71}";
const RERUN: &str = "\u{ead2}";

const MAIN: &str = "package com.example;\npublic class Main {\n    int field = 1;\n    public static void main(String[] a) {\n        System.out.println(\"hi\");\n    }\n}\n";

fn java(name: &str) -> Project {
    Project::new(
        name,
        &[
            ("build.gradle.kts", "plugins { java }\n"),
            ("src/main/java/com/example/Main.java", MAIN),
            ("README.md", "# demo\n"),
        ],
    )
}

const JAVA_FILE: &str = "src/main/java/com/example/Main.java";

fn git(root: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(args)
        .output()
        .expect("git is installed");
    assert!(output.status.success(), "git {args:?}");
}

/// The columns of the gutter (TUI-GUI-090): the change bar, then the breakpoint lane, then the
/// run lane, then the line numbers, as the editor's first column says.
struct Columns {
    bar: u16,
    breakpoint: u16,
    run: u16,
    number: u16,
}

fn columns(tui: &mut Tui) -> Columns {
    let x = tui.app.shell_mut().geo.editor.x;
    Columns {
        bar: x,
        breakpoint: x + 1,
        run: x + 3,
        number: x + 5,
    }
}

/// The first cell of row `y` that holds `symbol`.
fn column_of(tui: &Tui, y: u16, symbol: &str) -> Option<u16> {
    (0..tui.buffer().area.width).find(|x| tui.cell(*x, y).symbol() == symbol)
}

/// The row where `needle` is on screen.
fn row_of(tui: &Tui, needle: &str) -> u16 {
    tui.find(needle)
        .unwrap_or_else(|| panic!("{needle} is not on screen\n{}", tui.text()))
        .1
}

/// Opens `project`'s file and waits until the run icons are computed.
fn open_java(project: &Project) -> Tui {
    let mut tui = Tui::open(&project.root, &[JAVA_FILE], 110, 24);
    tui.expect("the editor text", 10, |t| {
        t.contains("public static void main")
    });
    tui.advance(0.4);
    tui
}

fn breakpoints(tui: &Tui) -> Vec<usize> {
    tui.app.ide().state().breakpoint_lines(Path::new(JAVA_FILE))
}

#[test]
fn run_icons_sit_on_the_main_lines_and_the_text_does_not_shift() {
    let project = java("run-icons");
    let mut tui = open_java(&project);
    let gutter = columns(&mut tui);
    let main = row_of(&tui, "public static void main");
    let class = row_of(&tui, "public class Main");
    let field = row_of(&tui, "int field = 1;");
    let package = row_of(&tui, "package com.example;");
    // The icons are in the run lane, a green play triangle (Tone::Run).
    assert_eq!(tui.cell(gutter.run, main).symbol(), RUN, "{}", tui.text());
    assert_eq!(
        tui.cell(gutter.run, class).symbol(),
        RUN,
        "the class has a main too"
    );
    assert_eq!(tui.fg_at(gutter.run, main), tui.rt.look.palette.green);
    assert_ne!(tui.cell(gutter.run, field).symbol(), RUN);
    assert_ne!(tui.cell(gutter.run, package).symbol(), RUN);
    // The lanes are reserved: the code starts at the same column on a line with an icon, on
    // a line without one, and in a file that has no icon at all.
    let text_x = |tui: &Tui, needle: &str| tui.find(needle).unwrap().0;
    let start = text_x(&tui, "package com.example;");
    assert_eq!(
        text_x(&tui, "public class Main"),
        start,
        "an icon on the line"
    );
    assert_eq!(text_x(&tui, "public static void main"), start + 4);
    assert_eq!(text_x(&tui, "int field = 1;"), start + 4);
    tui.app
        .ide_mut()
        .dispatch("editor.openFile", serde_json::json!({"path": "README.md"}))
        .unwrap();
    tui.advance(0.2);
    assert_eq!(text_x(&tui, "# demo"), start, "{}", tui.text());
}

#[test]
fn run_icons_are_computed_after_a_pause_while_typing() {
    let project = java("run-icons-typing");
    let mut tui = open_java(&project);
    let gutter = columns(&mut tui);
    let main = row_of(&tui, "public static void main");
    assert_eq!(tui.cell(gutter.run, main).symbol(), RUN);
    // Type a new method above `main`: the old icons stay while typing, then they move.
    tui.app
        .ide_mut()
        .dispatch(
            "editor.gotoLine",
            serde_json::json!({"line": 3, "column": 1}),
        )
        .unwrap();
    tui.press_with(KeyCode::Home, KeyModifiers::NONE);
    tui.type_text("    void other() {}");
    tui.press(KeyCode::Enter);
    tui.advance(0.4);
    let main = row_of(&tui, "public static void main");
    assert_eq!(tui.cell(gutter.run, main).symbol(), RUN, "{}", tui.text());
    let other = row_of(&tui, "void other()");
    assert_ne!(tui.cell(gutter.run, other).symbol(), RUN);
}

#[test]
fn a_click_on_a_line_number_toggles_a_breakpoint_and_writes_the_file() {
    let project = java("click-toggle");
    let mut tui = open_java(&project);
    let gutter = columns(&mut tui);
    let row = row_of(&tui, "int field = 1;");
    // Line 3: its number is in the number column.
    assert_eq!(
        tui.cell(gutter.number + 1, row).symbol(),
        "3",
        "{}",
        tui.text()
    );
    tui.click(gutter.number + 1, row);
    assert_eq!(breakpoints(&tui), vec![2], "{}", tui.text());
    let file = project.read(".archipel/breakpoints.json");
    assert!(file.contains("Main.java"), "{file}");
    // The mark is a red circle in the first lane.
    tui.advance(0.1);
    assert_eq!(tui.cell(gutter.breakpoint, row).symbol(), BREAKPOINT);
    assert_eq!(tui.fg_at(gutter.breakpoint, row), tui.rt.look.palette.red);
    // A second click removes it.
    tui.click(gutter.number + 1, row);
    assert_eq!(breakpoints(&tui), Vec::<usize>::new());
    assert!(
        !project
            .read(".archipel/breakpoints.json")
            .contains("Main.java"),
        "{}",
        project.read(".archipel/breakpoints.json")
    );
    tui.advance(0.1);
    assert_ne!(tui.cell(gutter.breakpoint, row).symbol(), BREAKPOINT);
}

#[test]
fn hovering_a_line_number_previews_a_breakpoint_that_follows_the_pointer() {
    let project = java("hover-preview");
    let mut tui = open_java(&project);
    let gutter = columns(&mut tui);
    let row = row_of(&tui, "int field = 1;");
    let number = gutter.number + 1;
    use archipel_kit_tui::input::{Event, MouseEvent, MouseEventKind};
    let hover = |tui: &mut Tui, x: u16, y: u16| {
        tui.send(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }));
    };
    hover(&mut tui, number, row);
    assert_eq!(tui.cell(gutter.breakpoint, row).symbol(), BREAKPOINT);
    let dim = tui.fg_at(gutter.breakpoint, row);
    assert_ne!(
        dim, tui.rt.look.palette.red,
        "the preview is dim, not a breakpoint"
    );
    assert_eq!(breakpoints(&tui), Vec::<usize>::new());
    // The pointer moves up a row: the preview follows.
    hover(&mut tui, number, row - 1);
    assert_ne!(tui.cell(gutter.breakpoint, row).symbol(), BREAKPOINT);
    assert_eq!(tui.cell(gutter.breakpoint, row - 1).symbol(), BREAKPOINT);
    // And leaves with the pointer.
    let (tx, ty) = tui.find("int field = 1;").unwrap();
    hover(&mut tui, tx, ty);
    assert_ne!(tui.cell(gutter.breakpoint, row - 1).symbol(), BREAKPOINT);
}

/// Replays a case of the specs: the `before` block of `case` of
/// `docs/specs/execution/cases/05-debugger-core.cases.md` (file name, text with `<caret>` and
/// `/*B*/` markers), as (file, text, caret line (0-based), line with a breakpoint).
fn debugger_case(case: &str) -> (String, String, usize, Option<usize>) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/specs/execution/cases/05-debugger-core.cases.md");
    let text = std::fs::read_to_string(path).unwrap();
    let section = text
        .split("### ")
        .find(|section| section.starts_with(case))
        .unwrap_or_else(|| panic!("no case {case}"));
    let before = section
        .split("```before\n")
        .nth(1)
        .unwrap()
        .split("```")
        .next()
        .unwrap();
    let mut lines = before.lines();
    let file = lines.next().unwrap().trim_start_matches("--- ").to_owned();
    let mut body = String::new();
    let (mut caret, mut marked) = (0, None);
    for (index, line) in lines.enumerate() {
        let mut line = line.to_owned();
        if line.contains("<caret>") {
            caret = index;
            line = line.replace("<caret>", "");
        }
        if line.contains("/*B*/") {
            marked = Some(index);
            line = line.replace("/*B*/", "");
        }
        body.push_str(&line);
        body.push('\n');
    }
    (file, body, caret, marked)
}

#[test]
fn replay_exe_dbg_010_ctrl_f8_toggles_and_removes_a_breakpoint() {
    for (case, expected) in [
        ("EXE-DBG-010 case 1", vec![2usize]),
        ("EXE-DBG-010 case 2", vec![]),
    ] {
        let (file, text, caret, marked) = debugger_case(case);
        let project = Project::new("replay-dbg", &[(&file, &text)]);
        let mut tui = Tui::open(&project.root, &[&file], 110, 24);
        tui.expect("the text", 10, |t| t.contains("public class Main"));
        if let Some(line) = marked {
            tui.app
                .ide_mut()
                .dispatch(
                    "debug.toggleBreakpoint",
                    serde_json::json!({"path": file, "line": line}),
                )
                .unwrap();
        }
        tui.app
            .ide_mut()
            .dispatch(
                "editor.gotoLine",
                serde_json::json!({"line": caret + 1, "column": 1}),
            )
            .unwrap();
        tui.advance(0.1);
        tui.press_with(KeyCode::F(8), KeyModifiers::CONTROL);
        tui.advance(0.1);
        let lines = tui.app.ide().state().breakpoint_lines(Path::new(&file));
        assert_eq!(lines, expected, "{case}\n{}", tui.text());
        assert!(
            !tui.contains("not available"),
            "Ctrl+F8 is not refused: {}",
            tui.text()
        );
    }
}

// ---------------------------------------------------------------------- change bars

const COMMITTED: &str = "one\ntwo\nthree\nfour\nfive\nsix\n";
/// Line 2 modified, `four` deleted, `seven` added.
const WORKING: &str = "one\nTWO\nthree\nfive\nsix\nseven\n";

/// A Git repository with `notes.txt` committed as [`COMMITTED`] and edited to [`WORKING`].
fn repo(name: &str) -> Project {
    let project = Project::new(name, &[("notes.txt", COMMITTED)]);
    git(&project.root, &["init", "-q", "-b", "main"]);
    git(&project.root, &["add", "."]);
    git(&project.root, &["commit", "-qm", "initial"]);
    std::fs::write(project.root.join("notes.txt"), WORKING).unwrap();
    project
}

fn open_repo(project: &Project) -> Tui {
    let mut tui = Tui::open(&project.root, &["notes.txt"], 110, 24);
    tui.expect("the change bars", 15, |t| t.contains("▎"));
    tui
}

#[test]
fn change_bars_show_added_modified_and_deleted_lines_in_the_palette_colors() {
    let project = repo("bars");
    let mut tui = open_repo(&project);
    let gutter = columns(&mut tui);
    let palette = tui.rt.look.palette;
    let bar = |tui: &Tui, needle: &str| {
        let row = row_of(tui, needle);
        (
            tui.cell(gutter.bar, row).symbol().to_owned(),
            tui.fg_at(gutter.bar, row),
        )
    };
    assert_eq!(
        bar(&tui, "TWO"),
        ("▎".to_owned(), palette.blue),
        "modified\n{}",
        tui.text()
    );
    assert_eq!(bar(&tui, "seven"), ("▎".to_owned(), palette.green), "added");
    // `four` is gone: the bar sits on the line after it, `five`, as a red overline.
    assert_eq!(bar(&tui, "five"), ("▔".to_owned(), palette.red), "deleted");
    assert_eq!(tui.cell(gutter.bar, row_of(&tui, "three")).symbol(), " ");
}

#[test]
fn a_change_bar_moves_with_its_line_when_lines_are_inserted_above() {
    let project = repo("bars-move");
    let mut tui = open_repo(&project);
    let gutter = columns(&mut tui);
    let before = row_of(&tui, "TWO");
    assert_eq!(tui.cell(gutter.bar, before).symbol(), "▎");
    tui.app
        .ide_mut()
        .dispatch(
            "editor.gotoLine",
            serde_json::json!({"line": 1, "column": 1}),
        )
        .unwrap();
    tui.advance(0.1);
    tui.type_text("zero");
    tui.press(KeyCode::Enter);
    tui.advance(0.2);
    let after = row_of(&tui, "TWO");
    assert_eq!(after, before + 1, "the line moved down\n{}", tui.text());
    let palette = tui.rt.look.palette;
    assert_eq!(tui.cell(gutter.bar, after).symbol(), "▎");
    assert_eq!(tui.fg_at(gutter.bar, after), palette.blue);
    // The new line is an addition.
    let zero = row_of(&tui, "zero");
    assert_eq!(tui.fg_at(gutter.bar, zero), palette.green);
}

#[test]
fn a_rolled_back_change_loses_its_bar() {
    let project = repo("bars-rollback");
    let mut tui = open_repo(&project);
    let gutter = columns(&mut tui);
    let row = row_of(&tui, "TWO");
    // Click the bar: the change popup opens; Rollback restores the HEAD text of the line.
    tui.click(gutter.bar, row);
    assert!(tui.contains("Rollback"), "{}", tui.text());
    let (x, y) = tui.find("Rollback").unwrap();
    tui.click(x + 1, y);
    tui.advance(0.2);
    assert!(!tui.contains("TWO"), "{}", tui.text());
    assert!(tui.contains("two"));
    let row = row_of(&tui, "two");
    assert_ne!(tui.cell(gutter.bar, row).symbol(), "▎", "the bar is gone");
    assert!(!tui.contains("Rollback"), "the popup closed");
}

// ---------------------------------------------------------------------- run popup

#[test]
fn the_run_icon_opens_the_popup_with_the_gui_rows_and_shortcuts() {
    let project = java("run-popup");
    let mut tui = open_java(&project);
    let gutter = columns(&mut tui);
    let main = row_of(&tui, "public static void main");
    tui.click(gutter.run, main);
    let screen = tui.text();
    for row in [
        "Run 'Main.main()'",
        "Debug 'Main.main()'",
        "Run 'Main.main()' with Coverage",
        "Modify Run Configuration…",
    ] {
        assert!(screen.contains(row), "{row}\n{screen}");
    }
    // The rows are in the GUI's order and the shortcut column shows the keymap's keys.
    let y = |needle: &str| row_of(&tui, needle);
    assert!(y("Run 'Main.main()'") < y("Debug 'Main.main()'"));
    assert!(y("Debug 'Main.main()'") < y("with Coverage"));
    assert!(y("with Coverage") < y("Modify Run Configuration"));
    assert!(screen.contains("Ctrl+Shift+F10"), "{screen}");
    // Coverage is greyed out.
    let c = tui.rt.look.theme.colors;
    let disabled = tui.rt.look.over(c.text_disabled, c.popup);
    let (x, row) = tui.find("with Coverage").unwrap();
    assert_eq!(tui.fg_at(x, row), disabled);
    // Escape closes it.
    tui.press(KeyCode::Esc);
    assert!(!tui.contains("Modify Run Configuration"), "{}", tui.text());
}

#[test]
fn a_press_outside_closes_the_run_popup_and_a_right_click_on_the_icon_opens_it() {
    let project = java("run-popup-outside");
    let mut tui = open_java(&project);
    let gutter = columns(&mut tui);
    let main = row_of(&tui, "public static void main");
    tui.right_click(gutter.run, main);
    assert!(tui.contains("Modify Run Configuration"), "{}", tui.text());
    // A press far from the popup, in the empty part of the editor.
    let editor = tui.app.shell_mut().geo.editor;
    tui.click(editor.right() - 8, editor.bottom() - 3);
    assert!(!tui.contains("Modify Run Configuration"), "{}", tui.text());
    // A line without an icon has no popup.
    let field = row_of(&tui, "int field = 1;");
    tui.click(gutter.run, field);
    assert!(!tui.contains("Modify Run Configuration"), "{}", tui.text());
}

#[test]
fn modify_run_configuration_selects_the_configuration_without_starting_a_run() {
    let project = java("run-popup-modify");
    let mut tui = open_java(&project);
    let gutter = columns(&mut tui);
    let main = row_of(&tui, "public static void main");
    tui.click(gutter.run, main);
    let (x, y) = tui.find("Modify Run Configuration").unwrap();
    tui.click(x + 2, y);
    tui.advance(0.2);
    let state = tui.app.ide().state();
    assert!(!state.execution().is_running(), "nothing was started");
    assert!(
        state
            .execution()
            .configurations()
            .configurations()
            .iter()
            .any(|config| config.name.contains("Main")),
        "the temporary configuration of the icon exists"
    );
    // The configurations list opens (the terminal's Edit Run/Debug Configurations).
    assert!(
        tui.contains("Enter runs the configuration"),
        "{}",
        tui.text()
    );
}

#[test]
fn debug_is_disabled_while_something_runs() {
    let project = Project::new(
        "run-popup-running",
        &[
            ("build.gradle.kts", "plugins { java }\n"),
            (JAVA_FILE, MAIN),
            (
                ".archipel/runConfigurations/slow.toml",
                "name = \"slow\"\ntype = \"shell\"\nprogram = \"sh\"\narguments = [\"-c\", \"echo slow-started; sleep 30\"]\n",
            ),
        ],
    );
    let mut tui = open_java(&project);
    tui.app
        .ide_mut()
        .dispatch("runConfig.run", serde_json::json!({"name": "slow"}))
        .unwrap();
    tui.expect("the run", 20, |t| {
        t.app.ide().state().execution().is_running()
    });
    let gutter = columns(&mut tui);
    let main = row_of(&tui, "public static void main");
    tui.click(gutter.run, main);
    let c = tui.rt.look.theme.colors;
    let disabled = tui.rt.look.over(c.text_disabled, c.popup);
    for row in ["Debug 'Main.main()'", "Modify Run Configuration"] {
        let (x, y) = tui
            .find(row)
            .unwrap_or_else(|| panic!("{row}\n{}", tui.text()));
        assert_eq!(tui.fg_at(x, y), disabled, "{row} is greyed out");
    }
    tui.press(KeyCode::Esc);
    tui.press_with(KeyCode::F(2), KeyModifiers::CONTROL);
    tui.expect("the stop", 20, |t| {
        !t.app.ide().state().execution().is_running()
    });
}

// ---------------------------------------------------------------------- breakpoint form

#[test]
fn a_right_click_on_a_number_opens_the_form_and_a_condition_turns_the_mark_orange() {
    let project = java("form-condition");
    let mut tui = open_java(&project);
    let gutter = columns(&mut tui);
    let row = row_of(&tui, "int field = 1;");
    tui.right_click(gutter.number + 1, row);
    let screen = tui.text();
    assert!(screen.contains("Line Breakpoint — Main.java:3"), "{screen}");
    for label in [
        "Condition",
        "Log message",
        "Hit count",
        "Done",
        "Remove Breakpoint",
    ] {
        assert!(screen.contains(label), "{label}\n{screen}");
    }
    // The breakpoint exists as soon as the form opens, as in the GUI.
    assert_eq!(breakpoints(&tui), vec![2]);
    // The condition field has the focus: type, then Enter is Done.
    tui.type_text("x > 1");
    tui.press(KeyCode::Enter);
    tui.advance(0.1);
    assert!(!tui.contains("Line Breakpoint —"), "{}", tui.text());
    let state = project.read(".archipel/debug-state.json");
    assert!(state.contains("x > 1"), "{state}");
    let options = tui
        .app
        .ide()
        .state()
        .breakpoint_options(Path::new(JAVA_FILE), 2)
        .expect("options");
    assert_eq!(options.condition.as_deref(), Some("x > 1"));
    // The mark is the conditional variant: the same circle in the warning color.
    assert_eq!(tui.cell(gutter.breakpoint, row).symbol(), BREAKPOINT);
    assert_eq!(
        tui.fg_at(gutter.breakpoint, row),
        tui.rt.look.palette.yellow
    );
    // Opening it again shows what was stored; Remove clears the breakpoint and its options.
    tui.right_click(gutter.number + 1, row);
    assert!(tui.contains("x > 1"), "{}", tui.text());
    let (x, y) = tui.find("Remove Breakpoint").unwrap();
    tui.click(x + 2, y);
    tui.advance(0.1);
    assert_eq!(breakpoints(&tui), Vec::<usize>::new());
    assert!(
        !project.read(".archipel/debug-state.json").contains("x > 1"),
        "{}",
        project.read(".archipel/debug-state.json")
    );
    assert_ne!(tui.cell(gutter.breakpoint, row).symbol(), BREAKPOINT);
    assert!(!tui.contains("Line Breakpoint —"));
}

#[test]
fn a_log_message_makes_a_logpoint_and_escape_keeps_the_breakpoint_as_it_was() {
    let project = java("form-logpoint");
    let mut tui = open_java(&project);
    let gutter = columns(&mut tui);
    let row = row_of(&tui, "int field = 1;");
    tui.right_click(gutter.number + 1, row);
    tui.press(KeyCode::Tab);
    tui.type_text("hit {x}");
    tui.press(KeyCode::Enter);
    tui.advance(0.1);
    let options = tui
        .app
        .ide()
        .state()
        .breakpoint_options(Path::new(JAVA_FILE), 2)
        .expect("options");
    assert_eq!(options.log_message.as_deref(), Some("hit {x}"));
    assert_eq!(options.condition, None);
    let logpoint = tui.fg_at(gutter.breakpoint, row);
    let palette = tui.rt.look.palette;
    assert_ne!(logpoint, palette.red);
    assert_ne!(logpoint, palette.yellow);
    // Escape closes the form without changing anything.
    tui.right_click(gutter.number + 1, row);
    tui.type_text("zzz");
    tui.press(KeyCode::Esc);
    tui.advance(0.1);
    assert!(!tui.contains("Line Breakpoint —"));
    assert_eq!(breakpoints(&tui), vec![2]);
    assert_eq!(
        tui.app
            .ide()
            .state()
            .breakpoint_options(Path::new(JAVA_FILE), 2)
            .and_then(|options| options.condition),
        None
    );
    assert!(
        tui.app
            .ide()
            .state()
            .debugger()
            .state()
            .editing_breakpoint
            .is_none()
    );
}

#[test]
fn the_edit_breakpoint_key_opens_the_form_for_the_caret_line() {
    let project = java("form-key");
    let mut tui = open_java(&project);
    tui.app
        .ide_mut()
        .dispatch(
            "editor.gotoLine",
            serde_json::json!({"line": 3, "column": 1}),
        )
        .unwrap();
    tui.advance(0.1);
    tui.press_with(KeyCode::F(8), KeyModifiers::CONTROL | KeyModifiers::SHIFT);
    assert!(
        tui.contains("Line Breakpoint — Main.java:3"),
        "{}",
        tui.text()
    );
    tui.press(KeyCode::Esc);
}

// ---------------------------------------------------------------------- change popup

fn caret_line(tui: &Tui) -> usize {
    let state = tui.app.ide().state();
    let document = state.active_doc().expect("a document");
    document
        .text
        .char_to_line(document.selections.primary().head)
}

#[test]
fn the_change_popup_shows_the_old_lines_and_steps_between_changes_like_the_keys() {
    use archipel_kit_tui::Glyph;
    let project = repo("popup");
    let mut tui = open_repo(&project);
    let gutter = columns(&mut tui);
    // What `vcs.nextChange` does from the first change.
    tui.app
        .ide_mut()
        .dispatch(
            "editor.gotoLine",
            serde_json::json!({"line": 2, "column": 1}),
        )
        .unwrap();
    tui.advance(0.1);
    tui.press_with(
        KeyCode::Down,
        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT,
    );
    let by_key = caret_line(&tui);
    assert_eq!(by_key, 3, "the deletion before `five`");
    tui.app
        .ide_mut()
        .dispatch(
            "editor.gotoLine",
            serde_json::json!({"line": 1, "column": 1}),
        )
        .unwrap();
    tui.advance(0.1);
    // Click the bar of the modified line.
    let row = row_of(&tui, "TWO");
    tui.click(gutter.bar, row);
    let (rx, ry) = tui.find("Rollback").expect("the toolbar row");
    // The popup content is the HEAD text of the change.
    let base = {
        let state = tui.app.ide().state();
        let hunks = state.document_changes(0);
        state.hunk_base_text(0, &hunks[0]).unwrap()
    };
    assert_eq!(base, "two\n");
    let line_below = tui.screen()[ry as usize + 1].clone();
    assert!(
        line_below.contains("two") && !line_below.contains("TWO"),
        "{}",
        tui.text()
    );
    // The toolbar: up, down, Rollback, Show Diff (greyed out in the terminal), close.
    for glyph in [Glyph::ChevronUp, Glyph::ChevronDown, Glyph::Close] {
        let symbol = tui.rt.look.glyph(glyph);
        assert!(
            column_of(&tui, ry, &symbol).is_some(),
            "{glyph:?}\n{}",
            tui.text()
        );
    }
    assert!(tui.screen()[ry as usize].contains("Show Diff"));
    let c = tui.rt.look.theme.colors;
    let disabled = tui.rt.look.over(c.text_disabled, c.popup);
    let (dx, dy) = tui.find("Show Diff").unwrap();
    assert_eq!(
        tui.fg_at(dx, dy),
        disabled,
        "Show Diff waits for the diff viewer"
    );
    // Previous is disabled on the first change; clicking Show Diff does nothing.
    let up = tui.rt.look.glyph(Glyph::ChevronUp);
    let (ux, uy) = (column_of(&tui, ry, &up).unwrap(), ry);
    assert_eq!(tui.fg_at(ux, uy), disabled);
    tui.click(dx + 1, dy);
    assert!(tui.contains("Rollback"), "still open");
    let _ = rx;
    // Next shows the next change (a deletion: its old line is `four`) and moves the caret
    // exactly where the key moved it.
    let down = tui.rt.look.glyph(Glyph::ChevronDown);
    let nx = column_of(&tui, ry, &down).unwrap();
    tui.click(nx, ry);
    assert_eq!(caret_line(&tui), by_key, "{}", tui.text());
    let (_, ry) = tui.find("Rollback").expect("the popup stays open");
    let line_below = tui.screen()[ry as usize + 1].clone();
    assert!(line_below.contains("four"), "{}", tui.text());
    // Previous goes back.
    let ux = column_of(&tui, ry, &up).unwrap();
    tui.click(ux, ry);
    assert_eq!(caret_line(&tui), 1);
    // Close with the x; Escape also closes.
    let close = tui.rt.look.glyph(Glyph::Close);
    let cx = column_of(&tui, ry, &close).unwrap();
    tui.click(cx, ry);
    assert!(!tui.contains("Rollback"), "{}", tui.text());
    tui.click(gutter.bar, row_of(&tui, "TWO"));
    assert!(tui.contains("Rollback"));
    tui.press(KeyCode::Esc);
    assert!(!tui.contains("Rollback"));
}

#[test]
fn the_change_popup_caps_the_old_lines_at_twenty() {
    let committed: String = (1..=30).map(|n| format!("old {n}\n")).collect();
    let project = Project::new("popup-long", &[("long.txt", &committed)]);
    git(&project.root, &["init", "-q", "-b", "main"]);
    git(&project.root, &["add", "."]);
    git(&project.root, &["commit", "-qm", "initial"]);
    std::fs::write(project.root.join("long.txt"), "replaced\n").unwrap();
    let mut tui = Tui::open(&project.root, &["long.txt"], 110, 40);
    tui.expect("the change bar", 15, |t| t.contains("▎"));
    let gutter = columns(&mut tui);
    let row = row_of(&tui, "replaced");
    tui.click(gutter.bar, row);
    assert!(
        tui.contains("old 20") && !tui.contains("old 21"),
        "{}",
        tui.text()
    );
    assert!(tui.contains("… 10 more lines"), "{}", tui.text());
}

#[test]
fn rust_main_and_test_functions_get_run_icons() {
    let project = Project::new(
        "run-icons-rust",
        &[
            (
                "Cargo.toml",
                "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            ),
            (
                "src/main.rs",
                "fn main() {\n    helper();\n}\n\nfn helper() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn works() {}\n}\n",
            ),
        ],
    );
    let mut tui = Tui::open(&project.root, &["src/main.rs"], 110, 24);
    tui.expect("the text", 10, |t| t.contains("fn helper"));
    tui.advance(0.4);
    let gutter = columns(&mut tui);
    let icon = |tui: &Tui, needle: &str| {
        tui.cell(gutter.run, row_of(tui, needle))
            .symbol()
            .to_owned()
    };
    assert_eq!(icon(&tui, "fn main()"), RUN, "{}", tui.text());
    assert_eq!(icon(&tui, "fn works()"), RUN, "{}", tui.text());
    // The test module is a container: the "run all" arrow.
    assert_eq!(icon(&tui, "mod tests"), RERUN, "{}", tui.text());
    assert_eq!(
        tui.fg_at(gutter.run, row_of(&tui, "fn works()")),
        tui.rt.look.palette.green
    );
    assert_ne!(icon(&tui, "fn helper()"), RUN);
    assert_ne!(icon(&tui, "#[cfg(test)]"), RUN);
}

// ---------------------------------------------------------------------- keys

#[test]
fn every_gutter_action_has_a_binding_a_legacy_terminal_can_send_and_which_key_lists_the_chords() {
    use archipel_ide_view::keys::{CHORDS, leader_for};
    use archipel_kit::action_system::Os;
    use archipel_kit_tui::input::{Caps, key_with};
    let project = java("keys");
    // Tall enough for the whole which-key list.
    let mut tui = Tui::open(&project.root, &[JAVA_FILE], 110, 70);
    tui.expect("the editor text", 10, |t| {
        t.contains("public static void main")
    });
    let keymap = tui.app.keymap(Os::Linux);
    let set = archipel_ide_keymap::KeymapSet::shared().unwrap();
    let ide = set
        .keymap("$default", &archipel_ide_keymap::Overrides::default())
        .unwrap();
    let leader = leader_for(&ide);
    assert_eq!(leader, "ctrl \\");
    for action in [
        "ToggleLineBreakpoint",
        "EditBreakpoint",
        "Vcs.RollbackChangedLines",
        "VcsShowNextChangeMarker",
        "VcsShowPrevChangeMarker",
        "RunClass",
    ] {
        assert!(
            tui.app.ide().knows_action(action),
            "{action} is implemented"
        );
        // The chord: the leader and a key.
        let (_, keys) = CHORDS
            .iter()
            .find(|(id, _)| *id == action)
            .unwrap_or_else(|| panic!("{action} has no chord"));
        let mut rt = archipel_kit_tui::Runtime::with(
            archipel_kit_tui::Look::dark(),
            keymap.clone(),
            Caps::legacy(),
        );
        for stroke in [leader, keys] {
            let mut mods = KeyModifiers::NONE;
            let mut key = None;
            for part in stroke.split_whitespace() {
                match part {
                    "ctrl" => mods |= KeyModifiers::CONTROL,
                    "alt" => mods |= KeyModifiers::ALT,
                    "shift" => mods |= KeyModifiers::SHIFT,
                    k => key = k.chars().next(),
                }
            }
            let c = key.unwrap();
            let c = if mods.contains(KeyModifiers::SHIFT) {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            };
            rt.preprocess(&key_with(KeyCode::Char(c), mods));
        }
        assert!(
            rt.actions().iter().any(|id| id == action),
            "{leader} {keys} performs {action}: {:?}",
            rt.actions()
        );
    }
    // After the leader the which-key list names the gutter's chords.
    tui.send(key_with(KeyCode::Char('\\'), KeyModifiers::CONTROL));
    let screen = tui.text();
    for label in [
        "Toggle Line Breakpoint",
        "Rollback Lines",
        "Next Change",
        "Previous Change",
        "Edit Breakpoint",
    ] {
        assert!(screen.contains(label), "which-key lists {label}\n{screen}");
    }
    tui.press(KeyCode::Esc);
}

#[test]
fn the_chords_and_the_gui_keys_toggle_a_breakpoint_roll_back_and_step_through_changes() {
    use archipel_kit_tui::input::key_with;
    let project = repo("keys-run");
    let mut tui = open_repo(&project);
    // Ctrl+\ then Shift+... : the toggle chord sets a breakpoint on the caret line.
    tui.app
        .ide_mut()
        .dispatch(
            "editor.gotoLine",
            serde_json::json!({"line": 3, "column": 1}),
        )
        .unwrap();
    tui.advance(0.1);
    tui.send(key_with(KeyCode::Char('\\'), KeyModifiers::CONTROL));
    tui.send(key_with(KeyCode::Char('p'), KeyModifiers::NONE));
    let lines = tui
        .app
        .ide()
        .state()
        .breakpoint_lines(Path::new("notes.txt"));
    assert_eq!(lines, vec![2]);
    // The rollback chord restores the HEAD text of the change at the caret.
    tui.app
        .ide_mut()
        .dispatch(
            "editor.gotoLine",
            serde_json::json!({"line": 2, "column": 1}),
        )
        .unwrap();
    tui.advance(0.1);
    tui.send(key_with(KeyCode::Char('\\'), KeyModifiers::CONTROL));
    tui.send(key_with(KeyCode::Char('z'), KeyModifiers::ALT));
    tui.advance(0.1);
    assert!(
        tui.contains("two") && !tui.contains("TWO"),
        "{}",
        tui.text()
    );
}

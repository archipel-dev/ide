mod common;

use archipel_kit_tui::input::{KeyCode, KeyModifiers};
use common::{Project, Tui};

const MAIN: &str = "package demo;\n\npublic class Main {\n    public static void main(String[] args) {\n        System.out.println(\"hi\");\n    }\n}\n";

fn fixture(name: &str) -> Project {
    Project::new(
        name,
        &[("README.md", "# Fixture\n"), ("src/Main.java", MAIN)],
    )
}

#[test]
fn syntax_is_colored_from_the_core_analysis() {
    let project = fixture("colors");
    let mut tui = Tui::open(&project.root, &["src/Main.java"], 110, 24);
    tui.advance(0.2);
    let (kx, ky) = tui.find("public class").expect("the keyword is drawn");
    let (cx, cy) = tui.find("Main {").expect("the class name is drawn");
    let keyword = tui.fg_at(kx, ky);
    let plain = tui.fg_at(cx + 5, cy);
    assert_ne!(
        keyword,
        plain,
        "keywords have their own color\n{}",
        tui.text()
    );
    let expected = tui.rt.look.color(tui.rt.look.theme.editor.keyword);
    assert_eq!(keyword, expected);
}

#[test]
fn typing_edits_the_core_document_and_save_writes_the_file() {
    let project = fixture("typing");
    let mut tui = Tui::open(&project.root, &["src/Main.java"], 110, 24);
    tui.advance(0.1);
    // Caret on the first line: add a comment above the package.
    tui.type_text("// typed in the terminal");
    tui.press(KeyCode::Enter);
    let text = tui.app.ide().state().documents()[0].text.text();
    assert!(
        // ED-TYPE-066: Enter in the middle of a line comment continues it.
        text.starts_with("// typed in the terminal\n// package demo;"),
        "{text}"
    );
    assert!(tui.app.ide().state().documents()[0].text.is_dirty());
    assert!(tui.contains("// typed in the terminal"));
    assert_eq!(project.read("src/Main.java"), MAIN, "not saved yet");
    tui.ctrl('s');
    tui.advance(0.1);
    assert!(
        project
            .read("src/Main.java")
            .starts_with("// typed in the terminal\n"),
        "saved by Save All"
    );
    assert!(!tui.app.ide().state().documents()[0].text.is_dirty());
    // Undo is the core's (`editor.undo`), like in the GUI.
    for _ in 0..3 {
        tui.ctrl('z');
    }
    let text = tui.app.ide().state().documents()[0].text.text();
    assert_eq!(text, MAIN);
    assert!(
        tui.app.ide().state().documents()[0].text.is_dirty()
            || project.read("src/Main.java") != MAIN
    );
}

#[test]
fn a_diagnostic_shows_up_and_alt_enter_applies_the_quick_fix() {
    let project = Project::new(
        "quickfix",
        &[("src/A.java", "class A {\n  void f(int a) {\n    \n  }\n}\n")],
    );
    let mut tui = Tui::open(&project.root, &["src/A.java"], 120, 30);
    tui.advance(0.1);
    // Go to line 3, column 5, and type a self assignment.
    tui.ctrl('g');
    tui.type_text("3:5");
    tui.press(KeyCode::Enter);
    tui.type_text("a = a;");
    tui.ctrl('s');
    // The daemon reports it.
    tui.expect("the problem of the self assignment", 30, |tui| {
        !tui.app.ide().state().active_problems().is_empty()
    });
    let message = tui.app.ide().state().active_problems()[0].message.clone();
    tui.alt('6');
    tui.advance(0.1);
    assert!(tui.contains("Problems"), "{}", tui.text());
    assert!(
        tui.contains(&message),
        "the Problems list shows {message:?}\n{}",
        tui.text()
    );
    // Back in the editor with the caret inside the statement.
    tui.press(KeyCode::Esc);
    tui.press(KeyCode::Left);
    tui.press(KeyCode::Left);
    tui.press_with(KeyCode::Enter, KeyModifiers::ALT);
    tui.expect("the quick fixes popup", 20, |tui| {
        tui.contains("Remove the assignment")
    });
    println!("{}", tui.text());
    tui.press(KeyCode::Enter);
    tui.advance(0.1);
    let text = tui.app.ide().state().documents()[0].text.text();
    assert!(!text.contains("a = a"), "{text}");
    assert!(!tui.contains("Remove the assignment"), "the popup closed");
    tui.ctrl('s');
    assert!(!project.read("src/A.java").contains("a = a"));
}

fn shift_ctrl(tui: &mut Tui, c: char) {
    use archipel_kit_tui::input::key_with;
    tui.send(key_with(
        KeyCode::Char(c),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    ));
}

#[test]
fn find_action_lists_actions_with_their_shortcuts_and_runs_one() {
    let project = fixture("findaction");
    let mut tui = Tui::open(&project.root, &["src/Main.java"], 120, 30);
    tui.advance(0.1);
    tui.type_text("// via find action");
    assert!(tui.app.ide().state().documents()[0].text.is_dirty());
    shift_ctrl(&mut tui, 'a');
    tui.advance(0.1);
    // Find Action is Search Everywhere with the Actions tab selected (TUI-GUI-136).
    assert!(
        tui.contains("Include disabled actions") && tui.contains("Type / to see commands"),
        "{}",
        tui.text()
    );
    tui.type_text("Save All");
    tui.expect("the Save All row", 10, |tui| tui.contains("Save All"));
    println!("{}", tui.text());
    assert!(
        tui.contains("Ctrl+S"),
        "the row shows the GUI shortcut\n{}",
        tui.text()
    );
    tui.press(KeyCode::Enter);
    tui.advance(0.1);
    assert!(!tui.app.ide().state().documents()[0].text.is_dirty());
    assert!(project.read("src/Main.java").contains("// via find action"));
}

#[test]
fn go_to_file_opens_the_chosen_file_in_a_tab() {
    let project = Project::new(
        "gotofile",
        &[
            ("src/Main.java", MAIN),
            (
                "src/util/Strings.java",
                "package demo.util;\n\npublic class Strings {}\n",
            ),
        ],
    );
    let mut tui = Tui::open(&project.root, &["src/Main.java"], 120, 30);
    tui.advance(0.1);
    shift_ctrl(&mut tui, 'n');
    tui.type_text("Strings");
    tui.expect("the file row", 10, |tui| tui.contains("Strings.java"));
    tui.press(KeyCode::Enter);
    tui.advance(0.1);
    let state = tui.app.ide().state();
    assert_eq!(state.documents().len(), 2);
    assert_eq!(
        state.active_file().unwrap().to_str(),
        Some("src/util/Strings.java")
    );
    assert!(tui.contains("public class Strings"), "{}", tui.text());
}

#[test]
fn a_shell_run_configuration_prints_its_output_in_the_run_window() {
    let project = Project::new(
        "run",
        &[
            ("README.md", "# run\n"),
            (
                ".archipel/runConfigurations/hello.toml",
                "name = \"hello\"\ntype = \"shell\"\nprogram = \"sh\"\narguments = [\"-c\", \"echo hello-from-run; echo second line\"]\n",
            ),
        ],
    );
    let mut tui = Tui::open(&project.root, &["README.md"], 120, 30);
    tui.advance(0.1);
    assert!(
        tui.contains("hello"),
        "the run segment names the configuration\n{}",
        tui.text()
    );
    tui.press_with(KeyCode::F(10), KeyModifiers::SHIFT);
    tui.expect("the output", 20, |tui| tui.contains("hello-from-run"));
    tui.expect("the exit line", 20, |tui| {
        tui.contains("Process finished with exit code 0")
    });
    println!("{}", tui.text());
    assert!(tui.contains("second line"));
    assert!(
        tui.contains("Build"),
        "the Build window shows the run output"
    );
}

#[test]
fn find_in_files_lists_results_and_enter_opens_the_match() {
    let project = Project::new(
        "findinfiles",
        &[
            ("src/Main.java", MAIN),
            ("notes/todo.txt", "buy milk\nneedle in a haystack\nwalk\n"),
        ],
    );
    let mut tui = Tui::open(&project.root, &["src/Main.java"], 120, 30);
    tui.advance(0.1);
    shift_ctrl(&mut tui, 'f');
    tui.advance(0.1);
    assert!(tui.contains("Find in Files"), "{}", tui.text());
    tui.type_text("needle");
    tui.press(KeyCode::Enter);
    tui.expect("the result", 20, |tui| tui.contains("needle in a haystack"));
    assert!(
        tui.contains("1 result in 1 file for 'needle'"),
        "{}",
        tui.text()
    );
    println!("{}", tui.text());
    // The Find window has the focus: move to the result row and open it.
    tui.press(KeyCode::Down);
    tui.press(KeyCode::Enter);
    tui.advance(0.1);
    let state = tui.app.ide().state();
    assert_eq!(
        state.active_file().unwrap().to_str(),
        Some("notes/todo.txt")
    );
    let document = state.active_doc().unwrap();
    let selected = document.selections.primary();
    assert_eq!(
        document
            .text
            .slice(selected.start(), selected.end())
            .unwrap(),
        "needle",
        "the match is selected"
    );
}

#[test]
fn go_to_declaration_and_back_navigate_through_the_language_server() {
    let project = Project::new(
        "declaration",
        &[("Calc.fake", "greeting\nsomething else\nuse greeting here\n")],
    );
    let Some(mut tui) = Tui::with_fake_lsp(&project.root, &["Calc.fake"], 120, 30) else {
        return;
    };
    tui.advance(0.1);
    tui.ctrl('g');
    tui.type_text("3:7");
    tui.press(KeyCode::Enter);
    tui.advance(0.1);
    assert!(tui.contains("3:7"), "{}", tui.text());
    // The server may still be starting: ask until it answers.
    tui.expect("the declaration", 30, |tui| {
        tui.ctrl('b');
        tui.advance(0.2);
        tui.contains("1:1")
    });
    // Back returns to the usage.
    tui.press_with(KeyCode::Left, KeyModifiers::CONTROL | KeyModifiers::ALT);
    tui.advance(0.1);
    assert!(tui.contains("3:7"), "{}", tui.text());
    // Forward goes to the declaration again; the leader chords do the same.
    tui.ctrl('\\');
    tui.type_text("]");
    tui.advance(0.1);
    assert!(tui.contains("1:1"), "{}", tui.text());
}

#[test]
fn code_completion_opens_a_popup_and_enter_inserts_the_item() {
    let project = Project::new("completion", &[("App.fake", "app.")]);
    let Some(mut tui) = Tui::with_fake_lsp(&project.root, &["App.fake"], 120, 30) else {
        return;
    };
    tui.advance(0.1);
    tui.ctrl('g');
    tui.type_text("1:5");
    tui.press(KeyCode::Enter);
    tui.advance(0.1);
    // Ctrl+Space until the server is ready and answers.
    tui.expect("the completion popup", 30, |tui| {
        tui.press_with(KeyCode::Char(' '), KeyModifiers::CONTROL);
        tui.advance(0.2);
        tui.contains("getRequiredField")
    });
    println!("{}", tui.text());
    assert!(tui.contains("greeting"), "{}", tui.text());
    tui.press(KeyCode::Down);
    tui.press(KeyCode::Enter);
    tui.advance(0.2);
    let text = tui.app.ide().state().documents()[0].text.text();
    assert!(
        text.contains("app.getRequiredField"),
        "the item was inserted: {text:?}"
    );
    assert!(
        !tui.contains("m getClass"),
        "the popup closed\n{}",
        tui.text()
    );
}

fn git(root: &std::path::Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn the_commit_window_lists_changes_and_commits_with_a_message() {
    let project = Project::new("commit", &[("a.txt", "one\n"), ("b.txt", "two\n")]);
    git(&project.root, &["init", "-q", "-b", "main"]);
    git(&project.root, &["config", "user.name", "Tester"]);
    git(
        &project.root,
        &["config", "user.email", "tester@example.com"],
    );
    git(&project.root, &["add", "-A"]);
    git(&project.root, &["commit", "-q", "-m", "initial"]);
    std::fs::write(project.root.join("a.txt"), "one changed\n").unwrap();
    std::fs::write(project.root.join("c.txt"), "new file\n").unwrap();
    let mut tui = Tui::open(&project.root, &["b.txt"], 120, 30);
    tui.expect("the branch in the status line", 20, |tui| {
        tui.contains("main")
    });
    // Alt+0: the Commit window, like the GUI.
    tui.alt('0');
    tui.expect("the changes", 20, |tui| tui.contains("Changes (1)"));
    println!("{}", tui.text());
    assert!(tui.contains("[x] a.txt"), "{}", tui.text());
    assert!(tui.contains("Unversioned Files (1)"), "{}", tui.text());
    assert!(
        tui.contains("[ ] c.txt"),
        "unversioned files start unchecked\n{}",
        tui.text()
    );
    tui.press(KeyCode::Char('c'));
    tui.advance(0.1);
    assert!(tui.contains("Commit"), "{}", tui.text());
    tui.type_text("change a");
    tui.advance(0.4);
    tui.press(KeyCode::Enter);
    tui.advance(0.2);
    tui.expect("the commit", 20, |_| {
        git(&project.root, &["log", "--format=%s"]).starts_with("change a")
    });
    let status = git(&project.root, &["status", "--porcelain"]);
    assert!(
        status.contains("?? c.txt") && !status.contains("a.txt"),
        "{status}"
    );
}

#[test]
fn the_terminal_window_runs_a_shell() {
    let project = fixture("terminal");
    let mut tui = Tui::open(&project.root, &["src/Main.java"], 120, 30);
    tui.advance(0.1);
    tui.press_with(KeyCode::F(12), KeyModifiers::ALT);
    tui.expect("the Terminal window", 20, |tui| tui.contains("Terminal"));
    // The shell starts asynchronously; type until the echo shows up.
    // The shell computes the marker, so the typed command line (which wraps in a narrow
    // window after a long prompt) never contains it.
    tui.type_text("echo tw-$((20+22))");
    tui.press(KeyCode::Enter);
    tui.expect("the shell output", 20, |tui| tui.contains("tw-42"));
    println!("{}", tui.text());
    // Shift+Esc hides the window and the focus returns to the editor.
    tui.press_with(KeyCode::Esc, KeyModifiers::SHIFT);
    tui.advance(0.1);
    assert!(!tui.contains("tw-42"), "{}", tui.text());
}

fn select_in_tree(tui: &mut Tui, path: &str) {
    for _ in 0..12 {
        if tui.app.ide().data.tree_selection.as_deref() == Some(std::path::Path::new(path)) {
            return;
        }
        tui.press(KeyCode::Down);
        tui.advance(0.05);
    }
    panic!("could not select {path}\n{}", tui.text());
}

#[test]
fn the_project_tree_opens_files_and_creates_and_deletes_them() {
    let project = Project::new(
        "tree",
        &[("src/Main.java", MAIN), ("notes.txt", "remember\n")],
    );
    let mut tui = Tui::open(&project.root, &[], 120, 30);
    tui.advance(0.1);
    assert!(
        tui.contains("Search Everywhere"),
        "the dashboard shows while nothing is open\n{}",
        tui.text()
    );
    // Alt+1 puts the focus in the tree; open src, then Main.java.
    tui.alt('1');
    select_in_tree(&mut tui, "src");
    tui.press(KeyCode::Right);
    tui.advance(0.1);
    select_in_tree(&mut tui, "src/Main.java");
    tui.press(KeyCode::Enter);
    tui.advance(0.1);
    assert_eq!(
        tui.app.ide().state().active_file().unwrap().to_str(),
        Some("src/Main.java")
    );
    assert!(tui.contains("public class Main"), "{}", tui.text());
    // New File goes next to the selected file.
    tui.alt('1');
    shift_ctrl(&mut tui, 'a');
    tui.type_text("New File");
    tui.expect("the New File action", 10, |tui| tui.contains("New File"));
    tui.press(KeyCode::Enter);
    tui.advance(0.1);
    tui.type_text("Extra.java");
    tui.press(KeyCode::Enter);
    tui.advance(0.2);
    assert!(
        project.root.join("src/Extra.java").is_file(),
        "{}",
        tui.text()
    );
    assert_eq!(
        tui.app.ide().state().active_file().unwrap().to_str(),
        Some("src/Extra.java")
    );
    // Delete asks first.
    tui.alt('1');
    select_in_tree(&mut tui, "notes.txt");
    tui.press(KeyCode::Delete);
    tui.advance(0.1);
    assert!(tui.contains("Delete notes.txt?"), "{}", tui.text());
    assert!(project.root.join("notes.txt").exists());
    tui.press(KeyCode::Enter);
    tui.advance(0.2);
    assert!(!project.root.join("notes.txt").exists());
}

#[test]
fn losing_the_terminal_focus_saves_modified_files_like_the_gui() {
    let project = fixture("autosave");
    let mut tui = Tui::open(&project.root, &["src/Main.java"], 110, 24);
    tui.advance(0.1);
    tui.type_text("// unsaved");
    assert_eq!(project.read("src/Main.java"), MAIN);
    tui.send(archipel_kit_tui::Event::FocusLost);
    assert!(project.read("src/Main.java").starts_with("// unsaved"));
}

#[test]
fn a_click_on_a_tab_selects_it_and_recent_files_lists_the_history() {
    let project = Project::new(
        "tabs",
        &[("one.txt", "first file\n"), ("two.txt", "second file\n")],
    );
    let mut tui = Tui::open(&project.root, &["one.txt", "two.txt"], 120, 24);
    tui.advance(0.1);
    assert_eq!(
        tui.app.ide().state().active_file().unwrap().to_str(),
        Some("two.txt")
    );
    let (x, y) = tui.find("one.txt").expect("the first tab is drawn");
    assert_eq!(y, 1, "the tab line is below the toolbar");
    tui.click(x + 1, y);
    tui.advance(0.1);
    assert_eq!(
        tui.app.ide().state().active_file().unwrap().to_str(),
        Some("one.txt")
    );
    assert!(tui.contains("first file"), "{}", tui.text());
    // Ctrl+E: Recent Files, newest first; choosing the second entry opens it.
    tui.ctrl('e');
    tui.expect("the recent files", 10, |tui| tui.contains("two.txt"));
    tui.press(KeyCode::Down);
    tui.press(KeyCode::Enter);
    tui.advance(0.1);
    assert_eq!(
        tui.app.ide().state().active_file().unwrap().to_str(),
        Some("two.txt")
    );
}

#[test]
fn the_run_configuration_chooser_runs_the_pick_and_stop_ends_a_running_task() {
    let project = Project::new(
        "stop",
        &[
            ("README.md", "# stop\n"),
            (
                ".archipel/runConfigurations/quick.toml",
                "name = \"quick\"\ntype = \"shell\"\nprogram = \"sh\"\narguments = [\"-c\", \"echo quick-done\"]\n",
            ),
            (
                ".archipel/runConfigurations/slow.toml",
                "name = \"slow\"\ntype = \"shell\"\nprogram = \"sh\"\narguments = [\"-c\", \"echo slow-started; sleep 30\"]\n",
            ),
        ],
    );
    let mut tui = Tui::open(&project.root, &["README.md"], 120, 30);
    tui.advance(0.1);
    // Alt+Shift+F10 lists the configurations; choose `slow`.
    tui.press_with(KeyCode::F(10), KeyModifiers::ALT | KeyModifiers::SHIFT);
    tui.expect("the chooser", 10, |tui| tui.contains("Edit Configurations"));
    assert!(
        tui.contains("quick") && tui.contains("slow"),
        "{}",
        tui.text()
    );
    let (x, y) = tui.find("slow").expect("the slow row");
    tui.click(x + 1, y);
    // As in the GUI the choice selects the configuration; Shift+F10 runs it.
    tui.advance(0.1);
    tui.press_with(KeyCode::F(10), KeyModifiers::SHIFT);
    tui.expect("the output", 20, |tui| tui.contains("slow-started"));
    assert!(tui.app.ide().state().execution().is_running());
    // Ctrl+F2 stops it.
    tui.press_with(KeyCode::F(2), KeyModifiers::CONTROL);
    tui.expect("the stop", 20, |tui| tui.contains("Process stopped"));
    assert!(!tui.app.ide().state().execution().is_running());
}

fn doc_text(tui: &Tui) -> String {
    tui.app.ide().state().documents()[0].text.text()
}

fn selections(tui: &Tui) -> Vec<(usize, usize)> {
    tui.app.ide().state().documents()[0]
        .selections
        .selections()
        .iter()
        .map(|s| (s.anchor, s.head))
        .collect()
}

#[test]
fn keys_edit_through_the_core_input_layer_and_the_screen_shows_its_document() {
    let project = Project::new("core-input", &[("A.java", "class A {\n}\n")]);
    let mut tui = Tui::open(&project.root, &["A.java"], 100, 24);
    tui.advance(0.1);
    // End of the first line, then Enter in braces: the core indents and puts the caret in
    // the middle (ED-TYPE-064), the screen shows what the core has.
    tui.press(KeyCode::End);
    tui.press(KeyCode::Enter);
    assert_eq!(doc_text(&tui), "class A {\n    \n}\n");
    assert_eq!(selections(&tui), vec![(14, 14)]);
    // Typed `(` pairs, `)` overtypes (ED-TYPE-010, ED-TYPE-020).
    tui.type_text("f(");
    assert_eq!(doc_text(&tui), "class A {\n    f()\n}\n");
    tui.type_text(")");
    assert_eq!(doc_text(&tui), "class A {\n    f()\n}\n");
    assert_eq!(selections(&tui), vec![(17, 17)]);
    assert!(tui.contains("f()"), "{}", tui.text());
    // Backspace over the typed pair, Ctrl+D duplicates the line (ED-LINE-001).
    tui.press(KeyCode::Backspace);
    assert_eq!(doc_text(&tui), "class A {\n    f(\n}\n");
    tui.ctrl('d');
    assert_eq!(doc_text(&tui), "class A {\n    f(\n    f(\n}\n");
    // The editor model of the terminal is only a picture of the core document.
    assert!(tui.contains("f("), "{}", tui.text());
}

#[test]
fn several_carets_type_together_and_all_are_the_cores() {
    let project = Project::new("carets", &[("a.txt", "ab\nab\nab\n")]);
    let mut tui = Tui::open(&project.root, &["a.txt"], 80, 20);
    tui.advance(0.1);
    // Clone Caret Below twice (Alt+Shift+Down in the default keymap is Move Statement; the
    // action is Find Action's), then type at three carets.
    for _ in 0..2 {
        tui.app.run_action(
            &mut tui.rt,
            "EditorCloneCaretBelow",
            archipel_ide_actions::ActionPlace::MainMenu,
        );
    }
    assert_eq!(selections(&tui).len(), 3);
    tui.type_text("x");
    assert_eq!(doc_text(&tui), "xab\nxab\nxab\n");
    tui.press(KeyCode::Esc);
    assert_eq!(selections(&tui).len(), 1, "Escape keeps one caret");
}

#[test]
fn a_mouse_click_moves_the_core_caret_and_clipboard_keys_use_the_core() {
    let project = Project::new("mouse", &[("a.txt", "hello world\nsecond\n")]);
    let mut tui = Tui::open(&project.root, &["a.txt"], 80, 20);
    tui.advance(0.1);
    let (x, y) = tui.find("second").expect("the text is drawn");
    tui.click(x + 3, y);
    assert_eq!(
        selections(&tui),
        vec![(15, 15)],
        "the click is a core caret"
    );
    // Copy without a selection copies the line (the core's rule), Paste puts it above.
    tui.ctrl('c');
    assert_eq!(tui.rt.clipboard.get(), "second\n");
    assert!(tui.rt.clipboard.is_whole_line());
    tui.ctrl('v');
    assert_eq!(doc_text(&tui), "hello world\nsecond\nsecond\n");
    // A bracketed paste of the terminal goes the same way.
    tui.send(archipel_kit_tui::input::Event::Paste("abc".to_owned()));
    assert!(doc_text(&tui).contains("abc"), "{}", doc_text(&tui));
}

#[test]
fn the_page_keys_know_how_many_lines_the_screen_shows() {
    let text: String = (1..=100).map(|n| format!("line {n}\n")).collect();
    let project = Project::new("pages", &[("long.txt", text.as_str())]);
    let mut tui = Tui::open(&project.root, &["long.txt"], 80, 20);
    tui.advance(0.1);
    tui.press(KeyCode::PageDown);
    let (_, head) = selections(&tui)[0];
    let line = tui.app.ide().state().documents()[0].text.char_to_line(head);
    assert!(
        (10..=19).contains(&line),
        "a page of this screen, not of a default one: line {line}"
    );
    assert!(
        tui.contains(&format!("line {}", line + 1)),
        "{}",
        tui.text()
    );
}

#[test]
fn the_caret_stays_on_screen_while_typing_below_the_last_visible_line() {
    let text: String = (1..=60).map(|n| format!("row {n}\n")).collect();
    let project = Project::new("follow", &[("long.txt", text.as_str())]);
    let mut tui = Tui::open(&project.root, &["long.txt"], 80, 16);
    tui.advance(0.1);
    tui.press_with(KeyCode::End, KeyModifiers::CONTROL);
    tui.type_text("tail");
    assert!(tui.contains("tail"), "{}", tui.text());
    tui.press_with(KeyCode::Home, KeyModifiers::CONTROL);
    assert!(tui.contains("row 1"), "{}", tui.text());
}

#[test]
fn every_editor_key_of_the_kit_keymap_is_an_action_of_the_core() {
    let project = Project::new("keys", &[("a.txt", "x\n")]);
    let tui = Tui::open(&project.root, &["a.txt"], 80, 20);
    let keymap = tui.app.keymap(archipel_kit::action_system::Os::Linux);
    // Keys the terminal editor no longer takes itself: the scroll actions have no terminal
    // surface and the kit's own find actions are the widget's find bar.
    let widget = [
        "Find",
        "Replace",
        "FindNext",
        "FindPrevious",
        "FindWordAtCaret",
    ];
    let mut missing = Vec::new();
    for id in keymap.action_ids() {
        let editor = id.starts_with("Editor")
            || matches!(
                id.as_str(),
                "$Undo" | "$Redo" | "$Copy" | "$Cut" | "$Paste" | "$SelectAll"
            );
        if editor && !widget.contains(&id.as_str()) && !tui.app.ide().knows_action(&id) {
            missing.push(id);
        }
    }
    missing.sort();
    // Scrolling the view without the caret has no core command.
    missing.retain(|id| !id.starts_with("EditorScroll"));
    assert!(
        missing.is_empty(),
        "editor keys without a core action: {missing:?}"
    );
}

/// QA bug 7 of the TUI list: Enter on a project-relative location in the Run output (`src/main.rs:3:26`, as
/// rustc, javac and Maven print them) opens the file at that line and column.
#[test]
fn enter_on_a_relative_location_in_the_run_output_opens_the_file() {
    let project = Project::new(
        "run-link",
        &[
            ("README.md", "# run\n"),
            (
                "src/main.rs",
                "fn main() {\n    let x = 1;\n    println!(\"{}\", y); // undefined\n}\n",
            ),
            (
                ".archipel/runConfigurations/err.toml",
                "name = \"err\"\ntype = \"shell\"\nprogram = \"sh\"\narguments = [\"-c\", \"echo ' --> src/main.rs:3:26'\"]\n",
            ),
        ],
    );
    let mut tui = Tui::open(&project.root, &["README.md"], 120, 30);
    tui.advance(0.1);
    tui.press_with(KeyCode::F(10), KeyModifiers::SHIFT);
    tui.expect("the exit line", 20, |tui| {
        tui.contains("Process finished with exit code 0")
    });
    tui.alt('4');
    tui.advance(0.1);
    tui.press(KeyCode::Home);
    tui.press(KeyCode::Down);
    tui.press(KeyCode::Enter);
    tui.advance(0.2);
    let state = tui.app.ide().state();
    assert_eq!(
        state.active_file().unwrap().to_str(),
        Some("src/main.rs"),
        "{}",
        tui.text()
    );
    let caret = state.active_doc().unwrap().selections.primary().head;
    assert_eq!(
        state
            .active_doc()
            .unwrap()
            .text
            .char_to_line_column(caret)
            .unwrap(),
        (2, 25)
    );
}

/// QA bug 8 of the TUI list: the Run Configurations chooser lists the targets the build system detected, and
/// a Run that cannot start says why.
#[test]
fn the_run_chooser_lists_detected_targets_and_a_blocked_run_explains_itself() {
    let project = Project::new(
        "detected",
        &[
            (
                "pom.xml",
                "<project><modelVersion>4.0.0</modelVersion></project>\n",
            ),
            ("src/main/java/App.java", "class App {}\n"),
        ],
    );
    let mut tui = Tui::open(&project.root, &["src/main/java/App.java"], 120, 30);
    tui.advance(0.1);
    tui.press_with(KeyCode::F(10), KeyModifiers::ALT | KeyModifiers::SHIFT);
    tui.advance(0.2);
    assert!(tui.contains("Maven Compile"), "{}", tui.text());
    assert!(tui.contains("Maven Test"), "{}", tui.text());
    assert!(!tui.contains("No matches"), "{}", tui.text());
    tui.press(KeyCode::Esc);
    tui.advance(0.1);
    // Maven has no standard run goal: Shift+F10 says so instead of a bare "disabled".
    tui.press_with(KeyCode::F(10), KeyModifiers::SHIFT);
    tui.advance(0.2);
    assert!(tui.contains("Run is not available"), "{}", tui.text());
    assert!(tui.contains("no standard run goal"), "{}", tui.text());
    assert!(!tui.contains("run.project"), "{}", tui.text());
}

/// QA bug 19 of the TUI list: typing at the end of a 5 000-character line scrolls the wrapped rows so that
/// the caret and the typed text are visible.
#[test]
fn the_view_follows_the_caret_inside_a_very_long_wrapped_line() {
    let long = format!("{} end\n", "x".repeat(5000));
    let project = Project::new("long-line", &[("long.txt", long.as_str())]);
    let mut tui = Tui::open(&project.root, &["long.txt"], 100, 30);
    tui.advance(0.1);
    tui.press(KeyCode::End);
    tui.type_text("ZZZ");
    tui.advance(0.2);
    assert!(
        tui.contains("ZZZ"),
        "the typed text is visible\n{}",
        tui.text()
    );
}

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
    // Undo is the editor's; the core follows.
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
    assert!(
        tui.contains("Find Action") || tui.contains("Search Everywhere"),
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
    assert!(tui.contains("Run"), "the Run window is open");
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
    assert!(tui.contains("Ln 3, Col 7"), "{}", tui.text());
    // The server may still be starting: ask until it answers.
    tui.expect("the declaration", 30, |tui| {
        tui.ctrl('b');
        tui.advance(0.2);
        tui.contains("Ln 1,")
    });
    // Back returns to the usage.
    tui.press_with(KeyCode::Left, KeyModifiers::CONTROL | KeyModifiers::ALT);
    tui.advance(0.1);
    assert!(tui.contains("Ln 3,"), "{}", tui.text());
    // Forward goes to the declaration again; the Ctrl+K fallbacks do the same.
    tui.ctrl('k');
    tui.type_text("]");
    tui.advance(0.1);
    assert!(tui.contains("Ln 1,"), "{}", tui.text());
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
    tui.type_text("echo terminal-works");
    tui.press(KeyCode::Enter);
    tui.expect("the shell output", 20, |tui| {
        tui.text().matches("terminal-works").count() >= 2
    });
    println!("{}", tui.text());
    // Shift+Esc hides the window and the focus returns to the editor.
    tui.press_with(KeyCode::Esc, KeyModifiers::SHIFT);
    tui.advance(0.1);
    assert!(!tui.contains("terminal-works"), "{}", tui.text());
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
    assert_eq!(y, 0, "the tab line is the top row");
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
    // Alt+Shift+F10 lists the configurations; choose `slow` and it runs.
    tui.press_with(KeyCode::F(10), KeyModifiers::ALT | KeyModifiers::SHIFT);
    tui.expect("the chooser", 10, |tui| tui.contains("Run Configurations"));
    assert!(
        tui.contains("quick") && tui.contains("slow"),
        "{}",
        tui.text()
    );
    tui.type_text("slow");
    tui.expect("the slow row", 10, |tui| tui.contains("Shell Command"));
    tui.press(KeyCode::Enter);
    tui.expect("the output", 20, |tui| tui.contains("slow-started"));
    assert!(tui.app.ide().state().execution().is_running());
    // Ctrl+F2 stops it.
    tui.press_with(KeyCode::F(2), KeyModifiers::CONTROL);
    tui.expect("the stop", 20, |tui| tui.contains("Process stopped"));
    assert!(!tui.app.ide().state().execution().is_running());
}

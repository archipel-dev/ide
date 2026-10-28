//! Spec: TUI-GUI-135 (Search Everywhere anatomy), TUI-GUI-136 (Go to ... are its tabs),
//! TUI-GUI-138 (completion rows), TUI-GUI-139 (Alt+Enter with submenus), UI-POP-030..032,
//! and typing into a popup that a click opened.

mod common;

use archipel_kit_tui::input::{KeyCode, KeyModifiers, key_with};
use common::{Project, Tui};

fn fixture(name: &str) -> Project {
    Project::new(
        name,
        &[
            ("src/A.java", "class A {\n  void f(int a) {\n    \n  }\n}\n"),
            ("src/Widget.java", "class Widget {}\n"),
            ("notes.txt", "widget notes\n"),
        ],
    )
}

fn shift_ctrl(tui: &mut Tui, c: char) {
    tui.send(key_with(
        KeyCode::Char(c),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    ));
}

/// TUI-GUI-135: tabs and the include checkbox in the header band, the hint in the search row,
/// group separator rows, match highlights, the location and the link in the footer.
#[test]
fn search_everywhere_has_the_gui_anatomy() {
    let project = fixture("popups-se");
    let mut tui = Tui::open(&project.root, &["src/A.java"], 140, 36);
    tui.advance(0.2);
    // Go to Class opens Search Everywhere with the Classes tab (TUI-GUI-136); BackTab goes
    // to All.
    tui.ctrl('n');
    tui.advance(0.1);
    tui.press_with(KeyCode::BackTab, KeyModifiers::SHIFT);
    tui.advance(0.1);
    let text = tui.text();
    assert!(
        text.contains("All  Classes  Files  Symbols  Actions  Text"),
        "{text}"
    );
    assert!(text.contains("[ ] Include non-project items"), "{text}");
    assert!(text.contains("Type / to see commands"), "{text}");
    tui.type_text("widget");
    tui.expect("the rows", 20, |tui| {
        tui.contains("Widget") && tui.contains("Files")
    });
    let text = tui.text();
    assert!(text.contains("Open In Right Split"), "{text}");
    // A group separator row precedes the first row of its group.
    let group_y = tui
        .screen()
        .iter()
        .position(|line| line.contains("Classes") && !line.contains("Symbols"))
        .expect("the Classes group row") as u16;
    assert!(group_y > 4, "below the header band\n{text}");
    // The matched characters carry the search match background.
    let (x, y) = tui
        .screen()
        .iter()
        .enumerate()
        .find_map(|(y, line)| {
            line.find("Widget")
                .filter(|_| y as u16 > group_y)
                .map(|x| (line[..x].chars().count() as u16, y as u16))
        })
        .expect("a Widget row");
    let colors = tui.rt.look.theme.colors;
    let want = tui.rt.look.over(colors.search_match, colors.selection);
    let plain = tui.rt.look.over(colors.search_match, colors.popup);
    let bg = tui.cell(x + 1, y).bg;
    assert!(
        bg == want || bg == plain,
        "match highlight at ({x},{y})\n{text}"
    );
    // The footer shows the location of the selected row.
    assert!(tui.contains("src/Widget.java"), "{text}");
    // Alt+P flips the checkbox.
    tui.press_with(KeyCode::Char('p'), KeyModifiers::ALT);
    tui.advance(0.1);
    assert!(
        tui.contains("[x] Include non-project items"),
        "{}",
        tui.text()
    );
}

/// TUI-GUI-136: Find Action and Go to File are the same popup with another tab.
#[test]
fn go_to_file_and_find_action_are_tabs_of_search_everywhere() {
    let project = fixture("popups-tabs");
    let mut tui = Tui::open(&project.root, &["src/A.java"], 140, 36);
    tui.advance(0.2);
    shift_ctrl(&mut tui, 'n');
    tui.advance(0.1);
    let text = tui.text();
    assert!(text.contains("All  Classes  Files"), "{text}");
    // Files has no checkbox.
    assert!(!text.contains("Include non-project"), "{text}");
    tui.press(KeyCode::Esc);
    tui.advance(0.1);
    shift_ctrl(&mut tui, 'a');
    tui.advance(0.1);
    assert!(tui.contains("Include disabled actions"), "{}", tui.text());
}

/// Typing into a popup that a click opened reaches the query.
#[test]
fn typing_after_a_click_opened_search_everywhere_reaches_the_query() {
    let project = fixture("popups-click");
    let mut tui = Tui::open(&project.root, &["src/A.java"], 140, 36);
    tui.advance(0.2);
    // The search glyph of the toolbar sits six cells from the right edge.
    tui.click(140 - 6, 0);
    tui.advance(0.2);
    assert!(tui.contains("Type / to see commands"), "{}", tui.text());
    tui.type_text("widget");
    tui.expect("the typed query", 10, |tui| {
        tui.screen().iter().any(|line| line.contains("widget")) && tui.contains("Widget.java")
    });
}

/// The tail of a completion row is right aligned (UI-POP-030): the type of a member.
#[test]
fn a_completion_row_shows_its_type_at_the_right_edge() {
    let project = Project::new("popups-completion", &[("App.fake", "app.")]);
    let Some(mut tui) = Tui::with_fake_lsp(&project.root, &["App.fake"], 120, 30) else {
        return;
    };
    tui.advance(0.1);
    tui.ctrl('g');
    tui.type_text("1:5");
    tui.press(KeyCode::Enter);
    tui.advance(0.1);
    tui.expect("the completion popup", 30, |tui| {
        tui.press_with(KeyCode::Char(' '), KeyModifiers::CONTROL);
        tui.advance(0.2);
        tui.contains("greeting(String name)")
    });
    let screen = tui.screen();
    let row = screen
        .iter()
        .find(|line| line.contains("greeting(String name)"))
        .expect("the greeting row");
    assert!(row.trim_end().ends_with("String"), "{row:?}");
    // The signature follows the label; the tail is separated from it.
    let at = row.find("(String name)").unwrap();
    let tail = row.rfind("String").unwrap();
    assert!(tail > at + "(String name)".len() + 1, "{row:?}");
    let other = screen
        .iter()
        .find(|line| line.contains("getRequiredField"))
        .expect("the field row");
    assert!(other.trim_end().ends_with("Field"), "{other:?}");
}

fn open_alt_enter_popup(tui: &mut Tui) {
    tui.advance(0.1);
    tui.ctrl('g');
    tui.type_text("3:5");
    tui.press(KeyCode::Enter);
    tui.type_text("a = a;");
    tui.ctrl('s');
    tui.expect("the problem", 30, |tui| {
        !tui.app.ide().state().active_problems().is_empty()
    });
    tui.press(KeyCode::Left);
    tui.press(KeyCode::Left);
    tui.press_with(KeyCode::Enter, KeyModifiers::ALT);
    tui.expect("the popup", 20, |tui| tui.contains("Remove the assignment"));
}

/// QA: the entries of a submenu were dropped from the terminal popup.
#[test]
fn alt_enter_lists_submenus_and_right_opens_them() {
    let project = fixture("popups-alt");
    let mut tui = Tui::open(&project.root, &["src/A.java"], 120, 30);
    open_alt_enter_popup(&mut tui);
    assert!(
        tui.contains("Inspection 'Self assignment'"),
        "{}",
        tui.text()
    );
    assert!(!tui.contains("Suppress for file"), "collapsed at first");
    tui.press(KeyCode::Down);
    tui.press(KeyCode::Right);
    tui.advance(0.1);
    let text = tui.text();
    assert!(text.contains("Suppress for file"), "{text}");
    assert!(
        text.contains("Disable inspection 'Self assignment'"),
        "{text}"
    );
    tui.press(KeyCode::Left);
    tui.advance(0.1);
    assert!(!tui.contains("Suppress for file"), "{}", tui.text());
    // Enter on the header toggles it too.
    tui.press(KeyCode::Enter);
    tui.advance(0.1);
    assert!(tui.contains("Suppress for file"), "{}", tui.text());
    tui.press(KeyCode::Esc);
    tui.advance(0.1);
    assert!(!tui.contains("Suppress for file"));
}

/// Several targets: the chooser of the Go to Declaration family is shown and Enter opens one.
#[test]
fn several_navigation_targets_are_shown_in_a_chooser() {
    let project = Project::new(
        "popups-choices",
        &[(
            "src/A.java",
            "class A {\n  void f() {}\n  void g() { f(); }\n}\n",
        )],
    );
    let mut tui = Tui::open(&project.root, &["src/A.java"], 120, 30);
    tui.advance(0.1);
    let item = |line: usize| archipel_ide_core::NavigationItem {
        path: "src/A.java".into(),
        external: false,
        line,
        column: 7,
        end_column: 8,
        line_text: "  void f() {}".into(),
    };
    tui.app.ide_mut().state_mut().navigation_mut().choices =
        Some(archipel_ide_core::NavigationList {
            kind: archipel_ide_core::NavigationKind::Declaration,
            title: "Choose Declaration of f".into(),
            items: vec![item(1), item(2)],
            truncated: false,
        });
    tui.advance(0.1);
    let text = tui.text();
    assert!(text.contains("Choose Declaration of f"), "{text}");
    assert!(text.contains("2 results"), "{text}");
    assert!(text.contains("A.java 2"), "{text}");
    tui.press(KeyCode::Down);
    tui.press(KeyCode::Enter);
    tui.advance(0.2);
    assert!(!tui.contains("Choose Declaration of f"));
    assert!(
        tui.contains("3:"),
        "the caret moved to line 3\n{}",
        tui.text()
    );
}

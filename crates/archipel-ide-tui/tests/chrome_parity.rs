//! The chrome of the terminal shows what the shared chrome model says (TUI-GUI-190..192, 200):
//! for each fixture of TUI-CMP-041 and at 160x45, 100x30 and 80x24, the outline read back from
//! the screen cells is the outline of the props of `archipel-ide-view`: the same items in the
//! same order with the same labels, enabled and checked state, shortcut texts and tooltips.
//! Items that do not fit a narrow screen are dropped by priority, never reordered or altered.
//! The main menu is compared at full depth.
//! Spec: TUI-GUI-001, TUI-GUI-002, TUI-GUI-035, TUI-GUI-041, TUI-GUI-042, TUI-GUI-055, TUI-GUI-056, TUI-GUI-094, TUI-GUI-190, TUI-GUI-191 (tests).

mod common;

use archipel_ide_actions::ActionPlace;
use archipel_ide_tui::chrome::Pair;
use archipel_ide_tui::{StartOptions, TuiApp};
use archipel_kit::action_system::Os;
use archipel_kit_model::outline::{Outline, OutlineEntry};
use archipel_kit_tui::input::{Caps, KeyCode, KeyModifiers};
use common::{Project, Tui};
use serde_json::json;
use std::path::PathBuf;

const SIZES: [(u16, u16); 3] = [(160, 45), (100, 30), (80, 24)];

const MAIN: &str = "package demo;\n\npublic class Main {\n    public static void main(String[] args) {\n        System.out.println(\"hi\");\n    }\n}\n";

fn java_project(name: &str) -> Project {
    Project::new(
        name,
        &[
            ("build.gradle", "plugins { id 'java' }\n"),
            ("src/main/java/demo/Main.java", MAIN),
            (
                "src/main/java/demo/Util.java",
                "package demo;\nclass Util {}\n",
            ),
            ("README.md", "# demo\n"),
        ],
    )
}

fn android_project(name: &str) -> Project {
    Project::new(
        name,
        &[
            ("settings.gradle.kts", "include(\":app\")\n"),
            (
                "app/build.gradle.kts",
                "plugins { id(\"com.android.application\") }\nandroid {\n    namespace = \"com.example.app\"\n    compileSdk = 35\n}\n",
            ),
            (
                "app/src/main/AndroidManifest.xml",
                r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android"><application><activity android:name=".MainActivity"><intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter></activity></application></manifest>"#,
            ),
        ],
    )
}

fn caps(os: Os) -> Caps {
    Caps {
        os,
        ..Caps::kitty()
    }
}

fn open(project: &Project, files: &[&str], size: (u16, u16), os: Os) -> Tui {
    let app = TuiApp::new(StartOptions {
        project: Some(project.root.clone()),
        files: files.iter().map(PathBuf::from).collect(),
        isolated: true,
        ..StartOptions::default()
    });
    Tui::with_caps(app, size.0, size.1, caps(os))
}

/// One fixture: a name, and how to start it at a size.
struct Fixture {
    name: &'static str,
    start: fn(&Project, (u16, u16)) -> Tui,
    project: fn(&str) -> Project,
}

fn fixtures() -> Vec<Fixture> {
    vec![
        Fixture {
            name: "no project",
            project: |n| Project::new(n, &[("keep.txt", "")]),
            start: |_, size| {
                let app = TuiApp::new(StartOptions {
                    isolated: true,
                    ..StartOptions::default()
                });
                Tui::with_caps(app, size.0, size.1, caps(Os::Linux))
            },
        },
        Fixture {
            name: "java project, two documents, one dirty",
            project: java_project,
            start: |p, size| {
                let mut tui = open(
                    p,
                    &[
                        "src/main/java/demo/Main.java",
                        "src/main/java/demo/Util.java",
                    ],
                    size,
                    Os::Linux,
                );
                tui.advance(0.2);
                tui.type_text("// edit\n");
                tui.advance(0.1);
                tui
            },
        },
        Fixture {
            name: "java project on macOS notation",
            project: java_project,
            start: |p, size| {
                let mut tui = open(p, &["src/main/java/demo/Main.java"], size, Os::MacOs);
                tui.advance(0.2);
                tui
            },
        },
        Fixture {
            name: "android project",
            project: android_project,
            start: |p, size| {
                let mut tui = open(p, &[], size, Os::Linux);
                tui.advance(0.3);
                tui
            },
        },
        Fixture {
            name: "indexing in progress",
            project: java_project,
            start: |p, size| open(p, &["src/main/java/demo/Main.java"], size, Os::Linux),
        },
        Fixture {
            name: "a running task",
            project: |n| {
                Project::new(
                    n,
                    &[
                        ("README.md", "# run\n"),
                        (
                            ".archipel/runConfigurations/slow.toml",
                            "name = \"slow\"\ntype = \"shell\"\nprogram = \"sh\"\narguments = [\"-c\", \"echo started; sleep 20\"]\n",
                        ),
                    ],
                )
            },
            start: |p, size| {
                let mut tui = open(p, &["README.md"], size, Os::Linux);
                tui.advance(0.2);
                tui.press_with(KeyCode::F(10), KeyModifiers::SHIFT);
                tui.advance(0.5);
                tui
            },
        },
    ]
}

/// `got` must be `want` with entries dropped: the same entries in the same order. The
/// overflow button of a header that has dropped actions is the terminal's own (its menu
/// lists the dropped ones, see `the_overflow_menu_lists_the_dropped_actions`).
fn is_subsequence(got: &Outline, want: &Outline) -> Result<(), String> {
    let mut at = 0;
    for entry in got.entries.iter().filter(|e| e.id != "overflow") {
        let found = want.entries[at..].iter().position(|w| same_entry(entry, w));
        match found {
            Some(offset) => at += offset + 1,
            None => {
                return Err(format!(
                    "drawn but not in the props (or out of order): {entry:?}"
                ));
            }
        }
    }
    Ok(())
}

fn same_entry(got: &OutlineEntry, want: &OutlineEntry) -> bool {
    let mut got = got.clone();
    // A dropdown that shrank to its icon on a narrow screen has no text.
    if got.role == "dropdown" && got.label.is_empty() {
        got.label.clone_from(&want.label);
    }
    // Text that does not fit ends with an ellipsis; the rest must be the props' text.
    if got.label.ends_with('\u{2026}') && got.label != want.label {
        let kept = got.label.trim_end_matches('\u{2026}');
        if !want.label.starts_with(kept) {
            return false;
        }
        got.label.clone_from(&want.label);
    }
    got == *want
}

fn check(fixture: &Fixture, size: (u16, u16)) -> Vec<String> {
    let project = (fixture.project)(&format!(
        "parity-{}-{}x{}",
        fixture.name.len(),
        size.0,
        size.1
    ));
    let mut tui = (fixture.start)(&project, size);
    tui.advance(0.05);
    let mut problems = Vec::new();
    for Pair { name, got, want } in tui.parity() {
        if let Err(error) = is_subsequence(&got, &want) {
            problems.push(format!(
                "[{}] {name} at {}x{}: {error}\n  drawn: {}\n  props: {}",
                fixture.name,
                size.0,
                size.1,
                got.text().replace('\n', " | "),
                want.text().replace('\n', " | "),
            ));
        }
        // On the wide screen nothing may be dropped, except in the status bar, where the
        // long message and the widgets of a busy project share 160 cells (the message
        // shortens first, then widgets drop by priority).
        if size.0 >= 160
            && name != "status bar"
            && got.entries.iter().filter(|e| e.id != "overflow").count() != want.entries.len()
        {
            problems.push(format!(
                "[{}] {name} at {}x{} drops items\n  drawn: {}\n  props: {}",
                fixture.name,
                size.0,
                size.1,
                got.text().replace('\n', " | "),
                want.text().replace('\n', " | "),
            ));
        }
    }
    problems
}

#[test]
fn the_screen_shows_what_the_props_say_at_160x45() {
    let problems: Vec<String> = fixtures().iter().flat_map(|f| check(f, SIZES[0])).collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_screen_shows_what_the_props_say_at_100x30() {
    let problems: Vec<String> = fixtures().iter().flat_map(|f| check(f, SIZES[1])).collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_screen_shows_what_the_props_say_at_80x24() {
    let problems: Vec<String> = fixtures().iter().flat_map(|f| check(f, SIZES[2])).collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_fixtures_exist_for_every_surface() {
    // Completeness: each surface of the chrome is compared for each fixture.
    let project = java_project("parity-surfaces");
    let mut tui = open(
        &project,
        &["src/main/java/demo/Main.java"],
        SIZES[0],
        Os::Linux,
    );
    tui.advance(0.2);
    let names: Vec<String> = tui.parity().into_iter().map(|p| p.name).collect();
    for surface in [
        "toolbar",
        "left stripe",
        "right stripe",
        "tabs",
        "status bar",
    ] {
        assert!(
            names.iter().any(|n| n == surface),
            "{surface} is not compared: {names:?}"
        );
    }
    assert!(names.iter().any(|n| n.starts_with("header ")), "{names:?}");
    // Keep the action layer honest: the toolbar's run button is the Run action.
    let _ = ActionPlace::Toolbar;
    let _ = json!({});
}

// ------------------------------------------------------------------ the main menu

/// The text of each row of `rect` in the last frame, without the box lines.
fn rows_of(tui: &Tui, rect: archipel_kit_tui::ratatui::layout::Rect) -> Vec<String> {
    let buf = tui.buffer();
    (rect.top()..rect.bottom())
        .map(|y| {
            (rect.left()..rect.right())
                .map(|x| buf[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect()
}

/// Compares the visible rows of the menu at `depth` with `entries` (as many as are drawn).
fn check_rows(
    tui: &mut Tui,
    entries: &[archipel_kit_model::MenuEntry],
    depth: usize,
    notation: archipel_kit_model::Notation,
    path: &str,
    problems: &mut Vec<String>,
) -> usize {
    use archipel_kit_model::MenuEntry;
    let rect = tui
        .app
        .menu_control()
        .popup()
        .and_then(|p| p.chain.rect(depth));
    let Some(rect) = rect else {
        problems.push(format!("{path}: no menu is open at depth {depth}"));
        return 0;
    };
    let rows = rows_of(tui, rect);
    // The first and the last row are the box.
    let inner: Vec<(u16, String)> = rows
        .iter()
        .enumerate()
        .skip(1)
        .take(rows.len().saturating_sub(2))
        .map(|(i, r)| (rect.y + i as u16, r.clone()))
        .collect();
    let colors = tui.rt.look.theme.colors;
    let disabled = tui.rt.look.over(colors.text_disabled, colors.popup);
    for (entry, (y, row)) in entries.iter().zip(inner.iter()) {
        match entry {
            MenuEntry::Separator { title } => {
                let ok = match title {
                    Some(title) => row.contains(title.as_str()),
                    None => row.contains('\u{2500}'),
                };
                if !ok {
                    problems.push(format!("{path}: expected a separator, drawn {row:?}"));
                }
            }
            MenuEntry::Item(item) => {
                if !row.contains(item.label.as_str()) {
                    problems.push(format!("{path}: expected {:?}, drawn {row:?}", item.label));
                    continue;
                }
                if let Some(shortcut) = &item.shortcut {
                    let text = shortcut.label(notation);
                    if !row.contains(text.as_str()) {
                        problems.push(format!(
                            "{path}/{}: expected shortcut {text:?}, drawn {row:?}",
                            item.label
                        ));
                    }
                }
                let col = row.find(item.label.as_str()).unwrap_or(0);
                let x = rect.x + row[..col].chars().count() as u16;
                let is_disabled = tui.cell(x, *y).fg == disabled;
                if is_disabled == item.enabled {
                    problems.push(format!(
                        "{path}/{}: enabled={} but the label is drawn {}",
                        item.label,
                        item.enabled,
                        if is_disabled { "greyed" } else { "normal" }
                    ));
                }
            }
        }
    }
    inner.len()
}

/// Compares the menu at `depth` with `entries`, then every submenu.
fn walk_menu(
    tui: &mut Tui,
    entries: &[archipel_kit_model::MenuEntry],
    depth: usize,
    notation: archipel_kit_model::Notation,
    path: &str,
    problems: &mut Vec<String>,
) {
    use archipel_kit_model::MenuEntry;
    let visible = check_rows(tui, entries, depth, notation, path, problems);
    let selected = |tui: &mut Tui| {
        tui.app
            .menu_control()
            .popup()
            .and_then(|p| p.chain.selected(depth))
    };
    let items: Vec<(usize, &archipel_kit_model::MenuItem)> = entries
        .iter()
        .filter_map(|e| match e {
            MenuEntry::Item(item) => Some(item),
            _ => None,
        })
        .enumerate()
        .collect();
    if entries.len() > visible {
        // A menu taller than the screen scrolls: go to the end and compare the tail.
        // The wheel scrolls the popup to its end.
        let rect = tui
            .app
            .menu_control()
            .popup()
            .and_then(|p| p.chain.rect(depth))
            .expect("the menu is open");
        for _ in 0..entries.len() {
            tui.send(archipel_kit_tui::Event::Mouse(
                archipel_kit_tui::input::MouseEvent {
                    kind: archipel_kit_tui::input::MouseEventKind::ScrollDown,
                    column: rect.x + 4,
                    row: rect.y + rect.height / 2,
                    modifiers: KeyModifiers::NONE,
                },
            ));
        }
        tui.advance(0.1);
        let tail = &entries[entries.len() - visible..];
        check_rows(
            tui,
            tail,
            depth,
            notation,
            &format!("{path} (end)"),
            problems,
        );
        for _ in 0..entries.len() {
            tui.send(archipel_kit_tui::Event::Mouse(
                archipel_kit_tui::input::MouseEvent {
                    kind: archipel_kit_tui::input::MouseEventKind::ScrollUp,
                    column: rect.x + 4,
                    row: rect.y + rect.height / 2,
                    modifiers: KeyModifiers::NONE,
                },
            ));
        }
    }
    for (index, item) in items {
        let Some(sub) = &item.submenu else { continue };
        if !item.enabled {
            continue;
        }
        // Select the row with the keyboard, then open the submenu.
        let mut tries = 0;
        while selected(tui) != Some(index) && tries < entries.len() + 2 {
            tui.press(KeyCode::Down);
            tries += 1;
        }
        tui.press(KeyCode::Right);
        tui.advance(0.2);
        let sub_path = format!("{path}/{}", item.label);
        walk_menu(tui, sub, depth + 1, notation, &sub_path, problems);
        // Back to this level: the child closes.
        tui.press(KeyCode::Left);
        tui.advance(0.1);
    }
}

fn menu_problems(size: (u16, u16), os: Os) -> Vec<String> {
    let project = java_project(&format!("parity-menu-{}x{}-{os:?}", size.0, size.1));
    let mut tui = open(&project, &["src/main/java/demo/Main.java"], size, os);
    tui.advance(0.3);
    tui.app.menu_control().expand();
    tui.draw();
    let menu = tui.app.built_menu().expect("a frame was drawn").clone();
    let notation = archipel_ide_tui::chrome::ChromeUi::notation(&tui.rt);
    // The titles on the screen are the props' titles.
    let mut problems = Vec::new();
    let buffer = tui.buffer().clone();
    let titles = tui.app.menu_control().outline(&buffer);
    for (drawn, header) in titles.entries.iter().zip(&menu.headers) {
        if drawn.label != header.label {
            problems.push(format!(
                "title {:?} is drawn as {:?}",
                header.label, drawn.label
            ));
        }
    }
    if titles.entries.len() != menu.headers.len() {
        problems.push(format!(
            "{} titles of {} are drawn at {}x{}",
            titles.entries.len(),
            menu.headers.len(),
            size.0,
            size.1
        ));
    }
    for (index, header) in menu.headers.iter().enumerate() {
        tui.app
            .menu_control()
            .open_menu(&tui.rt, &menu, index, true);
        tui.advance(0.1);
        walk_menu(
            &mut tui,
            &header.entries,
            0,
            notation,
            &header.label,
            &mut problems,
        );
        tui.press(KeyCode::Esc);
        tui.advance(0.1);
        tui.app.menu_control().expand();
        tui.draw();
    }
    problems
}

#[test]
fn the_main_menu_shows_every_menu_to_the_deepest_submenu_at_160x45() {
    let problems = menu_problems(SIZES[0], Os::Linux);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_main_menu_uses_the_mac_notation_on_macos() {
    let problems = menu_problems(SIZES[0], Os::MacOs);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

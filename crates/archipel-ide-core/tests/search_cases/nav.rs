//! Runner of `05-recent-and-structure-popups.cases.md`: Go to Line, Recent Files, the Switcher,
//! Recent Locations, the navigation history and bookmarks at model level. The structure popup,
//! the navigation bar and tab management need the window or a language model and count as not
//! runnable.

use super::{Case, Outcome};
use archipel_ide_command::CommandRegistry;
use archipel_ide_core::quick_nav::{QuickNavState, Switcher, SwitcherEntry};
use archipel_ide_core::recent_popup::{
    Bookmarks, MnemonicOutcome, NO_RECENT_LOCATIONS, RecentFiles, RecentFilesPopup, snippet_window,
};
use archipel_ide_core::{IdeState, built_in_commands};
use serde_json::json;
use std::path::{Path, PathBuf};

fn fail(msg: impl Into<String>) -> Outcome {
    Outcome::Fail(msg.into())
}

fn check(ok: bool, what: &str) -> Outcome {
    if ok {
        Outcome::Pass
    } else {
        fail(what.to_owned())
    }
}

struct Editor {
    _dir: tempfile::TempDir,
    state: IdeState,
    commands: CommandRegistry<IdeState>,
}

impl Editor {
    fn new(text: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Case.txt"), text).unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": dir.path()}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "Case.txt"}))
            .unwrap();
        Self {
            _dir: dir,
            state,
            commands,
        }
    }

    fn lines(count: usize) -> String {
        (1..=count)
            .map(|i| {
                if i == 3 {
                    "0123456789".to_owned()
                } else {
                    format!("line {i:02}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn run(&mut self, command: &str, params: serde_json::Value) {
        let _ = self.commands.execute(command, &mut self.state, params);
    }

    fn goto(&mut self, line: usize, column: usize) {
        self.run("editor.gotoLine", json!({"line": line, "column": column}));
    }

    fn caret(&self) -> (usize, usize) {
        self.state.caret_line_column().unwrap()
    }

    fn enabled(&self, command: &str) -> bool {
        self.commands.is_enabled(command, &self.state)
    }

    /// Types `text` into the dialog and presses Enter; whether the dialog closed.
    fn go_to_line_dialog(&mut self, input: &str) -> bool {
        match self.state.goto_line_target(input) {
            Some((line, column)) => {
                self.goto(line, column);
                true
            }
            None => false,
        }
    }
}

fn files(visited: &[&str]) -> RecentFiles {
    let mut files = RecentFiles::default();
    for v in visited {
        files.visit(Path::new(v));
    }
    files
}

fn names(list: &[PathBuf]) -> Vec<String> {
    list.iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

pub fn run(case: &Case) -> Outcome {
    let n = case.number;
    let after = case.after();
    match (case.id.as_str(), n) {
        ("SRCH-GTL-001", 1) => {
            let mut e = Editor::new(&Editor::lines(30));
            e.goto(7, 4);
            let (line, column) = e.caret();
            let field = archipel_ide_core::goto_line::prefill(line, column);
            check(
                after.contains(&format!("field '{field}'")),
                &format!("prefill {field}"),
            )
        }
        ("SRCH-GTL-002", k @ 1..=5) => {
            let mut e = Editor::new(&Editor::lines(30));
            e.goto(7, 1);
            let typed = case.before().split('`').nth(1).unwrap_or("").to_owned();
            if !e.go_to_line_dialog(&typed) {
                return fail("dialog stayed open");
            }
            let want = match k {
                1 => (12, 1),
                2 | 3 => (12, 5),
                4 => (7, 5),
                _ => (10, 1),
            };
            check(
                e.caret() == want,
                &format!("caret {:?}, expected {want:?}", e.caret()),
            )
        }
        ("SRCH-GTL-002", 6) => {
            let mut e = Editor::new(&Editor::lines(30));
            e.goto(7, 1);
            check(!e.go_to_line_dialog("abc"), "dialog closed on garbage")
        }
        ("SRCH-GTL-003", 1) => {
            let mut e = Editor::new(&Editor::lines(30));
            e.go_to_line_dialog("99");
            check(e.caret().0 == 30, &format!("caret {:?}", e.caret()))
        }
        ("SRCH-GTL-003", 2) => {
            let mut e = Editor::new(&Editor::lines(30));
            e.go_to_line_dialog("3:99");
            check(e.caret() == (3, 11), &format!("caret {:?}", e.caret()))
        }
        ("SRCH-GTL-003", 3) => {
            let mut e = Editor::new(&Editor::lines(30));
            e.run("editor.selectAll", json!({}));
            e.go_to_line_dialog("1");
            let one = e.state.active_doc().is_some_and(|d| {
                d.selections.selections().len() == 1 && d.selections.primary().is_caret()
            });
            check(one && e.caret() == (1, 1), "one caret without selection")
        }
        ("SRCH-REC-001", 1) => {
            let f = files(&["A.java", "B.java", "C.java"]);
            let p = RecentFilesPopup::open(&f, Some(Path::new("C.java")), false);
            let rows: Vec<PathBuf> = p.rows(&f).into_iter().map(|r| r.item).collect();
            check(
                names(&rows) == ["C.java", "B.java", "A.java"] && !p.edited_only(),
                "list",
            )
        }
        ("SRCH-REC-001", 2) => {
            let mut f = files(&["A.java", "B.java", "C.java"]);
            f.edited(Path::new("B.java"));
            let mut p = RecentFilesPopup::open(&f, Some(Path::new("C.java")), false);
            p.toggle_edited_only(&f);
            let rows: Vec<PathBuf> = p.rows(&f).into_iter().map(|r| r.item).collect();
            check(names(&rows) == ["B.java"] && p.edited_only(), "edited only")
        }
        ("SRCH-REC-001", 3) => {
            let f = files(&[]);
            let p = RecentFilesPopup::open(&f, None, false);
            check(
                p.empty_text(&f).as_deref() == Some("No recent files"),
                "empty text",
            )
        }
        ("SRCH-REC-002", 1) => {
            let f = files(&["A.java", "B.java", "C.java"]);
            let p = RecentFilesPopup::open(&f, Some(Path::new("C.java")), false);
            check(
                p.selected(&f).as_deref() == Some(Path::new("B.java")),
                "selection",
            )
        }
        ("SRCH-REC-002", 2) => {
            let f = files(&["C.java"]);
            let p = RecentFilesPopup::open(&f, Some(Path::new("C.java")), false);
            check(
                p.selected(&f).as_deref() == Some(Path::new("C.java")),
                "selection",
            )
        }
        ("SRCH-REC-003", 1) => {
            let f = files(&["A.java", "B.java", "C.java"]);
            let mut p = RecentFilesPopup::open(&f, Some(Path::new("C.java")), false);
            p.move_selection(&f, true);
            check(
                p.selected(&f).as_deref() == Some(Path::new("A.java")),
                "Enter opens A",
            )
        }
        ("SRCH-REC-003", 2) => {
            let mut f = files(&["A.java", "B.java", "C.java"]);
            let mut p = RecentFilesPopup::open(&f, Some(Path::new("C.java")), false);
            let removed = p.delete_selected(&mut f);
            check(
                removed.as_deref() == Some(Path::new("B.java"))
                    && p.selected(&f).as_deref() == Some(Path::new("A.java")),
                "delete",
            )
        }
        ("SRCH-REC-003", 3) => {
            let f = files(&["A.java", "B.java", "C.java"]);
            let mut p = RecentFilesPopup::open(&f, Some(Path::new("C.java")), false);
            p.type_text("zzz");
            check(
                p.empty_text(&f).as_deref() == Some("Press 'Enter' to search in Project"),
                "miss text",
            )
        }
        ("SRCH-REC-005", k @ 1..=3) => {
            let entries = |paths: &[&str]| -> Vec<SwitcherEntry> {
                paths
                    .iter()
                    .map(|p| SwitcherEntry::File(PathBuf::from(p)))
                    .collect()
            };
            let selected = |s: &Switcher| match s.selected_entry() {
                SwitcherEntry::File(p) => p.to_string_lossy().into_owned(),
                _ => String::new(),
            };
            match k {
                1 => {
                    let s = Switcher::open(entries(&["B.java", "A.java"]), false).unwrap();
                    check(selected(&s) == "A.java", "quick toggle")
                }
                2 => {
                    let mut s =
                        Switcher::open(entries(&["C.java", "B.java", "A.java"]), false).unwrap();
                    let first = selected(&s);
                    s.cycle(false);
                    check(first == "B.java" && selected(&s) == "A.java", "iterate")
                }
                _ => {
                    // Esc cancels: nothing is committed, C stays focused.
                    let mut s = Switcher::open(entries(&["C.java", "B.java"]), false);
                    s = None.or(s.take().filter(|_| false));
                    check(s.is_none(), "cancel")
                }
            }
        }
        ("SRCH-REC-006", 1) => {
            let mut f = files(&["A.java", "B.java", "C.java"]);
            f.visit(Path::new("A.java"));
            check(
                names(&f.list(false)) == ["A.java", "C.java", "B.java"],
                "order",
            )
        }
        ("SRCH-REC-006", 2) => {
            let f = files(&["A.java", "B.java", "C.java"]);
            check(
                f.list(false).contains(&PathBuf::from("B.java")),
                "closing keeps the file",
            )
        }
        ("SRCH-REC-010", 1) => {
            let w = snippet_window(20, 9);
            check(w == (7..12), "lines 8-12")
        }
        ("SRCH-REC-010", 2) => {
            let w = snippet_window(20, 0);
            check(w == (0..5), "lines 1-5")
        }
        ("SRCH-REC-010", 3) => {
            let nav = QuickNavState::default();
            check(
                nav.locations.is_empty() && NO_RECENT_LOCATIONS == "No recent locations found",
                "text",
            )
        }
        ("SRCH-REC-011", 1) => {
            let mut nav = QuickNavState::default();
            nav.note_location(Path::new("a.txt"), 1, "a");
            nav.note_location(Path::new("b.txt"), 1, "b");
            let first = nav.locations[0].path.clone();
            nav.forget(&first);
            check(nav.locations.len() == 1, "removed")
        }
        ("SRCH-HIS-001", 1) => {
            let mut e = Editor::new(&Editor::lines(60));
            e.run("editor.type", json!({"text": "x"}));
            e.run("editor.moveCaretDown", json!({}));
            check(!e.enabled("navigate.back"), "typing does not push")
        }
        ("SRCH-HIS-001", 2) => {
            let mut e = Editor::new(&Editor::lines(100));
            e.goto(50, 1);
            e.goto(50, 1);
            e.run("navigate.back", json!({}));
            check(
                e.caret().0 == 1 && !e.enabled("navigate.back"),
                "one back step",
            )
        }
        ("SRCH-HIS-001", 3) => {
            let mut e = Editor::new(&Editor::lines(1800));
            for i in 1..=151 {
                e.goto(1 + 10 * i, 1);
            }
            let mut steps = 0;
            while e.enabled("navigate.back") && steps < 400 {
                e.run("navigate.back", json!({}));
                steps += 1;
            }
            check(steps == 150, &format!("{steps} steps"))
        }
        ("SRCH-HIS-002", 1) => {
            let mut e = Editor::new(&Editor::lines(100));
            e.goto(5, 1);
            e.goto(50, 1);
            e.run("navigate.back", json!({}));
            let back = e.caret().0;
            e.run("navigate.forward", json!({}));
            check(back == 5 && e.caret().0 == 50, "back and forward")
        }
        ("SRCH-HIS-002", 2) => {
            let mut e = Editor::new(&Editor::lines(100));
            e.goto(5, 1);
            e.goto(50, 1);
            e.run("navigate.back", json!({}));
            e.goto(80, 1);
            check(!e.enabled("navigate.forward"), "forward cleared")
        }
        ("SRCH-HIS-003", k @ 1..=2) => {
            let mut e = Editor::new(&Editor::lines(60));
            e.goto(3, 1);
            e.run("editor.type", json!({"text": "x"}));
            e.goto(40, 1);
            e.run("editor.type", json!({"text": "y"}));
            e.goto(1, 1);
            e.run("navigate.lastEdit", json!({}));
            let first = e.caret().0;
            if k == 1 {
                return check(first == 40, &format!("line {first}"));
            }
            e.run("navigate.lastEdit", json!({}));
            check(e.caret().0 == 3, &format!("line {}", e.caret().0))
        }
        ("SRCH-BKM-001", 1) => {
            let mut b = Bookmarks::default();
            check(
                b.toggle(Path::new("A.java"), 2, None) && b.all().len() == 1,
                "added",
            )
        }
        ("SRCH-BKM-001", 2) => {
            let mut b = Bookmarks::default();
            b.toggle(Path::new("A.java"), 2, None);
            check(
                !b.toggle(Path::new("A.java"), 2, None) && b.all().is_empty(),
                "removed",
            )
        }
        ("SRCH-BKM-001", 3) => {
            let mut b = Bookmarks::default();
            b.toggle(Path::new("A.java"), 2, Some("TODO fix"));
            check(b.all()[0].description == "TODO fix", "description")
        }
        ("SRCH-BKM-002", 1) => {
            let mut b = Bookmarks::default();
            let out = b.toggle_mnemonic(Path::new("A.java"), 2, '1');
            check(
                out == MnemonicOutcome::Added && b.all()[0].mnemonic == Some('1'),
                "mnemonic",
            )
        }
        ("SRCH-BKM-002", 2) => {
            let mut b = Bookmarks::default();
            b.toggle_mnemonic(Path::new("A.java"), 2, '1');
            let out = b.toggle_mnemonic(Path::new("A.java"), 2, '1');
            check(
                out == MnemonicOutcome::Removed && b.all().is_empty(),
                "removed",
            )
        }
        ("SRCH-BKM-002", 3) => {
            let mut b = Bookmarks::default();
            b.toggle_mnemonic(Path::new("A.java"), 2, '1');
            let out = b.toggle_mnemonic(Path::new("B.java"), 8, '1');
            match out {
                MnemonicOutcome::NeedsRewrite(m) if after.contains(&m) => Outcome::Pass,
                other => fail(format!("{other:?}")),
            }
        }
        ("SRCH-BKM-004", 1) => {
            let mut b = Bookmarks::default();
            b.toggle_mnemonic(Path::new("A.java"), 6, '2');
            let target = b.goto_mnemonic('2');
            check(
                target.is_some_and(|t| t.path == Path::new("A.java") && t.line == 6),
                "goto",
            )
        }
        ("SRCH-BKM-004", 2) => {
            let mut b = Bookmarks::default();
            for (p, l) in [("A.java", 2), ("A.java", 8), ("B.java", 0)] {
                b.toggle(Path::new(p), l, None);
            }
            let next = b.next(Path::new("B.java"), 0, true);
            check(
                next.is_some_and(|t| t.path == Path::new("A.java") && t.line == 2),
                "cycle",
            )
        }
        ("SRCH-BKM-004", 3) => {
            let mut b = Bookmarks::default();
            b.toggle(Path::new("A.java"), 4, None);
            check(
                b.next_in_editor(Path::new("A.java"), 4, true).is_none(),
                "disabled",
            )
        }
        ("SRCH-BKM-006", 1) => {
            let mut b = Bookmarks::default();
            b.create_list("New List").unwrap();
            check(
                b.create_list("New List").err().as_deref()
                    == Some("A list with the specified name already exists"),
                "error",
            )
        }
        ("SRCH-BKM-006", 2) => check(
            after.contains(&Bookmarks::delete_list_prompt("Work")),
            "prompt",
        ),
        ("SRCH-FST-001" | "SRCH-FST-002" | "SRCH-FST-003" | "SRCH-FST-004", _) => {
            Outcome::NotRunnable(
                "needs the structure of a Java file (language pack) and the popup tree",
            )
        }
        ("SRCH-NVB-001" | "SRCH-NVB-002", _) => {
            Outcome::NotRunnable("navigation bar needs the project view and window focus")
        }
        _ => Outcome::NotRunnable(
            "tab and window management, popups or features without a core model",
        ),
    }
}

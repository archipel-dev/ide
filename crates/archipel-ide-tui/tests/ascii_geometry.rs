//! TUI-GUI-131, 132 and 195: the ASCII glyph mode replaces each Nerd Font glyph by its ASCII
//! fallback and keeps every layout width; glyphs take exactly one cell.
//! Spec: TUI-GUI-131, TUI-GUI-132, TUI-GUI-195 (tests).

mod common;

use archipel_ide_tui::{StartOptions, TuiApp};
use common::{Project, Tui};
use unicode_width::UnicodeWidthStr;

fn is_private_use(c: char) -> bool {
    matches!(c as u32, 0xE000..=0xF8FF | 0xF0000..=0x10FFFF)
}

fn screen_of(ascii: bool, size: (u16, u16)) -> (Project, Vec<Vec<String>>) {
    let project = Project::new(
        &format!(
            "ascii-{}-{}x{}",
            if ascii { "b" } else { "a" },
            size.0,
            size.1
        ),
        &[
            ("build.gradle", "plugins { id 'java' }\n"),
            ("src/Main.java", "class Main {}\n"),
            ("README.md", "# x\n"),
        ],
    );
    let mut app = TuiApp::new(StartOptions {
        project: Some(project.root.clone()),
        files: vec!["src/Main.java".into(), "README.md".into()],
        isolated: true,
        ..StartOptions::default()
    });
    app.set_ascii_glyphs(ascii);
    let mut tui = Tui::with_app(app, size.0, size.1);
    tui.advance(0.4);
    tui.advance(0.1);
    let buffer = tui.buffer();
    let rows = (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect()
        })
        .collect();
    (project, rows)
}

#[test]
fn ascii_mode_has_no_private_use_glyphs_and_the_same_geometry() {
    for size in [(160, 45), (100, 30), (80, 24)] {
        let (_a, nerd) = screen_of(false, size);
        let (_b, ascii) = screen_of(true, size);
        let mut glyph_cells = 0;
        for (y, (nerd_row, ascii_row)) in nerd.iter().zip(&ascii).enumerate() {
            assert_eq!(nerd_row.len(), ascii_row.len());
            for (x, (n, a)) in nerd_row.iter().zip(ascii_row).enumerate() {
                if n.chars().any(is_private_use) {
                    glyph_cells += 1;
                    assert!(
                        !a.chars().any(is_private_use),
                        "{size:?} ({x},{y}) is still a Nerd Font glyph in ASCII mode: {a:?}"
                    );
                    assert!(
                        a.width() <= 1,
                        "{size:?} ({x},{y}): {a:?} is wider than a cell"
                    );
                } else if y + 1 < nerd.len()
                    && a != n
                    && !(n.chars().all(char::is_alphanumeric)
                        && a.chars().all(char::is_alphanumeric))
                {
                    // (The two fixtures have project folders that differ in one character; the
                    // last row has the indexing progress, which is not deterministic.)
                    // Text, boxes and blanks do not move: only the animated spinner and
                    // the glyph substitutes differ.
                    assert!(
                        n.chars().all(|c| !c.is_alphanumeric()) && a.is_ascii(),
                        "{size:?} ({x},{y}): {n:?} became {a:?}"
                    );
                }
            }
        }
        assert!(
            glyph_cells > 20,
            "{size:?}: only {glyph_cells} glyph cells compared"
        );
    }
}

#[test]
fn glyphs_occupy_one_cell() {
    let (_project, rows) = screen_of(false, (160, 45));
    for (y, row) in rows.iter().enumerate() {
        for (x, symbol) in row.iter().enumerate() {
            if symbol.chars().any(is_private_use) {
                assert_eq!(symbol.chars().count(), 1, "({x},{y}) {symbol:?}");
                assert!(symbol.width() <= 1, "({x},{y}) {symbol:?}");
            }
        }
    }
}

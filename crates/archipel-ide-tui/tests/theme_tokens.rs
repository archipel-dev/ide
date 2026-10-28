//! TUI-GUI-120, 121 and 195: the colors of the chrome are the tokens of `Theme::colors`,
//! blended over the surface they are drawn on, and the Settings theme re-themes live.

mod common;

use archipel_ide_tui::{StartOptions, TuiApp};
use archipel_kit_tui::ratatui::style::Color;
use common::{Project, Tui};
use serde_json::json;

fn open(name: &str, width: u16, height: u16) -> (Project, Tui) {
    let project = Project::new(
        name,
        &[("a.java", "class A {}\n"), ("b.java", "class B {}\n")],
    );
    let app = TuiApp::new(StartOptions {
        project: Some(project.root.clone()),
        files: vec!["a.java".into(), "b.java".into()],
        isolated: true,
        ..StartOptions::default()
    });
    let mut tui = Tui::with_app(app, width, height);
    tui.advance(0.3);
    (project, tui)
}

/// The first cell of row `y` that is blank, from the right.
fn blank_in_row(tui: &Tui, y: u16, from: u16, to: u16) -> (u16, u16) {
    for x in (from..to).rev() {
        if tui.cell(x, y).symbol() == " " {
            return (x, y);
        }
    }
    panic!("no blank cell in row {y}\n{}", tui.text());
}

#[test]
fn surfaces_use_the_frame_island_and_popup_tokens() {
    let (_project, mut tui) = open("tokens-surfaces", 160, 45);
    let geo = tui.app.shell_mut().geo.clone();
    let colors = tui.rt.look.theme.colors;
    let look = tui.rt.look.clone();
    let frame = look.over(colors.frame, colors.frame);
    let island = look.over(colors.island, colors.island);
    let bg = |tui: &Tui, (x, y): (u16, u16)| tui.cell(x, y).bg;

    // Toolbar, stripes and status bar are the frame.
    let toolbar = blank_in_row(&tui, geo.toolbar.y, 60, 120);
    assert_eq!(bg(&tui, toolbar), frame, "toolbar");
    let stripe = (geo.left_stripe.x + 1, geo.left_stripe.bottom() - 6);
    assert_eq!(bg(&tui, stripe), frame, "left stripe");
    let status = blank_in_row(&tui, geo.status.y, 0, 160);
    assert_eq!(bg(&tui, status), frame, "status bar");
    // Tool windows, the editor and the tab strip are the island.
    let left = geo.left.expect("the project window is open");
    assert_eq!(
        bg(&tui, (left.body.x + 4, left.body.bottom() - 3)),
        island,
        "tree body"
    );
    assert_eq!(
        bg(&tui, (left.header.x + 20, left.header.y)),
        island,
        "header"
    );
    assert_eq!(
        bg(&tui, (geo.editor.x + 40, geo.editor.bottom() - 3)),
        island,
        "editor"
    );
    assert_eq!(
        bg(&tui, (geo.tabs.right() - 2, geo.tabs.y)),
        island,
        "tab strip"
    );
}

#[test]
fn tabs_and_accent_use_their_tokens() {
    let (_project, mut tui) = open("tokens-tabs", 160, 45);
    let geo = tui.app.shell_mut().geo.clone();
    let colors = tui.rt.look.theme.colors;
    let look = tui.rt.look.clone();
    // The selected tab: tab_selected fill with the tab_selected_border underline.
    let (x, y) = tui.find("b.java").expect("the selected tab");
    let cell = tui.cell(x + 2, y);
    assert_eq!(
        cell.bg,
        look.over(colors.tab_selected, colors.island),
        "selected tab fill"
    );
    assert_eq!(
        cell.underline_color,
        look.over(colors.tab_selected_border, colors.tab_selected),
        "selected tab underline"
    );
    // The other tab sits on the island with the tab text color.
    let (x, y) = tui.find("a.java").expect("the other tab");
    let cell = tui.cell(x + 2, y);
    assert_eq!(
        cell.bg,
        look.over(colors.island, colors.island),
        "other tab"
    );
    assert_eq!(
        cell.fg,
        look.over(colors.tab_text, colors.island),
        "other tab text"
    );
    // The splitter between the tree and the editor is the separator color.
    let left = geo.left.expect("the project window");
    let split = geo
        .splitters
        .iter()
        .find(|(_, r)| r.x == left.rect.right())
        .map(|(_, r)| *r)
        .expect("a splitter next to the tree");
    assert_eq!(
        tui.cell(split.x, split.y + 5).fg,
        look.over(colors.separator, colors.island),
        "splitter"
    );
}

#[test]
fn popups_use_the_popup_token_and_its_border() {
    let (_project, mut tui) = open("tokens-popup", 160, 45);
    let colors = tui.rt.look.theme.colors;
    let look = tui.rt.look.clone();
    tui.app.menu_control().expand();
    tui.draw();
    let menu = tui.app.built_menu().expect("a frame").clone();
    tui.app.menu_control().open_menu(&tui.rt, &menu, 0, true);
    tui.draw();
    let rect = tui
        .app
        .menu_control()
        .popup()
        .and_then(|p| p.chain.rect(0))
        .expect("the File menu is open");
    // A row's fill: the popup surface (the preselected row has the selection color).
    let row = (rect.x + rect.width / 2, rect.bottom() - 3);
    assert_eq!(
        tui.cell(row.0, row.1).bg,
        look.over(colors.popup, colors.popup),
        "popup fill"
    );
    assert_eq!(
        tui.cell(rect.x, rect.y + 4).fg,
        look.over(colors.popup_border, colors.popup),
        "popup border"
    );
}

#[test]
fn a_translucent_token_is_blended_over_its_surface() {
    // Theme colors are premultiplied (egui's Color32): the cell is `over + under * (1 - a)`.
    let (_project, tui) = open("tokens-blend", 160, 45);
    let colors = tui.rt.look.theme.colors;
    assert!(
        colors.separator.a() < 255,
        "the dark separator is translucent"
    );
    let blended = tui.rt.look.over(colors.separator, colors.island);
    let Color::Rgb(r, g, b) = blended else {
        panic!("{blended:?} is not 24-bit");
    };
    let island = colors.island;
    let a = f32::from(colors.separator.a()) / 255.0;
    let mix = |over: u8, under: u8| (f32::from(over) + f32::from(under) * (1.0 - a)).round() as u8;
    assert_eq!(
        (r, g, b),
        (
            mix(colors.separator.r(), island.r()),
            mix(colors.separator.g(), island.g()),
            mix(colors.separator.b(), island.b())
        )
    );
}

#[test]
fn the_settings_theme_retheme_the_chrome_live() {
    let (_project, mut tui) = open("tokens-live", 160, 45);
    let dark_frame = tui.rt.look.theme.colors.frame;
    tui.app
        .ide_mut()
        .dispatch(
            "settings.set",
            json!({"id": "appearance.theme", "value": "Islands Light"}),
        )
        .expect("the setting exists");
    tui.advance(0.1);
    let colors = tui.rt.look.theme.colors;
    assert_ne!(colors.frame, dark_frame, "the theme changed");
    let look = tui.rt.look.clone();
    let geo = tui.app.shell_mut().geo.clone();
    let toolbar = blank_in_row(&tui, geo.toolbar.y, 60, 120);
    assert_eq!(
        tui.cell(toolbar.0, toolbar.1).bg,
        look.over(colors.frame, colors.frame),
        "the toolbar follows the light frame"
    );
}

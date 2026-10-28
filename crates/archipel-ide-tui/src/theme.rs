//! The theme of the terminal: the Settings theme (the same setting as the GUI, default
//! Islands Dark) resolved to a kit `Theme`, as `look` in 24-bit colors (TUI-GUI-006, 120).

use archipel_ide_core::IdeState;
use archipel_kit_tui::{Look, Theme};

/// The default theme.
pub const DEFAULT_THEME: &str = "Islands Dark";

/// The kit theme for the theme name `name`: Islands Dark and Light are the kit's measured
/// themes, the others the generated IntelliJ ones; an unknown name is the default.
pub fn resolve(name: &str) -> Theme {
    let all = Theme::all();
    let find = |wanted: &str| {
        all.iter()
            .find(|info| info.name == wanted || info.id == wanted)
    };
    let info = find(name).or_else(|| find(DEFAULT_THEME));
    let mut theme = match info.map(|i| i.id) {
        Some("islands_light") => Theme::light(),
        Some("islands_dark") | None => Theme::dark(),
        Some(_) => info.map_or_else(Theme::dark, |i| (i.constructor)()),
    };
    if let Some(info) = info {
        theme.name = info.name.to_owned();
    }
    theme
}

/// The name of the theme chosen in Settings.
pub fn setting(state: &IdeState) -> String {
    state
        .settings()
        .read()
        .text(archipel_ide_settings::ids::THEME)
        .unwrap_or_else(|| DEFAULT_THEME.to_owned())
}

/// The look for the theme named `name`.
pub fn look(name: &str, ascii: bool) -> Look {
    Look::gui(resolve(name)).with_ascii(ascii)
}

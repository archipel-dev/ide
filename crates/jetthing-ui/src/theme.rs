//! "Islands" look: color tokens, fonts and egui visuals.
//!
//! Color tokens resolve against the active theme (Islands Dark or Islands Light) from
//! `docs/spec/data/themes.json` and `docs/spec/data/color-schemes.json`; see
//! `theme_engine` for the keys behind each token.
//! Fonts are Inter and JetBrains Mono, both bundled under the SIL Open Font License by
//! `archipel-kit`, which also owns the palette values, the layout measurements below and
//! the egui visuals: [`install`] and [`apply_settings`] hand the kit a `Theme` built from
//! the active palette.

use archipel_kit::tokens::{radius, size, typography};
use eframe::egui::{self, Color32, FontFamily};

// Workbench color tokens; values come from the active theme (see `theme_engine`).
pub use crate::theme_engine::Token;
pub use crate::theme_engine::tokens::{
    ACCENT, ADDED, BANNER_WARNING, BANNER_WARNING_BORDER, BORDER, COMPLETION_SELECTION, ERROR,
    FIELD, FRAME, HOVER, ICON, ISLAND, LINK, MODIFIED, MUTED, NOTIFICATION, POPUP, POPUP_BORDER,
    PRESSED, RUN_GREEN, SELECTION, SELECTION_DISABLED, STOP_RED, SUCCESS, TAB_SELECTED,
    TAB_SELECTED_BORDER, TEXT, TEXT_BRIGHT, TEXT_DISABLED, TEXT_INFO, TEXT_SECONDARY,
    TOOLBAR_SEPARATOR, UNTRACKED, WARNING,
};

/// Corner radius of an island (kit token).
pub const ISLAND_RADIUS: u8 = radius::ISLAND;
/// Margin on each side of an island: half of the kit's gap between islands.
pub const ISLAND_GAP: f32 = size::ISLAND_GAP / 2.0;

pub mod editor {
    //! Editor colors of the active theme's color scheme. Syntax colors are resolved
    //! by text-attribute key in `crate::syntax`, from the same scheme.
    use crate::theme_engine::Token;
    use crate::theme_engine::tokens as t;

    pub const BACKGROUND: Token = t::EDITOR_BACKGROUND;
    pub const TEXT: Token = t::EDITOR_TEXT;
    pub const CARET: Token = t::EDITOR_CARET;
    pub const CARET_ROW: Token = t::EDITOR_CARET_ROW;
    pub const LINE_NUMBER: Token = t::EDITOR_LINE_NUMBER;
    pub const LINE_NUMBER_CARET: Token = t::EDITOR_LINE_NUMBER_CARET;
    pub const SELECTION: Token = t::EDITOR_SELECTION;
    pub const BACKGROUND_PANEL: Token = t::EDITOR_BACKGROUND_PANEL;
    pub const SEARCH_RESULT: Token = t::EDITOR_SEARCH_RESULT;
    pub const RENAME_OCCURRENCE: Token = t::EDITOR_RENAME_OCCURRENCE;
    pub const MATCHED_BRACE: Token = t::EDITOR_MATCHED_BRACE;
    pub const INDENT_GUIDE: Token = t::EDITOR_INDENT_GUIDE;
    pub const ERROR_EFFECT: Token = t::EDITOR_ERROR_EFFECT;
    pub const WARNING_EFFECT: Token = t::EDITOR_WARNING_EFFECT;
    pub const BREAKPOINT_LINE: Token = t::EDITOR_BREAKPOINT_LINE;
    pub const EXECUTION_LINE: Token = t::EDITOR_EXECUTION_LINE;
    pub const INLINE_VALUE: Token = t::EDITOR_INLINE_VALUE;
    pub const WEAK_WARNING_EFFECT: Token = t::EDITOR_WEAK_WARNING_EFFECT;
    pub const ERROR_STRIPE: Token = t::EDITOR_ERROR_STRIPE;
    pub const WARNING_STRIPE: Token = t::EDITOR_WARNING_STRIPE;
}

pub mod layout {
    //! Layout tokens shared by every tool window, so panels line up with each other.
    //! They are the kit's (`archipel_kit::tokens`), re-exported under the IDE's names.

    use archipel_kit::tokens::size;

    /// Left/right inset of tool-window content.
    pub const CONTENT_INSET: f32 = size::CONTENT_INSET;
    /// List and tree row height.
    pub const ROW_HEIGHT: f32 = size::ROW_HEIGHT;
    /// Icon edge length.
    pub const ICON_SIZE: f32 = size::ICON;
    /// Gap between an icon and its text.
    pub const ICON_TEXT_GAP: f32 = size::ICON_TEXT_GAP;
    /// Tool-window title bar height.
    pub const TOOLBAR_HEIGHT: f32 = size::HEADER_HEIGHT;
    /// Square icon-button size in toolbars and headers.
    pub const ICON_BUTTON: f32 = size::ICON_BUTTON;
    /// Space before the first toolbar button so its glyph lines up with the content inset.
    pub const TOOLBAR_LEAD: f32 = CONTENT_INSET - (ICON_BUTTON - ICON_SIZE) / 2.0;
    /// Gap between toolbar items.
    pub const TOOLBAR_GAP: f32 = size::TOOLBAR_GAP;
    /// Vertical gap between stacked sections inside a tool window.
    pub const SECTION_SPACING: f32 = size::SECTION_SPACING;
    /// Padding inside dialog frames.
    pub const DIALOG_PADDING: f32 = size::DIALOG_PADDING;
}

/// Stable avatar color for a project name, like the project widget's badge (the kit's).
pub fn avatar_color(name: &str) -> Color32 {
    archipel_kit::theme::avatar_color(name)
}

/// Project badge initials: first letters of the first two words, else the first two letters.
pub fn project_initials(name: &str) -> String {
    let words: Vec<&str> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let initials: String = match words.as_slice() {
        [] => "?".into(),
        [single] => single.chars().take(2).collect(),
        [first, second, ..] => first
            .chars()
            .take(1)
            .chain(second.chars().take(1))
            .collect(),
    };
    initials.to_uppercase()
}

pub const UI_FONT_SIZE: f32 = typography::BODY;
pub const EDITOR_FONT_SIZE: f32 = typography::CODE;
pub const SEMIBOLD: &str = typography::SEMIBOLD;
/// Bold monospace family (completion matches).
pub const MONO_BOLD: &str = typography::MONO_BOLD;

pub fn mono_bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name(MONO_BOLD.into()))
}

pub fn semibold(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

/// Editor font: the Monospace text style, sized by Settings | Editor | Font.
pub fn editor_font(style: &egui::Style) -> egui::FontId {
    egui::TextStyle::Monospace.resolve(style)
}

fn line_height_id() -> egui::Id {
    egui::Id::new("editor_line_height")
}

/// Editor row height for `font`: `base` at the default 13 pt / 1.2 line height,
/// scaled by the configured size and line height.
pub fn editor_row_height(ctx: &egui::Context, font: &egui::FontId, base: f32) -> f32 {
    let line_height = ctx
        .data(|data| data.get_temp::<f32>(line_height_id()))
        .unwrap_or(DEFAULT_LINE_HEIGHT);
    (base * font.size / EDITOR_FONT_SIZE * line_height / DEFAULT_LINE_HEIGHT)
        .round()
        .max(8.0)
}

const DEFAULT_LINE_HEIGHT: f32 = 1.2;

fn pairing_id() -> egui::Id {
    egui::Id::new("editor_pairing")
}

/// Bracket/quote auto-insert from Settings | Editor | Code Editing.
pub fn pair_options(ctx: &egui::Context) -> jetthing_text::ops::PairOptions {
    ctx.data(|data| data.get_temp(pairing_id()))
        .unwrap_or_default()
}

/// Applies settings that change the look live: theme, editor color scheme, editor
/// font size and line height.
pub fn apply_settings(ctx: &egui::Context, settings: &jetthing_core::Settings) {
    use jetthing_settings::ids;
    let theme = settings
        .text(ids::THEME)
        .unwrap_or_else(|| crate::theme_engine::THEMES[0].into());
    let scheme = settings.text(ids::EDITOR_SCHEME).filter(|s| !s.is_empty());
    // Settings validate the names; an error leaves the previous theme active.
    if let Ok(true) = crate::theme_engine::set_active(&theme, scheme.as_deref()) {
        apply_kit_theme(ctx);
    }
    sync_window_theme(ctx);
    let line_height = settings
        .float(jetthing_settings::ids::EDITOR_LINE_HEIGHT)
        .map(|value| value as f32)
        .unwrap_or(DEFAULT_LINE_HEIGHT);
    ctx.data_mut(|data| data.insert_temp(line_height_id(), line_height));
    let pairing = jetthing_text::ops::PairOptions {
        brackets: settings
            .bool(jetthing_settings::ids::PAIR_BRACKETS)
            .unwrap_or(true),
        quotes: settings
            .bool(jetthing_settings::ids::PAIR_QUOTES)
            .unwrap_or(true),
    };
    ctx.data_mut(|data| data.insert_temp(pairing_id(), pairing));
    let size = settings
        .int(jetthing_settings::ids::EDITOR_FONT_SIZE)
        .map(|size| size as f32)
        .unwrap_or(EDITOR_FONT_SIZE);
    let current = ctx
        .style()
        .text_styles
        .get(&egui::TextStyle::Monospace)
        .map(|font| font.size);
    if current != Some(size) {
        ctx.style_mut(|style| {
            style
                .text_styles
                .insert(egui::TextStyle::Monospace, egui::FontId::monospace(size));
        });
    }
}

/// Installs the kit (fonts, spacing, egui visuals) with the active theme. Call once at
/// startup; the kit's cursor rules and components then work everywhere in the IDE.
pub fn install(ctx: &egui::Context) {
    archipel_kit::install(ctx, crate::theme_engine::kit_theme());
}

/// Applies the active palette's kit theme: components read it with
/// `archipel_kit::Theme::colors(ctx)`, and plain egui widgets follow its visuals.
fn apply_kit_theme(ctx: &egui::Context) {
    crate::theme_engine::kit_theme().apply(ctx);
}

/// Window appearance for a theme: dark NSAppearance (title bar, native dialogs)
/// for dark themes, light otherwise.
pub fn window_theme(dark: bool) -> egui::SystemTheme {
    if dark {
        egui::SystemTheme::Dark
    } else {
        egui::SystemTheme::Light
    }
}

/// Keeps the native window appearance in step with the active theme. Only
/// macOS needs it (its title bar and dialogs follow the window appearance);
/// elsewhere the OS controls are left alone.
fn sync_window_theme(ctx: &egui::Context) {
    if !cfg!(target_os = "macos") {
        return;
    }
    let wanted = window_theme(crate::theme_engine::active().dark);
    let id = egui::Id::new("window_theme");
    if ctx.data(|data| data.get_temp::<egui::SystemTheme>(id)) != Some(wanted) {
        ctx.data_mut(|data| data.insert_temp(id, wanted));
        ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(wanted));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn window_appearance_follows_the_theme() {
        assert_eq!(window_theme(true), egui::SystemTheme::Dark);
        assert_eq!(window_theme(false), egui::SystemTheme::Light);
    }
    use super::*;

    #[test]
    fn initials_follow_project_widget_rules() {
        assert_eq!(project_initials("jetthing"), "JE");
        assert_eq!(project_initials("my-cool_app"), "MC");
        assert_eq!(project_initials("---"), "?");
        assert_eq!(project_initials("ärger"), "ÄR");
    }

    #[test]
    fn avatar_color_is_stable() {
        assert_eq!(avatar_color("jetthing"), avatar_color("jetthing"));
    }
}

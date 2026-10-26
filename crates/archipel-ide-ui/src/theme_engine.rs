//! Theme engine: resolves the workbench color tokens from the kit theme.
//!
//! Every color the workbench paints is a [`Token`]. A token names a field of the kit's
//! `Colors` (workbench) or `EditorColors` (editor scheme). Tokens dereference to the active
//! theme's color, so `theme::TEXT` can be passed to egui wherever a color is expected and
//! switching the theme takes effect on the next frame without touching call sites.
//!
//! The kit (`archipel-kit`) owns every theme: the IntelliJ UI themes (`Theme::all()`) and
//! the editor color schemes (`EditorScheme::all()`). A [`Palette`] pairs one theme with one
//! scheme, builds the [`archipel_kit::Theme`] that is applied to the egui context and
//! resolves the tokens from it, so kit components and the IDE's own painting always agree.

use archipel_kit::{EditorScheme, Theme};
use eframe::egui::Color32;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, RwLock};

/// A semantic workbench color, resolved against the active theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token(pub(crate) usize);

impl std::ops::Deref for Token {
    type Target = Color32;

    fn deref(&self) -> &Color32 {
        &active().colors[self.0]
    }
}

impl From<Token> for Color32 {
    fn from(token: Token) -> Color32 {
        *token
    }
}

macro_rules! tokens {
    ($( $(#[$doc:meta])* $name:ident; )+) => {
        tokens!(@consts 0usize; $($(#[$doc])* $name;)+);
        /// Token names in index order.
        pub(crate) const NAMES: &[&str] = &[$(stringify!($name)),+];
    };
    (@consts $index:expr; $(#[$doc:meta])* $name:ident; $($rest:tt)*) => {
        $(#[$doc])*
        pub const $name: Token = Token($index);
        tokens!(@consts $index + 1usize; $($rest)*);
    };
    (@consts $index:expr;) => {};
}

/// Token constants re-exported by `theme` (workbench) and `theme::editor` (editor scheme).
pub mod tokens {
    use super::Token;

    tokens! {
        /// `MainWindow.background`, `MainToolbar.background`, `StatusBar.background`.
        FRAME;
        /// `ToolWindow.background`, `EditorTabs.background`.
        ISLAND;
        /// `Popup.background`.
        POPUP;
        /// `Popup.borderColor`.
        POPUP_BORDER;
        /// `*.foreground`.
        TEXT;
        /// Emphasized labels (`Gray12` dark, `text-default` light).
        TEXT_BRIGHT;
        /// `StatusBar.Widget.foreground`.
        TEXT_SECONDARY;
        /// `*.infoForeground`.
        TEXT_INFO;
        /// `*.disabledForeground`.
        TEXT_DISABLED;
        /// `ActionButton.hoverBackground`.
        HOVER;
        /// `ActionButton.pressedBackground`.
        PRESSED;
        /// `*.selectionBackground`.
        SELECTION;
        /// `ToolWindow.Button.selectedBackground`, `*.focusColor`.
        ACCENT;
        /// Disabled default button fill (`Gray5` dark, `control-bg-small` light).
        SELECTION_DISABLED;
        /// `Notification.background`.
        NOTIFICATION;
        /// `Component.borderColor`.
        BORDER;
        /// `TextField.background`.
        FIELD;
        /// `MainToolbar.separatorColor`.
        TOOLBAR_SEPARATOR;
        /// `RunWidget.runIconColor`.
        RUN_GREEN;
        /// `RunWidget.stopBackground`.
        STOP_RED;
        /// `Label.errorForeground`.
        ERROR;
        /// `Label.warningForeground`.
        WARNING;
        /// `Label.successForeground`.
        SUCCESS;
        /// `*.modifiedItemForeground`.
        MODIFIED;
        /// `Link.activeForeground` (links in documentation popups).
        LINK;
        /// Added file color (`FILESTATUS_ADDED`).
        ADDED;
        /// Untracked file color (`Red8` dark, `FILESTATUS_UNKNOWN` light).
        UNTRACKED;
        /// Ignored/deleted text (`Gray7`).
        MUTED;
        /// `CompletionPopup.selectionBackground`.
        COMPLETION_SELECTION;
        /// Default icon stroke (`Gray11` dark, `icon-default-stroke` light).
        ICON;
        /// `EditorTabs.underlinedTabBackground` (selected editor tab).
        TAB_SELECTED;
        /// `EditorTabs.underlinedBorderColor`.
        TAB_SELECTED_BORDER;
        /// `Banner.warningBackground` (editor notification banners).
        BANNER_WARNING;
        /// `Banner.warningBorderColor`.
        BANNER_WARNING_BORDER;
        /// Editor `TEXT` background.
        EDITOR_BACKGROUND;
        /// Editor `TEXT` foreground.
        EDITOR_TEXT;
        /// `CARET_COLOR`.
        EDITOR_CARET;
        /// `CARET_ROW_COLOR`.
        EDITOR_CARET_ROW;
        /// `LINE_NUMBERS_COLOR`.
        EDITOR_LINE_NUMBER;
        /// `LINE_NUMBER_ON_CARET_ROW_COLOR`.
        EDITOR_LINE_NUMBER_CARET;
        /// `SELECTION_BACKGROUND`.
        EDITOR_SELECTION;
        /// `Editor.SearchField.background`.
        EDITOR_BACKGROUND_PANEL;
        /// `TEXT_SEARCH_RESULT_ATTRIBUTES` background.
        EDITOR_SEARCH_RESULT;
        /// `IDENTIFIER_UNDER_CARET_ATTRIBUTES` background (rename preview of the occurrences).
        EDITOR_RENAME_OCCURRENCE;
        /// `MATCHED_BRACE_ATTRIBUTES` background.
        EDITOR_MATCHED_BRACE;
        /// `INDENT_GUIDE`.
        EDITOR_INDENT_GUIDE;
        /// `ERRORS_ATTRIBUTES` effect color.
        EDITOR_ERROR_EFFECT;
        /// `WARNING_ATTRIBUTES` effect (or error-stripe) color.
        EDITOR_WARNING_EFFECT;
        /// `BREAKPOINT_ATTRIBUTES` background (line tint of a breakpoint line).
        EDITOR_BREAKPOINT_LINE;
        /// `EXECUTIONPOINT_ATTRIBUTES` background (suspended line).
        EDITOR_EXECUTION_LINE;
        /// `DEBUGGER_INLINED_VALUES` foreground (inline variable values while suspended).
        EDITOR_INLINE_VALUE;
        /// `INFO_ATTRIBUTES` (weak warning) effect color.
        EDITOR_WEAK_WARNING_EFFECT;
        /// `ERRORS_ATTRIBUTES` error-stripe color.
        EDITOR_ERROR_STRIPE;
        /// `WARNING_ATTRIBUTES` error-stripe color.
        EDITOR_WARNING_STRIPE;
    }
}

/// A resolved theme.
#[derive(Debug, Clone)]
pub struct Palette {
    pub dark: bool,
    /// Id of the kit editor scheme the editor tokens (and `crate::syntax`) use.
    pub scheme: &'static str,
    pub colors: Vec<Color32>,
    /// The kit theme the egui context uses for this palette: the UI theme with the colors
    /// of the editor scheme.
    pub theme: Theme,
    /// Token names the kit theme has no color for (empty; asserted by the tests).
    #[cfg_attr(not(test), allow(dead_code))]
    pub missing: Vec<&'static str>,
}

/// The kit's value of a workbench token, if the kit has a counterpart.
fn kit_color(token: &str, c: &archipel_kit::theme::Colors) -> Option<Color32> {
    Some(match token {
        "FRAME" => c.frame,
        "ISLAND" => c.island,
        "POPUP" => c.popup,
        "POPUP_BORDER" => c.popup_border,
        "TEXT" => c.text,
        "TEXT_BRIGHT" => c.text_strong,
        "TEXT_SECONDARY" => c.text_secondary,
        "TEXT_INFO" => c.text_placeholder,
        "TEXT_DISABLED" => c.text_disabled,
        "HOVER" => c.hover,
        "PRESSED" => c.pressed,
        "SELECTION" => c.selection,
        "ACCENT" => c.accent,
        "SELECTION_DISABLED" => c.accent_disabled,
        "NOTIFICATION" => c.notification,
        "BORDER" => c.border,
        "FIELD" => c.field,
        "TOOLBAR_SEPARATOR" => c.separator,
        "RUN_GREEN" => c.run,
        "STOP_RED" => c.stop,
        "ERROR" => c.error,
        "WARNING" => c.warning,
        "SUCCESS" => c.success,
        "MODIFIED" => c.vcs_modified,
        "LINK" => c.link,
        "ADDED" => c.vcs_added,
        "UNTRACKED" => c.vcs_untracked,
        "MUTED" => c.vcs_ignored,
        "COMPLETION_SELECTION" => c.selection_popup,
        "ICON" => c.icon,
        "TAB_SELECTED" => c.tab_selected,
        "TAB_SELECTED_BORDER" => c.tab_selected_border,
        "BANNER_WARNING" => c.banner_warning.background,
        "BANNER_WARNING_BORDER" => c.banner_warning.border,
        _ => return None,
    })
}

/// An editor token's color in the kit theme's editor colors.
fn editor_color(token: &str, theme: &Theme) -> Option<Color32> {
    let e = &theme.editor;
    Some(match token {
        "EDITOR_BACKGROUND" => e.background,
        "EDITOR_TEXT" => e.text,
        "EDITOR_CARET" => e.caret,
        "EDITOR_CARET_ROW" => e.caret_row,
        "EDITOR_LINE_NUMBER" => e.line_number,
        "EDITOR_LINE_NUMBER_CARET" => e.line_number_caret,
        "EDITOR_SELECTION" => e.selection,
        "EDITOR_BACKGROUND_PANEL" => e.background_panel,
        "EDITOR_SEARCH_RESULT" => e.search_result,
        "EDITOR_RENAME_OCCURRENCE" => e.rename_occurrence,
        "EDITOR_MATCHED_BRACE" => e.matched_brace,
        "EDITOR_INDENT_GUIDE" => e.indent_guide,
        "EDITOR_ERROR_EFFECT" => e.error_effect,
        "EDITOR_WARNING_EFFECT" => e.warning_effect,
        "EDITOR_BREAKPOINT_LINE" => e.breakpoint_line,
        "EDITOR_EXECUTION_LINE" => e.execution_line,
        "EDITOR_INLINE_VALUE" => e.inline_value,
        "EDITOR_WEAK_WARNING_EFFECT" => e.weak_warning_effect,
        "EDITOR_ERROR_STRIPE" => e.error_stripe,
        "EDITOR_WARNING_STRIPE" => e.warning_stripe,
        _ => return None,
    })
}

/// The IntelliJ UI themes offered in Settings | Appearance, in display order. The kit's own
/// measured `Archipel Dark` and `Archipel Light` are the kit's base, not IntelliJ themes.
pub fn themes() -> impl Iterator<Item = &'static archipel_kit::theme::ThemeInfo> {
    Theme::all()
        .iter()
        .filter(|info| !info.id.starts_with("archipel_"))
}

/// The default theme.
pub const DEFAULT_THEME: &str = "Islands Dark";

/// Editor scheme names that older versions stored, mapped to the kit's.
fn scheme_alias(name: &str) -> &str {
    match name {
        "Light" => "New UI Light",
        "Dark" => "New UI Dark",
        "High contrast" => "High Contrast",
        other => other,
    }
}

fn find_scheme(name: &str) -> Option<EditorScheme> {
    let name = scheme_alias(name);
    EditorScheme::all()
        .iter()
        .copied()
        .find(|scheme| scheme.name() == name || scheme.id() == name)
}

fn find_theme(name: &str) -> Option<&'static archipel_kit::theme::ThemeInfo> {
    themes().find(|info| info.name == name || info.id == name)
}

/// The UI look the theme was made for (a suggestion for the look picker).
#[cfg(test)]
pub fn suggested_look(theme: &str) -> Option<archipel_kit::UiLook> {
    find_theme(theme).map(|info| info.look)
}

impl Palette {
    /// Resolves every token for `theme` (a name from [`themes`]) with the editor colors
    /// of `scheme` (default: the theme's own editor scheme).
    pub fn resolve(theme: &str, scheme: Option<&str>) -> Result<Self, String> {
        let info = find_theme(theme).ok_or_else(|| format!("theme `{theme}` is not offered"))?;
        // Islands Dark and Islands Light are the kit's hand-measured themes
        // (`Theme::dark()` / `light()`, the default look of Android Studio); the generated
        // IntelliJ data differs from the measurement in a few shades. The others are the
        // generated themes.
        let (mut kit, own_scheme) = match info.id {
            "islands_dark" => (Theme::dark(), "islands_dark"),
            "islands_light" => (Theme::light(), "new_ui_light"),
            _ => (
                (info.constructor)(),
                (info.constructor)().source.scheme.unwrap_or("default"),
            ),
        };
        kit.name = info.name.to_owned();
        let scheme = match scheme {
            Some(name) => find_scheme(name)
                .ok_or_else(|| format!("editor color scheme `{name}` is not offered"))?,
            None => {
                EditorScheme::by_id(own_scheme).ok_or("the theme's editor scheme is missing")?
            }
        };
        kit = kit.with_editor_scheme(scheme);
        let scheme_id = kit.editor_scheme().map_or("default", |scheme| scheme.id());
        let mut missing = Vec::new();
        let colors = tokens::NAMES
            .iter()
            .map(|name| {
                kit_color(name, &kit.colors)
                    .or_else(|| editor_color(name, &kit))
                    .unwrap_or_else(|| {
                        missing.push(*name);
                        Color32::MAGENTA
                    })
            })
            .collect();
        Ok(Self {
            dark: kit.dark,
            scheme: scheme_id,
            colors,
            theme: kit,
            missing,
        })
    }

    #[cfg(test)]
    pub fn get(&self, token: Token) -> Color32 {
        self.colors[token.0]
    }
}

type PaletteKey = (String, Option<String>);

/// Resolved palettes are built once per (theme, scheme) pair and kept for the process
/// lifetime, so tokens can hand out `'static` references.
static CACHE: LazyLock<Mutex<HashMap<PaletteKey, &'static Palette>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static ACTIVE: LazyLock<RwLock<&'static Palette>> = LazyLock::new(|| {
    RwLock::new(palette(DEFAULT_THEME, None).expect("the default theme is bundled in the kit"))
});

fn palette(theme: &str, scheme: Option<&str>) -> Result<&'static Palette, String> {
    let key = (theme.to_owned(), scheme.map(str::to_owned));
    let mut cache = CACHE.lock().unwrap_or_else(|poison| poison.into_inner());
    if let Some(palette) = cache.get(&key) {
        return Ok(palette);
    }
    let palette: &'static Palette = Box::leak(Box::new(Palette::resolve(theme, scheme)?));
    cache.insert(key, palette);
    Ok(palette)
}

/// The kit theme of the active palette, to apply to an egui context.
pub fn kit_theme() -> Theme {
    active().theme.clone()
}

pub fn active() -> &'static Palette {
    *ACTIVE.read().unwrap_or_else(|poison| poison.into_inner())
}

/// Activates `theme` with editor `scheme` (`None`: the theme's own scheme). Returns
/// whether the palette changed; unknown names are an error and keep the current one.
pub fn set_active(theme: &str, scheme: Option<&str>) -> Result<bool, String> {
    let next = palette(theme, scheme)?;
    let mut active = ACTIVE.write().unwrap_or_else(|poison| poison.into_inner());
    let changed = !std::ptr::eq(*active, next);
    *active = next;
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_resolves_every_token() {
        for info in themes() {
            let palette = Palette::resolve(info.name, None).unwrap();
            assert!(
                palette.missing.is_empty(),
                "{}: {:?}",
                info.name,
                palette.missing
            );
            assert_eq!(palette.dark, info.dark, "{}", info.name);
            assert_eq!(palette.get(tokens::FRAME), palette.theme.colors.frame);
            assert_eq!(
                palette.get(tokens::EDITOR_BACKGROUND),
                palette.theme.editor.background
            );
        }
    }

    #[test]
    fn every_scheme_pairs_with_a_theme() {
        for scheme in EditorScheme::all() {
            let palette = Palette::resolve(DEFAULT_THEME, Some(scheme.name())).unwrap();
            assert!(
                palette.missing.is_empty(),
                "{}: {:?}",
                scheme.name(),
                palette.missing
            );
            assert_eq!(palette.scheme, scheme.id());
        }
        let monokai = Palette::resolve(DEFAULT_THEME, Some("Monokai")).unwrap();
        let default = Palette::resolve(DEFAULT_THEME, None).unwrap();
        assert_ne!(
            monokai.get(tokens::EDITOR_BACKGROUND),
            default.get(tokens::EDITOR_BACKGROUND)
        );
    }

    #[test]
    fn schemes_stored_by_older_versions_still_resolve() {
        for old in [
            "Light",
            "Dark",
            "High contrast",
            "Islands Dark",
            "Darcula",
            "IntelliJ Light",
        ] {
            assert!(Palette::resolve(DEFAULT_THEME, Some(old)).is_ok(), "{old}");
        }
    }

    #[test]
    fn unknown_themes_are_rejected() {
        assert!(Palette::resolve("Solarized", None).is_err());
        assert!(Palette::resolve(DEFAULT_THEME, Some("Nope")).is_err());
        assert!(set_active("Solarized", None).is_err());
    }

    #[test]
    fn classic_themes_suggest_the_classic_look() {
        assert_eq!(
            suggested_look("Darcula"),
            Some(archipel_kit::UiLook::Classic)
        );
        assert_eq!(
            suggested_look("Islands Dark"),
            Some(archipel_kit::UiLook::Islands)
        );
        assert_eq!(
            suggested_look("New UI Dark"),
            Some(archipel_kit::UiLook::NewUi)
        );
    }
}

#[cfg(test)]
mod settings_tests {
    use super::*;
    use archipel_ide_settings::{SettingKind, SettingsRegistry, ids};

    fn option_ids(id: &str) -> Vec<String> {
        let registry = SettingsRegistry::intellij();
        match &registry.setting(id).unwrap().kind {
            SettingKind::Enum { options } => options.iter().map(|o| o.id.clone()).collect(),
            other => panic!("unexpected kind {other:?}"),
        }
    }

    #[test]
    fn settings_offer_exactly_the_kit_themes_schemes_and_looks() {
        let themes: Vec<_> = themes().map(|info| info.name.to_owned()).collect();
        assert_eq!(option_ids(ids::THEME), themes);
        let schemes = option_ids(ids::EDITOR_SCHEME);
        assert_eq!(schemes[0], "");
        let kit: Vec<_> = EditorScheme::all()
            .iter()
            .map(|s| s.name().to_owned())
            .collect();
        assert_eq!(&schemes[1..], kit);
        let looks: Vec<_> = archipel_kit::UiLook::ALL
            .iter()
            .map(|l| l.id().to_owned())
            .collect();
        assert_eq!(option_ids(ids::UI_LOOK), looks);
        assert_eq!(themes[0], DEFAULT_THEME);
    }
}

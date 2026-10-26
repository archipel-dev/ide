//! Resolves highlight attribute keys to egui text formats through the active
//! editor color scheme (the one the theme engine selected in Settings › Appearance).
//! The schemes are the kit's (`archipel_kit::EditorScheme`).

use archipel_kit::EditorScheme;
use archipel_kit::theme::{EffectKind, TextAttributes};
use eframe::egui::{self, Stroke};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Resolved attributes for `key` in the active scheme, cached per scheme and key.
pub fn attributes(key: &'static str) -> TextAttributes {
    attributes_in(crate::theme_engine::active().scheme, key)
}

fn none() -> TextAttributes {
    TextAttributes {
        foreground: None,
        background: None,
        bold: false,
        italic: false,
        effect: None,
        stripe: None,
    }
}

fn attributes_in(scheme_id: &'static str, key: &'static str) -> TextAttributes {
    type Cache = HashMap<(&'static str, &'static str), TextAttributes>;
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let resolve = || {
        EditorScheme::by_id(scheme_id)
            .and_then(|scheme| scheme.attributes(key))
            .unwrap_or_else(none)
    };
    match CACHE.get_or_init(Default::default).lock() {
        Ok(mut cache) => *cache.entry((scheme_id, key)).or_insert_with(resolve),
        Err(_) => resolve(),
    }
}

/// Applies the attributes of `key` on top of the editor's base text format.
pub fn text_format(key: &'static str, base: &egui::TextFormat) -> egui::TextFormat {
    let attributes = attributes(key);
    let mut format = base.clone();
    if let Some(foreground) = attributes.foreground {
        format.color = foreground;
    }
    if let Some(background) = attributes.background {
        format.background = background;
    }
    format.italics = attributes.italic;
    if let Some(effect) = attributes.effect
        && let Some(effect_color) = effect.color
    {
        let stroke = Stroke::new(1.0_f32, effect_color);
        match effect.kind {
            EffectKind::Strikeout => format.strikethrough = stroke,
            EffectKind::LineUnderscore
            | EffectKind::BoldLineUnderscore
            | EffectKind::BoldDottedLine => format.underline = stroke,
            // Wave underlines are painted by the diagnostics pass; boxes are not
            // supported by egui text formats.
            _ => {}
        }
    }
    format
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Color32;

    #[test]
    fn keys_resolve_to_islands_dark_formats() {
        // The active palette is process-global (app tests switch it), so resolve in a
        // named scheme and apply the attributes by hand.
        let keyword = attributes_in("islands_dark", "DEFAULT_KEYWORD");
        assert_eq!(
            keyword.foreground,
            Some(Color32::from_rgb(0xCF, 0x8E, 0x6D))
        );
        let doc = attributes_in("islands_dark", "DEFAULT_DOC_COMMENT");
        assert_eq!(doc.foreground, Some(Color32::from_rgb(0x5F, 0x82, 0x6B)));
        assert!(doc.italic);
        let base = egui::TextFormat::default();
        assert_eq!(text_format("UNKNOWN_KEY", &base).color, base.color);
    }

    #[test]
    fn other_schemes_resolve_their_own_colors() {
        // New UI Light: keyword 0033B3.
        assert_eq!(
            attributes_in("new_ui_light", "DEFAULT_KEYWORD").foreground,
            Some(Color32::from_rgb(0, 0x33, 0xB3))
        );
        assert!(
            attributes_in("monokai", "DEFAULT_KEYWORD")
                .foreground
                .is_some()
        );
    }
}

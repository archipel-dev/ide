//! Resolves highlight attribute keys to egui text formats through the active
//! editor color scheme (the one the theme engine selected in Settings › Appearance).

use archipel_ide_language::{ColorScheme, EffectType, Rgba, TextAttributes};
use eframe::egui::{self, Color32, Stroke};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn color(rgba: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(rgba.r, rgba.g, rgba.b, rgba.a)
}

/// Bundled scheme by name, parsed once (an unknown name falls back to Islands Dark).
fn scheme(name: &str) -> &'static ColorScheme {
    static SCHEMES: OnceLock<Mutex<HashMap<String, &'static ColorScheme>>> = OnceLock::new();
    let load = || -> &'static ColorScheme {
        Box::leak(Box::new(
            ColorScheme::bundled(name).unwrap_or_else(|_| ColorScheme::islands_dark()),
        ))
    };
    match SCHEMES.get_or_init(Default::default).lock() {
        Ok(mut schemes) => schemes.entry(name.to_owned()).or_insert_with(load),
        Err(_) => load(),
    }
}

/// Resolved attributes for `key` in the active scheme, cached per scheme and key.
pub fn attributes(key: &'static str) -> TextAttributes {
    attributes_in(&crate::theme_engine::active().scheme, key)
}

fn attributes_in(scheme_name: &str, key: &'static str) -> TextAttributes {
    type Cache = HashMap<(String, &'static str), TextAttributes>;
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Ok(mut cache) = cache.lock() {
        return *cache
            .entry((scheme_name.to_owned(), key))
            .or_insert_with(|| scheme(scheme_name).attributes(key));
    }
    scheme(scheme_name).attributes(key)
}

/// Applies the attributes of `key` on top of the editor's base text format.
pub fn text_format(key: &'static str, base: &egui::TextFormat) -> egui::TextFormat {
    let attributes = attributes(key);
    let mut format = base.clone();
    if let Some(foreground) = attributes.foreground {
        format.color = color(foreground);
    }
    if let Some(background) = attributes.background {
        format.background = color(background);
    }
    format.italics = attributes.font.italic;
    if let Some((effect, Some(effect_color))) = attributes.effect {
        let stroke = Stroke::new(1.0_f32, color(effect_color));
        match effect {
            EffectType::Strikeout => format.strikethrough = stroke,
            EffectType::Underline | EffectType::BoldUnderline | EffectType::DottedUnderline => {
                format.underline = stroke
            }
            // Wave underlines are painted by the diagnostics pass; boxes are not
            // supported by egui text formats.
            EffectType::WaveUnderline | EffectType::Boxed => {}
        }
    }
    format
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_resolve_to_islands_dark_formats() {
        let base = egui::TextFormat::default();
        assert_eq!(
            text_format("DEFAULT_KEYWORD", &base).color,
            Color32::from_rgb(0xCF, 0x8E, 0x6D)
        );
        let doc = text_format("DEFAULT_DOC_COMMENT", &base);
        assert_eq!(doc.color, Color32::from_rgb(0x5F, 0x82, 0x6B));
        assert!(doc.italics);
        assert_eq!(text_format("UNKNOWN_KEY", &base).color, base.color);
    }

    #[test]
    fn other_schemes_resolve_their_own_colors() {
        // Scheme "Light": keyword 0033B3.
        assert_eq!(
            attributes_in("Light", "DEFAULT_KEYWORD")
                .foreground
                .map(color),
            Some(Color32::from_rgb(0, 0x33, 0xB3))
        );
    }
}

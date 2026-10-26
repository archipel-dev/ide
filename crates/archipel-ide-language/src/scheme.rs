//! Editor color schemes: IntelliJ text attributes and editor colors, loaded
//! from the scheme data extracted into `docs/spec/data/color-schemes.json`.
//!
//! Schemes inherit through `lineage` (e.g. Default → Darcula → Islands Dark);
//! attributes may `inherit` another key, and keys without an explicit value
//! fall back along [`fallback_key`] (mirroring IntelliJ's `TextAttributesKey`
//! fallbacks to `DEFAULT_*` keys).

use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

const SCHEME_DATA: &str = include_str!("../../../docs/spec/data/color-schemes.json");

/// An sRGB color with alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Parses IntelliJ scheme hex values. Leading zeros are omitted in the
    /// source data (`ff` is blue), and an optional alpha may trail (`RRGGBBAA`).
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim().trim_start_matches('#');
        if value.is_empty() || value.len() > 8 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let number = u32::from_str_radix(value, 16).ok()?;
        if value.len() == 8 {
            let [r, g, b, a] = number.to_be_bytes();
            return Some(Self { r, g, b, a });
        }
        let [_, r, g, b] = number.to_be_bytes();
        Some(Self::rgb(r, g, b))
    }
}

/// Font style flags from `FONT_TYPE` (bit 0 bold, bit 1 italic).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FontStyle {
    pub bold: bool,
    pub italic: bool,
}

/// Text decorations from `EFFECT_TYPE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    Boxed,
    Underline,
    WaveUnderline,
    Strikeout,
    BoldUnderline,
    DottedUnderline,
}

impl EffectType {
    fn from_code(code: &str) -> Option<Self> {
        Some(match code.trim() {
            "0" => Self::Boxed,
            "1" => Self::Underline,
            "2" => Self::WaveUnderline,
            "3" => Self::Strikeout,
            "4" => Self::BoldUnderline,
            "5" => Self::DottedUnderline,
            _ => return None,
        })
    }
}

/// Fully resolved text attributes for one key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextAttributes {
    pub foreground: Option<Rgba>,
    pub background: Option<Rgba>,
    pub font: FontStyle,
    pub effect: Option<(EffectType, Option<Rgba>)>,
}

/// One attribute entry as written in a scheme. A child scheme's entry
/// replaces the parent's entry as a whole, as in IntelliJ.
#[derive(Debug, Clone, Default)]
struct RawAttributes {
    inherit: Option<String>,
    foreground: Option<String>,
    background: Option<String>,
    font_type: Option<String>,
    effect_type: Option<String>,
    effect_color: Option<String>,
}

impl RawAttributes {
    fn from_json(value: &Value) -> Self {
        let field = |name: &str| value.get(name).and_then(Value::as_str).map(str::to_owned);
        Self {
            inherit: value
                .get("inherit")
                .and_then(Value::as_str)
                .map(str::to_owned),
            foreground: field("FOREGROUND"),
            background: field("BACKGROUND"),
            font_type: field("FONT_TYPE"),
            effect_type: field("EFFECT_TYPE"),
            effect_color: field("EFFECT_COLOR"),
        }
    }

    fn is_empty(&self) -> bool {
        self.inherit.is_none()
            && self.foreground.is_none()
            && self.background.is_none()
            && self.font_type.is_none()
            && self.effect_type.is_none()
    }
}

/// Built-in key fallbacks for keys language mappings use, following the
/// `TextAttributesKey` fallback chains of the IntelliJ platform.
pub fn fallback_key(key: &str) -> Option<&'static str> {
    Some(match key {
        "DEFAULT_FUNCTION_CALL"
        | "DEFAULT_CLASS_REFERENCE"
        | "DEFAULT_LOCAL_VARIABLE"
        | "DEFAULT_PARAMETER"
        | "DEFAULT_INSTANCE_FIELD"
        | "DEFAULT_TAG" => "DEFAULT_IDENTIFIER",
        "DEFAULT_STATIC_FIELD" | "DEFAULT_GLOBAL_VARIABLE" => "DEFAULT_INSTANCE_FIELD",
        "DEFAULT_BLOCK_COMMENT" => "DEFAULT_LINE_COMMENT",
        "DEFAULT_DOC_COMMENT" => "DEFAULT_BLOCK_COMMENT",
        "DEFAULT_BRACES" | "DEFAULT_PARENTHS" | "DEFAULT_BRACKETS" | "DEFAULT_COMMA"
        | "DEFAULT_SEMICOLON" | "DEFAULT_DOT" => "DEFAULT_OPERATION_SIGN",
        "XML_TAG_NAME" | "HTML_TAG_NAME" => "DEFAULT_KEYWORD",
        "XML_ATTRIBUTE_NAME" | "HTML_ATTRIBUTE_NAME" => "DEFAULT_ATTRIBUTE",
        "PROPERTIES.KEY" => "DEFAULT_KEYWORD",
        "MARKDOWN_LINK_TEXT" | "MARKDOWN_AUTO_LINK" => "HYPERLINK_ATTRIBUTES",
        _ => return None,
    })
}

/// A resolved editor color scheme.
#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub name: String,
    pub dark: bool,
    colors: HashMap<String, Rgba>,
    attributes: HashMap<String, RawAttributes>,
}

/// Errors from loading scheme data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemeError {
    InvalidData(String),
    UnknownScheme(String),
}

impl std::fmt::Display for SchemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidData(message) => write!(f, "invalid color scheme data: {message}"),
            Self::UnknownScheme(name) => write!(f, "unknown color scheme `{name}`"),
        }
    }
}

impl std::error::Error for SchemeError {}

fn scheme_data() -> Result<&'static Value, SchemeError> {
    static DATA: OnceLock<Result<Value, String>> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(SCHEME_DATA).map_err(|error| error.to_string()))
        .as_ref()
        .map_err(|error| SchemeError::InvalidData(error.clone()))
}

impl ColorScheme {
    /// Names of the bundled schemes, in data order.
    pub fn bundled_names() -> Vec<String> {
        scheme_data()
            .ok()
            .and_then(|data| data.get("schemes"))
            .and_then(Value::as_object)
            .map(|schemes| schemes.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Loads a bundled scheme by name, merging its parent lineage.
    pub fn bundled(name: &str) -> Result<Self, SchemeError> {
        let schemes = scheme_data()?
            .get("schemes")
            .and_then(Value::as_object)
            .ok_or_else(|| SchemeError::InvalidData("missing `schemes`".into()))?;
        let entry = schemes
            .get(name)
            .ok_or_else(|| SchemeError::UnknownScheme(name.into()))?;
        let lineage: Vec<&str> = entry
            .get("lineage")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).collect())
            .unwrap_or_else(|| vec![name]);
        let mut scheme = Self {
            name: name.into(),
            dark: false,
            colors: HashMap::new(),
            attributes: HashMap::new(),
        };
        for ancestor in lineage {
            let Some(entry) = schemes.get(ancestor) else {
                continue;
            };
            if let Some(colors) = entry.get("colors").and_then(Value::as_object) {
                for (key, value) in colors {
                    match value.as_str().and_then(Rgba::parse) {
                        Some(color) => scheme.colors.insert(key.clone(), color),
                        None => scheme.colors.remove(key),
                    };
                }
            }
            if let Some(attributes) = entry.get("attributes").and_then(Value::as_object) {
                for (key, value) in attributes {
                    scheme
                        .attributes
                        .insert(key.clone(), RawAttributes::from_json(value));
                }
            }
        }
        scheme.dark = scheme.attributes("TEXT").background.is_some_and(|color| {
            u16::from(color.r) + u16::from(color.g) + u16::from(color.b) < 384
        });
        Ok(scheme)
    }

    /// The default scheme of the Islands Dark theme.
    pub fn islands_dark() -> Self {
        Self::bundled("Islands Dark").unwrap_or_else(|_| Self::empty("Islands Dark"))
    }

    fn empty(name: &str) -> Self {
        Self {
            name: name.into(),
            dark: true,
            colors: HashMap::new(),
            attributes: HashMap::new(),
        }
    }

    /// An editor color such as `CARET_ROW_COLOR` or `SELECTION_BACKGROUND`.
    pub fn color(&self, key: &str) -> Option<Rgba> {
        self.colors.get(key).copied()
    }

    /// Resolves the text attributes for `key`, following `inherit` references
    /// and the built-in fallback chain. Unknown keys resolve to empty attributes.
    pub fn attributes(&self, key: &str) -> TextAttributes {
        let mut current = key.to_owned();
        for _ in 0..16 {
            match self.attributes.get(&current) {
                Some(raw) if raw.inherit.is_some() => {
                    current = raw.inherit.clone().unwrap_or_default();
                }
                Some(raw) if !raw.is_empty() => return resolve(raw),
                _ => match fallback_key(&current) {
                    Some(next) => current = next.into(),
                    None => return TextAttributes::default(),
                },
            }
        }
        TextAttributes::default()
    }
}

fn resolve(raw: &RawAttributes) -> TextAttributes {
    let color = |value: &Option<String>| value.as_deref().and_then(Rgba::parse);
    let font_type = raw
        .font_type
        .as_deref()
        .and_then(|value| value.trim().parse::<u8>().ok())
        .unwrap_or(0);
    let effect = raw
        .effect_type
        .as_deref()
        .and_then(EffectType::from_code)
        .map(|effect| (effect, color(&raw.effect_color)))
        .filter(|(_, color)| color.is_some());
    TextAttributes {
        foreground: color(&raw.foreground),
        background: color(&raw.background),
        font: FontStyle {
            bold: font_type & 1 != 0,
            italic: font_type & 2 != 0,
        },
        effect,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fg(scheme: &ColorScheme, key: &str) -> Option<Rgba> {
        scheme.attributes(key).foreground
    }

    #[test]
    fn parses_short_and_alpha_hex() {
        assert_eq!(Rgba::parse("ff"), Some(Rgba::rgb(0, 0, 0xff)));
        assert_eq!(Rgba::parse("#cf8e6d"), Some(Rgba::rgb(0xcf, 0x8e, 0x6d)));
        assert_eq!(
            Rgba::parse("FFFFFF17"),
            Some(Rgba {
                r: 255,
                g: 255,
                b: 255,
                a: 0x17
            })
        );
        assert_eq!(Rgba::parse("xyz"), None);
        assert_eq!(Rgba::parse(""), None);
    }

    #[test]
    fn islands_dark_matches_spec_values() {
        let scheme = ColorScheme::bundled("Islands Dark").expect("bundled");
        assert!(scheme.dark);
        assert_eq!(fg(&scheme, "DEFAULT_KEYWORD"), Rgba::parse("cf8e6d"));
        assert_eq!(fg(&scheme, "DEFAULT_STRING"), Rgba::parse("6aab73"));
        assert_eq!(fg(&scheme, "DEFAULT_NUMBER"), Rgba::parse("2aacb8"));
        assert_eq!(fg(&scheme, "DEFAULT_LINE_COMMENT"), Rgba::parse("7a7e85"));
        assert_eq!(
            fg(&scheme, "DEFAULT_FUNCTION_DECLARATION"),
            Rgba::parse("56a8f5")
        );
        assert_eq!(fg(&scheme, "DEFAULT_METADATA"), Rgba::parse("b3ae60"));
        assert_eq!(fg(&scheme, "XML_TAG_NAME"), Rgba::parse("d5b778"));
        let doc = scheme.attributes("DEFAULT_DOC_COMMENT");
        assert_eq!(doc.foreground, Rgba::parse("5f826b"));
        assert!(doc.font.italic);
        let constant = scheme.attributes("DEFAULT_CONSTANT");
        assert_eq!(constant.foreground, Rgba::parse("c77dbb"));
        assert!(constant.font.italic);
        // `DEFAULT_FUNCTION_CALL` inherits `DEFAULT_IDENTIFIER`.
        assert_eq!(fg(&scheme, "DEFAULT_FUNCTION_CALL"), Rgba::parse("bcbec4"));
        let errors = scheme.attributes("ERRORS_ATTRIBUTES");
        assert_eq!(
            errors.effect,
            Some((EffectType::WaveUnderline, Rgba::parse("fa6675")))
        );
        assert_eq!(scheme.color("SELECTION_BACKGROUND"), Rgba::parse("214283"));
        assert_eq!(scheme.color("CARET_ROW_COLOR"), Rgba::parse("1F2024"));
    }

    #[test]
    fn light_scheme_resolves_and_is_light() {
        let scheme = ColorScheme::bundled("Light").expect("bundled");
        assert!(!scheme.dark);
        assert_eq!(fg(&scheme, "DEFAULT_KEYWORD"), Rgba::parse("0033b3"));
    }

    #[test]
    fn unknown_keys_and_schemes_are_handled() {
        let scheme = ColorScheme::islands_dark();
        assert_eq!(scheme.attributes("NOPE"), TextAttributes::default());
        assert!(matches!(
            ColorScheme::bundled("Nope"),
            Err(SchemeError::UnknownScheme(_))
        ));
        assert!(
            ColorScheme::bundled_names()
                .iter()
                .any(|name| name == "Darcula")
        );
    }
}

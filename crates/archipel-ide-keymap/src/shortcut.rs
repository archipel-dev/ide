//! Keystrokes, chords, mouse shortcuts and gestures in the reference IDE's notation.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    /// ⌘ on macOS, the Windows/Super key elsewhere.
    pub meta: bool,
}

impl Modifiers {
    pub const NONE: Modifiers = Modifiers {
        ctrl: false,
        alt: false,
        shift: false,
        meta: false,
    };

    pub fn is_empty(self) -> bool {
        self == Self::NONE
    }
}

/// One key press with modifiers. `key` is a canonical key name such as `A`, `F12`,
/// `Enter`, `Left`, `Page Down`, `NumPad *`, `/`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct KeyStroke {
    pub modifiers: Modifiers,
    pub key: String,
}

impl KeyStroke {
    pub fn new(modifiers: Modifiers, key: &str) -> Self {
        Self {
            modifiers,
            key: canonical_key(key),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Back,
    Forward,
    Other(u16),
}

/// Anything a keymap can bind to an action.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Shortcut {
    /// One keystroke or a two-stroke chord (`Ctrl+K, Ctrl+1`).
    Keys(Vec<KeyStroke>),
    Mouse {
        modifiers: Modifiers,
        button: MouseButton,
        double: bool,
    },
    /// Pressing a modifier twice (`double Shift`, `double control`).
    DoubleModifier(Modifier),
    /// Trackpad and test gestures that Archipel IDE does not trigger (`Force touch`).
    Gesture(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Meta,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("cannot parse shortcut `{input}`: {reason}")]
pub struct ParseError {
    pub input: String,
    pub reason: String,
}

fn parse_error(input: &str, reason: &str) -> ParseError {
    ParseError {
        input: input.into(),
        reason: reason.into(),
    }
}

/// Canonical spelling of a key name. Accepts the display names used by the reference
/// IDE (`Page Down`, `NumPad *`) and Java key code names (`back_space`, `close_bracket`).
pub fn canonical_key(key: &str) -> String {
    let trimmed = key.trim();
    let lower = trimmed.to_ascii_lowercase();
    let mapped = match lower.as_str() {
        "back_space" | "backspace" => "Backspace",
        "delete" => "Delete",
        "enter" | "return" => "Enter",
        "tab" => "Tab",
        "space" => "Space",
        "escape" | "esc" => "Esc",
        "insert" => "Insert",
        "home" => "Home",
        "end" => "End",
        "page_up" | "page up" | "pageup" => "Page Up",
        "page_down" | "page down" | "pagedown" => "Page Down",
        "up" => "Up",
        "down" => "Down",
        "left" => "Left",
        "right" => "Right",
        "close_bracket" => "]",
        "open_bracket" => "[",
        "back_slash" => "\\",
        "back_quote" | "dead_grave" => "`",
        "slash" => "/",
        "equals" => "=",
        "minus" => "-",
        "period" => ".",
        "comma" => ",",
        "semicolon" => ";",
        "quote" => "'",
        "add" => "NumPad +",
        "subtract" => "NumPad -",
        "multiply" => "NumPad *",
        "divide" => "NumPad /",
        "pause" => "Pause",
        "cancel" => "Cancel",
        "context menu" | "context_menu" => "Context Menu",
        _ => "",
    };
    if !mapped.is_empty() {
        return mapped.into();
    }
    if let Some(rest) = lower.strip_prefix("numpad") {
        return format!("NumPad {}", rest.trim().to_uppercase());
    }
    if lower.len() > 1 && lower.starts_with('f') && lower[1..].chars().all(|c| c.is_ascii_digit()) {
        return lower.to_uppercase();
    }
    if trimmed.chars().count() == 1 {
        return trimmed.to_uppercase();
    }
    trimmed.into()
}

fn modifier_prefix(part: &str) -> Option<Modifier> {
    match part.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Some(Modifier::Ctrl),
        "alt" | "option" | "altgraph" => Some(Modifier::Alt),
        "shift" => Some(Modifier::Shift),
        "cmd" | "meta" | "command" => Some(Modifier::Meta),
        _ => None,
    }
}

fn set(modifiers: &mut Modifiers, modifier: Modifier) {
    match modifier {
        Modifier::Ctrl => modifiers.ctrl = true,
        Modifier::Alt => modifiers.alt = true,
        Modifier::Shift => modifiers.shift = true,
        Modifier::Meta => modifiers.meta = true,
    }
}

/// Splits `Ctrl+Shift+NumPad +` into modifiers and the remaining key text.
fn split_modifiers(stroke: &str) -> (Modifiers, &str) {
    let mut modifiers = Modifiers::NONE;
    let mut rest = stroke.trim();
    while let Some((head, tail)) = rest.split_once('+') {
        match modifier_prefix(head.trim()) {
            Some(modifier) if !tail.is_empty() => {
                set(&mut modifiers, modifier);
                rest = tail;
            }
            _ => break,
        }
    }
    (modifiers, rest)
}

fn parse_mouse(modifiers: Modifiers, key: &str) -> Option<Shortcut> {
    let (name, double) = match key.strip_suffix("(double)") {
        Some(name) => (name.trim(), true),
        None => (key.trim(), false),
    };
    let button = match name {
        "Click" => MouseButton::Left,
        "Middle Click" => MouseButton::Middle,
        "Right Click" => MouseButton::Right,
        "Mouse Back" => MouseButton::Back,
        "Mouse Forward" => MouseButton::Forward,
        other => MouseButton::Other(other.strip_prefix("button")?.parse().ok()?),
    };
    Some(Shortcut::Mouse {
        modifiers,
        button,
        double,
    })
}

fn parse_stroke(input: &str, stroke: &str) -> Result<KeyStroke, ParseError> {
    let (modifiers, key) = split_modifiers(stroke);
    if key.trim().is_empty() {
        return Err(parse_error(input, "missing key"));
    }
    let bare = key.trim().trim_end_matches('+');
    if modifier_prefix(bare).is_some() {
        return Err(parse_error(input, "a modifier alone is not a keystroke"));
    }
    Ok(KeyStroke {
        modifiers,
        key: canonical_key(key),
    })
}

impl FromStr for Shortcut {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<Self, ParseError> {
        let text = input.trim();
        if text.is_empty() {
            return Err(parse_error(input, "empty shortcut"));
        }
        if let Some(rest) = text.strip_prefix("double ") {
            return Ok(match modifier_prefix(rest.trim()) {
                Some(modifier) => Shortcut::DoubleModifier(modifier),
                None => Shortcut::Gesture(text.into()),
            });
        }
        if text.eq_ignore_ascii_case("force touch") {
            return Ok(Shortcut::Gesture(text.into()));
        }
        let (modifiers, key) = split_modifiers(text);
        if let Some(mouse) = parse_mouse(modifiers, key) {
            return Ok(mouse);
        }
        // A chord separator is ", " (the key itself may be ",").
        let strokes: Vec<&str> = match text.split_once(", ") {
            Some((first, second)) if !second.trim().is_empty() => vec![first, second],
            _ => vec![text],
        };
        let strokes = strokes
            .into_iter()
            .map(|stroke| parse_stroke(input, stroke))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Shortcut::Keys(strokes))
    }
}

/// Platform conventions for showing shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notation {
    /// `Ctrl+Shift+A`
    Text,
    /// `⇧⌘A`
    Mac,
}

impl Notation {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Notation::Mac
        } else {
            Notation::Text
        }
    }
}

fn mac_key(key: &str) -> &str {
    match key {
        "Enter" => "↩",
        "Backspace" => "⌫",
        "Delete" => "⌦",
        "Esc" => "⎋",
        "Tab" => "⇥",
        "Up" => "↑",
        "Down" => "↓",
        "Left" => "←",
        "Right" => "→",
        "Page Up" => "⇞",
        "Page Down" => "⇟",
        "Home" => "↖",
        "End" => "↘",
        other => other,
    }
}

fn modifiers_label(modifiers: Modifiers, notation: Notation) -> String {
    match notation {
        Notation::Mac => {
            let mut out = String::new();
            for (on, symbol) in [
                (modifiers.ctrl, "⌃"),
                (modifiers.alt, "⌥"),
                (modifiers.shift, "⇧"),
                (modifiers.meta, "⌘"),
            ] {
                if on {
                    out.push_str(symbol);
                }
            }
            out
        }
        Notation::Text => {
            let mut out = String::new();
            for (on, name) in [
                (modifiers.ctrl, "Ctrl+"),
                (modifiers.alt, "Alt+"),
                (modifiers.shift, "Shift+"),
                (modifiers.meta, "Meta+"),
            ] {
                if on {
                    out.push_str(name);
                }
            }
            out
        }
    }
}

impl KeyStroke {
    pub fn label(&self, notation: Notation) -> String {
        let key = match notation {
            Notation::Mac => mac_key(&self.key),
            Notation::Text => &self.key,
        };
        format!("{}{key}", modifiers_label(self.modifiers, notation))
    }
}

fn modifier_name(modifier: Modifier, notation: Notation) -> &'static str {
    match (modifier, notation) {
        (Modifier::Ctrl, Notation::Mac) => "⌃",
        (Modifier::Alt, Notation::Mac) => "⌥",
        (Modifier::Shift, Notation::Mac) => "⇧",
        (Modifier::Meta, Notation::Mac) => "⌘",
        (Modifier::Ctrl, Notation::Text) => "Ctrl",
        (Modifier::Alt, Notation::Text) => "Alt",
        (Modifier::Shift, Notation::Text) => "Shift",
        (Modifier::Meta, Notation::Text) => "Meta",
    }
}

impl Shortcut {
    /// Human-readable label, e.g. `Ctrl+Shift+A`, `⇧⌘A`, `Double Shift`, `Ctrl+Click`.
    pub fn label(&self, notation: Notation) -> String {
        match self {
            Shortcut::Keys(strokes) => strokes
                .iter()
                .map(|stroke| stroke.label(notation))
                .collect::<Vec<_>>()
                .join(", "),
            Shortcut::Mouse {
                modifiers,
                button,
                double,
            } => {
                let name = match button {
                    MouseButton::Left => "Click".to_owned(),
                    MouseButton::Middle => "Middle Click".to_owned(),
                    MouseButton::Right => "Right Click".to_owned(),
                    MouseButton::Back => "Mouse Back".to_owned(),
                    MouseButton::Forward => "Mouse Forward".to_owned(),
                    MouseButton::Other(number) => format!("Button {number}"),
                };
                let mut label = modifiers_label(*modifiers, notation);
                if notation == Notation::Mac && !label.is_empty() {
                    label.push(' ');
                }
                label.push_str(&name);
                if *double {
                    label.push_str(" (double)");
                }
                label
            }
            Shortcut::DoubleModifier(modifier) => {
                format!("Double {}", modifier_name(*modifier, notation))
            }
            Shortcut::Gesture(name) => name.clone(),
        }
    }

    /// First keystroke of a keyboard shortcut.
    pub fn first_stroke(&self) -> Option<&KeyStroke> {
        match self {
            Shortcut::Keys(strokes) => strokes.first(),
            _ => None,
        }
    }
}

impl fmt::Display for Shortcut {
    /// Canonical text form; it parses back to the same shortcut.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Shortcut::Keys(strokes) => {
                let text: Vec<String> = strokes
                    .iter()
                    .map(|stroke| {
                        let mut out = String::new();
                        for (on, name) in [
                            (stroke.modifiers.ctrl, "Ctrl+"),
                            (stroke.modifiers.alt, "Alt+"),
                            (stroke.modifiers.shift, "Shift+"),
                            (stroke.modifiers.meta, "Cmd+"),
                        ] {
                            if on {
                                out.push_str(name);
                            }
                        }
                        out.push_str(&stroke.key);
                        out
                    })
                    .collect();
                write!(f, "{}", text.join(", "))
            }
            Shortcut::DoubleModifier(modifier) => {
                let name = match modifier {
                    Modifier::Ctrl => "control",
                    Modifier::Alt => "alt",
                    Modifier::Shift => "Shift",
                    Modifier::Meta => "meta",
                };
                write!(f, "double {name}")
            }
            Shortcut::Mouse { modifiers, .. } => {
                let label = self.label(Notation::Text);
                // Text notation spells ⌘ as Meta; the data uses Cmd.
                if modifiers.meta {
                    write!(f, "{}", label.replace("Meta+", "Cmd+"))
                } else {
                    write!(f, "{label}")
                }
            }
            Shortcut::Gesture(name) => write!(f, "{name}"),
        }
    }
}

//! Platform-appropriate shortcut labels.
//!
//! Command metadata stores macOS notation (see `docs/spec/keymaps.md`). On
//! Windows/Linux the default keymap differs, so labels are translated: modifier
//! glyphs become words and a few bindings use their documented non-macOS form.

use eframe::egui;

/// Translates a macOS-notation shortcut for display on the current platform.
pub fn display(mac: &str) -> String {
    if cfg!(target_os = "macos") {
        mac.to_owned()
    } else {
        for_windows_linux(mac)
    }
}

pub fn for_windows_linux(mac: &str) -> String {
    match mac {
        "" => return String::new(),
        "Double ⇧" => return "Double Shift".into(),
        // Default keymap differences between macOS and Windows/Linux.
        "⌃R" => return "Shift+F10".into(),
        "⌃⇧R" => return "Ctrl+Shift+F10".into(),
        "⌃D" => return "Shift+F9".into(),
        "⌥⌘R" => return "F9".into(),
        "⌘W" => return "Ctrl+F4".into(),
        "⌥F12" => return "Alt+F12".into(),
        "⌘," => return "Ctrl+Alt+S".into(),
        "⇧⌘]" => return "Alt+Right".into(),
        "⇧⌘[" => return "Alt+Left".into(),
        "⌃G" => return "Alt+J".into(),
        "⌃⇧G" => return "Alt+Shift+J".into(),
        "⌃⌘G" => return "Ctrl+Alt+Shift+J".into(),
        "⇧⌘8" => return "Alt+Shift+Insert".into(),
        "⌘↩" => return "Ctrl+Enter".into(),
        "⌥⌘↩" => return "Ctrl+Alt+Enter".into(),
        "⌘[" => return "Ctrl+Alt+Left".into(),
        "⌘]" => return "Ctrl+Alt+Right".into(),
        // Quick Documentation.
        "F1" => return "Ctrl+Q".into(),
        "⌥↑" => return "Ctrl+W".into(),
        "⌥↓" => return "Ctrl+Shift+W".into(),
        _ => {}
    }
    let mut chars = mac.chars().peekable();
    let (mut ctrl, mut alt, mut shift, mut command) = (false, false, false, false);
    while let Some(&c) = chars.peek() {
        match c {
            '⌃' => ctrl = true,
            '⌥' => alt = true,
            '⇧' => shift = true,
            '⌘' => command = true,
            _ => break,
        }
        chars.next();
    }
    let key: String = chars.collect();
    // Tool window activation (⌘1 … ⌘9, ⌘0) is Alt+digit on Windows/Linux.
    if command
        && !ctrl
        && !alt
        && !shift
        && key.len() == 1
        && key.chars().all(|c| c.is_ascii_digit())
    {
        return format!("Alt+{key}");
    }
    let mut parts = Vec::new();
    if ctrl || command {
        parts.push("Ctrl");
    }
    if alt {
        parts.push("Alt");
    }
    if shift {
        parts.push("Shift");
    }
    parts.push(&key);
    parts.join("+")
}

/// Drops the text some platforms emit alongside an Alt chord (Alt+7 → "7"),
/// so an unbound chord does not type into the focused field. Characters
/// composed with AltGr or macOS Option differ from the key's own symbol
/// ("@" for AltGr+Q, "¶" for ⌥7) and are kept.
pub fn strip_alt_chord_text(events: &mut Vec<egui::Event>) {
    let mut echoed: Vec<String> = Vec::new();
    events.retain(|event| match event {
        egui::Event::Key {
            key,
            modifiers,
            pressed: true,
            ..
        } if modifiers.alt && !modifiers.ctrl && !modifiers.mac_cmd => {
            let symbol = key.symbol_or_name();
            if symbol.chars().count() == 1 {
                echoed.push(symbol.to_lowercase());
            }
            true
        }
        egui::Event::Text(text) => !echoed.iter().any(|symbol| text.to_lowercase() == *symbol),
        _ => true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_mac_notation() {
        assert_eq!(for_windows_linux("⇧⌘A"), "Ctrl+Shift+A");
        assert_eq!(for_windows_linux("⌘F9"), "Ctrl+F9");
        assert_eq!(for_windows_linux("⌘1"), "Alt+1");
        assert_eq!(for_windows_linux("⇧⌘F12"), "Ctrl+Shift+F12");
        assert_eq!(for_windows_linux("⌃R"), "Shift+F10");
        assert_eq!(for_windows_linux("⌃⇧R"), "Ctrl+Shift+F10");
        assert_eq!(for_windows_linux("Double ⇧"), "Double Shift");
        assert_eq!(for_windows_linux("⌥↑"), "Ctrl+W");
        assert_eq!(for_windows_linux("⌥↓"), "Ctrl+Shift+W");
        assert_eq!(for_windows_linux(""), "");
        assert_eq!(for_windows_linux("⌘S"), "Ctrl+S");
        assert_eq!(for_windows_linux("⌃G"), "Alt+J");
        assert_eq!(for_windows_linux("⇧⌘8"), "Alt+Shift+Insert");
        assert_eq!(for_windows_linux("⌘["), "Ctrl+Alt+Left");
        assert_eq!(for_windows_linux("⌥⌘B"), "Ctrl+Alt+B");
    }

    fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn unbound_alt_digit_does_not_type() {
        let mut events = vec![
            key(egui::Key::Num7, egui::Modifiers::ALT),
            egui::Event::Text("7".into()),
        ];
        strip_alt_chord_text(&mut events);
        assert_eq!(events.len(), 1);
        // AltGr composes another character, which must still be typed.
        let altgr = egui::Modifiers {
            alt: true,
            ..Default::default()
        };
        let mut events = vec![key(egui::Key::Q, altgr), egui::Event::Text("@".into())];
        strip_alt_chord_text(&mut events);
        assert_eq!(events.len(), 2);
        // Plain typing is untouched.
        let mut events = vec![
            key(egui::Key::Num7, egui::Modifiers::NONE),
            egui::Event::Text("7".into()),
        ];
        strip_alt_chord_text(&mut events);
        assert_eq!(events.len(), 2);
    }
}

//! Keys: the keymap the terminal runtime resolves (the kit's terminal keymap plus the IDE's
//! active keymap, so the shortcuts are the GUI's) and the bytes a terminal session gets.

use archipel_ide_keymap::{KeyStroke, Keymap as IdeKeymap, Shortcut as IdeShortcut};
use archipel_kit::action_system::{Keymap, Os, Shortcut};
use archipel_kit_tui::input::{KeyCode, KeyEvent, KeyModifiers};

fn key_name(key: &str) -> Option<String> {
    Some(match key {
        "Page Up" => "PgUp".to_owned(),
        "Page Down" => "PgDn".to_owned(),
        "Delete" => "Del".to_owned(),
        "Insert" => "Ins".to_owned(),
        other if other.starts_with("NumPad") => return None,
        other if other.chars().count() == 1 || other.starts_with('F') || other.len() > 1 => {
            other.to_owned()
        }
        _ => return None,
    })
}

/// A stroke of the IDE keymap in the kit's text form (`ctrl shift A`).
pub fn stroke_text(stroke: &KeyStroke) -> Option<String> {
    if stroke.modifiers.meta {
        return None;
    }
    let mut parts = Vec::new();
    if stroke.modifiers.ctrl {
        parts.push("ctrl".to_owned());
    }
    if stroke.modifiers.alt {
        parts.push("alt".to_owned());
    }
    if stroke.modifiers.shift {
        parts.push("shift".to_owned());
    }
    parts.push(key_name(&stroke.key)?);
    Some(parts.join(" "))
}

/// The kit keymap for the terminal: the kit's defaults, every keyboard shortcut of the IDE
/// keymap for the actions `knows` accepts, then the chords of the single leader
/// ([`archipel_ide_view::keys::CHORDS`]); a chord never replaces a GUI shortcut.
pub fn build_keymap(os: Os, ide: &IdeKeymap, knows: impl Fn(&str) -> bool) -> Keymap {
    let mut keymap = Keymap::default_for(os);
    for (action, shortcuts) in ide.bindings() {
        if action.starts_with('$') || !knows(action) {
            continue;
        }
        for shortcut in shortcuts {
            let IdeShortcut::Keys(strokes) = shortcut else {
                continue;
            };
            let Some(texts) = strokes
                .iter()
                .map(stroke_text)
                .collect::<Option<Vec<String>>>()
            else {
                continue;
            };
            if texts.is_empty() || texts.len() > 2 {
                continue;
            }
            if let Some(parsed) = Shortcut::parse(&texts.join(" > ")) {
                keymap.add_shortcut(action, parsed);
            }
        }
    }
    // F10 shows the menu bar and opens File (TUI-GUI-037).
    if let Some(parsed) = Shortcut::parse("F10") {
        keymap.add_shortcut("MainMenu", parsed);
    }
    let leader = archipel_ide_view::keys::leader_for(ide);
    // Single strokes a legacy terminal sends as other keys: Ctrl+/ arrives as Ctrl+7 and
    // Ctrl+Backspace as Ctrl+H.
    for (action, keys) in SINGLE_FALLBACKS {
        if let Some(parsed) = Shortcut::parse(keys) {
            keymap.add_shortcut(action, parsed);
        }
    }
    for (action, keys) in archipel_ide_view::keys::CHORDS {
        if !knows(action) && !action.starts_with('$') && *action != "MainMenu" {
            continue;
        }
        if let Some(parsed) = Shortcut::parse(&format!("{leader} > {keys}")) {
            keymap.add_shortcut(action, parsed);
        }
    }
    keymap
}

/// Single strokes for editor actions whose GUI shortcut a legacy terminal cannot send.
pub const SINGLE_FALLBACKS: [(&str, &str); 2] = [
    ("CommentByLineComment", "ctrl 7"),
    ("EditorDeleteToWordStart", "ctrl H"),
];

/// The bytes a terminal session receives for `key`, `None` for keys it has no code for.
pub fn key_bytes(key: &KeyEvent) -> Option<Vec<u8>> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let mut out: Vec<u8> = match key.code {
        KeyCode::Char(c) if ctrl && c.is_ascii_alphabetic() => {
            vec![c.to_ascii_lowercase() as u8 - b'a' + 1]
        }
        KeyCode::Char('[') if ctrl => vec![0x1b],
        KeyCode::Char('\\') if ctrl => vec![0x1c],
        KeyCode::Char(']') if ctrl => vec![0x1d],
        KeyCode::Char(' ') if ctrl => vec![0],
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => csi(b'A', ctrl, alt, shift),
        KeyCode::Down => csi(b'B', ctrl, alt, shift),
        KeyCode::Right => csi(b'C', ctrl, alt, shift),
        KeyCode::Left => csi(b'D', ctrl, alt, shift),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::F(n) => match n {
            1 => b"\x1bOP".to_vec(),
            2 => b"\x1bOQ".to_vec(),
            3 => b"\x1bOR".to_vec(),
            4 => b"\x1bOS".to_vec(),
            5 => b"\x1b[15~".to_vec(),
            6 => b"\x1b[17~".to_vec(),
            7 => b"\x1b[18~".to_vec(),
            8 => b"\x1b[19~".to_vec(),
            9 => b"\x1b[20~".to_vec(),
            10 => b"\x1b[21~".to_vec(),
            11 => b"\x1b[23~".to_vec(),
            12 => b"\x1b[24~".to_vec(),
            _ => return None,
        },
        _ => return None,
    };
    if alt
        && matches!(
            key.code,
            KeyCode::Char(_) | KeyCode::Enter | KeyCode::Backspace
        )
    {
        out.insert(0, 0x1b);
    }
    Some(out)
}

fn csi(final_byte: u8, ctrl: bool, alt: bool, shift: bool) -> Vec<u8> {
    let modifier = 1 + u8::from(shift) + 2 * u8::from(alt) + 4 * u8::from(ctrl);
    if modifier == 1 {
        vec![0x1b, b'[', final_byte]
    } else {
        format!("\x1b[1;{modifier}{}", final_byte as char).into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_keymap::{Modifiers, Notation};

    #[test]
    fn strokes_become_kit_text() {
        let stroke = KeyStroke::new(
            Modifiers {
                ctrl: true,
                shift: true,
                ..Modifiers::NONE
            },
            "A",
        );
        assert_eq!(stroke_text(&stroke).as_deref(), Some("ctrl shift A"));
        let page = KeyStroke::new(Modifiers::NONE, "Page Down");
        assert_eq!(stroke_text(&page).as_deref(), Some("PgDn"));
        let _ = Notation::Text;
    }

    #[test]
    fn the_ide_keymap_reaches_the_runtime() {
        let set = archipel_ide_keymap::KeymapSet::shared().unwrap();
        let ide = set
            .keymap("$default", &archipel_ide_keymap::Overrides::default())
            .unwrap();
        let keymap = build_keymap(Os::Linux, &ide, |_| true);
        let text = |action: &str| {
            keymap
                .shortcuts(action)
                .iter()
                .map(|s| archipel_kit::action_system::KeymapUtil::text(s, false))
                .collect::<Vec<_>>()
        };
        assert!(
            text("GotoFile").iter().any(|t| t == "Ctrl+Shift+N"),
            "{:?}",
            text("GotoFile")
        );
        assert!(
            text("Run").iter().any(|t| t == "Shift+F10"),
            "{:?}",
            text("Run")
        );
        assert!(
            text("SearchEverywhere")
                .iter()
                .any(|t| t.contains("Ctrl+\\")),
            "{:?}",
            text("SearchEverywhere")
        );
    }

    #[test]
    fn keys_become_the_bytes_a_shell_expects() {
        use archipel_kit_tui::input::{key, key_with};
        let bytes = |event: archipel_kit_tui::Event| match event {
            archipel_kit_tui::Event::Key(k) => key_bytes(&k),
            _ => None,
        };
        assert_eq!(bytes(key(KeyCode::Char('a'))), Some(b"a".to_vec()));
        assert_eq!(bytes(key(KeyCode::Enter)), Some(b"\r".to_vec()));
        assert_eq!(bytes(key(KeyCode::Up)), Some(b"\x1b[A".to_vec()));
        assert_eq!(
            bytes(key_with(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(vec![3])
        );
        assert_eq!(
            bytes(key_with(KeyCode::Char('f'), KeyModifiers::ALT)),
            Some(b"\x1bf".to_vec())
        );
        assert_eq!(
            bytes(key_with(KeyCode::Right, KeyModifiers::CONTROL)),
            Some(b"\x1b[1;5C".to_vec())
        );
        assert_eq!(bytes(key(KeyCode::F(5))), Some(b"\x1b[15~".to_vec()));
    }

    /// The shortcuts `docs/tui.md` lists, from the bundled default keymap.
    #[test]
    fn the_documented_shortcuts_are_the_default_keymaps() {
        let set = archipel_ide_keymap::KeymapSet::shared().unwrap();
        let ide = set
            .keymap("$default", &archipel_ide_keymap::Overrides::default())
            .unwrap();
        let keymap = build_keymap(Os::Linux, &ide, |_| true);
        let first = |action: &str| {
            keymap
                .shortcuts(action)
                .iter()
                .map(|s| archipel_kit::action_system::KeymapUtil::text(s, false))
                .collect::<Vec<_>>()
        };
        let documented = [
            ("GotoAction", "Ctrl+Shift+A"),
            ("GotoFile", "Ctrl+Shift+N"),
            ("GotoLine", "Ctrl+G"),
            ("RecentFiles", "Ctrl+E"),
            ("FindInPath", "Ctrl+Shift+F"),
            ("SaveAll", "Ctrl+S"),
            ("GotoDeclaration", "Ctrl+B"),
            ("Back", "Ctrl+Alt+Left"),
            ("Forward", "Ctrl+Alt+Right"),
            ("CodeCompletion", "Ctrl+Space"),
            ("ShowIntentionActions", "Alt+Enter"),
            ("GotoNextError", "F2"),
            ("GotoPreviousError", "Shift+F2"),
            ("Run", "Shift+F10"),
            ("Stop", "Ctrl+F2"),
            ("Rerun", "Ctrl+F5"),
            ("ChooseRunConfiguration", "Alt+Shift+F10"),
            ("ActivateProjectToolWindow", "Alt+1"),
            ("ActivateFindToolWindow", "Alt+3"),
            ("ActivateRunToolWindow", "Alt+4"),
            ("ActivateProblemsViewToolWindow", "Alt+6"),
            ("ActivateCommitToolWindow", "Alt+0"),
            ("ActivateTerminalToolWindow", "Alt+F12"),
            ("HideActiveWindow", "Shift+Esc"),
            ("HideAllWindows", "Ctrl+Shift+F12"),
            ("JumpToLastWindow", "F12"),
            ("PreviousTab", "Alt+Left"),
            ("NextTab", "Alt+Right"),
            ("CloseContent", "Ctrl+F4"),
        ];
        for (action, text) in documented {
            assert!(
                first(action).iter().any(|t| t == text),
                "{action}: expected {text}, got {:?}",
                first(action)
            );
        }
    }
}

//! Shortcuts for display, what a terminal can send, and the chord alternatives.
//!
//! The shortcuts are the GUI keymap's (`$default` or the chosen keymap with the user's
//! overrides). A legacy terminal cannot send everything (spec appendix B): there an action
//! whose first shortcut cannot arrive shows the first one that can, else the GUI's, and is
//! reachable by `LEADER` followed by its key from [`CHORDS`].
//! Spec: TUI-GUI-156, TUI-GUI-157, TUI-GUI-165, TUI-GUI-170 (chords of the single leader).

use crate::cx::Caps;
use archipel_ide_keymap::{KeyStroke, Keymap, Shortcut};
use archipel_kit_model::{Chord, Shortcut as ModelShortcut};

/// The single leader of the terminal's chord alternatives (kit text form). `Ctrl+\` is
/// `0x1C`: unbound in the default keymaps, never swallowed by a terminal or tmux.
pub const LEADER: &str = "ctrl \\";

/// Leaders to fall back to when the active keymap binds [`LEADER`] to an action of this IDE
/// (the NetBeans keymap binds `Ctrl+\` to Code Completion): `Ctrl+]` (`0x1D`) and `Ctrl+^`
/// (`0x1E`) are control codes too.
pub const FALLBACK_LEADERS: [&str; 2] = ["ctrl ]", "ctrl ^"];

/// Whether `keymap` binds the single stroke `leader` (kit text form, `ctrl X`) to an action
/// this IDE implements.
pub fn leader_is_bound(keymap: &Keymap, leader: &str) -> bool {
    let key = leader.trim_start_matches("ctrl ").to_owned();
    keymap.bindings().iter().any(|(action, shortcuts)| {
        crate::actions::is_implemented(action)
            && shortcuts.iter().any(|s| match s {
                Shortcut::Keys(strokes) => strokes.first().is_some_and(|k| {
                    k.modifiers.ctrl
                        && !k.modifiers.alt
                        && !k.modifiers.shift
                        && !k.modifiers.meta
                        && k.key == key
                }),
                _ => false,
            })
    })
}

/// The leader to use with `keymap`: [`LEADER`] unless the keymap binds it.
pub fn leader_for(keymap: &Keymap) -> &'static str {
    std::iter::once(LEADER)
        .chain(FALLBACK_LEADERS)
        .find(|leader| !leader_is_bound(keymap, leader))
        .unwrap_or(LEADER)
}

/// Chord alternatives for actions a legacy terminal cannot send, as (action id, key after
/// the leader in the kit's text form). They also work with the kitty protocol, and a chord
/// never replaces a GUI shortcut. `tests/keys_collisions.rs` checks that no chord, and not
/// the leader, is bound in any shipped keymap and that no two chords collide.
pub const CHORDS: &[(&str, &str)] = &[
    // Double Shift and Ctrl+Shift+letter.
    ("SearchEverywhere", "E"),
    ("GotoAction", "A"),
    ("GotoFile", "N"),
    ("FindInPath", "F"),
    ("RecentLocations", "L"),
    ("$Redo", "Z"),
    ("CloseContent", "W"),
    ("EditorUnSelectWord", "shift W"),
    ("EditorToggleCase", "U"),
    ("EditorJoinLines", "shift J"),
    ("EditorMatchBrace", "shift M"),
    // The main menu (spec L: `Ctrl+\\ M`).
    ("MainMenu", "M"),
    ("GotoTest", "shift T"),
    ("GotoTypeDeclaration", "shift B"),
    ("FindPrevious", "shift F"),
    ("RunClass", "X"),
    ("EditBreakpoint", "shift X"),
    ("HideAllWindows", "H"),
    // Ctrl+Alt+Shift.
    ("GotoSymbol", "S"),
    ("SelectAllOccurrences", "G"),
    ("EditorPasteSimple", "shift V"),
    ("VcsShowNextChangeMarker", "."),
    ("VcsShowPrevChangeMarker", ","),
    // Ctrl+Alt+arrows, Alt+digit, Alt+letter, Alt+Enter.
    ("Back", "["),
    ("Forward", "]"),
    ("ActivateProjectToolWindow", "1"),
    ("ActivateFindToolWindow", "3"),
    ("ActivateRunToolWindow", "4"),
    ("ActivateProblemsViewToolWindow", "6"),
    ("ActivateStructureToolWindow", "7"),
    ("ActivateVersionControlToolWindow", "9"),
    ("ActivateCommitToolWindow", "0"),
    ("ActivateTerminalToolWindow", "F12"),
    ("SelectNextOccurrence", "D"),
    ("UnselectPreviousOccurrence", "shift D"),
    ("Generate", "Insert"),
    ("ShowIntentionActions", "Enter"),
    ("MoveLineUp", "Up"),
    ("MoveLineDown", "Down"),
    ("MoveStatementUp", "shift Up"),
    ("MoveStatementDown", "shift Down"),
    ("ChooseRunConfiguration", "shift R"),
    ("FindUsages", "shift U"),
    ("EvaluateExpression", "shift E"),
    // Ctrl+digit and punctuation, control aliases, Ctrl+Space and friends.
    ("CommentByLineComment", "/"),
    ("ExpandRegion", "="),
    ("CollapseRegion", "-"),
    ("ExpandRegionRecursively", "alt ="),
    ("CollapseRegionRecursively", "alt -"),
    ("ExpandAllRegions", "shift ="),
    ("CollapseAllRegions", "shift -"),
    ("InsertLiveTemplate", "J"),
    ("TypeHierarchy", "Y"),
    ("Switcher", "T"),
    ("CodeCompletion", "Space"),
    ("MaximizeToolWindow", "'"),
    ("Exit", "Q"),
    // Ctrl+Enter family, Ctrl+. and Ctrl+Alt+digit.
    ("EditorCompleteStatement", ";"),
    ("EditorSplitLine", "Tab"),
    ("EditorStartNewLineBefore", "shift O"),
    ("CollapseSelection", "O"),
    ("ShowUsages", "B"),
    ("Vcs.Push", "K"),
];

/// Whether a terminal with `caps` can deliver `stroke` (spec appendix B, kit `sendable`).
pub fn sendable(stroke: &KeyStroke, caps: &Caps) -> bool {
    if caps.kitty {
        return true;
    }
    let m = &stroke.modifiers;
    if m.meta {
        return caps.super_reported;
    }
    let key = stroke.key.as_str();
    let letter = key.len() == 1 && key.chars().all(|c| c.is_ascii_alphabetic());
    let digit = key.len() == 1 && key.chars().all(|c| c.is_ascii_digit());
    if m.ctrl {
        if letter && m.shift {
            return false;
        }
        if digit || matches!(key, "'" | "." | "," | ";" | "/" | "=" | "-") {
            return false;
        }
        if matches!(key, "Tab" | "Enter") || (key == "Space" && m.shift) {
            return false;
        }
        // Control aliases: Ctrl+I is Tab, Ctrl+M and Ctrl+J are Enter, Ctrl+H is Backspace.
        if letter && !m.alt && matches!(key, "I" | "M" | "J" | "H") {
            return false;
        }
        if m.shift && m.alt {
            // Ctrl+Alt+Shift combinations.
            return false;
        }
    }
    if m.alt && (digit || letter) && caps.mac {
        return false;
    }
    if m.alt && m.shift && key.starts_with('F') {
        return false;
    }
    true
}

fn chord_of(stroke: &KeyStroke) -> Chord {
    Chord {
        ctrl: stroke.modifiers.ctrl,
        alt: stroke.modifiers.alt,
        shift: stroke.modifiers.shift,
        meta: stroke.modifiers.meta,
        key: stroke.key.clone(),
    }
}

/// A keymap shortcut as the model's data.
pub fn model_shortcut(shortcut: &Shortcut, mac: bool) -> ModelShortcut {
    match shortcut {
        Shortcut::Keys(strokes) => ModelShortcut::Keys(strokes.iter().map(chord_of).collect()),
        Shortcut::DoubleModifier(modifier) => ModelShortcut::DoubleModifier(
            match modifier {
                archipel_ide_keymap::Modifier::Ctrl => "Ctrl",
                archipel_ide_keymap::Modifier::Alt => "Alt",
                archipel_ide_keymap::Modifier::Shift => "Shift",
                archipel_ide_keymap::Modifier::Meta => "Meta",
            }
            .to_owned(),
        ),
        other => ModelShortcut::Other(other.label(if mac {
            archipel_ide_keymap::Notation::Mac
        } else {
            archipel_ide_keymap::Notation::Text
        })),
    }
}

/// The shortcut of `action` to show for a keyboard with `caps`: the first keyboard shortcut
/// it can send, else the first shortcut the GUI would show. With the kitty protocol (and in
/// the GUI) this is the GUI's text.
pub fn display_shortcut(keymap: &Keymap, action: &str, caps: &Caps) -> Option<ModelShortcut> {
    let shortcuts: Vec<&Shortcut> = keymap
        .shortcuts(action)
        .iter()
        .filter(|s| !matches!(s, Shortcut::Gesture(_)))
        .collect();
    let first = shortcuts.first().copied()?;
    if caps.kitty {
        return Some(model_shortcut(first, caps.mac));
    }
    let deliverable = |s: &Shortcut| match s {
        Shortcut::Keys(strokes) => strokes.iter().all(|k| sendable(k, caps)),
        // Mouse shortcuts arrive in every terminal; a double modifier needs the protocol.
        Shortcut::Mouse { .. } => true,
        _ => false,
    };
    Some(model_shortcut(
        shortcuts
            .iter()
            .copied()
            .find(|s| deliverable(s))
            .unwrap_or(first),
        caps.mac,
    ))
}

/// The chord alternative of `action`, as the text after the leader.
pub fn chord_for(action: &str) -> Option<&'static str> {
    CHORDS.iter().find(|(a, _)| *a == action).map(|(_, k)| *k)
}

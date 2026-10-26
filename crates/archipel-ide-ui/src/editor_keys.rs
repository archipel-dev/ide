//! Editor-local keyboard bindings for caret/selection and text actions, the
//! double-modifier "clone caret" gesture, and multi-caret movement.
//!
//! Bindings follow the default keymaps in `docs/spec/keymaps.md`: the macOS
//! keymap on macOS and the Windows/Linux default keymap elsewhere.

use archipel_ide_core::Document;
use archipel_ide_text::multicaret::{BlockPoint, BlockSelection};
use archipel_ide_text::{Selection, SelectionSet, TextDocument};
use eframe::egui::{self, Key, Modifiers};

/// Exact modifier set, distinguishing Ctrl from ⌘ on macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Mods {
    ctrl: bool,
    alt: bool,
    shift: bool,
    cmd: bool,
}

impl Mods {
    fn from(modifiers: Modifiers) -> Self {
        Self {
            ctrl: modifiers.ctrl,
            alt: modifiers.alt,
            shift: modifiers.shift,
            cmd: modifiers.mac_cmd,
        }
    }

    const fn new(ctrl: bool, alt: bool, shift: bool, cmd: bool) -> Self {
        Self {
            ctrl,
            alt,
            shift,
            cmd,
        }
    }
}

/// Command bound to `key` in the editor, if any. The active keymap
/// (`crate::keybindings`) handles these actions first; this table is the fallback
/// for keystrokes the keymap leaves unbound.
pub fn action_for(key: Key, modifiers: Modifiers, mac: bool) -> Option<&'static str> {
    let m = Mods::from(modifiers);
    let (alt, alt_shift, ctrl_shift) = (
        Mods::new(false, true, false, false),
        Mods::new(false, true, true, false),
        Mods::new(true, false, true, false),
    );
    if mac {
        let ctrl = Mods::new(true, false, false, false);
        let cmd = Mods::new(false, false, false, true);
        return match (key, m) {
            (Key::G, m) if m == ctrl => Some("editor.selectNextOccurrence"),
            (Key::G, m) if m == ctrl_shift => Some("editor.unselectOccurrence"),
            (Key::G, m) if m == Mods::new(true, false, false, true) => {
                Some("editor.selectAllOccurrences")
            }
            (Key::G, m) if m == alt_shift => Some("editor.addCaretsToLineEnds"),
            (Key::Num8, m) if m == Mods::new(false, false, true, true) => {
                Some("editor.toggleColumnMode")
            }
            (Key::J, m) if m == ctrl_shift => Some("editor.joinLines"),
            (Key::U, m) if m == Mods::new(false, false, true, true) => Some("editor.toggleCase"),
            (Key::Enter, m) if m == cmd => Some("editor.splitLine"),
            (Key::Enter, m) if m == Mods::new(false, true, false, true) => {
                Some("editor.startNewLineBefore")
            }
            _ => None,
        };
    }
    match (key, m) {
        (Key::J, m) if m == alt => Some("editor.selectNextOccurrence"),
        (Key::J, m) if m == alt_shift => Some("editor.unselectOccurrence"),
        (Key::J, m) if m == Mods::new(true, true, true, false) => {
            Some("editor.selectAllOccurrences")
        }
        (Key::J, m) if m == ctrl_shift => Some("editor.joinLines"),
        (Key::G, m) if m == alt_shift => Some("editor.addCaretsToLineEnds"),
        (Key::Insert, m) if m == alt_shift => Some("editor.toggleColumnMode"),
        (Key::U, m) if m == ctrl_shift => Some("editor.toggleCase"),
        (Key::Enter, m) if m == Mods::new(true, false, false, false) => Some("editor.splitLine"),
        (Key::Enter, m) if m == Mods::new(true, true, false, false) => {
            Some("editor.startNewLineBefore")
        }
        _ => None,
    }
}

/// Double-tap-and-hold modifier gesture for Clone Caret Above/Below: Ctrl on
/// Windows/Linux, ⌥ on macOS. Lives in egui temp memory per editor.
#[derive(Debug, Clone, Copy, Default)]
struct Gesture {
    held: bool,
    last_press: Option<f64>,
    armed: bool,
}

const DOUBLE_TAP_SECONDS: f64 = 0.4;

/// Updates the gesture from the current modifier state; returns whether the
/// second press is still held so arrows clone carets.
pub fn clone_gesture_armed(ctx: &egui::Context, id: egui::Id, mac: bool) -> bool {
    let (modifiers, now) = ctx.input(|input| (input.modifiers, input.time));
    let down = if mac {
        modifiers.alt && !modifiers.ctrl && !modifiers.shift && !modifiers.mac_cmd
    } else {
        modifiers.ctrl && !modifiers.alt && !modifiers.shift
    };
    let any_other_key = ctx.input(|input| {
        input.events.iter().any(|event| {
            matches!(event, egui::Event::Key { key, pressed: true, .. }
                if !matches!(key, Key::ArrowUp | Key::ArrowDown))
        })
    });
    let id = id.with("clone_caret_gesture");
    let mut gesture: Gesture = ctx.data(|data| data.get_temp(id)).unwrap_or_default();
    if down && !gesture.held {
        gesture.armed = gesture
            .last_press
            .is_some_and(|last| now - last <= DOUBLE_TAP_SECONDS);
        gesture.last_press = Some(now);
    } else if !down {
        gesture.armed = false;
    }
    if any_other_key {
        // A chord such as Ctrl+C is not a modifier tap.
        gesture.last_press = None;
        gesture.armed = false;
    }
    gesture.held = down;
    ctx.data_mut(|data| data.insert_temp(id, gesture));
    gesture.armed
}

/// Shift+arrow deltas (lines, columns) for column selection mode.
pub fn column_delta(key: Key, modifiers: Modifiers) -> Option<(isize, isize)> {
    if !modifiers.shift || modifiers.alt || modifiers.command {
        return None;
    }
    match key {
        Key::ArrowUp => Some((-1, 0)),
        Key::ArrowDown => Some((1, 0)),
        Key::ArrowLeft => Some((0, -1)),
        Key::ArrowRight => Some((0, 1)),
        _ => None,
    }
}

/// Extends the remembered rectangular block in column mode.
pub fn extend_block(document: &mut Document, lines: isize, columns: isize) {
    let block =
        document
            .selections
            .current_block(&document.text)
            .moved(&document.text, lines, columns);
    block.apply(&document.text, &mut document.selections);
}

/// Rectangular selection from a mouse drag between two points.
pub fn drag_block(
    text: &TextDocument,
    selections: &mut SelectionSet,
    anchor: BlockPoint,
    head: BlockPoint,
) {
    BlockSelection { anchor, head }.apply(text, selections);
}

/// Moves every caret with `target`, extending selections when `extend`.
/// Carets that land on the same offset merge.
pub fn move_carets(
    document: &mut Document,
    extend: bool,
    target: impl Fn(&Document, Selection) -> usize,
) {
    let moved: Vec<Selection> = document
        .selections
        .selections()
        .iter()
        .map(|selection| {
            let head = target(document, *selection);
            Selection {
                anchor: if extend { selection.anchor } else { head },
                head,
            }
        })
        .collect();
    if moved.len() == 1 {
        document.selections.set_single(moved[0]);
    } else {
        document.selections.set_all(moved);
    }
}

/// Alt+Shift+click (⌥⇧click): adds a caret, or removes the caret already there.
pub fn toggle_caret(selections: &mut SelectionSet, offset: usize) {
    let current = selections.selections();
    if current.len() > 1 && current.iter().any(|s| s.is_caret() && s.head == offset) {
        let rest: Vec<Selection> = current
            .iter()
            .copied()
            .filter(|s| !(s.is_caret() && s.head == offset))
            .collect();
        selections.set_all(rest);
    } else {
        selections.add(Selection::caret(offset));
    }
}

/// Caret position widget text: `line:column`, the selected character and line
/// break counts for a selection, or the caret count with several carets.
pub fn caret_position_label(
    text: &TextDocument,
    selections: &SelectionSet,
    (line, column): (usize, usize),
) -> String {
    let count = selections.selections().len();
    if count > 1 {
        return format!("{count} carets");
    }
    let primary = selections.primary();
    let position = format!("{}:{}", line + 1, column + 1);
    if primary.is_caret() {
        return position;
    }
    let chars = primary.end() - primary.start();
    let breaks = text.char_to_line(primary.end()) - text.char_to_line(primary.start());
    let chars_label = if chars == 1 { "char" } else { "chars" };
    match breaks {
        0 => format!("{position} ({chars} {chars_label})"),
        1 => format!("{position} ({chars} {chars_label}, 1 line break)"),
        n => format!("{position} ({chars} {chars_label}, {n} line breaks)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mods(ctrl: bool, alt: bool, shift: bool) -> Modifiers {
        Modifiers {
            alt,
            ctrl,
            shift,
            mac_cmd: false,
            command: ctrl,
        }
    }

    #[test]
    fn windows_linux_bindings_match_the_default_keymap() {
        let f = |key, m| action_for(key, m, false);
        assert_eq!(
            f(Key::J, mods(false, true, false)),
            Some("editor.selectNextOccurrence")
        );
        assert_eq!(
            f(Key::J, mods(false, true, true)),
            Some("editor.unselectOccurrence")
        );
        assert_eq!(
            f(Key::J, mods(true, true, true)),
            Some("editor.selectAllOccurrences")
        );
        assert_eq!(f(Key::J, mods(true, false, true)), Some("editor.joinLines"));
        assert_eq!(
            f(Key::U, mods(true, false, true)),
            Some("editor.toggleCase")
        );
        assert_eq!(
            f(Key::Insert, mods(false, true, true)),
            Some("editor.toggleColumnMode")
        );
        assert_eq!(
            f(Key::Enter, mods(true, false, false)),
            Some("editor.splitLine")
        );
        assert_eq!(f(Key::J, mods(false, false, false)), None);
    }

    #[test]
    fn mac_bindings_match_the_macos_keymap() {
        let ctrl = Modifiers {
            ctrl: true,
            ..Default::default()
        };
        let cmd_shift = Modifiers {
            mac_cmd: true,
            command: true,
            shift: true,
            ..Default::default()
        };
        assert_eq!(
            action_for(Key::G, ctrl, true),
            Some("editor.selectNextOccurrence")
        );
        assert_eq!(
            action_for(Key::Num8, cmd_shift, true),
            Some("editor.toggleColumnMode")
        );
        assert_eq!(
            action_for(Key::U, cmd_shift, true),
            Some("editor.toggleCase")
        );
    }

    #[test]
    fn caret_widget_describes_selections_and_carets() {
        let doc = TextDocument::new("ab\ncd\nef");
        let mut selections = SelectionSet::single(1);
        assert_eq!(caret_position_label(&doc, &selections, (0, 1)), "1:2");
        selections.set_single(Selection { anchor: 1, head: 7 });
        assert_eq!(
            caret_position_label(&doc, &selections, (2, 1)),
            "3:2 (6 chars, 2 line breaks)"
        );
        selections.add(Selection::caret(8));
        assert_eq!(caret_position_label(&doc, &selections, (0, 0)), "2 carets");
    }

    #[test]
    fn column_mode_uses_plain_shift_arrows() {
        assert_eq!(
            column_delta(Key::ArrowUp, mods(false, false, true)),
            Some((-1, 0))
        );
        assert_eq!(column_delta(Key::ArrowUp, mods(false, false, false)), None);
        assert_eq!(column_delta(Key::ArrowLeft, mods(true, false, true)), None);
    }
}

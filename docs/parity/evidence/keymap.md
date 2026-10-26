# Keymap

Captures in `archipel-ide/keymap/` (Linux, Xvfb 1600×1000, no desktop environment, so the
XWin keymap is the default as in the reference IDE).

## Steps

1. Ctrl+Alt+S › Keymap → `keymap-page.png`: the keymap selector (all 26 bundled keymaps;
   XWin selected) and the action list. The 803 actions bound in XWin are listed, with
   Archipel IDE's implemented actions first and the rest marked "not available in Archipel IDE".
2. Type `settings` into the search → the list filters by action name. The "Find by
   shortcut" checkbox filters by shortcut text instead (`ctrl+shift+a`).
3. Double-click Settings… and press Ctrl+D → `keymap-capture-conflicts.png`: the recorder
   shows the stroke and warns that it is already assigned (12 actions, Duplicate Line or
   Selection among them).
4. OK → `keymap-conflict-dialog.png`: "The shortcut is already assigned to other actions.
   Do you want to remove other assignments?" with Remove / Leave / Cancel.
5. Leave → `keymap-selected-action.png`: Settings… now has Ctrl+Alt+S and Ctrl+D, each
   with a remove button, and Reset is enabled. `settings.toml` stores
   `"keymap.overrides" = ["Default for XWin\tShowSettings\tCtrl+Alt+S\tCtrl+D"]`.
6. In the editor, Ctrl+D still duplicates the line. The dispatcher runs the first
   applicable action in keymap order, and Duplicate applies while the editor has focus.

## Dispatch

Key presses become keystrokes (`archipel-ide-ui::keybindings::stroke`) and resolve against the
active keymap. Chords show "Ctrl+K was pressed. Waiting for the second stroke…" in the
status bar. While the terminal has focus, only tool-window, Find Action, Settings and
Search Everywhere shortcuts are intercepted, and every other key goes to the shell.
Editor-scoped actions (Undo/Redo, Duplicate, Delete Line, Comment, Move Line,
Completion) only run while the editor has focus, so text fields keep their own keys.
Because the keymap data is faithful, GNOME has no Move Line Up/Down shortcut and KDE
builds with Ctrl+9, exactly like the reference keymaps.

## Gaps

- The reference Keymap page shows an action tree (Main Menu › File › …), mouse-shortcut and
  abbreviation editors, and duplicates a bundled keymap before editing it. Archipel IDE shows
  a flat list and stores overrides per bundled keymap. Edits apply immediately, not on
  Apply.
- Mouse shortcuts and gestures are parsed and listed, but nothing triggers them yet.
- egui does not report numpad keys separately, so NumPad shortcuts never fire.
- Context-dependent resolution of keys bound to several actions is a fixed priority:
  workbench popups, then commands in keymap order.

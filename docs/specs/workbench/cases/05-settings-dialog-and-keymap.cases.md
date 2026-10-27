# Cases for 05-settings-dialog-and-keymap

Dialog notation: `dialog:` shows what is visible; `tree:` the page tree (`*` = selected, `(mod)` = drawn in the modified color, `(err)` = drawn in the error color);
`buttons:` marks disabled buttons as `[Apply:disabled]`. Keymap tables list `action | shortcuts`.

### WB-SET-002 case 1: Apply enablement
settings: default
language: plain
```before
Settings dialog open on page "Appearance"; no changes made
buttons: [OK] [Cancel] [Apply:disabled]
```
keys: toggle the checkbox "Show tool window numbers"
```after
tree: * Appearance (mod)
buttons: [OK] [Cancel] [Apply]
```

### WB-SET-002 case 2: Apply keeps the dialog open
settings: default
language: plain
```before
page "Appearance", "Show tool window numbers" toggled on, Apply enabled
```
keys: click Apply
```after
dialog open; tree: * Appearance (normal color); buttons: [Apply:disabled]
setting showToolWindowsNumbers = true is persisted
```

### WB-SET-002 case 3: Cancel discards silently
settings: default
language: plain
```before
page "Appearance", "Show tool window numbers" toggled on, not applied
```
keys: click Cancel
```after
dialog closed with no confirmation; showToolWindowsNumbers still false
reopening the dialog: "Show tool window numbers" unchecked
```

### WB-SET-002 case 4: OK applies and closes
settings: default
language: plain
```before
page "Appearance", "Show tool window numbers" toggled on
```
keys: click OK
```after
dialog closed; showToolWindowsNumbers = true
```

### WB-SET-003 case 1: apply error keeps the dialog open
settings: page "HTTP Proxy" with manual proxy, port field "99999" (invalid), page "Appearance" modified
language: plain
```before
two modified pages: Appearance (valid), HTTP Proxy (invalid port)
```
keys: click Apply
```after
dialog open; selection moves to HTTP Proxy
tree: Appearance (normal), * HTTP Proxy (err)
red error band below the page shows the error message of the proxy page
Appearance change is persisted; HTTP Proxy change is not
```

### WB-SET-004 case 1: modified marker
settings: default
language: plain
```before
tree: * Keymap
```
keys: change the active keymap in the selector to "Eclipse"
```after
tree: * Keymap (mod)
```

### WB-SET-005 case 1: pending changes survive page switches
settings: default
language: plain
```before
page "Appearance": "Show tool window names" toggled on (unapplied)
```
keys: select page "Keymap" in the tree, select page "Appearance" again
```after
"Show tool window names" still checked; tree: * Appearance (mod)
```

### WB-SET-006 case 1: revert changes
settings: default
language: plain
```before
page "Appearance" with "Show tool window names" toggled on; header shows the action "Revert changes" enabled
```
keys: click "Revert changes"
```after
checkbox unchecked; "Revert changes" disabled; tree: * Appearance (normal)
```

### WB-SET-007 case 1: Escape clears the search first
settings: default
language: plain
```before
dialog open; focus in the search field containing "font"
```
keys: Escape, Escape
```after
after first Escape: search field empty, dialog still open, full tree visible
after second Escape: dialog closed (as Cancel)
```

### WB-SET-007 case 2: Enter moves focus into the page
settings: default
language: plain
```before
dialog open; search field contains "keym", tree selection Keymap
```
keys: Enter
```after
focus is inside the Keymap page (the keymap selector), search text unchanged
```

### WB-SET-020 case 1: search by page name
settings: default
language: plain
```before
dialog open, empty search
```
keys: type "keym"
```after
tree (filtered):
  * Keymap
```

### WB-SET-020 case 2: search by option text
settings: default
language: plain
```before
dialog open, empty search
```
keys: type "tool window names"
```after
tree (filtered):
  Appearance & Behavior
    * Appearance
the option "Show tool window names" is highlighted on the page (spotlight)
```

### WB-SET-022 case 1: clear search
settings: default
language: plain
```before
dialog open, search "keym", Keymap selected
```
keys: select all in the search field, Delete
```after
full tree restored; * Keymap still selected
```

### WB-SET-030 case 1: project vs application scope
settings: two projects A and B open
language: plain
```before
in project A: Settings > Editor > File Encodings, project encoding set to ISO-8859-1, OK
```
keys: open Settings in project B, page File Encodings
```after
project encoding in B: unchanged (default); in A: ISO-8859-1
the Keymap change made in A (application scope) is visible in B
```

### WB-SET-043 case 1: Quick Switch Scheme
settings: default
language: plain
```before
editor focused
```
keys: Ctrl+BACK_QUOTE
```after
popup entries: Theme, Editor Color Scheme, Keymap, Code Style Scheme, Error Highlighting, View Mode, Layout, Zoom
```

### WB-KEY-002 case 1: editing a bundled keymap makes a copy
settings: active keymap "Windows" (Windows OS)
language: plain
```before
Keymap page; selector: Windows
action "Rename" | Shift+F6
```
keys: right click "Rename", Add Keyboard Shortcut, press F2, OK
```after
selector: "Windows copy"  (label: Based on Windows keymap)
action "Rename" | Shift+F6, F2   (in the copy)
the bundled "Windows" keymap still has Rename = Shift+F6 only
```

### WB-KEY-004 case 1: context menu entries
settings: default keymap
language: plain
```before
Keymap page; action "Rename" | Shift+F6
```
keys: right click the action row "Rename"
```after
popup "Edit Shortcuts" entries:
  Add Keyboard Shortcut
  Add Mouse Shortcut
  Add Abbreviation
  ---
  Remove Shift+F6
  Remove All Shortcuts
  Reset Shortcuts
```

### WB-KEY-005 case 1: conflict list
settings: default keymap (Windows)
language: plain
```before
Add Keyboard Shortcut dialog open for action "Find Usages"
```
keys: press Ctrl+Alt+L
```after
text under the capture field: Already assigned to:
list: Reformat Code
```

### WB-KEY-006 case 1: conflict, Remove
settings: default keymap (Windows)
language: plain
```before
Add Keyboard Shortcut dialog open for action "Find Usages", shortcut Ctrl+Alt+L captured
```
keys: OK, choose "Remove" in the warning
```after
warning title "Warning": This shortcut is already assigned to other actions. Do you want to remove the other assignments?   buttons: Remove, Keep, Cancel
result in the copy "Windows copy": "Find Usages" | Alt+F7, Ctrl+Alt+L ; "Reformat Code" | (no shortcut)
```

### WB-KEY-006 case 2: conflict, Keep
settings: same as the previous case
language: plain
```before
same as the previous case, warning shown
```
keys: choose "Keep"
```after
"Find Usages" | Alt+F7, Ctrl+Alt+L ; "Reformat Code" | Ctrl+Alt+L (both bound)
```

### WB-KEY-006 case 3: conflict, Cancel
settings: same
language: plain
```before
same as the previous case, warning shown
```
keys: choose "Cancel"
```after
no keymap is changed (no copy "Windows copy" is created, Find Usages and Reformat Code keep their bindings)
```

### WB-KEY-007 case 1: search by action name
settings: default
language: plain
```before
Keymap page, empty search
```
keys: type "reformat" in the page's search field
```after
tree shows matching actions only (groups expanded), including:
  Reformat Code | Ctrl+Alt+L
  Reformat File... | Ctrl+Alt+Shift+L
```

### WB-KEY-008 case 1: find by shortcut
settings: default keymap (Windows)
language: plain
```before
Keymap page; popup "Find Shortcut" open
```
keys: press Ctrl+S
```after
tree shows: Save All | Ctrl+S
"Clear Filtering" is enabled
```

### WB-KEY-009 case 1: mouse shortcut text
settings: default
language: plain
```before
dialog "Mouse Shortcut" open: pad text "Enter a shortcut here: single or double-click, scroll the wheel, modify with Ctrl, Alt and Shift"
```
keys: Ctrl+click on the pad
```after
pad shows: Ctrl+Click
```

### WB-KEY-011 case 1: unique names
settings: user keymaps "A" and "B"
language: plain
```before
selector shows keymap "B"; gear > Rename
```
keys: type "A", Enter
```after
error: All keymaps should have unique names; rename rejected
```

### WB-KEY-011 case 2: empty name
settings: user keymap "A"
language: plain
```before
gear > Rename on keymap "A"
```
keys: clear the text, Enter
```after
error: All keymaps should have non empty names; rename rejected
```

### WB-KEY-003 case 1: unbinding in a child keymap
settings: keymap "Windows copy" (parent Windows)
language: plain
```before
action "Copy" | Ctrl+C, Ctrl+Insert in the parent
```
keys: in the copy: right click "Copy", Remove Ctrl+C
```after
copy: Copy | Ctrl+Insert ; parent: Copy | Ctrl+C, Ctrl+Insert
```

### WB-KEY-003 case 2: reset shortcuts
settings: keymap "Windows copy" where Copy has only Ctrl+Insert
language: plain
```before
copy: Copy | Ctrl+Insert
```
keys: right click "Copy", Reset Shortcuts
```after
copy: Copy | Ctrl+C, Ctrl+Insert (the override entry is removed, the parent's bindings apply)
```

### WB-KEY-020 case 1: chord
settings: keymap "Visual Studio"
language: plain
```before
caret on a line in a file; action ToggleBookmark bound to Ctrl+K, K
```
keys: Ctrl+K
```after
status bar text: Prefix key pressed
```

### WB-KEY-020 case 2: chord completes
settings: keymap "Visual Studio"
language: plain
```before
caret on a line in a file; no bookmarks
```
keys: Ctrl+K, K
```after
bookmark on the caret line; status text cleared
```

### WB-KEY-020 case 3: chord cancelled by Escape
settings: keymap "Visual Studio"
language: plain
```before
caret on a line in a file; no bookmarks
```
keys: Ctrl+K, Escape
```after
no bookmark; status text cleared
```

### WB-KEY-030 case 1: default keymap per desktop
settings: Linux with KDE desktop, first start
language: plain
```before
no saved keymap
```
keys: open Settings > Keymap
```after
selector: KDE ; effective Close Tab = Ctrl+4 ; Activate Project = Alt+1
```

### WB-KEY-030 case 2: default on macOS
settings: macOS, first start
language: plain
```before
no saved keymap
```
keys: open Settings > Keymap
```after
selector: macOS ; Activate Project = Cmd+1 ; Settings = Cmd+, ; Toggle Bookmark = F3
```

### WB-KEY-033 case 1: hidden keymaps
settings: Windows
language: plain
```before
Keymap page
```
keys: open the selector
```after
list does not contain: GNOME, KDE, XWin, macOS, IntelliJ IDEA Classic
list contains: Windows, Eclipse, Emacs, NetBeans 6.5, QtCreator, ReSharper, Sublime Text, Visual Assist, Visual Studio, Visual Studio 2022, VSCode, Xcode(only if the keymap plugin is bundled)
```

### WB-KEY-021 case 1: inherited keymap lookup
settings: keymap "GNOME" (parent XWin, parent Windows)
language: plain
```before
active keymap GNOME
```
keys: query shortcuts of ActivateProjectToolWindow, SelectIn, Back
```after
ActivateProjectToolWindow: Alt+1 (from Windows)
SelectIn: Shift+Alt+1, Alt+F1 (GNOME override)
Back: Shift+Alt+Left (GNOME override)
```

### WB-SET-001 case 1: last page remembered
settings: default
language: plain
```before
Settings closed after selecting Keymap
```
keys: Ctrl+Alt+S
```after
dialog opens on Keymap
```

### WB-SET-008 case 1: Ctrl+F focuses search
settings: default
language: plain
```before
dialog open, focus in the page
```
keys: Ctrl+F
```after
caret in the search field
```

### WB-SET-021 case 1: first match selected
settings: default
language: plain
```before
dialog open
```
keys: type "keym"
```after
Keymap selected
```

### WB-KEY-001 case 1: switching keymap
settings: Windows keymap active
language: plain
```before
Keymap page
```
keys: select Eclipse, OK, then Ctrl+Shift+R
```after
Go to File opens (Eclipse binding)
```

### WB-KEY-013 case 1: remap applies immediately
settings: Windows keymap, copy active
language: plain
```before
Rename = Shift+F6
```
keys: bind F2 to Rename, OK, F2 in an editor on a symbol
```after
rename starts; Refactor menu shows F2
```

### WB-KEY-015 case 1: mnemonics disabled
settings: macOS keymap
language: plain
```before
editor focused
```
keys: Alt+F
```after
File menu does not open
```

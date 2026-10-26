# Interaction specification

The contract every dialog, popup, menu, list, tree, tab bar, tool window and chrome widget in
`archipel-ide-ui` must meet so that Archipel IDE feels like an IntelliJ-platform IDE. Workers
implement against this document; `docs/ux/audit-v0.0.8.md` records where v0.0.8 breaks it,
and `cargo xtask ux-smoke` checks the golden paths with real input.

Terminology follows the public IntelliJ Platform UI Guidelines
(<https://plugins.jetbrains.com/docs/intellij/ui-guidelines-welcome.html>: Dialog, Popup,
Tooltip, Button, Context menu, Tool window, Tabs, List, Tree, Notification, Status bar) and
the IntelliJ IDEA help. Behaviour is specified by observation of those products; nothing in
this document is copied from JetBrains code or artwork. Clean-room boundaries from
`AGENTS.md` apply.

Scope of a "layer": anything drawn above the workbench in an egui `Area` with
`Order::Middle` or `Order::Foreground`. Every layer is one of

| Kind | Examples | Blocks the workbench | Closes on click outside | Closes on Escape |
|---|---|---|---|---|
| **Modal dialog** | New Project, Open Project, Get from VCS, Settings, Rename, Delete, Run/Debug Configurations, Push, first-run | yes (dimmed backdrop swallows input) | no | yes (= Cancel) |
| **Popup** | Search Everywhere, Find in Files, Recent Files, File Structure, Switcher, context menus, main menu, combo lists, run/branch/project widgets, completion, intentions, gutter menu | no | yes | yes |
| **Hover** | tooltips, quick documentation on hover, diagnostic tooltips, parameter info while typing | no | yes | yes |
| **Balloon** | notifications | no | no | no (they time out or have a close button) |

## 1. Layering and keys that cross components

1.1 One **popup stack** owns all open layers in opening order. Escape closes the top-most
*visible* layer only; nothing lower may consume it. A hidden or stale layer (an invisible
completion session, a quick-doc popup whose anchor scrolled away) is not on the stack.

1.2 Opening a modal dialog closes every popup and hover. Opening a popup closes hovers.
Only one hover is visible at a time.

1.3 A pointer press outside a popup's rect (and outside its anchor control, for widgets that
toggle) closes the popup before the press is delivered to whatever is underneath, except that
the press on the anchor control of a *toggle* popup only closes it.

1.4 Keyboard focus leaving the editor closes editor-anchored popups (completion, parameter
info, intentions, quick doc).

1.5 While a modal dialog is open no workbench shortcut fires; while a popup is open only the
popup's own keys and Escape fire (menus additionally pass through nothing).

1.6 The read-only `state` snapshot (`popups.*`, `popups.settings`, `window`) reports the
stack so scenes and the smoke harness can assert it.

## 2. Dialogs

2.1 Built with one shared component (`archipel_ide_ui::dialog`) that provides: title bar with
the dialog title (left) and a close button (right), optional `?` help button, content area,
button row, backdrop, placement, sizing and key handling. No dialog assembles these itself.

2.2 **Placement**: centred on the content rect, clamped so the whole frame stays inside it.
**Size**: preferred size, never larger than window − 40 pt in either direction; when the
content does not fit, the content area scrolls and the button row stays visible. The
800×500 minimum window must show title, at least two rows of content and the button row of
every dialog.

2.3 **Button row**: right-aligned; on Linux/Windows `[secondary…] [Cancel] [Default]`, on
macOS the same order (Cancel left of the default). Exactly one **default** button, painted
in the accent colour; it is the only accent button. Destructive defaults (Delete) keep the
accent but say what they do. Disabled defaults show why in inline validation text (2.6).

2.4 **Keys**: Enter activates the default button unless focus is in a multi-line text field,
an open combo box or a list that consumes Enter; Escape activates Cancel (and closes an open
combo/menu first, without closing the dialog). Tab/Shift+Tab cycle through every control in
reading order including buttons; the focus does not leave the dialog. Space toggles
checkboxes and activates the focused button.

2.5 **Focus on open**: the first editable field; its text is selected when it holds a
default value (project name `untitled`, the file name without extension in Rename).

2.6 **Validation**: errors appear as red text directly under the field with the field
outlined in the error colour; the default button is disabled while the form is invalid.
Messages are complete sentences in the user's terms, naming the field and the value
("There is no directory at /x/y"), never the internal error chain, command ids or OS codes.
Recoverable conditions are done for the user where the reference does it (a missing
project location is created).

2.7 **Modality**: a translucent backdrop over the whole window swallows pointer and wheel
input; the workbench does not react to hover beneath it. Closing a dialog returns focus to
the control or editor that had it.

2.8 Long-running work started from a dialog (clone, JDK download, project creation) shows
progress inside the dialog with a Cancel that stops it; the dialog does not close before
success.

## 3. Popups

3.1 **Anchored popups** (combo lists, context menus, gutter menus, widget dropdowns,
completion, intentions) open at their anchor: below it, left-aligned; if that overflows the
window they flip above / align right; submenus open to the right and flip left. Never
clipped by the window edge; never covering their own anchor.

3.2 **Centred popups** (Search Everywhere, Find in Files, Recent Files, File Structure,
Switcher) sit at 12 % from the top, horizontally centred, width `min(preferred,
window − 40)`, list height `≤ 70 %` of the window.

3.3 Open with the first row selected; ↑↓ move the selection and keep it in view (no wrap in
menus, wrap allowed in search lists); PgUp/PgDn page; Home/End jump; Enter activates;
Escape closes; typing filters where the popup has a field, otherwise starts speed search.
Click activates a row; hovering highlights it without changing the keyboard selection.

3.4 Toggle widgets (run configuration, project, branch, device) show hover and pressed
states, an open state while the popup is visible, and a chevron.

3.5 Completion: closes on Escape, on focus loss, when the typed prefix no longer matches,
on a click outside; Enter inserts, Tab replaces; the popup never hides the line being typed.
Parameter info and quick documentation are hovers (1.1–1.4).

## 4. Menus

4.1 The ☰ button opens the main menu with one click (a vertical list of the top-level
menus on Linux/Windows), and a second click on a title opens that menu; hovering across titles
switches menus while one is open.

4.2 Every menu, submenu and context menu: ↓↑ move the highlight (skipping separators and
disabled items), → opens the highlighted submenu, ← closes a submenu, Enter and Space
activate, Escape closes one level, typing a mnemonic or the first letter jumps. Shortcuts
are right-aligned in secondary text. Disabled items stay visible and dimmed.

4.3 Items depend on context: no Java/Kotlin class templates in a project without a JVM
module; no per-tab actions on the empty tab-bar area.

4.4 Right-click surfaces: project tree rows and header, editor, tabs, status bar (widget
toggles), toolbar (customise), recent-project rows (Open, Remove), tool window headers
(move/resize/hide).

## 5. Lists and trees

5.1 One focusable `ListView`/`TreeView` primitive paints rows; nothing allocates rows ad hoc.
Row height 24 pt; states hover, pressed, selected, selected+focused (brighter), drop target.

5.2 Keys: ↑↓ move, Home/End, PgUp/PgDn, ← collapses or goes to the parent, → expands or goes
to the first child, Enter opens (files) or toggles (directories), Space selects, Shift+arrows
extend, Ctrl+click toggles, F2 or Shift+F6 renames, Delete asks to delete, Ctrl+C/X/V copy,
cut and paste nodes.

5.3 **Speed search**: typing while the component has focus shows the typed text in a small
field at the top-left of the component, selects the first match and highlights the matched
substring in every row; ↑↓ move between matches; Escape clears; Backspace edits. Non-matching
rows stay visible.

5.4 Mouse: single click selects (and previews a file where the setting is on); double click
opens a file or toggles a directory; click on the chevron toggles; right-click selects and
opens the context menu; drag moves nodes with a ghost and a drop-target highlight; wheel
scrolls 3 rows per notch; the selection is scrolled into view after keyboard moves and after
reveal-in-tree.

5.5 Long names are elided with "…" and get a tooltip with the full name; nothing wraps.

## 6. Editor tabs

6.1 Click activates; middle click and the × button close; drag reorders with the dragged
tab following the pointer and an insertion marker; dragging out of the bar is ignored (no
split by drag until splits exist). Double-click on a tab maximises the editor (hides tool
windows) and again restores.

6.2 Right-click opens the tab menu; the empty area has none. Modified tabs mark the name;
pinned tabs show the pin and do not close with "Close Others".

6.3 Overflow scrolls horizontally with the wheel; the active tab is scrolled into view.

## 7. Tool windows

7.1 Stripe buttons toggle; the header shows the title, actions, a hide button and reacts to
double-click (maximise) and right-click (view mode menu). The resize edge is 4 pt wide, shows
`ew-resize`/`ns-resize`, and the size persists in the workspace.

7.2 Showing a tool window moves focus into it (its primary control); hiding it returns focus
to the editor. Escape inside a tool window returns focus to the editor and keeps the window
open; Shift+Escape hides it.

7.3 Minimum sizes: 160 pt wide, 100 pt high; content never paints over the editor island.

## 8. Chrome widgets and status bar

8.1 Every status bar widget is a control: hover and pressed states, a tooltip with its
meaning, and a click that opens its popup (caret position → Go to Line; line separator;
encoding; indent; language server status; toolchain warning → Set up). Nothing in the status
bar is painted as a button without behaviour.

8.2 Status text is elided to the space left of the right-hand widgets and never overlaps
the breadcrumbs; the full text is in its tooltip. Transient messages disappear after 10 s.

8.3 Toolbar buttons show hover, pressed and disabled states and a tooltip "Action
(Shortcut)".

## 9. Hover, cursors, tooltips

9.1 Tooltip initial delay 1.0–1.2 s, dismissed when the pointer moves off the control, never
duplicating a visible label, never covering the hovered control. Shortcuts in tooltips use
the platform notation.

9.2 Cursors: text over editable text, pointer over links and link-styled actions, resize
arrows over panel and window edges, default elsewhere; a drag shows `grabbing`.

9.3 Hover states appear within one frame; no hover state lingers after the pointer left.

## 10. Focus and keyboard

10.1 Exactly one focused control at a time, visible as a 2 pt accent ring (text fields,
buttons, checkboxes, combos, lists, trees, tabs when focused by Tab).

10.2 Tab/Shift+Tab move in reading order inside the focused container (dialog, tool window,
editor header); Ctrl+Tab is the switcher; Escape returns to the editor from any tool
window; F12 goes back to the last tool window.

10.3 Opening a file focuses its editor; closing a popup or dialog restores the previous
focus; `Alt+1…9` focus tool windows.

## 11. Window sizes and placement

11.1 Every screen is usable at the 800×500 minimum: dialogs per 2.2, popups per 3.1/3.2, the
toolbar collapses optional widgets before it clips the project/run widgets, tool windows
clamp to 7.3.

11.2 Nothing positions itself with a fixed offset from the top that is not clamped to the
window; nothing assumes 1440×900.

## 12. Feedback, timing and performance

12.1 Any operation longer than 300 ms shows progress (status bar progress or balloon with a
spinner), longer than 2 s a cancellable one. Completed operations the user started (commit,
push, install, build, run) end with a balloon or a Run/Build tool window result; failures
name the failing step and offer the remedy.

12.2 Input-to-paint latency ≤ 1 frame for typing, selection and hover; popups open within
100 ms of the gesture (completion may fill asynchronously but the frame opens immediately).

12.3 An idle window (no pointer motion, no animation, no running task) repaints at 0 Hz
until an event or a service wakes it; background services use the waker, not timers.
Spinners animate only while visible.

## 13. Text and empty states

13.1 Labels in sentence case, buttons name the action ("Create", "Open", "Delete"); an
ellipsis marks actions that open a dialog. No internal identifiers, no OS error codes, no
"command failed:" prefixes in user-facing text.

13.2 Every page and list has an empty state that says what belongs there and offers the
next action (e.g. "No run configurations. Add one…"), instead of blank space.

13.3 Toolbar texts never wrap; long values elide with a tooltip.

## 14. Verification

Each rule maps to a check:

| Rules | Check |
|---|---|
| 1.x, 2.4, 2.7 | `ux-smoke` scene `first-run-dialog-is-modal`; unit tests on the popup stack |
| 2.2, 11.1 | `ux-smoke` scene `new-project-in-minimum-window` |
| 2.3–2.6 | `ux-smoke` scene `new-project-by-mouse`; dialog component unit tests |
| 3.x, 4.x, 5.x, 6.x, 7.x | `ux-smoke` scene `workbench-tree-tabs-popups-run` (extend per feature) |
| 8.1 | smoke step per widget once implemented |
| 12.3 | frame counter asserted idle in `ux-smoke` once C6 lands |

New interaction code lands with its scene step; a change to a layout promise updates
`xtask/src/ux_smoke.rs::layout` in the same change.

# Workbench spec 01: tool window framework

Status: specification (clean-room). ID prefix `WB-TW`. Priorities `MUST` / `SHOULD` / `LATER` as in `docs/specs/README.md`.
Executable cases: `cases/01-tool-window-framework.cases.md`.

Scope: the frame around every tool window: anchors and stripes, show / hide / activate, focus rules, view modes, split (side) tool
windows, maximize and resize, remembered sizes, layouts, default shortcuts, the list of tool windows the Community product registers
and their defaults. What a tool window *contains* is specified in the area files (`02`..`08`, other areas).

Sources studied: `ic: platform/platform-impl/src/com/intellij/openapi/wm/impl/ToolWindowManagerImpl.kt`,
`ic: platform/platform-impl/src/com/intellij/openapi/wm/impl/WindowInfoImpl.kt`, `ic: platform/ide-core/src/com/intellij/openapi/wm/ToolWindowAnchor.java`,
`ic: platform/ide-core/src/com/intellij/openapi/wm/ToolWindowType.java`, `ic: platform/ide-core/src/com/intellij/openapi/wm/ToolWindowContentUiType.java`,
`ic: platform/platform-impl/src/com/intellij/ide/actions/ActivateToolWindowAction.kt`, `HideToolWindowAction.kt`, `HideAllToolWindowsAction.kt`,
`JumpToLastWindowAction.java`, `ResizeToolWindowAction.java`, `ic: platform/platform-impl/src/com/intellij/openapi/wm/impl/MaximizeToolWindowAction.java`,
`ic: platform/editor-ui-api/src/com/intellij/ide/ui/UISettingsState.kt`, `ic: platform/platform-resources/src/keymaps/$default.xml`,
`ic: platform/platform-resources-en/src/messages/ActionsBundle.properties`, `ic: platform/platform-api/resources/messages/IdeBundle.properties`.

## 1. Model

A **tool window** is a named panel with content, registered by id. Its per-project, persisted **window info**:

| Field | Meaning | Default |
|---|---|---|
| id | unique string, for example `Project`, `Problems View` | - |
| anchor | side of the frame: `left`, `right`, `bottom`, `top` | `left` |
| type | `docked`, `sliding` (menu: "Undock"), `floating`, `windowed` | `docked` |
| auto-hide | docked but hides when focus leaves it (menu: "Dock Unpinned") | off |
| visible | currently shown (docked and sliding: in the frame; floating and windowed: in their own window) | off |
| active-on-start | had focus when the project closed | off |
| show-stripe-button | button is present on its stripe | on |
| split (side tool) | placed in the secondary half of the anchor area | off |
| weight | fraction of the frame the window takes along its axis, 0..1 | 0.33 |
| side weight | share of the anchor area a split window takes, 0..1 | 0.5 |
| order | position of the button within its stripe, -1 = not assigned yet | -1 |
| content UI type | `tabs` or `combo` | `tabs` |
| maximized | window fills the frame (windowed type) or its area | off |
| floating bounds | rectangle for floating and windowed types | none |

Weights are clamped to 0..1 when set. The internal type (docked or sliding) is remembered while the window is floating or windowed, so that
returning from float restores it.

## 2. Requirements: model, stripes, buttons

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-TW-001 | MUST | A tool window registry maps id to window info (section 1) and content. Registering an id twice is an error that does not panic. Unknown persisted ids are kept in the layout and applied when the id registers later. | Register `Foo` after loading a layout containing `Foo` at `bottom`: it appears at `bottom`. Registering `Foo` twice returns an error value, no panic. |
| WB-TW-002 | MUST | Anchors are `top`, `left`, `bottom`, `right`; `left` and `right` are vertical stripes, `top` and `bottom` horizontal. Anchor display names are the lower-case words (menus use the capitalized form: "Left", "Right", "Bottom", "Top"). | Moving a window to `Right` puts its button on the right stripe; its menu entry reads "Right". |
| WB-TW-003 | MUST | The default anchor of a window without a saved layout entry is the anchor its registration declares; without a declaration it is `left`. | A window registered without anchor opens at `left`. |
| WB-TW-004 | MUST | Each visible-capable window has a **stripe button** on its anchor's stripe, ordered by `order`; a window with `order = -1` that becomes visible gets the next free order of its anchor (max order + 1). | Show two windows with order -1 on `bottom` in sequence: the first button is left of the second. |
| WB-TW-005 | MUST | The button shows the stripe title (not the tab title) and the icon. With "Show tool window names" off (default) only the icon is shown and the title is the tooltip; with it on, icon and title are shown. | Setting off: stripe button has no text; tooltip is "Project". Setting on: text "Project" is visible. |
| WB-TW-006 | MUST | "Hide tool window bars" (`hideToolStripes`, default off) hides all stripes. Windows stay usable through their activate actions. The Notifications status widget (see `04-status-bar.md`) becomes available exactly then. | Turn the option on: all stripes disappear; Alt+1 still opens Project. |
| WB-TW-007 | SHOULD | "Show tool window numbers" (default off) prefixes the stripe button with the digit of its Alt+digit shortcut (see 6) if the active keymap binds Alt+digit (Cmd+digit on macOS) to the window's activate action. | Option on, default keymap: Project button shows "1", Commit shows "0", Terminal shows no number. |
| WB-TW-008 | MUST | The context menu of a stripe button contains: the view mode group (5), "Move to" Left / Right / Bottom / Top, "Split mode" toggle, resize group (Stretch to ...), "Hide" (hides and removes the button from the stripe: sets show-stripe-button off, anchor/pane reset to default pane). | Choose Hide on the Bookmarks button: window hidden and its button gone; window is reachable again through View > Tool Windows > Bookmarks or Alt+2, which also restores the button. |
| WB-TW-009 | SHOULD | Dragging a stripe button within or to another stripe changes order and anchor and persists them. | Drag Terminal's button from `bottom` to `right`: window info anchor becomes `right`, order is the next free of `right`. |
| WB-TW-010 | LATER | "Use Single Tool Window Bar" (top stripe merging all stripes, actions `TopStripeEnableAction`, `TopStripePinButton`, `TopStripeUnPinButton`). | Out of scope for Archipel 1.0. |

## 3. Requirements: show, hide, activate

Every action id for activating a window is `Activate<id without spaces>ToolWindow` (for example `ActivateProblemsViewToolWindow`). It is registered
automatically for every registered window and appears in the keymap under "Tool Windows".

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-TW-020 | MUST | **Activate** (`Activate<Id>ToolWindow`, default shortcut Alt+digit, section 6): if the editor currently has focus, or the window is not the active (focused) tool window, the window is shown (if hidden) and receives focus in its preferred component. If it is already the active window, the action **hides** it and focus moves to the editor. | Editor focused, press Alt+1: Project visible and focused. Press Alt+1 again: Project hidden, caret in editor. Focus in Project, press Alt+4 (Run): Run shown and focused, Project stays visible. |
| WB-TW-021 | MUST | The activate action is disabled (greyed in menus, hidden in popups) while the window is unavailable (`available = false`, for example Commit without VCS). Activating an unavailable window does nothing. | Without VCS roots, Alt+0 does nothing; View > Tool Windows > Commit is disabled. |
| WB-TW-022 | MUST | Activating a window that is visible but not focused only moves focus into it (no hide). If the application is not the foreground app, the window is only recorded as last-active and focus is requested lazily. | Project and Run visible, focus in Run: Alt+1 focuses Project and does not hide it. Project visible, focus in editor: Alt+1 focuses Project and does not hide it. |
| WB-TW-023 | MUST | **Show** (programmatic, without focus) on an already visible window is a no-op. Showing an unavailable window does nothing. | Calling show twice leaves one decorator. |
| WB-TW-024 | MUST | Showing a docked or sliding window **hides the other window occupying the same slot**: same anchor, same split flag, same type, same pane. The occupant is hidden (not removed from its stripe) and, if docked and not auto-hide, remembered on the **side stack** of that anchor. | Project (left) visible; show Commit (left, not split): Project is hidden, Commit visible. Show Structure (left, split): Project/Commit stay visible, Structure appears in the secondary half. |
| WB-TW-025 | MUST | **Hide** of a window restores the most recently stacked window of the same anchor and split flag whose stored state still matches (same pane, anchor, type, auto-hide). Stale stack entries are dropped. When no entry matches nothing is restored. | Project visible, show Commit (Project stacked), hide Commit: Project is visible again. Move Project to `right` in between: hiding Commit restores nothing. |
| WB-TW-026 | MUST | Floating and windowed windows never take part in slot sharing or the side stack. | Two floating windows on the left anchor are both visible simultaneously. |
| WB-TW-027 | MUST | Activating a window moves its id to the front of the **recently used list** (most recent first, no duplicates). | After Alt+1, Alt+4, Alt+1 the list is `[Project, Run, ...]`. |
| WB-TW-028 | MUST | When the active window is hidden by hide, the editor regains focus, provided the hidden window had focus. Hiding a window that did not have focus does not move focus. | Focus in editor, Run visible: Alt+4 (focuses Run), Shift+Escape: Run hidden, caret in editor. Focus in editor, hide Run from its stripe menu: focus stays in editor. |
| WB-TW-029 | MUST | On activate of a hidden window, "active-on-start" is set to whether focus was requested; on hide it is cleared. Visible windows restored at project open get focus only if active-on-start is true. | Close the project with Run focused, reopen: Run visible and focused. Close with Run visible but editor focused: reopen: Run visible, editor focused. |
| WB-TW-030 | SHOULD | A window flagged "do not activate on start" is hidden when the layout loads even if it was saved visible. | Layout with `Run` visible and flag set: after load Run is hidden. |

## 4. Requirements: hide actions, focus rules, Escape

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-TW-040 | MUST | **Hide Active Tool Window** (`HideActiveWindow`, default Shift+Escape, text "Hide Active Tool Window"): hides the active window, or, if none has focus, the **last active** window. Enabled only when a window exists and (focus is inside it, or it is visible and has an internal type: docked or sliding). | Focus in Terminal: Shift+Escape hides Terminal. Focus in editor, Run visible and last active: Shift+Escape hides Run. No visible window and none last active: the action is disabled. |
| WB-TW-041 | MUST | **Escape** inside a tool window runs `FocusEditor` (default Escape): focus returns to the last active editor; the window stays visible (docked pinned) or hides by its auto-hide rule (WB-TW-053). A component that consumes Escape itself (popup open, speed search active, inline edit) handles it first. | Focus in Project tree, press Escape: caret back in the editor, Project still visible. Speed search active in the tree: Escape closes the search and focus stays in the tree. |
| WB-TW-042 | MUST | **Hide All Tool Windows** (`HideAllWindows`, Ctrl+Shift+F12): hides every visible window of internal type (docked or sliding), clears all side stacks, focuses the editor and remembers the previous layout. When no window is visible and a remembered layout exists, the same action reads "Restore Windows" and restores it. The text is "Hide All Windows" while windows are visible. | Project and Run visible: Ctrl+Shift+F12 hides both, focus editor. Press again: Project and Run are visible again at their old anchors and sizes. With nothing visible and nothing remembered the action is disabled. |
| WB-TW-043 | SHOULD | Menu entries "Hide Side Tool Windows" (`HideSideWindows`) and "Hide Bottom Tool Windows" (`HideBottomWindows`) hide all docked/sliding windows of the anchor group of the active window and clear that anchor's side stack. No default shortcut. | Project (left) and Structure (left, split) visible, focus Project: Hide Side Tool Windows hides both; Run (bottom) stays. |
| WB-TW-044 | MUST | **Jump to Last Tool Window** (`JumpToLastWindow`, F12): activates the last active window if it is available (shows and focuses). Disabled when there is no last-active window or it is unavailable. | Use Run, go back to editor (Shift+Escape), press F12: Run is shown and focused. |
| WB-TW-045 | SHOULD | The tool window header has a hide button that hides the window exactly like WB-TW-040 for that window (focus moves to the editor only if the window had focus). The header also carries the window's gear menu with the view mode group (5). | Click the hide button of Run's header while focus is in the editor: Run hidden, focus stays in the editor. |
| WB-TW-046 | MUST | Alt+digit mnemonics are only the *default keymap*; the real binding comes from the active keymap (see `05-settings-dialog-and-keymap.md`, WB-KEY). Tool window activate actions are therefore normal actions that can be re-bound. | Remove Alt+1 in the keymap page, bind Ctrl+Alt+P to "Project": Ctrl+Alt+P activates Project, Alt+1 does nothing. |

## 5. View modes

The **View Mode** submenu (stripe menu, tool window gear menu) has five entries, in this order: "Dock Pinned", "Dock Unpinned", "Undock",
"Float", "Window". Exactly one is checked. Descriptions (tooltips, as in the reference): Dock Pinned: "Make a tool window docked and pinned";
Dock Unpinned: "Make the tool window docked but auto-hiding on focus lost"; Undock: "Make tool window sliding and auto-hiding on focus lost";
Float: "Move the tool window to a separate dialog"; Window: "Move the tool window to a separate frame". Legacy toggles exist for keymap use:
"Pinned Mode", "Docked Mode", "Floating Mode", "Windowed Mode", "Split Mode".

| Mode | type | auto-hide | Placement |
|---|---|---|---|
| Dock Pinned | docked | off | in the frame, takes layout space, stays open on focus loss |
| Dock Unpinned | docked | on | in the frame, takes layout space, hides when focus leaves it |
| Undock | sliding | on (implied) | overlays the editor like a drawer, hides on focus loss, does not resize the editor |
| Float | floating | - | own small window (dialog) owned by the frame, bounds persisted |
| Window | windowed | - | own top-level window with its own frame decoration; may be maximized |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-TW-050 | MUST | Switching mode changes `type` and `auto-hide` per the table, keeps the window visible, and (for docked/sliding) remembers the type as the internal type. | Project in Dock Pinned: choose Float: Project is in a dialog; choose Dock Pinned again: back at `left`, same weight. |
| WB-TW-051 | MUST | Entering Float or Window mode stores the current bounds on exit and restores them on the next entry; initial bounds are a default rectangle near the frame. | Float Run, resize, switch to Dock Pinned, Float again: same size and place. |
| WB-TW-052 | SHOULD | The Preview-like window ids that cannot dock use `sliding` instead of `docked` when asked for `docked`. | Registry rule: a window declared "never docked" never reports type `docked`. |
| WB-TW-053 | MUST | A window with auto-hide (Dock Unpinned) or type sliding hides when the user clicks into the editor or focus otherwise leaves it for a non-descendant (not when a popup or dialog owned by it takes focus). Pinned windows never auto-hide. | Dock Unpinned Project: click in editor: Project hides. Dock Pinned: click in editor: Project stays. |
| WB-TW-054 | MUST | "Dock" (stripe menu entry) docks a floating/windowed window back; "Undock" is the sliding mode of the table. | Float Run, choose Dock: Run docked at its previous anchor. |
| WB-TW-055 | SHOULD | `Open as Editor Tab` (`MoveToolWindowTabToEditorAction`, Alt+Shift+Enter) moves the selected content tab of a window that supports it into an editor tab; `Return to Tool Window` reverses it. | Terminal tab selected, Alt+Shift+Enter: a Terminal editor tab opens and the tool window tab disappears. |

## 6. Default shortcuts (default keymap, Windows/Linux)

| Action | Shortcut | Notes |
|---|---|---|
| Activate Project | Alt+1 | |
| Activate Bookmarks | Alt+2 | |
| Activate Find | Alt+3 | |
| Activate Run | Alt+4 | |
| Activate Debug | Alt+5 | |
| Activate Problems View | Alt+6 | |
| Activate Structure | Alt+7 | |
| Activate Services | Alt+8 | |
| Activate Version Control | Alt+9 | |
| Activate Commit | Alt+0 | |
| Activate Terminal | Alt+F12 | |
| Activate Unit Tests / NuGet | Alt+Shift+8 / Alt+Shift+7 | not in Archipel |
| Hide Active Tool Window | Shift+Escape | |
| Hide All Tool Windows | Ctrl+Shift+F12 | |
| Jump to Last Tool Window | F12 | |
| Restore Current Layout | Shift+F12 | |
| Maximize Tool Window | Ctrl+Shift+' (QUOTE) | |
| Stretch to Left / Right / Top / Bottom | Ctrl+Alt+Shift+Left / Right / Up / Down | |
| Focus Editor | Escape | in tool windows |
| Activate Favorites, Hierarchy | none | |
| Open as Editor Tab | Alt+Shift+Enter | |

macOS (`Mac OS X 10.5+` keymap) replaces Alt+digit by Cmd+digit for the same ten windows (Cmd+1 Project, Cmd+2 Bookmarks, Cmd+3 Find, Cmd+4 Run,
Cmd+5 Debug, Cmd+6 Problems, Cmd+7 Structure, Cmd+8 Services, Cmd+9 Version Control, Cmd+0 Commit); the Stretch actions become Ctrl+Alt+arrows.
Other keymaps (section 05 of `05-settings-dialog-and-keymap.md`) inherit the default unless they override.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-TW-060 | MUST | The default keymap binds exactly the shortcuts of the table above to the listed actions. | The keymap dump for the default keymap contains these rows and no other tool-window activation shortcuts. |
| WB-TW-061 | MUST | The mnemonic digit of a window (used for the number overlay and header tooltips) is derived from the active keymap: the first keyboard shortcut of its activate action that is Alt+digit (Cmd+digit on macOS) or digit-only modifiers Alt/Meta; otherwise none. | Keymap with Alt+F12 only: Terminal has no digit. |
| WB-TW-062 | SHOULD | Pressing the activation modifier (Alt, Cmd) twice and holding it ("Press, release and hold") temporarily overlays the stripes with digit labels, even when stripes are hidden; releasing hides them. | Hide stripes; press Alt, release, press Alt and hold: stripes overlay; release: gone. |

## 7. Maximize, resize, remembered sizes

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-TW-070 | MUST | **Maximize Tool Window** toggles the window that owns focus (data key `TOOL_WINDOW`); its text is "Maximize Tool Window" and, when maximized, "Restore Tool Window Size". Disabled (not hidden) when no tool window owns focus. Maximizing a docked window makes it fill the frame center area; the editor area is hidden until restored. | Focus in Run: Ctrl+Shift+' maximizes (the editor disappears); same keys restore the previous size; menu text flips. |
| WB-TW-071 | MUST | **Stretch to Left / Right / Up / Down** change the weight of the active docked or sliding window of the main frame. Left/Right act only for `left`/`right` anchors, Up/Down only for `top`/`bottom`. Direction: for a `left` window "Stretch to Right" widens and "Stretch to Left" narrows; for a `right` window the reverse. For a `bottom` window "Stretch to Top" grows it and "Stretch to Bottom" shrinks it; for a `top` window the reverse. | `left` window: Ctrl+Alt+Shift+Right widens by one step; Ctrl+Alt+Shift+Left narrows. `bottom` window: Ctrl+Alt+Shift+Up grows. `left` window and Up: action disabled. |
| WB-TW-072 | MUST | The stretch actions are hidden/disabled when a regular editor (not console, not viewer) has focus, when the window is floating or windowed, when it is not visible, and for windows not in the main frame. | Focus in editor: Ctrl+Alt+Shift+Right does nothing for tool windows (the editor keymap may use it). |
| WB-TW-073 | MUST | The weight is clamped to 0..1; `stretch` never produces 0 or a size below the window's minimum. | Stretching a window to the minimum keeps it visible. |
| WB-TW-074 | MUST | **Remembered sizes**: with "Remember size for each tool window" on, each window keeps its own weight. With it off, all docked windows on one anchor share one size (the *unified anchor weight*): activating a docked window assigns that anchor's unified weight to it. Defaults: Old UI on, New UI off. A pair of split windows on one anchor never uses the unified weight when another visible docked window of the other split half exists on the same anchor (they keep individual weights). | New UI default: resize Project to 300 px; show Commit (same anchor): Commit also 300 px. Option on: Commit keeps its own saved size. |
| WB-TW-075 | MUST | The default weight of a window without saved size is 0.33 of the frame along its axis. | Fresh layout, Run at bottom: its height is about one third of the frame content height. |
| WB-TW-076 | SHOULD | Size persists per project: `weight`, `side weight`, floating bounds and `maximized`. | Resize, close and reopen project: size restored. |

## 8. Split (side) windows, content UI, widescreen

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-TW-080 | MUST | A window with split on is placed in the secondary half of its anchor area: the left/right anchors divide **vertically** (top half primary, bottom half secondary) unless the "side-by-side layout on the left/right" option (`leftHorizontalSplit`, `rightHorizontalSplit`, default off) is on, in which case the halves sit side by side. For `top`/`bottom` the halves are side by side (left primary, right secondary). The ratio is the side weight (default 0.5). | Left anchor with Project and Structure (split) visible: Project on top, Structure below, equal height. |
| WB-TW-081 | MUST | Primary and secondary windows of one anchor are independent slots: showing a primary window never hides a secondary one. | See WB-TW-024. |
| WB-TW-082 | SHOULD | "Widescreen tool window layout" (default off) maximizes the height of vertical windows by limiting the width of horizontal ones (bottom windows do not run under left/right windows). | With the option on, the left window extends to the frame bottom; the bottom window starts right of it. |
| WB-TW-083 | MUST | "Split Mode" in the stripe menu toggles `split` for that window; the window moves to the other half at once. | Toggle Structure from split to non-split while Project is visible: Project is hidden (slot conflict, WB-TW-024) and Structure takes the primary slot. |
| WB-TW-084 | SHOULD | Content UI type `combo` shows a dropdown of contents instead of tabs; "Group Tabs" (`ToggleContentUiTypeMode`) toggles between `tabs` and `combo`. | Toggle on Run: its tabs collapse into a combo box; toggle again: tabs. |
| WB-TW-085 | LATER | Tool window tab split (`TW.SplitRight`, `TW.SplitDown`, `TW.SplitAndMoveRight`, `TW.SplitAndMoveDown`, `TW.Unsplit`, `TW.MoveToNextSplitter`, `TW.MoveToPreviousSplitter`, `TW.CloseAllTabs`, `TW.CloseOtherTabs`): split a window's tab area into two tab groups. | Out of scope for 1.0; the actions exist as disabled entries. |

## 9. Layouts

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-TW-090 | MUST | The layout (all window infos plus editor-area state) persists per project and is applied before the first frame is shown. | Move Project to `right`, reopen: it is on `right`. |
| WB-TW-091 | MUST | **Restore Current Layout** (`RestoreDefaultLayout`, Shift+F12, "Restore Current Layout") resets all windows to the saved copy of the currently selected named layout; the **Default** layout (`RestoreFactoryDefaultLayout`, "Default") resets to factory defaults. | Rearrange windows, press Shift+F12: arrangement returns to the layout saved last. |
| WB-TW-092 | SHOULD | "Save Changes in Current Layout" (`StoreDefaultLayout`) overwrites the selected layout with the current arrangement; "Save Current Layout as New..." (`StoreNewLayout`) asks for a name; named layouts can be restored and deleted, except the active one ("The active layout cannot be deleted"). | Save as "Debugging", restore Default, restore "Debugging": the arrangement returns. |
| WB-TW-093 | MUST | **Hide All** remembers the layout in memory only (not persisted) for the restore toggle of WB-TW-042. | After IDE restart the toggle starts as "Hide All Windows". |

## 10. Windows registered by the Community product (defaults)

Anchor and split come from registration; shortcut from the default keymap. "Owner" names the spec area that defines contents.

| Window id (title) | Anchor | Split | Shortcut | Owner |
|---|---|---|---|---|
| Project (Project) | left | no | Alt+1 | `02-project-view.md` |
| Bookmarks (Bookmarks) | left | yes | Alt+2 | `03-structure-problems-todo.md` |
| Structure (Structure) | left | yes | Alt+7 | `03-structure-problems-todo.md` |
| Commit (Commit) | left | no | Alt+0 | VCS area |
| Pull Requests, Merge Requests | left | no | none | Git hosting area |
| Settings (Settings), Backup and Sync History | left | no | none | `05-settings-dialog-and-keymap.md` (settings opened as tool window, non-modal) |
| Problems View (Problems) | bottom | no | Alt+6 | `03-structure-problems-todo.md` |
| TODO (TODO) | bottom | no | none | `03-structure-problems-todo.md` |
| Terminal (Terminal) | bottom | no | Alt+F12 | Terminal area |
| Version Control (Version Control) | bottom | no | Alt+9 | VCS area |
| Find, Run, Debug, Services, Hierarchy | bottom | no | Alt+3, Alt+4, Alt+5, Alt+8, none | other areas (product knowledge: all `bottom`) |
| Notifications (Notifications) | right | yes | none | `07-notifications-and-background-tasks.md` |
| Gradle, Ant, Commander, UI Theme Color Picker | right | no | none | build / internal areas |
| Coverage (Coverage) | right | yes | none | test runner area |
| Statistics Event Log (two ids), EventQueue Watcher | bottom | no | none | internal, LATER |
| Modules (internal) | right | no | none | internal, LATER |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-TW-100 | MUST | Archipel registers at least Project, Bookmarks, Structure, Problems, TODO, Notifications, Settings (non-modal variant LATER), Terminal and the VCS/Run windows of the other areas with the anchors, split flags and shortcuts of the table. | A test dumps the registry for a fresh profile and compares against the table. |
| WB-TW-101 | MUST | Titles shown on stripe/tab bars for these ids: Project "Project", Bookmarks "Bookmarks", Structure "Structure", Problems View "Problems", TODO "TODO", Notifications "Notifications". The stripe title may differ from the id (`Problems View` shows "Problems"). | Stripe tooltip of `Problems View` is "Problems". |

## 11. Frame toggles, focus shortcuts, IDE zoom; inventory coverage

View menu toggles (group `UIToggleActions`, View > Appearance): **Main Menu** (`ViewMainMenu`), **Toolbar** (`ViewNewToolbarAction`; classic `ViewObsoleteToolbarAction` "Toolbar Classic"), **Status Bar** (`ViewStatusBar`), **Tool Window Bars** (`ViewToolButtons`),
**Presentation Assistant**. Defaults: main menu on (not on macOS where the global menu is used), status bar on, navigation bar on, main toolbar off in the classic layout, tool window bars on.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-TW-110 | SHOULD | The View > Appearance toggles above exist with these texts, show a check mark for the current state and apply at once; the state persists in the application settings. | Toggle "Tool Window Bars" off: stripes hidden (same as hideToolStripes, WB-TW-006); toggle on: restored. |
| WB-TW-111 | SHOULD | `FocusMainToolbar` (Alt+PageUp) and `FocusStatusBar` (Alt+PageDown) move keyboard focus to the main toolbar and the status bar; arrow keys move between items, Escape returns focus to the editor. `MainMenuButton.ShowMenu` (Alt+Backslash) opens the collapsed main menu. | Alt+PageDown: first status bar widget is focused; Escape: editor focused. |
| WB-TW-112 | SHOULD | IDE zoom: Zoom In (`ZoomInIdeAction`, Shift+Alt+=), Zoom Out (`ZoomOutIdeAction`, Shift+Alt+-), Reset (`ResetIdeScaleAction`, Shift+Alt+0), plus the "Zoom" scheme entry (`ChangeIdeScale`, `SwitchIdeScaleAction` "Zoom IDE") scale the whole UI; the step list is the platform's scale list; Linux keymaps use Shift+Ctrl+Alt for these. | Shift+Alt+=: UI one step larger; Shift+Alt+0: back to 100%. |
| WB-TW-113 | LATER | Distraction Free, Zen, Presentation, Full Screen and Compact modes (`ToggleDistractionFreeMode`, `ToggleZenMode`, `TogglePresentationMode`, `ToggleFullScreen`, `ToggleCompactMode`); the Minimap actions; the project-window cycling actions (`NextWindow`, `PreviousWindow`, `PreviousProjectWindow`, `ZoomCurrentWindow`, `MinimizeCurrentWindow`). | n/a |

Inventory `window-ui.md` (137 actions + the Notifications window), disposition:

| Disposition | Items |
|---|---|
| Covered in this file | `ActivateFindToolWindow`, `ActivateHierarchyToolWindow`, `Decrement/IncrementWindowHeight/Width`, `DockPinnedMode`, `DockToolWindow`, `DockUnpinnedMode`, `FloatMode`, `UndockMode`, `WindowMode`, `ToggleDockMode`, `ToggleFloatingMode`, `TogglePinnedMode`, `ToggleSideMode`, `ToggleWindowedMode`, `HideActiveWindow`, `HideAllWindows`, `HideBottomWindows`, `HideSideWindows`, `HideToolWindowStripeButton`, `MaximizeToolWindow`, `ResizeToolWindowDown/Left/Right/Up`, `RestoreDefaultLayout`, `RestoreFactoryDefaultLayout`, `StoreDefaultLayout`, `StoreNewLayout`, `ToggleContentUiTypeMode`, `ToolWindowShowNamesAction`, `ViewToolButtons`, `FocusEditor`, `ChangeToolWindowLayout`, `ViewMainMenu`, `ViewNewToolbarAction`, `ViewObsoleteToolbarAction`, `ViewToolBar`, `ViewStatusBar`, `FocusMainToolbar`, `FocusStatusBar`, `MainMenuButton.ShowMenu`, `ChangeIdeScale`, `SwitchIdeScaleAction`, `ZoomInIdeAction`, `ZoomOutIdeAction`, `ResetIdeScaleAction` |
| Covered in `07` / `04` | `CloseAllNotifications`, `CloseFirstNotification`, `MarkNotificationsAsRead`, `ShowProcessWindow`, `AutoShowProcessWindow`, tool window `Notifications`; `StatusTextModeAction` (04) |
| LATER, tool window tabs (WB-TW-085, WB-TW-010) | `TW.CloseAllTabs`, `TW.CloseOtherTabs`, `TW.MoveToNextSplitter`, `TW.MoveToPreviousSplitter`, `TW.SplitAndMoveDown`, `TW.SplitAndMoveRight`, `TW.SplitDown`, `TW.SplitRight`, `TW.Unsplit`, `PinToolwindowTab`, `TopStripeEnableAction` |
| LATER, window modes (WB-TW-113) | `ToggleCompactMode`, `ToggleDistractionFreeMode`, `ToggleFullScreen`, `TogglePresentationMode`, `TogglePresentationAssistantAction`, `ToggleZenMode`, `ChangeView`, `ColoredHeaderAction`, `NextWindow`, `PreviousWindow`, `PreviousProjectWindow`, `ZoomCurrentWindow`, `MinimizeCurrentWindow`, `MaximizeActiveDialog`, `NonModalWindow.PinUnpin`, `main.toolbar.Filename`, `main.toolbar.Project`, `ToggleMinimap`, `ToggleMinimapScaleMode`, `EnableMinimap`, `DisableMinimap`, `OpenMinimapSettings` |
| LATER, owned by the editor-area (tabs, splits) specification, not by the workbench | `ChangeSplitOrientation`, `CloseActiveTab`, `CloseAllEditors`, `CloseAllEditorsButActive`, `CloseAllReadonly`, `CloseAllToTheLeft`, `CloseAllToTheRight`, `CloseAllUnpinnedEditors`, `CloseContent`, `CloseEditor`, `ConfigureEditorTabs`, `EditorTab`, `GoToLastTab`, `GoToTab1`..`GoToTab9`, `KeepTabOpen`, `MaximizeEditorInSplit`, `OpenEditorInOppositeTabGroup`, `PinActiveEditorTab`, `PinActiveTab`, `PinActiveTabToggle`, `ReopenClosedTab`, `SplitHorizontally`, `SplitVertically`, `StretchSplitToBottom/Left/Right/Top`, `TabList`, `Unsplit`, `UnsplitAll`, `TextEditorWithPreview.*` (5 layout actions), `IncrementalSearch`, `PreviousOccurence` (see CI-NAV-021) |
| LATER, other areas | `ByteCodeDecompiler`, `BytecodeViewer` (Java tooling), `CommanderSwapPanels`, `CommanderSyncViews` (Commander window, LATER) |

## 12. Open questions

1. Exact pixel minimum sizes and the registry values `ide.windowSystem.hScrollChars` / `vScrollChars` (stretch step): not read; the implementation picks a step and keeps it stable. Needs a measurement on a running reference IDE.
2. Escape handling when a tool window is in `sliding` mode: whether Escape hides it. The keymap binds Escape to FocusEditor only; auto-hide on focus loss then hides it (assumed). Needs a reference run.
3. Behavior of "Hide" of a split primary window when the secondary is visible alone: whether the secondary expands. Assumed yes (slot disappears, stretch fills).
4. The `FocusEditor` double-handling with `EditorEscape` (both bound to Escape in the default keymap): precedence rule inside editors is in `docs/specs/codeinsight` and not repeated here.

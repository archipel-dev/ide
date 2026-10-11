# Human walkthrough audit v0.0.8

A first-hour walkthrough of Jetthing `0.0.8` (`b4836d0`, debug build) done the way a person
uses an IDE: every interaction was a real X11 mouse move, click, double click, drag, scroll
or key press sent with `xdotool` to the window on `Xvfb :77` (1440×900, `JETTHING_RENDERER=glow`,
no window manager). No command was dispatched and no input was injected through the control
socket; it was used read-only for the `state` snapshot and for a few PNGs. Screenshots are in
`evidence/` (what the X server showed, 256 colours).

Projects: a Java/Gradle sample (`App.java`, `Greeter.java`, Gradle 8.14, JDK 21, jdtls and
java-debug installed through the in-app "Set up" link) and a Rust sample created with the
New Project wizard itself (`cargo` available, `rust-analyzer` not installed). Not covered:
Android and Kotlin generators beyond opening their wizard pages, native file dialogs
(`Browse…` spawns an OS dialog that has no desktop under Xvfb), the Plugins marketplace tab,
Bazel, AI.

Why the jank was invisible to the existing checks: `docs/parity/scenes` and the unit tests
drive the workbench through `execute`/`key`/`type` on the control socket, so a control that
is painted off-screen, a popup that only reacts to Enter, or a widget with no click handler
all pass. This document records what breaks when the only input is the pointer and the
keyboard, groups it by root cause, and `cargo xtask ux-smoke` (see below) keeps the golden
paths from regressing. The contract every component must meet is
[`interaction-spec.md`](interaction-spec.md).

Severity: **blocker** = a new user cannot complete a golden path or loses work;
**major** = the path works but needs a workaround, a second attempt or a shortcut a new user
does not know; **polish** = visibly unfinished. "Expected" describes the observable
behaviour of IntelliJ IDEA 2025.x (New UI) from its public documentation; no JetBrains
code or artwork was consulted.

## Findings

| ID | Area | What happens (steps) | Expected on the IntelliJ platform | Sev | Evidence | Suspected code | Cluster | Status |
|---|---|---|---|---|---|---|---|---|
| UX-001 | First run | With the "Welcome to Jetthing" dialog open, click **New Project** behind it. The wizard opens on top of the first-run dialog; Escape then closes the *first-run* dialog underneath, the wizard stays. | Dialogs are modal: the window behind is dimmed and inert; Escape closes the top-most dialog. | blocker | [01](evidence/01-first-run-stacked-dialogs.png) | `first_run.rs` (no backdrop); Escape chain in `lib.rs::handle_shortcuts` orders `first_run_open` before `dialog` | C1 | **fixed** (modal backdrop, `popups::modal_backdrop`) |
| UX-002 | New Project | Resize the window to the 800×500 minimum and open New Project. The content row is a fixed 420 pt, the frame runs past the bottom edge; the button row sits on the last pixels. (v0.0.7 shipped with Create entirely off-screen.) | Dialogs size to the screen and keep their button row visible; content scrolls. | blocker | [02](evidence/02-new-project-800x500-clipped.png) | `new_project.rs::CONTENT_HEIGHT` | C1 | **fixed** (`content_height()` shrinks with the window; smoke scene `new-project-in-minimum-window`) |
| UX-003 | New Project | Type a Location that does not exist yet (the natural thing for a new project), click Create: `command failed: filesystem error: No such file or directory (os error 2)`. | The location is created; errors name the field and the path. | blocker | [03](evidence/03-new-project-raw-error.png) | `jetthing-project::generator::create_project_with` canonicalizes the parent | C5 | **fixed** (`create_dir_all` + `ProjectError::CreateLocation`) |
| UX-004 | Open Project | Type a wrong path in Open Project, press Enter: `command failed: filesystem error: No such file or directory (os error 2)`. | "There is no directory at …" next to the field. | major | [21](evidence/21-open-project-raw-error.png) | `jetthing-project::Project::open` | C5 | **fixed** (`ProjectError::NotFound`) |
| UX-005 | Dialogs | No dialog has a default (accent) button except Delete; button order differs per dialog: `Create · Cancel`, `Cancel · Delete`, `Browse… · Open · Cancel`, `Clone · Cancel`; Rename has no buttons at all; no title-bar close or `?`; dialogs sit at 20 % from the top instead of centred. | One default button (Enter), a consistent `[Cancel] [OK]` order per platform, a title bar, centred placement. | major | [08](evidence/08-rename-dialog.png) [09](evidence/09-delete-dialog.png) [22](evidence/22-get-from-vcs-focus-ring.png) | `popups.rs::popup_frame`, each dialog's own button row | C1 | open |
| UX-006 | Push | Push with no remote: a small error box appears at the dialog anchor, overlapping the Commit tool window, with a raw `Git command failed:` prefix. | A dialog with a clear message and the remedy as an action ("Define remote"). | polish | [20](evidence/20-push-dialog-error.png) | `push_dialog.rs` | C1, C5 | open |
| UX-007 | Project tree | Click a row, type `Car`: nothing happens (speed search), the selection stays. | Typing in a tree starts speed search: matching rows are highlighted and selected. | major | [06](evidence/06-tree-speed-search-and-drag.png) | `toolwindows.rs` tree rows (`Sense::click` only, no focus id) | C2 | open |
| UX-008 | Project tree | Click a row, press ↓/↑/←/→: the selection does not move, directories do not collapse/expand. Only Enter is handled, and only while the pointer is over the panel. | Arrow keys move the selection, ← collapses / → expands, Enter opens. | major | [06](evidence/06-tree-speed-search-and-drag.png) | `toolwindows.rs` (`ui_contains_pointer()` + Enter only) | C2 | open |
| UX-009 | Project tree | Drag `README.md` onto `src`: no drop indicator, nothing moves. | Drag and drop moves files between directories with a drop target highlight. | major | [06](evidence/06-tree-speed-search-and-drag.png) | `toolwindows.rs` (no `drag_started`/`dnd` handling) | C4 | open |
| UX-010 | New Project | Click a generator, press ↓: the selection does not move; the same for Recent Files when the filter field is not focused. | Lists are keyboard-navigable. | major | [26](evidence/26-generator-list-no-keyboard.png) | `new_project.rs` generator list (`widgets::list_row`) | C2 | open |
| UX-011 | Editor tabs | Drag a tab across the bar: no ghost, no insertion marker, the order does not change. | Tabs can be reordered by dragging; the dragged tab follows the pointer. | major | [10](evidence/10-tab-drag-no-feedback.png) | `editor.rs::tabs` | C4 | open |
| UX-012 | Editor tabs | Double-click a tab or a tool window header: nothing. Right-click the empty tab-bar area: the per-tab menu (Close Tab …) opens for the first tab. | Double-click maximises the editor / tool window; the empty tab bar has no per-tab actions. | polish | [12](evidence/12-context-menus-tabbar-status-toolbar.png) | `editor.rs::tabs`, `widgets::tool_window_header` | C4 | open |
| UX-013 | Menus | Open the main menu (☰ → File). ↓/↑ do not move a highlight, →/← do not switch to Edit. Two clicks are needed to reach any menu. | Menus are fully keyboard-navigable (arrows, Enter, mnemonics); the ☰ button opens the menu directly. | major | [15](evidence/15-menu-keyboard-navigation.png) [36](evidence/36-hamburger-menu-bar.png) | `main_menu.rs` (egui `MenuBar`/`Popup` without key handling), `context_menus.rs` | C2 | open |
| UX-014 | Status bar | Click `1:1`, `LF`, `UTF-8`, `4 spaces`, the toolchain warning: they highlight on hover and press but do nothing. | Caret widget opens Go to Line, LF opens the line-separator popup, UTF-8 the encoding popup, indent the indent popup. | major | [13](evidence/13-status-bar-widgets-inert.png) | `chrome.rs::status_bar` (`flat_widget` results dropped) | C4 | open |
| UX-015 | Status bar | A long status message (e.g. "Gradle sync finished: 1 project, 40 tasks") paints over the breadcrumbs when the right-hand widgets are wide (jdtls progress). | Text is elided; the navigation bar stays readable. | major | [14](evidence/14-status-bar-overlap.png) | `chrome.rs::status_bar` (`flat_widget` ignores available width) | C5 | **fixed** (`widgets::flat_widget_elided`) |
| UX-016 | Find bar | Ctrl+F, type, press Escape: the bar stays open, focus leaves the field; a second Escape closes it. | One Escape closes the bar and returns to the editor. | major | [30](evidence/30-find-bar-open-after-escape.png) | `lib.rs::handle_shortcuts` tests focus that egui already cleared on Escape | C3 | **fixed** |
| UX-017 | Completion | With the completion popup open, click a tree row: focus moves, the popup stays until Escape. | The popup closes when the editor loses focus. | major | [17](evidence/17-completion-persists-after-focus-change.png) | `completion_ui.rs::show` | C3 | **fixed** (closes on press outside while unfocused) |
| UX-018 | Quick documentation | Hovering an identifier shows a documentation popup that stays after the pointer moves on, covers the Debug tool window and swallows the next Escape. Hovering an error shows the diagnostic *and* a documentation popup for a different symbol. | One hover popup at a time, dismissed when the pointer leaves; the diagnostic tooltip alone over an error. | major | [18](evidence/18-error-hover-two-popups.png) [19](evidence/19-debug-paused-sticky-quickdoc.png) | `doc_ui.rs::show_quick_doc`, `state.docs.popup` lifetime | C3 | open |
| UX-019 | Escape handling | Which layer Escape closes is a 20-branch `if/else` over unrelated flags; a hidden layer (stale quick-doc, invisible completion session) can take the key from the visible one. | A popup stack: Escape always closes the top-most visible layer. | major | — | `lib.rs::handle_shortcuts` | C3 | partly fixed (completion visibility) |
| UX-020 | Focus | The focus ring on text fields is a 1 px hairline; Tab in Settings and in the New Project form gives no visible indication of where focus went. Tab order in New Project skips nothing, which is good. | A 2 px accent focus ring on every focusable control. | major | [04](evidence/04-new-project-focus-ring.png) | `theme.rs` visuals (`selection.stroke`) | C2 | open |
| UX-021 | Pressed states | Welcome tiles, tree rows, tabs and list rows have hover but no distinct pressed state. | Hover, pressed and selected states differ. | polish | [25](evidence/25-tooltip-delay-and-pressed-state.png) | `welcome.rs::action_button`, `widgets::list_row`, `editor.rs::tabs` | C4 | open |
| UX-022 | Tooltips | Tooltips appear after ~0.5 s and the welcome tiles repeated their own label. | Initial delay ~1.2 s (Settings › Appearance › Tooltip); no tooltip that duplicates the label. | polish | [25](evidence/25-tooltip-delay-and-pressed-state.png) | egui `style.interaction.tooltip_delay`, `welcome.rs` | C5 | partly fixed (tile tooltip removed) |
| UX-023 | Feedback | Commit and "toolchain installed" only flash in the status bar; Ctrl+Z with nothing to undo shows `command is disabled: editor.undo`; a failed Gradle build lists every task green and hides the failing one. | Balloon notifications for completed operations; no internal ids in user-facing text; the failing task is marked. | polish | [31](evidence/31-build-failed-no-failing-task.png) | `notifications.rs` (only update/LSP balloons), `CommandError::Disabled` display, `build_window.rs` | C5 | open |
| UX-024 | Empty states | Empty Project / Rust / Android generator pages show three fields and 300 pt of nothing; the welcome Customize/Plugins/Learn pages are a heading and one button. | Pages fill the space with the relevant options or a description. | polish | [05](evidence/05-new-project-empty-generator.png) [23](evidence/23-welcome-info-pages.png) | `new_project.rs`, `welcome.rs::info_page` | C5 | open |
| UX-025 | Context menus | Right-click on the status bar, the toolbar, the Project header or a recent-project row shows nothing; the tree "New" submenu offers Java/Kotlin classes in a Rust project. | Every surface has a context menu or none is expected; entries depend on context. | polish | [12](evidence/12-context-menus-tabbar-status-toolbar.png) | `chrome.rs`, `welcome.rs`, `context_menus.rs` | C4 | open |
| UX-026 | Performance | With `rust-analyzer` failing to start, the idle workbench used 34 % of a core (`top`, nothing moving); the Java project idled at 2 %. | An idle window repaints only on events. | major | measurement below | `lib.rs::frame` `request_repaint_after` while `lsp.is_busy()` | C6 | open |
| UX-028 | Project tree (main after #217) | Double-click a file: it opens (the first click already showed it as a preview) but nothing has keyboard focus; typing goes nowhere until the editor is clicked. Found by `ux-smoke` after merging main. | Opening a file by double-click or Enter focuses its editor. | blocker | smoke log | `toolwindows.rs` open handler; egui surrenders focus requested in the frame of a click elsewhere | C3 | **fixed** (`focus_editor_next_frame`, requested one frame later) |
| UX-027 | Diff viewer | The "Collapse unchanged" checkbox label wraps onto two lines in the toolbar. | Toolbar labels never wrap. | polish | [29](evidence/29-diff-viewer.png) | `diff_ui.rs` | C5 | open |

What worked well and should be kept: Search Everywhere, Find Action, Find in Files, Settings,
the Plugins page, Recent Files, completion and quick-fix once jdtls is indexed, Go to
Definition, breakpoints by clicking the line number, the debugger (frames, variables, step,
resume), the Commit tool window, the diff viewer, the branches popup, the terminal, window
resize cursors, context menus clamping to the window edge, and Escape closing a combo box
without closing its dialog.

### Measurements

| What | Value | How |
|---|---|---|
| Idle CPU, Rust project, rust-analyzer failed | 34 % | `top -d 5` over 5 s, pointer still |
| Idle CPU, Java project, jdtls starting | 2.4 % | same |
| Idle CPU, welcome screen | 0 % | same |
| Tooltip initial delay | between 0.25 s and 0.6 s | screenshots at 0.25/0.6/1.2 s |
| New Project dialog height at 1440×900 | 510 pt (fixed 420 pt content) | before the fix |

## Root-cause clusters

### C1 — No shared dialog component (#220)

Every dialog assembles itself: `popup_frame` gives a frame at 20 % from the top and nothing
else. Modality, placement, sizing to the window, a title bar, the button row, the default
button, Enter/Escape and focus-on-open are re-implemented (or forgotten) per dialog
(`new_project.rs`, `clone_dialog.rs`, `popups.rs` × 9, `push_dialog.rs`, `run_config_dialog.rs`,
`settings_dialog.rs`, `first_run.rs`). Findings: UX-001, UX-002, UX-005, UX-006.

Fix: one `jetthing_ui::dialog::Dialog` builder (title, optional help, content closure,
`buttons: [Button]` with exactly one `default` and one `cancel`, `modal: bool`) that owns the
backdrop, centring with clamping to the content rect, max size = window − 40 pt with the
content in a scroll area, the button row pinned at the bottom, Enter → default button
(unless a multi-line field or an open combo has focus), Escape → cancel, focus of the first
field on open, and the layer registration of C3. `modal_backdrop` added by this audit is
the first piece.

### C2 — No keyboard model for lists, trees and menus (#221)

Rows are `allocate_exact_size(…, Sense::click)` with hand-painted selection
(`widgets::list_row`, the project tree, the generator list, Recent Files, the structure
popup). None of them has a focus id, so arrow keys, Home/End, Page keys, type-ahead
(speed search) and Enter are per-widget afterthoughts; menus built on egui `Popup` have no
keyboard handling at all. Findings: UX-007, UX-008, UX-010, UX-013, UX-020.

Fix: a `jetthing_ui::list::ListView`/`TreeView` primitive that is focusable, owns
selection + anchor, handles ↑↓←→ Home End PgUp PgDn Enter Space, exposes speed search
(typed prefix shown in a small field at the top-left of the component, matches highlighted,
Escape clears), scrolls the selection into view and paints hover/selected/focused states from
the theme. Menus get the same navigation through a `MenuPopup` wrapper (↓↑ move, → opens a
submenu, ← closes it, Enter activates, mnemonics). The theme gets a visible 2 pt focus ring.

### C3 — Popup lifecycle is not owned centrally (#222)

Completion, quick documentation, parameter info, intentions, context menus, Search
Everywhere, the find bar and ten dialogs each keep their own `open` flag; Escape, click-outside
and focus-loss handling are written per popup, and `handle_shortcuts` decides precedence with
a 20-branch chain. A stale or invisible layer therefore eats Escape or stays on screen.
Findings: UX-016, UX-017, UX-018, UX-019.

Fix: a `PopupStack` in `IdeApp` where every transient layer registers for the frame with a
kind (`Modal`, `Popup`, `Hover`) and a close callback. Escape pops the top visible layer; a
pointer press outside a `Popup`/`Hover` rect closes it; focus leaving the editor closes
editor-anchored popups; `Hover` layers close when the pointer leaves anchor+popup; opening a
`Modal` closes all popups. `control_state` reports the stack so scenes and `ux-smoke` can
assert it.

### C4 — Mouse gestures unimplemented in composite widgets (#223)

Custom-painted controls accept a click and nothing else: no drag (tabs, tree), no
double-click semantics (headers), no pressed state, inert status-bar widgets, no context
menu on chrome surfaces. Findings: UX-009, UX-011, UX-012, UX-014, UX-021, UX-025.

Fix: a `widgets::interactive_row` that returns hover/pressed/selected and supports
`Sense::click_and_drag` with egui's `dnd` payload; tab reordering and tree moves implemented
on top of it with an insertion marker and a drop-target highlight; every status-bar widget
dispatches a command and opens its popup; headers toggle maximise on double-click.

### C5 — Feedback, messages and empty states (#224)

Errors surface the internal error chain (`command failed: filesystem error … (os error 2)`,
`command is disabled: editor.undo`, `Git command failed:`), success only flashes in the status
bar, long status text overflowed, pages are empty, tooltips repeat labels. Findings: UX-003,
UX-004, UX-006, UX-015, UX-022, UX-023, UX-024, UX-027.

Fix: `CommandError` carries a user-facing message and an optional remedy action; a
`notifications::balloon(kind, title, actions)` API used by commit, push, toolchain install
and build results; inline validation under the field (red text + disabled default button)
for dialogs; `elided` text helpers in all chrome; tooltip delay and wording rules from the
spec.

### C6 — Repaint while idle (#225)

`frame()` requests repaints on timers while any background service reports busy
(`lsp.is_busy()`, toolchains, runner). A language server stuck in "starting"/"failed"
keeps the whole window repainting at 10 Hz (UX-026).

Fix: services wake the UI through `state.waker`/`repaint` callbacks when they have news
(they already exist for files, search, runner); the timers go, and `JETTHING_DEBUG_FRAMES=1`
logs frames per second so `ux-smoke` can assert an idle budget.

## Fixed in this change

- UX-001 first-run dialog is modal (`popups::modal_backdrop`, used by `first_run.rs`).
- UX-002 New Project content height follows the window (`new_project::content_height`).
- UX-003 the wizard creates a missing location; errors name the path.
- UX-004 Open Project says "there is no directory at …".
- UX-015 status text is elided before it reaches the breadcrumbs.
- UX-016 one Escape closes the find bar.
- UX-017 the completion popup closes when a click takes focus elsewhere.
- UX-022 (part) welcome tiles no longer repeat their label as a tooltip.
- UX-028 double-click open in the tree focuses the editor again (regression on main after #217, caught by `ux-smoke`).

Everything else is tracked as one GitHub issue per cluster (label `ux`): C1 #220, C2 #221, C3 #222,
C4 #223, C5 #224, C6 #225.

## Regression harness

`cargo xtask ux-smoke` (`xtask/src/ux_smoke.rs`, CI: `.github/workflows/ux-smoke.yml`,
optional, runs on pull requests touching the UI crates) starts Xvfb, builds and launches the
IDE with a fresh home and drives four scenes with `xdotool` only: the first-run dialog is modal,
New Project is completed with the mouse, the same in the 800×500 minimum window (Create must
be on screen), and on a Cargo project: tree expand, double-click open, context menu + Escape,
Search Everywhere from the toolbar, tab close button, tool window hide/show, ☰ → File →
Settings and Escape, Run from the toolbar. Outcomes are checked against the read-only `state`
snapshot; failures save `<scene>-failed.png`. Coordinates live in `ux_smoke::layout` and
follow the layout promises in `interaction-spec.md`.

## Main moved during the audit

`origin/main` advanced to 0.0.9 while this walkthrough ran (#213 tree/overlay caching, #215 pointer
cursor on buttons and gutter-popup hover, #216 generate/hierarchy popups, #217 editor splits and
preview tabs, #218 debugger watches/conditional breakpoints). The findings and screenshots describe
0.0.8 (`b4836d0`); the fixes and the smoke harness were re-verified after merging main. #215 may
already soften UX-021 (pressed/hover states on buttons) and should be re-checked with `ux-smoke`
screenshots before closing #223.

## Decisions taken without asking

- Read-only use of the control socket (`state`, `screenshot`) is allowed in the harness;
  all input is X11 input.
- Fixed logical coordinates for a 1440×900 / 800×500 window are acceptable for the smoke
  harness; a future widget-rect query on the socket can replace them.
- The Java sample used in the walkthrough had its JUnit dependency removed because Maven
  Central is rate-limited from the audit environment; it is not part of the repository.
- The IntelliJ-platform behaviours cited come from the public IntelliJ Platform UI
  Guidelines and IntelliJ IDEA help; no JetBrains source was read.

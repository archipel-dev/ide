# Android Studio parity programme

The plan after the kit foundation (archipel-kit v0.2.0 and the workbench chrome on kit
components): one vertical slice per PR, ordered by daily-use value. Every slice ends with
its parity rows updated from evidence, an audit test (`archipel_kit::testing::audit`) for
the UI it touches, kittest tests that use pointer and key events (not command dispatch),
and an Xvfb walkthrough with real mouse and keyboard input whose screenshots go into
`docs/ux/evidence/`. "Kit:" names what the kit has to grow first (a kit PR with tests,
snapshots, docs and a gallery entry, then a kit release the IDE moves to).

Reference: Android Studio Rabbit 1 (2026.2.1, Islands Dark) on Linux, driven under Xvfb on
the same Kotlin sample as the IDE (`docs/ux/evidence/kit-chrome/README.md` lists what was
compared). Android Studio screenshots stay outside the repository (clean-room); only
observations are written down.

## Where the IDE stands

- Chrome on the kit (the foundation PR, kit v0.3.0): `WindowToolbar` with `ToolbarButton` project, VCS,
  run configuration and device widgets, `Avatar` project tile and the project-color glow,
  `IconButton::large` for Build/Run/Debug/Stop/Search/Settings, `ToolStripe` with
  `StripeButton`s on both sides, tool window title bars from `Panel`, and a status bar of
  `Breadcrumbs` and `StatusBarItem`s. `WindowChrome` configures the native window.
- Still hand-painted (see `xtask/design-lint-baseline.txt` and the area table in
  `docs/ui-migration.md`): editor tabs, the editor's own popups (completion, intentions,
  quick documentation, parameter info), the gutter, Search Everywhere and the other centred
  popups, dialogs (Settings, New Project, Run Configurations, plugins), the Commit, Git,
  Problems, Find, Run/Build, Debug, Logcat tool window bodies, the welcome screen and the
  bundled plugins.
- The editor parity table (`docs/parity/intellij-editor.md`) is out of date in both
  directions: rows marked Missing that the code implements (caret up/down with column
  memory, word moves, Home/End/Page keys, double/triple click, drag selection, the error
  stripe, automatic completion on trigger characters, bracket auto-pairing), and rows that
  are still genuinely missing (below).

## Next PRs

### 1. Editor tabs on the kit `TabBar`

Kit: `TabBar` per-tab responses (so callers attach `ContextMenu`, `DragSource`,
`DropTarget` and double-click), an italic preview tab and a pinned tab. Overflow scrolling,
the "Show hidden tabs" chevron and `TabBar::more_menu` arrived in kit v0.3.0.

Acceptance: `editor.rs::tabs` and `tab_drag` are gone; click, middle-click, close button,
drag to reorder and to another split, double-click to maximise, the tab context menu, pinned
and preview tabs work by mouse under Xvfb exactly as before; plugin tabs share the bar;
`editor.rs` rounded-fill count drops in the design-lint baseline; audit test of a split
with three tabs.

### 2. Tool windows on the kit `Workbench`

Kit: `Workbench` with several tool windows per region (one visible per anchor, like the
IDE's model) and `frame_background(false)` so the IDE's one-surface frame and glow stay,
stripe badges (`StripeButton::badge(Tone)` for the Problems error dot and the
running build), and a header-only `Panel` variant whose hover-actions use the whole island
(today `widgets::tool_window_header` draws a `Panel` with an empty body, so header actions
are always visible instead of appearing on hover).

Acceptance: `toolwindows::left/bottom`, the right-anchored windows and the plugin tool
window render through `Workbench`; gaps are the splitters (no lines), resize persists in
the workspace, Hide and Maximize work from the header, actions appear on hover and fade
after 200 ms; the Problems stripe button shows the error badge again; Escape returns focus
to the editor, Shift+Escape hides (interaction spec 7.2).

### 3. Editor popups on kit surfaces: completion, parameter info, quick documentation

Kit: the completion list (`components::completion`) gains icons, type text on the right,
a documentation side panel and a "No suggestions" hint; `Surface::Tooltip`/`Popover` for
parameter info and quick documentation.

Acceptance: Ctrl+Space with nothing to offer shows "No suggestions" at the caret (the IDE
only writes it into the status bar); completion opens on typing after an identifier
character, filters and ranks as you type, Enter inserts and Tab replaces, the doc panel
follows the selection; parameter info highlights the active parameter; all three close on
Escape and on focus loss; `completion_ui.rs`, `doc_ui.rs`, `intentions_ui.rs` leave the
design-lint baseline.

### 4. Centred popups: Search Everywhere, Find in Files, Recent Files, File Structure, Switcher

Kit: a `SearchPopup` shell (title row with tabs/filters, a `TextField` with an id, a
`VirtualList` of `MenuItem`-like rows, a preview pane, a footer) shared by all five.

Acceptance: `palette.rs`, `find_popup.rs`, `switcher.rs`, `navigation_ui.rs` and the File
Structure popup in `popups.rs` use it; Find in Files shows the scope row (In Project,
Module, Directory, Scope), file mask, Cc/W/.* toggles and "Open in Find Window" like the
reference; speed search and keyboard rules from interaction spec 3.3.

### 5. Settings dialog on kit `Dialog`, `Form` and `Tree`

Observed defect: the Settings dialog's left column is an empty bordered box with the search
icon centred in it, and the settings tree is pushed below the dialog's bottom edge
("Appearance & Behavior" overlaps the window's lower area). Acceptance: search field above
the tree, tree scrolls inside the dialog at 800×500, breadcrumbs above the page, OK /
Cancel / Apply right-aligned, pages built with `Form` rows and kit choices; matches the
reference's layout at 1440×900 and 800×500.

### 6. Editor gutter and right margin like the reference

Acceptance: line numbers right-aligned with the reference's spacing, fold chevrons that
appear on hover of the gutter, run and breakpoint icons in their own column, VCS change
markers; the inspection widget (error/warning counts at the top right, "OFF" in safe mode)
and the error stripe as kit parts; no horizontal scrollbar at rest (observed: a horizontal
scrollbar thumb is always visible under the editor); stray "s" glyphs after declaration
lines (observed after `class`, `fun` and `object` lines in Kotlin files without a language
server) removed or replaced by real code-vision text.

### 7. Missing editor actions (keyboard-first)

Each with its keymap action id in `ACTION_COMMANDS`, a `jetthing-text` unit test, a
registry test and a kittest that presses the shortcut:

- Comment with Block Comment (Ctrl+Shift+/) — IJ-ED-011.
- Move Caret to Matching Brace (Ctrl+Shift+M) and Code Block Start/End — IJ-ED-035/036.
- Start New Line (Shift+Enter) — IJ-ED-004.
- Move Statement Up/Down (Ctrl+Shift+Up/Down) — IJ-ED-008.
- Complete Current Statement (Ctrl+Shift+Enter) — IJ-ED-013.
- Auto-Indent Lines (Ctrl+Alt+I) — IJ-ED-006.
- Cyclic word completion (Alt+/) — IJ-ED-090.
- Increase/decrease editor font size and Ctrl+wheel zoom — IJ-ED-014.

### 8. Highlight usages and the structure-aware navigation path

Acceptance: the identifier at the caret highlights its usages in the file (LSP
`documentHighlight`, Tree-sitter fallback for locals), with marks on the error stripe;
Highlight Usages in File (Ctrl+Shift+F7) and F3 between them — IJ-ED-072, IJ-NAV-032.
The status bar navigation path becomes navigable: clicking a directory crumb opens a popup
of its children, a declaration crumb a popup of sibling declarations (the reference's
behaviour), and Alt+Home focuses it.

### 9. Sticky lines

Acceptance: enclosing class/function headers stay pinned at the top of the editor while
scrolling (Tree-sitter scopes, at most 5 lines, click jumps to the declaration, setting in
Editor › General › Appearance) — IJ-ED-019.

### 10. Soft wraps, whitespace, indent guides and right margin

IJ-ED-015, IJ-ED-016: soft wrap per file and by default for Markdown, visible whitespace
toggle, indent guides and the right margin at 120 columns in the editor's colours.

### 11. Remaining tool window bodies on the kit

In the order of `docs/ui-migration.md` (Problems, Commit, Find, Run/Build, Git, Debug,
Terminal tabs, Logcat): `ListRow`/`Tree`/`Table` bodies, `Panel::tabs` for content tabs,
toolbars of `IconButton`s, `EmptyState` texts. Each brings its files to zero in the
design-lint baseline.

### 12. Dialogs and welcome screen

New Project, Run Configurations, Plugins, Clone, Push, first run, About on kit `Dialog`;
the welcome screen on `EmptyState`, `ListRow` and `TextField::search`.

### 13. Local History

IJ-ED-028: automatic revisions on save and external change, Show History with a diff,
revert.

### 14. Refactoring and navigation completeness

Rename in place (inline rename with live preview instead of a dialog for locals), Quick
Definition (Ctrl+Shift+I), Type Info (Ctrl+Shift+P), Error description (Ctrl+F1), Select
In (Alt+F1), Last Edit Location — IJ-ED-102/103/106, IJ-NAV-013/014.

### Small kit follow-ups the chrome needs

- `IconButton` tooltip with a shortcut (today the label carries `Run (Shift+F10)`, and a
  `Tooltip` behaviour on an `IconButton` shows a second tooltip).
- An accessible name for icon-only `StatusBarItem`s (AI and healthy-LSP widgets are small
  `IconButton`s meanwhile).
- The ☰ main menu as kit `MenuItem` submenus instead of egui's `MenuBar`; the VCS widget's
  branch menu (`branches_popup.rs`) as kit rows with the search field on top.
- Run widget like the reference: Run, Debug, ⋮ more actions (Build, Stop, profile),
  instead of Build left of the configuration.

## UX defects found in this round

Found with real mouse and keyboard input under Xvfb on the Kotlin sample (no command
dispatch); each becomes part of the slice named above or its own fix:

| Defect | Where | Slice |
|---|---|---|
| Settings dialog: empty bordered search column, tree pushed out of the dialog | Settings (Ctrl+Alt+S) | 5 |
| Ctrl+Space with no suggestions only writes "No suggestions" into the status bar | Editor | 3 |
| Stray "s" glyphs after declaration lines when no language server runs | Editor (code vision) | 6 |
| Horizontal scrollbar thumb visible at rest under the editor | Editor | 6 |
| Status bar breadcrumbs are not navigable (clicks do nothing) | Status bar | 8 |
| Tool window header actions always visible (no hover fade) | Tool windows | 2 |
| Problems stripe button lost its error dot with the kit stripe (no badge in the kit yet) | Left stripe | 2 |
| Problems content tabs are underlined text tabs, the reference uses pills | Problems tool window | 11 |
| Double-clicking a file in the tree right after expanding folders by double click sometimes leaves it a preview tab: the next file opened replaces it ("Closed …/Main.kt"); a slow double click works | Project tree | own fix |
| The "Java project" missing-server banner shows first on a Kotlin project, then a "Kotlin project" banner after dismissing | Editor banner | own fix |
| Find in Files has no file-mask checkbox/combo, filter or pin in its title row | Find in Files popup | 4 |

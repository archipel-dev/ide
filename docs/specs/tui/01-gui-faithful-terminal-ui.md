# TUI-GUI: the terminal UI is the GUI, drawn with cells

Status: specification (binding, product owner decision). Requirement prefix `TUI-GUI`. Priorities
`MUST` / `SHOULD` / `LATER`. Companion: `02-shared-component-layer.md` (prefix `TUI-CMP`: how both
frontends get the same UI from one code base). Plan: `docs/tui-parity.md` section 6, Wave 0.

Sources read for this spec (nothing built, nothing run): `docs/tui.md`, `docs/tui-parity.md`,
`docs/specs/ui/00..07` and `gaps.md` (the measured IntelliJ look), and the GUI code that builds the
chrome: `crates/archipel-ide-ui/src/{chrome,main_menu,toolwindows,widgets,editor,context_menus,popups,
palette,keybindings,actions,icons}.rs`, `crates/archipel-ide-core/src/toolwindow.rs`, and the terminal
kit `archipel-kit-tui` at the commit that `Cargo.lock` pins (`577ce54`). No JetBrains source was read;
IntelliJ values come from the `docs/specs/ui` captures only.

## 0. Decision

The terminal frontend (`archipel-ide --tui`) must **look and behave exactly like the GUI**
(`crates/archipel-ide-ui`, the IntelliJ-like egui frontend): same chrome, same items in the same order,
same popups, same shortcuts, same theme; only the drawing differs (terminal cells instead of pixels).
The earlier TUI design ("like NvChad": no menu, no toolbar, no stripes, hidden `Ctrl+K` chords, an
INSERT/NORMAL mode indicator) is **replaced**. The product owner could not find how to open Go to File;
that is the failure this spec removes: every function is reachable through visible UI first, with the
mouse, the same way as in the GUI. Keyboard shortcuts are the GUI's keymap; where a terminal cannot send
one, the visible path remains and a documented alternative exists (section L).

Terminology: a **cell** is one terminal character cell. Rows and columns are counted from 0 at the top
left. "GUI" means `archipel-ide-ui`. "Model" means the shared view model of `archipel-ide-view`
(`02-shared-component-layer.md`).

## A. Principles

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-001 | MUST | The TUI shows every chrome element the GUI shows, for the same `IdeState`: main toolbar, main menu, tool window stripes, tool window headers, editor tabs, gutter, error stripe, breadcrumbs, status bar, popups, dialogs, notifications. An element that the GUI has and the TUI lacks is a defect, not a design choice. | The parity tests of section N pass; the element checklist of sections C to K has no open item. |
| TUI-GUI-002 | MUST | Items, their order, labels, enabled state, checked state, shortcuts and tooltips come from one place (the shared model in `archipel-ide-view`, built from the core action registry, the keymap and `IdeState`), and both frontends render it. The TUI contains no menu list, label, shortcut string or enabled rule of its own. | `crates/archipel-ide-tui` has no string literal that names a menu entry or action label (checked by a test over the crate source, allow-list for terminal-only texts of section L and O). |
| TUI-GUI-003 | MUST | Every function of the TUI is reachable without a keyboard chord: through a menu, toolbar button, stripe button, header action, status bar widget, context menu or popup that the GUI also has, by mouse click. Keyboard shortcuts are accelerators of those paths. | Test: for each action id in the key table (appendix B), the action is reachable by clicks only, found through the model (`Chrome::reach(action) -> path`). |
| TUI-GUI-004 | MUST | No modal editing: there is no INSERT, NORMAL or other mode indicator anywhere, and no key is interpreted differently by mode. Typing inserts text, as in the GUI editor. | The status bar has no mode slot; a test types `i`, `:` and `dd` and finds literal text in the document. |
| TUI-GUI-005 | MUST | The default shortcuts are exactly the GUI keymap (`$default` or the keymap chosen on the Settings Keymap page, user overrides included, same `archipel-ide-keymap` data). A terminal-only alternative is never a replacement for a GUI shortcut; the GUI shortcut wins when both exist, and a collision between a terminal alternative and any GUI shortcut is a test failure (TUI-GUI-170). | Collision test over the whole keymap. |
| TUI-GUI-006 | MUST | The TUI uses the same theme as the GUI (Settings theme, default Islands Dark), mapped to 24-bit color (section I), not the NvChad palette. | `Look::gui(theme)` replaces `NvChadPalette` in `archipel-ide-tui`; a test compares sampled cell colors with `Theme::colors`. |
| TUI-GUI-007 | MUST | The same IDE core and commands serve both (`project.open`, `ActionRegistry::perform`, `editor.*`), as in `docs/tui.md` "How it is built"; this does not change. What changes is that presentation decisions move to the model. | `docs/tui.md` section "How it is built" stays valid. |
| TUI-GUI-008 | SHOULD | When the GUI changes a chrome element (an item, a label, a shortcut), the TUI changes in the same commit without TUI code changes, because both read the model. | A test that edits a menu composition fixture changes both outlines. |
| TUI-GUI-009 | MUST | Terminal-only elements are limited to: the one-time keyboard notice (L), the "terminal too small" message (B), the ASCII icon fallback (J), and OSC 2 window title and OSC 52 clipboard. Nothing else is added to the chrome that the GUI lacks. | Review against this list. |

## B. Grid and window layout

The screen is a grid of cells. All sizes below are in cells; the GUI's pixel values map as 40 px bars to
1 row, 40 px stripes to 3 columns, 24 px list rows to 1 row, so one row is about 20 to 24 px and one
column about 8 px (cell aspect 1:2).

```
col 0..2   stripe  | col 3..N  left tool window | splitter 1 col | editor | splitter | right tool window | stripe 3 cols
row 0      main toolbar (full width)
row 1      left: tool window header | editor: tab strip | right: tool window header
rows 2..   bodies
           bottom tool window (under the editor column): header row, then body
row H-1    status bar (full width)
```

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-010 | MUST | Fixed rows: row 0 is the main toolbar (UI-CHR-001, 40 px), row H-1 is the status bar (UI-SB-001, 30 px). No separator rows: the 1 px borders of the GUI (`#1E1F22`) are expressed by the surface color change between rows. | `H` = terminal height; cell (x,0) background is `frame`, cell (x,H-1) is `frame`. |
| TUI-GUI-011 | MUST | Left and right stripes are 3 columns wide, full body height (rows 1..H-2), background `frame` (UI-TW-001). The left tool window starts at column 3, the right one ends at column W-4. A hidden anchor leaves no gap. | Cells (0..2, 1..H-2) are the stripe; with no left window the editor starts at column 3. |
| TUI-GUI-012 | MUST | A visible left or right tool window has a header row at row 1 and its body below down to row H-2. A bottom tool window spans the editor column only (not under the side windows, as in the GUI where side panels are laid out first), its header is one row, its body fills the rest down to row H-2. | Layout test at 160x45 gives the rectangles of the mockup in appendix A. |
| TUI-GUI-013 | MUST | The editor column has one tab strip row (row 1), then the editor (gutter, text, error stripe at its last column). The editor needs at least 20 columns and 5 rows; side windows take at most 40 percent of the width each, the bottom window at most 60 percent of the body height, the active drag clamps to these. | Layout test with hostile ratios. |
| TUI-GUI-014 | MUST | Splitters are draggable with the mouse: the one-column boundary between a side window and the editor (vertical splitter), and the header row of a bottom window (drag up and down). Double click resets to the default. Default widths: left 28.8 percent of the columns (UI-TW-006), at least 24, right 25 percent, bottom 11 rows at 45 rows (about 25 percent). `Ctrl+Alt+Shift+arrow` (the GUI's resize keys) resize by two cells. | Mouse test: press on the splitter cell, drag three cells, release: the ratio persists. |
| TUI-GUI-015 | MUST | Sizes are stored in cells under `tui.*` keys, visibility, anchor, order and active windows in the shared core layout, so that resizing in one frontend never changes the other (TUI-CMP-007). | Test: drag in the TUI, `gui.*` sizes unchanged. |
| TUI-GUI-016 | MUST | Minimum terminal 100x30 for the full layout. Below 100 columns: the right tool window and then the left one collapse to their stripe, status bar widgets drop by priority (H), toolbar widget labels shorten (C). Below 60x16 the screen shows only `Terminal too small: need 60x16, have WxH` and takes no input except resize and `Ctrl+Q` (quit). | Render tests at 100x30, 80x24, 60x16, 59x15. |
| TUI-GUI-017 | MUST | The IDE hides the terminal cursor except in a text field or the editor, uses the alternate screen, mouse capture (press, release, drag, move, wheel), bracketed paste and focus events (kit `runtime::setup`). Holding Shift (most terminals) gives the terminal's own text selection; the first-run notice says so. | Setup test with a fake backend. |
| TUI-GUI-018 | SHOULD | The window title (OSC 2) is the GUI's `window_title(project, file)` (`chrome::window_title`, to be moved to the model), updated on change, restored on exit. | Test reads the emitted OSC sequence. |

## C. Row 0: the main toolbar

Content, left to right, exactly as `chrome::main_toolbar` composes it (UI-CHR-005), one row high, a
1-cell margin at both ends, widgets separated by one blank cell:

```
 ≡  [DM] demo ▾  ⎇ main ▾  [Code|Agents]             ...free space...       Main ▾  ▶  ⚒  ◉  ⌕  ⚙
 1   2            3           4                                              5       6  7  8  9  10
```

1 hamburger (main menu); 2 project widget (colored initials badge, project name, chevron); 3 VCS widget
(branch icon, branch name, chevron; absent without a Git repository, as `vcs_widget`); 4 Code | Agents
mode switch (`agents_ui::mode_switch`); 5 run configuration widget (label, chevron; label from
`state.run_widget_label()`); 6 Run; 7 Build; 8 Debug (becomes Stop Debugging while debugging; a red Stop
joins while a task runs); 9 Search Everywhere; 10 Settings gear. Android projects add the device
selector left of 5 (`device_manager::device_selector`). Plugin widgets keep their place (`plugin_ui`).
The right cluster is right aligned. The settings gear is new in the GUI too (UI-CHR gaps: "gear
missing"): the model adds `Settings` (`ShowSettings`, Ctrl+Alt+S) to both frontends in Wave 0.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-020 | MUST | Row 0 contains the widgets 1 to 10 above in that order, composed by the model (`main_toolbar(&ViewCx)`), identical for both frontends: same widgets present for the same state, same order, same enabled state (Run, Build, Debug enabled via `action_enabled` as the GUI computes), same tone (Run green `RUN_GREEN`, Stop red `STOP_RED`, disabled `TEXT_DISABLED`). | Parity test fixtures of TUI-CMP-041. |
| TUI-GUI-021 | MUST | Icon buttons are 3 cells wide (glyph in the middle cell), one blank cell between buttons (40 px pitch, UI-CHR-003). Widgets with text are `1 + text + 1` cells with the chevron after the text. Hover fills the whole widget with the `hover` token, press with `pressed`, a disabled button uses `text_disabled` and shows its reason as tooltip (the GUI's `on_disabled_hover_text`). | Cell tests on fills for hover, press, disabled. |
| TUI-GUI-022 | MUST | Every toolbar widget is clickable with the left mouse button and opens exactly what the GUI opens: 1 expands the menu bar (D); 2 opens the project popup (root path, Open Project…, New Project…, Refresh Plugins, active plugin count, as `project_widget`); 3 opens the branches popup (`branches_popup::menu`, model M8); 4 switches mode; 5 opens the run configuration popup (rows `Current File`, each configuration, separator, `Edit Configurations…`; a hovered or selected row shows `▶`, `◉`, `⋮` buttons that run, debug and edit that configuration; `Run unavailable: <reason>` line); 6 to 8 run the same commands (`run.project` or `android.app.deploy`, `build.project`, `debug.start`, `run.cancel`, `debug.stop`); 9 opens Search Everywhere (`navigate.searchEverywhere`); 10 opens Settings. | Click tests for each widget; the opened popup's outline equals the GUI's (N). |
| TUI-GUI-023 | MUST | Tooltips: after 500 ms rest on a button or widget a tooltip float appears below it with the GUI's text and shortcut (`Run 'Main' (Shift+F10)`, `Search Everywhere (Double Shift)`, `Stop (Ctrl+F2)`, `Git Branch`, the project root path). Toolbars are not in the keyboard focus order (as in IntelliJ), so the tooltip appears on hover only; the status bar shows the action description while hovered (kit `ActionToolbar` behavior). | Tooltip tests with a fake clock. |
| TUI-GUI-024 | MUST | The toolbar is never hidden and never replaced by hints. The hamburger is always at column 1 so that it is the first thing a new user sees and clicks. | Test at all sizes ≥ 60 columns. |
| TUI-GUI-025 | MUST | Width budget: at 100 columns the project name truncates to 14 cells with `…`, the branch to 12, the run label to 16; below 100 the Code/Agents switch hides first, then the branch label shortens to the icon, then the project name. The right cluster never truncates or overlaps. | Render tests at 100, 90, 80 columns. |
| TUI-GUI-026 | SHOULD | The project badge uses the same avatar color as the GUI (`theme::avatar_color(project_name)`) and initials (`theme::project_initials`), drawn as a filled 4-cell badge `▐DM▌`-style block in 24-bit color; the toolbar glow (UI-CHR-002) is approximated by a 12-cell horizontal fade of the badge color blended 8 percent over `frame` behind the left cluster when "project colors in main toolbar" is on. | Cell test on badge color; setting off removes the fade. |
| TUI-GUI-027 | SHOULD | Empty toolbar space has no function in the terminal (no window drag). Double click on it does nothing. | - |
| TUI-GUI-028 | LATER | Compact toolbar and presentation mode (UI-CHR-009). | - |

## D. The main menu

The GUI hides the menu behind the hamburger; clicking it expands the menu titles inline in the toolbar
(`main_menu::show`, UI-CHR-008, UI-POP-025). The TUI does the same, with the same composition.

```
 ≡  File  Edit  View  Navigate  Code  Refactor  Build  Run  Tools  Git  Window  Help
 ╭────────────────────────────────────╮
 │ New                              ▸ │
 │ Open…                              │
 │ Recent Projects                  ▸ │
 │ Close Project                      │
 │ ────────────────────────────────── │
 │ Settings…               Ctrl+Alt+S │
 │ Project Structure…  Ctrl+Alt+Shift+S │   (shortcut column right aligned; widths grow with content)
 │ Save All                    Ctrl+S │
 │ Reload All from Disk    Ctrl+Alt+Y │
 │ Exit                               │
 ╰────────────────────────────────────╯
```

(The figure shows a subset of the File menu to explain the anatomy; the real content is the model's.)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-035 | MUST | The menu bar has the twelve titles in this order: File, Edit, View, Navigate, Code, Refactor, Build, Run, Tools, Git, Window, Help (the reference group ids `FileMenu`, `EditMenu`, `ViewMenu`, `GoToMenu`, `CodeMenu`, `RefactoringMenu`, `BuildMenu`, `RunMenu`, `ToolsMenu`, `VcsGroups`, `WindowMenu`, `HelpMenu`), from the same composition as `main_menu::main_menu_items()` (moved to the model, M20): reference structure of `Catalog::shared_menu("MainMenu")`, minus `HIDDEN_AT_RUNTIME`, File filtered to implemented actions with `New Project…` first in New, plus `EXTRA_CATEGORIES` commands appended after a separator, plus plugin menu items (`plugin_ui::menu_items`). | Outline equality with the GUI for every menu to the deepest submenu. |
| TUI-GUI-036 | MUST | Titles are padded by one cell each side; the mnemonic letter of each is underlined: File F, Edit E, View V, Navigate N, Code C, Refactor R, Build B, Run u, Tools T, Git G, Window W, Help H. The open title has the `selection_popup`-style raised fill (`#404144` family, token `hover`). | Mnemonic uniqueness test; cell attribute test. |
| TUI-GUI-037 | MUST | Opening: click the hamburger (expands the bar and opens nothing), click a title (opens its menu below the title, the menu's top edge at row 1), `F10` (expands the bar and focuses File), `Alt+<mnemonic>` where Alt reaches the app (kitty protocol, or xterm with `metaSendsEscape`), `Ctrl+\ M` otherwise (L). Esc closes the open menu, a second Esc collapses the bar back to the hamburger layout; a click outside does the same. While a menu is open, hovering another title switches to it (GUI `switch_on_hover`); Left and Right move between titles. | Interaction tests for each path. |
| TUI-GUI-038 | MUST | Menu rows (UI-POP-020..024) are one row; layout: 1 cell border, icon column 2 cells (glyph when the action has one, else blank), text, at least 3 cells gap, shortcut right aligned in `text_secondary`, then a 2-cell column holding `▸` for submenus; 1 cell border. The mnemonic letter of the row is underlined. Width is content-driven, maximum 60 percent of the screen. Separators are a full-width `─` line in `popup_border`. Disabled items are drawn in `text_disabled` and cannot be chosen; the reason ("not available") is a tooltip, as in the GUI. | Cell test of a row. |
| TUI-GUI-039 | MUST | Submenus open to the right of the parent row after 250 ms hover (the kit's `PopupChain` value, faster than the GUI's 500 ms because terminal hover is coarser; the model carries the delay as `tuning.submenu_delay`), immediately on `Right` or `Enter`, attached to the parent's edge (flip to the left when no room). `Left` and `Esc` close one level. | Test with a fake clock. |
| TUI-GUI-040 | MUST | Keyboard in an open menu: `Up`/`Down` skip separators and disabled rows and wrap, `Home`/`End`, `Enter` chooses, a mnemonic letter chooses its row, `Right`/`Left` as above. Choosing performs the action through `ActionRegistry::perform` (the same call as the GUI) and closes all menus. | Key tests. |
| TUI-GUI-041 | MUST | Shortcuts in the shortcut column are the keymap's text for the action (first keyboard shortcut, formatted as the GUI formats it: `Ctrl+Shift+N`, macOS symbols on macOS). On a legacy terminal an item whose first shortcut cannot be delivered shows the first deliverable shortcut of the same action if the keymap has one (`display_shortcut`), else the GUI shortcut followed by nothing: the shortcut is still the GUI's and the item still works by click. The parity test compares with kitty `Caps`. | Outline test with `Caps::kitty()` equals the GUI; with `Caps::legacy()` differs only in the cases of appendix B. |
| TUI-GUI-042 | MUST | Enabled state comes from `action_presentation(id, ActionPlace::MainMenu)` with the same `DataContext` the GUI builds (`with_data`), so the same items are greyed in the same state (for example no active editor: Save, Undo greyed). Checked items (toggle actions: Power Save Mode, Soft-Wrap, Column Selection Mode, tool window toggles in View > Tool Windows) show `✓` in the icon column. | Fixture matrix of TUI-CMP-041. |
| TUI-GUI-043 | MUST | Context menus use the same rule set as the GUI (`context_menus::Target`: project tree node, editor, editor tab) and open on right click (and `Shift+F10`/the context key where deliverable, GUI parity) with the same items, order, separators and enabled state, from `context_menu(target)` in the model (M19). Menus open at the pointer; they flip up or left near the screen edge. | Shared test over M19 `applies` table. |
| TUI-GUI-044 | SHOULD | Menu placement near the bottom keeps the whole menu visible by scrolling its rows with the wheel and `PgUp/PgDn`, with `▲ ▼` cells at the ends (long menus: Code, Git, Refactor). | Render test at 24 rows. |
| TUI-GUI-045 | MUST | Help contains `Find Action…` and a terminal-only entry `Terminal Keyboard Setup…` that reopens the notice of section L (an extra item in the model marked `terminal_only`, listed in TUI-GUI-009's allow-list). | Menu outline differs from the GUI by exactly this item, asserted by the parity test's allow-list. |

## E. Tool window stripes

Left stripe (columns 0..2) and right stripe (columns W-3..W-1), each button 3 cells wide and one row
high with one blank row between buttons (the 40 px pitch, UI-TW-003). The glyph is in the middle cell.
The top group starts at row 1 (5 px below the toolbar in the GUI); the bottom group is bottom aligned,
its last button on row H-2; a one-row separator (`─` in the middle cell, `TOOLBAR_SEPARATOR`) sits after
the top group.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-055 | MUST | The left stripe has the buttons `chrome::left_stripe` builds, in this order. Top group: Project, Commit, Structure, then Agents (when agents sessions exist or the window is visible), then for Android projects Resource Manager and Build Variants. Separator. Bottom group, listed from the bottom up: Git, Hierarchy (when a hierarchy is shown), Logcat (Android), Problems, TODO, Terminal, Build, Debug, Find. Composition conditions are those of the GUI and come from the model (`stripes(&ViewCx)`). | Outline equality with the GUI for fixtures: plain, Android, agents, hierarchy. |
| TUI-GUI-056 | MUST | The right stripe has `chrome::right_stripe_windows` then AI Chat: Gradle (Gradle project or visible), Bazel, Device Manager, Running Devices (Android), then AI Chat, top to bottom from row 1, same conditions as the GUI. Plugin tool windows join as `plugin_ui::right_side` does. | Same. |
| TUI-GUI-057 | MUST | Clicking a button toggles that tool window (`app.toggle_tool_window`, i.e. the core `toolwindow.toggle`): hidden windows show and take focus, a visible one hides; at most one window per anchor is visible. Middle click or Shift+click hides; right click opens the stripe menu (kit `StripeMenu`: View Mode, Move to, Hide). | Click tests; layout state asserted in the core. |
| TUI-GUI-058 | MUST | Active (visible) button: fill `accent` over the three cells with the glyph in `text_on_accent` (white); other buttons: glyph `icon` color (Gray11 token), hover fill `hover`, pressed `pressed`. A visible-but-unfocused window uses the neutral gray fill of UI-TW-004. | Cell tests. |
| TUI-GUI-059 | MUST | Badges: a 1-cell `●` in the top-right corner cell of the button, red `ERROR` when Problems has errors, `WARNING` when only warnings, green `RUN_GREEN` on Build while a task runs, `WARNING` on Agents when input is needed else green when working (exact rules of `stripe_button`). | Fixture tests. |
| TUI-GUI-060 | MUST | Tooltip after 500 ms: `<Title> (<shortcut>)` from `ToolWindow::title()` and the keymap's `ActivateXToolWindow` shortcut (`Project (Alt+1)`); windows with no shortcut show the title only. This is the terminal's label: stripes have no text. | Tooltip test per window. |
| TUI-GUI-061 | SHOULD | A setting `tui.stripe_labels` (off by default) widens the stripes to 12 cells and prints the title next to each glyph, for users who do not know the icons. The GUI has no equivalent; this is the only allowed terminal-side addition besides section L and is off by default. | Layout test with the flag. |
| TUI-GUI-062 | MUST | Stripe icons are the Nerd Font glyphs of the table in appendix C (the GUI's icons through the same `Glyph` ids), ASCII fallbacks per J. | Glyph table test: every `ToolWindow` has a glyph. |
| TUI-GUI-063 | MUST | Window shortcuts work as in the GUI: `Alt+1` Project, `Alt+0` Commit, `Alt+7` Structure, `Alt+3` Find, `Alt+4` Build, `Alt+6` Problems, `Alt+5` Debug, `Alt+9` Git, `Alt+F12` Terminal, `Shift+Esc` hide active, `Ctrl+Shift+F12` hide all, `F12` jump to last, `Esc` back to the editor, `Ctrl+Shift+'` maximize (kit `Workbench::perform`); delivery per section L. | Key tests with kitty caps. |

## F. Tool window headers, content tabs and splitters

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-070 | MUST | Every tool window has a one-row header (UI-TW-005): title in `text_strong` bold at column offset 1 from the pane edge, a `▾` chevron after it when the title is a view selector (Project), actions right aligned with one blank cell between them and a 1-cell right margin, the last action is Hide (`─`). Header actions come from the model per window and equal the GUI's: Project: New, Locate (select opened file), Expand All, Collapse All, Options (kebab, menu `project_options_menu`), Hide; Commit: Refresh, Hide; Resource Manager: Refresh, Hide; others as their GUI builders add (`tool_window_header` closures); the GUI gets the full Project action set of UI-TW-005 in Wave 0 where it lacks it. | Outline equality of headers for all 20 windows. |
| TUI-GUI-071 | MUST | Header actions are icon buttons with the toolbar's states and tooltips; the action list wraps into a `⋮` overflow menu when the pane is narrower than title + actions. | Narrow pane test. |
| TUI-GUI-072 | MUST | Bottom windows with content tabs (Build, Problems, Find, Terminal, Debug, Git) draw the tabs in the header row after the title: selected tab in `text_strong` with a 1-cell high underline (`▔`/underline attribute in `tab_selected_border`; UI-TW-010), count in `text_secondary` (`Problems  File 1  Project Errors 3`), `✕` on closable tabs. Tab click selects, middle click closes. | Cell tests. |
| TUI-GUI-073 | MUST | A bottom window's vertical toolbar (rerun, stop, scroll to end, soft wrap in Run; eye, bulb in Problems) is a 3-cell column at the left of the body (UI-TW-012) built from the same action list as the GUI. | Outline test. |
| TUI-GUI-074 | MUST | Tree rows (Project, Structure, Gradle): one row each, indent 2 cells per level, `▸`/`▾` chevron one cell before the icon, selection fill `selection` (focused) or `selection_inactive`, full row width minus 1 cell each side; root row bold with the path in `text_secondary`; file-status tints full width: excluded `FILE_YELLOW`, test sources `FILE_GREEN` (UI-TW-007..009); git status colors on names (`git_colors`). | Cell tests on a fixture tree. |
| TUI-GUI-075 | MUST | Double click on a header maximizes (kit); the Hide button hides; `Esc` returns focus to the editor. Focus is shown by the header title brightness and by the selection fill color (`selection` focused, `selection_inactive` otherwise). | Interaction tests. |
| TUI-GUI-076 | MUST | All 20 core windows have a TUI body (`ToolWindow::ALL`): the 6 existing plus Structure, Git, Debug, Hierarchy, TODO, Gradle, Bazel, AI Chat, Agents, Device Manager, Logcat, Build Variants, Resource Manager, Running Devices (the last four Android windows with a text rendering of their lists; pixel previews are `LATER`, see `docs/tui-parity.md` 9). Until a window's body exists its stripe button still shows and the window opens with an explanatory body ("Not available in the terminal yet: <what to do>"), never a status-line message only. | `WINDOWS` covers `ToolWindow::ALL`; test opens each. |

## G. Editor: tabs, gutter, error stripe, breadcrumbs

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-085 | MUST | The tab strip is row 1 of the editor column, background `island` (the editor's), no pill (UI-ED-002). Each tab: 1 cell padding, file icon (the GUI's `file_icon(path)`: language logo or symbol, one cell), 1 cell gap, name, 1 cell gap, then the state glyph, 1 cell padding: `✕` close (shown on hover and on the selected tab, in `text_secondary`), `●` modified (`MODIFIED` token; shown instead of `✕` when dirty and not hovered; hovering the dot shows `✕`), the Nerd Font pin glyph for pinned tabs (click unpins). Preview tabs are italic. A file with errors has a red curly underline under the name (SGR 4:3 with color, or plain underline when `Caps.curly` is false). | Cell tests per tab state, as the egui `tabs` function (editor.rs). |
| TUI-GUI-086 | MUST | The selected tab: text `text_strong`, fill `tab_selected`, a 1-cell underline in `tab_selected_border` (focused editor accent, unfocused `#5A5D63` token) via the underline attribute over the tab; other tabs `tab_text`, hover fill `hover`. | Cell tests. |
| TUI-GUI-087 | MUST | Tab interaction: click selects (on press), click on `✕` closes (150 ms guard of the kit), middle click or Shift+click closes, right click opens the tab menu (`Target::Tab`, same items as the GUI), drag reorders, the wheel scrolls an overflowing strip, `Alt+Left`/`Alt+Right` (the GUI's `PreviousTab`/`NextTab`) switch tabs, `Ctrl+F4` closes. Closing a dirty tab asks Save/Discard/Cancel with the GUI's dialog. | Interaction tests. |
| TUI-GUI-088 | MUST | The strip's right end shows `⋮` (UI-ED-005) opening the list of all open tabs when the strip overflows, with the same entries as the GUI's tab list; diff and plugin tabs appear after the file tabs as in the GUI (`diff_tab`, `plugin_ui::editor_tabs`). | Overflow test. |
| TUI-GUI-089 | SHOULD | Editor splits (`archipel-ide-split` groups) draw one tab strip per group and 1-cell dividers, drag to resize; until implemented the TUI shows the group's tabs of the focused group only. | Layout test (Wave 6). |
| TUI-GUI-090 | MUST | Gutter columns, left to right, as the GUI (`GutterLayout`, UI-ED-012): optional blame annotations; 1 cell breakpoint column (a `●` replaces the blank; clicking the number toggles a breakpoint, as in the GUI); line number, right aligned, minimum 3 cells, `text_placeholder` color and the caret line's number in `LINE_NUMBER_ON_CARET_ROW`; 2 cells run lane (run/test icon and lightbulb, `▶` in `RUN_GREEN`); 1 cell fold column (`▾` expanded region start, `▸` folded); 1 cell change bar (`▎` added `ADDED_LINES`, modified `MODIFIED_LINES`, `▔` deleted); then the text. Total gutter width 8 cells with up to 3 digit numbers, growing with the line count. | Cell test equals the mockup columns; Wave 1 delivers the lanes, this spec fixes the order. |
| TUI-GUI-091 | MUST | Gutter clicks are the GUI's: number click toggles a breakpoint, right click on the number opens the breakpoint form, run icon click opens the run popup (Run X, Debug X, Run with Coverage (disabled), Modify Run Configuration), change bar click opens the change popup, fold chevron toggles the fold; hover over a change bar shows the hunk preview tooltip. | Click tests (`docs/tui-parity.md` Wave 1). |
| TUI-GUI-092 | MUST | The caret line is highlighted across gutter and text in `caret_row` (UI-ED-011); selection, matched brace, occurrences under caret, search results, write usage and diagnostics use the scheme colors (UI-ED-030/031) through the editor color tokens; folded regions show the `{…}` placeholder box in `FOLDED_BG`. | Cell tests per color. |
| TUI-GUI-093 | MUST | The error stripe is the last column of the editor (UI-ED-020, UI-ED-022): `▪` ticks in error red, warning yellow, weak blue/teal at the row proportional to the line in the file (a tick per row bucket, error beats warning), search result ticks in `#42BD77`; its top cell shows the inspection widget (`✓` green when clean, `⚠` or `✖` with the worst severity); click on a tick moves the caret to the first problem in that bucket and reveals it; hover shows the message of that problem (kit `stripe_hover`). There is no scrollbar track; a 1-cell scroll thumb `▐` in `tab_scroll_thumb` appears in the same column only while scrolling or hovering (UI-ED-021). | Cell and click tests; stripe uses `EditorOutput.stripe_hover`. |
| TUI-GUI-094 | MUST | Breadcrumbs are in the status bar, left (UI-SB-002), exactly where the GUI draws them: project icon and name, then path components, then the code crumbs (class, method) with `›` separators in `TEXT_INFO`; a top-level declaration named like the file replaces the file crumb (`App.java` becomes `App`). They truncate from the left with `…` when space runs out, never the last crumb. Click on a crumb opens its popup (the GUI's `crumb` action: sibling list) or moves the caret to the declaration. | Parity test with `structure_view::breadcrumbs` data moved to the model (M13). |
| TUI-GUI-095 | MUST | Indent guides (`INDENT_GUIDE`), inlay hints, code vision and blame annotations follow the GUI when the kit editor can draw them; each missing item is listed as a kit gap (appendix D) and does not block the chrome. | - |
| TUI-GUI-096 | MUST | Long lines wrap in the TUI until horizontal scrolling lands (kit gap); soft-wrap setting remains the GUI's. | Documented limitation kept. |

## H. The status bar

Row H-1, background `frame`, text size 1 row (UI-SB-001). Left breadcrumbs (G, TUI-GUI-094), right
widgets in the GUI's visual order (`chrome::status_bar`, right-to-left insertion; shown here left to
right):

```
▣ demo › src › main › Main › main()          Indexing… ⟳   12:30   LF   UTF-8   4 spaces   Gradle ✓   Java LS ●   AI ✦
 breadcrumbs                                  message, spinner, update, column mode, read-only, caret, line separator,
                                              encoding, indent, external system, toolchain, index, language server,
                                              agents, AI chat, plugin widgets
```

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-105 | MUST | The right-hand widgets are, left to right: status message (elided to the free width, first line, 70 characters, `TEXT_INFO`; click opens the Build window when no bottom window is visible), busy spinner while a task or a search runs, `Update <version>` / `Restart to update` (when available), `Column` (column mode), `Read-only` (`WARNING`, file not writable), caret position (`line:column`, with selection size via `caret_position_label`), line separator (`LF`/`CRLF`/`CR`), `UTF-8`, indent (`4 spaces`, `Tab`...), external system status, toolchain status, indexing status, language server status (all configured servers, not just the first), agents status, AI chat status, plugin status widgets. All text `text_secondary` unless the GUI colors it. Widgets are separated by 3 cells (18 to 21 px, UI-SB-003). | Outline equality for fixtures; same order as the GUI. |
| TUI-GUI-106 | MUST | There is no mode slot, no file name slot, no Git counts slot, no problem counts slot and no project slot in the status bar (the old NvChad statusline is removed): that information is in the breadcrumbs, the toolbar (branch), the stripe badges and the Problems window as in the GUI. | Test asserts no `INSERT`, `NORMAL`, `Ln` text. |
| TUI-GUI-107 | MUST | Widgets are clickable and open what the GUI opens: caret → Go to Line:Column; line separator, encoding, indent → their chooser popups; Read-only → toggle read-only; language server → LSP popup (restart, dismiss; `lsp_status`); indexing → index popup; toolchain → setup action; update → update balloon; AI → AI Chat window; breadcrumbs as G. A hover tooltip gives the GUI's text. | Click tests. |
| TUI-GUI-108 | MUST | Progress and background tasks (indexing, run, search, toolchain setup) show as the spinner (braille frames at 10 Hz) plus text in the message area and, where the GUI shows a progress bar, a 12-cell `ProgressBar` before the message. Notifications are balloons (below). | Frame tests. |
| TUI-GUI-109 | MUST | When the width is short, the lowest-priority widgets drop first: plugin widgets, AI, agents, external, toolchain, then message elision, line separator, encoding; the caret position, language server and breadcrumbs' last crumb stay. Breadcrumbs shrink from the left. | Width tests 100, 80, 60. |
| TUI-GUI-110 | MUST | Notifications use the kit's `Balloons` in the top-right corner (below the toolbar) with the GUI's content and actions from `Notifications` (run finished, update, language server notices, crash notice, disk conflict, banners as `InlineBanner` above the editor: toolchain missing, Gradle changes). The core service is polled each frame (docs/tui-parity.md H06). | Notification fixture tests. |

## I. Colors

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-120 | MUST | A constructor `Look::gui(theme: &Theme)` in `archipel-kit-tui` maps `Theme::colors` tokens one to one to 24-bit `Color::Rgb`; `NvChadPalette` is no longer used by the IDE. A translucent token (alpha below 255, such as hover `#FFFFFF1A`) is blended over the surface it is drawn on (kit `Look::color_over`). The Settings theme id selects the theme (same setting as the GUI, default Islands Dark); changing it re-themes live. | Test: for every token below, a cell sample equals the blend of `Theme::colors` value. |
| TUI-GUI-121 | MUST | Token mapping (surface, token, cells): toolbar, stripes, status bar background = `frame`; tool windows, editor, gutter, tab strip = `island`; popups, menus, dialogs = `popup` with `popup_border` single-line box; text fields = `field`; text = `text`, titles and selected = `text_strong`, status widgets and row details = `text_secondary`, hints and placeholders = `text_placeholder`, disabled = `text_disabled`, text on accent = `text_on_accent`; button hover = `hover`, press = `pressed`; focused tree/list selection = `selection`, completion and popup-list selection = `selection_popup`, unfocused = `selection_inactive`; list hover = `row_hover`; accent (active stripe button, primary button, focused underline, spinner) = `accent`; selected tab = `tab_selected` with `tab_selected_border` underline, other tabs `tab_text`; separators = `separator`; matched text in lists = `search_match` with `search_match_text`; speed search box = `speed_search_background`/`speed_search_border`; keyboard focus ring = `focus_ring`; checkbox/radio = `control`, `control_border`, `control_selected`; badges = `counter`, `counter_text`; balloons = `notification`; tooltips = `tooltip`; mnemonic hints = `popup_mnemonic`. Editor tokens (`theme::editor::*`) map to the matching syntax and diagnostic colors. | A table-driven test (`tests/theme_tokens.rs`) renders one cell per row and asserts the RGB. |
| TUI-GUI-122 | MUST | Truecolor is detected at start (`COLORTERM` equal to `truecolor` or `24bit`, or a DA/XTGETTCAP answer). Without it the TUI quantizes to the nearest xterm-256 color (nearest in CIELAB, 16 to 255, avoiding the user palette's 0..15) and shows a one-time notice that colors will be approximate and how to enable truecolor. | Test with `COLORTERM` unset. |
| TUI-GUI-123 | MUST | No islands: the GUI's Islands look has 4 px gaps between panels; the terminal draws panels edge to edge and lets the surface colors (`frame` versus `island`) separate them, plus the 1-cell splitters. Light themes work through the same mapping. | Visual review of the snapshot. |
| TUI-GUI-124 | SHOULD | `NO_COLOR` is honored for the chrome (monochrome with reverse video for selection) only when the user sets `tui.no_color`; the default ignores it because the IDE is a full-screen app. | - |

## J. Glyphs

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-130 | MUST | Every icon of the GUI's `Icon` enum and the stripe, toolbar, header, menu and status icons maps to a kit `Glyph` and a Nerd Font codepoint (`archipel-kit-tui/src/glyphs.rs`), per appendix C. File-type icons use `Language`/`SymbolKind` logos in the language color, folders by `FolderKind` color. A GUI icon with no Nerd Font mapping is a defect. | Test enumerates `Icon` variants and asserts a codepoint. |
| TUI-GUI-131 | MUST | Nerd Font is required (as today); at start the TUI checks `tui.glyphs` (`nerd`, `ascii`). `ascii` replaces each glyph by its ASCII fallback of appendix C and keeps every layout width. A first-run hint says how to pick a Nerd Font and how to switch to ASCII. | Test renders both modes with equal geometry. |
| TUI-GUI-132 | MUST | Glyphs occupy exactly one cell. Glyphs that are East Asian ambiguous width or emoji (`●`, `✕`, `▶`, `📌`) are avoided where a codicon exists; where kept they sit inside a 2-cell lane (the editor lane) so a wide rendering does not shift text. | Width test with `unicode-width` ambiguous-wide mode. |

## K. Popups and dialogs

All popups and dialogs are bordered overlays: single-line box (`╭╮╰╯│─` rounded to match UI-POP-003's
radius), `popup` fill, `popup_border` line, no shadow, drawn above everything, with the GUI's anatomy.
Sizes are the GUI's percentages of the window (UI-POP-010): Search Everywhere 44 percent of the columns
(minimum 64, maximum 100) by 71 percent of the rows (minimum 16), centered horizontally, top at 19 percent
of the rows. A popup closes on `Esc`, on a click outside, and the topmost overlay takes keys first
(kit `Layers`).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-135 | MUST | Search Everywhere (see the mockup in appendix A): header band of 2 rows (tabs `All Classes Files Symbols Actions Text`, selected tab with the accent underline row; right side `[ ] Include non-project items` and three icon buttons preview, filter, open in window), search row (`⌕▾` glyph, query, right aligned hint `Type / to see commands` while empty), a divider, the list, a footer row (location of the selected item and the link `Open In Right Split`). List rows: 1 row, kind icon at +2, name at +4 with `search_match` on matched characters, location right aligned 2 cells from the border followed by a module/folder icon, action rows show the shortcut after the name in `text_secondary` and the group on the right, ON/OFF badges for toggles, group separator rows (`Classes`, `Files`, ...) in `text_secondary`. Ranking, tabs, scopes and results come from `quick_nav` through the model (M15, M13), identical to the GUI. Opened by the toolbar glyph, `Navigate > Search Everywhere`, double Shift, or the alternatives of section L. `Tab`/`Shift+Tab` and a click switch tabs; `Up/Down/PgUp/PgDn` move; `Enter` opens; `Ctrl+Enter` opens in the right split; `Alt+Enter` opens the item's action menu where the GUI has it. | Outline equality of rows for query fixtures; cell test of anatomy. |
| TUI-GUI-136 | MUST | Go to File, Go to Class, Go to Symbol, Text and Find Action are Search Everywhere with the matching tab preselected (`Ctrl+Shift+N`, `Ctrl+N`, `Ctrl+Alt+Shift+N`, `Ctrl+Alt+Shift+E`, `Ctrl+Shift+A`), same as the GUI's `palette.rs`; Go to Line:Column (`Ctrl+G`) is the small popup with one field. Menu: Navigate > File…, Class…, Symbol…, Line/Column…. | Key and menu tests open the same state. |
| TUI-GUI-137 | MUST | Find in Files (`Ctrl+Shift+F`, Edit > Find > Find in Files…): header band with title and result counts, query row with clear, `Cc`, `W`, `.*` toggles, scope tabs `In Project Module Directory Scope`, file mask, results list (UI font rows, `search_match`, test files tinted `FILE_GREEN`), preview pane in the lower half (file header, highlighted line, gutter), footer with `Open results in new tab` checkbox, hint `Ctrl+Enter` and the `Open in Find Window` button. Model M15. | Outline equality. |
| TUI-GUI-138 | MUST | Completion (typing or `Ctrl+Space`): popup below the caret line with the item name's first character at the caret column, 1-row items: star mark for best match, kind icon, name with matched characters in `COMPLETION_MATCH`, gray parameters, return type right aligned; selected row `selection_popup`; footer row with the tip text. Enter inserts, Tab replaces the identifier. Model M17. | Cell and flow tests with the fake LSP. |
| TUI-GUI-139 | MUST | `Alt+Enter` popup (intentions and quick fixes) with the GUI's grouping, submenu arrows, and preview; the lightbulb in the run lane opens the same popup; `Ctrl+Q` quick documentation, parameter info and hover docs as float panels with the GUI's markdown block model (M17). | Cell tests. |
| TUI-GUI-140 | MUST | Recent Files (`Ctrl+E`) and the Switcher (`Ctrl+Tab`), Recent Locations, File Structure (`Ctrl+F12`), Branches popup, run configuration popup, gutter run popup, change popup, breakpoint form are popups with the GUI's rows and footers, built from the same models (M1, M2, M8, M13, M21). | Outline equality. |
| TUI-GUI-141 | MUST | Dialogs are centered modal floats of the kit's `Dialog` with the GUI's title, fields, validation lines and buttons (primary right-most; default on `Enter`, `Esc` cancels, mnemonics on buttons, `Tab` cycles): Settings (page tree at left with search, page content with `PropertyGrid`, OK/Cancel/Apply), Run/Debug Configurations (left tree of configurations with add/remove/copy toolbar, form at right), Rename and the refactoring dialogs, New Project, Clone, Push, Commit, Local History, Welcome, About, confirmation and error messages. Models: M24 to M26 and `refactor_ui`. | Each dialog is a row in the dialog parity test as it is migrated (TUI-CMP-042). |
| TUI-GUI-142 | MUST | The command guard `unsupported()` of `tui/app.rs` is deleted at the end of the migration; until then a guarded command opens a dialog stating "not available in the terminal yet" and naming the GUI fallback, rather than a status line text. | Test asserts the dialog. |
| TUI-GUI-143 | SHOULD | Popup sizes below the minimum scale down to the screen with 1 cell margin; lists scroll with `▲ ▼` indicators at the ends (the 6 px scrollbar of the GUI). | Small-screen tests. |

## L. Keys

The TUI resolves key presses through the same keymap as the GUI. A terminal cannot send everything a
window can. This section fixes what happens when a combination cannot arrive. The principle: **a visible
path always exists** (TUI-GUI-003); the shortcut shown in menus is the GUI's; a chord alternative is an
extra.

Detection: at start the TUI asks the terminal for the kitty keyboard protocol
(`supports_keyboard_enhancement`, as `kit-tui runtime::setup`) and pushes the flags *disambiguate escape
codes*, *report event types*, *report alternate keys*, and, to see lone Shift presses for double Shift,
*report all keys as escape codes* (kit `reports_modifiers`). The result is a `Caps` value stored in the
IDE (`caps.kitty`) used by `display_shortcut`, `sendable` and the notice. Without the protocol the TUI is
in **legacy mode**.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-155 | MUST | The keyboard protocol is detected at start and the result is shown in `Help > Terminal Keyboard Setup…` and in the About dialog ("Keyboard: kitty protocol / legacy"). In kitty mode all GUI shortcuts are delivered (including Ctrl+Shift+letter, Ctrl+Alt+Shift+..., double Shift, Alt+digit, Super on terminals that report it). | Detection test with a fake terminal answer. |
| TUI-GUI-156 | MUST | In legacy mode every action whose default shortcut `sendable()` is false (appendix B) stays reachable through its menu entry, toolbar or stripe button, Search Everywhere's Actions tab and Find Action, and through its chord alternative `Ctrl+\` followed by the key of appendix B (one fixed leader for all alternatives; `Ctrl+\` (0x1C) is unbound in the `$default` keymap and survives every terminal and tmux; the GUI's `Ctrl+K` (Commit) and `Ctrl+W` (Extend Selection) are NOT leaders any more, which fixes the shadowing of the old design). After the leader a which-key overlay lists the alternatives (kit `draw_which_key`). | Reachability test over appendix B; collision test (TUI-GUI-170). |
| TUI-GUI-157 | MUST | The kit's leaders (`LEADER_GENERAL = "ctrl K"`, `LEADER_WINDOWS = "ctrl W"` in `input.rs`) become a configurable `Leaders` value set by the host; `archipel-ide-tui` sets both to the single leader `ctrl \`. The kit's `terminal_keymap` adds the chords under the host's leaders. | Kit change listed in appendix D; keymap unit test. |
| TUI-GUI-158 | MUST | Shortcut display: in menus, tooltips and the Search Everywhere action rows the shortcut text is `display_shortcut(keymap, action, caps)`: the first deliverable shortcut of the action, else the GUI default. In kitty mode this equals the GUI's text. | Outline test (TUI-GUI-041). |
| TUI-GUI-159 | MUST | One-time notice. On the first start (settings key `tui.notice.keyboard`, stored with the user settings) in legacy mode a sticky balloon appears: "This terminal cannot send every Archipel shortcut (Ctrl+Shift+letters, double Shift, Alt+digits). Everything is still in the menus and the toolbar. For the full keyboard enable the kitty keyboard protocol:" with the buttons `How…` (opens `Terminal Keyboard Setup…`) and `Don't show again`. It re-appears when the detected terminal changes (`TERM_PROGRAM`, `TERM`) and from the Help menu entry. The dialog lists, per terminal: **kitty** and **Ghostty**: on by default; **WezTerm**: `config.enable_kitty_keyboard = true` in `wezterm.lua`; **Alacritty** (0.13 and later), **foot**: on by default; **iTerm2**: Settings > Profiles > Keys > "Report keys using CSI u" (and Left Option key = Esc+); **Windows Terminal**, **GNOME Terminal and other VTE terminals**, **macOS Terminal.app**: do not implement the protocol at the time of writing, use the alternatives (Windows Terminal and VTE: try a newer release, or WezTerm, kitty, Alacritty); **xterm**: `XTerm*metaSendsEscape: true`, `XTerm*modifyOtherKeys: 2`; **tmux**: `set -g extended-keys on` and `set -g extended-keys-format csi-u` (tmux 3.5 and later) plus a protocol-capable outer terminal; **macOS terminals**: Option as Alt (`Esc+`) in the profile. The list is data in the model so a release can fix it without code; entries carry "verify" dates. | Dialog text test; notice shown once. |
| TUI-GUI-160 | MUST | Double Shift (Search Everywhere) needs kitty mode with all keys reported. The kit's `DoubleShift` detector (500 ms window) runs only then. In legacy mode the toolbar magnifier, Navigate > Search Everywhere and `Ctrl+\ E` open it, and a mouse double click on the magnifier is not required. | Test with synthetic modifier events. |
| TUI-GUI-161 | MUST | macOS: the GUI keymap on macOS uses Cmd shortcuts. A terminal never delivers Cmd (kitty-protocol terminals can report Super when the terminal does not consume it, e.g. kitty, WezTerm, Ghostty). On macOS the TUI uses the Cmd shortcuts once a key event has carried the Super modifier (`Caps.super_reported`, kitty mode only), and until then, and in legacy mode, selects the Ctrl-based `$default` keymap variant automatically (setting `tui.keymap_fallback`), with the notice saying so. | Test with fake caps. |
| TUI-GUI-162 | MUST | Keys that terminals deliver as other keys are never bound by the TUI to the wrong action: Ctrl+I is Tab, Ctrl+M and Ctrl+J are Enter, Ctrl+H is Backspace, Ctrl+[ is Esc (appendix B row "control aliases"). In legacy mode the GUI actions on these (Implement Methods Ctrl+I, Insert Live Template Ctrl+J, Type Hierarchy Ctrl+H) are chord or menu only; typing keeps Tab, Enter, Backspace behavior. | Key tests. |
| TUI-GUI-163 | MUST | The Terminal tool window passes keys to the shell except the actions of `keybindings::TERMINAL_ACTIONS` (Find Action, Settings, Hide all, window activation, Search Everywhere), as the GUI does; the leader `Ctrl+\` is not passed to the shell either, and `Ctrl+\ \` sends a literal Ctrl+\ . | Terminal flow test. |
| TUI-GUI-164 | MUST | Mouse modifiers: Ctrl+click (Go to Declaration), Ctrl+Alt+click, Alt+click, Shift+click work from the modifier bits of SGR mouse reports (all terminals); the hover underline for Ctrl needs kitty mode and degrades to "click navigates" otherwise. | Mouse tests. |
| TUI-GUI-165 | MUST | Chord alternatives never replace a GUI shortcut: the same action may be reached by the GUI shortcut and the chord; in kitty mode both work; the status bar message after a chord shows the GUI shortcut ("Go to File: Ctrl+Shift+N") as a teaching hint (3 s) so users migrate to the shortcut. | Test of the hint. |
| TUI-GUI-170 | MUST | Collision guard: a unit test loads the `$default` keymap (and each shipped keymap) and fails when any chord alternative's first stroke (`ctrl \`) or the whole chord is bound by the keymap, or when two alternatives collide. | `keys_collisions` test over `archipel-ide-keymap`. |
| TUI-GUI-171 | SHOULD | The which-key overlay after the leader groups the alternatives by menu name (Navigate, Window, Code...) and shows the GUI shortcut next to each. | Cell test. |
| TUI-GUI-172 | MUST | Anything the old design bound that the GUI keymap binds differently is removed: `Ctrl+P` as Find Action (GUI: Parameter Info), `Ctrl+K` leader (GUI: Commit), `Ctrl+W` leader (GUI: Extend Selection), `Ctrl+7` for comment is kept only as an additional alias where the keymap has none. | Collision test passes. |

## M. Mouse

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-180 | MUST | Mouse works everywhere the GUI's does: every button, widget, stripe, header action, tab, menu, row, splitter, tree chevron, fold marker, gutter, status widget and popup. Hover highlights as in the GUI (rows `row_hover`, buttons `hover`); the wheel scrolls three rows; click selects, double click activates (opens a file, expands a node), right click opens the context menu, middle click closes tabs and hides windows. | Mouse matrix test per component. |
| TUI-GUI-181 | MUST | Drag and drop of tabs and project tree entries (move target rules `move_entry`) with a 2-cell dead zone. | Drag tests. |
| TUI-GUI-182 | SHOULD | Hover over a truncated text shows the full text as a tooltip; pointer shape changes are not available in terminals and are not specified. | - |

## N. Parity tests

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-190 | MUST | One shared chrome model in the core side (`archipel-ide-view::Chrome`, section 3 of file 02, built on the core's action groups, `ToolWindow`, run configurations, VCS state, status services) is the only source of: action groups and main menu composition, toolbar widgets, stripe buttons per anchor, tool window header actions, editor tab data, breadcrumbs, status bar widgets. Both frontends render it. | `Chrome::build` is the only chrome entry in both frontends (grep test). |
| TUI-GUI-191 | MUST | `tests/chrome_parity.rs` (in `archipel-ide-view`): for the fixtures of TUI-CMP-041 it renders the TUI (ratatui `TestBackend` through the existing `tests/common/mod.rs` `Tui` helper) and the GUI (egui headless, kit `testing`) and asserts: same items, same order, same enabled state, same checked state, same shortcut texts (kitty caps), same labels and tooltips, for the toolbar, all twelve menus (full depth), stripes, headers, tabs, breadcrumbs, status bar. | Test green; fails with an outline diff. |
| TUI-GUI-192 | MUST | Layout tests at 160x45, 100x30 and 80x24 check rectangles: toolbar row, stripes, panes, splitters, status bar, error stripe column, popup rectangles (SE at the percentages of K), with snapshot files `tests/snapshots/*.txt` (text dumps of the buffer with colors as a second grid), accepted with `UPDATE_SNAPSHOTS=1`. | Snapshot tests; appendix A mockups are generated from them. |
| TUI-GUI-193 | MUST | A reachability test enumerates the keymap's actions (`Chrome::reach`) and asserts each is reachable by clicks from the toolbar, menu bar, stripe, header, status bar, context menu or Search Everywhere, as TUI-GUI-003 requires; the allow-list of unreachable actions is empty. | Test green. |
| TUI-GUI-194 | MUST | A key test table checks every row of appendix B: in kitty caps the GUI shortcut performs the action; in legacy caps the row's alternative performs it and `sendable()` agrees with the table. | Table-driven test. |
| TUI-GUI-195 | MUST | A color test (`theme_tokens`) per TUI-GUI-121 and an `ascii_geometry` test per TUI-GUI-131. | Tests green. |
| TUI-GUI-196 | SHOULD | `cargo xtask spec-coverage` counts these IDs (set `TUI`); tests cite them in comments, as the other sets do. | `docs/specs/coverage.md` lists TUI. |

## O. What is removed from the old design

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-GUI-200 | MUST | Removed: the INSERT/NORMAL mode slot, the file/Git-count/problem-count/project status line segments, the tab-line sun button for theme cycling (the theme is the Settings theme), the dashboard-only entry with letter keys (the Welcome screen of the GUI replaces it: Open, New Project, Clone, recent projects as buttons), the `Ctrl+K`/`Ctrl+W` chord system as primary path, `Ctrl+P` as Find Action, and the statement "There are no stripes and no toolbar" of `docs/tui.md`. | grep and tests. |
| TUI-GUI-201 | MUST | `docs/tui.md` sections that describe the old behavior are marked "being replaced (see docs/specs/tui/01-gui-faithful-terminal-ui.md)" until the Wave 0 pieces land, then rewritten. | Doc review. |
| TUI-GUI-202 | MUST | The TUI starts with no project on the GUI's Welcome screen (`welcome.rs`: Open, New Project, Clone, recent projects) drawn in the same layout with the toolbar and the status bar visible (the hamburger and Search Everywhere work). | Start test without a project. |

## Appendix A. Mockups (160x45)

Glyph stand-ins are used so the figures render everywhere: `≡` hamburger, `▤` Project, `◉` Commit
(stripe) / Debug (toolbar), `≣` Structure, `⎇` Git, `▭` Terminal, `⚒` Build, `⚠` Problems, `✓` TODO,
`⌕` Find / Search Everywhere, `⚙` Settings, `◇` Gradle, `✦` AI, `▶` Run, `█x█` an active stripe button
(accent fill). Real output uses the Nerd Font glyphs of appendix C. Colors are described in section I.

Full window: Project window (left, Alt+1), editor with three tabs (the first selected, the third has
unsaved changes shown by `●`), gutter (breakpoint in the number column, run icons, folds, change bars),
error stripe in the last editor column, Build window at the bottom (Alt+4), right stripe with Gradle and
AI Chat, status bar with breadcrumbs and widgets.

```
 ≡  [DM] demo ▾   ⎇ main ▾   [Code|Agents]                                                                                          Main ▾   ▶   ⚒   ◉   ⌕   ⚙  
█▤█ Project ▾         + ⌖ ⇟ ⇞ ⋮ ─     │  ▪ Main.java ✕   ▪ Util.java ●   ▪ README.md ✕                                                                ⋮       ◇ 
    ▾ demo  ~/work/demo               │  1     package com.example;                                                                                         ✓   
 ◉    ▾ src                           │  2                                                                                                                    ✦ 
        ▾ main                        │  3     import java.util.List;                                                                                           
 ≣        ▾ java                      │  4                                                                                                                      
            ▾ com.example             │  5 ▶ ▾ public class Main {                                                                                              
 ─ ▌          ▪ Main.java             │  6 ▶ ▾     public static void main(String[] args) {                                                                     
              ▪ Util.java             │  7    ▎        List<String> names = List.of("a", "b");                                                              ▪   
          ▸ resources                 │  8   ▾▎        for (String name : names) {                                                                              
        ▸ test                        │  9 ●               System.out.println(name);                                                                            
      ▸ build                         │ 10             }                                                                                                        
      ▪ build.gradle.kts              │ 11    ▎        int unused = compute(3);                                                                             ▪   
      ▪ README.md                     │ 12         }                                                                                                            
    ▸ External Libraries              │ 13                                                                                                                      
                                      │ 14   ▾     static int compute(int n) {                                                                              ▪   
                                      │ 15             return n * 2;                                                                                            
                                      │ 16         }                                                                                                            
                                      │ 17     }                                                                                                                
                                      │                                                                                                                         
                                      │                                                                                                                         
                                      │                                                                                                                         
                                      │                                                                                                                         
                                      │                                                                                                                         
                                      │                                                                                                                         
                                      │                                                                                                                         
                                      │                                                                                                                         
                                      │                                                                                                                         
                                      │                                                                                                                         
                                      │                                                                                                                         
                                      │                                                                                                                         
 ⌕                                    │                                                                                                                         
                                      │──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────   
 ◉                                    │ Build ▸ Run: Main    demo [run] ✕                                                                 ⚙ ⋮ ─                 
                                      │ ⟳   > Task :compileJava UP-TO-DATE                                                                                      
█⚒█                                   │ ■   > Task :run                                                                                                         
                                      │ ⇩   a                                                                                                                   
 ▭                                    │     b                                                                                                                   
                                      │     BUILD SUCCESSFUL in 1s                                                                                              
 ✓                                    │                                                                                                                         
                                      │     Process finished with exit code 0                                                                                   
 ⚠                                    │                                                                                                                         
                                      │                                                                                                                         
 ⎇                                    │                                                                                                                         
 ▣ demo › src › main › Main › main()                                                  Indexing… ⟳   12:30   LF   UTF-8   4 spaces   Gradle ✓   Java LS ●   AI ✦ 
```

Search Everywhere open (toolbar magnifier, Navigate > Search Everywhere, double Shift or `Ctrl+\ E`).
The popup is 70x24 cells at column 45, row 8 (44 percent of 160, centered, top at 19 percent of 45).

```
 ≡  [DM] demo ▾   ⎇ main ▾   [Code|Agents]                                                                                          Main ▾   ▶   ⚒   ◉   ⌕   ⚙  
█▤█ Project ▾         + ⌖ ⇟ ⇞ ⋮ ─     │  ▪ Main.java ✕   ▪ Util.java ●   ▪ README.md ✕                                                                ⋮       ◇ 
    ▾ demo  ~/work/demo               │  1     package com.example;                                                                                         ✓   
 ◉    ▾ src                           │  2                                                                                                                    ✦ 
        ▾ main                        │  3     import java.util.List;                                                                                           
 ≣        ▾ java                      │  4                                                                                                                      
            ▾ com.example             │  5 ▶ ▾ public class Main {                                                                                              
 ─ ▌          ▪ Main.java             │  6 ▶ ▾     public static void main(String[] args) {                                                                     
              ▪ Util.java             │  7   ╭────────────────────────────────────────────────────────────────────╮                                         ▪   
          ▸ resources                 │  8   │ All   Classes   Files   Symbols   [ ] Include non-project  ▤ ⊞ ⧉   │                                             
        ▸ test                        │  9 ● │ ▔▔▔                                                                │                                             
      ▸ build                         │ 10   │ ⌕▾ main                                  Type / to see commands    │                                             
      ▪ build.gradle.kts              │ 11   │────────────────────────────────────────────────────────────────────│                                         ▪   
      ▪ README.md                     │ 12   │ Classes                                                            │                                             
    ▸ External Libraries              │ 13   │ ▌◆ Main                                             com.example  ▤ │                                             
                                      │ 14   │  ◆ MainActivity                                 com.example.app  ▤ │                                         ▪   
                                      │ 15   │ Files                                                              │                                             
                                      │ 16   │  ▪ Main.java                             src/main/java/com/example │                                             
                                      │ 17   │  ▪ MainTest.java                         src/test/java/com/example │                                             
                                      │      │  ▪ Main.kt                                     app/src/main/kotlin │                                             
                                      │      │ Symbols                                                            │                                             
                                      │      │  ƒ main(String[])                                      Main.java:6 │                                             
                                      │      │  ƒ mainLoop()                                         Util.java:21 │                                             
                                      │      │ Actions                                                            │                                             
                                      │      │  ▣ Main Menu            F10                               Navigate │                                             
                                      │      │  ▣ Maximize Tool Window  Ctrl+Shift+'                       Window │                                             
                                      │      │                                                                    │                                             
                                      │      │                                                                    │                                             
                                      │      │                                                                    │                                             
                                      │      │────────────────────────────────────────────────────────────────────│                                             
                                      │      │ src/main/java/com/example/Main.java         Open In Right Split ⇧⏎ │                                             
 ⌕                                    │      ╰────────────────────────────────────────────────────────────────────╯                                             
                                      │──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────   
 ◉                                    │ Build ▸ Run: Main    demo [run] ✕                                                                 ⚙ ⋮ ─                 
                                      │ ⟳   > Task :compileJava UP-TO-DATE                                                                                      
█⚒█                                   │ ■   > Task :run                                                                                                         
                                      │ ⇩   a                                                                                                                   
 ▭                                    │     b                                                                                                                   
                                      │     BUILD SUCCESSFUL in 1s                                                                                              
 ✓                                    │                                                                                                                         
                                      │     Process finished with exit code 0                                                                                   
 ⚠                                    │                                                                                                                         
                                      │                                                                                                                         
 ⎇                                    │                                                                                                                         
 ▣ demo › src › main › Main › main()                                                  Indexing… ⟳   12:30   LF   UTF-8   4 spaces   Gradle ✓   Java LS ●   AI ✦ 
```

## Appendix B. Keys a legacy terminal cannot deliver

"Legacy" = no kitty keyboard protocol (xterm-compatible escape sequences). Delivery classes: **no**
(arrives as a different key or not at all), **unreliable** (arrives in xterm family terminals but is
often grabbed by the desktop, tmux or the terminal emulator, or arrives differently across terminals),
**yes**. "Visible path" is what exists regardless of keys. "Chord" is `Ctrl+\` then the key (final
letters are assigned by the collision test TUI-GUI-170; this is the proposal).

| Class | GUI default (Linux/Windows `$default`) | Action ids | Legacy delivery | Visible path | Chord alternative |
|---|---|---|---|---|---|
| Double Shift | double Shift | `SearchEverywhere` | no (lone modifier keys are never sent) | toolbar ⌕, Navigate > Search Everywhere | `E` |
| Ctrl+Shift+letter | Ctrl+Shift+A | `GotoAction` | no (reads as Ctrl+A) | Help > Find Action…, SE tab Actions | `A` |
| | Ctrl+Shift+N | `GotoFile` | no | Navigate > File… | `N` |
| | Ctrl+Shift+F, Ctrl+Shift+R | `FindInPath`, `ReplaceInPath` | no | Edit > Find > Find in Files…, Replace in Files… | `F`, `R` |
| | Ctrl+Shift+E | `RecentLocations` | no | View > Recent Locations | `L` |
| | Ctrl+Shift+Z | `$Redo` | no | Edit > Redo | `Z` |
| | Ctrl+Shift+C, Ctrl+Shift+V | `CopyPaths`, `PasteMultiple` | no | Edit menu | `C`, `V` |
| | Ctrl+Shift+W, Ctrl+Shift+U, Ctrl+Shift+J, Ctrl+Shift+M, Ctrl+Shift+T, Ctrl+Shift+B, Ctrl+Shift+I, Ctrl+Shift+P, Ctrl+Shift+H, Ctrl+Shift+L | `EditorUnSelectWord`, `EditorToggleCase`, `EditorJoinLines`, `EditorMatchBrace`, `GotoTest`, `GotoTypeDeclaration`, `QuickImplementations`, `ExpressionTypeInfo`, `MethodHierarchy`, `FindPrevious` (alt) | no | Edit, Code, Navigate, View menus | editor chords of the kit (`editor::chords()`) under the leader |
| | Ctrl+Shift+F10, Ctrl+Shift+F8, Ctrl+Shift+F12, Ctrl+Shift+F4 | `RunClass`, `EditBreakpoint`, `HideAllWindows`, `CloseActiveTab` | no (F-key with Ctrl+Shift is `unreliable` in xterm, `no` elsewhere) | Run menu, gutter, Window menu, tab `✕` | `R`, `X`, `H`, `W` |
| Ctrl+Alt+Shift | Ctrl+Alt+Shift+N, +E, +J, +V, +C, +S, +H, +I, +F7, +Up/Down | `GotoSymbol`, `TextSearchAction`, `SelectAllOccurrences`, `EditorPasteSimple`, `CopyReference`, `ShowProjectStructureSettings`, `PopupHector`, `RunInspection`, `ShowSettingsAndFindUsages`, `VcsShowNextChangeMarker`/`Prev` | no | menus; Search Everywhere tabs; change popup | `S`, `T`, `J`, ... (final letters by test); change markers `.` and `,` |
| Ctrl+Alt+letter / arrows | Ctrl+Alt+S, +Y, +B, +H, +T, +Z, +L, +O | `ShowSettings`, `Synchronize`, `GotoImplementation`, `CallHierarchy`, `SurroundWith`, `Vcs.RollbackChangedLines`, ... | unreliable (ESC + control char; desktops grab Ctrl+Alt+letter; arrives in xterm, kitty-less WezTerm, foot) | menus, toolbar gear (Settings) | none needed when delivered; `,` family otherwise |
| | Ctrl+Alt+Left / Right | `Back`, `Forward` | unreliable (`CSI 1;7D` in xterm; grabbed by many window managers) | Navigate > Back / Forward | `[`, `]` |
| Alt+digit | Alt+1 to Alt+9, Alt+0 | `ActivateXToolWindow` | unreliable: ESC-prefixed in terminals with Alt as Meta; xterm default sends 8-bit characters; macOS Terminal and iTerm2 type symbols unless Option = Esc+ | stripe click, View > Tool Windows | `1`..`9`, `0` |
| Alt+letter | Alt+J, +Q, +F1 (Select In), Alt+Insert, Alt+Home | `SelectNextOccurrence`, `EditorContextInfo`, `SelectIn`, `Generate`, `ShowNavBar` | unreliable, same as Alt+digit | Edit, Code, Navigate menus | `J` etc. |
| Alt+Enter | Alt+Enter | `ShowIntentionActions` | unreliable (Alt+Enter toggles fullscreen in several terminals) | lightbulb in the run lane, Code menu | `Enter` |
| Alt+Shift / Alt+F | Alt+Shift+Up/Down (move line), Alt+Shift+F10, Alt+F7, Alt+F8, Alt+F9, Alt+Shift+J | `MoveLineUp/Down`, `ChooseRunConfiguration`, `FindUsages`, `EvaluateExpression`, `RunToCursor` | unreliable | menus, run widget | chords |
| Ctrl+digit / punctuation | Ctrl+1..9 (rare), Ctrl+/ (comment), Ctrl+= / Ctrl+- (expand, collapse), Ctrl+. , Ctrl+' , Ctrl+; | `CommentByLineComment`, `ExpandRegion`, `CollapseRegion`, `Maximize...` | no (Ctrl+/ may arrive as Ctrl+_ (`0x1F`) in some terminals, which the TUI accepts as Ctrl+/) | Code > Comment with Line Comment, Edit > Folding | chords (kit also adds Ctrl+7) |
| Control aliases | Ctrl+I, Ctrl+J, Ctrl+H, Ctrl+M, Ctrl+[ | `ImplementMethods`, `InsertLiveTemplate`, `TypeHierarchy` | no (Tab, Enter, Backspace, Enter, Esc) | Code menu, Navigate menu | `I`, `J`, `Y` |
| Ctrl+Tab, Ctrl+Enter, Ctrl+Shift+Space, Shift+Enter, Ctrl+Shift+Enter, Ctrl+Backspace | Ctrl+Tab, Ctrl+Enter, Ctrl+Shift+Space, Shift+Enter, Ctrl+Shift+Enter | `Switcher`, `ViewSource`, `SmartTypeCompletion`, `OpenInRightSplit`, `EditorCompleteStatement` | no | Recent Files (`Ctrl+E`, delivered), Navigate, Code menus | `T`, `Enter`+, ... |
| Ctrl+Space | Ctrl+Space | `CodeCompletion` | unreliable (NUL; some terminals and IMEs capture it) | typing opens completion; Code > Code Completion | `Space` |
| F-keys with modifiers | Shift+F10, Shift+F9, Ctrl+F2, Ctrl+F5, Ctrl+F8, Shift+F6, Alt+F12, Shift+F2, Shift+F3, Ctrl+F12 | `Run`, `Debug`, `Stop`, `Rerun`, `ToggleLineBreakpoint`, `RenameElement`, `ActivateTerminalToolWindow`, `GotoPreviousError`, `FindPrevious`, `FileStructurePopup` | unreliable (xterm sends `CSI 1;2P` style; the Linux console and some multiplexers do not) | Run menu, toolbar ▶ ◉ ■, Refactor, Navigate | chords |
| Mouse + key | Ctrl+click, Alt+click, Ctrl+Alt+click | Go to Declaration, multi-caret, Go to Implementation | yes (modifier bits in SGR mouse reports) | context menu entries | n/a |
| Cmd (macOS) | all macOS-keymap shortcuts | all | no (Super is not sent) except kitty mode with Super reported | menus | Ctrl keymap variant (TUI-GUI-161) |

## Appendix C. Glyph table (GUI icon to Nerd Font)

Kit `Glyph` and codepoints from `archipel-kit-tui/src/glyphs.rs` (codicons) at `577ce54`; ASCII fallback
for `tui.glyphs = ascii`. File-type, folder and symbol icons use the kit's language and symbol tables.

| GUI icon (`icons::Icon`, use) | Kit `Glyph` | Nerd Font | ASCII |
|---|---|---|---|
| `MainMenu` (hamburger) | `Menu` | U+EB94 | `=` |
| `ChevronDown` / `ChevronRight` / `ChevronUp` (widgets, trees) | same names | U+EAB4 / U+EAB6 / U+EAB7 | `v` `>` `^` |
| `Project` (stripe, breadcrumbs) | `Project` | U+EB46 | `P` |
| `Commit` (stripe) | `Commit` | U+EAFC | `C` |
| `Structure` (stripe, Hierarchy) | `Structure` | U+EA91 | `S` |
| `Branch` (VCS widget, Git stripe) | `Branch` | U+EC6F | `Y` |
| `Terminal` (stripe) | `Terminal` | U+EA85 | `T` |
| `Problems` (stripe, errors) / `Error` | `Problems` / `Error` | U+EA87 | `!` |
| `Build` (toolbar, stripe) | `Build` | U+EB6D | `B` |
| `Find` (stripe), `Search` (Search Everywhere) | `Search` | U+EA6D | `/` |
| `Run` | `Run` | U+EB2C | `>` |
| `Stop` | `Stop` | U+EAD7 | `#` |
| `Debug` | `Debug` | U+EB91 | `D` |
| `Rerun` | `Rerun` | U+EAD2 | `R` |
| `Pause`, `StepOver`, `StepInto`, `StepOut` | same | U+EAD1, U+EAD6, U+EAD4, U+EAD5 | `\|\|` `o` `i` `u` |
| `Close` (tab close) | `Close` | U+EA76 | `x` |
| `Minimize` (Hide) | `Minimize` | U+EABA | `-` |
| `Refresh` | `Refresh` | U+EB37 | `r` |
| `Rollback` | `Undo` | U+EAE2 | `<` |
| `Diff` | `Diff` | U+EAE1 | `d` |
| `Warning`, `WeakWarning` | `Warning` | U+EA6C | `w` |
| `Success` (TODO stripe, clean) | `Success` | U+EAB2 | `v` |
| `Device` (Device Manager) | `Device` | U+EADB | `m` |
| `Logcat` | `Log` | U+EB9D | `L` |
| `ScrollEnd`, `SoftWrap` | same | U+EAF3, U+EB80 | `e` `~` |
| `Pin` | `Pin` | U+EB2B | `*` |
| `Variants` (Build Variants, Resource Manager) | `Layers` | U+EBD2 | `V` |
| `Ai` (AI Chat stripe) | `Sparkle` | U+EC10 | `*` |
| `Plus`, `Add` | `Plus` | U+EA60 | `+` |
| `Copy`, `Send` | `Copy`, `Send` | U+EBCC, U+EC0F | `c` `s` |
| `Model` (Gradle, Bazel) | `Package` | U+EB29 | `G` |
| `Task` (settings gear) | `Settings` | U+EAF8 | `*` |
| `ExpandAll`, `CollapseAll` | same | U+EB95, U+EAC5 | `E` `K` |
| Agents (stripe) | `Agent` | U+EC10 | `A` |
| Kebab (more actions) | `MoreVertical` | U+EB10 | `:` |
| Folder, file, project root | `Folder`, `FileText`, `Project` | U+EA83, U+EC5E, U+EB46 | `/` `.` `P` |
| Breakpoint, execution | `Breakpoint` | U+EA71 | `o` |
| Lightbulb (intentions) | `Thinking` | U+EA61 | `?` |
| Bell (notifications) | `Bell` | U+EAA2 | `!` |
| Spinner | braille `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` (not a glyph) | - | `\|/-\` |

## Appendix D. Core extractions needed first, and kit-tui gaps

### D.1 From `crates/archipel-ide-ui` into the shared layer (needed by Wave 0)

Full table: `02-shared-component-layer.md` section 3. The ones Wave 0 needs, in order, by the move ids
of `docs/tui-parity.md` section 5:

| Order | Move | What | Source | Why Wave 0 |
|---|---|---|---|---|
| 1 | new | `archipel-ide-view` crate, `ViewCx`, `Chrome`, `Outline` | - | the shared model |
| 2 | M20 | main menu composition (`main_menu_items`, extras, `HIDDEN_AT_RUNTIME`, plugin titles) | `ui/main_menu.rs` | TUI-GUI-035 |
| 3 | M19 | action applicability, context facts, `UiAction`, surface requests, context menu builder | `ui/context_menus.rs`, `ui/actions.rs`, `ui/keybindings.rs`, `tui/actions.rs` | TUI-GUI-043, 190 |
| 4 | M21 | run widget rows and row actions; widget data of project, VCS, run, Build, Debug, Stop | `ui/chrome.rs` | TUI-GUI-020, 022 |
| 5 | new | stripe composition and badges (`left_stripe`, `right_stripe_windows`, `stripe_button` rules) | `ui/chrome.rs` | TUI-GUI-055..059 |
| 6 | new | tool window header models (actions per window, Project options menu) | `ui/toolwindows.rs`, `ui/widgets.rs` | TUI-GUI-070 |
| 7 | new | editor tab data and tab actions | `ui/editor.rs` `tabs` | TUI-GUI-085..088 |
| 8 | M22 | status bar model: items in order, caret label, read-only probe, line separator, encoding, indent, update, LSP/index/toolchain/external summaries, window title | `ui/chrome.rs`, `ui/editor_keys.rs`, `ui/lsp_status.rs`, `ui/index_status.rs`, `ui/toolchain_status.rs`, `ui/external_status.rs` | TUI-GUI-105 |
| 9 | M13 (part) | breadcrumbs source (`structure_view::breadcrumbs`) | `ui/structure_view.rs` | TUI-GUI-094 |
| 10 | M8 | branches popup composition | `ui/branches_popup.rs` | VCS widget |
| 11 | M23 (part) | notices and banners content | `ui/notifications.rs`, banners | TUI-GUI-110 |
| 12 | M27 | toolkit-neutral key type and terminal key encoding (one implementation) | `ui/terminal_ui.rs`, `tui/keys.rs` | key table |

Later phases (popups: M13 M15 M17 M18; tool windows: M5 M7 M9..M12 M14 M28..M30; dialogs: M24..M26)
are in `docs/tui-parity.md` section 5 and file 02 section 4.

### D.2 `archipel-kit-tui` gaps (pin `577ce54`)

| Gap | Today | Needed |
|---|---|---|
| Menu bar | no menu bar component | `MenuBar`/`MenuHeader` with mnemonics underline, F10 focus, hover switching, anchored `ActionPopupMenu` below the title |
| Toolbar buttons | `ActionToolbar` for icon buttons | `ToolbarDropdown` widget (badge, label, chevron), per-button tone and badge, 3-cell pitch, disabled tooltip with reason |
| Stripes | `Workbench::stripes(true)`: top-down, 1 row each | two groups (top, bottom aligned), separator, 2-row pitch, badge dot, tone, order, right stripe content, labels mode |
| Tool window header | title line without actions | `PanelHeader`: view selector chevron, action buttons, overflow, Hide, content tabs with counts and underline |
| Status bar | `StatusBarControl` with 9 fixed NvChad slots | ordered `StatusBarItem` list with priority, popup, tooltip, spinner and progress |
| Breadcrumbs | none | `Breadcrumbs` component |
| Look | `NvChadPalette` roles | `Look::gui(theme)` token mapping (TUI-GUI-120), 256-color quantizer |
| Tabs | `JBTabs`, `TabsControl` | editor strip props: preview italic, error curly underline, tooltip, overflow list, pinned glyph |
| Tree | `TreeNode` with no tone | `tone`, full-width tint, bold root with gray path, checkbox column |
| List rows | `RowData` | right text, trailing icon, shortcut chip, match ranges with `search_match` colors, group separator rows |
| Leaders | `LEADER_GENERAL = "ctrl K"`, `LEADER_WINDOWS = "ctrl W"` constants | host-configurable `Leaders` (TUI-GUI-157) |
| Caps | `Caps { kitty, os }` | also `super_reported`, `curly`, `truecolor`, `nerd_font` hint; double Shift enabled only with all-keys flag |
| Title and clipboard | clipboard (OSC 52) | OSC 2 title |
| Popover sizing | fixed or anchored | percent-of-screen sizing with min and max |
| Spinner, PropertyGrid, ScrollView, `Button::selectable` | missing | as TUI-CMP-011 |
| Editor widget | gutter, change markers, stripe hover exist | reserved lanes, breakpoint column glyph shapes, line tints, end-of-line text, horizontal scroll (parity doc P4) |
| Pin | git branch `main` at `577ce54` | every kit change is a commit in `archipel-dev/kit` and a pin bump here |

## P. Open questions and deviations found while building Wave 0 (phases 0.1 and 0.2)

The spec said `Ctrl+\` is bound in no keymap, `Ctrl+W` stays free of a leader role and a handful of
actions have chords. Building it showed these points; each is the closest faithful behavior chosen,
recorded here so the spec can be corrected.

| # | Spec statement | What was found | Decision |
|---|---|---|---|
| P1 | TUI-GUI-170: `Ctrl+\` is free in every shipped keymap | NetBeans 6.5, VSCode, Visual Studio 2022 and macOS System Shortcuts bind it (`archipel-ide-view` test `keys_collisions`) | `leader_for(keymap)` picks `Ctrl+\`, else `Ctrl+]`, else `Ctrl+^`; the notice names the leader in use |
| P2 | Appendix B lists chords for CopyPaths, QuickImplementations, ExpressionTypeInfo, MethodHierarchy, TextSearchAction, CopyReference, ShowProjectStructureSettings, ActivateDebugToolWindow, EditorContextInfo, SelectIn, ShowNavBar, ImplementMethods, ViewSource, SmartTypeCompletion, ReplaceInPath, PasteMultiple | the IDE has no such action yet | `CHORDS` lists implemented actions only (test `keys_table`); a chord is added with its action. Appendix B also gained chords the table missed for actions whose only GUI stroke a legacy terminal cannot send (Complete Statement, Split Line, Start New Line Before, Collapse Selection, Show Usages, Push) |
| P3 | TUI-GUI-016: 100x30 is the minimum; below 100 columns the right then the left window collapse | the shell clamps the panes and shows the "terminal too small" message below 60x16; no automatic collapse | open: automatic collapse by width (phase 0.4 with the tool window bodies) |
| P4 | TUI-GUI-015: sizes are stored in cells under `tui.*` keys | not persisted yet; the shell keeps them for the session | open: persist with the user settings |
| P5 | TUI-GUI-159: the notice is remembered in the settings key `tui.notice.keyboard` | a marker file `$XDG_CONFIG_HOME/archipel-ide/tui-keyboard-notice` is used | open: move it to the user settings once the settings page has a TUI section |
| P6 | TUI-GUI-131: `tui.glyphs` setting | the environment variable `ARCHIPEL_TUI_GLYPHS=ascii` selects ASCII | open: setting |
| P7 | TUI-GUI-122: truecolor from `COLORTERM` or a DA/XTGETTCAP answer | only `COLORTERM` (and the quantizer for its absence) | open: terminal queries |
| P8 | TUI-GUI-058: a visible but unfocused window has the neutral gray fill | the GUI itself fills every visible window's button with the accent (`chrome.rs`, unchanged) | the terminal follows the spec (neutral gray when unfocused); the GUI should follow the measured reference in a later change |
| P9 | TUI-GUI-109 and TUI-CMP-041: nothing drops on a wide screen | on a busy project the status bar at 160 columns cannot show the breadcrumbs, the message and every widget (the Gradle import message alone is 54 cells) | widgets drop by priority and the message elides (test `chrome_parity` allows it for the status bar only) |
| P10 | TUI-GUI-141: dialogs are the GUI's | the kit `Dialog` now draws the Islands popup (surface, border, bold title) but its buttons keep the older markers | phase 0.5 |
| P11 | TUI-GUI-202: the GUI's Welcome screen | a list of Open, New Project and the recent projects on the editor area | phase 0.5 with the Welcome model |
| P12 | TUI-GUI-171: which-key groups the alternatives by menu name | the kit's flat which-key list is shown | open |
| P13 | TUI-GUI-014 splitters of editor splits and the bottom window height in `tui.*` | the three tool window splitters drag; editor split dividers are not drawn (the core's split groups show as one strip) | Wave 1 |

The GUI side of 0.2.1 to 0.2.8 is done for the main menu, the context menus, the action rules, both
stripes and the Settings gear (`archipel-ide-ui` renders the view crate's models); the toolbar widgets
(project, VCS, run widget), tool window headers, editor tabs and status bar widgets still compose
their data in `archipel-ide-ui` and are the remaining GUI work of phase 0.2.


# TUI-CMP: one UI layer, two renderers

Status: specification (binding, product owner). Requirement prefix `TUI-CMP`. Companion of
`01-gui-faithful-terminal-ui.md` (prefix `TUI-GUI`, what the terminal shows). This file says how both
frontends get the same UI from the same code.

Product rule: **the UI is written once, the frontend only renders.** The IDE's UI code (toolbar, main
menu, stripes, tool window headers, tabs, status bar, popups and lists, dialogs, tool window content)
is written against view models and components that have the same names and the same props in
`archipel-kit` (egui) and `archipel-kit-tui` (ratatui). Moving from the window to the terminal then
changes the renderer, not the UI. Nothing in `crates/archipel-ide-ui` that decides *what* is shown
(items, order, labels, enabled and checked state, shortcuts, tooltips, which popup opens on a click)
may stay there once it has moved.

Evidence for the starting point (read from source, nothing built): `crates/archipel-ide-ui` is 43k
lines. It calls the kit's components in some places (`ActionButton` 13 call sites, `ActionMenuItem` 17,
`Button::*` 69, `ComboBox` 18, `TreeRow` 28, `ListPopup`, `JBTabs`, `InlineBanner`) and raw egui in
many more (`ui.label` 427, `painter()` 294, `allocate_exact_size` 70, `TextEdit` 56, `egui::Popup` 34,
`ui.button` 67, `ui.checkbox` 20, `SidePanel` 17, `TopBottomPanel` 13, `ScrollArea` 71). The chrome
(`chrome.rs`, `main_menu.rs`, `toolwindows.rs`) paints stripes, widgets and status items by hand. The
terminal frontend (`crates/archipel-ide-tui`, 5.8k lines) re-implements the same decisions with
`archipel-kit-tui` (see `docs/tui-parity.md` 2.4). The kit pin is `577ce54` (`Cargo.lock`).

## 1. Architecture

```
archipel-ide-core, -actions, -keymap, -settings   (state, commands, action registry; no UI types)
            |
archipel-kit-model  (new crate in the kit repo; no egui, no ratatui, serde)
            |            props + events of every component: ToolbarButton, MenuItem, StripeButton, ...
            |
archipel-ide-view   (new crate in this repo; depends on core, actions, keymap, settings,
            |         archipel-kit-model; no egui, no ratatui)
            |            builders: &ViewCx -> props.   dispatch: event -> core commands / actions
     +------+------+
archipel-ide-ui   archipel-ide-tui
(egui renderer)   (ratatui renderer)
  uses archipel-kit   uses archipel-kit-tui
  (components render props)   (components render the same props)
```

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-CMP-001 | MUST | A crate `archipel-kit-model` in the kit repo defines the component vocabulary as plain data: for each component a `Props` struct (what to show) and an `Event` enum (what the user did), `Clone + PartialEq + Debug + Serialize`, no egui, no ratatui, no time and no I/O. `archipel-kit` and `archipel-kit-tui` both re-export it, so a kit component is `Component::show(props) -> Vec<Event>` (egui) and `Component::draw(props, area, buf) -> Vec<Event>` (ratatui) with the same props type. The kit's existing neutral types stay and are re-used, not duplicated: `action_system::{ActionGroup, Presentation, Keymap, KeyStroke, DataContext}`, `wm::{ToolWindow, ContentManager, ToolWindowPaneState}`, `Theme`/`Colors`, `notification::Notification`, `popup::PopupStep`, `Glyph`. | `cargo tree -p archipel-kit-model` lists neither `egui` nor `ratatui`; a doc test builds each `Props` and round-trips it through serde. |
| TUI-CMP-002 | MUST | A crate `crates/archipel-ide-view` (this repo) holds the IDE's view-model layer: `ViewCx` (a read-only view of `IdeState` plus the action registry, keymap and settings), one builder function per UI surface returning props (`main_toolbar`, `main_menu`, `stripes`, `tool_window_header`, `editor_tabs`, `status_bar`, `breadcrumbs`, `popup_*`, `dialog_*`, `context_menu`), and `dispatch(Event) -> Vec<Effect>` that turns a component event into core commands and action performs. It has no `egui`, `ratatui` or `crossterm` dependency (checked by a `deny`-style test over `cargo metadata`). | `crates/archipel-ide-view/tests/no_ui_deps.rs` passes; `cargo tree -p archipel-ide-view` shows neither toolkit. |
| TUI-CMP-003 | MUST | A frontend contains only: (a) the event loop and platform glue (window or terminal, clipboard, file dialogs), (b) layout of the areas (where the toolbar, stripes, panes go) and (c) calls of kit components with props from `archipel-ide-view`. It may not contain a list of menu entries, a label, a shortcut string, an enabled rule, a tooltip text or a decision which popup opens. `archipel-ide-ui/tests/no_subsystem_reads.rs` is extended: the egui crate may not call `IdeState` getters for the migrated surfaces, only `archipel-ide-view`. | A test greps the migrated modules for `state.` reads and fails on any outside the allowlist; the allowlist shrinks with each phase. |
| TUI-CMP-004 | MUST | Props carry everything a renderer needs and nothing a renderer must decide: text (already ellipsised by the renderer only), `Glyph` or file-type icon key (never a codepoint, never an egui texture), `Tone` (semantic color role, never a color), enabled, checked or selected, badge, tooltip (title, shortcut, description), accessible name, the shortcut as a `KeyboardShortcut` list (the renderer formats it: Ctrl+Shift+A, or the symbols on macOS), a stable `Id` for hit testing and tests, and the `Event` each interaction (click, double click, context click, hover rest, key) produces. | Reviewing a `Props` type shows no `Color32`, `Color`, `Rect`, `Pos2`, `egui::Id` or pixel value. |
| TUI-CMP-005 | MUST | Component names and props are identical in both kits. The vocabulary is exactly the table of section 2 (`ToolbarButton`, `ToolbarDropdown`, `MenuBar`, `MenuHeader`, `MenuItem`, `MenuSeparator`, `ToolStripe`, `StripeButton`, `PanelHeader`, `TabBar`, `Tab`, `StatusBar`, `StatusBarItem`, `Breadcrumbs`, `ListRow`, `Popover`, `TextField`, `Checkbox`, `Button`, `Tree`, `TreeRow`, ...). A component that exists in one kit and not in the other is a gap listed in `docs/tui-parity.md` and blocks the migration of the surfaces that use it. A required kit-only prop is added to `archipel-kit-model`, never to one renderer. | A kit test iterates the vocabulary list and asserts both kits export the component with the same `Props` type (`TypeId`). |
| TUI-CMP-006 | MUST | The IDE's view builders are pure functions of (`IdeState` snapshot, keymap, settings, `Caps`): same input, equal props. A builder never reads the clock (timers such as the 500 ms tooltip or the 250 ms run-icon debounce live in the renderer or in the core service, as today). | A property test calls each builder twice on the same fixture and compares. |
| TUI-CMP-007 | SHOULD | Layout units are abstract: props use abstract semantics (rows of a list, items of a bar, a ratio for a splitter) and the renderer maps them (GUI 24 px list rows and 40 px bars, TUI 1 cell rows). Stored sizes of resizable areas are kept per renderer (`gui.*` and `tui.*` keys, point versus cell units) so that a drag in one frontend never changes the other; visibility, anchor, order and the active window are shared. | Dragging the bottom splitter in the TUI test changes `tui.layout.bottom_rows` only. |
| TUI-CMP-008 | MUST | Interaction state that is not data (hover, press, scroll offset, speed search text, drag) stays inside the renderer's component state, as in the kits today (`ToolbarState`, `TabsState`, `CollectionState`). State that changes what is shown to other code (selected tab, expanded tree nodes, selected list row, query text of a popup) is in the view-model state (`archipel-ide-view::state`, one struct per surface) so that a test can drive both renderers with the same state. | Both renderers start a popup from the same `PopupState` and show the same rows. |

## 2. Component parity table

Every kit component that `archipel-ide-ui` uses, or must use to stop painting by hand, with the
`archipel-kit-tui` counterpart at `577ce54`. "Exists" means a type with the behavior in the doc
comment; "Differs" lists props the egui component has and the terminal one lacks (or the reverse);
"Missing" means nothing usable exists. Status is checked against source, not run.

| Vocabulary name | egui (`archipel-kit`) | kit-tui (`577ce54`) | Status and prop differences | Used for |
|---|---|---|---|---|
| ToolbarButton | `action_system::ActionButton` (`large`, colored variant) | `toolbar::ActionButton` in `ActionToolbar` | Exists. Differs: tone override (green Run, red Stop), badge dot, label variant (icon plus text), 3-cell width, pressed/selected state names | main toolbar buttons, bottom window toolbars, header actions |
| ToolbarDropdown | `widgets::flat_widget` (hand-built in `chrome.rs`) | missing | Missing in both as a named component: icon or badge, label, chevron, hover fill, click opens a `Popover`; props `badge`, `icon`, `label`, `chevron`, `max_width`, `tooltip`, `popup` | project widget, VCS widget, run configuration widget, device selector |
| SegmentedControl | `ui::SegmentedButton` | `controls::SegmentedControl` | Exists | Code / Agents switch, settings |
| MenuBar, MenuHeader | `egui::MenuBar` plus `ui.menu_button` (raw) | missing | Missing: row of titles with underlined mnemonic, open title fill, hover-switch between titles, F10 and Alt+mnemonic focus | main menu bar |
| MenuItem, MenuSeparator, submenu | `action_system::ActionMenuItem`, `ActionPopupMenu` | `popup::ActionMenuSource`, `PopupChain` | Exists. Differs: icon column, mnemonic underline (`ActionMenuItem::underlined`), disabled reason tooltip | main menu, context menus, header menus |
| ToolStripe, StripeButton | hand-built in `chrome.rs` (`stripe_button`) | `workbench::Workbench::stripes` (3 cells, one row per button, top only) | Differs: no bottom group, no separator, no badge dot, no per-button tone, no explicit order; props `groups: [top, bottom]`, `buttons: [{id, icon, tooltip, selected, badge}]` | left and right stripes |
| PanelHeader | `widgets::tool_window_header`, `panel_header` (hand-built) | workbench title line (no action icons) | Differs: needs `title`, `view_selector`, `actions: [ToolbarButton]`, `tabs`, hide button | tool window headers |
| TabBar, Tab | `ui::tabs::{JBTabs, TabInfo}`; the editor strip is hand-built in `editor.rs` `tabs` | `components::JBTabs`, `tabs::TabsControl`, `TabInfo` | Exists for tool window tabs. Differs: the editor strip needs `preview` (italic), `error` (wavy underline), `tooltip`, `context menu`, overflow list, drag reorder (exists), diff and plugin tab kinds | editor tab strip, tool window content tabs, SE tabs |
| StatusBar, StatusBarItem | `widgets::flat_widget` in `chrome.rs` `status_bar`; kit `status_text.rs` | `statusbar::StatusBarControl` with 9 fixed NvChad slots (`StatusSlot::{Mode, File, Git, Center, Diagnostics, Lsp, Cwd, Cursor, Run}`) | Differs: needs an ordered list of items `{id, icon, text, tone, tooltip, popup, priority, min_width}` instead of fixed slots; progress and busy spinner | status bar |
| Breadcrumbs | `ui::breadcrumbs::BreadcrumbsComponent` | missing | Missing | status bar left, file path crumbs, code crumbs |
| ListRow | `ColoredListCellRenderer`, `popup_rows`, `widgets::list_row` | `collections::RowData` with `ColoredListCellRenderer` | Exists. Differs: right-aligned secondary text and trailing icon (SE rows), shortcut chip, match highlight ranges (`SearchMatch`), row tint (full width), group separator rows | all lists and popups |
| JBList | `ui::JBList` | `collections::JBList` | Exists | lists |
| Tree, TreeRow | `ui::collections` tree, `TreeRow` (28 call sites) | `collections::JBTree`, `TreeNode` | Differs: no tone on a node (git colors, test and excluded tints), no bold root with gray path, no checkbox column (Commit) | Project, Commit, Structure, Gradle, Problems |
| Table | `ui::collections::tree_table` | `collections::JBTable` | Exists | Find results, Git log |
| Popover | `ui::popup::JBPopup`, `Popover` | `overlay::Layers`, `popup::PopupOverlay`, `dialog::Dialog` | Exists as float; Differs: anchoring (`Placement` exists), size as percentage of the window with a minimum | Search Everywhere, completion, quick doc |
| ListPopup | `ui::popup::{ListPopup, BaseListPopupStep, ListSeparator}` | `popup::ListPopup`, `PopupChain` | Exists | choosers, branches, run widget rows |
| TextField | `ui::{JBTextField, TextField::search}`, `SearchTextField` | `controls::TextFieldControl`, `SearchFieldState`, `textarea` | Exists. Differs: hint text, clear button, toggle chips (`Cc`, `W`, `.*`) | SE query, Find, dialogs |
| Checkbox | `ui::{JBCheckBox, Checkbox}` | `controls::CheckBoxControl` | Exists | dialogs, popups |
| Button | `ui::Button::{primary, secondary, ghost, danger, selectable}` | `components::Button`, `controls::ButtonControl` (`primary`, `secondary`, `ghost`, `danger`) | Differs: `selectable` missing | dialogs, banners |
| ComboBox | `egui::ComboBox` (18 raw uses) and kit combo | `combo::ComboBoxControl` | Exists | settings, run configuration |
| Banner | `ui::InlineBanner` | `components::InlineBanner`, `controls::BannerControl` | Exists | disk conflict, Gradle, toolchain |
| Balloon | `notification::Notification` shown by `notifications.rs` | `balloon::Balloons` | Exists | notifications |
| Tooltip | `ide::HelpTooltip` | `tooltip::Tooltip` | Exists | every button and stripe item |
| Dialog | `ui::DialogWrapper` | `dialog::Dialog`, `Messages` | Exists | dialogs |
| ProgressBar, Spinner | `ui::ProgressBar`, `AsyncProcessIcon` | `components::ProgressBar` | Spinner missing (use braille frames) | status bar, tool windows |
| PropertyGrid, TitledSeparator | `ui::{PropertyGrid, TitledSeparator}` | missing | Missing | Settings, Run configurations |
| Splitter | egui resizable panels | `workbench` splitters (drag, double click reset) | Exists; Differs: stored per renderer (TUI-CMP-007) | tool windows, editor split |
| ScrollView | `egui::ScrollArea` (71 uses) | scroll inside each collection; wheel | Differs: no generic container; props `offset`, `content_len`, `on_scroll` | any scrolled content |
| Kbd | `ui` shortcut text | `components::Kbd` | Exists | shortcut column |
| EditorComponent | `archipel-kit` editor (the GUI paints its own editor today) | `editor::EditorComponent` | Exists in the TUI over the core document; the GUI editor is custom (`editor.rs` 2355 lines). Both already read the core document (`editor.*` commands), so this component is migrated last and only for the gutter, tabs and error stripe | editor |
| Terminal view | `terminal_ui.rs` painter | `dashboard`/terminal drawn in `tui/terminal.rs` | Differs: both consume `archipel-ide-terminal` cells; key encoding is duplicated (`M27`) | Terminal window |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-CMP-010 | MUST | The table above is kept as `docs/ui-components.md`'s source of truth by a kit test: `archipel-kit` exports a `VOCABULARY` list; each entry names the egui type, the ratatui type and the `Props` type; the test fails when one side is missing, except for entries in an explicit `PENDING` list that mirrors the "Missing" and "Differs" rows. The `PENDING` list is empty when Wave 0.4 is done. | `cargo test -p archipel-kit-tui vocabulary` lists the pending entries; the count only goes down. |
| TUI-CMP-011 | MUST | Missing and differing components of the table are implemented in `archipel-kit-tui` and `archipel-kit` with the shared `Props`, in this order of need: ToolbarDropdown, MenuBar and MenuHeader, ToolStripe with groups, PanelHeader with actions, StatusBar with an item list, Breadcrumbs, editor `TabBar` props, `TreeRow` tone, `ListRow` extras (right text, chip, match ranges, tint), Spinner, `Button::selectable`, PropertyGrid, ScrollView. | Each component has a gallery entry in both kit galleries (`archipel-kit-gallery`, `docs/tui/README.md`) and a conformance test, as `docs/contributing.md` requires. |
| TUI-CMP-012 | MUST | Raw egui in `archipel-ide-ui` is replaced as the surface migrates: no `painter()` calls and no `allocate_exact_size` in a migrated module, no `ui.label` for chrome text, no `egui::Popup::menu` (use `MenuItem`, `ListPopup`), no `SidePanel`/`TopBottomPanel` except in the one layout module. | A per-module counter test (`tests/raw_egui_budget.rs`) holds a budget per module that must not grow and that is zero for migrated modules. |

## 3. What moves from `archipel-ide-ui` into `archipel-ide-view`

The decision parts only; painting stays in the egui renderer. Module sizes in lines. `M<n>` refers to
the move list of `docs/tui-parity.md` section 5 (still valid; this table says where each lands).

| Module (lines) | Moves to `archipel-ide-view` | Stays in `archipel-ide-ui` | Move |
|---|---|---|---|
| `main_menu.rs` (515) | `main_menu_items`, `HIDDEN_AT_RUNTIME`, `EXTRA_CATEGORIES`, `extras`, `with_new_project`, `plugin_menu_title`, `hidden_extra`; builder `main_menu(&ViewCx) -> MenuBar` (mnemonics, shortcut text, enabled from `action_presentation`, disabled reason) | rendering through `MenuBar` | M20 |
| `chrome.rs` (1087) | `project_widget` data (name, initials, avatar color id, root path, popup rows), `vcs_widget` (branch, popup), `run_widget` (label, run, build, debug, stop buttons with enabled, tone, tooltip text and shortcut), `run_row` rows and row actions, `stripe_button` badge rules, `left_stripe` and `right_stripe_windows` composition, `status_bar` items, `window_title`, `first_line_truncated`, `is_read_only` probe (as a core service) | `window_controls`, `window_resize_handles`, drag, gradient painting, egui panels | M21, M22 |
| `toolwindows.rs` (1377) | header model per window (title, actions, view selector, tabs), `project_options_menu`, commit and find models (`commit`, `find_params`, `result_location`), `git_colors`, `change_tone`, `new_entry_parent`, `move_entry` | row painting, tree widget glue | M5, M15, M28 |
| `widgets.rs` (776) | `tool_window_header` and `panel_header` props; `pluralize`; the rest is painting | painting helpers | - |
| `editor.rs` (2355) | `tabs` model (name, icon key, preview, active, dirty, pinned, error flag, tooltip, context target), tab actions, `GutterData`, `GutterLayout` hit logic as ratios, gutter request handling, breadcrumbs source | text painting, caret, selections | M1, M13 |
| `context_menus.rs` (736), `actions.rs` (483) | `applies`, `Model`, `ContextFacts`, `context_rule`, `is_target_action`, `Target`, builder `context_menu(target) -> MenuItem[]` | menu rendering | M19 |
| `keybindings.rs` (523) | `UiAction`, `ui_action`, `is_implemented`, `TERMINAL_ACTIONS`, key naming (`Key` to the keymap's text, made toolkit-neutral) | egui key translation | M19, M27 |
| `palette.rs` (634), `popups.rs` (905), `find_popup.rs` (636), `switcher.rs` (183) | Search Everywhere and Find Action state and rows (tabs, ranking via `quick_nav`, footers, hints), Go to Line, File Structure, Recent Files, Find in Files model, dialogs' field models and validation | frames, layout | M13, M15 |
| `completion_ui.rs` (418), `intentions_ui.rs` (327), `doc_ui.rs` | completion rows, kind glyph, intention rows, doc blocks | painting | M17, M18 |
| `branches_popup.rs` (548) | branch popup composition and rules | menu rendering | M8 |
| `lsp_status.rs` (258), `index_status.rs` (121), `toolchain_status.rs`, `external_status.rs`, `notifications.rs` (172), `disk_banner.rs`, `gradle_banner.rs`, `toolchain_banner.rs`, `crash_notice.rs` | status item data, notice and banner content and actions | painting | M22, M23 |
| `settings_dialog.rs` (1172), `run_config_dialog.rs` (865), `new_project.rs` (708), `push_dialog.rs`, `clone_dialog.rs`, `welcome.rs` | dialog models (pages, fields, validation, buttons) | form painting | M24 to M26 |
| `build_window.rs` (896), `problems_ui.rs` (811), `structure_view.rs` (391), `test_view.rs`, `todo_window.rs`, `gradle_window.rs`, `bazel_window.rs`, `debug_window.rs`, `git_log_ui.rs`, `logcat_ui.rs`, `terminal_ui.rs` | tool window content models (rows, groups, toolbars, link resolution) | row and tree painting | M4, M7, M9 to M12, M14, M29, M30 |
| `icons.rs` (347) | the `Icon` to `Glyph` and file-type mapping (`Icon::kit`, `file_icon`) become a view function returning icon keys | painting | - |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-CMP-020 | MUST | Each row of the table is moved by "extract, keep the existing `archipel-ide-ui` tests green, switch the GUI to the builder, then use the builder in the TUI". The GUI must look the same after each move (the snapshot scenes of `docs/ux` and `UI-*` acceptance probes stay green). | The GUI's unit tests and `ui_audit.rs` pass unchanged on the commit that introduces each builder; the TUI commit that uses it removes the TUI's own copy (`tui/nav.rs`, `tui/layout.rs` decisions). |
| TUI-CMP-021 | MUST | Duplicates listed in `docs/tui-parity.md` 2.4 disappear in the same commit as the move that makes them redundant (run configuration rows, Problems grouping, Commit groups, intention rows, kind letters, status texts, surface-opening actions, terminal key bytes). | `grep` for the old function names in `crates/archipel-ide-tui` finds none after the phase. |
| TUI-CMP-022 | SHOULD | `archipel-ide-view` exposes `Chrome` (toolbar, menu bar, stripes, status bar, breadcrumbs, tool window headers, tab strip) as one value `Chrome::build(&ViewCx) -> Chrome`, so a single call answers "what does the chrome show for this state", which is what the parity tests (section 5) compare. | `Chrome::build` is the only entry the frontends use for chrome. |

## 4. Migration order

Each phase ends green on the required checks, with both frontends using the phase's builders, and
trims `crates/archipel-ide-ui/tests/raw_egui_budget.rs`. The editor needs no phase of its own: both
frontends already render the same core document (`editor.*` commands); only its gutter marks, tab
strip, error stripe and breadcrumbs move with their phases.

| Phase | Surfaces | Kit work (shared props) | IDE work | Exit criteria |
|---|---|---|---|---|
| 0.1 Foundations | none visible | `archipel-kit-model` crate, `VOCABULARY` test, `Look::gui` (TUI-GUI-120) | `archipel-ide-view` crate with `ViewCx`, `Chrome` skeleton, `no_ui_deps` and budget tests | crates compile, tests of TUI-CMP-001/002/010/012 exist |
| 0.2 Chrome | toolbar, main menu, stripes, tool window headers, editor tabs, status bar, breadcrumbs, context and tab menus, branches and run widget popups | ToolbarDropdown, MenuBar, ToolStripe groups, PanelHeader actions, StatusBar item list, Breadcrumbs, tab props, spinner | M19, M20, M21, M22, M8 | chrome parity test (section 5) green for all fixtures; TUI-GUI chrome requirements met |
| 0.3 Popups and lists | Search Everywhere, Find Action, Go to *, Recent Files, File Structure, Find in Files, completion, Alt+Enter, quick doc, balloons, banners | ListRow extras, Popover sizing, TextField chips, Tree tone, ScrollView | M13, M15, M17, M18, M23 | popup parity test green; `tui/popups.rs`, `tui/picker.rs`, `tui/nav.rs` reduced to rendering |
| 0.4 Tool windows | Project, Commit, Find, Build, Problems, Structure, Terminal, then Git, Debug, Gradle, Todo, Hierarchy, Bazel, Logcat, Android | Tree checkboxes, Table, PropertyGrid | M5, M7, M9 to M12, M14, M28, M29, M30 | `ToolWindow::ALL` has a TUI body; `unsupported()` guard gone |
| 0.5 Dialogs | Settings, Run/Debug Configurations, New Project, Clone, Push, Rename and refactor dialogs, Commit dialog, About, Welcome | PropertyGrid, TitledSeparator, Dialog buttons | M24, M25, M26 | dialog parity test green |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-CMP-030 | MUST | The phases are done in this order: chrome, popups and lists, tool windows, dialogs. A phase does not start before the previous one's parity test is green, except that the editor gutter wave (`docs/tui-parity.md` Wave 1) may run in parallel after phase 0.2 because the editor already shares the core document. | The wave plan in `docs/tui-parity.md` lists the same order. |
| TUI-CMP-031 | MUST | New GUI UI work after this spec is written against `archipel-ide-view` and the kit vocabulary from the start; a pull request that adds an egui-only decision in a migrated surface fails the budget test. | CI runs `raw_egui_budget` and `no_subsystem_reads`. |
| TUI-CMP-032 | SHOULD | A surface not yet migrated is listed in `docs/ui-components.md` with its owner module, so the GUI-only share of the UI is a visible number. | The doc has a table of surfaces with `migrated` yes/no; the count of "no" only goes down. |

## 5. Shared-model render test

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| TUI-CMP-040 | MUST | One test renders the same props through both backends and compares structure. For a fixture `IdeState` it builds the props with `archipel-ide-view`, renders them with the egui kit (headless, `archipel-kit` feature `testing`, egui accesskit tree) and with the ratatui kit (`TestBackend`, the kit's hit map with accessible names), reduces each result to an **outline**: a flat list of `{role, id, label, shortcut text, tooltip title, enabled, checked or selected, expanded, badge, depth, order}`, and asserts the two outlines are equal to each other and to the outline computed directly from the props. Pixels and cells are not compared. | `crates/archipel-ide-view/tests/render_parity.rs`; failing output prints a diff of the two outlines. |
| TUI-CMP-041 | MUST | The fixtures are: no project; Java Gradle project with two open documents (one dirty, one pinned, one with errors); Android project (device selector, Build Variants, Logcat stripe buttons); a running task; a debug session; a read-only file; an update available; indexing in progress; a plugin with a menu item and a tool window; the macOS notation and the Windows/Linux notation of shortcuts. | Each fixture is a named function; the test table lists all of them. |
| TUI-CMP-042 | MUST | The surfaces compared are every one the product owner listed: main toolbar, main menu (all twelve menus expanded to the deepest submenu), run widget popup, project widget popup, VCS branch popup, left, right and bottom stripes, tool window headers, editor tab strip, tab and editor and tree context menus, breadcrumbs, status bar, Search Everywhere (each tab) and Find Action, Go to File, completion, Alt+Enter, and each dialog as it is migrated. | The test is parameterised over a `SURFACES` list; adding a surface without a test row fails a completeness test (as `tests/design_rules.rs` does for components). |
| TUI-CMP-043 | MUST | The outline extraction exists for both kits as a public test helper (`archipel_kit::testing::outline`, `archipel_kit_tui::testing::outline`) so that other crates and the gallery can reuse it. Every component sets its accessible name and state (a kit checklist item already); the helper reads those. | A kit unit test builds each component and asserts a non-empty outline entry. |
| TUI-CMP-044 | SHOULD | A second, cheaper test runs only the view layer: for each fixture `Chrome::build` is compared against golden JSON outlines checked into `crates/archipel-ide-view/tests/golden/`, so a change of an item, an order or a shortcut shows in review. | `UPDATE_SNAPSHOTS=1` regenerates the files, as the kit's snapshots do. |

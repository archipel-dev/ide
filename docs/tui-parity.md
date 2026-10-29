# TUI and GUI feature parity: matrix and plan

The terminal frontend (`crates/archipel-ide-tui`, started with `archipel-ide --tui`) is a second
frontend over the same core as the window (`crates/archipel-ide-ui`). Both call the same
commands, actions and services of `crates/archipel-ide-core`, so a feature of the GUI that the TUI
lacks is a missing terminal surface, not a missing engine. This file lists every GUI feature,
the core API behind it, what the TUI has today, how to present it in a terminal, and the order to
build it. It is a plan: nothing here changes code.

**Binding product decision (supersedes the NvChad-style design):** the TUI must look and behave exactly
like the GUI. The target is specified in `docs/specs/tui/01-gui-faithful-terminal-ui.md` (what the
terminal shows, cell by cell, and the key fallbacks) and `docs/specs/tui/02-shared-component-layer.md`
(one UI layer, two renderers). Where a row, a layout sentence or a key table below says "no toolbar", "keep
the no-toolbar layout", "status line segments" or "`Ctrl+K` chords", it describes the old design and is
**being replaced**; Wave 0 below is the new first step. The feature matrix itself (what exists in the
core) stays valid. Companion documents: `docs/tui.md` (what the
TUI does now), `docs/architecture.md`, `docs/specs/` (behavior), `docs/parity/` (GUI versus the
reference).

Written against `origin/main` of this worktree. Evidence was read from source; nothing was built or
run for this document, so "Done" means the code path exists in `crates/archipel-ide-tui/src` (and
the test named, if any), not that it was exercised again.

## 1. Conventions

Paths: `core/` is `crates/archipel-ide-core/src/`, `ui/` is `crates/archipel-ide-ui/src/`, `tui/`
is `crates/archipel-ide-tui/src/`, `kit-tui` is `archipel-kit-tui` (the `archipel-dev/kit` repo,
pinned by `Cargo.lock` to commit `577ce54`). Command ids (`debug.stepOver`) are the typed commands
of `archipel-ide-command`; action ids (`StepOver`) are reference actions of the keymap
(`core/keymap_commands.rs` maps them to commands).

TUI status:

- **Done**: the TUI offers the same behavior.
- **Partial**: some of it works; the cell says what is missing.
- **Missing**: no terminal surface (the command may still exist and run, see 2.3).

Core API column: `file:function` of the core or an engine crate. **GUI-ONLY** means the logic lives in
`archipel-ide-ui`; the `M<n>` tag points at the move list in section 5. Both frontends must share it,
so moving it into the core is a prerequisite and removes duplication that already exists in
`tui/`.

Effort: **S** about a day or less and under 200 lines; **M** a few days, one module plus tests;
**L** a week or more, or needs a new kit-tui component or an SDK change. `K` marks a dependency on
`kit-tui` (another repository), `P<n>` a prerequisite task of section 6.

## 2. What the code says today

These facts shape the plan; each was checked in source.

### 2.1 The terminal editor widget already has the gutter

`kit-tui`'s `EditorComponent` (`editor/mod.rs`, `editor/paint.rs`) draws a sign column (a change
column plus lanes of two cells each), line numbers and the fold column, and reports clicks. It takes
`.gutter_icons(&[GutterIcon])` (line, lane, `Glyph`, `Tone`, accessible name),
`.change_markers(&[ChangeMarker])` (added `▎` green, modified `▎` blue, deleted `▔` red),
`.virtual_text(bool)` (end-of-line diagnostic text only), `.scrollbar(bool)` and `.highlights(..)`.
Its `EditorOutput` carries `gutter_clicks` (line, area: line number, icon lane, change marker, other;
button; modifiers), `navigation` (Ctrl+click), `context_menu` (right press), `hover`,
`fold_preview` and `stripe_hover`.

`tui/view.rs` and `tui/layout.rs` use none of it: the editor is built with `.highlighter`,
`.language`, `.highlights` (diagnostics only), `.bracket_folding(false)` and `.wrap(true)`, and
`forward_to_editor_widget` (`tui/app.rs`) keeps only `out.hint` and `out.consumed`. So change bars,
run icons, breakpoints, Ctrl+click, the editor context menu and hover are not missing for lack
of a widget; they are missing because the TUI never feeds or reads it. Things the widget cannot do
yet (K): inline virtual text (inlay hints), end-of-line text other than diagnostics (code vision,
inline debug values), whole-line background tints (execution line, breakpoint line), distinct glyph
shapes for conditional, disabled and log breakpoints and the execution arrow (the glyph set has
`Breakpoint`, `Run`, `Debug`, `Bookmark`, `StepOver/Into/Out`, `Stop`, `Pause`, `Thinking`
(lightbulb) but no diamond, hollow circle or arrow), a way to reserve a fixed number of gutter
lanes (the number of lanes follows the highest lane used, minimum one, so a second lane appearing shifts the text), and a tone on `TreeNode`
(git colors in the project tree).

### 2.2 Tool windows: 6 of 20

The core knows 20 tool windows (`core/toolwindow.rs` `ToolWindow::ALL`). The TUI shows 6 (`tui/app.rs`
`WINDOWS`): Project, Commit, Build (as "Run"), Problems, Find, Terminal. The other 14 are
Structure, Device Manager, Logcat, Git, Build Variants, Resource Manager, Running Devices,
Gradle, Bazel, AI Chat, Debug, Hierarchy, Agents, Todo. `tool_window_command` answers "not
available in the terminal yet" for them.

### 2.3 A command guard hides features the core already has

`tui/app.rs` `unsupported(id)` (called from `TuiApp::command`) refuses these before they reach the
core: every id starting with `debug.`, `android.`, `ai.`, `agents.`, `hierarchy.`,
`plugin.manager`, `history.`, `diff.`, `settings.`, and the ids `refactor.rename`,
`code.generate`, `navigate.showUsages`. Two of them need no new engine work: `code.generate`
fills the same code-actions popup (`core/generate.rs` `finish_generate` sets
`code_actions.popup`) that the TUI already draws for Alt+Enter, and Rename is a three-command
session (`refactor.rename.setName`, `.apply`, `.cancel`, `core/refactor.rs`). Ids not in the
list (for example `git.annotate`, `git.showDiff`, `vcs.showChangeDiff`, `navigate.findUsages`,
`git.log.show`, `todo.show`) do run, and then nothing draws their result.

### 2.4 Logic already duplicated between the frontends

The TUI re-implements pieces that the GUI also owns privately, instead of both calling the core:
run output to file links (`tui/panels.rs` `refresh_run` versus `ui/build_window.rs`), the
Problems grouping (`refresh_problems_list` versus `ui/problems_ui.rs`), the Commit window groups,
path mapping and "included" defaults (`refresh_commit` versus `ui/toolwindows.rs` `commit`), the
intention rows (`tui/popups.rs` `intention_rows` versus `ui/intentions_ui.rs` `display_rows`),
completion kind letters (`tui/popups.rs` `kind_letter` versus `ui/completion_ui.rs`
`kind_glyph`), the run configuration rows (`tui/nav.rs` `config_rows` versus `ui/chrome.rs`
`run_row`), terminal key bytes (`tui/keys.rs` `key_bytes` versus `ui/terminal_ui.rs`
`key_sequence`), the surface-opening actions (`tui/actions.rs` `TuiAction` versus
`ui/keybindings.rs` `UiAction`) and the status texts (`tui/layout.rs` `draw_status` versus
`ui/chrome.rs` `status_bar`). Section 5 lists these as moves.

### 2.5 Things the GUI itself does not have

Parity means "what the GUI offers". The GUI has no Bookmarks tool window or gutter mark (the
model `core/recent_popup.rs` `Bookmark` exists, no command and no UI), no sticky lines, no override
or implement gutter markers (`archipel-ide-daemon/src/passes.rs` `LineMarker` exists, the editor
does not paint it), no Generate dialog of its own, no Replace in Files UI (`core/search_replace.rs`
exists), no Services or Maven tool window, no View Breakpoints dialog, no merge-conflict or shelf
UI, no coverage (the gutter menu entry is disabled, "IJ-RUN-069"). Table K lists them; they are
not counted as TUI gaps, and the TUI should wait for the GUI (or both get them from one core
model).

## 3. Summary

The counts are of the rows in section 4, parts A to E and G to J (part F indexes the same features and is
counted on its own; table K is excluded). They were produced from the status words of the tables.

| Area | Rows | Done | Partial | Missing |
|---|---|---|---|---|
| A. Editor surface and gutter | 37 | 12 | 8 | 17 |
| B. Debugger | 10 | 0 | 0 | 10 |
| C. Code intelligence, navigation, search | 18 | 4 | 6 | 8 |
| D. Run, build, test windows | 13 | 1 | 4 | 8 |
| E. Git and local history | 13 | 1 | 2 | 10 |
| G. Dialogs, settings, project lifecycle | 10 | 2 | 1 | 7 |
| H. Chrome, status bar, notices | 9 | 1 | 5 | 3 |
| J. Plugins, AI, Agents, Android | 14 | 1 | 1 | 12 |
| **Total** | **124** | **22** | **27** | **75** |

Tool windows (part F, an index of the same features, not added to the total): of the 20 windows of the core `ToolWindow` enum the TUI has 6 (Partial: Project, Commit, Find, Build as Run, Problems, Terminal) and lacks 14 (Structure, Device Manager, Logcat, Git, Build Variants, Resource Manager, Running Devices, Gradle, Bazel, AI Chat, Debug, Hierarchy, Agents, Todo), plus plugin tool windows.

40 of the 124 rows depend on logic that lives in the GUI crate (tagged GUI-ONLY); section 5 lists the 30 moves. 75 rows are Missing, but most of that gap is presentation, not engine: the core already has the command or model for nearly every one (section 2.1 and 2.3 show why the editor ones are not even blocked by the widget).

## 4. Matrix

### A. Editor surface and gutter

| ID | GUI feature (source) | Core API | TUI status and evidence | Terminal presentation | Effort, deps |
|---|---|---|---|---|---|
| A01 | Typing, multiple carets, clipboard, undo, line operations (`ui/editor.rs`, `ui/editor_keys.rs`) | `core/editor_input/*`, commands `editor.type`, `editor.action`, `editor.paste` | **Done**: `tui/view.rs` `DocView` mirrors the core document; `tests/editor_cases.rs` replays 341 editor spec cases | n/a | n/a |
| A02 | Syntax colors (`ui/editor.rs` `highlighted_line_job`, `ui/syntax.rs`) | `Document.analysis.highlights` (`archipel-ide-language`) | **Partial**: `tui/view.rs` `kind_of` folds the attribute keys into 11 kit kinds; the GUI colors by key | Pass true-color spans per key to the widget instead of kinds | S, K |
| A03 | Line numbers, caret row, indent guides, selection | kit | **Done**: `EditorComponent` | n/a | n/a |
| A04 | Code folding, gutter chevron, collapsed placeholder (`ui/folding.rs` `FoldView`) | `core/folds.rs`, `editor.toggleFold` | **Done**: `tui/view.rs` `fold_signature`, chevron click becomes `editor.toggleFold` (unit test `the_fold_regions_of_the_core_are_shown_and_a_chevron_click_toggles_them`) | n/a | n/a |
| A05 | Diagnostic squiggles and message on hover (`ui/problems_ui.rs`) | `core/problems.rs` `cached_active_problems` | **Done**: `tui/view.rs` `highlights`, `tests/flows.rs` `a_diagnostic_shows_up...` | n/a | n/a |
| A06 | Error stripe with marks, click to jump (`problems_ui::error_stripe`) | same | **Done**: kit scrollbar column shows when highlights exist | n/a | n/a |
| A07 | Inspection widget: counts, next/previous problem (`problems_ui::inspection_widget`) | `ProblemCounts::of`, `editor.nextProblem`, `editor.previousProblem` | **Done**: status line counts, F2 and Shift+F2, click opens Problems | n/a | n/a |
| A08 | Find and Replace bar (`ui/findbar.rs`) | `core/find_session.rs` `FindSession` exists but only tests use it; the GUI bar uses `archipel_ide_text::ops::find_all`. **GUI-ONLY** M16 | **Done**: kit find bar (`tui/layout.rs` `is_find_key`); behavior can drift from the spec model (no-wrap hint, preserve case, templates, history) | n/a | S: adopt `FindSession` in both |
| A09 | Git change bars in the gutter (`ui/change_popup.rs` `paint_marker`, `marker_color`) | `core/line_changes.rs` `document_changes` returns `Vec<archipel_ide_git::Hunk>`; `Hunk::kind()` | **Done**: `tui/view.rs` `refresh_markers` (hunks of `document_changes`, cached by text revision and Git status pointer) feeds the kit's change column through `.change_markers(..)` in `tui/layout.rs`; `tests/gutter.rs` `change_bars_show_added_modified_and_deleted_lines_in_the_palette_colors`, `a_change_bar_moves_with_its_line_when_lines_are_inserted_above` | `▎` in the change column: green added, blue modified, `▔` red deleted (kit `gutter_row`); hunk kind maps to `ChangeKind` | S, Wave 1 |
| A10 | Run and test icons in the gutter (`ui/editor.rs` `run_icons`) | `core/gutter_commands.rs` `gutter_marks`, `cached_gutter_marks`; the 0.25 s debounce is `archipel-ide-view` `popups::gutter::RunMarks`, used by both frontends (was **GUI-ONLY**) | **Done**: `tui/gutter.rs` `refresh_gutter` puts the icons in lane 1 (two lanes are reserved, so the text never shifts); `Glyph::Run` for a main or a single test and `Glyph::Rerun` for a test container, as the GUI's `paint_icon` (the plan's `Glyph::Test` was not used); `tests/gutter.rs` `run_icons_sit_on_the_main_lines_and_the_text_does_not_shift`, `run_icons_are_computed_after_a_pause_while_typing`, `rust_main_and_test_functions_get_run_icons` | Lane icon: `Glyph::Run` in `Tone::Run` (green) for a main, `Glyph::Test` for tests and test containers; accessible name is the label | S, Wave 1, K lanes (P4) |
| A11 | Run or Debug from the gutter icon: popup with Run X, Debug X, Run with Coverage (disabled), Modify Run Configuration (`ui/gutter_popup.rs`) | `core/gutter_commands.rs` `run_gutter(line, edit, debug)` via `run.gutter`; the entry list is `archipel-ide-view` `popups::gutter::run_popup`, drawn by the GUI and the terminal (was **GUI-ONLY**) | **Partial**: `tui/gutter.rs` routes a click or right click on a run icon to `Popup::Gutter` (a kit popup menu with the keymap's shortcuts); Modify Run Configuration selects the configuration and opens the configurations list; Run, Debug and Modify are disabled while something runs. Missing: Debug starts nothing yet (it says so; Wave 2) and there is no key for the popup of the caret line (`RunClass` runs the target at the caret); `tests/gutter.rs` `the_run_icon_opens_the_popup_with_the_gui_rows_and_shortcuts`, `modify_run_configuration_selects_the_configuration_without_starting_a_run`, `debug_is_disabled_while_something_runs` | kit popup menu anchored at the icon; click on the icon or `Ctrl+K G` on the caret line; entries show shortcuts from the keymap; Esc or outside click closes | S, Wave 1, P1 |
| A12 | Toggle a line breakpoint: click a line number, Ctrl+F8, hover preview (`ui/editor.rs` lines near `debug.toggleBreakpoint`) | `core/debug_commands.rs` `toggle_breakpoint`, `breakpoint_lines`; stored in `.archipel/breakpoints.json` | **Done**: `unsupported()` lets the four gutter ids of `debug.` through; a click on a number (or on the breakpoint lane) toggles, hover shows a grey preview that follows the pointer (kit `EditorOutput.gutter_hover`), Ctrl+F8 or `Ctrl+\ P`; `tests/gutter.rs` `a_click_on_a_line_number_toggles_a_breakpoint_and_writes_the_file`, `hovering_a_line_number_previews_a_breakpoint_that_follows_the_pointer`, `replay_exe_dbg_010_ctrl_f8_toggles_and_removes_a_breakpoint` | `●` red in lane 0 (existing `Glyph::Breakpoint`, `Tone::Error`); hover on a number shows a dim `●`; line number click, Ctrl+F8, or `Ctrl+K P` toggles | S, Wave 1 |
| A13 | Breakpoint properties: condition, log message (logpoint), hit count, Remove (`ui/debug_dialogs.rs` `breakpoint_properties`) | `core/debug_commands.rs` `breakpoint_options`, `set_breakpoint_options`; commands `debug.editBreakpoint`, `debug.setBreakpointOptions`, `debug.closeBreakpointEditor`; `archipel-ide-dap` `BreakpointOptions` has exactly three fields; the form fields, title and the Done and Remove effects are `archipel-ide-view` `popups::breakpoint` (was **GUI-ONLY** M4) | **Done**: a dialog with the three fields (`tui/dialogs.rs` `FormContent`), Done (Enter) and Remove Breakpoint, opened by a right click on a number, Ctrl+Shift+F8 or `Ctrl+\ X`; the mark is red for a line breakpoint, the warning color with a condition or hit count and the accent color for a logpoint (the kit has one breakpoint glyph, so the variants differ by color until it gets the shapes); `tests/gutter.rs` `a_right_click_on_a_number_opens_the_form_and_a_condition_turns_the_mark_orange`, `a_log_message_makes_a_logpoint_and_escape_keeps_the_breakpoint_as_it_was` | Marker variants: conditional is an orange `●` (a diamond `◆` once the kit has the glyph), logpoint a dim `●` with a different tone, plus a dialog with three fields and Done and Remove; opened by right click on the number, Ctrl+Shift+F8 or `Ctrl+K X` | M, Wave 1, K glyphs |
| A14 | Execution point: arrow and line tint, editor reveals the frame (`debug_window::paint_execution_arrow`, `ui/editor.rs`) | `core/debug_commands.rs` `execution_point` | **Missing** | Arrow `▶` in lane 0 and a row tint (K line highlight; fallback: bold line number in accent) | S, Wave 2, K |
| A15 | Inline debug values at the end of the line (`debug_dialogs::paint_inline_value`) | `core/inline_values.rs` `inline_values(path)` returns `Vec<InlineValue{line,text}>` | **Missing** | Dim virtual text after the line: `x = 1, name = "a"` | M, Wave 2, K end-of-line text |
| A16 | Blame annotations, click opens the commit in the Git log (`ui/blame_gutter.rs`) | `core/annotations.rs` `toggle_annotations`, `document_annotations` returns `Vec<BlameLine>`; tooltip text and age shade **GUI-ONLY** M3 | **Missing**: `git.annotate` runs but nothing draws | Extra left column (7-char hash, author, age) faded by age; Enter or click opens the Git log on that commit; toggled by `git.annotate` | M, Wave 4, K annotation column |
| A17 | Code vision: `N usages` and last author after declarations (`ui/editor.rs` lenses) | `core/code_vision.rs` `code_vision(index)` returns lenses and inlays | **Missing** (`docs/tui.md`: "no terminal surface yet") | Dim end-of-line text `2 usages  Ada`; click or Enter runs Find Usages at `CodeLens.line/column` | M, Wave 3, K end-of-line text |
| A18 | Parameter name inlay hints (`ui/editor.rs` `hints_by_line`) | `core/code_vision.rs` `InlayLabel{line,column,text}` | **Missing** | Dim `name:` inserted before the argument; caret and wrap math must skip it | L, Wave 7, K inline virtual text |
| A19 | Lightbulb at the caret line, red when a fix exists (`ui/intentions_ui.rs` `lightbulb`) | `core/code_actions.rs` `current_alerts`, `current_code_actions` | **Missing** | `Glyph::Thinking` in lane 1 on the caret line, `Tone::Error` for fixes, `Tone::Warning` otherwise; click or Alt+Enter opens the popup | S, Wave 3, K lanes |
| A20 | Alt+Enter intentions popup (`ui/intentions_ui.rs`) | `core/code_actions.rs` `request_code_actions`, `apply_alert`; rows **GUI-ONLY** M18 | **Done (0.3)**: rows from `view::popups::intentions`; submenu headers show `▸`, Right opens, Left closes, Enter toggles | Expandable rows with `▸`; Right opens, Left closes | S, Wave 3 |
| A21 | Completion popup with details and a documentation panel (`ui/completion_ui.rs`, `ui/doc_ui.rs` `show_completion_panel`) | `core/completion.rs`, `completion_documentation()`; rows **GUI-ONLY** M17 | **Partial**: list, kind letter, bold match, strike for deprecated, detail, Enter and Tab are done (`tui/popups.rs`); the right-aligned type or package is done (`view::popups::completion`); missing the documentation panel and the indexing footer | Side panel to the right (or below on narrow terminals) with the markdown documentation; footer line `Indexing...` | M, Wave 3, M17 |
| A22 | Quick Documentation (Ctrl+Q, pointer rest) (`ui/doc_ui.rs` `show_quick_doc`, `track_hover`) | `core/quick_doc.rs` `request_quick_doc`, `toggle_quick_doc`, command `language.quickDoc`, `language.hoverAt` | **Missing** (`docs/tui.md`) | Overlay near the caret rendering markdown (definition block first); Esc closes; pointer rest after 0.5 s (kit `hover`) | M, Wave 3, M17, K markdown renderer |
| A23 | Parameter info (Ctrl+P, auto after `(`) (`ui/doc_ui.rs` `show_parameter_info`) | `core/signature_help.rs` `request_parameter_info`, `signature_lines`, `close_parameter_info` | **Missing** (`docs/tui.md`). Ctrl+P is Find Action in terminals without the kitty protocol, so the shortcut needs a chord | One-line overlay above the caret with the active parameter bold; `Ctrl+K I` fallback | S, Wave 3 |
| A24 | Hover on code (documentation after a pause) | `language.hoverAt`, `core/quick_doc.rs` `request_quick_doc(offset, from_mouse)` | **Partial**: the kit float shows diagnostics under the pointer; no documentation hover (the widget's `hover` output is dropped) | Same overlay as A22, anchored at the pointer cell | S after A22 |
| A25 | Ctrl+click: Go to Declaration; with Shift the type; with Alt implementations (`ui/editor.rs`) | commands `navigate.declaration`, `navigate.typeDeclaration`, `navigate.implementation` | **Missing**: `forward_to_editor_widget` drops `EditorOutput.navigation` | Underline the identifier while Ctrl is held (needs terminal modifier reports); click runs the command | S, Wave 3 |
| A26 | Editor context menu (`ui/context_menus.rs`, `EditorPopupMenu`; Git submenu with Annotate, History, Show Diff, Rollback Lines) | catalog `archipel-ide-actions` `Catalog::shared_menu`; applicability **GUI-ONLY** M19 | **Missing**: `EditorOutput.context_menu` dropped | kit popup chain with submenus at the click cell; also `Menu` key and `Shift+F10`-style chord | M, Wave 3, M19 |
| A27 | Snippet and live template tab stops boxed while active (`ui/editor.rs`) | `core/snippet_session.rs`, `snippet.next`, `snippet.previous`, `snippet.exit` | **Partial**: templates expand on typing (`editor.type`, `live: true`); tab stops are not marked (`DocView.highlights` holds diagnostics only) | Reverse-video boxes on the active and pending tab stops (K range style) | S, Wave 3 |
| A28 | Rename occurrences preview while typing (`ui/refactor_ui.rs`, `ui/editor.rs` `rename`) | `core/refactor.rs` `RenameSession.occurrences` | **Missing** (see C03) | same as A27 | with C03 |
| A29 | Banner: file changed on disk while edited (`ui/disk_banner.rs`) | `Document.conflict` (`core/saving.rs`, `core/disk_sync.rs`), commands `editor.reloadFromDisk`, `editor.keepMemoryVersion`; text **GUI-ONLY** M23 | **Missing**: no surface for the conflict | Inline banner above the editor: message and two buttons (Load Disk Version, Keep My Changes); keys `L` and `K` when focused. High priority: silent data-loss risk | S, Wave 3, M23 |
| A30 | Save failure report dialog (`ui/popups.rs` `Dialog::SaveFailure`) | `core/services/saving.rs` `SaveFailureReport` | **Missing**: `tui/app.rs` `autosave` only dispatches `editor.saveAll` | Dialog listing files and reasons, Retry and Close | S, Wave 3 |
| A31 | Read-only file indicator, line separator, indent, encoding (`ui/chrome.rs` `status_bar`) | `archipel_ide_text::format::detect_indent`, `detect_line_separator`; **GUI-ONLY** M22 | **Missing** (see H03) | status segments | with H03 |
| A32 | Tabs: select, close, modified dot, reopen closed (`ui/editor.rs` `tabs`) | `core/tabs.rs`, `editor.activate`, `editor.close`, `editor.reopenClosed` | **Done**: `tui/layout.rs` tab line, `tests/flows.rs` `a_click_on_a_tab_selects_it...` | n/a | n/a |
| A33 | Tab extras: pin, preview tab, tab context menu (close others, left, right, unmodified), drag to reorder or to another group | `core/tabs.rs`, commands `editor.togglePin`, `editor.closeOthers`, `editor.closeLeft`, `editor.closeRight`, `editor.closeUnmodified`, `editor.keepTabOpen` | **Partial**: commands reachable through Find Action; no menu, no pin mark, no drag | Tab context menu (A26 popup), pin glyph, `Alt+Shift+Left/Right` reorder | S, Wave 3 |
| A34 | Editor splits (right, down, orientation, ratio, group focus) (`ui/editor.rs` `show_groups`, `dividers`) | `core/splits.rs`, `archipel-ide-split` | **Missing** (`docs/tui.md`: "shown as one tab list") | Split panes with a one-cell divider (kit workbench panes; the kit `Pane` exists); mouse drag of the divider | L, Wave 6, K |
| A35 | Markdown: editor, split and preview layouts, floating format bar (`ui/markdown_ui.rs`) | `core/markdown_commands.rs`, `archipel-ide-markdown`, command `markdown.layout` | **Missing** | Preview pane rendered with terminal styles (headings, lists, code, tables); images as a placeholder line | L, Wave 6, K markdown renderer |
| A36 | Column (block) selection by Alt-drag and Shift+arrows (`ui/editor.rs` `drag_block`) | `archipel_ide_text::multicaret`, `editor.extendColumnSelection` | **Partial** (`docs/tui.md`: Shift+arrows in column mode do not extend) | Alt-drag in the widget; fix the key path | S, Wave 6 |
| A37 | Inline AI completion ghost text, Tab, word, Esc (`ui/ai_inline.rs`) | `core/ai_inline.rs`, `ai.inline.request`, `.accept`, `.acceptWord`, `.dismiss` | **Missing**: `ai.` is guarded | Dim text after the caret (K inline virtual text); Tab accepts, Ctrl+Right word | M, Wave 7, K |

### B. Debugger

| ID | GUI feature (source) | Core API | TUI status and evidence | Terminal presentation | Effort, deps |
|---|---|---|---|---|---|
| B01 | Start Debug (toolbar, Shift+F9, Debug in the gutter popup), "Building before debugging" state (`ui/chrome.rs`, `ui/debug_window.rs`) | `core/debug_commands.rs` `start_debugging`, `DebuggerState.pending`, `core/java_debug.rs` `JavaStage`; command `debug.start` | **Missing**: guarded | Status line shows `Debugging <name>`; Debug window shows the build/attach state | M, Wave 2 |
| B02 | Stepping: Resume, Pause, Step Over, Into, Out, Force Step Into, Run to Cursor, Stop | commands `debug.resume`, `debug.pause`, `debug.stepOver`, `debug.stepInto`, `debug.stepOut`, `debug.forceStepInto`, `debug.runToCursor`, `debug.stop` | **Missing**: guarded | Keys as the keymap: F9, F8, F7, Shift+F8, Alt+Shift+F7, Alt+F9, Ctrl+F2 (Stop); toolbar row with the kit `ActionToolbar`; fallback chords in section 7 | M, Wave 2 |
| B03 | Debug tool window shell: toolbar, session name, tabs "Threads & Variables" and "Console" (`ui/debug_window.rs` `show`) | `core/debug_commands.rs` `DebuggerState`, `archipel_ide_dap::Session` | **Missing**: `Debug` has no kit window (`tui/app.rs` `WINDOWS`) | Bottom window `debug` (Alt+5) with `JBTabs`; frames left, variables right; same area logic as Run | M, Wave 2, P2 |
| B04 | Status text: starting, running, suspended at breakpoint (`debug_window::status_text`) | `Session::state`, **GUI-ONLY** M4 | **Missing** | Colored line (`Tone::Success` running, `Tone::Warning` suspended) | S, P2 |
| B05 | Threads and frames list, select frame moves the editor (`debug.selectFrame`) | `Session::frames`, `selected_frame`; `debug.selectFrame` | **Missing** | `JBList` rows `name  file:line` (dim), no source rows in `Tone::Secondary` | S, Wave 2 |
| B06 | Variables tree with lazy children (`debug.loadVariables`), expand state | `Session::scopes`, `debug.loadVariables`; row flattening and `expanded` **GUI-ONLY** M4 | **Missing** | `JBTree` with `name = value` and type dim; expanding a row sends the command and shows `...` until the answer | M, Wave 2, P2 |
| B07 | Watches: add, edit, remove, persisted (`debug_window::watches_panel`) | `core/debug_commands.rs` `add_watch`, `edit_watch`, `remove_watch`; `.archipel/debug-state.json` | **Missing** | Watches section above the variables; `Insert` adds, `F2` edits, `Delete` removes | S, Wave 2 |
| B08 | Evaluate expression: inline field and dialog (`debug.evaluate`, `debug.evaluateDialog`, `debug.closeEvaluateDialog`, Alt+F8) | `Session::evaluations`, `core/debug_commands.rs` | **Missing** | Text field in the Debug window; Alt+F8 opens a dialog (kit `DialogContent`), last 5 results listed | S, Wave 2 |
| B09 | Debug console output (`debug_window::console`) | `Session::output` with `OutputCategory` | **Missing** | `LogView` tab with stderr tone | S, Wave 2 |
| B10 | Debug and Stop buttons in the main toolbar, device and config selection (`ui/chrome.rs` `run_widget`) | `core/run_config_commands.rs`, `debug.start`, `debug.stop` | **Missing**: the run segment offers Run only | Click on the run segment opens the config picker; add Debug as a second action row (`D` key) | S, Wave 2 |

### C. Code intelligence, navigation and search

| ID | GUI feature (source) | Core API | TUI status and evidence | Terminal presentation | Effort, deps |
|---|---|---|---|---|---|
| C01 | Go to Declaration, Back, Forward | `core/navigation.rs` `request_navigation`, `navigate_history`; commands `navigate.declaration`, `navigate.back`, `navigate.forward` | **Done**: `tests/flows.rs` `go_to_declaration_and_back_navigate_through_the_language_server` | n/a | n/a |
| C02 | Chooser for several targets (Go to Declaration, Implementation, Go to Test, Super Method) (`ui/navigation_ui.rs`) | `core/navigation.rs` `NavigationState.choices` (`NavigationList`), `navigate_to` | **Missing**: no code reads `navigation().choices` | Picker overlay (reuse `tui/picker.rs`) with file and line preview footer | S, Wave 3 |
| C03 | Rename (Shift+F6): in-place name with live occurrences, dialog when the server has no `prepareRename` (`ui/refactor_ui.rs`) | `core/refactor.rs` `start_rename`, `set_rename_name`, `apply_rename`, `cancel_rename`; state `RenameSession` | **Missing**: `refactor.rename` is guarded | Input dialog (existing `tui/dialogs.rs` `InputContent`) prefilled with the name; occurrences boxed as A27; Enter applies, Esc cancels | M, Wave 3 |
| C04 | Generate: constructor, getters and setters, equals/hashCode, toString, override methods (Alt+Insert) | `core/generate.rs` `request_generate`, shows the code-actions popup | **Partial**: `code.generate` is guarded although the popup it fills is drawn (A20) | Remove the guard; popup shows disabled entries with a reason | S, Wave 3 |
| C05 | Extract variable, method, constant; Inline; Optimize Imports; Reformat Code | `core/refactor.rs` `request_refactoring`, `reformat_code`; commands `refactor.extract*`, `refactor.inline`, `refactor.optimizeImports`, `refactor.reformatCode` | **Partial**: not guarded, so Find Action and keys run them; no TUI test and no feedback beyond the status line | Same; add a flow test per command | S, Wave 3 |
| C06 | Import chooser ("Class to Import", `Method to Import`) (`ui/import_chooser.rs`) | `core/language_packs.rs` `import_chooser`, command `<provider>.chooseImport` | **Missing** | Picker with `Class.member` and package in the trailing column | S, Wave 3 |
| C07 | Show Usages popup and Find Usages view (`ui/navigation_ui.rs`) | `core/navigation.rs` (`navigate.showUsages`, `navigate.findUsages`) | **Missing**: `navigate.showUsages` guarded; the usages list has no view | Show Usages as a picker with preview; Find Usages as a grouped list in the Find window | M, Wave 3 |
| C08 | Structure tool window and File Structure popup (Ctrl+F12) (`ui/structure_view.rs`, `ui/popups.rs` `Dialog::FileStructure`) | `Document.analysis.structure()` (`archipel-ide-language`); cache, flatten, caret item **GUI-ONLY** M13 | **Missing** | Left window `structure` (`JBTree`, kind glyphs); popup is a picker over the flattened rows starting at the caret item | M, Wave 3, M13 |
| C09 | Breadcrumbs: project, directories, file, declarations around the caret (`ui/chrome.rs` `status_bar`) | `structure_view::breadcrumbs` **GUI-ONLY** M13 | **Missing** | Optional row above the editor or the center of the status line | S, Wave 3, M13 |
| C10 | Hierarchy tool window: type and call hierarchy, direction toggle (`ui/hierarchy_ui.rs`) | `core/hierarchy.rs` `request_hierarchy`, `toggle_hierarchy_node`, `toggle_hierarchy_direction`, `open_hierarchy_node` | **Missing**: `hierarchy.` guarded, no window | Left or bottom window with `JBTree`; `D` toggles direction | M, Wave 3, P2 |
| C11 | Search Everywhere: All, Classes, Files, Symbols, Actions, Text (`ui/palette.rs`) | `core/quick_nav.rs` | **Done**: `tui/nav.rs`, `tui/picker.rs` | n/a | n/a |
| C12 | Go to File, Find Action, Go to Line, Recent Files, Switcher, Recent Locations | `core/quick_nav.rs`, `core/goto_line.rs`, `core/recent_popup.rs` | **Done** (Switcher shows Recent Files: a terminal cannot track Ctrl release) | n/a | n/a |
| C13 | Find in Files popup: query, case, words, regex, file mask, scope tabs (Project, Module, Directory, named scopes), custom scopes, preview of the match (`ui/find_popup.rs`) | `core/search_scopes.rs`, `core/find_in_files.rs`, command `search.project`; tabs and params **GUI-ONLY** M15 | **Partial**: `DialogKind::FindInFiles` offers query, match case, words, regex; no mask, no scope, no preview, no custom scopes | Dialog gains a mask field and a scope selector (kit `SegmentedControl`); preview in the picker footer (3 lines) | M, Wave 3, M15 |
| C14 | Find window results and rerun (`ui/toolwindows.rs` `find`) | `search_results`, `search_truncated` | **Partial**: list and open (`tui/panels.rs` `refresh_find`); no "truncated" notice, no group by file, no rerun or toolbar | Group by file with counts; `r` reruns | S, Wave 3 |
| C15 | Project tree: open, toggle, compact packages, group toggles, options menu, git colors, select opened file (`ui/toolwindows.rs` `project_tree`) | `core/project_view.rs`, commands `project.toggleDirectory`, `project.toggleGroup`, `project.toggleCompactPackages` | **Partial**: open, expand, New File, New Directory, Delete (`tests/flows.rs` `the_project_tree_opens_files...`); no VCS colors, no options menu, no select-in-tree | Colors on the label (K `TreeNode.tone`) from `ui/toolwindows.rs` `git_colors` logic (move to core) ; `Alt+F1`-style Select In chord | M, Wave 5, K |
| C16 | Project tree operations: rename, move, cut, copy, paste, safe delete, mark directory, new scratch, new Java class and Kotlin file, reveal, copy path, drag and drop (`ui/context_menus.rs`, `ui/popups.rs`) | `core/project_view.rs` (`project.cutPaths`, `project.paste`, `project.safeDelete`, `project.markDirectory`, `project.newScratch`), `core/project_commands.rs` (`project.rename`, `project.newJavaClass`, `project.newKotlinFile`), `core/new_class.rs`; applicability **GUI-ONLY** M19, M28 | **Missing** (only New File, New Directory, Delete) | Tree context menu (popup chain, A26) plus keys `F6` rename, `Ctrl+X/C/V`, `Alt+Delete` safe delete; Safe Delete lists usages in a dialog | L, Wave 5, M19, M28 |
| C17 | Go to Test, Create Test | `core/test_nav.rs` `go_to_test`, `create_test` | **Partial**: commands run; the choice list needs C02 | picker | S with C02 |
| C18 | Project import, indexes, toolchain checks start on open | `project.open` | **Done** (`docs/tui.md`) | n/a | n/a |

### D. Run, build and test windows

| ID | GUI feature (source) | Core API | TUI status and evidence | Terminal presentation | Effort, deps |
|---|---|---|---|---|---|
| D01 | Run configurations: select, run, stop, rerun (`ui/chrome.rs`) | `core/run_config_commands.rs`, `core/build_commands.rs`; commands `runConfig.select`, `runConfig.run`, `run.cancel`, `run.rerun`; rows **GUI-ONLY** M21 | **Done**: `tui/nav.rs` `config_rows`, `tests/flows.rs` `the_run_configuration_chooser_runs_the_pick_and_stop_ends_a_running_task` | n/a | n/a |
| D02 | Run/Debug Configurations dialog: list, add by type, form per type, duplicate names, delete (`ui/run_config_dialog.rs`) | `core/run_config_commands.rs` (`runConfig.save`, `runConfig.delete`, `runConfig.edit`); `Draft` **GUI-ONLY** M24 | **Missing**: `take_edit_configurations_request` opens the select picker only | Two-pane dialog: list left, form right (labels and `TextFieldControl`, `CheckBoxControl`); `Ctrl+K O`-style chord for Edit Configurations | L, Wave 5, M24 |
| D03 | Run/Build console: output lines, links to source, exit line (`ui/build_window.rs` `console_view`) | `core/build_window.rs` `BuildSnapshot`, `archipel_ide_runner` `ConsoleBuffer`, `BuildEvents` | **Partial**: `tui/panels.rs` `refresh_run` shows lines, a link per build message, the exit line, and a balloon; no URL or `path:line:col` links found in plain text (**GUI-ONLY** M11), no soft wrap toggle, no clear, no scroll-to-end control, no timings | `LogView` with underlined links; toggles by key (`w` wrap, `c` clear, `End` follow) | M, Wave 3, M9, M11 |
| D04 | Build output tree: tasks and messages with timings, tabs per task kind (`build_window::message_node`, `show`) | `core/build_window.rs` `BuildTab`, `BuildSnapshot.events`; `build.showTab` | **Missing**: one flat console | `JBTree` left or a tabs row (Run, Build, Test); durations dim | M, Wave 3, M9 |
| D05 | Test results: suites, cases, filters (hide passed, ignored), rerun failed, stack frames resolve to files (`ui/test_view.rs`) | `archipel_ide_runner` (test events), `test.rerunFailed`; logic **GUI-ONLY** M10 | **Missing** | Tree with `Glyph::Success/Error`, summary line `3 passed, 1 failed`; Enter on a frame opens the file | M, Wave 3, M10 |
| D06 | Run on a file or main from the editor (`project.runFile`, `RunClass`) | `core/gutter_commands.rs` `run_file`, `run.gutter` | **Partial**: `Run` runs the selected configuration; Run Class from the caret is the gutter command (A11) | `Ctrl+K R` | S with A11 |
| D07 | Gradle tool window: projects, tasks, groups, sync, offline toggle, dismiss changes (`ui/gradle_window.rs`, `ui/gradle_banner.rs`) | `core/gradle_commands.rs` (`gradle.runTask`, `gradle.sync`, `gradle.refresh`), `archipel-ide-gradle` `GradleModel`; rows **GUI-ONLY** M29 | **Missing**: sync happens on open, no window | Right or bottom window `gradle`, `JBTree`, Enter runs a task, `s` syncs | M, Wave 6, M29 |
| D08 | Bazel tool window: packages, targets, Build, Run, Test, Sync (`ui/bazel_window.rs`) | `core/bazel_commands.rs` (`bazel.sync`, `bazel.runTarget`, `bazel.targets`); rows **GUI-ONLY** M29 | **Missing** | same | M, Wave 6, M29 |
| D09 | External system progress in the status bar, Cancel (`ui/external_status.rs`) | `core/project_model.rs`, `externalSystem.cancel` | **Missing** | status segment | S, Wave 5, M22 |
| D10 | TODO tool window: current file or project, grouping, filters (`ui/todo_window.rs`) | `core/todo_commands.rs` `todo_view`, `todo_open`, commands `todo.show`, `todo.setGroup`, `todo.setFilter`, `todo.setScope`, `todo.toggleFile`; `entries` **GUI-ONLY** M14 | **Missing**: no window | Bottom window `todo` with a grouped list | S, Wave 6, M14 |
| D11 | Problems window: file groups, severity, options, AI fix (`ui/problems_ui.rs` `tool_window`) | `core/problems.rs` `ProblemIndex`; grouping and columns **GUI-ONLY** M12 | **Partial**: project-wide file groups and rows open the location (`tui/panels.rs` `refresh_problems_list`); no File and Project Errors tabs (`ui/problems_ui.rs` `Tab`), no group by severity, no source-line preview (`Options`), no quick-fix or AI fix action | Tabs row, `s` toggles group by severity, `p` toggles the source preview line, `Alt+Enter` on a row | S, Wave 3, M12 |
| D12 | Terminal window: tabs (new, close, switch), scrollback, restart, paste, clear (`ui/terminal_ui.rs`) | `core/terminal_commands.rs`, `core/terminal_tabs.rs`; commands `terminal.newTab`, `terminal.selectTab`, `terminal.closeTab`, `terminal.restart`, `terminal.scroll`, `terminal.clear`, `terminal.paste`; key bytes **GUI-ONLY** M27 | **Partial**: one session, resize, key forwarding (`tests/flows.rs` `the_terminal_window_runs_a_shell`); no tabs, no scrollback keys, no restart | Tab strip on the window title, `Ctrl+Shift+T` new, wheel and `Shift+PgUp` scroll | M, Wave 6, M27 |
| D13 | AI terminal bar: describe a command, Run, Insert (`ui/ai_terminal.rs`) | `core/ai_terminal.rs` | **Missing** | see J01 | M, Wave 7 |

### E. Git and local history

| ID | GUI feature (source) | Core API | TUI status and evidence | Terminal presentation | Effort, deps |
|---|---|---|---|---|---|
| E01 | Branch and change counts in the status bar | `core/platform.rs` `git_status` | **Done**: `tui/layout.rs` `draw_status` (the counting is duplicated, M22) | n/a | n/a |
| E02 | Commit window: groups Changes and Unversioned, check files, message, Commit, Refresh, Amend, Rollback, Show Diff, AI message (`ui/toolwindows.rs` `commit`) | `core/vcs.rs`, commands `git.commit`, `git.refresh`, `git.rollback`, `git.showDiff`, `ai.vcs.generateCommitMessage`; model **GUI-ONLY** M5 | **Partial**: `tui/panels.rs` `refresh_commit` and `DialogKind::Commit` give groups, check, message, Commit, Refresh (`tests/flows.rs` `the_commit_window_lists_changes_and_commits_with_a_message`); no Amend, Rollback, Show Diff, per-file diff, AI message | Keys in the window: `a` amend toggle, `z` rollback (confirm dialog), `d` or `Enter` diff, `g` AI message | M, Wave 4, M5 |
| E03 | Change popup from the gutter bar: prev, next, Rollback, Show Diff, old lines (`ui/change_popup.rs`) | `core/line_changes.rs` `rollback_change`, `hunk_base_text`, `show_diff_at`, `hunk_at_line`; commands `vcs.rollbackChange`, `vcs.showChangeDiff`, `vcs.nextChange`, `vcs.previousChange`; the layout is `archipel-ide-view` `popups::change` (was **GUI-ONLY** M2) | **Partial**: `tui/gutter.rs` draws the toolbar row and up to 20 `HEAD` lines next to the bar; previous and next move the caret like the keys, Rollback restores the `HEAD` text and closes. Missing: Show Diff is greyed out until the diff viewer (Wave 4); `tests/gutter.rs` `the_change_popup_shows_the_old_lines_and_steps_between_changes_like_the_keys`, `the_change_popup_caps_the_old_lines_at_twenty`, `a_rolled_back_change_loses_its_bar` | Popup anchored right of the bar: toolbar row (`↑ ↓ Rollback Diff ×`) and up to 20 old lines in the editor style; keys Ctrl+Alt+Shift+Up/Down, Ctrl+Alt+Z, Esc; fallbacks section 7 | M, Wave 1, P1, M2 |
| E04 | Diff viewer tab: side by side and unified, word highlights, collapsed unchanged, next and previous change, apply chunk, whitespace and highlight options, sync scroll (`ui/diff_ui.rs`) | `core/vcs.rs` `DiffTab`, `show_diff`, `show_commit_diff`, `stage_hunks`, `apply_chunk`; commands `diff.activate`, `diff.close`, `diff.nextChange`, `diff.previousChange`, `diff.setViewer`, `diff.setWhitespace`, `diff.setHighlight`, `diff.toggleChunk`, `diff.toggleCollapseUnchanged`, `diff.toggleSyncScroll`, `diff.applyChunk`; rows **GUI-ONLY** M6 | **Missing**: `diff.` guarded; `DiffTab` has no drawing | Editor-area tab kind `Diff`: two panes with a middle gutter (`▶`/`◀` apply glyphs) or unified with `-`/`+` lines, word runs reverse-video, folded gaps `⋯ 42 lines`; `NextDiff` (F7) and `PreviousDiff` (Shift+F7) map to `diff.nextChange` and the previous one | L, Wave 4, M6, K two-pane component |
| E05 | Git tool window, Log tab: branches pane, commit list with graph, filters (branch, user, date, path, text), details and changed-files tree, cherry-pick (`ui/git_log_ui.rs`) | `core/git_log.rs` (`GitLogState`, `refresh_git_log`, `select_commit`), commands `git.log.*`, `git.cherryPick`; `archipel_ide_git` `GraphRow`; tree, times **GUI-ONLY** M7 | **Missing**: no `git` window | Bottom window `git` with three columns: refs, commits (graph drawn with `│ ├ ╯` box characters and lane tones), details; filter row | L, Wave 4, M7 |
| E06 | File history (Show History for a file) (`git.fileHistory`) | `core/annotations.rs` `show_file_history` | **Missing** | Opens the Git window filtered by path | S after E05 |
| E07 | Branches popup: checkout, new, rename, delete, merge, rebase, fetch, pull, push, stash, apply, pop, drop, abort operation, progress (`ui/branches_popup.rs`) | `core/branch_commands.rs` (`git.checkout`, `git.newBranch`, `git.renameBranch`, `git.deleteBranch`, `git.merge`, `git.rebase`, `git.fetch`, `git.pull`, `git.stash*`, `git.abortOperation`), `archipel_ide_core::abort_label`; menu **GUI-ONLY** M8 | **Missing**: the status segment opens Commit instead | Click or `Ctrl+K V` on the branch segment opens a popup chain: actions, local and remote branches, each with a submenu | M, Wave 4, M8 |
| E08 | Push dialog: outgoing commits, remote, force with lease (`ui/push_dialog.rs`) | `core/remote_commands.rs` `push_plan`, `git.push` (`archipel_ide_git::PushPlan`) | **Missing** | Dialog with a commit list and options; `Ctrl+Shift+K` | M, Wave 4 |
| E09 | Clone (Get from Version Control) with progress (`ui/clone_dialog.rs`) | `core/welcome_commands.rs` `project.clone`, `clone_progress` | **Missing** | Dialog with URL and directory, progress line | S, Wave 5 |
| E10 | Rollback confirmation (`GitDialog::Rollback`) | `git.rollback` | **Missing** | Confirm dialog listing files | S with E02 |
| E11 | Local History dialog: revisions, diff against current, revert, label (`ui/history_ui.rs`) | `core/history_commands.rs` (`history.show`, `.select`, `.compare`, `.revert`, `.label`, `.project`, `.close`) | **Missing**: `history.` guarded | Dialog: revision list left, diff right (uses E04 renderer) | M, Wave 4, E04 |
| E12 | Remote operation progress and errors (`remote_busy`, `remote_progress`) | `core/remote_commands.rs` | **Missing** | status segment and balloon | S with E07 |
| E13 | VCS colors on files in the tree and tabs (`ui/toolwindows.rs` `git_colors`, `change_color`) | `core/vcs.rs` `git_status`; mapping **GUI-ONLY** M28 | **Missing** | see C15 | S, K |

### F. Tool window inventory (core `ToolWindow`)

| ID | Window (GUI) | Core API | TUI status and evidence | Terminal presentation | Effort, deps |
|---|---|---|---|---|---|
| F01 | Project (`ui/toolwindows.rs`) | `core/project_view.rs` | **Partial**: C15, C16 | left | see C15 |
| F02 | Commit | `core/vcs.rs` | **Partial**: E02 | left | see E02 |
| F03 | Structure | see C08 | **Missing** | left | see C08 |
| F04 | Find | `core/search_scopes.rs` | **Partial**: C13, C14 | bottom | see C13 |
| F05 | Build (Run, Build, Test tabs) | `core/build_window.rs` | **Partial**: D03 to D05 | bottom | see D03 |
| F06 | Problems | `core/problems.rs` | **Partial**: D11 | bottom | see D11 |
| F07 | Terminal | `core/terminal_commands.rs` | **Partial**: D12 | bottom | see D12 |
| F08 | Debug | `core/debug_commands.rs` | **Missing**: B03 | bottom | see B03 |
| F09 | Git (Log) | `core/git_log.rs` | **Missing**: E05 | bottom | see E05 |
| F10 | Hierarchy | `core/hierarchy.rs` | **Missing**: C10 | left or bottom | see C10 |
| F11 | Todo | `core/todo_commands.rs` | **Missing**: D10 | bottom | see D10 |
| F12 | Gradle | `core/gradle_commands.rs` | **Missing**: D07 | bottom | see D07 |
| F13 | Bazel | `core/bazel_commands.rs` | **Missing**: D08 | bottom | see D08 |
| F14 | AI Chat | `core/ai_commands.rs` | **Missing**: J01 | right (a right anchor in the workbench) | see J01 |
| F15 | Agents (compact list) | `core/agents.rs` | **Missing**: J06 | left | see J06 |
| F16 | Device Manager | `core/android_devices.rs` | **Missing**: J07 | bottom | see J07 |
| F17 | Logcat | `core/android_logcat.rs` | **Missing**: J08 | bottom | see J08 |
| F18 | Running Devices | `core/android_deploy.rs` | **Missing**: J09 | bottom | see J09 |
| F19 | Build Variants | `core/android_variants.rs` | **Missing**: J10 | bottom | see J10 |
| F20 | Resource Manager | `core/android_resources.rs` | **Missing**: J11 | bottom | see J11 |
| F21 | Plugin tool windows (right anchored) (`ui/plugin_ui.rs`) | `core/plugin_host.rs`, `plugin.toolwindow.toggle` | **Missing**: J05 | right | see J05 |

### G. Dialogs, settings and project lifecycle

| ID | GUI feature (source) | Core API | TUI status and evidence | Terminal presentation | Effort, deps |
|---|---|---|---|---|---|
| G01 | Settings dialog: search, page tree, breadcrumb, OK, Cancel, Apply, reset page (`ui/settings_dialog.rs`) | `archipel-ide-settings` (`SettingsRegistry`, `SettingsDraft`, `PageDef`, `SettingDef`), commands `settings.get`, `settings.set`, `settings.apply`, `settings.reset`, `settings.search`; selection and search **GUI-ONLY** M26 | **Missing**: `settings.open` sets the status "Settings are not available in the terminal" (`tui/app.rs` `process_requests`) | Full-screen dialog: tree left, rows right (`CheckBoxControl`, `SegmentedControl`, `TextFieldControl`, `ComboBoxControl`), search field on top | L, Wave 5, M26 |
| G02 | Settings: Keymap page (search by name or shortcut, edit, conflicts) (`ui/settings_keymap.rs`) | `core/keymap_commands.rs` `KeymapState`, `keymap.bind`, `keymap.unbind`, `keymap.reset` | **Missing** (the TUI reads the keymap but cannot edit it) | list with a "press the keys" capture; terminal keys that cannot be sent are flagged | M, Wave 5 |
| G03 | Settings: Live Templates, Inspections, File and Code Templates, VCS mappings, Plugins, Updates (`ui/settings_live_templates.rs`, `ui/inspections_settings.rs`, `ui/settings_pages.rs`, `ui/plugin_manager.rs`) | `core/live_templates.rs` (`template.save`, `template.delete`, `template.setEnabled`), `inspection.profile.*`, `core/plugin_host.rs` | **Missing** | one dialog page each, same controls | L, Wave 5 |
| G04 | New Project wizard with generators (Java, Kotlin, Rust, Android, Spring with dependency picker) (`ui/new_project.rs`, `ui/new_project/spring.rs`) | `core/project_commands.rs` and `core/spring_commands.rs` (`project.create`, `spring.catalog.refresh`); `Generator`, `Preset`, `Form` **GUI-ONLY** M25 | **Missing** (the TUI opens an existing folder only) | Wizard dialog: generator list, form, Spring picker as a two-list transfer | L, Wave 5, M25 |
| G05 | Welcome window: recent projects with search, New, Open, Get from VCS, remove from recents (`ui/welcome.rs`) | `core/platform.rs` `recent_projects`, `project.removeRecent` | **Missing**: the dashboard (`tui/layout.rs` `dashboard`) has 5 buttons and no recent list (`Ide::start` loads the list but nothing shows it) | Dashboard gains a recent projects list; `Enter` opens | S, Wave 5 |
| G06 | Open Project dialog | `project.open` | **Done**: `DialogKind::OpenProject` path field (the GUI falls back to the same field when no native dialog exists) | n/a | n/a |
| G07 | Go to Line, New File, New Directory, Delete | commands | **Done**: `tui/actions.rs`, `tui/app.rs` `run_tui_action` | n/a | n/a |
| G08 | First start "Customize Archipel IDE": theme, keymap, import (`ui/first_run.rs`) | `core/first_run.rs` (`app.completeCustomize`, `app.skipCustomize`, `app.customize`) | **Missing** | First-run dialog with theme and keymap selectors | S, Wave 5 |
| G09 | About, crash report notice, update balloon and installer (`ui/about.rs`, `ui/crash_notice.rs`, `ui/notifications.rs`) | `core/update_commands.rs`, `core/diagnostics.rs` (`unseen_crash_reports`) | **Missing** | Balloons; About as a dialog | S, Wave 5, M23 |
| G10 | Project close and manage projects | `project.close` | **Partial**: reachable through Find Action; no welcome to return to | with G05 | S |

### H. Chrome: menus, toolbar, status bar, notices

| ID | GUI feature (source) | Core API | TUI status and evidence | Terminal presentation | Effort, deps |
|---|---|---|---|---|---|
| H01 | Main menu (File, Edit, View, Navigate, Code, Refactor, Build, Run, Tools, VCS, Window, Help) (`ui/main_menu.rs`) | `archipel-ide-actions` `Catalog::shared_menu("MainMenu")`, `filter_menu`; composition **GUI-ONLY** M20 | **Missing** (old design: no menu; being replaced by TUI-GUI-035..045) | Hamburger at toolbar column 1 expands the same menu bar as the GUI; `F10` or `Alt+mnemonic`; mnemonics, shortcut column, disabled entries greyed, submenus (kit `MenuBar` + `PopupChain`) | M, **Wave 0**, M20 |
| H02 | Toolbar: project widget, VCS widget, run widget (config list, Run, Debug, Stop), device selector, Search Everywhere (`ui/chrome.rs` `main_toolbar`) | `core/run_config_commands.rs`, `core/vcs.rs` | **Missing** (old design: status line segments instead of a toolbar; being replaced by TUI-GUI-020..028) | Row 0 toolbar exactly as the GUI: hamburger, project, VCS, mode switch, run configuration, Run, Build, Debug, Stop, Search Everywhere, Settings; all popups from the shared model | M, **Wave 0**, M19, M21 |
| H03 | Status bar widgets: breadcrumbs, plugin and AI widgets, language server, indexing, toolchain, external system, indent, encoding, line separator, caret and selection size, read-only, column mode, update, spinner, message (`ui/chrome.rs` `status_bar`) | `core/indexing_commands.rs` `index_status`, `core/platform.rs` `update_state`, `toolchains`, `archipel_ide_lsp`; labels **GUI-ONLY** M22 | **Partial**: mode, file, branch and counts, message, indexing and toolchain progress as busy text, problem counts, first language server, run configuration, project, `Ln, Col` (`tui/layout.rs` `draw_status`); missing indent, encoding, line separator, selection size, read-only, column mode, update, multiple servers, breadcrumbs | The GUI's widget order and set (TUI-GUI-105), no mode, no file, no Git counts; hide low priority ones under 100 columns | S, **Wave 0**, M22 |
| H04 | Language server status and notices: starting, missing server, restart, dismiss, toolchain setup (`ui/lsp_status.rs`) | `core/lsp_commands.rs` (`language.restartServers`, `language.dismissNotice`), `core/toolchain_commands.rs` (`toolchain.setup`); `summary` **GUI-ONLY** M22 | **Partial**: language name only; no notices or actions | Balloons with action buttons (kit `Balloons`) | S, Wave 5 |
| H05 | Banners: toolchain missing, Gradle changes pending, indexing in popups (`ui/toolchain_banner.rs`, `ui/gradle_banner.rs`, `ui/index_status.rs` `banner`) | `core/toolchain_commands.rs` (`toolchain.banner`, `toolchain.dismissBanner`, `toolchain.install`), `core/gradle_commands.rs` (`gradle.loadChanges`, `gradle.dismissChanges`) | **Missing** | `InlineBanner` above the editor with buttons (reuse A29) | S, Wave 5, M23 |
| H06 | Balloon notifications (run finished, agents, update) | `core/services/notifications.rs` `Notifications` | **Partial**: `tui/panels.rs` `run_finished_notice` only; the core service is never polled | Poll the service into `Balloons` | S, Wave 5, M23 |
| H07 | Theme (**Wave 0**, TUI-GUI-120): follows the Settings theme, project color gradient (`ui/theme_engine.rs`, `ui/toolbar_gradient.rs`) | `archipel_ide_settings` `ids` | **Partial** (`docs/tui.md`: the tab line sun button cycles kit themes; no link to the setting) | Read the theme id at start | S, Wave 5 |
| H08 | Window title: project, file (`chrome::window_title`) | **GUI-ONLY** M22 | **Missing** | Set the terminal title with OSC 2 | S, Wave 5 |
| H10 | Keymap resolution and chords | `archipel-ide-keymap`, `core/keymap_commands.rs` | **Done**: `tui/keys.rs`, tests in `tui/keys.rs` | n/a | n/a |

### J. Plugins, AI, Agents and Android

| ID | GUI feature (source) | Core API | TUI status and evidence | Terminal presentation | Effort, deps |
|---|---|---|---|---|---|
| J01 | AI Chat tool window: send, stop, new, context, apply with diff preview, copy and insert code, configure (`ui/ai_chat.rs`) | `core/ai_commands.rs` (`ai.chat.*`, `ai.configure`), `archipel-ide-ai` | **Missing**: `ai.` guarded | Right window `aiChat`: transcript (`LogView` with markdown), input at the bottom, apply opens the diff (E04) | L, Wave 7, E04 |
| J02 | AI agent mode in the chat window: goal, steps, approvals with diffs (`ui/ai_agent.rs`) | `core/ai_agent.rs` (`ai.agent.*`) | **Missing** | step list, approval prompt with `y`/`n` | M, Wave 7 |
| J03 | AI editor actions (explain, refactor, fix problems, fix build) and AI commit message (`ui/ai_actions.rs`, `ui/ai_vcs.rs`) | `core/ai_actions.rs`, `core/ai_vcs.rs` | **Missing** | entries in the context menu (A26), `g` in the Commit window | S, Wave 7 |
| J04 | Plugin commands | `core/plugin_host.rs`, `tui/ide.rs` command sync | **Done** (`docs/tui.md`: "plugin commands work") | n/a | n/a |
| J05 | Plugin views: tool windows, editor tabs, status widgets, menus, plugin settings (`ui/plugin_ui.rs`, `ui/plugin_manager.rs`) | `core/plugin_host.rs`; views render into `egui::Ui` through `archipel_plugin_ui::ViewFrame` (the SDK crate says "egui side") | **Missing** (`docs/tui.md`; `plugin.manager` guarded) | Needs a toolkit-neutral view description in the SDK before a terminal can draw plugin views; until then list plugins and toggle them in a dialog and show plugin status widgets as text | L, Wave 7, SDK change |
| J06 | Agents mode: second workbench arrangement, threads, composer, review of an agent's diff, balloons (`ui/agents_ui.rs`, `ui/agents_ui/*`) | `core/agents.rs` (`agents.*`, `workbench.setMode`, `workbench.toggleMode`) | **Missing**: `agents.` guarded | A full-screen mode: thread list left, terminal pane or review center, composer bottom | L, Wave 7, E04 |
| J07 | Device Manager and device selector (`ui/device_manager.rs`) | `core/android_devices.rs` (`android.devices.*`, `android.avd.*`, `android.emulator.*`); rows **GUI-ONLY** M30 | **Missing**: `android.` guarded | table (`JBTable`), toolbar | M, Wave 7 |
| J08 | Logcat: tabs, filters, favourites, pause, soft wrap, level colors (`ui/logcat_ui.rs`) | `core/android_logcat.rs` (`android.logcat.*`); colors **GUI-ONLY** M30 | **Missing** | `LogView` with level tones, tag color by hash | M, Wave 7 |
| J09 | Running Devices, run on device, mirror (`ui/running_devices_ui.rs`, `ui/android_run_ui.rs`) | `core/android_deploy.rs` (`android.app.run`, `android.mirror.*`) | **Missing** | run and stop on a device; the mirror stays GUI-only (pixels) | M, Wave 7 |
| J10 | Build Variants (`ui/build_variants_ui.rs`) | `core/android_variants.rs` | **Missing** | list | S, Wave 7 |
| J11 | Resource Manager (previews of drawables) (`ui/resource_manager_ui.rs`) | `core/android_resources.rs` | **Missing** | names and sizes only; previews are not terminal content (kitty graphics later) | M, Wave 7 |
| J12 | SDK Manager (platforms, tools, remote packages, update sites) (`ui/android_sdk.rs`) | `core/android_sdk_repo.rs`; row builders **GUI-ONLY** M30 | **Missing** | settings page (G03) | M, Wave 7 |
| J13 | Merged manifest view, layout editor (design, split, code), Android project view (`ui/merged_manifest_ui.rs`, `ui/layout_editor_ui.rs`, `ui/android_view.rs`) | `core/android_manifest.rs`, `core/android_tools.rs`, `core/android_commands.rs` | **Missing**; the layout design surface is a graphic preview | manifest as text tabs; layout editor stays GUI-only (no terminal equivalent) | M, Wave 7 |
| J14 | Toolchain install progress and setup | `core/toolchain_commands.rs` | **Partial**: progress in the status line, install runs on open (`Ide::start` `toolchain_setup`); no banner (H05) | see H05 | S |

Not applicable to a terminal and left out of the counts: tool window stripes (no stripes by design,
`docs/tui.md`) and the UI control socket (`ui/control.rs`, a GUI test hook; the TUI's equivalent is
the `TestBackend` harness). Rows J05 (egui-bound plugin views) and J11 and J13 (graphic previews)
are the ones where a terminal cannot fully reach the GUI for a structural reason; everything else is
presentation work.

### K. Not in the GUI either (not counted)

| Item | State in the repo | Note |
|---|---|---|
| Bookmarks (toggle, list, mnemonics, gutter mark) | model only: `core/recent_popup.rs` `Bookmark`, no command, no UI | Build the core command and one UI model first; the kit has `Glyph::Bookmark` |
| Sticky lines (scope headers) | none | New in both |
| Override and implement gutter markers | `archipel-ide-daemon/src/passes.rs` `LineMarker`, `LineMarkerProvider`; the editor does not paint them | Lane icons from the daemon pass; both frontends |
| Generate dialog | no dialog; the popup is the code-actions list (C04) | Done once C04 is |
| Replace in Files | `core/search_replace.rs`, commands `search.replaceInFiles`, `search.replacePreview`; no UI | Both |
| View Breakpoints dialog, exception and method breakpoints, disabled breakpoints | `BreakpointOptions` has condition, log message, hit count only | Both; the gutter glyph set already needs shapes for them |
| Merge conflicts, shelf | specs in `docs/specs/vcs/05`, `06`; no UI | Both |
| Maven tool window, Services tool window | none; Maven is imported through the project model | Both |
| Code coverage | disabled menu entry "Run with Coverage" (`ui/gutter_popup.rs`, IJ-RUN-069) | Both |
| Inspect Code results view | `core/inspection_commands.rs` (`inspection.runCode`, `inspection.results`); no view | Both |

## 5. GUI-only logic that must move into the core first

A feature built twice drifts. These pieces hold behavior (not painting) inside `archipel-ide-ui`.
Each move is "extract, keep the tests, make the GUI call the core, then build the TUI on it". The
target is a module of `crates/archipel-ide-core/src/` unless stated. `archipel-ide-ui/tests/no_subsystem_reads.rs`
and `archipel-ide-core/tests/ide_state_size.rs` (see `AGENTS.md`) constrain how: new state is a
service of the platform kernel, not a field of `IdeState`.

| ID | GUI-only logic (source) | Target module | Duplicated in TUI today | Needed by |
|---|---|---|---|---|
| M1 | Gutter model: which marks a line has, hit testing (number column toggles a breakpoint, icon lane opens the run popup, change bar opens the change popup, hover preview), run popup entries and enabled rules (`ui/editor.rs` `GutterData`, `GutterLayout`, request handling; `ui/gutter_popup.rs`); the 0.25 s debounce of run icons (`RUN_ICONS_STALE_FOR`) | new `gutter_model.rs` over `gutter_commands.rs`, `debug_commands.rs`, `line_changes.rs` | partly: `archipel-ide-view` `popups::gutter` (run icons debounce, run popup rows, breakpoint kinds); the hit routing is `tui/gutter.rs` over the kit's `gutter_clicks` | A09 to A14, A19 |
| M2 | Change popup: neighbour hunks, clipped old lines (20), hunk kind to marker kind (`ui/change_popup.rs`) | `line_changes.rs` | yes: `archipel-ide-view` `popups::change`; the GUI's `change_popup.rs` draws it too | E03 |
| M3 | Blame tooltip text, age shading buckets, relative and absolute time formats (`ui/blame_gutter.rs`, `ui/git_log_ui.rs` `relative_time`, `absolute_time`) | `annotations.rs`, `git_log.rs` | no | A16, E05 |
| M4 | Debug view model: status text, frame label, variables flattened with the expanded set, watch editing state, breakpoint form fields (`ui/debug_window.rs`, `ui/debug_dialogs.rs`) | new `debug_view.rs` | partly: the breakpoint form is `archipel-ide-view` `popups::breakpoint` (GUI and terminal); the debug window model waits for Wave 2 | B03 to B09, A13 |
| M5 | Commit model: groups, project-relative path mapping, included-by-default rule, overrides, amend, message (`ui/toolwindows.rs` `commit`; state in `IdeApp.commit_overrides`, `commit_amend`, `commit_message`) | `vcs.rs` `CommitModel` | yes: `tui/panels.rs` `refresh_commit`, `Panels.included` | E02, E10 |
| M6 | Diff rows: unified rows, folding of long unchanged runs with context, side-by-side alignment and fold map, labels (`ui/diff_ui.rs` `unified_rows`, `folds`, `fold_rows`, `FoldMap`, `line_starts`, `base_label`, `new_label`) | `vcs.rs` `DiffTab` methods | no | E04, E11, J01, J06 |
| M7 | Git log presentation: changed files as a folder tree, graph width, lane color index (`ui/git_log_ui.rs` `FileTree`, `tree_rows`, `row_width`, `lane_color`) | `git_log.rs` | no | E05 |
| M8 | Branches popup composition: repository actions, per-branch submenus, labels and enabled rules (`ui/branches_popup.rs`) | `branch_commands.rs` | no | E07 |
| M9 | Run and build view model: message tree, durations (`format_duration`), run subject, project-relative paths, stdout and stderr classification, console options (`ui/build_window.rs`) | `build_window.rs` | yes: `tui/panels.rs` `refresh_run` (tone from text, link resolution) | D03, D04 |
| M10 | Test results model: summary, visibility filters, suite short names, stack frame parsing and source lookup (`ui/test_view.rs` `summary`, `visible`, `short_suite`, `parse_frame`, `find_source`) | new `test_results.rs` (or `archipel-ide-runner`) | no | D05 |
| M11 | Console links: URLs and `path:line:col` in output (`core/console_links.rs` `find`, `IdeState::resolve_console_path`, with tests) | moved from `ui/console_links.rs` | yes: `tui/panels.rs` `refresh_run` opens relative locations | D03 |
| M12 | Problems grouping: columns on a line, problems at a position, file groups, path eliding, tab and option filters (`ui/problems_ui.rs`) | `problems.rs` | yes: `tui/panels.rs` `refresh_problems_list` | D11 |
| M13 | Structure: items with the stale-while-typing cache, flatten, item at caret, breadcrumbs, crumb labels, kind to icon key, File Structure filtering (`ui/structure_view.rs`, `ui/popups.rs`) | new `structure.rs` | no | C08, C09 |
| M14 | Todo entries by grouping and collapsed set (`ui/todo_window.rs` `entries`) | `todo_commands.rs` | no | D10 |
| M15 | Find in Files model: scope tabs and named scopes, search params, result location text, preview window (`ui/find_popup.rs` `TABS`, `CUSTOM_SCOPES`, `run_search`, `preview_job`; `ui/toolwindows.rs` `find_params`, `result_location`) | `search_scopes.rs`, `find_in_files.rs` | partly: `tui/app.rs` `DialogKind::FindInFiles` params | C13, C14 |
| M16 | In-file find: the GUI bar uses `ops::find_all` and its own match cache; the TUI uses the kit widget's find; neither uses the spec model `find_session.rs` `FindSession` | adopt `find_session.rs` in `ui/findbar.rs` and in the `tui/view.rs` bridge | yes (two implementations) | A08 |
| M17 | Completion rows (label with matched positions, detail, kind glyph, indexing notice) and documentation blocks: markdown to blocks (`ui/completion_ui.rs` `row_for`, `kind_glyph`, `indexing_notice`; `ui/doc_ui.rs` `blocks`, `signature_job`) | `completion.rs` (`CompletionRow`), `archipel-ide-markdown` (block model for docs) | yes: `tui/popups.rs` `kind_letter` and row building | A21 to A23 |
| M18 | Intention rows: grouping, expansion, rest delay before requesting (`ui/intentions_ui.rs` `display_rows`, `request_when_resting`) | `intentions.rs` / `code_actions.rs` | yes: `tui/popups.rs` `intention_rows` | A19, A20 |
| M19 | Action applicability and surface requests: context menu rules, context facts, `UiAction` set, which reference actions are implemented (`ui/context_menus.rs` `applies`, `Model`; `ui/actions.rs` `context_rule`, `ContextFacts`, `is_target_action`; `ui/keybindings.rs` `UiAction`, `ui_action`, `is_implemented`) | `ide_actions.rs` and `archipel-ide-actions` (a shared `WorkbenchRequest` enum for `ActionEffect::Request`) | yes: `tui/actions.rs` `TuiAction` | A26, C16, H01 |
| M20 | Main menu composition: hidden entries, extra command categories, New Project entry, plugin menu titles (`ui/main_menu.rs` `main_menu_items`, `HIDDEN_AT_RUNTIME`, `EXTRA_CATEGORIES`, `extras`, `with_new_project`) | `archipel-ide-actions` or `ide_actions.rs` | no | H01 |
| M21 | Run widget rows: grouping and ordering of configurations, row actions (`ui/chrome.rs` `run_widget`, `run_row`, `RowAction`) | `run_config_commands.rs` | yes: `tui/nav.rs` `config_rows` | D01, B10 |
| M22 | Status model: indent, line separator, encoding, caret and selection label, read-only, column mode, update label, language server summary, index summary, window title (`ui/chrome.rs` `status_bar`, `window_title`; `ui/editor_keys.rs` `caret_position_label`; `ui/lsp_status.rs` `summary`; `ui/index_status.rs` `visible_summary`; `ui/toolchain_status.rs`; `ui/external_status.rs`) | new `status_bar.rs` | yes: `tui/layout.rs` `draw_status` (git counts, index and toolchain texts) | H03, H04, D09, E01 |
| M23 | Banner and notice content: disk conflict text and actions, Gradle and toolchain banners, crash notice, update balloon, LSP notices (`ui/disk_banner.rs` `content`, `ui/gradle_banner.rs`, `ui/toolchain_banner.rs`, `ui/crash_notice.rs`, `ui/notifications.rs`, `ui/lsp_status.rs` `notices`) | `services/notifications.rs`, `disk_sync.rs` | no | A29, H05, H06, G09 |
| M24 | Run configuration form: draft from and to configuration, field table per type, unique names (`ui/run_config_dialog.rs` `Draft`, `fields`, `unique_name`) | `run_config_commands.rs` | no | D02 |
| M25 | New Project generators, presets, form and Spring selection (`ui/new_project.rs` `Generator`, `Preset`, `Form`; `ui/new_project/spring.rs`) | `project_commands.rs`, `spring_commands.rs`, `archipel-ide-templates` | no | G04 |
| M26 | Settings dialog state: page selection with history and expansion, first search hit, setting row to control mapping, idle seconds parsing (`ui/settings_dialog.rs` `SettingsDialog`, `first_hit`, `idle_seconds_from_text`) | `archipel-ide-settings` (dialog model) | no | G01 to G03 |
| M27 | Terminal key encoding (`ui/terminal_ui.rs` `key_sequence`) and `tui/keys.rs` `key_bytes` are two implementations | `archipel-ide-terminal` with a toolkit-neutral key type | yes | D12 |
| M28 | Project tree helpers: new entry parent, VCS color mapping, move target, dialogs' rules (`ui/toolwindows.rs` `new_entry_parent`, `git_colors`, `change_tone`, `move_entry`; `ui/popups.rs` dialogs) | `project_view.rs`, `project_commands.rs` | yes: `tui/app.rs` `new_entry_parent` | C15, C16, E13 |
| M29 | Gradle and Bazel tree rows with filter and expansion (`ui/gradle_window.rs` `rows`, `project_rows`; `ui/bazel_window.rs` `rows`) | `gradle_commands.rs`, `bazel_commands.rs` | no | D07, D08 |
| M30 | Android row builders and colors: SDK package rows and filters, device rows, selector label, logcat level and tag colors (`ui/android_sdk.rs`, `ui/device_manager.rs`, `ui/logcat_ui.rs`) | `android_sdk_repo.rs`, `android_devices.rs`, `android_logcat.rs` | no | J07, J08, J12 |

Not moved: `ui/folding.rs` `FoldView` (row-to-line mapping with placeholders) is painting support;
the TUI uses the kit widget's own fold model, fed from `core/folds.rs`. `ui/markdown_ui.rs` and
`ui/doc_ui.rs` painting stays in each frontend; only the block model moves (M17).

## 6. Plan in waves

Each wave ends with everything green on `cargo fmt --all --check`, clippy and `cargo test --workspace`
(the repository checks in `AGENTS.md`), the guard in `tui/app.rs` `unsupported()` trimmed by exactly the
ids the wave delivers, `docs/tui.md` "What works" and "Known limitations" updated, and a
`docs/parity` note when a spec case is replayed.

Test kinds used below: **cells** means a `TestBackend` test in `crates/archipel-ide-tui/tests/`
using `tests/common/mod.rs` `Tui` (`draw`, `click`, `press`, `fg_at`, `find`, `text`, `wait_for`) to
assert glyphs, colors and layout at cell positions; **flow** means a `tests/flows.rs`-style
scenario against the real core with a temporary Git repository (as the Commit window test does)
or the fake language server `archipel-fake-lsp`; **replay** means a spec-case replay in the manner
of `tests/editor_cases.rs` (same case files, same parser, judgement read from the core).

### Wave 0: the GUI-faithful chrome (first; binding)

Goal: `archipel-ide --tui` shows the same toolbar, main menu, stripes, tool window headers, tab strip,
breadcrumbs, status bar, popups and keys as the GUI, from one shared UI layer. The spec is
`docs/specs/tui/01-gui-faithful-terminal-ui.md` (IDs `TUI-GUI-*`) and
`docs/specs/tui/02-shared-component-layer.md` (IDs `TUI-CMP-*`). Nothing in Waves 1 to 7 is worth building
on the old layout: a gutter nobody can reach through a menu repeats the Go to File failure. Wave 0 has
five phases in the order the product owner set: chrome, popups and lists, tool windows, dialogs
(phases 0.2 to 0.5 of file 02), after the foundations (0.1). Phases 0.1 and 0.2 are the minimum before
Wave 1 starts; Wave 1 (the editor already shares the core document) may then run in parallel with 0.3 to 0.5.
The kit work is a change in `archipel-dev/kit` and a pin bump of `Cargo.lock` each time.

Test kinds as defined in this section's preamble, plus **outline**: the structural comparison of
TUI-CMP-040 (items, order, enabled, checked, shortcuts, labels, rendered through both backends).

#### Phase 0.1 Foundations

| Task | Crates and files | Tests |
|---|---|---|
| 0.1.1 `archipel-kit-model` crate (props and events of the vocabulary, no egui or ratatui), both kits re-export it; `VOCABULARY` list with `PENDING` entries | kit repo: new `crates/archipel-kit-model`, `archipel-kit/src/lib.rs`, `archipel-kit-tui/src/lib.rs` | kit: serde round trip per props type; `vocabulary` test lists pending entries (TUI-CMP-001, 005, 010) |
| 0.1.2 `archipel-ide-view` crate: `ViewCx`, `Chrome::build`, `Outline`, `dispatch`; no UI dependencies | new `crates/archipel-ide-view`, workspace `Cargo.toml` | `no_ui_deps`, purity property test (TUI-CMP-002, 006) |
| 0.1.3 Test infrastructure: outline extraction for both kits (`testing::outline`), `render_parity` skeleton with the fixtures of TUI-CMP-041, `raw_egui_budget` and extended `no_subsystem_reads` | kit `testing.rs` (both), `archipel-ide-view/tests/`, `archipel-ide-ui/tests/` | the tests themselves, budget baseline recorded |
| 0.1.4 `Look::gui(theme)`, 256-color quantizer, `Caps` additions (`truecolor`, `super_reported`, `curly`), `Leaders` | kit-tui `lib.rs`, `palette.rs`, `input.rs`, `runtime.rs` | `theme_tokens` (TUI-GUI-120, 121, 122), `keys_collisions` unit part |
| 0.1.5 Register the spec set | `xtask/src/spec_coverage.rs` `SETS` gets `("TUI", "Terminal UI")` | `cargo xtask spec-coverage` lists TUI |

#### Phase 0.2 Chrome (the product owner's ask)

| Task | Crates and files | Tests |
|---|---|---|
| 0.2.1 Extract M20 main menu composition and M19 action applicability, context menus, surface requests; builders `main_menu`, `context_menu` | core side `archipel-ide-view` (from `ui/main_menu.rs`, `ui/context_menus.rs`, `ui/actions.rs`, `ui/keybindings.rs`, `tui/actions.rs`); GUI switches to it | outline: GUI menu before and after the move identical; `ui_audit` unchanged |
| 0.2.2 Extract the toolbar model: project, VCS, mode switch, run configuration widget and rows (M21), Run, Build, Debug, Stop, Search Everywhere, Settings gear (added to both frontends) | `archipel-ide-view` (from `ui/chrome.rs`); `archipel-ide-ui/src/chrome.rs` renders `ToolbarButton`/`ToolbarDropdown` | outline equality; click flows in the TUI (TUI-GUI-020..026) |
| 0.2.3 Kit: `MenuBar`, `ToolbarDropdown`, tone and badge on toolbar buttons, tooltip with disabled reason | kit-tui `toolbar.rs`, new `menubar.rs`; kit `action_system` | kit conformance tests and gallery entries |
| 0.2.4 TUI row 0 and the menu bar: hamburger at column 1, expansion, F10, Alt mnemonics, hover switching, submenus, shortcut column | `archipel-ide-tui/src/layout.rs`, new `chrome.rs`, `app.rs` | cells at 160x45, 100x30, 80x24; click each widget and compare the opened popup's outline; menu key tests (TUI-GUI-035..045) |
| 0.2.5 Stripes: model (`left_stripe`, `right_stripe_windows`, badges), kit `ToolStripe` groups with separator and badge, TUI stripes with tooltips | `archipel-ide-view`; kit-tui `workbench.rs`; tui `layout.rs` `workbench()` (turn `.stripes(true)` on) | outline equality; click toggles layout state; badge tests (TUI-GUI-055..063) |
| 0.2.6 Tool window headers: model per window (title, actions, view selector, tabs), kit `PanelHeader`, header actions including the full Project set in both frontends, overflow, Hide | `archipel-ide-view` (from `ui/toolwindows.rs`, `ui/widgets.rs`); kit-tui `workbench.rs` | outline over all 20 windows; cells; splitter drag test (TUI-GUI-014, 070..075) |
| 0.2.7 Editor tab strip: model (name, icon, preview, dirty, pinned, errors, tooltip, tab menu), kit `TabBar` props, TUI tabs with close, dot, pin, overflow list, curly underline; keys Alt+Left/Right, Ctrl+F4 | `archipel-ide-view` (from `ui/editor.rs` `tabs`); kit-tui `tabs.rs`, `components.rs` | cells per tab state; flow: close dirty tab asks (TUI-GUI-085..088) |
| 0.2.8 Status bar: model M22 (items in the GUI's order, caret label, read-only, line separator, encoding, indent, update, LSP, index, toolchain, external, window title), kit `StatusBar` item list and `Breadcrumbs` (M13 part), remove mode, file, Git counts and problem counts segments; widget popups; width priorities | `archipel-ide-view` (from `ui/chrome.rs`, `ui/lsp_status.rs`, `ui/index_status.rs`, `ui/structure_view.rs`); kit-tui `statusbar.rs`, new `breadcrumbs.rs`; tui `layout.rs` `draw_status` deleted | outline equality; cells at three widths; a test asserts no `INSERT`/`Ln` text (TUI-GUI-004, 105..109) |
| 0.2.9 Error stripe column, gutter order contract (numbers, run lane, fold, change bar; lane content arrives in Wave 1), editor tick marks and inspection widget | tui `view.rs`, `layout.rs`; kit `editor` | cells; click tick moves the caret (TUI-GUI-090, 093) |
| 0.2.10 Keys: single leader `Ctrl+\`, Appendix B table as data, `display_shortcut` in menus and tooltips, collision test, kitty detection with the flags of section L, double Shift only in kitty mode, one-time notice and `Help > Terminal Keyboard Setup…`, macOS keymap fallback; remove `Ctrl+K`/`Ctrl+W` leaders and `Ctrl+P` Find Action | `archipel-ide-tui/src/keys.rs`, `app.rs`; `archipel-ide-view` (key table, notice text); kit-tui `input.rs` | `keys_collisions`, table-driven key test (TUI-GUI-194), notice shown once, reachability test (TUI-GUI-193) |
| 0.2.11 Theme and glyphs: `Look::gui(theme)` live from Settings, glyph table test, ASCII mode with equal geometry, OSC 2 title | tui `layout.rs`, `lib.rs`; kit-tui | `theme_tokens`, `ascii_geometry` (TUI-GUI-120..132) |
| 0.2.12 Remove the old chrome: mode slot, status segments, theme sun button, dashboard letter keys (replaced by the Welcome screen), the "no toolbar" statements; update `docs/tui.md` | `archipel-ide-tui`, `docs/tui.md` | grep tests; full `chrome_parity` (TUI-GUI-190..192, 200..202) |

Exit of 0.2: `chrome_parity` green for all fixtures at 160x45, 100x30 and 80x24; the mockups of the spec are
generated from the snapshot files; every GUI chrome element of `docs/specs/tui/01-...` sections C to H is
present; the guard in `tui/app.rs` `unsupported()` is unchanged (it goes in 0.3 to 0.5).

#### Phases 0.3 to 0.5 (shared layer migration; plan in file 02 section 4)

| Phase | Content | Exit |
|---|---|---|
| 0.3 Popups and lists | Search Everywhere with all tabs and Find Action (M13, M15), Go to File/Class/Symbol/Line, Recent Files and Switcher, File Structure, Find in Files, completion and docs (M17), Alt+Enter (M18), balloons and banners (M23), Recent Locations; kit `ListRow` extras, `Popover` sizing, `TextField` chips, `Tree` tone | popup `outline` tests; `tui/popups.rs`, `tui/picker.rs`, `tui/nav.rs` reduced to rendering; the Search Everywhere mockup is a snapshot |
| 0.4 Tool windows | the 6 existing windows on the shared models (M5, M9 to M12, M28), then Structure, Git, Debug, Hierarchy, TODO, Gradle, Bazel, AI Chat, Agents, Android windows (M7, M14, M29, M30); the `WINDOWS` const covers `ToolWindow::ALL`; `unsupported()` for windows removed | every stripe button opens a window body; `tui/panels.rs` split by window |
| 0.5 Dialogs | Settings (M26), Run/Debug Configurations (M24), New Project, Clone, Push, Rename and refactor dialogs, Commit dialog, Local History, Welcome, About (M25); kit `PropertyGrid`, `TitledSeparator`; `unsupported()` deleted | dialog `outline` tests; `docs/tui.md` "Known limitations" no longer lists dialogs |

#### Wave 0P: gutter prerequisites (parallel with Wave 0)

The former Wave 0 (P1 to P5); unchanged except that P3 is part of task 0.2.1 and P2 becomes phase 0.4.


| Task | Crates and files | Tests |
|---|---|---|
| P1 Extract M1 and M2 (gutter model, change popup model) | core: new `gutter_model.rs`, `line_changes.rs`; ui: `editor.rs`, `gutter_popup.rs`, `change_popup.rs` call it | core unit tests for hit testing and entries; the GUI's existing `ui_ed_012` tests stay green |
| P2 Tool window plumbing for more windows: delivered by phase 0.4 | see 0.4 | see 0.4 |
| P3 Surface request enum (M19, the part `TuiAction` duplicates): delivered by task 0.2.1 | see 0.2.1 | see 0.2.1 |
| P4 kit-tui: reserve N gutter lanes (`EditorComponent::lanes(n)`), glyphs `BreakpointConditional`, `BreakpointDisabled`, `Logpoint`, `ExecutionArrow`, `Lightbulb` (fallbacks exist: `Breakpoint` with tones, `Thinking`), line background tint (`LineHighlight`), `TreeNode.tone`, end-of-line text and inline hints, tab stop ranges | `archipel-dev/kit` (`crates/archipel-kit-tui/src/editor`, `glyphs.rs`, `collections.rs`) then bump the pin in `Cargo.lock` | kit's own tests; IDE cells tests pick them up |
| P5 Spec case adapter for execution cases: `docs/specs/execution/cases/*.cases.md` have prose "after" blocks (`line 3 has a line breakpoint (red circle), enabled`) and no runner exists (`crates/archipel-ide-core/tests` replays only the editor and search cases). Add machine-readable lines to the case format with the spec team (for example `breakpoints: 3`, `gutter: 3 run`), then a core runner and the TUI replay | docs/specs, core `tests/support`, tui `tests/execution_cases.rs` | the runner itself |

### Wave 1: editor gutter, breakpoints, change markers

Starts after phase 0.2 (the gutter column order and the error stripe are fixed there, TUI-GUI-090). Every gutter
function must also be reachable through the GUI's visible paths (gutter popup, Run menu), not only by the
chords listed in task 1.8; those chords become `Ctrl+\` alternatives (spec section L).

Goal: the terminal editor shows the same gutter marks as the GUI and runs and debugs from them.
Two lanes (breakpoint or execution arrow in lane 0, run icon and lightbulb in lane 1) need P4's reserved
lanes. Until P4 lands, ship one lane with the priority execution arrow, breakpoint, run icon, so
nothing waits on the kit.

| Task | Crates and files | Tests |
|---|---|---|
| 1.1 Change bars (A09): `DocView::refresh_decorations` gets `change_markers` from `state.document_changes(index)`, cached by `(revision, git status Arc pointer)`; map `HunkKind` to `ChangeKind` | `tui/view.rs`, `tui/layout.rs` (`.change_markers(..)`) | cells: temporary Git repo, edit a file: `▎` at the change column on the added line in the palette green, on a modified line in blue, `▔` red after a deleted line; marker moves when lines are inserted above |
| 1.2 Run and test icons (A10): `state.cached_gutter_marks()` and `gutter_marks()` after a 0.25 s pause, lane 1, `Tone::Run`, accessible names | `tui/view.rs`, `tui/layout.rs` | cells: Rust and Java fixture with `main` and a `#[test]`: icon on the right lines, none elsewhere, text does not shift when the icon appears (lane reserved) |
| 1.3 Gutter clicks (A11, A12, E03): read `EditorOutput.gutter_clicks` in `forward_to_editor_widget`; route by `GutterArea`; line number click toggles a breakpoint; icon click opens the run popup; change bar click opens the change popup; right click on a number opens the breakpoint form | `tui/app.rs`, new `tui/gutter.rs` | flow: click on a number toggles `debug.toggleBreakpoint` and writes `.archipel/breakpoints.json`; second click removes it; replay of EXE-DBG-010 cases 1 and 2 (Ctrl+F8 toggles and removes) |
| 1.4 Breakpoint display (A12, A13): lane 0 `●` red for line, orange for conditional, dim for logpoint from `breakpoint_lines(path)` plus `breakpoint_options`; hover preview | `tui/view.rs`, `tui/gutter.rs` | cells: glyph and `fg_at`; the dim preview follows the pointer |
| 1.5 Run popup: Run X, Debug X, Run with Coverage (disabled), Modify Run Configuration | `tui/gutter.rs` (kit `PopupChain`) | cells: popup rows and shortcut column; flow: Modify Run Configuration selects the configuration without starting a run; Debug stays disabled while running |
| 1.6 Breakpoint properties dialog: three fields, Done, Remove | `tui/dialogs.rs` (multi-field content), `tui/gutter.rs` | flow: set a condition, `.archipel/debug-state.json` has it, marker turns orange; Remove clears both |
| 1.7 Change popup: toolbar row, up to 20 old lines, previous and next hunk, Rollback (`vcs.rollbackChange`), Show Diff (disabled until Wave 4) | `tui/gutter.rs` | flow: Rollback restores the HEAD text and the marker disappears; cells: popup content equals `hunk_base_text`; previous and next move the caret as `vcs.nextChange` does |
| 1.8 Keys: Ctrl+F8, Ctrl+Alt+Z, Ctrl+Alt+Shift+Up and Down, plus the chords of section 7; `unsupported()` stops refusing `debug.toggleBreakpoint`, `debug.editBreakpoint`, `debug.setBreakpointOptions`, `debug.closeBreakpointEditor` (the rest of `debug.` stays guarded until Wave 2; `run.gutter` and `vcs.*` are not guarded today) | `tui/keys.rs`, `tui/app.rs` | keymap unit test: every listed action id has a terminal-sendable binding; which-key list shows them |

Exit: `docs/tui.md` loses the gutter and breakpoints limitation; rows A09 to A13, E03 are Done except
Show Diff.

Status: delivered (tasks 1.1 to 1.8). Two lanes were possible without waiting for the rest of P4 because
the kit gained `EditorComponent::lanes(n)` (reserved lanes) and `EditorOutput.gutter_hover` (the cell under
the pointer, for the breakpoint preview); the glyph shapes, the line tint and the end-of-line text of P4
are still open, so a conditional breakpoint and a logpoint differ by tone, not by shape. A09, A10, A12 and
A13 are Done; A11 and E03 are Partial (Debug from the popup and Show Diff wait for Waves 2 and 4, and the
run popup has no key for the caret line). Not delivered from the plan's wording: `Glyph::Test` for tests
(the GUI's `Run` and `Rerun` shapes are used), the `Ctrl+K` chords (the leader is `Ctrl+\`: `P` toggles,
`shift X` edits, `alt Z` rolls back, `.` and `,` step through changes).

### Wave 2: debugger

| Task | Crates and files | Tests |
|---|---|---|
| 2.1 Extract M4 (debug view model) | core `debug_view.rs`; ui `debug_window.rs`, `debug_dialogs.rs` | core unit tests (status text, expanded set, frame labels) |
| 2.2 Debug window `debug` (Alt+5): toolbar, status line, frames list, variables tree with lazy `debug.loadVariables`, watches, evaluate field, Console tab | tui new `debug.rs`; `WINDOWS`; `layout.rs` | cells: window layout with a `Session` fed scripted DAP messages through `Session::handle` (`archipel-ide-dap/src/session.rs`; `crates/archipel-ide-dap/tests/` and `crates/archipel-ide-core/tests/java_debug_flow.rs` show real-adapter setups); flow: expanding a variable sends `debug.loadVariables` |
| 2.3 Stepping and session keys: F7, F8, Shift+F8, F9, Alt+F9, Alt+F8, Shift+F9, Ctrl+F2; toolbar enabled state from `action_enabled`; remove the `debug.` guard | `tui/keys.rs`, `tui/app.rs` | keymap test; flow against `java_debug_flow`-style fixture if a JDK is available (skipped otherwise, like the fake LSP tests) |
| 2.4 Execution point: arrow and tint in the editor, reveal the frame's file on pause (A14), inline values (A15 with the P4 end-of-line text) | `tui/view.rs` | cells: arrow glyph at the paused line; value text at the line end; replay of EXE-DBG-041 to 049 (stepping and frames) via P5 |
| 2.5 Debug and Stop in the run segment (B10), `Debugging <name>` status | `tui/layout.rs`, `tui/nav.rs` | cells on the status line |
| 2.6 Watches and Evaluate dialog (B07, B08) | `tui/debug.rs`, `tui/dialogs.rs` | flow: add, edit, remove a watch, persisted in `.archipel/debug-state.json` |

### Wave 3: code intelligence, the Run and Problems windows, editing safety

By daily impact: these are used in every session.

| Task | Rows | Crates and files | Tests |
|---|---|---|---|
| 3.1 Disk conflict banner, save failure dialog (data loss) | A29, A30 | core M23; tui `layout.rs`, `dialogs.rs` | flow: touch the file on disk while dirty: banner text and both buttons work |
| 3.2 Quick Doc, hover docs, parameter info, completion docs (markdown block model M17 and a terminal markdown renderer) | A21 to A24 | core `completion.rs`, markdown crate; tui new `docs.rs`; kit markdown helper | flow with the fake LSP; cells for the overlay placement above or below the caret |
| 3.3 Rename, Generate unblock, refactorings, import chooser, chooser and Show Usages, Ctrl+click | C02 to C07, A25, A28 | tui `popups.rs`, `app.rs`, `picker.rs` | flow with the fake LSP: rename across two files, undo once per file; replay `docs/specs/refactoring/cases/02-rename.cases.md` where a runner exists |
| 3.4 Lightbulb, intention submenus, tab stop boxes | A19, A20, A27 | tui `view.rs`, `popups.rs` | cells |
| 3.5 Structure window and popup, breadcrumbs | C08, C09 | core M13; tui new `structure.rs` | flow: caret follows the selected row; replay `docs/specs/search/cases/05-recent-and-structure-popups.cases.md` |
| 3.6 Run, Build and Test windows | D03 to D05 | core M9, M10, M11; tui `panels.rs` splits into `run.rs`, `build.rs`, `tests.rs` | flow with a shell configuration that prints a URL and `file:line:col`; replay `docs/specs/execution/cases/03-run-tool-window-and-console.cases.md` and `04-test-runner.cases.md` via P5 |
| 3.7 Problems tabs and quick fix, Find window grouping and scopes, Find in Files options | D11, C13, C14 | core M12, M15 | flow: scope narrows results; replay `docs/specs/search/cases/02-find-in-files.cases.md` through the TUI |
| 3.8 Editor context menu, tab menu, pin | A26, A33 | core M19; tui new `menus.rs` | cells: menu rows for tree, editor and tab targets match the GUI's `applies` table (shared test over M19) |
| 3.9 Code vision lenses | A17 | tui `view.rs`; P4 | cells: dim text after a declaration, click runs Find Usages |

### Wave 4: Git and diff

| Task | Rows | Crates and files | Tests |
|---|---|---|---|
| 4.1 Commit model (M5) and window upgrade: amend, rollback, diff, AI hook | E02, E10 | core `vcs.rs`; tui `panels.rs` | flow on a temporary repo; replay `docs/specs/vcs/cases/02-commit.cases.md` where a runner exists |
| 4.2 Diff viewer tab (M6): unified first, then side by side; Show Diff from the change popup, Commit window, Git log, file history | E04, E03 | core `vcs.rs`; tui new `diff.rs`; K two-pane | cells for both layouts; replay `docs/specs/vcs/cases/08-diff-viewer.cases.md` |
| 4.3 Git window: Log tab, filters, details, file history | E05, E06 | core `git_log.rs` (M7); tui new `git_log.rs` | flow: graph rows for a merge fixture, filter by branch and text; replay `docs/specs/vcs/cases/04-log.cases.md` |
| 4.4 Branches popup, Push dialog, remote progress | E07, E08, E12 | core M8; tui new `branches.rs`, `push.rs` | flow with a bare remote in a temp dir; replay `docs/specs/vcs/cases/03-branches.cases.md` |
| 4.5 Local History dialog | E11 | tui new `history.rs` | flow: edit twice, revert to the first revision |
| 4.6 Blame annotations (A16), VCS colors (E13, C15) | A16, E13 | tui `view.rs`, `layout.rs`; P4 | cells; replay `docs/specs/vcs/cases/07-annotate-and-history.cases.md` |

### Wave 5: dialogs, settings, project lifecycle (chrome moved to Wave 0)

| Task | Rows | Notes |
|---|---|---|
| 5.1 Settings dialog over the registry, then Keymap, Live Templates, Inspections, Plugins pages | G01 to G03 | M26; tests: change a setting, Apply, the core store has it; the keymap page rejects keys a terminal cannot send |
| 5.2 Run/Debug Configurations dialog | D02 | M24; flow: add, rename, delete a Cargo configuration, file on disk matches |
| 5.3 Project tree operations and context menu | C15, C16 | M19, M28 |
| 5.4 Welcome, New Project, Clone, first run, About, notices | G04, G05, G08, G09, E09 | M25, M23 |
| 5.5 Main menu bar | H01 | moved to Wave 0, task 0.2.4 |
| 5.6 Status bar segments, theme from settings, terminal title | H03 to H08, D09 | moved to Wave 0, tasks 0.2.8 and 0.2.11; notices (H04 to H06) in phase 0.3 |

### Wave 6: the rest of the tool windows and editor layout

Gradle, Bazel, Todo, Hierarchy, Terminal tabs, editor splits, Markdown preview, column selection (rows D07,
D08, D10, C10, D12, A34, A35, A36).

### Wave 7: AI, Agents, plugins, Android

AI chat, agent mode, inline completion and AI actions (J01 to J03, A37, D13); Agents mode (J06);
plugin manager and, after an SDK change, plugin views (J05); Android windows (J07 to J13); inlay hints
(A18) last because it needs inline virtual text in the kit.

## 7. Keys and mouse

> **Being replaced** (see `docs/specs/tui/01-gui-faithful-terminal-ui.md` section L and appendix B): the
> `Ctrl+K` and `Ctrl+W` chord system below shadows GUI shortcuts (`Ctrl+K` is Commit, `Ctrl+W` is Extend
> Selection) and was the primary path for Go to File. The new rule: every action is reachable by menu,
> toolbar or stripe first; the GUI keymap is exact; one leader `Ctrl+\` carries the alternatives for keys a
> legacy terminal cannot send; the kitty keyboard protocol is detected and a one-time notice explains how to
> enable it. The chord proposals in this section are superseded by appendix B of that spec.

The GUI keymap is authoritative (`archipel-ide-keymap`, `docs/spec/keymaps.md`, `core/keymap_commands.rs`
`ACTION_COMMANDS`); the TUI resolves the same action ids (`tui/keys.rs`). Defaults for the new surfaces
(action id, shortcut of the default keymap): `ToggleLineBreakpoint` Ctrl+F8, `EditBreakpoint`
Ctrl+Shift+F8, `Debug` Shift+F9, `Resume` F9, `StepOver` F8, `StepInto` F7, `StepOut` Shift+F8,
`ForceStepInto` Alt+Shift+F7, `RunToCursor` Alt+F9, `EvaluateExpression` Alt+F8,
`ActivateDebugToolWindow` Alt+5, `RunClass` Ctrl+Shift+F10, `Vcs.RollbackChangedLines` Ctrl+Alt+Z,
`VcsShowNextChangeMarker` and `VcsShowPrevChangeMarker` Ctrl+Alt+Shift+Down and Up, `QuickJavaDoc` Ctrl+Q,
`ParameterInfo` Ctrl+P (collides with the TUI's Find Action fallback), `ShowUsages` Ctrl+Alt+F7.

Terminals cannot send some of them (Ctrl+Shift+letter, Ctrl+Alt+arrows, Ctrl+F-keys on many
emulators). Following `docs/tui.md`, each such action gets a `Ctrl+K` chord. `tui/keys.rs` `FALLBACKS`
uses C, N, E, `[`, `]`, Space, B today and `docs/tui.md` lists A, T, F, W, Q, Enter. Proposal,
to be checked for collisions against the kit keymap before merging (the which-key overlay lists them):

| Chord | Action |
|---|---|
| `Ctrl+K P` | Toggle Line Breakpoint |
| `Ctrl+K X` | Edit Breakpoint |
| `Ctrl+K G` | Gutter popup of the caret line (run, debug) |
| `Ctrl+K R` | Run the context target at the caret (`RunClass`) |
| `Ctrl+K D` | Debug |
| `Ctrl+K .` and `Ctrl+K ,` | Next and previous change |
| `Ctrl+K Z` | Rollback lines |
| `Ctrl+K I` | Parameter info |
| `Ctrl+K V` | Branches popup |

Mouse: line number click toggles a breakpoint (as in the GUI), right click on a number opens the
properties, click on a run icon or a change bar opens its popup, Ctrl+click navigates, right click
opens the context menu, double click on a window title maximizes it. All of it arrives as
`EditorOutput.gutter_clicks`, `navigation` and `context_menu`, which the TUI drops today (2.1).

## 8. Glyphs

The kit glyph set (`archipel-kit-tui/src/glyphs.rs`, Nerd Font codicons) already has what Wave 1
needs: `Run`, `Debug`, `Breakpoint`, `Test`, `Thinking`, `Bookmark`, `StepOver`, `StepInto`,
`StepOut`, `Pause`, `Stop`, `Rerun`, `Diff`, `History`, `Branch`, `Commit`. The shapes
the product owner asked for are Unicode geometric shapes and need new kit glyphs (P4):

| Mark | Glyph | Tone |
|---|---|---|
| Run, test | `▶` (`Glyph::Run`) | `Tone::Run` |
| Breakpoint | `●` (`Glyph::Breakpoint`) | `Tone::Error` |
| Disabled breakpoint (when the core has the state) | `○` | `Tone::Muted` |
| Conditional breakpoint | `◆` | `Tone::Warning` |
| Logpoint | `◇` | `Tone::Info` |
| Execution point | `➤` or `▶` over a tinted row | `Tone::Accent` |
| Change bars | `▎` added, modified; `▔` deleted | `Tone::Success`, blue, `Tone::Error` (already in kit `gutter_row`) |

Each lane is two cells wide, which covers the East Asian ambiguous-width risk of these characters; the
wave 1 fallback uses the existing `Breakpoint` glyph with tones so shipping does not wait for the kit.

## 9. Risks and open points

- Terminal limits: no pixel previews (Resource Manager, layout design, device mirror), no sub-cell
  hover, modifier reports need the kitty protocol (Ctrl+click underline, double Shift).
- The kit pin is a branch (`Cargo.lock` `577ce54`); every P4 item is a change in another repository
  and a pin bump here.
- The widget wraps long lines (`wrap(true)`), so inline text (inlay hints, end-of-line values) needs
  the kit to know about virtual columns before A15 and A18 can be exact.
- Debug sessions need a JDK or an adapter on the machine; CI tests must skip like the
  fake-LSP tests do when it is missing (`tests/common/mod.rs` `with_fake_lsp` returns `Option`).
- Moves in section 5 touch the GUI; each needs the GUI's existing tests (`archipel-ide-ui` unit tests
  such as `ui_ed_012_gutter_matches_the_reference_columns`) green before the TUI builds on them.

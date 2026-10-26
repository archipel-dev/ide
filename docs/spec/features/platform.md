# Platform feature inventory (language-agnostic IntelliJ Platform)

Status: research inventory, not an implementation claim. Nothing here asserts parity; parity
evidence belongs in `docs/parity/` (see `AGENTS.md`).

Target product: IntelliJ IDEA 2026.2 (platform build line 262). Scope: features that live in the
shared IntelliJ Platform and therefore appear in IDEA, PyCharm, WebStorm, GoLand, etc. Java/JVM-only
features are excluded unless they are the platform's generic framework (for example the
"Type Hierarchy" framework is listed; Java's class hierarchy implementation is not).

## Conventions

- **ID**: `PLATFORM-<AREA>-NNN`. IDs are stable; never renumber. New items are appended with the
  next free number. Retired items stay in the table marked "(retired)".
- **Action id**: where a public action id exists in the Apache-2.0 IntelliJ Community action
  descriptors (tag `idea/262.10968.63`), it is given in backticks. It is a cross-reference for
  mapping onto `archipel-ide-command` command ids, not something to copy.
- **Shortcuts**: "Win/Linux" is the default (`$default`) keymap; "macOS" is the
  "macOS" keymap (descriptor file `Mac OS X 10.5+`). Notation: `Ctrl`, `Alt`, `Shift`, `Cmd`,
  `Opt`; a comma separates the two strokes of a chord (e.g. `Alt+Shift+6, F`). `Double Shift`
  means tapping the key twice. `—` means no default shortcut. Shortcuts were extracted
  mechanically from the keymap descriptors and cross-checked against the public keymap reference;
  where the macOS keymap does not override an action, the platform derives the macOS binding by
  swapping Ctrl and Cmd. This rule was confirmed against the keymap loading behaviour; see
  [../README.md](../README.md#how-the-generated-data-is-resolved). The authoritative per-action
  values are in [../keymaps.md](../keymaps.md).
- **Entry point**: main menu path (`View | Tool Windows | ...`), tool window, popup, gutter,
  status bar widget, or Find Action only.
- **Priority**: P0 = required for a credible daily-driver IDE; P1 = expected by regular
  IntelliJ users, ship soon after P0; P2 = niche, commercial-only, optional, or AI.
- **(inferred)** marks behaviour reconstructed from observation or naming rather than an explicit
  public statement.
- **(plugin)** marks platform features that ship as bundled plugins rather than core modules
  (still language-agnostic). **(Ultimate)** marks commercial-only features.

## Area index

| Area code | Section |
|---|---|
| EDIT | 1. Editor core |
| NAV | 2. Navigation and search |
| PROJ | 3. Project model and files |
| VCS | 4. Version control (Git) |
| RUN | 5. Run / Debug infrastructure |
| TERM | 6. Terminal |
| TOOL | 7. Tool windows and workbench services |
| SET | 8. Settings and configuration |
| AI | 9. AI features (optional) |
| REMOTE | 10. Collaboration and remote |
| A11Y | 11. Accessibility and i18n |
| PERF | 12. Performance and background work |
| UI | 13. Window, tab and layout management |

---

## 1. Editor core (EDIT)

### 1.1 Caret, selection, multi-caret

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-EDIT-001 | Caret movement primitives (`EditorLeft/Right/Up/Down`, `EditorNextWord`, `EditorPreviousWord`, `EditorLineStart/End`, `EditorTextStart/End`, `EditorPageUp/Down`, `EditorMoveToPageTop/Bottom`) | Character, word, line, page and document movement. Word boundaries honour the "camel humps" setting (sub-word stops inside `camelCase`) when enabled. Home toggles between first non-whitespace char and column 0 ("smart home"). Each primitive has a `...WithSelection` twin that extends the selection. | Arrows; Ctrl+Left/Right; Home/End; Ctrl+Home/End; PgUp/PgDn; Ctrl+PgUp/PgDn | Arrows; Opt+Left/Right; Cmd+Left/Right; Cmd+Home/End; PgUp/PgDn; Cmd+PgUp/PgDn | Editor | P0 |
| PLATFORM-EDIT-002 | Selection model | Each caret owns at most one contiguous selection with an anchor and a lead; selection survives scrolling; Shift+click extends; double-click selects word, triple-click selects line; drag-and-drop of selected text moves it (Ctrl/Opt to copy). | Shift+arrows, mouse | Shift+arrows, mouse | Editor | P0 |
| PLATFORM-EDIT-003 | Select All (`$SelectAll`) | Selects whole document (in multi-caret mode collapses to one caret). | Ctrl+A | Cmd+A | Edit menu | P0 |
| PLATFORM-EDIT-004 | Extend Selection (`EditorSelectWord`) | Grows the selection to the next enclosing syntactic unit (word, expression, statement, block, ...). Uses the language's syntax tree; falls back to word/line/brace heuristics for plain text. | Ctrl+W | Opt+Up | Edit \| Extend Selection | P0 |
| PLATFORM-EDIT-005 | Shrink Selection (`EditorUnSelectWord`) | Reverses the last extend step(s). | Ctrl+Shift+W | Opt+Down | Edit \| Shrink Selection | P0 |
| PLATFORM-EDIT-006 | Add/remove caret by click (`EditorAddOrRemoveCaret`) | Clicking with the modifier adds a new caret at the click point or removes an existing one there. | Alt+Shift+Click | Opt+Shift+Click | Editor mouse | P0 |
| PLATFORM-EDIT-007 | Clone caret above/below (`EditorCloneCaretAbove`, `EditorCloneCaretBelow`) | Adds a caret on the adjacent line at the same visual column. Default binding is a hold-gesture: press the modifier twice and keep it held, then press Up/Down (gesture is hard-wired, not in the keymap descriptor (inferred)). | Ctrl, Ctrl (hold) + Up/Down | Opt, Opt (hold) + Up/Down | Find Action; gesture | P0 |
| PLATFORM-EDIT-008 | Add Selection for Next Occurrence (`SelectNextOccurrence`) | First press selects the word at caret; each next press adds a caret+selection at the next match of the selected text (case-sensitive, wraps). | Alt+J | Ctrl+G | Edit \| Find \| Add Selection for Next Occurrence | P0 |
| PLATFORM-EDIT-009 | Unselect Occurrence (`UnselectPreviousOccurrence`) | Removes the most recently added occurrence caret. | Alt+Shift+J | Ctrl+Shift+G | Edit \| Find | P0 |
| PLATFORM-EDIT-010 | Select All Occurrences (`SelectAllOccurrences`) | Places a caret+selection on every match of the current selection/word in the file. | Ctrl+Alt+Shift+J | Ctrl+Cmd+G | Edit \| Find | P0 |
| PLATFORM-EDIT-011 | Add Carets to Ends of Selected Lines (`EditorAddCaretPerSelectedLine`) | Converts a multi-line selection into one caret at the end of each line. | Alt+Shift+G | Opt+Shift+G | Edit \| Find / Find Action | P1 |
| PLATFORM-EDIT-012 | Column (block) selection mode (`EditorToggleColumnMode`) | Toggle where Shift+arrows and mouse drags create rectangular selections (one caret per line). Status bar shows the mode. | Alt+Shift+Insert | Cmd+Shift+8 | Edit \| Column Selection Mode | P0 |
| PLATFORM-EDIT-013 | Rectangular selection by mouse (`EditorCreateRectangularSelectionOnMouseDrag`, `EditorAddRectangularSelectionOnMouseDrag`) | Alt-drag or middle-button drag creates a rectangular selection without toggling the mode; adding Ctrl+Shift adds a further block. | Alt+Drag / Middle-drag; Ctrl+Alt+Shift+Drag | Opt+Drag; Cmd+Opt+Shift+Drag (inferred) | Editor mouse | P1 |
| PLATFORM-EDIT-014 | Multi-caret semantics | All typing, deletion, paste, completion, live templates and most editor actions apply per caret in one undoable command; carets that collide merge. Paste of N clipboard lines onto N carets distributes one line per caret. Esc collapses to the primary caret. | — | — | Editor | P0 |
| PLATFORM-EDIT-015 | Virtual space | Option "Allow placement of caret after end of line" lets the caret sit in whitespace beyond line end; typing there pads with spaces. Also "allow caret inside tab characters". | — | — | Settings \| Editor \| General \| Virtual Space | P2 |
| PLATFORM-EDIT-016 | Caret behaviour options | Caret blinking & width, "use block caret", camel-humps word navigation, "caret position on line/after end", restoring caret on reopen. | — | — | Settings \| Editor \| General \| Caret Movement / Appearance | P1 |
| PLATFORM-EDIT-017 | Insert/Overwrite toggle (`EditorToggleInsertState`) | Switches typing between insert and overwrite; status bar indicator. | Insert | Insert (Fn+Enter on laptops (inferred)) | Editor | P1 |
| PLATFORM-EDIT-018 | Move caret to matching brace (`EditorMatchBrace`) and code block start/end (`EditorCodeBlockStart/End`) | Jumps between paired delimiters / enclosing block bounds, with selecting variants. | Ctrl+Shift+M; Ctrl+[ / Ctrl+] | Ctrl+M; Cmd+Opt+[ / Cmd+Opt+] | Find Action | P1 |
| PLATFORM-EDIT-019 | Scroll without moving caret (`EditorScrollUp/Down`, `EditorScrollToCenter`) | Scrolls the viewport by a line or centres the caret line. | Ctrl+Up/Down; Ctrl+M | — ; Ctrl+L | Find Action | P1 |
| PLATFORM-EDIT-020 | Jump outside bracket/quote (`BraceOrQuoteOut`) | When caret is just before an auto-inserted closing bracket or quote, Tab steps over it instead of indenting. | Tab | Tab | Settings \| Editor \| General \| Smart Keys | P1 |

### 1.2 Clipboard, line and text manipulation

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-EDIT-030 | Cut/Copy/Paste (`$Cut`, `$Copy`, `$Paste`) | Standard clipboard; with no selection, Cut/Copy act on the whole current line (configurable). Paste reformats/reindents per "Smart Keys \| Reformat on paste" setting and may adjust imports (language plugins). | Ctrl+X / Ctrl+C / Ctrl+V (also Shift+Del, Ctrl+Ins, Shift+Ins) | Cmd+X / Cmd+C / Cmd+V | Edit menu | P0 |
| PLATFORM-EDIT-031 | Paste from History (`PasteMultiple`) | Popup listing recent clipboard entries (default 5 kept, configurable) with preview; choosing one pastes it; multiple entries can be merged. Entries come from copy/cut inside the IDE and from the system clipboard. | Ctrl+Shift+V | Cmd+Shift+V | Edit \| Paste Special \| Paste from History | P0 |
| PLATFORM-EDIT-032 | Paste as Plain Text (`EditorPasteSimple`) | Pastes without reformat/import/escaping processing. | Ctrl+Alt+Shift+V | Cmd+Opt+Shift+V | Edit \| Paste Special | P1 |
| PLATFORM-EDIT-033 | Copy Path / Copy Reference (`CopyPaths`, `CopyReference`) | Copies absolute path, path from content root, file name, or a language-qualified reference (e.g. `file:line` for plain files) of the element at caret. "Copy Path/Reference..." popup offers all variants. | Ctrl+Shift+C; Ctrl+Alt+Shift+C | Cmd+Shift+C; Cmd+Opt+Shift+C | Edit \| Copy Path/Reference | P1 |
| PLATFORM-EDIT-034 | Duplicate Line or Selection (`EditorDuplicate`) | Duplicates the selection, or the current line when nothing is selected; caret moves into the copy. Variant "Duplicate Entire Lines" duplicates whole lines spanned by selection. | Ctrl+D | Cmd+D | Edit \| Duplicate Line or Selection | P0 |
| PLATFORM-EDIT-035 | Delete Line (`EditorDeleteLine`) | Deletes all lines spanned by carets/selections. | Ctrl+Y | Cmd+Backspace | Edit / Find Action | P0 |
| PLATFORM-EDIT-036 | Delete to word start/end (`EditorDeleteToWordStart/End`) | Word-wise delete respecting camel humps setting. | Ctrl+Backspace / Ctrl+Delete | Opt+Backspace / Opt+Delete | Editor | P0 |
| PLATFORM-EDIT-037 | Move Line Up/Down (`MoveLineUp/Down`) | Swaps the current line(s) with the neighbour line, ignoring syntax. | Alt+Shift+Up/Down | Opt+Shift+Up/Down | Code \| Move Line Up/Down | P0 |
| PLATFORM-EDIT-038 | Move Statement Up/Down (`MoveStatementUp/Down`) | Moves the enclosing statement/member/block as a unit, respecting syntax (language-provided mover; falls back to line move). | Ctrl+Shift+Up/Down | Cmd+Shift+Up/Down | Code \| Move Statement Up/Down | P1 |
| PLATFORM-EDIT-039 | Move Element Left/Right (`MoveElementLeft/Right`) | Swaps list elements such as arguments or attributes (language-provided). | Ctrl+Alt+Shift+Left/Right | Cmd+Opt+Shift+Left/Right | Code menu | P2 |
| PLATFORM-EDIT-040 | Join Lines (`EditorJoinLines`) | Joins the next line onto the current one, collapsing whitespace and applying language-specific smart joins (e.g. concatenated strings, comment continuation). | Ctrl+Shift+J | Ctrl+Shift+J | Edit \| Join Lines | P0 |
| PLATFORM-EDIT-041 | Split Line / Start New Line / Start New Line Before (`EditorSplitLine`, `EditorStartNewLine`, `EditorStartNewLineBefore`) | Break line keeping caret; open new indented line below or above regardless of caret column. | Ctrl+Enter; Shift+Enter; Ctrl+Alt+Enter | Cmd+Enter; Shift+Enter; Cmd+Opt+Enter | Editor | P0 |
| PLATFORM-EDIT-042 | Complete Current Statement (`EditorCompleteStatement`) | Adds missing closing tokens/terminators and moves caret to the natural next position (language-provided; generic fallback = new line). | Ctrl+Shift+Enter | Cmd+Shift+Enter | Code \| Code Completion | P1 |
| PLATFORM-EDIT-043 | Toggle Case (`EditorToggleCase`) | Toggles selection (or word at caret) between upper and lower case. | Ctrl+Shift+U | Cmd+Shift+U | Edit \| Toggle Case | P0 |
| PLATFORM-EDIT-044 | Sort / Reverse Lines (`EditorSortLines`, `EditorReverseLines`) | Lexicographically sorts or reverses selected lines (whole file when no selection). | — | — | Edit \| Sort Lines / Reverse Lines | P1 |
| PLATFORM-EDIT-045 | Fill Paragraph (`FillParagraph`) | Re-wraps the paragraph/comment at caret to the right margin. | — | — | Edit \| Fill Paragraph | P2 |
| PLATFORM-EDIT-046 | Indent / Unindent selection (`EditorIndentSelection`, `EditorUnindentSelection`) | Tab/Shift+Tab shift the selected lines by one indent unit. | Tab / Shift+Tab | Tab / Shift+Tab | Edit \| Indent Selection | P0 |
| PLATFORM-EDIT-047 | Auto-Indent Lines (`AutoIndentLines`) | Recomputes indentation for current/selected lines using the formatter's indent model only. | Ctrl+Alt+I | Ctrl+Opt+I | Code \| Auto-Indent Lines | P0 |
| PLATFORM-EDIT-048 | Convert Indents (`ConvertIndentsToSpaces`, `ConvertIndentsToTabs`) | Converts leading whitespace in selection/file. | — | — | Edit \| Convert Indents | P1 |
| PLATFORM-EDIT-049 | Smart keys | Auto-insert pair brackets and quotes; smart Enter indentation; smart Backspace (unindent / remove paired char); "surround selection on typing quote or brace"; smart Home; "insert paired" for markup tags. Each is a toggle. | — | — | Settings \| Editor \| General \| Smart Keys | P0 |
| PLATFORM-EDIT-050 | Cyclic Expand Word (`HippieCompletion`, `HippieBackwardCompletion`) | Completes the current prefix with words found in visible text/open files, cycling on repeat; no language knowledge required. | Alt+/ ; Alt+Shift+/ | Opt+/ ; Opt+Shift+/ | Code \| Code Completion | P1 |
| PLATFORM-EDIT-051 | Emoji & Symbols picker (`EmojiPicker.Open`) (plugin) | Popup to insert emoji/symbols at caret. | Ctrl+Alt+; | Cmd+Opt+; | Edit menu | P2 |
| PLATFORM-EDIT-052 | Trailing whitespace & final newline on save | Strip trailing spaces (none / modified lines / all), keep caret line's whitespace, ensure line feed at EOF, remove extra blank lines at EOF. | — | — | Settings \| Editor \| General \| On Save | P0 |
| PLATFORM-EDIT-053 | Actions on Save | Configurable per-project list run when a file is saved: reformat code (whole file or changed lines), optimize imports, rearrange code, run code cleanup, plus plugin-contributed actions (e.g. Prettier, ESLint (plugins)). | — | — | Settings \| Tools \| Actions on Save | P1 |

### 1.3 Comments, folding, wraps, whitespace, display

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-EDIT-060 | Comment with Line Comment (`CommentByLineComment`) | Toggles the language's line-comment prefix on each selected line; caret moves to next line when no selection. Options: comment at first column, add space. Plain-text fallback: none (inferred). | Ctrl+/ | Cmd+/ | Code \| Comment with Line Comment | P0 |
| PLATFORM-EDIT-061 | Comment with Block Comment (`CommentByBlockComment`) | Wraps/unwraps selection in block comment delimiters. | Ctrl+Shift+/ | Cmd+Opt+/ | Code \| Comment with Block Comment | P0 |
| PLATFORM-EDIT-062 | Code folding: expand/collapse (`ExpandRegion`, `CollapseRegion`, `ExpandRegionRecursively`, `CollapseRegionRecursively`, `ExpandAllRegions`, `CollapseAllRegions`, `ExpandToLevel1..5`, `ExpandAllToLevel1..5`) | Fold regions come from the language (blocks, imports, comments) plus generic ones. Gutter shows fold markers; collapsed text shows a placeholder (`{...}`) with hover preview. | Ctrl+= / Ctrl+-; Ctrl+Alt+= / -; Ctrl+Shift+= / -; Ctrl+*, N; Ctrl+Shift+*, N | Cmd+= / Cmd+-; Cmd+Opt+= / -; Cmd+Shift+= / -; Cmd+*, N; Cmd+Opt+*, N | Code \| Folding | P0 |
| PLATFORM-EDIT-063 | Fold Selection / Fold Code Block (`CollapseSelection`, `CollapseBlock`) | Creates an ad-hoc fold over the selection, or folds the enclosing block. | Ctrl+. ; Ctrl+Shift+. | Cmd+. ; Cmd+Shift+. | Code \| Folding | P1 |
| PLATFORM-EDIT-064 | Custom folding regions (`GotoCustomRegion`) | Comment markers (`region`/`endregion`, `<editor-fold>`) define user folds; a popup lists them for navigation. | Ctrl+Alt+. | Cmd+Opt+. | Code \| Folding \| Custom Folding | P1 |
| PLATFORM-EDIT-065 | Fold state persistence and defaults | Fold states persist per file across sessions; "collapse by default" toggles per region kind. | — | — | Settings \| Editor \| General \| Code Folding | P1 |
| PLATFORM-EDIT-066 | Soft wraps (`EditorToggleUseSoftWraps`) | Visual wrapping at window edge or right margin, per editor toggle and global per-file-mask setting (default `*.md; *.txt; *.rst; *.adoc`); wrapped lines may keep indentation and show a wrap sign. | — | — | View \| Active Editor \| Soft-Wrap | P0 |
| PLATFORM-EDIT-067 | Hard wrap / right margin | Visual guide at right margin (default 120) plus optional extra visual guides; "wrap on typing" inserts line breaks automatically. | — | — | Settings \| Editor \| Code Style | P1 |
| PLATFORM-EDIT-068 | Show whitespaces (`EditorToggleShowWhitespaces`) | Renders spaces/tabs as glyphs (leading/inner/trailing selectable); also shows indent guides and optional "show line separators between methods". | — | — | View \| Active Editor \| Show Whitespaces | P1 |
| PLATFORM-EDIT-069 | Line numbers, gutter configuration (`EditorGutterToggleGlobalLineNumbers`) | Absolute / relative / hybrid line numbers; toggle gutter icons, breakpoints area, fold markers, VCS change markers. | — | — | Gutter context menu; Settings \| Editor \| General \| Appearance | P0 |
| PLATFORM-EDIT-070 | Sticky lines (`EditorGutterToggleGlobalStickyLines`, `EditorStickyLinesDisable`) | Pins headers of enclosing scopes (class/function/block) at the top of the viewport while scrolling; clickable to navigate; max lines configurable, per-language opt-out. | — | — | Editor context; Settings \| Editor \| General \| Sticky Lines | P1 |
| PLATFORM-EDIT-071 | Breadcrumbs (`EditorToggleShowBreadcrumbs`, `EditorBreadcrumbsShowAbove/Below`) | Strip showing the structural path to the caret (e.g. tag nesting, class > method); clicking navigates. Position top/bottom/hidden, per language. | — | — | View \| Active Editor; Settings \| Editor \| General \| Breadcrumbs | P1 |
| PLATFORM-EDIT-072 | Inlay hints framework (`ToggleInlayHintsGloballyAction`) | Inline non-text annotations (parameter names, types, code vision "N usages"/author) contributed by language plugins; a global toggle and per-provider settings with preview. | — | — | Settings \| Editor \| Inlay Hints | P1 |
| PLATFORM-EDIT-073 | Editor font & zoom (`EditorIncreaseFontSizeGlobal`, `EditorDecreaseFontSizeGlobal`) | Font family, size, line height, ligatures; Ctrl+mouse wheel zoom (opt-in) for current or all editors; reset font size. | Alt+Shift+. / Alt+Shift+, | Ctrl+Shift+. / Ctrl+Shift+, | Settings \| Editor \| Font; View \| Appearance | P0 |
| PLATFORM-EDIT-074 | Color schemes (`QuickChangeScheme`) | Bundled schemes (Light, Dark, Darcula, High Contrast, "Classic Light"...) with per-language attribute overrides, import/export (`.icls`), semantic-highlighting option. Quick Switch Scheme popup lists scheme, keymap, view modes, LaF. | Ctrl+` | Ctrl+` | Settings \| Editor \| Color Scheme | P0 |
| PLATFORM-EDIT-075 | Highlight current line / matching braces / identifier under caret | Caret-row background; matching bracket highlight; automatic highlighting of usages of the symbol under caret after a short delay. | — | — | Settings \| Editor \| General \| Appearance / Code Editing | P0 |
| PLATFORM-EDIT-076 | Highlight Usages in File (`HighlightUsagesInFile`) | Persistently highlights read/write usages of the symbol at caret; F3/Shift+F3 cycle between them; Esc clears. | Ctrl+Shift+F7 | Cmd+Shift+F7 | Edit \| Find \| Highlight Usages in File | P1 |
| PLATFORM-EDIT-077 | Editor minimap / error stripe | Right-hand marker bar showing errors, warnings, find results, VCS changes; click to jump; a traffic-light "inspection widget" at top summarising problem counts with navigation. (A full code minimap is only available via plugins (inferred).) | — | — | Editor | P0 |
| PLATFORM-EDIT-078 | Quick documentation / Context info (`QuickJavaDoc`, `EditorContextInfo`) | Generic popup framework showing docs for symbol at caret (on hover or on demand), pin-able to a tool window; Context Info shows the enclosing scope header when off-screen. Content is language-provided. | Ctrl+Q; Alt+Q | F1; Ctrl+Shift+Q | View \| Quick Documentation | P0 |
| PLATFORM-EDIT-079 | Quick Definition (`QuickImplementations`) | Popup previewing the definition source of the symbol at caret. | Ctrl+Shift+I | Opt+Space / Cmd+Y | View \| Quick Definition | P1 |
| PLATFORM-EDIT-080 | Parameter info / Type info / Error description (`ParameterInfo`, `ExpressionTypeInfo`, `ShowErrorDescription`) | Framework popups filled by language support; error description shows the full message of the highlight under caret. | Ctrl+P; Ctrl+Shift+P; Ctrl+F1 | Cmd+P; Ctrl+Shift+P; Cmd+F1 | View menu | P1 |
| PLATFORM-EDIT-081 | Rendered documentation comments (`ToggleRenderedDocPresentation`) | Shows doc comments rendered as formatted text inline in the editor; toggle back to raw. | Ctrl+Alt+Q | Ctrl+Opt+Q | Gutter icon; Find Action | P2 |
| PLATFORM-EDIT-082 | Image and binary viewers (`Images.Editor.*`) | Built-in image viewer (zoom, grid, background, open externally) and fall-back "file too large / binary" placeholder editor. | Ctrl+' (grid) | Cmd+' | Opens automatically | P2 |
| PLATFORM-EDIT-083 | Hex/byte view of binary files (inferred) | Binary files open in a read-only notice editor; a hex view is plugin territory. | — | — | — | P2 |

### 1.4 File-level settings: encodings, separators, EditorConfig, large files, read-only

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-EDIT-090 | Line separators (`ConvertToUnixLineSeparators`, `ConvertToWindowsLineSeparators`, `ConvertToMacLineSeparators`) | Detects LF/CRLF/CR per file, preserves on save; status bar widget converts; default for new files is configurable per project. | — | — | Status bar; File \| File Properties \| Line Separators | P0 |
| PLATFORM-EDIT-091 | File encoding (`ChangeFileEncodingAction`) | Status bar widget shows encoding; "Reload in" vs "Convert to" another encoding; per-file/per-directory mapping; BOM handling for UTF-8 (with/without/as-is); properties-file default; warning if chars not representable. | — | — | Status bar; Settings \| Editor \| File Encodings | P0 |
| PLATFORM-EDIT-092 | Indentation detection | When enabled, infers tab vs spaces and indent size from file content and overrides code style for that file; status bar shows the detected indent and lets the user switch/disable. | — | — | Status bar; Settings \| Editor \| Code Style \| Detect and use existing file indents | P0 |
| PLATFORM-EDIT-093 | EditorConfig support (plugin, bundled) | Reads `.editorconfig` hierarchies; standard properties (indent_style/size, tab_width, end_of_line, charset, trim_trailing_whitespace, insert_final_newline, max_line_length) override IDE settings; vendor `ij_*` keys map to code style options; export code style to `.editorconfig`; status bar indicates when overridden. | — | — | Settings \| Editor \| Code Style \| Enable EditorConfig support | P0 |
| PLATFORM-EDIT-094 | Large file handling | Files above a size threshold (Advanced Setting, default 2.5 MB (inferred)) open without code insight/highlighting; above a larger limit open read-only in a paged "large file" viewer with search (inferred); indexing excludes files above the intellisense limit. | — | — | Settings \| Advanced Settings | P0 |
| PLATFORM-EDIT-095 | Read-only files (`ToggleReadOnlyAttribute`) | Toggles the OS writable flag; lock icon in status bar/tab; editing a read-only or VCS-locked file prompts "Make writable?"; library and generated files are non-editable with a banner. | — | — | File \| File Properties \| Make File Read-Only; status bar lock | P0 |
| PLATFORM-EDIT-096 | External change detection (`Synchronize`) | Watches the file system; reloads unmodified editors automatically; prompts on conflicting changes with diff; "Reload All from Disk" forces a refresh. Options: sync on frame activation. | Ctrl+Alt+Y | Cmd+Opt+Y | File \| Reload All from Disk | P0 |
| PLATFORM-EDIT-097 | Save model (`SaveAll`) | Implicit autosave: on frame deactivation, on idle after N seconds, before run/VCS; Save All saves everything; unsaved marker on tabs is optional. | Ctrl+S | Cmd+S | File \| Save All | P0 |

### 1.5 Undo, local history, macros, scratch

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-EDIT-100 | Undo/Redo (`$Undo`, `$Redo`) | Per-document stacks plus global (cross-file) commands for refactorings; typing is coalesced into groups; caret/selection is restored; undoing a global command prompts if it affects other files; undo after file close works while history kept; optional confirmation for undoing multi-file changes. | Ctrl+Z; Ctrl+Shift+Z | Cmd+Z; Cmd+Shift+Z | Edit \| Undo/Redo | P0 |
| PLATFORM-EDIT-101 | Local History (`LocalHistory.ShowHistory`, `LocalHistory.ShowSelectionHistory`, `LocalHistory.PutLabel`, `RecentChanges`) | IDE-maintained revision store of every file/dir change (saves, refactorings, VCS ops, external changes), independent of VCS; time-ordered list with diff; revert a file, folder, or selection; user labels; retention period configurable (default 5 working days (inferred)). Recent Changes popup lists latest change sets. | — ; Recent Changes Alt+Shift+C | — ; Opt+Shift+C | File / context menu \| Local History | P0 |
| PLATFORM-EDIT-102 | Macro recording (`StartStopMacroRecording`, `PlaybackLastMacro`, `PlaySavedMacrosAction`, `EditMacros`) | Records keystrokes and editor actions (not mouse or dialogs); saved macros get names, can be bound to shortcuts, renamed or deleted. | — | — | Edit \| Macros | P1 |
| PLATFORM-EDIT-103 | Scratch files (`NewScratchFile`, `Scratch.ShowFilesPopup`) | Temporary files of any registered language stored outside projects (IDE config `scratches/`), with full editing support; visible in Project view "Scratches and Consoles"; can be run where language supports it. | Ctrl+Alt+Shift+Insert | Cmd+Shift+N | File \| New \| Scratch File | P1 |
| PLATFORM-EDIT-104 | Scratch buffers (`NewScratchBuffer`) | Numbered plain-text buffers (buffer1.txt, ...) recycled after a fixed count (default 5 (inferred)). | — | — | Find Action | P2 |

### 1.6 Find/replace in file, highlighting search

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-EDIT-110 | Find in file (`Find`) | Inline search bar over the editor: incremental highlight of all matches, count "N/M", options Match Case, Words, Regex; search in selection when multi-line selection exists; filter (anywhere / in comments / in string literals / except ...); history dropdown. | Ctrl+F | Cmd+F | Edit \| Find \| Find | P0 |
| PLATFORM-EDIT-111 | Find next/previous (`FindNext`, `FindPrevious`, `FindWordAtCaret`, `FindPrevWordAtCaret`) | Cycles matches, also works after the bar closes; word-at-caret variants search without opening the bar. | F3 / Shift+F3 (Ctrl+L / Ctrl+Shift+L); Ctrl+F3 | Cmd+G / Cmd+Shift+G | Edit \| Find | P0 |
| PLATFORM-EDIT-112 | Replace in file (`Replace`) | Adds a replace field; Replace, Replace All, Exclude; regex groups `$1`, case-preserving replace option, `\n` multiline; Ctrl+Alt+E toggles "in selection only". | Ctrl+R | Cmd+R | Edit \| Find \| Replace | P0 |
| PLATFORM-EDIT-113 | Regex engine & help | Java-flavoured regex syntax (named groups, lookaround, `\l \u \L \U` case conversion in replacement); regex syntax checking and a cheat-sheet popup in the search field. | — | — | Find bar | P0 |
| PLATFORM-EDIT-114 | Multi-line search field and search history (`ShowSearchHistory`) | Search field expands to multiline; previous queries via history popup. | Alt+Down | Opt+Down | Find bar | P1 |
| PLATFORM-EDIT-115 | Select occurrences from find bar | "Select All Occurrences" from the bar converts matches to multi-caret selection. | Alt+Enter (in bar) (inferred) | Opt+Enter (inferred) | Find bar | P1 |

### 1.7 Code style, formatting, templates, structure editing

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-EDIT-120 | Reformat Code (`ReformatCode`) | Formats selection, or whole file (or changed lines only when configured) per code style; works on directories/scopes from Project view with options (optimize imports, rearrange, cleanup, only VCS-changed text). Formatter-off/on markers respected. | Ctrl+Alt+L | Cmd+Opt+L | Code \| Reformat Code | P0 |
| PLATFORM-EDIT-121 | Reformat File dialog (`ShowReformatFileDialog`) | Dialog to choose scope and options before reformatting. | Ctrl+Alt+Shift+L | Cmd+Opt+Shift+L | Code \| Reformat File | P1 |
| PLATFORM-EDIT-122 | Code Style settings & schemes | Per-language tabs (Tabs and Indents, Spaces, Wrapping and Braces, Blank Lines, ...) with live preview; scheme stored at IDE level or in project (`.idea/codeStyles`); "Set from..." predefined styles; import/export. Generic "Other File Types" settings cover plain text. | — | — | Settings \| Editor \| Code Style | P0 |
| PLATFORM-EDIT-123 | Rearrange Code (`RearrangeCode`) | Reorders members per language arrangement rules (matching rules + section rules editable). | — | — | Code \| Rearrange Code | P2 |
| PLATFORM-EDIT-124 | Optimize Imports (`OptimizeImports`) | Framework action; language plugins implement import cleanup; "optimize on the fly" option. | Ctrl+Alt+O | Ctrl+Opt+O | Code \| Optimize Imports | P1 |
| PLATFORM-EDIT-125 | Live templates engine (`InsertLiveTemplate`, `ExpandLiveTemplateByTab`, `NextTemplateVariable`, `PreviousTemplateVariable`) | Abbreviation-triggered snippets with variables (`$NAME$`) bound to expression functions (e.g. date, clipboard, file name, camelCase, complete()), applicability contexts per language, expand key (Tab/Space/Enter), template groups, sharing via export, reformat-on-expand option; in-place variable editing cycles with Tab/Shift+Tab, Enter/Esc to finish. | Ctrl+J (insert); Tab (expand) | Cmd+J; Tab | Code \| Insert Live Template; Settings \| Editor \| Live Templates | P0 |
| PLATFORM-EDIT-126 | Surround With (`SurroundWith`, `SurroundWithLiveTemplate`) | Wraps the selection in a language construct or in a surround-type live template (those using `$SELECTION$`). | Ctrl+Alt+T; Ctrl+Alt+J | Cmd+Opt+T; Cmd+Opt+J | Code \| Surround With | P1 |
| PLATFORM-EDIT-127 | Unwrap/Remove (`Unwrap`) | Removes an enclosing construct, keeping its body (language-provided). | Ctrl+Shift+Delete | Cmd+Shift+Delete | Code \| Unwrap/Remove | P2 |
| PLATFORM-EDIT-128 | Postfix templates framework | Expression-suffix templates (`expr.if`) contributed by languages, editable per language. | Tab | Tab | Settings \| Editor \| General \| Postfix Completion | P2 |
| PLATFORM-EDIT-129 | Emmet / zen-coding expansion (`EmmetNextEditPoint`, `EmmetPreviousEditPoint`) | Abbreviation expansion for markup/CSS shipped with the HTML/XML support (not pure platform core); edit-point navigation; wrap with abbreviation; preview popup. | Tab; Alt+Shift+] / [ | Tab; Ctrl+Opt+Right/Left | Settings \| Editor \| Emmet | P2 |
| PLATFORM-EDIT-130 | Code completion framework (`CodeCompletion`, `SmartTypeCompletion`, `ClassNameCompletion`) | Lookup popup with prefix/camel-hump/fuzzy matching, sorting by relevance (ML ranking plugin), auto-popup delay, insert vs replace (Enter vs Tab), documentation side panel, "show completion variants as you type". Content is language-provided; generic word completion in plain text (inferred). | Ctrl+Space; Ctrl+Shift+Space; Ctrl+Alt+Space | Ctrl+Space; Ctrl+Shift+Space; Ctrl+Opt+Space | Code \| Code Completion | P0 |
| PLATFORM-EDIT-131 | Inline (full-line / multi-line) completion (`CallInlineCompletionAction`, `InsertInlineCompletionAction`, `Next/PrevInlineCompletionSuggestionAction`) | Grey ghost-text proposals; Tab accepts, word-by-word acceptance, cycle proposals. Local full-line models ship with the IDE; cloud providers via AI plugins. | Alt+Shift+\ ; Tab; Alt+] / Alt+[ | Opt+Shift+\ ; Tab; Opt+] / Opt+[ | Settings \| Editor \| General \| Inline Completion | P2 |
| PLATFORM-EDIT-132 | Intention / context actions framework (`ShowIntentionActions`, `QuickActionPopup`) | Light-bulb popup of quick-fixes and intentions at caret, with preview pane, sub-options (suppress, edit inspection settings), and the floating code toolbar. | Alt+Enter; Ctrl+Alt+Enter | Opt+Enter; Cmd+Opt+Enter | Editor bulb | P0 |
| PLATFORM-EDIT-133 | Generate menu (`Generate`) | Framework popup; languages contribute generators. Also used for "New..." in Project view. | Alt+Insert | Cmd+N | Code \| Generate | P1 |
| PLATFORM-EDIT-134 | Rename refactoring framework (`RenameElement`) | In-place rename with live preview of all occurrences; for files/directories opens dialog; language-agnostic for files, text-occurrence search in comments/strings optional. | Shift+F6 | Shift+F6 | Refactor \| Rename | P0 |
| PLATFORM-EDIT-135 | Refactor This & other refactoring framework actions (`Refactorings.QuickListPopupAction`, `Move`, `CopyElement`, `SafeDelete`) | Contextual list of applicable refactorings; Move/Copy/Safe Delete are generic for files/dirs (Safe Delete searches usages before deleting). Language-specific refactorings (extract, inline, change signature) are out of scope here. | Ctrl+Alt+Shift+T; F6; F5; Alt+Delete | Ctrl+T; F6; F5; Cmd+Delete | Refactor menu | P1 |

### 1.8 Spelling, natural language, TODO, injections, inspections

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-EDIT-140 | Spell checking | Checks identifiers (split by camel case/underscores), comments, string literals, and plain text against bundled English dictionary plus user dictionaries (application- and project-level) and custom `.dic` files; quick-fixes "Typo: change to...", "Save to dictionary". | — | — | Settings \| Editor \| Natural Languages \| Spelling | P0 |
| PLATFORM-EDIT-141 | Grammar & style checking (Grazie, bundled plugin) | Natural-language grammar/style rules in comments, docs, commits and text files; multiple languages downloadable on demand; per-rule toggles; scope selection (comments, literals, commit messages). Grazie Pro adds AI-backed features (P2). | — | — | Settings \| Editor \| Natural Languages \| Grammar and Style | P1 |
| PLATFORM-EDIT-142 | TODO patterns and filters | Regex patterns (defaults `\btodo\b`, `\bfixme\b`, case-insensitive) matched in comments; custom highlight colour/icon per pattern; named filters combining patterns; multi-line TODO continuation by indentation. | — | — | Settings \| Editor \| TODO | P0 |
| PLATFORM-EDIT-143 | Language injection framework | Embeds one language inside another's string literal or region (e.g. SQL/regex in a string) via annotations, comments (`language=...`) or "Inject language or reference" intention; fragment editor opens injected text in a separate editor. Generic mechanism is platform (IntelliLang plugin bundled). | Alt+Enter | Opt+Enter | Intention; Settings \| Editor \| Language Injections | P1 |
| PLATFORM-EDIT-144 | Inspections framework & highlighting levels | Background "daemon" analysis producing highlights by severity (Error, Warning, Weak Warning, Server Problem, Typo, Grammar error, custom); per-file highlighting level (none / syntax / all / essential) via status widget (`PopupHector`); inspection profiles (IDE/project) with scopes; suppression. | Ctrl+Alt+Shift+H | Cmd+Opt+Shift+H | Settings \| Editor \| Inspections; inspection widget | P0 |
| PLATFORM-EDIT-145 | Run inspections / Inspect Code / code cleanup (`RunInspection`, `CodeInspection.OnEditor`) | Batch analysis over a scope with results in the Problems tool window ("Inspection" tab), quick-fix-all, exporting results. Run a single inspection by name. | Ctrl+Alt+Shift+I; Alt+Shift+I | Cmd+Opt+Shift+I; Opt+Shift+I | Code \| Inspect Code / Analyze Code | P1 |
| PLATFORM-EDIT-146 | Navigate between problems (`GotoNextError`, `GotoPreviousError`) | Jumps to next/previous highlighted problem in file; option to prioritise errors over warnings. | F2 / Shift+F2 | F2 / Shift+F2 | Navigate \| Next/Previous Highlighted Error | P0 |
| PLATFORM-EDIT-147 | File types & syntax highlighting fallback via TextMate bundles (plugin) | TextMate grammars give highlighting and basic brace matching for languages lacking native support; users can add bundles. | — | — | Settings \| Editor \| TextMate Bundles | P1 |
| PLATFORM-EDIT-148 | Language Server Protocol client (platform LSP API) | Platform-provided LSP client used by plugins to supply completion, diagnostics, navigation, formatting for languages without native support (since 2023.2, extended in later releases). | — | — | — (plugin-provided) | P0 |

### 1.9 Editor view modes

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-EDIT-150 | Distraction-free mode (`ToggleDistractionFreeMode`) | Hides tool windows, toolbars, tabs, gutter adornments; centres the editor text column. | — | — | View \| Appearance \| Enter Distraction-Free Mode | P1 |
| PLATFORM-EDIT-151 | Zen mode (`ToggleZenMode`) | Distraction-free plus full screen. | — | — | View \| Appearance \| Enter Zen Mode | P1 |
| PLATFORM-EDIT-152 | Presentation mode (`TogglePresentationMode`) | Full-screen editor with large configurable font size for demos; tool windows hidden. | — | — | View \| Appearance \| Enter Presentation Mode | P2 |
| PLATFORM-EDIT-153 | Full screen (`ToggleFullScreen`) | Native OS full screen for the frame. | — | Ctrl+Cmd+F | View \| Appearance \| Enter Full Screen | P1 |
| PLATFORM-EDIT-154 | Presentation assistant (`TogglePresentationAssistantAction`) | Overlay showing the name and shortcut of each invoked action (for demos/screencasts). | — | — | View \| Appearance \| Presentation Assistant | P2 |
| PLATFORM-EDIT-155 | Reader mode | Library/read-only files render with rendered doc comments, ligatures and reduced highlighting noise by default; toggle in editor corner. | — | — | Settings \| Editor \| Reader Mode | P2 |

### Sources (Editor core)

- https://www.jetbrains.com/help/idea/working-with-source-code.html
- https://www.jetbrains.com/help/idea/multicursor.html
- https://www.jetbrains.com/help/idea/working-with-code-selection.html (inferred URL, extend/shrink selection)
- https://www.jetbrains.com/help/idea/working-with-clipboard.html (inferred URL, Paste from history)
- https://www.jetbrains.com/help/idea/code-folding.html
- https://www.jetbrains.com/help/idea/using-code-editor.html
- https://www.jetbrains.com/help/idea/configuring-line-endings-and-line-separators.html
- https://www.jetbrains.com/help/idea/encoding.html
- https://www.jetbrains.com/help/idea/editorconfig.html
- https://www.jetbrains.com/help/idea/local-history.html
- https://www.jetbrains.com/help/idea/using-macros-in-the-editor.html
- https://www.jetbrains.com/help/idea/scratches.html
- https://www.jetbrains.com/help/idea/finding-and-replacing-text-in-file.html
- https://www.jetbrains.com/help/idea/reformat-and-rearrange-code.html
- https://www.jetbrains.com/help/idea/using-live-templates.html
- https://www.jetbrains.com/help/idea/surrounding-blocks-of-code-with-language-constructs.html
- https://www.jetbrains.com/help/idea/spellchecking.html and https://www.jetbrains.com/help/idea/grammar.html
- https://www.jetbrains.com/help/idea/using-todo.html
- https://www.jetbrains.com/help/idea/using-language-injections.html
- https://www.jetbrains.com/help/idea/editor-modes.html (Zen / Distraction-free / Presentation)
- https://www.jetbrains.com/help/idea/reader-mode.html
- https://plugins.jetbrains.com/docs/intellij/editors.html, https://plugins.jetbrains.com/docs/intellij/language-server-protocol.html
- Keymap descriptors (Apache-2.0): IntelliJ Community `platform/platform-resources/src/keymaps/$default.xml`, `Mac OS X 10.5+.xml`; action ids from `PlatformActions.xml`, `LangActions.xml`; labels from `ActionsBundle.properties` (tag `idea/262.10968.63`).
- Parallel PyCharm pages used to confirm platform scope: https://www.jetbrains.com/help/pycharm/multicursor.html, https://www.jetbrains.com/help/pycharm/local-history.html

---
## 2. Navigation and search (NAV)

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-NAV-001 | Search Everywhere (`SearchEverywhere`) | Single popup with tabs All / Classes / Files / Symbols / Actions / Text (plus plugin tabs, e.g. Endpoints); fuzzy & camel-hump matching; `/` prefixed path matching; `:line` suffix to jump to line; results grouped with "more" links; "Include non-project items" checkbox (double press of the shortcut); filters per tab; recent-files list when empty; preview pane option; "Open in Find tool window". | Double Shift | Double Shift | Navigate \| Search Everywhere; toolbar magnifier | P0 |
| PLATFORM-NAV-002 | Find Action (`GotoAction`) | Searches all registered actions, settings options (toggleable inline), and intentions by name/synonym; shows shortcut; Enter runs. Same popup as Search Everywhere "Actions" tab. | Ctrl+Shift+A | Cmd+Shift+A | Help \| Find Action | P0 |
| PLATFORM-NAV-003 | Go to File (`GotoFile`) | Files tab of Search Everywhere: matches file name and path fragments, supports `name:line:col`, directories (trailing `/`), file-type filter. | Ctrl+Shift+N | Cmd+Shift+O | Navigate \| File | P0 |
| PLATFORM-NAV-004 | Go to Class (`GotoClass`) | Classes tab: top-level types contributed by languages, with language filter. | Ctrl+N | Cmd+O | Navigate \| Class | P0 |
| PLATFORM-NAV-005 | Go to Symbol (`GotoSymbol`) | Symbols tab: any named declaration (functions, fields, headings, keys) contributed by languages through index-backed "choose by name" contributors. | Ctrl+Alt+Shift+N | Cmd+Opt+O | Navigate \| Symbol | P0 |
| PLATFORM-NAV-006 | Find text in Search Everywhere (`TextSearchAction`) | Text tab: full-text search across project with preview, same engine as Find in Files. | Ctrl+Alt+Shift+E (inferred mapping) | Cmd+Opt+Shift+E | Search Everywhere Text tab | P1 |
| PLATFORM-NAV-007 | Go to Line:Column (`GotoLine`) | Dialog accepting `line[:column]`. | Ctrl+G | Cmd+L | Navigate \| Line/Column | P0 |
| PLATFORM-NAV-008 | Recent Files (`RecentFiles`) | Popup of recently opened files plus tool window shortcuts; type-to-filter; second press toggles "show edited only"; delete removes entries; option to include code snippet previews. | Ctrl+E | Cmd+E | View \| Recent Files | P0 |
| PLATFORM-NAV-009 | Recent Locations (`RecentLocations`) | Popup of recently visited/changed code fragments with a few lines of context each; "changed only" toggle; searchable by content. | Ctrl+Shift+E | Cmd+Shift+E | View \| Recent Locations | P1 |
| PLATFORM-NAV-010 | Switcher (`Switcher`) | Hold modifier + Tab to cycle open files and tool windows; release to switch. | Ctrl+Tab / Ctrl+Shift+Tab | Ctrl+Tab / Ctrl+Shift+Tab | Window \| Switcher (inferred) | P0 |
| PLATFORM-NAV-011 | Back / Forward (`Back`, `Forward`) | Navigation history of caret positions across files (jump-type moves recorded, not every caret step); mouse buttons 4/5. | Ctrl+Alt+Left / Right | Cmd+[ / Cmd+] | Navigate \| Back / Forward | P0 |
| PLATFORM-NAV-012 | Last Edit Location (`JumpToLastChange`) | Jumps back through positions where edits happened. | Ctrl+Shift+Backspace | Cmd+Shift+Backspace | Navigate \| Last Edit Location | P0 |
| PLATFORM-NAV-013 | Go to Declaration or Usages (`GotoDeclaration`) | Navigates to declaration; from a declaration shows usages popup. Ctrl+Click hyperlink navigation with hover underline. Language-provided targets, generic for file references. | Ctrl+B / Ctrl+Click | Cmd+B / Cmd+Click | Navigate \| Declaration or Usages | P0 |
| PLATFORM-NAV-014 | Go to Implementation / Type Declaration / Super / Related / Test (`GotoImplementation`, `GotoTypeDeclaration`, `GotoSuperMethod`, `GotoRelated`, `GotoTest`) | Framework navigation actions with language providers; multiple targets show a chooser popup. | Ctrl+Alt+B; Ctrl+Shift+B; Ctrl+U; Ctrl+Alt+Home; Ctrl+Shift+T | Cmd+Opt+B; Cmd+Shift+B; Cmd+U; Ctrl+Cmd+Up; Cmd+Shift+T | Navigate menu | P1 |
| PLATFORM-NAV-015 | Bookmarks: line bookmarks (`ToggleBookmark`) | Toggle an anonymous line bookmark (gutter icon); bookmarks persist per project and survive edits by tracking the line. | F11 | F3 | Edit \| Bookmarks; gutter | P0 |
| PLATFORM-NAV-016 | Mnemonic bookmarks (`ToggleBookmarkWithMnemonic`, `ToggleBookmark0..9`, `GotoBookmark0..9`, `ShowTypeBookmarks`) | Bookmarks labelled with a digit or letter; digits have direct toggle/jump shortcuts; a grid popup chooses mnemonic. | Ctrl+F11; Ctrl+Shift+0..9 (set); Ctrl+0..9 (go); Ctrl+Shift+F11 (go to mnemonic) | Opt+F3; Ctrl+Shift+0..9; Ctrl+0..9; Cmd+Opt+F3 | Edit \| Bookmarks | P1 |
| PLATFORM-NAV-017 | Bookmarks lists & tool window (`ShowBookmarks`, `GotoNextBookmark`, `GotoPreviousBookmark`, `ActivateBookmarksToolWindow`) | Bookmarks can be files, folders, lines, or tree nodes; organised into named lists (default list = project name); descriptions; next/previous in file; popup and tool window views. Replaced the older "Favorites" concept. | Shift+F11 (popup); Alt+2 (tool window) | Cmd+F3; Cmd+2 | Edit \| Bookmarks; View \| Tool Windows \| Bookmarks | P1 |
| PLATFORM-NAV-018 | Find in Files (`FindInPath`) | Non-modal popup: search text with Match Case / Words / Regex; scope = In Project / Module / Directory (with recursive toggle) / Scope (named scopes); file mask filter (e.g. `*.rs,!*.test.rs`); context filter (anywhere, comments, string literals, except...); live result list with preview editor; pin popup; "Open in Find Window" creates a persistent tab; search history; result limits with "show more". | Ctrl+Shift+F | Cmd+Shift+F | Edit \| Find \| Find in Files | P0 |
| PLATFORM-NAV-019 | Replace in Files (`ReplaceInPath`) | Same popup with replace field; Replace / Replace All with confirmation; per-occurrence preview in Find tool window with exclude/include; regex groups and case-preserve. | Ctrl+Shift+R | Cmd+Shift+R | Edit \| Find \| Replace in Files | P0 |
| PLATFORM-NAV-020 | Find tool window (`ActivateFindToolWindow`) | Hosts persistent usage/search result tabs: tree grouped by module/directory/file (toggles), preview pane, merge same-line usages, filter read/write/import usages, export to text file, rerun, next/prev occurrence (`NextOccurence`/`PreviousOccurence`). | Alt+3; Ctrl+Alt+Down/Up | Cmd+3; Cmd+Opt+Down/Up | View \| Tool Windows \| Find | P0 |
| PLATFORM-NAV-021 | Find Usages framework (`FindUsages`, `ShowUsages`, `FindUsagesInFile`, `ShowSettingsAndFindUsages`) | Finds references to the element at caret through language reference providers; results in Find tool window or quick popup (Show Usages) with in-place preview, scope switcher and grouping; options dialog for scope, text occurrences, etc. Usages counted as read/write/import with grouping actions (`UsageGrouping.*`, `UsageFiltering.*`). | Alt+F7; Ctrl+Alt+F7; Ctrl+F7; Ctrl+Alt+Shift+F7 | Opt+F7; Cmd+Opt+F7; Cmd+F7; Cmd+Opt+Shift+F7 | Edit \| Find Usages | P0 |
| PLATFORM-NAV-022 | Structural Search and Replace (`StructuralSearchPlugin.StructuralSearchAction`, `...StructuralReplaceAction`) (plugin) | Template-based search on syntax trees with typed variables and constraints; framework is generic but only languages that implement the SSR profile (Java, Kotlin, JS, XML/HTML, Groovy, ...) are supported. Saved templates can become custom inspections. | — | — | Edit \| Find \| Search Structurally | P2 |
| PLATFORM-NAV-023 | Navigation bar (`ShowNavBar`) | Breadcrumb-like bar of the path from project root to current file (and member, when enabled); each segment is a popup for browsing; can appear in toolbar, as standalone bar, or in the status bar (new UI default). | Alt+Home | Cmd+Up / Opt+Home | View \| Appearance \| Navigation Bar | P1 |
| PLATFORM-NAV-024 | File Structure popup (`FileStructurePopup`) | Speed-searchable popup of the current file's outline (language structure view model) with toggles (inherited members, fields, sort alphabetically). | Ctrl+F12 | Cmd+F12 | Navigate \| File Structure | P0 |
| PLATFORM-NAV-025 | Structure tool window (`ActivateStructureToolWindow`) | Persistent outline of current editor's file with sort/group/filter toggles, autoscroll to/from source, and for some languages logical views. Generic for any file type that supplies a structure view. | Alt+7 | Cmd+7 | View \| Tool Windows \| Structure | P0 |
| PLATFORM-NAV-026 | Next/Previous member (`MethodDown`, `MethodUp`) | Jumps between structure elements in the file. | Alt+Down / Alt+Up | Ctrl+Shift+Down / Up | Navigate menu | P1 |
| PLATFORM-NAV-027 | Type Hierarchy framework (`TypeHierarchy`) | Hierarchy tool window with supertype/subtype/full views, scope selector, auto-scroll; language-provided. | Ctrl+H | Ctrl+H | Navigate \| Type Hierarchy | P1 |
| PLATFORM-NAV-028 | Call Hierarchy framework (`CallHierarchy`) | Callers/callees tree, scope selector, expansion on demand; language-provided. | Ctrl+Alt+H | Ctrl+Opt+H | Navigate \| Call Hierarchy | P1 |
| PLATFORM-NAV-029 | Method/member hierarchy framework (`MethodHierarchy`) | Overriding-member hierarchy, language-provided. | Ctrl+Shift+H | Cmd+Shift+H | Navigate \| Method Hierarchy | P2 |
| PLATFORM-NAV-030 | Select In (`SelectIn`) | Popup of targets to reveal the current file/element: Project view, Structure, file manager ("Reveal in Explorer/Finder"), Commit tool window, terminal, etc. | Alt+F1 | Opt+F1 | Navigate \| Select In | P0 |
| PLATFORM-NAV-031 | Jump to Source / Show Source (`EditSource`, `ViewSource`) | From any tree/list node opens the element in editor with focus (F4) or without focus. | F4; Ctrl+Enter | Cmd+Down / F4; Cmd+Enter | Context menus | P0 |
| PLATFORM-NAV-032 | Speed search in trees/lists/popups | Typing in any tree, list or table filters/highlights matches with camel-hump matching; Up/Down cycles matches. | type to search | type to search | All trees/lists | P0 |
| PLATFORM-NAV-033 | File Path popup (`ShowFilePath`) | Shows path segments of current file; choosing one opens it in the OS file manager. | Ctrl+Alt+F12 | Cmd+Opt+F12 | Find Action | P2 |
| PLATFORM-NAV-034 | VCS change navigation in editor (`VcsShowNextChangeMarker`, `VcsShowPrevChangeMarker`) | Moves between changed hunks shown in the gutter. | Ctrl+Alt+Shift+Down/Up | Ctrl+Opt+Shift+Down/Up | Gutter change popup | P1 |
| PLATFORM-NAV-035 | Text search in libraries / non-project files | Find in Files and Search Everywhere can include libraries and excluded folders via scope options. | — | — | Scope selector | P2 |
| PLATFORM-NAV-036 | Go to by Reference prefix filters (inferred) | Search Everywhere supports `#` prefix for settings/inspections in Actions tab and `/` prefix to run commands (e.g. `/settings`) in some tabs. | — | — | Search Everywhere | P2 |
| PLATFORM-NAV-037 | Context-aware "Go to Changed File" (`GotoChangedFile`) | Lists locally changed files in a quick chooser. | — | — | Find Action | P2 |

### Sources (Navigation and search)

- https://www.jetbrains.com/help/idea/searching-everywhere.html
- https://www.jetbrains.com/help/idea/navigating-through-the-source-code.html
- https://www.jetbrains.com/help/idea/bookmarks.html
- https://www.jetbrains.com/help/idea/finding-and-replacing-text-in-project.html
- https://www.jetbrains.com/help/idea/find-usages.html (inferred URL; topic "Search for usages")
- https://www.jetbrains.com/help/idea/structural-search-and-replace.html
- https://www.jetbrains.com/help/idea/navigation-bar.html
- https://www.jetbrains.com/help/idea/structure-tool-window-file-structure-popup.html
- https://www.jetbrains.com/help/idea/viewing-structure-and-hierarchy-of-the-source-code.html
- https://www.jetbrains.com/help/pycharm/searching-everywhere.html (platform confirmation)
- https://plugins.jetbrains.com/docs/intellij/find-usages.html, https://plugins.jetbrains.com/docs/intellij/structure-view.html, https://plugins.jetbrains.com/docs/intellij/go-to-class-and-go-to-symbol.html
- Keymap/action descriptors as listed in section 1 sources.

---

## 3. Project model and files (PROJ)

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-PROJ-001 | Welcome screen | Shown when no project is open: recent projects list (search, groups, pin, remove), New Project, Open, Get from VCS, Plugins, Customize (theme, font, keymap, accessibility), Learn (IDE Features Trainer). A non-modal welcome editor variant also exists in recent builds (inferred from `non-modal-welcome-screen` module). | — | — | App start; File \| Close Project | P0 |
| PLATFORM-PROJ-002 | Open project (`OpenFile`) | Opens a directory (directory-based project), an `.idea`-containing folder, or a build file (offering "open as project"); single files open in "LightEdit" mode when launched from CLI (inferred). Option to open in new window / this window / attach to current (multi-root) per System settings. | — | — | File \| Open... | P0 |
| PLATFORM-PROJ-003 | New project wizard (`NewProject`) | Generator list (Empty project and language/framework generators contributed by plugins), location, VCS init option. | — | — | File \| New \| Project | P1 |
| PLATFORM-PROJ-004 | Recent projects (`ManageRecentProjects`, File \| Recent Projects) | MRU list with groups and project icons; opening reuses or creates a window; removal; reopen last project(s) on startup per setting. | — | — | File \| Recent Projects; project widget in toolbar | P0 |
| PLATFORM-PROJ-005 | Project widget / multiple windows (`NextProjectWindow`, `PreviousProjectWindow`) | Toolbar project switcher; each project gets its own frame (or tab on macOS native tabs); cycling between open project windows. | Ctrl+Alt+] / [ | Cmd+Opt+` / Cmd+Opt+Shift+` | Window menu | P0 |
| PLATFORM-PROJ-006 | Close project (`CloseProject`) | Saves state and closes the frame (returns to welcome screen if last). | — | — | File \| Close Project | P0 |
| PLATFORM-PROJ-007 | Trusted projects / Safe mode | On first open of an untrusted location the IDE asks to trust; untrusted (preview/safe mode) disables build-tool import, run configs, plugins' auto-execution. Trusted locations list in settings. | — | — | Dialog; Settings \| Build, Execution, Deployment \| Trusted Locations | P0 |
| PLATFORM-PROJ-008 | Project storage format | Directory-based `.idea/` with per-component XML files (workspace.xml local-only, shareable others); `.iml` module files; gitignore suggestions. Archipel IDE needs an equivalent split between shared and local state. | — | — | — | P0 |
| PLATFORM-PROJ-009 | Project tool window (`ActivateProjectToolWindow`) | Tree of project files with views and per-view options; file operations (new, rename, copy/move by drag, delete to trash), external drops, compact display of single-child directories. | Alt+1 | Cmd+1 | View \| Tool Windows \| Project | P0 |
| PLATFORM-PROJ-010 | Project view: views | Views selector: Project, Packages (JVM), Project Files (raw file system relative to roots), Open Files, Scratches and Consoles, Problems (files with errors), Changed Files (VCS), Production/Test (inferred), plus each named scope as a view. | — | — | Project tool window title dropdown | P0 (Project, Project Files), P1 (rest) |
| PLATFORM-PROJ-011 | Project view: options | Toggle: Autoscroll to Source, Autoscroll from Source (Always Select Opened File), Flatten Packages, Compact Middle Packages, Show/Hide Empty Middle Packages, Show Members, Show Excluded Files, Show Ignored Files (inferred), Show Visibility Icons, File Nesting rules, Sort by Type, Folders Always on Top, Open Files with Single Click, Enable Preview Tab, Abbreviate Qualified Package Names. "Select Opened File" button. | — | — | Project tool window gear/options menu | P0 (autoscroll, select opened), P1 (rest) |
| PLATFORM-PROJ-012 | File nesting | Rules (`parent suffix -> child suffixes`) that visually nest generated/related files under a parent file (e.g. `.ts` -> `.js; .js.map`). | — | — | Project view options \| File Nesting | P2 |
| PLATFORM-PROJ-013 | Preview tab | Single-click opens a reusable italic "preview" tab replaced on next single-click until edited or pinned. | — | — | Project view option | P1 |
| PLATFORM-PROJ-014 | Modules and content roots (`ShowProjectStructureSettings`) | A project contains modules; each has content roots with folder marks: Sources, Test Sources, Resources, Test Resources, Excluded; plus exclude patterns (file name globs). Language-agnostic mark actions in Project view "Mark Directory as". Build-tool importers populate the same model. | Ctrl+Alt+Shift+S | Cmd+; | File \| Project Structure; Project view context \| Mark Directory as | P0 |
| PLATFORM-PROJ-015 | Excluded folders & ignored files | Excluded folders are omitted from indexing/search/inspections; global ignore list of file/dir name patterns (e.g. `.git`, `__pycache__`, `*.pyc`; exact defaults (inferred)) in File Types settings. | — | — | Settings \| Editor \| File Types \| Ignored Files and Folders | P0 |
| PLATFORM-PROJ-016 | Libraries / SDKs model | Project/global libraries and SDK table (generic; language plugins define types). | — | — | Project Structure | P1 |
| PLATFORM-PROJ-017 | Workspace model (multi-root, attached projects) | Attach additional directories to a project window ("attach" option on open) for multi-project workspaces (in PyCharm/WebStorm "attach project" ; in IDEA via modules). | — | — | File \| Open (Attach) | P2 |
| PLATFORM-PROJ-018 | Scopes | Named sets of files defined by a pattern language (`file:`, `src:`, `lib:`, `test:` and `\|\|`, `&&`, `!`), shared (project) or local; predefined scopes (Project Files, Production, Tests, Open Files, Changed Files, Current File, Selection, VCS scopes, Recently Viewed/Changed). Used by Find, inspections, Project view, code style, copyright, file colours. | — | — | Settings \| Appearance & Behavior \| Scopes | P0 |
| PLATFORM-PROJ-019 | File colors | Background colours for files/tabs by scope (e.g. tests green, non-project yellow). | — | — | Settings \| Appearance & Behavior \| File Colors | P2 |
| PLATFORM-PROJ-020 | File types and associations | Registry of file types with filename patterns (globs and exact names), hashbang detection, "Override File Type" per file, "Associate with File Type" prompt for unknown files, user-defined file types with keyword sets/comment syntax/brace pairs for basic highlighting. | — | — | Settings \| Editor \| File Types; context \| Override File Type | P0 |
| PLATFORM-PROJ-021 | Open file in associated application / system (`ShowLog` sibling actions; "Open In") | Open in OS default app, Reveal in file manager, Open in terminal, Open in browser. | — | — | Context \| Open In | P1 |
| PLATFORM-PROJ-022 | File Watchers (plugin) | Per-project rules that run an external tool when files of a type change (compilers, transpilers, linters): program, args with macros, output paths refresh, scope, trigger on save/any change, show console on error. | — | — | Settings \| Tools \| File Watchers | P2 |
| PLATFORM-PROJ-023 | Virtual file system & refresh | Snapshot of the file system with native watcher; refresh on frame activation, explicit Reload from Disk; handles case sensitivity per directory and symlinks. | Ctrl+Alt+Y | Cmd+Opt+Y | File \| Reload All from Disk | P0 |
| PLATFORM-PROJ-024 | Indexing and dumb mode | Background indexing of file contents (word index, stub/symbol indexes, ID index); while indexing ("dumb mode") index-dependent features are disabled or show "not available during indexing"; status bar progress with pause; "smart" features resume automatically. Partial support during indexing for Search Everywhere files tab (inferred). | — | — | Status bar progress | P0 |
| PLATFORM-PROJ-025 | Shared indexes | Downloadable prebuilt indexes for JDKs/libraries (public CDN) and project shared indexes (generated via CLI tool, hosted by team); consent prompt; settings to auto-download or ask. | — | — | Settings \| Tools \| Shared Indexes | P2 |
| PLATFORM-PROJ-026 | Power Save Mode (`TogglePowerSave`) | Disables background code analysis, auto-import hints and other background activities to save battery; status shown in status bar. | — | — | File \| Power Save Mode | P1 |
| PLATFORM-PROJ-027 | Invalidate Caches / Repair IDE (`InvalidateCaches`, "Repair IDE") | Dialog clearing caches/indexes (options: clear file system cache & Local History, clear VCS log caches, clear downloaded shared indexes, delete embedded browser engine cache) then restart. "Repair IDE" runs a staged recovery (rescan project indexes, reopen project, drop shared indexes, reindex, invalidate & restart). | — | — | File \| Invalidate Caches...; File \| Repair IDE | P1 |
| PLATFORM-PROJ-028 | Reload/rescan project model from build tools (`ExternalSystem.ProjectRefreshAction`) | Generic "external system" framework (Gradle, Maven, sbt, Cargo via plugins) to import and resync a project model; floating "Load changes" button when build files change; auto-reload setting. | Ctrl+Shift+O | Cmd+Shift+I | Build tool window; floating editor button | P1 |
| PLATFORM-PROJ-029 | Copyright profiles (plugin) | Templates of license headers inserted/updated per scope; uses Velocity variables. | — | — | Settings \| Editor \| Copyright | P2 |
| PLATFORM-PROJ-030 | Project-level templates (File and Code Templates) | Templates for new files (Velocity-like variables, includes), per IDE or per project; "New" menu entries derive from them. | — | — | Settings \| Editor \| File and Code Templates | P1 |
| PLATFORM-PROJ-031 | New file/directory actions (`NewElement`, `NewElementSamePlace`) | Popup to create files, directories, scratch files, and template-based files in the selected location; path in directory name creates intermediate dirs. | Alt+Insert; Ctrl+Alt+Insert | Cmd+N; Ctrl+Opt+N | File \| New | P0 |
| PLATFORM-PROJ-032 | Save project as template / export project as zip (inferred) | Export the project to a zip archive; "Save Project as Template" exists in some products. | — | — | File \| Export | P2 |
| PLATFORM-PROJ-033 | Tasks and contexts (`tasks.switch`, `context.save/load/clear`) (plugin) | Save/restore sets of open editors/breakpoints/bookmarks as named contexts; link to issue tracker tasks and branches. | Alt+Shift+T; Alt+Shift+S/L/X | Opt+Shift+T; —; —; Opt+Shift+X | Tools \| Tasks & Contexts | P2 |

### Sources (Project model)

- https://www.jetbrains.com/help/idea/project-tool-window.html
- https://www.jetbrains.com/help/idea/creating-and-managing-projects.html
- https://www.jetbrains.com/help/idea/project-security.html (trusted projects)
- https://www.jetbrains.com/help/idea/content-roots.html
- https://www.jetbrains.com/help/idea/settings-scopes.html
- https://www.jetbrains.com/help/idea/creating-and-registering-file-types.html
- https://www.jetbrains.com/help/idea/using-file-watchers.html
- https://www.jetbrains.com/help/idea/indexing.html
- https://www.jetbrains.com/help/idea/shared-indexes.html
- https://www.jetbrains.com/help/idea/invalidate-caches.html
- https://www.jetbrains.com/help/idea/file-and-code-templates.html
- https://www.jetbrains.com/help/pycharm/project-tool-window.html (platform confirmation)
- https://plugins.jetbrains.com/docs/intellij/virtual-file-system.html, https://plugins.jetbrains.com/docs/intellij/indexing-and-psi-stubs.html, https://plugins.jetbrains.com/docs/intellij/project-model.html

---
## 4. Version control (VCS)

Git is the reference VCS (bundled `git4idea` plugin); the generic VCS framework also hosts
Mercurial, Subversion and Perforce plugins (P2).

### 4.1 Local changes and commit

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-VCS-001 | VCS root detection & enable integration (`Start.Use.Vcs`, `Git.Init`) | Detects repositories under project; prompts to register unregistered roots; create repository; directory mappings table. | — | — | Settings \| Version Control \| Directory Mappings; VCS \| Create Git Repository | P0 |
| PLATFORM-VCS-002 | Clone (`Vcs.VcsClone`, `Git.Clone`) | "Get from VCS" dialog: URL or hosted account repositories (GitHub/GitLab/...), target directory, shallow clone option (inferred), then open. | — | — | Welcome screen; File \| New \| Project from Version Control | P0 |
| PLATFORM-VCS-003 | Commit tool window (`ActivateCommitToolWindow`, `CheckinProject`) | Non-modal commit UI: changes tree (grouping by directory/module/repository), checkboxes and per-hunk/per-line inclusion, commit message editor with history and spell check, Commit / Commit and Push, Amend toggle, commit checks. A legacy modal commit dialog is available as an option. | Alt+0 (focus); Ctrl+K (commit) | Cmd+0; Cmd+K | View \| Tool Windows \| Commit; Git \| Commit | P0 |
| PLATFORM-VCS-004 | Commit and Push (`Git.Commit.And.Push.Executor`) | Commits then opens Push dialog (or pushes silently per option). | Ctrl+Alt+K | Cmd+Opt+K | Commit tool window button | P0 |
| PLATFORM-VCS-005 | Amend commit (`Vcs.ToggleAmendCommitMode`) | Toggle that loads last commit's message and amends. | Alt+M | Ctrl+Opt+M | Commit tool window | P0 |
| PLATFORM-VCS-006 | Commit message history & templates (`Vcs.ShowMessageHistory`) | Recent messages popup; Git commit template support; right-margin and subject-length checks; blank-line-after-subject inspection. | Ctrl+M | Ctrl+M | Commit message field | P1 |
| PLATFORM-VCS-007 | Commit checks & options (`ChangesView.ShowCommitOptions`, `Vcs.RunCommitChecks`) | Pre-commit: reformat, rearrange, optimize imports, analyze code, check TODO, cleanup, run tests / run configuration, update copyright; post-commit actions; author override, sign-off, GPG signing; checks run before or after commit (non-blocking "run in background"). | Ctrl+O (in commit UI) | Ctrl+O | Commit tool window gear | P1 |
| PLATFORM-VCS-008 | Changelists (`ChangesView.NewChangeList`, `ChangesView.SetDefault`, `ChangesView.Move`, `Vcs.MoveChangedLinesToChangelist`, `ChangesView.Rename`, `ChangesView.RemoveChangeList`) | Named groups of local changes, one active list receives new edits; move files or individual changed lines between lists; commit a list independently. Mutually exclusive with Git staging-area mode. | Ctrl+Space (set active); Alt+Shift+M (move); F2 (edit) | Ctrl+Space; Cmd+Shift+M; F2 | Commit tool window | P1 |
| PLATFORM-VCS-009 | Git staging area mode (`Git.Stage.Enable`, `Git.Stage.*`) | Optional mode showing Staged/Unstaged/Unversioned trees; stage/unstage files, hunks and lines; 3-way compare HEAD/Staged/Local; "commit all when nothing staged" option. | — | — | Settings \| Version Control \| Git \| Enable staging area | P1 |
| PLATFORM-VCS-010 | Unversioned & ignored files (`ChangesView.AddUnversioned`, `ChangesView.ShowIgnored`) | Unversioned node with Add to VCS; ignored files node (from `.gitignore`); "Add to .gitignore" action; `.gitignore` language support. | Ctrl+Alt+A | Cmd+Opt+A | Commit tool window | P0 |
| PLATFORM-VCS-011 | Rollback / revert local changes (`ChangesView.Revert`, `Vcs.RollbackChangedLines`) | Revert selected files, or the hunk at caret from the gutter; option to delete added files. | Ctrl+Alt+Z | Cmd+Opt+Z | Commit tool window; gutter popup | P0 |
| PLATFORM-VCS-012 | Gutter change markers | Coloured gutter bars for added/modified/deleted lines vs HEAD; click shows popup with inline diff, rollback, copy old text, go next/prev change, show diff; option to highlight changed text in-line. | Ctrl+Alt+Shift+Up/Down (nav) | Ctrl+Opt+Shift+Up/Down | Editor gutter | P0 |
| PLATFORM-VCS-013 | Shelf (`ChangesView.Shelve`, `ChangesView.ShelveSilently`, `ShelveChanges.UnshelveWithDialog`, `ChangesView.UnshelveSilently`, `Vcs.Show.Shelf`) | VCS-agnostic stash stored as patches in project; shelve whole files or partial; unshelve into a changelist with conflict merge; restore deleted shelves; rename; import patches. | Ctrl+Shift+H (shelve silently); Ctrl+Shift+U (unshelve); Ctrl+Alt+U (unshelve silently) | Cmd+Shift+H; Cmd+Shift+U; Cmd+Opt+U | Commit tool window \| Shelf tab | P1 |
| PLATFORM-VCS-014 | Git stash (`Git.Stash`, `Git.Stash.Silently`, `Git.Unstash`, `Git.Show.Stash`, `Git.Stash.Apply/Pop/Drop/Clear/UnstashAs`) | Create stash with message (optionally keep index), list stashes in a Stash tab with diff preview, apply/pop/drop/clear, unstash as new branch. | — | — | Git \| Uncommitted Changes \| Stash Changes | P1 |
| PLATFORM-VCS-015 | Patches (`ChangesView.CreatePatch`, `ChangesView.CreatePatchToClipboard`, `ChangesView.ApplyPatch`, `ChangesView.ApplyPatchFromClipboard`) | Create unified diff patch from changes or commits (file or clipboard); apply patch with preview, base-revision matching, and conflict resolution. | — | — | Git \| Patch | P1 |

### 4.2 Diff and merge

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-VCS-020 | Diff viewer (`Diff.ShowDiff`, `Vcs.Diff.ShowDiffInEditorTab`, `Vcs.Diff.ShowDiffInNewWindow`) | Side-by-side (two editable editors where possible), unified view, with modes: ignore whitespace (several levels), highlighting by line/word/char/none, collapse unchanged fragments, sync scroll, align changes, show in editor tab or separate window, syntax highlighting and navigation in both panes, apply/revert chunk arrows. | Ctrl+D | Cmd+D | Any changes list; context \| Show Diff | P0 |
| PLATFORM-VCS-021 | Diff navigation (`NextDiff`, `PreviousDiff`, `Diff.NextChange`, `Diff.PrevChange`, `Diff.FocusOppositePane`) | Next/previous change (continues to next file at end), next/previous file, switch pane. | F7 / Shift+F7; Alt+Shift+Right/Left; Ctrl+Shift+Tab | F7 / Shift+F7; Ctrl+Shift+Right/Left; Ctrl+Shift+Tab | Diff toolbar | P0 |
| PLATFORM-VCS-022 | Combined (multi-file) diff (`Vcs.Diff.ShowCombinedDiff`, `Vcs.CombinedDiff.*`) | All changed files of a commit/changelist in one scrollable diff with collapsible file blocks. | Right/Left, PgUp/PgDn (blocks); Ctrl+Esc (collapse) | same; Cmd+Esc | Diff settings | P2 |
| PLATFORM-VCS-023 | Compare files / clipboard / folders (`CompareTwoFiles`, Compare with Clipboard (inferred id), directory diff `DirDiffMenu.*`) | Compare any two files, file with clipboard, editor with another editor, two directories (with sync actions). | Ctrl+D (with two selected) | Cmd+D | Context \| Compare With...; View \| Compare With | P1 |
| PLATFORM-VCS-024 | Compare with revision/branch (`Compare.LastVersion`, `Compare.Selected`, `Compare.Specified`, `Git.CompareWithBranch`, `Compare.SameVersion`) | Diff the working file/dir against HEAD, a chosen revision, or another branch/tag. | — | — | Git \| Current File \| Compare with... | P0 |
| PLATFORM-VCS-025 | 3-way merge tool (`Git.ResolveConflicts`, `Diff.ApplyLeftSide`, `Diff.ApplyRightSide`) | Three panes (yours / result / theirs) with conflicting chunks; accept left/right, ignore, "Apply all non-conflicting changes" (from left/right/all), magic-wand auto-resolve of simple conflicts, import-conflict resolution, result editable with highlighting. Conflicts dialog lists files with Accept Yours/Theirs/Merge. | Ctrl+Alt+R / Ctrl+Alt+A (accept left/right) | Ctrl+Cmd+Right / Left | Git \| Resolve Conflicts; notifications | P0 |
| PLATFORM-VCS-026 | Streamlined conflict resolution flow (2026.2) | Release blog mentions a reworked conflict-resolution flow in 2026.2; specifics not documented publicly at time of writing (see Open questions). | — | — | — | P1 |
| PLATFORM-VCS-027 | External diff/merge tools | Configure external diff and merge executables per file type. | — | — | Settings \| Tools \| Diff & Merge \| External Diff Tools | P2 |

### 4.3 History, blame, log

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-VCS-030 | Annotate / blame (`Annotate`, `Vcs.Operations.Popup.Annotate`) | Gutter columns with author, date, revision (configurable, colour by age/author); hover shows commit message; click opens commit in log; "annotate previous revision"; options to ignore whitespace/detect moves. Inline blame of the caret line as code vision/inlay is also available (inferred from recent releases). | — | — | Gutter context \| Annotate with Git Blame | P0 |
| PLATFORM-VCS-031 | File/selection history (`Vcs.ShowTabbedFileHistory`, `Vcs.ShowHistoryForBlock`) | History tab for a file (with renames) or for selected lines; diff per revision, open revision, revert. | — | — | Git \| Current File \| Show History | P0 |
| PLATFORM-VCS-032 | Git log tool window (`Vcs.Show.Log`, `Git.Log`) | Graph of commits with branch labels; filters by branch, user, date, path, text/hash (regex, match case); details panel with changed files and diff preview (right/bottom); multiple log tabs; per-repository colouring; long-edge toggle; go to hash/branch/tag; persistent index for fast filtering (VCS log index, can be disabled for big repos). | Alt+9 (focus); Ctrl+L (focus filter in log) | Cmd+9; Cmd+L | View \| Tool Windows \| Git (Log tab) | P0 |
| PLATFORM-VCS-033 | Log commit actions | Checkout revision, new branch/tag from commit, cherry-pick, revert commit, reset current branch to here (soft/mixed/hard/keep), undo commit, edit message (reword), fixup/squash into, drop, interactive rebase from here, push all up to here, compare versions, show repository at revision, copy hash. | F2 (reword); Ctrl+Alt+N (new branch) | F2; Cmd+Opt+N | Log context menu | P1 |
| PLATFORM-VCS-034 | Interactive rebase editor (`Git.Interactive.Rebase`) | Table of commits with pick/edit/reword/squash/fixup/drop, reorder by drag or Alt+Up/Down (inferred), message editing, start; pauses surface Continue/Abort/Skip actions. | — | — | Log context; Git \| Rebase | P1 |
| PLATFORM-VCS-035 | Branches popup / VCS widget (`Git.Branches`, `main.toolbar.git.Branches`) | Toolbar widget showing current branch and incoming/outgoing indicators; popup with search, recent branches, local/remote groups (tree by prefix), favourites, per-branch actions: checkout, new branch from, checkout and rebase onto current, compare, show diff with working tree, rebase/merge into current, push, rename, delete, update; multi-repo synchronous control. | Ctrl+Shift+` | — (widget) | Toolbar; Git \| Branches | P0 |
| PLATFORM-VCS-036 | Create branch / tag (`Git.CreateNewBranch`, `Git.CreateNewTag`) | Dialog with name validation, checkout toggle, overwrite option. | Ctrl+Alt+N | Cmd+Opt+N | Branches popup | P0 |
| PLATFORM-VCS-037 | Git worktrees (`Git.CreateNewWorkingTree`, `Git.Show.WorkingTrees`, `Git.WorkingTrees.*`) | List, create, open as project, remove and prune linked worktrees (2026.1 highlights handing a worktree to an AI agent). | — | — | Git \| Worktrees; Git tool window tab | P2 |
| PLATFORM-VCS-038 | Clean up branches (`Git.Cleanup.Branches`, `Git.FindMergedLocalBranches`) | Find and delete merged/stale local branches. | — | — | Branches popup | P2 |

### 4.4 Remote operations and history rewriting

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-VCS-040 | Update Project (`Vcs.UpdateProject`) | Pulls all roots using merge or rebase strategy; auto-stash/shelve local changes; result tree of updated files ("Update Info" tab). | Ctrl+T | Cmd+T | Git \| Update Project | P0 |
| PLATFORM-VCS-041 | Fetch (`Git.Fetch`) | Fetch all remotes (background), optional auto-fetch interval (inferred). | — | — | Git \| Fetch; branches popup | P0 |
| PLATFORM-VCS-042 | Pull dialog (`Git.Pull`) | Choose remote/branch, options (rebase, ff-only, no-ff, squash, no-commit, no-verify). | — | — | Git \| Pull | P0 |
| PLATFORM-VCS-043 | Push dialog (`Vcs.Push`) | Lists outgoing commits per repo and target branch (editable, new branch creation), push tags (all/current branch), force-push (with protected-branch guard), "--set-upstream", commit diff preview; pre-push checks. | Ctrl+Shift+K | Cmd+Shift+K | Git \| Push | P0 |
| PLATFORM-VCS-044 | Merge (`Git.Merge`) | Dialog to merge a branch into current with options (no-ff, ff-only, squash, no-commit, message); abort merge. | — | — | Git \| Merge | P0 |
| PLATFORM-VCS-045 | Rebase (`Git.Rebase`, `Git.Rebase.Continue/Abort/Skip`) | Rebase onto branch with options (interactive, preserve/rebase merges, --onto); in-progress state shown in VCS widget with Continue/Abort. | — | — | Git \| Rebase | P1 |
| PLATFORM-VCS-046 | Cherry-pick & revert (`Vcs.CherryPick`, `Git.CherryPick.Continue/Abort`, `Git.Revert.In.Log`, `Vcs.ApplySelectedChanges`) | Apply commit(s) or selected file changes from a commit to the current branch; abort/continue on conflicts; revert commits. | — | — | Log context | P1 |
| PLATFORM-VCS-054 | Reset HEAD (`Git.Reset`) | Reset to a revision with soft/mixed/hard/keep modes. | — | — | Git \| Reset HEAD | P1 |
| PLATFORM-VCS-047 | Remotes management (`Git.Configure.Remotes`) | Add/edit/remove remotes. | — | — | Git \| Manage Remotes | P1 |
| PLATFORM-VCS-048 | VCS Operations popup (`Vcs.QuickListPopupAction`) | Quick list of VCS actions for the current context. | Alt+` | Ctrl+V | VCS menu | P0 |
| PLATFORM-VCS-049 | VCS console (`Vcs.ShowConsoleTab`) | Console tab logging each git command executed by the IDE with output. | — | — | Git tool window \| Console | P1 |
| PLATFORM-VCS-050 | Credentials & SSH | HTTP credential prompts with password-safe storage, native SSH or built-in SSH helper, GPG passphrase handling, credential helper integration. | — | — | Settings \| Appearance & Behavior \| System Settings \| Passwords | P0 |
| PLATFORM-VCS-051 | Incoming/outgoing indicators & auto-fetch (inferred) | Arrows in VCS widget for commits ahead/behind remote; branches popup marks branches needing update. | — | — | VCS widget | P1 |
| PLATFORM-VCS-052 | Git settings page | Path to git executable (auto-detected, WSL-aware), update method, commit checks, push protections (protected branches patterns), warn on CRLF, staging-area toggle, filter log by changed paths. | — | — | Settings \| Version Control \| Git | P0 |
| PLATFORM-VCS-053 | Copy commit hash / path from repository root (`Vcs.CopyRevisionNumberAction`, `CopyPathFromRepositoryRootProvider`) | Clipboard helpers. | — | — | Context menus | P2 |

### 4.5 Hosted code review integrations

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-VCS-060 | GitHub account & clone (plugin) | Login via browser/token, multiple accounts, clone from account list, share project on GitHub, create gist, open on GitHub. | — | — | Settings \| Version Control \| GitHub | P1 |
| PLATFORM-VCS-061 | GitHub Pull Requests tool window (plugin) | List/filter PRs, create PR (with templates, reviewers, labels), view timeline and description, review diff with inline comment threads, suggested changes, mark files viewed (`Github.PullRequest.Changed.MarkViewed.Toggle`), approve/request changes, merge/squash/rebase merge, checkout PR branch; editor gutter comments for checked-out PR branch. | Ctrl+Shift+S (toggle viewed) | Cmd+Shift+S | View \| Tool Windows \| Pull Requests | P1 |
| PLATFORM-VCS-062 | GitLab Merge Requests tool window (plugin) | Equivalent workflow for GitLab MRs (list, create, review, comment, approve, merge). | — | — | View \| Tool Windows \| Merge Requests | P2 |
| PLATFORM-VCS-063 | Code review editor comments (`Code.Review.Editor.New.Comment`) | Create review comment at caret line while in review mode. | Ctrl+Shift+X | Cmd+Shift+X | Editor gutter in review mode | P2 |
| PLATFORM-VCS-064 | Other hosts (Bitbucket, Gitea, Space) | Third-party/JetBrains plugins; not bundled in IDEA (inferred). | — | — | Marketplace | P2 |
| PLATFORM-VCS-065 | Other VCS plugins (Mercurial `hg4idea`, Subversion `svn4idea`, Perforce) | Same framework, VCS-specific actions. | — | — | Settings \| Version Control | P2 |

### Sources (Version control)

- https://www.jetbrains.com/help/idea/version-control-integration.html
- https://www.jetbrains.com/help/idea/commit-and-push-changes.html
- https://www.jetbrains.com/help/idea/managing-changelists.html
- https://www.jetbrains.com/help/idea/shelving-and-unshelving-changes.html
- https://www.jetbrains.com/help/idea/work-with-git-stash.html (inferred URL; topic "Stash changes")
- https://www.jetbrains.com/help/idea/comparing-file-versions.html
- https://www.jetbrains.com/help/idea/resolving-conflicts.html
- https://www.jetbrains.com/help/idea/investigate-changes.html (annotate, log)
- https://www.jetbrains.com/help/idea/log-tab.html
- https://www.jetbrains.com/help/idea/manage-branches.html
- https://www.jetbrains.com/help/idea/apply-changes-from-one-branch-to-another.html
- https://www.jetbrains.com/help/idea/edit-project-history.html
- https://www.jetbrains.com/help/idea/sync-with-a-remote-repository.html
- https://www.jetbrains.com/help/idea/github.html, https://www.jetbrains.com/help/idea/work-with-github-pull-requests.html
- https://www.jetbrains.com/help/idea/gitlab.html
- https://www.jetbrains.com/help/pycharm/commit-and-push-changes.html (platform confirmation)
- Release notes: https://blog.jetbrains.com/idea/2026/03/intellij-idea-2026-1/ (worktrees), https://blog.jetbrains.com/idea/2026/07/intellij-idea-2026-2/ (conflict resolution flow)
- Action descriptors (Apache-2.0): `platform/vcs-impl/resources/META-INF/VcsActions.xml` (path per task brief), `plugins/git4idea` resources, `platform/vcs-log` resources.

---
## 5. Run / Debug infrastructure (RUN)

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-RUN-001 | Run configuration model | Typed configurations (type contributed by plugins; generic ones include Shell Script, Compound, Docker, npm (plugin), "Run Anything" history) with name, options, environment variables (with `.env` files via plugin), working directory, macros (`$ProjectFileDir$`, ...), "allow multiple instances", "store as project file" (shared under `.run/` or `.idea/runConfigurations`), folders for grouping, temporary configs (limit on count) created from context. | — | — | Run \| Edit Configurations | P0 |
| PLATFORM-RUN-002 | Configuration templates | Per-type default values applied to new configurations; editable in Edit Configurations \| "Edit configuration templates". | — | — | Run \| Edit Configurations | P1 |
| PLATFORM-RUN-003 | Run from context (`RunClass`, `DebugClass`) | Gutter run icons and context menu "Run '...'" create a temporary configuration from the element at caret (language-provided producers). | Ctrl+Shift+F10 | Ctrl+Shift+R; Ctrl+Shift+D | Gutter; context menu | P0 |
| PLATFORM-RUN-004 | Run / Debug selected configuration (`Run`, `Debug`, `ChooseRunConfiguration`, `ChooseDebugConfiguration`) | Launch the selected configuration; chooser popup lists configs with executor switching (hold Shift/Alt to switch to debug/edit). | Shift+F10; Shift+F9; Alt+Shift+F10; Alt+Shift+F9 | Ctrl+R; Ctrl+D; Ctrl+Opt+R; Ctrl+Opt+D | Run menu | P0 |
| PLATFORM-RUN-005 | Run widget (main toolbar) | Combined selector + Run/Debug/More buttons in new UI toolbar; shows running state, Stop/Rerun, list of running processes, pinning configurations, "Current File" pseudo-configuration. | — | — | Main toolbar | P0 |
| PLATFORM-RUN-006 | Stop / Rerun (`Stop`, `Rerun`, `StopBackgroundProcesses`) | Stop current (popup when several running), graceful then forced termination on second press, Rerun in same tab; stop background processes. | Ctrl+F2; Ctrl+F5; Ctrl+Shift+F2 | Cmd+F2; Cmd+R; Cmd+Shift+F2 | Run menu; Run tool window | P0 |
| PLATFORM-RUN-007 | Before-launch tasks | Ordered tasks executed before a config: Build, Run another configuration, Run external tool, Run Gradle/Maven/npm task (plugins), Upload files, Activate tool window; failure aborts launch. | — | — | Edit Configurations \| Before launch | P1 |
| PLATFORM-RUN-008 | Compound configurations | Group of configurations launched together with one Run or Debug press; members start in parallel (inferred); nested compounds allowed (inferred). | — | — | Edit Configurations \| + Compound | P1 |
| PLATFORM-RUN-009 | Run tool window & consoles (`ActivateRunToolWindow`) | One tab per run, toolbar (rerun, stop, restore layout, pin, soft wrap, scroll to end, clear, print), console with stdout/stderr colouring, stdin input, ANSI SGR colours and cursor-control handling, folding of noisy lines (configurable patterns), hyperlinks to files with `path:line:col` and stack-trace filters (language filters), "Analyze stack trace". Console cycle buffer size limit; "show console on output to stdout/stderr" options; Send EOF (`SendEOF`). | Alt+4; Ctrl+D (send EOF) | Cmd+4; Cmd+D | View \| Tool Windows \| Run | P0 |
| PLATFORM-RUN-010 | Navigation from runtime output to source (2026.2 enhancement) | Release notes highlight improved jump from runtime output to code; underlying mechanism is console hyperlink filters. | — | — | Run console | P1 |
| PLATFORM-RUN-011 | Console folding, grep and filters | Fold lines matching patterns; "Grep console"-like filtering is plugin; search in console with Ctrl+F. | Ctrl+F | Cmd+F | Console settings | P1 |
| PLATFORM-RUN-012 | Services tool window (`ActivateServicesToolWindow`, `ServiceView.*`) | Aggregates long-running things (run configurations as Run Dashboard, Docker containers, databases, app servers, HTTP client runs) in a tree with per-service consoles; group by type/status; open in separate tab. | Alt+8 | Cmd+8 | View \| Tool Windows \| Services | P1 |
| PLATFORM-RUN-013 | Run Dashboard (`RunDashboard.*`) | Run configurations of selected types are shown in Services instead of Run tool window; add/hide types, group by status/type/folder, run/debug/stop from tree. | — | — | Services tool window \| Add Service | P2 |
| PLATFORM-RUN-014 | Run Anything (`RunAnything`) | Popup to run arbitrary commands (shell, build-tool goals, configurations by name) with completion, history, and modifier to debug; context chooser (project/module dir). | Double Ctrl | Double Ctrl | Run \| Run Anything | P1 |
| PLATFORM-RUN-015 | External tools | User-defined command-line tools with macros, invoked from Tools menu, context menus, or before-launch; output in Run console with output filters. | — | — | Settings \| Tools \| External Tools | P1 |
| PLATFORM-RUN-016 | Shell Script run configuration (plugin) | Runs a script file or inline text in a chosen interpreter/terminal. | — | — | Run configurations | P1 |
| PLATFORM-RUN-017 | Build output tool window (`CompileDirty`, `Compile`) | Generic Build tool window: tree of build events, errors with navigation, console view; Build/Rebuild framework actions (language/build plugins implement). | Ctrl+F9; Ctrl+Shift+F9 | Cmd+F9; Cmd+Shift+F9 | Build menu | P1 |
| PLATFORM-RUN-018 | Test runner UI framework (`ActivateUnitTestsToolWindow`, `RerunTests`, `ShowRecentTests`) | Generic SM test runner: test tree with status/duration, filters (passed/ignored), console per test, rerun failed, auto-test (rerun on save), history import/export, diff for assertion failures, jump to test source. | Alt+Shift+R (rerun); Ctrl+Shift+; (recent) | Ctrl+Cmd+R / Opt+Shift+R; Cmd+Shift+; | Run tool window | P1 |
| PLATFORM-RUN-019 | Coverage framework (`RunCoverage`, `SwitchCoverage`) | Run with coverage; per-line gutter coverage colours; Coverage tool window. Engines are language plugins. | —; Ctrl+Alt+F6 | —; Cmd+Opt+F6 | Run \| Run with Coverage | P2 |
| PLATFORM-RUN-020 | Debugger session UI (`ActivateDebugToolWindow`) | Debug tool window: Threads & Frames, Variables (tree with inline watches, "set value" F2, mark object, navigate to source/type), Watches (merged into Variables by default), Console, layout customisable; multiple sessions as tabs. Protocol-agnostic (XDebugger framework); engines contributed by languages. | Alt+5 | Cmd+5 | View \| Tool Windows \| Debug | P0 |
| PLATFORM-RUN-021 | Stepping (`StepOver`, `StepInto`, `SmartStepInto`, `StepOut`, `ForceStepOver`, `ForceStepInto`, `RunToCursor`, `ForceRunToCursor`, `Resume`, `ShowExecutionPoint`) | Standard stepping; smart step into chooses among calls on the line; run to cursor (also by clicking line number); pause; show execution point. | F8; F7; Shift+F7; Shift+F8; Alt+Shift+F8; Alt+Shift+F7; Alt+F9; Ctrl+Alt+F9; F9; Alt+F10 | F8; F7; Shift+F7; Shift+F8; Opt+Shift+F8; Opt+Shift+F7; Opt+F9; Cmd+Opt+F9; Cmd+Opt+R; Opt+F10 | Run menu; debug toolbar | P0 |
| PLATFORM-RUN-022 | Line breakpoints (`ToggleLineBreakpoint`, `ToggleTemporaryLineBreakpoint`) | Click gutter or shortcut to toggle; temporary (remove after hit); enable/disable with Alt/middle-click; drag to move; invalid breakpoints shown. Inline breakpoints at lambda positions (language). | Ctrl+F8; Ctrl+Alt+Shift+F8 | Cmd+F8; Cmd+Opt+Shift+F8 | Gutter; Run \| Toggle Breakpoint | P0 |
| PLATFORM-RUN-023 | Breakpoint properties & dialog (`EditBreakpoint`, `ViewBreakpoints`) | Conditions, hit counts/pass count (language), suspend policy (all/thread/none), log message, evaluate-and-log expression, "remove once hit", dependency ("disabled until hit of ..."), grouping, filters (language), exception breakpoints (language); "Mute breakpoints" toggle. | Ctrl+Shift+F8 | Cmd+Shift+F8 | Run \| View Breakpoints | P0 |
| PLATFORM-RUN-024 | Logpoints (2026.2) | Non-suspending breakpoints dedicated to logging an expression/message, presented as a first-class gutter element (release notes: "Logpoints"). Exact UI/shortcut unconfirmed (Open questions). | — | — | Gutter (inferred) | P1 |
| PLATFORM-RUN-025 | Evaluate expression & quick evaluate (`EvaluateExpression`, `QuickEvaluateExpression`) | Evaluate dialog/inline field with code completion, history, multi-line mode, add to watches; quick evaluate shows value of selection in a popup (or on Alt-hover). | Alt+F8; Ctrl+Alt+F8 / Alt+Click | Opt+F8; Cmd+Opt+F8 / Opt+Click | Run menu; debug toolbar | P0 |
| PLATFORM-RUN-026 | Watches (`XDebugger.NewWatch`) | Watch expressions evaluated each stop; inline watches in editor. | Insert | Insert | Variables view | P0 |
| PLATFORM-RUN-027 | Inline values in editor | Variable values rendered at line ends during debug, changed values highlighted. | — | — | Settings \| Build, Execution, Deployment \| Debugger \| Data Views | P1 |
| PLATFORM-RUN-028 | Attach to process (`XDebugger.AttachToProcess`) | Lists local (and remote via SSH/WSL/Docker) processes attachable by installed debugger providers. | Ctrl+Alt+F5 | Opt+Shift+F5 (as extracted) | Run \| Attach to Process | P1 |
| PLATFORM-RUN-029 | Hot-swap / update running application (`UpdateRunningApplication`) | Framework action; engines implement code reload. | Ctrl+F10 | Cmd+F10 | Run menu | P2 |
| PLATFORM-RUN-030 | Debugger settings | Stepping filters, data views (renderers, show values inline, tooltip on hover), hotswap, "focus application on breakpoint", "show debug tool window on breakpoint". | — | — | Settings \| Build, Execution, Deployment \| Debugger | P1 |
| PLATFORM-RUN-031 | Run targets | Run configurations can execute on a target (local machine, WSL, SSH, Docker) with path mapping and file sync (some types only). | — | — | Edit Configurations \| Run on | P2 |
| PLATFORM-RUN-032 | Process console input & ANSI emulation | Console processes run through PTY optionally ("emulate terminal in output console") enabling interactive programs and full ANSI behaviour. | — | — | Edit Configurations \| Modify options | P1 |
| PLATFORM-RUN-033 | Docker integration (bundled plugin) | Docker connections, Services nodes for containers/images/compose, Dockerfile/compose run configurations, logs, exec terminal; 2026.2 lists Docker Compose improvements. | — | — | Services tool window; Settings \| Build, Execution, Deployment \| Docker | P2 |
| PLATFORM-RUN-034 | HTTP Client (plugin; Ultimate) | `.http`/`.rest` request files with environments, run gutter, response viewer, history. Not available in Community (note). | — | — | Tools \| HTTP Client | P2 |

### Sources (Run / Debug)

- https://www.jetbrains.com/help/idea/run-debug-configuration.html
- https://www.jetbrains.com/help/idea/run-tool-window.html
- https://www.jetbrains.com/help/idea/services-tool-window.html
- https://www.jetbrains.com/help/idea/running-anything.html
- https://www.jetbrains.com/help/idea/debugging-code.html, https://www.jetbrains.com/help/idea/using-breakpoints.html, https://www.jetbrains.com/help/idea/debug-tool-window.html
- https://www.jetbrains.com/help/idea/settings-tools-external-tools.html
- https://www.jetbrains.com/help/idea/test-runner-tab.html (inferred URL)
- https://www.jetbrains.com/help/pycharm/run-debug-configuration.html (platform confirmation)
- https://plugins.jetbrains.com/docs/intellij/run-configurations.html, https://plugins.jetbrains.com/docs/intellij/execution.html
- Release notes: https://blog.jetbrains.com/idea/2026/07/intellij-idea-2026-2/ (logpoints, runtime-output navigation, Docker Compose)

---

## 6. Terminal (TERM)

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-TERM-001 | Terminal tool window (`ActivateTerminalToolWindow`) | Embedded terminal running the user's shell in a PTY at the project root; opens/focuses tool window. | Alt+F12 | Opt+F12 | View \| Tool Windows \| Terminal | P0 |
| PLATFORM-TERM-002 | Terminal engine selection | Settings choose engine: "Reworked 2025" (current default, rebuilt on the editor component with fast rendering), "Classic" (legacy JediTerm-based), and a deprecated "Experimental 2024" block terminal. | — | — | Settings \| Tools \| Terminal \| Terminal engine | P1 |
| PLATFORM-TERM-003 | Sessions & tabs (`Terminal.NewTab`, `Terminal.CloseTab`, `Terminal.MoveToolWindowTabLeft/Right`, `Terminal.CloseSession`) | Multiple sessions as tabs, rename tabs (default name configurable; option to use the app title), split tabs vertically/horizontally, move tab to editor area, sessions restored after restart (inferred). | Ctrl+Shift+T (new tab); Ctrl+D (close session) | Cmd+T; Ctrl+D | Terminal toolbar | P0 |
| PLATFORM-TERM-004 | Shell integration | Injected shell scripts (bash, zsh, fish, PowerShell) report prompt/command boundaries and cwd; enables command blocks, completion and history features. Toggle "Shell integration". | — | — | Settings \| Tools \| Terminal | P1 |
| PLATFORM-TERM-005 | Command blocks & navigation (`Terminal.SelectBlockAbove/Below`, `Terminal.SelectLastBlock`, `Terminal.CopyBlock`, `Terminal.ExpandBlockSelectionAbove/Below`) | Output grouped per command; select and copy a command's output; optional separators between commands. | Ctrl+Up / Down | Cmd+Up / Down | Terminal | P2 |
| PLATFORM-TERM-006 | Command completion (`Terminal.CommandCompletion*`) | Popup of command, sub-command, option and path completions while typing (show: always / only for parameters / never; configurable trigger and insert keys). AI inline proposals optional (`Terminal.InsertInlineCompletion`). | Tab; Right (inline) | Tab; Right | Settings \| Tools \| Terminal \| Command Completion | P1 |
| PLATFORM-TERM-007 | Command history search (`Terminal.SearchInCommandHistory`) | Popup over shell history. | Ctrl+R | Ctrl+R | Terminal | P1 |
| PLATFORM-TERM-008 | Smart command execution (`Terminal.SmartCommandExecution.Run/Debug`) | Highlights recognised commands (e.g. build tool invocations) and runs/debugs them via a matching IDE run configuration. | Ctrl+Enter; Ctrl+Shift+Enter | Cmd+Enter; Cmd+Shift+Enter | Terminal | P2 |
| PLATFORM-TERM-009 | Hyperlinks and file navigation | File paths, URLs and `file:line` in output are clickable. | Ctrl+Click (inferred) | Cmd+Click | Terminal | P1 |
| PLATFORM-TERM-010 | Copy/paste & selection (`Terminal.CopySelectedText`, `Terminal.Paste`, `Terminal.CreateRectangularSelectionOnMouseDrag`) | Copy on selection option, paste with bracketed paste, rectangular selection; Ctrl+C sends SIGINT when no selection. | Ctrl+C / Ctrl+V | Cmd+C / Cmd+V | Terminal | P0 |
| PLATFORM-TERM-011 | Scrolling (`Terminal.LineUp/Down`, `Terminal.PageUp/Down`) | Scrollback navigation with keyboard; configurable buffer size. | Ctrl+Up/Down; Shift+PgUp/PgDn | Cmd+Up/Down; Shift+PgUp/PgDn | Terminal | P0 |
| PLATFORM-TERM-012 | Terminal settings | Shell path, start directory, environment variables, tab name, audible bell, cursor shape, copy-to-clipboard-on-selection, paste on middle click, "override IDE shortcuts" (send keys to shell), font (can differ from editor), activate virtualenv/project env (language plugins). | — | — | Settings \| Tools \| Terminal | P0 |
| PLATFORM-TERM-013 | Open in Terminal (`Terminal.OpenInReworkedTerminal`) | Context action on directories/files to open a terminal at that location. | — | — | Project view context \| Open In \| Terminal | P1 |
| PLATFORM-TERM-014 | Remote / WSL / container shells | Shell may be WSL distribution, SSH session (via SSH configurations), or Docker exec (plugins). | — | — | Terminal new-tab dropdown | P2 |
| PLATFORM-TERM-015 | Find in terminal | Search the scrollback with the editor-style find bar. | Ctrl+F (inferred) | Cmd+F | Terminal | P1 |

### Sources (Terminal)

- https://www.jetbrains.com/help/idea/terminal-emulator.html
- https://www.jetbrains.com/help/idea/settings-tools-terminal.html
- https://www.jetbrains.com/help/pycharm/terminal-emulator.html (platform confirmation)
- https://blog.jetbrains.com/idea/2025/04/jetbrains-terminal-a-new-architecture/ (inferred URL; reworked terminal announcement)
- Labels/action ids: `plugins/terminal/resources/messages/TerminalBundle.properties` (Apache-2.0).

---
## 7. Tool windows and workbench services (TOOL)

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-TOOL-001 | Tool window framework | Dockable panels on left/right/bottom stripes, split top/bottom groups per side; view modes Dock Pinned, Dock Unpinned, Undock, Float, Window, Split; drag to rearrange; hide/remove from sidebar; tool window tabs; numbered quick-access shortcuts. | Alt+N | Cmd+N | View \| Tool Windows | P0 |
| PLATFORM-TOOL-002 | Tool window keyboard control (`HideActiveWindow`, `HideAllWindows`, `JumpToLastWindow`, `ResizeToolWindow*`, `MaximizeToolWindow`, `FocusEditor`) | Hide active (Shift+Esc), hide/restore all, jump to last tool window, resize, maximize, Esc returns focus to editor. | Shift+Esc; Ctrl+Shift+F12; F12; Ctrl+Alt+Shift+arrows; Ctrl+Shift+'; Esc | Shift+Esc; Cmd+Shift+F12; F12; Ctrl+Opt+arrows; Cmd+Shift+'; Esc | Window \| Active Tool Window | P0 |
| PLATFORM-TOOL-003 | Layouts (`RestoreDefaultLayout`) | Save current tool window arrangement as named layout, restore default/saved layouts. | Shift+F12 | Shift+F12 | Window \| Layouts | P1 |
| PLATFORM-TOOL-004 | Problems tool window (`ActivateProblemsViewToolWindow`) | Tabs: Current File (live highlights of the open file), Project Errors (from background analysis/build), Server-side analysis, Vulnerable dependencies (plugins), Inspection results; group by severity, filter, show preview, auto-scroll, quick-fix from list. | Alt+6 | Cmd+6 | View \| Tool Windows \| Problems | P0 |
| PLATFORM-TOOL-005 | TODO tool window (`ActivateTODOToolWindow`) | Lists TODO pattern matches by Project / Current File / Scope / Changelist, filter by named TODO filter, group by module/directory/flatten (`TodoViewGroupBy*`), preview. | — | — | View \| Tool Windows \| TODO | P1 |
| PLATFORM-TOOL-006 | Notifications tool window (`ActivateNotificationsToolWindow`) | History of balloon notifications (replaced the old Event Log), grouped by timeline with suggestions section, clear/mark read, links in notifications actionable; notification settings per group (balloon / sticky / tool-window only / disabled; play sound; read aloud). | — | — | View \| Tool Windows \| Notifications; Settings \| Appearance & Behavior \| Notifications | P0 |
| PLATFORM-TOOL-007 | Event Log (legacy) | Pre-2022 log tool window; retained in Classic UI plugin only (inferred). Treat Notifications as successor. | — | — | — | P2 |
| PLATFORM-TOOL-008 | Bookmarks tool window | See PLATFORM-NAV-017. | Alt+2 | Cmd+2 | View \| Tool Windows \| Bookmarks | P1 |
| PLATFORM-TOOL-009 | Services tool window | See PLATFORM-RUN-012. | Alt+8 | Cmd+8 | View \| Tool Windows \| Services | P1 |
| PLATFORM-TOOL-010 | Database tool window (Ultimate) | Data sources, SQL consoles, data editor; Community lacks it (note: the source tree contains shared grid/table components). 2026.1 lets AI agents query data sources. | — | — | View \| Tool Windows \| Database | P2 |
| PLATFORM-TOOL-011 | HTTP Client (Ultimate) | See PLATFORM-RUN-034. | — | — | — | P2 |
| PLATFORM-TOOL-012 | Endpoints tool window (Ultimate) | Lists HTTP/gRPC endpoints discovered by framework plugins with docs and HTTP client generation. | — | — | View \| Tool Windows \| Endpoints | P2 |
| PLATFORM-TOOL-013 | Hierarchy tool window (`ActivateHierarchyToolWindow`) | Hosts type/call/method hierarchy tabs. | — | — | View \| Tool Windows \| Hierarchy | P1 |
| PLATFORM-TOOL-014 | Documentation tool window | Pinned quick-documentation popup content, auto-updates from caret (toggle). | — | — | Pin button in doc popup | P2 |
| PLATFORM-TOOL-015 | Git / Version Control tool window (`ActivateVersionControlToolWindow`) | Tabs: Log, Console, Update Info, Stash/Shelf (when not in Commit tool window), Worktrees; plus per-repo log tabs. | Alt+9 | Cmd+9 | View \| Tool Windows \| Git | P0 |
| PLATFORM-TOOL-016 | Status bar & widgets (`FocusStatusBar`) | Configurable widgets: caret position (click = go to line), line separator, encoding, indent, read-only lock, VCS branch (Classic), inspection/highlight level, background tasks progress, memory indicator, power save, language-services status (LSP), notifications; navigation bar can live in the status bar. | Alt+PgDn (focus) | Opt+PgDn | View \| Appearance \| Status Bar Widgets | P0 |
| PLATFORM-TOOL-017 | Main toolbar (new UI) (`FocusMainToolbar`, `MainMenuButton.ShowMenu`) | Hamburger main menu (Windows/Linux), project widget, VCS widget, run widget, search, settings, AI entry; customisable via Customize Toolbar. | Alt+PgUp; Alt+\ | Opt+PgUp | Window title bar | P0 |
| PLATFORM-TOOL-018 | Context menus & toolbar customisation | "Menus and Toolbars" settings to add/remove/reorder actions in any registered group. | — | — | Settings \| Appearance & Behavior \| Menus and Toolbars | P2 |
| PLATFORM-TOOL-019 | Quick lists | User-defined popups of chosen actions bound to a shortcut. | — | — | Settings \| Appearance & Behavior \| Quick Lists | P2 |
| PLATFORM-TOOL-020 | Learn tool window / IDE Features Trainer (plugin) | Interactive lessons inside a sandbox project. | — | — | Help \| Learn IDE Features | P2 |
| PLATFORM-TOOL-021 | Tips of the Day & productivity guide | Tips dialog at startup (opt-out) and "My Productivity" usage statistics for features. | — | — | Help \| Tip of the Day; Help \| My Productivity | P2 |
| PLATFORM-TOOL-022 | Commander-style two-pane view (plugin `commander`) | Legacy dual-panel file manager view (`CommanderSwapPanels`). | Ctrl+U (swap) | Cmd+U | View \| Tool Windows \| Commander | P2 |
| PLATFORM-TOOL-023 | Markdown support (bundled plugin) | Split editor with live preview, formatting actions (bold/italic/code/strikethrough/insert image), tables editing, Mermaid diagrams (plugin), run code blocks. | Ctrl+B/Ctrl+I/Ctrl+Shift+C/Ctrl+Shift+S/Ctrl+U (in Markdown) | Cmd+B/Cmd+I/... | Markdown editor toolbar | P1 |

### Sources (Tool windows)

- https://www.jetbrains.com/help/idea/tool-windows.html
- https://www.jetbrains.com/help/idea/problems-tool-window.html
- https://www.jetbrains.com/help/idea/todo-tool-window.html
- https://www.jetbrains.com/help/idea/notifications.html (inferred URL)
- https://www.jetbrains.com/help/idea/new-ui.html
- https://www.jetbrains.com/help/idea/guided-tour-around-the-user-interface.html
- https://www.jetbrains.com/help/idea/database-tool-window.html, https://www.jetbrains.com/help/idea/http-client-in-product-code-editor.html, https://www.jetbrains.com/help/idea/endpoints-tool-window.html (Ultimate-only notes)
- https://www.jetbrains.com/help/idea/markdown.html
- https://plugins.jetbrains.com/docs/intellij/tool-windows.html, https://plugins.jetbrains.com/docs/intellij/notifications.html, https://plugins.jetbrains.com/docs/intellij/status-bar-widgets.html

---

## 8. Settings and configuration (SET)

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-SET-001 | Settings dialog (`ShowSettings`) | Tree of configurables with search (filters tree and highlights matching options), breadcrumbs, back/forward, "modified" markers, Apply/OK/Cancel, project-level vs IDE-level badge, "reset" link per page. | Ctrl+Alt+S | Cmd+, | File \| Settings (macOS: app menu \| Settings) | P0 |
| PLATFORM-SET-002 | Settings for New Projects / default project | Edit defaults applied to newly created projects. | — | — | File \| New Projects Setup \| Settings for New Projects | P2 |
| PLATFORM-SET-003 | Appearance (theme / LaF) | Themes: Dark, Light, Light with Light Header, High Contrast, Darcula (and macOS/Windows "sync with OS"); new UI vs classic UI (Classic UI now via plugin); UI font override; compact mode; tool window stripe options; "Zoom IDE" (`ZoomInIdeAction`, `ZoomOutIdeAction`, `ResetIdeScaleAction`). | Alt+Shift+= / - / 0 (IDE zoom) | Ctrl+Opt+= / - / 0 | Settings \| Appearance & Behavior \| Appearance | P0 |
| PLATFORM-SET-004 | Keymaps | Keymap editor: choose base keymap, derive a copy, search by action name or by pressing a shortcut, add keyboard/mouse/abbreviation shortcuts, conflict detection with resolution prompt, export/import, per-OS defaults. | — | — | Settings \| Keymap | P0 |
| PLATFORM-SET-005 | Bundled keymaps | Core descriptors: Windows default (`$default`), macOS (`Mac OS X 10.5+`), IntelliJ IDEA Classic macOS (`Mac OS X`), GNOME, KDE, XWin, Emacs, Sublime Text (+macOS), "macOS System Shortcuts" overlay. Keymap plugins: Eclipse (+macOS), NetBeans 6.5, Visual Studio (+OSX), Visual Studio 2022, Visual Studio for Mac, VS Code (+macOS), ReSharper (+OSX), Visual Assist (+OSX), Qt Creator (+macOS), Xcode. Which keymap plugins are pre-installed vs. Marketplace-only is an open question. | — | — | Settings \| Keymap | P0 (default + macOS), P1 (VS Code, Eclipse), P2 (rest) |
| PLATFORM-SET-006 | Plugins manager | Tabs Marketplace (search, tags, sort, paid/free, ratings, vendor), Installed (bundled/downloaded/enabled filters, enable/disable, uninstall, per-project enable (inferred)), updates with "Update all", install from disk, custom repositories (plugin repository URLs, `updatePlugins.xml`), dynamic load/unload without restart where supported, required-plugins per project, dependency resolution, compatibility checks (`since/until-build`). | — | — | Settings \| Plugins | P0 |
| PLATFORM-SET-007 | Settings Sync / Backup and Sync (`SettingsSyncOpenSettingsAction`) | Sync IDE settings through JetBrains Account across machines and products: categories (UI, keymaps, code settings, tools, system, plugins), cross-IDE sync option, push/pull now, conflict notification; replaced the older "Settings Repository" (Git-based, still a plugin `settings-repository`). | — | — | Settings \| Backup and Sync; status in gear menu | P2 |
| PLATFORM-SET-008 | Import / Export settings (`ExportSettings`, `ImportSettings`) | Export chosen components to a zip; import from zip, previous IDE version (on first start), or other IDEs (VS Code, Visual Studio, Eclipse importers). | — | — | File \| Manage IDE Settings | P1 |
| PLATFORM-SET-009 | Restore default settings | Backs up and resets all settings to defaults with restart. | — | — | File \| Manage IDE Settings \| Restore Default Settings | P2 |
| PLATFORM-SET-010 | Advanced Settings page | Flat list of rarely changed, typed options with search, grouped by area (editor, VCS, terminal, large files, ...). Replaces many Registry keys. | — | — | Settings \| Advanced Settings | P1 |
| PLATFORM-SET-011 | Registry (`ShowRegistry`) | Hidden key-value store of internal toggles (boolean, int, string, colour) with "restart required" markers, reset-to-default, search; unsupported territory. | — | — | Find Action \| "Registry..." | P2 |
| PLATFORM-SET-012 | System settings | Reopen projects on startup; open project in new/same window/ask; confirm before exiting; process running when closing; save files on frame deactivation and after N seconds idle; "use safe write"; sync external changes on activation; default project directory; HTTP proxy (auto/manual/PAC); passwords storage (KeePass/native keychain/in-memory); date formats; data sharing. | — | — | Settings \| Appearance & Behavior \| System Settings | P0 |
| PLATFORM-SET-013 | Updates (`CheckForUpdate`) | Channel selection (stable/EAP), automatic check with notification, patch-based updates, plugin updates; Toolbox App manages installs externally. | — | — | Help \| Check for Updates; Settings \| System Settings \| Updates | P1 |
| PLATFORM-SET-014 | IDE Scripting Console (`IdeScriptingConsole`) | Console evaluating scripts (Kotlin script, Groovy, JavaScript via JSR-223 engines) against the running IDE API; startup scripts in config dir. | — | — | Find Action \| IDE Scripting Console | P2 |
| PLATFORM-SET-015 | Path variables & macros | User-defined path variables substituted into stored settings to keep them portable. | — | — | Settings \| Appearance & Behavior \| Path Variables | P2 |
| PLATFORM-SET-016 | Custom VM options & properties (Help menu) | Edit `.vmoptions` (heap size etc.) and `idea.properties` from the IDE; "Change Memory Settings" dialog. | — | — | Help \| Edit Custom VM Options / Change Memory Settings | P2 |
| PLATFORM-SET-017 | Configuration directories layout | Config, system (caches, indexes), plugins, logs directories per product+version; command line `-Didea.*` overrides; first-start import wizard. | — | — | Help \| Show Log in Explorer/Finder (`ShowLog`) | P1 |
| PLATFORM-SET-018 | Diagnostics: logs, thread dumps, Collect logs, Report issue | Help menu items: Show Log, Collect Logs and Diagnostic Data, Diagnostic Tools (Debug Log Settings, Activity Monitor, Analyze Memory Use (inferred), Thread dump), Submit a Bug Report / Contact Support; internal errors indicator (red icon) with report dialog. | — | — | Help \| Diagnostic Tools | P1 |
| PLATFORM-SET-019 | Restart IDE (`RestartIde`) | Restart action used by plugin/settings changes. | — | — | File \| Restart IDE (inferred menu path) | P1 |
| PLATFORM-SET-020 | Command-line launcher & CLI tools | `idea` launcher script: open file/project, `--line`/`--column`, `--wait`, `diff`, `merge`, `format`, `inspect`, `installPlugins`; Toolbox-generated shell scripts. | — | — | Tools \| Create Command-line Launcher (inferred) | P1 |
| PLATFORM-SET-021 | Help & keymap reference (`Help.KeymapReference`, `ContextHelp`) | Opens web help for current dialog/topic; downloadable keymap PDF. | F1 (context help in dialogs) | Cmd+Shift+F1 (as extracted) | Help menu | P2 |
| PLATFORM-SET-022 | About & license | Version/build info with copy; license activation dialog for commercial products. | — | — | Help \| About | P2 |
| PLATFORM-SET-023 | Language & Region / Language packs | UI language selection (bundled language packs for Chinese, Japanese, Korean via plugins) and region (for marketplace/content). | — | — | Settings \| Appearance & Behavior \| Language and Region | P2 |

### Sources (Settings)

- https://www.jetbrains.com/help/idea/settings-preferences-dialog.html (inferred URL)
- https://www.jetbrains.com/help/idea/settings-keymap.html, https://www.jetbrains.com/help/idea/configuring-keyboard-and-mouse-shortcuts.html
- https://www.jetbrains.com/help/idea/managing-plugins.html
- https://www.jetbrains.com/help/idea/sharing-your-ide-settings.html
- https://www.jetbrains.com/help/idea/advanced-settings.html (inferred URL)
- https://www.jetbrains.com/help/idea/system-settings.html
- https://www.jetbrains.com/help/idea/ide-scripting-console.html
- https://www.jetbrains.com/help/idea/tuning-the-ide.html, https://www.jetbrains.com/help/idea/directories-used-by-the-ide-to-store-settings-caches-plugins-and-logs.html
- https://www.jetbrains.com/help/idea/working-with-the-ide-features-from-command-line.html
- https://plugins.jetbrains.com/docs/intellij/settings.html, https://plugins.jetbrains.com/docs/intellij/plugin-repository.html (inferred URL for custom repositories)
- Keymap descriptor file names: `platform/platform-resources/src/keymaps/*.xml`, `plugins/keymaps/*/resources/keymaps/*.xml` (Apache-2.0, names only).

---

## 9. AI features (AI) — optional, all P2

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-AI-001 | AI Chat / AI Assistant (plugin, subscription) | Chat tool window with project context, code generation, explain/refactor actions, commit-message generation, name suggestions. Archipel IDE note: provide only an extension point. | — | — | AI Chat tool window | P2 |
| PLATFORM-AI-002 | Junie / coding agents & ACP | Agent tool window; 2026.1 adds ACP (Agent Client Protocol) registry to install third-party agents (Codex, Cursor, any ACP agent); 2026.2 adds "agent skills". The IC tree has a `platform/acp` module. | — | — | AI Chat / agent panel | P2 |
| PLATFORM-AI-003 | Native GitHub Copilot integration (2026.2) | Copilot available without the third-party plugin (per release notes). | — | — | — | P2 |
| PLATFORM-AI-004 | Inline/cloud completion providers | Cloud multi-line completion and third-party completion providers (2026.2); local full-line completion is separate (see PLATFORM-EDIT-131). | Tab | Tab | Settings \| Editor \| General \| Inline Completion | P2 |
| PLATFORM-AI-005 | Next edit suggestions (2026.1) | Proposes follow-up edits propagated through the file after a change, without consuming AI quota. | Tab (inferred) | Tab | Editor | P2 |
| PLATFORM-AI-006 | AI in VCS / terminal / debugger touch points | Generate commit message button, explain terminal command/error, AI inline terminal suggestions, explain runtime errors in console. | — | — | Commit UI, Terminal, Run console | P2 |
| PLATFORM-AI-007 | MCP server (plugin `mcp-server`) | IDE exposes tools to external AI clients over Model Context Protocol. | — | — | Settings \| Tools \| MCP Server (inferred) | P2 |

### Sources (AI)

- https://www.jetbrains.com/help/ai-assistant/ , https://www.jetbrains.com/help/junie/ (inferred URLs)
- https://blog.jetbrains.com/idea/2026/03/intellij-idea-2026-1/
- https://blog.jetbrains.com/idea/2026/07/intellij-idea-2026-2/
- https://www.jetbrains.com/idea/whatsnew/

---

## 10. Collaboration and remote (REMOTE) — all P2

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-REMOTE-001 | Code With Me | Host shares a session; guests join via link with thin client; follow mode, permissions (read-only/edit files/full), audio/video calls. Plugin; licence-dependent; its availability in 2026.2 is unverified (see Open questions). | — | — | Main toolbar collaboration icon | P2 |
| PLATFORM-REMOTE-002 | Remote Development (Gateway / backend+client split) | IDE backend runs on a remote host (SSH, cloud providers) and a JetBrains Client renders the UI locally; port forwarding, backend management. | — | — | Welcome screen \| Remote Development | P2 |
| PLATFORM-REMOTE-003 | Dev Containers | Open `devcontainer.json` projects inside Docker containers; 2026.1 brings a native "open as if local" workflow. | — | — | Welcome screen; gutter in devcontainer.json | P2 |
| PLATFORM-REMOTE-004 | WSL support | Open projects stored in WSL, run configurations/terminal/Git inside WSL, WSL paths in file chooser (`FileChooser.GoToWslHome`); Eel/ijent remote execution layer. | — | — | File \| Open on `\\wsl$`; Run targets | P2 |
| PLATFORM-REMOTE-005 | Docker integration | See PLATFORM-RUN-033. | — | — | — | P2 |
| PLATFORM-REMOTE-006 | SSH configurations & remote hosts | SSH connection profiles shared by terminal, deployment, run targets; Remote Host tool window for SFTP browsing (Ultimate (inferred)). | — | — | Settings \| Tools \| SSH Configurations | P2 |
| PLATFORM-REMOTE-007 | Deployment / SFTP sync (Ultimate) | Upload on save, compare remote with local. | — | — | Tools \| Deployment | P2 |

### Sources (Remote)

- https://www.jetbrains.com/help/idea/code-with-me.html
- https://www.jetbrains.com/help/idea/remote-development-overview.html
- https://www.jetbrains.com/help/idea/connect-to-devcontainer.html
- https://www.jetbrains.com/help/idea/how-to-use-wsl-development-environment-in-product.html
- https://www.jetbrains.com/help/idea/docker.html
- https://blog.jetbrains.com/idea/2026/03/intellij-idea-2026-1/ (Dev Containers)

---
## 11. Accessibility and i18n (A11Y)

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-A11Y-001 | Screen reader support | Exposes accessible names/roles for components to NVDA/JAWS (via Java Access Bridge) and VoiceOver; "Support screen readers" option (requires restart) changes Tab behaviour so it moves focus. | — | — | Settings \| Appearance & Behavior \| Appearance \| Accessibility | P1 |
| PLATFORM-A11Y-002 | High contrast theme | Bundled High Contrast UI theme and editor scheme; follows OS high-contrast mode on Windows (inferred). | — | — | Appearance \| Theme | P1 |
| PLATFORM-A11Y-003 | Colour-deficiency adjustments | "Adjust colors for red-green vision deficiency" option altering diff/VCS/error colours. | — | — | Settings \| Appearance \| Accessibility | P2 |
| PLATFORM-A11Y-004 | Keyboard-only operation | Every tool window/popup reachable by keyboard; mnemonics in menus (Win/Linux); focus status bar/toolbar actions; Find Action as universal entry; gutter focus (`EditorFocusGutter`) and gutter icon tooltip (`EditorShowGutterIconTooltip`). | Alt+Shift+6, F; Alt+Shift+6, T | Opt+Shift+6, F; Opt+Shift+6, T | Find Action | P0 |
| PLATFORM-A11Y-005 | UI scaling & font sizes | IDE zoom, custom UI font size, editor font zoom, presentation mode (see SET-003, EDIT-073). | — | — | Appearance | P0 |
| PLATFORM-A11Y-006 | Contrast-scrollbars, animations off | "Use contrast scrollbars", disable animations / smooth scrolling. | — | — | Appearance; Editor \| General | P2 |
| PLATFORM-A11Y-007 | Audible / read-aloud notifications | Notifications can be read aloud (macOS) or play a sound. | — | — | Notifications settings | P2 |
| PLATFORM-A11Y-008 | Localization (language packs) | Language packs (Chinese Simplified, Japanese, Korean) distributed as plugins and selected in Language and Region; all UI strings resource-bundle based. | — | — | Settings \| Language and Region | P2 |
| PLATFORM-A11Y-009 | Bidirectional / complex text & IME | Correct rendering of RTL text, combining characters, emoji, CJK width; IME composition inline in editor (on-the-spot). | — | — | Editor | P0 |
| PLATFORM-A11Y-010 | Quick doc & error descriptions via keyboard | Non-hover access to all tooltips (Ctrl+F1 / Ctrl+Q) so that pointer is not needed. | Ctrl+F1; Ctrl+Q | Cmd+F1; F1 | View menu | P0 |

### Sources (Accessibility)

- https://www.jetbrains.com/help/idea/accessibility.html
- https://www.jetbrains.com/help/idea/settings-appearance.html
- https://www.jetbrains.com/help/pycharm/accessibility.html (platform confirmation)
- https://www.jetbrains.com/help/idea/language-and-region.html (inferred URL)

---

## 12. Performance and background work (PERF)

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-PERF-001 | Startup behaviour | Splash/progress, reopen last projects (setting), editors and tool window layout restored lazily; project usable before indexing completes ("smart mode" features progressively enabled); warmup/headless mode for CI. | — | — | — | P0 |
| PLATFORM-PERF-002 | Background tasks & progress | Long operations run as cancellable background tasks; status bar shows progress (multiple tasks popup with per-task cancel/pause); modal progress only for blocking operations; "Send to background" for modal ones. | — | — | Status bar | P0 |
| PLATFORM-PERF-003 | Indexing progress & pause | Indexing/scanning progress with file counts and pause button; "Indexing" diagnostics (per-project indexing history report). | — | — | Status bar; Help \| Diagnostic Tools \| Indexing Diagnostics (inferred path) | P0 |
| PLATFORM-PERF-004 | Memory indicator (`ShowMemoryView` / status widget) | Optional status-bar widget showing used/allocated heap; click forces GC. | — | — | Status bar widget "Memory Indicator" | P1 |
| PLATFORM-PERF-005 | Low-memory notifications | Notification when free heap is low, offering to increase `-Xmx` via Change Memory Settings and restart; also warns on high GC load (inferred). | — | — | Notification | P1 |
| PLATFORM-PERF-006 | Freeze detection & reporting | UI thread freeze detector collects thread dumps; reports attributed to plugins with suggestion to disable the plugin. | — | — | Notifications; internal errors indicator | P1 |
| PLATFORM-PERF-007 | Plugin performance attribution | Detects slow plugins (startup, freezes, analysis time) and suggests disabling. | — | — | Notification; Help \| Diagnostic Tools (inferred) | P2 |
| PLATFORM-PERF-008 | Cancellable read actions / responsiveness rules | Typing never blocked by background analysis; background tasks restart on document change. Implementation contract for archipel-ide: edits pre-empt analysis. | — | — | — | P0 |
| PLATFORM-PERF-009 | Power Save Mode | See PLATFORM-PROJ-026. | — | — | File \| Power Save Mode | P1 |
| PLATFORM-PERF-010 | Antivirus/Defender exclusion prompt (Windows) | Detects Microsoft Defender scanning project/system dirs and offers to add exclusions. | — | — | Notification | P2 |
| PLATFORM-PERF-011 | Activity monitor | Shows CPU usage per subsystem/plugin. | — | — | Help \| Diagnostic Tools \| Activity Monitor | P2 |
| PLATFORM-PERF-012 | Large-repository handling | VCS log indexing can be paused/disabled for huge repos (`Vcs.Log.ShowBigRepositories`), file-system watcher limits warnings (Linux inotify), "external file sync" tuning. | — | — | Notifications; Advanced Settings | P1 |

### Sources (Performance)

- https://www.jetbrains.com/help/idea/increasing-memory-heap.html
- https://www.jetbrains.com/help/idea/indexing.html
- https://www.jetbrains.com/help/idea/troubleshooting-performance-issues.html (inferred URL)
- https://plugins.jetbrains.com/docs/intellij/threading-model.html, https://plugins.jetbrains.com/docs/intellij/background-processes.html
- Release notes (freeze fixes): https://blog.jetbrains.com/idea/2026/07/intellij-idea-2026-2/

---

## 13. Window, editor tab and layout management (UI)

| ID | Feature (action id) | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| PLATFORM-UI-001 | Editor tabs | Tabs with placement (top/left/right/bottom/none), multiple rows or scroll, tab limit (default 10) with close policy (least recently used), close button position, modified marker, file-type icons, "mark modified (*)", show directory for non-unique names, tab colours by file colours. | — | — | Settings \| Editor \| General \| Editor Tabs | P0 |
| PLATFORM-UI-002 | Tab navigation (`NextTab`, `PreviousTab`, `ShowContent`, `CloseContent`, `CloseAllEditors`, `CloseAllEditorsButActive`, `CloseAllUnmodifiedEditors`, `ReopenClosedTab`) | Next/previous tab, list of tabs popup, close/close others/close all/close unmodified, reopen closed, close with middle-click. | Alt+Right/Left; Alt+Down; Ctrl+F4; —; —; — | Cmd+Shift+] / [; Ctrl+Shift+Down (as extracted); Cmd+W | Window \| Editor Tabs | P0 |
| PLATFORM-UI-003 | Pin tabs (`PinActiveEditorTab`) | Pinned tabs stay leftmost (or separate row) and survive "close all/others". | — | — | Tab context menu | P1 |
| PLATFORM-UI-004 | Split editors (`SplitVertically`, `SplitHorizontally`, `MoveTabRight`, `MoveTabDown`, `UnsplitAll`, `NextSplitter`, `PrevSplitter`, `OpenInRightSplit`, `SplitChooser`, `OpenEditorInOppositeTabGroup`) | Split right/down (same file in two editors sharing the document), split-and-move, unsplit, cycle between splits, open from tree in right split (Shift+Enter or Alt+double-click), split chooser overlay, drag tab to edge to split. | Shift+Enter (open in right split); Alt+Shift+Enter (chooser) | Shift+Enter; Opt+Shift+Enter; Opt+Tab (next splitter) | Window \| Editor Tabs; tab context menu | P0 |
| PLATFORM-UI-005 | Detached editor windows (`EditSourceInNewWindow`) | Move/open a tab in a separate frame (drag out or Shift+F4). | Shift+F4 | Shift+F4 | Tab context \| Open in New Window | P1 |
| PLATFORM-UI-006 | Multi-editor file tabs (`NextEditorTab`, `PreviousEditorTab`) | Files with several editors (e.g. Markdown text/preview, form designer) switch between them. | Alt+Shift+Right/Left | Ctrl+Shift+Right/Left | Editor bottom tabs | P2 |
| PLATFORM-UI-007 | Popups & choosers conventions | Speed search, Esc closes, popups can be pinned or resized, remembered size, preview panes; "Got It" tooltips (`CloseGotItTooltip`). | Esc | Esc | — | P0 |
| PLATFORM-UI-008 | Main menu & context menus | Full main menu (File, Edit, View, Navigate, Code, Refactor, Build, Run, Tools, VCS/Git, Window, Help), context menu key (`ShowPopupMenu`), macOS global menu. | Context Menu key / Shift+F10 (inferred) | — | Main menu | P0 |
| PLATFORM-UI-009 | Native file choosers / IDE file chooser | IDE chooser with path field, show hidden files, go to project/home/WSL, path bar (`FileChooser.*`); option to use native dialogs. | — | — | File \| Open | P0 |
| PLATFORM-UI-010 | Drag-and-drop | Drop files from OS onto editor (open) or Project view (copy); drag tabs between splits/windows; drag tool windows between stripes. | — | — | — | P1 |
| PLATFORM-UI-011 | macOS integration | Native full screen, native tabs for project windows (opt-in), touch bar support (legacy), Force Touch actions, Services menu, Dock menu with recent projects. | — | Ctrl+Cmd+F | — | P2 |
| PLATFORM-UI-012 | Window title & frame | Unified custom title bar on Windows/Linux with merged main menu; project name and file path in title (configurable). | — | — | Appearance | P1 |
| PLATFORM-UI-013 | UI Inspector (internal) (`UiInspector`) | Developer tool to inspect Swing component properties. | Ctrl+Alt+Click | Ctrl+Opt+Click | Internal | P2 |

### Sources (UI)

- https://www.jetbrains.com/help/idea/managing-editor-tabs.html
- https://www.jetbrains.com/help/idea/using-code-editor.html#split_screen (inferred anchor)
- https://www.jetbrains.com/help/idea/new-ui.html
- https://www.jetbrains.com/help/pycharm/managing-editor-tabs.html (platform confirmation)
- Keymap descriptors (Apache-2.0) as above.

---

## Cross-cutting implementation notes for archipel-ide (inferred)

- Every row above that has an action id should map to one `archipel-ide-command` command; the action
  id is recorded only as a cross-reference for keymap import, never as a dependency.
- Shortcut tables should be data (a keymap file), mirroring the parent/child keymap model and
  the macOS Ctrl/Cmd swap rule rather than duplicating every binding.
- Language-dependent frameworks (completion, structure view, find usages, hierarchies, folding,
  surround, move statement, inlay hints, breadcrumbs) need a plugin/service trait with a plain-text
  or tree-sitter fallback so the platform layer stays language-agnostic (see `AGENTS.md` layering).

---

## Open questions

1. **2026.2 conflict-resolution flow** (PLATFORM-VCS-026): release blog names it but the public
   help page describing what changed was not found; needs hands-on observation.
2. **Logpoints UI** (PLATFORM-RUN-024): gutter interaction, default shortcut (if any), and how
   they differ from the older "non-suspending breakpoint with log expression" need confirmation;
   the dedicated blog post could not be read by the fetch tool.
3. **Clone caret gesture**: the double-modifier-and-hold gesture for `EditorCloneCaretAbove/Below`
   is not in keymap descriptors; confirm exact behaviour on Linux window managers that grab
   double Ctrl.
4. **macOS shortcuts flagged "as extracted"** (Attach to Process `Opt+Shift+F5`, Context Help
   `Cmd+Shift+F1`, Show List of Tabs `Ctrl+Shift+Down`): derived mechanically from descriptors;
   verify against https://www.jetbrains.com/help/idea/reference-keymap-mac-default.html (the
   Windows keymap reference page failed to load during research).
5. **Keymap plugins**: which of Eclipse / NetBeans / VS / VS Code / Xcode / ReSharper / Qt Creator
   / Visual Assist keymaps are pre-installed in IDEA 2026.2 versus downloadable.
6. **Large-file thresholds**: exact default for "max file size for code insight" and for the
   read-only paged large-file viewer in 2026.2.
7. **Local History retention default** and whether it remains days-based.
8. **Code With Me status**: whether it is still offered in 2026.2.
9. **Classic UI / Event Log**: confirm that classic UI and the Event Log tool window are only
   available through the Classic UI plugin in 2026.2.
10. **Search Everywhere backend changes**: IC tree contains `search-everywhere-lucene` and a
    split frontend/backend Search Everywhere; confirm whether user-visible behaviour (tabs,
    prefixes) changed in 2026.x.
11. **Non-modal welcome screen** (`platform/non-modal-welcome-screen` module): whether it is the
    default welcome experience in 2026.2.
12. **Default soft-wrap file masks** and default clipboard history size (assumed `*.md; *.txt;
    *.rst; *.adoc` and 5).
13. **Text search in Search Everywhere** shortcut `Ctrl+Alt+Shift+E` (`TextSearchAction`) is in
    the descriptor; verify it is user-visible and not internal.
14. **PyCharm/WebStorm divergence**: Packages view, Project Structure and "Build" items differ
    per product; confirm which Project view views are present in non-JVM IDEs.
15. **Link verification**: help URLs were taken from search results and memory; outbound checks from the research container failed, so every jetbrains.com/help link (especially those marked "(inferred URL)") needs a reachability check before publishing.

## Summary counts

| Area | Features | P0 |
|---|---|---|
| EDIT | 118 | 60 |
| NAV | 37 | 21 |
| PROJ | 33 | 17 |
| VCS | 53 | 25 |
| RUN | 34 | 12 |
| TERM | 15 | 5 |
| TOOL | 23 | 7 |
| SET | 23 | 6 |
| AI | 7 | 0 |
| REMOTE | 7 | 0 |
| A11Y | 10 | 4 |
| PERF | 12 | 4 |
| UI | 13 | 6 |
| **Total** | **385** | **167** |

Rows with a split priority (e.g. "P0 (default + macOS), P1 (...)") are counted as P0.

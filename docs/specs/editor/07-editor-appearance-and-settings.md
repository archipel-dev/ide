# Editor spec 07: appearance, gutter, scrolling and the General editor settings

Status: specification, language independent. Requirement prefix `ED-APPEAR`.
Priorities: `MUST` / `SHOULD` / `LATER`.

What the editor shows and how it scrolls: the gutter and line numbers, caret and current line,
highlighting while the caret moves (matching braces, identifier under the caret, selected-text
occurrences), scroll margins and jumps, soft wraps, whitespace and indent guides, margins,
breadcrumbs, sticky lines, inlay hints, zoom, tooltip and popup timings, and the default value of
every setting on **Settings, Editor, General** and its sub-pages (the page is titled `General`; code
editing options are in `02-typing-and-typed-handlers.md` section 9). Colors, fonts and the color
schemes are the theme work and are not specified here; the names of the colors that carry meaning
are listed so the theme layer can map them.

Executable acceptance cases for the key-driven parts are in `cases/07-editor-appearance-and-settings.cases.md`
(format `cases/README.md`). Cases using `viewport:` describe the visible window: `viewport: 10 lines, top=0`
means lines 0 to 9 (zero-based) are visible; an `after viewport` line gives the top line afterwards.
Visual facts that have no key sequence are acceptance rows in the tables.

ic: `platform/ide-core-impl/src/com/intellij/openapi/editor/ex/EditorSettingsExternalizable.java`,
`platform/platform-impl/src/com/intellij/openapi/editor/impl/{EditorSettingsState.kt,SettingsImpl.kt}`,
`platform/editor-ui-api/src/com/intellij/openapi/editor/EditorSettings.java`,
`platform/editor-ui-api/src/com/intellij/openapi/editor/registry/EditorRegistry.kt`,
`platform/platform-impl/src/com/intellij/openapi/editor/actions/{ChangeEditorFontSizeAction,
EditorActionUtil,Toggle*}.java`, `platform/editor-ui-ex/src/com/intellij/application/options/
EditorFontsConstants.java`, `platform/lang-impl/src/com/intellij/codeInsight/highlighting/
BraceHighlightingHandler.kt`, `platform/lang-impl/src/com/intellij/codeInsight/stickyLines/*`,
`platform/platform-impl/src/com/intellij/openapi/editor/impl/stickyLines/*`,
`platform/lang-api/src/com/intellij/codeInsight/hints/{InlayHintsSettings,InlayParameterHintsProvider}.*`,
`java/java-impl/src/com/intellij/codeInsight/hints/JavaInlayParameterHintsProvider.kt`,
`platform/analysis-impl/src/com/intellij/codeInsight/CodeInsightSettings.java`,
`platform/ide-core/resources/messages/ApplicationBundle.properties`,
`platform/platform-api/resources/messages/EditorBundle.properties`,
`platform/util/resources/misc/registry.properties`,
`platform/platform-impl/resources/intellij.platform.ide.impl.xml` (advanced settings),
`platform/platform-resources/src/keymaps/$default.xml`.

## 1. Defaults of the Editor, General settings (the table every implementation must match)

Labels are the product labels (message bundle). Values are the shipped defaults from the settings
classes, registry and advanced settings.

### 1.1 Appearance

| Setting (label) | Default |
|---|---|
| Enable smooth scrolling | on (off on macOS with the JetBrains runtime) |
| Show whitespaces | off; sub-options Leading, Inner, Trailing, Selection are all on (so turning the main one on shows all four) |
| Show method separators | off |
| Show gutter icons | on |
| Show line numbers | on; numbering type `Absolute` (other values `Relative`, `Hybrid`) |
| Show indent guides | on |
| Highlight only the current declaration | off |
| Show intention bulb | on |
| Show preview for intention actions when available | on |
| Show breadcrumbs | on; placement `Bottom` (the other value is `Top`) |
| Show sticky lines while scrolling | on; maximum number of lines 5 |
| Use block caret | off |
| Use full line height caret | off |
| Use smooth caret movement | off (easing `Snappy`, the other value `Gliding`; duration 120 ms) |
| Caret blinking (ms) | on; period 500 (range 10 to 1500); smooth blinking off |
| Highlight occurrences of selected text | on |
| Show hard wrap and visual guides | on (the guide is drawn at the right margin from the code style, default 120 columns) |
| Mark modified tabs / with asterisk | modified tabs get the modified marker (dot); the asterisk variant is off |
| Show full path on mouse hover (tabs) | on |
| Enable rendered documentation comments | off |
| Use editor font in inlays | off |
| Show inspection widget (top right status) | on |
| Focus mode | off |

### 1.2 Code folding

See `04-folding.md` section 5 (imports and file header on, everything else off; folding outline on, arrows only on hover, bottom arrows off).

### 1.3 Soft wraps, virtual space, scrolling

| Setting (label) | Default |
|---|---|
| Soft-wrap these files | file masks `*.md; *.txt; *.rst; *.adoc` (soft wrap is **off** for every other file; the toolbar/gutter action `Soft-Wrap` toggles one editor) |
| Use soft wraps in console | off |
| Use the original line's indent for wrapped fragments | on; additional indent 0 symbols |
| Only show soft-wrap indicators for the current line | off |
| Force soft wrap in documents with lines longer than | 100000 characters (an advanced setting; the editor then shows the notice `This document contains very long lines. Soft wraps were enabled to improve editor performance.`) |
| Allow caret placement: After the end of line | off |
| Allow caret placement: Inside tabs | on |
| Show virtual space at the bottom of the file | off (the editor still scrolls so that 5 virtual lines, registry `editor.virtual.lines`, show below the last line) |
| Vertical scroll offset | 1 line |
| Vertical scroll jump | 0 lines |
| Horizontal scroll offset | 3 columns |
| Horizontal scroll jump | 0 columns |
| Extra empty columns after the longest line | 3 |

### 1.4 Caret movement and mouse

| Setting (label) | Default |
|---|---|
| When moving by words / Upon line break | see `ED-CARET-` section 3.2 (IDE default: current word boundaries / next-previous line boundaries) |
| Matched brace (highlight on caret movement) | on |
| Current scope (highlight on caret movement) | off |
| Change font size with Ctrl+Mouse Wheel in: | off (choices `Active editor`, `All editors`; default scope active editor) |
| Move code fragments with drag-and-drop | on (hold Ctrl to copy) |
| Honor "CamelHumps" words settings when selecting on double click | on |
| Add multiple carets on double Ctrl with arrow keys | on |

### 1.5 Other (hints, tooltips, saving)

| Setting (label) | Default |
|---|---|
| Show quick documentation on mouse move | on; delay 500 ms (range 0 to 5000) |
| Editor tooltips delay | 500 ms (range 1 to 5000) |
| Parameter info popup | on; delay 1000 ms |
| Auto-popup documentation on code completion | off; delay 500 ms |
| Remove trailing spaces on Save | `Modified lines` (stored value `Changed`; others `None`, `All lines`) |
| Keep trailing spaces on caret line | on |
| Ensure every saved file ends with a line break | off |
| Remove trailing blank lines at the end of saved files | off |
| Rename variables in place | on; preselect on rename on |
| Show tab character as | horizontal line (advanced setting `Tab character rendering`; the others are the long arrow and the `>` symbol) |
| Render special characters using Unicode names | on |
| Show zoom indicator | on |
| Bidi text base direction | content based |
| Show import popup | on |

## 2. Gutter

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-APPEAR-001 | MUST | The gutter lies between the text area and the left edge and has, left to right: gutter icons (breakpoints, run markers, implementation and override markers, bookmarks, VCS changes in a thin stripe), line numbers, annotations (VCS blame) when enabled, and the folding column. The relative order of line numbers and icons follows the setting (default: icons left of the numbers); the VCS change stripe is next to the text. | Default layout as listed. |
| ED-APPEAR-002 | MUST | **Line numbers**: shown by default, one per **logical** line (soft-wrapped continuation rows have none); numbering `Absolute` starts at 1; `Relative` shows the distance from the caret line for other lines and the absolute number for the caret line; `Hybrid` shows the absolute number for the caret line and relative distances for others. The width adapts to the number of digits of the last line. The caret line number is drawn with the current-line-number color; the font is the editor font adjusted by the advanced setting `Line number's font delta` (default -1 point). | A 120-line file: 3 digits wide; Relative mode on line 5 shows 5 on that line and 1 on lines 4 and 6. |
| ED-APPEAR-003 | MUST | Toggling `Show line numbers`, `Show gutter icons` and `Show indent guides` is possible from the editor context menu (`Appearance`), the settings page and, for line numbers, the gutter context menu; per-editor toggles (`Show Line Numbers`, `Show Indent Guides`, `Show Breadcrumbs`, `Show Gutter Icons`, `Show Whitespaces`, `Soft-Wrap`) affect only the current editor and its split copies; the global variants (`EditorGutterToggleGlobalLineNumbers`, `...IndentLines`, `...StickyLines`) change the setting. | Toggle in one editor: other editors unchanged. |
| ED-APPEAR-004 | MUST | Clicking a line number selects the line; dragging selects lines; clicking a gutter icon invokes its action. Hovering a gutter marker shows its tooltip (editor tooltip delay 500 ms). `Focus Gutter (Accessibility)` (Alt+Shift+6 then F) and `Show Gutter Icon Tooltip` (Alt+Shift+6 then T) are keyboard routes. | Click line number 3: line 3 selected including its line break. |
| ED-APPEAR-005 | MUST | The **error stripe** (right edge) shows one mark per highlighted line region with the severity color; marks have a minimum height (settable, `Error stripe mark min height`, default 2 px); clicking a mark scrolls there. | Error at line 40 of 100: mark at 40%. |

## 3. Caret, current line and highlighting while moving

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-APPEAR-010 | MUST | The caret is a vertical line 2 px wide (registry `editor.caret.width`, 2) in the insert mode, a block when `Use block caret` or in overwrite mode; blinking with a period of 500 ms (setting `Caret blinking (ms)`, 10 to 1500; 0 or off disables) and it does not blink while the user types or moves (it is shown solid after each edit and caret move, then resumes blinking). The line containing the caret is highlighted with the *caret row* color across the full text width (not in a viewer with a selection, and not for lines whose highlighting would hide a selection); multi-caret: every caret line. | Caret row color present on the caret line. |
| ED-APPEAR-011 | MUST | **Matching braces**: with `Matched brace` on (default) after each caret move the editor highlights the brace next to the caret and its partner: the brace right after the caret is taken if it is a brace, else the one before (blanks between the caret and a brace to its left or right are skipped); both get the matched-brace color (a mismatch uses the unmatched-brace color). If the partner is outside the viewport (above) a hint with the lines of the opening construct appears after 300 ms. The block caret and the highlighted left brace do not double-highlight the right brace. Highlighting is skipped inside collapsed regions. With `Current scope` on, the enclosing structural left brace is also highlighted (as the scope). | Caret after `)` highlights it and its `(`. |
| ED-APPEAR-012 | MUST | **Identifier under caret**: occurrences of the symbol (reads and writes in their colors, declaration emphasized) at the caret are highlighted in the file shortly after the caret stops (registry-free default: on, `HIGHLIGHT_IDENTIFIER_UNDER_CARET`); the highlight is part of the daemon work (`CI-DMN`), cancelled when the caret moves. | Caret on a variable: other uses highlighted. |
| ED-APPEAR-013 | MUST | **Selected-text occurrences** (`Highlight occurrences of selected text`, default on): while a non-empty selection exists on a single line, other whole matches of the selected text in the visible part of the document are highlighted with the search-result background; nothing is highlighted when there are more than 50 matches (registry `editor.highlight.selected.text.max.occurrences.threshold`) or when the selection spans lines or is blank. | Select `foo`: other `foo` highlighted. |
| ED-APPEAR-014 | SHOULD | **Selection**: the selection background covers text, and for a selection that extends past the end of a line the line break area is marked by a small extension (not the full width). Column (block) selections are drawn as rectangles. | Multi-line selection shows line-end extension. |
| ED-APPEAR-015 | SHOULD | **Intention bulb** (default on): a lightbulb icon appears in the gutter-adjacent margin of the caret line when an intention or quick fix is available (`CI-ALT`); clicking it opens the Alt+Enter popup. | Caret on an error: bulb. |

## 4. Scrolling

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-APPEAR-020 | MUST | **Scroll offset**: when a caret move would leave the caret within fewer than the vertical scroll offset (default **1** line) of the top or bottom edge, the viewport scrolls the minimum amount to restore that margin; likewise 3 columns horizontally. The **scroll jump** (default 0) is an extra number of lines scrolled at once; with 0 the scroll is minimal, one line per line. | Viewport 10 lines: moving down from the 9th visible line scrolls one line. |
| ED-APPEAR-021 | SHOULD | A programmatic caret move (navigation, search result) to an off-screen place **centers** the line when it is far away (more than a page), and scrolls minimally when it is near; the exact distance threshold is an open question (section 11). | Go to line 500 from line 1: line 500 is centered. |
| ED-APPEAR-022 | MUST | Below the last line the editor shows **5 virtual lines** (registry `editor.virtual.lines`) so the last line can be scrolled up; with `Show virtual space at the bottom of the file` it shows one more page. At the right the editor allows 3 extra columns. | Scroll to the end: last line is above the bottom by 5 line heights. |
| ED-APPEAR-023 | SHOULD | Smooth scrolling (default on) animates scrolls of the keyboard and wheel with an interpolation (about 100 ms, registry `editor.scrolling.animation.interval.ms` 100), disabled while a key repeat is held for caret moves and when the editor is hidden; navigation scrolls use their own duration (registry `editor.navigation.scroll.duration` is internal). | Scrolling animates; typing never animates. |
| ED-APPEAR-024 | SHOULD | Mouse wheel scrolls by the platform's wheel increment (typically three lines) per notch, Shift+wheel scrolls horizontally, Ctrl+wheel changes the font size only when `Change font size with Ctrl+Mouse Wheel` is on. | Wheel scrolls. |

## 5. Soft wraps

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-APPEAR-030 | MUST | Soft wrapping is **off** by default except for files matching the masks `*.md; *.txt; *.rst; *.adoc`; the action `Soft-Wrap` (editor and gutter menu) toggles the current editor only. A wrapped line breaks at the last place where a break is allowed (a blank or after punctuation) before the right edge of the viewport; continuation rows are indented by the original line's indent (setting on) plus the additional indent (0). Line numbers appear on the first row only. | A 300-character line in `a.txt` wraps at the window width. |
| ED-APPEAR-031 | MUST | Home and End on a wrapped line work on visual rows first (`ED-CARET-015`); Up and Down move by rows; the caret at a wrap point has two visual positions (end of the row and start of the next) that it can lean toward. | Caret at a wrap shows at the row start after Right. |
| ED-APPEAR-032 | SHOULD | The soft-wrap indicator (a return arrow at the wrap) is drawn at row ends; `Only show soft-wrap indicators for the current line` limits it to the caret line. | Indicator shown. |
| ED-APPEAR-033 | MUST | A document containing a line longer than 100000 characters is **force-wrapped** and the notice above is shown once, with the actions `Hide notification`, `Don't show again` and `Settings...`. | A 150000-character line: wrapped, notice. |

## 6. Whitespace, indent guides, margins, tabs

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-APPEAR-040 | MUST | `Show whitespaces` (default off) draws a middle dot for a space and the tab marker per `Tab character rendering` (default horizontal line; alternatives long arrow and `>`) for a tab, in the whitespace color, for the selected classes only: Leading, Inner, Trailing (all on) and Selection (also inside a selection even when the class is off). Line breaks are not drawn (a non-default CR/LF is shown by the status bar). | Turn it on: spaces become dots. |
| ED-APPEAR-041 | MUST | **Indent guides** (default on): a thin vertical line at each indent level of a block, from the block's first to its last line, in the indent-guide color; the guide of the block containing the caret is drawn in the stronger color (`Highlight only the current declaration` is a separate option that limits the declaration highlight). Guides are not drawn for lines with indent below one level or inside a string; tab characters count by tab size. | Nested block: guides at 4 and 8 columns. |
| ED-APPEAR-042 | MUST | The **right margin** guide (default on) is a vertical line at the code style's right margin column (default 120); it is hidden when the margin equals the maximum (a way to disable it); `Wrap when typing reaches right margin` is off by default; **soft margins** (additional guides) come from the code style. The guide is shown in read-only files by default (registry `editor.show.right.margin.in.read.only.files`). | Line at column 120. |
| ED-APPEAR-043 | SHOULD | Tab characters are rendered with a width to the next tab stop (`Tab size` from the code style, default 4); `Inside tabs` lets the caret sit inside the width (default on). | Caret inside a tab renders mid-tab. |
| ED-APPEAR-044 | SHOULD | Special characters (control characters, zero-width, bidirectional marks) are drawn as boxed abbreviations from the Unicode name (advanced setting, default on). | A U+200B is drawn as `ZWSP`. |

## 7. Breadcrumbs and sticky lines

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-APPEAR-050 | MUST | **Breadcrumbs** (default on, bottom): a strip below the text (above when configured) showing the chain of enclosing structural elements of the caret (for Java: class, method, ... block statements), built by a language *breadcrumbs provider*; the strip is shown only for languages enabled in the setting `Languages:` (a per-language switch, default: all languages with a provider). Clicking an item moves the caret to its start (Ctrl+click selects it); hovering shows a tooltip with the element text start; the item under the caret is emphasized; the strip scrolls horizontally when it does not fit. | Caret in a method: `Class > method()` shown. |
| ED-APPEAR-051 | MUST | **Sticky lines** (default on, maximum **5** lines): when the first line of a structural element (class header, method signature, loop, if, ...) scrolls above the top of the viewport while its end is still below it, that line is pinned at the top of the editor, nested elements stacking in order outermost first, up to the maximum; the pinned lines hide with `Show sticky lines while scrolling` off or for languages switched off; a click on a pinned line navigates to it; the element list comes from the same provider as the breadcrumbs and is refreshed by a background pass after document changes. | Scroll inside a long method: the method header stays pinned at the top. |
| ED-APPEAR-052 | SHOULD | The sticky-lines area and breadcrumbs respect the theme's sticky-line background and show a shadow below the last pinned line (registry `editor.show.sticky.lines.shadow`, default on). | Visual. |

## 8. Inlay hints (framework)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-APPEAR-060 | MUST | An **inlay** is a non-text element drawn between characters (*inline*), between lines (*block*) or after the end of a line (*after-line-end*). It occupies visual columns but no offsets (`ED-CARET-112`), is not part of selections or copy, and moves with its anchor offset. Inlays are created by *hint providers* and re-created when the file changes; they never block typing and are computed in the background. | Typing near an inlay does not change it until recomputed. |
| ED-APPEAR-061 | MUST | Settings, Editor, Inlay Hints: a master switch (default on), per-language switches, and per-provider switches with provider-specific options; changes apply immediately to open editors; a hint type disabled by default stays off until enabled. | Disable a provider: its hints disappear. |
| ED-APPEAR-062 | MUST | **Parameter name hints** (Java; default on): before each argument of a call the parameter name is shown as a small label `name:`; hints are not shown when the parameter name is contained in the method name, when the argument is a lone literal and the method matches a **block list** (default patterns such as `(begin*, end*)`, `(key, value)`, `(message)`, `*Exception`, `*.set*(*)`, `*.add(*)`, `*.get(*)`, `*.equals(*)`, `*.println(*)`, `java.lang.Math.*`, `*.Arrays.asList`), for builder-like methods, for sequentially numbered names, or when only one argument has the same type as the others. Options and defaults: show for enum constants on, for `new` expressions on, when the expression type is clear off, with names contained in the method name off, for non-literals with several same-type parameters off. | `foo("", false)` shows `name:` and `isChanged:` hints for a non-blacklisted method. |
| ED-APPEAR-063 | SHOULD | Other standard hint kinds (types of implicit `var`/lambdas, chained call types, return-value hints, code vision lines above declarations for usages and authors) follow the same provider and settings model; `Use editor font in inlays` (default off) chooses the font; hints are rendered in the inlay-hint color with rounded background. | Settings list the kinds. |
| ED-APPEAR-064 | SHOULD | Pressing Alt (or Ctrl+Alt) on the hint and the context action `Show hints for...` give quick toggles; the hint under the caret is skipped by Left/Right when it cannot be placed. | Not required for the first release. |

## 9. Fonts and zoom

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-APPEAR-070 | MUST | Editor font size ranges from **4** (minimum) to **40** (maximum, a system property can raise it), default 12 before scheme override; line spacing from 0.6 to 3, default 1.0. | A scheme font size of 60 is clamped to 40. |
| ED-APPEAR-071 | MUST | `Increase Font Size` and `Decrease Font Size` (actions `EditorIncreaseFontSize`, `EditorDecreaseFontSize`, no default keys) change the font size of the **current editor only** by 1 point; the `... in All Editors` actions (`EditorIncreaseFontSizeGlobal` Alt+Shift+Period and `EditorDecreaseFontSizeGlobal` Alt+Shift+Comma) change the scheme's font size by 1 point times the IDE scale; both are applied only while the resulting unscaled size is at least 8 and at most 40; `Reset Font Size` and `Reset Font Size in All Editors` restore the scheme size. With `Change font size with Ctrl+Mouse Wheel` on, Ctrl+wheel up increases and down decreases, by one point per notch. A zoom indicator (setting default on) shows the percentage. | Alt+Shift+Period from 12: 13; from 40: unchanged. |
| ED-APPEAR-072 | SHOULD | `Zoom In IDE` (Shift+Alt+=), `Zoom Out IDE` (Shift+Alt+-) and `Reset IDE Scale` (Shift+Alt+0) change the whole UI scale. | Menu present. |

## 10. Timings and popup behavior (summary table)

| Fact | Value |
|---|---|
| Caret blink period | 500 ms (10 to 1500) |
| Brace-scope hint delay (partner above the viewport) | 300 ms |
| Parameter-info auto-popup delay | 1000 ms (0 to 5000) |
| Documentation popup delay (auto and mouse) | 500 ms (0 to 5000) |
| Editor tooltip delay | 500 ms (1 to 5000) |
| General IDE tooltip initial delay (registry `ide.tooltip.initialDelay`) | 1200 ms; for highlighter tooltips 150 ms; dismiss after 4000 ms; re-show delay 500 ms |
| Mouse selection state reset (registry) | 1000 ms; dead zone 4 px; drag sensitivity 5 px |
| Smooth caret duration (when on) | 120 ms |
| Undo-transparent caret movement | off |

## 11. Open questions

* Exact hit boxes and pixel metrics of the gutter, folding markers, and sticky lines are measured
  against the reference IDE by the implementation team (they depend on the theme and font).
* The distance above which a programmatic navigation centers the target line instead of scrolling minimally.
* The completion auto-popup delay (the setting exists in the UI; no default found in the files read).
* Behavior of the caret row highlight when a selection covers the whole line.
* Language lists of the breadcrumbs and sticky line switches beyond Java and Kotlin.

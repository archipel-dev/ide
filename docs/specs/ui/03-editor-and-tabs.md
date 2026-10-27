# UI-ED: editor, tab strip and gutter

Reference: `04-editor-project-tree.png`, `09-completion.png`, `20-problems.png`. Archipel:
`new-ui-dark-u04-*.png` (New UI Dark / new_ui), `default-islands-dark-04-*.png` (settings default, islands look).
Window pixels (see `00`). Responsible code: `crates/archipel-ide-ui` (editor view, tab strip, gutter painting),
kit `Tabs`/`Tab` component and `EditorColors`.

## Requirements

### Tab strip

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| UI-ED-001 | MUST | The editor tab strip is 40 px high plus a 1 px bottom border Gray3 `#393B40` (`EditorTabs.underTabsBorderColor`), strip background Gray1 `#1E1F22` (the same as the editor). The first line of the editor starts at the next row. | Column x=900: `#1E1F22` run of 40 px (y 41..80), 1 px `#393B40` at y=81, editor from y=82. |
| UI-ED-002 | MUST | There is no tab "pill". The selected tab is marked by a **4 px** underline with rounded ends (`EditorTabs.underlineHeight` 4, `underlineArc` 4), at the tab's full width, drawn over the strip's bottom border (rows 78..81). Focused editor: Blue6 `#3574F0`; unfocused editor: Gray6 `#5A5D63`. Tab background equals the strip (`underlinedTabBackground` Gray1); hover background is transparent `#1E1F2200`. | Row y=79 across the selected tab is `#3574F0` (focused) or `#5A5D62` (unfocused) for the tab width, 117 px for "App.java". Tab outside the underline has no outline. |
| UI-ED-003 | MUST | Tab geometry for a file tab: left padding 13 px, 16 px file-type icon, 6 px gap, name in regular 13 px, 12 px gap, 8x8 close glyph, right padding 12 px. "App.java" is 117 px wide. First tab starts 1 px after the editor's left edge. | Reference: tab x 1..117 (relative), icon 14 px wide at +13, text at +34, close glyph at +98..105, tab ends at +116. |
| UI-ED-004 | MUST | Close glyph is shown on every tab in Gray7, hover fill; unselected tabs are drawn at 0.75 alpha (`EditorTabs.unselectedAlpha`). A tab that is not selected shows no underline. | - |
| UI-ED-005 | MUST | The strip's right end holds one kebab (3 dots vertical, 2x12 px glyph at x = W-60) for the tab actions; no other chrome. | - |
| UI-ED-006 | SHOULD | Tab text for a file with problems is underlined by a wavy red line (`20-problems`: the file name `App.java` with a squiggle) and the tab icon shows the class type badge. | - |

### Gutter and lines

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| UI-ED-010 | MUST | Editor text size is JetBrains Mono at the same size as Archipel (advance 7.8..8.0 px, measured 7.85 in both). | A line of 40 characters is 311..320 px wide. |
| UI-ED-011 | MUST | **Line pitch is 22 px.** (Archipel: 20 px.) The caret row (Gray `#26282E`) spans gutter and text, full row height, edge to edge of the editor area including the gutter and the error stripe. | Line numbers' vertical centers differ by 22 px; caret row run is 22 px tall. |
| UI-ED-012 | MUST | Gutter layout from the editor's left edge: line numbers right aligned, right edge at +23 px; run/debug icon 12 px wide at +30..+41; fold chevron 5x8 at +50..+54; VCS change bar 4 px wide at +62..+65 right before the text; **text starts at +68 px** (gutter 68 px wide). Gutter background equals the editor (`#1E1F22`), no separator line other than the 1 px vertical rule at +65 (`#32313A`). | Row of a line with an icon: run triangle bbox x 30..41 (12x14), chevron x 50..54. Text bbox of line 1 starts at +69. |
| UI-ED-013 | MUST | Line number color `#4B5059` (scheme `LINE_NUMBERS_COLOR`), the current line's number `#A1A3AB` (`LINE_NUMBER_ON_CARET_ROW_COLOR`); font size 12 (digit height 9 px). | - |
| UI-ED-014 | MUST | Modified-line change bars: 4 px wide in the gutter, added `#549159`, modified `#375FAD`, deleted marker `#868A91` (scheme `ADDED_LINES_COLOR`, `MODIFIED_LINES_COLOR`, `DELETED_LINES_COLOR`). | After typing a new line, a 4 px wide `#538F59` (reference rendering) bar at +62..+65 for that row. |
| UI-ED-015 | MUST | Indent guides are drawn in `#313438` (`INDENT_GUIDE`), 1 px, at each indent level, 8 px apart (4 columns); the one at the caret's block uses `#666870`. | Reference pixel at x=599 (line 11..15, window) is `#313438`. |
| UI-ED-016 | MUST | Folded regions show the placeholder (`{ return ... ; }`) as a one-line box with Gray3 `#393B40` background and text `#868991`, braces boxed. | - |
| UI-ED-017 | SHOULD | Inline "code vision" hints (author, N usages) use 11..12 px UI font, Gray `#868A91`, 12 px after the code, with the small user glyph before the name. | - |

### Right edge

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| UI-ED-020 | MUST | The error stripe is a 30 px wide column inside the editor (x = right edge - 31 .. -1) with a 1 px Gray3 `#393B40` rule on its left, background = editor background; it carries the inspection widget (green check near its top), problem counters and the 14x2 tick marks (red/yellow/blue) for the whole file. | Column at editor right - 31 is `#393B40` over the full height; check glyph about 14x10 within the top 30 px of the stripe. |
| UI-ED-021 | MUST | No always-visible scrollbar track: scrollbars are overlays that appear on hover/scroll (`ide.scroll.*`); the area right of the text is the error stripe. A horizontal scrollbar is not drawn at rest. | At rest, no `#424448` thumb along the bottom edge. |
| UI-ED-022 | MUST | There is no extra 12 px chrome-colored column between the editor and the right tool stripe. | Row y=500 near the right: error stripe (30 px, editor color) directly followed by the 1 px stripe border and the 40 px stripe. |

### Text attributes (Dark scheme)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| UI-ED-030 | MUST | Colors: text `#BCBEC4`, keyword `#CF8E6D`, string `#6AAB73`, number `#2AACB8`, comment `#7A7E85`, doc comment `#5F826B` italic, annotation `#B3AE60`, method declaration `#56A8F5`, instance/static method call `#57AAF7` (static italic), constant/static field `#C77DBB` italic, class/identifier `#BCBEC4`, caret `#CED0D6`, selection per scheme. | Dominant non-background color of the word `public` is `#CF8E6D` (+-3). |
| UI-ED-031 | MUST | Identifier under caret `#373B39` background; matched brace `#43454A` bold; search result `#2D543F` background with stripe mark `#42BD77`; text-search result (Find in Files preview) `#114957`; write usage `#66313F`; warning underline `#F2C55C`, error wave `#FA6675`. | In `10-find-in-files`/`04` the identifier under the caret is `#363A39`. |

## Measured values

| Value | IntelliJ | Archipel | Diff |
|---|---|---|---|
| Tab strip height | 40 + 1 border (m) | 42 (y 41..82), no border row | +2, border missing |
| Tab strip bg | `#1E1F22` | `#2B2D30` | color differs |
| Tab bottom border | `#393B40` | none | missing |
| Selected tab marker | underline 4 px, `#3474F0`, arc 4 | 28 px tall rounded outline pill: 1 px `#3574F0` border, fill `#1E1F22` (x 327..441, y 46..73) | different component |
| Tab width "App.java" | 117 | 115 | -2 |
| Tab icon / text / close | icon 14 px at +13, text at +34, close 8x8 at +98 | icon 12 px at +11, text at +32, close 8x8 at +95..102 | -2 each |
| Tab left offset in strip | 1 | 8 | +7 |
| Line pitch | 22 (m) | 20 | -2 px (-9 %) |
| First line center below editor top | 11 | 10 | -1 |
| Text start (from editor left) | 68..69 | 100..101 | +32 px |
| Line number right edge | +23 | +36 | +13 |
| Run icon | 12x14 at +30 | 7x10 at +46 | -5 px wide, +16 px right |
| Fold chevron | 5x8 at +50..54 | 5x8 at +62..66 | +12 |
| Gutter vertical rule | 1 px at +65 | none | missing |
| Indent guides | present (`#313438`) | none found | missing |
| Caret row color | `#25282D` (scheme `#26282E`) | `#26282E` | 0 |
| Line number colors | `#484C54`; current `#9DA1A8` | `#5D5F66`; current `#A1A3AB` | ours lighter by about 16 levels for non-current numbers |
| Error stripe | 30 px, 1 px `#393B40` left rule, editor bg | none; 12 px chrome-colored column x 1387..1398 then 1 px `#1E1F22` | structural |
| Horizontal scrollbar | none at rest | 7 px thumb `#424448` at y 864..870 across 350 px | extra |
| Inspection check glyph | about 14x10 near the top of the stripe | similar size, at editor right -26..-14 | position differs with the missing stripe |
| Editor font advance | 7.85..8.0 | 7.9 | 0 |
| Code vision text | "sample" with user glyph | fixture author text "Archipel IDE Scenes", no glyph | glyph missing |
| Fold placeholder | boxed `{ ... }` | boxed (bg `#393B40`) | 0 |

## Notes

- `ic: platform/platform-resources/src/themes/expUI/expUI_dark.theme.json` (`EditorTabs`), `expUI_darkScheme.xml` (all scheme colors),
  `ic: platform/util/resources/misc/registry.properties` (`editor.caret.width=2`; line height is not a registry value).
- The reference line pitch of 22 px was measured on line numbers at y=113, 135, 157, 179, 201 (png) and on the caret row (22 px). It is a
  derived value (font metrics times the New UI line spacing); implement it as "22 px at 13 px JetBrains Mono, 100 %".
- Text rendering: reference LCD antialiasing; Archipel grayscale. Not a spec item (see `00`).
- Open question: the 1 px vertical rule at +65 may be the New UI "gutter/editor" separator or a vertical indent line of the root block; it is full-height in all captures.

## After the ui-parity pass (New UI Dark capture `u04`, window 1440x900)

Captures: `docs/parity/evidence/archipel-ide/ui/after/`. Geometry applies to every look.

| Value | IntelliJ | Archipel now | Remaining diff |
|---|---|---|---|
| Line pitch | 22 | 22 (numbers' centers 93.5, 115.5, ...) | 0 |
| Line number right edge | +23 | +23 | 0 |
| Run icon | 12x14 at +30..41 | 16 px box centered at +36 (glyph about 11 px) | check glyph bbox against 12x14 |
| Fold chevron | 5x8 at +50..54 | 5x8 at +50..54 | 0 |
| Gutter rule | 1 px at +65 | 1 px `#313438` at +65 | 0 |
| Text start | +68..69 | +69 | 0 |
| Indent guides | `#313438`, per indent level | `#313438` at every level from the block's own, 4 columns apart | caret-block guide `#666870` missing |
| Non-current line number | `#4B5059` | `#4B5059` (kit scheme; the old `#5D5F66` was a stale capture) | 0 |
| Error stripe | 30 px, 1 px left rule `#393B40`, editor bg | 30 px, rule in the kit group-separator color (`#43454A` in New UI Dark), editor bg, 14x2 ticks, check in its top | rule 1 level lighter than `#393B40` |
| Chrome column right of the editor | none | none | 0 |
| Horizontal scrollbar at rest | none | none (width follows the longest line seen) | 0 |
| Search result highlight (editor) | `#2D543F` | kit `TEXT_SEARCH_RESULT` `#114957` | scheme value differs; kit |
| Tab strip, pill vs underline | see above | unchanged (kit `Tabs`, `new_ui` look has the underline) | Islands look keeps pills by design |

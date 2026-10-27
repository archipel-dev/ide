# UI-POP: popups, menus, completion and list rows

Reference: `05-search-everywhere.png`, `06-find-action.png`, `07-main-menu.png`, `09-completion.png`, `10-find-in-files.png`,
`19-project-context-menu.png`. Archipel: `new-ui-dark-u05/u06/u07/u10/u19`; the completion popup could not be captured in
the current build (jdtls still starting in the scene run), so the completion column uses
`docs/parity/evidence/lsp/09-java-completion-jdtls.png` (2026-10-10, Islands Dark, 1600x1000) and is marked "old".
Window pixels. Responsible code: `crates/archipel-ide-ui` `popups.rs`, `context_menus.rs`, `widgets.rs` (`list_row`, `menu_item`),
completion popup view; kit `Popover`, `ContextMenu`, `MenuItem`, `MenuSeparator`, `ListRow`.

## Requirements

### Common

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| UI-POP-001 | MUST | Popup frame: 1 px border Gray4 `#43454A` (`Popup.borderColor`; inactive `#393B40`), body Gray2 `#2B2D30`. Menus use Gray4 for border and separators. | Row through a popup: border pixel `#434549`+-3 at the edge; interior `#2B2D30`. |
| UI-POP-002 | MUST | List rows are **24 px** (`List.rowHeight`) with list border 4 px top and bottom (`List.border` 4,0,4,0); the selected row is Blue2 `#2E436E` (focused list), inset 8 px from the popup border, corner radius about 4. Text 13 px UI font, row icons 16 px, secondary text right aligned in Gray7. | Selected row run: 24 px tall in column x=popup+200; first row starts 4 px below the list top. |
| UI-POP-003 | MUST | Popup corner treatment is "square, 1 px border" in the reference capture; with compositing IntelliJ rounds popups. Archipel uses radius 8 and a soft shadow. Pick one rule and document it (needs live observation, `00`). | - |
| UI-POP-004 | MUST | Matched text in result lists is highlighted with Yellow5 `#BA9752` background and dark text (Search Everywhere, Find in Files). Not a blue/teal fill. | In `05` the matched "main" has background `#B59252..#BA9752`. |

### Search Everywhere / Find Action

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| UI-POP-010 | MUST | Size 705x700 at a 1600x980 window (about 44 % x 71 %), horizontally centered on the window, top at 189 px (19 % of the height). | Left edge x=463, top y=189. |
| UI-POP-011 | MUST | Layout, top to bottom: **header band** 41 px, Gray3 `#393B40`, tabs `All | Classes | Files | Symbols | Actions | Text` (13 px) on the left, "Include non-project items" checkbox and three 16 px icon buttons (preview, filter, open in window) on the right; the selected tab has a **3 px Blue underline** (`#2E74E8` measured), not a filled pill; **search row** 40 px with magnifier-with-dropdown glyph at +19, text, right aligned hint "Type / to see commands" in Gray7; 1 px divider `#393B40`; **list** (rows 24 px, 4 px top padding, first row at y=header+87 px); **footer** 29 px, Gray3 `#393B40`, selected item's path (Gray7) left and a blue link ("Open In Right Split") right. | Column x=700 from popup top: `#393B40` 41 px (y 21..61 rel.), `#2B2D30` search row, 1 px `#393B40`, `#2B2D30` list, `#393B40` footer 29 px, border. |
| UI-POP-012 | MUST | Row anatomy: kind icon at +18, text at +37, module/location right aligned 16 px from the border, followed by a 16 px folder icon for module rows; action rows show the shortcut inline in Gray7 at 11 px next to the name and the group on the right; toggles ("ON"/"OFF") as small badges about 48x15. Group separators render as rows with a Gray3 label. | - |
| UI-POP-013 | SHOULD | Scrollbar thumb at the popup's right edge, 6 px, Gray `#4C4D51`, 8 px from the border. | - |

### Context and main menu

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| UI-POP-020 | MUST | Menu items are **26 px** high (Archipel: 24); text starts at +36 from the menu's left border, a 12..16 px icon column at +17 (items with icons are common: Cut, Copy, Paste, Run, Debug, Reload from Disk); the shortcut is right aligned 13 px from the right border in Gray7 (`Menu`/`shortcutForeground`); sub-menu chevron 5x8 at -24..-20 from the right border. | Cut, Copy, Copy Path: centers 295, 321, 347 (26 px pitch). |
| UI-POP-021 | MUST | Mnemonics: one letter of each item is underlined (New, Cut, Copy, Paste, Find Usages, Analyze, Rename, Refactor, Reformat Code, Delete, Run, Debug, Local History, Git, ...). | Underline row under the mnemonic letter in `19` (`N` of "New" at y=265). |
| UI-POP-022 | MUST | Separators: 1 px Gray4 `#43454A` line, full width (no inset), inside a 9..10 px zone; first item is 5 px below the top border. | Separator lines at 277, 390, 451, 512, 547 (png) for the project menu: group zones of 35 px between item centers. |
| UI-POP-023 | MUST | Context menu width is content-driven: the project menu is 381 px, File menu 305 px; minimum padding between the longest label and its shortcut 24 px. | - |
| UI-POP-024 | MUST | A sub-menu opens 500 ms after hover (`ide.popup.auto.delay`) attached directly to the parent item's right edge, same height rhythm; it is not a detached wide panel. | Archipel's sub-menu "New" is a separate 600 px wide box 4 px right of the parent. |
| UI-POP-025 | SHOULD | Menu bar mode: hamburger replaced by `File Edit View Navigate Code Refactor Build Run Tools Git Window Help` at 13 px, items padded 8 px; opened File menu top edge at the toolbar border (y=40). | - |

### Code completion

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| UI-POP-030 | MUST | Lookup rows are 24 px, text in the **editor font** (method name) followed by the gray parameter list, return type right aligned in the editor font and Gray7; a 16 px kind icon (round "m" badge, red/pink for methods; field/variable/class badges) at +25; star mark for the most relevant item at +8; up/down arrows for override/implement hints. | Reference: rows at y=338, 362, 386 (png), popup x 620..1192. |
| UI-POP-031 | MUST | The selected lookup row is **Gray4 `#43454A`**, full list width (x+5 .. right-12), radius 4; NOT the blue list selection. | Row 326..349 (png) is `#43454A`. |
| UI-POP-032 | MUST | Popup size 573 px wide at 12 visible rows (11 full + a clipped one), list 24x12 plus 28 px footer (Gray3, tip text "Ctrl+Down and Ctrl+Up will move caret down and up in the editor" 11 px, blue "Next Tip" link, kebab at the right). Visible 6 px scrollbar (`#5C6164`) inside the list's right edge. | Popup y=302..612 (window), footer 585..612. |
| UI-POP-033 | MUST | Popup is placed directly below the caret line and its left edge is positioned so that the first character of the item name is at the caret column (reference: caret x=665 and item text x=665 png, popup left edge 620, i.e. 45 px left of the caret). | - |
| UI-POP-034 | SHOULD | Matched characters of the prefix are highlighted Blue8 `#548AF7` (`CompletionPopup.matchForeground`); popup foreground Gray10 `#B4B8BF`. | - |

### Find in Files popup

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| UI-POP-040 | MUST | The popup is 652 px wide at 1600 (41 %), top at y=116, tall enough to reach the window bottom with a preview editor in the lower half; header band Gray3 `#393B40` 42 px with bold title "Find in Files" and a gray count text "3 matches in 2 files"; "File mask:" checkbox + combo, filter and pin icons on the right. | - |
| UI-POP-041 | MUST | Query row: magnifier + field text, clear (x), newline and the three toggles `Cc`, `W`, `.*`; scope tabs `In Project | Module | Directory | Scope` as 13 px text links with the selected one in a filled chip; results use the **UI sans font** (not monospace), match highlight Yellow5 `#BA9752`, test file rows tinted Green2 `#273828`, selected row Blue2; preview header "App.java src/main/java/com/example" in Gray7; footer "Open results in new tab" checkbox, hint "Ctrl+Enter" and the **Open in Find Window** button. | - |

## Measured values

| Value | IntelliJ | Archipel | Diff |
|---|---|---|---|
| SE popup size | 705x700 (m) | 701x587 | -4, -113 |
| SE popup position | x=463 (center +15.5 from window center), y=189 | x=371 (center +0.5), y=109 (12 % of height) | y -80 px |
| SE border | 1 px `#434549` | 1 px `#43454A`, radius about 8, shadow | radius/shadow |
| SE header + search | 41 px tabs band `#393B40` + 40 px search row, divider at +82 | 70 px tabs + search block, all `#2B2D30`, divider at +70 | -12 px, color |
| SE selected tab | 3 px underline `#2E74E8` under "All" | filled pill `#2E436E`, no underline | different component |
| SE search row | glyph + text + hint "Type / to see commands" | glyph + text; no hint | hint missing |
| SE divider under search | `#393B40` at y=search+40 | 1 px `#43454A` | color |
| SE row height | 24, first row +87 below top | 24, first row +75 below top | -12 |
| SE selection color | `#2D436D`, 8 px gap to the border, radius ~4 | `#2E436E`, 5 px gap, radius ~4 | gap -3 |
| SE footer | 29 px band `#393B40` with path + link | no band; path text in Gray at the bottom (no link) | band, link missing |
| SE header right controls | checkbox + 3 icons | checkbox only | 3 icons missing |
| SE match highlight | `#B59252` (Yellow5) | `#2E436E`-ish blue chip | color |
| Menu item height | 26 | 24 | -2 |
| Menu text offset | +36 (icon column present) | +17 (no icons) | icons missing |
| Menu shortcut right padding | 13 | 17 | +4 |
| Menu width (project menu) | 381 | 600 | +219 |
| Menu mnemonics | underlined | none | missing |
| Menu corners | square (capture) | radius 8 + shadow | see UI-POP-003 |
| Sub-menu | attached to the parent item, content width | separate box about 600x112, 4 px right of the parent | structural |
| Separator | `#434549` 1 px, ~9 px zone | `#43454A` 1 px, ~9 px zone | 0 |
| Completion popup size | 573x311 | 430x268 (old capture) | -143 |
| Completion row height | 24 | 22 (old) | -2 |
| Completion selection | `#43454A` Gray4 | `#113A5C` blue (old, islands) | color |
| Completion icons | 16 px kind badge per row | none (old) | missing |
| Completion font | editor font | editor font | 0 |
| Completion footer | tip + Next Tip + kebab | none (old) | missing |
| FIF popup size | 652 x (>=864) | 780x651 | -128 wide |
| FIF results font | UI sans | monospace | differs |
| FIF match highlight | `#BA9752` | `#114957` | color |
| FIF preview | header + highlighted line + gutter | gutter + highlight, no header | header missing |

## Notes

- `ic: platform/platform-resources/src/themes/expUI/expUI_dark.theme.json` (`Popup`, `List`, `CompletionPopup`, `SearchEverywhere`, `Menu`).
- Menu hover/pressed colors are not visible in the captures (the reference menus show no hovered item); see `00`.
- The SE underline measured `#2E74E8` is not a theme literal; use Blue6 `#3574F0` unless a live check shows otherwise.

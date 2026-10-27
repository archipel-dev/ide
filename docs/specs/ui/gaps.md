# UI gaps: every measured difference, ranked by visibility

## Decision: the default stays Islands Dark

The default theme stays **Islands Dark** with the `islands` look (decision of the product owner, binding). Gap 1 is
therefore closed as "by decision, not a defect". Requirements in `01`..`07` that name colors or the frame of New UI
Dark (`#2B2D30` frame, `#1E1F22` editor, no islands, no 4 px gaps, 40 px stripes in the frame color) apply to the
**New UI Dark** theme option with the `new_ui` look. Look-independent geometry and behavior (line pitch, gutter,
error stripe, icon sizes and pitch, row heights, popup anatomy, mnemonics, underline thickness, status bar height,
control heights) apply to **all** looks. Gaps below are fixed under that rule.

Facts only. "Where" names the likely responsible place (crates/archipel-ide-ui files are under `crates/archipel-ide-ui/src/`; "kit" = archipel-kit component). Spec IDs in brackets.

| # | Visibility | Difference (IntelliJ vs Archipel) | Where | Spec |
|---|---|---|---|---|
| 1 | Very high | Default theme/look: reference New UI Dark (`#2B2D30`/`#1E1F22`, no islands); Archipel default Islands Dark (`#1A1B1E` frame, `#111213` islands, 4 px gaps) | `theme_engine.rs` `DEFAULT_THEME`, settings default `appearance.uiLook` | UI-COL-001 |
| 2 | Very high | Editor line pitch 22 px vs 20 px | editor view, `theme.rs` line metrics | UI-ED-011 |
| 3 | Very high | Gutter 68 px vs 100 px; icon and chevron positions, run icon 12 vs 7 px | editor gutter painting | UI-ED-012 |
| 4 | Very high | Editor tabs: 4 px underline vs outlined pill; strip bg, 1 px bottom border missing, strip 42 vs 41 | kit `Tabs`, tab strip in `editor_tabs` | UI-ED-001..003 |
| 5 | Very high | Toolbar content: extra "Code/Agents" switch, hammer, title text; missing kebab, collaboration, settings gear; icons 12..14 vs 16..19 px, pitch 24..29 vs 40 | toolbar/title code, kit icon button | UI-CHR-003..005 |
| 6 | High | Completion popup: Gray4 selection, kind icons, 24 px rows, footer, 573 px width (Archipel older capture: blue selection, 22 px rows, no icons) | completion popup view | UI-POP-030..032 |
| 7 | High | Search Everywhere: header band Gray3, underline tab, hint text, header icons, footer band + link, 705x700 at y=189 vs 701x587 at y=109 | `popups.rs` search everywhere | UI-POP-010..012 |
| 8 | High | Menus: 26 vs 24 px rows, icons column, mnemonics, widths (381 vs 600), detached sub-menu | `context_menus.rs`, kit `ContextMenu` | UI-POP-020..024 |
| 9 | High | Project tree: no row tints (excluded `#3D3223`, tests `#273828`), no root chevron | project panel | UI-TW-008, 009 |
| 10 | High | Right edge: no 30 px error stripe with rule; 12 px chrome column; visible horizontal scrollbar | editor view | UI-ED-020..022 |
| 11 | High | Find in Files: monospace results, teal match color, 780 vs 652 px, no preview header | find popup | UI-POP-040, 041 |
| 12 | Medium | Stripe buttons 32 vs 30 px, pitch 38 vs 40, different button sets | tool window stripe, kit | UI-TW-002, 003 |
| 13 | Medium | Status bar 26 vs 29+1 px, no top border, extra widgets, lock missing | status bar | UI-SB-001, 003 |
| 14 | Medium | Tool window header: no title chevron, 2 of 6 actions, bordered "..." box; underline 2 vs 3 px | tool window header | UI-TW-005, 010 |
| 15 | Medium | Project tool window width 19 % vs 29 % | layout defaults | UI-TW-006 |
| 16 | Medium | Indent guides missing | editor painting | UI-ED-015 |
| 17 | Medium | Settings: combo 24 vs 28 px and fill, tree selection inset+radius vs full-bleed, footer strip, help button, OK has border | settings view, kit `Button`/`ComboBox` | UI-DLG-001..005 |
| 18 | Medium | Search match highlight color (blue/teal vs Yellow5) | `widgets.rs`, popups | UI-POP-004 |
| 19 | Low | Non-current line number color `#5D5F66` vs `#484C54` | editor colors | UI-ED-013 |
| 20 | Low | Popup corners/shadow (radius 8 vs square capture) | kit `Popover` | UI-POP-003 |
| 21 | Low | First-run "Welcome to Archipel IDE" card and notification/banners present in a fresh capture | first-run flow, banners | - |
| 22 | Low | Code vision hint lacks user glyph | editor inlay | UI-ED-017 |

## Status after the ui-parity pass

Images: `docs/parity/evidence/archipel-ide/ui/after/`. Kit changes: branch `feat/ide-ui-parity`, commit `c7c25cd`.

| # | Status | Remaining |
|---|---|---|
| 1 | closed by decision (Islands Dark stays the default) | - |
| 2, 3, 10, 16 | fixed (22 px pitch, 68 px gutter with rule, 30 px stripe, no scrollbar at rest, indent guides) | run glyph about 1 px narrower |
| 4 | not touched: look dependent (`new_ui` look has the underline, Islands keeps pills) | - |
| 5 | pitch 40 and 30 px buttons fixed | kebab, collaboration, gear missing; Code/Agents switch kept |
| 6 | selection, 573 px width, 28 px footer band | tip text, Next Tip, kebab |
| 7 | size, position, header band, underline, hint, rows, footer band and link | yellow match needs the kit commit |
| 8 | content-driven width, icons | 26 px rows (look dependent), mnemonics (kit ready, IDE call missing), attached sub-menu |
| 9 | tints and root chevron | - |
| 11 | width 652, UI font | preview header |
| 12 | 30 px buttons, 40 px pitch | button sets differ |
| 13 | 30 px with 1 px line in the New UI look | extra widgets, lock |
| 14 | 3 px bottom underline, kebab button | title chevron, locate/expand/collapse |
| 15 | fixed (28.8 %) | - |
| 17 | kit buttons and combos, full-bleed selection, help button | footer strip |
| 18 | token in place | needs the kit commit |
| 19 | already `#4B5059` in the kit scheme | - |
| 20, 21, 22 | not touched | - |

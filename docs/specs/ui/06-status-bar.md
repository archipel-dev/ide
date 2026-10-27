# UI-SB: status bar

Reference: `04-editor-project-tree.png`, `09`, `20`. Archipel: `new-ui-dark-u04`. Responsible code: `crates/archipel-ide-ui` status bar, kit `StatusBar`/`Breadcrumbs`.

## Requirements

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| UI-SB-001 | MUST | Status bar is **29 px high plus a 1 px top border** `#1E1F22` (`StatusBar.topBorderWidth` 1) = 30 px; background Gray2 `#2B2D30`; the editor/tool windows end at the border. | Column x=700 at the bottom: `#1E1F22` 1 px at H-30, `#2B2D30` 29 px. |
| UI-SB-002 | MUST | Left: breadcrumbs (project, folders, current symbol) with a small chevron separator (5x8) every item, text 12..13 px Gray9 `#9DA0A8` (hover Gray12), starting at x=14 (first icon 12 px). | Chevron gaps about 10 px; first glyph bbox x=14. |
| UI-SB-003 | MUST | Right: widgets in order caret position `12:30`, line separator `LF`, encoding `UTF-8`, indent `4 spaces` (with a small icon), read-only lock toggle; widget padding 8 px each side (`StatusBar.Widget` insets 6,8), measured gaps 18..21 px; hover fill Gray4 `#43454A` when pressed. | Reference text boxes: 12:30 x 1338..1370, LF 1388..1400, UTF-8 1419..1457, 4 spaces 1477..1548, lock 1569..1581 (window). Gaps 18, 19, 20, 21. |
| UI-SB-004 | SHOULD | Progress and background task text appear on the right side before the widgets; no custom status strings in the position of the widgets. | - |

## Measured values

| Value | IntelliJ | Archipel | Diff |
|---|---|---|---|
| Height | 29 + 1 border | 26, no border | -3, border missing |
| Background | `#2B2D30` | `#2B2D30` | 0 |
| Text vertical extent | y 959..968 digits (9 px) | y 883..892 digits (9 px) | 0 |
| Widget gaps | 18..21 | 17..18 | -2 |
| Widgets | position, LF, UTF-8, indent, lock | message text, position, LF, UTF-8, indent, language server state, AI glyph | 2 extra, lock missing |
| Breadcrumb chevron | 5 wide, gap ~10 | 4 wide, gap ~11 | 0 |
| Left start | x=14 | x=16 | +2 |

`ic: platform/util/ui/src/com/intellij/util/ui/JBUI.java` (`StatusBar.Widget` insets 6,8; hover insets 3,4,1,4), `IdeStatusBarImpl.kt` (top border width), `expUI_dark.theme.json` (`StatusBar`).

## After the ui-parity pass (`u04`)

| Value | IntelliJ | Archipel now | Remaining diff |
|---|---|---|---|
| Height | 29 + 1 border | 29 + 1 line (`#1E1F22`) in the New UI look; 30 in Islands (kit metrics) | 0 |
| Widgets | position, LF, UTF-8, indent, lock | unchanged | lock missing, extra status items |

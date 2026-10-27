# UI-DLG: dialogs and the Settings dialog

Reference: `08-settings.png`, `15-run-config.png`, `02-new-project.png`. Archipel: `new-ui-dark-u08-settings-appearance.png`, `u28-settings-keymap.png`.
Reference dialogs sit in a 19 px OS title bar (openbox); sizes below are the dialog content. Responsible code: `crates/archipel-ide-ui` settings and dialog views, kit `Surface::Dialog`, `Button`, `Checkbox`, `ComboBox`, `SearchField`.

## Requirements

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| UI-DLG-001 | MUST | Buttons are 72x28 minimum (`Button.minimumSize` 72,28 in the theme; JBUI fallback 72x24), radius 4 (arc 8), 12 px gap, order **OK (primary, Blue6 `#3574F0`, white text, no border) | Cancel | Apply (disabled until a change)**, right aligned with 16 px right margin; a round `?` help button sits at the left (22x22 at x+15). | OK run 72 px wide, 28 px tall, color `#3574F0`; Cancel at +84, Apply at +168. |
| UI-DLG-002 | MUST | Secondary buttons: fill Gray2 `#2B2D30`, 1 px border Gray5 `#4E5157`; the primary has no visible border. | Cancel border `#4E5056`; OK interior `#3574F0` up to the edge. |
| UI-DLG-003 | MUST | Settings dialog: 992x710 content at a 1600x980 window, centered; footer band 51 px above the bottom: a 1 px `#1E1F22` rule over a Gray3 `#393B40` strip 15 px tall (shadow-like) then the buttons row. | Footer rule row `#1E1F22` at content bottom-55. |
| UI-DLG-004 | MUST | Left tree: 235 px wide, 1 px divider; search field at the top (28 px, radius 4, 1 px `#4E5056`, magnifier), rows **25 px**; top-level groups semibold, children regular indented 16 px with a chevron on groups; the selected row is **full-bleed** (from the panel's left edge to the divider, no inset, no radius) in Blue2 `#2E436E`. Project-level pages show a small "project" glyph at the right. | Selection run x from the dialog left to the divider, 25 px tall. |
| UI-DLG-005 | MUST | Right pane: breadcrumb "Appearance & Behavior > Appearance" semibold 13 at +16, back/forward arrows at the right; section headers ("Accessibility", "UI Options") are 13 px text followed by a 1 px Gray3 `#393B40` line that spans to the right margin; control labels Gray12, hints Gray7 11 px, checkboxes 16x16 (reference bbox 15x15), combos 28 px tall (`#393B40` fill, 1 px `#4E5056` border, radius 4, chevron right); two-column checkbox grid under UI Options. | Combo: 28 px tall run `#393B40` bounded by `#4E5056`. |
| UI-DLG-006 | SHOULD | Focus: Tab order follows the visual order; a focused field draws a 2 px outer Blue6 ring; the search field in Archipel starts focused, in IntelliJ it starts focused as well but without ring until typing (ring appears on focus in the capture of 02 "Name"). | - |
| UI-DLG-007 | SHOULD | The dialog title is in the OS bar; for an IDE-drawn title Archipel centers the title text and a close glyph at the right. | - |

## Measured values

| Value | IntelliJ | Archipel | Diff |
|---|---|---|---|
| Settings size | 992x710 | 990x728 | -2, +18 |
| Button size | 72x28 (x 1041..1112) | 70x28 (OK x 965..1034) | -2 |
| Button gap | 12 | 12 | 0 |
| OK border | none | 1 px `#4E5157` ring around the primary | extra |
| Button radius | about 3..4 | about 4 | 0 |
| Left panel width | 235 (+divider) | 236 | 0 |
| Tree row | 25 | 25 | 0 |
| Tree selection | full width, square | inset 8, radius 4 | differs |
| Search field | 28 px, `#4E5056` border, no ring when idle | 28 px, 2 px blue ring (focused) | focus state |
| Combo height | 28 (y 210..237) | 24 (y 168..191) | -4 |
| Combo fill | `#393B40` | `#2B2D30` (outline only) | fill |
| Section header rule | `#393B40` | `#43454A`-ish line | color |
| Footer | rule `#1E1F22` + strip `#393B40`, help button | 1 px `#43454A` rule, no help button | strip, help missing |
| Title | OS bar | self drawn centered title and X | platform |
| Extra rows | Theme, scheme + gear, Zoom... | adds "UI look" combo, no "Use custom font" combo row | content differs |

`ic: platform/platform-resources/src/themes/expUI/expUI_dark.theme.json` (`Button`, `ComboBox`, `TextField`, `CheckBox`: borderInsets 4, icon 24).

## After the ui-parity pass (`u08`)

| Value | IntelliJ | Archipel now | Remaining diff |
|---|---|---|---|
| Buttons | 72x28, OK primary no border, gap 12 | kit `Button` 72x28, gap 12 | 0 |
| Combo | 28 px, filled, bordered | kit `ComboBox` | 0 |
| Tree selection | full-bleed, square | full-bleed, square | 0 |
| Help button | present | present (kit action button, opens the help index) | round 22 px style differs |
| Footer strip | rule + Gray3 strip | rule only | strip missing |

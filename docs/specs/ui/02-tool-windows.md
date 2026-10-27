# UI-TW: tool window stripes, headers and trees

Reference: `04-editor-project-tree.png`, `20-problems.png`, `19-project-context-menu.png` (bottom Run window).
Archipel: `new-ui-dark-u04-*.png`, `new-ui-dark-u20-problems.png`. Window pixels. Responsible code:
`crates/archipel-ide-ui` (tool window shell, project panel, tree rows), kit `ToolWindowStripe`/`ListRow` parts.

## Requirements

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| UI-TW-001 | MUST | Left and right stripes are 40 px wide with a 1 px `#1E1F22` border towards the content (left: x=40, right: x=W-41); stripe background Gray2 `#2B2D30`. | Row y=300: 40 px `#2B2D30`, 1 px `#1E1F22`. Same mirrored on the right. |
| UI-TW-002 | MUST | A stripe button is a **30x30** rounded square (arc 12 = radius 6, `Button.ToolWindow.arc`) centered in a 40x40 cell (icon padding 5; `StripeToolbar.Button.size` 40x40, icon 20). Visible button inset: 5 px from the stripe edges. | Active button fill `#3574F0`, run from x=5 to x=34 and y=T to T+29 (30 px each). |
| UI-TW-003 | MUST | Buttons are laid out on a **40 px pitch**; the first top button starts 5 px below the toolbar border (y=46). Top group: Project, Commit, (gap 10) Structure, "..." (more). Bottom group, bottom aligned: Build, Services/Run, Terminal, Problems, Git, last button 5 px above the status bar border. | Reference centers: top 61, 101, 151, 192; bottom 770, 810, 850, 890, 930 (window y). Pitch 40 +-1. |
| UI-TW-004 | MUST | Active tool window button: Blue6 `#3574F0` fill with the glyph recolored to white-ish (`ToolWindow.Button.selectedForeground` Gray14); other buttons: glyph Gray9 `#9DA0A8`, hover fill `#FFFFFF16`, pressed `#FFFFFF26`. A selected-but-unfocused button (popup open over it) is a neutral gray fill (`#494A4D` in 05). | Sample the button interior away from the glyph. |
| UI-TW-005 | MUST | Tool window header is 40 px (JBUI default 41 including the bottom 1 px border): title in semibold 13 px at x = panel left + 12, followed by a chevron when the title is a view selector (Project), actions right aligned with 14 px right padding, glyph pitch about 26 px. Reference actions of Project: New (+), Locate, Expand all, Collapse all, kebab, Hide (dash). | Title text bbox starts at x=53; right-most action glyph ends 14 px from the panel edge. |
| UI-TW-006 | MUST | Default Project tool window width: 460 px at a 1600 px window (28.8 %). | Panel spans x=41..500. |
| UI-TW-007 | MUST | Tree rows are 24 px (`Tree.rowHeight`); indentation per level is 19 px; selection fill is inset 12 px from the panel edges (`Tree.border` 4,12,4,12), radius 4, color Blue2 `#2E436E` when the tree has focus; inactive (unfocused) selection is Gray4 `#43454A` (see `07`). Chevron 8x8 at 4 px before the icon. | Selection run: x 53..488 for a panel 41..500, 24 px tall. Chevron x of levels: 57, 76, 95, 114, 133. |
| UI-TW-008 | MUST | The tree root row shows a chevron and the project name in bold followed by the path in gray (`Gray7`). | Root chevron present at x=57. |
| UI-TW-009 | MUST | File-status row tints are full-width (panel edge to edge, not inset): excluded/ignored directories Yellow1 `#3D3223` with orange (`#D69A6B`-like) text; test source roots Green2 `#273828`; source/test root folder icons with root marker (`main`, `test` bold). | Row y=130..153 (png 150..173): `#3D3223` from x=41 to x=500; test subtree rows `#273828`. |
| UI-TW-010 | SHOULD | Bottom tool windows: header 40 px; selected header tab underlined by a **3 px** Blue6 bar (4 px with rounded ends in the editor tab strip, see `03`), underline spans the tab text plus 12 px padding each side (File 1: 59 px). Header tabs: semibold title ("Problems"), then regular tabs (`File 1`, `Project Errors`, ...) with a count in Gray7. | Underline rows y=666..668 (3 px), x from 131 to 189 for the tab "File 1" (window pixels of a bottom window whose header starts at y=628). |
| UI-TW-011 | SHOULD | No visible splitter line between editor and bottom tool window (only the color change Gray1 / Gray2); no 12 px "island" gaps in the New UI look. | Column x=520 across the boundary: `#1E1F22` run then `#2B2D30` run with no third color. |
| UI-TW-012 | SHOULD | Vertical toolbar of a bottom tool window (eye, bulb, list in Problems; refresh, stop... in Run) sits left of the content, no divider, icons 16 px at a pitch of about 25 px, centered 19 px right of the panel edge. | Image 20 (reference): icon centers x=60, y=687, 712, 738; Archipel: x=338-321=17 right of its panel edge, y=673, 698, 722. |

## Measured values

| Value | IntelliJ | Archipel | Diff |
|---|---|---|---|
| Stripe width (left) | 40 + 1 border (m) | 40 + 1 border | 0 |
| Stripe width (right) | 39..40 + 1 border (m) | 40 + 1 border | 0 |
| Stripe button size | 30x30 (m; s: 40 cell, padding 5) | 32x32 (x 4..35) | +2 px |
| Stripe button radius | about 6 (m; s: arc 12) | about 7 | +1 |
| Stripe button pitch | 40 (m) | 38 | -2 px |
| First top button top | y=46 | y=46 | 0 |
| Top group content | Project, Commit, Structure, "..." | Project, Commit, Structure | missing "more" |
| Bottom group content | Build, Services, Terminal, Problems, Git (5) | Find, Debug, Build, Terminal, Run (check), Problems, Git (7) | 2 extra |
| Right stripe content | Notifications, AI Chat, Gradle (3) | 2 buttons (a package icon and a sparkle) | different set |
| Tool window width | 460 px at 1600 (28.8 %) | 278 px at 1440 (19.3 %) | -9.5 points; needs live check of the persisted layout |
| Header height | 40 + 1 border (m, s: `ToolWindow.Header.height` default 41) | title row center at y=61, same as reference | 0 |
| Header title | semibold 13 at x=53 + chevron | semibold 13 at x=53, no chevron | missing chevron |
| Header actions | + locate expand collapse kebab hide (6 glyphs, 16 px) | "..." box button (27x24) and a dash | 4 missing, extra bordered box |
| Tree row height | 24 (m, s) | 24 | 0 |
| Tree indent per level | 19 (m) | 19 | 0 |
| Selection inset | 12 px both sides (m, s) | 12 px both sides (x 53..306 in 41..318) | 0 |
| Selection radius | 4 | 4 | 0 |
| Root row chevron | yes (x=57) | no | missing |
| Exclude / test row tints | `#3D3223` / `#273828` full row | none visible (test folder row untinted) | missing |
| Row icon/text offset (root) | icon x=75, name x=93 | icon x=75, name x=93 | 0 |
| Bottom window header | 40 px, underline 3 px (`#3474F0`) | about 40 px, underline 2 px (`#3574F0`) | -1 px |
| Underline of "File 1" tab | 59 px wide | 48 px wide | -11 (padding) |
| Bottom window origin | flush with editor, no gap | flush (separator `#1E1F22` row above) | 0 |

## Notes

- `ic: platform/util/ui/src/com/intellij/util/ui/JBUI.java` (`defaultHeaderHeight` 41, stripe button 40x40, icon 20, padding 5, arc 12 / compact 8),
  `expUI_dark.theme.json` (`ToolWindow.Button`, `Tree.rowHeight` 24, `Tree.border`, `List.rowHeight`).
- Stripe button tooltip shows the tool window name and shortcut after the tooltip delay (`00`).
- Differences caused by the default Islands look (rounded island panels with 12 px gaps, `Code/Agents` strip) are listed in `gaps.md`, not here.

## After the ui-parity pass (`u04`, `u20`)

| Value | IntelliJ | Archipel now | Remaining diff |
|---|---|---|---|
| Stripe button | 30x30, radius 6 | 30x30, radius 6, icon 20 | 0 |
| Stripe pitch | 40 | 40 (centers 59.5, 99.5, 139.5; bottom 608.5, 648, 688.5, ...) | 0 |
| First button top | y=46 | y=45 | 1 px (no 1 px toolbar border row) |
| Project width | 28.8 % | 28.8 % (415 px at 1440) | 0 |
| Excluded / test row tint | `#3D3223` / `#273828` full width | theme `FileColor.Yellow` / `Green` (banner colors in the hand-measured Islands themes), full width | Islands themes use their own shades |
| Root chevron | present | present | 0 |
| Header actions | + locate expand collapse kebab hide | kebab (kit icon button, popup) and hide | + locate, expand all, collapse all missing (no commands yet) |
| Title chevron | present for view selector | absent | needs a view selector |
| Bottom tab underline | 3 px, tab text + 12 px each side | 3 px (kit token), 12 px padding | 0 |

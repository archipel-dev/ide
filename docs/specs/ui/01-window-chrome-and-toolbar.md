# UI-CHR: window chrome and main toolbar

Reference: `java-gradle/04-editor-project-tree.png` (also 05..10, 19, 20 share the toolbar). Archipel:
`archipel-ide/ui/new-ui-dark-u04-editor-project-tree.png`. Positions in window pixels (see `00`).
Responsible code: `crates/archipel-ide-ui` (toolbar/title area, `theme.rs`), kit parts `archipel-kit` toolbar and
icon button components.

## Requirements

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| UI-CHR-001 | MUST | The main toolbar is 40 px high, bottom border 1 px in the border color (Gray1 `#1E1F22`); the tool window stripes and tool windows start directly below the border. | Column x=700: run of 40 px `#2B2D30` (y 0..39), then 1 px `#1E1F22` at y=40. |
| UI-CHR-002 | MUST | Toolbar background is Gray2 `#2B2D30`; the "project colors in main toolbar" tint is a faint horizontal glow over the left 300 px that fades out (reference `#32313A` at x=300, `#2B2D30` right of x=330). | Pixel (700, 20) = `#2B2D30`; pixel (300, 20) within 8 levels of `#32313A` when the setting is on (default on). |
| UI-CHR-003 | MUST | Toolbar buttons are 30x30 hit areas (`MainToolbar.Button.size`) with 16 px glyphs, laid out on a **40 px pitch** on the right cluster (centers 1460, 1501, 1541, 1581 in the reference = pitch 40). The rightmost glyph ends 10 px before the window edge (reference: glyph x 1572..1590 of a 1599 wide window area). | Glyph centers of the last four right icons differ by 40 +-1 px; right margin 9..11 px. |
| UI-CHR-004 | MUST | Glyph visual sizes (bounding boxes) in the right cluster: run triangle 13x14, debug 16x14, kebab 4x14, search 17x17, settings 19x19; hamburger 16x16 at x=12. | Bounding box of each glyph within +-1 px. |
| UI-CHR-005 | MUST | Left to right content: hamburger; **project widget** (colored initials badge, project name, chevron) starting at x=42; **VCS widget** (branch icon, branch name, chevron) at x=201; flexible space; right: run configuration widget ("Current File" + chevron), Run, Debug, kebab (more run actions); then AI assistant, collaboration, Search Everywhere (magnifier), Settings gear with update dot. Nothing else (no segmented control, no title text) in the toolbar. | Scan segments of the toolbar: left group starts x=13; the right cluster contains exactly 4 icon buttons after the kebab. |
| UI-CHR-006 | MUST | The window title ("java-gradle - App.java [java-gradle.main]") belongs to the OS title bar (20 px in openbox), not to the toolbar. Where Archipel draws its own title bar it is a separate 20..30 px strip or the title sits in the toolbar's free middle; window buttons are 40x40 (`TitlePane.Button.preferredSize`, compact 40x30). | Window buttons pitch 40 px. |
| UI-CHR-007 | SHOULD | The project widget is `ProjectWidget`: 20x20 rounded badge with the initials (default color from a stable hash of the project name), name in semibold 13 px, chevron-down 8 px; widget hover fill `#FFFFFF1A` (`MainToolbar.Dropdown.transparentHoverBackground`), pressed Gray3; dropdown hover arc 12; maximum width 350. | Hover over the widget: fill within 4 levels of `#FFFFFF1A` over `#2B2D30`. |
| UI-CHR-008 | SHOULD | When the main menu is expanded from the hamburger the toolbar's left part becomes a menu bar `File Edit View Navigate Code Refactor Build Run Tools Git Window Help` (mnemonic letter underlined); the open menu title gets a Gray `#404144` rounded fill (reference x 5..39 for "File"). | Open File: fill `#404144` spans 35 px, y 0..39 of the toolbar. |
| UI-CHR-009 | LATER | Compact mode (`MainToolbar` 30 px) and presentation-mode toolbar. | - |

## Measured values

| Value | IntelliJ | Archipel (New UI Dark / new_ui) | Diff |
|---|---|---|---|
| Toolbar height | 40 px + 1 px border (m, s: theme `MainToolbar`) | 40 px + 1 px border | 0 |
| Toolbar background | `#2B2D30` (m, Gray2) | `#2B2D30` | 0 |
| Border color below toolbar | `#1E1F22` | `#1E1F22` | 0 |
| Hamburger glyph | 16x16 at x=12 | 12x12 at x=11 | -4 px |
| Search glyph | 17x17 | 14x13 | -3..-4 px |
| Run triangle | 13x14 | 9x12 | -4 px wide |
| Debug glyph | 16x14 | 12x12 | -4 px |
| Right icon pitch | 40 px (m) | 24..29 px: centers 1210, 1235, 1259, 1288 | -11..-16 px |
| Kebab (more run actions) | present, 4x14 | missing | missing |
| AI / collaboration / settings gear | present | AI only in the right tool stripe; no collaboration; no gear in the toolbar | missing |
| Extra items | none | "Code / Agents" segmented switch at x=280..406; hammer (Build) button between Run and Debug; centered title text (x 592..845) | extra |
| Window buttons | OS frame (not part of the IDE image) | self drawn, centers 1330, 1372, 1414 = pitch 42, 10..12 px glyphs | 2 px; platform specific |
| Run widget text "Current File" | starts x=1188, 48 px wide, chevron 10 px | starts x=1099, 47 px wide, chevron 8 px | position depends on cluster |
| Project colors glow | yes, x 40..330 | yes (visible tint over x 40..250) | similar |
| Window title | in WM title bar (20 px, openbox) | in toolbar | structural |

## Notes

- `MainToolbar.Button.size` 30x30 and glyph size come from `ic: platform/util/ui/src/com/intellij/util/ui/JBUI.java`
  (`experimentalToolbarButtonSize`), `MainToolbar` colors from `expUI_dark.theme.json`.
- IntelliJ draws the project/VCS/run widgets as split dropdowns: insets 5,10,5,6 (dropdown) and 5,5,5,3 (split) from `JBUI.CurrentTheme.MainToolbar`.
- Open question: whether Archipel keeps its own title strip on Linux; the reference with a decorated frame shows the title
  only in the OS bar. Decide per platform, and keep the toolbar 40 px either way.

## After the ui-parity pass (`u04`)

| Value | IntelliJ | Archipel now | Remaining diff |
|---|---|---|---|
| Right icon pitch | 40 | 40 (centers 1168, 1208, 1248, 1288) | 0 |
| Button size | 30x30 | 30x30 kit `ActionButton::large` | 0 |
| Window buttons pitch | OS frame | 40 | 0 (self drawn) |
| Run / debug / search glyph | 13x14 / 16x14 / 17x17 | 11 / 16 / 16 wide | run glyph about 2 px narrower |
| Kebab, collaboration, gear | present | missing | missing (no actions behind them) |
| "Code / Agents" switch | absent | present | product feature, kept |
| Hammer between Run and Debug | absent | present | kept |

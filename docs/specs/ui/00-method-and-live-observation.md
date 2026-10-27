# UI look and feel: method, conventions and what a screenshot cannot tell

Prefix `UI-`. This set turns "the IDE does not feel 1:1 with IntelliJ" into measured requirements.
Every number in `01`..`07` is either **m** (measured from a reference screenshot), **s** (a value
the IntelliJ resources state, cited as `ic:`), or both. Archipel numbers are measured from our own
captures. Nothing here is guessed; where a value could not be measured it says so and is listed in
the last section.

## Reference

| Item | Value |
|---|---|
| Product | IntelliJ IDEA Community 2025.2.6.3 (IC-252.28539.97), New UI, theme **Dark** (`New UI Dark`), editor scheme "Dark Theme default" |
| Capture | Xvfb 1600x1000x24, openbox; IDE window 1600x980 below a 20 px openbox title bar, 1 px WM border at the left/right. Files: `docs/parity/evidence/reference/intellij-community/java-gradle/*.png`, notes in `NOTES.md` there |
| Scale | 100 % (Settings > Appearance > Zoom `100%`), UI font Inter 13, editor font JetBrains Mono |
| Window origin | Reference pixel (x, y) is window pixel (x-1, y-20). All positions in `01`..`07` are given in **window pixels**, size values are unaffected |

IntelliJ values that are stated by resources, with the files they come from (paths in the
intellij-community repository, read-only; the values are restated in our own words):

- `ic: platform/platform-resources/src/themes/expUI/expUI_dark.theme.json` - palette (Gray1..14, Blue1..13,
  Green, Yellow, Red, Orange, Purple, Teal), and the `ui` defaults: tree and list row height, tab underline,
  popup borders, button sizes, tool window and status bar colors.
- `ic: platform/platform-resources/src/themes/expUI/expUI_darkScheme.xml` - editor scheme "Dark" (caret row,
  line numbers, matched brace, search results, syntax colors).
- `ic: platform/util/ui/src/com/intellij/util/ui/JBUI.java` - layout constants (tool window header height,
  stripe button size, status bar widget insets, main toolbar button size).
- `ic: platform/util/resources/misc/registry.properties` - registry defaults (timings, caret width).
- `ic: platform/analysis-impl/src/com/intellij/codeInsight/CodeInsightSettings.java`,
  `ic: platform/ide-core-impl/src/com/intellij/openapi/editor/ex/EditorSettingsExternalizable.java` - popup delays and caret blink.

## Archipel captures

Archipel is captured with `cargo run -p xtask -- scenes` (Xvfb 1600x1000 screen). Facts about the capture:

1. **Size.** The window is created at 1440x900 logical pixels (`crates/archipel-ide-app/src/main.rs`,
   `with_inner_size([1440.0, 900.0])`), there is no size option in `xtask scenes`. All Archipel captures are
   therefore **1440x900, pixels-per-point 1**, with no WM title bar (Archipel draws its own title inside the
   toolbar). Fixed-size items (heights, paddings, row heights) are directly comparable; widths that depend on the
   window size (tool window width, popup position) are compared as fractions or noted.
2. **Which look.** The kit has three looks (`islands`, `new_ui`, `classic`) and 12 themes
   (`crates/archipel-ide-settings/src/intellij.rs`). The settings default is theme `Islands Dark` with look
   `islands`. The IntelliJ reference is `New UI Dark` with look `new_ui`. This spec compares **New UI Dark / new_ui**
   (the like-for-like counterpart) and records the default Islands look separately where it differs.
3. **Scenes.** `docs/parity/scenes/u*.json` are copies of the existing parity scenes that first apply
   `appearance.theme = New UI Dark`, `appearance.uiLook = new_ui`, dismiss the toolchain banner and then replay the
   reference steps (`u04` editor/tree, `u05` Search Everywhere, `u06` Find Action, `u07` main menu, `u08` Settings >
   Appearance, `u10` Find in Files, `u12` Commit, `u19` project context menu, `u20` Problems, `u28` Settings > Keymap, `u01` welcome).
   Run: `cargo run -p xtask -- scenes --only u04,u05,...`.
4. **Settings leak.** `xtask scenes` runs the IDE against the real `~/.config/archipel/settings.toml`, so a scene that
   applies settings changes the next run (and parallel runs). Captures that must show the true default need an empty
   `HOME`/`XDG_CONFIG_HOME` (the default captures here were taken that way, see `docs/parity/evidence/archipel-ide/ui/`).
5. **Images.** `docs/parity/evidence/archipel-ide/ui/new-ui-dark-*.png` (New UI Dark) and `default-islands-dark-*.png`
   (settings default), 256 colors, about 0.8 MB. Reference images are the files named above; no icon or artwork is
   extracted from them.

## How values were measured

Pixel scans with Python/PIL over the PNGs: runs of equal color along a row or column give edges and sizes, bounding
boxes of non-background pixels give glyph and icon extents, dominant colors give fills. Text is LCD-antialiased in the
reference, so text colors are taken from the theme/scheme (stated values), not from the screenshot; fills, borders and
lines are exact. A 1 px difference in a reference color against the theme file (for example selection `#2D426D` against
Blue2 `#2E436E`) is a capture/rounding artifact; **the theme file value is normative**. Acceptance criteria are phrased as
probes: "scan column x = N, expect run of K px of color C"; tolerance is +-1 px for sizes and +-2 per color channel.

## Priorities

`MUST` = visible in the first minute of use or breaks the 1:1 feel; `SHOULD` = visible on inspection; `LATER` = needs
infrastructure we do not have (IDs are kept so work can be tracked).

## What cannot be measured from static screenshots

| Item | What is known | Source / value | Status |
|---|---|---|---|
| Tooltip show delay | initial delay 1200 ms; re-show 500 ms; initial re-show 500 ms; dismiss 4000 ms; dead zone 300 px; fade/animation cycle 150 ms; editor highlighter tooltips (error under caret) 150 ms | `ic: platform/util/resources/misc/registry.properties` (`ide.tooltip.*`) | value known |
| Auto sub-menu delay | a sub-popup opens 500 ms after hover; re-opening a closed popup is blocked for 200 ms | `ide.popup.auto.delay`, `ide.popup.hide.show.threshold` | value known |
| Component show/hide fade | show: delay 0, 130 ms; hide: delay 140 ms, 150 ms (hover-revealed controls) | `ide.animation.*` | value known |
| Caret blink | blink period 500 ms (range 10..1500), blinking on by default; caret width 2 px | `EditorSettingsExternalizable.BLINKING_RANGE`, `editor.caret.width=2` | value known |
| Smooth caret movement | animation 120 ms when enabled (off by default in the IDE) | `editor.smooth.caret.duration` | value known |
| Smooth scrolling | mouse wheel interpolation delay between 60 and 140 ms, scrollbar interpolation 30 ms, pixel-perfect scrolling; wheel scrolls 5 lines per notch | `idea.true.smooth.scrolling.*`, `ide.windowSystem.vScrollChars` | value known; the easing curve needs live observation |
| Drag-select edge scrolling | one step per 100 ms | `editor.scrolling.animation.interval.ms` | value known |
| Auto-popup parameter info | 1000 ms (range 0..5000) | `CodeInsightSettings.PARAMETER_INFO_DELAY_RANGE` | value known |
| Auto quick documentation | off by default; delay 500 ms when on | `CodeInsightSettings.JAVADOC_INFO_DELAY_RANGE` | value known |
| Autopopup completion | on by default; delay 0 (starts when typing) | `CodeInsightSettings.AUTO_POPUP_COMPLETION_LOOKUP`; delay value needs live observation | partly known |
| Hover, pressed, focus states | colors known from the theme: hover Gray3 `#393B40` (lists), ActionButton hover `#FFFFFF16`, pressed `#FFFFFF26`, status bar widget pressed Gray4 `#43454A`, focus ring Blue6 `#3574F0` (2 px outer, see `07`) | `expUI_dark.theme.json` `ActionButton`, `*`, `StatusBar` | colors known; transition timing needs live observation |
| Tool window open/close animation | the New UI animates tool window show/hide (`ide.experimental.ui.animations=true`); duration not in the registry | - | needs live observation |
| Popup open animation, menu fade | none in the registry; heavyweight popups appear instantly in the capture | - | needs live observation |
| Scrollbar thickness and fade | overlay scrollbars fade in/out; thickness at rest and hovered not in the theme | - | needs live observation |
| Cursor shapes (hand on links, I-beam, resize on splitters) | not visible in screenshots | - | needs live observation |
| Mouse-down repeat, drag thresholds, double-click interval | OS settings | - | needs live observation |
| Popup corner radius with real transparency | the Xvfb captures show square popups (heavyweight popup, no per-pixel alpha); with a compositor IntelliJ rounds popups; the arc value is not in the theme file | - | needs live observation |
| Window shadow, resize border, title bar when decorated by the IDE | the capture uses the WM frame | - | needs live observation |
| HiDPI rendering at 125/150/200 % | only 100 % captured | `JBUIScale` rounds each dimension | needs live observation |
| Text rendering (LCD vs grayscale, hinting) | reference is LCD-antialiased | - | decision, not measurement |

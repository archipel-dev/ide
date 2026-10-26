# Themes: Islands Dark / Islands Light

Captures in `archipel-ide/themes/` (Linux, Xvfb 1600×1000, `ARCHIPEL_RENDERER=glow`).

## Steps

1. Put `"appearance.theme" = "Islands Light"` under `[settings]` in
   `$ARCHIPEL_CONFIG_DIR/settings.toml` (or pick it in Settings › Appearance › Theme), then
   `cargo run -p archipel-ide-app -- . crates/archipel-ide-settings/src/lib.rs`.
2. Alt+F12, then Double Shift and type `setting` → `islands-light-workbench.png`: toolbar,
   stripes, islands, project tree, editor (scheme "Light"), terminal and Search
   Everywhere all use the Islands Light tokens.
3. Ctrl+Alt+S → `islands-light-settings.png`.
4. Theme › Islands Dark, Apply → `islands-dark-after-live-switch.png`: the whole window
   switches on the next frame without a restart. `settings.toml` drops the override
   because Islands Dark is the default.

## Project color in the main toolbar

Reference: `reference/android-studio/04-editor-project-tree.png`. Archipel IDE:
`archipel-ide/themes/toolbar-gradient-dark.png` (project `my-application`, whose badge hashes to
teal like the reference's "MA" tile), `toolbar-gradient-light.png` (Islands Light) and
`toolbar-gradient-compare.png` (top: reference toolbar band y 18–82, bottom: Archipel IDE y 0–64,
both 700 px wide, scaled 2×).

Steps: `cargo run -p archipel-ide-app -- <dir named my-application> <file>` under Xvfb 1600×1000;
turn it off with `"appearance.projectColors" = false` (Settings › Appearance › "Use project
colors in main toolbar"), which leaves the toolbar at the plain frame color (38,40,44).

Pixel samples (the reference's toolbar starts at y 19 below the window manager's title bar,
Archipel IDE's at y 0):

| x | reference (y 22) | Archipel IDE (y 10) | Archipel IDE (y 38) |
|---:|---|---|---|
| 5 | 38,40,44 | 39,45,48 | 38,44,47 |
| 20 | 41,53,56 | 41,58,59 | 41,56,57 |
| 45 | 44,70,72 | 46,80,78 | 44,75,73 |
| 150 | 46,75,75 | 45,77,76 | 45,74,72 |
| 300 | 44,66,68 | 43,65,66 | 42,63,63 |
| 450 | 41,53,56 | 40,53,55 | 41,53,55 |
| 550 | 40,45,49 | 39,45,48 | 39,45,49 |
| 650 | 38,40,44 | 38,40,44 | 38,40,44 |

The glow (`crates/archipel-ide-ui/src/toolbar_gradient.rs`) is a vertex-colored mesh on the
background layer, below the toolbar widgets and the islands: it rises from the left edge
to the badge, stays flat to 30 px past it, fades linearly to neutral 570 px right of the
badge centre, and loses 15 % towards the toolbar's bottom edge before fading out in the
island gaps. Dark themes blend the badge color (lightened 12 % towards white) at 32 %,
light themes at 16 %. Differences: Archipel IDE's badge sits ~12 px further left (no VCS
widget offset), so the rise is slightly steeper; the light tint has no reference to check
against.

## How it works

Every color is a token (`crates/archipel-ide-ui/src/theme_engine.rs`) naming, per theme,
`ui:` keys and `palette:` entries in `docs/spec/data/themes.json` and `color:` / `attr:`
entries of the theme's editor scheme in `docs/spec/data/color-schemes.json` (falling back
along the scheme's lineage). A test asserts that resolving Islands Dark from the data
reproduces the previous hand-written values exactly, so the dark look is unchanged.

## Gaps

- There are no Islands Light reference screenshots yet (the PR #40 references are dark), so
  light parity is checked against the data, not against pixels.
- File-type icon accents (Rust, TOML, Markdown, …) and the window-close hover red are fixed
  colors in both themes.
- Only Islands Dark and Islands Light are offered as themes. "Sync with OS" is missing.

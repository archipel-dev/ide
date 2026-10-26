# Settings dialog (scene 08)

Reference: `reference/intellij-community/java-gradle/08-settings.png` and
`reference/android-studio/08-settings.png` (PR #40). Archipel IDE captures:
`archipel-ide/settings/08-settings.png` … `08d-editor-font-applied.png`.

## Steps (Linux, Xvfb 1600×1000, `ARCHIPEL_RENDERER=glow`)

1. `ARCHIPEL_CONFIG_DIR=<empty dir> cargo run -p archipel-ide-app -- . crates/archipel-ide-settings/src/lib.rs`
2. Ctrl+Alt+S → `08-settings.png`: the dialog opens on Appearance & Behavior › Appearance.
3. Type `font size` into the search field → the tree shrinks to Editor › General / Font and
   Advanced Settings; the first hit is selected and matching options are outlined. Select
   Font, type `18` into Size → `08b-settings-search-modified.png`: Editor and Font turn
   blue (modified), the page shows **Reset**, Apply becomes enabled.
4. Apply, then OK → `08d-editor-font-applied.png`: the editor re-renders at 18 pt with
   proportionally taller rows; the status bar reads "1 setting changed" and
   `<config dir>/settings.toml` contains `"editor.font.size" = 18`.
5. Ctrl+Alt+S, Advanced Settings → `08c-advanced-settings.png`: the 124 entries of
   `docs/spec/data/settings.json`, grouped and sorted like `docs/spec/settings-tree.md`.

## Matches the reference

- Window title "Settings", search field above the tree, tree order (Appearance & Behavior,
  Keymap, Editor, Plugins, Version Control, Build/Execution/Deployment, Languages &
  Frameworks, Tools, Backup and Sync, Advanced Settings) and Appearance children in the
  reference order; bold top-level rows; project-level marker on the right of rows such
  as System Settings, File Colors, Scopes, Required Plugins, Path Variables, Version Control.
- Breadcrumb header (`Appearance & Behavior › Appearance`) with back/forward arrows.
- Appearance page: Theme and Editor color scheme combos, Accessibility (Zoom, custom
  font, screen readers, contrast scrollbars) and UI Options checkboxes with the
  reference labels and defaults.
- OK (accent), Cancel, Apply (disabled until something changes) at the bottom right.

## Still differs

- Options are laid out in one column; the reference uses two columns for UI Options and
  aligns label/field pairs in a grid. Checkboxes use egui's tick style, not the filled
  accent box. No help (?) button, no "Plugins" update badge.
- Search outlines whole matching options instead of highlighting the matched substring.
- Theme and Keymap selections are stored but not applied yet (#8, #2/#14); the Theme
  combo lists Islands Dark/Light (the reference shows the theme names of its build).
- Most pages show "This page has no options in Archipel IDE yet." Group pages without their
  own options list their child pages as links.
- The dialog is modal (it dims and blocks the workbench); #15 asks for non-modal.
- Android Studio's tree (Android-specific pages) is not modelled.

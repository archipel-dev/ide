# Merged Manifest view (AND-014)

Reference: Android Studio scene `a07-manifest.png` (the Merged Manifest tab: merged XML tinted by
origin, with the "Manifest Sources" legend) in `docs/parity/evidence/reference/android-studio/`.

## Setup

`HelloAndroid` (see `android-run.md`) with `implementation("androidx.core:core:1.13.0")` added,
built for real with AGP 8.13.0 (`./gradlew :app:assembleDebug`). AGP writes
`app/build/intermediates/manifest_merge_blame_file/debug/processDebugMainManifest/manifest-merger-blame-debug-report.txt`,
which numbers every merged line and gives a `N-->source` attribution for each. The view parses
that report. It does not re-implement the manifest merger.

| File | Shows |
|---|---|
| `archipel-ide/android/a07-merged-manifest.png` | Find Action › "Show Merged Manifest": merged manifest of `:app (debug)`, lines tinted by source; legend with build.gradle injection (package, versionCode, uses-sdk, debuggable), `core:1.13.0 manifest` (permission, appComponentFactory), `app/src/main/AndroidManifest.xml manifest`, `profileinstaller:1.3.0 manifest` (provider) |
| `archipel-ide/android/a07-merged-manifest-jump.png` | clicking the merged `<activity` line opens `app/src/main/AndroidManifest.xml` at line 4 |

## Behaviour

- `android.manifest.merged {variant?}` loads the report of the app module's active build variant.
  When the app hasn't been built, it says which `assemble<Variant>` task produces the report.
- Lines without attribution are injected by the build. Blank lines and closing tags inherit the
  source of the line before them.
- Hovering a line shows its source. Clicking a line that comes from a project manifest opens that
  file at the attributed line.

## Differences (not claimed)

- Shown as an overlay opened from Find Action / Tools, not as a "Merged Manifest" tab at the bottom
  of the manifest editor. There is no element tree folding and no attribute-level tinting (the
  tint is per line).
- No "Other Manifest Files" list and no merge-error panel. The report must already exist; the view
  does not trigger `process<Variant>Manifest` itself.

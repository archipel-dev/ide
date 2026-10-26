# Android project view (AND-011)

Reference: the "Android ▾" Project tool window in Android Studio scenes `a01-device-manager.png` and
`a02-logcat.png` (app › manifests / kotlin+java / res, Gradle Scripts) in
`docs/parity/evidence/reference/android-studio/`.

Captured on Linux under Xvfb (1600x1000x24) with the `MyApplication` sample. Besides the files
listed in `android-sdk.md`, the sample has a unit test, `values`/`values-night` themes, `mipmap`
density variants, `gradle.properties`, the wrapper properties and `proguard-rules.pro`.

| File | Shows |
|---|---|
| `archipel-ide/android/a01-android-project-view.png` | default view of an Android project: **Android ▾** header; app › manifests › AndroidManifest.xml, kotlin+java › `com.example.myapplication` › MainActivity and `com.example.myapplication (test)` › GreetingTest, res, Gradle Scripts |
| `archipel-ide/android/a01-android-view-res-gradle.png` | res grouped by type with qualifier groups: `ic_launcher.webp (2)`, `ic_launcher.xml (anydpi-v26)`, `themes.xml (2)`; Gradle Scripts annotated (Project: …, Module :app, ProGuard Rules for ":app", Project Properties, Gradle Version, Version Catalog, Project Settings); double-click opens `strings.xml` |
| `archipel-ide/android/a01-android-view-switcher.png` | view switcher popup (Project / Android) |

## Behaviour

- The view is computed from the project tree's file index (no extra filesystem walk) by
  `archipel_ide_android::view::android_view`. It refreshes when files are added, renamed or deleted.
- Source sets: `src/<set>/java` and `src/<set>/kotlin` are merged into one `kotlin+java` group
  (`java` when there is no Kotlin). Packages are compacted to dotted names, non-main sets get a
  `(test)` / `(androidTest)` / `(debug)` suffix, and `.kt`/`.java` extensions are hidden.
- Resources: `res/<type>[-qualifiers]/<file>` is grouped by type, then by file name, with
  qualifiers as suffixes.
- Android projects open in the Android view. `android.projectView.select {"view": "project"}`
  switches back. `android.projectView.tree` returns the tree as JSON for headless clients.

## Differences (not claimed)

- No class/interface icons or Kotlin file icons. Source files use the generic file icon, and
  there is no per-class parsing (a file is shown under its file name).
- No `res (generated)` or `java (generated)` nodes, no "Gradle Scripts" for included builds, no
  `cpp` / `assets` / `aidl` groups, and no context menu (New › Activity, resource wizards).
- Non-Android Gradle modules (plain Java/Kotlin libraries) are not listed in the Android view.
- The selected view is not persisted in the workspace.

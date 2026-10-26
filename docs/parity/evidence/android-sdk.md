# Android SDK detection and SDK page (AND-001, AND-002)

Reference: Android Studio scene `a05-sdk-manager.png` (Settings › Languages & Frameworks ›
Android SDK) in `docs/parity/evidence/reference/android-studio/`.

## Setup

Captured 2026-10-10 on Linux under Xvfb (1600x1000x24, openbox), `ARCHIPEL_RENDERER=glow`,
`ARCHIPEL_NO_UPDATE_CHECK=1`. The SDK was installed with the public command-line tools only:

```bash
# commandlinetools-linux-13114758_latest.zip unpacked to ~/Android/Sdk/cmdline-tools/latest
sdkmanager --sdk_root=$HOME/Android/Sdk "platform-tools" "platforms;android-35" "build-tools;35.0.0"
ANDROID_HOME=$HOME/Android/Sdk archipel-ide <MyApplication> app/src/main/AndroidManifest.xml
```

`MyApplication` mirrors the Studio "Empty Views Activity" template: `settings.gradle.kts` with
`include(":app")`, a `gradle/libs.versions.toml` catalog (`agp = "8.13.0"`), plugins applied as
`alias(libs.plugins.android.application)`, and `app/src/main/AndroidManifest.xml` with a
`.MainActivity` launcher. The SDK page was opened with Find Action (Ctrl+Shift+A) › "SDK Manager"
(also Main Menu › Tools).

| File | Shows |
|---|---|
| `archipel-ide/android/a05-sdk-platforms.png` | SDK Platforms tab: `Android 15.0 ("VanillaIceCream")`, API 35, revision 2, Installed; location `/root/Android/Sdk` "Found via ANDROID_HOME" |
| `archipel-ide/android/a05-sdk-tools.png` | SDK Tools tab: Build-Tools 35.0.0, Platform-Tools 37.0.1, Command-line Tools 19.0 (read from `source.properties`, no `package.xml`) |
| `archipel-ide/android/a05-sdk-invalid-location.png` | Edit › `/tmp/not-an-sdk` › Apply: the location is rejected with an actionable error and the page keeps the previous SDK |

Headless equivalent:

```bash
ANDROID_HOME=~/Android/Sdk cargo run -p archipel-ide-cli -- run android.sdk.detect
env -u ANDROID_HOME cargo run -p archipel-ide-cli -- run android.sdk.detect   # lists every probed location
```

## Matches the reference

- Breadcrumb "Languages & Frameworks › Android SDK", the "Manager for the Android SDK and Tools used
  by the IDE" line, "Android SDK Location" field with **Edit**, pill tabs SDK Platforms / SDK Tools,
  a Name / API Level / Revision / Status table, "Show package details" and OK / Cancel / Apply.
- Platform rows use Android release names derived from the API level, like Studio.
- Show package details expands to individual components (system images, sources, every
  build-tools version) with their `sdkmanager` paths.

## Differences (not claimed)

- No remote repository: only installed packages are listed, so there are no "Not installed" rows,
  no install/uninstall/update and no "SDK Update Sites" tab. Use `sdkmanager`.
- The page is a standalone overlay, not a page inside a full Settings dialog (no settings tree,
  search, or "Optimize disk space").
- The chosen location is kept for the session and written to the project's `local.properties`
  (`sdk.dir`) like Studio's sync does; there is no IDE-wide settings store yet.

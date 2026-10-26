# Run app on device, APK install and Build Variants (AND-007, AND-012, AND-013)

Reference: Android Studio's Run 'app' (device selector + ▶ next to the run configuration, scene
`a01-device-manager.png`) and its Build Variants tool window.

## Setup

Linux under Xvfb (1600x1000x24), `ARCHIPEL_RENDERER=glow`, `ARCHIPEL_NO_UPDATE_CHECK=1`.

`HelloAndroid` is a minimal real Android app: AGP 8.13.0, `compileSdk = 35`, build-tools 35.0.0,
and a Java `MainActivity` declared as the launcher in the manifest. Its `local.properties` points
to an SDK whose `platforms`, `build-tools` and `cmdline-tools` are **symlinks to the real SDK**,
so **Gradle builds the APK for real**. Only `platform-tools/adb` is the simulated adb from
`android-devices.md`, extended to answer `install -r -t` with `Success` and `shell am start`
with the usual `Starting: Intent …` line. There is no KVM here, so no real device or emulator can
receive the APK.

The same `./gradlew :app:assembleDebug` was first run directly. It produced
`app/build/outputs/apk/debug/app-debug.apk` and `output-metadata.json`, which `find_apks` reads.
Maven Central returned 429 for this IP, so the sample resolves Maven Central artifacts through
Google's mirror (`maven-central.storage-download.googleapis.com`).

| File | Shows |
|---|---|
| `archipel-ide/android/run-device-selector.png` | device selector with **Run 'app' on Pixel 8 API 35**; Build Variants stripe button for the Android project |
| `archipel-ide/android/run-app-deployed.png` | Build window: real `Gradle :app:assembleDebug` output (`BUILD SUCCESSFUL`); **Run 'app'** balloon: `Launched com.example.hello/.MainActivity on Pixel 8 API 35`, with the adb install (`Performing Streamed Install` / `Success`) and `am start` lines |
| `archipel-ide/android/build-variants.png` | Build Variants tool window: module `app`, active variant `debug` |

## Behaviour

1. `android.app.run` picks the selected online device and the application module (`:app`
   preferred). It uses the module's active build variant (Build Variants window, default: the first
   debug variant) and starts `./gradlew :app:assemble<Variant>` (or `gradle` without a wrapper) in
   the runner. The build output appears in the Build tool window.
2. When the build exits 0, a background job finds the APK in `build/outputs/apk/<flavors>/<type>/`
   (via `output-metadata.json`, falling back to `*.apk`) and runs `adb -s <serial> install -r -t`.
   It then runs `adb -s <serial> shell am start -n <pkg>/<activity> -a MAIN -c LAUNCHER`, where the
   activity is the manifest's launcher and the package is `applicationId`, else `namespace`.
3. Failures stop the chain with an actionable message: build exit code, missing APK, install
   `Failure [INSTALL_FAILED_…]` with a hint (signature mismatch, minSdk, storage, downgrade, ABI,
   USB install restriction), no launcher activity, or `am start` `Error:` lines.

Other commands: `android.app.deploy` (install + launch without building), `android.app.launch`,
`android.apk.install {path}`, `android.app.status`, `android.variants.list`,
`android.variants.select {module, variant}`.

## Differences (not claimed)

- The toolbar ▶ still runs the generic detected run task. The Android run is reached from the
  device selector or Find Action until the build-system and run-configuration work lands, and
  there is no Run tool window tab for the app process.
- No Apply Changes / Apply Code Changes, debugger attach, or install options (`-g`, `--user`).
  Split APKs from App Bundles are not supported.
- Variants are read statically (`buildTypes`, `productFlavors`, `flavorDimensions`), not from a
  Gradle sync. Variant filters, ABI selection and per-module library variants are pending.
- Install and launch could not be verified on a real device in this container.

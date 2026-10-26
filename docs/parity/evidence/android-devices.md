# Android devices: selector and Device Manager (AND-004, AND-005)

Reference: Android Studio scene `a01-device-manager.png` (Device Manager docked right, "No Devices"
selector left of the run configuration) in `docs/parity/evidence/reference/android-studio/`.

## Setup

Linux under Xvfb (1600x1000x24, openbox), `ARCHIPEL_RENDERER=glow`, `ARCHIPEL_NO_UPDATE_CHECK=1`,
the `MyApplication` sample from `android-sdk.md`. The container has **no KVM**, so no emulator can
boot here. Evidence therefore comes in two kinds:

- **Real tools**: SDK from `sdkmanager` (platform-tools 37.0.1, emulator package) with two AVDs in
  `~/.android/avd` (`Pixel_8_API_35`, `Medium_Tablet_API_34`; `emulator -list-avds` lists the same
  two). The system images are not installed, so starting one makes the real emulator exit with
  its own FATAL message.
- **Simulated devices**: `ANDROID_HOME` points to an SDK whose `platform-tools/adb` is a script
  speaking the `track-devices -l` wire format (4-hex-digit length + `adb devices -l` text) and
  answering `getprop` / `emu avd name`. It reports `emulator-5554` (AVD `Pixel_8_API_35`) and then,
  2 s later, a USB `Pixel 8` (API 36). The same kind of script drives the automated tests.

| File | Kind | Shows |
|---|---|---|
| `archipel-ide/android/a01-device-manager-avds.png` | real | Device Manager on the right stripe: AVDs with name, `Android 15.0 ("VanillaIceCream") \| x86_64`, API, Type and a start button; toolbar selector "No Devices" |
| `archipel-ide/android/a01-emulator-no-acceleration.png` | real | ▶ on Pixel 8 API 35: the emulator exits and its FATAL line is shown in the tool window |
| `archipel-ide/android/a01-device-selector-simulated.png` | simulated | selector "Pixel 8 API 35" with Running devices (emulator, Google Pixel 8) and Available devices (the stopped tablet AVD) |
| `archipel-ide/android/a01-device-manager-simulated.png` | simulated | running AVD with a stop button, physical device row with release name and serial |

## Behaviour

- Opening an Android project (or the Device Manager) starts `adb track-devices -l` from the SDK's
  platform-tools, falling back to `adb` on `PATH`. New online devices are described once with
  `adb -s <serial> shell getprop` (plus `emu avd name` for emulators). If adb is missing, the tool
  window says how to install Platform-Tools. If the server dies, tracking stops with its message
  and Refresh restarts it.
- The selector keeps the chosen serial while it stays connected; otherwise it picks the first
  online device. Unauthorized devices are labelled with the action needed.
- AVDs are read from `ANDROID_AVD_HOME` / `ANDROID_USER_HOME/avd` / `ANDROID_SDK_HOME/.android/avd`
  / `~/.android/avd` (`<name>.ini` + `config.ini`, with `path.rel` fallback for moved homes).
- ▶ runs `emulator -avd <name>`; ■ sends `adb -s <serial> emu kill`. If the emulator exits early,
  its last ERROR/FATAL line is reported.

Commands: `android.devices.list`, `android.devices.track`, `android.devices.select`,
`android.avd.list`, `android.emulator.start`, `android.emulator.stop`,
`toolwindow.activate.deviceManager`.

## Differences (not claimed)

- No "Create Virtual Device" wizard (+), Wi-Fi pairing, Firebase device streaming, device details
  panel or per-row ⋮ menu (wipe data, cold boot, duplicate, delete).
- No embedded "Running Devices" emulator display (scene a03).
- The emulator boot itself could not be verified in this container (no KVM).
- The right tool window layout is not persisted in `.archipel/workspace.json` yet.

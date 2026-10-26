# Logcat (AND-008)

Reference: Android Studio scene `a02-logcat.png` (Logcat docked at the bottom with device chooser,
`package:mine` filter, left toolbar with clear / pause / restart / scroll to end / soft wrap) in
`docs/parity/evidence/reference/android-studio/`.

## Setup

Linux under Xvfb (1600x1000x24), `ARCHIPEL_RENDERER=glow`, `ARCHIPEL_NO_UPDATE_CHECK=1`, the
`MyApplication` sample (`applicationId = "com.example.myapplication"`). No device can run in this
container (no KVM), so these captures use the **simulated** SDK from `android-devices.md`. Its
`adb` script answers `-s <serial> logcat -v threadtime` with a fixed threadtime log, including an
ActivityManager `Start proc 4321:com.example.myapplication/…` line and a crash, and answers
`shell ps -A -o PID,NAME`. The same protocol drives the automated tests in
`archipel-ide-android/src/logcat.rs` and `archipel-ide-core/src/android_logcat.rs`.

| File | Shows |
|---|---|
| `archipel-ide/android/a02-logcat-package-mine.png` | Logcat from the left stripe; device chooser "Pixel 8 API 35 (emulator-5554) API 35"; default `package:mine` shows only `com.example.myapplication` lines; columns time, pid-tid, coloured tag, package, level badge (D/I/W/E colours) and message, sticking to the end |
| `archipel-ide/android/a02-logcat-filter-softwrap.png` | `package:mine level:warn -tag:OpenGL` with soft wrap on |

## Filter language

`key:value` (contains), `key=:value` (exact), `key~:regex`, `-key:…` (exclude), quoted values, and
bare words that match tag, message or package. The keys are `tag`, `message`, `package` / `process`
(`package:mine` means the project's application IDs, including `:remote` subprocesses) and `level`
(that level and above). Positive terms on one key are ORed; different keys and exclusions are
ANDed. **Cc** toggles match case. An invalid regex or level turns the field red with the error as
tooltip, and the last valid filter stays active.

The pid → package map comes from ActivityManager `Start proc` lines straight away and from
`ps -A -o PID,NAME` every 2 s. Earlier lines are re-filtered when a mapping arrives.

Commands: `android.logcat.start`, `android.logcat.filter`, `android.logcat.clear`,
`android.logcat.pause`, `android.logcat.softWrap`, `android.logcat.stop`,
`toolwindow.activate.logcat`.

## Differences (not claimed)

- No year in timestamps (threadtime format), no `age:` / `is:crash` / `is:stacktrace` keys, no `|`
  / `&` / parentheses, no saved filters (☆), no query history or completion.
- No multiple Logcat tabs, split panes, "Logcat formatting options" or column configuration.
- Clear only clears the view, not the device buffer (`logcat -c`).
- Soft wrap renders the last 5,000 matching lines; unwrapped mode is fully virtualized.
- Logcat could not be verified against a real device in this container.

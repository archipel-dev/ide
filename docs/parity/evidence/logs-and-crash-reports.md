# Log file and crash reports

Reference: IntelliJ-based IDEs write `idea.log` to a per-OS log directory and offer
**Help → Show Log in Explorer/Finder/Files**. After an internal error or crash they show
a notification that links to the report.

## Archipel IDE behaviour

| OS | Log directory (`ARCHIPEL_LOG_DIR` overrides) |
|---|---|
| Linux | `$XDG_STATE_HOME/archipel/logs`, default `~/.local/state/archipel/logs` |
| macOS | `~/Library/Logs/Archipel` |
| Windows | `%LOCALAPPDATA%\Archipel\logs` |

- **`archipel.log`** records a session-start line (build, OS, pid), renderer fallbacks,
  failed commands and panics. Each record is one UTC-timestamped line. Above 5 MB the
  file rotates to `archipel.1.log`.
- **Crash reports.** A panic writes `crash-<UTC time>-<pid>.txt` containing the build
  details (as in Help → About), the thread, the panic message and location, a full
  backtrace and the last 50 log lines. The path is also printed to stderr. Missing
  desktop libraries still produce the install hint instead of a report.
- **Next start.** A balloon reads "Archipel IDE closed unexpectedly", with links to
  **Open Report** (`app.openCrashReport`) and **Show Log in Files** (`app.showLog`).
  Dismissing it (`app.dismissCrashReport`) writes the newest report's name to
  `crash-reports.seen`, so the balloon does not come back for that report.
- **Help → Show Log in Files** (`app.showLog`) opens the directory in Explorer, Finder
  or the `xdg-open` handler.

## Reproduction (Linux, Xvfb)

Debug builds can trigger a test panic with `ARCHIPEL_DEBUG_PANIC=1` (ignored in
release builds):

```text
$ export ARCHIPEL_LOG_DIR=/tmp/jt-logs
$ ARCHIPEL_DEBUG_PANIC=1 cargo run -q -p archipel-ide-app
archipel-ide: crash report written to /tmp/jt-logs/crash-20261010145126-16552.txt
thread 'main' (16552) panicked at crates/archipel-ide-app/src/main.rs:160:9:
ARCHIPEL_DEBUG_PANIC requested a test crash
$ head -11 /tmp/jt-logs/crash-*.txt
Archipel IDE crash report
Time: 2026-10-10 14:51:26 UTC
Version: 0.0.2
Build: 4d34dd0594b3 (2026-10-10)
Target: x86_64-unknown-linux-gnu
Profile: debug
Runtime: linux x86_64
Thread: main
Panic: ARCHIPEL_DEBUG_PANIC requested a test crash
Location: crates/archipel-ide-app/src/main.rs:160:9
$ cat /tmp/jt-logs/archipel.log
2026-10-10 14:51:26 INFO  session start: archipel-ide 0.0.2 (4d34dd0594b3 2026-10-10, x86_64-unknown-linux-gnu, debug) on linux x86_64 (pid 16552)
2026-10-10 14:51:26 ERROR panic: ARCHIPEL_DEBUG_PANIC requested a test crash; report /tmp/jt-logs/crash-20261010145126-16552.txt

# next start, driven over the control socket
$ DISPLAY=:99 ARCHIPEL_RENDERER=glow ARCHIPEL_CONTROL_SOCKET=/tmp/jt.sock archipel-ide . README.md &
$ archipel-cli ui state                         # popups.crashReport == true
$ archipel-cli ui screenshot crash-notice.png
$ archipel-cli ui execute app.dismissCrashReport
$ archipel-cli ui state                         # popups.crashReport == false
$ cat /tmp/jt-logs/crash-reports.seen
crash-20261010145126-16552.txt
```

![Crash notice](crash-notice.png)

Tests (`archipel-ide-core`):
- `diagnostics`: per-OS directories and override, UTC timestamps, single-line
  records, report contents, rotation, unseen ordering and dismissal, platform open
  handler.
- `diagnostic_commands`: notice shown until dismissed.

# Debugger (DAP)

Captured 2026-10-10 under Xvfb (1600x1000, `ARCHIPEL_RENDERER=glow`,
`ARCHIPEL_NO_UPDATE_CHECK=1`, debug build), driven through the control socket
(`archipel-cli ui execute …`). Adapter: `lldb-dap-18`. Sample: a 15-line Cargo binary
(`square(n)` called from a loop in `main`) with a breakpoint on line 2.
Reference: `reference/android-studio/23-debugger.png`.

| File | Steps → result |
|---|---|
| `debugger/23-debugger-breakpoint.png` | `debug.start` builds with `cargo build`, launches through lldb-dap and stops at the line breakpoint: execution-line highlight and arrow in the gutter, Debug tool window with status `"dbgsample"@16357: suspended (breakpoint 1.1)`, frames list, Locals (`n = {int} 1`), Globals, Registers and the evaluate field |
| `debugger/23-debugger-step-over.png` | `debug.stepOver` (F8): line 2 → 4, status `suspended (step over)`, frame `main.rs:4` |

Breakpoint and execution-line colours come from the theme engine
(`BREAKPOINT_ATTRIBUTES` / `EXECUTIONPOINT_ATTRIBUTES` of the active colour scheme; the dot
uses the `ERROR` token, the arrow `ACCENT`).

Found while capturing: `cargo_package_name` parsed Cargo.toml with `toml::Value::from_str`
(single value in toml 0.9), so Debug on a Cargo project without a selected configuration
always failed with "cannot tell which binary to debug". Fixed, with a test.

## Remaining differences from the reference

- Java debugging works against real jdtls (see below); the remaining items apply to it as well.
- No Breakpoints dialog (Ctrl+Shift+F8 edits the breakpoint at the caret), no suspend policy, Smart Step Into or Quick Evaluate.
- The tool window has no separate Threads and Variables panes, only one combined view.
- Locals only list the adapter's `Locals` scope; Rust-aware pretty printing is lldb's.

## Java (jdtls + java-debug)

`debug.start` on a Gradle configuration (or the gutter `Debug 'App.main()'` entry, Shift+F9) runs
`vscode.java.buildWorkspace`, `vscode.java.resolveMainClass`, `vscode.java.resolveClasspath` and
`vscode.java.startDebugSession` through jdtls, then connects `archipel-ide-dap` to the returned
localhost port and launches the main class. The `java-debug` toolchain component (Maven Central,
`com.microsoft.java.debug.plugin`) is installed with jdtls and passed in `initializationOptions.bundles`.

Reproducible evidence: `cargo test -p archipel-ide-core --test java_debug_flow` (scripted jdtls and DAP
server: start, breakpoint stop, execution point) and `cargo test -p archipel-ide-dap --test java_tcp`.


### Live Java verification (#146, 2026-10-11)

Real jdtls 1.61.0 + java-debug 0.53.1, installed by `toolchain.setup` into an empty `$HOME`, Gradle
Java sample (`App.square(int)` called from a loop in `main`, breakpoint on line 5), Xvfb 1600x1000.

| File | Steps → result |
|---|---|
| `debugger/146-java-breakpoint.png` | `debug.start` → build, jdtls resolves main class and classpath, java-debug starts a DAP session; stops at the breakpoint (`"Thread [main]"@1: suspended (breakpoint)`), frames `App.square(int)` / `App.main(String[])`, local `n = {int} 1` |
| `debugger/146-java-step-over.png` | Step Over: line 5 → 6, `suspended (step)`, locals now include `result = {int} 1` |
| `debugger/146-java-second-stop.png` | Step Into / Out then Resume: stops at the breakpoint again with the next `n` |
| `debugger/146-java-shift-f9.png` | `debug.stop`, then the key chord Shift+F9 starts a fresh session and stops at the breakpoint |

Found and fixed: the `java-debug` install failed (`…0.53.1.jar.sha256` → HTTP 404) because Maven Central
publishes only `.sha1`/`.md5` for it, so the debugger could never be provisioned. The provider now pins the
SHA-256 of 0.53.1 (computed from a download whose SHA-1 matched Central's `.sha1`). Central rate-limited
(HTTP 429) once during capture; the download was retried with backoff.


## Watches, conditions, logpoints, Evaluate, inline values, Run to Cursor (#19)

Protocol-level evidence (scripted adapter, no I/O): `cargo test -p archipel-ide-dap session::` covers
conditions/logpoints/hit counts in `setBreakpoints`, watch evaluation on each stop (`context: watch`),
Run to Cursor (temporary breakpoint removed at the next stop), Force Step Into (`stepInTargets` →
`stepIn` + `targetId`, plain `stepIn` fallback) and `attach` sessions. Core tests:
`cargo test -p archipel-ide-core debug_commands inline_values java_debug`.

Screenshots (Xvfb 1600x1000, `ARCHIPEL_RENDERER=glow`, debug build, gdb DAP adapter, a small C
program with `square(int)` called from a loop; shell run configuration `./app`):

| File | Steps → result |
|---|---|
| `debugger/19-breakpoint-properties.png` | right-click the line number of a breakpoint → properties popup with Condition `i == 3`, Log message and Hit count |
| `debugger/19-watches-inline-values.png` | stopped at the conditional breakpoint: Watches `total`, `i * 2` evaluated in the Variables pane and `name = value` hints at the end of the lines of the current function |
| `debugger/19-evaluate-dialog.png` | Alt+F8: Evaluate Expression dialog with the result of `total + i` |
| `debugger/19-run-to-cursor.png` | `debug.runToCursor` (caret on line 15, past the loop): execution moves there; the watch `i * 2` shows the out-of-scope error. Alt+F9 is the default chord only in some keymaps (the XWin keymap used here binds another one) |

JUnit: the gutter Debug entry of a Gradle test runs `test --tests … --debug-jvm` and attaches
java-debug to the test JVM. Verified at the unit level only (banner parsing, `attach` request); no
live jdtls run in this session.

### Real mouse and keyboard pass (Xvfb 1280x800, `xdotool` only)

Same sample, no control socket and no command dispatch: double-click `main.c`, hover and click the
line number, right-click for the breakpoint properties, type the condition, click Done, click the
Debug toolbar button, type watches into the add field, double-click a watch to edit it inline,
Alt+F8 for Evaluate. Everything stays inside the 1280x800 window.

| File | What it shows |
|---|---|
| `debugger/19-mouse-01-gutter-hover.png` | hovering a line number previews the breakpoint dot |
| `debugger/19-mouse-02-breakpoint-popup.png` | right-click on the breakpoint: properties popup, condition typed, filled default `Done` button (Escape closes it) |
| `debugger/19-mouse-03-watches-hover.png` | stopped at the conditional breakpoint (`i = 3`), watch row hover highlight with Edit / × |
| `debugger/19-mouse-04-watch-inline-edit.png` | double-click on a watch row edits it inline (Enter commits, Escape cancels) |
| `debugger/19-mouse-05-evaluate-dialog.png` | Alt+F8: the dialog takes keyboard focus, `Evaluate` is the filled default button |
| `debugger/19-mouse-06-evaluate-result.png` | Enter evaluates (`total + i = 8`) and the edited watch shows `total * 2 = 10` |
| `debugger/19-mouse-07-after-escape.png` | Escape closed the dialog |

Found and fixed by this pass: Enter did not commit an inline watch edit (focus was re-requested every
frame), the add-watch field lost focus after the first Enter (unstable widget id), the Evaluate dialog
did not take focus when another field held it, and Escape did not close either popup (the workbench
consumed it first).

The same real-input pass was repeated at 1440x900 (right-click popup: Enter applies the condition,
Escape closes it; Alt+F8 dialog: Enter evaluates, Escape closes; double-click edits a watch):
`debugger/19-mouse-08-1440-evaluate.png`, `debugger/19-mouse-09-1440-watches-after-escape.png`.

Editor splits (#217): with two groups (`main.c` left, `util.c` right, made with the tab menu's Move to
Opposite Group), a right-click on the right group's line number opens the properties popup at that
position (`debugger/19-mouse-10-split-right-popup.png`); inline values and the hover preview are painted per group.

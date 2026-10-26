# UI automation control socket

Reference: IntelliJ-based IDEs are driven in UI tests by an in-process robot
(Remote Robot / Driver) that executes actions, sends keystrokes and captures the
window. Archipel IDE's equivalent is an opt-in local socket, so parity scenes and
smoke tests need no `xdotool` clicking.

## Protocol

Start the IDE with `ARCHIPEL_CONTROL_SOCKET=/path/ctl.sock` (a Unix domain socket on
Linux and macOS). On Windows the transport is a named pipe: a `\\.\pipe\name` value is
used as given, any other value names a pipe by its file name
(`ARCHIPEL_CONTROL_SOCKET=archipel-ide` → `\\.\pipe\archipel-ide`). Each request is one
JSON line and gets one JSON response line (`{"ok":true,"result":…}` or
`{"ok":false,"error":"…"}`):

| Request | Effect |
|---|---|
| `{"op":"execute","command":"editor.openFile","args":{"path":"src/main.rs"}}` | runs any registered command |
| `{"op":"state"}` | project, tabs (modified flag, `diskConflict` for outside changes), active tab, visible tool windows, Project tree selection and expanded directories, focused component, active editor text/caret/selection count, popups (Search Everywhere tab, dialog kind, find bar, completion, notification, any open menu), click targets in window pixels (`targets.firstRunSkip`: centre of the first-run Skip button, `null` when closed), Find in Files query and result count, Problems count for the active file, run task and running flag, status text, background busy flag |
| `{"op":"key","chord":"Ctrl+Shift+A"}` | injects press/text/release events through egui input, i.e. the same shortcut and focus paths as a real keystroke; modifiers `Ctrl`, `Alt`, `Shift`, `Cmd`, and `Primary` (⌘ on macOS, Ctrl elsewhere) |
| `{"op":"key","chord":"DoubleShift"}` | the modifier-only Search Everywhere gesture, injected over four frames |
| `{"op":"type","text":"save"}` | text input into the focused component |
| `{"op":"click","x":18,"y":20,"button":"secondary","count":2}` | pointer move, then press/release per frame (`button` primary/secondary/middle, `count` 1–3) |
| `{"op":"move","x":60,"y":168}` | pointer move, e.g. to open a submenu on hover |
| `{"op":"screenshot","path":"out.png"}` | PNG of the window via egui's viewport screenshot (glow and wgpu; no X11 dependency) |
| `{"op":"wait","until":"idle\|popup\|text","text":"…","timeoutMs":10000}` | blocks until no search/task/update/screenshot is running and input has settled for 3 frames, a popup is visible, or the active editor, terminal screen or status bar contains `text` |

`archipel-cli ui [--socket PATH] <execute|state|key|type|screenshot|wait|raw> …`
wraps the socket. Once a client injects input the window counts as focused, so keys reach
widgets even without a window manager (Xvfb/CI); autosave-on-focus-loss therefore does not
fire under automation. A stale socket file from a crashed instance is replaced; a
live one is refused. The socket file is removed on exit.

Scripted scenes built on this protocol live in `docs/parity/scenes/` (`cargo run -p xtask --
scenes`); see the README there.

## Reproduction (Linux, Xvfb)

```bash
Xvfb :99 -screen 0 1600x1000x24 &
export ARCHIPEL_CONTROL_SOCKET=/tmp/jt.sock
DISPLAY=:99 ARCHIPEL_RENDERER=glow ARCHIPEL_NO_UPDATE_CHECK=1 \
  cargo run -p archipel-ide-app -- . crates/archipel-ide-control/src/lib.rs &
cli() { cargo run -q -p archipel-ide-cli -- ui "$@"; }
cli wait idle
cli key Ctrl+Shift+A && cli wait popup && cli type save && cli wait idle
cli screenshot docs/parity/evidence/control-socket-find-action.png
cli key Escape
cli execute toolwindow.toggle '{"id":"terminal"}' && cli wait idle
cli state   # toolWindows.bottom == "terminal", focus == "terminal"
```

![Find Action opened and filtered over the control socket](control-socket-find-action.png)

Observed on 2026-10-10 (Xvfb, glow): Find Action opened by the injected chord,
the query typed, a 1440×900 PNG written; `key Ctrl+Bogus` and `execute nope`
returned structured errors.

## Gaps

- Pixel comparison against the reference is deliberately not automated; scenes assert state.

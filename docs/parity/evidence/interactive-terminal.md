# Interactive terminal

Open a project, press Alt/Option+F12, and type directly into the terminal. There is
no separate command entry field. Enter submits; Up/Down use shell history; Tab is
sent to the shell; Ctrl+C interrupts. Paste follows the application's bracketed
paste mode. ANSI colors, cursor motion/erase and alternate screen state are parsed.

Drag to select, copy with Cmd+C (macOS) or Ctrl+Shift+C (other platforms), and scroll
with the wheel. On other platforms Ctrl+C also copies when a selection exists;
otherwise it interrupts. Clear sends Ctrl+L; Restart starts a new shell in the
current project. The PTY follows panel dimensions. Editor shortcuts do not consume
terminal Ctrl+R, Ctrl+Z or similar shell controls.

```sh
cargo run -p archipel-ide-app -- .
cargo test -p archipel-ide-terminal
cargo test -p archipel-ide-core terminal_commands_validate
cargo test -p archipel-ide-ui forwards_shell_keys
cargo test -p archipel-ide-ui forward_and_backward_cell_selection
```

Native macOS walkthrough, 2026-10-10: project-relative zsh prompt, direct pasted
`printf` command, green ANSI output, Up/Enter recall, and `sleep 10` interrupted
with Ctrl+C, returning to the prompt. Native screenshot showed the live cursor.

Automated evidence covers real PTY input/resize and Unicode, ANSI foreground color,
cursor query reply, bounded poll work under continuous output, nonblocking queued
input to a blocked child, selection direction and command bounds.

Limits: one session, 2000 scrollback rows, dimensions capped at 200x500, paste capped
at 1 MiB. No mouse reporting, OSC hyperlinks, advanced IME composition or complete
xterm conformance. Windows native execution remains unverified. This is a partial
terminal implementation, not complete parity with IntelliJ's terminal.

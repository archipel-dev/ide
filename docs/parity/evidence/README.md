# Visual evidence

Screenshots of the native shell captured on Linux under Xvfb (software
rendering, OpenGL fallback). They document the current workbench look; they are
not a pixel-parity claim.

Reproduce:

```bash
cargo build --release -p archipel-ide-app
Xvfb :99 -screen 0 1440x900x24 &
DISPLAY=:99 ./target/release/archipel-ide . crates/archipel-ide-ui/src/keys.rs &
DISPLAY=:99 import -window root shot.png   # ImageMagick
```

Interactions (Double Shift, Alt+F12, Alt+0, Shift+F10) were driven with
`xdotool` after `xdotool windowfocus`.

| File | Shows |
|---|---|
| `linux-editor-project-tree.png` | Project tree with VCS colors, editor tabs, highlighted Rust, breadcrumbs/status widgets |
| `linux-search-everywhere.png` | Search Everywhere with ranked actions and files |
| `linux-commit-terminal.png` | Commit tool window (changes/unversioned) and PTY terminal |
| `linux-run-tool-window.png` | Run widget output for `cargo run` of a sample project |

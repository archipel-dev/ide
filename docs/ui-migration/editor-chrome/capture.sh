#!/usr/bin/env bash
# Captures the editor tabs and chrome with real mouse and keyboard input only (xdotool under
# Xvfb): tab hover, pressed and dragging, the completion, quick-fix and quick-documentation
# popups and a banner. The same steps give before and after screenshots.
#
#   docs/ui-migration/editor-chrome/capture.sh <jetthing-app binary> <output prefix>
#
# Needs: Xvfb, xdotool, ImageMagick `import`, rust-analyzer, libxkbcommon-x11.
# Coordinates assume a 1440x900 screen and the Islands Dark theme.
set -euo pipefail
BIN=${1:?binary}; OUT=${2:?output prefix}
DISPLAY_NUM=:77
export DISPLAY=$DISPLAY_NUM JETTHING_RENDERER=glow JETTHING_NO_UPDATE_CHECK=1 LIBGL_ALWAYS_SOFTWARE=1
WORK=$(mktemp -d); export HOME=$WORK/home; mkdir -p "$HOME"
DEMO=$WORK/demo
mkdir -p "$DEMO/src"
cat > "$DEMO/Cargo.toml" <<'TOML'
[package]
name = "demo"
version = "0.1.0"
edition = "2021"
TOML
cat > "$DEMO/src/main.rs" <<'RS'
use std::collections::HashMap;

struct Counter {
    count: usize,
}

impl Counter {
    fn increment(&mut self) {
        self.count += 1;
    }
}

fn main() {
    let mut counter = Counter { count: 0 };
    counter.increment();
    let names: Vec<String> = Vec::new();
    println!("{}", names.len());
}
RS
printf 'pub fn twice(x: i32) -> i32 {\n    x * 2\n}\n' > "$DEMO/src/util.rs"
printf '# Demo\n' > "$DEMO/README.md"
( cd "$DEMO" && git init -q -b master && git add -A && git -c user.name=x -c user.email=x@x.invalid commit -q -m init )

pgrep -f "Xvfb $DISPLAY_NUM" >/dev/null || { Xvfb $DISPLAY_NUM -screen 0 1440x900x24 >/dev/null 2>&1 & sleep 2; }
"$BIN" "$DEMO" >"$WORK/app.log" 2>&1 &
APP=$!
trap 'kill $APP 2>/dev/null || true' EXIT
sleep 15
WID=$(xdotool search --onlyvisible --name demo | head -1)
xdotool windowfocus "$WID" || true   # there is no window manager under Xvfb

move() { xdotool mousemove "$1" "$2"; sleep 0.7; }
click() { move "$1" "$2"; xdotool click 1; sleep 0.9; }
dbl() { move "$1" "$2"; xdotool click --repeat 2 --delay 80 1; sleep 0.9; }
shot() { import -window root "$OUT-$1.png"; }

click 673 411                       # first-run dialog: Skip
click 80 113                        # src chevron
dbl 150 137                         # src/main.rs
dbl 150 161                         # src/util.rs
sleep 4
# Tabs (main.rs is the inactive one on the left): hover, pressed (button held), dragging
# (the tab follows the pointer, an insertion marker shows where it lands).
move 370 61; shot tab-hover
xdotool mousedown 1; sleep 0.5; shot tab-pressed
for x in 390 420 450 480 500; do xdotool mousemove $x 61; sleep 0.15; done; sleep 0.6
shot tab-dragging
xdotool mouseup 1; sleep 1
# The editor features need rust-analyzer: give it time to load the project.
dbl 150 137                         # main.rs (focus its tab whatever the tab order is)
sleep 30
# Quick documentation: rest the mouse on `Counter` in `struct Counter`.
move 456 124; sleep 1; move 462 126; sleep 6
shot quick-doc
move 900 600
# Completion: type `counter.` on a new line inside `main`.
click 800 374                       # end of the println! line
xdotool key End Return
xdotool type --delay 90 'counter.'; sleep 8
shot completion
xdotool key Escape; sleep 0.5
xdotool key shift+Home; xdotool key BackSpace
# Quick fix: a type error under the caret, then the quick-fix popup.
move 900 600
xdotool key ctrl+End
xdotool type --delay 60 'let y: i32 = "a";'; sleep 12
xdotool key Left Left Left; sleep 3
shot quick-fix-bulb
xdotool key alt+Return; sleep 2
shot quick-fix
xdotool key Escape; sleep 0.5
# Banner: the file changes on disk while it has unsaved edits.
echo '// changed outside' >> "$DEMO/src/main.rs"; sleep 6
shot banner
echo "wrote $OUT-*.png"

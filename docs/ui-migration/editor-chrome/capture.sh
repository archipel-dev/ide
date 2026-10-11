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

move() { xdotool mousemove "$1" "$2"; sleep 0.7; }
click() { move "$1" "$2"; xdotool click 1; sleep 0.9; }
dbl() { move "$1" "$2"; xdotool click --repeat 2 --delay 80 1; sleep 0.9; }
shot() { import -window root "$OUT-$1.png"; }

click 673 411                       # first-run dialog: Skip
click 80 113                        # src chevron
dbl 150 137                         # src/main.rs
dbl 150 161                         # src/util.rs
sleep 4
# Tabs: hover, pressed (button held), dragging (tab follows the pointer).
move 520 61; shot tab-hover
xdotool mousedown 1; sleep 0.5; shot tab-pressed
xdotool mousemove 500 61 mousemove 470 61 mousemove 400 61; sleep 0.6
shot tab-dragging
xdotool mouseup 1; sleep 1
# Completion: click into main.rs after `counter.` and type.
click 380 61                        # first tab (main.rs)
click 700 420                       # end of the `counter.increment();` line area
xdotool key End; xdotool type --delay 80 $'\ncounter.'; sleep 6
shot completion
xdotool key Escape; sleep 0.5
# Quick documentation: rest the mouse on `Counter`.
move 480 240; sleep 3
shot quick-doc
# Quick fix: an error under the caret, then the bulb.
xdotool key ctrl+a; xdotool key BackSpace
xdotool type --delay 40 $'fn main() {\n    let x: i32 = "a";\n}\n'; sleep 8
click 600 168
sleep 2
shot quick-fix-bulb
xdotool key alt+Return; sleep 1.5
shot quick-fix
xdotool key Escape; sleep 0.5
# Banner: the file changes on disk while it has unsaved edits.
echo '// changed outside' >> "$DEMO/src/main.rs"; sleep 5
shot banner
echo "wrote $OUT-*.png"

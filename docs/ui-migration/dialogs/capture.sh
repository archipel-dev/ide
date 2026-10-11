#!/usr/bin/env bash
# Dialog screenshots with real mouse input only (xdotool under Xvfb :77, 1440x900).
#   docs/ui-migration/dialogs/capture.sh <jetthing-app binary> <prefix>
set -euo pipefail
BIN=${1:?binary}; OUT=${2:?prefix}
export DISPLAY=:77 JETTHING_RENDERER=glow LIBGL_ALWAYS_SOFTWARE=1 JETTHING_NO_UPDATE_CHECK=1
WORK=$(mktemp -d); export HOME=$WORK/home; mkdir -p "$HOME"
pgrep -f "Xvfb :77" >/dev/null || { Xvfb :77 -screen 0 1440x900x24 >/dev/null 2>&1 & sleep 2; }
"$BIN" >"$WORK/app.log" 2>&1 & APP=$!
trap 'kill $APP 2>/dev/null || true' EXIT
sleep 20; xdotool windowfocus --sync "$(xdotool search --onlyvisible --class jetthing | head -1)"
move() { xdotool mousemove "$1" "$2"; sleep 0.6; }
click() { move "$1" "$2"; xdotool click 1; sleep 1; }
shot() { import -window root "$OUT-$1.png"; }
shot first-run
click "${SKIP_X:-753}" "${SKIP_Y:-577}"
click 319 204; shot new-project
click 100 456; shot new-project-generator
move 700 300; xdotool mousedown 1; sleep 0.5; shot new-project-pressed; xdotool mouseup 1
xdotool key Escape; sleep 1
click 563 204; shot clone
xdotool key Escape; sleep 1

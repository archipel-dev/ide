#!/usr/bin/env bash
# Dialog screenshots with real mouse input only (xdotool under Xvfb :77, 1440x900).
#   docs/ui-migration/dialogs/capture.sh <jetthing-app binary> <prefix> before|after
# `before` uses the pre-migration dialog positions (top-aligned, 20 % from the top);
# `after` the centred ones. Run it once with a binary built from main and once with
# the migrated binary.
set -euo pipefail
BIN=${1:?binary}; OUT=${2:?prefix}; MODE=${3:?before|after}
export DISPLAY=:77 JETTHING_RENDERER=glow LIBGL_ALWAYS_SOFTWARE=1 JETTHING_NO_UPDATE_CHECK=1
WORK=$(mktemp -d); export HOME=$WORK/home; mkdir -p "$HOME"
pgrep -x Xvfb >/dev/null || { Xvfb :77 -screen 0 1440x900x24 >/dev/null 2>&1 & sleep 2; }
if [ "$MODE" = before ]; then
  SKIP="672 411"; NP_RUST="440 587"; NP_NAME="850 231"; NP_CREATE="1006 665"; NP_SMALL_CREATE="686 475"
  CLONE_URL="760 231"
else
  SKIP="753 577"; NP_RUST="420 445"; NP_NAME="846 155"; NP_CREATE="1060 801"; NP_SMALL_CREATE="730 441"
  CLONE_URL="720 330"
fi
DEMO=$WORK/demo; mkdir -p "$DEMO/src"; echo '# demo' > "$DEMO/README.md"; echo 'a' > "$DEMO/notes.txt"
win() { xdotool search --onlyvisible --class jetthing | head -1; }
move() { xdotool mousemove "$1" "$2"; sleep 0.6; }
click() { move "$1" "$2"; xdotool click 1; sleep 1; }
rclick() { move "$1" "$2"; xdotool click 3; sleep 1; }
shot() { import -window root "$OUT-$1.png"; }
start() { "$BIN" "$@" >"$WORK/app.log" 2>&1 & APP=$!; sleep 20; xdotool windowfocus --sync "$(win)"; }
stop() { kill $APP 2>/dev/null || true; wait $APP 2>/dev/null || true; sleep 2; }
trap 'kill $APP 2>/dev/null || true' EXIT

# Welcome screen: first run, New Project, Get from VCS.
start
shot first-run
click $SKIP
click 319 204; shot new-project
click $NP_RUST; shot new-project-rust
move $NP_CREATE; shot new-project-hover
xdotool mousedown 1; sleep 0.5; shot new-project-pressed; xdotool mouseup 1; sleep 1
stop
start
click $SKIP
click 563 204; shot clone
stop
# 800x500 minimum window.
start
click $SKIP
xdotool windowsize --sync "$(win)" 800 500; sleep 2
click 318 204; shot new-project-800x500
stop
# 1280x800.
start
click $SKIP
xdotool windowsize --sync "$(win)" 1280 800; sleep 2
click 318 204; shot new-project-1280x800
stop
# Project dialogs from the tree context menu.
start "$DEMO"
click $SKIP 2>/dev/null || true
rclick 150 137; shot tree-menu
if [ "$MODE" = after ]; then R="210 389"; D="210 514"; else R="210 389"; D="210 514"; fi
click $R; shot rename
xdotool key Escape; sleep 1
rclick 150 137; click $D; shot delete
stop

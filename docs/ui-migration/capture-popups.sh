#!/usr/bin/env bash
# Captures Search Everywhere (each tab, hover, selected row) and the Switcher with real
# mouse input (xdotool under Xvfb): the toolbar search icon opens the popup, tabs are clicked.
#
#   docs/ui-migration/capture-popups.sh <jetthing-app binary> <output prefix>
#
# Same environment and demo project as capture.sh; coordinates assume 1440x900.
set -euo pipefail
BIN=${1:?binary}; OUT=${2:?output prefix}
DISPLAY_NUM=:77
export DISPLAY=$DISPLAY_NUM JETTHING_RENDERER=glow JETTHING_NO_UPDATE_CHECK=1 LIBGL_ALWAYS_SOFTWARE=1
WORK=$(mktemp -d); export HOME=$WORK/home; mkdir -p "$HOME"
DEMO=$WORK/demo
mkdir -p "$DEMO/src/main/java/com/example/app" "$DEMO/docs"
cat > "$DEMO/src/main/java/com/example/app/Main.java" <<'JAVA'
package com.example.app;

public class Main {
    public static void main(String[] args) {
        System.out.println(new Main().greet());
    }

    public String greet() {
        return "Hello";
    }

    interface Listener {
        void onChange(int value);
    }
}
JAVA
printf 'package com.example.app;\n\npublic final class Util {}\n' > "$DEMO/src/main/java/com/example/app/Util.java"
echo '# Demo' > "$DEMO/README.md"; echo 'plugins {}' > "$DEMO/build.gradle.kts"
pgrep -f "Xvfb $DISPLAY_NUM" >/dev/null || { Xvfb $DISPLAY_NUM -screen 0 1440x900x24 >/dev/null 2>&1 & sleep 2; }
"$BIN" "$DEMO" >"$WORK/app.log" 2>&1 &
APP=$!
trap 'kill $APP 2>/dev/null || true' EXIT
sleep 15
xdotool search --name Jetthing windowfocus 2>/dev/null || true
move() { xdotool mousemove "$1" "$2"; sleep 0.6; }
click() { move "$1" "$2"; xdotool click 1; sleep 0.8; }
shot() { import -window root "$OUT-$1.png"; }

click 673 411                       # first-run dialog: Skip
click 1357 107                      # banner: Dismiss
sleep 2
click 1287 20                       # toolbar search icon
sleep 1
xdotool type --delay 80 main; sleep 1
shot all-hover-none
move 600 214; shot all-hover-row
xdotool key Down Down; sleep 0.5; shot all-selected
# Tabs, by mouse: the strip sits at the top of the popup.
for entry in "classes 469" "files 531" "symbols 595" "actions 668" "text 730"; do
  set -- $entry; click "$2" 125; shot "tab-$1"
done
xdotool key Escape; sleep 0.8; shot closed

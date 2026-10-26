#!/usr/bin/env bash
# Captures the Project tree and Structure tool window with real mouse input only
# (xdotool under Xvfb), so the same steps give before and after screenshots.
#
#   docs/ui-migration/capture.sh <archipel-ide binary> <output prefix> [demo project dir]
#
# Needs: Xvfb, xdotool, ImageMagick `import`, libxkbcommon-x11.
# Coordinates assume a 1440x900 screen and the Islands Dark theme with the tool window
# stripes on the left; rows are 24 px apart.
set -euo pipefail
BIN=${1:?binary}; OUT=${2:?output prefix}; DEMO=${3:-}
DISPLAY_NUM=:77
export DISPLAY=$DISPLAY_NUM ARCHIPEL_RENDERER=glow ARCHIPEL_NO_UPDATE_CHECK=1
WORK=$(mktemp -d); export HOME=$WORK/home; mkdir -p "$HOME"
if [ -z "$DEMO" ]; then
  DEMO=$WORK/demo
  mkdir -p "$DEMO/src/main/java/com/example/app" "$DEMO/src/test/java/com/example/app" "$DEMO/docs" "$DEMO/build"; echo out > "$DEMO/build/out.txt"
  cat > "$DEMO/src/main/java/com/example/app/Main.java" <<'JAVA'
package com.example.app;

public class Main {
    private final String name = "demo";
    private int counter;

    public static void main(String[] args) {
        System.out.println(new Main().greet());
    }

    public String greet() {
        return "Hello " + name;
    }

    public void increment() {
        counter++;
    }

    interface Listener {
        void onChange(int value);
    }

    enum Mode { FAST, SLOW }
}
JAVA
  printf 'package com.example.app;\n\npublic final class Util {\n    public static int twice(int x) { return x * 2; }\n}\n' > "$DEMO/src/main/java/com/example/app/Util.java"
  printf 'package com.example.app;\n\nclass MainTest {\n    void greets() {}\n}\n' > "$DEMO/src/test/java/com/example/app/MainTest.java"
  echo '# Demo' > "$DEMO/README.md"; echo '{"a":1}' > "$DEMO/docs/config.json"; echo 'plugins {}' > "$DEMO/build.gradle.kts"
fi

pgrep -f "Xvfb $DISPLAY_NUM" >/dev/null || { Xvfb $DISPLAY_NUM -screen 0 1440x900x24 >/dev/null 2>&1 & sleep 2; }
"$BIN" "$DEMO" >"$WORK/app.log" 2>&1 &
APP=$!
trap 'kill $APP 2>/dev/null || true' EXIT
sleep 15

move() { xdotool mousemove "$1" "$2"; sleep 0.6; }
click() { move "$1" "$2"; xdotool click 1; sleep 0.8; }
dbl() { move "$1" "$2"; xdotool click --repeat 2 --delay 80 1; sleep 0.8; }
rclick() { move "$1" "$2"; xdotool click 3; sleep 0.8; }
shot() { import -window root "$OUT-$1.png"; }

click 673 411                       # first-run dialog: Skip
click 1357 107                      # banner: Dismiss
dbl 120 161                         # src
dbl 142 185                         # main
dbl 158 209                         # java
dbl 218 233                         # com.example.app
move 200 257; shot project-hover    # hover a file row
rclick 200 257; shot project-context-menu
xdotool key Escape; sleep 0.5
click 200 257                       # select (single click previews the file)
dbl 200 257                         # open Main.java
click 20 136                        # Structure tool window stripe button
move 190 150; shot structure-hover
click 190 174; shot structure-selected
rclick 190 198; shot structure-context-menu
xdotool key Escape; sleep 0.5
# Drag Util.java onto the test folder.
click 20 62                         # back to the Project tool window
move 200 281
xdotool mousedown 1; sleep 0.3; xdotool mousemove 190 300 mousemove 140 330 mousemove 140 305; sleep 0.6
shot project-drag
xdotool mouseup 1; sleep 1.5
shot project-dropped                # after: Util.java moved into test/; before: nothing happens
echo "wrote $OUT-*.png"

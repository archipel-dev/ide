# Android layout editor, Resource Manager and Running Devices

Captured on Linux/Xvfb 1600×1000 (glow) with the UI control socket, against a generated
`my-application` project (PNG and vector drawables, colours, strings, a layout) and a fake
`adb` (`track-devices`, `getprop`, `exec-out screencap -p` returning a PNG).

Reproduction: `ANDROID_HOME=<fake sdk> ARCHIPEL_CONTROL_SOCKET=… cargo run -p archipel-ide-app -- my-application`,
then `archipel-cli ui execute toolwindow.activate.resourceManager '{}'` and `screenshot <path>`.

| Item | Result | Evidence |
|---|---|---|
| #158 Resource Manager | Find Action lists "Resource Manager Tool Window"; module combo, 5 tabs, search, previews for PNG, vector (with `@color/` resolved) and colour; string values | `audit-v0.0.5/a08-resource-manager-fixed-drawable.png`, `…-color.png`, `…-string.png` |
| #168 Running Devices | "Running Devices Tool Window" findable; empty state with Device Manager link; screen mirrored through polling `screencap` | `audit-v0.0.5/a03-running-devices-fixed-empty.png`, `…-mirror.png` |
| #154 Layout editor | `Code \| Split \| Design`; Component Tree, labelled-box preview, Attributes | `audit-v0.0.5/a04-layout-editor-fixed-split.png`, `…-design.png` |

Limits: the preview is an approximation of the view tree, not Android's layout engine; mirroring is
view only at the polling rate of `screencap`.

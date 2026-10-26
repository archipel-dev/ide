# Toolbar alignment

The macOS toolbar previously inserted a 72-point spacer for native window controls
while the viewport hid its title bar. The menu consequently started about 80 points
from the content edge with no controls occupying the reserved space.

The toolbar now starts at its six-point content margin on every platform. macOS
uses its standard native title bar above the toolbar, retaining window controls.
This is a deliberate interim layout, not a claim of merged-title-bar parity.

Reproduce on macOS:

```sh
ARCHIPEL_NO_UPDATE_CHECK=1 cargo run -p archipel-ide-app -- .
```

Check the menu aligns with the left content edge, native close/minimize/zoom controls
are visible, and the project, VCS and run widgets still work. Repeat in full screen
and restore. The existing Linux screenshots predate this fix.

Manual macOS verification on 2026-10-10 with the welcome-screen development
preview confirmed the left-aligned menu and project/VCS/run widgets. Native
accessibility exposed close, minimize and full-screen controls. Full-screen
transition and pixel-level comparison remain unverified. See
`welcome-project-opening.md` for the project-opening walkthrough.

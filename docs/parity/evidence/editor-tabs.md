# Editor tabs: pin, close variants, reopen, navigation

Linux, Xvfb 1600x1000, `ARCHIPEL_RENDERER=glow`, debug build; project with `src/main.rs`,
`Cargo.toml`, `README.md` opened from the command line. Input via `xdotool`. Reference behaviour:
`PLATFORM-UI-001..003` in `docs/spec/features/platform.md` (no IntelliJ capture of the tab context
menu exists in `reference/` yet).

| File | Steps | Shows |
|---|---|---|
| `editor-tabs/linux-tab-context-menu.png` | right-click a tab | Close (Ctrl+F4), Close Other/All/Unmodified Tabs, Close Tabs to the Left/Right, Pin Tab, Reopen Closed Tab (Ctrl+Shift+T) |
| `editor-tabs/linux-pin-close-reopen-strip.png` | Pin `Cargo.toml`; Ctrl+F4 on `README.md`; Ctrl+Shift+T; Alt+← | row 1: pinned tab moved leftmost with a pin, README closed; row 2: README reopened at its old place at the end; row 3: Alt+← selected the previous tab |

Automated coverage: `archipel-ide-core::tabs` tests (close variants keep pinned tabs, reopen restores
order and caret, LRU tab limit, pinned workspace round-trip, wraparound navigation).

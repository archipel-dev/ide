# Workbench chrome on archipel-kit: before and after

Same walkthrough twice on the same small Kotlin/Gradle sample (`Main.kt`, `Util.kt`,
`build.gradle.kts`, `README.md`, a git repository), Xvfb `:95` at 1440×900 with openbox,
`JETTHING_RENDERER=glow`, a fresh `HOME`. Every step was a real `xdotool` mouse move,
click, double click or key press on the window; nothing was dispatched through the control
socket. `before-*` is `origin/main` (`c7fafe6`, archipel-kit 0.1), `after-*` is this branch
(archipel-kit 0.3.0). The images are reduced to 256 colours.

Android Studio Rabbit 1 (2026.2.1, Islands Dark, Linux, safe mode so that no Gradle sync
ran) was driven the same way on a copy of the sample for comparison. Its screenshots are
not in the repository (clean-room); what they showed is described below.

| Step | Before | After |
|---|---|---|
| Open the project (first-run dialog, toolbar, stripes, status bar) | [before-01](before-01-open-project.png) | [after-01](after-01-open-project.png) |
| Expand `src/main/kotlin/…`, open `Main.kt` and `Util.kt` by double click | [before-02](before-02-editor.png) | [after-02](after-02-editor.png) |
| Find in Files (Ctrl+Shift+F, type `counter`) | [before-03](before-03-find-in-files.png) | [after-03](after-03-find-in-files.png) |
| Terminal stripe button | [before-04](before-04-terminal.png) | [after-04](after-04-terminal.png) |
| Problems stripe button | [before-05](before-05-problems.png) | [after-05](after-05-problems.png) |
| Settings (Ctrl+Alt+S) | [before-06](before-06-settings.png) | [after-06](after-06-settings.png) |
| Completion (type `parts.jo`, Ctrl+Space) | [before-07](before-07-completion.png) | [after-07](after-07-completion.png) |
| Project widget menu | — | [after-10](after-10-project-menu.png) |
| VCS widget menu | — | [after-11](after-11-vcs-menu.png) |
| Run configuration menu | — | [after-12](after-12-run-config-menu.png) |
| Settings gear menu | — | [after-13](after-13-settings-menu.png) |
| Run button tooltip (hover) | — | [after-14](after-14-run-tooltip.png) |
| Stripe button tooltip (hover) | — | [after-15](after-15-stripe-tooltip.png) |

## What changed

- **Main toolbar** is the kit's `WindowToolbar` (undecorated window with the kit's
  minimize, maximize and close on Linux and Windows, traffic lights in the toolbar band on
  macOS). The project widget is a `ToolbarButton` with the project's `Avatar` tile and a
  chevron, and the project-color glow is the kit's `project_glow` across the toolbar,
  stripes and gaps, so the frame is one surface as in Android Studio. The VCS, run
  configuration and device widgets are `ToolbarButton`s that stay filled while their menu
  is open. Build, Run, Debug, Stop, Search Everywhere and the Settings gear are 30 pt
  `IconButton::large` buttons. The painted window title in the toolbar is gone (Android
  Studio has none on Linux either).
- **Menus** of the toolbar widgets (project, run configurations, gear, devices) are kit
  `MenuItem`s, `MenuHeader`s and `MenuSeparator`s: 24 pt rows, inset pills, check marks
  for the current run configuration, Run/Debug/Edit… submenus per configuration.
- **Stripes** are the kit's `ToolStripe`: 38 pt, 30 pt buttons on a 40 pt pitch, open
  (pressed fill) and active (accent, focus inside the tool window) states, tooltips beside
  the button at once with the shortcut. Plugin tool windows are on the right stripe with
  kit glyphs.
- **Tool window headers** are the kit's `Panel` header: 40 pt, semibold title, kit
  `IconButton` actions and the named Hide button.
- **Status bar** is 30 pt on the frame with the kit's `Breadcrumbs` (project, folders,
  file, declarations around the caret, with icons) and `StatusBarItem`s for indent,
  encoding, line separator, caret position, LSP, toolchain, AI and plugin widgets: dimmed
  at rest, full strength and a hover pill under the pointer, tooltips.

## Compared with Android Studio

- Same: toolbar band height and layout (menu, project widget with tile and glow, VCS
  widget left; run configuration, Run, Debug, then search and settings right), stripe
  geometry and tooltips, status bar height and navigation path with icons, 40 pt island
  headers.
- Different, planned in `docs/roadmap-as-parity.md`:
  - Android Studio's run widget has a ⋮ "more actions" button after Debug and keeps Build
    there; Jetthing shows Build (hammer) left of the configuration.
  - Tool window header actions are always visible; Android Studio shows them only while the
    pointer is over the tool window (needs a header-only `Panel` in the kit).
  - Editor tabs are still Jetthing's own painting (kit `TabBar` has no per-tab context
    menu, drag, pinned or preview state yet).
  - Problems content tabs (`File`, `Project Errors`) are underlined text tabs; Android
    Studio's are pills (`Panel::tabs`).
  - Settings: Android Studio has the search field above the tree and the tree inside the
    dialog; Jetthing's search column is an empty bordered box and the tree runs below the
    dialog (before and after).
  - Completion with nothing to offer: Android Studio shows "No suggestions" in a hint at
    the caret; Jetthing only writes it into the status bar.
  - Find in Files: Android Studio has a file-mask checkbox with a combo, a filter and a pin
    button in the title row and a "Open results in new tab" checkbox in the footer.

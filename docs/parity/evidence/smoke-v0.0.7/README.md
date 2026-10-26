# Usability smoke test, v0.0.7 release candidate

Release build of `main` at `1725ba7` (plus the fixes in this PR), driven over the
control socket (`docs/parity/evidence/ui-control-socket.md`) under Xvfb 1440×900 with
`ARCHIPEL_RENDERER=glow`, Ubuntu 24.04, JDK 21, Gradle on `PATH`, no `jdtls` and no
`java-debug` installed. Observed on 2026-10-11.

Every flow below was driven end to end; the app never crashed, panicked or hung.
The app log contained no `panicked` line in any run.

| # | Flow | Result | Evidence |
|---|------|--------|----------|
| 1 | Welcome screen on a fresh config dir | shows New Project / Open / Get from VCS, no recent projects | `01-welcome.png` |
| 2 | New Project wizard (click New Project, type name + location, Enter) | creates a Gradle/Java/Kotlin-DSL project with sample code and opens it | `02-new-project-wizard.png`, `02b-new-project-filled.png`, `03-project-opened.png` |
| 3 | Close project → welcome shows Recent Projects → click the row | project reopens | `04-welcome-recent.png`, `05b-reopened-from-recent-row.png` |
| 4 | Typing in the editor | text and caret update, no lag | `06-completion.png` |
| 5 | Completion popup after `Sys` | **not shown**: no Java language server (`jdtls`) in the container, the editor shows the toolchain banner instead; not a bug, see note | `06-completion.png` |
| 6 | `sout` + Tab live template | expands to `System.out.println();` | `07-sout-live-template.png` |
| 7 | `new StringBuilder().var` + Tab postfix | expands to `StringBuilder stringBuilder = new StringBuilder();` | `08-postfix-var.png` |
| 8 | Search Everywhere (Double Shift), query `Main` | opens on the All tab with results, Escape closes | `09-search-everywhere.png` |
| 9 | Find Action (Ctrl+Shift+A), query `settings` | opens on the Actions tab | `10-find-action.png` |
| 10 | Find in Files (Ctrl+Shift+F), query `Hello` | 1 result in the project | `11-find-in-files.png` |
| 11 | Run/Debug Configurations dialog (`runConfig.edit`) | opens, lists the 8 configuration types | `12-run-configurations.png` |
| 12 | Run (Shift+F10) on `Main.java` | Gradle `run` task, status `Main finished successfully`, output in the Run window | `13-run-output.png` |
| 13 | Debug (`debug.start`) | Debug window opens and reports the missing `jdtls`/`java-debug` as a structured message, no crash | `14-debugger.png` |
| 14 | Git commit view (Alt+0) after an outside change and an unversioned file | Changes 1 file, Unversioned 1 file, commit message box | `15-commit.png` |
| 15 | Settings → Keymap | 822 actions grouped by menu, shortcuts shown | `16-settings-preferences.keymap.png` |
| 16 | Settings → Editor → Live Templates | page renders | `16-settings-editing.templates.png` |
| 17 | Settings → Build Tools → Gradle | page renders | `16-settings-reference.settingsdialog.project.gradle.png` |
| 18 | Project tree right-click | context menu opens with New ▸ | `19-project-context-menu.png` |

## Findings

- **Missing system library is reported, not crashed on.** On a container without
  `libxkbcommon-x11`, the app exits with an actionable message naming the package to
  install. Installing `libxkbcommon-x11-0` and `libegl1` fixed the start-up.
- **Completion without a language server.** With no `jdtls` the completion popup does
  not open after typing an identifier, so live/postfix templates are only reachable
  through Tab expansion (which works). The toolchain banner above the editor offers
  Set up. This matches the design (completion sessions are driven by the server).
- **Control socket gaps found while scripting** (not user-facing): the Settings dialog
  is not reported under `popups.dialog`, so scenes must assert it by screenshot.
- No crash, panic or hang was found in any flow, so no regression test was added for
  the smoke test itself; the code fixes in this PR each carry their own test where one
  existed for the touched code.

## Reproduction

```bash
Xvfb :99 -screen 0 1440x900x24 &
cargo build --release -p archipel-ide-app -p archipel-ide-cli
export DISPLAY=:99 ARCHIPEL_RENDERER=glow ARCHIPEL_NO_UPDATE_CHECK=1 \
       ARCHIPEL_CONTROL_SOCKET=/tmp/jt.sock
target/release/archipel-ide &                     # welcome screen
cli() { target/release/archipel-cli ui --socket /tmp/jt.sock "$@"; }
cli wait idle && cli click 318 205                 # New Project
cli key Primary+A && cli type demo && cli key Tab && cli key Primary+A && cli type /tmp/projects
cli key Enter && cli wait idle                     # project.create → project opens
cli execute editor.openFile '{"path":"src/main/java/org/example/Main.java"}'
cli execute editor.gotoLine '{"line":5}' && cli key End && cli key Enter
cli type sout && cli key Tab                       # live template
cli key DoubleShift && cli wait popup && cli type Main && cli screenshot se.png
```

# Parity scenes

Each `NN-name.json` scripts one scene of the reference walkthrough
(`docs/parity/evidence/reference/*/NOTES.md`, same numbering). `cargo run -p xtask -- scenes`
starts a fresh IDE per scene on a generated fixture project, replays the steps over the
control socket (`docs/parity/evidence/ui-control-socket.md`), asserts workbench state and
writes PNGs to `docs/parity/evidence/archipel-ide/scenes/` (or `--out DIR`). Scenes assert state,
never pixels; the screenshots are evidence to compare with the reference by eye.

```bash
cargo run -p xtask -- scenes                    # all scenes; starts Xvfb when DISPLAY is unset
cargo run -p xtask -- scenes --only 05,06       # a subset
cargo run -p xtask -- scenes --out /tmp/shots   # somewhere else
```

CI runs every scene headless (Xvfb + `ARCHIPEL_RENDERER=glow`) in the `parity scenes` job
and uploads the PNGs and app logs as the `parity-scenes` artifact.

## Format

```json
{
  "id": "05",
  "title": "Search Everywhere",
  "reference": "reference/intellij-community/java-gradle/05-search-everywhere.png",
  "fixture": "java-gradle",
  "open": ["src/main/java/com/example/App.java"],
  "steps": [
    {"op": "key", "chord": "DoubleShift"},
    {"op": "wait", "until": "popup"},
    {"op": "type", "text": "main"},
    {"op": "assert", "pointer": "/popups/searchEverywhere", "equals": "all"},
    {"op": "screenshot", "name": "05-search-everywhere.png"}
  ]
}
```

- `fixture` (optional) generates a project and commits it to a fresh Git repository on
  `master`. Known fixtures: `java-gradle`, `rust-cargo` (builds with Cargo; `src/broken.rs`
  is outside the crate and has syntax errors). Without a fixture the welcome screen shows.
- `open` lists project-relative files passed on the command line.
- `env` (optional) adds environment variables to the IDE process (scene 30 slows indexing
  down with `ARCHIPEL_INDEX_THROTTLE_MS`). Every scene gets its own `ARCHIPEL_INDEX_DIR`, so
  indexes and Local History never touch the user's cache.
- Steps are control requests (`execute`, `state`, `key`, `type`, `click`, `move`, `wait`)
  or runner steps:
  - `screenshot {name}` writes `<out>/<name>`;
  - `assert {pointer, equals | contains}` checks a JSON pointer into the `state` snapshot;
  - `sleep {ms}`;
  - `writeFile` / `appendFile {path, contents}` change fixture files, e.g. to create VCS
    changes.

Chords use the Linux keymap (`Alt+0`, `Alt+F12`); `Primary` means ⌘ on macOS and Ctrl elsewhere.

| Scene | Script | Asserts |
|---|---|---|
| 01 Welcome | `01-welcome.json` | no project, no tabs |
| 04 Editor + project tree | `04-editor-project-tree.json` | App.java active and revealed/selected in the tree, Project tool window visible, editor text |
| 05 Search Everywhere | `05-search-everywhere.json` | Double Shift opens the All tab; Escape closes it |
| 06 Find Action | `06-find-action.json` | Ctrl+Shift+A opens the Actions tab |
| 11 Terminal | `11-terminal.json` | terminal visible and focused; `ls --color` and `git log --oneline` output on screen |
| 12 Commit | `12-commit.json` | Alt+0 shows Commit after an outside change and an unversioned file |
| 07 Main menu | `07-main-menu.json` | hamburger click opens the menu; hovering Code shows its submenu |
| 10 Find in Files | `10-find-in-files.json` | Ctrl+Shift+F focuses the query; `greeting` gives 4 results |
| 16 Build output | `16-build-output.json` | Ctrl+F9 runs `cargo build` in the Build window and finishes successfully |
| 17 Run output | `17-run-output.json` | Shift+F10 runs `cargo run` and finishes successfully |
| 19 Project context menu | `19-project-context-menu.json` | right-click on a tree row opens the context menu with New ▸ |
| 20 Problems | `20-problems.json` | Alt+6 lists the 2 syntax errors of `src/broken.rs` |
| 30 Indexing | `30-indexing.json` | while the content indexes build the IDE is in dumb mode with `FileName` ready and `Words` building; afterwards every index is ready |
| 31 TODO window | `31-todo.json` | an outside change adds a TODO and a FIXME comment (a TODO in a string is ignored): the TODO window lists 2 items for the project and for the current file |
| 32 Local History | `32-history.json` | an outside change to an open file records two revisions (initial and external); Show History lists them and previews the diff of the older one |

Coordinates in `click`/`move` are logical points in the default 1440×900 window.

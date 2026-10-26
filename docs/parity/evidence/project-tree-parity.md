# Project tree parity (#150, #164)

Reference: the Project tool window of the reference IDE (scenes 04 and 19 of
[audit-v0.0.5](audit-v0.0.5/)). Screenshots below are Archipel IDE on Xvfb against a
java-gradle sample with a cached Gradle model (`.archipel/gradle/model.json`).

## Behavior

- **Compact middle packages**: `com` + `example` is one `com.example` row (toggle in the
  header `…` menu, command `project.toggleCompactPackages`, saved in the workspace). The row
  expands and collapses as the deepest directory.
- **Excluded directories**: `build`, `.gradle`, `target`, `out`, `node_modules`, `dist`
  (and anything marked Excluded) are drawn orange with a tinted row and start collapsed;
  `target` and `node_modules` are listed but only walked when expanded.
- **Source / test roots**: from the Gradle source sets, or *Mark Directory as*
  (`project.markDirectory`: Sources, Test Sources, Resources, Test Resources, Excluded,
  Unmark; saved in the workspace). Test roots and their packages get a green row tint.
- **External Libraries** (only after a Gradle sync): the JDK that ran Gradle and the
  resolved runtime jars of every module (`libraries` in the model, written by the init script).
- **Scratches and Consoles**: scratch files live in `.archipel/scratches`; the *Scratches*
  node offers *New Scratch File* (`project.newScratch`). No database consoles.
- **Context menu** (file and directory): New, Cut/Copy/Copy Path/Paste, Edit Source, Find in
  Files (scoped to the clicked path: `search.project {scope}`, shown as an `In src ×` chip),
  Rename, Refactor (Rename File, Move…, Safe Delete…), Delete, Run 'App'
  (`project.runFile`), Open In (Show in File Manager, Terminal), Local History (disabled),
  Git (Show Diff, Add, Unstage), Mark Directory as, Reload from Disk, Compare With HEAD.
  Entries without a command are hidden. Find Usages (needs a caret) and Replace in Files stay
  hidden. Reformat Code and Optimize Imports are keyed on the keymap's `ACTION_COMMANDS`
  (files only): they open the file and dispatch the mapped command.

## Screenshots

![Project tree](project-tree-parity/project-tree.png)
![File menu](project-tree-parity/context-menu-file.png)
![Refactor submenu](project-tree-parity/context-menu-refactor.png)
![Directory menu](project-tree-parity/context-menu-directory.png)
![Find in a directory](project-tree-parity/find-in-directory.png)

## Reproduction

```bash
Xvfb :99 -screen 0 1600x1000x24 &
export ARCHIPEL_CONTROL_SOCKET=/tmp/jt.sock DISPLAY=:99 ARCHIPEL_RENDERER=glow ARCHIPEL_NO_UPDATE_CHECK=1
cargo run -p archipel-ide-app -- /path/to/java-gradle &
cli() { cargo run -q -p archipel-ide-cli -- ui "$@"; }
cli execute project.toggleGroup '{"id":"external-libraries"}'
cli click 208 257 --button secondary; cli screenshot context-menu-file.png
```

## Gaps

- No Packages / Project Files view switcher, no file nesting, no drag and drop.
- Debug entries and test-directory runs are missing (the gutter has no debug variant yet).
- Library rows are leaves: no jar browsing or source attachment.
- Safe Delete is a text search for the name, not a semantic usage search.

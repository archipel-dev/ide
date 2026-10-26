# External file changes (#47 bug 2)

Reference behaviour (IntelliJ Platform VFS refresh): files changed outside the IDE are
reloaded into unmodified editors automatically; when the editor also has unsaved changes
the IDE asks which version to keep (File Cache Conflict) instead of overwriting either
side. File → Reload All from Disk (⌥⌘Y / Ctrl+Alt+Y) forces a refresh.

## Archipel IDE

- `archipel-ide-project::ProjectWatcher` (notify crate) watches the project recursively and
  batches events until the next frame; build output (`target`, `node_modules`, `.gradle`)
  and `.archipel` are ignored, `.git` changes only set `vcs_stale` for the VCS refresh.
- Each open document remembers a hash of the content it last loaded or saved, so the
  IDE's own saves are never mistaken for outside edits.
- Unmodified buffer + outside change → reloaded as one undoable edit
  ("Reloaded X (changed on disk)"). Unmodified buffer + deletion → tab closed.
- Modified buffer + outside change/deletion → a warning banner over the editor with
  **Load Disk Version** (`editor.reloadFromDisk`) / **Keep My Changes**
  (`editor.keepMemoryVersion`). Until one is chosen `editor.save`, `editor.saveAll`,
  autosave on focus loss and closing the tab refuse to write the file.
- `file.synchronize` (File → Reload All from Disk, Ctrl+Alt+Y) re-checks every open
  document and refreshes the Project tree; new and deleted files also refresh the tree
  automatically.
- If the watcher cannot start (e.g. inotify limit) the status bar says so and
  Reload All from Disk keeps working.

## Evidence

Captured 2026-10-10 on Xvfb/glow: `Other.java` (clean) and `App.java` (typed
`// my edit`) open, then both files rewritten with `printf > file` from a shell.

![Conflict banner, Load Disk Version result, clean tab reloaded](workbench-bugs/external-changes.png)

1. `App.java` shows the banner automatically (no input needed: the watcher wakes the UI);
   the file on disk still holds the outside content.
2. After **Load Disk Version** the buffer holds the disk content and is clean.
3. `Other.java` was reloaded silently with the outside change.

Tests: `archipel-ide-project` watcher filtering and a real write; `archipel-ide-core::disk_sync`
reload, conflict with save/saveAll/autosave refusal, keep-mine, load-disk, own-save,
deletion (clean closes, dirty conflicts and recreates on keep), and a real watcher round
trip; `archipel-ide-ui::disk_banner` command wiring.

## Gaps

- No diff view inside the conflict prompt ("Show Difference") yet.
- Large-file and binary changes are ignored for open editors (they cannot be opened).
- VCS status refresh on `vcs_stale` is owned by the VCS workstream (#47 bug 3).

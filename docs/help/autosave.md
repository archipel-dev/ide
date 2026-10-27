# Autosave

Archipel IDE writes unsaved editor text to disk at these moments:

- when you switch to another application or a built-in terminal (setting "Save files when
  switching to a different application or a built-in terminal", on by default);
- when the IDE has been idle for the configured time (setting "Save files if the IDE is idle
  for ... seconds", off by default, 1 to 300 seconds, 15 when enabled);
- before a run, debug session, test run, build, Gradle or Maven task, reload of the build
  model, and before version control operations such as commit, update, checkout and rollback;
- when you press Save All, close a project, or quit.

Neither setting disables the save points before runs, builds and version control: "Autosave
cannot be disabled completely."

Closing an editor tab does not save and does not ask. The text stays pending and is written by
the next of the moments above. Typing never saves.

"Back up files before saving" writes a temporary file next to the target and replaces the
target only when the write completed, so a failed write cannot destroy the previous content.

A file that changed on disk while it has unsaved edits is never overwritten silently: the editor
shows a banner to load the disk version or keep yours. If a file cannot be written (read-only,
disk full), "Cannot Save Files" lists the errors; "Revert Changes" reloads those files from disk,
"Close" keeps your text unsaved.

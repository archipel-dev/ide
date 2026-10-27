# Cases for build spec 02

Block format as in the build brief.

### BLD-SYNC-001 case 1: Build Tools page
settings: default
language: plain
```before
Open Settings | Build, Execution, Deployment | Build Tools
```
keys: (inspect)
```after
Page title: Build Tools
Group: Auto-Sync Projects
Sync project after changes in the build scripts:
( ) Any changes
(o) External changes   Changes outside the IDE or from version control.
```

### BLD-SYNC-002 case 1: edit and revert
settings: default
language: plain
```before
Gradle project synced; build.gradle checksum A
```
keys: edit build.gradle in editor (checksum B), then undo (checksum A)
```after
Project is up to date; Load button disappears (checksum equals the synced one).
```

### BLD-SYNC-002 case 2: created file
settings: default
language: plain
```before
Gradle project synced; no gradle.properties
```
keys: create gradle.properties on disk
```after
Counted as created: project stale; with External changes the reload auto-starts after 3 s.
```

### BLD-SYNC-002 case 3: restored dirty state
settings: default
language: plain
```before
Previous session closed while Load button was visible
```
keys: reopen project
```after
Project is dirty at start; after the tracker is activated the decision of BLD-SYNC-004 runs (button or auto-sync).
```

### BLD-SYNC-003 case 1: editor change
settings: default
language: plain
```before
build.gradle open in the editor
```
keys: type a character
```after
Modification type INTERNAL.
```

### BLD-SYNC-003 case 2: git pull
settings: default
language: plain
```before
build.gradle changed by a VCS update (one batch of events)
```
keys: (update)
```after
One EXTERNAL change; decision happens after the batch ends, not per file.
```

### BLD-SYNC-003 case 3: own save ignored
settings: default
language: plain
```before
Document modified in editor, then Save All
```
keys: Ctrl+S
```after
The save event is not an additional external change; type stays INTERNAL.
```

### BLD-SYNC-004 case 1: Any changes + editor edit
settings: auto-sync=Any changes
language: plain
```before
build.gradle edited in the editor
```
keys: type, wait 4 s
```after
Sync starts automatically after 3 s of quiet; no button.
```

### BLD-SYNC-004 case 2: External changes + editor edit
settings: auto-sync=External changes
language: plain
```before
build.gradle edited in the editor
```
keys: type, wait 4 s
```after
No sync. Floating button shown: Sync Gradle Changes
```

### BLD-SYNC-004 case 3: External changes + disk edit
settings: auto-sync=External changes
language: plain
```before
build.gradle modified by another program
```
keys: (file refresh)
```after
Sync starts automatically after 3 s; no button.
```

### BLD-SYNC-004 case 4: HIDDEN type
settings: auto-sync=Any changes
language: plain
```before
Change marked HIDDEN by the build tool
```
keys: (event)
```after
No automatic sync; button shown.
```

### BLD-SYNC-005 case 1: lookup open
settings: auto-sync=Any changes
language: plain
```before
Completion popup open while external change arrives
```
keys: (event)
```after
No auto-sync while the popup is open; when the popup closes the decision is re-evaluated (sync starts).
```

### BLD-SYNC-005 case 2: reload running
settings: default
language: plain
```before
Sync running; build file changes again
```
keys: (event)
```after
No second sync; after the sync finished the new change is processed.
```

### BLD-SYNC-005 case 3: untrusted
settings: default
language: plain
```before
Project untrusted, build.gradle changed
```
keys: (event)
```after
No auto-sync; button shown; invoking it asks for trust (BLD-SYNC-050).
```

### BLD-SYNC-006 case 1: merge window
settings: auto-sync=Any changes
language: plain
```before
Change a, 200 ms later change b, 200 ms later change c
```
keys: (events)
```after
One sync starts 3 s after change c.
```

### BLD-SYNC-007 case 1: button removal
settings: default
language: plain
```before
Button visible; user reverts the file to the synced content
```
keys: (undo)
```after
Button disappears.
```

### BLD-SYNC-008 case 1: failed sync
settings: default
language: plain
```before
Sync fails with a script error
```
keys: Sync Gradle Changes
```after
Build tab Sync shows failure; the floating button stays visible.
```

### BLD-SYNC-009 case 1: wrapper update by sync
settings: default
language: plain
```before
Sync updates gradle/wrapper/gradle-wrapper.properties
```
keys: (sync)
```after
After the sync the project is up to date; no button.
```

### BLD-SYNC-010 case 1: setting change
settings: default
language: plain
```before
External-only setting, editor edit pending with button
```
keys: switch to Any changes
```after
Auto-sync starts within the delay; button disappears.
```

### BLD-SYNC-020 case 1: one system
settings: default
language: plain
```before
Gradle changes pending
```
keys: (look)
```after
Floating button text: Sync Gradle Changes; tooltip: Gradle project structure has been changed. Sync changes with <product> to make it work correctly.
```

### BLD-SYNC-020 case 2: two systems
settings: default
language: plain
```before
Gradle and Maven changes pending
```
keys: (look)
```after
Text: Sync Gradle and Maven Changes; generic icon
```

### BLD-SYNC-020 case 3: none
settings: default
language: plain
```before
No pending change
```
keys: (look)
```after
Not visible; template text Sync Changes, description Sync build system changes with <product>.
```

### BLD-SYNC-021 case 1: load
settings: default
language: plain
```before
Button visible, project trusted
```
keys: click button
```after
Immediate sync of all pending projects; button disappears when sync starts.
```

### BLD-SYNC-022 case 1: hide
settings: default
language: plain
```before
Button visible
```
keys: click Hide This Notification
```after
Button hidden, no sync; next change re-shows it.
```

### BLD-SYNC-023 case 1: shortcut
settings: default
language: plain
```before
Button visible
```
keys: Ctrl+Shift+O (macOS 10.5+ keymap: Cmd+Shift+I)
```after
Sync starts.
```

### BLD-SYNC-030 case 1: background sync
settings: default
language: plain
```before
Open project linked to Gradle
```
keys: action ExternalSystem.RefreshAllProjects
```after
Progress title: Importing 'app' Gradle Project; Build tool window tab Sync: app Sync
```

### BLD-SYNC-030 case 2: modal import
settings: default
language: plain
```before
Importing a new project
```
keys: (import)
```after
Progress title: Building 'app' Gradle Project Info (modal)
```

### BLD-SYNC-030 case 3: before-sync fails
settings: default
language: plain
```before
Task 'gen' assigned Execute Before Sync and fails
```
keys: sync
```after
Sync does not start; no model change.
```

### BLD-SYNC-031 case 1: statuses
settings: default
language: plain
```before
Sync running then finished
```
keys: (watch)
```after
Root: app Sync - syncing… ; then successful
```

### BLD-SYNC-031 case 2: cancel
settings: default
language: plain
```before
Sync running
```
keys: Cancel in progress
```after
Tab status: cancelled; no data imported.
```

### BLD-SYNC-032 case 1: failure title
settings: default
language: plain
```before
Gradle script with syntax error
```
keys: sync
```after
Tree: Gradle 'app' project sync failed; message with 'Consult IDE log for more details (Help | Show Log)'; notification title Gradle 'app' project sync failed
```

### BLD-SYNC-032 case 2: already running
settings: default
language: plain
```before
Sync running; second sync requested for the same path
```
keys: sync
```after
Failure message: Another 'Sync project' task is currently running for the project:  /work/app
```

### BLD-SYNC-032 case 3: missing script
settings: default
language: plain
```before
Linked build.gradle deleted
```
keys: sync
```after
Gradle script file '/work/app/build.gradle' not found
```

### BLD-SYNC-032 case 4: empty message
settings: default
language: plain
```before
Resolver fails with empty message, reason 'x'
```
keys: sync
```after
Can't resolve Gradle project at '/work/app'. Reason: x
```

### BLD-SYNC-033 case 1: quick fix link
settings: default
language: plain
```before
Failure text contains a link
```
keys: click the link
```after
Fix runs; if it is a Reload fix, a new sync starts.
```

### BLD-SYNC-034 case 1: sync types
settings: default
language: plain
```before
New project
```
keys: first sync; second sync; untrusted open
```after
import; re-import; preview
```

### BLD-SYNC-035 case 1: summary words
settings: default
language: plain
```before
Task finished/failed/cancelled
```
keys: (watch)
```after
Summary lines: Gradle finished / Gradle failed / Gradle cancelled
```

### BLD-SYNC-036 case 1: incomplete deps
settings: default
language: java
```before
Sync running
```
keys: open a file using a library not yet imported
```after
No unresolved-reference errors reported for not-yet-imported libraries until sync ends.
```

### BLD-SYNC-040 case 1: action texts
settings: default
language: plain
```before
Gradle tool window
```
keys: open menus
```after
Sync All Gradle Projects / Sync Gradle Project / Unlink Gradle Project / Ignore Gradle Project / Open Gradle Config / Select Project Data To Import / Gradle Settings… / Auto-Sync Settings…
```

### BLD-SYNC-041 case 1: unlink
settings: default
language: plain
```before
Gradle project 'app' with module app selected
```
keys: Unlink Gradle Project
```after
Confirmation: Gradle Module app will be unlinked. No files will be deleted. (two modules: Gradle Modules a, b will be unlinked.)
```

### BLD-SYNC-042 case 1: ignore
settings: default
language: plain
```before
Project 'lib' ignored
```
keys: toggle Show Ignored off
```after
lib node disappears; sync does not include lib.
```

### BLD-SYNC-043 case 1: orphans
settings: default
language: plain
```before
Module 'old' removed from settings.gradle
```
keys: sync
```after
Notification: The IDE modules below were removed by the Gradle project sync: old  You can open a dialog to select the ones you need to restore. Dialog title Orphan Modules.
```

### BLD-SYNC-044 case 1: select data
settings: default
language: plain
```before
Project with two data nodes
```
keys: Select Project Data To Import
```after
Dialog 'Gradle Project Data To Import': Please select the modules/data to include in the project. [Select All] [Unselect All]
```

### BLD-SYNC-045 case 1: recovery
settings: default
language: plain
```before
(any)
```
keys: search everywhere: Invalidate Build System IDE caches and Re-sync
```after
Cached data cleared, full sync starts.
```

### BLD-SYNC-046 case 1: dedup
settings: default
language: plain
```before
Two modules declare the same content root
```
keys: sync
```after
Notification Duplicate content roots detected: Path [/w/shared] of module [b] was removed from modules [a]
```

### BLD-SYNC-050 case 1: preview
settings: default
language: plain
```before
Untrusted project
```
keys: sync
```after
Preview sync: scripts are not executed; message: Gradle project is untrusted, so its task cannot be executed. appears when running a task.
```

### BLD-SYNC-050 case 2: refuse
settings: default
language: plain
```before
Load clicked, trust dialog
```
keys: Stay in Safe Mode
```after
No sync.
```

### BLD-SYNC-051 case 1: dialog
settings: default
language: plain
```before
Untrusted Gradle and Maven
```
keys: Load
```after
Title Trust Gradle and Maven Projects?  Buttons: Trust Project | Stay in Safe Mode
```

### BLD-SYNC-060 case 1: toggle
settings: default
language: plain
```before
Task 'build' selected
```
keys: check Execute After Sync
```after
Task stored under After Sync; menu shows checked.
```

### BLD-SYNC-061 case 1: after sync
settings: default
language: plain
```before
Task bound After Sync
```
keys: successful sync
```after
Task runs after import.
```

### BLD-SYNC-061 case 2: preview
settings: default
language: plain
```before
Untrusted
```
keys: sync
```after
After-sync task does not run.
```

### BLD-SYNC-062 case 1: build bind
settings: default
language: plain
```before
Task bound Before Build
```
keys: Ctrl+F9
```after
Task runs before the build.
```

### BLD-SYNC-063 case 1: dialog
settings: default
language: plain
```before
Project node
```
keys: Tasks Activation
```after
Dialog lists Before Sync, After Sync, Before Build, After Build, Before Rebuild, After Rebuild, Before Run
```

### BLD-SYNC-070 case 1: availability
settings: default
language: plain
```before
No linked projects
```
keys: (look)
```after
Gradle tool window button not available; after linking: available; while loading: Initializing Gradle projects data…
```

### BLD-SYNC-071 case 1: defaults
settings: default
language: plain
```before
Fresh project
```
keys: gear menu
```after
[x] Group Modules [x] Group Tasks [x] Display Tasks Inherited from Sub-Projects [x] Show Ignored
```

### BLD-SYNC-071 case 2: persist
settings: default
language: plain
```before
Group Tasks turned off
```
keys: reopen project
```after
Still off.
```

### BLD-SYNC-072 case 1: grouping
settings: default
language: plain
```before
Tasks: build(group build), clean(group build), x(no group), help(group help)
```
keys: (look)
```after
Tasks
  build
    build
    clean
  help
    help
  other
    x
```

### BLD-SYNC-072 case 2: flat
settings: Group Tasks=off
language: plain
```before
same tasks
```
keys: (look)
```after
Tasks
  build
  clean
  help
  x   (flat; ordering by name)
```

### BLD-SYNC-073 case 1: task menu
settings: default
language: plain
```before
Right-click task
```
keys: (look)
```after
Run Task | Create '...' | Jump to Source | Execute Before/After ... | Assign Shortcut…
```

### BLD-SYNC-074 case 1: shortcut
settings: default
language: plain
```before
Task
```
keys: Assign Shortcut…
```after
Keymap dialog under group External Build Systems
```

### BLD-SYNC-075 case 1: run
settings: default
language: plain
```before
Task build
```
keys: double-click
```after
Run tab: Gradle: Executing build…  then Gradle: Execution finished build.
```

### BLD-SYNC-080 case 1: notification
settings: default
language: plain
```before
build.gradle found, not linked
```
keys: (open project)
```after
Notification: Gradle 'app' build scripts found  [Load Gradle Project] [Skip]
```

### BLD-SYNC-080 case 2: skip
settings: default
language: plain
```before
Notification
```
keys: Skip
```after
Not shown again for Gradle in this project.
```

### BLD-SYNC-081 case 1: explicit link
settings: default
language: plain
```before
Project view: build.gradle
```
keys: context menu
```after
Link Gradle Project; after click a sync starts.
```

### BLD-SYNC-090 case 1: analyzer
settings: default
language: plain
```before
Gradle project synced
```
keys: Analyze Gradle Dependencies…
```after
Editor tab Dependency Analyzer with Scope: Standard, Resolved Dependencies, Usages of <name>; Show Conflicts Only
```

# Cases for build spec 01

Block format as in the build brief. For UI behaviour the `after` block lists the expected menu, tree, output or notification contents in order. Unless stated, `language: java`.

### BLD-MAKE-001 case 1: Build menu content
settings: default
language: plain
```before
Project open, no build running, editor with a Java file focused.
```
keys: open main menu "Build"
```after
Build Project        Ctrl+F9
Build Module 'app'
Recompile 'Main.java'   (file target; for a module/package the same slot reads "Rebuild ...", see BLD-MAKE-008)
---
Rebuild Project
---
Build Artifacts…
```

### BLD-MAKE-002 case 1: Disabled while building
settings: default
language: plain
```before
A build is running (status "running…").
```
keys: open Build menu
```after
Build Project: disabled
Build Module: disabled
Rebuild/Recompile: disabled
Rebuild Project: disabled
(after the build finishes, all enabled again)
```

### BLD-MAKE-003 case 1: auto-import before build
settings: default
language: java
```before
--- Main.java
class Main { <caret>List<String> l; }   // unresolved List, import candidate java.util.List
```
keys: Ctrl+F9
```after
--- Main.java
import java.util.List;

class Main { List<String> l; }
(then the build runs with the import in place)
```

### BLD-MAKE-004 case 1: incremental Build Project
settings: default
language: java
```before
Modules app, lib. All built once. Only lib/Util.java is edited.
```
keys: Ctrl+F9
```after
Build tab "Build": root "Build" finished; recompiled: lib (Util.java) and dependents in app; untouched modules not listed.
Status notification: "Build completed successfully in <d>"
```

### BLD-MAKE-005 case 1: single module text
settings: default
language: plain
```before
Project view: module node 'app' selected.
```
keys: right-click, look at Build Module item
```after
Build Module 'app'  (enabled)
```

### BLD-MAKE-005 case 2: two short names
settings: default
language: plain
```before
Project view: modules 'a' and 'b' selected.
```
keys: open Build menu
```after
Build Modules 'a' and 'b'
```

### BLD-MAKE-005 case 3: long names
settings: default
language: plain
```before
Project view: modules 'application-core' and 'application-web' selected (joined text longer than 20 chars).
```
keys: open Build menu
```after
Build Selected Modules
```

### BLD-MAKE-005 case 4: no module, context menu
settings: default
language: plain
```before
Right click on a non-module file in project view that is outside every module.
```
keys: look at the popup
```after
"Build Module" item is hidden (project view popup), no entry.
Main menu instead shows: "Build Module" disabled
```

### BLD-MAKE-006 case 1: module context
settings: default
language: plain
```before
Module node 'app' selected in project view.
```
keys: Ctrl+Shift+F9
```after
Build tab "Rebuild": module app compiled non-incrementally; item text was "Rebuild Module 'app'"
```

### BLD-MAKE-006 case 2: file selection
settings: default
language: java
```before
--- Main.java
class Main {<caret>}
```
keys: Ctrl+Shift+F9
```after
Build tab "Recompile": only Main.java compiled; item text was "Recompile 'Main.java'"
```

### BLD-MAKE-006 case 3: no file, via shortcut
settings: default
language: plain
```before
Focus in the Git tool window, no module/file context.
```
keys: Ctrl+Shift+F9
```after
Nothing happens (item disabled by shortcut fallback rule: module fallback not used for shortcut place).
```

### BLD-MAKE-006 case 4: no file, via menu
settings: default
language: plain
```before
Focus in Git tool window; current module 'app' is known from the project.
```
keys: open Build menu
```after
"Rebuild Module 'app'" enabled
```

### BLD-MAKE-006 case 5: package
settings: default
language: java
```before
Package com.acme in the project view selected.
```
keys: Ctrl+Shift+F9
```after
item text: "Rebuild 'com.acme'"; all sources of the package compiled
```

### BLD-MAKE-007 case 1: compilable resource filter
settings: resource patterns default
language: plain
```before
Selection: src/main/resources/logo.png and src/main/resources/app.properties
```
keys: open Build menu
```after
Resource patterns default exclude only java/form/class/groovy/scala/flex/kt/clj/aj, so both are compilable resources:
text "Recompile Selected Files"
```

### BLD-MAKE-008 case 1: default package
settings: default
language: java
```before
Default (unnamed) package directory selected.
```
keys: open Build menu
```after
Rebuild '<default>'
```

### BLD-MAKE-008 case 2: long package name
settings: default
language: java
```before
Package directory com.example.very.long.package.name.of.a.module.that.is.deep selected (quoted description over 50 chars).
```
keys: open Build menu
```after
Rebuild '...<last 20 characters>   (leading quote, "...", then the last 20 characters of the quoted string)
```

### BLD-MAKE-008 case 3: file set
settings: default
language: java
```before
Two java files selected.
```
keys: open Build menu
```after
Recompile Selected Files
(the "Rebuild" slot is invisible)
```

### BLD-MAKE-009 case 1: local history label
settings: default
language: java
```before
Project with a compile error.
```
keys: Build | Rebuild Project
```after
Build tab: "Build completed with 1 error and 0 warnings in <d>"
Local History has a system label: 'Rebuild Project' with errors
```

### BLD-MAKE-010 case 1: artifact chooser
settings: default
language: plain
```before
Project has artifacts "web" and "cli".
```
keys: Build | Build Artifacts…
```after
Popup list (items per BLD-MAKE-010; submenu actions not specified, open question): All Artifacts, web, cli
```

### BLD-MAKE-011 case 1: request parameters
settings: default
language: plain
```before
Any project.
```
keys: action CompileDirty; action MakeModule (module a); action CompileProject
```after
CompileDirty: incremental=true, dependents=all, runtime=false
MakeModule: incremental=true, dependents=true, runtime=false
CompileProject: incremental=false, dependents=false, runtime=false
```

### BLD-MAKE-012 case 1: mixed runners
settings: default (Gradle build and run using = Gradle)
language: plain
```before
Module g (Gradle) and module i (plain IDE module).
```
keys: Ctrl+F9
```after
Build tabs: one "Build" tab for Gradle task batch (module g), one for the IDE builder (module i); aggregated result success only if both succeeded.
```

### BLD-MAKE-013 case 1: veto
settings: default
language: plain
```before
A before-run listener throws an execution exception.
```
keys: Ctrl+F9
```after
No tool window activity; result is "aborted"; no notification.
```

### BLD-MAKE-014 case 1: save before build
settings: default
language: java
```before
Unsaved edit in Main.java.
```
keys: Ctrl+F9
```after
Main.java is saved to disk before compilation starts; the build sees the edit.
```

### BLD-MAKE-015 case 1: dirty outputs
settings: default
language: plain
```before
Gradle-delegated build that does not report generated files.
```
keys: Ctrl+F9
```after
Output roots of the built modules are marked dirty and refreshed after completion.
```

### BLD-MAKE-016 case 1: scratch configuration
settings: default
language: java
```before
Run configuration for a Java scratch file.
```
keys: Shift+F10
```after
Build before run uses the IDE builder, not Gradle/Maven, even when delegation is on.
```

### BLD-MAKE-017 case 1: confirm rebuild
settings: default
language: plain
```before
Legacy compiler API rebuild request.
```
keys: (trigger)
```after
Dialog "Confirm Project Rebuild"
Message: "You are about to rebuild the whole project." newline "Run 'Build Project' instead?"
Buttons: [Build] (default) [Rebuild]
Build -> incremental build; Rebuild -> full rebuild.
```

### BLD-MAKE-020 case 1: success
settings: default
language: plain
```before
Build without messages, took 3 seconds.
```
keys: Ctrl+F9
```after
Notification text: Build completed successfully in 3 sec
```

### BLD-MAKE-020 case 2: warnings plural
settings: default
language: plain
```before
Build finished with 1 warning, then another with 2 warnings.
```
keys: Ctrl+F9
```after
Build completed successfully with 1 warning in <d>
Build completed successfully with 2 warnings in <d>
```

### BLD-MAKE-020 case 3: errors
settings: default
language: plain
```before
Build with 1 error and 0 warnings; then with 2 errors and 1 warning.
```
keys: Ctrl+F9
```after
Build completed with 1 error and 0 warnings in <d>
Build completed with 2 errors and 1 warning in <d>
```

### BLD-MAKE-020 case 4: up to date and cancelled
settings: default
language: plain
```before
Nothing changed since the last build; later a build is cancelled by the user.
```
keys: Ctrl+F9
```after
All files are up-to-date   (notification is not a link)
Compilation aborted
```

### BLD-MAKE-021 case 1: notification type
settings: default
language: plain
```before
Build with errors.
```
keys: Ctrl+F9
```after
Balloon group "Build finished", type error, text is a link to the Build tool window; clicking the link activates the Build tool window.
```

### BLD-MAKE-022 case 1: long build balloon
settings: Display notification on build completion=on
language: plain
```before
Build that takes 70 seconds.
```
keys: Ctrl+F9
```after
Balloon on the Build stripe button with the status text; for a 20-second build, no such balloon.
```

### BLD-MAKE-023 case 1: footer line
settings: default
language: plain
```before
Build with one warning message.
```
keys: Ctrl+F9
```after
Last INFORMATION line of the output: "<date time> - Build completed successfully with 1 warning in <d>"
For an UP-TO-DATE build with no messages no such line is added.
```

### BLD-MAKE-024 case 1: titles
settings: default
language: plain
```before
Project.
```
keys: Ctrl+F9; Ctrl+Shift+F9 on a file; Rebuild Project
```after
Tab titles: Build; Recompile; Rebuild
```

### BLD-MAKE-025 case 1: categories
settings: default
language: plain
```before
Build output with errors, warnings, infos, statistics.
```
keys: expand all
```after
Severity labels in text export: ERROR, WARNING, INFORMATION, STATISTICS
```

### BLD-MAKE-026 case 1: module without SDK
settings: default
language: plain
```before
Module 'app' has no SDK and the project has no SDK.
```
keys: Ctrl+F9
```after
Notification "Cannot start compiler" (group "Build failed to start"): "The SDK is not specified for module app", action [Configure…]
No Build tab created.
```

### BLD-MAKE-026 case 2: several modules
settings: default
language: plain
```before
Modules a, b, c without SDK.
```
keys: Ctrl+F9
```after
Notification text: "The SDK is not specified for modules" followed by the names a, b, c each on its own line
```

### BLD-MAKE-026 case 3: output path
settings: default
language: plain
```before
Module 'app' has no output path and no project output.
```
keys: Ctrl+F9
```after
"The output path is not specified for module app"
```

### BLD-MAKE-030 case 1: tab order
settings: default
language: plain
```before
Run, Debug, Build, Sync tabs created in that order.
```
keys: look at the tool window tabs
```after
Sync | Build | Run | Debug
```

### BLD-MAKE-031 case 1: reuse
settings: default
language: plain
```before
Build tab exists (finished).
```
keys: Ctrl+F9
```after
Same single "Build" tab is reused; the root line reads "Build running…" then "Build finished".
```

### BLD-MAKE-032 case 1: tree on failure
settings: default
language: java
```before
--- Main.java
class Main { void f() { int x = "s"; } }
```
keys: Ctrl+F9
```after
Tree:
Build: failed
  Compiler
    Main.java  1 error
      incompatible types: String cannot be converted to int   (at Main.java:1:34)
Selected: the first error node; console pane shows the same message.
```

### BLD-MAKE-033 case 1: warnings filter default
settings: default
language: java
```before
Build with one warning in A.java and one successful step.
```
keys: Filters popup
```after
[x] Show Warnings
[ ] Show Successful Steps
Tree shows A.java warning; successful steps hidden.
```

### BLD-MAKE-033 case 2: toggle off
settings: default
language: java
```before
Same build.
```
keys: uncheck "Show Warnings"
```after
Only failing nodes remain. Setting persisted: next build tab starts with Show Warnings off.
```

### BLD-MAKE-034 case 1: navigate
settings: default
language: java
```before
Failed build tree, two errors.
```
keys: select first error, F4; then action NextOccurence
```after
Editor opens Main.java at the error's line and column; Next Occurrence selects the second error node.
```

### BLD-MAKE-035 case 1: first error
settings: Automatically show first error in editor=on
language: java
```before
Build with two errors in different files.
```
keys: Ctrl+F9
```after
The editor opens the first error location once; the second error does not move the caret. With the setting off nothing opens.
```

### BLD-MAKE-036 case 1: show more
settings: default
language: plain
```before
Console line block exceeding the preview limit.
```
keys: click inlay
```after
Inlay text toggles "Show more" / "Show less".
```

### BLD-MAKE-037 case 1: failure activation
settings: default
language: plain
```before
Build tool window hidden. Manual build fails; then an auto-build fails.
```
keys: (events)
```after
Manual: window shown. Auto-build: window not shown, balloon "Auto build completed with errors" only.
```

### BLD-MAKE-038 case 1: pin
settings: default
language: plain
```before
Pinned Build tab.
```
keys: Ctrl+F9
```after
A new tab is created; pinned one unchanged.
```

### BLD-MAKE-039 case 1: sync tab
settings: default
language: plain
```before
Gradle project sync started.
```
keys: (event)
```after
Tool window "Build" gets tab "Sync"; it is ordered before "Build".
```

### BLD-MAKE-040 case 1: compiler page
settings: default
language: plain
```before
Open Settings | Build, Execution, Deployment | Compiler.
```
keys: (inspect)
```after
Resource patterns: !?*.java;!?*.form;!?*.class;!?*.groovy;!?*.scala;!?*.flex;!?*.kt;!?*.clj;!?*.aj
[x] Clear output directory on rebuild
[x] Add runtime assertions for notnull-annotated methods and parameters   Configure annotations…
[x] Automatically show first error in editor
[x] Display notification on build completion
[ ] Build project automatically   (Only works while not running / debugging)
[x] Rebuild module on dependency change
Compile independent modules in parallel: [Automatic]
Build Process: Shared heap size: 700 Mbytes; Shared VM options: ; User-local heap size: ; User-local VM options:
```

### BLD-MAKE-041 case 1: off by default
settings: default
language: java
```before
Edit a file, wait 10 s.
```
keys: (wait)
```after
No build starts.
```

### BLD-MAKE-041 case 2: running app blocks
settings: Build project automatically=on
language: java
```before
Application run configuration is running.
```
keys: edit file, wait
```after
No auto-build. With advanced setting compiler.automake.allow.when.app.running=on it builds.
```

### BLD-MAKE-041 case 3: untrusted project
settings: Build project automatically=on
language: java
```before
Project in safe mode (untrusted).
```
keys: edit, wait
```after
No auto-build.
```

### BLD-MAKE-042 case 1: postpone
settings: Build project automatically=on
language: java
```before
Edit, save, keep typing every second.
```
keys: type continuously for 10 s, then stop for 4 s
```after
No build during activity (idle < 3000 ms or unsaved documents); one build ≥3 s after the last activity.
```

### BLD-MAKE-043 case 1: repeat
settings: Build project automatically=on
language: java
```before
File changed on disk while an auto-build is running.
```
keys: (events)
```after
After the run a second auto-build is scheduled once.
```

### BLD-MAKE-044 case 1: failure
settings: Build project automatically=on
language: java
```before
Auto-build with a compile error.
```
keys: (wait)
```after
Notification: Auto build completed with errors
```

### BLD-MAKE-044 case 2: crash
settings: Build project automatically=on
language: java
```before
Build process crashes with "boom".
```
keys: (wait)
```after
Notification: Auto build failure: boom
```

### BLD-MAKE-045 case 1: dependency change
settings: Rebuild module on dependency change=on
language: java
```before
Library added to module app after a successful build.
```
keys: Ctrl+F9
```after
Whole module app recompiled (not only dirty files). With setting off, only dirty files.
```

### BLD-MAKE-046 case 1: heap
settings: shared heap 700, user-local heap 1024
language: plain
```before
Build process started.
```
keys: (inspect command line)
```after
-Xmx1024m. With user-local VM options "-Xmx2g" -> -Xmx2g and shared heap field is disabled.
```

### BLD-MAKE-050 case 1: Gradle delegated
settings: Gradle build and run using=Gradle
language: plain
```before
Gradle module 'app'.
```
keys: Ctrl+F9
```after
Gradle runs "classes"/"testClasses"-style tasks (see BLD-GRADLE-031); Build tab "Build app".
```

### BLD-MAKE-050 case 2: Maven not delegated by default
settings: default
language: plain
```before
Maven module.
```
keys: Ctrl+F9
```after
IDE builder is used (Maven delegation is off by default).
```

### BLD-MAKE-051 case 1: serialization
settings: Gradle
language: plain
```before
Build requested twice quickly.
```
keys: Ctrl+F9, Ctrl+F9
```after
Second build starts after the first finished (one Gradle build at a time).
```

### BLD-MAKE-053 case 1: before task fails
settings: Gradle task "prepare" assigned "Execute Before Build"
language: plain
```before
Task prepare fails.
```
keys: Ctrl+F9
```after
Build aborted, message: Before build triggering task failed
```

### BLD-MAKE-052 case 1: task mapping reference
settings: Gradle delegation on
language: plain
```before
Module app.main of a Gradle project.
```
keys: Ctrl+F9
```after
Gradle task list equals the one specified by BLD-GRADLE-031 (':classes'); for Maven delegation BLD-MAVEN-031.
```

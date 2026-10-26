# Archipel IDE capture notes (gap-analysis evidence)

Captured 2026-10-10 on Linux under Xvfb (`DISPLAY=:96`, 1600x1000x24, openbox). The window was
resized with `xdotool windowsize <id> 1600 1000 windowmove <id> 0 0`. Screenshots:
`import -window root`, then `convert -strip -colors 256` (the quantization makes some hover
highlights look dithered; that is a capture artifact, not a rendering bug).

## Binaries

| Build | Path | Commit | Version |
|---|---|---|---|
| main | `/home/user/archipel-ide/target/release/archipel-ide` | `d1eb921` (Merge PR #36, new-project wizard) | 0.0.1 |
| PR #38 | `/home/user/archipel-ide-pr38/target/release/archipel-ide` | `3d5fb56` feat(terminal): interactive ANSI grid with direct PTY input (on top of `d1eb921`) | 0.0.1 |

Environment: `ARCHIPEL_NO_UPDATE_CHECK=1`. wgpu failed every time with
`Failed to create surface for any enabled backend`, and the app fell back to OpenGL on its own
(stderr: `archipel-ide: wgpu unavailable ...; falling back to OpenGL`), so `ARCHIPEL_RENDERER` was not
needed. The first launch **panicked** because `libxkbcommon-x11.so` was missing (see Bugs). I
installed `libxkbcommon-x11-0` with apt to continue.

Launch: `archipel-ide <project-dir> [file]`, or with no argument for the welcome screen.

Shared-sample caveat: the Android Studio (`:98`) and IntelliJ (`:97`) agents were editing the
same sample repositories during this session. They changed `MainActivity.kt`, `app/build.gradle.kts`
and `App.java` and reverted them again. Some Archipel IDE observations below (stale editor, stale
VCS state) come from those outside edits. That is how the external-change bugs were found. I
reverted my only on-disk edit (a trailing comment in `java-gradle/.../App.java`) with
`git checkout` straight after scene 12. The remaining diffs in the samples belong to the other
agents. No `.archipel/` workspace files were written: the app was always stopped with SIGTERM.

## Keymap used (Linux, from `crates/archipel-ide-ui/src/lib.rs`)

Double Shift = Search Everywhere; Ctrl+Shift+A = Find Action; Ctrl+Shift+N = Go to File;
Ctrl+E = Recent Files; Alt+1/0/3/4/6 = Project/Commit/Find/Build/Problems; Alt+F12 or Ctrl+` =
Terminal; Ctrl+Shift+F = Find in Files; Ctrl+F9 = Build; Shift+F10 = Run; Ctrl+F2 = Stop;
Ctrl+Space = completion; Ctrl+K = Commit; Ctrl+Shift+F12 = hide all tool windows; Ctrl+F4 = close tab.
**Not bound:** Ctrl+Alt+S (Settings), Ctrl+F12 (File Structure), Alt+7 (Structure), Alt+9 (Git),
Shift+F9 (Debug) and Ctrl+D in the commit view (diff), going by the key handler in `lib.rs`.

## Global scenes (`docs/parity/evidence/archipel-ide/`)

### 01 Welcome: `01-welcome.png` (+ `01b-open-project-dialog.png`)
Steps: launch with no arguments, then maximize.
- The main toolbar keeps its project-widget slot: avatar "NP" and "No project ▾". The Run/Build
  icons on the right are greyed out, next to Search and custom window controls (min/max/close).
- The centred column shows the "Archipel IDE" title, "Welcome to Archipel IDE", the subtitle "Open a
  project folder to start working.", the buttons **New Project...** and **Open Project...**, and the
  hint "Open Project: Ctrl+O".
- There is no recent-projects list, left nav (Projects/Customize/Plugins/Learn), search box,
  "Get from VCS" or clone button. There is no tool window stripe. The status bar reads "No project".
- `01b`: Open Project... opens a modal **text field** "Project directory" with the hint "Enter the
  full path to an existing folder." and the buttons Open (disabled until text is entered) and
  Cancel. There is no native folder picker, path completion or recent list. Typing a path and
  pressing Enter opens it, and that works.

### 02 New Project wizard (template list): `02-new-project.png`
Steps: Welcome → New Project..., then click the Template combo.
- The wizard is a single small modal dialog, not a two-pane wizard. There is no generator list
  on the left, and the templates sit in a combo box: Java / Gradle / Kotlin DSL (default),
  Java / Gradle / Groovy DSL, Rust application, Rust library, Empty project.
- There are no Android, Kotlin, Maven, Bazel or Compose templates, and no template preview.

### 03 New Project details: `03-new-project-details.png`
- The fields are Name (`untitled`), Location (`/root`, plain text and no Browse button),
  Template (combo) and Group / package (`org.example`).
- Below them are the derived "Project folder: /root/untitled" and the warning "Requires JDK 21 and
  Gradle on PATH. Includes a sample application.", then the buttons Create and Cancel.
- It has no JDK/SDK selector, no language or build-system toggle buttons, no "Create Git
  repository" checkbox, no "Add sample code" option, no Advanced settings and no artifact or
  version fields.

## Android: `/home/user/samples/MyApplication` (`android/`)

Opened with File/Welcome → Open Project... and the typed path. The toolbar shows "MY
MyApplication ▾", "master ▾", the run widget "Gradle Run ▾", ▶, a hammer, and search.

| # | File | Status |
|---|---|---|
| 04 | `04-editor-project-tree.png` | present |
| 05 | `05-search-everywhere.png` | present |
| 06 | `06-find-action.png` | present |
| 07 | `07-main-menu.png`, `07b-all-menus.png` | present |
| 08 | — | **absent**. Ctrl+Alt+S does nothing. Find Action "settings" → "Nothing found". No menu entry |
| 09 | `09-completion-absent.png` | **absent** for Kotlin. Ctrl+Space shows only the status text "command is disabled: language.complete" |
| 10 | `10-find-in-files.png` | present |
| 11 | `11-terminal.png`, `pr38-11-terminal.png`, `pr38-11b-terminal-less-altscreen.png` | present |
| 12 | `12-commit.png` | present (the modified file came from the Android Studio agent's edit) |
| 13 | `13-diff-absent.png` | **absent**. Double-clicking a changed file opens it in the editor. No diff viewer |
| 14 | `14-git-log-absent.png` | **absent**. The VCS widget popup only has "Current branch: master", Commit..., Refresh Status |
| 15 | `15-run-config.png` | present (minimal) |
| 16 | `16-build-output.png`, `16b-...` | present |
| 17 | `17-run-output.png`, `17b-...` | present (runs a non-existent task) |
| 18 | — | **absent**. There is no test runner. Find Action "test" offers no test action |
| 19 | `19-project-context-menu.png`, `19b-...` | present |
| 20 | `20-problems.png` | Problems window present, but it never reports anything for Kotlin |
| 21 | — | **absent**. No Gradle tool window (not in the View menu, stripe or keymap) |
| 22 | — | **absent**. Ctrl+F12 and Alt+7 do nothing (Alt+7 types "7", see Bugs) |
| 23 | — | **absent**. No debugger, breakpoints or Debug action |
| 24 | — | **absent**. No Plugins page. The only plugin item is the action "Refresh Plugins" |

Android-specific scenes: **a01** Device Manager, **a02** Logcat, **a03** Running Devices,
**a05** SDK Manager, **a06** Gradle Sync, **a08** Resource Manager, **a09** App Inspection,
**a10** Profiler and **a11** Build Variants are all **absent**. None of them has a menu entry, a
Find Action hit or a tool window. **a04** Layout editor is **absent**: `activity_main.xml` opens as
plain unhighlighted text with no Design/Split modes (`a04-layout-editor-absent.png`). **a07**
Merged manifest is **absent**: `AndroidManifest.xml` opens as plain text with no "Merged
Manifest" tab (`a07-manifest-absent.png`).

### 04 Editor + project tree
Steps: expand app/src/main/java/com/example/myapplication with the chevrons. A single click
selects. A **double-click** opens the file. I dragged the tree/editor splitter from x≈320 to
x≈400, which needs a slow drag with intermediate mouse moves.
- The tree is a plain directory view with no Android view: `.gradle`, `.idea`, `build` (and later
  `.kotlin`) show as normal folders. Excluded/generated folders are not dimmed, there are no
  module or source-root icons, and packages are not compacted: `com`, `example`, `myapplication`
  are separate nodes. Every file uses one generic page icon, with no Kotlin, Gradle or XML icons.
- VCS colouring: the folder `gradle` (which holds an untracked file) is blue. After the outside
  edit, `app/.../MainActivity.kt` and its parents turned blue. Untracked files are red only in the
  commit view.
- Before widening, the tree clipped the label "MainActivity.l…" with no ellipsis, tooltip or
  horizontal scroll. The root row shows the truncated path "/home/user/samples".
- The editor has the tab "MainActivity.kt ×", line numbers and indent guides. There is **no
  syntax highlighting** for Kotlin, Java or XML (all text is one colour). There are no gutter run
  icons, inlay hints, breadcrumbs above the editor or editor-tab actions.
- The status bar shows breadcrumbs (MyApplication › app › … › MainActivity.kt), the last message,
  caret `1:1`, `LF`, `UTF-8` and `4 spaces`. There is no file-type or language widget.
- A thin horizontal scrollbar thumb always shows at the bottom of the editor (y≈970), even when
  the content fits.
- Opening a file from Find/Search does not select it in the tree (no autoscroll-from-source).

### 05 Search Everywhere
Steps: tap Shift twice, then type `main`.
- The popup is centred over the editor and has three tabs: All / Files / Actions. There are no
  Classes, Symbols or Text tabs, no filter or scope button and no preview.
- Results for `main`: MainActivity.kt (app/src/main/java/com/example/myapplication),
  activity_main.xml (app/src/main/res/layout), "Terminal Tool Window" (Window, Alt+F12, a
  subsequence match) and the disabled "Dismiss Update Notification" (Help). The footer reads
  "↑↓ to navigate · Enter to run · Esc to close".
- In java-gradle the same query returns the Terminal action and two
  `build/tmp/.../previous-compilation-data.bin` files. `App.java`, which declares `main()`, is not
  found, because there is no symbol index and build output is not excluded.

### 06 Find Action
Steps: Ctrl+Shift+A, then type `run`.
- The popup opens on the Actions tab. Results: Run (Shift+F10), Stop Running Task (disabled),
  Refresh Plugins, Undo (disabled, Ctrl+Z), Unindent Line or Selection (Shift+⇥).
- Disabled actions are mixed into the results, and for `test` a disabled item is ranked and
  selected first. There are no option toggles (IntelliJ shows on/off switches inline), no
  "Include disabled actions" checkbox and no plugin or setting hits.

### 07 Main menu
Steps: click the hamburger at the top left, then hover File.
- Top level: File, Edit, View, Navigate, Code, Build, Run, Git, Help, then Find Action...
  (Ctrl+Shift+A) and Exit.
- File holds only New Project..., Open Project... (Ctrl+O) and Refresh Plugins. It has no
  Settings, Recent Projects, Close Project, Save All, Invalidate Caches or Project Structure.
- `07b-all-menus.png` shows every submenu:
  - Edit is alphabetical rather than grouped: Close Tab, Comment with Line Comment, Copy, Cut,
    Delete Line, Duplicate Line or Selection, Indent Selection, Move Line Down/Up, Redo, Save,
    Save All, Select All, Undo, Unindent. Save and Save All **both show Ctrl+S**.
  - View: Build/Commit/Find/Problems/Project/Terminal Tool Window, Hide All Tool Windows, Save
    Workspace Layout.
  - Navigate: only "Cancel Search" (disabled). Go to File, Recent Files and Go to Line are not in
    the menu.
  - **Code: empty submenu** (a blank popup opens).
  - Build: Build Project (Ctrl+F9).
  - Run: Stop Running Task, Run (Shift+F10).
  - Git: Refresh Git Status.
  - Help: About Archipel IDE, Check for Updates..., Dismiss Update Notification, Update and Restart,
    Save Session for Restart.
- Esc closes the menu.

### 10 Find in Files
Steps: Ctrl+Shift+F opens the "Find in Files" bottom tool window with focus in the query field.
Type `greet` and press Enter.
- The view is a single field plus "5 matches", with rows of the form `<line text>` ... `File.kt
  <line>` on the right. The status bar reads "Found 5 results in 30 files".
- There are no Match case, Words or Regex toggles, no file mask, no scope (Project/Module/
  Directory), no preview pane, no grouping by file and no "Open in Find Window".
- The **match is not highlighted** in the result rows. A line with two hits (greeting, greet) is
  listed twice with identical text.
- Clicking a result opens the file with the caret at 1:1, not at the match (a known limitation).
- In this scene the results showed the on-disk content, including the outside agent's new line
  12 `textSize = 24f`, while the open editor still showed the old buffer (see Bugs).

### 11 Terminal (main: `11-terminal.png`, PR #38: `pr38-11-terminal.png`)
Steps: Alt+F12, type `ls --color` and Enter, then `git log --oneline` and Enter.
- Main build: the bottom "Terminal" tool window is a transcript plus a separate input line at the
  bottom ("❯ Type a command and press Enter"). Output is monochrome: `ls --color` shows **no
  colours** and git decorations are uncoloured. There are no tabs, new-session button or Clear.
- **PR #38**: the input line is gone and the caret sits in the shell prompt grid (block cursor).
  `ls --color` shows blue directories and green executables. `git log` shows a yellow hash and
  cyan/green `HEAD -> master`. The header gains Restart (↻) and **Clear** buttons. With the PR,
  the terminal panel opens shorter than in main (top edge y≈790 against y≈715).
- PR #38 interaction checks (`pr38-11b-terminal-less-altscreen.png`):
  - `sleep 30` then Ctrl+C prints `^C` and returns to the prompt.
  - Up arrow recalls history. Typing continues the recalled line.
  - `less /etc/services` uses the alternate screen (top pane of the composite) and `q` restores
    the previous screen (bottom pane).
- Still missing in PR #38: terminal tabs, multiple sessions, split, and the session name in the
  tab.

### 12 Commit
Steps: Alt+0. The Commit window replaces Project in the left slot.
- Tree: ☑ Changes (1 file) → ☑ MainActivity.kt with its path in grey. ☐ Unversioned Files (1
  file) → gradle-daemon-jvm.properties in red. Each row has a checkbox.
- At the bottom: a "Commit Message" box (the label sits inside the box), a **Commit** button and
  "1 file selected". The header has Refresh (↻) and Hide.
- Missing: Amend, Commit and Push, the Git options gear, the toolbar row (Rollback, Show Diff,
  Group By, Expand/Collapse), changelists, a diff preview, and gutter change markers in the editor.
- The status bar reads "2 changes on master" (in java-gradle: "1 changes on master", wrong
  plural).
- Right-clicking a changed file opens no context menu.

### 13 Diff: absent
Double-clicking `MainActivity.kt` in the commit tree opens the plain editor tab. There is no
Show Diff action anywhere (menus, Find Action, Ctrl+D).

### 14 Git log: absent
There is no Git tool window or Alt+9. The branch widget popup lists only "Current branch:
master", Commit... (Ctrl+K) and Refresh Status, with no branch list, checkout, new branch,
push/pull or log.

### 15 Run configuration
Steps: click "Gradle Run ▾" in the toolbar.
- The popup has two plain items, Run and Build Project. There is no configuration list, no
  Edit Configurations..., no Debug/Profile/Coverage and no "Current File".
- The widget name is detected from the build files (Gradle → "Gradle Run"). For Bazel the name is
  empty but ▶ and the hammer still show as enabled.

### 16 Build output
Steps: Ctrl+F9 runs `./gradlew build` in the "Gradle Build" bottom tool window.
- The header reads "⟳ Running... ./gradlew build", with a red stop square in the header and in
  the toolbar (▶ and the hammer greyed out). The stripe's hammer icon shows a green "running" dot.
- The output is raw console text with no tree of tasks, durations, warnings or errors, no
  clickable file links and no filter.
- Android result: "Failed with exit code 1" in red after about 70 s. The cause (in `16b`, after
  scrolling) was a dependency download that got HTTP 429 from repo.maven.apache.org, an
  environment problem.
- The output shows **several blank lines** where Gradle progress lines were. The proxy's
  `JAVA_TOOL_OPTIONS` line is echoed in full.

### 17 Run output
Steps: Shift+F10 runs `./gradlew run` in the same bottom tool window, now titled "Gradle Run". The
stripe has no separate Run window.
- **Android: the Gradle `run` task does not exist** → "Task 'run' not found in root project 'My
  Application'", exit code 1 (`17b`). There is no Android run path (assembleDebug, install,
  launch).
- The first visible output line is cut in half under the status line (the scroll area clips the
  text, visible at y≈780).

### 19 Project context menu
Steps: right-click MainActivity.kt in the tree, then hover New.
- Items: New ▸ (File, Directory), Open, a separator, Rename... (Shift+F6), Delete... (Delete).
- Missing: New Kotlin/Java class or Android resource, Copy Path/Reference, Paste, Find Usages,
  Refactor, Reformat, Open In (Terminal, Files), Local History, Git submenu, Run/Debug, Compare
  With and Mark Directory As.
- **Escape does not close this menu** (`19b`). After Esc and Alt+6 the menu stayed open over the
  tree while the Problems window opened. It closes only on a click outside.

### 20 Problems
Steps: Alt+6.
- The bottom "Problems" window lists the current file: "MainActivity.kt 0 problems" / "No
  problems found". No typo was introduced in Android, to avoid touching the file the Android
  Studio agent was editing.
- java-gradle: typing `intt x = ;` into App.java still shows "App.java 0 problems". There are no
  red squiggles and no error stripe, because diagnostics exist only for Rust. There are no
  Project Errors or Server-side tabs.

## Java/Gradle: `/home/user/samples/java-gradle` (`java-gradle/`)

Launched with `archipel-ide /home/user/samples/java-gradle src/main/java/com/example/App.java`.
The file opened in the editor, but the tree was **not** expanded or revealed to it. I expanded it
by hand.

| # | File | Notes |
|---|---|---|
| 04 | `04-editor-project-tree.png` | same look as Android: no Java highlighting, no module or source-root markers, `build`/`.gradle` not excluded |
| 05 | `05-search-everywhere.png` | `main` finds no App.java or `main()`, and does find build/tmp `.bin` files |
| 06 | `06-find-action.png` | same results as Android |
| 07 | `07-main-menu.png` | same menu as Android |
| 08 | — | absent (as above) |
| 09 | `09-completion-absent.png` | typed `app.` then Ctrl+Space → "command is disabled: language.complete" with no popup |
| 10 | `10-find-in-files.png` | `greeting` → 3 matches (App.java 6, App.java 14, AppTest.java 10). Line 14 matched the on-disk file (with the IntelliJ agent's added comment), not the editor buffer, which had the println on line 13 |
| 11 | `11-terminal.png`, `pr38-11-terminal.png` | as Android: main monochrome, PR #38 coloured |
| 12 | `12-commit.png`, `12b-commit-stale-no-autorefresh.png` | `12b`: App.java was modified on disk by another process, yet the Commit window showed "Changes 0 files / No changes" until ↻ was clicked. `12`: my edit (`// archipel-ide-capture`, Ctrl+S) shown after ↻, then reverted |
| 13 | — | absent (as above) |
| 14 | — | absent |
| 15 | `15-run-config.png` | Run / Build Project |
| 16 | `16-build-output.png` | `./gradlew build` → "Finished successfully" (green), BUILD SUCCESSFUL in 4s. Tests ran inside the build, but no test results are shown |
| 17 | `17-run-output.png` | `./gradlew run` → "Hello, Gradle!" / "Hello, Java!", Finished successfully. The top output line is clipped under the header |
| 18 | — | absent. There is no test tree, and Find Action "test" offers only "Duplicate Line", "Unindent" and the disabled "Update and Restart" (selected first) |
| 19 | `19-project-context-menu.png` | on the `test` folder: New ▸, Rename..., Delete... (no Open for folders) |
| 20 | `20-problems.png` | syntax error `intt x = ;` with no diagnostics (reverted with Ctrl+Z, file unchanged) |
| 21 | — | absent (no Gradle tool window) |
| 22 | — | absent (Ctrl+F12 does nothing) |
| 23 | — | absent |
| 24 | — | absent |

## Bazel: `/home/user/samples/bazel-java` (`bazel/`)

| # | File | Notes |
|---|---|---|
| 04 | `04-editor-project-tree.png`, `04b-build-bazel-editor.png` | The tree shows `bazel-bazel-java`, `bazel-bin`, `bazel-out` and `bazel-testlogs` as **plain files**, though they are directory symlinks (and gitignored). There are no BUILD-file icons or target nodes. `BUILD.bazel` opens as unhighlighted text with no Starlark support. `MODULE.bazel.lock` has a different (lock-badged) file icon |
| 16 | `16-build-output-absent.png` | **absent**. Ctrl+F9 → status bar "command failed: no supported build system found in /home/user/samples..." (truncated). The Build window (Alt+4) says "Nothing has run yet". The runner only detects Cargo, Gradle, Maven, npm and Go |
| 17 | `17-run-output-absent.png` | **absent**. ▶ gives the same "no supported build system" status message. There is no `bazel run //:app` |
| 21 | `21-bazel-tool-window-absent.png` | **absent**. There is no Bazel tool window, target list or sync. Find Action "bazel" → "Nothing found" |

PR #38 changes only the terminal (`crates/archipel-ide-ui/src/terminal_ui.rs`, `toolwindows.rs`). In
the other scenes the PR build looked the same as main, so no further PR scenes were captured.

## Performance and rendering

- Startup to first frame took about 2 s with the OpenGL fallback. Typing, popups and tree
  expansion were instant at 1600x1000.
- Find in Files over about 30 files was effectively instant.
- Android `./gradlew build` took about 70 s and failed on the network. java-gradle build took
  4 s.
- No crashes after the libxkbcommon fix.
- The modified-dot on an editor tab overlaps the last letter of the filename ("MainActivity.kt"
  with the dot over the "t", visible in `09-completion-absent.png`).
- A horizontal scrollbar thumb always shows at the bottom of the editor.
- The first output line in the Build/Run window is clipped under the status line.

## Bugs found

1. **Panic on startup without `libxkbcommon-x11`.** Repro: on a minimal Ubuntu 24.04 container
   without `libxkbcommon-x11-0`, run `DISPLAY=:96 archipel-ide`. Result: `thread 'main' panicked
   at xkbcommon-dl-0.4.2/src/x11.rs:59:28: Library libxkbcommon-x11.so could not be loaded.` with
   a backtrace. There is no actionable error message, and the README does not list the
   dependency.
2. **Editor does not reload files changed on disk.** Repro: open `MainActivity.kt` (or
   `App.java`), then change the file from another process (`echo // x >> file`, or
   `git checkout file`). The editor keeps the old buffer with no "file changed" prompt, and Find
   in Files then reports lines that do not match the editor (`10-find-in-files.png`,
   `bug-editor-stale-after-external-revert.png`). Because the editor autosaves on focus loss, a
   later edit in that tab can **silently overwrite** the outside change. I avoided triggering
   this.
3. **VCS state is not refreshed automatically.** Repro: open java-gradle, then modify App.java
   outside Archipel IDE and press Alt+0. Result: "No changes" until ↻ is clicked
   (`12b-commit-stale-no-autorefresh.png`). Likewise, after `git checkout` the tree kept
   `App.java` and its parent folders blue (seen in `java-gradle/16-build-output.png`).
4. **Unbound Alt+digit chords type the digit into the editor.** Repro: focus the editor and press
   Alt+7. Result: "7" is inserted and the tab becomes dirty (`android/bug-alt7-types-digit.png`).
   Bound chords (Alt+1/0/3/4/6) are filtered, unbound ones are not.
5. **Project-tree context menu ignores Escape.** Repro: right-click a file in the tree, then press
   Esc (twice). The menu stays open, and other shortcuts (Alt+6) still run underneath it
   (`android/19b-context-menu-survives-escape.png`).
6. **Code menu is an empty submenu.** Repro: hamburger → hover Code. A blank popup opens
   (`android/07b-all-menus.png`).
7. **Save and Save All show the same shortcut, Ctrl+S, in the Edit menu.** `keys.rs` maps macOS
   ⌥⌘S (Save All) to Ctrl+S, which matches the reference Linux keymap, but the separate Save
   action also shows Ctrl+S, so two menu items claim one shortcut.
8. **Android "Run" invokes the non-existent `./gradlew run`.** Repro: open MyApplication and press
   Shift+F10. Result: "Task 'run' not found in root project 'My Application'"
   (`android/17b-run-output-task-not-found.png`). The run widget still advertises "Gradle Run".
9. **Bazel: Run/Build look enabled but only fail in the status bar.** Repro: open bazel-java and
   press Ctrl+F9 or click ▶. The only feedback is a truncated status-bar message. Nothing appears
   in the Build window and the toolbar buttons are not disabled.
10. **Directory symlinks are shown as files and cannot be opened.** Repro: in bazel-java,
    double-click `bazel-bin`. Status: "command failed: path is outside the project: ..." (the
    folders render with a file icon and no chevron, see
    `bazel/bug-symlinked-dirs-shown-as-files.png`).
11. **Find in Files duplicates rows** for a line with more than one occurrence and does not
    highlight the match. Navigating to a result places the caret at 1:1.
12. **The first output line in the Build/Run tool window is clipped** under the header/status
    line (`android/17-run-output.png`, `java-gradle/17-run-output.png`). Gradle progress output
    leaves blank runs of lines.
13. **Opening a file via the CLI argument does not reveal it in the project tree.** Neither does
    opening via Find in Files or Go to File (the tree selection stays on the previous file).
14. **Undo moves the caret to an unrelated position.** After a coalesced undo of an inserted
    line, the caret moved to 14:7 (Android) or 12:10 (Java) instead of where the edit was.
15. Minor: the status bar says "1 changes on master" (wrong plural). The tree clips long names
    with no ellipsis or tooltip. The modified-dot overlaps the tab title.

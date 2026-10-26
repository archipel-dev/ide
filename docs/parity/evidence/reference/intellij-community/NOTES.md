# IntelliJ IDEA Community reference captures (Java/Gradle + Bazel)

Comparison evidence for Archipel IDE gap analysis. Screenshots are observational references only;
no icons/artwork may be extracted from them (clean-room rule).

## Environment

- Product: IntelliJ IDEA Community Edition **2025.2.6.3**, build **IC-252.28539.97**
  (`/home/user/tools/idea-IC-252.28539.97`), New UI, default **Dark** theme, editor scheme "Dark Theme default".
- Bundled runtime JBR 21; project JDK auto-detected: `Java 21.0.12 /usr/lib/jvm/java-1.21.0-openjdk-amd64`.
- Bazel plugin: **org.jetbrains.bazel 2025.2.12** ("Bazel by JetBrains", `since-build=252.23892 until-build=252.*`),
  installed headless with `bin/idea installPlugins org.jetbrains.bazel` before first launch. It uses
  Bazel BSP 3.2.0 under the hood (`--tool_tag=bazelbsp:3.2.0` in sync output).
- Display: Xvfb `:97`, 1600x1000x24, openbox. IDE window sized `1600x980` + 20 px openbox title bar,
  so every capture is 1600x1000 and the IDE status bar is visible.
- Launch: `IDEA_VM_OPTIONS=<file>` with `-Xmx3g -Didea.trust.all.projects=true
  -Djb.consents.confirmation.enabled=false "-Djb.privacy.policy.text=<!--999.999-->"
  -Dide.show.tips.on.startup.default.value=false -Didea.initially.ask.config=never`,
  `XDG_RUNTIME_DIR=/tmp/xdg-ic`. With these flags **no** data-sharing / user-agreement dialog appeared.
- First-run noise: a balloon "The use of Java options environment variables detected ... JAVA_TOOL_OPTIONS"
  with a `Don't show again` button, and "IntelliJ IDEA 2025.3 available / Update..." (both dismissed).
- Samples: `/home/user/samples/java-gradle` (Gradle 9.8.1 wrapper, Kotlin DSL, JUnit 5) and
  `/home/user/samples/bazel-java` (Bazel 8.4.2 bzlmod, `java_library :greeter` + `java_binary :app`).
- Capture: `DISPLAY=:97 import -window root f.png; convert f.png -strip -define png:compression-level=9 -colors 256 f.png`.
- All sample edits reverted (`git checkout`), IDE-created `.idea/` and `.bazelbsp/` removed afterwards.

Known environment issue: the first Gradle sync failed with
`Could not resolve org.junit.jupiter:junit-jupiter:5.11.4` because Maven Central returned HTTP 429 through the
proxy. The Sync tab showed a red error node ("java-gradle: finished At ... with 1 error") with the child
"Could not resolve org.junit.jupiter:junit-jupiter:5.11.4 for java-gradle:test" and the right pane printed
"Possible solution: - Declare repository providing the artifact, see the documentation at https://docs.gradle.org/...".
After warming the cache with `./gradlew test` from the shell (succeeded on the 4th retry) the re-sync
(circular-arrows button in the Sync tab's left toolbar) succeeded in ~2 s.

## Scenes — java-gradle (`java-gradle/`)

### 01-welcome.png — Welcome screen
Steps: start IDE with no project.
- Window title "Welcome to IntelliJ IDEA (ROOT)". Left column: product logo + "IntelliJ IDEA" / "2025.2.6.3",
  then nav items **Projects** (selected, blue pill), **Kotlin Notebooks**, **Customize**, **Plugins** (badge "1"
  = pending update), **Learn**; gear icon bottom-left.
- Empty state (no recent projects): heading "Welcome to IntelliJ IDEA (ROOT)", two grey lines "Create a new project
  to start from scratch." / "Open existing project from disk or version control." and three large square
  buttons **New Project** (+, focused, blue outline), **Open** (folder), **Clone Repository** (VCS icon).
- Bottom banner card "Take a quick onboarding tour" with split button "Start Tour in Java | v" and an X close.
- Notifications bell bottom-right of the right pane.

### 02-new-project.png / 03-new-project-details.png — New Project wizard
Steps: Welcome > New Project. 03: click **Gradle** in Build system, expand **Advanced Settings**.
- Single-page dialog titled "New Project" (no separate "details" page: the generator list on the left and the
  form on the right are shown together; 03 shows the same page with Gradle-specific fields).
- Left list has a search field at top, section **New Project**: Java (selected), Kotlin, Groovy, Empty Project;
  section **Generators**: Maven Archetype, JavaFX, Spring (lock icon = Ultimate-only), Multi-module Project;
  link "More via plugins..." at bottom.
- Form: Name `untitled` (focused, text selected), Location `~/IdeaProjects` with folder button and hint
  "Project will be created in: ~/IdeaProjects/untitled", checkbox "Create Git repository",
  **Build system:** segmented control `IntelliJ | Maven | Gradle | Bazel` (Bazel added by the plugin),
  **JDK:** combo `Java 21.0.12 /usr/lib/jvm/java-1.21.0-openjdk-amd64`, checkbox "Add sample code" (checked),
  collapsible "Advanced Settings".
- With Gradle: extra row **Gradle DSL:** `Kotlin | Groovy`; Advanced Settings shows "Gradle distribution:" `Wrapper`,
  "Gradle version:" `8.14` (disabled) + "Auto-select" (checked), "Use these settings for future projects" (checked),
  "GroupId:" `org.example`, "ArtifactId:" `untitled` (with ? help icons).
- Footer: `?` help, primary **Create**, **Cancel**.
- `bazel/b06-new-project-bazel.png`: Bazel build system selected — JDK, sample code and advanced settings all
  disappear; only Name/Location/Git/Build system remain.

### 04-editor-project-tree.png — Project open, tree + editor
Steps: `bin/idea /home/user/samples/java-gradle` (opens without trust dialog). Expand root > src > main > java >
com.example by selecting a node and pressing Right repeatedly; select **App**; App.java opened via Ctrl+Shift+N.
- Main toolbar: hamburger, project widget with coloured "JG" initials badge + "java-gradle v", VCS widget
  "master v", right: run widget "Current File v", Run (green triangle), Debug (bug), kebab, AI icon, Code With Me,
  Search (magnifier), Settings gear (with update dot).
- Left stripe: Project, Commit, Structure, "..." (more); bottom group: Build (hammer), Services/Run, Terminal,
  Problems, Git. Right stripe: Notifications, AI Chat, **Gradle** (elephant).
- Project tree header "Project v" + icons: New (+), Locate/Select opened file (crosshair), Expand all,
  Collapse all, kebab, hide (–). Root row `java-gradle  /home/user/samples/java-gradle` (path grey).
  Excluded/ignored dirs `.gradle`, `build` shown with **orange folder text and brownish row background**;
  `.idea` normal. Source roots: `main`/`test` bold with module marker, `java` blue (main) / green (test).
  Test sources subtree has a **green row tint**. Classes shown without `.java` extension (App, AppTest).
  `External Libraries`, `Scratches and Consoles` at the bottom.
- Editor: tab "App.java" with class icon; gutter run triangles on `class App` (line 5) and `main` (line 10);
  inline "code vision" author hint `👤 sample` after class/method declarations; one-line method body
  `greeting(...) { return "Hello, " + name + "!"; }` is shown **folded** with boxed braces (later shows
  "2 usages" hint). Green check (no problems) top-right of editor. Indent guides visible.
- Status bar: breadcrumb `java-gradle > src > main > java > com > example > App`, right: `12:30  LF  UTF-8
  4 spaces` and a lock/readonly toggle. Window title `java-gradle – App.java [java-gradle.main]` (module in brackets).
- Clicking the tree-header "Locate" icon via xdotool did not register; keyboard tree navigation worked.

### 05-search-everywhere.png — Search Everywhere
Steps: focus editor, press Shift twice quickly, type `main`.
- Centered popup with tabs **All | Classes | Files | Symbols | Actions | Text**, checkbox "Include non-project
  items", icons for preview / filter / open-in-window. Search field with hint "Type / to see commands" on right.
- Results mix: first `main(String[]) of com.example.App` (match highlighted orange) with module `java-gradle.main`
  on the right; then `java-gradle.main src/main` (module dir); then text-occurrence rows (`gradlew 60`,
  `build.gradle.kts 5`, `App.java 10` ...); then actions/settings (Maintenance, Main Menu, Main Toolbar, Plugins
  "Welcome Screen", Build Project `Ctrl+F9` "Build", Maven Settings, Move Caret to Matching Brace `Ctrl+Shift+M`
  "Navigate | Navigate in File", `View | Appearance: Toolbar` with an inline **ON** toggle, Update Loaded Classes On
  Debugger Stop with **OFF** toggle, Build Module 'java-gradle.main').
- Footer shows path of selected item `com/example/App.java` and link "Open In Right Split".

### 06-find-action.png — Find Action
Steps: Ctrl+Shift+A, type `run`.
- Same popup, **Actions** tab active; checkbox "Include disabled actions". Rows show icon, name, inline shortcut
  (e.g. `Run Alt+4`, `Run 'App.java' Shift+F10`, `Run... Alt+Shift+F10`, `Run Inspection by Name...
  Ctrl+Alt+Shift+I`), and group on the right (`Run`, `Code | Analyze Code`, `File`).
- Footer "Assign Shortcut Alt+Enter".

### 07-main-menu.png — Main menu
Steps: click hamburger (top-left).
- Hamburger turns into an inline menu bar `File Edit View Navigate Code Refactor Build Run Tools Git Window Help`
  with File expanded. File items (with separators): New >, Open..., Recent Projects >, Close Project |
  Settings... `Ctrl+Alt+S`, Project Structure... `Ctrl+Alt+Shift+S`, File Properties > | Local History > |
  Save All `Ctrl+S`, Reload All from Disk `Ctrl+Alt+Y`, Repair IDE, Invalidate Caches... | Manage IDE Settings >,
  New Projects Setup >, Save File as Template... | Export >, Print... | Power Save Mode | Exit.
  Mnemonic underlines are shown.

### 08-settings.png — Settings
Steps: Ctrl+Alt+S.
- Modal "Settings" dialog; left tree with search box: Appearance & Behavior (expanded: Appearance, Menus and
  Toolbars, System Settings >, File Colors, Scopes, Notifications, Data Editor and Viewer, Quick Lists,
  Required Plugins, Trusted Locations, Path Variables, Presentation Assistant), Keymap, Editor, Plugins (badge 1),
  Version Control, Build, Execution, Deployment, Languages & Frameworks, Tools, Backup and Sync, Advanced Settings.
  Project-level pages carry a small "project" icon on the right.
- Right pane breadcrumb "Appearance & Behavior > Appearance", back/forward arrows. Theme `Dark`, Editor color
  scheme `Dark Theme default` + gear; Accessibility: Zoom `100%` with hint "Change with Ctrl+Alt+Shift+= or
  Ctrl+Alt+Shift+Minus...", Use custom font (Inter, 13), Support screen readers "Requires restart", contrast
  scrollbars, red-green deficiency; UI Options two-column checkboxes (Compact mode, Smooth scrolling, Use project
  colors in main toolbar, ...), "Main menu: Hide under Hamburger Button".
- Footer `?`, **OK** (primary), Cancel, Apply (disabled until a change).

### 09-completion.png — Code completion
Steps: in main, Ctrl+G `11`, End, Enter, type `app.` (edit reverted afterwards with Ctrl+Y).
- Popup opens automatically after `.`; first item `★ greeting(String name)` (star = ML/most relevant) with
  return type `String` right-aligned; then `clone()`, `equals(Object obj)`, `hashCode()`, `getClass()` (with
  up-arrow), `toString()` (down-arrow), `notify()`, `notifyAll()`, `wait(...)` overloads, each with a method icon
  and grey parameter list. Footer tip "Ctrl+Down and Ctrl+Up will move caret down and up in the editor Next Tip"
  and a kebab. Editor shows "Paused..." indicator top-right (highlighting paused while typing).
- Modified line has a green bar in the gutter; author hint gains `*` (uncommitted change).

### 10-find-in-files.png — Find in Files
Steps: Ctrl+Shift+F, type `greeting`.
- Floating popup "Find in Files  3 matches in 2 files"; "File mask:" checkbox + `*.java` combo; filter + pin icons.
  Query row with clear (x), newline/regex-in-line toggles **Cc** (match case), **W** (words), **.*** (regex).
  Scope tabs **In Project | Module | Directory | Scope**.
- Results list: matched line with highlighted term and `File.java N` on right; test file row tinted green.
- Lower half: live preview editor of the selected file ("App.java src/main/java/com/example") with the match
  highlighted. Footer checkbox "Open results in new tab", hint `Ctrl+Enter`, button **Open in Find Window**.
- Results update while typing (no Enter needed).

### 11-terminal.png — Terminal
Steps: Alt+F12, type `ls --color` Enter, `git log --oneline` Enter.
- Bottom tool window "Terminal" with tab "Local x", `+` (new tab) and `v` (shell choice). This is the reworked
  terminal: each command is a separate **block** separated by thin horizontal rules; prompt
  `root@vm:/home/user/samples/java-gradle#`; ANSI colours honoured (dirs blue, executable `gradlew` green,
  `HEAD -> master` coloured). Opens in project root.

### 12-commit.png / 13-diff.png — Commit and diff
Steps: add a comment line in App.java, Ctrl+S, Alt+0; tick App.java; double-click it for diff.
- Commit tool window replaces the Project panel on the left (non-modal). Toolbar: refresh, rollback, shelve/
  update, show diff (eye), expand/collapse. Tree "Changes 1 file" with tri-state checkboxes; file row shows
  name + grey relative dir. Below: "Amend" checkbox + history clock, "1 modified" link, "Commit Message"
  text area (placeholder), **Commit** (primary) and **Commit and Push...**, settings gear.
- Diff opens as an editor tab "Commit" (tab icon) — not a dialog. Toolbar: prev/next diff arrows, edit
  (pencil), back/forward, combos `Side-by-side viewer v`, `Do not ignore v`, `Highlight words v`, collapse
  unchanged, sync scroll, settings, help; "1 difference" counter. Left header `57836f46 src/main/java/com/example/App.java`
  (read-only lock), right header checkbox "Current version". Inserted line highlighted green across both sides
  with a ">>" apply arrow and a per-chunk checkbox (partial commit).

### 14-git-log.png — Git log
Steps: Alt+9 (after reverting the file and Ctrl+Alt+Y reload).
- Bottom tool window "Git" with tab "Log" (+ / v for more logs). Left: branches panel (HEAD (Current Branch),
  Local > master with tag icon) and vertical action icons. Center: filter bar "Text or hash" with `.*` and `Cc`,
  combos Branch / User / Date / Paths, plus sort, refresh, cherry-pick, eye, search. Row: green dot + "init",
  branch label `master`, author `sample`, "15 minutes ago". Right: changed-files tree of the selected commit
  (java-gradle 8 files > gradle 3 files > wrapper 2 files ...) and commit details ("init", `57836f46 sample <a@b> on 10/10/26 at 1:26 PM`).
- Note: externally reverting the file (git checkout) was not reflected in the editor until Ctrl+Alt+Y / focus.

### 15-run-config.png and x02-run-debug-configurations.png
Steps: run App once (creates a config), click run widget `App v`; then "Edit Configurations...".
- Run widget popup: highlighted row "App" with inline Run, Debug and kebab buttons; "Current File"; separator;
  "Edit Configurations...". Before any run the widget says **Current File** and only that + Edit Configurations exist.
- Run/Debug Configurations dialog: left tree `Application > App` with toolbar (+, –, copy, save, move to folder,
  sort), link "Edit configuration templates..."; right: Name `App`, "Store as project file" checkbox,
  "Build and run" with **Modify options v Alt+M**, rows `java 21 SDK of 'java-gradle.main' module`,
  `-cp java-gradle.main`, main class `com.example.App`, "Program arguments", hint "Press Alt for field hints",
  Working directory, Environment variables ("Separate variables with semicolon: VAR=value; VAR1=value1"),
  chip "Open run/debug tool window when started x", Code Coverage section with "Modify v" and package list
  `com.example.*`. Footer: **Run** split button, OK, Cancel, Apply.

### 16-build-output.png — Build tool window
Steps: Ctrl+F9 (Build Project) — Gradle is the default "Build and run using" delegate.
- Ctrl+F9 does not auto-open the window when nothing to do; opened via hammer stripe icon.
- Build tool window tabs: **Sync**, **Build Output** (active). Two-level layout: far-left tree
  "java-gradle: build finished / Executing post-compile tasks..." and "Build java-gradle: successful", a middle
  tree "java-gradle: build finished UP-TO-DATE 2 sec, 818 ms" with **warning children**
  "Unable to make 'kotlin.scripts.KotlinSettingsScriptTemplate.settings.gradle.kts'" etc., and console on the
  right ("Executing pre-compile tasks...", "Running 'before' tasks", "Checking sources", ...). Right toolbar:
  soft-wrap, scroll-to-end, clear.

### 17-run-output.png — Run tool window
Steps: Ctrl+Shift+F10 in App.java (run context configuration).
- Run tool window, tab `java-gradle [:com.example.App.main()]` (Gradle elephant icon: main is run through a
  generated Gradle task). Left toolbar: rerun, stop, eye (show/hide), kebab. Left tree:
  "✓ java-gradle [:com.example.App.main()]: successful At ... 3 sec, 520 ms". Console:
  `> Task :com.example.App.main()`, `Hello, Gradle!`, `Hello, Java!`, `BUILD SUCCESSFUL in 3s`, Gradle hints,
  stderr line `Picked up JAVA_TOOL_OPTIONS...` in red, "1:41:46 PM: Execution finished ':com.example.App.main()'."
- Run widget changes from "Current File" to "App" (temporary config created).

### 18-test-results.png — Test runner after Gradle `test`
Steps: Gradle tool window > Tasks > verification > double-click **test**.
- Run tool window tab `java-gradle [test]`; run widget becomes `java-gradle [test]`.
- Toolbar: rerun, rerun failed, toggle auto-test, stop, eye, **Show Passed** (✓) and **Show Ignored** (⊘) toggles,
  sort, import, expand/collapse arrow, history clock, kebab.
- **Passed tests are hidden by default**: tree initially showed only "Test Results 21ms"; enabling "Show Passed"
  reveals `Test Results > AppTest > greets` with green checks and durations (21ms).
- Right header: green "✓ 1 test passed  1 test total, 21ms" with a green progress line, then Gradle console
  (`> Task :compileJava UP-TO-DATE` ... `> Task :test`, `BUILD SUCCESSFUL in 1s`). Right toolbar: up/down
  (navigate), soft-wrap, scroll end, print, clear.

### 19-project-context-menu.png — Project tree context menu
Steps: right-click **App** in the tree.
- Items: New > | Cut `Ctrl+X`, Copy `Ctrl+C`, Copy Path/Reference..., Paste `Ctrl+V` | Find Usages `Alt+F7`,
  Analyze > | Rename... `Shift+F6`, Refactor > | Bookmarks > | Reformat Code `Ctrl+Alt+L`, Optimize Imports
  `Ctrl+Alt+O`, Delete... `Delete`, Override File Type | Run 'App.main()' `Ctrl+Shift+F10`, Debug 'App.main()',
  More Run/Debug > | Open in Right Split `Shift+Enter`, Open In > | Local History >, Git > , Repair IDE on File,
  Reload from Disk | Compare With... `Ctrl+D` | Convert Java File to Kotlin File `Ctrl+Alt+Shift+K`.

### 20-problems.png — Problems / error highlight
Steps: insert `app.greting("typo");` after line 11, Ctrl+S, Alt+6 (reverted afterwards).
- Editor: `greting` in red; top-right inspection widget "❗1 ^ v"; red stripe mark in scrollbar;
  **file tab name, and every ancestor in the project tree (java-gradle, src, main, java, com.example, App) get a
  red wavy underline**.
- Problems tool window tabs: **File 1** (active), Project Errors, Vulnerable Dependencies, Qodana.
  Row `App.java /home/user/.../com/example  1 problem` > "❗ Cannot resolve method 'greting' in 'App' :12".
  Left toolbar: eye (preview), bulb (quick-fix), group toggle.

### 21-gradle-tool-window.png — Gradle tool window
Steps: click elephant icon on right stripe; expand Tasks, application, verification; select test.
- Title "Gradle", toolbar icons (left to right): reload (circular arrows), attach/link Gradle project,
  download sources, +, – , execute task, toggle offline, analyze dependencies, expand, collapse, settings
  (tooltips not captured for Gradle).
- Tree: `java-gradle` > `Tasks` > groups `application (run)`, `build`, `build setup`, `distribution`,
  `documentation`, `help`, `other`, `verification (check, test)`; `Dependencies`. Tasks have gear icons;
  double-click runs a task. Opens as right-side panel splitting the editor.
- `x01-gradle-settings.png`: Settings > Build, Execution, Deployment > Build Tools > Gradle: "Gradle user home"
  `/root/.gradle`, "Generate *.iml files...", "Enable parallel Gradle model fetching for Gradle 7.4+" (warning icon),
  per-project section: "Download external annotations for dependencies", **Build and run using: Gradle (Default)**,
  **Run tests using: Gradle (Default)**, Distribution `Wrapper`, "Gradle JVM criteria" Version `25`.
  Build Tools node lists Maven, Gradle, Gant, **Bazel**.

### 22-structure.png — File structure popup
Steps: Ctrl+F12 in App.java.
- Popup titled "App.java" with checkboxes "Inherited members (Ctrl+F12)", "Anonymous classes (Ctrl+I)",
  "Lambdas (Ctrl+L)" and a gear. Tree: `App` > `greeting(String): String`, `main(String[]): void` (current
  member pre-selected); visibility markers next to icons. Typing filters (speed search).

### 23-debugger.png — Debugger at breakpoint
Steps: click gutter at line 11 (red dot), click gutter run icon on line 10 > "Debug 'App.main()'", F8 once.
- `x03-gutter-run-popup.png`: gutter popup "Run 'App.main()' Ctrl+Shift+F10", "Debug 'App.main()'",
  "Run 'App.main()' with Coverage", "Modify Run Configuration...".
- Debug tool window tab `java-gradle [:com.example.App.main()]` (debug through Gradle). Toolbar: rerun, stop,
  eye, resume, pause, step over, step into, step out, view breakpoints, mute breakpoints, kebab. Sub-tabs
  **Threads & Variables** | **Console**; layout button on right.
- Frames: thread combo `"main"@1 in g...main": RUNNING` with filter; frame `main:12, App (com.example)`.
  Variables pane: "Evaluate expression (Enter) or add a watch (Ctrl+Shift+Enter)" field, `args = {String[0]@907} []`,
  `app = {App@908}` (expandable).
- Editor: execution line highlighted blue, breakpoint line red, **inline values** `args: []`, `app: App@908`
  after code. Main toolbar shows green "resume"-style debug button and red stop square; tip
  "Switch frames from anywhere in the IDE with Ctrl+..." at the bottom.

### 24-plugins.png — Plugins
Steps: Settings > Plugins (Marketplace tab).
- Header "Plugins" with tabs **Marketplace** | **Installed 1** (badge) and gear. "Plugins Auto-Update" GotIt
  balloon appears first time (dismissed). Search "Type / to see options". "Suggested": Docker (Tools
  Integration) with Install; "Staff Picks" + "Show all": Junie, JetBrains AI Assistant, Kotlin Multiplatform,
  AWS Toolkit, Key Promoter X, IdeaVim — each with downloads, rating, vendor, **Install** button.
  Right pane: detail of selected plugin (tag, name, vendor, homepage, Install, version, reason
  "This plugin is suggested because you have docker installed", tabs Overview / What's New / Reviews /
  Additional Info, screenshots carousel).
- `bazel/b07-plugin-installed.png`: Installed tab: "Bundled plugins updates - Update all 1" (Subversion
  252.28539.97 -> 252.28539.109 Update), "Downloaded (1 of 1 enabled)" > **Bazel 2025.2.12 JetBrains** with checkbox;
  grouped sections "AI-Powered" (MCP Server), "Build Tools" (Artifacts Repository Search, Gradle, Gradle for Java,
  Maven) with "Disable all" links. Bazel detail: Disable split button, version, features list (Open Bazel projects,
  Java and Kotlin, build/run/test/debug targets, Starlark syntax/completion/navigation, Starlark debugging, tests).

## Scenes — bazel-java (`bazel/`)

Bazel plugin status: **installed and working** (2025.2.12 on build 252). Open flow: running
`bin/idea /home/user/samples/bazel-java` while a project is open shows the "New Project — Where would you like to
open the project 'bazel-java'?" prompt with **This Window / New Window / Cancel** and "Don't ask again".
No import wizard: the plugin auto-detects `MODULE.bazel`, creates `.bazelbsp/.bazelproject`, opens it in the
editor and starts sync immediately.

### b01-bazel-import.png / b01b-bazel-sync-done.png — Import / sync
- During sync (~5 s after open): Build tool window **Sync** tab with a green "running" dot on the tab; tree
  "Bazel: Sync: Syncing... 4 sec" > "Collect project targets 2 sec" (spinner icons); console streams the exact
  Bazel commands: `Invoking: '/usr/local/bin/bazel' 'mod' 'dump_repo_mapping' '--tool_tag=bazelbsp:3.2.0' ...`,
  `'mod' 'graph' ... '--output=json'`, `bazel info` output (output_base, workspace, bazel-bin, starlark-semantics)
  and "Command completed in 272ms (exit code 0)". Status bar: "Syncing project..." + progress bar + cancel (x) +
  "Show all (2)". Toolbar VCS widget temporarily reads "Version control v".
- Project tree during sync shows raw folders incl. `.bazelbsp`, `bazel-bazel-java`, `bazel-bin`, `bazel-out`,
  `bazel-testlogs` (orange = excluded). **After sync** these collapse into an **"Excluded Directories"** node and
  `.bazelversion`, `BUILD.bazel`, `MODULE.bazel` get the Bazel (green double-heart) icon; `MODULE.bazel.lock`
  turns blue (modified — the sync rewrote the lock file; reverted afterwards).
- Done: "✓ Bazel: Sync: Sync done. At 10/10/26, 1:50 PM  8 sec, 430 ms"; console ends with
  "Analyzing: 3 targets (2 packages loaded, 0 targets configured)", "INFO: Build completed successfully, 5 total
  actions", "INFO: Build Event Protocol files produced successfully." Then "Waiting for 'Scanning' to complete...".
- Generated `.bazelproject` content: comments ("This project view file may be overwritten by the Bazel plugin..."),
  `derive_targets_from_directories: true`, `directories: .`, commented `# import_depth: 0`, `# import_ijars: true`.

### b02-bazel-targets.png / b02b-bazel-target-menu.png — Bazel tool window
Steps: click the Bazel icon on the right stripe; right-click `app`.
- Title "Bazel"; toolbar tooltips (verified by hover): **Resync project**, **Generate sources and resync project**,
  **Filter Targets** (dropdown), **Open Bazel settings**, **Open project view file in editor**,
  **Show Bazel Query Tool Window**. Below: "Search targets (Ctrl+F)" field with Cc, `.*` and a
  flat/tree toggle; target list `app`, `greeter` (flat, labels without `//:`).
- Context menu on a target: Copy Target Label `Ctrl+C` | Build Target, Run, Debug run, Jump to BUILD file `F4`,
  Starlark Debug.

### b03-bazel-build.png (= 16-build-output.png) — Build a target
Steps: Bazel tool window > right-click app > Build Target.
- Build tool window gains a **Build Output** tab: "✓ Bazel: Build: Successfully completed. At ... 536 ms";
  console: "INFO: Found 1 target...", "Target //:app up-to-date:" with clickable links `bazel-bin/app`,
  `bazel-bin/app.jar`, elapsed time, "Command completed in 495ms (exit code 0)".
  `16-build-output.png` is a byte-identical copy of b03 (same scene).

### 17-run-output.png (bazel) — Run a target
Steps: right-click app > Run.
- Run tool window tab "Run //:app"; run widget becomes "Run //:app" with Bazel icon. Console: bazel build INFO
  lines, "INFO: Running command line: bazel-bin/app" (link), program output `Hello, Bazel!`,
  "Command completed in 366ms (exit code 0)". No tree pane (plain console with left toolbar).

### b04-bazel-editor.png / b04b-starlark-completion.png — BUILD.bazel (Starlark)
Steps: double-click BUILD.bazel; then Ctrl+End and type `java_` (reverted).
- Starlark highlighting: rule names (`java_library`, `java_binary`) and `load` call coloured, keyword args
  (`name`, `srcs`, `main_class`, `deps`) purple, strings green. **Gutter**: hammer (build) icon on
  `java_library(` (line 3), run triangle on `java_binary(` (line 8). Status bar breadcrumb `bazel-java > BUILD.bazel`.
- Completion after `java_`: `java_binary` (twice: function + loaded symbol icon), `java_import`, `java_library`
  (twice), `java_lite_proto_library`, `java_package_configuration`, `java_plugin`, `java_proto_library`,
  `java_runtime`, `java_test`, `java_toolchain`...; footer "Press Enter to insert, Tab to replace Next Tip".

### 04-editor-project-tree.png (bazel) — Tree + App.java
- Tree: `src > main > java > com > example > App, Greeter` (java is not shown as a coloured source root; `com`
  and `example` are separate package nodes, not compacted), Bazel files with Bazel icon, "Excluded Directories"
  and "External Libraries".
- Editor App.java: gutter run icons on class and main; parameter-name inlay `name:` in `greet( name: "Bazel")`;
  folded one-line main body. Window title `bazel-java – App.java [app]` (target name in brackets);
  **status bar target widget `//:app`** (Bazel icon). `b08-target-widget.png`: clicking it opens popup
  "Targets Available For The File" listing `//:app >`.

### 21-gradle-tool-window.png (bazel) — Bazel tool window equivalent
- Bazel tool window docked right next to App.java with `greeter` selected (same toolbar as b02).

### b05-maven-gradle-import.png / b05a-import-chooser.png — Import from external model
Steps: Find Action "Project from Existing Sources" > "Import Project from Existing Sources..." > path
`/home/user/samples/java-gradle` > Enter; choose "Import project from external model" > Gradle.
- Chooser "Select File or Directory to Import": text "Select directory with existing sources, Eclipse project
  (.project) or classpath (.classpath) file, Maven project file (pom.xml), Gradle build script (*.gradle,
  *.gradle.kts, *.gradle.dcl)"; path field with history dropdown, toolbar (home, desktop, project, module,
  new folder, delete, refresh), "Hide path", "Drag and drop a file into the space above to quickly locate it".
- "Import Project" dialog: radios "Create project from existing sources" / "Import project from external model"
  with list **Eclipse, Gradle, Maven** — **Bazel is not offered here**; Bazel import happens only by opening
  the folder. Primary button changes from "Next" to "Create" when an external model is selected. Cancelled.

### b09-find-action-bazel.png / b10-bazel-settings.png
- Find Action "bazel": Bazel (tool window, "Activate Bazel window"), Bazel (Settings), Bazel Package (New),
  Bazel notifications / BazelPlugin notifications / BazelSlowPlugins notifications (Settings > Notifications),
  Bazel ProjectView (Settings), "Bazel: Bazel Version Check" ON toggle, "Plugins: Bazel" ON toggle,
  Show Bazel Query Tool Window, "Enable options for running Bazel targets using local JVM runner"
  (Settings > Experimental Settings).
- Settings > Build, Execution, Deployment > Build Tools > Bazel: "Project View Path:"
  `/home/user/samples/bazel-java/.bazelbsp/.bazelproject`, "Buildifier Executable Path:" field in error state
  "Unable to auto-detect buildifier executable in $PATH", checkboxes "Run buildifier on file save" (on),
  "Show excluded directories as a separate node in the project tree" (on).

## Skipped / not captured

- None of scenes 01–24 skipped. 02 and 03 are the same single-page wizard (IntelliJ has no separate details page).
- Bazel equivalents of 04, 16, 21 captured; 16 equals b03.
- Bazel debugging of `//:app` and Starlark debug were not exercised.
- Gradle tool-window toolbar tooltips were not hover-verified (names in 21 are from icon shapes).

## Cross-cutting UX observations (acceptance-criteria candidates)

1. Opening a folder auto-detects the build system (Gradle via `build.gradle.kts`, Bazel via `MODULE.bazel`) and
   starts sync immediately with no wizard; trust prompt suppressed only by `-Didea.trust.all.projects=true`.
2. Sync progress = Build tool window **Sync** tab (tree of phases with durations + raw console of tool commands)
   plus a status-bar progress bar ("Syncing project..." / "Indexing...") with cancel and "Show all (n)".
3. Sync failure shows a red error node per unresolved dependency and a "Possible solution" text; re-sync button
   lives in the Sync tab's left toolbar and in the Gradle/Bazel tool window toolbar.
4. Run/Debug from gutter icons in Java (class/main) and in BUILD files (hammer on library, play on binary);
   Gradle projects run `main` through a generated Gradle task (`:com.example.App.main()`), Bazel via `bazel run`.
5. Run widget in the main toolbar reflects the last-run temporary config (Current File -> App -> java-gradle [test]
   -> Run //:app) and its popup has inline Run/Debug buttons per config.
6. Test runner hides passed tests by default (Show Passed toggle); summary "1 test passed 1 test total, 21ms".
7. Errors propagate visually: red wavy underline on tab title and on every ancestor node in the project tree,
   plus the editor's top-right counter and the Problems "File" tab with line suffix `:12`.
8. Search Everywhere/Find Action results show shortcut inline, group/location right-aligned, and boolean options
   as inline ON/OFF toggles.
9. Project tree semantics: excluded dirs orange; test roots green-tinted; source roots coloured; Bazel moves
   bazel-* symlinks into "Excluded Directories".
10. Diff and commit are non-modal: Commit panel replaces the Project panel; diff opens as an editor tab with
    per-chunk checkboxes for partial commits.

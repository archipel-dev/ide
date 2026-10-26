# Java + Gradle feature inventory (target: IntelliJ IDEA 2026.2)

Status: research spec, clean-room. Every description below is written in our own words from public
documentation, observable behaviour, and facts (IDs, names, defaults, shortcuts) read from the
Apache-2.0 IntelliJ Community sources at tag `idea/262.10968.63`. No code, artwork, or documentation prose
is copied. Short UI labels are quoted as names only.

Archipel IDE models IntelliJ IDEA's Java/Gradle support as plugins on the platform core
(see `docs/architecture.md`): a **Java language plugin** (PSI, resolve, completion, inspections,
refactorings, generation, run/debug), a **Gradle/external-system plugin** (project model import,
sync, tool window, task execution) and a **JVM build plugin** (internal builder / compiler integration).
This file is the inventory those plugins must cover. It makes **no parity claim**; parity evidence lives
in `docs/parity/`.

## How to read this document

- **ID**: stable identifier. `GRADLE-NNN` for build-tool/project-model features, `JAVA-<AREA>-NNN` for
  everything else. IDs are never reused; deleted features are struck through, not renumbered.
- **Shortcut** columns give the default keymaps: **Win/Linux** = the `$default` keymap ("Windows" /
  "Linux" in the UI); **macOS** = the `Mac OS X 10.5+` keymap (shown as "macOS" in the UI). `—` means no
  default binding (the action is still reachable through Find Action, `Ctrl+Shift+A` / `Cmd+Shift+A`).
  macOS keys are written as `Cmd`, `Opt`, `Ctrl`, `Shift`.
- **Entry point**: main menu path (`Menu | Item`), tool window, popup, or gutter/editor affordance.
- **Pri**: `P0` = needed for a usable everyday Java + Gradle workflow; `P1` = expected by most users,
  ship soon after P0; `P2` = advanced / niche.
- **Paid**: `$` marks features that require an IntelliJ IDEA (Ultimate) subscription in the unified
  distribution (see [Community vs Ultimate](#community-vs-ultimate)). Unmarked = free core.
- "(inferred)" marks behaviour deduced from observation or from resource files rather than stated in
  documentation.
- Shortcut facts were resolved from the keymap XML files and the action registrations in the
  Community sources (`platform/platform-resources/src/keymaps/*.xml`, `*Actions.xml`); for macOS, actions
  that the macOS keymap does not override inherit the default binding with `Ctrl` mapped to `Cmd`
  (inferred from keymap inheritance rules; verify against the published macOS keymap PDF).

## Contents

1. [Community vs Ultimate](#community-vs-ultimate)
2. [Gradle project model, sync and tooling](#1-gradle-project-model-sync-and-tooling) (GRADLE)
3. [JDK management and language level](#2-jdk-management-and-language-level) (JAVA-JDK)
4. [Java editing](#3-java-editing) (JAVA-EDIT, JAVA-LT, JAVA-PFX, JAVA-FMT, JAVA-LANG, JAVA-LOMBOK)
5. [Navigation and search](#4-navigation-and-search) (JAVA-NAV)
6. [Inspections, intentions and quick-fixes](#5-inspections-intentions-and-quick-fixes) (JAVA-INSP)
7. [Refactorings](#6-refactorings) (JAVA-REF)
8. [Code generation](#7-code-generation) (JAVA-GEN)
9. [Run, test, debug, coverage, profiling](#8-run-test-debug-coverage-profiling) (JAVA-RUN, JAVA-TEST, JAVA-DBG, JAVA-COV, JAVA-PROF)
10. [Build](#9-build) (JAVA-BUILD)
11. [VCS touchpoints](#10-vcs-touchpoints-for-java-projects) (JAVA-VCS)
12. [Summary counts](#summary-counts)
13. [Open questions](#open-questions)

---

## Community vs Ultimate

Since release 2025.3, IntelliJ IDEA ships as a **single unified distribution**. There is no separate
"Community Edition" download any more: one IDE installs, and its core (Java, Kotlin, Groovy, Gradle,
Maven, the Java debugger, JUnit/TestNG runners, code coverage, VCS, basic database tools, and more)
works for free without a subscription. "Advanced tooling" (historically the Ultimate feature set) is
unlocked with a subscription, and a 30-day trial is offered. When a subscription lapses the free core
keeps working.

For this Java + Gradle inventory almost everything is in the free core. Items that require a paid
subscription (marked `$` in tables) are, to our knowledge:

| Feature | Paid? | Note |
|---|---|---|
| IntelliJ Profiler (CPU/allocation profiling, live charts, memory snapshots UI) | `$` (inferred) | Historically Ultimate-only; the unified-product page does not list it as free. See Open questions. |
| Spring / Spring Boot support, Spring debugger insights, Jakarta EE, Micronaut, Quarkus, Helidon | `$` | Framework support; out of scope here but they hook into Java run configurations. |
| HTTP Client, endpoints tool window | `$` (inferred) | Out of scope. |
| Full database tools (Database tool window beyond the basic set, SQL in Java strings with schema awareness) | partly `$` | Basic DB tools became free in 2025.3. |
| Remote development / Code With Me host features | `$` (inferred) | Platform, out of scope. |
| JVM debugger, HotSwap, coverage, JUnit/TestNG, Gradle, Maven, inspections, refactorings | free | Core. |
| AI Assistant / next-edit suggestions | separate licence | Not part of this inventory. |

Sources:
- https://www.jetbrains.com/help/idea/intellij-idea-single-distribution.html
- https://lp.jetbrains.com/intellij-idea-unified-faq/
- https://www.jetbrains.com/idea/whatsnew/2025-3/
- https://www.jetbrains.com/idea/features/

---

## 1. Gradle project model, sync and tooling

Terminology note: in 2026.x the UI wording moved from "reload/re-import" to **"sync"**
(e.g. "Sync Gradle Changes", "Sync All Gradle Projects", "Auto-Sync Settings…"). Older help pages still
say "Load Gradle Changes" / "Reload All Gradle Projects". Archipel IDE should use the "sync" vocabulary and
treat the others as aliases in Find Action.

### 1.1 Opening, importing, linking

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| GRADLE-001 | Open Gradle project | Opening a directory (or a `build.gradle`, `build.gradle.kts`, `settings.gradle(.kts)` file) that contains Gradle build files detects it as a Gradle project, links it and starts a sync that builds the IDE project model. | — | — | File \| Open…; Welcome screen \| Open | P0 |
| GRADLE-002 | "Open as project" choice | When a build file is opened and several build systems are detected (e.g. both `pom.xml` and `build.gradle`), the user is asked which model to use. | — | — | Open dialog follow-up popup | P1 |
| GRADLE-003 | Trust project prompt | Before running any build logic of a newly opened project the IDE asks whether the project is trusted; untrusted projects open in safe mode without executing Gradle. | — | — | Modal on open | P0 |
| GRADLE-004 | New Gradle project wizard | Creates a Java project with Gradle as build system: JDK, Gradle DSL (Kotlin or Groovy), optional sample code, group/artifact/version, Gradle distribution and version, wrapper generation, JUnit 5 test setup by default. | — | — | File \| New \| Project…; Welcome screen \| New Project | P0 |
| GRADLE-005 | New Gradle module | Adds a sub-project to an existing build (creates directory, build script and an `include(...)` entry in settings). | — | — | File \| New \| Module… | P1 |
| GRADLE-006 | Link Gradle Project | Attaches an additional Gradle build (another root build script) to the open IDE project; several linked builds may coexist. Action id `Gradle.ImportExternalProject`. | — | — | Gradle tool window toolbar "+"; context menu on build file "Link Gradle Project" / "Import Gradle Project" | P1 |
| GRADLE-007 | Unlink Gradle Project | Detaches a linked build; a dialog lets the user keep or remove the derived modules. | Delete (in tool window) | Delete | Gradle tool window toolbar / context menu | P1 |
| GRADLE-008 | Ignore / Unignore Gradle project | Marks a (sub)project as ignored: it stays visible but is excluded from import (e.g. `buildSrc`). | — | — | Gradle tool window context menu | P2 |
| GRADLE-009 | Orphan modules dialog | After sync removes modules that previously existed, offers to restore modules that carried user settings. | — | — | Modal after sync | P2 |
| GRADLE-010 | Convert plain project to Gradle | Adding build/settings scripts to an unlinked project triggers a "Gradle build script found" notification offering to link it. | — | — | Editor notification / balloon | P1 |
| GRADLE-011 | Gradle Project Data To Import | Selects which modules/data of a linked build are imported (partial import for large multi-project builds); has Select All, Unselect All, Show Selected Only, Select Required. Action id `ExternalSystem.SelectProjectDataToImport`. | — | — | Gradle tool window toolbar | P2 |

### 1.2 Sync / reload

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| GRADLE-020 | Initial sync on open | Runs Gradle (Tooling API) to fetch the build model: projects, source sets, dependencies, Java toolchain/language level, tasks; maps it into modules, content roots, libraries and SDKs. | — | — | Automatic | P0 |
| GRADLE-021 | Floating "Sync Gradle Changes" button | When build scripts, `gradle.properties`, wrapper properties or version catalogs change in the editor, a floating button appears in the editor's top-right corner; clicking it syncs. Action id `ExternalSystem.ProjectRefreshAction`; it can be dismissed (`ExternalSystem.HideProjectRefreshAction`). Older label: "Load Gradle Changes". | Ctrl+Shift+O | Cmd+Shift+I | Floating editor toolbar; Find Action | P0 |
| GRADLE-022 | Sync All Gradle Projects | Re-syncs every linked Gradle build. Action id `ExternalSystem.RefreshAllProjects` (label "Sync All Gradle Projects"; older "Reload All Gradle Projects"). | — | — | Gradle tool window toolbar | P0 |
| GRADLE-023 | Sync single Gradle project | Re-syncs only the selected linked build. Action id `ExternalSystem.RefreshProject`. | — | — | Gradle tool window context menu "Sync Gradle Project"; context menu on build script | P1 |
| GRADLE-024 | Auto-sync settings | Settings page with "Sync project after changes in the build scripts", choices **Any changes** vs **External changes** (only VCS / outside-IDE changes); enabled by default with External changes (inferred default). Action `ExternalSystem.ShowCommonSettings` ("Auto-Sync Settings…"). | — | — | Settings \| Build, Execution, Deployment \| Build Tools; Gradle tool window settings menu | P0 |
| GRADLE-025 | Sync progress and errors in Build tool window | Sync runs as a background task with a "Sync" tab in the Build tool window: tree of phases, console output, warnings, errors with links to the script line, and quick-fix links (e.g. "Download Gradle JDK", "Upgrade wrapper"). | Alt+4 shows Run; Build has no default shortcut | — | Build tool window \| Sync tab; status bar | P0 |
| GRADLE-026 | Sync failure notifications with fixes | Well-known failures (incompatible JDK/Gradle version, missing wrapper, unresolved dependency, offline cache miss, proxy issues) are recognised and offered fixes. | — | — | Build tool window, balloon | P1 |
| GRADLE-027 | Refresh dependencies | Re-sync forcing Gradle to re-resolve dynamic/changing dependencies (`--refresh-dependencies`). Action id `Gradle.RefreshDependencies`. | — | — | Gradle tool window context menu | P2 |
| GRADLE-028 | Download Sources | Downloads sources jars for all libraries (sources are normally downloaded on demand when opening a decompiled class; since 2026.1 sources/docs may download automatically). Action id `Gradle.DownloadSources`. | — | — | Gradle tool window toolbar; editor banner on decompiled class "Download Sources" | P1 |
| GRADLE-029 | Parallel model fetch | Option to fetch Gradle models in parallel for Gradle 7.4+ (setting "Enable parallel Gradle model fetching"). | — | — | Settings \| … \| Build Tools \| Gradle | P2 |
| GRADLE-030 | Execute task before/after sync | Gradle tasks can be bound to run before or after sync (task activation). Action ids `ExternalSystem.BeforeSync`, `ExternalSystem.AfterSync`. | — | — | Gradle tool window task context menu | P2 |
| GRADLE-031 | Declarative Gradle sync (experimental) | Support for the experimental Declarative Gradle (`.gradle.dcl`) project model and its sync (bundled module `gradle-declarative-sync`). | — | — | Automatic when DCL files present | P2 |

### 1.3 Project model mapping

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| GRADLE-040 | Module per Gradle project | Every Gradle (sub)project becomes an IDE module group named after its path (`root`, `root.app`, …). | — | — | Project Structure \| Modules | P0 |
| GRADLE-041 | Module per source set | Each source set (`main`, `test`, custom ones like `integrationTest`) becomes its own module (`root.app.main`, `root.app.test`) with its own dependencies, output and roots; the legacy "single module per project" mode is deprecated and kept only for troubleshooting. | — | — | Project Structure; Project tool window | P0 |
| GRADLE-042 | Content roots and root types | `src/<set>/java` → Sources/Test Sources, `src/<set>/resources` → Resources/Test Resources, `build/generated/...` → Generated Sources, `build` and `.gradle` excluded. | — | — | Project tool window icons | P0 |
| GRADLE-043 | Test source-set detection | Source sets used by `Test` tasks (or by the `jvm-test-suite` plugin) are marked as test roots. | — | — | Automatic | P0 |
| GRADLE-044 | Dependency scopes mapping | Gradle configurations map to module dependency scopes: `implementation`/`api` → Compile, `compileOnly` → Provided, `runtimeOnly` → Runtime, `testImplementation` → Test (on the test module), `annotationProcessor` → annotation processor path. | — | — | Project Structure \| Modules \| Dependencies | P0 |
| GRADLE-045 | Inter-project dependencies | `project(":lib")` dependencies become module dependencies (source navigation instead of jar). Test-fixtures dependencies supported. | — | — | Automatic | P0 |
| GRADLE-046 | External libraries | Resolved artifacts become project-level libraries named `Gradle: group:artifact:version`, with sources/javadoc attached when available. | — | — | Project tool window \| External Libraries | P0 |
| GRADLE-047 | Composite builds | Builds included via `includeBuild` (Gradle 4.5.1+ for IDE support) are linked; substituted dependencies become module dependencies. Composite participants can also be chosen in a dialog. Action id `Gradle.OpenProjectCompositeConfiguration`. | — | — | Automatic; Gradle tool window context menu "Composite Build Configuration" | P1 |
| GRADLE-048 | buildSrc support | `buildSrc` is imported as its own build with its own module(s); its classes resolve in build scripts. | — | — | Automatic | P1 |
| GRADLE-049 | Included plugin builds (`pluginManagement { includeBuild }`) | Convention-plugin builds (e.g. `build-logic`) are imported and navigable. | — | — | Automatic | P1 |
| GRADLE-050 | Language level per module | Each module's language level comes from `java.sourceCompatibility`/`release`/toolchain; project-level value is the lowest used (single linked build). | — | — | Project Structure \| Modules \| Sources | P0 |
| GRADLE-051 | Module SDK from Java toolchain | If `java.toolchain.languageVersion` is set, the module SDK is matched to a registered JDK with that version (inferred: picks a matching registered JDK, otherwise offers download). | — | — | Automatic | P1 |
| GRADLE-052 | Compiler args / encoding from build | `options.encoding`, `compilerArgs`, `--enable-preview` etc. are propagated to the IDE compiler and highlighting (inferred for compiler args). | — | — | Automatic | P1 |
| GRADLE-053 | Annotation processor config import | `annotationProcessor` dependencies become per-module processor profiles; generated sources directories registered. | — | — | Settings \| Build \| Compiler \| Annotation Processors (read-only "Gradle Imported" profile) | P0 |
| GRADLE-054 | IDE settings via `idea-ext` plugin | The `org.jetbrains.gradle.plugin.idea-ext` plugin can set package prefixes, encodings, run configurations, code style, etc. which are applied on sync. | — | — | Build script | P2 |
| GRADLE-055 | Generate `.iml` files option | Store generated module files in `.idea` / project dir (for mixed projects) instead of the system directory. | — | — | Settings \| … \| Gradle "Generate *.iml files for modules imported from Gradle" | P2 |
| GRADLE-056 | Manual changes are transient | Dependencies edited in Project Structure on Gradle modules are overwritten by the next sync (Gradle is the source of truth); the UI warns about it. | — | — | Project Structure | P1 |
| GRADLE-057 | External annotations download | Optionally downloads external nullability/contract annotations for known libraries from a public repository. | — | — | Settings \| … \| Gradle "Download external annotations for dependencies" | P2 |
| GRADLE-058 | Java module system (module-info) in Gradle | JPMS modules declared in `module-info.java` are resolved against Gradle module path. | — | — | Automatic | P1 |

### 1.4 Gradle JVM, distribution, wrapper

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| GRADLE-060 | Gradle JVM selection | Chooses the JDK that runs the Gradle daemon for sync and tasks. Choices: Project SDK, `JAVA_HOME`, any registered JDK, `#GRADLE_LOCAL_JAVA_HOME` (from `gradle.properties` `org.gradle.java.home`), Download JDK…, Add JDK…. | — | — | Settings \| … \| Build Tools \| Gradle \| Gradle JVM | P0 |
| GRADLE-061 | Gradle JVM auto-resolution | On first open, if no suitable JVM is configured the IDE picks a compatible one or offers to download one ("Download Java to run Gradle …"); JRE is rejected (JDK required). | — | — | Automatic, Build tool window fix link | P0 |
| GRADLE-062 | Gradle/JDK compatibility check | Validates the Gradle version against the selected JVM (e.g. "JDK X isn't supported by Gradle Y") and offers to change JVM or upgrade the wrapper. | — | — | Sync errors; settings validation | P0 |
| GRADLE-063 | Daemon JVM criteria (daemon toolchain) | For Gradle 8.8+, `gradle/gradle-daemon-jvm.properties` defines version/vendor criteria; the settings page then shows "Gradle JVM criteria" (Version, Vendor) instead of a fixed JVM; enabled by default since 2025.1 for existing criteria files. Task "Configure Daemon JVM Criteria" can generate the file. | — | — | Settings \| … \| Gradle | P2 |
| GRADLE-064 | Distribution source | "Distribution": **Wrapper** (from `gradle-wrapper.properties`, recommended default), **Wrapper task** (from `wrapper` task in the script), **Local installation** (path). | — | — | Settings \| … \| Gradle | P0 |
| GRADLE-065 | Gradle user home | Overrides the Gradle user home (default `~/.gradle`, or `GRADLE_USER_HOME`). | — | — | Settings \| … \| Gradle | P2 |
| GRADLE-066 | Wrapper generation and upgrade | New projects get a wrapper; quick-fixes/notifications can upgrade the wrapper version ("Upgrade Gradle wrapper"); inspection flags non-latest minor version in `gradle-wrapper.properties`. | — | — | Build tool window link; intention in properties file | P1 |
| GRADLE-067 | Gradle VM options migration | Legacy IDE-side Gradle VM options are migrated into `org.gradle.jvmargs` in `gradle.properties`. | — | — | Settings link "Migrate" | P2 |
| GRADLE-068 | Gradle Daemons dialog | Lists running/stopped daemons with status and description; actions Refresh, Stop Selected, Stop All, Stop When Idle All, Show Stopped. Action id `Gradle.ShowDaemons`. | — | — | Find Action "Show Gradle Daemons" | P2 |
| GRADLE-069 | Per-linked-build settings | Each linked Gradle build has its own JVM, distribution and build/run delegation settings (list of linked projects on the settings page). | — | — | Settings \| … \| Gradle | P1 |

### 1.5 Offline mode, proxies, repositories

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| GRADLE-075 | Toggle Offline Mode | Passes `--offline` to all Gradle invocations; dependencies resolve only from cache, missing ones fail with a hint. Action id `Gradle.ToggleOfflineAction`. | — | — | Gradle tool window toolbar; Find Action | P1 |
| GRADLE-076 | IDE proxy propagation | IDE HTTP proxy settings are offered/passed to Gradle (inferred: prompts to copy proxy settings into `gradle.properties`). | — | — | Settings \| Appearance & Behavior \| System Settings \| HTTP Proxy | P2 |

### 1.6 Gradle tool window

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| GRADLE-080 | Gradle tool window | Right-side tool window listing linked builds as a tree: project → Tasks (grouped by `group`), Dependencies (per configuration / source set), Run Configurations, and sub-projects. | — (no default) | — | View \| Tool Windows \| Gradle | P0 |
| GRADLE-081 | Run task by double-click | Double-clicking a task runs it, creating a temporary Gradle run configuration. | — | — | Gradle tool window | P0 |
| GRADLE-082 | Task context menu: Run / Debug / Modify Run Configuration | Run, debug (attaches debugger to forked JVMs such as tests/`JavaExec`), or create/edit a run configuration for a task. | — | — | Gradle tool window context menu | P0 |
| GRADLE-083 | Execute Gradle Task (Run Anything) | Popup to type one or more task names with arguments (completion of task names and options), runs them in the selected project. Action id `Gradle.ExecuteTask`; also via Run Anything with `gradle` prefix. | Ctrl, Ctrl (Run Anything) | Ctrl, Ctrl | Gradle tool window toolbar "Execute Gradle Task" | P0 |
| GRADLE-084 | Task activation (before/after build, rebuild, sync, run) | Binds tasks to IDE events; nodes show the binding. Actions `ExternalSystem.BeforeCompile`, `AfterCompile`, `BeforeRebuild`, `AfterRebuild`, `BeforeSync`, `AfterSync`, `BeforeRun`; dialog `ExternalSystem.OpenTasksActivationManager`. | — | — | Task context menu | P2 |
| GRADLE-085 | Assign shortcut to task | Binds a keyboard shortcut to a task or run configuration (`ExternalSystem.AssignShortcut`). | — | — | Task context menu | P2 |
| GRADLE-086 | Open Gradle Config | Opens the build script of the selected project node. Action id `ExternalSystem.OpenConfig`. | F4 | Cmd+Down / F4 | Gradle tool window context menu | P1 |
| GRADLE-087 | View options | Group Modules, Group Tasks, Show Ignored, Display Tasks Inherited from Sub-Projects (`ExternalSystem.ShowInheritedTasks`), Expand All / Collapse All. | Ctrl+NumPad+ / Ctrl+NumPad- | Cmd+= / Cmd+- | Gradle tool window options menu | P1 |
| GRADLE-088 | Speed search in tree | Typing filters/highlights tasks and dependencies. | type | type | Gradle tool window | P1 |
| GRADLE-089 | Dependencies tree node | Shows each configuration's resolved dependency tree, with navigation to declaration. | — | — | Gradle tool window \| Dependencies | P1 |
| GRADLE-090 | Dependency Analyzer | Dedicated view: list of all resolved dependencies per scope/module, conflict and omitted-version highlighting, "show usages" paths explaining why a dependency is present, navigation to declaration (`Gradle.DependencyAnalyzer.GoTo`), open build config. Entry ids `Gradle.ViewDependencyAnalyzer`, `Gradle.ToolbarDependencyAnalyzer`, `Gradle.ProjectViewDependencyAnalyzer`. | — | — | Gradle tool window toolbar "Analyze Dependencies"; Project view context menu | P1 |
| GRADLE-091 | Dependencies diagram | Graph visualisation of module/dependency relations. | — | — | Gradle tool window "Show Diagram" (inferred `$`) | P2 |
| GRADLE-092 | Build Tools settings shortcut | Toolbar menu opening Gradle settings and Auto-Sync settings. | — | — | Gradle tool window toolbar | P1 |
| GRADLE-093 | Run Configurations node | Lists saved Gradle run configurations; double-click runs; context menu edits/removes (`ExternalSystem.EditRunConfiguration`, `ExternalSystem.RemoveRunConfiguration`). | — | — | Gradle tool window | P1 |

### 1.7 Build/run delegation

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| GRADLE-100 | "Build and run using" | Per linked build: **Gradle** (default) or **IntelliJ IDEA** (internal JPS builder). With Gradle, Build Project, Run and Debug of Java applications invoke Gradle tasks (`classes`, `testClasses`, generated init scripts for running main classes); with IntelliJ IDEA the internal incremental compiler is used. | — | — | Settings \| … \| Build Tools \| Gradle | P0 |
| GRADLE-101 | "Run tests using" | Per linked build: **Gradle** (default), **IntelliJ IDEA**, or **Choose per test**. Determines whether gutter/context test runs create Gradle or JUnit/TestNG configurations. | — | — | Same page | P0 |
| GRADLE-102 | Gradle-backed Build Project | Build \| Build Project maps to Gradle compile tasks of all modules; Rebuild maps to clean + compile (inferred). | Ctrl+F9 | Cmd+F9 | Build menu | P0 |
| GRADLE-103 | Gradle-backed application run | Running a `main` method from the gutter generates a Gradle-executed Application run (init script adds a JavaExec task). | Ctrl+Shift+F10 | Ctrl+Shift+R | Gutter, context menu | P0 |
| GRADLE-104 | HotSwap with Gradle delegation | When debugging with delegated build, recompiling via Gradle still allows class reload into the debuggee. | Ctrl+F9 | Cmd+F9 | Build menu during debug | P1 |

### 1.8 Gradle run configurations and execution

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| GRADLE-110 | Gradle run configuration type | Fields: Name, Run (tasks and arguments in one command line with completion; "Default tasks will be executed if no tasks are specified"), Gradle project (path), Environment variables, VM options (for the Gradle client), "Store as project file", Before launch list. Modify options: Run as test ("shows test view and forces test re-execution"), Debug Gradle scripts, Debug forked Gradle tasks in separate debug tabs, Debug all tasks on the execution graph, Debugging/Profiling/Coverage disabled toggles, Open/redirect output, Show console when output, logs. | — | — | Run \| Edit Configurations… \| + \| Gradle | P0 |
| GRADLE-111 | Task and argument completion | Completion of task paths (`:app:test`), command-line options (`--info`, `--tests`), and source-set names in the run field. | Ctrl+Space | Ctrl+Space | Run configuration editor, Run Anything | P1 |
| GRADLE-112 | Gradle console output | Task output appears in the Run tool window (tree of task executions on the left, console on the right; "Toggle view" between tree and raw text). Failed tasks link to the failing line. | — | — | Run tool window | P0 |
| GRADLE-113 | Build output in Build tool window | Build Project via Gradle shows a "Build Output" tab with task tree, compiler errors navigable to source, and raw console. | — | — | Build tool window | P0 |
| GRADLE-114 | Debug Gradle build scripts | Breakpoints in `build.gradle(.kts)` are honoured when running tasks in debug with "Debug Gradle scripts" enabled. | Shift+F9 | Ctrl+D | Run configuration option | P2 |
| GRADLE-115 | Rerun / stop | Rerun last Gradle execution; stop terminates the build (cancellation via Tooling API). | Ctrl+F5 / Ctrl+F2 | Cmd+R / Cmd+F2 | Run tool window | P0 |
| GRADLE-116 | Continuous build | Running with `--continuous` keeps watching; inferred support in console. (inferred) | — | — | Run configuration args | P2 |
| GRADLE-117 | Environment / gradle.properties handoff | Project and system properties (`-P`, `-D`) and env vars per configuration; "Pass parent environment variables" toggle (inferred). | — | — | Run configuration | P1 |

### 1.9 Test runner integration (Gradle)

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| GRADLE-120 | Gradle test execution with IDE test tree | Running tests via Gradle (`test --tests <filter>`) streams results into the standard test runner UI (tree, pass/fail, timing, output per test, diff for assertion failures). | Ctrl+Shift+F10 | Ctrl+Shift+R | Gutter run icons, context menu | P0 |
| GRADLE-121 | Test filter generation | Selecting a class/method/package generates the right `--tests` filter and target `Test` task (task chooser when several test tasks cover the source set, e.g. `test` vs `integrationTest`). | — | — | Automatic; popup | P0 |
| GRADLE-122 | Rerun failed tests via Gradle | Reruns only the failed tests with filters. | — | — | Test runner toolbar | P1 |
| GRADLE-123 | Debug tests via Gradle | Debugging a test attaches to the forked test JVM. | Shift+F9 (rerun debug) | Ctrl+D | Gutter, context menu | P0 |
| GRADLE-124 | Up-to-date test rerun | IDE forces test task re-execution (`--rerun` / cleanTest) so "up-to-date" does not hide results (inferred mechanism). | — | — | Automatic | P1 |

### 1.10 Build script editing: Kotlin DSL and Groovy DSL

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| GRADLE-130 | Kotlin DSL highlighting and resolve | `.gradle.kts` scripts are analysed as Kotlin scripts with the Gradle script classpath (requires a successful sync to supply script dependencies); full Kotlin editing features. | — | — | Editor | P0 |
| GRADLE-131 | Script configuration loading | Kotlin script definitions/classpath are loaded during sync; notification if script configuration is out of date. | — | — | Editor notification | P0 |
| GRADLE-132 | Groovy DSL support | `.gradle` scripts get Groovy highlighting, Gradle-API-aware completion (closures delegate to Gradle types), navigation into Gradle API sources. | — | — | Editor | P0 |
| GRADLE-133 | Completion in dependencies/plugins blocks | Completion for configuration names, plugin ids, task names, extension properties; dependency coordinate completion (group/artifact/version, from local cache and repository search). 2026.2 adds improved "dependency completion" (details unknown). | Ctrl+Space | Ctrl+Space | Editor | P1 |
| GRADLE-134 | Generate / add dependency | Alt+Insert in a build script offers "Add Maven artifact dependency" / plugin (groups `GenerateGradleDslGroup`, `Gradle.GenerateGroup`, actions `Gradle.AddGradleDslDependencyAction`, `AddGradleDslPluginAction`) with artifact search. | Alt+Insert | Cmd+N | Editor in build script | P1 |
| GRADLE-135 | Navigation in build scripts | Ctrl+Click on `project(":x")`, on included builds, on task names, on plugin ids, on version catalog accessors, and into Gradle/plugin API sources. | Ctrl+B | Cmd+B | Editor | P1 |
| GRADLE-136 | Quick documentation for Gradle DSL | Ctrl+Q shows docs for Gradle DSL elements and configurations (messages in `GradleDocumentationBundle`). | Ctrl+Q | F1 | Editor | P2 |
| GRADLE-137 | Gutter run icons in build scripts | Tasks declared in scripts get gutter "run" icons. | — | — | Gutter | P1 |
| GRADLE-138 | `gradle.properties` support | Completion/docs for well-known `org.gradle.*` properties; navigation to usages (module `intellij.gradle.java.properties`). | — | — | Editor | P2 |
| GRADLE-139 | Settings script support | `settings.gradle(.kts)`: `include` entries navigate to sub-projects; unresolvable includes flagged (inferred). | — | — | Editor | P1 |

### 1.11 Version catalogs

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| GRADLE-145 | TOML editing for `libs.versions.toml` | Syntax highlighting, structure for `[versions]`, `[libraries]`, `[bundles]`, `[plugins]`. | — | — | Editor | P1 |
| GRADLE-146 | Catalog accessor resolve | `libs.foo.bar` in Kotlin/Groovy scripts resolves to the TOML entry; Ctrl+Click navigates; Find Usages from TOML entry to scripts. | Ctrl+B / Alt+F7 | Cmd+B / Opt+F7 | Editor | P1 |
| GRADLE-147 | Catalog completion | Completion of `libs.` accessors in build scripts, and of version refs in TOML. | Ctrl+Space | Ctrl+Space | Editor | P1 |
| GRADLE-148 | Unused version catalog entry inspection | `UnusedVersionCatalogEntry` (Style, warning, on by default) flags entries no script uses. | — | — | Editor highlighting | P2 |
| GRADLE-149 | Rename catalog entries | Renaming a TOML key updates accessor usages (inferred). | Shift+F6 | Shift+F6 | Refactor \| Rename | P2 |

### 1.12 Gradle inspections (build scripts)

All enabled by default unless noted; registered for Groovy/Kotlin build scripts, properties or TOML.

| ID | Inspection (short name) | Group | Default level | What it flags | Pri |
|---|---|---|---|---|---|
| GRADLE-160 | Configuration avoidance (`ConfigurationAvoidance`) | Best practices | Warning | Eager task APIs (`create`, `getByName`, `tasks.all`) where lazy `register`/`named`/`configureEach` should be used. | P2 |
| GRADLE-161 | Unrecognized dependency notation (`DependencyNotationArgument`) | Probable bugs | Warning | Dependency arguments Gradle cannot interpret. | P2 |
| GRADLE-162 | Possibly misplaced call to Gradle method (`ForeignDelegate`) | Probable bugs | Weak warning | Calls resolved against an outer closure delegate unexpectedly. | P2 |
| GRADLE-163 | Plugin DSL structure (`IncorrectPluginDslStructure`) | Validity issues | Error | Illegal statements inside `plugins {}`. | P1 |
| GRADLE-164 | Deprecated configurations (`DeprecatedConfigurations`) | Validity issues | Warning | Use of removed/deprecated configurations such as `compile`, `testCompile`. | P1 |
| GRADLE-165 | Redundant Kotlin standard library dependency (`RedundantKotlinStdLibDependency`) | Best practices | Warning | Explicit `kotlin-stdlib` already added by the Kotlin plugin. | P2 |
| GRADLE-166 | Named-arguments dependency notation (`AvoidDependencyNamedArgumentsNotation`) | Best practices | Weak warning | Map-style `group:, name:, version:` notation; quick-fix to GAV string. | P2 |
| GRADLE-167 | Legacy `apply` plugin notation (`AvoidApplyPluginMethod`) | Best practices | Weak warning | `apply plugin:` instead of `plugins {}`. | P2 |
| GRADLE-168 | Duplicate dependencies (`AvoidDuplicateDependencies`) | Best practices | Weak warning | Same dependency declared twice in a configuration. | P2 |
| GRADLE-169 | Duplicate repositories (`AvoidDuplicateRepositories`) | Best practices | Weak warning | Repeated repository declarations. | P2 |
| GRADLE-170 | Describe custom tasks (`TaskMissingDescription`) | Best practices | Weak warning | Registered tasks without `description`. | P2 |
| GRADLE-171 | Repositories in build scripts (`AvoidRepositoriesInBuildGradle`) | Best practices | Weak warning, **off** by default | Repositories declared per-project instead of centrally in settings. | P2 |
| GRADLE-172 | Latest Gradle minor version (`LatestMinorVersion`) | Best practices | Warning | Wrapper not on latest minor of its major line (in `gradle-wrapper.properties`). | P2 |
| GRADLE-173 | Multiple repository URLs (`MultipleRepositoryUrls`) | Probable bugs | Warning | More than one `url` in a single Maven repository block (Groovy). | P2 |
| GRADLE-174 | JCenter repository (`JCenterRepository`) | Probable bugs | Warning | Use of the sunset JCenter repository (Groovy). | P2 |
| GRADLE-175 | Bintray publishing plugin (`BintrayPublishingPlugin`) | Probable bugs | Warning | Obsolete Bintray plugin (Groovy). | P2 |
| GRADLE-176 | Unused version catalog entry | see GRADLE-148 | Warning | — | P2 |

The 2026.1 release describes these as the first batch of "official Gradle best practices" turned into
inspections with quick-fixes.

### 1.13 Settings | Build, Execution, Deployment | Build Tools (| Gradle) — option inventory

| ID | Option | Values / default | Pri |
|---|---|---|---|
| GRADLE-180 | Sync project after changes in the build scripts | checkbox (on) + Any changes / External changes | P0 |
| GRADLE-181 | Gradle user home | path; default `~/.gradle` / `GRADLE_USER_HOME` | P2 |
| GRADLE-182 | Generate *.iml files for modules imported from Gradle | checkbox (off, inferred) | P2 |
| GRADLE-183 | Enable parallel Gradle model fetching for Gradle 7.4+ | checkbox (off for new projects per docs) | P2 |
| GRADLE-184 | Download external annotations for dependencies | checkbox | P2 |
| GRADLE-185 | Linked Gradle projects list | select which build the options below apply to | P1 |
| GRADLE-186 | Build and run using | Gradle (default) / IntelliJ IDEA | P0 |
| GRADLE-187 | Run tests using | Gradle (default) / IntelliJ IDEA / Choose per test | P0 |
| GRADLE-188 | Distribution | Wrapper (default) / Wrapper task / Local installation (+ location) | P0 |
| GRADLE-189 | Gradle JVM | registered JDKs, Project SDK, JAVA_HOME, `#GRADLE_LOCAL_JAVA_HOME`, Download/Add JDK | P0 |
| GRADLE-190 | Gradle JVM criteria | Version + Vendor (only with daemon toolchain file) | P2 |
| GRADLE-191 | "Use these settings for future projects" | checkbox on distribution in New Project flow | P2 |
| GRADLE-192 | Generate module per source set | deprecated, troubleshooting only (hidden/advanced) | P2 |
| GRADLE-193 | Gradle VM options | deprecated; migrated to `org.gradle.jvmargs` | P2 |

Sources (section 1):
- https://www.jetbrains.com/help/idea/gradle.html
- https://www.jetbrains.com/help/idea/work-with-gradle-projects.html
- https://www.jetbrains.com/help/idea/jetgradle-tool-window.html
- https://www.jetbrains.com/help/idea/gradle-settings.html
- https://www.jetbrains.com/help/idea/gradle-jvm-selection.html
- https://www.jetbrains.com/help/idea/work-with-gradle-dependency-diagram.html
- https://www.jetbrains.com/help/idea/run-debug-gradle.html
- https://www.jetbrains.com/help/idea/gradle-run-configuration.html
- https://www.jetbrains.com/idea/whatsnew/2026-1/ , https://www.jetbrains.com/idea/whatsnew/2026-2/
- https://docs.gradle.org/current/userguide/tooling_api.html , https://docs.gradle.org/current/userguide/platforms.html (version catalogs), https://docs.gradle.org/current/userguide/gradle_daemon.html , https://docs.gradle.org/current/userguide/toolchains.html , https://docs.gradle.org/current/userguide/composite_builds.html , https://docs.gradle.org/current/userguide/task_configuration_avoidance.html
- Community sources: `plugins/gradle/resources/messages/GradleBundle.properties`, `platform/external-system-impl/resources/META-INF/ExternalSystemActions.xml`, `plugins/gradle/java/resources/intellij.gradle.java.xml` (and `.groovy`, `.toml`, `.properties` variants), `ExternalSystemBundle.properties`.

---

## 2. JDK management and language level

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-JDK-001 | Project Structure dialog | Central dialog: Project (SDK, language level, compiler output), Modules, Libraries, Facets, Artifacts, Platform Settings (SDKs, Global Libraries), Problems. | Ctrl+Alt+Shift+S | Cmd+; | File \| Project Structure… | P0 |
| JAVA-JDK-002 | Project SDK | Default JDK for all modules; used for resolve, highlighting and (internal) build. | — | — | Project Structure \| Project | P0 |
| JAVA-JDK-003 | Module SDK | Per-module override ("Project SDK" or specific JDK); for Gradle modules derived from toolchain (GRADLE-051). | — | — | Project Structure \| Modules \| Dependencies | P0 |
| JAVA-JDK-004 | SDK registry (Platform Settings \| SDKs) | List of registered JDKs: add from disk, rename, remove; per-SDK classpath, sourcepath, annotations and documentation paths. | — | — | Project Structure \| SDKs | P0 |
| JAVA-JDK-005 | Auto-detection of installed JDKs | Detects JDKs in standard locations (`JAVA_HOME`, `~/.jdks`, `/usr/lib/jvm`, SDKMAN, `/Library/Java/JavaVirtualMachines`, Windows registry) and offers them in pickers. | — | — | SDK combo boxes "Detected SDKs" | P0 |
| JAVA-JDK-006 | Download JDK dialog | Pick Version and Vendor (e.g. Oracle OpenJDK, Eclipse Temurin, Amazon Corretto, Azul Zulu, GraalVM, JetBrains Runtime, BellSoft Liberica, IBM Semeru, SAP Machine, Microsoft); choose install location (default `~/.jdks`); downloads, verifies and registers. Includes Java early-access builds (2026.1). | — | — | SDK combo \| Download JDK…; Project Structure \| SDKs \| + | P0 |
| JAVA-JDK-007 | Missing SDK banner | Editor notification "Project JDK is not defined"/"Setup SDK" with quick picker. | — | — | Editor banner | P0 |
| JAVA-JDK-008 | Project language level | Combo of Java versions (e.g. 8 … 27, plus "X (Preview)" levels and "SDK default"). Determines allowed syntax and inspections. | — | — | Project Structure \| Project | P0 |
| JAVA-JDK-009 | Module language level | Per-module override (from Gradle `sourceCompatibility`/`release`). | — | — | Project Structure \| Modules \| Sources | P0 |
| JAVA-JDK-010 | Language-level quick-fixes | Using newer syntax offers "Set language level to N" (for Gradle: edits or suggests build file change, inferred). | Alt+Enter | Opt+Enter | Intention | P0 |
| JAVA-JDK-011 | Preview features handling | Preview language levels enable preview syntax; `PreviewFeature` warning; `--enable-preview` passed to compiler/run (inferred). | — | — | Language level combo | P1 |
| JAVA-JDK-012 | JDK sources & docs | Attaches `src.zip` for navigation; external Javadoc URL for Shift+F1. | — | — | SDK settings | P1 |
| JAVA-JDK-013 | Compiler output paths | Project compiler output and per-module output/test output (internal builder only). | — | — | Project Structure \| Project / Modules \| Paths | P1 |
| JAVA-JDK-014 | Libraries management | Project/global libraries, add jar/dir, attach sources/Javadoc, "From Maven…" download. | — | — | Project Structure \| Libraries | P1 |
| JAVA-JDK-015 | Problems page | Lists invalid SDKs/libraries/dependencies with fixes. | — | — | Project Structure \| Problems | P2 |
| JAVA-JDK-016 | Choose Boot Java Runtime for the IDE | Selects the runtime the IDE itself runs on (platform-level). | — | — | Find Action | P2 |
| JAVA-JDK-017 | Supported Java versions | 2026.2 announces Java 27 support; 2026.1 Java 26 (incl. preview pattern features, lazy constants). | — | — | — | P0 |

Sources (section 2):
- https://www.jetbrains.com/help/idea/sdk.html
- https://www.jetbrains.com/help/idea/project-settings-and-structure.html
- https://www.jetbrains.com/help/idea/supported-java-versions.html
- https://www.jetbrains.com/help/idea/whatsnew (2026.1, 2026.2 pages)

---
## 3. Java editing

### 3.1 Highlighting and analysis levels

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-EDIT-001 | Lexer/syntax highlighting | Keywords, literals, comments, Javadoc tags, annotations, text blocks, string templates escapes coloured per colour scheme. | — | — | Editor | P0 |
| JAVA-EDIT-002 | Semantic highlighting | Resolved references coloured by kind: fields (instance/static), local vars, parameters, reassigned vars (underlined), methods (static/abstract/constructors), types (class/interface/enum/record/type params), implicit/anonymous; unused symbols greyed. | — | — | Editor | P0 |
| JAVA-EDIT-003 | Error highlighting | Compile errors (unresolved symbols, type mismatch, unhandled exceptions, missing return, …) underlined red as you type, without a build. | — | — | Editor | P0 |
| JAVA-EDIT-004 | Error stripe and traffic light | Right gutter marks for errors/warnings; widget at top shows counts and navigates. | F2 / Shift+F2 (next/prev highlighted error) | F2 / Shift+F2 | Editor | P0 |
| JAVA-EDIT-005 | Highlighting level per file | "Highlight: All Problems / Syntax / None" and "Configure Current File Analysis" popup (also Power Save Mode, Reader Mode). Action `PopupHector`. | Ctrl+Alt+Shift+H | Ctrl+Opt+Shift+H | Traffic-light widget; Find Action | P1 |
| JAVA-EDIT-006 | Error description tooltip | Shows the full problem description at caret. | Ctrl+F1 | Cmd+F1 | Hover / action | P0 |
| JAVA-EDIT-007 | Reader mode for library sources | Read-only/decompiled files render Javadoc as rich text with ligatures and hints (toggle). | Ctrl+Alt+Q (toggle rendered doc) | Ctrl+Opt+Q | Editor top-right | P2 |
| JAVA-EDIT-008 | Decompiler for `.class` files | Opening a library class without sources shows decompiled Java (Fernflower) with a "Download Sources"/"Choose Sources" banner. | — | — | Editor | P0 |
| JAVA-EDIT-009 | Brace/paren matching & highlighting | Highlights matching delimiters; jump to matching brace. | Ctrl+Shift+M | Ctrl+M | Editor | P0 |
| JAVA-EDIT-010 | Gutter markers | Override/implement arrows, recursion icon, run/test icons, breakpoints, lambda/functional markers, colour/icon previews, VCS change markers. | — | — | Left gutter | P0 |
| JAVA-EDIT-011 | Language injections | Regex, SQL, JSON, HTML etc. injected into Java string literals by annotation (`@Language`) or by known APIs (e.g. `Pattern.compile`); edit fragment in separate editor. | Alt+Enter | Opt+Enter | Intention "Inject language or reference" | P2 |

### 3.2 Code completion

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-EDIT-020 | Basic completion | Context-sensitive list of visible symbols, keywords, members; auto-popup while typing; camel-hump and substring matching; ranking by relevance/ML. | Ctrl+Space | Ctrl+Space | Code \| Code Completion \| Basic | P0 |
| JAVA-EDIT-021 | Second basic completion (non-imported classes, inaccessible members) | Invoking basic completion twice widens scope: classes not yet imported, static members of unimported classes, chained getters. Action id `ClassNameCompletion`. | Ctrl+Space ×2 / Ctrl+Alt+Space | Ctrl+Space ×2 | Same | P0 |
| JAVA-EDIT-022 | Type-matching (smart) completion | Lists only items whose type fits the expected type (assignments, arguments, return, casts, `new` expressions). | Ctrl+Shift+Space | Ctrl+Shift+Space | Code \| Code Completion \| Type-Matching | P1 |
| JAVA-EDIT-023 | Second smart completion | Pressing twice adds chained calls/collections/arrays converted to the expected type. | Ctrl+Shift+Space ×2 | Ctrl+Shift+Space ×2 | Same | P2 |
| JAVA-EDIT-024 | Statement completion | Completes the current construct: adds parentheses, braces, semicolons, formats, moves caret to the next logical position (`if (` → `if () {}` etc.). Action id `EditorCompleteStatement`. | Ctrl+Shift+Enter | Cmd+Shift+Enter | Code \| Code Completion \| Complete Current Statement | P0 |
| JAVA-EDIT-025 | Completion insertion modes | Enter inserts; Tab replaces identifier to the right; `.` inserts + dot (Ctrl+.); `!` negates boolean; Ctrl+Shift+Enter in lookup completes statement. | Enter / Tab / Ctrl+. | Enter / Tab / Ctrl+. | Lookup | P0 |
| JAVA-EDIT-026 | Method argument placeholders / template | Completing a method may insert parameter-name placeholders to tab through (setting). | Tab | Tab | Lookup | P1 |
| JAVA-EDIT-027 | Member/name suggestions | Suggests variable/field/parameter names derived from the type and expected usage; suggests getter/setter/override method names when typing in class body. | Ctrl+Space | Ctrl+Space | Declaration context | P0 |
| JAVA-EDIT-028 | Postfix completion | Expression-suffix templates (see 3.4) appear in the completion list and expand with Tab. | Tab | Tab | Lookup | P0 |
| JAVA-EDIT-029 | Command completion (`..`) | Typing `..` (or `.` in some contexts) lists IDE actions applicable at the caret (refactorings, generate, postfix templates, fixes) inside the completion popup. 2026.1 exposes all postfix templates through it. | `..` | `..` | Editor | P2 |
| JAVA-EDIT-030 | Lookup quick doc / quick definition | While the lookup is open, show docs/definition for the selected item. | Ctrl+Q / Ctrl+Shift+I | F1 / Opt+Space | Lookup | P1 |
| JAVA-EDIT-031 | Lookup sorting and filtering options | Sort lexicographically vs relevance; match case setting; show/hide "useless" items. | — | — | Lookup menu; Settings \| Editor \| General \| Code Completion | P2 |
| JAVA-EDIT-032 | Hippie (cyclic word) completion | Completes from words in open files. | Alt+/ ; Alt+Shift+/ | Opt+/ ; Opt+Shift+/ | Code \| Code Completion \| Cyclic Expand Word | P2 |
| JAVA-EDIT-033 | Exception, annotation, keyword-aware completion | `catch (` suggests thrown exceptions; annotation attribute names/values; `@Override`-style annotations; `switch` case labels for enums/sealed types. | Ctrl+Space | Ctrl+Space | Editor | P1 |
| JAVA-EDIT-034 | Completion in string literals | File paths, property keys, regex, format strings where the target type is known (inferred list). | Ctrl+Space | Ctrl+Space | Editor | P2 |
| JAVA-EDIT-035 | Full-line / inline completion (local ML) | Grey-text whole-line proposals by a local model; accept with Tab, word with Alt+], cycle Alt+[ . | Tab / Alt+] / Alt+[ | Tab / Opt+] / Opt+[ | Editor | P2 |
| JAVA-EDIT-036 | Code completion settings | Settings \| Editor \| General \| Code Completion: case sensitivity, auto-insert single suggestion, show auto-popup, delay, parameter info popup, sort by relevance, ML ranking, Java-specific "show static members after instance qualifier". | — | — | Settings | P1 |

### 3.3 Live templates (Java group)

Live templates are named snippets expanded by typing the abbreviation and Tab (or via Insert Live Template).
Shortcuts: Insert Live Template `Ctrl+J` / `Cmd+J`; Surround with Live Template `Ctrl+Alt+J` / `Cmd+Opt+J`;
Surround With (statements and templates) `Ctrl+Alt+T` / `Cmd+Opt+T`. Settings: Editor | Live Templates.
Template set names and descriptions are from `java/java-backend/resources/liveTemplates/*.xml` (set "Java").

| ID | Abbrev | Expands to (our wording) | Pri |
|---|---|---|---|
| JAVA-LT-001 | `psvm` / `main` | `main` method declaration (form adapts to language level: classic `public static void main(String[] args)`, or instance/implicit-class `void main()` on new levels). | P0 |
| JAVA-LT-002 | `sout` | Print a value to standard output. | P0 |
| JAVA-LT-003 | `soutv` | Print "name = value" for the nearest variable. | P0 |
| JAVA-LT-004 | `soutm` | Print current class and method name. | P1 |
| JAVA-LT-005 | `soutp` | Print method parameter names and values. | P1 |
| JAVA-LT-006 | `souf` | `System.out.printf` with format string. | P1 |
| JAVA-LT-007 | `serr` | Print to standard error. | P1 |
| JAVA-LT-008 | `soutc` / `serrc` | Print using the `IO` console API variant for newer language levels (inferred from template set: "c" variants). | P2 |
| JAVA-LT-009 | `iop` / `ior` | Print / read a string using the `java.lang.IO` API (Java 25+). | P2 |
| JAVA-LT-010 | `fori` | Indexed `for` loop. | P0 |
| JAVA-LT-011 | `iter` | Enhanced `for` over an Iterable or array (variable picked from context). | P0 |
| JAVA-LT-012 | `itar` | Indexed loop over array elements. | P1 |
| JAVA-LT-013 | `ritar` | Reverse indexed loop over array. | P2 |
| JAVA-LT-014 | `itli` | Indexed loop over a `List`. | P1 |
| JAVA-LT-015 | `itco` | Iterator-based loop over a `Collection`. | P2 |
| JAVA-LT-016 | `itit` | `while (it.hasNext())` loop over an `Iterator`. | P2 |
| JAVA-LT-017 | `iten` | Loop over an `Enumeration`. | P2 |
| JAVA-LT-018 | `itm` | Loop over `Map` entries (keys and values). | P1 |
| JAVA-LT-019 | `ittok` | Loop over `StringTokenizer` tokens. | P2 |
| JAVA-LT-020 | `ifn` | `if (x == null)` block. | P1 |
| JAVA-LT-021 | `inn` | `if (x != null)` block. | P1 |
| JAVA-LT-022 | `inst` | `instanceof` check with cast (pattern form on newer levels). | P1 |
| JAVA-LT-023 | `lst` | Last element of an array. | P2 |
| JAVA-LT-024 | `mn` / `mx` | Assign min / max of variable and expression. | P2 |
| JAVA-LT-025 | `toar` | Copy a collection into an array. | P2 |
| JAVA-LT-026 | `lazy` | Lazy-initialisation `if (x == null) x = …`. | P2 |
| JAVA-LT-027 | `geti` | Singleton `getInstance()` method. | P2 |
| JAVA-LT-028 | `psf`, `prsf`, `psfi`, `psfs` | `public static final` / `private static final` / `… int` / `… String` modifiers. | P1 |
| JAVA-LT-029 | `St` | `String` type. | P2 |
| JAVA-LT-030 | `thr` | `throw new`. | P1 |
| JAVA-LT-031 | `else-if` | Adds an `else if` branch. | P2 |
| JAVA-LT-032 | `sts` (+ `stss`, `stsf` preview-level variants) | Create and open a `StructuredTaskScope` (structured concurrency). | P2 |
| JAVA-LT-033 | Surround templates `C`, `RL`, `WL`, `I` | Surround with `Callable`, `ReadWriteLock.readLock`, `…writeLock`, and iterate (`for` over selected Iterable). | P2 |
| JAVA-LT-034 | Platform-wide templates usable in Java | `todo`/`fixme` comments; "Surround with" `<tag>` etc. (other groups). | P2 |
| JAVA-LT-035 | Template editor | Create/edit templates with variables and expressions (`className()`, `methodName()`, `suggestVariableName()`, `typeOfVariable()`, `iterableComponentType()` …), contexts (Java: statement, expression, declaration, comment, string, …), expand key, reformat option. | P1 |
| JAVA-LT-036 | Template variables navigation | Tab / Shift+Tab move between template variables; Enter finishes. | P0 |

### 3.4 Postfix completion (Java)

Expanded by typing `expr.<key>` then Tab (configurable: Tab / Space / Enter); `Ctrl+J` lists applicable ones.
Settings: Editor | General | Postfix Completion. Key list from public docs and observation (inferred where marked).

| ID | Key | Result | Pri |
|---|---|---|---|
| JAVA-PFX-001 | `.var` | Introduce local variable for expression. | P0 |
| JAVA-PFX-002 | `.if` | `if (expr) {}` for boolean. | P0 |
| JAVA-PFX-003 | `.not` | Negate boolean expression. | P1 |
| JAVA-PFX-004 | `.for` / `.iter` | Enhanced for over iterable/array. | P0 |
| JAVA-PFX-005 | `.fori` / `.forr` | Indexed loop ascending / descending (for number, array, list). | P1 |
| JAVA-PFX-006 | `.null` / `.nn` (`.notnull`) | `if (expr == null)` / `if (expr != null)`. | P1 |
| JAVA-PFX-007 | `.return` | `return expr;`. | P1 |
| JAVA-PFX-008 | `.sout` / `.serr` / `.soutv` / `.souf` | Print expression. | P0 |
| JAVA-PFX-009 | `.cast` / `.castvar` | Cast expression (and assign to variable). | P1 |
| JAVA-PFX-010 | `.inst` / `.instanceof` | `instanceof` check with pattern. | P1 |
| JAVA-PFX-011 | `.field` | Introduce field. | P1 |
| JAVA-PFX-012 | `.par` | Wrap in parentheses. | P2 |
| JAVA-PFX-013 | `.new` | `new T()` for a type reference. | P2 |
| JAVA-PFX-014 | `.arg` | Wrap as argument of a call to be typed. | P2 |
| JAVA-PFX-015 | `.lambda` | Wrap in `() -> expr`. | P2 |
| JAVA-PFX-016 | `.opt` | `Optional.ofNullable(expr)`. | P2 |
| JAVA-PFX-017 | `.stream` | `Arrays.stream(array)`. | P2 |
| JAVA-PFX-018 | `.switch` | `switch (expr)` (enum/sealed/primitive). | P1 |
| JAVA-PFX-019 | `.synchronized` | `synchronized (expr) {}`. | P2 |
| JAVA-PFX-020 | `.throw` | `throw expr;` for Throwable. | P1 |
| JAVA-PFX-021 | `.try` / `.twr` | Surround with try-catch / try-with-resources. | P1 |
| JAVA-PFX-022 | `.while` / `.else` | `while (expr)` / `if (!expr)`. | P2 |
| JAVA-PFX-023 | `.format` | `String.format(expr, …)`. | P2 |
| JAVA-PFX-024 | `.reqnonnull` | `Objects.requireNonNull(expr)`. | P2 |
| JAVA-PFX-025 | `.assert` | `assert expr;`. | P2 |
| JAVA-PFX-026 | `.serr` / `.soutv` variants and `.optional` aliases (inferred; exact key list to be verified, see Open questions) | — | P2 |
| JAVA-PFX-027 | Custom postfix templates | User-defined templates with applicable-type conditions (void, non-void, boolean, number, not primitive, array, array of non-primitive, specific class) and minimum language level. | P2 |

### 3.5 Parameter info, documentation, type info

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-EDIT-040 | Parameter info | Popup with overload signatures, current parameter in bold; auto-shows after `(` (configurable delay). | Ctrl+P | Cmd+P | View \| Parameter Info | P0 |
| JAVA-EDIT-041 | Overload switching | Switch among overloads while editing arguments. | Ctrl+Up/Down | Opt+Up/Down | Editor | P2 |
| JAVA-EDIT-042 | Quick documentation | Popup with rendered Javadoc, signature, annotations (incl. inferred nullability/contracts), links; can be pinned as a tool window; also on mouse hover. | Ctrl+Q | F1 (or Ctrl+J) | View \| Quick Documentation | P0 |
| JAVA-EDIT-043 | External documentation | Opens Javadoc in browser (SDK/library doc URL). | Shift+F1 | Shift+F1 | View \| External Documentation | P2 |
| JAVA-EDIT-044 | Quick definition | Shows the source of the symbol in a popup. | Ctrl+Shift+I | Opt+Space / Cmd+Y | View \| Quick Definition | P1 |
| JAVA-EDIT-045 | Expression type info | Shows type of selected expression. | Ctrl+Shift+P | Ctrl+Shift+P | View \| Type Info | P1 |
| JAVA-EDIT-046 | Context info | Shows the enclosing declaration header when it is scrolled away. | Alt+Q | Ctrl+Shift+Q | View \| Context Info | P2 |
| JAVA-EDIT-047 | Sticky lines | Class/method headers stick to the top of the editor while scrolling. | — | — | Editor; Settings \| Editor \| General \| Sticky Lines | P2 |
| JAVA-EDIT-048 | Breadcrumbs | Path of class › method › lambda at bottom/top of editor. | — | — | Editor | P2 |

### 3.6 Inlay hints and code vision

Settings: Editor | Inlay Hints | Java (each type toggleable).

| ID | Feature | Behaviour | Pri |
|---|---|---|---|
| JAVA-EDIT-050 | Parameter name hints | Show parameter names before literal/null/complex arguments; exclude list. | P1 |
| JAVA-EDIT-051 | Implicit type hints for `var` | Show inferred type after `var` declarations (off by default, inferred). | P2 |
| JAVA-EDIT-052 | Method chain type hints | Type of each call in multi-line chains (streams, builders). | P2 |
| JAVA-EDIT-053 | Annotation hints | Inferred annotations (nullability, `@Contract`) and external annotations. | P2 |
| JAVA-EDIT-054 | Code vision: usages | "N usages" above classes/methods; click opens usages. | P1 |
| JAVA-EDIT-055 | Code vision: inheritors / implementations | "N implementations/inheritors/overrides". | P1 |
| JAVA-EDIT-056 | Code vision: related problems | Count of compilation problems in other files caused by this declaration's change. | P2 |
| JAVA-EDIT-057 | Code vision: VCS author | Last author(s) of the declaration. | P2 |
| JAVA-EDIT-058 | Code vision: JVM language/test status hints | e.g. "no usages" style markers (inferred). | P2 |
| JAVA-EDIT-059 | Value hints for `switch`/`yield` & lambda return (inferred) | — | P2 |

### 3.7 Folding

| ID | Feature | Behaviour | Win/Linux | macOS | Pri |
|---|---|---|---|---|---|
| JAVA-EDIT-060 | Fold/unfold regions | Collapse/expand the region at caret, recursively, all, to level N. | Ctrl+- / Ctrl+= ; Ctrl+Alt+- / Ctrl+Alt+= ; Ctrl+Shift+- / Ctrl+Shift+= ; Ctrl+* , 1..5 | Cmd+- / Cmd+= ; Cmd+Opt+- / … ; Cmd+Shift+- / … | P0 |
| JAVA-EDIT-061 | Default folding of imports, file header, Javadoc | Imports and leading comment folded on open (setting). | — | — | P1 |
| JAVA-EDIT-062 | Closure-style folding of anonymous classes / lambdas | Anonymous single-method classes shown as lambda-like fold. | — | — | P2 |
| JAVA-EDIT-063 | Getter/setter one-liners folding | Simple accessors folded into one line. | — | — | P2 |
| JAVA-EDIT-064 | Generic constructor parameters, `final var` and i18n string folding | Fold redundant type args; property keys shown as values (inferred). | — | — | P2 |
| JAVA-EDIT-065 | Custom folding regions | `//region … //endregion` and `<editor-fold>` comments; Surround with region; Go to custom region. | Ctrl+Alt+T; Ctrl+Alt+. | Cmd+Opt+T; Cmd+Opt+. | P2 |
| JAVA-EDIT-066 | Fold selection / code block | Fold arbitrary selection or the enclosing block. | Ctrl+. / Ctrl+Shift+. | Cmd+. / Cmd+Shift+. | P2 |

### 3.8 Imports

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-EDIT-070 | Import popup / quick-fix | Unresolved class shows a hint "Import class?"; Alt+Enter offers import choices (multiple candidates sorted by proximity). | Alt+Enter | Opt+Enter | Editor | P0 |
| JAVA-EDIT-071 | Auto-import on completion | Selecting an unimported class in completion inserts the import. | — | — | Completion | P0 |
| JAVA-EDIT-072 | Add unambiguous imports on the fly | Silently adds imports when exactly one candidate exists (setting, off by default — inferred). | — | — | Settings \| Editor \| General \| Auto Import | P1 |
| JAVA-EDIT-073 | Optimize imports on the fly | Removes unused imports and reorders automatically while editing (setting). | — | — | Same | P1 |
| JAVA-EDIT-074 | Optimize Imports action | Removes unused, sorts according to the import layout, collapses to `*` per thresholds. | Ctrl+Alt+O | Ctrl+Opt+O | Code \| Optimize Imports | P0 |
| JAVA-EDIT-075 | Exclude from auto-import | Packages/classes excluded from import suggestions (e.g. `java.awt.List`). | — | — | Settings \| Editor \| General \| Auto Import | P1 |
| JAVA-EDIT-076 | Static import intentions | Add single-member / on-demand static import; expand static import; "Static import can be used based on the auto-import table" inspection. | Alt+Enter | Opt+Enter | Intention | P1 |
| JAVA-EDIT-077 | Insert imports on paste | When pasting code, offers/adds imports for referenced classes (Always / Ask / Never). | — | — | Settings \| Editor \| General \| Auto Import | P1 |
| JAVA-EDIT-078 | Import layout and thresholds | Code style: class count to use `*` (default 5), names count for static `*` (default 3), packages always with `*`, import order layout (static first/last, module imports), use FQN options. | — | — | Settings \| Editor \| Code Style \| Java \| Imports | P1 |
| JAVA-EDIT-079 | Module imports (`import module`) | Supports module import declarations on supporting language levels (inferred, Java 25). | — | — | Editor | P2 |

### 3.9 Formatting and code style (Java)

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-FMT-001 | Reformat Code | Formats file or selection per code style; also "only changed lines (VCS)" option. | Ctrl+Alt+L | Cmd+Opt+L | Code \| Reformat Code | P0 |
| JAVA-FMT-002 | Reformat File dialog | Options: scope (file / selection / only VCS-changed text), optimize imports, rearrange entries, code cleanup. | Ctrl+Alt+Shift+L | Cmd+Opt+Shift+L | Code \| Reformat File… | P1 |
| JAVA-FMT-003 | Auto-indent lines | Re-indents lines only. | Ctrl+Alt+I | Ctrl+Opt+I | Code \| Auto-Indent Lines | P1 |
| JAVA-FMT-004 | Rearrange code | Reorders members per Arrangement rules (fields, constructors, methods, keep getters/setters together, overridden methods order). | — | — | Code \| Rearrange Code | P2 |
| JAVA-FMT-005 | Code style: Tabs and Indents | Use tab character, tab size (4), indent (4), continuation indent (8), keep indents on empty lines, label indent. | — | — | Settings \| Editor \| Code Style \| Java | P0 |
| JAVA-FMT-006 | Code style: Spaces | Before parentheses, around operators, before left brace, within parentheses/brackets, in ternary, type arguments, etc. | — | — | Same \| Spaces | P1 |
| JAVA-FMT-007 | Code style: Wrapping and Braces | Hard wrap at (120), wrap on typing, keep when reformatting (line breaks, comments, simple blocks in one line), per-construct wrapping (params, call args, chained calls, binary ops, ternary, `extends`/`throws` lists, annotations, enum constants, records components), brace placement (end of line / next line), force braces (`if`, `for`, `while`, `do`). | — | — | Same \| Wrapping and Braces | P1 |
| JAVA-FMT-008 | Code style: Blank Lines | Keep maximum blank lines (in declarations/code/before `}`), minimum blank lines around class/field/method/initializer, after imports, etc. | — | — | Same \| Blank Lines | P1 |
| JAVA-FMT-009 | Code style: JavaDoc | Alignment of parameter descriptions, blank lines between groups, wrap at right margin, generate `<p>` on empty lines, keep empty `@param`/`@return`/`@throws`, enable/disable Javadoc formatting. | — | — | Same \| JavaDoc | P2 |
| JAVA-FMT-010 | Code style: Imports | See JAVA-EDIT-078. | — | — | Same \| Imports | P1 |
| JAVA-FMT-011 | Code style: Arrangement | Matching/grouping rules for Rearrange. | — | — | Same \| Arrangement | P2 |
| JAVA-FMT-012 | Code style: Code Generation | Naming prefixes/suffixes for fields, static fields, parameters, locals; final modifiers for generated locals/params; comment generation style (line comment at first column, add space); default visibility; `@Override` insertion; `var` preference; replace `null` check with `Objects` (inferred). | — | — | Same \| Code Generation | P1 |
| JAVA-FMT-013 | Code style: Wrapping of records, switch, text blocks, lambdas | Newer construct-specific options (e.g. "Record components", "Switch case labels", "Multiline text block" indent). | — | — | Same | P2 |
| JAVA-FMT-014 | Scheme management | Project vs IDE schemes; import/export (IntelliJ XML, Eclipse XML formatter profiles, Checkstyle via plugin); "Set from…" predefined styles (e.g. Google Java Style is plugin/import, inferred). | — | — | Code Style settings gear | P1 |
| JAVA-FMT-015 | EditorConfig support | `.editorconfig` overrides indentation and IntelliJ-specific `ij_java_*` properties. | — | — | Automatic | P1 |
| JAVA-FMT-016 | Formatter markers | `@formatter:off` / `@formatter:on` comments disable formatting regions (setting). | — | — | Code Style \| Formatter tab | P2 |
| JAVA-FMT-017 | Smart Enter/typing assists | Auto-close brackets/quotes, smart indent on Enter, Javadoc stub on `/**` + Enter, surround selection with quotes/brackets, smart Backspace (unindent), auto-insert `;` (inferred), paste-reformat. | Enter | Enter | Editor | P0 |
| JAVA-FMT-018 | Join lines (smart) | Joins lines merging declarations and assignments, string concatenations, if-conditions. | Ctrl+Shift+J | Ctrl+Shift+J | Edit \| Join Lines | P1 |
| JAVA-FMT-019 | Comment / uncomment | Line comment, block comment. | Ctrl+/ ; Ctrl+Shift+/ | Cmd+/ ; Cmd+Opt+/ | Code \| Comment with … | P0 |
| JAVA-FMT-020 | Move statement / line | Move whole statements/members up or down preserving syntax; move lines. | Ctrl+Shift+Up/Down ; Alt+Shift+Up/Down | Cmd+Shift+Up/Down ; Opt+Shift+Up/Down | Code \| Move Statement … | P0 |
| JAVA-FMT-021 | Move element left/right | Swap arguments/parameters/array elements. | Ctrl+Alt+Shift+Left/Right | Cmd+Opt+Shift+Left/Right | Code \| Move Element … | P2 |
| JAVA-FMT-022 | Extend/shrink selection | Grows selection by syntactic units. | Ctrl+W / Ctrl+Shift+W | Opt+Up / Opt+Down | Edit \| Extend Selection | P0 |
| JAVA-FMT-023 | Duplicate line / delete line | — | Ctrl+D / Ctrl+Y | Cmd+D / Cmd+Backspace | Edit | P0 |
| JAVA-FMT-024 | Unwrap/Remove | Removes enclosing `if`/`try`/loop/braces keeping body, or deletes a branch. | Ctrl+Shift+Delete | Cmd+Shift+Delete | Code \| Unwrap/Remove… | P1 |
| JAVA-FMT-025 | Surround With | Wrap selected statements with `if`, `if/else`, `while`, `do/while`, `for`, `try/catch`, `try/finally`, `try/catch/finally`, `synchronized`, `Runnable`, `{}`, region comments, plus surround live templates; expression surrounders: `(expr)`, `((Type) expr)`, `!(expr)`, `if (expr)` etc. | Ctrl+Alt+T | Cmd+Opt+T | Code \| Surround With… | P1 |
| JAVA-FMT-026 | Multi-caret editing | Add caret at next occurrence, select all occurrences, column mode, Alt+Shift+click carets. | Alt+J / Ctrl+Alt+Shift+J / Alt+Shift+Insert | Ctrl+G / Cmd+Ctrl+G / Cmd+Shift+8 | Edit \| Find / Selection | P1 |
| JAVA-FMT-027 | Code Cleanup | Applies all "cleanup" inspection fixes in scope according to profile. | — | — | Code \| Code Cleanup… | P2 |

### 3.10 Javadoc support

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-EDIT-080 | Javadoc stub generation | Typing `/**` + Enter above a declaration generates `@param`/`@return`/`@throws` tags. | Enter | Enter | Editor | P0 |
| JAVA-EDIT-081 | Add/fix Javadoc | "Add Javadoc" intention; "Fix Doc Comment" adds missing/removes stale tags. | Alt+Enter | Opt+Enter | Intention; Code \| Fix Doc Comment | P1 |
| JAVA-EDIT-082 | Javadoc completion & resolve | Completion of tag names, `{@link}` targets, parameter names; references are navigable and refactor-safe. | Ctrl+Space | Ctrl+Space | Editor | P1 |
| JAVA-EDIT-083 | Rendered Javadoc in editor | Toggle to show doc comments rendered as formatted text inline (reader mode); per-comment toggle. | Ctrl+Alt+Q | Ctrl+Opt+Q | Gutter icon | P2 |
| JAVA-EDIT-084 | Markdown documentation comments (`///`) | JEP 467 Markdown doc comments: highlighting, rendering and completion (inferred for full feature set). | — | — | Editor | P2 |
| JAVA-EDIT-085 | Javadoc inspections | Group "Javadoc" (16 inspections) e.g. missing/dangling Javadoc, declaration problems, unresolved references in docs. | — | — | Inspections | P2 |
| JAVA-EDIT-086 | Generate JavaDoc (HTML) | Runs the `javadoc` tool on a scope with options (output dir, visibility level, locale, extra args, open in browser). Action id `GenerateJavadoc`. | — | — | Tools \| Generate JavaDoc… | P2 |
| JAVA-EDIT-087 | Javadoc snippets (`{@snippet}`) | Highlighting/injection of code in `@snippet` tags, external snippet file references (inferred). | — | — | Editor | P2 |

### 3.11 Modern Java language support

| ID | Feature | Behaviour | Pri |
|---|---|---|---|
| JAVA-LANG-001 | Records | Parsing, highlighting, compact/canonical constructors, accessor resolution, `Generate` support, inspections (redundant record constructor, meaningless annotations, serializable record ignored members), intentions (convert class ↔ record, compact ↔ canonical constructor). | P0 |
| JAVA-LANG-002 | Sealed classes/interfaces | `sealed`/`non-sealed`/`permits`; errors for invalid hierarchies; intention "Seal class"; quick-fix to add permits/inheritors; switch exhaustiveness. | P0 |
| JAVA-LANG-003 | Pattern matching for `instanceof` | Type patterns, flow-scoping; inspections suggesting pattern variable use. | P0 |
| JAVA-LANG-004 | Switch expressions & pattern switch | Arrow labels, `yield`, exhaustiveness checks, guarded patterns (`when`), `case null`, record deconstruction patterns; intentions: convert statement ↔ expression, `if` → `switch` ("'if' can be replaced with 'switch'"), create missing branches. | P0 |
| JAVA-LANG-005 | Record patterns | Nested deconstruction, in `for` (preview history), inspection "Enhanced 'for' with a record pattern can be used". | P1 |
| JAVA-LANG-006 | Text blocks | `"""` literals with indentation-aware editing, escapes (`\s`, line continuation), inspections (inconsistent indentation), intentions convert concatenation ↔ text block, fragment editor. | P0 |
| JAVA-LANG-007 | `var` local type inference | Intentions replace `var` ↔ explicit type; completion and hints. | P0 |
| JAVA-LANG-008 | Unnamed variables/patterns (`_`) | Support and quick-fix "replace with unnamed". | P1 |
| JAVA-LANG-009 | Implicitly declared classes & instance main | Compact source files with top-level methods, instance `main`, `java.lang.IO`; matching live templates and run gutter. | P1 |
| JAVA-LANG-010 | Primitive patterns, flexible constructor bodies, module imports, scoped values, structured concurrency, stable/lazy values | Support follows the JDK release cadence including preview levels (Java 26 preview pattern matching + lazy constants in 2026.1, Java 27 in 2026.2). | P1 |
| JAVA-LANG-011 | Lambdas and method references | Full type inference, conversions anonymous ↔ lambda ↔ method reference; inspections in "Java 8" group. | P0 |
| JAVA-LANG-012 | Streams | Stream-chain inspections and intentions (loop ↔ stream, simplify call chains), Stream debugger (see JAVA-DBG). | P1 |
| JAVA-LANG-013 | Generics | Inference, raw-type warnings, unchecked warnings, diamond conversion, "Generify" refactoring. | P0 |
| JAVA-LANG-014 | Java modules (JPMS) | `module-info.java` editing/completion, requires/exports checks, "Generate module-info Descriptors", module-aware resolve. | P1 |
| JAVA-LANG-015 | Annotations and nullability | Recognition of JetBrains, JSpecify, JSR-305, Checker, Android, Lombok nullability annotations; configurable defaults (Settings \| … \| Inspections \| Probable bugs \| Nullability…); `@Contract`, `@Range`, `@MagicConstant`. | P1 |
| JAVA-LANG-016 | String templates | Was a preview then withdrawn; inspection "Redundant embedded expression in string template" exists for older preview levels. | P2 |
| JAVA-LANG-017 | Data-flow analysis | Constant conditions, nullability, ranges, contracts, mutation tracking inter-procedurally within a method plus inferred contracts (bytecode analysis of libraries). | P0 |
| JAVA-LANG-018 | Infer Nullity | Analyze \| Infer Nullity… adds nullability annotations across a scope. Action id `InferNullity`. | P2 |

### 3.12 Lombok support (bundled plugin)

| ID | Feature | Behaviour | Pri |
|---|---|---|---|
| JAVA-LOMBOK-001 | Generated-member resolution | `@Getter`, `@Setter`, `@Data`, `@Value`, `@Builder`, `@SuperBuilder`, `@With`, constructors (`@NoArgsConstructor`, `@AllArgsConstructor`, `@RequiredArgsConstructor`), `@ToString`, `@EqualsAndHashCode`, `@Log*` loggers, `@Delegate`, `@UtilityClass`, `@FieldDefaults`, `val`/`var`, `@Cleanup`, `@SneakyThrows`, `@Synchronized`, `@Accessors`, `@Jacksonized` etc. are visible to completion, navigation, find usages, structure view. | P1 |
| JAVA-LOMBOK-002 | `lombok.config` support | Completion and inspection-aware parsing of config keys. | P2 |
| JAVA-LOMBOK-003 | Lombok inspections | Group "Lombok": Lombok annotations, Deprecated Lombok annotations, Lombok flag usages, `@Qualifier` not copyable, static import of Lombok-generated methods; "Redundant modifiers" for `val`/`@Value`; redundant `@Slf4j` definition; "Lombok @Getter/@Setter may be used". | P2 |
| JAVA-LOMBOK-004 | Delombok | Refactor \| Delombok replaces annotations with generated code (per annotation or all). | P2 |
| JAVA-LOMBOK-005 | Lombokify intentions | Replace explicit getters/setters/etc. with annotations. | P2 |
| JAVA-LOMBOK-006 | Annotation processing prompt | Warns if annotation processing is disabled for Lombok (internal builder; inferred). | P1 |
| JAVA-LOMBOK-007 | Generated members in Structure view | Shown with Lombok marker (inferred). | P2 |

Sources (section 3):
- https://www.jetbrains.com/help/idea/auto-completing-code.html , https://www.jetbrains.com/help/idea/postfix-code-completion.html , https://www.jetbrains.com/help/idea/command-completion.html
- https://www.jetbrains.com/help/idea/using-live-templates.html , https://www.jetbrains.com/help/idea/template-variables.html
- https://www.jetbrains.com/help/idea/viewing-reference-information.html (parameter info, quick doc, type info)
- https://www.jetbrains.com/help/idea/inlay-hints.html , https://www.jetbrains.com/help/idea/code-folding-settings.html
- https://www.jetbrains.com/help/idea/creating-and-optimizing-imports.html
- https://www.jetbrains.com/help/idea/reformat-and-rearrange-code.html , https://www.jetbrains.com/help/idea/code-style-java.html
- https://www.jetbrains.com/help/idea/working-with-code-documentation.html , https://www.jetbrains.com/help/idea/javadocs.html
- https://www.jetbrains.com/help/idea/lombok.html
- https://www.jetbrains.com/help/idea/supported-java-versions.html
- Community sources: `java/java-backend/resources/liveTemplates/Java.xml`, `JavaImplicitClass.xml`, `JavaNormalClassesWithInstanceMain.xml`, `JavaPreviousPreview.xml`, `messages/JavaBundle.properties` (template descriptions), keymaps.

---
## 4. Navigation and search

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-NAV-001 | Search Everywhere | Unified popup with tabs All / Classes / Files / Symbols / Actions / Text; camel-hump and path matching; `:line` suffix. | Shift, Shift | Shift, Shift | Navigate \| Search Everywhere | P0 |
| JAVA-NAV-002 | Go to Class | Finds classes (incl. library classes with "Include non-project items" on second press), inner classes, by FQN or camel humps. | Ctrl+N | Cmd+O | Navigate \| Class… | P0 |
| JAVA-NAV-003 | Go to File | Finds files by name/path, directories with trailing `/`. | Ctrl+Shift+N | Cmd+Shift+O | Navigate \| File… | P0 |
| JAVA-NAV-004 | Go to Symbol | Finds methods, fields, and other symbols by name. | Ctrl+Alt+Shift+N | Cmd+Opt+O | Navigate \| Symbol… | P0 |
| JAVA-NAV-005 | Go to Action | Find any IDE action by name, toggle settings. | Ctrl+Shift+A | Cmd+Shift+A | Help \| Find Action… | P0 |
| JAVA-NAV-006 | Go to Text | Full-text search tab in Search Everywhere. | Ctrl+Alt+Shift+E | Cmd+Opt+Shift+E | Navigate \| Text… | P1 |
| JAVA-NAV-007 | Go to Line:Column | — | Ctrl+G | Cmd+L | Navigate \| Line/Column… | P0 |
| JAVA-NAV-008 | Go to Declaration or Usages | On a reference jumps to its declaration; on a declaration shows usages; Ctrl/Cmd+click. | Ctrl+B / Ctrl+Click | Cmd+B / Cmd+Click | Navigate \| Declaration or Usages | P0 |
| JAVA-NAV-009 | Go to Implementation(s) | Implementations of interface/abstract methods or subclasses; popup when several. | Ctrl+Alt+B | Cmd+Opt+B | Navigate \| Implementation(s) | P0 |
| JAVA-NAV-010 | Go to Type Declaration | Jumps to the declaration of the variable/expression type. | Ctrl+Shift+B | Cmd+Shift+B | Navigate \| Type Declaration | P1 |
| JAVA-NAV-011 | Go to Super Method / Super Class | Navigates to overridden method or superclass. | Ctrl+U | Cmd+U | Navigate \| Super Method | P0 |
| JAVA-NAV-012 | Go to Test / Test Subject | Toggles between class and its test(s); offers "Create New Test…" when none. | Ctrl+Shift+T | Cmd+Shift+T | Navigate \| Test | P0 |
| JAVA-NAV-013 | Go to Related Symbol | e.g. from class to related config/resources (framework-specific). | Ctrl+Alt+Home | Ctrl+Cmd+Up | Navigate \| Related Symbol… | P2 |
| JAVA-NAV-014 | File Structure popup | Members of the current file with filters (show inherited, fields, anonymous classes, lambdas), speed search, navigation. | Ctrl+F12 | Cmd+F12 | Navigate \| File Structure | P0 |
| JAVA-NAV-015 | Structure tool window | Persistent tree of file members with sort (alphabetical, visibility), grouping (by defining type, properties), show inherited/fields/non-public. | Alt+7 | Cmd+7 | View \| Tool Windows \| Structure | P1 |
| JAVA-NAV-016 | Type Hierarchy | Class hierarchy / supertypes / subtypes views; scope selector; "base on this class". | Ctrl+H | Ctrl+H | Navigate \| Type Hierarchy | P1 |
| JAVA-NAV-017 | Method Hierarchy | Which classes in the hierarchy define/override/must implement a method. | Ctrl+Shift+H | Cmd+Shift+H | Navigate \| Method Hierarchy | P1 |
| JAVA-NAV-018 | Call Hierarchy | Callers / callees trees for a method with scope filter. | Ctrl+Alt+H | Ctrl+Opt+H | Navigate \| Call Hierarchy | P1 |
| JAVA-NAV-019 | Find Usages | Searches usages of a symbol across scope; results in Find tool window grouped by module/package/file/usage type (read/write, import, Javadoc, test/production); options dialog (overloads, base methods, text occurrences, inheritors). | Alt+F7 | Opt+F7 | Edit \| Find Usages \| Find Usages | P0 |
| JAVA-NAV-020 | Find Usages settings dialog | Choose options before searching. | Ctrl+Alt+Shift+F7 | Cmd+Opt+Shift+F7 | Edit \| Find Usages \| Find Usages Settings… | P1 |
| JAVA-NAV-021 | Show Usages popup | Inline popup list of usages with preview. | Ctrl+Alt+F7 | Cmd+Opt+F7 | Edit \| Find Usages \| Show Usages | P0 |
| JAVA-NAV-022 | Find usages in file | Highlights usages in the current file; next/previous via F3/Shift+F3. | Ctrl+F7 | Cmd+F7 | Edit \| Find Usages \| Find Usages in File | P1 |
| JAVA-NAV-023 | Highlight usages in file | Highlights identifier usages (read vs write colours) and exit points (`return`/`throw` when on method keyword), thrown exception sources. | Ctrl+Shift+F7 | Cmd+Shift+F7 | Edit \| Find Usages \| Highlight Usages in File | P0 |
| JAVA-NAV-024 | Automatic usage highlighting at caret | Passive highlight of the symbol under caret; navigate highlighted usages. | — (use Ctrl+Alt+Up/Down "Next/Previous Occurrence" in Find results) | Ctrl+Opt+Up/Down | Editor | P0 |
| JAVA-NAV-025 | Usage filtering and grouping in Find tool window | Filter read/write/imports; group by module, directory, file structure, usage type; preview pane; exclude/include; recent searches. | Ctrl+R/W/I, Ctrl+Alt+D/F/O/M/T | Ctrl+… | Find tool window | P1 |
| JAVA-NAV-026 | Recent Files | Popup of recently opened files and tool windows, with "show changed only" toggle. | Ctrl+E | Cmd+E | View \| Recent Files | P0 |
| JAVA-NAV-027 | Recent Locations | Code snippets of recently visited/changed places. | Ctrl+Shift+E | Cmd+Shift+E | View \| Recent Locations | P1 |
| JAVA-NAV-028 | Back / Forward | Navigation history. | Ctrl+Alt+Left/Right | Cmd+[ / Cmd+] | Navigate \| Back / Forward | P0 |
| JAVA-NAV-029 | Last Edit Location | Jumps to last change; cycles. | Ctrl+Shift+Backspace | Cmd+Shift+Backspace | Navigate \| Last Edit Location | P1 |
| JAVA-NAV-030 | Switcher | Ctrl+Tab popup for files and tool windows. | Ctrl+Tab | Ctrl+Tab | Window \| Switcher | P1 |
| JAVA-NAV-031 | Navigation bar | Breadcrumb path to file and member; popup navigation. | Alt+Home | Cmd+Up / Opt+Home | View \| Appearance \| Navigation Bar | P1 |
| JAVA-NAV-032 | Select In | Locate the current element in Project view, Structure, Finder/Explorer, etc. | Alt+F1 | Opt+F1 | Navigate \| Select In… | P1 |
| JAVA-NAV-033 | Next/previous method | Moves caret between member declarations. | Alt+Down/Up | Ctrl+Shift+Down/Up | Navigate \| Next/Previous Method | P2 |
| JAVA-NAV-034 | Next/previous highlighted error | — | F2 / Shift+F2 | F2 / Shift+F2 | Navigate \| Next/Previous Highlighted Error | P0 |
| JAVA-NAV-035 | Bookmarks: toggle | Line bookmark (anonymous). | F11 | F3 | Edit \| Bookmarks \| Toggle Bookmark | P1 |
| JAVA-NAV-036 | Bookmarks: mnemonic | Bookmark with digit/letter; jump by Ctrl+digit. | Ctrl+F11; Ctrl+Shift+0..9 toggle; Ctrl+0..9 go | Opt+F3; Ctrl+Shift+0..9; Ctrl+0..9 | Edit \| Bookmarks | P2 |
| JAVA-NAV-037 | Bookmarks tool window / popup | List, group, rename, describe bookmarks. | Shift+F11 (popup); Alt+2 (window) | Cmd+F3; Cmd+2 | View \| Tool Windows \| Bookmarks | P2 |
| JAVA-NAV-038 | Find / Replace in file | Incremental search bar with regex, case, words, in-selection, filters (in comments/literals). | Ctrl+F / Ctrl+R | Cmd+F / Cmd+R | Edit \| Find | P0 |
| JAVA-NAV-039 | Find / Replace in Files | Project/module/directory/scope search with file mask, preview, open in Find window. | Ctrl+Shift+F / Ctrl+Shift+R | Cmd+Shift+F / Cmd+Shift+R | Edit \| Find \| Find in Files… | P0 |
| JAVA-NAV-040 | Structural Search and Replace | Template-based search of Java code shapes with variables/constraints; saved templates; inspections from templates. | — | — | Edit \| Find \| Search/Replace Structurally… | P2 |
| JAVA-NAV-041 | Project tool window (Java view) | Packages/Project/Project Files views, compact middle packages, show members, flatten packages, external libraries, scratch files. | Alt+1 | Cmd+1 | View \| Tool Windows \| Project | P0 |
| JAVA-NAV-042 | Copy Reference | Copies FQN / `Class#method` reference. | Ctrl+Alt+Shift+C | Cmd+Opt+Shift+C | Edit \| Copy Path/Reference… | P1 |
| JAVA-NAV-043 | Jump to source from tool windows | — | F4 / Ctrl+Enter (view source) | Cmd+Down / Cmd+Enter | Tool windows | P0 |
| JAVA-NAV-044 | Analyze Dependencies / Backward / Cyclic / Module dependencies | Dependency Structure Matrix (DSM, `$`?), dependency viewer of packages/modules. | — | — | Code \| Analyze Code \| Dependencies… | P2 |
| JAVA-NAV-045 | Analyze Data Flow to/from Here | Dataflow-to-here / from-here trees (value origins and destinations). | — | — | Code \| Analyze Code \| Data Flow to/from Here… | P2 |
| JAVA-NAV-046 | Analyze Stack Trace or Thread Dump | Paste a stack trace to get a navigable console (unscramble obfuscated, optional). | — | — | Code \| Analyze Stack Trace or Thread Dump… | P2 |
| JAVA-NAV-047 | Diagrams (UML) | Class diagrams of packages/classes. | Ctrl+Alt+Shift+U (inferred) | Cmd+Opt+Shift+U | Diagrams \| Show Diagram… `$` | P2 |

Sources (section 4):
- https://www.jetbrains.com/help/idea/navigating-through-the-source-code.html , https://www.jetbrains.com/help/idea/searching-everywhere.html
- https://www.jetbrains.com/help/idea/find-highlight-usages.html , https://www.jetbrains.com/help/idea/viewing-structure-and-hierarchy-of-the-source-code.html
- https://www.jetbrains.com/help/idea/bookmarks.html , https://www.jetbrains.com/help/idea/structural-search-and-replace.html
- https://www.jetbrains.com/help/idea/reference-keymap-win.html , https://www.jetbrains.com/help/idea/reference-keymap-mac-default.html
- Community sources: `platform/platform-resources/src/keymaps/$default.xml`, `Mac OS X 10.5+.xml`.

---

## 5. Inspections, intentions and quick-fixes

### 5.1 Model

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-INSP-001 | On-the-fly inspections | Enabled inspections run in the editor as the file changes (local inspections); results highlighted per severity. | — | — | Editor | P0 |
| JAVA-INSP-002 | Severities | Error, Warning, Weak Warning, Server Problem, Grammar Error, Typo, Consideration, No highlighting (fix only), plus "Text attributes" customisation and user-defined severities. Default levels in registrations: ERROR, WARNING, WEAK WARNING, INFORMATION. | — | — | Settings \| Editor \| Inspections | P0 |
| JAVA-INSP-003 | Inspection profiles | Named sets of enabled inspections, severities and options; `Project Default` (stored in `.idea/inspectionProfiles`) and IDE-level `Default`; copy, rename, export/import XML. | — | — | Settings \| Editor \| Inspections; profile combo | P1 |
| JAVA-INSP-004 | Per-scope severity | Each inspection can have different severity/enablement per scope (e.g. Tests, Production, custom scopes). | — | — | Inspections settings | P2 |
| JAVA-INSP-005 | Inspection options | Many inspections have options (e.g. ignore in tests, thresholds, annotation lists). | — | — | Inspections settings right pane | P1 |
| JAVA-INSP-006 | Show Context Actions (Alt+Enter) | Single menu listing quick-fixes (for highlighted problems) first, then intentions; submenu per item: Edit inspection settings, Suppress (for statement/method/class/file via `@SuppressWarnings` or `//noinspection`), Disable inspection, Fix all '…' problems in file, Run inspection on…; preview of change (intention preview). | Alt+Enter | Opt+Enter | Editor; lightbulb icon | P0 |
| JAVA-INSP-007 | Intention preview | Side preview showing the diff before applying a fix/intention. | Ctrl+Q while menu open (inferred) | F1 | Alt+Enter popup | P1 |
| JAVA-INSP-008 | Batch fix in file | "Fix all 'X' problems in file" from quick-fix submenu. | — | — | Alt+Enter submenu | P1 |
| JAVA-INSP-009 | Suppression | `@SuppressWarnings("ShortName")` or comment `//noinspection ShortName`; inspection "Redundant suppression" detects stale suppressions. | — | — | Alt+Enter submenu | P1 |
| JAVA-INSP-010 | Inspect Code (batch) | Runs the profile over a scope (whole project, module, uncommitted files, directory, custom scope, include tests); results in the Problems/Inspection results tool window grouped by severity/inspection/directory; preview, apply fix, suppress, exclude. | — | — | Code \| Inspect Code… | P1 |
| JAVA-INSP-011 | Run Inspection by Name | Runs one inspection over a scope. | Ctrl+Alt+Shift+I | Cmd+Opt+Shift+I | Code \| Analyze Code \| Run Inspection by Name… | P1 |
| JAVA-INSP-012 | Inspect code with editor settings | Batch run of on-the-fly profile on current file. | Alt+Shift+I | Opt+Shift+I | — | P2 |
| JAVA-INSP-013 | Global inspections | Whole-program analyses (unused declarations across project, can-be-final, same parameter value, duplicates) that only run in batch mode or via special passes. | — | — | Inspect Code | P2 |
| JAVA-INSP-014 | Problems tool window | Tabs: **File** (problems in current file), **Project Errors** (compilation problems from the whole-project error analysis), **Server-Side Analysis**, **Vulnerable Dependencies** (`$`, inferred); grouping, sort by severity, autoscroll, show quick-fix. | Alt+6 | Cmd+6 | View \| Tool Windows \| Problems | P0 |
| JAVA-INSP-015 | Code Cleanup | Applies all cleanup-marked fixes over a scope. | — | — | Code \| Code Cleanup… | P2 |
| JAVA-INSP-016 | Silent code cleanup on save / commit | Actions on Save / pre-commit checks can run reformat, optimize imports, rearrange, cleanup, and inspection checks before commit. | — | — | Settings \| Tools \| Actions on Save; Commit options | P2 |
| JAVA-INSP-017 | Command-line inspector | `inspect.sh` / Qodana-like batch run with profile export (platform). | — | — | CLI | P2 |
| JAVA-INSP-018 | Inspection descriptions | Each inspection has an HTML description with example and options shown in settings. | — | — | Settings | P1 |
| JAVA-INSP-019 | Spell checking (Typo) and grammar | Typo inspection on identifiers, comments, strings. | — | — | Editor | P2 |
| JAVA-INSP-020 | Intentions settings | Enable/disable intention actions by category with before/after examples (Settings \| Editor \| Intentions). Java categories: Declaration, Control flow, Expressions, Strings, Annotations, Conditional operator, Refactorings, Try statements, JUnit, Imports, Streams, Concurrency, Comments, Collections, I18n, Other. | — | — | Settings \| Editor \| Intentions | P1 |

### 5.2 Java inspection groups

Counts are the number of Java-related inspection registrations per group in the Community sources
(`InspectionGadgets.xml`, `Inspections.xml`, `GlobalInspections.xml`, JUnit/TestNG/Lombok plugin XML);
about 950 registrations in total, about 480 enabled by default.

| Group | Count | Group | Count |
|---|---|---|---|
| Probable bugs | 112 | Code style issues | 80 |
| Threading issues | 52 | Control flow issues | 50 |
| Performance | 47 | Verbose or redundant code constructs | 44 |
| Class structure | 31 | Numeric issues | 28 |
| Error handling | 28 | Declaration redundancy | 25 |
| Serialization issues | 23 | Inheritance issues | 21 |
| Visibility | 18 | Java language level migration aids (Java 5…26 subgroups) | ~60 |
| JUnit | 17 | Javadoc | 16 |
| Security | 16 | Abstraction issues | 15 |
| Internationalization | 13 | Code maturity | 13 |
| Naming conventions (+ Class/Method subgroups) | 25 | Method metrics / Class metrics | 20 |
| Initialization | 11 | Assignment issues | 11 |
| Embedded (Java in other languages) | 11 | TestNG | 10 |
| Portability | 10 | Resource management | 8 |
| Memory | 8 | Imports | 8 |
| Encapsulation | 8 | Data flow | 8 |
| Packaging issues | 7 | Java language level issues | 7 |
| Dependency issues | 7 | Cloning issues | 7 |
| Modularization issues | 6 | Logging | 6 |
| Concurrency annotation issues | 6 | Test frameworks | 5 |
| Lombok | 5 | JavaBeans issues | 5 |
| Bitwise operation issues | 3 | Finalization | 3 |
| Nullability problems | 3 | Compiler issues | 4 |
| Reflective access | 4 | toString() issues | 2 |

| ID | Feature | Behaviour | Pri |
|---|---|---|---|
| JAVA-INSP-030 | Group tree in settings | Inspections shown in a tree under language (Java) → group → inspection, with search by name/short name/description, filters (enabled, modified, severity). | P1 |
| JAVA-INSP-031 | Language-level gating | Migration-aid inspections only fire when the module language level supports the replacement. | P0 |
| JAVA-INSP-032 | Test-source awareness | Some inspections ignore test sources or have "ignore in tests" options. | P1 |

### 5.3 Representative inspections enabled by default

Short names are the stable identifiers used in `@SuppressWarnings` and profile XML. Level = default
highlighting. All listed are `enabledByDefault="true"` in the Community registrations.

| ID | Short name | Display name | Group | Level | Pri |
|---|---|---|---|---|---|
| JAVA-INSP-101 | `unused` | Unused declaration | Declaration redundancy | Warning | P0 |
| JAVA-INSP-102 | `UNUSED_IMPORT` | Unused import | Imports | Warning | P0 |
| JAVA-INSP-103 | `DataFlowIssue` | Nullability and data flow problems | Probable bugs | Warning | P0 |
| JAVA-INSP-104 | `ConstantValue` | Constant values | Probable bugs | Warning | P0 |
| JAVA-INSP-105 | `UnusedAssignment` | Unused assignment | Probable bugs | Warning | P0 |
| JAVA-INSP-106 | `WrongPackageStatement` | Wrong package statement | Probable bugs | Error | P0 |
| JAVA-INSP-107 | `UNCHECKED_WARNING` | Unchecked warning | Compiler issues | Warning | P0 |
| JAVA-INSP-108 | `RedundantCast` | Redundant type cast | Verbose or redundant code constructs | Warning | P1 |
| JAVA-INSP-109 | `StringEquality` | String comparison using '==', instead of 'equals()' | Probable bugs | Warning | P0 |
| JAVA-INSP-110 | `NumberEquality` | Number comparison using '==', instead of 'equals()' | Probable bugs | Warning | P1 |
| JAVA-INSP-111 | `EqualsBetweenInconvertibleTypes` | 'equals()' between objects of inconvertible types | Probable bugs | Warning | P1 |
| JAVA-INSP-112 | `IgnoreResultOfCall` | Result of method call ignored | Probable bugs | Warning | P1 |
| JAVA-INSP-113 | `OptionalGetWithoutIsPresent` | Optional.get() is called without isPresent() check | Probable bugs | Warning | P1 |
| JAVA-INSP-114 | `MismatchedCollectionQueryUpdate` | Mismatched query and update of collection | Probable bugs | Warning | P1 |
| JAVA-INSP-115 | `InfiniteRecursion` | Infinite recursion | Probable bugs | Warning | P1 |
| JAVA-INSP-116 | `MalformedFormatString` | Malformed format string | Probable bugs | Warning | P1 |
| JAVA-INSP-117 | `SuspiciousMethodCalls` | Suspicious collection method call | Probable bugs | Warning | P1 |
| JAVA-INSP-118 | `ArrayEquals` | 'equals()' called on array | Probable bugs | Warning | P1 |
| JAVA-INSP-119 | `ImplicitArrayToString` | Call to 'toString()' on array | Probable bugs | Warning | P1 |
| JAVA-INSP-120 | `ThrowableNotThrown` | 'Throwable' not thrown | Probable bugs | Warning | P1 |
| JAVA-INSP-121 | `MagicConstant` | Magic constant | Probable bugs | Warning | P2 |
| JAVA-INSP-122 | `Contract` | Contract issues | Probable bugs | Warning | P2 |
| JAVA-INSP-123 | `NullableProblems` | Nullability problems | Nullability problems | Warning | P1 |
| JAVA-INSP-124 | `NotNullFieldNotInitialized` | Non-null field is not initialized | Nullability problems | Warning | P2 |
| JAVA-INSP-125 | `PointlessBooleanExpression` | Pointless boolean expression | Control flow issues | Warning | P1 |
| JAVA-INSP-126 | `SimplifiableConditionalExpression` | Simplifiable conditional expression | Control flow issues | Warning | P1 |
| JAVA-INSP-127 | `TrivialIf` | Redundant 'if' statement | Control flow issues | Warning | P1 |
| JAVA-INSP-128 | `LoopStatementsThatDontLoop` | Loop statement that does not loop | Control flow issues | Warning | P2 |
| JAVA-INSP-129 | `InfiniteLoopStatement` | Infinite loop statement | Control flow issues | Warning | P2 |
| JAVA-INSP-130 | `DuplicateCondition` | Duplicate condition | Control flow issues | Warning | P2 |
| JAVA-INSP-131 | `UnnecessaryLocalVariable` | Redundant local variable | Data flow | Warning | P1 |
| JAVA-INSP-132 | `CatchMayIgnoreException` | Catch block may ignore exception | Error handling | Warning | P1 |
| JAVA-INSP-133 | `ReturnFromFinallyBlock` | 'return' inside 'finally' block | Error handling | Warning | P2 |
| JAVA-INSP-134 | `AutoCloseableResource` | AutoCloseable used without 'try'-with-resources | Resource management | Warning | P1 |
| JAVA-INSP-135 | `Convert2Lambda` | Anonymous type can be replaced with lambda | Java language level migration aids \| Java 8 | Warning | P1 |
| JAVA-INSP-136 | `Convert2MethodRef` | Lambda can be replaced with method reference | … \| Java 8 | Warning | P1 |
| JAVA-INSP-137 | `Convert2Diamond` | Explicit type can be replaced with '<>' | … \| Java 7 | Warning | P1 |
| JAVA-INSP-138 | `TryFinallyCanBeTryWithResources` | 'try finally' can be replaced with 'try' with resources | … \| Java 7 | Warning | P2 |
| JAVA-INSP-139 | `ForCanBeForeach` | 'for' loop can be replaced with enhanced for loop | … \| Java 5 | Warning | P2 |
| JAVA-INSP-140 | `RawUseOfParameterizedType` | Raw use of parameterized class | … \| Java 5 | Warning | P1 |
| JAVA-INSP-141 | `UnnecessaryBoxing` / `UnnecessaryUnboxing` | Unnecessary boxing / unboxing | … \| Java 5 | Warning | P2 |
| JAVA-INSP-142 | `IfCanBeSwitch` | 'if' can be replaced with 'switch' | Java language level migration aids | Warning | P2 |
| JAVA-INSP-143 | `SimplifyStreamApiCallChains` | Stream API call chain can be simplified | Verbose or redundant code constructs | Warning | P2 |
| JAVA-INSP-144 | `RedundantThrows` | Redundant 'throws' clause | Declaration redundancy | Warning | P2 |
| JAVA-INSP-145 | `SameParameterValue` | Method parameter always has the same value | Declaration redundancy | Warning | P2 |
| JAVA-INSP-146 | `UnusedReturnValue` | Method can be made 'void' | Declaration redundancy | Warning | P2 |
| JAVA-INSP-147 | `AccessStaticViaInstance` | Access static member via instance reference | Declaration redundancy | Warning | P2 |
| JAVA-INSP-148 | `FieldCanBeLocal` | Field can be local variable | Class structure | Warning | P2 |
| JAVA-INSP-149 | `RedundantMethodOverride` | Method is identical to its super method | Inheritance issues | Warning | P2 |
| JAVA-INSP-150 | `NumericOverflow` | Numeric overflow | Numeric issues | Warning | P2 |
| JAVA-INSP-151 | `IntegerDivisionInFloatingPointContext` | Integer division in floating-point context | Numeric issues | Warning | P2 |
| JAVA-INSP-152 | `DivideByZero` | Division by zero | Numeric issues | Warning | P2 |
| JAVA-INSP-153 | `JUnitMalformedDeclaration` | JUnit malformed declaration | JUnit | Warning | P1 |
| JAVA-INSP-154 | `JUnit5AssertionsConverter` | JUnit 5 obsolete assertions | JUnit | Warning | P2 |
| JAVA-INSP-155 | `SerialVersionUIDNotStaticFinal` | 'serialVersionUID' field not declared 'private static final long' | Serialization issues | Warning | P2 |
| JAVA-INSP-156 | `PreviewFeature` | Preview Feature warning | Compiler issues | Warning | P1 |
| JAVA-INSP-157 | `JavaModuleDefinition` | Java module definition problems | Declaration redundancy | Warning | P2 |
| JAVA-INSP-158 | `InconsistentTextBlockIndent` | Inconsistent whitespace indentation in text block | Probable bugs | Warning | P2 |
| JAVA-INSP-159 | `UnnecessaryReturn` / `UnnecessaryContinue` / `UnnecessaryBreak` | Unnecessary 'return'/'continue'/'break' statement | Verbose or redundant code constructs | Warning | P2 |
| JAVA-INSP-160 | `RedundantStreamOptionalCall` | Redundant step in 'Stream' or 'Optional' call chain | Verbose or redundant code constructs | Warning | P2 |
| JAVA-INSP-161 | `MissingSerialAnnotation` | '@Serial' annotation can be used | Serialization issues | Warning | P2 |
| JAVA-INSP-162 | `ReplaceAssignmentWithOperatorAssignment` | Assignment can be replaced with operator assignment | Assignment issues | Information (fix only) | P2 |

Note: compile errors themselves (unresolved symbol, incompatible types, unhandled exception, …) are
produced by the highlighting pass, not by inspections, and cannot be disabled.

### 5.4 Representative intentions / quick-fixes (Java)

| ID | Intention | Pri |
|---|---|---|
| JAVA-INSP-201 | Create class / interface / enum / record / method / field / local variable / parameter / constructor from unresolved usage | P0 |
| JAVA-INSP-202 | Import class / static member (see JAVA-EDIT-070) | P0 |
| JAVA-INSP-203 | Add exception to method signature / Surround with try-catch / Add runtime exception to throws | P0 |
| JAVA-INSP-204 | Change variable type / method return type / cast expression to fix type mismatch | P0 |
| JAVA-INSP-205 | Implement methods / make class abstract (for missing abstract implementations) | P0 |
| JAVA-INSP-206 | Add missing return / initialize variable / remove unreachable code | P1 |
| JAVA-INSP-207 | Invert 'if' condition; Split into declaration and assignment; Join declaration and assignment; Split 'if'; Merge nested 'if' | P1 |
| JAVA-INSP-208 | Flip comparison / Flip binary expression / Replace with `equals`, De Morgan conversions | P2 |
| JAVA-INSP-209 | Convert to switch expression / enhanced switch / create missing switch branches | P1 |
| JAVA-INSP-210 | Introduce local variable / Extract method (intention forms) | P1 |
| JAVA-INSP-211 | Bind constructor parameters to fields / Create field for parameter / Assign parameter to field | P1 |
| JAVA-INSP-212 | Replace with for-each / stream ↔ loop conversions (`Replace with forEach`, `Replace Stream API chain with loop`) | P2 |
| JAVA-INSP-213 | Annotate as @NotNull/@Nullable, Add `@Override`, Annotate deprecated, Make external annotation explicit | P2 |
| JAVA-INSP-214 | Replace concatenation with text block / `String.format` / `MessageFormat` / `StringBuilder` | P2 |
| JAVA-INSP-215 | Create test (from class) / Create missing tests | P1 |
| JAVA-INSP-216 | Create subclass / Implement interface / Seal class / Unimplement interface | P2 |
| JAVA-INSP-217 | Encapsulate field / Replace constructor with factory or builder (intention entry points of refactorings) | P2 |
| JAVA-INSP-218 | Add Javadoc / Fix doc comment | P2 |
| JAVA-INSP-219 | Convert compact ↔ canonical record constructor; Convert class ↔ record | P2 |
| JAVA-INSP-220 | Set language level / Add library to classpath / Add dependency (for Gradle modules may edit build script — inferred) | P0 |
| JAVA-INSP-221 | Iterate over iterable; Unroll loop; Collapse into loop; Move into if branches; Extract if condition | P2 |
| JAVA-INSP-222 | Inject language or reference; Edit regex fragment / Check RegExp | P2 |
| JAVA-INSP-223 | Change serialVersionUID; Sort content (enum constants, array literal); Wrap with unmodifiable collection | P2 |

Sources (section 5):
- https://www.jetbrains.com/help/idea/code-inspection.html , https://www.jetbrains.com/help/idea/running-inspections.html , https://www.jetbrains.com/help/idea/customizing-profiles.html , https://www.jetbrains.com/help/idea/configuring-inspection-severities.html , https://www.jetbrains.com/help/idea/disabling-and-enabling-inspections.html
- https://www.jetbrains.com/help/idea/problems-tool-window.html , https://www.jetbrains.com/help/idea/intention-actions.html
- https://www.jetbrains.com/help/inspectopedia/ (per-inspection reference)
- Community sources: `java/java-backend/resources/META-INF/InspectionGadgets.xml`, `Inspections.xml`, `GlobalInspections.xml`, `Intentions.xml`; `plugins/junit`, `plugins/testng`, `plugins/lombok` plugin XML; `messages/*Bundle.properties` for display names.

---
## 6. Refactorings

Common model (applies to all rows): refactorings are invoked on the element at caret or selection;
most open a dialog or run **in-place** (inline rename/introduce with a popup of options, Alt+Shift+O shows
the options popup, Tab/Enter to finish). Many offer **Preview** (Find-style tool window listing every
affected usage, with exclude/include before "Do Refactor"), conflict detection dialog ("Problems Detected"
with Continue/Show in View), and are a single undoable command (also undoable from other files).
`Refactor This` (`Ctrl+Alt+Shift+T` / `Ctrl+T` macOS) lists refactorings applicable at caret.
Menu: **Refactor**.

| ID | Refactoring | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-REF-001 | Refactor This | Context popup of applicable refactorings. | Ctrl+Alt+Shift+T | Ctrl+T | Refactor \| Refactor This… | P0 |
| JAVA-REF-002 | Rename | Renames class, method (with overriders/overloads options), field, variable, parameter, package, module, file; updates references, comments/strings (option), test classes, getters/setters, related variables (inheritor naming), Javadoc; in-place first, Shift+F6 again opens dialog. | Shift+F6 | Shift+F6 | Refactor \| Rename… | P0 |
| JAVA-REF-003 | Rename File | Rename file independently of its class (e.g. for non-Java files). | — | — | Refactor \| Rename File… | P1 |
| JAVA-REF-004 | Change Signature | Edit method name, return type, visibility, parameters (add/remove/reorder, default values for callers, propagate to callers), exceptions (propagate); delegate via overloading option; also for classes' type parameters. In-place variant: editing a signature shows a gutter hint to update callers. | Ctrl+F6 | Cmd+F6 | Refactor \| Change Signature… | P0 |
| JAVA-REF-005 | Type Migration | Changes the type of a variable/field/parameter/return and propagates through the data flow; preview of conflicts. | Ctrl+Shift+F6 | Cmd+Shift+F6 | Refactor \| Type Migration… | P1 |
| JAVA-REF-006 | Introduce Variable | Extracts expression into local variable (choose occurrences, `final`/`var` options, name suggestions). | Ctrl+Alt+V | Cmd+Opt+V | Refactor \| Introduce \| Variable… | P0 |
| JAVA-REF-007 | Introduce Constant | Extracts expression to `static final` field; choose class, visibility, replace all occurrences, move to another class. | Ctrl+Alt+C | Cmd+Opt+C | Refactor \| Introduce \| Constant… | P0 |
| JAVA-REF-008 | Introduce Field | Extracts to field; initialise in declaration / constructor / current method / setUp. | Ctrl+Alt+F | Cmd+Opt+F | Refactor \| Introduce \| Field… | P0 |
| JAVA-REF-009 | Introduce Parameter | Turns expression/local into a new parameter of the enclosing method, updating callers; option to delegate. | Ctrl+Alt+P | Cmd+Opt+P | Refactor \| Introduce \| Parameter… | P1 |
| JAVA-REF-010 | Introduce Functional Parameter | Extracts code fragment into a functional-interface parameter (lambda passed by callers). | Ctrl+Alt+Shift+P | Cmd+Opt+Shift+P | Refactor \| Introduce \| Functional Parameter… | P2 |
| JAVA-REF-011 | Introduce Functional Variable | Extracts code into a local lambda variable. | — | — | Refactor \| Introduce \| Functional Variable… | P2 |
| JAVA-REF-012 | Introduce Parameter Object | Groups parameters into a new or existing class/record and updates callers. | — | — | Refactor \| Introduce \| Parameter Object… | P2 |
| JAVA-REF-013 | Extract Method | Extracts statements/expression into a method; infers parameters/return; finds and offers to replace duplicates; in-place with options popup; handles multiple outputs by suggesting object/record. | Ctrl+Alt+M | Cmd+Opt+M | Refactor \| Extract/Introduce \| Method… | P0 |
| JAVA-REF-014 | Extract Method Object | Replaces a method with a class whose fields are former locals (Replace Method With Method Object). | — | — | Refactor \| Replace Method with Method Object… | P2 |
| JAVA-REF-015 | Extract Delegate (Extract Class) | Moves selected members into a new class and delegates. | — | — | Refactor \| Extract \| Delegate… | P2 |
| JAVA-REF-016 | Extract Interface | Creates an interface from selected methods; optionally replace usages with the interface ("Use interface where possible"). | — | — | Refactor \| Extract \| Interface… | P1 |
| JAVA-REF-017 | Extract Superclass | Creates a superclass with selected members (optionally abstract). | — | — | Refactor \| Extract \| Superclass… | P1 |
| JAVA-REF-018 | Inline | Inlines variable, method (all or this call), constant, field, superclass ("Inline Superclass"), anonymous-to-class (Inline to Anonymous Class), parameter (Inline Parameter), pattern variable. | Ctrl+Alt+N | Cmd+Opt+N | Refactor \| Inline… | P0 |
| JAVA-REF-019 | Move class / package | Moves classes/packages to another package or source root/module; updates imports/references; option to search in comments and strings. Drag-and-drop in Project view triggers it. | F6 | F6 | Refactor \| Move… | P0 |
| JAVA-REF-020 | Move static members | Move static methods/fields to another class. | F6 | F6 | Refactor \| Move… | P1 |
| JAVA-REF-021 | Move instance method | Move an instance method to the class of one of its parameters/fields. | F6 | F6 | Refactor \| Move… | P2 |
| JAVA-REF-022 | Move inner class to upper level / Convert anonymous to inner | — | F6 | F6 | Refactor \| Move… / Convert Anonymous to Inner… | P2 |
| JAVA-REF-023 | Copy class / file | Copies a class into another package with a new name, updating self-references. | F5 | F5 | Refactor \| Copy… | P1 |
| JAVA-REF-024 | Safe Delete | Deletes a symbol only after checking usages; shows usages and allows "Delete anyway" or view; handles overriding methods and parameters (removes arguments). | Alt+Delete | Cmd+Delete | Refactor \| Safe Delete… | P0 |
| JAVA-REF-025 | Pull Members Up | Moves members to a superclass/interface (optionally as abstract). | — | — | Refactor \| Pull Members Up… | P1 |
| JAVA-REF-026 | Push Members Down | Moves members to subclasses. | — | — | Refactor \| Push Members Down… | P2 |
| JAVA-REF-027 | Use Interface Where Possible | Replaces usages of a class with a supertype where valid (TurnRefsToSuper). | — | — | Refactor \| Use Interface Where Possible… | P2 |
| JAVA-REF-028 | Replace Inheritance with Delegation | Replaces `extends` by a field and delegating methods. | — | — | Refactor \| Replace Inheritance with Delegation… | P2 |
| JAVA-REF-029 | Encapsulate Fields | Generates accessors and replaces direct field access; options for visibility and which accesses to replace. | — | — | Refactor \| Encapsulate Fields… | P2 |
| JAVA-REF-030 | Convert to Instance Method | Makes a static method an instance method of a parameter's class. | — | — | Refactor \| Convert To Instance Method… | P2 |
| JAVA-REF-031 | Make Static | Converts an instance method to static, adding parameters for used fields. | — | — | Refactor \| Make Static… | P2 |
| JAVA-REF-032 | Invert Boolean | Inverts a boolean method/variable and all usages. | — | — | Refactor \| Invert Boolean… | P2 |
| JAVA-REF-033 | Replace Constructor with Factory Method | Introduces static factory and replaces `new` calls (intention/refactoring). | Alt+Enter | Opt+Enter | Intention on constructor; Refactor This | P2 |
| JAVA-REF-034 | Replace Constructor with Builder | Generates a builder class (new or existing) and replaces calls. | Alt+Enter | Opt+Enter | Intention on constructor; Refactor This | P2 |
| JAVA-REF-035 | Generify | Adds type arguments to raw-type usages across scope (bundled "rare refactorings"). | — | — | Refactor \| Generify… | P2 |
| JAVA-REF-036 | Migrate | Applies a package/class migration map (e.g. JUnit 4→5, `javax`→`jakarta` maps; custom maps via Create New Migration). | — | — | Refactor \| Migrate Packages and Classes… | P2 |
| JAVA-REF-037 | Remove Middleman | Replaces calls to delegating methods with direct calls to the delegate. | — | — | Refactor \| Remove Middleman… | P2 |
| JAVA-REF-038 | Wrap Method Return Value | Wraps return value in a new or existing wrapper class. | — | — | Refactor \| Wrap Method Return Value… | P2 |
| JAVA-REF-039 | Replace Temp with Query | Replaces a local variable with a method call. | — | — | Refactor \| Replace Temp with Query… | P2 |
| JAVA-REF-040 | Find and Replace Code Duplicates | Finds code equivalent to a method body and replaces with calls. | — | — | Refactor \| Find and Replace Code Duplicates… | P2 |
| JAVA-REF-041 | Extract Module | Creates a module from a package (IDE module; Gradle build file not updated — inferred). | — | — | Refactor \| Extract \| Module… | P2 |
| JAVA-REF-042 | Convert Anonymous to Inner | Turns anonymous class into a named inner class. | — | — | Refactor \| Convert Anonymous to Inner… | P2 |
| JAVA-REF-043 | Delombok / Lombok refactorings | See JAVA-LOMBOK-004. | — | — | Refactor \| Delombok | P2 |
| JAVA-REF-044 | I18nize hard-coded string | Moves a string literal to a resource bundle (Internationalization). | — | — | Intention / Refactor | P2 |
| JAVA-REF-045 | Refactoring preview and conflicts UI | Shared infrastructure: preview usages view, conflicts dialog, "Refactoring Preview" with exclude, undo across files, VCS-aware file moves. | — | — | — | P0 |
| JAVA-REF-046 | Safe rename of tests / getters / overloads | Rename offers to rename related test classes, getters/setters for fields, variables named after classes, and overloads. | — | — | Rename dialog follow-ups | P1 |
| JAVA-REF-047 | Rename in comments and strings, text occurrences | Option to update non-code occurrences with preview. | — | — | Rename dialog | P1 |
| JAVA-REF-048 | Drag-and-drop move | Dragging classes/packages in Project view triggers Move with dialog. | — | — | Project tool window | P1 |
| JAVA-REF-049 | Paste-time refactoring hints | e.g. "Update usages" balloon after manual rename of a declaration (in-place "rename" suggestion via gutter). | — | — | Editor gutter | P2 |

Sources (section 6):
- https://www.jetbrains.com/help/idea/refactoring-source-code.html and the linked per-refactoring pages (rename-refactorings.html, change-signature.html, extract-method.html, extract-constant.html, extract-field.html, extract-parameter.html, extract-variable.html, extract-interface.html, extract-superclass.html, inline.html, move-refactorings.html, safe-delete.html, pull-members-up.html, push-members-down.html, introduce-parameter-object.html, encapsulate-fields.html, convert-to-instance-method.html, make-method-static.html, migrate.html, type-migration.html, replace-constructor-with-factory-method.html, replace-constructor-with-builder.html, generify-refactoring.html, invert-boolean.html, remove-middleman.html, wrap-return-value.html, replace-temp-with-query.html, use-interface-where-possible.html, replace-inheritance-with-delegation.html)
- Community sources: `platform/platform-resources/src/idea/LangActions.xml` (RefactoringMenu structure), `java/java-backend/resources/META-INF/JavaActions.xml`, `Intentions.xml`, `plugins/rareJavaRefactorings`, keymaps.

---

## 7. Code generation

`Generate` popup: `Alt+Insert` (Win/Linux), `Cmd+N` or `Ctrl+Enter` (macOS). Menu: Code | Generate….
Entries depend on context (class body, test class, build script). Override/Implement have their own shortcuts.

| ID | Entry | Behaviour | Win/Linux | macOS | Pri |
|---|---|---|---|---|---|
| JAVA-GEN-001 | Generate popup | Context menu of generators. | Alt+Insert | Cmd+N / Ctrl+Enter | P0 |
| JAVA-GEN-002 | Constructor | Choose fields to initialise; call super constructor variant; for records: canonical/compact. | — | — | P0 |
| JAVA-GEN-003 | Getter | Choose fields; template choice (IntelliJ default, builder-style, Lombok-like…), `@Nullable/@NotNull` propagation. | — | — | P0 |
| JAVA-GEN-004 | Setter | Same; template choice (default, builder/fluent). | — | — | P0 |
| JAVA-GEN-005 | Getter and Setter | Both at once. | — | — | P0 |
| JAVA-GEN-006 | equals() and hashCode() | Wizard: template (IntelliJ default, `java.util.Objects` (Java 7+), Apache Commons Lang 2/3, Guava), accept subclasses option, choose fields for equals/hashCode, non-null fields. | — | — | P0 |
| JAVA-GEN-007 | toString() | Template choice (String concat, StringBuilder, `String.format`, Objects/ToStringBuilder, etc.), field selection, insert position, settings (fully qualified class name, getters, etc.). | — | — | P0 |
| JAVA-GEN-008 | Override Methods | Choose methods of supertypes to override (with `@Override`, call super, copy Javadoc options). | Ctrl+O | Ctrl+O | P0 |
| JAVA-GEN-009 | Implement Methods | Choose abstract methods to implement. | Ctrl+I | Ctrl+I | P0 |
| JAVA-GEN-010 | Delegate Methods | Generate methods delegating to a field's methods. | — | — | P2 |
| JAVA-GEN-011 | Super Method Call | Inserts call to the overridden super method. | — | — | P2 |
| JAVA-GEN-012 | Test… | Create a test class for the current class (framework JUnit 4/5/TestNG/…, superclass, destination package, generate setUp/tearDown, select methods). Also via Navigate \| Test, `Ctrl+Shift+T`. | — | — | P0 |
| JAVA-GEN-013 | Test Method / SetUp / TearDown / BeforeClass / AfterClass / Parameters Method (in test classes) | Framework-aware stubs from file templates (JUnit 3/4/5, TestNG). | — | — | P1 |
| JAVA-GEN-014 | Logger | Inserts a logger field for the configured/available logging library (e.g. SLF4J, Log4j2, JUL, Lombok `@Slf4j`); configurable preferred logger. | — | — | P2 |
| JAVA-GEN-015 | Copyright | Inserts/updates the copyright header from the active copyright profile (Settings \| Editor \| Copyright). | — | — | P2 |
| JAVA-GEN-016 | Generate module-info descriptors | Creates `module-info.java` for modules from dependencies. | — | — | P2 |
| JAVA-GEN-017 | Javadoc generation intention | See JAVA-EDIT-081. | — | — | P2 |
| JAVA-GEN-018 | serialVersionUID | Generated via quick-fix when Serializable inspection enabled. | Alt+Enter | Opt+Enter | P2 |
| JAVA-GEN-019 | New Java Class / Interface / Enum / Record / Annotation / Exception / package-info / module-info | File \| New \| Java Class popup (kind chooser), based on file templates with `${PACKAGE_NAME}`, `${NAME}`, includes File Header. | Alt+Insert (in Project view) | Cmd+N | P0 |
| JAVA-GEN-020 | File and code templates | Editable templates for new files and generated code (getters, equals, test methods, catch body, method body). | — | — | P1 |
| JAVA-GEN-021 | Add dependency (build script) | In `build.gradle(.kts)` Generate offers Add dependency / plugin (GRADLE-134). | Alt+Insert | Cmd+N | P1 |
| JAVA-GEN-022 | Lombok-aware generation | Generate menu offers Lombok annotations when Lombok is on classpath (inferred). | — | — | P2 |
| JAVA-GEN-023 | Generate code from usage | Create-from-usage quick-fixes (JAVA-INSP-201) are the main "code first" generator. | Alt+Enter | Opt+Enter | P0 |

Sources (section 7):
- https://www.jetbrains.com/help/idea/generating-code.html , https://www.jetbrains.com/help/idea/generate-equals-and-hashcode-wizard.html , https://www.jetbrains.com/help/idea/generate-tostring-dialog.html , https://www.jetbrains.com/help/idea/create-test.html , https://www.jetbrains.com/help/idea/copyright.html , https://www.jetbrains.com/help/idea/using-file-and-code-templates.html
- Community sources: `JavaActions.xml` (GenerateConstructor, GenerateGetter, GenerateSetter, GenerateGetterAndSetter, GenerateEquals, GenerateToString, GenerateSuperMethodCall, GenerateLogField, GenerateTestMethod, GenerateSetUpMethod, GenerateTearDownMethod, GenerateBeforeClassMethod, GenerateAfterClassMethod, GenerateDataMethod, DelegateMethods, OverrideMethods, ImplementMethods, GenerateModuleDescriptors).

---
## 8. Run, test, debug, coverage, profiling

### 8.1 Run configurations and execution (general + Application)

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-RUN-001 | Run gutter icons | `main` methods, test classes/methods, and Gradle tasks get a gutter run icon with Run / Debug / Run with Coverage / Profile / Modify Run Configuration; icon shows last test status. | — | — | Gutter | P0 |
| JAVA-RUN-002 | Run context configuration | Runs the class/method/test at caret (creates temporary configuration). | Ctrl+Shift+F10 | Ctrl+Shift+R | Context menu \| Run '…' | P0 |
| JAVA-RUN-003 | Debug context configuration | — | (no default on Win; via gutter) | Ctrl+Shift+D | Context menu \| Debug '…' | P0 |
| JAVA-RUN-004 | Run / Debug selected configuration | Runs the configuration selected in the toolbar widget. | Shift+F10 / Shift+F9 | Ctrl+R / Ctrl+D | Run \| Run '…' | P0 |
| JAVA-RUN-005 | Choose and run / debug | Popup to choose a configuration (Alt+Shift+F10 / F9). | Alt+Shift+F10 / Alt+Shift+F9 | Ctrl+Opt+R / Ctrl+Opt+D | Run \| Run… | P0 |
| JAVA-RUN-006 | Run widget (toolbar) | Configuration selector with run/debug/stop/more actions, recent configurations, "Current File" option. | — | — | Main toolbar | P0 |
| JAVA-RUN-007 | Edit Configurations dialog | Tree of configuration types and instances; templates; Before launch tasks (Build, Gradle task, run another configuration, external tool); "Allow multiple instances"; "Store as project file"; folders. | — | — | Run \| Edit Configurations… | P0 |
| JAVA-RUN-008 | Temporary configurations | Context runs create temporary configs (limit, default 5) which can be saved. | — | — | Run widget | P1 |
| JAVA-RUN-009 | Application configuration: core fields | Name, JRE (module SDK by default, or another JDK/alternative JRE), "Use classpath of module" (`-cp`), Main class (with browse/search), Program arguments, Working directory, Environment variables (with "include system env"), VM options (via "Modify options"). | — | — | Edit Configurations \| Application | P0 |
| JAVA-RUN-010 | Application configuration: Modify options | Add VM options; Shorten command line (none / JAR manifest / classpath file / `@argfile`); Add dependencies with "provided" scope to classpath; Redirect input from; Do not build before run; Open run/debug tool window when started; Show console on stdout/stderr; Save console output to file; Emulate terminal in output console; Allow multiple instances; Logs to show in console; Enable/disable coverage/capture settings; Java Flight Recorder options (`$` for profiler UI). | — | — | Modify options popup | P1 |
| JAVA-RUN-011 | Run tool window | Console with ANSI colours, clickable stack traces (with "Analyze stack trace"), input for stdin, soft-wrap, scroll-to-end, clear, search, fold lines matching patterns; toolbar Rerun, Stop, Restore layout, Pin tab, Exit. Multiple tabs for concurrent runs. | Alt+4 | Cmd+4 | View \| Tool Windows \| Run | P0 |
| JAVA-RUN-012 | Rerun | — | Ctrl+F5 | Cmd+R | Run tool window | P0 |
| JAVA-RUN-013 | Stop | Stop process; Stop again forces kill; "Stop Background Processes". | Ctrl+F2 | Cmd+F2 | Run \| Stop | P0 |
| JAVA-RUN-014 | Services tool window | Groups running configurations (e.g. multiple apps) and allows start/stop. | Alt+8 | Cmd+8 | View \| Tool Windows \| Services | P2 |
| JAVA-RUN-015 | Run Anything | Run commands/configs/Gradle tasks by typing. | Ctrl, Ctrl | Ctrl, Ctrl | Double Ctrl | P1 |
| JAVA-RUN-016 | Compound configurations | Run several configurations together. | — | — | Edit Configurations \| Compound | P2 |
| JAVA-RUN-017 | Remote JVM Debug configuration | Attach debugger to a JVM with JDWP; generates the agent command line string per JDK version; listen/attach modes. | — | — | Edit Configurations \| Remote JVM Debug | P1 |
| JAVA-RUN-018 | Attach to Process | Lists local JVMs and attaches the debugger. | Ctrl+Alt+F5 | Opt+Shift+F5 | Run \| Attach to Process… | P2 |
| JAVA-RUN-019 | JAR Application configuration | Runs an existing JAR with arguments and JRE. | — | — | Edit Configurations \| JAR Application | P2 |
| JAVA-RUN-020 | Shell Script / Docker / other types | Platform run types usable alongside Java (not Java-specific). | — | — | — | P2 |
| JAVA-RUN-021 | Run configuration from Gradle `application` plugin | Running the `run` task or the main class via Gradle (see GRADLE-103). | — | — | Gradle tool window | P0 |
| JAVA-RUN-022 | Exception stack trace links | Navigate from console frames to source; "Create breakpoint" for exception class (inferred). | — | — | Console | P0 |

### 8.2 Testing (JUnit, TestNG)

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-TEST-001 | JUnit configuration type | Test kind: All in package, All in directory, Pattern (regex), Class, Method, Category/Tags (JUnit 5 tag expressions), UniqueId, Search for tests (in single module / across module dependencies / whole project); fork mode (none, method, class); repeat (once, N times, until failure, until stop); VM options, program args, env, working dir, JRE, shorten command line, use module classpath. Supports JUnit 3, 4, 5 (Jupiter, Vintage), and JUnit 6 platform (inferred version). | — | — | Edit Configurations \| JUnit | P0 |
| JAVA-TEST-002 | TestNG configuration type | Test kind: All in package, Suite (XML), Group, Class, Method, Pattern; listeners, parameters, output dir, VM options. | — | — | Edit Configurations \| TestNG | P1 |
| JAVA-TEST-003 | Test runner UI | Tree of suites/classes/methods with status icons, durations, counts; filters (show passed, show ignored), sort alphabetically/by duration; output per node; failure diff link ("Click to see difference") opening a diff viewer for `assertEquals` / expected-actual; navigation to failing line; test history import/export. | — | — | Run tool window test tab | P0 |
| JAVA-TEST-004 | Rerun failed tests | Re-executes only failed tests. | — | — | Test runner toolbar | P0 |
| JAVA-TEST-005 | Rerun tests (last) | — | Alt+Shift+R | Ctrl+Cmd+R | Run \| Rerun Tests | P1 |
| JAVA-TEST-006 | Toggle auto-test | Re-runs tests automatically after changes (with delay). | — | — | Test runner toolbar | P2 |
| JAVA-TEST-007 | Recent tests popup | Lists recently run tests/configurations with status. | Ctrl+Shift+; | Cmd+Shift+; | Run \| Recent Tests (Find Action) | P2 |
| JAVA-TEST-008 | Test results history | Previous runs saved (configurable count) and openable. | — | — | Test runner toolbar "Test History" | P2 |
| JAVA-TEST-009 | Export test results | HTML/XML export. | — | — | Test runner toolbar | P2 |
| JAVA-TEST-010 | Create Test dialog | See JAVA-GEN-012; also "Fix" to add the testing library to the module (for Gradle: adds dependency in the build script — inferred). | Ctrl+Shift+T | Cmd+Shift+T | Navigate \| Test | P0 |
| JAVA-TEST-011 | Parameterized and dynamic tests | Nodes per invocation (`@ParameterizedTest`, `@TestFactory`, `@RepeatedTest`, `@Nested`), run single invocation (inferred for single invocation). | — | — | Test tree | P1 |
| JAVA-TEST-012 | JUnit/TestNG inspections & intentions | JUnit malformed declarations, mixing frameworks, JUnit 3 style in JUnit 4, conversions (JUnit 4 → 5 migration via inspection fixes / Migrate). | — | — | Inspections | P1 |
| JAVA-TEST-013 | Run tests in directory/package | Context menu on a directory/package. | Ctrl+Shift+F10 | Ctrl+Shift+R | Project view | P0 |
| JAVA-TEST-014 | Debug tests | — | via gutter / context | Ctrl+Shift+D | Gutter | P0 |
| JAVA-TEST-015 | Test frameworks: Spock/Cucumber/etc. | Out of scope; plugins (Cucumber, Spock via Groovy) register additional runners (inferred). | — | — | — | P2 |
| JAVA-TEST-016 | Test location from Gradle output | Gradle test events map to source via class/method names (see GRADLE-120). | — | — | — | P0 |

### 8.3 Debugger

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-DBG-001 | Line breakpoint | Toggle by gutter click; suspends when line reached; supports lambda selection on lines with lambdas (popup: line, lambda, all). | Ctrl+F8 | Cmd+F8 | Run \| Toggle Breakpoint \| Line Breakpoint | P0 |
| JAVA-DBG-002 | Temporary line breakpoint | Removed after first hit. | Ctrl+Alt+Shift+F8 | Cmd+Opt+Shift+F8 | Run \| Toggle Breakpoint | P1 |
| JAVA-DBG-003 | Method breakpoint | Suspends on method entry/exit; works on interface methods (slower; emulated by default — inferred). | — | — | Run \| Toggle Breakpoint \| Method Breakpoint | P1 |
| JAVA-DBG-004 | Field watchpoint | Suspends on field access and/or modification. | — | — | Run \| Toggle Breakpoint \| Field Watchpoint | P1 |
| JAVA-DBG-005 | Exception breakpoint | Suspends when an exception class is thrown (caught/uncaught options, class filters); "Any exception". | — | — | Breakpoints dialog \| + Java Exception Breakpoints | P1 |
| JAVA-DBG-006 | Class prepare / collection breakpoints (inferred) | Suspend on class load; "Collection breakpoint" for collection field modifications. | — | — | Breakpoints dialog | P2 |
| JAVA-DBG-007 | Breakpoint properties | Enabled, Suspend (All / Thread / none), Condition (Java expression with completion), Log ("Breakpoint hit" message, stack trace, evaluate and log expression), Remove once hit, Disable until hitting another breakpoint, Instance filters, Class filters, Caller filters, Pass count. | Ctrl+Shift+F8 (edit at caret) | Cmd+Shift+F8 | Right-click breakpoint; Breakpoints dialog | P0 |
| JAVA-DBG-008 | Logpoints (non-suspending log breakpoints) | 2026.2 headlines "Logpoints": breakpoints that only log an evaluated message without suspending, with a dedicated quick creation (details: inferred from older "Add Logging Breakpoint"/Shift+click behaviour). Action id `AddLoggingBreakpoint`. | Shift+Click gutter | Shift+Click gutter | Gutter | P1 |
| JAVA-DBG-009 | Breakpoints dialog | List/groups of all breakpoints with properties, enable/disable, export/import, group by file/package/type. | Ctrl+Shift+F8 | Cmd+Shift+F8 | Run \| View Breakpoints… | P1 |
| JAVA-DBG-010 | Mute breakpoints | Temporarily disables all breakpoints for the session. | — | — | Debug toolbar | P1 |
| JAVA-DBG-011 | Debug tool window: Frames | Call stack of the selected thread; library frames folded; async stack trace segments; "Drop frame"/"Reset frame"; copy stack; navigation. | Alt+5 (window) | Cmd+5 | View \| Tool Windows \| Debug | P0 |
| JAVA-DBG-012 | Threads view | Thread list with states (and virtual threads), thread dump, switch current thread, filter. | — | — | Debug tool window \| Threads | P1 |
| JAVA-DBG-013 | Variables view | Tree of locals, `this`, fields, return values ("Show method return values" option), lambda captures, with renderers (collections, maps, toString, custom type renderers), "Set Value" (F2), "Jump to Type Source", "Mark Object" (label), "Show referring objects", inline watch. | F2 (set value) | F2 | Debug tool window \| Variables | P0 |
| JAVA-DBG-014 | Watches | Expressions re-evaluated at each stop; can be merged into Variables view; add from editor. | Insert (new watch) | Insert | Debug tool window | P0 |
| JAVA-DBG-015 | Evaluate Expression | Dialog for expressions or code fragments (multi-line) with completion, history; result tree. | Alt+F8 | Opt+F8 | Run \| Debugging Actions \| Evaluate Expression… | P0 |
| JAVA-DBG-016 | Quick evaluate | Evaluate selection/hover expression in a popup; Alt+click. | Ctrl+Alt+F8 / Alt+Click | Cmd+Opt+F8 / Opt+Click | Run \| Debugging Actions \| Quick Evaluate | P1 |
| JAVA-DBG-017 | Inline values (inline debugging) | Shows variable values next to lines in the editor while suspended; changed values highlighted. | — | — | Editor | P0 |
| JAVA-DBG-018 | Step Over | — | F8 | F8 | Run \| Debugging Actions | P0 |
| JAVA-DBG-019 | Step Into | Respects stepping filters (skip JDK/synthetic/getters per settings). | F7 | F7 | Same | P0 |
| JAVA-DBG-020 | Smart Step Into | Choose which call on the line to enter (including lambdas/method refs). | Shift+F7 | Shift+F7 | Same | P1 |
| JAVA-DBG-021 | Force Step Into | Steps into methods normally skipped. | Alt+Shift+F7 | Opt+Shift+F7 | Same | P2 |
| JAVA-DBG-022 | Step Out | — | Shift+F8 | Shift+F8 | Same | P0 |
| JAVA-DBG-023 | Force Step Over | Steps over ignoring breakpoints in called methods. | Alt+Shift+F8 | Opt+Shift+F8 | Same | P2 |
| JAVA-DBG-024 | Run to Cursor / Force Run to Cursor | Runs until caret line (force ignores breakpoints); also clicking line number. | Alt+F9 / Ctrl+Alt+F9 | Opt+F9 / Cmd+Opt+F9 | Same | P0 |
| JAVA-DBG-025 | Resume / Pause | — | F9 / — | Cmd+Opt+R (or F9) / — | Same | P0 |
| JAVA-DBG-026 | Show Execution Point | Jumps editor to current suspended line. | Alt+F10 | Opt+F10 | Same | P0 |
| JAVA-DBG-027 | Step out of code block (inferred) | Steps until leaving the current block. | — | — | Same | P2 |
| JAVA-DBG-028 | Reset Frame (Drop Frame) | Pops the current frame and re-enters the method (side effects not undone). | — | — | Frames context / toolbar | P1 |
| JAVA-DBG-029 | Force Return / Throw Exception | Return early with a value or throw from current frame. | — | — | Frames context menu | P2 |
| JAVA-DBG-030 | HotSwap (Reload Changed Classes) | After recompilation during debug, changed classes are redefined in the JVM (method bodies only with standard JVM; "Reload classes after compilation" setting: Always / Ask / Never); floating "Code changed" hint to apply (inferred for current UI). Build-then-reload shortcut via Build Project. | Ctrl+F9 then reload; Ctrl+F10 "Update Running Application" | Cmd+F9; Cmd+F10 | Run \| Debugging Actions \| Reload Changed Classes | P1 |
| JAVA-DBG-031 | Async stack traces | Stitches stack across executors/CompletableFuture/`@Async` annotated points; custom capture/insert points via annotations or settings (Settings \| Build, Execution, Deployment \| Debugger \| Async Stack Traces). | — | — | Frames view | P1 |
| JAVA-DBG-032 | Stepping filters / "Do not step into" | Lists of classes to skip; skip synthetic, constructors, class loaders, simple getters. | — | — | Settings \| … \| Debugger \| Stepping | P1 |
| JAVA-DBG-033 | Data type renderers | Custom renderers for types (expression or children list); built-ins for collections/maps/arrays/strings; toString on/off. | — | — | Settings \| … \| Debugger \| Data Views \| Java Type Renderers | P2 |
| JAVA-DBG-034 | Stream trace (Java Stream Debugger) | Visualises each stage of a Stream pipeline at a breakpoint (split / flat mode). | — | — | Debug toolbar "Trace Current Stream Chain" | P2 |
| JAVA-DBG-035 | Memory view | Counts instances of classes in heap, diff between stops, show instances (Memory tab). | — | — | Debug tool window \| Memory | P2 |
| JAVA-DBG-036 | Overhead view | Shows debugger overhead per breakpoint/renderer. | — | — | Debug tool window \| Overhead | P2 |
| JAVA-DBG-037 | Thread dump / Get thread dump | Capture thread dump from running process. | — | — | Debug/Run toolbar | P2 |
| JAVA-DBG-038 | Exception analysis | On exception breakpoint, `$exception` var; exception in Variables. | — | — | — | P2 |
| JAVA-DBG-039 | Debugger settings | Transport (socket/shared memory), "Enable auto-reload of classes", "Show alternative source switcher", "Kill process immediately", "Attach to the process with memory agent" (inferred), Hotswap options. | — | — | Settings \| Build, Execution, Deployment \| Debugger | P1 |
| JAVA-DBG-040 | Breakpoints in libraries / decompiled code | Works with sources or decompiled code (line mapping). | — | — | — | P1 |
| JAVA-DBG-041 | Virtual threads support | Lists/filters virtual threads, shows in dumps. | — | — | Threads view | P2 |
| JAVA-DBG-042 | Debug Gradle tasks / scripts | See GRADLE-114, GRADLE-123. | — | — | — | P1 |
| JAVA-DBG-043 | Remote debugging | See JAVA-RUN-017. | — | — | — | P1 |
| JAVA-DBG-044 | Spring/framework runtime insights | Spring debugger (beans, properties, endpoints) without suspending — 2026.1. `$` | — | — | — | P2 |

### 8.4 Coverage

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-COV-001 | Run with Coverage | Runs application/test configuration with the IntelliJ coverage agent (JaCoCo selectable, inferred); also for Gradle test runs (unless disabled in config). | — | — | Gutter / Run menu \| Run with Coverage | P1 |
| JAVA-COV-002 | Coverage tool window | Per package/class: % class, method, line, branch coverage; flatten packages; generate HTML report. | — | — | Coverage tool window | P1 |
| JAVA-COV-003 | Editor coverage highlighting | Gutter colours for covered/partially/uncovered lines; popup per line with hit info and tests covering it. | — | — | Gutter | P1 |
| JAVA-COV-004 | Coverage suites | Show/hide/merge multiple coverage data sets. | Ctrl+Alt+F6 | Cmd+Opt+F6 | Run \| Show Coverage Data… | P2 |
| JAVA-COV-005 | Coverage settings | Tracing vs sampling, branch coverage, include/exclude patterns, "track per-test coverage", test folder exclusion. | — | — | Run configuration \| Code Coverage; Settings \| Build, Execution, Deployment \| Coverage | P2 |

### 8.5 Profiling (`$`)

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-PROF-001 | Run with Profiler | Launch a configuration with IntelliJ Profiler (async-profiler/JFR backends); `$` (inferred). | — | — | Run widget \| Profile | P2 |
| JAVA-PROF-002 | CPU & allocation profiling views | Flame graph, call tree, method list, timeline, per-thread filters; open `.jfr`/`.hprof` snapshots. `$` (inferred) | — | — | Profiler tool window | P2 |
| JAVA-PROF-003 | Live CPU/memory charts | Live charts for running processes; attach to running JVM. `$` (inferred) | — | — | Profiler tool window | P2 |
| JAVA-PROF-004 | Memory snapshots | Capture and analyze heap dumps (Analyze Heap Dump). `$` (inferred) | — | — | Profiler | P2 |
| JAVA-PROF-005 | Thread dumps | Capture thread dumps. | — | — | Profiler / Run toolbar | P2 |
| JAVA-PROF-006 | Profiler inlay/gutter hints | Hot-method highlighting in editor after a profiling session. `$` (inferred) | — | — | Editor | P2 |

Sources (section 8):
- https://www.jetbrains.com/help/idea/run-debug-configuration.html , https://www.jetbrains.com/help/idea/run-debug-configuration-application.html , https://www.jetbrains.com/help/idea/run-debug-configuration-junit.html , https://www.jetbrains.com/help/idea/run-debug-configuration-testng.html , https://www.jetbrains.com/help/idea/run-debug-configuration-remote-debug.html
- https://www.jetbrains.com/help/idea/performing-tests.html , https://www.jetbrains.com/help/idea/viewing-and-exploring-test-results.html , https://www.jetbrains.com/help/idea/create-tests.html
- https://www.jetbrains.com/help/idea/debugging-code.html , https://www.jetbrains.com/help/idea/using-breakpoints.html , https://www.jetbrains.com/help/idea/stepping-through-the-program.html , https://www.jetbrains.com/help/idea/examining-suspended-program.html , https://www.jetbrains.com/help/idea/altering-the-program-s-execution-flow.html , https://www.jetbrains.com/help/idea/analyze-java-stream-operations.html , https://www.jetbrains.com/help/idea/async-stacktraces.html , https://www.jetbrains.com/help/idea/analyze-objects-in-the-jvm-heap.html
- https://www.jetbrains.com/help/idea/code-coverage.html , https://www.jetbrains.com/help/idea/running-test-with-coverage.html
- https://www.jetbrains.com/help/idea/profiler-intro.html
- https://www.jetbrains.com/idea/whatsnew/2026-2/ (Logpoints)
- Community sources: keymaps; `platform/xdebugger-impl/ui/resources/intellij.platform.debugger.impl.ui.xml`; `plugins/junit`, `plugins/testng`, `plugins/coverage`, `plugins/stream-debugger`, `java/debugger`.

---

## 9. Build

| ID | Feature | Behaviour | Win/Linux | macOS | Entry point | Pri |
|---|---|---|---|---|---|---|
| JAVA-BUILD-001 | Build Project | Incremental build of all modules (via Gradle when delegated, otherwise internal JPS builder). Action id `CompileDirty`. | Ctrl+F9 | Cmd+F9 | Build \| Build Project | P0 |
| JAVA-BUILD-002 | Build Module | Build selected module(s). Action id `MakeModule`. | — | — | Build \| Build Module '…' | P1 |
| JAVA-BUILD-003 | Recompile file / Rebuild module | Recompile selected file(s)/package or rebuild selected module from scratch. Action id `Compile`. | Ctrl+Shift+F9 | Cmd+Shift+F9 | Build \| Recompile '…' / Rebuild Module | P1 |
| JAVA-BUILD-004 | Rebuild Project | Clean full rebuild. Action id `CompileProject`. | — | — | Build \| Rebuild Project | P0 |
| JAVA-BUILD-005 | Build tool window | "Build Output" with tree of messages (errors/warnings grouped by file) and console; navigate to source; filter warnings; "Sync" tab for Gradle sync. | — | — | View \| Tool Windows \| Build | P0 |
| JAVA-BUILD-006 | Compiler settings | Settings \| Build, Execution, Deployment \| Compiler: resource patterns (`!?*.java` etc.), Clear output directory on rebuild, Add runtime assertions for not-null-annotated methods/parameters, Automatically show first error in editor, Display notification on build completion, Build project automatically (when not running), Compile independent modules in parallel, Rebuild module on dependency change, Shared build process heap size and VM options, User-local build process options. | — | — | Settings | P1 |
| JAVA-BUILD-007 | Java Compiler settings | Compiler choice (Javac, Eclipse, Javac in-process; Ajc via plugin), "Use '--release' option for cross-compilation", Project bytecode version, Per-module bytecode version, Generate debug info, Report use of deprecated features, Generate no warnings, Additional command line parameters, Override compiler parameters per module. | — | — | Settings \| … \| Compiler \| Java Compiler | P1 |
| JAVA-BUILD-008 | Annotation processors | Profiles: Enable annotation processing, obtain processors from project classpath or processor path, processor FQNs, options (key=value), generated sources directory (relative to output/content root), store generated sources for production/test, module assignment per profile; Gradle import creates read-only "Gradle Imported" profile(s). | — | — | Settings \| … \| Compiler \| Annotation Processors | P0 |
| JAVA-BUILD-009 | Excludes from compilation | Files/dirs excluded from internal compiler. | — | — | Settings \| … \| Compiler \| Excludes | P2 |
| JAVA-BUILD-010 | RMI / validation | Validation of forms/XML before build (legacy, inferred). | — | — | Settings | P2 |
| JAVA-BUILD-011 | Build artifacts (JAR) | Artifacts in Project Structure (JAR from modules with dependencies, manifest, output layout); Build \| Build Artifacts… (for internal builder; Gradle users typically use `jar`/`shadowJar` tasks). | — | — | Build \| Build Artifacts… | P2 |
| JAVA-BUILD-012 | Build on save / automatically | "Build project automatically" works when no run is active (or with advanced option while running). | — | — | Compiler settings | P2 |
| JAVA-BUILD-013 | Problems from build into Problems window | Compilation errors are mirrored in Problems \| Project Errors (inferred integration). | — | — | Problems tool window | P1 |
| JAVA-BUILD-014 | Build progress in status bar | Background task indicator with cancel. | — | — | Status bar | P0 |
| JAVA-BUILD-015 | Debug Build Process | Attach a debugger to the external build process (diagnostics). Action id `DebugBuildProcess`. | — | — | Find Action | P2 |
| JAVA-BUILD-016 | Build notifications | Balloon on completion with errors/warnings counts and "show" link. | — | — | Notifications | P1 |
| JAVA-BUILD-017 | Before-launch Build | Run configurations build before launch by default (Build / Build, no error check / Gradle task). | — | — | Run configuration \| Before launch | P0 |
| JAVA-BUILD-018 | Delegation note | With Gradle delegation (default for Gradle projects) items 001–004 map to Gradle tasks and most internal compiler settings do not apply; annotation processing then comes from Gradle config. | — | — | — | P0 |

Sources (section 9):
- https://www.jetbrains.com/help/idea/compiling-applications.html , https://www.jetbrains.com/help/idea/build-tool-window.html
- https://www.jetbrains.com/help/idea/compiler.html , https://www.jetbrains.com/help/idea/java-compiler.html , https://www.jetbrains.com/help/idea/annotation-processors-support.html
- https://www.jetbrains.com/help/idea/working-with-artifacts.html
- Community sources: `ActionsBundle.properties` (Build Project / Build Module / Recompile / Rebuild / Rebuild Project), keymaps.

---

## 10. VCS touchpoints for Java projects

VCS is a platform feature specified elsewhere (see `docs/parity/platform.md` and future
`docs/spec/features/platform-*.md`). Java-specific integration points only:

| ID | Feature | Behaviour | Win/Linux | macOS | Pri |
|---|---|---|---|---|---|
| JAVA-VCS-001 | Refactoring-aware VCS moves | Move/rename refactorings perform VCS moves (`git mv` semantics) so history follows. | — | — | P1 |
| JAVA-VCS-002 | Pre-commit checks | Commit dialog/tool window options: Reformat code, Rearrange code, Optimize imports, Analyze code (run inspections on changed files), Check TODO, Cleanup, Run tests / run configuration before commit. | — | — | P1 |
| JAVA-VCS-003 | Gutter change markers and rollback | Changed lines markers, show diff, rollback lines. | Ctrl+Alt+Z (rollback) | Cmd+Opt+Z | P0 |
| JAVA-VCS-004 | Annotate / blame and code vision author | Per-line author; code vision "author" hint on declarations. | — | — | P1 |
| JAVA-VCS-005 | `.gitignore` for build outputs | Suggests ignoring `build/`, `.gradle/`, `out/`, `.idea/workspace.xml` (inferred). | — | — | P2 |
| JAVA-VCS-006 | Sync after VCS update | External-change auto-sync triggers Gradle sync after pull/checkout (GRADLE-024). | — | — | P0 |
| JAVA-VCS-007 | Shared run configurations / code style / inspection profile | `.idea/runConfigurations`, `codeStyles`, `inspectionProfiles` intended for VCS sharing. | — | — | P1 |
| JAVA-VCS-008 | VCS quick list | VCS operations popup. | Alt+Backquote | Ctrl+V | P1 |

Sources (section 10):
- https://www.jetbrains.com/help/idea/version-control-integration.html , https://www.jetbrains.com/help/idea/commit-and-push-changes.html

---
## Summary counts

Counted from the tables above (one row = one feature ID).

| Area (ID prefix) | Features | P0 |
|---|---|---|
| Gradle project model, sync, tool window, DSL, catalogs, inspections, settings (`GRADLE`) | 132 | 44 |
| JDK management and language level (`JAVA-JDK`) | 17 | 11 |
| Editing: highlighting, completion, docs, hints, folding, imports, Javadoc (`JAVA-EDIT`) | 72 | 21 |
| Live templates (`JAVA-LT`) | 36 | 6 |
| Postfix templates (`JAVA-PFX`) | 27 | 4 |
| Formatting and code style (`JAVA-FMT`) | 27 | 7 |
| Modern language support (`JAVA-LANG`) | 18 | 9 |
| Lombok (`JAVA-LOMBOK`) | 7 | 0 |
| Navigation and search (`JAVA-NAV`) | 47 | 22 |
| Inspections, intentions, quick-fixes (`JAVA-INSP`) | 108 | 19 |
| Refactorings (`JAVA-REF`) | 49 | 11 |
| Code generation (`JAVA-GEN`) | 23 | 12 |
| Run configurations (`JAVA-RUN`) | 22 | 13 |
| Testing (`JAVA-TEST`) | 16 | 7 |
| Debugger (`JAVA-DBG`) | 44 | 13 |
| Coverage (`JAVA-COV`) | 5 | 0 |
| Profiler (`JAVA-PROF`, `$`) | 6 | 0 |
| Build (`JAVA-BUILD`) | 18 | 7 |
| VCS touchpoints (`JAVA-VCS`) | 8 | 2 |
| **Total** | **682** | **208** |

Suggested first vertical slice for archipel-ide (all P0): GRADLE-001/003/020/021/040–046/060–064/080–083/100–103/110/112/113/120,
JAVA-JDK-001–010, JAVA-EDIT-001–004/020/024/040/042/070/074, JAVA-NAV-001–003/008/009/019/021, JAVA-INSP-006/014/101–107,
JAVA-REF-002/006/013/018/024, JAVA-GEN-001–009, JAVA-RUN-001–013, JAVA-DBG-001/011/013–019/022/024–026, JAVA-BUILD-001/004/005.

---

## Open questions

1. **Paid-feature boundary in the unified distribution.** The public unified-product page only says "advanced
   tooling" requires a subscription; it does not enumerate whether IntelliJ Profiler, the Dependencies diagram
   (GRADLE-091), UML diagrams (JAVA-NAV-047), the DSM / dependency analysis views, and the "Vulnerable
   Dependencies" Problems tab are free. Needs confirmation from the feature-comparison page and observation
   of a 2026.2 install without a licence.
2. **2026.2 "Logpoints" (JAVA-DBG-008).** What's New only names the feature. Unknown: separate breakpoint
   type vs. preset of a non-suspending line breakpoint, default gutter gesture, message syntax, and whether
   output goes to console or a dedicated view.
3. **2026.2 "dependency completion" (GRADLE-133).** Scope unknown: Gradle Kotlin/Groovy DSL only, version
   catalogs, or also Maven `pom.xml`; data source (local cache vs. online index).
4. **"Early Gradle 10 support".** Which Gradle 10 changes required IDE work (e.g. removed APIs in the Tooling
   API, Declarative Gradle, configuration-cache-by-default)? Minimum supported Gradle version for sync in 2026.2.
5. **Sync vs reload naming.** Bundles in build 262 say "Sync Gradle Changes" / "Sync All Gradle Projects" /
   "Auto-Sync Settings"; some 2026 help pages still say "Re-import" / "Reload". Confirm the visible labels.
6. **Auto-sync default.** The Build Tools page default appears to be "External changes"; verify on a clean
   profile.
7. **macOS shortcuts for actions without explicit macOS bindings.** We derived them via Ctrl→Cmd inheritance
   (e.g. Change Signature `Cmd+F6`, Type Migration `Cmd+Shift+F6`, Recompile `Cmd+Shift+F9`). Resolved: the
   macOS keymap does swap Ctrl and Cmd on inherited shortcuts, and these values match the generated
   [../keymaps.md](../keymaps.md), which is authoritative.
8. **Debug context configuration on Windows/Linux (JAVA-RUN-003).** The `$default` keymap has no binding for
   `DebugClass`; confirm there is no default and that the gutter is the main entry point.
9. **Postfix template list (JAVA-PFX).** The help pages fetched did not enumerate keys; our list is from public
   docs/observation. Need a definitive list for 2026.2 (and which ones are disabled by default) — verify in
   Settings | Editor | General | Postfix Completion.
10. **Live templates `soutc`/`serrc`.** Exact expansion (IO API vs. `System.console()`) and context
    conditions are not documented publicly; verify by observation.
11. **Gradle JVM criteria UI.** When exactly the "Gradle JVM criteria" field replaces "Gradle JVM" (file presence
    vs. registry key), and its interaction with auto-provisioning.
12. **Test-task chooser (GRADLE-121).** Behaviour when a class is covered by several `Test` tasks
    (`test`, `integrationTest`, JVM test suites): popup vs. run all; persisted choice?
13. **Coverage engine choice.** Whether JaCoCo is still selectable alongside the IntelliJ agent in 2026.2 and
    whether coverage for Gradle-run tests uses the IDE agent injected via init script.
14. **HotSwap UI.** Current UX for applying changes (floating "Code changed" button vs. Reload Changed Classes
    action) and support for enhanced class redefinition with JetBrains Runtime/DCEVM.
15. **Inspection counts.** Group counts were computed from Community XML registrations only; additional Java
    inspections may come from plugins bundled only in the paid set (e.g. Spring, JPA, security). Confirm whether
    any "Java"-group inspections are paid.
16. **Java 27 feature list.** Which JEPs are in Java 27 (GA expected 2026-09) and which ones the 2026.2 IDE
    supports at preview level.
17. **Declarative Gradle (GRADLE-031).** Whether it is enabled by default or behind a registry flag in 2026.2.
18. **Command completion (`..`).** Availability by default for Java in 2026.2 and exact trigger rules.

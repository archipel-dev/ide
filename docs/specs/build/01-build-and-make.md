# Build spec 01: Build, Rebuild, Recompile, the Build tool window and auto-build

Status: specification (clean-room). ID prefix `BLD-MAKE`. Priorities `MUST` / `SHOULD` / `LATER` as in `docs/specs/README.md`. Executable cases: `cases/01-build-and-make.cases.md`.

Scope: the user-visible build actions, how they are routed to a build engine (the IDE's own builder or a build tool such as Gradle or Maven), the status texts and notifications, the Build tool window, and the automatic build. Project model terms (modules, source roots, output paths) are in `docs/specs/platform/07-project-structure-and-scopes.md`; the Gradle and Maven delegation settings are in specs 03 and 04, sync in spec 02, JDK resolution in spec 05. The compilation engine itself (javac invocation, incremental dependency tracking) is Archipel's own and not specified here: this document fixes what the user sees and what the build contract is.

ic: `java/java-backend/resources/META-INF/JavaActions.xml`, `java/compiler/impl/src/com/intellij/compiler/actions/` (`CompileActionBase`, `CompileDirtyAction`, `MakeModuleAction`, `CompileAction`, `CompileFileAction`, `CompileProjectAction`), `platform/execution-impl/src/com/intellij/task/impl/ProjectTaskManagerImpl.java`, `java/compiler/impl/src/com/intellij/compiler/impl/CompileDriver.java`, `java/compiler/impl/src/com/intellij/compiler/server/BuildManager.java`, `java/compiler/openapi/src/com/intellij/compiler/CompilerWorkspaceConfiguration.java`, `java/compiler/impl/src/com/intellij/compiler/CompilerConfigurationImpl.java`, `java/compiler/impl/src/com/intellij/compiler/options/` (`CompilerUIConfigurableKt`, `BuildOnSaveInfo`), `java/compiler/openapi/resources/messages/JavaCompilerBundle.properties`, `platform/platform-resources-en/src/messages/ActionsBundle.properties`, `platform/platform-resources/src/keymaps/$default.xml`, `platform/buildView/src/com/intellij/build/` (`BuildContentManagerImpl`, `BuildTreeConsoleView`, `buildTreeFilters.kt`), `platform/lang-api/resources/messages/LangBundle.properties`.

## 1. Actions in the Build menu

The main menu `Build` (mnemonic B) contains, in this order: `Build Project`, `Build Module`, `Rebuild`, `Recompile` (the last two are the same menu slot and exclude each other, see 1.3), a separator, `Rebuild Project`, a separator, `Build Artifacts…`. Build tool contributions (e.g. `Load Gradle Changes`) are listed in their own specs.

| Action id | Menu text | Description (status bar) | Default shortcut |
|---|---|---|---|
| `CompileDirty` | Build Project | Compile all modified and dependent files in the project | Ctrl+F9 |
| `MakeModule` | Build Module | Compile all modified and dependent files in the module | none |
| `Compile` | Rebuild | Force recompilation for the selected module, file, or package | Ctrl+Shift+F9 |
| `CompileFile` | Recompile | (shares the `Compile` description) | Ctrl+Shift+F9 (same as `Compile`) |
| `CompileProject` | Rebuild Project | Recompile all files in the project | none |
| `BuildArtifact` | Build Artifacts… | Select and build artifacts configured in the project | none |

`Build Project` also appears as a toolbar button (right toolbar group, first position) with the compile icon.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAKE-001 | MUST | The Build menu has exactly the entries, order and texts of the table above; mnemonics: B in `Build`, P in `Build Project`, M in `Build Module`, E in `Rebuild`, e in `Recompile` (the `_` in the bundle text marks it), R in `Rebuild Project`. | Case BLD-MAKE-001 case 1 lists the menu. |
| BLD-MAKE-002 | MUST | All five build actions are disabled while a compilation is active (`isCompilationActive`) and while no project is open. Their enablement is computed off the UI thread and works in dumb mode. | Case BLD-MAKE-002 case 1: start a build, open Build menu: `Build Project` disabled; finish: enabled. |
| BLD-MAKE-003 | MUST | Before every build action runs, if an editor with a file is focused and indexing is finished, pending auto-import of the reference at the caret is completed first, then the build starts. The originating action class is remembered as the build originator (telemetry only). | Case BLD-MAKE-003 case 1: unresolved `List` at caret, auto-import available; Ctrl+F9 adds the import before compiling. |
| BLD-MAKE-004 | MUST | `Build Project` (Ctrl+F9) runs an incremental build of all modules of the project: only changed sources and their dependents are rebuilt. | Case BLD-MAKE-004 case 1. |
| BLD-MAKE-005 | MUST | `Build Module`: when the context has one or more selected modules (project view, module context) the item is enabled and its text is `Build Module '<name>'` for one module; for several modules the text is `Build Modules '<a>' and '<b>'` (names quoted, joined with the locale's narrow "and") if the joined names are 20 characters or fewer, otherwise `Build Selected Modules`. Without selection and not from a context menu, the module of the current editor file is used (`Build Module '<name>'`); if none, the item shows the plain template text `Build Module` and is disabled. In the project view popup an unusable item is hidden. The build is incremental and includes the module's dependencies. | Cases BLD-MAKE-005 case 1..4. |
| BLD-MAKE-006 | MUST | `Rebuild` / `Recompile` (shortcut Ctrl+Shift+F9): resolves the target in this order: (1) a module context (module selected in project view or module node) : rebuild the module; (2) the compilable files of the selection; (3) if there are none and the action was not invoked by shortcut and not from a context menu, the current module. Not applicable: disabled (and hidden in context menus). | Cases BLD-MAKE-006 case 1..5. |
| BLD-MAKE-007 | MUST | Compilable files of a selection: a file qualifies only if it is in source content, is on the local file system, and (for a file) its type is compilable or it is a compilable resource file by the resource patterns; a directory qualifies only if it is a Java package directory. Others are dropped. | Case BLD-MAKE-007 case 1: a `.png` in `src/main/resources` is a compilable resource only if the resource patterns allow it. |
| BLD-MAKE-008 | MUST | The item text is `Rebuild ` plus a description of the target: `Module '<name>'`; `'<package>'` (for the default package the literal `'<default>'`); `'<file name>'`; or `Selected Files` for several files. Descriptions longer than 50 characters are shortened to an optional leading `'`, then `...`, then the last 20 characters (the closing quote is part of those 20). `Recompile` is shown instead of `Rebuild` when the target is a file set (single file or selection of files), `Rebuild` for modules and packages: exactly one of the two is visible. | Cases BLD-MAKE-008 case 1..3. |
| BLD-MAKE-009 | MUST | `Rebuild Project` rebuilds all modules non-incrementally. After completion (and not aborted) a local-history system label is written, named `'Rebuild Project' with no errors` or `'Rebuild Project' with errors`. | Case BLD-MAKE-009 case 1. |
| BLD-MAKE-010 | SHOULD | `Build Artifacts…` opens a chooser listing the project's artifacts plus an `All Artifacts` entry (group `Build Artifact`); the chosen artifacts are built with the build tool that owns them. | Case BLD-MAKE-010 case 1. |

### 1.1 Build request semantics (what each action asks the engine to do)

| Request | Incremental | Includes dependent modules | Includes runtime deps |
|---|---|---|---|
| Build Project | yes | all modules listed | no |
| Build Module(s) | yes | yes (module dependencies, compile classpath) | no |
| Rebuild Module(s) | no (forces recompilation of the module's sources) | no | no |
| Rebuild Project | no | n/a (all modules) | no |
| Recompile files | no | n/a (files only) | no |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAKE-011 | MUST | The build entry points are modelled as project tasks: module build, module resources build, module files build, artifact build; each carries `incremental`, `includeDependentModules`, `includeRuntimeDependencies`. The values per action are those of the table above. | Case BLD-MAKE-011 case 1. |
| BLD-MAKE-012 | MUST | Routing: every task is offered to the registered task runners in priority order, the first that accepts it runs it; tasks accepted by the same runner are batched into one run; tasks nobody accepts are handled by the IDE builder. A project may thus mix runners (e.g. Gradle-delegated modules and IDE-built modules). The result of the whole request is the aggregate of the runner results: aborted if any runner aborted, has-errors if any has errors. | Case BLD-MAKE-012 case 1. |
| BLD-MAKE-013 | MUST | Before running, registered build listeners get a veto (`beforeRun`); a veto aborts the request with an aborted empty result and no tool window activity. An empty request (nothing to run) completes as success. Before-run work must not run on the UI thread. | Case BLD-MAKE-013 case 1. |
| BLD-MAKE-014 | MUST | All documents are saved before a build starts (unless the build is an automatic build, see 4). | Case BLD-MAKE-014 case 1. |
| BLD-MAKE-015 | MUST | Runners that cannot report which files they generated make the engine treat the output roots of the built modules as dirty after the build. | Case BLD-MAKE-015 case 1. |
| BLD-MAKE-016 | MUST | Tasks of a run configuration (build-before-run) go through the same routing; a Java scratch file configuration is never delegated to a build tool. | Case BLD-MAKE-016 case 1. |

### 1.2 Rebuild confirmation

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAKE-017 | SHOULD | When a legacy "rebuild" request is started through the compiler API (not through the Rebuild Project action) the user sees a question dialog titled `Confirm Project Rebuild` with text `You are about to rebuild the whole project.` / `Run 'Build Project' instead?` and the two buttons `Build` (default, runs an incremental build instead) and `Rebuild`. | Case BLD-MAKE-017 case 1. |

## 2. Status messages and notifications

After every IDE-builder build a status text and a notification are produced. `{d}` is a human duration of the build rounded approximately (for example `3 sec`, `1 min 5 sec`; format of the platform's approximate duration).

| Outcome | Text |
|---|---|
| Cancelled | `Compilation aborted` |
| Nothing to do | `All files are up-to-date` |
| Success | `Build completed successfully in {d}` |
| Success with warnings | `Build completed successfully with {n} warning(s) in {d}` (`1 warning`, otherwise `N warnings`) |
| Errors | `Build completed with {e} error(s) and {w} warning(s) in {d}` (singular for exactly 1, plural for 0 and 2+) |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAKE-020 | MUST | The status texts are exactly those of the table, with the pluralisation rule: `0 warnings`, `1 warning`, `2 warnings`; likewise errors. | Cases BLD-MAKE-020 case 1..4. |
| BLD-MAKE-021 | MUST | A notification of group `Build finished` is shown with the status text; its type is error if errors > 0, warning if warnings > 0, else info. Except for `All files are up-to-date` the text is a link that activates the Build tool window. The notification is not important and expires when the next build session closes. | Case BLD-MAKE-021 case 1. |
| BLD-MAKE-022 | SHOULD | If the build took longer than one minute and the setting `Display notification on build completion` is on (default on), the status text is additionally shown as a balloon on the Build tool window stripe button. | Case BLD-MAKE-022 case 1. |
| BLD-MAKE-023 | MUST | Unless the build was UP-TO-DATE and unless there were no messages at all, a final line `<date time> - <status text>` is appended as an INFORMATION message to the build output. | Case BLD-MAKE-023 case 1. |
| BLD-MAKE-024 | MUST | The three build kinds are named in the tool window tab and events as `Build`, `Rebuild`, `Recompile`. A Rebuild or Recompile uses the title of its kind. | Case BLD-MAKE-024 case 1. |
| BLD-MAKE-025 | MUST | Message categories shown in the tree are `ERROR`, `WARNING`, `INFORMATION`, `STATISTICS`. | Case BLD-MAKE-025 case 1. |

### 2.1 Failures before the build can start

All of these are shown as a notification of group `Build failed to start`, titled `Cannot start compiler`, with the action `Configure…` that opens the project/module settings (module settings selected where a module is named). Only one such notification is active per project.

| Condition | Text |
|---|---|
| Project SDK / module SDK missing | `The SDK is not specified for module X` (one module) or `The SDK is not specified for modules` followed by a newline-separated list; long lists end with `...` |
| SDK missing or corrupted, automatic fix available | dialog: `SDK is not specified or corrupted for module X. Resolve the following SDKs automatically?` with button `Fix automatically`; when nothing can be fixed: `Manual configuration is still required for:` |
| Output path missing | `The output path is not specified for module X` / `modules` list |
| Cyclic modules with different JDK | `The following modules must have the same JDK assigned because of cyclic dependencies between them:` list `Please update modules configuration` |
| Cyclic modules with different language level | same with `language level assigned` |
| No javac | `No system java compiler is provided by the JRE. Make sure tools.jar is present in IntelliJ IDEA classpath.` |
| Build process JDK too old | `The build process requires JDK {n} or newer, but no compatible JDK was found. Install a JDK {n}+ in the project's environment ({env}) and add it in File | Project Structure | Platform Settings | SDKs.` or, when some JDK exists, `... but the highest JDK in this project is {v}. Install a JDK {n}+ in the project's environment ({env}) and add it in File | Project Structure | Platform Settings | SDKs.` |
| Out of memory in javac | `Out of memory. Increase the maximum heap size in Project Properties|Compiler settings.` |
| Unloaded modules affected | `There are unloaded modules in the project which depend on changed files.` |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAKE-026 | MUST | The precondition checks of the table run before the engine starts; on failure no build output tab is created, the notification of 2.1 is shown, and the build result is aborted. The SDK check uses the resolution order of spec 05. | Cases BLD-MAKE-026 case 1..3. |

## 3. Build tool window

Tool window id `Build`, stripe title `Build`, anchored at the bottom (observed; the descriptor does not state it). One tool window hosts several tabs (contents); tab titles come from the task: `Sync`, `Build`, `Run`, `Debug`, or a custom name. Tab ordering rule: preset order `Sync`, `Build`, `Run`, `Debug`; other tabs follow in creation order.

Each tab has a split view: left a tree of execution nodes, right the console text of the selected node (a `Build Output` text view is the tab title for the plain text output mode). The tree root shows the task name with its status (`running…`, `finished`, `failed`, `cancelled`) and the duration; nodes below group messages by kind: `Build Issues`, `Compiler`, `Kotlin Compiler`, `Other Messages`, per module or file; message nodes show `<text> at <file>:<line>` style positions (`At {0}`, `{0} at {1}`), counts `{n} error(s)`, `{n} warning(s)`, `{x} with {y}`, and `UP-TO-DATE` for up-to-date steps.

Toolbar (group `BuildView`, content assembled by the platform): contains at least the `Filters` popup group (icon "show"), the pin action `PinBuildView`, and Next/Previous Occurrence navigation; further buttons (rerun, stop, expand, collapse) are contributed by the running task and are not enumerated here (open question 4). Filters group:

| Filter action | Id | Default | Persistence key |
|---|---|---|---|
| `Show Warnings` | `BuildViewFilterWarnings` | on | `build.toolwindow.show.warnings.selection.state` |
| `Show Successful Steps` | `BuildViewFilterSuccessful` | off | `build.toolwindow.show.successful.steps.selection.state` |

Semantics of the filters: warnings filter on shows nodes that have warnings or infos; successful-steps filter on shows nodes that neither failed nor have warnings. With both off, only failing nodes (and their ancestors) are visible. State is global and restored for each new build tab.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAKE-030 | MUST | Tool window `Build` exists, is created lazily on the first build/sync event, has stripe title `Build`, and keeps tabs in the preset order above. | Case BLD-MAKE-030 case 1. |
| BLD-MAKE-031 | MUST | A new build of the same task kind reuses its tab. The tree root text carries the status words `running…`, `finished`, `failed`, `cancelled` (`{task} finished` / `failed` / `cancelled` in the summary line). | Case BLD-MAKE-031 case 1. |
| BLD-MAKE-032 | MUST | Tree content per build: root node, then groups; failures are expanded and selected automatically on completion; successful steps are hidden unless the filter is on. Messages carry severity icon, text, and a navigation target (file, line, column). | Case BLD-MAKE-032 case 1. |
| BLD-MAKE-033 | MUST | `Show Warnings` default on, `Show Successful Steps` default off, persisted globally by the keys above; toggling re-filters the visible tree immediately. | Cases BLD-MAKE-033 case 1..2. |
| BLD-MAKE-034 | MUST | Double-click or Enter (Jump to Source, F4) on a message node opens its file at line and column. Next/Previous Occurrence (`Next Occurrence`, `Previous Occurrence`) moves between problem nodes of the tree, skipping group nodes. | Case BLD-MAKE-034 case 1. |
| BLD-MAKE-035 | MUST | On the first error message of a build the editor navigates to it, if the setting `Automatically show first error in editor` is on (default on); a build may override this per run. It happens once per build. | Case BLD-MAKE-035 case 1. |
| BLD-MAKE-036 | SHOULD | `Show More` / `Show Less` inlay appears for long console output lines blocks (texts `Show more` / `Show less`). | Case BLD-MAKE-036 case 1. |
| BLD-MAKE-037 | SHOULD | The tool window is activated on build failure unless the build was an automatic build; an automatic build never steals focus or activates the window. | Case BLD-MAKE-037 case 1. |
| BLD-MAKE-038 | SHOULD | The `Pin` toggle (`PinBuildView`) keeps the tab from being reused; its exact semantics are an open question (5), implement as: a pinned tab is left untouched and the next build opens a new tab. | Case BLD-MAKE-038 case 1. |
| BLD-MAKE-039 | SHOULD | Tool window `Build` mirrors the build tool windows of build tools (`Sync` tab for Gradle/Maven sync, see spec 02). | Case BLD-MAKE-039 case 1. |

## 4. Automatic build ("Build project automatically")

Settings, page `Settings | Build, Execution, Deployment | Compiler`:

| UI label | Stored as | Default | Notes |
|---|---|---|---|
| `Resource patterns:` | project, wildcard list | `!?*.java;!?*.form;!?*.class;!?*.groovy;!?*.scala;!?*.flex;!?*.kt;!?*.clj;!?*.aj` | Legend `Use ; to separate patterns and ! to negate a pattern.`; help `Accepted wildcards` |
| `Clear output directory on rebuild` | workspace | on | warning: `Warning: if enabled, the entire contents of directories where generated sources are stored will be cleared on rebuild.` |
| `Add runtime assertions for notnull-annotated methods and parameters` | project | on | link `Configure annotations…` |
| `Automatically show first error in editor` | workspace | on | |
| `Display notification on build completion` | workspace | on | |
| `Build project automatically` | workspace (`MAKE_PROJECT_ON_SAVE`) | **off** | comment `Only works while not running / debugging` |
| `Rebuild module on dependency change` | workspace | on | |
| `Compile independent modules in parallel:` | project | `Automatic` (options `Enabled`, `Automatic`, `Disabled`) | help `May require a larger heap size`: `In automatic mode, parallel compilation is enabled based on hardware specifications.` |
| Group `Build Process`: `Shared heap size:` (Mbytes) | project | 700 | disabled when a user-local size or an `-Xmx` in user-local VM options is set |
| `Shared VM options:` | project | empty | read-only when user-local VM options are set |
| `User-local heap size:` (Mbytes) | workspace | empty (0 = use shared) | comment `Overrides Shared size` |
| `User-local VM options:` | workspace | empty | comment `Overrides Shared options` |

`Automatic` parallel mode is on iff the machine has at least 4 logical cores and at least 8 GiB of RAM (rounded). The same checkbox appears on `Settings | Tools | Actions on Save` as `Build project` with the comment `Not triggered while running/debugging` and a link `Compiler settings…`. Advanced settings (group Compiler): `compiler.automake.allow.when.app.running` default off (allow auto-build while an application is running), `compiler.lower.process.priority` default on.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAKE-040 | MUST | The settings of the table exist with the exact labels and defaults; stored per scope (project file vs workspace/user-local). | Case BLD-MAKE-040 case 1. |
| BLD-MAKE-041 | MUST | Auto-build runs only if all hold: `Build project automatically` is on; the project is trusted; no run/debug process of this project is active (a process may opt out of blocking), unless the advanced setting `compiler.automake.allow.when.app.running` is on; the machine is not in power-save mode. | Case BLD-MAKE-041 case 1..3. |
| BLD-MAKE-042 | MUST | Trigger and postpone rules: a change schedules an auto-build 300 ms later (restarted by further changes); the run is postponed while there are unsaved documents or the user was active within the last 3000 ms. With several projects open only the focused project is built, except that after 60 s of idle time all open projects are built. A project open also schedules one auto-build. | Case BLD-MAKE-042 case 1. |
| BLD-MAKE-043 | MUST | While auto-build is on, documents are saved automatically 1.5 s after the change (registry-controlled, default off in the registry, implied by the on-save action), and an auto-build is an incremental build of all modules and never prompts. If file system changes were not yet seen by the builder the build is repeated once more. | Case BLD-MAKE-043 case 1. |
| BLD-MAKE-044 | MUST | Auto-build failure is reported as a notification `Auto build failure: {reason}`; a build that finished with errors shows `Auto build completed with errors`. A successful auto-build is silent except for status refresh of the editor error marks. | Case BLD-MAKE-044 case 1..2. |
| BLD-MAKE-045 | MUST | `Rebuild module on dependency change` on: a module whose dependency set changed is fully rebuilt by the next build; off: only dirty files. | Case BLD-MAKE-045 case 1. |
| BLD-MAKE-046 | SHOULD | `Build Process` values: shared heap default 700 MB; the effective heap is the user-local value if > 0, otherwise the shared value; an `-Xmx` in the effective VM options wins over both. | Case BLD-MAKE-046 case 1. |

## 5. Delegation of build and run to a build tool

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAKE-050 | MUST | A task runner provided by a build tool accepts a module build task only if the module belongs to that tool (Gradle: external-system aware module of a linked Gradle project and `Build and run using` is `Gradle`; Maven: Maven module and `Delegate IDE build/run actions to Maven` on). Gradle runs have priority over the IDE builder; Maven delegation is opt-in (off by default). | Cases BLD-MAKE-050 case 1..2 (details in BLD-GRADLE-030, BLD-MAVEN-030). |
| BLD-MAKE-051 | MUST | Delegated builds appear in the Build tool window like IDE builds: tab title `Build`, execution name `Build <project folder name>` for Gradle, status texts of the build tool. The Gradle runner allows only one delegated build at a time (a second request waits). | Case BLD-MAKE-051 case 1. |
| BLD-MAKE-052 | MUST | The tasks that Build/Rebuild map to under Gradle and the phases under Maven are specified in BLD-GRADLE-031 and BLD-MAVEN-031. | n/a (reference) |
| BLD-MAKE-053 | SHOULD | Before/after Build and Rebuild tasks configured in the build tool tool windows (`Execute Before Build`, `Execute After Build`, `Execute Before Rebuild`, `Execute After Rebuild`) run around the corresponding request; a failing before-task aborts it with the dialog message `Before build triggering task failed`; failing after-task: `After build triggering task failed`. | Case BLD-MAKE-053 case 1. |

## 6. Inventory coverage (build area)

| Inventory item | Where | Priority |
|---|---|---|
| `CompileDirty`, `MakeModule`, `Compile`, `CompileFile`, `CompileProject`, `BuildArtifact` | 1 | MUST (BuildArtifact SHOULD) |
| `BuildViewFilterWarnings`, `BuildViewFilterSuccessful` | 3 | MUST |
| `PinBuildView` | BLD-MAKE-038 | SHOULD |
| Settings page `Compiler` (`project.propCompiler`) | 4 | MUST (child tabs Java Compiler, Annotation Processors, Validation, RMI: LATER) |
| `PackageFile`, `ExcludeFromValidation` | - | LATER: artifacts/validation packaging is out of the first release |
| `GenerateJavadoc` | - | LATER: tools action, not a build requirement |
| `CompilationCharts*` (4 actions) | - | LATER: profiling overlay of the compiler, no user need in first release |
| Ant: `ActivateAntToolWindow`, `AddAntBuildFile`, `RunTargetAction`, tool window `Ant`, run type `Ant Target` | - | LATER: Ant is not a supported build tool in Archipel |
| Settings pages `Kubernetes`, `Java Profiler` (promos), `Clouds` | - | LATER: promotional / remote-deployment pages |
| `Remote Jar Repositories` page (`RemoteRepositoriesConfigurable`) | spec 04 BLD-MAVEN-070 | SHOULD |

## Open questions

1. The `Confirm Project Rebuild` dialog is reached only on one code path of the compile driver (when a build task is started in the "rebuild" mode and the user is asked after the up-to-date check). Which user gesture exactly triggers it in current IDEs? Archipel treats it as LATER until observed.
2. The duration format of the approximate duration (`NlsMessages.formatDurationApproximate`) is locale-dependent; Archipel should pick `N sec`, `N min N sec` (observed English form) and confirm by observation.
4. The full list of Build tool window toolbar buttons is assembled at runtime by the platform; list by observation.
5. Exact semantics of the pin action.
3. Whether `Build Module` includes runtime dependencies is false in the source; confirm by observation of a Gradle `runtimeOnly` dependency.

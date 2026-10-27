# Build spec 03: Gradle

Status: specification (clean-room). ID prefix `BLD-GRADLE`. Executable cases: `cases/03-gradle.cases.md`. The generic external-system behaviour (auto-sync, Load button, sync tab, tool window skeleton, task activation, trust, unlinked-project detection) is in spec 02 and is not repeated; JDK/Gradle-version compatibility and Gradle JVM resolution are in spec 05; build/run delegation as seen from the Build menu is in spec 01.

ic: `plugins/gradle/settings/src/settings/` (`GradleProjectSettings`, `GradleSettings`, `GradleSystemSettings`, `DistributionType`, `TestRunner`), `plugins/gradle/settings/src/util/GradleConstants.java`, `plugins/gradle/src/org/jetbrains/plugins/gradle/service/settings/` (`IdeaGradleProjectSettingsControlBuilder`, `IdeaGradleSystemSettingsControlBuilder`, `GradleLocationSettingType`, `GradleConfigurable`), `plugins/gradle/plugin-resources/intellij.gradle.xml`, `plugins/gradle/resources/messages/GradleBundle.properties`, `plugins/gradle/src/org/jetbrains/plugins/gradle/service/project/GradleAutoImportAware.java`, `plugins/gradle/src/org/jetbrains/plugins/gradle/action/` (`GradleExecuteTaskAction`, `ToggleOfflineAction`, `GradleDownloadSourcesAction`, `GradleRefreshProjectDependenciesAction`, `ImportProjectFromScriptAction`), `plugins/gradle/src/org/jetbrains/plugins/gradle/execution/GradleRunAnythingProvider.kt`, `plugins/gradle/src/org/jetbrains/plugins/gradle/service/execution/` (`GradleRunConfiguration`, `GradleExternalTaskConfigurationType`), `plugins/gradle/java/src/execution/build/` (`GradleProjectTaskRunner`, `TasksExecutionSettingsBuilder`), `plugins/gradle/src/org/jetbrains/plugins/gradle/issue/`, `plugins/gradle/src/org/jetbrains/plugins/gradle/internal/daemon/`, `plugins/gradle/java/groovy/src/execution/GradleGroovyRunLineMarkerProvider.java`, `plugins/gradle/src/org/jetbrains/plugins/gradle/properties/`, `platform/external-system-impl/.../service/execution/configuration/`.

## 1. Settings page

Page `Settings | Build, Execution, Deployment | Build Tools | Gradle` (id `reference.settingsdialog.project.gradle`, title `Gradle`), per linked project (a project chooser appears when several are linked). Layout, top to bottom:

| Group / control | Label (exact) | Default | Notes |
|---|---|---|---|
| (import) | `Download external annotations for dependencies` | off | stored in project settings |
| (import, deprecated) | `Generate {IDE} module per Gradle source set` with warning `This setting is deprecated and remains only for troubleshooting since it's not fully compatible with Gradle's model. Please consider restoring it to the default (checked)` | on | shown only when it is off, or when the registry `gradle.settings.showDeprecatedSettings` is on |
| group `Build and Run` | hint `By default {IDE} uses Gradle to build the project and run the tasks.` / `In a pure Java/Kotlin project, building and running by means of the IDE might be faster, thanks to optimizations. Note, that the IDE doesn't support all Gradle plugins and the project might not be built correctly with some of them.` | | |
| | `Build and run using:` combo: `Gradle` (marked `Default`) / `{IDE}` | `Gradle` | stored as "delegated build" boolean |
| | `Run tests using:` combo: `Gradle` (marked `Default`) / `{IDE}` / `Choose per test` | `Gradle` | values GRADLE, PLATFORM, CHOOSE_PER_TEST |
| group `Gradle` | `Distribution:` combo, see 1.1 | `Wrapper` | |
| | `Gradle location:` / `Location:` path field with browse (`Choose Gradle Installation`), empty text `Path to local Gradle` | empty | visible only for `Local installation` |
| | `Gradle JVM:` JDK combo (see spec 05) or, for projects using Daemon JVM criteria, the group `Gradle JVM criteria:` with `Version:` and `Vendor:` | `#USE_PROJECT_JDK` ("Project SDK") | |
| global part | `Gradle user home:` path field (hint `Override the default location where Gradle stores downloaded files, e.g. to tune anti-virus software on Windows`; chooser `Select Gradle User Home Directory`; error `Can not find Gradle user home`) | empty (placeholder shows the real default) | project-level |
| | `Gradle VM options:` (deprecated: tooltip `This setting is deprecated, please use 'org.gradle.jvmargs' property in 'gradle.properties' file instead`; link `Migrate`; when migrated shows `VM options have been moved to gradle.properties`) | empty | application-level |
| | `Generate *.iml files for modules imported from Gradle` (hint `Enable if you have a mixed project with {IDE} modules and Gradle modules so that it could be shared via VCS`) | on | inverse of "store project files externally" |
| | `Enable parallel Gradle model fetching for Gradle 7.4+` (hint `Allow collecting Gradle models in parallel during a project reload.`, warning `Incubating. Non-thread safe access to shared Gradle configuration data in build.gradle may cause concurrency issues.`) | off | experimental feature flag |

### 1.1 Distribution

| Item text | Meaning | Default |
|---|---|---|
| `Wrapper` | use `gradle/wrapper/gradle-wrapper.properties` of the project; if its files are missing an error icon and text `'gradle-wrapper.properties' not found` is shown | **yes** (selected when none is stored) |
| `Wrapper task` | wrapper configured by a `wrapper` task in the build script; removed from the list when creating a default module | no |
| `Local installation` | user-chosen Gradle home | no |
| `Bundled` | Gradle distribution shipped with the IDE | no |

Location validation (on Apply when `Local installation`): empty → `Gradle location is not specified`; invalid → `Gradle location is incorrect.` + newline + `Location:{path}`. Location colouring and balloons (shown shortly after the page opens): deduced from the environment (inactive grey text; balloon `Gradle location is deduced`), unknown (warning balloon `Gradle location is unknown`), explicit correct (`Gradle location is defined`), explicit incorrect (`Gradle location is incorrect`). Gradle JVM validation: unset → `Please set the Gradle JVM option`; invalid path → `Gradle JVM option is incorrect:` + newline + `Path:{path}`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-GRADLE-001 | MUST | The page exists under Build Tools with the controls, labels and defaults of section 1; a control that does not apply to the context is hidden, not disabled. | Case BLD-GRADLE-001 case 1. |
| BLD-GRADLE-002 | MUST | Defaults of the project settings: Gradle JVM = project SDK, distribution = wrapper, `Build and run using` = Gradle, `Run tests using` = Gradle, module per source set = on, download external annotations = off, offline = off. | Case BLD-GRADLE-002 case 1. |
| BLD-GRADLE-003 | MUST | Distribution combo, location field visibility (only for `Local installation`) and the wrapper-missing hint of 1.1; validation messages of 1.1 block Apply with the exact texts. | Cases BLD-GRADLE-003 case 1..4. |
| BLD-GRADLE-004 | MUST | `Build and run using` / `Run tests using` combos with the `Default` marker, tooltip hint text, and the behaviours of BLD-GRADLE-030, 050. | Case BLD-GRADLE-004 case 1. |
| BLD-GRADLE-005 | SHOULD | Gradle user home field: placeholder shows the resolved default (`GRADLE_USER_HOME` env, else `~/.gradle`); changing it notifies listeners and applies to following sync/run. | Case BLD-GRADLE-005 case 1. |
| BLD-GRADLE-006 | SHOULD | `Gradle VM options` migration: `Migrate` asks `Would you like to move VM options to the '{file}' file?` + `Note that the existing 'org.gradle.jvmargs' property will be overwritten.` + `You can do it manually any time later`; on error title `Migration Error`, text `{message}` + `Please migrate settings manually`. | Case BLD-GRADLE-006 case 1. |
| BLD-GRADLE-007 | SHOULD | Parallel model fetch and `*.iml` generation checkboxes as in the table. | Case BLD-GRADLE-007 case 1. |
| BLD-GRADLE-008 | MUST | Changing the Gradle home, JVM, distribution type, source-set option, build delegation or test runner is detected and published to listeners; changing "module per source set" marks the project dirty (Load button / reload per spec 02). | Case BLD-GRADLE-008 case 1. |
| BLD-GRADLE-009 | SHOULD | Advanced settings (page `Advanced Settings`, group `Build Tools. Gradle`): `Run using Gradle` (default on; description `By default, the IDE uses Gradle to build the project and run tasks. If disabled, application configurations will run using the IDE.`), `Download sources` (default on; `Download sources for project dependencies during the project import. A project sync is required to download the sources after selecting this option.`), `Attach scripts dependencies sources` (default off; `Attach sources for build scripts dependencies during the project import. A project sync is required to attach the sources after selecting this option.`). | Case BLD-GRADLE-009 case 1. |

## 2. Auto-sync files for Gradle

The set of settings files of a linked Gradle project (a change to any of them makes the project stale, spec 02):

1. every `*.gradle` and `*.gradle.kts` file directly inside each module directory of the project (not recursive);
2. `gradle.properties` in the Gradle user home, in the build root and in the Gradle home;
3. `.gradle/config.properties` of the build root (local properties, key `java.home`);
4. `gradle/gradle-daemon-jvm.properties`;
5. the user's `init.gradle` in the Gradle user home;
6. all version catalog files of the imported project;
7. `gradle/wrapper/gradle-wrapper.properties` when the distribution is `Wrapper`.

A changed path is mapped to the linked project that contains it by walking up the directories to the nearest module directory; files inside compiler output directories and directories are ignored; only `*.gradle` / `*.gradle.kts` files are accepted by this mapping step.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-GRADLE-020 | MUST | The seven classes of settings files above are tracked. | Cases BLD-GRADLE-020 case 1..3. |
| BLD-GRADLE-021 | MUST | Mapping of a changed script to its linked project (walk up to the nearest known module directory; ignore output dirs, directories and non-Gradle extensions; not applicable when a partial-resolution policy is used). | Case BLD-GRADLE-021 case 1. |

## 3. Build and run delegation

Delegation is on when the linked project's `Build and run using` is `Gradle` (default). Run delegation additionally requires the advanced setting `Run using Gradle` (default on).

Which tasks Build and Rebuild ask Gradle for: for each module (a source-set module is one Gradle source set), the Gradle task path is `<gradlePath>:<name>` where the name is `classes` for the `main` source set and `<sourceSet>Classes` for others (`testClasses` for tests); resources-only builds use `processResources`/`process<SourceSet>Resources`. If the task does not exist but the source set is `main` or `test`, the module's `assemble` task is used. For a non-source-set module the `classes` task (and `testClasses` when `classes` exists) or `assemble` is used. A non-incremental request (Rebuild) additionally forces compile tasks to be considered out of date through an init script. Runtime dependencies required → a compile-classpath packaging property is set. Tasks are grouped per Gradle root project and each root is executed as one Gradle invocation named `Build {root folder name}`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-GRADLE-030 | MUST | A module build is delegated iff the module belongs to a linked Gradle project and delegation is on; a run configuration is delegated iff the module belongs to Gradle and run delegation is on; Java scratch configurations are never delegated. Gradle's runner has priority over the IDE builder. | Cases BLD-GRADLE-030 case 1..3. |
| BLD-GRADLE-031 | MUST | The task selection rules above. | Cases BLD-GRADLE-031 case 1..4. |
| BLD-GRADLE-032 | MUST | One delegated build at a time per project; the Build tool window opens its tab named `Build {folder}`; on failure the tool window is activated unless the build is automatic; afterwards output roots (reported by an init script or computed from the modules) are refreshed so that class files are visible. | Case BLD-GRADLE-032 case 1. |

## 4. Gradle tool window

Tool window `Gradle` (right). Toolbar order: sync all, select data, download sources, unlink, execute task, toggle offline, composite build configuration, dependency analyzer, expand, collapse, settings. Project context menu additionally: `Composite Build Configuration`, `Refresh Gradle Dependencies` (description `Refresh dependencies in the Gradle cache using --refresh-dependencies argument`), `Analyze Gradle Dependencies…`. The project view popup offers `Link Gradle Project` on a build script.

| Action | Text | Behaviour |
|---|---|---|
| `Gradle.ExecuteTask` | `Execute Gradle Task` (search title; icon run anything) | opens Run Anything prefilled with `gradle ` |
| `Gradle.ToggleOfflineAction` | `Toggle Offline Mode` (desc `Toggle offline mode for Gradle builds`) | toggles `--offline` for all runs |
| `Gradle.DownloadSources` | `Download Sources` (desc `Download the source code of all libraries used in the project`; busy text `Downloading sources…`) | downloads sources of all dependencies |
| `Gradle.OpenProjectCompositeConfiguration` | `Composite Build Configuration` | dialog (section 7) |
| `Gradle.RefreshDependencies` | `Refresh Gradle Dependencies` | runs a build with `--refresh-dependencies` |
| `Gradle.ShowDaemons` | `Show Gradle Daemons` | dialog (section 9) |
| `Gradle.ImportExternalProject` | `Link Gradle Project` (desc `Link Gradle project described by this file`) | links the selected build file |

Nodes: project and module nodes, `Tasks` (grouped per spec 02), `Run Configurations`, `Dependencies`-style nodes are not present in the Gradle view (dependencies are analyzed in the Dependency Analyzer).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-GRADLE-040 | MUST | Toolbar and context menu entries as listed; texts exact. | Case BLD-GRADLE-040 case 1. |
| BLD-GRADLE-041 | MUST | Run Anything provider: the help command `gradle <taskName...> <--option-name...>` (aliases `gradlew`, `./gradlew`, `gradle.bat`), group title `Gradle Tasks`, execution context = project (the linked project path, or the project path) or a module (its linked module path, but not a source-set module). Completion ordering depends on the typed prefix: after `--`, task options then long then short options then class args then wildcard tasks then `:path` tasks; after `-`: task options, short, long, ...; after `:`: `:path` tasks first; otherwise class args, wildcard tasks, path tasks, task options, long options, short options. Each group is sorted alphabetically. Run creates a temporary Gradle run configuration (or selects the equal existing one). | Cases BLD-GRADLE-041 case 1..4. |
| BLD-GRADLE-042 | MUST | Invalid command line (cannot be parsed): a warning balloon titled `Command-line arguments cannot be parsed` with the italic command line and the parser message; nothing runs. | Case BLD-GRADLE-042 case 1. |
| BLD-GRADLE-043 | MUST | Gradle run configuration (type id `GradleRunConfiguration`, factory `Gradle`, dumb-aware): fields `Gradle project` (working directory), `Tasks and arguments` (command line, hint `Example: build --debug. Default tasks will be executed if no tasks are specified.`, empty states `No Gradle Tasks` / `No Gradle Arguments`, table columns `Task`, `Argument`, `Description`, dialog title `Tasks and Arguments`, tooltip `Insert Tasks and Arguments…`), VM options, environment variables, `Gradle JVM`/distribution fragments, debug options: `Debug Gradle scripts`, `Debug forked Gradle tasks in separate debug tabs` (comment `A single Gradle debug session is used for all tasks. Enable this option to debug tasks running in parallel separately.`), `Debug all Gradle tasks on the execution graph` (comment `Debug flags are added to all compatible tasks on the Gradle execution graph. May re-trigger task execution.`), `Run as test` (comment `Shows test view and forces test tasks re-execution`), `Debugging disabled`/`Profiling disabled`/`Coverage disabled` (hints: `Disables and hides the debugging action for this run configuration`, `Disables and hides the profiling action for this run configuration`, `Disables and hides the "Run with Coverage" action for this run configuration`). Empty project path error: `Gradle project is not specified`. Task names and script parameters are stored separately. | Cases BLD-GRADLE-043 case 1..3. |
| BLD-GRADLE-044 | MUST | Offline mode: stored per project (workspace-shared `gradle.xml`), default off; when on, `--offline` is added to every Gradle invocation including sync; a failed dependency resolution in offline mode shows the quick fix `Disable offline mode and reload the project` (sync) or `Disable offline mode and rerun the build` (run). | Cases BLD-GRADLE-044 case 1..2. |
| BLD-GRADLE-045 | SHOULD | `Execute Gradle Task` shortcut/search entry; `Toggle Gradle Offline Mode` searchable. | Case BLD-GRADLE-045 case 1. |
| BLD-GRADLE-046 | SHOULD | `Download Sources`: runs with a background progress `Downloading {name}`; on failure notification (sticky balloon, group `Gradle`) titled `Sources download failed` with `Sources were not downloaded for: {list}` and, per repository failure, `Unable to download {x} from {repo}. Ensure that all the artifact's components are available in this repository. Alternatively, adjust the order of repositories in build.gradle to prioritize downloading from the repository with the full set of components.` | Case BLD-GRADLE-046 case 1. |
| BLD-GRADLE-047 | SHOULD | Tasks index: the list of tasks (name, description, group) per project is cached from sync and used for completion; run configuration names for tests follow `{task} in {module}` / `{task} for {target}` / `{first} and {n} more`. | Case BLD-GRADLE-047 case 1. |

## 5. Test runner integration

`Run tests using` selects who runs tests started from the gutter, `Run '...'` and the context menu:

| Value | Behaviour |
|---|---|
| Gradle | tests run through the Gradle `test` task with a test filter (`--tests`); results arrive as test events into the Run tool window test tree; `Run as test` is on |
| {IDE} | IDE-native JUnit/TestNG runner; Gradle is used only to build when delegation is on |
| Choose per test | the first run asks (popup `Run Tasks` / `Run Tasks for {module}`, hint `Select several tasks to run them at once`, empty `No tasks available`) |

Test tree extras: when no events are received the node text is `Test events were not received`; actions `Open Gradle test report` (`Open Gradle reports produced by the Test task`), `Show Internal Gradle Test Suites` (`Show internal Gradle test suites, e.g. 'Gradle Test Executor N'`, default off), a link `See tests view at Run tool window` from the Build window.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-GRADLE-050 | MUST | Test run routing by the setting; for `Gradle`, a test class/method/pattern/directory/package becomes a Gradle run configuration with `--tests` filters; for `Choose per test` the chooser of the table. | Cases BLD-GRADLE-050 case 1..3. |
| BLD-GRADLE-051 | MUST | Test events populate the test tree; internal executor suites hidden unless the action is on; `Test events were not received` when the build ended without events. | Case BLD-GRADLE-051 case 1. |
| BLD-GRADLE-052 | SHOULD | `Open Gradle test report` opens the HTML report of the test task. | Case BLD-GRADLE-052 case 1. |

## 6. Task activation, run configurations in the view

As spec 02 section 7. Run configurations of Gradle type appear under `Run Configurations`; Gradle tasks gain gutter run icons (see 8).

## 7. Composite builds

`Composite Build Configuration` dialog (title `Composite Build Configuration`): text `Please select participant projects to include in the Gradle composite.`, a checkbox tree of candidate builds, buttons `Select All`, `Unselect All`. A composite definition is either detected from `includeBuild` in settings or chosen explicitly; stored in `gradle.xml` under the linked project. Participants are passed to Gradle as included builds (`--include-build`). Orphan modules caused by composites on old Gradle get the message `Gradle project structure problems` / `Can't find the following module(s): {list}` + `It can be caused by composite build configuration inside your *.gradle scripts with Gradle version older than 3.3.` + `Try Gradle 3.3 or better or enable '{option}' option`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-GRADLE-054 | SHOULD | Composite dialog and storage as above; an included build's directory is its own build root for property lookups. | Case BLD-GRADLE-054 case 1. |
| BLD-GRADLE-055 | SHOULD | The orphan-module message. | Case BLD-GRADLE-055 case 1. |

## 8. Source sets → modules, editor features

With `Generate {IDE} module per Gradle source set` on (default) each Gradle project yields a parent module `app` plus child modules `app.main`, `app.test` (one per source set); Gradle-managed dependency scopes map as in spec 07 of platform. `buildSrc` is its own build root. Gutter: in a Groovy `build.gradle` (and Kotlin DSL scripts through the Kotlin plugin), a green run icon is shown at each task declaration, at the same size as the ordinary run marker; its popup lists the run/debug/coverage executor actions for that task. Kotlin script tasks behave the same.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-GRADLE-056 | MUST | Source-set to module mapping, module naming `{project}.{sourceSet}`; `Tasks` of a module do not include the tasks of other source sets. | Case BLD-GRADLE-056 case 1. |
| BLD-GRADLE-057 | SHOULD | Task gutter icon in `build.gradle` / `build.gradle.kts` task declarations with the executor actions. | Case BLD-GRADLE-057 case 1. |
| BLD-GRADLE-058 | LATER | Gradle script editing intelligence (DSL completion, version catalogs `libs.x` resolve, `Add Gradle Plugin`, `Add Maven artifact dependency` generate actions: `Add Gradle Plugin` / `Add selected Gradle plugin to the project`, popup `Choose Plugin`; `Add Maven artifact dependency`): belongs to the language specs, not the build tool. | n/a |

## 9. Gradle daemons dialog

`Show Gradle Daemons` (dialog `Gradle Daemons`): columns `PID`, `Status`, `Timestamp`, `Info`; buttons `Show Stopped`, `Refresh`, `Stop Selected`, `Stop All`, `Stop When Idle All`, `Close`; description header `Description`; footnote `Daemons started by {IDE} (or other daemons of the similar configuration) are displayed. Gradle 4.5 or greater supported.`

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-GRADLE-059 | LATER | Daemons dialog (texts above); a diagnostic tool, not needed for first release. | n/a |

## 10. Sync issues and quick fixes

Issues turn a raw Gradle failure into a structured build issue with title, description and quick fix links. Each is a node in the Sync tab with a link row.

| ID | Pri | Condition → title / text / fix | Acceptance |
|---|---|---|---|
| BLD-GRADLE-060 | MUST | Gradle JVM incompatible with the Gradle version: title `Incompatible Gradle JVM`; text `Your build is currently configured to use incompatible Java {j} and Gradle {g}. Cannot sync the project.` (variants for unknown Gradle: `... incompatible Java {j}. Cannot sync the project.`, unknown Java: `... incompatible Java. Cannot sync the project.`), then `The minimum compatible Gradle JVM version is {v}.` / `The maximum compatible Gradle JVM version is {v}.` / `The minimum compatible Gradle version is {v}.` / `The maximum compatible Gradle version is {v}.`; fixes `Use Java {v} as Gradle JVM: Open Gradle settings` (or `Use Java {v} or newer as Gradle JVM: ...`), `Change Gradle wrapper version to {v} and re-sync`. | Cases BLD-GRADLE-060 case 1..2. |
| BLD-GRADLE-061 | MUST | JVM unsupported by the IDE: title `Unsupported Gradle JVM`; `Java {j} is not supported for Gradle execution. Cannot sync the project.`; unknown: `Failed to get a Java version for Gradle sync. This could happen because of invalid or outdated Gradle JVM.`; `The minimum supported Gradle JVM version is {v}.` | Case BLD-GRADLE-061 case 1. |
| BLD-GRADLE-062 | MUST | Unsupported Gradle: title `Unsupported Gradle Version`; `Gradle {g} is not supported. Cannot sync the project.`; unknown `Failed to get a Gradle version. This could happen because of invalid or outdated Gradle wrapper.`; `We recommend upgrading to Gradle version {v}.` / `We recommend upgrading to at least Gradle version {v}.` / `The minimum supported Gradle version is {v}.`; deprecated: title `Deprecated Gradle Version`, `Gradle {g} support is deprecated and scheduled for removal.`; outdated minor: `New Minor Gradle Version Available`, `Gradle {g} is not the latest minor version.` | Case BLD-GRADLE-062 case 1. |
| BLD-GRADLE-063 | MUST | Incorrect Gradle JVM setting: title `Gradle JVM option is incorrect`; `Unable to use JDK from {path} with project located in {project}.`; no JDK for tooling: `Unable to run Gradle` / `Unable to find SDK for Gradle execution` with action `Set Project SDK`; fix link `Open Gradle Settings`. | Case BLD-GRADLE-063 case 1. |
| BLD-GRADLE-064 | SHOULD | Daemon JVM criteria issues: titles `Not Found Toolchain Matching Gradle Daemon JVM Criteria`, `Invalid Gradle Daemon JVM Criteria`, `Undefined Gradle Daemon JVM Criteria`, `Unable to Install Toolchain Matching Criteria`, `Unexpected Downloaded Toolchain Specs`, `Undefined Toolchain Download Repositories`; fixes `Add default toolchain download repository plugin`, `Add default Daemon JVM criteria`, `Modify Daemon JVM criteria`, `Install required toolchain`, `Recreate toolchain download URLs`. Text `Failed to locate and download a Daemon JVM with matching criteria (version={v}, vendor={vendor}). Consider installing it manually or modifying Daemon JVM criteria.` (vendor `Any` when unset). | Case BLD-GRADLE-064 case 1. |
| BLD-GRADLE-065 | SHOULD | Unresolved dependency: title `Could Not Resolve {artifact}` / `Could Not Resolve {artifact} for {owner}`; fix text `Declare repository providing the artifact, see the documentation at {url}`; offline-mode fixes of BLD-GRADLE-044; JUnit 6 on Java < 17: `Dependency resolution is looking for a library compatible with JVM runtime version {v}, but JUnit 6 is only compatible with JVM runtime version 17 or newer.` | Case BLD-GRADLE-065 case 1. |
| BLD-GRADLE-066 | SHOULD | Wrapper update failure: notification group `Gradle wrapper update failed`; text `Unable to update wrapper files` + `See IDE log for the details.`; command name `Update Gradle Wrapper Version`. | Case BLD-GRADLE-066 case 1. |
| BLD-GRADLE-067 | SHOULD | The Gradle daemon JVM migration notification: title `Migrate to Gradle Daemon toolchain`, text `Projects using Daemon toolchain allow builds to automatically detect installed toolchains given the defined JVM criteria or download a compatible one if cannot be found locally. In addition, using the Daemon toolchain aligns the selection between CLI and IDE, avoiding spawning multiple Daemons improving performance but also makes it simple to handle the required toolchain on different machines.`, buttons `Migrate`, `Ignore`, `Learn more`; error `Project Migration Error`. | Case BLD-GRADLE-067 case 1. |

## 11. Dependency analyzer, misc

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-GRADLE-070 | SHOULD | Dependency analyzer entries: `Analyze Gradle Dependencies…` in the project/module/dependency menus, toolbar button, project view popup; `Go to Gradle Dependency`, `Open Gradle Config`; loading text `Load Gradle Dependencies`. See spec 02 section 11. | Case BLD-GRADLE-070 case 1. |
| BLD-GRADLE-071 | SHOULD | Gradle run/sync progress titles: `Running tasks`, `Configuring projects`, `Building`, `Building model`, `Configuring build`, `Running task`, `Downloading {x}`; execution names `Build {folder}`, `Upgrade Gradle wrapper`, `Configure Daemon JVM Criteria`; generating a wrapper: `Generating Gradle Wrapper`. | Case BLD-GRADLE-071 case 1. |
| BLD-GRADLE-072 | LATER | Remote targets (Gradle on a WSL/docker target: `Gradle configuration`, `Run Gradle task`, `Project folder:`, `Gradle home path:`), `Gradle DSL:` choice Kotlin/Groovy in the new project wizard. | n/a |

## 12. Inventory coverage

Actions `Gradle.ExecuteTask`, `Gradle.ToggleOfflineAction`, `Gradle.DownloadSources`, `Gradle.OpenProjectCompositeConfiguration`, `Gradle.RefreshDependencies`, `Gradle.ShowDaemons` (LATER), `Gradle.ImportExternalProject`, `Gradle.ViewDependencyAnalyzer`, `Gradle.ToolbarDependencyAnalyzer`, `Gradle.ProjectViewDependencyAnalyzer`, `Gradle.DependencyAnalyzer.GoTo`, `Gradle.DependencyAnalyzer.OpenConfig` (BLD-GRADLE-070), `AddGradleDslPluginAction`, `Gradle.AddGradleDslDependencyAction` (LATER BLD-GRADLE-058). Settings page `reference.settingsdialog.project.gradle`, tool window `Gradle`, run configuration type `GradleExternalTaskConfigurationType` (BLD-GRADLE-043).

## Open questions

1. Which distribution items the IDE page really shows by default (`Wrapper`, `Wrapper task`, `Local installation`, `Bundled`): the builder can drop each; confirm `Bundled`/`Wrapper task` visibility in a current IDE.
2. Exact toolbar button order of the Gradle tool window beyond the groups named in the plugin descriptor.
3. Task names for non-Java source sets (Kotlin multiplatform) and Android.
4. Whether the module-per-source-set naming is `app.main` or `app:main` in the current import mode (both exist, depending on qualified-name option).

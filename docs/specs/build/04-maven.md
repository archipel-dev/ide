# Build spec 04: Maven

Status: specification (clean-room). ID prefix `BLD-MAVEN`. Executable cases: `cases/04-maven.cases.md`. The generic behaviour (auto-sync tracker, Load button, Sync tab, trust, unlinked-project detection, task activation, dependency analyzer) is in spec 02; the importer JDK and Maven JDK requirements are in spec 05; delegating the IDE build to Maven as seen from the Build menu is in spec 01.

ic: `plugins/maven/src/main/resources/intellij.maven.xml`, `plugins/maven/src/main/resources/messages/` (`MavenConfigurableBundle`, `MavenProjectBundle`, `MavenRunnerBundle`, `MavenSyncBundle`, `MavenTasksBundle`), `plugins/maven/src/main/java/org/jetbrains/idea/maven/project/` (`MavenGeneralSettings`, `MavenImportingSettings`, `MavenProjectsManager`, `auto/reload/`), `.../execution/` (`MavenRunnerSettings`, `MavenRunConfiguration`, `MavenRunnerParameters`, `build/MavenProjectTaskRunner.kt`, `build/MavenDelegateBuildPhase.kt`), `.../navigator/` (`MavenProjectsNavigatorState`, `structure/MavenProjectsStructure.kt`), `plugins/maven-server-api/src/main/java/org/jetbrains/idea/maven/model/MavenConstants.java`.

## 1. Settings pages

Page tree under `Build Tools`: `Maven` (general), children `Importing`, `Ignored Files`, `Runner`, `Running Tests` (only for real projects, not the default project template), `Archetype Catalogs`. The page id is `MavenSettings`.

### 1.1 Maven (general) page

| Label | Default | Notes |
|---|---|---|
| `Maven home path` combo | `Bundled (Maven 3)` | other items: `Bundled (Maven 4)`, `Use Maven wrapper` (secondary text `Use version defined in .mvn/wrapper/maven-wrapper.properties`), `Maven at '{path}'` for each detected/entered home; `Specify Maven home` browse (`Select Maven Home Directory`); invalid home: label `Invalid Maven home directory` with `(Version: {v})`; message `Maven home directory is invalid: {path}`; bundled variants carry the secondary text `Bundled` |
| `User settings file` | computed (`~/.m2/settings.xml`); `Override` checkbox | chooser `Select User Settings File`; background progress `Computing user settings location…` |
| `Local repository` | computed (`~/.m2/repository`) with `Override` | chooser `Select Local Repository`; progress `Computing repository location…` |
| `Use settings from .mvn/maven.config` | on | tooltip `Settings from maven.config file override the IDE Maven UI settings.` / `If this option is enabled, use maven.config to change the settings.` / `Otherwise, any changes made in the UI might be lost after reimporting.`; with it on the controls overridden by the file show hint `Any changes made in its configuration might be lost after reimporting.` |
| `Show settings dialog for new Maven projects` | off | |
| `Work offline` | off | adds `--offline` |
| `Execute goals recursively` | on | off adds `--non-recursive` |
| `Print exception stack traces` | off | adds `--errors` |
| `Always update snapshots` | off | forces `-U` |
| `Output level:` | `Info` (items `Debug`, `Info`, `Warn`, `Error`, `Fatal`, `Disabled`) | |
| `Checksum policy:` | `No Global Policy` (items `Fail`, `Warn`) | |
| `Multiproject build fail policy:` | `Default` (items `Fail Fast`, `Fail At End`, `Fail Never`) | |
| `Thread count:` | empty | passed as `-T`, e.g. `2.0C`; note `-T option` |
| Emulate terminal | off | `Emulate terminal` / `Enables native terminal emulation` |

### 1.2 Importing page

| Label | Default | Notes |
|---|---|---|
| `Enable fast import` | per product | uses the workspace-model import path |
| `Search for projects recursively` | off | toggle action `Toggle Search For Maven Project Recursively` |
| `Use Maven output directories` | on | tooltip `IDEA compiler will use output directory defined in maven model (target/classes by default)` |
| `Exclude build directory (%PROJECT_ROOT%/target)` | on | tooltip: `If you select this options folder /target will be excluded from project during reimport. Compiler output (/target/classes and /target/test-classes) will be excluded anyway` |
| `Use M2E compatibility for Maven plugins` | off | tooltip `IntelliJ IDEA will use M2E compatibility information to run Maven goals after syncing the project and before starting the build` |
| `Generated sources folders:` | `Detect automatically` (items `Don't detect`, `target/generated-sources`, `subdirectories of "target/generated-sources"`) | |
| `Phase to be used for folders update:` | `process-resources` (items: `generate-sources`, `process-sources`, `generate-resources`, `process-resources`, `generate-test-sources`, `process-test-sources`, `generate-test-resources`, `process-test-resources`) | note: `IntelliJ IDEA needs to execute one of the listed phases in order to discover all source folders that are configured via Maven plugins.` / `Note that all test-* phases firstly generate and compile production sources.` |
| `Automatically download:` `Sources` (default on), `Documentation` (off), `Annotations` (off) | | |
| `Keep source and test folders on project reload` | on | |
| `Detect compiler automatically` | on | |
| `Dependency types:` | `jar, test-jar, maven-plugin, ejb, ejb-client, jboss-har, jboss-sar, war, ear, bundle` | tooltip `Comma-separated list of dependency types that should be imported` |
| `VM options for importer:` | empty | tooltip `Options specified in this field override the ones in .mvn/jvm.config files` |
| `JDK for importer:` | project JDK (`Use Project JDK`) | errors in spec 05 |
| `Store generated project files under the project root (deprecated)` | off | hint explains deprecation |

### 1.3 Runner page

| Label | Default |
|---|---|
| `Delegate IDE build/run actions to Maven` | off |
| `VM Options:` | empty |
| `JRE:` | `Use Project JDK` (also `Use JAVA_HOME`, `Use Internal JRE`, named JDKs) |
| `Properties:` (name/value table, empty text `No properties defined`) | empty |
| `Skip Tests` | off |
| Environment variables | pass parent env on |
| `Resolve Workspace artifacts` | on in run configurations; tooltip `In case of multi-project workspace, dependencies will be looked for in the workspace first, and only after that in local repository` |

### 1.4 Other pages

`Ignored Files` (title `Ignored Files`, label `Ignored files:`, tooltip `Path patterns (comma-separated, '*' and '?' wildcards allowed):`, empty `No Maven files`), `Running Tests` (caption `Pass to JUnit process following maven-surefire-plugin and maven-failsafe-plugin settings:` with checkboxes `argLine`, `systemPropertyVariables`, `environmentVariables`), `Archetype Catalogs` (LATER).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAVEN-001 | MUST | Pages and controls of 1.1 with defaults and exact texts; the Maven options map to command line flags as listed. | Case BLD-MAVEN-001 case 1..2. |
| BLD-MAVEN-002 | MUST | Maven home resolution: bundled version default `Bundled (Maven 3)`; `Use Maven wrapper` reads `.mvn/wrapper/maven-wrapper.properties` and downloads the distribution if needed (see 5.3); a home that is not a Maven installation shows the invalid label and message. | Cases BLD-MAVEN-002 case 1..3. |
| BLD-MAVEN-003 | MUST | `User settings file` and `Local repository` show the computed value and become editable with `Override`; the overrides are stored in the project. | Case BLD-MAVEN-003 case 1. |
| BLD-MAVEN-004 | MUST | `.mvn/maven.config` precedence: with the option on, flags found in the file (`-o`, `-N`, `-e`, `-U`, `-T`, `-s`, `-P` etc.) override the corresponding UI settings for the project; a parse error shows `Error parsing '.mvn/maven.config' file: {message}` with link `Fix maven config`. | Cases BLD-MAVEN-004 case 1..2. |
| BLD-MAVEN-010 | MUST | Importing page of 1.2 with defaults. | Case BLD-MAVEN-010 case 1. |
| BLD-MAVEN-011 | MUST | Generated sources, dependency types, update-folders phase options change what the next sync produces. | Cases BLD-MAVEN-011 case 1..2. |
| BLD-MAVEN-012 | SHOULD | Auto-download options: after sync, sources (default) and optionally documentation/annotations are fetched in the background; progress texts `Downloading Maven library sources…`, `…documentation…`, `Downloading Maven library sources and documentation…`; cancellation text `Downloading library components has been canceled`; failure `Cannot download sources`, `Sources not found for:` list. | Case BLD-MAVEN-012 case 1. |
| BLD-MAVEN-013 | SHOULD | Runner page of 1.3 with defaults. | Case BLD-MAVEN-013 case 1. |
| BLD-MAVEN-014 | SHOULD | Other pages of 1.4. | Case BLD-MAVEN-014 case 1. |
| BLD-MAVEN-015 | LATER | Archetype catalogs page and `Create from archetype` wizard: (`Add Archetype…`, GroupId/ArtifactId/Version/Repository). Project creation wizards are not part of this area. | n/a |

## 2. Auto-sync files for Maven

Maven registers one tracked project (id = system Maven + project name). Its settings files: the originally added pom files and every pom of the project tree, every module path, and per project directory `.mvn/jvm.config`, `.mvn/maven.config`, `.mvn/wrapper/maven-wrapper.properties`. Additionally the global settings watcher marks the project dirty when the Maven settings.xml, home, user settings or local repository setting changes, and a profile watcher when the set of active profiles changes.

Reload strategy (when the tracker reloads): if the change has unknown modifications, an incremental sync of all Maven projects; otherwise, if every changed settings file belongs to the project tree, only the changed poms are re-read (created/updated: update; deleted: remove); else an incremental sync of all projects. A sync saves all documents first.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAVEN-020 | MUST | Settings file set above; the tracker follows spec 02 rules (default `External changes`). | Cases BLD-MAVEN-020 case 1..3. |
| BLD-MAVEN-021 | MUST | Reload strategy above. | Cases BLD-MAVEN-021 case 1..2. |
| BLD-MAVEN-022 | SHOULD | Settings/profile watchers mark dirty on changes of the general settings and the active profile set. | Case BLD-MAVEN-022 case 1. |

## 3. Delegating build and run to Maven

Off by default (`Delegate IDE build/run actions to Maven`). When on and the module is a Maven module, module builds run Maven: goals are `clean` (if any task is a non-incremental module request), then the phase: `resources:resources` for resources-only builds, `compile` for file-only builds, otherwise the phase from the advanced setting `Delegated build lifecycle phase` (`Experimental. Maven lifecycle phase executed when the IDE build is delegated to Maven.`; default `process-test-classes`; choices: validate, initialize, generate-sources, process-sources, generate-resources, process-resources, compile, process-classes, generate-test-sources, process-test-sources, generate-test-resources, process-test-resources, test-compile, process-test-classes, test, prepare-package, package, pre-integration-test, integration-test, post-integration-test, verify, install, deploy). With several modules of one reactor and no "include dependents" the module list is passed as `--projects groupId:artifactId,…`, the pom is the reactor root's; one run per reactor root, with the explicitly enabled/disabled profiles passed. Documents are saved first. Exit code 0 means success, anything else is an aborted result. Run configurations of Maven modules are run through Maven's environment provider; Java scratch configurations are not.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAVEN-030 | MUST | Acceptance rule: delegation off → the IDE builder; on and module is Maven → Maven (artifact tasks only for Maven-backed artifacts); scratch configurations never. | Cases BLD-MAVEN-030 case 1..2. |
| BLD-MAVEN-031 | MUST | Goal construction rules above. | Cases BLD-MAVEN-031 case 1..4. |
| BLD-MAVEN-032 | SHOULD | `Skip Tests` and `Offline` apply to delegated runs through the same settings as run configurations. | Case BLD-MAVEN-032 case 1. |

## 4. Maven tool window

Tool window `Maven` (right). View state defaults: `Group Modules` off (persisted state default; open question 1), `Show Ignored Projects` on, `Show Basic Phases Only` on, `Always Show ArtifactId` off, `Show project version next to name` off.

Tree: root `Maven projects` containing project nodes (`artifactId`, or project name unless `Always Show ArtifactId`; version appended when `Show Versions`); under a project node: `Lifecycle`, `Plugins`, `Profiles` (root level), `Dependencies`, `Run Configurations`; the bundle also defines the node names `Repositories` and `Modules` (open question 2). When no project is linked: `There are no Maven projects to display.` / `You may use {add action} to add a project or {reload action} to let the plugin find all pom.xml files under the project roots and reload them`.

Lifecycle node phases: with `Show Basic Phases Only` on: `clean`, `validate`, `compile`, `test`, `package`, `verify`, `install`, `deploy`, `site` (this is the order of the model constant); off: the full list `clean validate generate-sources process-sources generate-resources process-resources compile process-classes generate-test-sources process-test-sources generate-test-resources process-test-resources test-compile process-test-classes test prepare-package package pre-integration-test integration-test post-integration-test verify install site deploy`. Profiles node: one checkbox per profile (states: active, disabled, default); toggling uses `Activate`, `Deactivate`, `Use Default`; `Reset Profiles` returns to the defaults. Plugins node: plugins with their goals; Dependencies node: resolved dependencies with scope.

Toolbar (group `Maven.NavigatorActionsToolbar`, in order): `Sync All Maven Projects` split button (`Reload All Maven Projects` in its popup `Sync/Reload All Maven Projects`), `Generate Sources and Update Folders For All Projects`, Download popup (`Download Sources and/or Documentation`: `Download Sources`, `Download Documentation`, `Download Sources and Documentation`), separator, `Add Maven Projects` (`Add and reload Maven projects to the projects tree`), `Unlink Maven Projects`, separator, `Run Maven Build`, `Execute Maven Goal`, `Toggle Offline Mode`, `Toggle 'Skip Tests' Mode`, separator, `Analyze Maven Dependencies…`, separator, `Expand All`, `Collapse All`, separator, `Build Tools Settings` popup (`Maven Settings`, `Auto-Sync Settings…`).

Context menus: project: `Sync Project`, `Generate Sources and Update Folders`, `Ignore Projects`, `Unlink Maven Projects`, `Open settings.xml`, `Open toolchains.xml`, `Show Effective POM`, `Run Maven Build`, `Create Run Configuration`, `Jump to Source`, `Download Sources and Documentation` submenus; `Repository` submenu: `Reindex repository`; module menu; dependency menu (`Download …` selected, `Analyze Maven Dependencies…`); goal menu: `Run Maven Build`, `Execute Before Build`, `Execute After Build`, `Execute Before Rebuild`, `Execute After Rebuild`, `Execute Before Run/Debug…`, `Assign Shortcut…`; run configuration menu: `Edit Run Configuration…`, `Remove Run Configuration` (confirmation `Delete "{name}"?`).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAVEN-040 | MUST | Offline toggle: `Toggle Offline Mode` (description `Toggle offline mode for Maven builds`) flips the `Work offline` setting; on switching off from another place a notification `Maven Offline Mode was switched off` may appear. `Toggle 'Skip Tests' Mode` flips Runner `Skip Tests` (adds `-DskipTests=true`). | Cases BLD-MAVEN-040 case 1..2. |
| BLD-MAVEN-041 | MUST | Tree structure and nodes as above, with phases lists by the basic flag. | Cases BLD-MAVEN-041 case 1..3. |
| BLD-MAVEN-042 | MUST | Toolbar entries and order as above with exact texts. | Case BLD-MAVEN-042 case 1. |
| BLD-MAVEN-043 | MUST | `Run Maven Build` (mnemonic R, description `Execute selected phases or goals`) runs the selected phases/goals (several selections run in order) in the Run tool window; `Execute Maven Goal` opens a goal input with completion (title `Execute Maven Goal`; popup of goals `Maven Goals`). | Cases BLD-MAVEN-043 case 1..2. |
| BLD-MAVEN-044 | MUST | Sync actions: `Sync All Maven Projects` (incremental), `Reload All Maven Projects` (full re-read), `Sync Project` (selected, description `Sync selected Maven projects`); `Generate Sources and Update Folders` runs the phase from the importing page. | Cases BLD-MAVEN-044 case 1..2. |
| BLD-MAVEN-045 | MUST | Linking: `Add Maven Projects` (file chooser; invalid selections: warning title `Unrecognized Maven Project` with `Selected file '{f}' isn't Maven file` / `Selected directory '{d}' doesn't have Maven files`), `Add as Maven Project` in the project view popup, `Unlink Maven Projects` (`Unlink selected Maven projects and their subprojects from the project tree`; only root projects can be removed: `Failed to remove project` / `You can not remove {x} because it's imported as a module of another project {y}. You can use Ignore action. Only root project can be removed.`). | Cases BLD-MAVEN-045 case 1..3. |
| BLD-MAVEN-046 | SHOULD | Ignore: `Ignore Projects` (title text `Ignore Projects`/`Unignore Projects`) and `Edit Ignored Files…`; ignored projects show only with `Show Ignored Projects`. | Case BLD-MAVEN-046 case 1. |
| BLD-MAVEN-047 | SHOULD | Profiles menu: toggle, reset, states and the text of the toggle action `Toggle Profile` (`Activate/deactivate profile`). A profile change marks the project dirty. | Case BLD-MAVEN-047 case 1. |
| BLD-MAVEN-048 | SHOULD | Download actions (all/selected: sources, documentation, both) with progress and results of BLD-MAVEN-012. | Case BLD-MAVEN-048 case 1. |
| BLD-MAVEN-049 | SHOULD | `Show Effective POM` generates and shows the effective POM in an editor; failure: `Failed to evaluate effective pom.` titled `Error`. `Open settings.xml`, `Open toolchains.xml` (offer `Create '{file}'` if absent). `Reindex repository` (`Reindex local repository to search for classes`). | Case BLD-MAVEN-049 case 1. |

## 5. Sync

Maven sync steps shown in the Sync tab (title `Sync`, per-project `Sync {name}`): `Scanning Maven projects…`, `Locating Maven project files…`, `Reading Maven projects…` / `Reading {pom}…`, `Resolving Maven dependencies…` / `Resolving dependencies of {pom}…`, `Generating Maven sources…`, `Downloading Maven plugins…` / `Downloading plugins for {pom}…`, `Configuring Maven projects`, `Updating Maven project models…`, `Importing Maven projects`, `Starting Maven server` (progress), plus nodes `Resolve plugins`, `Resolve dependencies`, `Downloading plugins`, `Downloading dependencies` with `Downloading {artifact}` children, and groups `Compiler`, `Error`, `Warning`. Wrapper steps: `Running maven wrapper`, `Downloading Maven wrapper from {url}`, `Downloading Maven wrapper with Basic authentication`, `Downloading Maven wrapper {done}/{percent}%`, `Unpacking archive…`, `Maven wrapper unpacked into {dir}`.

Maven project problems (shown as errors of a pom): `Recursive inheritance found`, `Self-inheritance found`, `Parent '{x}' has problems`, `Parent '{x}' not found`, `Module '{x}' not found`, `Unresolved dependency: '{x}'`, `Unresolved build extension: '{x}'`, `Unresolved plugin: '{x}'`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAVEN-050 | MUST | Sync tab content and progress texts above, in the order of the pipeline: scan → read → resolve plugins → resolve dependencies → generate sources/update folders → import → post-process. | Case BLD-MAVEN-050 case 1. |
| BLD-MAVEN-051 | MUST | Project problem messages above, attached to the pom with file/line. | Case BLD-MAVEN-051 case 1. |
| BLD-MAVEN-052 | MUST | Version and JDK issues: `No valid Maven installation found. Either set the home directory in the configuration dialog or set the M2_HOME environment variable on your system.`; `Specified Maven home directory ({d}) does not exist`; `{d} is not a valid Maven home directory` (each with a `Configure Maven home` link variant); `Maven 3.0 is not supported anymore, use maven 3.1+`; `Maven version issue` / `Update Maven version to {v}` / `Use Bundled {x}` / `Open Settings`; `Maven 3.3.1+ requires JDK 1.7+. Please set appropriate JDK`; `Maven version {v} requires at least JDK version 1.7`; Maven 4 with old JDK: `Apache Maven 4.X Requires Java 17 or Newer to Run.` with fix `Choose another JDK for Importer and Runner…`; incompatible importer JDK title `Maven JDK Version for Importer Is Too Low` text `For Maven {v} at least JDK {n} is required.` with links `Choose another JDK for Importer and Runner…`, `Open Importer Settings…`. | Cases BLD-MAVEN-052 case 1..4. |
| BLD-MAVEN-053 | MUST | Repository issues: `Since Maven 3.8.1 http repositories are blocked` with the possible solutions list (`Check that Maven settings.xml does not contain http repositories`, `Check that Maven pom files do not contain http repository {r}`, `Add a mirror(s) for {r} that allows http url in the Maven settings.xml`, `Downgrade Maven to version 3.8.1 or earlier in settings`); stale artifact fix `Try to run Maven import with -U flag (force update snapshots)`; core extension failure `Maven Core Components Cannot Be Resolved`. | Case BLD-MAVEN-053 case 1. |
| BLD-MAVEN-054 | SHOULD | Wrapper issues: `Cannot install wrapped maven, set Bundled Maven` + `The Maven wrapper was disabled. The Maven bundled version will be used instead.`; notification group `Maven wrapper` messages (`DistributionUrl is empty. The Maven bundled version will be used instead.`, `File .mvn/wrapper/maven-wrapper.properties not found. The Maven bundled version will be used instead.`, `Start downloading. Please wait.`, `Successfully installed.`, `Cannot download ZIP distribution from {url}. Please check distributionUrl`, `Zip archive at {x} is not correct Maven distribution. Please check distributionUrl`, `HTTP used to download maven distribution` / `HTTP used to download maven distribution. Consider changing distributionURL to HTTPS`). | Case BLD-MAVEN-054 case 1. |
| BLD-MAVEN-055 | SHOULD | Language level / compiler issues: `Language level is invalid or missing in pom.xml. Current project JDK is {j}. Specify language level in {pom}`; `Module {m} SDK {j} is not compatible with the source version {v}. Upgrade Module SDK in project settings to {v} or higher.`; `Module {m} SDK {j} does not support source version {v}.` with solutions; `Maven compiler annotation processor problem`; toolchains: `Toolchain {x} Is Absent` / `Toolchain with requirements {r} not found` + `Add SDK to toolchains.xml…`, `JDK Added to IntelliJ IDEA`, `Cannot import JDK`. | Case BLD-MAVEN-055 case 1. |
| BLD-MAVEN-056 | SHOULD | Untrusted project: sync title `Untrusted project`, text `The opened untrusted project is not fully loaded.` / `IntelliJ IDEA will not resolve any plugins in the project, download artifacts, or execute any goals`, action `Trust project`; running a goal: `Project is untrusted, the Maven goal will not be executed`. | Case BLD-MAVEN-056 case 1. |
| BLD-MAVEN-057 | SHOULD | WSL specific messages (WSL JDK vs Windows JDK, Maven home outside the distribution): LATER on non-WSL platforms. | Case BLD-MAVEN-057 case 1. |

## 6. Run configuration

Type id `MavenRunConfiguration`, name `Maven`, description `Maven build`, tab sections `Maven` / `Java` with `Inherit from settings` links.

| Field | Default |
|---|---|
| `Working directory` (project/pom directory; chooser `Select Maven Working Directory`) | the Maven project directory |
| `Command line:` (hint `Example: compile --debug. Use {shortcut} for autocompletion.`; table `Maven Command` / `Maven Option` / `Description`; empty `No Maven Commands`/`No Maven Options`; title `Maven Commands`; tooltip `Insert Maven Commands…`) | empty |
| `Maven home` (distribution) | inherited |
| `Profiles:` (hint `Separate with spaces. Use "-" prefix to disable a profile, for example, -test`) | empty |
| `User settings`, `Local repository`, `Thread count`, `Multi Module Directory`, `Checksum policy`, `Output level`, `Multi-project build fail policy` | inherited |
| `Properties:` (dialog `Properties`, tooltip `Edit Maven Properties…`, empty `No properties`, button `Save`) | empty |
| `VM options`, `JRE:` (hint `Specify JRE to run Maven goals`), env vars, `Resolve Workspace artifacts`, `Skip Tests` | inherited from Runner page |
| `Emulate terminal` | off |

Maven options offered by completion with descriptions: `-am`, `-amd`, `-B`, `-b`, `-C`, `-c`, `-cpu`, `-D`, `-e`, `-emp`, `-ep`, `-f`, `-fae`, `-ff`, `-fn`, `-gs`, `-gt`, `-h`, `-l`, `-llr`, `-N`, `-npr`, `-npu`, `-nsu`, `-o`, `-P`, `-pl`, `-q`, `-rf`, `-s`, `-t`, `-T`, `-U`, `-up`, `-v`, `-V`, `-X`, with the description texts of the bundle (for example `-N`: `Do not recurse into sub-projects`, `-o`: `Work offline`, `-T`: `Thread count, for instance 2.0C where C is core multiplied`).

Execution messages: `Maven execution finished`, `Maven execution aborted`, `Maven execution terminated abnormally`, `BUILD FAILED`, `Running {x}`; invalid JRE: `Cannot find JRE '{x}'` (with `You can specify JRE to run maven goals in Settings | Maven | Runner`), `JAVA_HOME environment variable not defined`, title `Cannot use specified JRE`; `Failed to execute Maven goal`; `Project JDK is not specified. Configure`; `Resume Build From "{module}"` (title `Resume Build From Specified Module`) action in the output of a failed reactor build.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAVEN-060 | MUST | Run configuration fields of the table, inheritance from settings and overrides (a field edited in the configuration overrides the inherited one for that configuration only). | Cases BLD-MAVEN-060 case 1..3. |
| BLD-MAVEN-061 | MUST | Command line assembly: goals/phases, then options from settings (offline `-o`, `-N`, `-e`, `-U`, `-T`, output level `-X`/`-q`…), profiles as `-P a,b,-c`, properties as `-Dk=v`, skip tests as `-DskipTests=true`. | Cases BLD-MAVEN-061 case 1..3. |
| BLD-MAVEN-062 | MUST | Execution messages and errors above. | Case BLD-MAVEN-062 case 1. |
| BLD-MAVEN-063 | SHOULD | Before-launch tasks (`Run Maven Goal`, `Run Maven Goal '{goal}'`, select dialog `Select Maven Goal`, edit `Edit Maven Goal`, error `Working directory is empty`) and goal activation phases (`Before Build`, `After Build`, `Before Rebuild`, `After Rebuild`, `Before Run`) from spec 02 section 7, counter `{goal} ({i} of {n})`. | Case BLD-MAVEN-063 case 1. |
| BLD-MAVEN-064 | SHOULD | Run Anything provider for `mvn ...`. | Case BLD-MAVEN-064 case 1. |

## 7. Remote repositories

Page `Remote Jar Repositories` (`com.intellij.jarRepository.settings.RemoteRepositoriesConfigurable`, group Build Tools, project scope): list of repositories (default Maven Central and the JetBrains repositories), add/edit/remove, `Unindexed Maven repositories found` notification group.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-MAVEN-070 | SHOULD | The page lists repositories with id and URL; add/edit/remove; used for library downloads. | Case BLD-MAVEN-070 case 1. |

## 8. Inventory coverage

All 55 Maven actions of the inventory are covered: sync (`SyncIncrementally`, `Reimport`, `ReimportProject`, `SyncAllSplitAction`: BLD-MAVEN-044), folders (`UpdateFolders`, `UpdateFoldersForProject`: 044), downloads (6 actions: 048), linking (`AddManagedFiles`, `AddFileAsMavenProject`, `RemoveManagedFiles`: 045), `IgnoreProjects` (046), profiles (`ToggleProfile`, `ResetProfiles`: 047), `RunBuild`, `ExecuteGoal` (043), `ToggleOffline`, `ToggleSkipTests` (040), `ToggleLookNested` (010), tree toggles (`GroupProjects`, `ShowIgnored`, `ShowBasicPhasesOnly`, `AlwaysShowArtifactId`, `ShowVersions`, `ExpandAll`, `CollapseAll`: 041), `OpenSettingsXml`, `OpenToolchainsXml`, `ShowEffectivePom`, `UpdateLocalIndex` (049), `ShowSettings` (001), task activation (`BeforeCompile`, `AfterCompile`, `BeforeRebuild`, `AfterRebuild`, `BeforeRun`, `AssignShortcut`: 063), `EditRunConfiguration`, `RemoveRunConfiguration` (060), dependency analyzer (`ViewDependencyAnalyzer`, `NavigatorDependencyAnalyzer`, `ProjectViewDependencyAnalyzer`, `DependencyAnalyzer.GoTo`, `DependencyAnalyzer.OpenConfig`: spec 02 section 11, BLD-SYNC-090). Internal diagnostics `Maven.PrintStatistics`, `Maven.ResetReadStatistic`, `Maven.Killall`, `Maven.ShowConnectors`, `Maven.ReReadAll`: LATER (internal-mode developer tools). Refactorings `refactoring.extract.dependency` (`Extract Managed Dependency`) and `refactoring.introduce.property` (`Property`): LATER (POM editing refactorings, language side). Settings page `MavenSettings`: BLD-MAVEN-001. Run configuration `MavenRunConfigurationType`: BLD-MAVEN-060.

## Open questions

1. `Group Modules` default: the persisted state of the navigator has the structural grouping flag false by default while the toolbar offers `Group Modules`; observe a fresh IDE before fixing the default.
2. Whether the tree contains `Repositories` and `Modules` nodes in the current navigator (bundle keys exist: `Lifecycle`, `Profiles`, `Plugins`, `Repositories`, `Run Configurations`, `Dependencies`, `Modules`).
3. Exact list of goals shown under `Plugins` (resolved from the plugin descriptors; needs the Maven model).
4. Product behaviour of `Enable fast import` default.

# Build spec 02: External-system sync (import/reload), auto-sync, tool window framework

Status: specification (clean-room). ID prefix `BLD-SYNC`. Executable cases: `cases/02-external-system-sync.cases.md`.

"External system" is the generic framework under Gradle and Maven: linked projects, importing their model into the IDE project model, a common tool window, task activation, run configurations. This spec covers the framework; Gradle and Maven specifics are in specs 03 and 04. The imported data (modules, content roots, libraries, SDKs) land in the project model of `docs/specs/platform/07-project-structure-and-scopes.md`.

ic: `platform/external-system-impl/src/com/intellij/openapi/externalSystem/autoimport/` (`AutoImportProjectTracker`, `AutoImportProjectStatus`, `AutoImportProjectSettingsFilesTracker`, `AutoImportProjectTrackerSettings`, `AutoImportProjectNotificationAware`, `ProjectRefreshAction`, `HideProjectRefreshAction`, `ProjectRefreshFloatingProvider`, `changes/`), `.../autolink/UnlinkedProjectNotificationAware.kt`, `.../util/ExternalSystemUtil.java`, `.../service/project/trusted/ExternalSystemTrustedProjectDialog.kt`, `.../view/` (`ExternalProjectsViewImpl`, `TasksNode`, `ProjectNode`), `.../service/task/ui/AbstractExternalSystemToolWindowFactory.java`, `.../service/project/manage/ExternalSystemTaskActivator.java`, `platform/external-system-impl/resources/META-INF/ExternalSystemActions.xml`, `platform/external-system-api/resources/messages/ExternalSystemBundle.properties`, `platform/external-system-api/src/com/intellij/openapi/externalSystem/view/ExternalProjectsViewState.java`, `platform/external-system-api/src/com/intellij/openapi/externalSystem/autoimport/ExternalSystemModificationType.kt`, `platform/platform-api/resources/messages/IdeBundle.properties` (trust dialog texts), `plugins/gradle/src/org/jetbrains/plugins/gradle/service/project/GradleAutoImportAware.java`, `plugins/maven/src/main/java/org/jetbrains/idea/maven/project/auto/reload/`.

## 1. Auto-sync setting

Page `Settings | Build, Execution, Deployment | Build Tools` (display name `Build Tools`; id `build.tools`). It contains the group `Auto-Sync Projects` (the string key says "hit"; used as the section title) with a radio group titled `Sync project after changes in the build scripts:` and two options:

| Radio | Meaning | Default |
|---|---|---|
| `Any changes` | reload automatically after any change of a build script, whether made in the IDE editors or outside | no |
| `External changes` | reload automatically only after changes made outside the IDE or from version control (the comment under the radio reads `Changes outside the IDE or from version control.`); changes made in the editor only show the Load button | **yes** |

A third internal value (never) exists in the model but is not offered in the UI. The value is stored per project in the workspace file; a registered default provider may override the initial value, otherwise `External changes`. The page also hosts the `Auto-Sync Settings…` action (id `ExternalSystem.ShowCommonSettings`) that opens it, found in the tool windows' gear menu.

## 2. Change tracking

Each linked project is registered with a tracker as a "project aware": id (system, path), the set of its settings files (build scripts and other files whose change makes the model stale), and callbacks to reload. A settings file is any file the build tool says influences the model (see 3 and 4 for Gradle and Maven lists). Change detection compares a content checksum (CRC) of each settings file against the checksum at the last successful sync, so editing then reverting a file leaves the project up to date.

Per project the state is one of: synchronized, modified (with modification type), dirty (with type), reverted, broken. Modification types:

| Type | Source |
|---|---|
| EXTERNAL | file-system change found by refresh (not from a save by the IDE) |
| INTERNAL | change in an editor document, or a VFS event caused by the IDE |
| HIDDEN | like INTERNAL but never reloaded automatically even with `Any changes` |
| UNKNOWN | no information |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-SYNC-001 | MUST | The Build Tools page, the group, radios and texts of section 1 exist; the default is `External changes`. | Case BLD-SYNC-001 case 1. |
| BLD-SYNC-002 | MUST | Settings-file change detection by checksum: a project is stale iff the checksum of any settings file differs from the checksum recorded at the last sync; created and deleted files count as changes. A state restored from the previous session (checksums saved with the project cache) marks the project dirty if the previous session ended dirty or the project model cache is missing/invalid. | Cases BLD-SYNC-002 case 1..3. |
| BLD-SYNC-003 | MUST | Modification type: a change event from an editor document is INTERNAL; a VFS change from a file refresh is EXTERNAL; a VFS change caused by the IDE saving is ignored (not a change) when the project is configured to ignore its own saves; a build tool may adjust the type of a path. A bulk change (a batch of file events, e.g. a VCS update) is processed as one unit: the reload decision waits for the batch to end. | Cases BLD-SYNC-003 case 1..3. |
| BLD-SYNC-004 | MUST | Decision when a project is stale, with `Any changes`: EXTERNAL and INTERNAL changes reload automatically; HIDDEN and UNKNOWN only show the Load button. With `External changes`: EXTERNAL reloads automatically; INTERNAL, HIDDEN, UNKNOWN only show the Load button. | Cases BLD-SYNC-004 case 1..4. |
| BLD-SYNC-005 | MUST | Automatic reload is additionally suppressed (Load button shown instead) while: a completion lookup popup is open; the global auto-sync switch is off (registry `external.system.auto.import.disabled`); a reload of the same project is already running; a project change batch is running; the project tracker is not yet activated (project still opening); the build tool vetoes it; the project is untrusted. | Cases BLD-SYNC-005 case 1..3. |
| BLD-SYNC-006 | MUST | Timing: changes are merged in a 300 ms window (every new change restarts the window); an automatic reload starts after a delay of 3 s from the last change (a build tool may ask for another delay). An explicit user reload starts immediately. | Case BLD-SYNC-006 case 1. |
| BLD-SYNC-007 | MUST | An up-to-date project removes the Load button; a project whose changes are disabled for reload removes the button too (no button for something that cannot be loaded). | Case BLD-SYNC-007 case 1. |
| BLD-SYNC-008 | MUST | A sync that fails marks the project broken: it stays stale, so the Load button stays until a sync succeeds. | Case BLD-SYNC-008 case 1. |
| BLD-SYNC-009 | MUST | When the reload finishes the checksums are re-taken (after refreshing settings files on disk) so that changes made by the sync itself (for example a wrapper update) are not counted; the tracker may ignore specific file events during a reload. | Case BLD-SYNC-009 case 1. |
| BLD-SYNC-010 | SHOULD | After the IDE settings (auto-sync radio) change, the decision is re-evaluated at once. | Case BLD-SYNC-010 case 1. |

## 3. The Load Changes floating button and actions

When at least one project has a pending notification, a floating toolbar (group `ExternalSystem.ProjectRefreshActionGroup`) is shown in the top-right area of every main editor (not diffs or consoles). It disappears when no project has a pending notification. It never auto-hides.

| Element | Text | Description / notes |
|---|---|---|
| Load action `ExternalSystem.ProjectRefreshAction` | `Sync {systems} Changes` where `{systems}` is the natural join of the build systems with pending changes (`Sync Gradle Changes`, `Sync Gradle and Maven Changes`); template text with no pending changes: `Sync Changes` | description: `{systems} project structure has been changed. Sync changes with {product} to make it work correctly.`; template: `Sync build system changes with {product}.`; icon "load changes" (a system-specific reload icon when exactly one system is pending) |
| Hide action `ExternalSystem.HideProjectRefreshAction` | `Hide This Notification` | close icon with hover variant; hides the button without syncing; enabled only while a notification is visible |
| Shortcuts of Load | Windows/Linux: Ctrl+Shift+O (not in the Eclipse, NetBeans, Visual Studio keymaps); macOS keymap: Cmd+Shift+O; macOS 10.5+ keymap: Cmd+Shift+I | |

Note: older IDE versions word the same action `Load Gradle Changes` / `Load Maven Changes`; the current bundle wording is `Sync … Changes`. Archipel implements the current wording; this is an open question for the product owner (see 8).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-SYNC-020 | MUST | Floating button visible iff a notification is pending; text, description and icon rules of the table. | Cases BLD-SYNC-020 case 1..3. |
| BLD-SYNC-021 | MUST | Invoking Load first asks to trust an untrusted project (see 6), then schedules an immediate explicit reload of all pending projects. | Case BLD-SYNC-021 case 1. |
| BLD-SYNC-022 | MUST | `Hide This Notification` removes all pending notifications without syncing; a later change shows the button again. | Case BLD-SYNC-022 case 1. |
| BLD-SYNC-023 | MUST | Shortcuts of the table. | Case BLD-SYNC-023 case 1. |

## 4. Sync execution

A sync of one linked project (also called refresh, import or reload):

1. All unsaved documents are saved. Earlier sync notifications of this build system are cleared.
2. The project is trusted? If not, the sync runs in preview mode (see 6).
3. If another sync of the same project is running, this one fails at once with the message `Another 'Sync project' task is currently running for the project:  {path}` (two spaces before the path, as in the source).
4. The "before sync" tasks run (see 7); a failing task cancels the sync.
5. A progress with title `Importing '{project}' {system} Project` (background mode) or `Building '{project}' {system} Project Info` (modal mode, used when opening/importing a new project) runs; the Build tool window tab `Sync` (execution name `{project} Sync`, e.g. `app Sync`) is created.
6. The tool resolves the model. On success the data is imported into the project model; optionally the user selects which data to import (see 5.4); the "after sync" tasks run.
7. On failure: partial data, if the tool produced some, is still imported; cancellation stops silently; otherwise the failure is shown (see 4.2).

Sync modes: `background` (default for reloads: the IDE stays usable; indexing waits), `modal` (new project creation/import), no-progress modes are not allowed for sync.

### 4.1 Sync tab content

Tab `Sync` in the Build tool window. Root node title `{project} Sync` (for Gradle `{project name}: sync`-style naming is tool-specific, see 03); status words: `syncing…` while running, then `successful`/`finished`, `failed`, `cancelled`. Children: tool-specific steps (Gradle: `Configure build`, `Build model`, …; Maven: `Resolve plugins`, `Resolve dependencies`, `Downloading …`), then messages grouped into `Error`/`Warning`. Build console text uses the standard tree + console split of BLD-MAKE-030..034.

### 4.2 Failures

| Situation | User-visible result |
|---|---|
| Resolve fails with a message | Failure node in the tree titled `{system} '{project}' project sync failed` carrying the message; a notification (group of the build system) with the same title; the first error is shown in the editor if it has a location |
| Message empty | the description `Can't resolve {system} project at '{path}'. Reason: {reason}` |
| Detail text | the failure text ends with `Consult IDE log for more details (Help | Show Log)` |
| Script file missing | `{system} script file '{path}' not found` |
| Project already linked | `The project is already registered` |
| Cancellation fails | `Cancellation failed` |
| Sync finished event never produced | the tab ends with status `cancelled` |
| Dialog title for modal error | `Project Sync Failed` |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-SYNC-030 | MUST | Steps 1-7 of the sync algorithm in the given order, with the exact progress titles. | Cases BLD-SYNC-030 case 1..3. |
| BLD-SYNC-031 | MUST | Sync tab content and status words of 4.1; a running sync is cancellable from the progress and the tab. | Cases BLD-SYNC-031 case 1..2. |
| BLD-SYNC-032 | MUST | Failure behaviour of 4.2 with exact texts. | Cases BLD-SYNC-032 case 1..4. |
| BLD-SYNC-033 | MUST | Quick fixes in sync failures are links in the failure text; a link action with the id of the fix runs it (Gradle and Maven define theirs: BLD-GRADLE-060..065, BLD-MAVEN-050..054). A `Reload` fix re-syncs the project. | Case BLD-SYNC-033 case 1. |
| BLD-SYNC-034 | MUST | The first sync of a project (previous sync type "preview") is an import; later syncs are re-imports; untrusted projects only do previews. Sync types are persisted per project path. | Case BLD-SYNC-034 case 1. |
| BLD-SYNC-035 | SHOULD | Sync progress is exposed as tool window events: `Sync` tab name from the build tool. Statuses `finished`, `failed`, `cancelled`, `running…`; messages `{0} finished`, `{0} failed`, `{0} cancelled`. | Case BLD-SYNC-035 case 1. |
| BLD-SYNC-036 | SHOULD | During a sync the project model is marked as having incomplete dependencies (so analysis does not report false unresolved errors for libraries not yet imported). | Case BLD-SYNC-036 case 1. |

## 5. Managing linked projects

| Action id | Text | Where |
|---|---|---|
| `ExternalSystem.RefreshAllProjects` | `Sync All {system} Projects` (desc `Sync all linked {system} projects`) | tool window toolbar |
| `ExternalSystem.RefreshProject` | `Sync {system} Project` (desc `Sync selected linked {system} project`) | project/module context menu |
| `ExternalSystem.DetachProject` | `Unlink {system} Project` (desc `Unlink selected project`); shortcut as Delete | tool window toolbar/context menu |
| `ExternalSystem.IgnoreProject` | `Ignore {system} Project(s)` / `Unignore {system} Project(s)` (desc `Ignore selected {system} project(s)`) | context menu |
| `ExternalSystem.OpenConfig` | `Open {system} Config` (desc `Opens the {system} project file in the editor`); shortcut as Edit Source | context menu |
| `ExternalSystem.SelectProjectDataToImport` | `Select Project Data To Import` | toolbar |
| `ExternalSystem.ShowSettings` | `{system} Settings…` (desc `Edit {system} settings for the current project`) | gear |
| `ExternalSystem.ShowCommonSettings` | `Auto-Sync Settings…` (desc `Edit build tools settings for the current project`) | gear |
| `ExternalSystem.AttachProject` | (set in code; text `Link {system} Project`) | toolbar |
| Recovery action (`ExternalSystem.RecoveryAction`) | `Invalidate Build System IDE caches and Re-sync` | search everywhere / tool window |

Unlink confirmation: `{system} Module(s) {names} will be unlinked.` / `No files will be deleted.` (choice: `Module` for 1, `Modules` for 2+). Orphan modules after a sync (modules that existed before and are no longer in the build): notification group `Modules no longer referenced by external system`, text `The IDE modules below were removed by the {system} project sync:` + list + `You can open a dialog to select the ones you need to restore.` The dialog is titled `Orphan Modules` with text `The modules below were removed from the project.` / `Select the ones you need to restore.`; a setting `Remove orphan modules after project sync`.

Select-data dialog titled `{system} Project Data To Import` with text `Please select the modules/data to include in the project.`, a tree with checkboxes, buttons `Select All`, `Unselect All`, and dependency helpers: `Show Selected Only`, `Select Required`; when disabling a module that others depend on: dialog `Disable Modules with Dependency on This` / `The following module(s) {names} is/are enabled and depend(s) on selected modules. Would you like to disable it/them too?`; the symmetric `Enable Dependant Modules`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-SYNC-040 | MUST | The actions of the table with texts, descriptions and placements; text parameters use the build system's readable name (Gradle, Maven). | Case BLD-SYNC-040 case 1. |
| BLD-SYNC-041 | MUST | Unlinking a project removes its modules from the IDE model and the project from the tool window; files are untouched; confirmation text of above. | Case BLD-SYNC-041 case 1. |
| BLD-SYNC-042 | MUST | Ignored projects are hidden from the tool window when `Show Ignored` is off and are not synced; `Unignore` reverts. | Case BLD-SYNC-042 case 1. |
| BLD-SYNC-043 | SHOULD | Orphan modules handling: after a sync, modules of the previous import missing from the new data are removed and the notification with the restore dialog is shown. | Case BLD-SYNC-043 case 1. |
| BLD-SYNC-044 | SHOULD | `Select Project Data To Import` dialog behaviours incl. dependent-module prompts; only shown when more than one data node exists. | Case BLD-SYNC-044 case 1. |
| BLD-SYNC-045 | SHOULD | `Invalidate Build System IDE caches and Re-sync` clears cached external project data and runs a full sync. | Case BLD-SYNC-045 case 1. |
| BLD-SYNC-046 | MUST | Duplicate content roots found while importing produce a notification (group `Duplicate content roots detected`) titled `Duplicate content roots detected` with text `Path [{path}] of module [{module}] was removed from modules [{others}]` and `Also {n} more path(s) was/were deduplicated. See idea log for details` when more than listed. | Case BLD-SYNC-046 case 1. |

## 6. Trust

An untrusted project (opened in safe mode) never executes build tool code. Sync of an untrusted project is a preview: it reads nothing that executes scripts. Loading or linking asks the user: dialog title `Trust {system} Project?` (plural `Trust Gradle and Maven Projects?` for several systems), buttons `Trust Project` and `Stay in Safe Mode`; for linking: title `Trust and Link {system} Project '{name}'?` with cancel button `Don't Link`; folder variant button `Trust '{folder}' Folder`. Running a task in an untrusted project fails with `{system} project is untrusted, so its task cannot be executed.`

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-SYNC-050 | MUST | Preview sync and refused task execution as above; the Load action asks for trust first and does nothing on refusal. | Cases BLD-SYNC-050 case 1..2. |
| BLD-SYNC-051 | MUST | Dialog titles and buttons as above. | Case BLD-SYNC-051 case 1. |

## 7. Task activation (before/after sync, build, rebuild, run)

Tasks of a build tool can be bound to phases in the tool window: right-click a task, submenu entries (group `ExternalSystemView.TaskActivationGroup`): `Execute Before Sync`, `Execute After Sync`, `Execute Before Build`, `Execute After Build`, `Execute Before Rebuild`, `Execute After Rebuild`, and (as an action outside this group) `Execute Before Run/Debug…`. Each is a checkbox toggle. The phase names in the activation dialog (`Tasks Activation`, desc `Edit {system} tasks activation settings`) are `Before Sync`, `After Sync`, `Before Build`, `After Build`, `Before Rebuild`, `After Rebuild`, `Before Run`; the dialog is opened from the project/module context menu. Activation is stored per linked project in the project settings. Before-run tasks appear in run configurations' Before launch list as `Run {system} task` / `Run {system} task '{name}'`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-SYNC-060 | MUST | Toggling stores the task under the phase for the linked project; a task may be bound to several phases; run configurations may also be bound. | Case BLD-SYNC-060 case 1. |
| BLD-SYNC-061 | MUST | Before-sync tasks run before the model resolution; if one fails the sync does not start. After-sync tasks run only after a successful import and not in preview mode. | Case BLD-SYNC-061 case 1..2. |
| BLD-SYNC-062 | SHOULD | Before/after build and rebuild tasks run around the corresponding IDE build request when the project builds through the IDE builder as well as through a build tool (see BLD-MAKE-053). | Case BLD-SYNC-062 case 1. |
| BLD-SYNC-063 | SHOULD | The Tasks Activation dialog lists phases with their tasks (add/remove/move). | Case BLD-SYNC-063 case 1. |

## 8. External system tool window (shared by Gradle and Maven-like views)

Tool window id and title are the system's (`Gradle`; Maven's tool window is its navigator, see 04), anchored right, available only when at least one project is linked (`shouldBeAvailable`). While loading its content shows `Initializing {system} projects data…`. Its view-state defaults (persisted in the project): `Group Modules` on, `Group Tasks` on, `Display Tasks Inherited from Sub-Projects` on, `Show Ignored` on (these four toggles live in the gear menu).

Tree: project node (tooltip `Project: {name}` + `Location: {path}` + description), module nodes (grouped by qualified-name prefix when `Group Modules` is on; a single top-level module collapses into the project node), under each: `Tasks` node (hidden if empty), `Run Configurations` node, further nodes contributed by the system (dependencies, plugins ...). Tasks are grouped by the group name lower-cased; tasks with no group fall into group `other`, always listed last; with `Group Tasks` off the tasks are listed flat. Dependency reference nodes show tooltip `Dependencies omitted (listed previously).` / `Press Enter or left mouse button double click to navigate to dependencies.` Toolbar: sync, select data, unlink, run, other, expand all, collapse all, settings group.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-SYNC-070 | MUST | Availability and loading label above. | Case BLD-SYNC-070 case 1. |
| BLD-SYNC-071 | MUST | View state defaults and persistence of the four toggles; toggling rebuilds the affected nodes. | Cases BLD-SYNC-071 case 1..2. |
| BLD-SYNC-072 | MUST | Tasks grouping and ordering rules above. | Cases BLD-SYNC-072 case 1..2. |
| BLD-SYNC-073 | MUST | Context menus: project node (`Open … Config`, `Sync … Project`, `Unlink`, `Ignore`, `Tasks Activation`, tool extras), module node (same without unlink), task node (`Run Task`, `Create Run Configuration` entries, `Jump to Source`, task activation submenu, `Assign Shortcut…`), run configuration node (`Edit Run Configuration…`, `Remove Run Configuration`, activation submenu, `Assign Shortcut…`). | Case BLD-SYNC-073 case 1. |
| BLD-SYNC-074 | SHOULD | `Assign Shortcut…` opens the keymap assignment for the task (the label of the chooser link: `Choose a task to assign a shortcut`); shortcuts are listed in the keymap under the group `External Build Systems`. | Case BLD-SYNC-074 case 1. |
| BLD-SYNC-075 | SHOULD | Run: double-click or Enter on a task runs it (`Run Task`, mnemonic R, icon execute); window shows `{system}: Executing {name}…` and `{system}: Execution finished {name}.` as console lines. | Case BLD-SYNC-075 case 1. |

## 9. Unlinked project detection

When a build script of a supported system exists in the project and no linked project covers it, a notification (group `External system build scripts found`) offers: title `{system} '{name}' build scripts found`; actions `Load {system} Project` and `Skip`; help text: `The IDE can import project information (e.g. sources and dependencies) from the {system} build script.` / `If you are unsure or this is not a {system} project, press 'Skip'.` / `You'll be able to import the {system} project later from context menu of {system} build script.` `Skip` disables the notification for that system for this project. An environment key can pre-decide it (`all`, or `System:import`, `System:skip` comma list).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-SYNC-080 | SHOULD | Notification texts and actions as above, one notification per unlinked project; `Skip` persisted per project and system. | Cases BLD-SYNC-080 case 1..2. |
| BLD-SYNC-081 | SHOULD | The project view context action `Link Gradle Project` / `Add as Maven Project` links a build file explicitly. | Case BLD-SYNC-081 case 1. |

## 10. Offline mode

Each build system has an offline toggle (`Toggle Offline Mode`, descriptions `Toggle offline mode for Gradle builds`, `Toggle offline mode for Maven builds`). It adds `--offline` to every command line of the tool. Details: BLD-GRADLE-044, BLD-MAVEN-040. A toggle is also reachable by search (`Toggle Gradle Offline Mode`, `Toggle Maven Offline Mode`).

## 11. Dependency analyzer (common part)

`Analyze Dependencies…` / `Analyze {system} Dependencies…` opens an editor tab titled `Dependency Analyzer`. Elements: project chooser, `Scope:` filter (`Standard`, `Custom`, `Any`, `None`, `{n} scopes`, `default`), list of `Resolved Dependencies` (`Show as Tree`, `Collapse`, `Expand`, empty text `Nothing found.`), usages list `Usages of {name}`, `Omitted`, option `Show Conflicts Only`, `Show GroupId`, `View Options`; warnings `conflict with {version}` (title `Conflict with {version}`) and `unresolved` (title `Unresolved`); tooltip fields `Module:`, `Artifact:`, `Scopes:`, `Status:`; actions `Go to {system} Dependency` (`Navigates to dependency in parent {system} config file`) and `Open {system} Config`. While loading: `Loading dependencies list…` and for Gradle `Load Gradle Dependencies`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| BLD-SYNC-090 | SHOULD | Dependency analyzer UI with texts above. | Case BLD-SYNC-090 case 1. |
| BLD-SYNC-091 | LATER | Full dependency graph interactions (selection sync between lists, usage tree expansion) - not needed for the first release. | n/a |

## 12. Inventory coverage (framework)

All `ExternalSystem.*` actions of the inventory are covered: `AfterCompile/AfterRebuild/AfterSync/BeforeCompile/BeforeRebuild/BeforeSync/BeforeRun` (7), `AssignShortcut`, `AssignRunConfigurationShortcut` (BLD-SYNC-074), `AttachProject`, `DetachProject`, `IgnoreProject`, `OpenConfig`, `OpenTasksActivationManager`, `RefreshAllProjects`, `RefreshProject`, `SelectProjectDataToImport` (BLD-SYNC-040), `ProjectRefreshAction`, `HideProjectRefreshAction` (BLD-SYNC-020..023), `CollapseAll`, `ExpandAll`, `GroupModules`, `GroupTasks`, `ShowIgnored`, `ShowInheritedTasks` (BLD-SYNC-071), `RunTask` (BLD-SYNC-075), `EditRunConfiguration`, `RemoveRunConfiguration` (BLD-SYNC-073), `ShowSettings`, `ShowCommonSettings` (BLD-SYNC-001, 040). Settings page `Build Tools` (BLD-SYNC-001).

## Open questions

1. Text of the Load button: the task description calls it `Load Gradle Changes`/`Load Maven Changes`; the current bundle uses `Sync {system} Changes`. Spec follows the bundle.
2. The exact wording of the sync tab root for Gradle (`… sync` vs `{project} Sync`): to be taken from observation.
3. Which settings files exactly count for Gradle and Maven: see 03 BLD-GRADLE-020 and 04 BLD-MAVEN-020.
4. The behaviour of the `Hide This Notification` action when later changes arrive for the same files.

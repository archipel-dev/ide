# Platform kernel

Status: Phase 1 of the platform refactor plan (`docs/architecture/platform-refactor-plan.md`).
Code: `crates/archipel-ide-platform` (the kernel) and
`crates/archipel-ide-core/src/{platform.rs,services/}` (the IDE's wiring and the first services).

`IdeState` used to hold the state of every feature (98 fields), and 73 files added `impl IdeState`
blocks, so features reached into each other. The IntelliJ Platform avoids this with an
`Application` and a `Project`, each owning lazily created, typed **services**; a **message bus**
with typed topics for events; **extension points** for contributions; and a **disposer** tree
that releases everything in the right order. The kernel gives Archipel IDE the same shapes. It
has no UI toolkit dependency and no global mutable state: everything hangs off an `Application`
value that `IdeState` owns.

Behavior and names follow IntelliJ's public documentation and its Apache-2.0 source; the code is
ours (clean room, see `AGENTS.md`).

## Concepts and their IntelliJ counterparts

| Kernel | IntelliJ | Notes |
|---|---|---|
| `Application` | `Application` | Process-wide: application services, application bus, extension points, `ProjectManager`. Cheap to clone (a handle). |
| `Project` | `Project` | One open project: project services and its own bus. Not to be confused with `archipel_ide_project::Project` (the file model of the root), which `IdeState::project` still holds. |
| `ComponentManager` (trait) | `ComponentManager` | What both share: `service::<T>()`, `get_service::<T>()`, `try_service::<T>()`, `register_service`, `message_bus()`, `disposable()`. |
| `ServiceDescriptor<O>` | `<applicationService>` / `<projectService>` in `plugin.xml` | A factory. The service is created on first access, once. Add one at any time, so plugins and language packs can register services when they load. |
| `ServiceState<T>` | (a service with its own locking) | Reader/writer cell for the mutable state of a service. |
| `Disposable`, `Disposer`, `DisposableId` | `Disposable`, `Disposer` | Parent/child tree. Children are disposed first, newest registration first, the parent last. Idempotent. An instance (one per application), not a static. |
| `MessageBus`, `Topic<L>`, `MessageBusConnection` | `MessageBus`, `Topic`, `MessageBusConnection` | `L` is a trait object: `Topic<dyn ProjectManagerListener>`. Subscriptions end with their parent `Disposable`. |
| `BusSender` + `deliver_pending` | `invokeLater` plus the bus's queue | Background threads `post`; the UI thread delivers once per frame. |
| `ExtensionPointName<T>`, `ExtensionRegistry`, `LoadingOrder` | `ExtensionPointName`, `ExtensionsArea`, `LoadingOrder` | Ordered and id-addressable. `First`, `Last`, `Before(id)`, `After(id)`. |
| `ProjectManager`, `ProjectManagerListener` | `ProjectManager`, `ProjectManagerListener` | `open` fires `project_opened`; `close` fires `project_closing`, disposes the project, then fires `project_closed`. |

Differences worth knowing:

- **Services are `Arc<T>`.** `T` is `Send + Sync` and uses interior mutability (`ServiceState`,
  `Mutex`, atomics). A service must not store an `Application` or `Project` handle (a reference
  cycle); use `application.downgrade()`, a clone of the `MessageBus`, or the `Arc`s of the services
  it needs.
- **`service::<T>()` panics** when `T` is not registered, as IntelliJ's `service<T>()` throws.
  It is a programming error. Use `get_service::<T>()` when absence is normal (for example a
  project service while no project is open, or after the owner was disposed).
- **No generated listener proxy.** The publisher passes a closure that calls the listener:
  `bus.publish(TOPIC, |l| l.something_happened(&data))`. Publishing is synchronous and runs the
  subscribers in subscription order before it returns. A panicking subscriber is recorded
  (`take_failures`) and the others still run.
- **No broadcast to child buses.** Subscribe on the bus you want events from.
- **Lock order.** Service creation is serialized per owner. Application services must not depend
  on project services.

## How it is wired in the IDE

```
IdeState
 └─ platform: Platform                      (crates/archipel-ide-core/src/platform.rs)
     ├─ application: Application
     │   ├─ services    ToolchainManager, SettingsService, Notifications, KeymapManager,
     │   │            UpdateChecker, RecentProjectsManager, CloneManager, ActionManager,
     │   │            SearchEverywhereManager
     │   ├─ bus         ProjectManager::TOPIC, TOOLCHAIN_TOPIC, NOTIFICATION_TOPIC, SETTINGS_TOPIC,
     │   │            UPDATE_TOPIC, CLONE_TOPIC
     │   └─ project services declared for every project: ToolchainRequirementsService, GitRepositories,
    │      FindInProjectService, GitLogService, BlameAnnotations,
    │      LineStatusTracker, GitRemoteOperations
     └─ project: Option<Project>            (None on the welcome screen)
         ├─ services    created lazily, disposed on close
         └─ bus         GIT_REPO_CHANGE, FIND_TOPIC, LINE_STATUS_TOPIC, REMOTE_TOPIC
```

- `open_project` and `close_project` on `IdeState` call `Platform::open_project` and
  `close_project`, which go through the `ProjectManager`: opening fires `project_opened`, closing
  fires `project_closing`, disposes the project (its services, watchers and subscriptions) and
  fires `project_closed`. Opening a project while another is open closes the old one first.
- `IdeState` reaches services through thin accessors in `platform.rs`:
  `state.settings()`, `state.toolchains()`, `state.notifications()`, `state.git()`,
  `state.git_status()`, `state.toolchain_requirements()`, `state.find()`, `state.keymap()`,
  `state.updates()`, `state.git_log()`, `state.execution()`, `state.debugger()`, `state.terminal()`,
  `state.spring()` and so on. Commands and the UI use these (or
  `state.application().service::<T>()` and `state.platform_project()`).
- **Run, debug and terminal without a project.** The Run, debug and Terminal services are project
  services, but the UI asks for them on the welcome screen too. Their accessors return the
  project's service while one is open, and otherwise a *detached* instance owned by `Platform`
  (so call sites need no `Option`). It never belongs to a project: opening a project resets it
  (a shell started on the welcome screen is killed), and a project's own services start empty.
- **Guards.** These services keep their state in a `Cell` (`services/mod.rs`): a read-write lock
  whose reads nest, so the UI can paint from a read guard (`state.execution().runner()`) and
  call commands that read again. Writes (`runner_mut`, `state_mut`, `configurations_mut`, ...)
  are exclusive: do not write while a guard of the same cell is alive on the thread. Debug
  builds panic after ten seconds with the name of the cell instead of hanging. Release guards
  before dispatching a command that changes the state.
- **Reactions.** A feature that still lives on `IdeState` needs `&mut IdeState` to react to an
  event, which a bus listener cannot have. Its listener records a `Reaction` in a queue and
  `IdeState::dispatch_reactions` runs it right after the publisher returns. Today:
  `VcsChanged` (rescan the tree if files were added or removed; refresh the open diff; the line
  changes, Git log and blame invalidate themselves), `SearchFinished`, `RemoteFinished`,
  `UpdateMessage` (status line), `ToolchainInstalled` (restart language servers that were
  missing, start the Gradle sync once the install queue is idle), `RunExited` (a task exited on
  its own: continue a debug session that waited for its build, finish a Gradle or Bazel sync,
  status bar text) and `DebugPaused` (reveal the execution point in the editor and show the
  Debug tool window). The Build tool window is not a reaction: it is a listener of
  `EXECUTION_TOPIC` that updates the state inside `ExecutionManager`. When the reacting feature
  becomes a service, its listener does the work itself and the reaction disappears.

### Services in place

| Service | Scope | Replaced | Events |
|---|---|---|---|
| `ToolchainManager` (`ServiceState<ToolchainState>`) | application | `IdeState::toolchains`: discovery, user pins, background installs | publishes `TOOLCHAIN_TOPIC` (`component_installed`, `component_failed`) |
| `ToolchainRequirementsService` (`ServiceState<ToolchainRequirements>`) | project | `ToolchainState::{requirements, missing, dismissed}`: what the open project needs, the "missing toolchain" bars | |
| `SettingsService` (`ServiceState<Settings>`) | application | `IdeState::settings` | |
| `Notifications` | application | `IdeState::update_notification`, `PluginHostState::notifications` | publishes `NOTIFICATION_TOPIC` (`notification_shown`) |
| `GitRepositories` | project | `IdeState::{git, git_watcher, git_pending, vcs_stale}` | publishes `GIT_REPO_CHANGE` (`repository_changed(status, structure_changed)`) on the project bus |
| `ExecutionManager` | project, disposable | `IdeState::{runner, build_window, run_configurations, edit_run_configurations, runnable_cache}`: the task service, run/debug configurations, Build tool window, gutter run targets | publishes `EXECUTION_TOPIC` (`process_started`, `process_terminated`) |
| `XDebuggerManager` | project, disposable | `IdeState::debugger`: breakpoints, options, watches, the DAP session | publishes `DEBUG_SESSION_TOPIC` (`session_started`, `session_paused`, `session_resumed`, `session_stopped`) |
| `TerminalService` | project, disposable | `IdeState::{terminal, terminal_tabs}`: the shells of the Terminal tool window | publishes `TERMINAL_TOPIC` (`terminal_exited`) |
| `SpringStarters` | application | `IdeState::spring`: starter catalog of the New Project wizard | |
| `FindInProjectService` | project | `IdeState::{search, search_results, search_query, search_scope_kind, custom_scopes, search_custom_scope, search_background, search_options, search_files_scanned, search_truncated}`: Find in Files | publishes `FIND_TOPIC` (`search_finished(summary)`); the status line follows |
| `SearchEverywhereManager` (`ServiceState<QuickNavState>`) | application | `IdeState::quick_nav`: Search Everywhere, Recent Locations, Switcher. Application scope because the popup works on the welcome screen; `project_opened` resets it | |
| `ActionManager` (`ServiceState<ActionOptions>`) | application | `IdeState::include_disabled_actions` | |
| `GitLogService` | project | `IdeState::git_log` (snapshots are `Arc<GitLogState>`) | subscribes `GIT_REPO_CHANGE`: marks the log stale |
| `BlameAnnotations` | project | `IdeState::annotations` | subscribes `GIT_REPO_CHANGE`: re-blame |
| `LineStatusTracker` | project | `IdeState::line_changes` | subscribes `GIT_REPO_CHANGE`: drops `HEAD` texts, publishes `LINE_STATUS_TOPIC` (`line_status_invalidated`) |
| `GitRemoteOperations` | project | `IdeState::{remote_job, remote_progress, remote_error}` | publishes `REMOTE_TOPIC` (`remote_operation_finished(outcome)`) |
| `CloneManager` | application | `IdeState::{clone_job, clone_progress, clone_error}` (a clone runs before a project exists) | publishes `CLONE_TOPIC` (`clone_finished(outcome)`) |
| `KeymapManager` | application | `IdeState::keymap` | subscribes `SETTINGS_TOPIC`: rebuilds itself |
| `UpdateChecker` | application | `IdeState::{updates, update_state, update_silent, update_executable, update_installer}` | publishes `UPDATE_TOPIC` (`update_state_changed(state, message)`); balloon and status line follow |
| `RecentProjectsManager` (`ServiceState<RecentProjects>`) | application | `IdeState::recent_projects` | |

`SettingsService` now publishes `SETTINGS_TOPIC` (`settings_changed(&Settings)`) when a command
calls `notify_changed()` after a set, reset, apply or load. The keymap and the Gradle options
(`archipel_ide_gradle::set_options`) subscribe; nobody calls them after a settings change.

Left alone on purpose, because another change is working there: the LSP registry (`lsp`), the
daemon and the problems model (`problems`, `analysis`, `code_actions`).

## DataContext and actions (Phase 5)

Menus, the toolbar, tool window buttons, context menus, Find Action and keyboard shortcuts no
longer decide for themselves whether a command applies. They ask one registry of actions, and an
action answers from a `DataContext`.

| Here | IntelliJ | Where |
|---|---|---|
| `DataKey<T>`, `DataProvider`, `DataContext` | `DataKey`, `DataProvider`, `DataContext` | `archipel-ide-platform` (`data.rs`; the common keys are `data_keys::*`) |
| `AnAction`, `AnActionEvent`, `Presentation`, `ActionPlace`, `ActionEffect` | `AnAction`, `AnActionEvent`, `Presentation`, `ActionPlaces` | `archipel-ide-actions` (`anaction.rs`) |
| `ACTIONS`, `ActionRegistry` | the `<action>` extension point, `ActionManager` | the same: actions are extensions of `dev.archipel.action`, id = action id |

- **A `DataContext` is a stack of providers.** `IdeDataProvider` (`archipel-ide-core/src/ide_actions.rs`)
  answers the project root, the active file, caret, selected text, "has unsaved changes", Git,
  debug session, Android project and "command X is enabled". `UiDataProvider`
  (`archipel-ide-ui/src/actions.rs`) adds what only the window knows: Find bar text, Project view
  selection, editor focus (a key press), the target of a context menu. The innermost provider
  answers first; values are computed only when a key is asked for. Building a context copies
  nothing (`IdeApp::with_data`).
- **`update(&AnActionEvent) -> Presentation`** returns `enabled`, `visible`, `text`,
  `description` and `icon` from the context. It must be cheap: an open menu calls it every frame.
  `perform` returns an `ActionEffect` (dispatch a command, or a host request such as "open the Go
  to Line dialog"), so an action needs no `&mut` access to the IDE; `IdeApp::apply_effect` runs it.
- **Every typed command and every reference action id the keymap maps to one is an action**
  (`CommandAction`; `CommandActions::sync` registers them and follows plugin commands that come
  and go). The window's own actions (Find, Go to Line, New File, clipboard, ...) are `UiActionImpl`
  in the UI crate and register first, so they own their ids.
- **Places.** `ActionPlace::Keymap` is where shortcuts differ from menus: menu-only bindings and
  editor-only bindings stay with the focused widget, diff keys apply to the diff tab, debugger keys
  stay free outside a debug session, and Android redirects (Run deploys `app`, Go to Declaration
  resolves `R.x.y`) apply. All of it is in `CommandAction::update`/`perform`, not in the dispatcher.
- **Context menus** put their per-target rules (`CloseAllToTheLeft` needs a tab to the left, Paste
  needs a clipboard) in front of an action (`ContextMenuAction`), fed by a `ContextFacts` value in
  the data context.

### Adding an action

```rust
struct CloseFile;
impl AnAction for CloseFile {
    fn update(&self, e: &AnActionEvent) -> Presentation {
        Presentation::when(e.data().has(&data_keys::ACTIVE_FILE))
    }
    fn perform(&self, _: &AnActionEvent) -> ActionEffect {
        ActionEffect::command("editor.close", Value::Null)
    }
}
ActionRegistry::new(app.extensions()).register("CloseContent", Arc::new(CloseFile), None)?;
```

Prefer a command (`archipel_ide_core`, registered in `built_in_commands`) when the action changes
state: it is then an action automatically, with `enabled` from the command's own predicate. Write
an `AnAction` for what only needs to ask the window to do something. A new fact for `update` is a
`DataKey` that `IdeDataProvider` (state) or `UiDataProvider` (window) answers; do not read
`state.<field>` in the UI.

### The UI reads through handles, and a lint counts what is left

`crates/archipel-ide-ui/tests/no_subsystem_reads.rs` has two ratchets, like
`IDE_STATE_FIELD_BASELINE`:

1. `SUBSYSTEM_READ_BASELINE` counts `state.<field>` for every `pub` field of `IdeState` in
   `archipel-ide-ui/src` (test modules and comments excluded). It is **0**: the UI does not name
   a field of `IdeState`. Methods (`state.settings()`, `state.active_doc()`,
   `state.project_root()`) are accessors and do not count.
2. `HANDLE_BASELINE` counts calls of the *transitional handles* in
   `archipel-ide-core/src/views.rs` (`state.android()`, `state.agents_mut()`, ...). A handle hands
   out a whole subsystem that still lives on `IdeState`; it is the seam where the subsystem
   becomes a service (the handle then returns the service's guard and call sites do not change).
   Each call is a read left to migrate to a `DataKey`, a narrower accessor or a service call.
   Lower the baseline when calls go; never raise it. Adding a handle for a new subsystem is a
   sign the subsystem should be a service instead (see "How to add a service").

Both failure messages say what to lower the baseline to.

## How to add a service

1. **Decide the scope.** One per process (settings, toolchains, keymap) is an application
   service; one per open project (index, Git state, run configurations) is a project service.
2. **Write the type.** `Send + Sync`, owning its state, with `&self` methods. For mutable state use
   `ServiceState<State>` (`read()` and `write()` guards, `with` and `with_mut` closures) or a
   `Mutex`. Keep guards short and never call into another feature while holding a write guard.
   If it owns resources (threads, files), implement `Disposable` and register it with
   `ServiceDescriptor::disposable`; it is then disposed with its owner.
3. **Register it** in `register_core_services` (`platform.rs`):

   ```rust
   application.register_service(ServiceDescriptor::new(|app: &Application| {
       Notifications::new(app.message_bus().clone())
   }))?;
   application.register_project_service(ServiceDescriptor::disposable(
       |project: &Project| GitRepositories::new(project.message_bus().clone()),
   ))?;
   ```

   A plugin or language pack registers its own descriptors when it loads (`register_service`,
   `register_project_service`); a project service declared later also reaches projects that are
   already open.
4. **Add an accessor** on `IdeState` in `platform.rs`
   (`pub fn git(&self) -> Option<Arc<GitRepositories>>`) so call sites do not repeat the type.
5. **Delete the field** from `IdeState`, move its methods into the service, and update call sites
   (`state.git.as_ref()` becomes `state.git_status()`). Lower `IDE_STATE_FIELD_BASELINE` in
   `crates/archipel-ide-core/tests/ide_state_size.rs` and tick the field in the checklist below.
6. **Test the service by itself**, without `IdeState`: build an `Application`, register the
   descriptor, call `service::<T>()` (see `services/git.rs` and `services/notifications.rs`).

## How to add a topic

```rust
pub trait GitRepositoryChangeListener: Send + Sync {
    fn repository_changed(&self, _status: &RepositoryStatus, _structure_changed: bool) {}
}
pub const GIT_REPO_CHANGE: Topic<dyn GitRepositoryChangeListener> =
    Topic::new("GitRepositoryChangeListener");
```

- Default every method to a no-op, so listeners implement only what they need and adding an
  event is not a breaking change.
- Publish on the bus of the owner the event belongs to (`project.message_bus()` for project
  events): `bus.publish(GIT_REPO_CHANGE, |l| l.repository_changed(&status, changed))`.
- Subscribe with a parent that bounds the subscription:
  `project.message_bus().subscribe(project.disposable(), GIT_REPO_CHANGE, Arc::new(listener))`.
  Use `bus.connect(parent)` to group several subscriptions and `disconnect()` to end them early.
- From a background thread use `bus.sender().post(TOPIC, |l| ...)`; the UI thread calls
  `bus.deliver_pending()` once per frame (`set_waker` can request a repaint).
- An event is something that **happened** (`component_installed`), not a request for another
  feature to do something. If you are tempted to name a topic after an action, call that
  feature's service instead.

## How to add an extension point

```rust
pub const INSPECTIONS: ExtensionPointName<dyn Inspection> =
    ExtensionPointName::new("dev.archipel.inspection");

// contributor (a feature, a plugin, a language pack):
app.extensions().register(INSPECTIONS, "java.unusedImport", LoadingOrder::Anywhere,
                          Arc::new(UnusedImport), Some(pack_disposable))?;

// consumer:
for inspection in app.extensions().extension_list(INSPECTIONS) { /* in loading order */ }
let one = app.extensions().find_by_id(INSPECTIONS, "java.unusedImport");
```

- Ids are unique per extension point. A duplicate is an error, not an override.
- Pass a parent `DisposableId` and the extension is removed when the contributor is disposed
  (plugin unload). `add_change_listener` tells consumers that cache the list.
- `LoadingOrder::First` and `Last` pin to the ends; `Before(id)` and `After(id)` place next to
  another extension (a target that is not registered is ignored).
- The language extension points are declared (`archipel-ide-analysis::langext`, plus
  `LANGUAGE_SERVER_PROVIDER` and `TOOLCHAIN_REQUIREMENT_PROVIDER` in their crates); see
  [`language-packs.md`](language-packs.md). Run configuration types arrive with Phase 2.

## Migration recipe for the remaining fields

Do one feature at a time; every step keeps the tests green.

1. Pick a group from the checklist below and read how the UI, the commands and other features use
   the fields (`rg 'state\.<field>|self\.<field>'`).
2. Move the fields, and the code that only touches them, into a service type under
   `archipel-ide-core/src/services/` (or its own crate when it has a stable engine, as with
   `archipel-ide-git`). Keep behavior; do not redesign while moving.
3. Replace each direct cross-feature call that is really an event (`self.invalidate_x()` after a
   state change, `self.restart_y()` after an install) by a topic. While the reacting code still
   needs `&mut IdeState`, record a `Reaction` as described above.
4. Replace `state.<field>` at every call site with the accessor or `service::<T>()`. The UI reads
   through the service (never `state.<feature>.<internal>`); for data that is read every frame,
   return an `Arc` snapshot (`git_status()`) instead of cloning.
5. Delete the field, lower `IDE_STATE_FIELD_BASELINE`, tick the checklist.
6. The UI reads through service accessors, handles (`views.rs`) and `DataContext` keys; do not
   add `state.<field>` reaches in `archipel-ide-ui` (a lint fails; see "DataContext and actions").

Pitfalls seen while moving the first four:

- A guard held across a call that locks the same service for writing deadlocks. Take a guard in a
  block, copy out what you need, drop it, then call.
- `ServiceState::read` is reentrant, `write` is not.
- State that used to be reset in `open_project` (`self.git = None`) is now reset by the project's
  disposal; state that survives projects belongs in an application service.
- Types that are not `Sync` (an `mpsc::Receiver`) need a `Mutex` around them to live in a service.

## Checklist: remaining `IdeState` fields by future service

`crates/archipel-ide-core/tests/ide_state_size.rs` fails if a field of `IdeState` is missing from
this list (so it cannot go stale) or if the struct grows. Tick a field when it has left
`IdeState`, and remove it from the struct in the same change.

**Stays on `IdeState` for now (the glue)**
- [ ] `platform`: the kernel itself (`Platform`).
- [ ] `project`: the file model of the root; becomes part of the project model (Phase 2).
- [ ] `status`: status bar text; becomes the status bar of a `WindowManager` service in Phase 5.

**Project model and file index (Phase 2: `ProjectFileIndex`, VFS, ExternalSystem)**
- [ ] `files`, `files_generation`, `model_generation`, `tree_cache`: the file list and the Project
  tool window's row cache.
- [ ] `watcher`: the project file watcher (VFS refresh).
- [ ] `expanded_dirs`, `project_view`: Project tool window state.
- [ ] `gradle_model`, `gradle_error`, `gradle_changes_pending`: Gradle import.
- [ ] `bazel_targets`, `bazel_error`: Bazel import.
- [ ] `detected_build`, `build_systems`: build system detection.
- [ ] `android`: Android SDK and project state (project service).

**Editor (`FileEditorManager`, `EditorHistoryManager`)**
- [ ] `documents`, `active_document`: open documents and the selected one.
- [ ] `tabs`, `splits`, `splits_synced`: tab strip, pinned and preview tabs, editor splits.
- [ ] `recent_files`: recent files list.
- [ ] `markdown_layouts`: Markdown editor and preview layout per file.
- [ ] `completion`, `completion_stats`, `snippet`: completion popup and live template session.
- [ ] `diff`, `diff_active`: the diff viewer tab.

**Code intelligence (Phase 3 and 4; the LSP registry, daemon and problems are changing elsewhere)**
- [ ] `lsp`: language server registry.
- [ ] `analysis`: native analysis world.
- [ ] `problems`, `problems_cache`: problem index (`DaemonCodeAnalyzer`).
- [ ] `code_actions`: Alt+Enter state (intentions and quick fixes).
- [ ] `code_vision`, `signature`, `docs`: lenses, parameter info, quick documentation.
- [ ] `refactor`, `generate`, `hierarchy`, `navigation`: refactoring, Generate popup, hierarchy
  trees, navigation history and requests.

**Search and navigation (`SearchEverywhere`, `FindManager`)**
- [x] `search`, `search_results`, `search_query`, `search_scope_kind`, `custom_scopes`,
  `search_custom_scope`, `search_background`, `search_options`, `search_files_scanned`,
  `search_truncated`: Find in Files.
- [x] `quick_nav`: Search Everywhere, Recent Locations, Switcher.
- [x] `include_disabled_actions`: Find Action option (`ActionManager`).

**VCS (project services next to `GitRepositories`)**
- [x] `git_log`: Git Log tab state.
- [x] `annotations`: blame annotations.
- [x] `line_changes`: gutter change markers (`LineStatusTracker`).
- [x] `remote_job`, `remote_progress`, `remote_error`: fetch, pull and push.
- [x] `clone_job`, `clone_progress`, `clone_error`: Get from VCS clone.

**Run and build (`RunManager`, `ExecutionManager`, `XDebuggerManager`)**
- [x] `runner`: the task service, now `ExecutionManager::runner()` (project service).
- [x] `build_window`: Build tool window tabs and console, now `ExecutionManager::build_window()`.
- [x] `run_configurations`, `edit_run_configurations`: run/debug configurations and the dialog request,
  now `ExecutionManager::configurations()` and `request_edit_configurations`.
- [x] `runnable_cache`: gutter run targets, now `ExecutionManager::runnables()`.
- [x] `debugger`: breakpoints and the debug session, now `XDebuggerManager` (project service).
- [x] `spring`: Spring Boot starter catalog of the New Project wizard, now `SpringStarters`, a
  small application service (the wizard runs without a project, and there is no generator
  extension point yet; when it exists the Spring generator contributes to it and uses this
  service as its store).

**Workbench (`ToolWindowManager`, `WindowManager`)**
- [ ] `tool_windows`: tool window layout.
- [ ] `workbench_mode`: Code or Agents arrangement.
- [x] `terminal`, `terminal_tabs`: Terminal tool window sessions, now `TerminalService` (project service).
- [ ] `repaint`, `waker`: UI wake hooks (one application `UiWaker` service).
- [ ] `injected_dialog_paths`: paths queued for the next native file dialog (tests, control socket).
- [ ] `settings_request`: settings page the UI should open.
- [ ] `about_open`, `first_run_open`, `first_run_dir`, `crash_report`: dialogs and first-run state.
- [x] `recent_projects`: welcome screen list (`RecentProjectsManager`, application service).

**Application services still to extract**
- [x] `keymap`: `KeymapManager` (application service; reads `SettingsService`).
- [x] `updates`, `update_state`, `update_silent`, `update_executable`, `update_installer`: update
  checking and installation (`UpdateChecker`).
- [ ] `plugin_host`: plugin registry, plugin tool windows and tabs (`PluginManager`;
  contributions become extensions).
- [ ] `ai`, `agents`: AI assistant and agents (application and project services).

**Already moved** (see "Services in place"): `toolchains`, `settings`, `update_notification`,
`git`, `git_watcher`, `git_pending`, `vcs_stale`, and the ticked groups above (including `runner`,
`build_window`, `run_configurations`, `edit_run_configurations`, `runnable_cache`, `debugger`, `spring`,
`terminal`, `terminal_tabs`).

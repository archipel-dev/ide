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
     │   ├─ services    ToolchainManager, SettingsService, Notifications
     │   ├─ bus         ProjectManager::TOPIC, TOOLCHAIN_TOPIC, NOTIFICATION_TOPIC
     │   └─ project services declared for every project: ToolchainRequirementsService, GitRepositories
     └─ project: Option<Project>            (None on the welcome screen)
         ├─ services    created lazily, disposed on close
         └─ bus         GIT_REPO_CHANGE
```

- `open_project` and `close_project` on `IdeState` call `Platform::open_project` and
  `close_project`, which go through the `ProjectManager`: opening fires `project_opened`, closing
  fires `project_closing`, disposes the project (its services, watchers and subscriptions) and
  fires `project_closed`. Opening a project while another is open closes the old one first.
- `IdeState` reaches services through thin accessors in `platform.rs`:
  `state.settings()`, `state.toolchains()`, `state.notifications()`, `state.git()`,
  `state.git_status()`, `state.toolchain_requirements()`. Commands and the UI use these (or
  `state.application().service::<T>()` and `state.platform_project()`).
- **Reactions.** A feature that still lives on `IdeState` needs `&mut IdeState` to react to an
  event, which a bus listener cannot have. Its listener records a `Reaction` in a queue and
  `IdeState::dispatch_reactions` runs it right after the publisher returns. Today:
  `VcsChanged` (invalidate line changes, Git log and blame; rescan the tree if files were added or
  removed; refresh the open diff) and `ToolchainInstalled` (restart language servers that were
  missing, start the Gradle sync once the install queue is idle). When the reacting feature
  becomes a service, its listener does the work itself and the reaction disappears.

### Services in place

| Service | Scope | Replaced | Events |
|---|---|---|---|
| `ToolchainManager` (`ServiceState<ToolchainState>`) | application | `IdeState::toolchains`: discovery, user pins, background installs | publishes `TOOLCHAIN_TOPIC` (`component_installed`, `component_failed`) |
| `ToolchainRequirementsService` (`ServiceState<ToolchainRequirements>`) | project | `ToolchainState::{requirements, missing, dismissed}`: what the open project needs, the "missing toolchain" bars | |
| `SettingsService` (`ServiceState<Settings>`) | application | `IdeState::settings` | |
| `Notifications` | application | `IdeState::update_notification`, `PluginHostState::notifications` | publishes `NOTIFICATION_TOPIC` (`notification_shown`) |
| `GitRepositories` | project | `IdeState::{git, git_watcher, git_pending, vcs_stale}` | publishes `GIT_REPO_CHANGE` (`repository_changed(status, structure_changed)`) on the project bus |

Left alone on purpose, because another change is working there: the LSP registry (`lsp`), the
daemon and the problems model (`problems`, `analysis`, `code_actions`).

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
- No extension point exists yet. The first ones (languages, file types, inspections, run
  configuration types) arrive with the language packs and Phase 2.

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
6. Phase 5 turns the remaining accessors into `DataContext` reads; do not add new `state.*`
   reaches in `archipel-ide-ui`.

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
- [ ] `git_log`: Git Log tab state.
- [ ] `annotations`: blame annotations.
- [ ] `line_changes`: gutter change markers (`LineStatusTracker`).
- [ ] `remote_job`, `remote_progress`, `remote_error`: fetch, pull and push.
- [ ] `clone_job`, `clone_progress`, `clone_error`: Get from VCS clone.

**Run and build (`RunManager`, `ExecutionManager`, `XDebuggerManager`)**
- [ ] `runner`: the task service (project service).
- [ ] `build_window`: Build tool window tabs and console.
- [ ] `run_configurations`, `edit_run_configurations`: run/debug configurations and the dialog request.
- [ ] `runnable_cache`: gutter run targets.
- [ ] `debugger`: breakpoints and the debug session.
- [ ] `spring`: Spring Boot starter catalog of the New Project wizard (becomes a project
  generator extension).

**Workbench (`ToolWindowManager`, `WindowManager`)**
- [ ] `tool_windows`: tool window layout.
- [ ] `workbench_mode`: Code or Agents arrangement.
- [ ] `terminal`, `terminal_tabs`: Terminal tool window sessions.
- [ ] `repaint`, `waker`: UI wake hooks (one application `UiWaker` service).
- [ ] `injected_dialog_paths`: paths queued for the next native file dialog (tests, control socket).
- [ ] `settings_request`: settings page the UI should open.
- [ ] `about_open`, `first_run_open`, `first_run_dir`, `crash_report`: dialogs and first-run state.
- [ ] `recent_projects`: welcome screen list (`RecentProjectsManager`, application service).

**Application services still to extract**
- [ ] `keymap`: `KeymapManager` (application service; reads `SettingsService`).
- [ ] `updates`, `update_state`, `update_silent`, `update_executable`, `update_installer`: update
  checking and installation (`UpdateChecker`).
- [ ] `plugin_host`: plugin registry, plugin tool windows and tabs (`PluginManager`;
  contributions become extensions).
- [ ] `ai`, `agents`: AI assistant and agents (application and project services).

**Already moved** (see "Services in place"): `toolchains`, `settings`, `update_notification`,
`git`, `git_watcher`, `git_pending`, `vcs_stale`.

# 02. Run from context, the Run menu, the toolbar selector and the launch lifecycle

Status: specification. Requirement prefix `EXE-CTX`. Configuration model: `01-run-configurations.md`.
Console and Run tool window: `03-...`. Saving documents before the launch: `PLAT-SAVE-030`..`PLAT-SAVE-035`.
Cases: `cases/02-run-from-context.cases.md`.

ic: `platform/execution/src/com/intellij/execution/actions/{ConfigurationContext,RunConfigurationProducer,ConfigurationFromContext}.java`,
`platform/execution-impl/src/com/intellij/execution/actions/{BaseRunConfigurationAction,RunContextAction,ChooseRunConfigurationPopup,StopAction,RunConfigurationsComboBoxAction,RunCurrentFileExecutorAction}.java`,
`platform/execution-impl/src/com/intellij/execution/lineMarker/{RunLineMarkerProvider,RunLineMarkerContributor,ExecutorAction}.*`,
`platform/execution-impl/src/com/intellij/execution/impl/ExecutionManagerImpl.kt`,
`platform/execution-impl/resources/intellij.platform.execution.impl.actions.xml`,
`platform/xdebugger-impl/ui/resources/intellij.platform.debugger.impl.ui.actions.xml`,
`platform/execution/src/com/intellij/execution/executors/DefaultRunExecutor.java`,
`platform/xdebugger-api/src/com/intellij/execution/executors/DefaultDebugExecutor.java`,
`java/execution/impl/src/com/intellij/execution/application/{AbstractApplicationConfigurationProducer,ApplicationRunLineMarkerProvider}.java`,
`java/java-impl/src/com/intellij/testIntegration/TestRunLineMarkerProvider.java`,
`platform/platform-resources/src/keymaps/$default.xml`, `.../Mac OS X 10.5+.xml`,
`platform/platform-resources-en/src/messages/ActionsBundle.properties`,
`platform/execution/resources/messages/ExecutionBundle.properties`.

## 1. Executors and the Run menu

An **executor** has an id, a start text, an icon, a tool window and a context action id:

| Executor | Id | Start text | With name | Context action | Tool window |
|---|---|---|---|---|---|
| Run | `Run` | `Run` (mnemonic `u`) | `Run '<name>'` | `RunClass` | Run |
| Debug | `Debug` | `Debug` (mnemonic `D`) | `Debug '<name>'` | `DebugClass` | Debug |

Further executors (Coverage: `Run '<name>' with Coverage`, `Run with Coverage`) register the same way.

Menu `Run` (top-level, after Refactor): `Run`-group items in this order. Shortcuts are the default
Windows/Linux keymap; the macOS keymap is listed in the last column where it differs.

| Entry (text) | Action id | Shortcut | macOS | Enabled when |
|---|---|---|---|---|
| `Run '<selected>'` | `Run` | `Shift+F10` | `Ctrl+R` | a configuration is selected and a runner supports it |
| `Debug '<selected>'` | `Debug` | `Shift+F9` | `Ctrl+D` | same |
| other executor items (`Run '<selected>' with Coverage`, ...) | per executor | none | none | same |
| separator | | | | |
| `Run...` | `ChooseRunConfiguration` | `Alt+Shift+F10` | | always |
| `Debug...` | `ChooseDebugConfiguration` | `Alt+Shift+F9` | | always |
| `Edit Configurations...` | `editRunConfigurations` | none | | always |
| `Manage Targets...` | `ManageTargets` | none | | a target type exists |
| separator | | | | |
| `Stop` / `Stop '<name>'` | `Stop` | `Ctrl+F2` | | a process is running |
| `Stop Background Processes...` | `StopBackgroundProcesses` | `Ctrl+Shift+F2` | | background processes exist |
| `Show Running List` | `ShowLiveRunConfigurations` | none | | always |
| debug actions group (section 8 of `05`) | `DebugActionsInRunGroup` | | | |
| test group (`RunTestGroup`) | | | | |

Other actions of the area (not in the Run menu): `Run Context Configuration` (`RunClass`,
`Ctrl+Shift+F10`, macOS `Ctrl+Shift+R`), `Debug Context Configuration` (`DebugClass`, no default
shortcut), `Rerun` (`Ctrl+F5`, macOS `Cmd+R`), `Rerun Tests` (`Alt+Shift+R`), `Rerun Failed Tests`
(none), `Update Running Application` (`Ctrl+F10`), `Send EOF` (`Ctrl+D` in a console), `Attach Debugger`
(Run tool window), `Select Run/Debug Configuration` (toolbar combo, none), `Toggle All Run/Debug
Configurations` (none), `Activate Run Tool Window` (`Alt+4`), `Activate Debug Tool Window` (`Alt+5`),
`Activate Services Tool Window` (`Alt+8`), `Show Code Coverage Data` (`Ctrl+Alt+F6`), `Run Anything`
(double Ctrl).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CTX-001 | MUST | The Run menu has the entries of the table in that order with those texts, shortcuts, and enabled rules. Mnemonics: `R_un` for the menu; `Edit Configu_rations...`; `Show Running List` has none. | Opening the menu with no configurations: `Run...`/`Debug...`/`Edit Configurations...` enabled, `Run`/`Debug`/`Stop` disabled. Case EXE-CTX-001 case 1. |
| EXE-CTX-002 | MUST | The shortcut set above is the default keymap. In the macOS keymap `Run`=`Ctrl+R`, `Debug`=`Ctrl+D`, `Rerun`=`Cmd+R`, `Run Context Configuration`=`Ctrl+Shift+R`. | Pressing `Shift+F10` runs the selected configuration (Windows/Linux). |
| EXE-CTX-003 | MUST | Item texts embed the configuration name between single quotes, shortened by EXE-CONF-006. A mnemonic character in the name is escaped (an `_`/`&` in a name never becomes a mnemonic). | Configuration `A&B` shows `Run 'A&B'` with mnemonic `u` only. |

## 2. Launch lifecycle (executing a configuration)

Starting a configuration with an executor runs these steps, in order. A step that fails ends the
launch with the stated message and no process.

1. **Resolve** the runner for (executor id, configuration). None: error
   `{0}: there are no runners for {1}` (`dialog.message.no.runners.for.configuration`, `{0}` the
   executor id, `{1}` the configuration) or `Cannot find runner for {0}`.
2. **Untrusted project check**: if the target file is not trusted the error is `'{0}' is not trusted. Trust it to run configuration.`
3. **Target check**: unsupported target gives `Cannot run '{0}' on '{1}'`.
4. If the configuration has `Show the run/debug configuration settings before start`, open the Edit
   dialog (title `Edit Configuration`); Cancel aborts.
5. **Validate** the settings (EXE-CONF-046). A settings *error* opens the Edit dialog (title `Edit
   Configuration`); after OK the validation repeats; if still invalid the choice dialog titled `Change Configuration Settings` shows `Configuration is still incorrect. Do you want to edit it again?` with the buttons `Edit` and `Continue Anyway`. A *warning* does not stop the launch.
6. **Update** the usage order (recency) of temporary configurations.
7. **Save and build**: all documents are saved (`PLAT-SAVE-030`..`PLAT-SAVE-035`), before-launch tasks run (EXE-CONF-070..076). Failure cancels.
8. Wait for indexing when the type is not dumb aware (EXE-CONF-076).
9. **Start** the process; show or reuse the tool window tab; attach the exit listener.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CTX-010 | MUST | The launch follows steps 1..9 above in that order. The error dialog for failures at steps 1..3 and 9 has the title `Run Configuration Error` (for execution errors `Error running '<name>'`, i.e. `Error running ''{0}''`); the body is the failure message. | Case EXE-CTX-010 case 1..3. |
| EXE-CTX-011 | MUST | A second start request of the same (configuration, executor, runner) while the previous one is still starting is ignored; the Run action is disabled during start. | Rapid double `Shift+F10` starts one process. |
| EXE-CTX-012 | MUST | Content of the Run tool window for a configuration is reused (same tab) when the previous run of that configuration has terminated; a still running one blocks reuse (see EXE-CTX-020). | Second run reuses the tab named after the configuration. |
| EXE-CTX-013 | MUST | On normal completion nothing is added besides the exit line; on a failed launch no tab is created. | A failed Build leaves no Run tab. |

## 3. Rerun, Stop and multiple instances

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CTX-020 | MUST | `Allow multiple instances` off (default): starting a configuration that already runs asks `'{0}' is not allowed to run in parallel.` + newline + `Would you like to stop the running one?` (or `{1} running instances` for more than one) with the button `Stop and Rerun`. Accepted: the running process is stopped, the launch waits until it terminated (polled every 100 ms), then starts. Declined: nothing happens. The question has a `Don't ask again` check box (advanced setting `confirm.rerun.with.termination`, default on). | Case EXE-CTX-020 case 1. |
| EXE-CTX-021 | MUST | `Allow multiple instances` on: start never asks and never stops the earlier instance; each start opens its own tab. | Two instances run side by side. |
| EXE-CTX-022 | MUST | `Rerun` (`Ctrl+F5`, text `Rerun '<name>'`) stops the process of the selected tab (when still running; with the confirmation of EXE-CTX-020 when it applies), waits for termination, and starts the same configuration with the same executor again, reusing the tab; for a configuration with `Show the run/debug configuration settings before start` the dialog shows again. | Rerun of a running app: process 1 ends, process 2 starts in the same tab. |
| EXE-CTX-023 | MUST | Incompatible running configurations (a configuration declares that it cannot run beside another type) are stopped after confirmation: title `Incompatible Configuration Is Running` (`Incompatible Configurations Are Running` for several), message `'{0}' run configuration is not compatible with currently running {1} configuration.` (or `with following running configurations:` list), question `Are you sure you want to stop incompatible running instance?` button `Stop incompatible`. | Case EXE-CTX-023 case 1. |
| EXE-CTX-024 | MUST | `Stop` (`Ctrl+F2`): from the Run tool window or with a selected tab it stops that process; from the main menu/toolbar: one running process stops directly (text `Stop '<name>'`); several running processes open a popup titled `Stop Process` listing them by display name, with the extra first/last row `Stop All (<shortcut>)` and the focus on the most recently started; choosing an item stops it. A single-item popup has the title `Confirm Process Stop`. The Stop button icon shows a counter for 2+ processes and its text becomes `Stop...`. | Case EXE-CTX-024 case 1. |
| EXE-CTX-025 | MUST | Stopping first requests graceful termination (destroy); a process that does not end within the platform's wait stays in `terminating` state; Stop then offers `Kill process` (action description `Kill process`) which kills it. The tab shows no further output after the process exit line. | Second Stop press on a hung process kills it. |
| EXE-CTX-026 | SHOULD | `Stop Background Processes...` (`Ctrl+Shift+F2`) lists running background tasks (titles `Stop Background Process` / `Confirm Background Process Stop`). | With no background task the action is disabled. |
| EXE-CTX-027 | SHOULD | `Show Running List` shows the balloon `Running List` listing the running configurations; empty: `Nothing to show`; hint `Click to navigate` (selecting an entry activates its tab). | Case EXE-CTX-027 case 1. |
| EXE-CTX-028 | SHOULD | `Rerun Tests` (`Alt+Shift+R`) reruns the last test configuration (the configuration of the last executed test run); `Rerun Failed Tests` is specified in `04`. | After a test run in a project with another selected configuration, `Alt+Shift+R` runs the test configuration. |
| EXE-CTX-029 | SHOULD | `Update Running Application` (`Ctrl+F10`) lists the processes that support it (`Select Process to Update`); LATER for frameworks. | Not applicable without a supporting runner. |

## 4. Producers: configurations from context

The IDE derives a configuration from the editor caret/selection or the selected files in a tree
(the **context**: an element, its module, and whether several items are selected).

Procedure (`ConfigurationContext`):

1. Each registered producer (a language- or framework-specific component) is asked, for the
   context, either to *fill* a fresh copy of its factory's template from the context (the copy
   starts from the configuration the user selected, when it is of the same type; this carries
   options such as VM parameters over) or to say no.
2. Producers that say yes yield **candidates**, each with its *source element* (the most specific
   element the candidate is about, e.g. the method vs. its class).
3. Candidates are ordered: a candidate whose source element is *nested inside* another's wins
   (most specific first). When neither is nested, a candidate that *replaces* the other wins, then
   one that is *preferred*; otherwise registration order.
4. An **existing** configuration is looked for first: each producer is asked whether a stored
   configuration of its type is what it would produce for this context. If an existing one equals
   a candidate, the existing one is used and no candidate creates a duplicate. When the user
   explicitly selected a configuration (the context carries it), that one is used.
5. Otherwise the best candidate becomes a new **temporary** configuration with a unique name (EXE-CONF-002).

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CTX-030 | MUST | Context resolution: in an editor the element at the caret (the previous element when the caret is at the file end); else the first selected element; else the first selected file's PSI. Multiple selected items form a multi-selection context (producers decide). | Caret at end of file resolves the last element. |
| EXE-CTX-031 | MUST | Candidate ordering as in step 3. | Method `main` inside `Main`: the method candidate precedes the class candidate. Case EXE-CTX-031 case 1. |
| EXE-CTX-032 | MUST | An existing equivalent configuration is reused (matched by the producer's own equivalence, not by name); a configuration is created only when none matches. Equivalence for Application: same main class name, and the module of the context equals the configuration's module or the template's module; a test method in the class is not an Application context. | Run `Main` twice from the gutter creates one configuration. |
| EXE-CTX-033 | MUST | A producer failing (exception, index not ready, wrong type) yields no candidate and never breaks the others; compiled (binary) elements produce nothing except for equivalence matching with a source mirror. | An exception in one producer leaves the Run item available through the others. |
| EXE-CTX-034 | MUST | **Application producer**: applies when the context lies in a class (or file) that has a main method (static `main(String[])`, instance main and unnamed classes per the language level; see the Java spec). The source element is the main method when the caret is inside it, otherwise the class. The new configuration has main class = the class qualified name, generated name (EXE-CONF-004), module = the context module; the unnamed (implicit) class uses the file name and `Implicitly declared class run configuration`. Not produced for physical-less (non-physical) elements, for compiled elements, nor for Java hash-bang scripts. | Case EXE-CTX-034 case 1..3. |
| EXE-CTX-035 | MUST | **JUnit producers**: *method* (caret in a test method, or its name), *class* (caret in a test class outside a method, or on its name), *package* (selected package), *directory* (selected directory containing tests), *pattern* (multiple classes/methods selected), *unique id* and *category/tags* (framework-specific). Method and class producers apply only for classes recognized by a detected test framework; a class inside a nested test class targets the nested class. | Case EXE-CTX-035 case 1..4. |
| EXE-CTX-036 | MUST | JUnit method vs class: the method candidate's source element is the method; a caret inside the class body but outside any test method yields the class candidate; a caret inside a test method yields the method candidate first and the class candidate second (both listed in the *more* group). | Case EXE-CTX-036 case 1. |
| EXE-CTX-037 | SHOULD | Test method with parameters/source data: `Run 'x' with parameter` offers choosing the parameter index (`Run all parameters`, `Choose a parameter...`, `All Parameters ({0})`, title `Run '{0}'`, progress `Collecting parameters of '{0}'...`). | Parameterized test shows the chooser. |
| EXE-CTX-038 | SHOULD | Producers for Gradle tasks (build files), Maven goals (pom), Shell scripts (`.sh` file), JAR files, and Kotlin `main` follow the same procedure; their specific rules are in their plugin specs. | `.sh` file context offers Run of a Shell Script configuration. |
| EXE-CTX-039 | MUST | Producers are disabled while the project is dumb unless dumb aware; dumb-unaware producers are skipped silently. The Run item for such a configuration is absent (or shows `This action is not available while indexing`; the action's availability rule is in the platform dumb-mode spec). | During indexing a JUnit gutter item is missing. |

## 5. Run context actions (`Ctrl+Shift+F10`, context menu, project view)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CTX-040 | MUST | `Run Context Configuration` (`Ctrl+Shift+F10`) and the context-menu item `Run '<name>'` run the **existing** configuration for the context when there is one (EXE-CTX-032), else the best candidate (made temporary, selected in the toolbar combo, registered in start history). `Debug Context Configuration` does the same with Debug. | Case EXE-CTX-040 case 1. |
| EXE-CTX-041 | MUST | When no existing configuration matches and more than one candidate exists, the item is a popup group (the item text is empty, children list the candidates). Child text: the generated action name (for Application `<ShortClass>.main()` where the class part is shortened to 6 characters plus the 60-char rule; others `<Name><presentable type>`: names of 20 or more characters are cut to 20 plus `...`), or the type display name when not locatable; alternative-location candidates append the location name. When the candidates come from alternative locations and the executor is Run, a final child `Run All` with description `Run all configurations available in this context` starts them together. | Case EXE-CTX-041 case 1. |
| EXE-CTX-042 | MUST | The context menu has the group `Run Configurations` (title) containing `Run '<name>'`, `Debug '<name>'` and other executors, then (in `More Run/Debug`) `Modify Run Configuration...` and `Create '<name>'...`. `Modify Run Configuration...` opens the dialog titled `Create Run Configuration: '<name>'` for a not yet existing one or `Edit Run Configuration: '<name>'` for an existing one. | Right click on `Main`: items in that order. |
| EXE-CTX-043 | MUST | The items are hidden in an untrusted project; disabled while the same configuration/executor is starting. | Untrusted project: no Run items. |
| EXE-CTX-044 | SHOULD | The same group appears in the project view and navigation bar popup menus above the split/reveal group. Selecting several files may produce the pattern producer result. | Select two test classes in the tree: `Run 'FooTest and 1 more'`. |
| EXE-CTX-045 | SHOULD | `Run Current File` entry of the combo (when the setting is on): description `Run the file opened in the focused editor`, tooltip errors `Cannot run the current file: the editor is not in focus` and `The file in the editor is not runnable`; extra item `Run with Parameters...` (`Edit runner parameters and run the file opened in the focused editor`). | Combo item text `Current File`. |

## 6. Gutter run markers

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CTX-050 | MUST | A run marker is drawn at the **identifier** of (a) a class that has a main method, (b) the main method, (c) a test class, (d) a test method. Class marker for an implicit/nested rule: not shown for classes nested in an implicit class; for a method only when it is the main method of the class that would be chosen (own main or inherited one). Not shown for hash-bang scripts, compiled files, in-diff editors, injected fragments, untrusted files, or when the language contributor says hidden. | Case EXE-CTX-050 case 1..3. |
| EXE-CTX-051 | MUST | Icons: main marker = the green play triangle; test class/method marker = play triangle with the last stored state: passed -> green check variant, failed or error -> red cross variant, otherwise plain (class marker uses the double-arrow variant "run all"). State is the last known result for the test URL (`java:suite://<class>` for classes, `java:test://<class>/<method>` for methods) persisted across sessions. | After a failed test run the method marker turns red. |
| EXE-CTX-052 | MUST | Tooltip: for tests `Run Test` followed by the shortcut of the Run context action in grey (`Ctrl+Shift+F10`); for main markers the executor action names (`Run 'Main.main()'`, `Debug 'Main.main()'`), one per line. | Hover shows the text. |
| EXE-CTX-053 | MUST | Click: with exactly one action (only Run) it runs; otherwise the click opens the popup menu with the executor actions in registration order (Run, Debug, Run with Coverage, ...) then the extra group (`Modify Run Configuration...`). Right click always opens the popup. Each action acts on the element of its marker (not the caret). | Case EXE-CTX-053 case 1. |
| EXE-CTX-054 | MUST | Several markers on one line merge into one marker with the popup concatenating their actions, separated, with duplicates removed when one info replaces another. | A line with a class and a method name shows one marker. |
| EXE-CTX-055 | SHOULD | The class marker for a test class whose marker info also has a main method uses order 1 so the second candidate (the main) is available from the popup. | Class with tests and main: popup shows both. |
| EXE-CTX-056 | SHOULD | Dumb mode: markers of dumb-aware contributors stay (Application marker uses a syntactic main check); others appear after indexing (slow pass). | During indexing the Application marker is shown. |

## 7. Toolbar selector, popup and Run/Debug buttons

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CTX-060 | MUST | The main toolbar shows (in this order) the configuration selector, the executor buttons (`Run` green triangle, `Debug` bug), and `Stop` (red square, disabled when nothing runs). The selector shows the selected configuration's icon and name; when there is none it shows `Add Configuration...` (opens the dialog). Tooltip of the selector: `Run / Debug Configurations` (with `: <name>` when selected); description `Open run/debug configurations dropdown`. The Run button changes to the Rerun icon while the selected configuration is running; with `Allow multiple instances` off it reads `Rerun '<name>'`. | Case EXE-CTX-060 case 1. |
| EXE-CTX-061 | MUST | Selector popup content: the recent/available configurations grouped by type (under the header `Run Configurations` for existing items and `Recent Configurations`), each row with a submenu of executor actions; below: `Edit Configurations...`, `Run Configuration Templates`... as the dialog entries; with pinned items under `Pinned` (`Pin`/`Unpin` per item). Rows have a `Delete...` entry asking `Delete '{0}' from Configurations?` (title `Delete Configuration?`). | Popup lists `Edit...` and `Delete...` for temporary items. |
| EXE-CTX-062 | MUST | Chooser popup (`Alt+Shift+F10` Run, `Alt+Shift+F9` Debug): lists the configurations (permanent, then recent temporary) with numeric mnemonics `0-9` then letters; `Enter` runs/debugs the item with the popup's executor; holding `Shift` switches to the alternative executor (Run popup acts as Debug and vice versa; `Shift+Enter` and `Shift+<number>` invoke the alternative action); `F4` edits (dialog title `Edit Run Configuration: '<name>'`... via `Edit Configuration Settings`), `Delete` deletes (with confirmation `Are you sure you want to delete {0}?` and `Don't ask again`); the status line (ad) texts are `Hold Shift to <executor action>`, `Press F4 to Edit`, `Press Delete to Delete configuration`. | Case EXE-CTX-062 case 1..2. |
| EXE-CTX-063 | MUST | Selecting a configuration (by a run from context with `shouldSetRunConfigurationFromContext` or in the combo) stores it as selected; `selected` is persisted (EXE-CONF-020). Running from a context selects the configuration run. | After a gutter run the combo shows that configuration. |
| EXE-CTX-064 | SHOULD | `Add New Configuration` entries of the popup show only relevant types first and `{0} more items...` for the rest (EXE-CONF-040). | The first-level list shows at most the frequent types. |
| EXE-CTX-065 | SHOULD | Pinned configurations survive the temporary limit; a setting (`start.run.configurations.pinned`, default off) pins a configuration when it starts; `max.recent.run.configurations` (default 5) bounds the recents shown. | Case EXE-CTX-065 case 1. |
| EXE-CTX-066 | LATER | New-UI run widget variants (several slots, `Launch Several Configurations Simultaneously`, docked configurations, `Move Configuration to Top`) are not specified. | Not applicable. |

## 8. Open questions

- The exact enabled rules of `Show Running List` / `Manage Targets...` in the menu (observed only
  by running the IDE).
- Whether the alternative-location (`Run All`) flow has additional dialogs in the Services mode.
- Keymap differences for the Linux `Default for XWin` keymap beyond the debug keys listed in `05`.

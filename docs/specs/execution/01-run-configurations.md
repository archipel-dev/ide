# 01. Run configurations: model, storage, Edit Configurations, before-launch tasks

Status: specification. Requirement prefix `EXE-CONF`. Launching and the toolbar are in
`02-run-from-context.md`; the console in `03-run-tool-window-and-console.md`. Saving open
documents before a launch is specified in `docs/specs/platform/09-saving-and-autosave.md`
(`PLAT-SAVE-030`..`PLAT-SAVE-035`) and is only referenced here. Executable cases are in
`cases/01-run-configurations.cases.md`.

ic: `platform/execution/src/com/intellij/execution/RunManager.kt`,
`platform/execution-impl/src/com/intellij/execution/impl/{RunManagerImpl.kt,RunnerAndConfigurationSettingsImpl.kt,RunConfigurable.kt,SingleConfigurationConfigurable.java,RunConfigurationStorageUi.java,ProjectFileStorageSelector.kt,BeforeRunStepsPanel.java,RunConfigurationBeforeRunProvider.java,TemplateConfigurable.java}`,
`platform/execution-impl/src/com/intellij/execution/RunManagerConfig.java`,
`platform/execution-impl/src/com/intellij/execution/compound/CompoundRunConfiguration.kt`,
`platform/execution/resources/messages/ExecutionBundle.properties`,
`java/execution/impl/src/com/intellij/execution/application/ApplicationConfiguration.java`,
`java/execution/impl/src/com/intellij/compiler/options/CompileStepBeforeRun.java`,
`plugins/junit/src/com/intellij/execution/junit/JUnitConfiguration.java`,
`plugins/junit/resources/messages/JUnitBundle.properties`,
`platform/platform-impl/resources/intellij.platform.ide.impl.xml` (advanced settings).

## 1. Concepts

- A **configuration type** (`Application`, `JUnit`, `Remote JVM Debug`, ...) owns one or more
  **factories**. A factory creates configurations and a **template**. The template is a hidden
  configuration per factory whose field values seed every new configuration of that factory.
- A **configuration** has: a name (user-visible, may be empty only transiently), its type and
  factory ids, its type-specific fields, its **before-launch tasks**, per-executor runner
  settings, and the generic flags in 1.1.
- **Settings** wrap a configuration with storage-level facts: `temporary`, `shared` (stored in a
  project file), `folderName` (grouping inside the dialog tree), and the three flags in 1.1.
- Executors (Run, Debug, Coverage, ...) are independent of the configuration; a configuration is
  runnable with an executor when some runner supports the pair (`02`).

### 1.1 Generic flags and defaults

| Flag | UI label | Default | Persisted attribute |
|---|---|---|---|
| allow parallel run | `Allow multiple instances` (mnemonic `u`) | per factory, off for most types | `singleton` (written only when it differs from the factory default; value is the inverse) |
| edit before run | `Show the run/debug configuration settings before start` (Before launch section) | off | `editBeforeRun="true"` |
| activate tool window | `Open run/debug tool window when started` | on | `activateToolWindowBeforeRun="false"` only when off |
| focus tool window | `Focus run/debug tool window when started` | off | `focusToolWindowBeforeRun="true"` only when on |
| temporary | (not editable) | off | `temporary="true"` |
| folder | (set by drag or `Move into New Folder`) | none | `folderName` |

## 2. Requirements: identity, naming, ordering

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CONF-001 | MUST | A configuration's identity inside a project is the pair (type display name, configuration name), rendered `<Type display name>.<name>`. Two configurations of the same type cannot share a name; the identity changes when the name changes. | Create `Application` named `Main`, rename to `App`: lookup by `Application.Main` fails, by `Application.App` succeeds. Case EXE-CONF-001 case 1. |
| EXE-CONF-002 | MUST | The unique-name rule: if the wanted name is free among configurations of the type, keep it. Otherwise take the base name (a trailing ` (N)` removed, whitespace before it removed) and append ` (1)`, ` (2)`, ... to the first free value. A blank wanted name becomes `Unnamed`. | Existing `Main`, wanted `Main` gives `Main (1)`; existing `Main`, `Main (1)`, wanted `Main (1)` gives `Main (2)`; wanted `` gives `Unnamed`. Cases EXE-CONF-002 case 1..3. |
| EXE-CONF-003 | MUST | Names generated from a context (see `02` EXE-CTX) are flagged `nameIsGenerated`. While the flag holds, the name tracks the configuration's target (renaming the class renames the configuration). When the user types a different name the flag is cleared and the name is fixed. | Generated `Main`, user edits the field to `My app`: flag false, later main-class change leaves the name `My app`. |
| EXE-CONF-004 | MUST | Application-type generated name: the presentable (short) class name of the main class. If another configuration of the same type already carries that short name for a different main class, the fully qualified name is used instead. | Main class `p.Main` gives `Main`; with an existing `Main` for `q.Main`, gives `p.Main`. |
| EXE-CONF-005 | MUST | JUnit generated names: class kind gives the short class name; method kind gives `Class.method`, followed by ` with parameter "N"` when the run targets one parameter index; package kind gives the package name (or `<default>` for the default package); directory kind gives the directory's last segment; whole-project scope gives `Whole project`; package/directory with a module scope and empty name gives `All in <module>`; package with module gives `<package> in <module>`; pattern kind gives the first pattern's short name, or `<first> and N more` for several, or `Temp suite` when empty; category gives `@Category(<name>)` (`Invalid` for an empty name); tags give `Tags (<expr>)`; unique-id gives the ids joined by a space. A repeat mode other than once appends ` [N]` (count) or ` [*]` (until failure/success/stopped). | See cases EXE-CONF-005 case 1..4. |
| EXE-CONF-006 | MUST | Long names are shortened only for display: action texts shorten at 60 characters minus extra prefix length (minimum 10) followed by `...`; executor start texts (`Run 'name'`) trim the middle of names longer than 80 characters. The stored name is never shortened. | Name of 100 chars: menu shows an 80-char middle-trimmed text, dialog shows the full name. |
| EXE-CONF-007 | MUST | In every list (toolbar popup, dialog tree) configurations of one type group under the type; inside a type, folders first, then permanent configurations sorted by name (case-insensitive), then temporary ones. Template entries sort before all others. | Types `Application` with `B`, `A` (permanent) and `T` (temporary): order `A`, `B`, `T`. |

## 3. Requirements: temporary and permanent

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CONF-010 | MUST | Configurations created implicitly (from context, see `02`) are **temporary**: shown with a dimmed (faded) icon/name in the toolbar popup and in the dialog tree below permanent ones. Configurations created through the dialog are permanent. | Gutter-run `Main`: `Main` appears temporary (case EXE-CONF-010 case 1). |
| EXE-CONF-011 | MUST | At most N temporary configurations are kept, N = advanced setting `temporary.configurations.limit`, default 5, minimum 0. Pinned configurations do not count. When a new temporary one is added over the limit, the oldest temporaries not among the N most recently *used* ones are deleted. Usage order is refreshed by every start. | Limit 2; run temporary A, B, C in that order: A is deleted, B and C remain. Case EXE-CONF-011 case 1. |
| EXE-CONF-012 | MUST | A temporary configuration becomes permanent (`Save Configuration`) by: the action `Save Configuration` (row text `Save ''{0}'' Configuration` in the dialog toolbar, `Save Configuration` in the popup), editing and applying it in the Edit Configurations dialog, or sharing it (`Store as project file`). Making permanent removes it from the recent-temporary list and clears `temporary`. | After save, restart-less reload keeps the configuration and `temporary` attribute is gone. |
| EXE-CONF-013 | MUST | Temporary configurations are stored with the workspace, restored on project reopen, and the list of recently used temporaries is persisted (`recent_temporary`). | Reopen the project: temporaries and their recency order are unchanged. |
| EXE-CONF-014 | SHOULD | Dropping a configuration between the permanent and temporary groups in the dialog tree is refused: a permanent cannot be dropped below temporaries, a temporary cannot be dropped above permanents. | Drag `Main` (permanent) below `T` (temporary): drop indicator not shown. |

## 4. Requirements: storage and format

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CONF-020 | MUST | The default storage of a configuration is the project workspace file (`.idea/workspace.xml`, component `RunManager`, one `configuration` element per item). `RunManager` state also records `selected` (unique id of the selected configuration, written only when there is more than one configuration), the order list, and `recent_temporary`. | A saved project with two configurations has `selected="..."` on the state element; with one it has none. |
| EXE-CONF-021 | MUST | With `Store as project file` on, the configuration is written to its own file `<name>.xml` under `.idea/runConfigurations/` (a shared configuration). Turning it off moves it back to the workspace. A temporary configuration is made permanent first. | Case EXE-CONF-021 case 1. |
| EXE-CONF-022 | SHOULD | `Manage File Location` allows storing the file in an arbitrary folder inside the project: validation messages are `Path not specified`, `Folder path expected`, `Folder must be within the project`, `Folder is excluded. Select a folder within the project`, and `Can't store inside .idea folder (except .idea{0}runConfigurations)` (with the platform separator). Label `Store configuration file in:`; button `Done`. | Typing `.idea/foo` shows the last message; `Done` disabled until valid. |
| EXE-CONF-023 | MUST | Element form of one configuration: root `configuration` with attributes `name`, `type` (type id), `factoryName` (omitted for single-factory simple types), optional `folderName`, `temporary`, `nameIsGenerated`, `editBeforeRun`, `activateToolWindowBeforeRun`, `focusToolWindowBeforeRun`, `singleton`; children: the type-specific `option` elements, `method` (before-launch tasks), optional per-runner settings. Default-valued options are omitted. | Round trip of an Application configuration reproduces the same element. |
| EXE-CONF-024 | MUST | Paths inside stored configurations are collapsed with the standard macros (`$PROJECT_DIR$`, `$MODULE_DIR$` for module-based ones, `$USER_HOME$`); the project's parent directory is written as `$PROJECT_DIR$/..`. Reading expands them. | Working dir `/home/u/p/sub` for project `/home/u/p` is stored as `$PROJECT_DIR$/sub`. |
| EXE-CONF-025 | MUST | A configuration whose type or factory is not available (plugin missing) is kept verbatim as an *unknown* configuration: listed under `Unknown`, description `Configuration, which cannot be loaded due to some reasons`, cannot be run or edited (`This configuration cannot be edited`), written back unchanged. Run attempt error: `Broken configuration due to unavailable plugin or invalid configuration data.` | Open a project with a `Foo` type element: entry listed; saving keeps the element. |
| EXE-CONF-026 | MUST | Configurations in project files (shared) with a name already used by a workspace configuration of the same type are loaded with a unique name; a shared configuration that includes (before launch `Run another configuration`) a non-shared one is made non-shared with the message `{0} configuration was made "not shared" since included Run Configuration ''{1}'' is not shared.` | Case EXE-CONF-026 case 1. |

## 5. Requirements: templates

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CONF-030 | MUST | Every factory has a template. New configurations start as a copy of it (all fields, before-launch tasks, runner settings), then the generated or entered name is applied. | Template JUnit has VM options `-ea -Xmx1g`; new JUnit config has the same VM options. |
| EXE-CONF-031 | MUST | `Edit Configuration Templates...` (in the dialog toolbar: `Edit Templates`) shows the template list; the disclaimer text is `Templates are used to create new configurations. Changing a template does not affect the existing configurations.` Existing configurations are never modified by a template change. | Change template VM options; existing configuration keeps its own. |
| EXE-CONF-032 | MUST | A configuration is stored as a difference to its template: a value equal to the template's default is still stored when the template later changes (the stored value wins; only absent values follow the template at load). | Stored element lacking `vmParameters` loads with the template's current value. |
| EXE-CONF-033 | SHOULD | `Run Configuration Templates for New Projects...` (File | Other Settings, and the welcome screen Configure menu) edits templates applied to projects created afterwards; page title `Run/Debug Configuration Templates`. | Opening it with no project shows the same page. |
| EXE-CONF-034 | MUST | Template names in internal lists are rendered `<template> of {0}`. They never appear in run popups. | Popup does not list templates. |

## 6. Requirements: Edit Configurations dialog

Dialog title `Run/Debug Configurations`; action `Edit Configurations...` (Run menu; id
`editRunConfigurations`; no default shortcut); toolbar combo `Edit Configurations...` entry.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CONF-040 | MUST | Left side: tree of types (nodes with their configurations beneath, folders, and the last node group `Templates` only through `Edit configuration templates...`). Toolbar: `Add New Configuration` (+; popup with all types, short list first then `{0} more items...`, `Other`), `Remove Configuration`, copy, `Save ''{0}'' Configuration` (visible for temporary ones), `Move Up`, `Move Down`, `Create New Folder`, `Sort Configurations` (alphabetical), `Edit Templates`. Tree accessible name `Run Configurations`. | Dialog opens with the tree focused; the empty state text is `No run configurations added.` with `Add new...`. |
| EXE-CONF-041 | MUST | Right side: the settings editor of the selected configuration: `Name:` text field; `Allow multiple instances` and `Store as project file` check boxes at the top right; the type's own fields; a `Modify options` drop-down listing optional fields (a field set to its default is hidden and its first hiding shows the hint `'{0}' is currently set to the default value. Options set to their default value are hidden from the run configuration. To configure this option later use the 'Modify options' menu.`); a `Before launch` section (section 8); `Run on:` target combo with `Manage targets...` and the comment `Run configurations may be executed locally or on a target: for example, in a Docker Container or on a remote host using SSH.` | Dialog for `Application` shows `Build and run` row (`Java ... main class ...`). |
| EXE-CONF-042 | MUST | Buttons `OK`, `Cancel`, `Apply` (mnemonic `A`), `Help`. `Apply` writes all changed configurations; `OK` applies then closes; `Cancel` discards. Settings errors found at apply time show the dialog title `Invalid Data` with the message. | Editing a name then Cancel leaves the old name. |
| EXE-CONF-043 | MUST | Names are validated: empty name is refused; a name already used inside the type shows `{0} with name ''{1}'' already exists` where `{0}` is the type display name; folder names must be non-empty (`Folder name shouldn't be empty`) and unique (`Folders name ''{0}'' is duplicated`). Invalid input shows the message under the form and disables `OK`/`Apply`. | Case EXE-CONF-043 case 1. |
| EXE-CONF-044 | MUST | Remove with the `Delete` key or toolbar; removal of folders removes only the folder (items move up); the dialog asks no confirmation for configurations but the toolbar popup `Delete` asks (EXE-CTX-050). | Remove `A`: gone after OK. |
| EXE-CONF-045 | SHOULD | Folders: `Create New Folder` prompts `Folder name:`; note `Note: empty folders won't be saved`; moving items by drag and drop. | Create folder `tools`, drag `A` in: stored `folderName="tools"`. |
| EXE-CONF-046 | MUST | Settings errors are classified: *error* (the configuration cannot start; shown red, prevents `Run`) and *warning* (shown, but the start continues). A runtime check is executed in the background against the current project state each time the dialog selects a configuration. | Application with empty main class: error `No main class specified`; main class without main method: warning `Main method not found in class X`. |

## 7. Requirements: types and their editors

Fields are listed with their UI label, hint text and default. Type-specific validation messages are
facts and are quoted.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CONF-050 | MUST | **Application** (id `Application`, tag `java`, description `Java application configuration`). Fields: `JRE` (combo, `Default` / `Bundled` / installed JDKs / `Select alternative JRE...`), `Main class` (hint `The fully qualified name of the class that contains the main method. Example: com.package.MainClass`; chooser `Choose Main Class`), `Use classpath of module` (hint `Select the module whose classpath should be used to run the application`), `VM options` (hint `VM options. CLI arguments to the 'Java' command. Example: -ea -Xmx2048m`), `Program arguments` (hint `CLI arguments to your application`), `Working directory` (default the project root, shown as `$MODULE_WORKING_DIR$`-resolved path; a value equal to the project base path is not stored), `Environment variables` (separator `;`, hint `Separate variables with semicolon: VAR=value; VAR1=value1`), `Paths to ".env" files`, check boxes `Add dependencies with "provided" scope to classpath`, `Use module path`, `Redirect input from:`, `Shorten command line:`, logs and `Save console output to file`. | Dialog fields appear in the listed order after `Modify options` choices. |
| EXE-CONF-051 | MUST | Application validity checks in this order and messages: alternative JRE valid (`'{0}' is not a valid JRE home`, `No JDK specified`), module and class exist (`No module defined`, `Module ''{0}'' does not exist`, `Class ''{0}'' not found in module ''{1}''`, `No main class specified`), has a main method (warning `Main method not found in class {0}`), working directory exists (`Working directory ''{0}'' doesn't exist`), env files valid. Implicit class (Java unnamed class) configurations use the file name as the main class (`File name` placeholder, hint `The name of the file that contains the method. Example: Main`). | Cases EXE-CONF-051 case 1..3. |
| EXE-CONF-052 | MUST | **JUnit** (id `JUnit`, description `JUnit test configuration`). `Test kind` (hint `The type of resource to search for tests`) one of `All in package`, `All in directory`, `Pattern`, `Class`, `Method`, `Category`, `UniqueId`, `Tags` (plus `Through source location` and `Changes` internal kinds). Defaults: kind `Class`, VM options `-ea`, working directory `$MODULE_WORKING_DIR$`, pass parent environment on, search scope `In single module`, fork mode `None`, repeat `Once`. Field hints: pattern `A regexp to match the names of classes that contain tests`, class `The fully qualified name of a class that contains tests`, method `The fully qualified name of a test method`, package `The name of the package that contains tests`. Search scope values: `In whole project`, `In single module`, `Across module dependencies`. Repeat modes: `Once`, `N times`, `Until failure`, `Until success`, `Until stopped`. Fork modes: `None`, `Method`, `Class`, `Repeat`. | Dialog with kind switched to `Method` shows `Class` and `Method` fields. |
| EXE-CONF-053 | MUST | JUnit validation messages: `No test class specified`, `Method name not specified`, `Test method ''{0}'' doesn't exist`, `Class ''{0}'' is not a test`, `Package ''{0}'' does not exist`, `Directory is not specified`, `Directory ''{0}'' is not found`, `No pattern selected`, `Category is not specified`, `Tags are not specified`, `No unique id specified`, `No module selected`, `JUnit not found in module ''{0}''`, `Running tests is disabled during index update`. | Case EXE-CONF-053 case 1. |
| EXE-CONF-054 | SHOULD | **TestNG**: kinds package, class, method, pattern, group, suite, source-location; same VM options default `-ea`, and the same unique name/generation rules; TestNG is a bundled plugin and is `SHOULD`. | TestNG type listed in `Add New Configuration`. |
| EXE-CONF-055 | MUST | **Remote JVM Debug** (`Remote`, display name `Remote JVM Debug`): `Debugger mode` (`Attach to remote JVM` / `Listen to remote JVM`), `Transport` (`Socket`, `Shared memory` only on Windows), `Host`, `Port` (validation `Port value should be a number between 0 and 65535`), `Use module classpath`, and the read-only `Command line arguments for remote JVM` block selectable by `JVM arguments format` (`JDK 9 or later`, `JDK 5 - 8`, `JDK 1.4.x`, `JDK 1.3.x or earlier`) with the text `Copy and paste the arguments to the command line when JVM is started`. | Default is attach, `localhost:5005`. |
| EXE-CONF-056 | SHOULD | **JAR Application** (id `JarApplication`, description `Configuration to run a JAR file using the 'java-jar' command`): `Path to JAR`, VM/program options, working directory, JRE. Error text `Neither main class nor JAR path is specified`. | Empty path gives that error. |
| EXE-CONF-057 | SHOULD | **Compound** (id from the platform, display name `Compound`, description `It runs batch of run configurations at once`): an ordered list of other configurations; starting it starts each in the same group id with the chosen executor; a cyclic inclusion is refused with `'{0}' has already configured to be launched before {1}.` / `Such cyclic dependencies are not allowed.`. | Adding itself is refused. |
| EXE-CONF-058 | SHOULD | **Shell Script** (`Shell Script`): `Script path` or `Script text`, interpreter path and options, script options, working directory, environment, `Execute in the terminal` option. Default: none. | Listed type; Run executes the script and streams the output to the console (03). |
| EXE-CONF-059 | SHOULD | **Gradle** (`Gradle`): `Gradle project`, `Tasks`, `Arguments`, `Environment variables`, `Java options`, shown with the Gradle tool-window integration. **Maven** (`Maven`): `Working directory`, `Command line` (goals), `Profiles`, `Resolve Workspace artifacts` and the Maven runner settings. Details belong to the build specs; here only the type listings and the before-launch interplay are required. | Both types appear in `Add New Configuration` when their plugins are enabled. |
| EXE-CONF-060 | LATER | Java Scratch, DevKit Plugin, Multi-Launch, XSLT types listed in the inventory (`docs/specs/inventory/run-debug.md`): present, not specified. | Not applicable. |

## 8. Requirements: before-launch tasks

Section label `Before launch` (tab/section title `Before launch` with a count suffix `: 1 task` / `: {0} tasks` when collapsed);
empty text `There are no tasks to run before launch`; add button `Add task` (menu `Add New Task`);
tooltip `Double click to edit settings.<br>Drag and drop to reorder.`

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CONF-070 | MUST | A configuration carries an ordered list of before-launch tasks, each with a provider id and an enabled flag. Providers: `Build` (id `Make`), `Build Project`, `Build, no error check`, `Run another configuration` (shown as `Run '{0}'`), `Run External tool`, `Run Gradle task`/`Run Maven Goal`, `Launch Web Browser`, `Build Artifacts`, `Run Ant target`. Unknown providers are kept as `Unknown Task`. | Case EXE-CONF-070 case 1. |
| EXE-CONF-071 | MUST | Tasks execute sequentially in list order before the process starts; if any task fails, is cancelled or reports an error, the launch is cancelled and no process starts. Only enabled tasks run. | Task 2 fails: task 3 and the process do not run. |
| EXE-CONF-072 | MUST | `Build`: enabled by default for JVM configurations that support compile-before-launch (Application, JUnit, TestNG, Kotlin, ...), absent for `Remote JVM Debug`. It builds the modules the configuration uses (module scope including test sources when the target class is a test), with errors failing the task; `Build, no error check` proceeds despite errors; `Build Project` builds all modules. A configuration may exclude the task (`Do not build before run`, hint `Run the application without launching the build process`). Before the build, all unsaved documents are saved (see `PLAT-SAVE-030`..`PLAT-SAVE-035`) and pending VFS updates are flushed. | Run with a compile error: no process starts and the Build messages are shown. |
| EXE-CONF-073 | MUST | `Run another configuration` picks a configuration (title `Run Another Configuration`); the included configuration is started with the same executor (falls back to `Run` when the executor is unsupported) and awaited to completion before the next task. A missing target is reported as `{0} Run Configuration ''{1}'' not found, removed from the list.` and removed. Error if it cannot start: `cannot start before run task ''{0}''.` | Include a configuration that fails: parent does not start. |
| EXE-CONF-074 | MUST | Before-launch options (checkboxes in the section): `Show the run/debug configuration settings before start` (opens the dialog titled `Edit Configuration` on each start; Cancel aborts the launch), `Open run/debug tool window when started`, `Focus run/debug tool window when started`. Defaults in 1.1. | Option on: pressing Run shows the dialog, `Run` button label `Run` (or `Debug`). |
| EXE-CONF-075 | SHOULD | Task enabled flag toggled by the checkbox in the section; tasks reordered by drag. Persisted in the `method` element (`option name=... enabled`). | Reorder persists. |
| EXE-CONF-076 | MUST | Launch is refused while the project is being indexed unless the configuration type is dumb aware: the start waits until the project is ready (queued) when the type is not dumb aware; otherwise `Cannot start while project analysis is in progress.` is shown for a type that fails with an index error. Running tests: `Exact test search is impossible without a full index. You can either:` with the choices `retry after project analysis finishes` / `or use available partial index results to run tests (possibly incomplete or outdated!)`; button `Run with Partial Index`. | Application type starts during indexing; JUnit package kind prompts. |

## 9. Requirements: environment, working directory, macros

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| EXE-CONF-080 | MUST | Environment variables dialog (title `Environment Variables`): table of name/value rows, `Include system environment variables` check box (default on), and `Paste` of `NAME=value` lines; `.env` files (`Paths to ".env" files`, file chooser tooltip `Choose ".env" file`) are parsed with `KEY=VALUE` lines, `#` comments, quotes. Explicit variables override env-file values which override system ones. | Variable `A=1` in the table and `A=2` in the file: process sees `A=1`. |
| EXE-CONF-081 | MUST | Working directory default is the module's directory (`$MODULE_WORKING_DIR$`), and for Application the project directory; the field supports the macros dialog (`Insert Macros...`) with `$ProjectFileDir$`, `$ModuleFileDir$`, `$FilePath$`, `$FileDir$`, `$FileName$`, `$FileDirRelativeToProjectRoot$`, `$FilePathRelativeToProjectRoot$`, `$SourcepathEntry$`, `$Prompt$`, `$ClasspathEntry$`, `$FileClass$`, `$FileExt$`, `$FileNameWithoutExtension$`, `$LineNumber$`, `$ColumnNumber$`, `$SelectedText$`, `$ProjectName$`, `$Clipboard$`, `$Date$`, `$Time$`, `$JDKPath$`. Macros are expanded when the configuration starts; a macro that cannot be expanded (no editor file) fails with a message naming the macro. | `-Dfile=$FileName$` with `Main.java` active expands to `-Dfile=Main.java`. |
| EXE-CONF-082 | MUST | VM options and program arguments are split on whitespace with double-quote grouping and backslash escapes; empty fields add nothing. Program arguments may contain macros; VM options too. | `-Dgreeting="hello world" a b` yields three program arguments and one VM argument with a space. |
| EXE-CONF-083 | SHOULD | `Shorten command line:` options `none` (default for old JDK), `JAR manifest`, `classpath file`, and `@argfile (Java 9+)`. When the command line is too long for the OS the start fails with `Error running <a href="edit">{0}</a>. Command line is too long.` plus the suitable hint (`Shorten the command line and rerun.` for Java 9+; or the manifest/classfile hint for Java 8). | Link `args` sets `@argfile` and reruns. |
| EXE-CONF-084 | SHOULD | `Redirect input from:` a file path; the file content is the process standard input. | Reading stdin yields the file's bytes. |
| EXE-CONF-085 | MUST | The `JRE` field resolves in order: explicit alternative JRE (checkbox state stored as `alternativeJrePathEnabled`), otherwise the project JDK / module JDK. Invalid path `'{0}' is not a valid JRE home`; downloading JDK `'{0}' is downloading`. | Valid directory with `bin/java` accepted. |
| EXE-CONF-086 | MUST | Class path: the classpath of the chosen module (runtime classpath, plus test classpath for tests); entries in VM options `-cp`/`-classpath` take precedence (hint text `the classpath specified in the VM options takes precedence over this one`). `Modify classpath` allows including/excluding paths (`Include...`, `Exclude...`). | Excluding a library removes it from the command line. |
| EXE-CONF-087 | SHOULD | Run targets (`Run on:` combo, `Local machine`, saved targets, `Manage targets...`) are LATER for Docker/SSH implementations; the combo shows only `Local machine` when no target type is installed. | Combo contains the single item. |
| EXE-CONF-088 | LATER | Startup tasks (settings page `Startup Tasks`: `Run tasks and tools via run configurations`, `To be started on project opening:`), Services tool window grouping of run configurations, and the run widget settings are not specified. | Not applicable. |

## 10. Open questions

- The exact set of types in the default `Add New Configuration` list depends on bundled plugins; the
  Archipel set is Application, JUnit, Remote JVM Debug, Compound, Shell Script, Gradle and Maven
  initially.
- Whether the `Modify options` drop-down content order is fixed per type (observed per type; not in
  the bundle).
- Which exact message the Application check emits for a module without JDK (`No JDK for module ''{0}''`
  is in the bundle; it is raised by module checks).

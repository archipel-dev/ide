# Workbench spec 06: Welcome screen, New Project, Open, trust

Status: specification (clean-room). ID prefix `WB-WEL`. Priorities `MUST` / `SHOULD` / `LATER` as in `docs/specs/README.md`.
Executable cases: `cases/06-welcome-new-project-open.cases.md`.

Scope: the Welcome screen and its recent-projects list with actions, project entry points (New Project, Open, Open recent, Get from Version Control, project from existing sources),
the New Project wizard (generators, fields, defaults, validation), the open-in-window decision, the project trust dialog and safe mode.

Sources studied: `ic: platform/platform-impl/src/com/intellij/ide/wizard/` (`NewProjectWizardBaseStep.kt`, `GitNewProjectWizardStep.kt`, `NewProjectWizardLanguageStep.kt`, `language/EmptyProjectGeneratorNewProjectWizard.kt`),
`ic: java/idea-ui/src/com/intellij/ide/projectWizard/generators/` (`JavaNewProjectWizard.kt`, `IntelliJJavaNewProjectWizard.kt`, `IntelliJNewProjectWizardStep.kt`, `BuildSystemJavaNewProjectWizard.kt`),
`ic: platform/external-system-impl/src/com/intellij/openapi/externalSystem/service/project/wizard/MavenizedNewProjectWizardStep.kt`, `ic: plugins/gradle/java/src/service/project/wizard/GradleJavaNewProjectWizard.kt`,
`ic: platform/non-modal-welcome-screen/resources/messages/NonModalWelcomeScreenBundle.properties`, `ic: platform/platform-api/resources/messages/IdeBundle.properties`, `UIBundle.properties`, `ActionsBundle.properties`,
`ic: java/idea-ui/resources/messages/JavaUiBundle.properties`, `ExternalSystemBundle.properties`, `GradleBundle.properties`, inventory `welcome-project.md`.

## 1. Welcome screen

The Welcome screen is the frame shown when no project is open. Left navigation tabs (reference `welcome.screen.*`): **Projects**, **Customize**, **Plugins**, **Learn** (Learn only with learning content).
The Projects tab shows the **recent projects list** with a search field (hint **"Search projects"**) and the actions **New Project**, **Open**, **Get from VCS** ("Get from Version Control"). A status/footer shows the version ("Version {x}") and
a **More Actions** / Options menu (Configure: Settings, Plugins, Import Settings, ... ; Get Help). The frame accepts **file drop** (hint "Drop files here to open") and opens a dropped folder or project file.

Empty state (no recent project): text "No recent projects" or the two comment lines "Create a new project to start from scratch." and "Open existing project from disk or version control." with links to New Project / Open / Get from VCS.

### 1.1 Recent projects list

Entries (one per project path), most recently opened first. A project may be placed into a **project group** ("New Project Group", "Edit...") that groups entries with an expandable header. Each entry shows the project **name**, the **path** (home shortened to `~`),
and the **current branch** (from VCS, accessible name "Branch {x}") when known; a project whose path no longer exists shows its name followed by "(unavailable)" and is greyed.
Hovering an entry shows a "Version Control" action when the project has a VCS repository (tooltip "Version Control"; popup content "No VCS repository in the project" otherwise). A currently running background activity (opening) shows a cancel action "Cancel".

Context menu of an entry (group `WelcomeScreenRecentProjectActionGroup`): **Open Selected** (Enter), **Remove from Recent Projects...** (Delete, shown as `Remove from Recent Projects...`), **Copy Path** (Ctrl+Shift+C = `CopyPaths`), **Show in File Manager** ("Reveal" action),
**Set Custom Project Icon...** (opens a chooser "Choose SVG file..." with "The SVG file will be saved to the .idea folder. To share the project icon, add it to the repository." and "Reset"), **New Project Group**, **Edit...** (rename/assign group).
Remove confirmation: dialog titled "Remove Recent Project" (plural "Remove Recent Projects") with the message `Remove '{path-or-name}' from recent projects list?` (several: "Remove projects from recent projects list?") and the buttons "Remove From List" and Cancel.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-WEL-001 | MUST | The Welcome screen has the three entry actions **New Project**, **Open**, **Get from VCS** and the recent projects list; the tab names are Projects, Customize, Plugins. | Start Archipel without a project: these controls exist. |
| WB-WEL-002 | MUST | The recent list is ordered by last-opened time, newest first, each opened project moved to the top; the list persists across restarts; duplicates by canonical path are merged. | Open A, then B: list is B, A. Reopen A: list is A, B. |
| WB-WEL-003 | MUST | Enter or double-click on an entry opens that project (WB-WEL-030 rules apply). Up/Down moves selection; typing filters via the search field. | Select entry, Enter: project opens in this window. |
| WB-WEL-004 | MUST | The search field filters entries by name or path substring, case-insensitively; empty text restores all. The field's empty text is "Search projects". | Type "dem": only entries with "dem" in name or path. |
| WB-WEL-005 | MUST | Delete (or the menu entry) removes the selected entries from the list **after confirmation** as above; the project files are untouched; Cancel keeps them. | Delete, confirm: entry gone, directory intact. |
| WB-WEL-006 | MUST | "(unavailable)" is shown for a missing path; opening it shows the error "Cannot Open Project" with the file-not-found text and offers removal. | Delete the directory on disk: entry greyed with "(unavailable)". |
| WB-WEL-007 | SHOULD | Copy Path puts the absolute path on the clipboard; Show in File Manager reveals it. | Copy Path then paste: absolute path. |
| WB-WEL-008 | SHOULD | Project groups: "New Project Group" asks for a name ("Create New Project Group", prompt "Project group name"); entries can be added to a group via "Edit...". | Create group "Work", add A: header "Work" with A below. |
| WB-WEL-009 | SHOULD | Custom project icon from an SVG file stored in the project's config directory; "Reset" removes it. | Set icon: the entry shows it. |
| WB-WEL-010 | MUST | The count shown is bounded by a limit (default 50 entries; oldest drop off); manage via File > Open Recent > **Manage Projects...** (`ManageRecentProjects`). | After opening 51 distinct projects the oldest is dropped (limit, see open question 1). |

## 2. Entry points

| Action (id) | Text | Where | Shortcut |
|---|---|---|---|
| `NewProject` | Project... (menu File > New > Project...); project widget: "New Project..." | File menu, Welcome | none |
| `WelcomeScreen.CreateNewProject`, `WelcomeScreen.DefaultNewProjectAction` | New Project | Welcome | none |
| `NewDirectoryProject` | New Project... (Welcome: "Project...") | File, Welcome | none |
| `OpenFile` / `WelcomeScreen.OpenDirectoryProject` / `WelcomeScreen.OpenProject` | Open... / Open / "Open or Import" | File, Welcome | none |
| `ReopenProject`, `$LRU` (File > Open Recent) | Reopen Project, Manage Projects... | File | none |
| `Vcs.VcsClone` | Get from Version Control... (Welcome: "Get from Version Control"; project widget: "Clone Repository...") | File, Welcome | none |
| `ImportProject` | Project from Existing Sources... | File > New | none |
| `ImportModule` | Module from Existing Sources... | LATER | |
| `CloseProject` | Close Project | File | none |
| `CloseAllProjects`, `CloseOtherProjects` | Close All Projects / Close Other Projects | File | none |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-WEL-020 | MUST | File > New > Project..., Welcome New Project and the project widget's "New Project..." open the same New Project wizard. | Each entry shows the dialog titled "New Project". |
| WB-WEL-021 | MUST | File > Open... and Welcome Open show the file chooser titled **"Open File or Project"** (`title.open.file.or.project`; older text "Open Project") filtering to directories and recognized project files (see 4). | Open: chooser title as above. |
| WB-WEL-022 | MUST | File > Open Recent lists recent projects (Reopen), then "Manage Projects..." (see open question 2); choosing one opens it. | Open Recent submenu lists the last opened projects. |
| WB-WEL-023 | MUST | **Close Project** closes the project and shows the Welcome screen when it was the only project; All/Other variants close the respective windows. | Close the only project: Welcome screen. |
| WB-WEL-024 | SHOULD | `ImportProject` "Project from Existing Sources..." creates a project from a directory without project files (see 4.2). | n/a |

## 3. New Project wizard

The wizard is a modal dialog titled **"New Project"** with buttons **Create** and Cancel. Layout: when several **generators** exist, a list of them is on the left; the right side is the form of the selected generator. Generators for the Community product:
**Java**, **Kotlin**, **Groovy** (language generators, in that order, ordinal 0 first), **Empty Project** (description "A basic project with free structure."), plus plugin generators. Archipel MUST ship **Java**, **Kotlin**, **Empty Project**; Groovy LATER.

Common fields (top to bottom), values in parentheses are defaults:

| Field | Control | Default | Rules |
|---|---|---|---|
| **Name:** | text | `untitled`; if `{location}/untitled` already exists, the first free of `untitled1`, `untitled2`, ... (also not a module name already used) | non-empty, valid module name, no existing project at `{location}/{name}` |
| **Location:** | text + folder browse | the IDE's default projects directory (reference: `~/IdeaProjects`); for a module in an open project: the parent of the project directory | non-empty, must be a valid directory path |
| comment under Location | label | `Project will be created in: {location}/{name}` (long paths shortened with an ellipsis in the middle) | updates live |
| **Language:** | segmented (Java / Kotlin / Groovy) | the generator's language | switches the generator |
| **Build system:** | segmented | **IntelliJ**; order IntelliJ, Maven, Gradle (ordinals 0, then Maven, then 200) | switching shows that system's extra fields |
| **JDK:** | combo | no JDK selected ("NoJdk") until detected; offers detected JDKs and "Download JDK..." | the choice is remembered |
| **Add sample code** | checkbox | **on** (value remembered across wizard runs) | generates `src/Main` with a hello-world |
| **Generate code with onboarding tips** | checkbox (Java sample) | see open question 3 | sample code variant |
| **Create Git repository** | checkbox | **off** (value remembered), shown only for new projects when Git is available | runs `git init` after creation; failure: "Unable to create a Git repository" |
| **Advanced Settings** | collapsible group, collapsed | - | contains the three fields below |
| Module name: | text | the project name | non-empty; must not exist in the project: "Module '{0}' already exist in project. Please, specify another name." / "Enter a module name" |
| Content root: | text + browse (title "Module Content Root") | `{location}/{name}` | "Failed to set directory '{0}' as content root" if it cannot be created |
| Module file location: | text + browse (title "Module File") | equal to the content root | "Enter module file location" if empty |

Build-system specific fields:
* **Maven** (Advanced Settings): **Parent:** (`<none>`), **GroupId:** (default `org.example`, last used value remembered), **ArtifactId:** (= project name, kept in sync: changing the name changes ArtifactId and vice versa), **Version:** (`1.0-SNAPSHOT`). Hints: GroupId "Uniquely identifies your project, usually starts with a reversed company domain name."; ArtifactId "The name of the artifact within the group, usually a {x} name." Errors: "Specify GroupId", "Specify ArtifactId", "Specify Version", "Project name and artifactId must be the same", "Project with coordinates '{x}' already exists".
* **Gradle**: **Gradle DSL:** segmented **Kotlin** (default) / **Groovy**; plus GroupId, ArtifactId, Version as for Maven; sample code; Gradle JVM/distribution in Advanced Settings.
* **IntelliJ**: Module name, content root, module file in Advanced Settings (above).

Validation messages (shown inline, red = error blocks Create, amber = warning does not): "Directory {0} is not readable in this location", "Directory {0} is not writable in this location" (errors), "Directory {0} is not empty in this location" (warning, Create stays enabled), "{0} is not a directory in this location" (error),
"Project {0} already exists in this location" (error), "Module {0} already exists in this location", "Module '{0}' already exists", "Project path is empty", "Invalid project directory path".

On **Create**: (1) the directory `{location}/{name}` is created (error "Unable to create a project directory" -> dialog "New Project Wizard Error"); (2) project files are written: for IntelliJ build system `.idea` metadata and a module file, a `.gitignore` for the IDE's own files when creating a new project; for Maven `pom.xml`, Gradle `build.gradle(.kts)`, `settings.gradle(.kts)`; (3) sample code is generated: "Generating Project {0} Sources..." progress, error "Unable to generate project sources"; (4) Git init when checked; (5) the project opens, **in this window if no project is open, otherwise the "open in window" decision of 4.1 applies**; the new project is **trusted** automatically (no trust dialog) because the user created it in the IDE.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-WEL-040 | MUST | Dialog title "New Project", buttons Create / Cancel; Create is disabled while any error is shown. | Empty name: Create disabled. |
| WB-WEL-041 | MUST | Defaults of the fields as in the table, including the sequential name rule `untitled`, `untitled1`, `untitled2`, ... | `~/IdeaProjects/untitled` exists: Name `untitled1`. |
| WB-WEL-042 | MUST | The location comment `Project will be created in: {path}` updates as name or location change, with `{path}` = `{location}/{name}` (home shown as `~`). | Name "demo", location `~/work`: `Project will be created in: ~/work/demo`. |
| WB-WEL-043 | MUST | Validation of name and location: empty name or location is an error; an existing **non-empty** target directory is a **warning** ("Directory {0} is not empty in this location"); an existing **project** at the target is an error ("Project {0} already exists in this location"); a **file** at the target path is an error ("{0} is not a directory in this location"); a non-writable parent is an error. | Case WB-WEL-043 cases. |
| WB-WEL-044 | MUST | Name changes propagate to the dependent defaults while those fields have not been edited by the user: module name, content root (`{location}/{name}`), module file location, ArtifactId. Fields edited by the user keep their value. | Type name "demo": module name "demo". Edit module name to "app", then change name to "demo2": module name stays "app", content root becomes `{location}/demo2` unless edited. |
| WB-WEL-045 | MUST | Build system selection shows only that system's fields; the choice is remembered for the next wizard run. | Choose Maven, reopen the wizard: Maven selected. |
| WB-WEL-046 | MUST | Maven: ArtifactId and project name stay in sync both ways; if they differ at Create the error "Project name and artifactId must be the same" is shown. | Name "demo": ArtifactId "demo"; edit ArtifactId "x": name becomes "x". |
| WB-WEL-047 | MUST | Defaults: GroupId `org.example` (or the last used), Version `1.0-SNAPSHOT`, Gradle DSL Kotlin. | Fresh profile: values as listed. |
| WB-WEL-048 | MUST | On Create the project directory and files are created and the project opens; the new project is trusted. | Create "demo" with Maven: `pom.xml` exists, project window opens without a trust prompt. |
| WB-WEL-049 | MUST | The wizard performs no filesystem writes before Create; Cancel leaves the disk untouched. | Fill fields, Cancel: no directory created. |
| WB-WEL-050 | SHOULD | Remembered values (sample code, Git, JDK, build system, GroupId) persist per IDE (application settings). | Uncheck "Add sample code", reopen wizard: unchecked. |
| WB-WEL-051 | SHOULD | Empty Project generator shows only Name, Location and Create Git repository and a comment "A basic project with free structure.". | Choose Empty Project: three controls and the comment. |

## 4. Open

### 4.1 Opening a project in a window

When a project is opened while another project is open, a dialog **"Open Project"** asks where: message "Projects can either be opened in a new window, or replace the project in the current window, or be attached to the already opened projects. How would you like to open the project?" (the "or be attached" part appears
only if attaching is supported) with buttons **This Window**, **New Window**, **Attach**, and Cancel. A setting "Open project in: ..." (System Settings; values Ask / New window / This window) skips the dialog; default **Ask**; a "do not ask again" checkbox stores the choice. When no project is open the project simply opens.

### 4.2 What the chooser accepts

* a **directory containing project metadata** (`.idea`, `.archipel`): opened as that project;
* a **build file** (`pom.xml`, `build.gradle`, `build.gradle.kts`, `settings.gradle(.kts)`): opened as an imported project;
* a **project file** (`*.ipr`, `*.iml` module file): opened;
* **any other directory**: a new project is created over it (the directory content becomes the content root; the "directory is not empty" case of `NewDirectoryProject`: "The directory '{0}' is not empty. Do you want to create a project from existing sources?");
* **a single file**: opened in the editor of the **default/light mode** (light edit) or in the current project when one is open.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-WEL-030 | MUST | With another project open, opening a project asks "Open Project" with This Window / New Window / Attach, or follows the stored setting; This Window closes the current project first (saving, with the usual unsaved-changes checks). | Open B while A is open: dialog; choose New Window: two windows. |
| WB-WEL-031 | MUST | Opening a path already open focuses its window; opening the same project twice never creates two windows for one path. | Open A again: A's window is activated. |
| WB-WEL-032 | MUST | The chooser has the title "Open File or Project", shows hidden folders optionally (toggle "Show Hidden Files and Directories"), supports a path text field (Ctrl+P toggles "Show/Hide Path Input Field"), the toolbar Home (Ctrl+1), Project directory (Ctrl+2), Desktop (Ctrl+D), Parent (Backspace), Refresh (Ctrl+F5). | Press Ctrl+P: path field toggles. |
| WB-WEL-033 | MUST | Opening a missing or unreadable path shows the dialog **"Cannot Open Project"** (`title.cannot.open.project`) with the reason (for a missing file: `File {path} does not exist.`) and does not change the recent list. | Open a deleted dir: message, no list change. |
| WB-WEL-034 | MUST | A successfully opened project is added to the recent list (top) and set as last-opened project; with "Reopen projects on startup" on, the next start reopens the previously open projects. | Open A, quit, start: A is open again. |
| WB-WEL-035 | SHOULD | "Get from Version Control" opens the clone dialog (Version Control: URL, Directory, Clone); after clone the project opens through the same trust rules. | Clone a repo: project opens, trust dialog appears (4.3). |

### 4.3 Trust (safe mode)

A project opened from a location that is **not trusted** triggers the trust dialog before any project code or build scripts run. Trusted: locations listed in Settings | Appearance & Behavior | **Trusted Locations**, projects the user created, projects under a trusted parent folder.

Dialog **"Trust and Open Project '{name}'?"** (title `untrusted.project.open.dialog.title`): text "{IDE} provides features that may execute potentially malicious code from this folder. If you don't trust the source, preview the project in the safe mode to only browse its code.";
checkbox "Trust all projects in '{parent folder}' folder" (the parent path, home shortened, for example `~/IdeaProjects`); buttons **Trust Project** (default), **Preview in Safe Mode**, **Don't Open**. For the build-script variant (importing Maven/Gradle) the title is `Trust {0} Project?`
("Trust {tool} Project?") and the text "Loading, running, or building a {tool} project may execute potentially malicious code from its build scripts. If you don't trust the source, stay in the safe mode." with buttons **Trust Project** and **Stay in Safe Mode**.
**Safe mode** opens the project with limited functionality: configuration files of the project are not loaded, build tool import and VCS activation are disabled, run configurations cannot be run, and the editor shows the banner **"Safe mode, limited functionality. Trust the project to access full IDE functionality."**
with links **"Trust project..."** and **"Read more"**; version control actions are unavailable ("The project is in safe mode, so {IDE} has not activated version control for it. Trust the project to use version control actions."). For a single file opened from outside a trusted project: banner "Safe mode. Trust this file to access full IDE functionality." with "Trust file..." and dialog "Trust File '{name}'?" (buttons "Trust File"); navigating from an untrusted file asks "Navigate from Untrusted File?" ("'{0}' is opened in the safe mode. Navigation resolves references and can open other files." button "Navigate").

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| WB-WEL-060 | MUST | Opening an untrusted project shows the dialog with the three buttons and the folder checkbox; the answer is persisted: Trust Project trusts the project (and, with the checkbox, the parent folder); the choice is not asked again for that path. | Open `~/IdeaProjects/x` for the first time: dialog "Trust and Open Project 'x'?". After Trust Project: reopen: no dialog. |
| WB-WEL-061 | MUST | **Preview in Safe Mode** opens the project with no code execution (analysis of project sources continues, but build scripts, run, VCS and plugin hooks are off) and the banner of this section; **Don't Open** returns to the previous screen without opening. | Choose Preview in Safe Mode: project opens, banner visible; Run is disabled. |
| WB-WEL-062 | MUST | "Trust project..." in the banner (or Settings > Trusted Locations) trusts the project and reloads it without safe mode. | Click the link: dialog "Trust Project?"; confirm: banner disappears and the project reloads. |
| WB-WEL-063 | MUST | Trusted Locations page lists trusted paths; "+" adds via chooser "Select Trusted Location" (dialog "New Trusted Location"); "-" removes. Projects under a listed path are trusted. | Add `~/work`: projects under it open without a prompt. |
| WB-WEL-064 | MUST | Projects created through the New Project wizard and projects already trusted open without a prompt. | Create via wizard: no dialog. |
| WB-WEL-065 | SHOULD | The Windows Defender exclusion checkbox (`Add IDE and '{path}' folders to the Microsoft Defender exclusions list`, comment "This will improve IDE performance. You must have administrator rights to use it.") appears only on Windows. | Linux: checkbox absent. |

## 5. Inventory coverage (`welcome-project.md`)

Covered: `CloseProject`, `ImportProject`, `ManageRecentProjects`, `NewProject`, `WelcomeScreen.CreateNewProject`, `WelcomeScreen.DefaultNewProjectAction`, `WelcomeScreen.OpenDirectoryProject`, `WelcomeScreen.OpenProject`, `WelcomeScreen.OpenSelected`, `WelcomeScreen.RemoveSelected`,
`WelcomeScreen.CopyProjectPath`, `WelcomeScreen.RevealIn`, `WelcomeScreen.ChangeProjectIcon`, `WelcomeScreen.NewGroup`, `WelcomeScreen.EditGroup`, `WelcomeScreen.CancelRecentProjectTask`.
LATER (new non-modal "welcome project" experience): `NonModalWelcomeScreen.LeftTabActions.New.Action` (New), `WelcomeNewEmptyFile` (Text File, as Scratch File), `WelcomeMarkdownNewFile`, `welcome.screen.left.tab.new.empty.file` (File), `WelcomeSaveFileAction`, `WelcomeReopenProjectAction` ("Open Welcome Project"),
`WelcomeFileChangeLanguage`, `ShowNewUsersOnboardingDialog`, `WelcomeScreen.Branches`, `WelcomeScreen.DevelopPlugins` ("Plugin Development"). Not in the scope: Plugin Development.

## 6. Open questions

1. Recent-projects limit (default 50 was not read from the source).
2. Whether File > Open Recent has "Clear Recent Projects" in Archipel (the reference has an entry `ClearRecentProjects`? not in the inventory).
3. Default of "Generate code with onboarding tips" and its visibility rules.
4. The exact wording of the New Project dialog's error text for an empty name (the platform "non-empty" validator).
5. The location default `~/IdeaProjects`: the reference derives it from the last used directory or a setting; Archipel uses `~/ArchipelProjects`? to be decided by product.
6. Setting texts for "Open project in" (values Ask / New window / This window) were not read from the source; they follow the dialog's button labels.

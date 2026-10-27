# Cases for 06-welcome-new-project-open

`dialog:` lists visible fields as `Label: value`; `errors:` / `warnings:` list inline messages; `buttons:` marks disabled buttons as `[Create:disabled]`.
`~` is the user's home directory. Default projects directory in these cases: `~/IdeaProjects`.

### WB-WEL-041 case 1: default name and location
settings: default; `~/IdeaProjects` is empty
language: plain
```before
Welcome screen, no project open
```
keys: click New Project
```after
dialog "New Project": generator list: Java, Kotlin, Empty Project (Groovy later)
Name: untitled
Location: ~/IdeaProjects
comment: Project will be created in: ~/IdeaProjects/untitled
Language: Java   Build system: IntelliJ
Add sample code: checked
Create Git repository: unchecked
Advanced Settings: collapsed
buttons: [Create] [Cancel]
```

### WB-WEL-041 case 2: sequential default name
settings: `~/IdeaProjects/untitled` and `~/IdeaProjects/untitled1` exist
language: plain
```before
Welcome screen
```
keys: click New Project
```after
Name: untitled2
comment: Project will be created in: ~/IdeaProjects/untitled2
```

### WB-WEL-042 case 1: comment follows edits
settings: default
language: plain
```before
New Project dialog, Name: untitled, Location: ~/IdeaProjects
```
keys: set Name to "demo", set Location to "~/work"
```after
comment: Project will be created in: ~/work/demo
```

### WB-WEL-043 case 1: empty name
settings: default
language: plain
```before
New Project dialog
```
keys: clear the Name field
```after
error under Name (non-empty rule); buttons: [Create:disabled]
```

### WB-WEL-043 case 2: non-empty directory is a warning
settings: `~/IdeaProjects/demo` exists and contains a file
language: plain
```before
New Project dialog
```
keys: set Name to "demo"
```after
warnings: Directory ~/IdeaProjects/demo is not empty in this location
buttons: [Create] enabled
```

### WB-WEL-043 case 3: project already exists
settings: `~/IdeaProjects/demo/.idea` exists
language: plain
```before
New Project dialog
```
keys: set Name to "demo"
```after
errors: Project ~/IdeaProjects/demo already exists in this location
buttons: [Create:disabled]
```

### WB-WEL-043 case 4: a file is in the way
settings: `~/IdeaProjects/demo` is a regular file
language: plain
```before
New Project dialog
```
keys: set Name to "demo"
```after
errors: ~/IdeaProjects/demo is not a directory in this location
buttons: [Create:disabled]
```

### WB-WEL-043 case 5: read-only parent
settings: `~/IdeaProjects` is not writable
language: plain
```before
New Project dialog
```
keys: set Name to "demo"
```after
errors: Directory ~/IdeaProjects is not writable in this location
buttons: [Create:disabled]
```

### WB-WEL-044 case 1: dependent defaults
settings: Java, IntelliJ build system, Advanced Settings expanded
language: plain
```before
Name: untitled, Location: ~/IdeaProjects
Module name: untitled, Content root: ~/IdeaProjects/untitled, Module file location: ~/IdeaProjects/untitled
```
keys: set Name to "demo"
```after
Module name: demo
Content root: ~/IdeaProjects/demo
Module file location: ~/IdeaProjects/demo
```

### WB-WEL-044 case 2: user-edited field is kept
settings: Java, IntelliJ build system, Advanced Settings expanded
language: plain
```before
Name: demo, Module name: demo
```
keys: set Module name to "app", set Name to "demo2"
```after
Module name: app
Content root: ~/IdeaProjects/demo2
```

### WB-WEL-044 case 3: module name must be unique in the open project
settings: Java, creating a module in an open project that already has module "app"
language: plain
```before
dialog for a new module, Advanced Settings expanded
```
keys: set Module name to "app"
```after
error: Module 'app' already exist in project. Please, specify another name.   (module-name validation)
buttons: [Create:disabled]
```

### WB-WEL-046 case 1: Maven coordinates
settings: Build system Maven
language: plain
```before
New Project dialog, Build system: Maven, Advanced Settings expanded; Name: untitled
```
keys: (none)
```after
Parent: <none>
GroupId: org.example
ArtifactId: untitled
Version: 1.0-SNAPSHOT
```

### WB-WEL-046 case 2: artifactId and name stay in sync
settings: Build system Maven
language: plain
```before
Name: untitled, ArtifactId: untitled
```
keys: set ArtifactId to "billing"
```after
Name: billing
Location comment: Project will be created in: ~/IdeaProjects/billing
```

### WB-WEL-046 case 3: empty coordinates
settings: Build system Maven
language: plain
```before
Maven fields visible
```
keys: clear GroupId, press Create
```after
error: Specify GroupId; dialog stays open; nothing created on disk
```

### WB-WEL-047 case 1: Gradle defaults
settings: Build system Gradle
language: plain
```before
New Project dialog
```
keys: select Build system Gradle
```after
Gradle DSL: Kotlin (selected), Groovy
GroupId: org.example   ArtifactId: untitled   Version: 1.0-SNAPSHOT
```

### WB-WEL-048 case 1: Create writes files and opens the project
settings: Java, IntelliJ, Add sample code on, Git off, no project open
language: plain
```before
New Project dialog: Name demo, Location ~/IdeaProjects
```
keys: click Create
```after
directory ~/IdeaProjects/demo exists with: .idea/, demo.iml (module file), .gitignore, src/Main.java (sample code)
project "demo" opens in the current window; no trust dialog
```

### WB-WEL-048 case 2: Maven project
settings: Build system Maven
language: plain
```before
New Project dialog: Name demo, GroupId org.example
```
keys: click Create
```after
~/IdeaProjects/demo/pom.xml exists with groupId org.example, artifactId demo, version 1.0-SNAPSHOT
```

### WB-WEL-049 case 1: Cancel leaves no trace
settings: default
language: plain
```before
New Project dialog with Name "demo" filled in
```
keys: click Cancel
```after
~/IdeaProjects/demo does not exist
```

### WB-WEL-050 case 1: remembered values
settings: default
language: plain
```before
New Project dialog: uncheck "Add sample code", check "Create Git repository", choose Maven; create a project; open the wizard again
```
keys: click New Project
```after
Build system: Maven; Add sample code: unchecked; Create Git repository: checked
```

### WB-WEL-051 case 1: Empty Project generator
settings: default
language: plain
```before
New Project dialog
```
keys: select "Empty Project" in the generator list
```after
comment: A basic project with free structure.
fields: Name, Location, Create Git repository
no Language, Build system or JDK controls
```

### WB-WEL-002 case 1: recent list order
settings: default
language: plain
```before
recent: A, B (A most recent)
```
keys: open project B from the Welcome screen, close it
```after
recent: B, A
```

### WB-WEL-004 case 1: search field
settings: recent: demo (~/work/demo), shop (~/IdeaProjects/shop), api (~/work/api)
language: plain
```before
Welcome screen, search field empty text "Search projects"
```
keys: type "work"
```after
entries shown: demo, api (path match)
```

### WB-WEL-005 case 1: remove with confirmation
settings: recent: demo
language: plain
```before
Welcome screen, selection demo
```
keys: Delete
```after
dialog "Remove Recent Project": Remove 'demo' from recent projects list?   buttons: Remove From List, Cancel
after Remove From List: recent list empty; directory ~/work/demo still exists
```

### WB-WEL-006 case 1: unavailable project
settings: recent: demo; directory deleted on disk
language: plain
```before
Welcome screen
```
keys: (none)
```after
entry text: demo (unavailable), greyed
```

### WB-WEL-030 case 1: open in window decision
settings: project A open, "Open project in" = Ask
language: plain
```before
File > Open..., choose directory B (an existing project)
```
keys: confirm chooser
```after
dialog "Open Project": Projects can either be opened in a new window, or replace the project in the current window, or be attached to the already opened projects. How would you like to open the project?
buttons: This Window, New Window, Attach, Cancel
```

### WB-WEL-030 case 2: New Window
settings: as before
language: plain
```before
dialog "Open Project" shown for project B
```
keys: click New Window
```after
two windows: A and B; recent list: B, A
```

### WB-WEL-031 case 1: already open
settings: projects A and B open in two windows
language: plain
```before
window B active; File > Open... choose A
```
keys: confirm chooser
```after
window A is activated; no third window, no dialog
```

### WB-WEL-033 case 1: missing path
settings: path ~/gone does not exist
language: plain
```before
Open File or Project chooser, path field "~/gone"
```
keys: Enter
```after
dialog "Cannot Open Project": File ~/gone does not exist.   (no change to the recent list)
```

### WB-WEL-060 case 1: untrusted project
settings: ~/Downloads/lib never trusted; Trusted Locations empty
language: plain
```before
Open File or Project, choose ~/Downloads/lib (contains pom.xml)
```
keys: confirm chooser
```after
dialog "Trust and Open Project 'lib'?"
text: <IDE> provides features that may execute potentially malicious code from this folder. If you don't trust the source, preview the project in the safe mode to only browse its code.
checkbox: Trust all projects in '~/Downloads' folder (unchecked)
buttons: Trust Project, Preview in Safe Mode, Don't Open
```

### WB-WEL-060 case 2: Trust Project with folder checkbox
settings: as before
language: plain
```before
dialog open
```
keys: check "Trust all projects in '~/Downloads' folder", click Trust Project
```after
project opens normally; Settings > Trusted Locations contains ~/Downloads; opening ~/Downloads/other later shows no dialog
```

### WB-WEL-061 case 1: Preview in Safe Mode
settings: as before
language: plain
```before
dialog open
```
keys: click Preview in Safe Mode
```after
project opens; editor banner: Safe mode, limited functionality. Trust the project to access full IDE functionality.   links: Trust project..., Read more
Run and VCS actions are disabled; build-script import did not run
```

### WB-WEL-061 case 2: Don't Open
settings: as before
language: plain
```before
dialog open
```
keys: click Don't Open
```after
Welcome screen (or the previous window) unchanged; recent list unchanged
```

### WB-WEL-062 case 1: trust from the banner
settings: project open in safe mode
language: plain
```before
banner visible
```
keys: click "Trust project...", confirm "Trust Project"
```after
banner disappears; project is reloaded with full functionality; path now trusted
```

### WB-WEL-064 case 1: wizard-created project is trusted
settings: default
language: plain
```before
New Project dialog filled
```
keys: click Create
```after
project opens with no trust dialog and no banner
```

### WB-WEL-003 case 1: Enter opens
settings: recent: demo
language: plain
```before
Welcome screen, selection demo
```
keys: Enter
```after
project demo opens in this window
```

### WB-WEL-020 case 1: all entries open the same wizard
settings: default
language: plain
```before
Welcome screen
```
keys: New Project button; later File > New > Project...
```after
both show dialog "New Project"
```

### WB-WEL-021 case 1: chooser title
settings: default
language: plain
```before
Welcome screen
```
keys: click Open
```after
chooser titled "Open File or Project"
```

### WB-WEL-034 case 1: reopen on startup
settings: Reopen projects on startup on
language: plain
```before
project A open
```
keys: quit, start
```after
project A open
```

### WB-WEL-045 case 1: build system remembered
settings: default
language: plain
```before
wizard run with Maven chosen
```
keys: open the wizard again
```after
Build system: Maven
```

### WB-WEL-063 case 1: trusted location
settings: Trusted Locations empty
language: plain
```before
Settings > Trusted Locations
```
keys: add ~/work, OK, open ~/work/x
```after
no trust dialog
```

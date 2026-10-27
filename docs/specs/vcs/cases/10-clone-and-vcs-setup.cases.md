# Cases for 10 (clone, setup, ignore)

Notation as in `01-changes-and-changelists.cases.md`.

### VCS-SETUP-001 case 1: clone dialog
settings: default
language: plain
```before
action Vcs.VcsClone
```
keys: (open)
```after
dialog "Clone Repository"; fields: "URL:", "Directory:"; buttons "Clone" (disabled), "Cancel"; with Git a "Test" button
```

### VCS-SETUP-002 case 1: validation
settings: default
language: plain
```before
clone dialog open
```
keys: enter each
```after
URL "" -> "Empty repository URL"
URL "not a url" -> "Repository URL is a malformed URL or non-existent directory"
Directory pointing to a non-empty existing folder -> "The directory already exists and it is not empty"
URL "https://h/x/y.git" -> target folder name "y"
```

### VCS-SETUP-003 case 1: clone local repository
settings: default
language: plain
```before
source repo /tmp/src with 2 commits on main
```
keys: URL /tmp/src, directory /tmp/dst, Clone
```after
/tmp/dst/src contains .git and the files; progress text "Cloning repository..."; submodules cloned recursively when present
```

### VCS-SETUP-010 case 1: no VCS menu
settings: default
language: plain
```before
folder project without .git
```
keys: open VCS menu
```after
entries include "Enable Version Control Integration..." and "Create Git Repository..."; Version Control tool window text "To track changes to code:" with links "Share project on", "Create Git repository...", "Version control integration"
```

### VCS-SETUP-011 case 1: enable integration
settings: default
language: plain
```before
folder project
```
keys: action Start.Use.Vcs
```after
dialog "Enable Version Control Integration": "Select a version control system to associate with <project>:" combo with "Git"; OK creates mapping "<Project>" -> Git
```

### VCS-SETUP-012 case 1: git init
settings: default
language: plain
```before
folder project without .git
```
keys: action Git.Init, accept the project directory
```after
chooser titled "Create Git Repository" with description "Select directory where the new Git repository will be created."; after OK: .git created; notification "Created Git repository in <dir>"; Local Changes lists all files under "Unversioned Files"
```

### VCS-SETUP-012 case 2: already under git
settings: default
language: plain
```before
directory already contains .git
```
keys: Git.Init on it
```after
warning "The selected directory <dir> is already under Git." "Are you sure that you want to create a new VCS root?" titled "Git Init"
```

### VCS-SETUP-020 case 1: mappings page
settings: default
language: plain
```before
Git project with a single repository at the project root
```
keys: Settings > Version Control
```after
table with columns "Directory", "VCS"; row "<Project>" -> "Git"; checkbox "Enable automatic mapping detection"
```

### VCS-SETUP-021 case 1: root detected
settings: automatic detection on
language: plain
```before
project without mapping but with a nested directory containing .git
```
keys: open project
```after
notification "Git Repository Found" with action "Enable Integration"; after the action: "Added Git root: <path>"
```

### VCS-SETUP-022 case 1: safe mode
settings: project untrusted
language: plain
```before
untrusted project
```
keys: open Settings > Version Control
```after
text "Safe mode, Version Control is not available. Trust the project to access full IDE functionality."
```

### VCS-SETUP-030 case 1: duplicate ignore entry
settings: default
language: plain
```before
.gitignore:
build/
build/
```
keys: (highlight)
```after
second line flagged with "build/ pattern is defined more than once" (quick fix "Remove pattern")
```

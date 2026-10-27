# Cases for 02-project-view

Tree notation: indentation is nesting, `[x]` expanded, `[ ]` collapsed, `*` selected row. `files:` lists the file system. Grey location text is written after two spaces
(`name  location`). Settings not named are at the defaults of `02-project-view.md` section 3.

### WB-PV-010 case 1: root children order with defaults
settings: default
language: plain
```before
files:
  demo/ (content root of module "demo")
    src/ (sources root)
    pom.xml
scratch files: scratch.txt exists
libraries: JDK 21
```
keys: action ActivateProjectToolWindow
```after
tree:
[x] demo  ~/IdeaProjects/demo
  [ ] src  sources root
  pom.xml
[ ] External Libraries
[ ] Scratches and Consoles
```

### WB-PV-010 case 2: options off remove the trailing nodes
settings: showLibraryContents=false, showScratchesAndConsoles=false
language: plain
```before
files: same as WB-PV-010 case 1
```
keys: action ActivateProjectToolWindow
```after
tree:
[x] demo  ~/IdeaProjects/demo
  [ ] src  sources root
  pom.xml
```

### WB-PV-012 case 1: module name differs from directory name
settings: default
language: plain
```before
files: demo/ is the content root of module "demo-app"
```
keys: action ActivateProjectToolWindow
```after
row text: "demo [demo-app]" ("[demo-app]" bold), location "~/IdeaProjects/demo"
```

### WB-PV-012 case 2: module name equals directory name
settings: default
language: plain
```before
files: demo/ is the content root of module "demo"
```
keys: action ActivateProjectToolWindow
```after
row text: "demo" bold, location "~/IdeaProjects/demo"
```

### WB-PV-013 case 1: excluded folder visibility
settings: showExcludedFiles=true
language: plain
```before
files:
  demo/
    src/
    target/ (excluded)
```
keys: toggle "Show Excluded Files" off
```after
before toggle: demo > src, target (excluded icon)
after toggle:  demo > src
```

### WB-PV-015 case 1: non-content parent directory
settings: default
language: plain
```before
files:
  repo/ (not a content root)
    a/ (content root of module a)
    b/ (content root of module b)
    notes/ (no module)
```
keys: action ActivateProjectToolWindow
```after
tree:
[x] repo
  [ ] a
  [ ] b
(notes is not listed)
```

### WB-PV-030 case 1: default sort
settings: default
language: plain
```before
directory x/ contains: b.txt, Alpha.txt, lib/, a10.txt, a2.txt, Docs/
```
keys: expand x
```after
Docs
lib
a2.txt
a10.txt
Alpha.txt
b.txt
```

### WB-PV-031 case 1: folders not on top
settings: foldersAlwaysOnTop=false
language: plain
```before
directory x/ contains: b.txt, Alpha.txt, lib/, a10.txt, a2.txt, Docs/
```
keys: expand x
```after
a2.txt
a10.txt
Alpha.txt
b.txt
Docs
lib
```

### WB-PV-032 case 1: sort by type
settings: sortKey=BY_TYPE
language: plain
```before
directory x/ contains: c.java, b.txt, a.java, sub/
```
keys: expand x
```after
sub
a.java
c.java
b.txt
```

### WB-PV-033 case 1: sort by modification time, newest first
settings: sortKey=BY_TIME_DESCENDING
language: plain
```before
directory x/ contains: old.txt (modified 2024-01-01), mid.txt (2025-01-01), new.txt (2026-01-01)
```
keys: expand x
```after
new.txt
mid.txt
old.txt
```

### WB-PV-040 case 1: hide empty middle packages
settings: hideEmptyMiddlePackages=true
language: java
```before
src/ (sources root)
  com/acme/app/Main.java
```
keys: expand src
```after
src
  [x] com.acme.app
        Main.java
```

### WB-PV-040 case 2: option off
settings: hideEmptyMiddlePackages=false
language: java
```before
src/ (sources root)
  com/acme/app/Main.java
```
keys: expand src, expand all
```after
src
  [x] com
    [x] acme
      [x] app
            Main.java
```

### WB-PV-042 case 1: middle package with files is not collapsed
settings: hideEmptyMiddlePackages=true
language: java
```before
src/ (sources root)
  com/acme/Util.java
  com/acme/app/Main.java
```
keys: expand src, expand all
```after
src
  [x] com.acme
        [x] app
              Main.java
        Util.java
```

### WB-PV-043 case 1: compact directories
settings: compactDirectories=true
language: plain
```before
docs/guide/img/a.png   (docs contains only guide, guide contains only img)
```
keys: expand project root
```after
[x] docs.guide.img
      a.png
```

### WB-PV-044 case 1: flatten packages
settings: flattenPackages=true
language: java
```before
src/ (sources root)
  com/                (no files)
  com/acme/A.java
  com/acme/app/B.java
```
keys: expand src
```after
src
  [ ] com.acme
  [ ] com.acme.app
```

### WB-PV-045 case 1: abbreviate qualified names
settings: abbreviatePackageNames=true, hideEmptyMiddlePackages=true
language: java
```before
src/com/acme/app/Main.java
```
keys: expand src
```after
src
  [x] c.a.app     (tooltip: com.acme.app)
        Main.java
```

### WB-PV-046 case 1: select opened file inside a compact node
settings: compactDirectories=true
language: plain
```before
docs/guide/img/a.png open in the editor; tree fully collapsed
```
keys: Select Opened File (header button)
```after
[x] docs.guide.img
      * a.png
```

### WB-PV-050 case 1: file nesting rule
settings: useFileNestingRules=true, rules: ".ts -> .js"
language: plain
```before
directory x/ contains: foo.ts, foo.js, bar.js
```
keys: expand x
```after
[x] foo.ts
      foo.js
bar.js
```

### WB-PV-050 case 2: nesting switched off
settings: useFileNestingRules=false, rules: ".ts -> .js"
language: plain
```before
directory x/ contains: foo.ts, foo.js, bar.js
```
keys: expand x
```after
bar.js
foo.js
foo.ts
```

### WB-PV-050 case 3: transitive rules
settings: useFileNestingRules=true, rules: ".ts -> .js", ".js -> .js.map"
language: plain
```before
directory x/ contains: foo.ts, foo.js, foo.js.map
```
keys: expand x, expand all
```after
[x] foo.ts
      foo.js
      foo.js.map
```

### WB-PV-054 case 1: invalid nesting rule
settings: default
language: plain
```before
File Nesting dialog open
```
keys: add rule parent ".ts" child ".ts", press OK
```after
rule rejected, dialog stays open, no change in stored rules
```

### WB-PV-060 case 1: select opened file expands ancestors
settings: default
language: java
```before
src/main/java/com/acme/Main.java open in the editor; Project tree collapsed to the root
```
keys: Select Opened File (header button)
```after
tree (Hide Empty Middle Packages on):
[x] demo
  [x] src
    [x] main
      [x] java
        [x] com.acme
              * Main.java
```

### WB-PV-061 case 1: always select opened file
settings: autoscrollFromSource=true
language: plain
```before
tabs: A.java, B.java; A.java active; tree selection on A.java; focus: editor
```
keys: click tab B.java
```after
tree selection on B.java; focus: editor
```

### WB-PV-062 case 1: Enter opens with focus, Ctrl+Enter keeps focus
settings: default
language: plain
```before
focus: Project tree, selection on A.java, not open
```
keys: Enter
```after
editor tab A.java active; focus: editor
```

### WB-PV-062 case 2: Ctrl+Enter
settings: default
language: plain
```before
focus: Project tree, selection on A.java, not open
```
keys: Ctrl+Enter
```after
editor tab A.java active; focus: Project tree
```

### WB-PV-065 case 1: arrow key navigation
settings: default
language: plain
```before
[ ] src (selected)
    (children: a, b)
```
keys: Right, Right, Left, Left
```after
after Right: [x] src, selection src
after Right: selection a
after Left: selection src
after Left: [ ] src collapsed, selection src
```

### WB-PV-066 case 1: collapse all
settings: default
language: plain
```before
everything expanded; focus: Project tree
```
keys: Ctrl+Minus
```after
only the direct children of the root are visible and collapsed
```

### WB-PV-070 case 1: speed search selection
settings: default
language: plain
```before
visible nodes: server, service, util; focus: tree; selection: util
```
keys: type "ser"
```after
speed search field shows "ser"; selection: server; highlight in server and service
```

### WB-PV-070 case 2: speed search navigation and escape
settings: default
language: plain
```before
visible nodes: server, service, util; speed search open with "ser"; selection: server
```
keys: Down, Escape
```after
after Down: selection service
after Escape: speed search field closed, selection service
```

### WB-PV-071 case 1: camel hump match
settings: default
language: plain
```before
visible nodes: StringMap.java, SortedMap.java, readme.md
```
keys: type "sm"
```after
selection: SortedMap.java (first match in tree order, names sorted); Down moves to StringMap.java
both highlighted, readme.md not matched
```

### WB-PV-073 case 1: no match
settings: default
language: plain
```before
visible nodes: server, service, util; selection: util
```
keys: type "zzz"
```after
selection: util (unchanged); speed search field in error state
```

### WB-PV-080 case 1: Alt+Insert popup in a plain directory
settings: default
language: plain
```before
focus: Project tree, selection on directory "docs" (not a package root)
```
keys: Alt+Insert
```after
popup entries include, in this order: "File", "Scratch File", "Directory"
```

### WB-PV-080 case 2: popup in a Java source root
settings: default
language: java
```before
focus: Project tree, selection on "src" (sources root)
```
keys: Alt+Insert
```after
popup entries start with language/class templates (Java Class ...), then include "File", "Scratch File", "Package" (instead of "Directory")
```

### WB-PV-081 case 1: new file creates intermediate directories
settings: default
language: plain
```before
tree selection: docs/ ; New popup open, choice "File" -> popup "New File"
```
keys: type "guide/intro.md", Enter
```after
files: docs/guide/intro.md created; editor opens intro.md; tree selection intro.md
```

### WB-PV-081 case 2: empty name
settings: default
language: plain
```before
popup "New File" open, empty field
```
keys: Enter
```after
popup stays open, nothing created
```

### WB-PV-082 case 1: file exists
settings: default
language: plain
```before
docs/README.md exists; popup "New File" open for docs/
```
keys: type "README.md", Enter
```after
popup stays open; error under the field: A file with the name 'README.md' already exists
```

### WB-PV-082 case 2: directory with that name exists
settings: default
language: plain
```before
docs/README.md is a directory; popup "New File" open for docs/
```
keys: type "README.md", Enter
```after
popup stays open; error under the field: A directory with the name 'README.md' already exists
```

### WB-PV-083 case 1: nested directories
settings: default
language: plain
```before
popup "New Directory" (prompt "Enter new directory name:") open for docs/
```
keys: type "a/b/c", Enter
```after
docs/a/b/c created (each missing level); tree selection c
```

### WB-PV-083 case 2: last segment exists
settings: default
language: plain
```before
docs/a exists as a directory; popup "New Directory" for docs/
```
keys: type "a", Enter
```after
error: A directory with the name 'a' already exists; nothing created
```

### WB-PV-083 case 3: intermediate segment is a file
settings: default
language: plain
```before
docs/a is a file; popup "New Directory" for docs/
```
keys: type "a/b", Enter
```after
error: A file with the name 'a' already exists; nothing created
```

### WB-PV-083 case 4: dot segment at the end
settings: default
language: plain
```before
popup "New Directory" for docs/
```
keys: type "x/..", Enter
```after
error: '..' is an invalid directory name
```

### WB-PV-083 case 5: parent segment
settings: default
language: plain
```before
popup "New Directory" for docs/guide/ (docs/guide exists)
```
keys: type "../sibling", Enter
```after
docs/sibling created (parent of docs/guide is docs)
```

### WB-PV-084 case 1: new package
settings: hideEmptyMiddlePackages=true
language: java
```before
popup "New Package" (prompt "Enter new package name:") for src (sources root)
```
keys: type "com.acme", Enter
```after
directories src/com/acme created; tree shows node "com.acme" under src; selection com.acme
```

### WB-PV-084 case 2: invalid package segment
settings: default
language: java
```before
popup "New Package" for src
```
keys: type "com.1acme"
```after
warning in popup: Invalid package name; it's impossible to create a Java class inside
```

### WB-PV-085 case 1: dot warning
settings: default
language: plain
```before
popup "New Directory" for docs/
```
keys: type "a.b"
```after
warning: Note: "." in the name is treated as a regular character. Use "/" instead if you mean to create nested directories
Enter creates directory "a.b"
```

### WB-PV-086 case 1: file-name-like directory prompt
settings: prompt enabled
language: plain
```before
popup "New Directory" for docs/
```
keys: type "notes.txt", Enter
```after
dialog "File Name Detected": The name you entered looks like a file name. Do you want to create a file named notes.txt instead?
buttons: Yes, Create File | No, Create Directory | Cancel
```

### WB-PV-090 case 1: mark excluded
settings: default
language: plain
```before
demo/build/ (plain directory); tree selection build
```
keys: menu Mark Directory As | Excluded
```after
build shows excluded icon; menu now offers "Not Excluded" / "Unmark" entries; with Show Excluded Files off the node disappears
```

### WB-PV-091 case 1: mark sources root
settings: default
language: plain
```before
demo/src/ (plain directory); tree selection src
```
keys: menu Mark Directory As | Sources Root
```after
src location text "sources root"; sources-root icon; entry "Sources Root" no longer offered
```

### WB-PV-100 case 1: delete confirmation
settings: default
language: plain
```before
tree selection: old/ (directory with 3 files)
```
keys: Delete
```after
dialog titled "Delete" asks to delete the directory; text mentions that all files and subdirectories will be deleted; buttons Delete, Cancel
Delete confirmed: old/ removed, tabs of its files closed
Cancel: nothing happens
```

### WB-PV-101 case 1: copy paste into directory
settings: default
language: plain
```before
files: a.txt, docs/ ; selection a.txt
```
keys: Ctrl+C, select docs, Ctrl+V
```after
files: a.txt, docs/a.txt
```

### WB-PV-102 case 1: drag onto own descendant is refused
settings: default
language: plain
```before
files: docs/img/
```
keys: drag docs onto img
```after
drop rejected, no change
```

### WB-PV-011 case 1: module with several roots
settings: default
language: plain
```before
module app with content roots app/ and shared/
```
keys: expand the project root
```after
[x] app  (module node)
  [ ] app
  [ ] shared
```

### WB-PV-016 case 1: children of a directory
settings: default
language: plain
```before
demo/ contains src/, README.md, target/ (excluded)
```
keys: expand demo
```after
src
target
README.md
```

### WB-PV-017 case 1: no content roots
settings: default
language: plain
```before
project without content roots
```
keys: Alt+1
```after
Project tree is empty; no error dialog
```

### WB-PV-022 case 1: toggling an option keeps expansion and selection
settings: default
language: java
```before
src > com.acme.app expanded, selection Main.java, showMembers=false
```
keys: toggle "Show Members" on, toggle it off
```after
after toggling on: Main.java expandable (class Main and members as children)
after toggling off: same expanded paths as before; selection Main.java
```

### WB-PV-023 case 1: exclusive sort entries
settings: default
language: plain
```before
Sort by Type checked
```
keys: gear > Sort by Modification Time (Newest First)
```after
checked: Modification Time (Newest First); unchecked: Name, Type, Modification Time (Oldest First)
```

### WB-PV-063 case 1: open on single click selection
settings: autoscrollToSource=true
language: plain
```before
focus: Project tree, selection on docs (directory); A.java not open
```
keys: Down (selection moves to A.java)
```after
editor tab A.java active; focus: Project tree
```

### WB-PV-064 case 1: single click expands a directory
settings: openDirectoriesWithSingleClick=true
language: plain
```before
[ ] src collapsed
```
keys: single click on src
```after
[x] src
```

### WB-PV-067 case 1: state restored on reopen
settings: default
language: plain
```before
expanded: demo, src; selection: src/Main.java; project closed and reopened
```
keys: (none)
```after
expanded: demo, src; selection: src/Main.java
```

### WB-PV-072 case 1: Enter opens the match
settings: default
language: plain
```before
visible nodes: README.md, server.txt; focus tree; speed search "ser"
```
keys: Enter
```after
server.txt opens in the editor; speed search closed
```

### WB-PV-092 case 1: marking persists
settings: default
language: plain
```before
demo/src marked as Sources Root
```
keys: close and reopen the project
```after
src location text: sources root
```

### WB-PV-103 case 1: rename clash
settings: default
language: plain
```before
files: a.txt, b.txt; selection a.txt
```
keys: Shift+F6, type "b.txt", Enter
```after
error shown (a file with that name exists); both files unchanged
```

### WB-PV-105 case 1: read-only directory delete
settings: default
language: plain
```before
old/ contains a read-only file
```
keys: select old/, Delete, confirm
```after
dialog text: Directory old contains read-only file(s). Delete it anyway?   (Delete it anyway / Cancel behavior as the platform dialog)
```

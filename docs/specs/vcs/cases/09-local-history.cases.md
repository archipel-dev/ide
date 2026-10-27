# Cases for 09 (local history)

Notation as in `01-changes-and-changelists.cases.md`; `lh:` lists journal entries newest first.

### VCS-LH-002 case 1: ignored names
settings: default
language: plain
```before
project content root with files: Main.java, Main.class, workspace.xml, proj.iws, notes.txt in excluded folder out/
```
keys: edit and save each of them
```after
lh: entries only for Main.java (Modify); none for Main.class, workspace.xml, proj.iws; none for out/notes.txt
```

### VCS-LH-003 case 1: binary content
settings: default
language: plain
```before
img.png (binary) replaced on disk by an editor
```
keys: Show History on img.png, select the entry
```after
entry "Modify img.png"; diff pane text "File content is not available"
```

### VCS-LH-004 case 1: save creates entry
settings: default
language: plain
```before
A.txt "one"
```
keys: type "two" replacing text, Ctrl+S
```after
lh: "Modify A.txt" (content before: "one")
```

### VCS-LH-004 case 2: external change
settings: default
language: plain
```before
A.txt "one"
```
keys: modify A.txt from a shell, return focus to the IDE
```after
lh: activity named "External change"
```

### VCS-LH-005 case 1: retention
settings: localHistory.daysToKeep=5
language: plain
```before
entries 6 days old and 4 days old
```
keys: restart IDE
```after
only the 4 days old entry remains
```

### VCS-LH-010 case 1: menu
settings: default
language: plain
```before
VCS > Local History submenu
```
keys: (open)
```after
items in order: "Show History...", "Show History for Selection...", "Show Project History...", "Put Label...", "Recent Changes" (Alt+Shift+C)
```

### VCS-LH-013 case 1: put label
settings: default
language: plain
```before
project open
```
keys: action LocalHistory.PutLabel, type "before-refactor", OK
```after
dialog title "Put Label", prompt "Label name:"; notification "Label before-refactor created" with action "View local history"; lh top entry is the label "before-refactor"
```

### VCS-LH-013 case 2: empty label
settings: default
language: plain
```before
dialog open
```
keys: leave empty
```after
OK disabled
```

### VCS-LH-020 case 1: item texts
settings: default
language: java
```before
actions: rename A.java to B.java; delete C.java; create D.java; edit E.java
```
keys: Show Project History
```after
list: "Modify E.java", "Create D.java", "Delete C.java", "Rename A.java to B.java" (newest first)
```

### VCS-LH-021 case 1: empty states
settings: default
language: plain
```before
Recent tab with no changes
```
keys: Alt+Shift+C
```after
text "No recent activity detected"
```

### VCS-LH-022 case 1: diff mode text
settings: default (diffMode local directory)
language: plain
```before
item selected with 1 file
```
keys: (view)
```after
below the tree: "Diff with the local directory." and link "Switch to diff with the next revision"
```

### VCS-LH-030 case 1: revert
settings: default
language: plain
```before
A.txt: "one" -> save -> "two" -> save -> "three" (three entries)
```
keys: select the entry for "two" (change from one to two), action ActivityView.Revert
```after
A.txt content "one"; a new top entry "Revert"; revert is undone by one Ctrl+Z
```

### VCS-LH-031 case 1: read-only file
settings: default
language: plain
```before
target file read-only
```
keys: Revert
```after
message "Cannot revert because some files are read-only"
```

### VCS-LH-032 case 1: revert selection
settings: default
language: plain
```before
A.txt: "a\nb\nc" -> "A\nb\nC" (two chunks); history diff shows both
```
keys: select only chunk 1 in the diff, Revert Selection
```after
A.txt "a\nb\nC"
```

### VCS-LH-050 case 1: update labels
settings: default
language: plain
```before
Git update modifies B.java
```
keys: Ctrl+T, then Show Project History
```after
entries top to bottom include: label "After update", change entry for B.java, label "Before update"
```

### VCS-LH-051 case 1: restore a deleted file
settings: default
language: plain
```before
Deleted file src/Old.java (content "x") from the IDE
```
keys: Show History on src, select the deletion, Revert
```after
src/Old.java exists with content "x"
```

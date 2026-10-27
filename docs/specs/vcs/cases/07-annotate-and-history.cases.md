# Cases for 07 (annotate, history)

Notation as in `01-changes-and-changelists.cases.md`.

### VCS-ANN-001 case 1: toggle annotate
settings: default
language: java
```before
A.java tracked, 3 lines, last commit by "Ann Lee" 2024-03-01
```
keys: right click gutter > Annotate with Git Blame
```after
annotation column left of the code: each line shows "Ann Lee" and the short date; the menu entry is now checked
```

### VCS-ANN-001 case 2: directory
settings: default
language: plain
```before
folder selected
```
keys: action Annotate
```after
action disabled; message "Cannot annotate directory" when forced
```

### VCS-ANN-002 case 1: uncommitted line
settings: default
language: java
```before
annotation shown; user inserts a new line 2 and saves
```
keys: (wait for refresh)
```after
line 2 annotation text "Not committed yet"; other lines keep author names, shifted down by one
```

### VCS-ANN-003 case 1: colors
settings: color mode Author, light theme
language: java
```before
commits by 2 authors
```
keys: (view)
```after
background colors of annotation cells use the interpolated sequence from anchors a1bef7, c9dafb, dee8fc, edf2fa, f7f8fa; the two authors get different colors
```

### VCS-ANN-004 case 1: tooltip
settings: default
language: java
```before
annotation shown; commit abc1234 by Ann Lee on 2024-03-01 touching path src/A.java
```
keys: hover annotation
```after
tooltip lines: "commit abc1234", "Author: Ann Lee", "Date: <date>", then the commit message
```

### VCS-ANN-006 case 1: defaults
settings: default
language: java
```before
annotation shown, open Options group
```
keys: (menu)
```after
"Ignore Whitespaces" checked; "Detect Movements Within File" unchecked; "Detect Movements Across Files" unchecked
```

### VCS-HIST-001 case 1: renamed file history
settings: default
language: plain
```before
commits: r3 "edit B", r2 "rename A->B", r1 "add A"
```
keys: Show History on B.java
```after
history rows newest first: r3 "edit B", r2 "rename A->B", r1 "add A" (the file name column, when shown, changes from B.java to A.java at r1)
```

### VCS-HIST-002 case 1: limit message
settings: Limit history to 1000 rows (default)
language: plain
```before
file with 1500 commits
```
keys: Show History
```after
only 1000 entries; banner "File History: only 1000 revisions were loaded for <file>" followed by "To change the history limit, go to <settings path>"
```

### VCS-HIST-003 case 1: diff with parent
settings: default
language: java
```before
history of A.java with 3 revisions; revision 2 selected
```
keys: Ctrl+D
```after
diff opens comparing revision 1 content (left) with revision 2 content (right); title strings use "Revision <n>"-style content titles
```

### VCS-HIST-003 case 2: compare two
settings: default
language: java
```before
revisions 1 and 3 selected
```
keys: Ctrl+D
```after
diff of rev 1 vs rev 3; action text is "Compare" instead of "Show Diff"
```

### VCS-HIST-005 case 1: selection history
settings: default
language: java
```before
file with methods f (lines 1-3) and g (lines 5-7); commits c1 edits f, c2 edits g, c3 edits f; select lines of f
```
keys: action Vcs.ShowHistoryForBlock
```after
revision list: c3, c1, plus the row "Local Changes" at top when modified; c2 absent
```

### VCS-HIST-006 case 1: compare with revision popup
settings: default
language: plain
```before
file A.java with 3 commits
```
keys: action Compare.Selected
```after
popup titled "File Revisions" listing 3 revisions newest first; choosing one opens a diff with the local file
```

### VCS-HIST-020 case 1: operations popup
settings: default
language: plain
```before
Git project; Alt+Backquote
```
keys: Alt+Backquote
```after
popup titled "VCS Operations"
```

# Cases for 08 (diff viewer)

Notation as in `01-changes-and-changelists.cases.md`. Diff cases give `left` and `right`
contents; line numbers start at 1.

### VCS-DIFF-001 case 1: open from changes
settings: default
language: plain
```before
Local Changes with A.java modified
```
keys: Ctrl+D
```after
editor tab "Changes: A.java" with a diff viewer; the left (base) content title reads "Base revision"; the right side is the editable local content
```

### VCS-DIFF-002 case 1: compare with clipboard
settings: default
language: plain
```before
clipboard: "alpha\nbeta"; editor selection: "alpha\ngamma"
```
keys: action CompareClipboardWithSelection
```after
diff dialog titled "Clipboard vs A.txt"; the right content title is "Selection from A.txt"; the left content title is "From clipboard"
change: line 2 "beta" -> "gamma"
```

### VCS-DIFF-004 case 1: identical
settings: default
language: plain
```before
left "x" right "x"
```
keys: open diff
```after
label "Contents are identical"
```

### VCS-DIFF-004 case 2: only line separators
settings: default
language: plain
```before
left "a\nb\n" (LF), right "a\r\nb\r\n" (CRLF)
```
keys: open diff
```after
label "Contents have differences only in line separators"
```

### VCS-DIFF-011 case 1: changed block connector
settings: default, side-by-side
language: plain
```before
left:  1 a / 2 b / 3 c / 4 d
right: 1 a / 2 B1 / 3 B2 / 4 B3 / 5 d
```
keys: open
```after
one change connecting left line 2 to right lines 2-4; without alignment the right side has 5 lines; with "Align Changes" on the left side gets 2 filler lines after "b"
status line "1 difference"
```

### VCS-DIFF-013 case 1: collapse unchanged
settings: Context lines 4, collapse on
language: plain
```before
left and right have 30 lines each, differing only at line 15
```
keys: open
```after
visible lines 11..19 on both sides; lines 1..10 folded into one marker above and 20..30 into one marker below
```

### VCS-DIFF-013 case 2: changes view default
settings: place CHANGES_VIEW defaults
language: plain
```before
same files
```
keys: open from Local Changes preview
```after
unchanged fragments collapsed by default; line numbers hidden
```

### VCS-DIFF-015 case 1: read-only lock
settings: default
language: plain
```before
diff of a local file against HEAD
```
keys: type "x" in the right editor
```after
typing blocked; hint "This view is read-only. Enable editing"; clicking the link allows typing
```

### VCS-DIFF-017 case 1: settings popup
settings: default
language: plain
```before
diff open
```
keys: Ctrl+Shift+D
```after
popup: group "Highlighting Differences": Lines, Words (selected), Split changes, Characters, None; group "Ignore Differences": None (selected), Trim whitespaces, Ignore whitespaces, Ignore whitespaces and empty lines, Ignore formatting; then toggles
```

### VCS-DIFF-020 case 1: word highlight
settings: Words
language: plain
```before
left "int a = 1;"  right "int a = 2;"
```
keys: open
```after
line 1 is a changed block; inner highlight on "1" (left) and "2" (right) only
```

### VCS-DIFF-021 case 1: ignore whitespaces
settings: Ignore whitespaces
language: plain
```before
left "if (x) {\n  y();\n}"  right "if (x) {\n\ty();\n}"
```
keys: open
```after
no differences ("No differences" status or dimmed ignored band)
```

### VCS-DIFF-021 case 2: trim whitespaces
settings: Trim whitespaces
language: plain
```before
left "a  \nb"  right "a\nb"
```
keys: open
```after
no differences (trailing whitespace ignored); with policy None one change at line 1
```

### VCS-DIFF-021 case 3: whitespaces and empty lines
settings: Ignore whitespaces and empty lines
language: plain
```before
left "a\nb"  right "a\n\nb"
```
keys: open
```after
no differences
```

### VCS-DIFF-022 case 1: F7 across files
settings: Go to the next file after reaching last change = on
language: plain
```before
two files in the chain, caret at the last change of file 1
```
keys: F7, F7
```after
first F7: hint "Press again to go to the next file"; second F7: file 2 opens at its first change
```

### VCS-DIFF-023 case 1: status text
settings: default
language: plain
```before
two changes
```
keys: open
```after
status "2 differences"
```

### VCS-DIFF-030 case 1: accept left
settings: default
language: plain
```before
left "A\nx\nB"  right "A\ny\nB", right editable and unlocked, caret in the change
```
keys: action Diff.ApplyLeftSide (Ctrl+Alt+R)
```after
right "A\nx\nB"; undo entry named "Replace change"
```

### VCS-DIFF-030 case 2: revert from right
settings: default
language: plain
```before
left "A\nx\nB"  right "A\ny\nB", editable left
```
keys: action Diff.ApplyRightSide (Ctrl+Alt+A)
```after
left "A\ny\nB"
```

### VCS-DIFF-031 case 1: selected lines
settings: default
language: plain
```before
left "1\n2\n3\n4"  right "a\nb\nc\nd" (one change block of 4 lines), selection on right lines 2-3, revert
```
keys: revert selected
```after
right "a\n2\n3\nd"
```

### VCS-DIFF-040 case 1: combined diff header
settings: All-in-One on
language: plain
```before
three changed files selected
```
keys: Ctrl+D
```after
file counter reads "3 files" when all are shown ("1 out of 3 files" as a link when filtered); three stacked blocks with headers; Ctrl+Esc collapses the focused block
```

### VCS-DIFF-060 case 1: dir diff toolbar
settings: default
language: plain
```before
two folders compared
```
keys: Ctrl+D on both
```after
directory viewer: columns Name, Size, Date (left) | operation | Name, Size, Date (right); filter field "Filter:"; "Compare by:" combo with Size and Timestamp (default), Size, Text, Binary Content
```

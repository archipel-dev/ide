# Cases for 05-recent-and-structure-popups.md

Conventions: `popup:` the popup content and selection; `editor:` the editor state (`<caret>` marks the caret); `list:` ordered rows. Files are `A.java`, `B.java`, ...
Unless stated the project has these files open or visited in this order (oldest first): A.java, B.java, C.java, then C.java is focused.

### SRCH-REC-001 case 1: recent files list
settings: default
language: plain
```before
visited A, B, C (C focused)
```
keys: Ctrl+E
```after
popup: 'Recent Files'; list: C.java, B.java, A.java; check box 'Show edited only' unchecked
```

### SRCH-REC-001 case 2: edited only
settings: default
language: plain
```before
only B.java was edited
```
keys: Ctrl+E, Ctrl+E
```after
popup: check box checked; list: B.java
```

### SRCH-REC-001 case 3: empty list
settings: default
language: plain
```before
no file ever opened
```
keys: Ctrl+E
```after
popup text: 'No recent files'
```

### SRCH-REC-002 case 1: the focused file is skipped
settings: default
language: plain
```before
visited A, B, C (C focused)
```
keys: Ctrl+E
```after
popup: selection on B.java (second row)
```

### SRCH-REC-002 case 2: a single entry
settings: default
language: plain
```before
only C.java, focused
```
keys: Ctrl+E
```after
popup: selection on C.java
```

### SRCH-REC-003 case 1: Enter opens
settings: default
language: plain
```before

```
keys: Ctrl+E, Down, Enter
```after
editor: A.java focused; popup closed
```

### SRCH-REC-003 case 2: Delete closes the tab
settings: default
language: plain
```before

```
keys: Ctrl+E, Delete
```after
B.java tab closed; the selection moves to the row that took its place
```

### SRCH-REC-003 case 3: speed search miss
settings: default
language: plain
```before

```
keys: Ctrl+E, type 'zzz'
```after
popup text: Press 'Enter' to search in Project
```

### SRCH-REC-005 case 1: quick toggle
settings: default
language: plain
```before
visited A, B (B focused)
```
keys: Ctrl+Tab (press and release)
```after
editor: A.java focused
```

### SRCH-REC-005 case 2: hold and iterate
settings: default
language: plain
```before
visited A, B, C (C focused)
```
keys: hold Ctrl, Tab, Tab, release Ctrl
```after
popup 'Switcher' while held; selection moves B then A; on release A.java opens
```

### SRCH-REC-005 case 3: Esc cancels
settings: default
language: plain
```before

```
keys: hold Ctrl, Tab, Esc
```after
popup closed, C.java still focused
```

### SRCH-REC-006 case 1: selection moves a file to the top
settings: default
language: plain
```before
visited A, B, C
```
keys: click the A.java tab, Ctrl+E
```after
list: A.java, C.java, B.java
```

### SRCH-REC-006 case 2: closing keeps the file
settings: default
language: plain
```before

```
keys: close B.java, Ctrl+E
```after
list contains B.java
```

### SRCH-REC-010 case 1: snippet window
settings: default
language: plain
```before
caret on line 10 of a 20-line file
```
keys: Ctrl+Shift+E
```after
entry: lines 8-12 with line 10 highlighted, header: breadcrumb or file name
```

### SRCH-REC-010 case 2: window slides at the file start
settings: default
language: plain
```before
caret on line 1
```
keys: Ctrl+Shift+E
```after
entry: lines 1-5
```

### SRCH-REC-010 case 3: empty
settings: default
language: plain
```before
no navigation yet
```
keys: Ctrl+Shift+E
```after
popup text: 'No recent locations found'
```

### SRCH-REC-011 case 1: Delete removes
settings: default
language: plain
```before
two entries
```
keys: Delete on the first
```after
entry removed from the list and from the history
```

### SRCH-HIS-001 case 1: typing does not push
settings: default
language: plain
```before
A.java
```
keys: type text, move with arrows
```after
Back stack unchanged
```

### SRCH-HIS-001 case 2: same place replaces
settings: default
language: plain
```before
navigate twice to the same place
```
keys: Ctrl+Alt+Left
```after
one Back step only
```

### SRCH-HIS-001 case 3: stack limit
settings: default
language: plain
```before
151 distinct navigations
```
keys: Back 151 times
```after
only 150 steps are possible
```

### SRCH-HIS-002 case 1: back and forward
settings: default
language: plain
```before
go to declaration from line 5 to line 50
```
keys: Ctrl+Alt+Left, Ctrl+Alt+Right
```after
editor: caret line 5, then line 50
```

### SRCH-HIS-002 case 2: a new navigation clears forward
settings: default
language: plain
```before
after Back
```
keys: navigate elsewhere, Ctrl+Alt+Right
```after
nothing happens (disabled)
```

### SRCH-HIS-002 case 3: deleted file
settings: default
language: plain
```before
history entry for a deleted file
```
keys: Back
```after
the entry is skipped
```

### SRCH-HIS-003 case 1: last edit location
settings: default
language: plain
```before
edit line 3, then line 40, caret at line 1
```
keys: Ctrl+Shift+Backspace
```after
editor: caret at line 40
```

### SRCH-HIS-003 case 2: repeat goes older
settings: default
language: plain
```before

```
keys: Ctrl+Shift+Backspace twice
```after
editor: caret at line 3
```

### SRCH-GTL-001 case 1: dialog
settings: default
language: plain
```before
caret line 7 column 4
```
keys: Ctrl+G
```after
dialog 'Go to Line:Column', label '[Line] [:column]:', field '7:4' selected
```

### SRCH-GTL-002 case 1: line only
settings: default
language: plain
```before
caret line 7 in a 30-line file, `12` typed
```
keys: Ctrl+G, type '12', Enter
```after
line 12 column 1
```

### SRCH-GTL-002 case 2: line and column
settings: default
language: plain
```before
caret line 7 in a 30-line file, `12:5` typed
```
keys: Ctrl+G, type '12:5', Enter
```after
line 12 column 5
```

### SRCH-GTL-002 case 3: comma
settings: default
language: plain
```before
caret line 7 in a 30-line file, `12,5` typed
```
keys: Ctrl+G, type '12,5', Enter
```after
line 12 column 5
```

### SRCH-GTL-002 case 4: column only
settings: default
language: plain
```before
caret line 7 in a 30-line file, `:5` typed
```
keys: Ctrl+G, type ':5', Enter
```after
current line, column 5
```

### SRCH-GTL-002 case 5: relative down
settings: default
language: plain
```before
caret line 7 in a 30-line file, `+3` typed
```
keys: Ctrl+G, type '+3', Enter
```after
caret line + 3, column 1
```

### SRCH-GTL-002 case 6: garbage
settings: default
language: plain
```before
caret line 7 in a 30-line file, `abc` typed
```
keys: Ctrl+G, type 'abc', Enter
```after
dialog stays open
```

### SRCH-GTL-003 case 1: clamped line
settings: default
language: plain
```before
30-line file
```
keys: Ctrl+G, '99', Enter
```after
editor: caret on the last line
```

### SRCH-GTL-003 case 2: clamped column
settings: default
language: plain
```before
line 3 has 10 chars
```
keys: Ctrl+G, '3:99', Enter
```after
editor: caret at the end of line 3
```

### SRCH-GTL-003 case 3: secondary carets and selection removed
settings: default
language: plain
```before
3 carets, a selection
```
keys: Ctrl+G, '1', Enter
```after
editor: one caret, no selection, line 1 centred
```

### SRCH-FST-001 case 1: popup opens on the caret element
settings: default
language: plain
```before
class A { void a(){} void <caret>b(){} }
```
keys: Ctrl+F12
```after
popup: tree A > a(), b(); b() selected
```

### SRCH-FST-001 case 2: no structure
settings: default
language: plain
```before
empty plain text file
```
keys: Ctrl+F12
```after
popup text: 'Structure is empty'
```

### SRCH-FST-002 case 1: narrowing
settings: default
language: plain
```before
class A { void alpha(){} void beta(){} }
```
keys: Ctrl+F12, type 'alp'
```after
tree: A > alpha() only, alpha() selected
```

### SRCH-FST-002 case 2: narrow down off
settings: default
language: plain
```before
same
```
keys: untick 'Narrow down on typing', type 'bet'
```after
tree shows both methods; beta() selected
```

### SRCH-FST-002 case 3: not found
settings: default
language: plain
```before
same
```
keys: type 'zzz'
```after
text: 'Structure is empty' replaced by the not-found text
```

### SRCH-FST-003 case 1: navigate
settings: default
language: plain
```before
same
```
keys: Ctrl+F12, type 'bet', Enter
```after
editor: caret on beta in the declaration; popup closed
```

### SRCH-FST-004 case 1: inherited members
settings: default
language: plain
```before
class B extends A { void b(){} }, A has a()
```
keys: Ctrl+F12, Ctrl+F12
```after
tree now also lists a() (grey) from A
```

### SRCH-FST-004 case 2: anonymous classes
settings: default
language: plain
```before
a method containing new Object(){}
```
keys: Ctrl+F12, Ctrl+I
```after
anonymous class node appears under the method
```

### SRCH-NVB-001 case 1: path
settings: default
language: plain
```before
file p/q/A.java, caret inside method run() of class A
```
keys: (look at the bar)
```after
bar: project > module > p > q > A.java > A > run()
```

### SRCH-NVB-001 case 2: members off
settings: Members in navigation bar=off
language: plain
```before
same
```
keys: 
```after
bar ends at A.java
```

### SRCH-NVB-002 case 1: keyboard
settings: default
language: plain
```before
same
```
keys: Alt+Home, Left, Down
```after
focus in bar; popup lists the children of the previous item, directories before files, natural name order
```

### SRCH-NVB-002 case 2: Enter navigates
settings: default
language: plain
```before
same
```
keys: Alt+Home, Left, Left, Down, Enter on a file
```after
file opens; bar loses focus
```

### SRCH-BKM-001 case 1: toggle on
settings: default
language: plain
```before
<caret> on line 3
```
keys: F11
```after
gutter icon on line 3; bookmark in the default list
```

### SRCH-BKM-001 case 2: toggle off
settings: default
language: plain
```before
bookmark on line 3
```
keys: F11
```after
bookmark removed
```

### SRCH-BKM-001 case 3: description from selection
settings: default
language: plain
```before
selected text 'TODO fix' on line 3
```
keys: F11
```after
bookmark description 'TODO fix'
```

### SRCH-BKM-002 case 1: numbered
settings: default
language: plain
```before
line 3 without bookmark
```
keys: Ctrl+Shift+1
```after
bookmark with mnemonic 1
```

### SRCH-BKM-002 case 2: same mnemonic removes
settings: default
language: plain
```before
line 3 has mnemonic 1
```
keys: Ctrl+Shift+1
```after
bookmark removed
```

### SRCH-BKM-002 case 3: taken mnemonic
settings: default
language: plain
```before
mnemonic 1 on A.java:3; caret on B.java:9
```
keys: Ctrl+Shift+1
```after
dialog 'Rewrite Mnemonic': ‘1’ mnemonic is already taken by ‘A.java:3’. Do you want to rewrite it?  buttons Rewrite, Cancel
```

### SRCH-BKM-003 case 1: chooser
settings: default
language: plain
```before

```
keys: Ctrl+F11
```after
popup 'Add Mnemonic Bookmark' with 36 buttons, field 'Description (Optional)'
```

### SRCH-BKM-004 case 1: go to
settings: default
language: plain
```before
mnemonic 2 on A.java:7
```
keys: Ctrl+2
```after
editor: A.java caret on line 7
```

### SRCH-BKM-004 case 2: next bookmark cycles
settings: default
language: plain
```before
line bookmarks A:3, A:9, B:1; caret at B:1
```
keys: run Next Line Bookmark
```after
editor: A.java line 3
```

### SRCH-BKM-004 case 3: in editor, no wrap target on the same line
settings: default
language: plain
```before
single bookmark on this line
```
keys: run Next Line Bookmark in Editor
```after
disabled
```

### SRCH-BKM-005 case 1: popup
settings: default
language: plain
```before

```
keys: Shift+F11
```after
popup 'Bookmarks' with the tree of lists
```

### SRCH-BKM-006 case 1: create list
settings: default
language: plain
```before

```
keys: Create Bookmark List…, name 'New List' exists
```after
error 'A list with the specified name already exists'
```

### SRCH-BKM-006 case 2: delete list
settings: default
language: plain
```before
list 'Work'
```
keys: Delete
```after
dialog: Are you sure you want to delete 'Work' bookmark list? This action can't be undone.
```


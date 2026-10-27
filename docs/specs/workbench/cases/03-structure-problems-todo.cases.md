# Cases for 03-structure-problems-todo

Editor text uses `<caret>` and `<selection>...</selection>`. State lines such as `bookmarks:` describe the bookmark model: `file:line [mnemonic] "description" in list`.
Line numbers are 1-based as shown in the IDE.

### WB-STR-020 case 1: File Structure popup filters while typing
settings: narrowDown=true
language: java
```before
--- A.java
class A {
  void getName() {}
  void setName() {}
  int count;<caret>
}
```
keys: Ctrl+F12, type "name"
```after
popup tree (filtered, parents kept):
A
  getName()
  setName()
(count is hidden)
speed search text: name
```

### WB-STR-020 case 2: narrow down off keeps all rows
settings: narrowDown=false
language: java
```before
--- A.java
class A {
  void getName() {}
  void setName() {}
  int count;<caret>
}
```
keys: Ctrl+F12, type "name"
```after
popup tree: A, getName(), setName(), count all visible
getName() selected; "getName" and "setName" highlighted in the match
```

### WB-STR-022 case 1: Escape closes the filter first
settings: default
language: java
```before
--- A.java
class A { void foo() {} }<caret>
```
keys: Ctrl+F12, type "fo", Escape
```after
popup open, filter field closed, full tree visible, foo() selected
```

### WB-STR-022 case 2: second Escape closes the popup
settings: default
language: java
```before
--- A.java
class A { void foo() {} }<caret>
```
keys: Ctrl+F12, Escape
```after
popup closed, caret unchanged, focus editor
```

### WB-STR-022 case 3: Enter navigates and closes
settings: default
language: java
```before
--- A.java
class A {
  void foo() {}
  void bar() {}
}<caret>
```
keys: Ctrl+F12, type "bar", Enter
```after
--- A.java
class A {
  void foo() {}
  void <caret>bar() {}
}
```

### WB-STR-023 case 1: no match status
settings: default
language: java
```before
--- A.java
class A { void foo() {} }<caret>
```
keys: Ctrl+F12, type "zzz"
```after
status text under the tree: 'zzz' not found
```

### WB-STR-002 case 1: no structure
settings: default
language: plain
```before
all editors closed; Structure window open (Alt+7)
```
keys: action ActivateStructureToolWindow
```after
tree empty text: No structure
```

### WB-STR-005 case 1: Always Select Opened Element
settings: Structure "Always Select Opened Element"=on
language: java
```before
--- A.java
class A {
  void foo() {}
  void bar() { <caret> }
}
```
keys: move caret into foo's body (Up arrow, Up arrow)
```after
Structure tree selection: foo()
```

### WB-PRB-003 case 1: empty states of the File tab
settings: default
language: plain
```before
Problems window open on tab "File"; all editors closed
```
keys: action ActivateProblemsViewToolWindow
```after
tab text: Open file in editor to see problems
```

### WB-PRB-003 case 2: clean file
settings: default
language: java
```before
--- A.java
class A {}<caret>
```
keys: action ActivateProblemsViewToolWindow (after analysis finished)
```after
tab "File": No problems in A.java
```

### WB-PRB-002 case 1: errors listed with counts
settings: default
language: java
```before
--- A.java
class A {
  int x = "s";
  void f() { undefined(); }
}<caret>
```
keys: action ActivateProblemsViewToolWindow (after analysis finished)
```after
tab "File": 2 problems
  Errors (2)
    'incompatible types: String cannot be converted to int'  A.java:2
    'cannot find symbol: method undefined()'  A.java:3
```

### WB-PRB-004 case 1: F4 on a problem
settings: default
language: java
```before
Problems window focused, selection on the problem at A.java:3 (see previous case)
```
keys: F4
```after
--- A.java
class A {
  int x = "s";
  void f() { <caret>undefined(); }
}
focus: editor
```

### WB-PRB-005 case 1: sort by name in Project Errors
settings: Project Errors tab, files with errors: b.java (1 error), A.java (1 warning)
language: plain
```before
Problems window, tab "Project Errors", sorted by Severity: b.java, A.java
```
keys: gear menu | Sort by | Name
```after
order: A.java, b.java (alphabetical, case-insensitive)
```

### WB-TODO-001 case 1: default patterns
settings: default patterns
language: java
```before
--- A.java
class A {
  // TODO fix this
  // todo lower case
  // FIXME: later
  // TODOS plural
  // autotodo
  String s = "TODO in a string";
}
```
keys: open TODO tool window, tab "Project"
```after
Found 3 TODO items in 1 file
  A.java (3 items)
    TODO fix this   (2:6)
    todo lower case (3:6)
    FIXME: later    (4:6)
```

### WB-TODO-003 case 1: multi-line item
settings: multiline=true
language: java
```before
--- A.java
class A {
  // TODO first line
  //   continued here
  // not indented
}
```
keys: open TODO tool window, tab "Project"
```after
Found 1 TODO item in 1 file
  TODO first line continued here
```

### WB-TODO-003 case 2: multi-line off
settings: multiline=false
language: java
```before
--- A.java
class A {
  // TODO first line
  //   continued here
}
```
keys: open TODO tool window, tab "Project"
```after
Found 1 TODO item in 1 file
  TODO first line
```

### WB-TODO-007 case 1: group by packages
settings: arePackagesShown=false
language: java
```before
--- src/p/A.java
// TODO a
class A {}
--- src/q/B.java
// TODO b
class B {}
```
keys: Ctrl+Alt+P in the TODO tree
```after
Found 2 TODO items in 2 files
  p (1 item)
    A.java (1 item)
  q (1 item)
    B.java (1 item)
```

### WB-TODO-009 case 1: empty pattern
settings: default
language: plain
```before
Settings | Editor | TODO | Add Pattern dialog open, Pattern field empty
```
keys: press OK
```after
error: Pattern should be specified; dialog stays open
```

### WB-TODO-009 case 2: duplicate pattern
settings: default
language: plain
```before
Add Pattern dialog open
```
keys: type "\btodo\b.*", press OK
```after
error: Same pattern already exists; dialog stays open
```

### WB-BKM-001 case 1: toggle line bookmark
settings: default
language: java
```before
--- A.java
class A {
  void f() {
    int a = 1;<caret>
  }
}
bookmarks: none
```
keys: F11
```after
bookmarks: A.java:3 [-] "int a = 1;" in list "<project>"
gutter icon at line 3
```

### WB-BKM-001 case 2: toggle off
settings: default
language: java
```before
--- A.java
class A {
  void f() {
    int a = 1;<caret>
  }
}
bookmarks: A.java:3 [-] "int a = 1;" in list "<project>"
```
keys: F11
```after
bookmarks: none
gutter icon removed
```

### WB-BKM-002 case 1: list selection dialog
settings: lists "A" and "B", no default
language: plain
```before
--- a.txt
one<caret>
```
keys: F11
```after
dialog "Select Bookmark List": label "Bookmark list:", entries A, B, button "Select"
Select "B": bookmarks: a.txt:1 [-] "one" in list "B"
```

### WB-BKM-003 case 1: description from selection
settings: default
language: plain
```before
--- a.txt
  return <selection>foo(bar)</selection>;
```
keys: F11
```after
bookmarks: a.txt:1 [-] "foo(bar)" in list "<project>"
```

### WB-BKM-004 case 1: bookmark follows inserted lines
settings: default
language: plain
```before
--- a.txt
l1
l2
l3<caret>
bookmarks: a.txt:3 [-] "l3"
```
keys: Ctrl+Home, Enter, Enter
```after
--- a.txt


l1
l2
l3
bookmarks: a.txt:5 [-] "l3"
```

### WB-BKM-005 case 1: digit mnemonic toggles and jumps
settings: default
language: plain
```before
--- a.txt
line1<caret>
line2
line3
```
keys: Ctrl+Shift+1, Down, Down, Ctrl+1
```after
--- a.txt
<caret>line1
line2
line3
bookmarks: a.txt:1 [1] "line1"
```

### WB-BKM-005 case 2: same type toggles off, other type changes type
settings: default
language: plain
```before
--- a.txt
line1<caret>
bookmarks: a.txt:1 [1] "line1"
```
keys: Ctrl+Shift+2, Ctrl+Shift+2
```after
after first: bookmarks: a.txt:1 [2] "line1"
after second: bookmarks: none
```

### WB-BKM-006 case 1: chooser popup, type a letter
settings: default
language: plain
```before
--- a.txt
text<caret>
bookmarks: none
```
keys: Ctrl+F11, type "a"
```after
popup title was "Add Mnemonic Bookmark"; popup closed
bookmarks: a.txt:1 [A] "text"
```

### WB-BKM-006 case 2: change mnemonic popup title
settings: default
language: plain
```before
--- a.txt
text<caret>
bookmarks: a.txt:1 [A] "text"
```
keys: Ctrl+F11
```after
popup title: Change Mnemonic; cell "A" marked "Current"
```

### WB-BKM-007 case 1: rewrite a taken mnemonic (confirm)
settings: askBeforeRewritingMnemonic=true
language: plain
```before
--- a.txt
one
two<caret>
bookmarks: a.txt:1 [1] "one"
```
keys: Ctrl+Shift+1, confirm "Rewrite"
```after
dialog "Rewrite Mnemonic": ‘1’ mnemonic is already taken by ‘a.txt:1’. Do you want to rewrite it?   buttons: Rewrite, Cancel
bookmarks: a.txt:2 [1] "two"   (the former line bookmark a.txt:1 is removed)
```

### WB-BKM-007 case 2: rewrite cancelled
settings: askBeforeRewritingMnemonic=true
language: plain
```before
--- a.txt
one
two<caret>
bookmarks: a.txt:1 [1] "one"
```
keys: Ctrl+Shift+1, Cancel
```after
bookmarks: a.txt:1 [1] "one"   (unchanged)
```

### WB-BKM-008 case 1: bookmarks popup
settings: default
language: plain
```before
bookmarks: a.txt:1 [-] "one", b.txt:4 [B] "four", c.txt:2 [-] "two"
```
keys: Shift+F11
```after
popup "Bookmarks" rows:
  one      a.txt, line 1
  four  [B] b.txt, line 4
  two      c.txt, line 2
```

### WB-BKM-009 case 1: go to mnemonic popup
settings: default
language: plain
```before
bookmarks: b.txt:4 [B] "four"; current file a.txt
```
keys: Ctrl+Shift+F11, type "b"
```after
editor shows b.txt with caret on line 4
```

### WB-BKM-010 case 1: next line bookmark cycles
settings: default
language: plain
```before
bookmarks: a.txt:3, a.txt:9, b.txt:1 ; caret a.txt:9
```
keys: action GotoNextBookmark, action GotoNextBookmark
```after
after first: caret b.txt:1
after second: caret a.txt:3
```

### WB-BKM-010 case 2: previous wraps to the last
settings: default
language: plain
```before
bookmarks: a.txt:3, a.txt:9, b.txt:1 ; caret a.txt:3
```
keys: action GotoPreviousBookmark
```after
caret b.txt:1
```

### WB-BKM-011 case 1: empty state
settings: default
language: plain
```before
no bookmarks; Bookmarks window open
```
keys: Alt+2
```after
empty text:
  No bookmarks added.
  Add Bookmark (F11) to code lines,
  symbols, files, and directories.
```

### WB-BKM-012 case 1: create list
settings: default
language: plain
```before
Bookmarks window focused
```
keys: Alt+Insert
```after
dialog "Create Bookmark List": label "Bookmark list:", value "New List", checkbox "Use as default list" unchecked, buttons Create / Cancel
```

### WB-BKM-012 case 2: duplicate name
settings: list "Todo" exists
language: plain
```before
dialog "Create Bookmark List" open
```
keys: type "Todo"
```after
error under the field: A list with the specified name already exists; Create disabled
```

### WB-BKM-013 case 1: delete list confirmation
settings: askBeforeDeletingLists=true
language: plain
```before
Bookmarks window, selection list "Todo" with 2 bookmarks
```
keys: Delete
```after
dialog "Delete Bookmark List": Are you sure you want to delete 'Todo' bookmark list? This action can't be undone.   buttons: Delete, Cancel
```

### WB-BKM-014 case 1: mark list as default
settings: lists in order: P (default), Q, R
language: plain
```before
Bookmarks window, selection R
```
keys: menu Mark List as Default
```after
order: R (Default), P, Q
```

### WB-BKM-016 case 1: add to another list
settings: lists P (default), Q; bookmark a.txt:1 in P
language: plain
```before
--- a.txt
one<caret>
```
keys: action AddAnotherBookmark, choose Q
```after
a.txt:1 listed in P and Q
```

### WB-STR-001 case 1: structure follows the editor
settings: default
language: java
```before
tabs A.java (class A with foo()), B.java (class B with bar()); A.java active; Structure window open
```
keys: click tab B.java
```after
Structure tree: B > bar()
```

### WB-STR-003 case 1: hidden window is rebuilt on show
settings: default
language: java
```before
Structure window hidden; active file switches from A.java to B.java
```
keys: Alt+7
```after
Structure tree: B > bar()
```

### WB-STR-006 case 1: Enter navigates and focuses the editor
settings: default
language: java
```before
--- A.java
class A { void foo() {} void bar() {} }
Structure window focused, selection bar()
```
keys: Enter
```after
caret on bar in A.java; focus: editor
```

### WB-STR-021 case 1: narrow down default
settings: default
language: java
```before
popup File Structure open for class with foo(), bar()
```
keys: type "fo"
```after
visible: class, foo(); bar() hidden (narrow down on typing is on by default)
```

### WB-STR-025 case 1: unavailable without structure
settings: default
language: plain
```before
an image file is open
```
keys: Ctrl+F12
```after
no popup opens
```

### WB-PRB-001 case 1: tabs and default selection
settings: default
language: plain
```before
Problems window closed
```
keys: Alt+6
```after
tabs in order: File, Project Errors; selected: File
```

### WB-PRB-008 case 1: project errors for an unopened file
settings: default
language: java
```before
B.java contains an error, is not open; A.java is open and clean
```
keys: Alt+6, select tab "Project Errors"
```after
tree lists B.java with its error
```

### WB-TODO-002 case 1: only comments count
settings: default
language: java
```before
--- A.java
class A { String s = "TODO x"; // TODO real
}
```
keys: open TODO, tab Project
```after
Found 1 TODO item in 1 file
  TODO real
```

### WB-TODO-004 case 1: current file tab
settings: default
language: java
```before
A.java has "// TODO a"; B.java has "// TODO b"; editor shows A.java
```
keys: TODO window, tab "Current File"
```after
only: TODO a
```

### WB-TODO-006 case 1: F4 jumps
settings: default
language: java
```before
--- A.java
class A {
  // TODO a
}
TODO tree focused, selection "TODO a"
```
keys: F4
```after
caret at the start of TODO a (line 2), focus: editor
```

### WB-BKM-015 case 1: edit description
settings: default
language: plain
```before
bookmark a.txt:1 [-] "one"
```
keys: Bookmarks window, select it, F2, type "first", Enter
```after
bookmark a.txt:1 [-] "first"
```

### WB-BKM-021 case 1: deleted file
settings: default
language: plain
```before
bookmark a.txt:1
```
keys: delete a.txt on disk
```after
bookmark kept, drawn invalid; navigation disabled
```

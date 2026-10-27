# Cases for 01 (changes, changelists, file status, gutter markers)

Notation used by all VCS case files. `language:` is `plain` unless a language matters. A `before`
block lists the starting state, one fact per line: `repo:` (Git repository: branch and commits,
newest first, `hash "subject"`), `tree:` (tracked files in HEAD), `work:` (working tree status per
file: `M` modified, `A` added, `D` deleted, `?` unversioned, `!` ignored, `R old -> new`),
`index:` (staged files, only when staging matters), `lists:` (changelists as
`name[*=active]: files`), `ui:` (open window/tab/selection) and `settings:`. An `after` block lists the
expected state or, for UI behavior, the visible contents in order, top to bottom or left to right.
`keys:` lists keystrokes or `action <id>`; `type "x"` types text, `click "label"` presses a button,
`choose "item"` selects a list entry.

### VCS-CHG-001 case 1: first changelist
settings: default
language: plain
```before
repo: main (a1 "Initial commit")
tree: src/A.java
work: M src/A.java
lists: (none yet)
ui: Commit tool window, Local Changes tab
```
keys: action ChangesView.Refresh
```after
Local Changes tree, in order:
  Changes (1 file)  [active, bold]
    src (1 file)
      A.java  [modified color 0033B3 light]
  Unversioned Files (0 files) is not shown
```

### VCS-CHG-002 case 1: assignment follows the active list at detection time
settings: default
language: plain
```before
repo: main (a1 "Initial commit")
tree: A.java, B.java
work: M A.java
lists: Changes*: A.java; Fix: (empty)
```
keys: action ChangesView.SetDefault on "Fix", then modify B.java in an editor and save
```after
lists: Changes: A.java; Fix*: B.java
tree order: Changes (1 file), Fix (1 file) [Fix bold]
```

### VCS-CHG-003 case 1: new changelist dialog
settings: default
language: plain
```before
ui: Local Changes tab focused
```
keys: action ChangesView.NewChangeList
```after
dialog "New Changelist":
  field "Name:" (empty, focused)
  field "Comment:" (empty, multi-line)
  checkbox "Set active" (unchecked)
  buttons: OK (disabled while Name is empty), Cancel
```

### VCS-CHG-004 case 1: duplicate and blank names
settings: default
language: plain
```before
lists: Changes*: (empty); Fix: (empty)
ui: dialog "New Changelist"
```
keys: type "Fix"
```after
dialog stays open, error text under the field: "A changelist with that name already exists"
OK disabled
```

### VCS-CHG-004 case 2: blank name
settings: default
language: plain
```before
ui: dialog "New Changelist", Name field empty
```
keys: type "   "
```after
error text: "Cannot create new changelist with empty name"
OK disabled
```

### VCS-CHG-005 case 1: rename with F2
settings: default
language: plain
```before
lists: Changes*: A.java
ui: Local Changes, node "Changes" selected
```
keys: F2, select all in Name, type "Work", Enter
```after
dialog titled "Edit Changelist" opened with Name "Changes" before typing
lists: Work*: A.java
```

### VCS-CHG-006 case 1: set active with Ctrl+Space
settings: default
language: plain
```before
lists: Changes*: A.java; Fix: B.java
ui: node "Fix" selected
```
keys: Ctrl+Space
```after
lists: Changes: A.java; Fix*: B.java
```

### VCS-CHG-007 case 1: move with Alt+Shift+M
settings: default
language: plain
```before
lists: Changes*: A.java, B.java; Fix: (empty)
ui: A.java and B.java selected
```
keys: Alt+Shift+M, choose "Fix"
```after
popup before choosing, in order: "Changes", "Fix", "New Changelist..."
lists: Changes*: (empty); Fix: A.java, B.java
```

### VCS-CHG-008 case 1: moving an unversioned file adds it
settings: default
language: plain
```before
work: ? n.txt
lists: Changes*: (empty); Fix: (empty)
ui: Unversioned Files node expanded, n.txt selected
```
keys: Alt+Shift+M, choose "Fix"
```after
index: A n.txt
lists: Fix: n.txt (status added, green)
Unversioned Files node is gone
```

### VCS-CHG-009 case 1: delete a non-active changelist
settings: default
language: plain
```before
lists: Changes*: A.java; Fix: B.java
ui: node "Fix" selected
```
keys: action ChangesView.RemoveChangeList
```after
confirmation dialog titled "Delete Changelist" with text
  "Are you sure want to remove changelist 'Fix'?"
  "All changes will be moved to the active changelist."
after confirming: lists: Changes*: A.java, B.java
```

### VCS-CHG-010 case 1: delete the active changelist
settings: default
language: plain
```before
lists: Changes*: A.java; Fix: B.java; Other: (empty)
ui: node "Changes" selected
```
keys: action ChangesView.RemoveChangeList
```after
dialog "Delete Active Changelist" with prompt
  "Select the changelist to which the changes should be moved:"
  chooser entries: "Fix", "Other"
choose "Fix" -> lists: Fix*: B.java, A.java (A.java now in Fix); Other: (empty)
```

### VCS-CHG-014 case 1: prompt for empty inactive list
settings: remove empty inactive changelists = Ask (default)
language: plain
```before
lists: Changes*: A.java; Fix: B.java
```
keys: commit B.java's list contents (select Fix, Commit), then set Changes active
```after
dialog titled "Remove Empty Changelist":
  "The empty changelist 'Fix' is no longer active."
  "Do you want to remove it?"
  buttons: Remove, Cancel (checkbox "Remember my choice")
```

### VCS-CHG-020 case 1: tree layout with unversioned files
settings: default
language: plain
```before
tree: src/A.java, src/B.java, lib/C.java
work: M src/A.java, M src/B.java, M lib/C.java, ? notes.txt, ? tmp.txt
lists: Changes*: all three
```
keys: (view opened)
```after
Local Changes tree, top to bottom:
  Changes (3 files)
    lib (1 file)
      C.java
    src (2 files)
      A.java
      B.java
  Unversioned Files (2 files)
    notes.txt
    tmp.txt
```

### VCS-CHG-021 case 1: group by module and repository toggles
settings: default
language: plain
```before
ui: view options popup (eye icon) open
```
keys: (popup listing)
```after
popup items in order:
  "Group By" (submenu): Directory (checked, Ctrl+Alt+P), Module (Ctrl+Alt+M), Repository
  separator "Show"
  "Ignored Files" (unchecked)
```

### VCS-CHG-022 case 1: ignored files node
settings: default
language: plain
```before
work: ! build/out.bin
ui: Local Changes
```
keys: toggle "Ignored Files" in view options
```after
tree gains node "Ignored Files (1 file)" containing build/out.bin [color 8C4F00 light]
```

### VCS-CHG-025 case 1: double click opens editor preview
settings: default
language: plain
```before
ui: Local Changes, A.java node
```
keys: double click A.java
```after
a diff opens in an editor tab; title "Changes: A.java"; the project file A.java is not opened in an editor
```

### VCS-CHG-041 case 1: add-file prompt
settings: When files are created = Ask
language: plain
```before
repo: main (a1 "Initial commit")
ui: Project view, folder src selected
```
keys: action NewFile, type "New.java", Enter
```after
dialog titled "Add File to Git":
  "Do you want to add the following file to Git?"
  "<path>/src/New.java"
  ""
  "If you choose Cancel, you can still add it later manually."
  buttons: Add, Cancel
```

### VCS-CHG-042 case 1: delete prompt
settings: When files are deleted = Ask
language: plain
```before
tree: src/Old.java
ui: Project view, Old.java selected
```
keys: Delete, confirm "OK"
```after
dialog titled "Delete File from Git":
  "Do you want to delete the following file from Git?"
  "<path>/src/Old.java"
  ""
  "If you choose Cancel, you can still delete it later manually."
```

### VCS-CHG-045 case 1: add to .gitignore creating the file
settings: default
language: plain
```before
work: ? out/ (directory with files)
tree: (no .gitignore)
```
keys: right click out/ > Git > Add to .gitignore
```after
dialog "Create .gitignore": "Create .gitignore file in <project>?"
after OK: file .gitignore content "out/\n" (one line), out/ disappears from Unversioned Files
```

### VCS-CHG-051 case 1: status colors light theme
settings: light theme (expUI)
language: plain
```before
work: M m.txt, A a.txt (staged), D d.txt, ? u.txt, ! i.txt, conflict c.txt
```
keys: (Project view rendering)
```after
foreground colors: m.txt 0033B3, a.txt 067D17, d.txt 6C707E, u.txt B23247, i.txt 8C4F00, c.txt DE1B2E
```

### VCS-CHG-051 case 2: status colors dark theme
settings: dark theme (expUI)
language: plain
```before
work: M m.txt, A a.txt, D d.txt, ? u.txt, ! i.txt, conflict c.txt
```
keys: (Project view rendering)
```after
foreground colors: m.txt 70AEFF, a.txt 73BD79, d.txt 6F737A, u.txt E88F89, i.txt D69A6B, c.txt DE6A66
```

### VCS-CHG-061 case 1: debounce
settings: default
language: plain
```before
work: clean
```
keys: save 100 files within 200 ms
```after
at most one provider run is started during the first 300 ms after the last save; after it the tree lists all 100 files
```

### VCS-CHG-070 case 1: gutter markers
settings: default (light theme)
language: java
```before
file A.java in HEAD:
1: class A {
2:   int a;
3:   int b;
4: }
editor text:
class A {
  int a2;
  int b;
  int c;
}
```
keys: (open editor)
```after
gutter bars (left of the line numbers, 4 px wide, rounded):
  line 2: modified, color 88adf7
  line 4: added, color 7fc784
```

### VCS-CHG-070 case 2: deleted lines marker
settings: default (dark theme)
language: java
```before
HEAD:      class A {\n  int a;\n  int b;\n}
editor:    class A {\n  int b;\n}
```
keys: (open editor)
```after
a small deleted marker of height 8 px centered on the boundary between line 1 and line 2, color 868a91
```

### VCS-CHG-074 case 1: next change marker
settings: default
language: java
```before
editor with three changed ranges at lines 3, 10, 20; caret at line 1
```
keys: Shift+Ctrl+Alt+Down
```after
caret/scroll at line 3, popup for the change at line 3 open with toolbar: Rollback, Next Change, Previous Change, Copy, Show Diff, Highlight Words (Previous Change disabled)
```

### VCS-CHG-075 case 1: popup toolbar for a modified line
settings: default
language: java
```before
HEAD line: "int a;"  editor line: "int a2;"  popup opened by clicking the marker
```
keys: (popup)
```after
popup, top to bottom: toolbar [Rollback (Ctrl+Alt+Z), Next Change, Previous Change, Copy, Show Diff (Ctrl+D), Highlight Words (toggle, on)]
below: read-only editor showing "int a;" with "a" highlighted as word change
```

### VCS-CHG-076 case 1: rollback lines
settings: default
language: java
```before
HEAD: "int a;"   editor: "int a2;<caret>"   popup open
```
keys: Ctrl+Alt+Z
```after
editor line: "int a;<caret>"
undo stack top is named "Rollback Change"
```

### VCS-CHG-077 case 1: show diff for lines
settings: default
language: java
```before
HEAD: lines 1..5, editor changes line 3 only; popup open on that range
```
keys: Ctrl+D
```after
diff window titled "Diff for Range", side titles "Base revision" | "Current", each showing lines 2..4 (range plus one context line)
```

# Cases for 08-file-operations-and-scratches

`disk:` lists files on disk with their bytes in hex or text; `scratch dir:` is the IDE scratch directory. Editor markers as in the other case files.

### WB-FILE-001 case 1: Save All writes every modified document
settings: default
language: plain
```before
--- a.txt (disk: "one")
one<caret>
--- b.txt (disk: "two")
two
```
keys: type "!" in a.txt, switch to b.txt, type "?" at the end, Ctrl+S
```after
disk: a.txt = "one!", b.txt = "two?"
no dirty markers on either tab
```

### WB-FILE-002 case 1: Save File saves only the current document
settings: default
language: plain
```before
--- a.txt (disk: "one")
one<caret>
--- b.txt (disk: "two")
two
```
keys: type "!" in a.txt, switch to b.txt, type "?", switch back to a.txt, run action SaveDocument
```after
disk: a.txt = "one!", b.txt = "two"
tab b.txt still shows the dirty marker
```

### WB-FILE-004 case 1: cannot save a read-only file
settings: default
language: plain
```before
--- a.txt (opened writable, then made read-only on disk by another program)
one<caret>
```
keys: type "!", Ctrl+S
```after
dialog "Cannot Save Files": Following errors occurred on attempt to save files: (error line for a.txt)
buttons: Revert Changes, Ignore Changes
```

### WB-FILE-004 case 2: Ignore Changes keeps the edit
settings: default
language: plain
```before
dialog "Cannot Save Files" shown for a.txt with the typed "!"
```
keys: click Ignore Changes
```after
editor text: one!   tab a.txt still dirty; disk unchanged
```

### WB-FILE-004 case 3: Revert Changes drops the edit
settings: default
language: plain
```before
dialog "Cannot Save Files" shown for a.txt with the typed "!"
```
keys: click Revert Changes
```after
editor text: one   (disk content); tab not dirty
```

### WB-FILE-005 case 1: reload an unmodified file silently
settings: sync on frame activation off
language: plain
```before
--- a.txt (disk: "one")
one<caret>
```
keys: change disk content to "two" with another program, Ctrl+Alt+Y
```after
editor text: two; no dialog
```

### WB-FILE-007 case 1: file cache conflict
settings: default
language: plain
```before
--- a.txt (disk: "one")
one<caret>
```
keys: type "!" (unsaved), change disk content to "two", activate the IDE window
```after
dialog "File Cache Conflict": Changes have been made to 'a.txt' in memory and on disk.
buttons: Load File System Changes, Keep Memory Changes, Show Difference
```

### WB-FILE-007 case 2: load disk changes
settings: default
language: plain
```before
conflict dialog shown (memory "one!", disk "two")
```
keys: click Load File System Changes
```after
editor text: two; tab not dirty
```

### WB-FILE-007 case 3: keep memory changes
settings: default
language: plain
```before
conflict dialog shown (memory "one!", disk "two")
```
keys: click Keep Memory Changes
```after
editor text: one!; tab dirty; disk still "two" until the next save, which overwrites it
```

### WB-FILE-007 case 4: show difference
settings: default
language: plain
```before
conflict dialog shown (memory "one!", disk "two")
```
keys: click Show Difference
```after
diff window with left side titled "File system content" (two) and right side titled "Memory content" (one!)
the window has a button "Save Memory Content"
```

### WB-FILE-008 case 1: automatic reload of unmodified editors
settings: sync on frame activation on
language: plain
```before
--- a.txt (disk: "one")
one<caret>
```
keys: change disk content to "two", switch to the IDE window
```after
editor text: two; no dialog
```

### WB-FILE-010 case 1: Force Refresh
settings: default
language: plain
```before
--- a.txt (disk: "one"); file is replaced on disk by content "two" with the same size and modification time
```
keys: select a.txt in the project view, Ctrl+Shift+F5
```after
editor text: two
```

### WB-FILE-020 case 1: new scratch file
settings: default
language: plain
```before
no scratch files
```
keys: Ctrl+Alt+Shift+Insert, choose "Plain text"
```after
scratch dir: scratch.txt (empty)
editor shows scratch.txt; Project view: Scratches and Consoles > scratches > scratch.txt
```

### WB-FILE-021 case 1: name uniqueness
settings: default
language: plain
```before
scratch dir: scratch.txt
```
keys: Ctrl+Alt+Shift+Insert, choose "Plain text", repeat once more
```after
scratch dir: scratch.txt, scratch_1.txt, scratch_2.txt
```

### WB-FILE-021 case 2: different language different extension
settings: default
language: java
```before
scratch dir: scratch.txt
```
keys: Ctrl+Alt+Shift+Insert, choose "Java"
```after
scratch dir: scratch.txt, scratch.java
```

### WB-FILE-022 case 1: from selection
settings: default
language: java
```before
--- A.java
class A {
  <selection>int a = 1;</selection>
}
```
keys: Ctrl+Alt+Shift+Insert, choose the first entry "From selection (Java)"
```after
scratch dir: scratch.java containing "int a = 1;"
editor: scratch.java
```

### WB-FILE-024 case 1: change file language
settings: default
language: plain
```before
--- scratch.txt
class A {}<caret>
```
keys: run action Scratch.ChangeLanguage, choose Java
```after
popup entries began with "Change Language (Plain text) to..."; after choosing Java the editor treats scratch.txt as Java (keyword highlighting for "class")
```

### WB-FILE-025 case 1: empty scratch popup
settings: no scratch files
language: plain
```before
(any editor)
```
keys: run action Scratch.ShowFilesPopup
```after
popup titled "Scratch Files" with empty text: No scratch files found
```

### WB-FILE-027 case 1: undo of creation
settings: default
language: plain
```before
scratch dir: empty
```
keys: Ctrl+Alt+Shift+Insert, choose "Plain text", Ctrl+Z
```after
scratch dir: empty
```

### WB-FILE-040 case 1: new class from template
settings: Java sources root src
language: java
```before
project view selection: src ; popup "New" open
```
keys: choose "Java Class", type "Foo", Enter
```after
--- src/Foo.java
public class Foo {
}
(editor opens Foo.java; project view selects Foo.java)
```

### WB-FILE-041 case 1: kind switching
settings: Java sources root src
language: java
```before
popup "New Java Class" with kind list: Class (selected), Interface, Record, Enum, Annotation
```
keys: type "Foo", Down
```after
selected kind: Interface (the kind list highlights Interface; Enter then creates Foo.java as an interface)
```

### WB-FILE-042 case 1: empty name
settings: Java sources root src
language: java
```before
popup "New Java Class", name empty
```
keys: Enter
```after
popup stays open; error text: File name cannot be empty
```

### WB-FILE-042 case 2: existing file
settings: src/Foo.java exists
language: java
```before
popup "New Java Class"
```
keys: type "Foo", Enter
```after
popup stays open; error text: A file with the name 'Foo.java' already exists
```

### WB-FILE-043 case 1: variable substitution
settings: template body "// ${NAME} ${YEAR}" for extension txt; current year 2026
language: plain
```before
New File from that template, directory docs/
```
keys: type "notes", Enter
```after
--- docs/notes.txt
// notes 2026
```

### WB-FILE-060 case 1: new file line separator
settings: new-file line separator = Windows (\r\n)
language: plain
```before
(no file)
```
keys: New File "a.txt", type "x", Enter, "y", Ctrl+S
```after
disk: a.txt bytes 78 0D 0A 79
status bar LineSeparator: CRLF
```

### WB-FILE-060 case 2: system dependent on Linux
settings: new-file line separator = System-Dependent, OS Linux
language: plain
```before
(no file)
```
keys: New File "a.txt", type "x", Enter, "y", Ctrl+S
```after
disk: a.txt bytes 78 0A 79
status bar LineSeparator: LF
```

### WB-FILE-061 case 1: existing file keeps its separator
settings: new-file line separator = Windows (\r\n)
language: plain
```before
disk: old.txt bytes 61 0A 62 (LF)
```
keys: open old.txt, type "c" at the end, Ctrl+S
```after
disk: old.txt bytes 61 0A 62 63; status bar LineSeparator: LF
```

### WB-FILE-062 case 1: BOM wins over the project encoding
settings: project encoding ISO-8859-1
language: plain
```before
disk: bom.txt bytes EF BB BF 61 C3 A9 (UTF-8 with BOM, "aé")
```
keys: open bom.txt
```after
editor text: aé ; status bar Encoding: UTF-8; tooltip: File Encoding: UTF-8 (BOM-related reason text)
```

### WB-FILE-063 case 1: add BOM
settings: default
language: plain
```before
disk: a.txt bytes 61 (UTF-8 without BOM)
```
keys: File > File Properties > Add BOM, Ctrl+S
```after
disk: a.txt bytes EF BB BF 61
```

### WB-FILE-063 case 2: remove BOM
settings: default
language: plain
```before
disk: a.txt bytes EF BB BF 61 (UTF-8 with BOM, encoding not mandatory BOM)
```
keys: File > File Properties > Remove BOM, Ctrl+S
```after
disk: a.txt bytes 61
```

### WB-FILE-065 case 1: BOM policy for new UTF-8 files
settings: "Create UTF-8 files" = with BOM, encoding UTF-8
language: plain
```before
(no file)
```
keys: New File "a.txt", type "x", Ctrl+S
```after
disk: a.txt bytes EF BB BF 78
```

### WB-FILE-064 case 1: toggle read-only
settings: default
language: plain
```before
--- a.txt (writable)
x<caret>
```
keys: File > File Properties > Toggle Read-Only Attribute, type "y"
```after
file attribute read-only; editor text: x (typing rejected)
```

### WB-FILE-070 case 1: chooser shortcuts
settings: default
language: plain
```before
Open File or Project chooser, directory ~/work/demo shown
```
keys: Ctrl+1, then Backspace
```after
after Ctrl+1: directory ~ (home)
after Backspace: parent of ~
```

### WB-FILE-006 case 1: reload a subtree
settings: default
language: plain
```before
docs/a.txt and src/b.txt both changed externally
```
keys: select docs, Reload from Disk
```after
docs/a.txt updated; src/b.txt not yet
```

### WB-FILE-009 case 1: sync on activation
settings: sync on frame activation on
language: plain
```before
a.txt open, changed on disk
```
keys: activate the window
```after
editor shows disk text
```

### WB-FILE-023 case 1: scratch survives restart
settings: default
language: plain
```before
scratch.txt with "x"
```
keys: restart
```after
scratch.txt with "x" listed under Scratches and Consoles
```

### WB-FILE-029 case 1: cannot create
settings: scratch dir read-only
language: plain
```before
none
```
keys: Ctrl+Alt+Shift+Insert, Plain text
```after
dialog titled Error: Could not create file 'scratch.txt'
```

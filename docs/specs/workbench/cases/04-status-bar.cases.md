# Cases for 04-status-bar

`status bar:` lines give the text of the named widget after the keys. The editor tab size is 4 and tab characters are real `\t` where written `<tab>`.

### WB-SB-020 case 1: plain position
settings: default
language: plain
```before
ab
cd<caret>e
```
keys: (none)
```after
status bar Position: 2:3
```

### WB-SB-020 case 2: position after a tab uses the expanded column
settings: tab size 4
language: plain
```before
<tab>x<caret>
```
keys: (none)
```after
status bar Position: 1:6
```

### WB-SB-020 case 3: selection with line break
settings: default
language: plain
```before
<selection>ab
cd</selection><caret>
```
keys: (none)
```after
status bar Position: 2:3 (5 chars, 1 line break)
```

### WB-SB-020 case 4: single character selection, no line break
settings: default
language: plain
```before
a<selection>b</selection>c
```
keys: (none; caret at the selection end)
```after
status bar Position: 1:3 (1 char)
```

### WB-SB-020 case 5: two line breaks
settings: default
language: plain
```before
<selection>a
b
c</selection>
```
keys: (none; caret at the selection end)
```after
status bar Position: 3:2 (5 chars, 2 line breaks)
```

### WB-SB-020 case 6: several carets
settings: default
language: plain
```before
<caret>aa
<caret>bb
<caret>cc
```
keys: (none)
```after
status bar Position: 3 carets
```

### WB-SB-020 case 7: no editor
settings: default
language: plain
```before
all editors closed
```
keys: (none)
```after
status bar: Position, LineSeparator, Encoding, Indentation widgets are not shown
```

### WB-SB-020 case 8: surrogate pairs count as one char
settings: default
language: plain
```before
x<selection>😀😀</selection>
```
keys: (none; caret at the selection end)
```after
status bar Position text ends with: (2 chars)
```
(the count is in Unicode code points, not UTF-16 units)

### WB-SB-022 case 1: dialog prefill
settings: default
language: plain
```before
l1
l2
l3<caret>
```
keys: Ctrl+G
```after
dialog title: Go to Line:Column
label: [Line] [:column]:
field: 3:3 (all text selected)
```

### WB-SB-022 case 2: line and column
settings: default
language: plain
```before
abcdef
ghijkl<caret>
mnopqr
```
keys: Ctrl+G, type "1:3", Enter
```after
ab<caret>cdef
ghijkl
mnopqr
```

### WB-SB-022 case 3: line only goes to column 1
settings: default
language: plain
```before
abcdef
ghijkl<caret>
mnopqr
```
keys: Ctrl+G, type "3", Enter
```after
abcdef
ghijkl
<caret>mnopqr
```

### WB-SB-022 case 4: column only stays on the current line
settings: default
language: plain
```before
abcdef
ghi<caret>jkl
mnopqr
```
keys: Ctrl+G, type ":5", Enter
```after
abcdef
ghij<caret>kl
mnopqr
```

### WB-SB-022 case 5: comma separator
settings: default
language: plain
```before
a
b<caret>
c
d
```
keys: Ctrl+G, type "4,1", Enter
```after
a
b
c
<caret>d
```

### WB-SB-022 case 6: relative movement
settings: default
language: plain
```before
a
b
c<caret>
d
```
keys: Ctrl+G, type "-2", Enter
```after
<caret>a
b
c
d
```
(line 3 minus 2 = line 1, column 1)

### WB-SB-022 case 7: relative movement is clamped
settings: default
language: plain
```before
a
b<caret>
c
```
keys: Ctrl+G, type "+50", Enter
```after
a
b
<caret>c
```

### WB-SB-022 case 8: invalid input keeps the dialog open
settings: default
language: plain
```before
a<caret>b
```
keys: Ctrl+G, type "x1", Enter
```after
dialog still open, caret unchanged
```

### WB-SB-023 case 1: line beyond the end
settings: default
language: plain
```before
a<caret>
b
c
```
keys: Ctrl+G, type "99:1", Enter
```after
a
b
<caret>c
```

### WB-SB-022 case 9: selection and secondary carets are dropped
settings: default
language: plain
```before
<selection>ab</selection>
c<caret>d
```
keys: Ctrl+G, type "1:1", Enter
```after
<caret>ab
cd
```

### WB-SB-024 case 1: Escape cancels
settings: default
language: plain
```before
ab<caret>c
```
keys: Ctrl+G, type "1:1", Escape
```after
ab<caret>c
```

### WB-SB-030 case 1: line separator text and tooltip
settings: default
language: plain
```before
file a.txt saved with CRLF line breaks, open in the editor
```
keys: (none)
```after
status bar LineSeparator: CRLF; tooltip: Line Separator: \r\n
```

### WB-SB-031 case 1: popup contents
settings: default
language: plain
```before
file a.txt with LF line breaks
```
keys: click the LineSeparator widget
```after
popup "Line Separator":
  CRLF - Windows (\r\n)
  LF - Unix and macOS (\n)  (checked)
  CR - Classic Mac OS (\r)
```

### WB-SB-032 case 1: convert LF to CRLF
settings: default
language: plain
```before
--- a.txt (on disk: "a\nb\n")
a
b<caret>
```
keys: click the LineSeparator widget, choose "CRLF - Windows (\r\n)", Ctrl+S
```after
status bar LineSeparator: CRLF
file bytes on disk: 61 0D 0A 62 0D 0A
```

### WB-SB-033 case 1: undo restores
settings: default
language: plain
```before
--- a.txt (LF) converted to CRLF as in the previous case, not yet saved
```
keys: Ctrl+Z
```after
status bar LineSeparator: LF
```

### WB-SB-040 case 1: encoding text
settings: default
language: plain
```before
file a.txt open, charset UTF-8
```
keys: (none)
```after
status bar Encoding: UTF-8; tooltip: File Encoding: UTF-8
```

### WB-SB-041 case 1: silent change for ASCII-only text
settings: default
language: plain
```before
file a.txt contains only ASCII "hello", charset UTF-8
```
keys: click the Encoding widget, choose "Change encoding to 'windows-1252'"
```after
no dialog; status bar Encoding: windows-1252; text unchanged
```

### WB-SB-042 case 1: incompatible encoding dialog
settings: default
language: plain
```before
file a.txt on disk bytes C3 A9 ("é" in UTF-8), charset UTF-8, editor shows "é"
```
keys: click the Encoding widget, choose "Change encoding to 'ISO-8859-1'"
```after
dialog titled "Incompatible Encoding: a.txt" or "a.txt: Reload or Convert to ISO-8859-1"
text: The encoding you've chosen ('ISO-8859-1') may change the contents of 'a.txt'. Do you want to 1. Reload the file from disk in the new encoding 'ISO-8859-1' and overwrite editor contents or 2. Convert the text and overwrite file in the new encoding?
buttons: Reload, Convert, Cancel
```

### WB-SB-042 case 2: Reload shows mojibake, Convert preserves the text
settings: default
language: plain
```before
same as the previous case; dialog open
```
keys: press Reload
```after
editor text: Ã© (two characters); status bar Encoding: ISO-8859-1; disk bytes unchanged C3 A9
```

### WB-SB-042 case 3: Convert
settings: default
language: plain
```before
same as the previous case before Reload; dialog open
```
keys: press Convert
```after
editor text: é; status bar Encoding: ISO-8859-1; disk bytes E9
```

### WB-SB-042 case 4: convert refused for unsupported characters
settings: default
language: plain
```before
editor text: "日本" in a UTF-8 file; dialog open for 'ISO-8859-1'
```
keys: press Convert
```after
message: Encoding 'ISO-8859-1' does not support some characters from the text.; file and encoding unchanged
```

### WB-SB-050 case 1: indentation widget text
settings: default code style (indent 4, spaces)
language: java
```before
--- A.java
class A {}<caret>
```
keys: (none)
```after
status bar Indentation: 4 spaces; tooltip: Indent: 4 spaces
```

### WB-SB-050 case 2: tab indentation
settings: .editorconfig "indent_style = tab"
language: java
```before
--- A.java
class A {}<caret>
```
keys: (none)
```after
status bar Indentation: Tab
```

### WB-SB-050 case 3: one space
settings: .editorconfig "indent_style = space, indent_size = 1"
language: java
```before
--- A.java
class A {}<caret>
```
keys: (none)
```after
status bar Indentation: 1 space
```

### WB-SB-052 case 1: popup entries
settings: default
language: java
```before
--- A.java
class A {}<caret>
```
keys: click the Indentation widget
```after
popup title: Java Code Style
entries include: Configure Indents for Java...
```

### WB-SB-053 case 1: read-only file hides Indentation
settings: default
language: java
```before
--- A.java (file attribute read-only)
class A {}
```
keys: (none)
```after
status bar: Indentation widget not shown
```

### WB-SB-060 case 1: toggle read-only
settings: default
language: plain
```before
--- a.txt (writable)
x<caret>
```
keys: click the Read-Only Attribute widget, type "y"
```after
file a.txt is read-only (lock closed); the editor rejects the typing; text: x
```

### WB-SB-062 case 1: power save tooltip
settings: PowerSaveMode widget enabled, power save mode off
language: plain
```before
(any file)
```
keys: hover the PowerSaveMode widget, click it, hover again
```after
first tooltip: Power save mode: disabled
second tooltip: Power save mode: enabled
```

### WB-SB-003 case 1: widget toggled through the context menu
settings: default
language: plain
```before
any file open; status bar shows Position
```
keys: right click the status bar, uncheck "Line:Column Number"
```after
Position widget hidden; the entry stays in the menu unchecked; after restart still hidden
```

### WB-SB-010 case 1: defaults
settings: fresh profile
language: java
```before
--- A.java
class A {}<caret>
```
keys: right click the status bar
```after
checked: Line:Column Number, Line Separator, File Encoding, Indentation, Read-Only Attribute
unchecked: Power Save Mode, Memory Indicator, File System Sync
not offered: Notifications (stripes visible), Internal Error, Editor Selection Mode (not in column mode)
```

### WB-SB-065 case 1: notifications widget with hidden stripes
settings: hideToolStripes=true
language: plain
```before
(any file); a warning notification "x" arrives
```
keys: (none)
```after
Notifications widget shown with badge 1; tooltip: 1 notification pending
```

### WB-SB-011 case 1: order
settings: Power Save Mode and Memory Indicator enabled, column mode on, Git branch widget present
language: java
```before
--- A.java
class A {}<caret>
```
keys: (none)
```after
right side, left to right: Position, LineSeparator, Encoding, PowerSaveMode, Column, Indentation, git, ReadOnlyAttribute, Memory, settings entry point
```

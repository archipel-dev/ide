# Cases for 04-search-everywhere.md

Conventions: `popup:` describes the Search Everywhere popup (tab, field, filters); `list:` the ordered rows (a number after a row is its weight, from
`07-matching-and-ranking.md` and section 7 of the spec); `editor:` the editor state after navigation. Action rows are written `text [disabled]` when unavailable.

### SRCH-SE-002 case 1: two quick Shift presses open the All tab
settings: default
language: plain
```before
(editor focused)
```
keys: Shift, Shift (within the double-click interval)
```after
popup: open on tab All, field empty, recent files listed
```

### SRCH-SE-003 case 1: the shortcut of the selected tab toggles the scope
settings: default
language: plain
```before
(popup open on Classes)
```
keys: Ctrl+N
```after
popup: tab Classes, filter 'Include non-project items' on
```

### SRCH-SE-003 case 2: another tab's shortcut switches the tab and keeps the text
settings: default
language: plain
```before
(popup open on Classes with text 'main')
```
keys: Ctrl+Shift+N
```after
popup: tab Files, field 'main'
```

### SRCH-SE-004 case 1: a single-line selection becomes the text
settings: default
language: plain
```before
class A { int <selection>counter</selection>; }
```
keys: Ctrl+N
```after
popup: tab Classes, field 'counter' selected
```

### SRCH-SE-004 case 2: a multi-line selection is ignored, the history fills the field
settings: default
language: plain
```before
(history of Classes: HashMap) editor with a 2-line selection
```
keys: Ctrl+N
```after
popup: field 'HashMap' selected
```

### SRCH-SE-004 case 3: no selection, no history
settings: default
language: plain
```before
(empty history)
```
keys: Ctrl+Shift+A
```after
popup: tab Actions, field empty
```

### SRCH-SE-010 case 1: tab order
settings: default
language: plain
```before
(se.text.search on)
```
keys: Shift, Shift
```after
popup: tabs All, Classes, Files, Symbols, Actions, Text
```

### SRCH-SE-010 case 2: Tab cycles with wrap-around
settings: default
language: plain
```before
(popup on tab Text)
```
keys: Tab
```after
popup: tab All
```

### SRCH-SE-011 case 1: Actions tab filter
settings: default
language: plain
```before
(popup on Actions)
```
keys: look at the header
```after
popup: check box 'Include disabled actions' unchecked
```

### SRCH-SE-011 case 2: All tab filters
settings: default
language: plain
```before
(popup on All)
```
keys: look at the header
```after
popup: switch 'Include non-project items' off, toggle 'Preview', type filter menu
```

### SRCH-SE-011 case 3: the type filter excludes a provider
settings: default
language: plain
```before
(popup on All, text 'main')
```
keys: uncheck 'Files' in the type filter
```after
list: no file rows; the choice is remembered at the next opening
```

### SRCH-SE-012 case 1: auto-widening
settings: default
language: plain
```before
(no project class Zzz; a library has class Zzzx)
```
keys: Ctrl+N, type 'Zzz'
```after
popup: 'Include non-project items' switches on by itself; list: Zzzx
```

### SRCH-SE-012 case 2: switching back
settings: default
language: plain
```before
(after the previous case)
```
keys: select all, type 'Hash'
```after
popup: 'Include non-project items' off again
```

### SRCH-SE-014 case 1: Esc closes and saves the text
settings: default
language: plain
```before
(popup with text 'abc')
```
keys: Esc, then Ctrl+N
```after
popup: field 'abc' selected (history of the tab)
```

### SRCH-SE-015 case 1: history order
settings: default
language: plain
```before
(tab Classes)
```
keys: type 'a', Esc; type 'b', Esc; type 'a', Esc; Ctrl+N, Alt+Down
```after
popup: field 'b' (history oldest-to-newest: b, a; the field first showed 'a')
```

### SRCH-SE-015 case 2: Alt+Up goes newer and wraps
settings: default
language: plain
```before
(history b, a)
```
keys: Ctrl+N, Alt+Down, Alt+Down, Alt+Down
```after
popup: the field cycles through b, a, b
```

### SRCH-SE-016 case 1: project scope hides library classes
settings: default
language: plain
```before
(a library has class LibOnly)
```
keys: Ctrl+N, type 'LibOnly', manual toggle off
```after
list: empty
```

### SRCH-SE-016 case 2: an absolute path outside the project
settings: registry search.everywhere.absolute.path.outside.project=on
language: plain
```before
(file /tmp/x/notes.txt exists)
```
keys: Ctrl+Shift+N, type '/tmp/x/notes.txt'
```after
list: notes.txt 15000
```

### SRCH-SE-020 case 1: Searching text
settings: default
language: plain
```before
(slow provider)
```
keys: type 'abc'
```after
list: only the row text 'Searching…' until the first item arrives
```

### SRCH-SE-020 case 2: empty text on Classes
settings: default
language: plain
```before

```
keys: Ctrl+N
```after
popup: compact, no list
```

### SRCH-SE-021 case 1: mixed order by weight, ties by provider
settings: default
language: plain
```before
class HashMap 10990, file hashmap.txt 11001, action 'Hash Code' 11001
```
keys: type 'hash' on All
```after
list: hashmap.txt (Files 200), Hash Code (Actions 400), HashMap
```

### SRCH-SE-021 case 2: exact match inside one provider
settings: default
language: plain
```before
files map.txt 11003 and mapper.txt 11003
```
keys: type 'map.txt'
```after
list: map.txt first
```

### SRCH-SE-021 case 3: recent files pinned when the setting is on
settings: search.everywhere.recent.at.top=on
language: plain
```before
recent file zz.txt (weak match), class Zebra (strong)
```
keys: type 'z' on All
```after
list: zz.txt first
```

### SRCH-SE-021 case 4: a single-provider tab keeps the provider order
settings: default
language: plain
```before
Classes tab, corpus of the 07 case 'hm'
```
keys: type 'hm'
```after
list: HashMap, HashMapTest, ConcurrentHashMap, LinkedHashMap, MyHashMap
```

### SRCH-SE-022 case 1: a recent file that is also a file appears once
settings: default
language: plain
```before
recent: A.java; the file index has A.java
```
keys: type 'A.j' on All
```after
list: A.java once, at its Files position
```

### SRCH-SE-022 case 2: an action found as action and as option
settings: default
language: plain
```before
action 'Soft-Wrap Files' and its option
```
keys: type 'soft wrap' on All
```after
list: one row, the action
```

### SRCH-SE-023 case 1: frozen list
settings: default
language: plain
```before
results arrive at 100 ms and 900 ms; the late one is stronger
```
keys: type 'x'
```after
list: the late result appears below the rows already shown
```

### SRCH-SE-024 case 1: nothing found on a tab
settings: default
language: plain
```before
(no class named Qqq)
```
keys: Ctrl+N, type 'Qqq' (everywhere switched off by hand)
```after
list text: 'No classes with such a name found'
```

### SRCH-SE-024 case 2: nothing found on All
settings: default
language: plain
```before

```
keys: Shift, Shift, type 'Qqq'
```after
list text: 'Nothing found'
```

### SRCH-SE-024 case 3: dumb mode
settings: default
language: plain
```before
(indexing)
```
keys: Ctrl+N
```after
list text: "Search in 'Classes' will be available once project analysis is finished"
```

### SRCH-SE-025 case 1: the first row is auto-selected and stays
settings: default
language: plain
```before
classes HashMap, HashSet
```
keys: Ctrl+N, type 'hash'
```after
list: HashMap selected; the selection does not move when HashSet arrives
```

### SRCH-SE-025 case 2: group jump
settings: default
language: plain
```before
list: class A, class B, file C, file D
```
keys: Ctrl+Down on A
```after
list: selection on C
```

### SRCH-SE-026 case 1: restoring the selection
settings: default
language: plain
```before
(earlier: text 'hash', second row selected, popup closed)
```
keys: reopen the tab, type 'hash'
```after
list: the second row is selected again
```

### SRCH-SE-030 case 1: class row
settings: default
language: plain
```before
com.acme.util.StringUtil
```
keys: Ctrl+N, type 'SU'
```after
list: StringUtil (com.acme.util)
```

### SRCH-SE-030 case 2: same short name
settings: default
language: plain
```before
com.acme.User, org.other.User
```
keys: type 'User'
```after
list: both rows, closer package first, then qualified name
```

### SRCH-SE-030 case 3: qualified pattern
settings: default
language: plain
```before
com.acme.User, org.other.User
```
keys: type 'other.User'
```after
list: org.other.User 16053
```

### SRCH-SE-031 case 1: absolute path first
settings: default
language: plain
```before
project file <root>/src/A.java
```
keys: Ctrl+Shift+N, type '<root>/src/A.java'
```after
list: A.java first, 15000
```

### SRCH-SE-031 case 2: directory part
settings: default
language: plain
```before
the file corpus of 07 (UserService.java, UserController.java, user-service.properties)
```
keys: type 'src/main/Us'
```after
list: user-service.properties 996, UserController.java 992, UserService.java 992, other/UserService.java 991
```

### SRCH-SE-031 case 3: directories after files
settings: default
language: plain
```before
directory util, file Util.java
```
keys: type 'util'
```after
list: Util.java, util (Directory)
```

### SRCH-SE-031 case 4: file type filter
settings: default
language: plain
```before
A.java, a.txt
```
keys: Files tab, keep only Java checked, type 'a'
```after
list: A.java
```

### SRCH-SE-032 case 1: symbol row
settings: default
language: plain
```before
class Outer { void run() {} }
```
keys: Ctrl+Alt+Shift+N, type 'run'
```after
list: run()  Outer
```

### SRCH-SE-032 case 2: qualified symbol
settings: default
language: plain
```before
class Outer { void run(){} } class Other { void run(){} }
```
keys: type 'Outer.run'
```after
list: Outer.run()
```

### SRCH-SE-033 case 1: empty text lists recent files
settings: default
language: plain
```before
history: B.java (newest), A.java; A.java is open
```
keys: Shift, Shift
```after
list: B.java (A.java excluded)
```

### SRCH-SE-033 case 2: filtered by text
settings: default
language: plain
```before
history: Alpha.java, Beta.java
```
keys: type 'al'
```after
list: Alpha.java (Recent Files)
```

### SRCH-SE-034 case 1: option row
settings: default
language: plain
```before
settings option 'Show line numbers'
```
keys: Shift, Shift, type 'line numbers'
```after
list: 'Show line numbers' with a check box (weight reduced by 100)
```

### SRCH-SE-034 case 2: toggling an option
settings: default
language: plain
```before

```
keys: Enter on the option row
```after
the option flips; popup closes
```

### SRCH-SE-040 case 1: candidates
settings: default
language: plain
```before

```
keys: Ctrl+Shift+A, type 'find'
```after
list: Find…, Find Action…, Find in Files…, Find Next, Find Previous, Find Usages
```

### SRCH-SE-040 case 2: abbreviation
settings: default
language: plain
```before
abbreviation 'ff' for Find in Files
```
keys: type 'ff'
```after
list: abbreviation row for Find in Files first
```

### SRCH-SE-041 case 1: 'find'
settings: default
language: plain
```before
Find…, Find Next, Find Previous, Find Usages, Find in Files…, Find Action…, Replace…
```
keys: type 'find' on Actions
```after
Find…  11002
Find Action…  11001
Find in Files…  11001
Find Next  11001
Find Previous  11001
Find Usages  11001
```

### SRCH-SE-041 case 2: 'rename'
settings: default
language: plain
```before
Rename…, Rename File…, Rename Directory… [disabled]
```
keys: type 'rename' (Include disabled on)
```after
Rename…  11002
Rename File…  11001
Rename Directory… [disabled]  11001
```

### SRCH-SE-041 case 3: a space adds 100
settings: default
language: plain
```before
Go to Class…, Go to File…, Go to Symbol…, Go to Line:Column…
```
keys: type 'go to'
```after
Go to Class…  11103
Go to File…  11103
Go to Line:Column…  11103
Go to Symbol…  11103
```

### SRCH-SE-041 case 4: word humps
settings: default
language: plain
```before
Go to Class…, Go to Line:Column…
```
keys: type 'gtc'
```after
Go to Class…  10959
Go to Line:Column…  10939
```

### SRCH-SE-042 case 1: disabled after enabled with equal weight
settings: default
language: plain
```before
Rerun [disabled], Rename…
```
keys: type 're' (Include disabled on)
```after
Rename… 11001 before Rerun [disabled] 11001
```

### SRCH-SE-042 case 2: tie by text ascending
settings: default
language: plain
```before
the 31 sample actions of the reference run
```
keys: type 're'
```after
Recent Files, Recent Locations, Refactor This…, Reformat Code, Reformat Code with Dependencies, Reformat File…, Reload All from Disk, Rename…, Rename File…, Replace…, Replace in Files… (all 11001)
```

### SRCH-SE-043 case 1: abbreviation
settings: default
language: plain
```before
abbreviation 'ff'
```
keys: type 'ff'
```after
the abbreviation row is first
```

### SRCH-SE-044 case 1: row details
settings: default
language: plain
```before

```
keys: type 'find action'
```after
row: Find Action…, group 'Navigate', shortcut Ctrl+Shift+A
```

### SRCH-SE-044 case 2: toggle action row
settings: default
language: plain
```before

```
keys: type 'soft wrap'
```after
row with a check box showing the state; clicking toggles and keeps the popup open
```

### SRCH-SE-045 case 1: executing
settings: default
language: plain
```before
editor focused
```
keys: Ctrl+Shift+A, type 'reformat code', Enter
```after
popup closes, then Reformat Code runs on the editor's file
```

### SRCH-SE-045 case 2: a disabled action is only listed when included
settings: default
language: plain
```before
no editor focused
```
keys: type 'reformat code'
```after
list: Reformat Code [disabled] only with Include disabled actions on
```

### SRCH-SE-046 case 1: include disabled
settings: default
language: plain
```before

```
keys: Alt+D on the Actions tab
```after
check box checked; disabled actions appear after available ones
```

### SRCH-SE-050 case 1: Enter opens in the editor
settings: default
language: plain
```before
class A in A.java
```
keys: Ctrl+N, type 'A', Enter
```after
editor: A.java open, caret at the class name; popup closed
```

### SRCH-SE-050 case 2: Shift+Enter opens in the right split
settings: default
language: plain
```before

```
keys: type 'A.java', Shift+Enter
```after
editor: A.java in the right split
```

### SRCH-SE-050 case 3: multi-select opens both
settings: default
language: plain
```before
files A.java, B.java
```
keys: type '.java', Shift+Down, Enter
```after
both files open
```

### SRCH-SE-051 case 1: line suffix
settings: default
language: plain
```before
A.java with 20 lines
```
keys: Ctrl+Shift+N, type 'A.java:12', Enter
```after
editor: caret line 12 column 1
```

### SRCH-SE-051 case 2: line and column
settings: default
language: plain
```before

```
keys: type 'A.java:12:5', Enter
```after
editor: caret line 12 column 5
```

### SRCH-SE-051 case 3: suffix on a class
settings: default
language: plain
```before
class A in A.java
```
keys: Ctrl+N, type 'A:7', Enter
```after
editor: A.java at line 7
```

### SRCH-SE-052 case 1: member
settings: default
language: plain
```before
class Outer { void run(){} void walk(){} }
```
keys: Ctrl+N, type 'Outer#ru', Enter
```after
editor: caret on run
```

### SRCH-SE-060 case 1: command list
settings: default
language: plain
```before

```
keys: type '/'
```after
list: the commands; hint 'Type / to see commands'
```

### SRCH-SE-060 case 2: complete a command
settings: default
language: plain
```before

```
keys: type '/pl', Tab
```after
field '/plugins '
```

### SRCH-SE-061 case 1: text tab
settings: default
language: plain
```before
A.java line 3 contains needle
```
keys: Ctrl+Alt+Shift+E, type 'needle'
```after
list: A.java:3 with the match highlighted; toggles Match case, Words, Regex in the field
```

### SRCH-SE-061 case 2: Enter opens the match
settings: default
language: plain
```before

```
keys: Enter
```after
editor: A.java line 3, match selected
```


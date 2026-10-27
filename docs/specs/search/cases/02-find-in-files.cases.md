# Cases for 02-find-in-files.md

Conventions: `--- path` separates files of a multi-file project (the path is project-relative; `.idea/` and `*.iml` files are part of the project
content unless a case says otherwise). A line `open: <path>` in `before` says which file is open in the editor; `<caret>` and `<selection>` mark its caret.
`keys:` lists key presses; names of controls are their texts. In `after` blocks, `list:` introduces the ordered rows of the result list
(`path:line  text`), `label:` the info label, `empty:` the empty text lines, `dialog:` a dialog (title, then message, then buttons), `status:`
the status bar text. Rows show the matched text between square brackets for clarity only.

### SRCH-FIP-010 case 1: a single-line selection becomes the search text
settings: default
language: plain
```before
--- src/A.java
open: src/A.java
class A { int <selection>counter</selection> = 0; }
```
keys: Ctrl+Shift+F
```after
dialog: popup "Find in Files", mode find, search field "counter", scope In Project selected
```

### SRCH-FIP-010 case 2: a directory in the project view presets the Directory scope
settings: default
language: plain
```before
--- src/main/A.java
class A {}
--- src/test/B.java
class B {}
focus: Project view, directory src/main selected
```
keys: Ctrl+Shift+F
```after
dialog: popup "Find in Files", scope Directory selected, directory field "<project>/src/main", Recursively checked
```

### SRCH-FIP-010 case 3: a file in the project view presets the Directory of its parent
settings: default
language: plain
```before
--- src/main/A.java
class A {}
focus: Project view, file src/main/A.java selected
```
keys: Ctrl+Shift+F
```after
dialog: popup "Find in Files", scope Directory selected, directory field "<project>/src/main"
```

### SRCH-FIP-010 case 4: a module node presets the Module scope
settings: default
language: plain
```before
modules: app, lib
focus: Project view, module node "lib" selected
```
keys: Ctrl+Shift+F
```after
dialog: popup "Find in Files", scope Module selected, module combo "lib"
```

### SRCH-FIP-010 case 5: no context
settings: default
language: plain
```before
open: none
```
keys: Ctrl+Shift+F
```after
dialog: popup "Find in Files", scope In Project selected, empty search field, empty results with the text "Type search query to find in files"
```

### SRCH-FIP-011 case 1: In Project excludes excluded folders and libraries
settings: default
language: plain
```before
--- src/A.java
// needle
--- build/gen/G.java
// needle
--- lib/dep.txt
needle
```
setup: folder build is marked excluded; lib/dep.txt belongs to a library root, not to the project content
keys: Ctrl+Shift+F, type "needle"
```after
list:
src/A.java:1  // [needle]
label: 1 match in 1 file
```

### SRCH-FIP-011 case 2: Directory scope without Recursively
settings: default
language: plain
```before
--- src/main/A.java
needle
--- src/main/sub/B.java
needle
```
keys: Ctrl+Shift+F, choose scope Directory, directory "<project>/src/main", untick Recursively, type "needle"
```after
list:
src/main/A.java:1  [needle]
label: 1 match in 1 file
```

### SRCH-FIP-011 case 3: Directory scope, recursive
settings: default
language: plain
```before
--- src/main/A.java
needle
--- src/main/sub/B.java
needle
--- src/main/sub/target/C.java
needle
```
setup: folder src/main/sub/target is excluded
keys: Ctrl+Shift+F, choose scope Directory, directory "<project>/src/main", type "needle"
```after
list:
src/main/A.java:1  [needle]
src/main/sub/B.java:1  [needle]
label: 2 matches in 2 files
```

### SRCH-FIP-011 case 4: Module scope
settings: default
language: plain
```before
--- app/A.java
needle
--- lib/B.java
needle
```
setup: modules app (content root app) and lib (content root lib)
keys: Ctrl+Shift+F, choose scope Module, module "lib", type "needle"
```after
list:
lib/B.java:1  [needle]
label: 1 match in 1 file
```

### SRCH-FIP-012 case 1: binary files are skipped
settings: default
language: plain
```before
--- a.txt
needle
--- b.png
(binary bytes containing the ASCII text needle)
```
keys: Ctrl+Shift+F, type "needle"
```after
list:
a.txt:1  [needle]
label: 1 match in 1 file
```

### SRCH-FIP-012 case 2: project configuration files are searched by default
settings: default
language: plain
```before
--- .idea/misc.xml
<project name="needle"/>
--- src/A.java
// needle
```
keys: Ctrl+Shift+F, type "needle"
```after
list:
.idea/misc.xml:1  <project name="[needle]"/>
src/A.java:1  // [needle]
label: 2 matches in 2 files
```

### SRCH-FIP-013 case 1: unsaved editor text is searched
settings: default
language: plain
```before
--- src/A.java
open: src/A.java
class A { }<caret>
```
keys: type " // needle" (unsaved), Ctrl+Shift+F, type "needle"
```after
list:
src/A.java:1  class A { } // [needle]
```

### SRCH-FIP-020 case 1: one positive mask
settings: default
language: plain
```before
names: A.java, a.JAVA, A.javax, java, Main.kt, MyTestCase.java, notes.md
```
keys: mask "*.java"
```after
accepted: A.java, a.JAVA, MyTestCase.java
```

### SRCH-FIP-020 case 2: positive and negative entries
settings: default
language: plain
```before
names: app.js, app_min.js, app.ts
```
keys: mask "*.js, !*_min.js"
```after
accepted: app.js
```

### SRCH-FIP-020 case 3: only negative entries mean everything else
settings: default
language: plain
```before
names: index.html, test.xml, pom.xml, notes.md
```
keys: mask "!*.xml"
```after
accepted: index.html, notes.md
```

### SRCH-FIP-020 case 4: a star on both sides is case-insensitive
settings: default
language: plain
```before
names: MyTestCase.java, test.xml, notes.md
```
keys: mask "*test*"
```after
accepted: MyTestCase.java, test.xml
```

### SRCH-FIP-020 case 5: question mark is exactly one character
settings: default
language: plain
```before
names: foo1.txt, foo.txt, foo12.txt
```
keys: mask "foo?.txt"
```after
accepted: foo1.txt
```

### SRCH-FIP-020 case 6: the dot is literal, the whole name must match
settings: default
language: plain
```before
names: .gitignore, xgitignore, Makefile, makefile, Dockerfile
```
keys: mask ".gitignore, Makefile"
```after
accepted: .gitignore, Makefile, makefile
```

### SRCH-FIP-020 case 7: spaces around entries and several positives
settings: default
language: plain
```before
names: A.java, Main.kt, B.kt, notes.md
```
keys: mask "*.java, *.kt , !Main*"
```after
accepted: A.java, B.kt
```

### SRCH-FIP-020 case 8: characters that are special in regular expressions are literal
settings: default
language: plain
```before
names: a+b.c, a(1).c, ab.c, aab.c
```
keys: mask "a+b.c, a(1).c"
```after
accepted: a+b.c, a(1).c
```

### SRCH-FIP-021 case 1: empty mask
settings: default
language: plain
```before
(popup open, File mask ticked)
```
keys: type "needle", clear the file mask field
```after
dialog: popup; the mask field is marked with the message "Empty file mask"; no search runs
```

### SRCH-FIP-021 case 2: semicolons
settings: default
language: plain
```before
(popup open)
```
keys: tick File mask, mask "*.java;*.kt", type "needle"
```after
dialog: the mask field is marked with "File masks should be comma-separated"
```

### SRCH-FIP-021 case 3: masks are remembered most recent first
settings: default
language: plain
```before
(three searches done with the masks "*.java", "*.kt", "*.java")
```
keys: open the file mask drop-down
```after
dialog: drop-down entries top to bottom: *.java, *.kt
```

### SRCH-FIP-022 case 1: reset filters
settings: Match case on, Words on, File mask "*.java" ticked, context "In Comments"
language: plain
```before
(popup open)
```
keys: Alt+Shift+Backspace
```after
dialog: Match case off, Words off, Regex off, File mask box unticked, context "Anywhere"
```

### SRCH-FIP-022 case 2: In Comments context
settings: context="In Comments"
language: java
```before
--- A.java
// needle
class A { String s = "needle"; /* needle */ }
```
keys: Ctrl+Shift+F, choose context "In Comments", type "needle"
```after
list:
A.java:1  // [needle]
A.java:2  class A { String s = "needle"; /* [needle] */ }
label: 2 matches in 1 file
```

### SRCH-FIP-030 case 1: empty text
settings: default
language: plain
```before
(popup open)
```
keys: Ctrl+Shift+F
```after
empty: "Type search query to find in files"
```

### SRCH-FIP-030 case 2: invalid regex
settings: default
language: plain
```before
(popup open)
```
keys: Alt+X, type "("
```after
dialog: the search field is marked with the message `Bad pattern "(": <reason of the regex engine>`; no results
```

### SRCH-FIP-030 case 3: regex that matches the empty string
settings: default
language: plain
```before
(popup open)
```
keys: Alt+X, type "a*"
```after
dialog: the search field is marked with the message "Regular expression matches empty string"
```

### SRCH-FIP-030 case 4: an anchored regex that can match empty is allowed
settings: default
language: plain
```before
(popup open)
```
keys: Alt+X, type "^"
```after
dialog: no validation message; the search runs
```

### SRCH-FIP-030 case 5: missing directory
settings: default
language: plain
```before
(popup open)
```
keys: choose scope Directory, directory "/no/such/dir", type "x"
```after
dialog: the directory field is marked with "Directory is not found"
```

### SRCH-FIP-040 case 1: info label
settings: default
language: plain
```before
--- A.java
needle needle
--- B.java
x
--- C.java
needle
```
keys: Ctrl+Shift+F, type "needle"
```after
label: 3 matches in 2 files
```

### SRCH-FIP-040 case 2: one match and zero matches
settings: default
language: plain
```before
--- A.java
needle
```
keys: Ctrl+Shift+F, type "needle", then type "x" so that the text is "needlex"
```after
label: 0 matches in 0 files (after "needle" it was 1 match in 1 file)
empty: "Nothing found"
```

### SRCH-FIP-041 case 1: open files come first, earlier results stay on top
settings: default
language: plain
```before
--- a/A.java
needle
--- z/Z.java
open: z/Z.java
needle
```
keys: Ctrl+Shift+F, type "needle"
```after
list:
z/Z.java:1  [needle]
a/A.java:1  [needle]
```
(the file of the first reported result, here the open file, is pinned at the top)

### SRCH-FIP-042 case 1: paging
settings: usages page size = 100
language: plain
```before
--- big.txt
(250 lines, each containing the word needle)
```
keys: Ctrl+Shift+F, type "needle", scroll the list to its end
```after
list: 100 rows, then 200 rows after the scroll, then 250 rows after the next scroll
label: "100+ matches in 1+ files" after the first page; "250 matches in 1 file" when exhausted
```

### SRCH-FIP-043 case 1: rows are ordered by path, line, offset
settings: default
language: plain
```before
--- b/B.java
x
needle
--- a/A.java
needle
```
keys: Ctrl+Shift+F, type "needle"
```after
list:
a/A.java:1  [needle]
b/B.java:2  [needle]
selected row: 1
```

### SRCH-FIP-043 case 2: repeated file names are dimmed
settings: default
language: plain
```before
--- A.java
needle
needle
```
keys: Ctrl+Shift+F, type "needle"
```after
list:
A.java 1  [needle]
A.java 2  [needle]   (the file name of this row is drawn dimmed because it equals the previous row's)
```

### SRCH-FIP-044 case 1: preview texts
settings: default
language: plain
```before
--- A.java
needle
--- B.java
needle
```
keys: Ctrl+Shift+F, type "needle", Shift+Down
```after
dialog: preview shows "Several files selected. Select occurrences from one file to preview."
```

### SRCH-FIP-045 case 1: Enter opens the match and closes the popup
settings: default
language: plain
```before
--- A.java
class A {
  int needle = 1;
}
```
keys: Ctrl+Shift+F, type "needle", Enter
```after
--- A.java (editor)
class A {
  int <selection>needle<caret></selection> = 1;
}
dialog: closed
```

### SRCH-FIP-045 case 2: Shift+Enter opens in the right split
settings: default
language: plain
```before
--- A.java
needle
```
keys: Ctrl+Shift+F, type "needle", Shift+Enter
```after
editor: A.java open in the right split with the match selected; popup closed
```

### SRCH-FIP-046 case 1: nothing found, no options
settings: default
language: plain
```before
--- A.java
x
```
keys: Ctrl+Shift+F, type "needle"
```after
empty:
Nothing found
```

### SRCH-FIP-046 case 2: one option in use
settings: Match case on
language: plain
```before
--- A.java
Needle
```
keys: Ctrl+Shift+F, type "needle"
```after
empty:
Nothing found.
Search option used: Match case
Clear option Alt+Shift+Backspace
```

### SRCH-FIP-046 case 3: two options in use
settings: Match case on, Words on
language: plain
```before
--- A.java
needles
```
keys: Ctrl+Shift+F, type "needle"
```after
empty:
Nothing found.
Search options used:
Match case  Words
Clear all options Alt+Shift+Backspace
```

### SRCH-FIP-046 case 4: regex suggestion
settings: default
language: plain
```before
--- A.java
foo bar
```
keys: Ctrl+Shift+F, type "fo+ b"
```after
empty:
Nothing found.
Search with Regex Alt+X
```

### SRCH-FIP-046 case 5: non-recursive directory
settings: default
language: plain
```before
--- src/A.java
x
--- src/sub/B.java
needle
```
keys: Ctrl+Shift+F, choose scope Directory "<project>/src", untick Recursively, type "needle"
```after
empty:
Nothing found. Search recursively in subdirectories
```
(the second sentence is a link; activating it ticks Recursively and the list then shows src/sub/B.java:1)

### SRCH-FIP-060 case 1: Open in Find Window
settings: default
language: plain
```before
--- A.java
needle
--- B.java
needle
```
keys: Ctrl+Shift+F, type "needle", Ctrl+Enter
```after
dialog: popup closed
find tool window: tab "Occurrences of 'needle'", title "Occurrences of 'needle' in Project", 2 results in 2 files
```

### SRCH-FIP-060 case 2: with a file mask and a directory
settings: default
language: plain
```before
--- src/A.java
needle
```
keys: Ctrl+Shift+F, scope Directory "<project>/src", File mask "*.java", type "needle", Ctrl+Enter
```after
find tool window title: "Occurrences of 'needle' in Directory <project>/src with mask '*.java'"
```

### SRCH-FIP-070 case 1: replace the selected result
settings: default
language: plain
```before
--- A.java
foo
--- B.java
foo
```
keys: Ctrl+Shift+R, type "foo", Tab, type "bar", Enter
```after
--- A.java
bar
--- B.java
foo
list: B.java:1  [foo]   (the replaced row is removed and the row that moved into its place is selected)
```

### SRCH-FIP-070 case 2: malformed template
settings: regex on
language: plain
```before
--- A.java
foo
```
keys: Ctrl+Shift+R, type "(foo)", Tab, type "$3", Enter
```after
dialog: error "Replace Error": "Malformed replacement string: No group 3"
--- A.java
foo
```

### SRCH-FIP-071 case 1: confirmation for several files
settings: default
language: plain
```before
--- A.java
foo
foo
--- B.java
foo
```
keys: Ctrl+Shift+R, type "foo", Tab, type "bar", Alt+Shift+Enter
```after
dialog: title "Replace All", message "Replace 3 occurrences of 'foo' across 2 files with 'bar'?", buttons "Replace", "Cancel"
```

### SRCH-FIP-071 case 2: confirmed
settings: default
language: plain
```before
--- A.java
foo
foo
--- B.java
foo
```
keys: Ctrl+Shift+R, type "foo", Tab, type "bar", Alt+Shift+Enter, click "Replace"
```after
--- A.java
bar
bar
--- B.java
bar
status: 3 occurrences replaced
dialog: popup closed
```

### SRCH-FIP-071 case 3: Cancel changes nothing; one file needs no confirmation
settings: default
language: plain
```before
--- A.java
foo
foo
```
keys: Ctrl+Shift+R, type "foo", Tab, type "bar", Alt+Shift+Enter
```after
--- A.java
bar
bar
status: 2 occurrences replaced
dialog: no confirmation was shown (all matches are in one file)
```

### SRCH-FIP-072 case 1: Local History label
settings: default
language: plain
```before
--- A.java
foo
--- B.java
foo
```
keys: Ctrl+Shift+R, type "foo", Tab, type "bar", Alt+Shift+Enter, click "Replace"
```after
local history: one action "Replace all 'foo' in Project with 'bar'" covering A.java and B.java
```

### SRCH-FIP-072 case 2: regex and preserve case apply per file
settings: Regex on, Preserve case on
language: plain
```before
--- A.java
Foo foo
--- B.java
FOO
```
keys: Ctrl+Shift+R, type "foo", Tab, type "bar", Alt+Shift+Enter, click "Replace"
```after
--- A.java
Bar bar
--- B.java
BAR
```

### SRCH-FIP-073 case 1: a group name that does not exist is only detected per match
settings: Regex on
language: plain
```before
--- A.java
a
```
keys: Ctrl+Shift+R, type "(a)", Tab, type "${nope}", Alt+Shift+Enter
```after
--- A.java
a
notification (error, group "Find in Files operation failed"): "One or more malformed replacement strings"
```
(the template passes the dry run because group names are not checked there; each match then fails and is excluded)

### SRCH-FIP-031 case 1: the dry run reports an invalid group number before replacing
settings: Regex on
language: plain
```before
--- A.java
a
```
keys: Ctrl+Shift+R, type "(a)", Tab, type "$2", Alt+Shift+Enter
```after
--- A.java
a
dialog: error "Error": "Malformed replacement string: No group 2"
```

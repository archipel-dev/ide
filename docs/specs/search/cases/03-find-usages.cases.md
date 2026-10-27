# Cases for 03-find-usages.md

Conventions: multi-file projects use `--- <module>: <path>` separators (`<module>` omitted when the project has one module). `<caret>` marks the
caret in the file named by `focus:`. In `after` blocks: `tree:` introduces the Find tool window tree (two spaces of indentation per level; a number
after a node text is its secondary-colour usage count), `popup:` the Show Usages popup table, `status:` the status/header text, `hint:` an editor hint,
`dialog:` a dialog or list popup, `editor:` the editor state after navigation. Unless a case says otherwise: scope `Project Files`, default groupings
(usage type, module, package, file structure), all filters on, merge same-line usages on.

Shared project `P1` (two modules `m1`, `m2`; the class Util has 5 usages: two imports and three static member accesses):

```
--- m2: src/q/Util.java
package q;
public class Util { public static void run() {} }
--- m1: src/p/A.java
package p;
import q.Util;
class A {
  void f() { Util.run(); }
  void g() { Util.run(); }
}
--- m1: src/p/B.java
package p;
import q.Util;
class B { /* Util */ void h() { Util.run(); } }
```

### SRCH-USG-002 case 1: several targets give a choice popup
settings: default
language: java
```before
--- A.java
class A {
  void foo(String s) {}
  void foo(Integer i) {}
  void t() { foo(<caret>null); }
}
```
keys: Alt+F7
```after
dialog: list popup "Find Usages Of" with two rows, in source order: "foo(String)" and "foo(Integer)" (icon, name, grey location "A"); the first row is selected
```

### SRCH-USG-002 case 2: the popup of Show Usages has its own title
settings: default
language: java
```before
--- A.java
class A {
  void foo(String s) {}
  void foo(Integer i) {}
  void t() { foo(<caret>null); }
}
```
keys: Ctrl+Alt+F7
```after
dialog: list popup "Show Usages Of" with the same two rows
```

### SRCH-USG-002 case 3: no target at the caret
settings: default
language: java
```before
--- A.java
class A {   <caret>   }
```
keys: Alt+F7
```after
hint: red hint under the caret: "Cannot search for usages from this location." + line break + "Place the caret on the element to find usages for and try again."
```

### SRCH-USG-003 case 1: the target disappeared
settings: default
language: java
```before
--- A.java
class A { void foo() {} void t() { <caret>foo(); } }
```
keys: Alt+F7 with the ambiguity popup open; before choosing, delete the method foo in another way; Enter
```after
dialog: title "Cannot Search for Usages", text "Searched elements have been changed." + line break + "Cannot search for usages."
```

### SRCH-USG-005 case 1: the options dialog only on the settings key
settings: default
language: java
```before
--- A.java
class A { void <caret>foo() {} void t() { foo(); } }
```
keys: Ctrl+Alt+Shift+F7
```after
dialog: "Find Usages" with the line "Method foo()", group "Find:" (check boxes Usages checked, Overriding methods, ...), group "Scope:" (combo "Project Files"), buttons Find, Cancel, Help
```

### SRCH-USG-010 case 1: Find is disabled with nothing to search
settings: default
language: java
```before
--- A.java
class A { void <caret>foo() {} }
```
keys: Ctrl+Alt+Shift+F7, untick "Usages", untick every other option
```after
dialog: the button "Find" is disabled
```

### SRCH-USG-010 case 2: the chosen scope becomes the default
settings: default
language: java
```before
--- A.java
class A { void <caret>foo() {} void t() { foo(); } }
```
keys: Ctrl+Alt+Shift+F7, choose scope "Current File", Enter, then Ctrl+Alt+Shift+F7 again
```after
dialog: the second dialog shows the scope combo "Current File"
```

### SRCH-USG-011 case 1: options are per kind of target
settings: default
language: java
```before
--- A.java
class A { int <caret>f; void m() {} }
```
keys: Ctrl+Alt+Shift+F7, tick "Search for accessors", Enter; move the caret to method m; Ctrl+Alt+Shift+F7
```after
dialog: the method dialog does not show the field option; its own saved state is shown
```

### SRCH-USG-020 case 1: popup delay
settings: default
language: java
```before
--- A.java
class A { void <caret>foo() {} }
```
keys: Ctrl+Alt+F7 (the search needs about one second and finds nothing)
```after
popup: appears after 300 ms with the status "0 usages" and a spinner; when the search finishes without usages the popup closes and the hint "No usages found in Project Files" is shown
```

### SRCH-USG-021 case 1: header status while more exist
settings: usages page size = 100
language: java
```before
(a method with 250 call sites)
```
keys: Ctrl+Alt+F7 on the method
```after
status: "100+ usages" (the search stopped at the page limit); last table row "more usages"
```

### SRCH-USG-021 case 2: filters hide usages
settings: default
language: java
```before
(P1)
focus: m2: src/q/Util.java, caret on the class name Util
```
keys: Ctrl+Alt+F7, switch "Show Import Statements" off
```after
status: "3 of 5 usages"
popup: 3 rows (the two imports are hidden), a row "2 usages were filtered out"
```

### SRCH-USG-021 case 3: finished
settings: default
language: java
```before
(P1)
focus: m2: src/q/Util.java, caret on the class name Util
```
keys: Ctrl+Alt+F7
```after
status: "5 usages"
```

### SRCH-USG-022 case 1: row content
settings: default
language: java
```before
(P1)
focus: m1: src/p/A.java line 4, caret on Util in Util.run()
```
keys: Ctrl+Alt+F7
```after
popup rows (grouped by usage type, then file):
  Usage in import
    A.java    import q.Util;           (usage "Util" highlighted)
    B.java    import q.Util;
  Class static member access
    A.java    void f() { Util.run(); }   <- marked "Current"
    ...
```

### SRCH-USG-022 case 2: usages on one line are one row
settings: merge same line = on
language: java
```before
(P2 = P1 with line 5 of A.java changed to `  void g() { Util.run(); Util.run(); }`)
focus: m1: src/p/A.java line 5, caret on the first Util
```
keys: Ctrl+Alt+F7
```after
popup: the line "void g() { Util.run(); Util.run(); }" appears once, with both occurrences highlighted; the status counts usages (6), the table has 5 rows
```

### SRCH-USG-023 case 1: special rows come last in a fixed order
settings: default
language: java
```before
(P1 with 250 usages of Util, three of them inside a library root outside the scope, and the import filter off for 2 usages)
```
keys: Ctrl+Alt+F7
```after
popup: ... usage rows ..., "more usages", "3 usages are out of scope 'Project Files'", "2 usages were filtered out"
```

### SRCH-USG-024 case 1: more usages
settings: usages page size = 100
language: java
```before
(a method with 250 call sites)
```
keys: Ctrl+Alt+F7, End, Enter (on "more usages")
```after
status: "200+ usages"; the popup keeps the selection; Enter on the new "more usages" row again gives "250 usages" and no more row
```

### SRCH-USG-025 case 1: Enter navigates and closes
settings: default
language: java
```before
(P1)
focus: m2: src/q/Util.java, caret on Util
```
keys: Ctrl+Alt+F7, Down, Enter
```after
editor: the file of the selected row open, the usage range selected, caret at its start; popup closed
```

### SRCH-USG-025 case 2: speed search in the table
settings: default
language: java
```before
(P1)
focus: m2: src/q/Util.java, caret on Util
```
keys: Ctrl+Alt+F7, type "B.java"
```after
popup: only the rows whose text or file name contain "B.java" remain (two rows); the first is selected
```

### SRCH-USG-026 case 1: widening the scope
settings: default scope = Project Files
language: java
```before
(a method used in the project and in a library)
```
keys: Ctrl+Alt+F7, Ctrl+Alt+F7
```after
popup: advertisement text before the second key: "Press Ctrl+Alt+F7 again to search in 'Project and Libraries'"; after it the search restarts in that scope and the scope chooser of the popup shows "Project and Libraries"
```

### SRCH-USG-027 case 1: the filtered-out row
settings: Show Import Statements off
language: java
```before
(P1)
focus: m2: src/q/Util.java, caret on Util
```
keys: Ctrl+Alt+F7, select the row "2 usages were filtered out", Enter
```after
popup: reruns with the filter on; status "5 usages"; the row is gone
tooltip of the row before: "Some filters are configured to not show these usages. Click or press Enter to restart with filters toggled"
```

### SRCH-USG-029 case 1: navigation selects the usage
settings: default
language: java
```before
(P1)
focus: m1: src/p/B.java, caret on the second line import q.Util;, on Util
```
keys: Ctrl+Alt+F7, choose the row "void h() { Util.run(); }", Enter
```after
editor: B.java: the text "Util" of Util.run() selected on line 3
```

### SRCH-USG-030 case 1: default grouping
settings: default
language: java
```before
(P1)
focus: m2: src/q/Util.java, caret on the class name Util
```
keys: Alt+F7
```after
tree:
Usages
  Usage in import
    Module:m1
      p 2
        A.java 1
          import q.Util;
        B.java 1
          import q.Util;
  Class static member access
    Module:m1
      p 3
        A.java 2
          void f() { Util.run(); }
          void g() { Util.run(); }
        B.java 1
          void h() { Util.run(); }
```

### SRCH-USG-030 case 2: grouping switched off one level at a time
settings: group by usage type off, module off
language: java
```before
(P1, tree of the previous case)
```
keys: Ctrl+Alt+T, Ctrl+Alt+M
```after
tree:
Usages
  p 5
    A.java 3
      import q.Util;
      void f() { Util.run(); }
      void g() { Util.run(); }
    B.java 2
      import q.Util;
      void h() { Util.run(); }
```

### SRCH-USG-030 case 3: directory structure instead of packages
settings: group by usage type off, module off
language: java
```before
(P1)
```
keys: Ctrl+Alt+D
```after
tree:
Usages
  src 5
    p 5
      A.java 3
        ...
      B.java 2
        ...
```

### SRCH-USG-030 case 4: everything off is a flat list
settings: all grouping off except the mandatory file level
language: java
```before
(P1)
```
keys: toggle every grouping action off
```after
tree: the usage rows follow each other without group nodes, ordered by file path then offset: A.java:2, A.java:4, A.java:5, B.java:2, B.java:3; each row is prefixed with its short file path ("A.java 2  import q.Util;")
```

### SRCH-USG-031 case 1: counters
settings: default
language: java
```before
(P1)
focus: m2: src/q/Util.java, caret on Util
```
keys: Alt+F7
```after
status: tab title "Usages of Util", root counter "5 results"; footer "5 usages found"
```

### SRCH-USG-032 case 1: auto-expand
settings: Expand = off
language: java
```before
(P1)
```
keys: Alt+F7 on Util
```after
tree: the path down to the first usage row is expanded and that row is selected; other groups collapsed
```

### SRCH-USG-033 case 1: single-click navigation
settings: Navigate with Single Click = on
language: java
```before
(P1)
```
keys: Alt+F7 on Util, click a usage row
```after
editor: the usage is shown and selected in the editor; focus stays in the Find tool window
```

### SRCH-USG-040 case 1: read and write filters
settings: default
language: java
```before
--- A.java
class A {
  int count;
  void a() { count = 1; }
  void b() { int y = count; }
}
```
keys: Alt+F7 on count, switch "Show Write Access" off
```after
tree: Usages > Value read > A.java > "void b() { int y = count; }" only; status 1 of 2 usages
```

### SRCH-USG-040 case 2: both access filters off
settings: default
language: java
```before
--- A.java
class A {
  int count;
  void a() { count = 1; }
  void b() { int y = count; }
}
```
keys: Alt+F7 on count, Ctrl+R, Ctrl+W
```after
tree: empty ("0 of 2 usages" filtered out)
```

### SRCH-USG-040 case 3: imports and comments
settings: default
language: java
```before
(P1)
```
keys: Alt+F7 on Util, then toggle "Show Import Statements" off
```after
tree: the "Usage in import" group disappears; the remaining usages are 3
```

### SRCH-USG-041 case 1: merging lines
settings: merge same line = off
language: java
```before
(P2 = P1 with line 5 of A.java changed to `  void g() { Util.run(); Util.run(); }`)
```
keys: Alt+F7 on Util
```after
tree: under A.java the line "void g() { Util.run(); Util.run(); }" appears twice, once per usage
```

### SRCH-USG-050 case 1: exclude a usage
settings: default
language: java
```before
(P1)
```
keys: Alt+F7 on Util, select the row "void f() { Util.run(); }", Delete
```after
tree: the row is shown dimmed with the suffix "excluded"; the counter is unchanged; pressing Insert reverts it
```

### SRCH-USG-050 case 2: exclude a group
settings: default
language: java
```before
(P1)
```
keys: Alt+F7 on Util, select the node "A.java", Delete
```after
tree: all rows under A.java are dimmed with "excluded"
```

### SRCH-USG-051 case 1: rerun
settings: default
language: java
```before
(P1)
```
keys: Alt+F7 on Util, edit A.java so that one usage disappears, click Rerun
```after
tree: the usage is gone, counter "4 results"
```

### SRCH-USG-052 case 1: next and previous occurrence
settings: default
language: java
```before
(P1)
```
keys: Alt+F7 on Util, Ctrl+Alt+Down, Ctrl+Alt+Down, Ctrl+Alt+Up
```after
tree: the second usage row in tree order is selected; editor shows it
```

### SRCH-USG-053 case 1: recent find usages
settings: default
language: java
```before
(P1)
```
keys: Alt+F7 on Util, Alt+F7 on run, Alt+Down in the view
```after
dialog: list popup "Recent Find Usages" with the rows: "Method 'run' in Project Files", "Class 'Util' in Project Files" (newest first)
```

### SRCH-USG-055 case 1: nothing found
settings: default
language: java
```before
--- A.java
class A { void <caret>foo() {} }
```
keys: Alt+F7
```after
hint: "Usages of method foo not found" (no tab is created)
```

### SRCH-USG-055 case 2: one usage and skip results tab
settings: Skip results tab with one usage = on
language: java
```before
--- A.java
class A { void <caret>foo() {} void t() { foo(); } }
```
keys: Alt+F7
```after
editor: the call foo() in t selected; no tab created
```

### SRCH-USG-056 case 1: tab reuse
settings: Open results in new tab = off
language: java
```before
(P1)
```
keys: Alt+F7 on Util, Alt+F7 on run
```after
tool window: one tab "Usages of 'run'" (the previous tab content was replaced)
```

### SRCH-USG-060 case 1: highlight usages
settings: default
language: java
```before
--- A.java
class A {
  int <caret>count;
  void a() { count = 1; int y = count; }
}
```
keys: Ctrl+Shift+F7
```after
editor: "count" on lines 3 (twice) highlighted with the search-result attributes (the write one with the write attributes), the declaration is not highlighted by this action
status: "2 usages of count found (press Ctrl+Shift+F7 again to remove the highlighting, Escape to remove all highlighting)"
```

### SRCH-USG-060 case 2: toggling off
settings: default
language: java
```before
--- A.java
class A {
  int <caret>count;
  void a() { count = 1; int y = count; }
}
```
keys: Ctrl+Shift+F7, Ctrl+Shift+F7
```after
editor: no highlighting
```

### SRCH-USG-060 case 3: nothing found
settings: default
language: java
```before
--- A.java
class A { int <caret>unused; }
```
keys: Ctrl+Shift+F7
```after
status: "No usages of unused found"
```

### SRCH-USG-061 case 1: stepping past the last highlight
settings: default
language: java
```before
--- A.java
class A {
  int <caret>count;
  void a() { count = 1; int y = count; }
}
```
keys: Ctrl+Shift+F7, run action "Next Highlighted Usage" three times
```after
editor: caret on the second usage of count after the second call; hint after the third: "No more highlights found"
```

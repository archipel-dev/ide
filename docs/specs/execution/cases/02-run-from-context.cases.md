# Cases: run from context, Run menu, lifecycle (EXE-CTX)

### EXE-CTX-001 case 1: Run menu without configurations
settings: default
language: plain
```before
project with no run configurations, no running process
```
keys: open main menu Run
```after
Run                       Shift+F10      disabled (text without a name)
Debug                     Shift+F9       disabled (text without a name)
---
Run...                    Alt+Shift+F10  enabled
Debug...                  Alt+Shift+F9   enabled
Edit Configurations...    enabled
---
Stop                      Ctrl+F2        disabled
Stop Background Processes...  Ctrl+Shift+F2  disabled
Show Running List         enabled
```

### EXE-CTX-010 case 1: no runner
settings: default
language: plain
```before
configuration of a type that has no debug runner
```
keys: Shift+F9
```after
dialog title: Run Configuration Error
dialog text: Debug: there are no runners for <configuration>
```

### EXE-CTX-010 case 2: validation error leads to the Edit dialog
settings: default
language: java
```before
Application: Main with empty Main class
```
keys: Shift+F10
```after
dialog titled Edit Configuration opens; Run button visible
error line in the form: No main class specified
after pressing OK with the class still empty: dialog titled Change Configuration Settings
text: Configuration is still incorrect. Do you want to edit it again?
buttons: Edit, Continue Anyway
```

### EXE-CTX-010 case 3: execution failure
settings: default
language: java
```before
Application: Main, JRE alternative path `/nope`
```
keys: Shift+F10
```after
error (dialog or balloon) title: Error running 'Main'
message: '/nope' is not a valid JRE home
no Run tab
```

### EXE-CTX-020 case 1: not allowed to run in parallel
settings: confirm.rerun.with.termination=true
language: java
```before
Application: Main (Allow multiple instances off), process running
```
keys: Shift+F10
```after
dialog text: 'Main' is not allowed to run in parallel.
Would you like to stop the running one?
buttons: Stop and Rerun, Cancel; checkbox: Don't ask again
after Stop and Rerun: first process ends (exit line shown), second process starts in the same tab
```

### EXE-CTX-023 case 1: incompatible configuration
settings: default
language: plain
```before
configuration X running; configuration Y declares that X must be stopped
```
keys: start Y
```after
dialog title: Incompatible Configuration Is Running
text: 'Y' run configuration is not compatible with currently running X configuration.
question: Are you sure you want to stop incompatible running instance?
button: Stop incompatible
```

### EXE-CTX-024 case 1: Stop with several processes
settings: default
language: plain
```before
running: Application A, Application B (selected tab), JUnit C
```
keys: Ctrl+F2 (focus in the editor)
```after
popup title: Stop Process
rows: A, B (selected), C, Stop All (Ctrl+F2)
toolbar Stop button text: Stop...
```

### EXE-CTX-027 case 1: Show Running List
settings: default
language: plain
```before
no running configuration
```
keys: Run | Show Running List
```after
balloon title: Running List
balloon text: Nothing to show
```

### EXE-CTX-031 case 1: ordering of candidates
settings: default
language: java
```before
--- Main.java
public class Main {
    public static void <caret>main(String[] a) { }
}
```
keys: action RunClass (Ctrl+Shift+F10)
```after
new temporary Application configuration: name Main, main class Main
source element: the method main (not the class)
Run tab title: Main
```

### EXE-CTX-034 case 1: main method class
settings: default
language: java
```before
--- p/Main.java
package p;
public class Main {
    void helper() { <caret> }
    public static void main(String[] a) { }
}
```
keys: Ctrl+Shift+F10
```after
Application: Main (main class p.Main) [temporary]; source element: class Main
```

### EXE-CTX-034 case 2: class without main
settings: default
language: java
```before
--- p/Util.java
package p;
public class Util { <caret> }
```
keys: open the editor context menu
```after
no entries "Run 'Util'" and "Debug 'Util'"
```

### EXE-CTX-034 case 3: second run reuses the configuration
settings: default
language: java
```before
--- p/Main.java
package p;
public class Main { public static void <caret>main(String[] a) { } }
Application: Main (main class p.Main) [temporary] exists
```
keys: Ctrl+Shift+F10, then Ctrl+Shift+F10 again after the process ends
```after
configurations: Application: Main (still one)
```

### EXE-CTX-035 case 1: JUnit method
settings: default
language: java
```before
--- p/FooTest.java
package p;
import org.junit.Test;
public class FooTest {
    @Test public void <caret>bar() { }
}
```
keys: Ctrl+Shift+F10
```after
JUnit: FooTest.bar [temporary], kind Method, class p.FooTest, method bar
```

### EXE-CTX-035 case 2: JUnit class (caret outside methods)
settings: default
language: java
```before
--- p/FooTest.java
package p;
import org.junit.Test;
public class FooTest {
    <caret>
    @Test public void bar() { }
}
```
keys: Ctrl+Shift+F10
```after
JUnit: FooTest [temporary], kind Class, class p.FooTest
```

### EXE-CTX-035 case 3: package selected in the project tree
settings: default
language: java
```before
project view: package p.q selected (contains FooTest)
```
keys: right click, Run 'Tests in 'p.q''
```after
JUnit: p.q [temporary], kind All in package, package p.q
```

### EXE-CTX-035 case 4: two classes selected
settings: default
language: java
```before
project view: FooTest.java and BarTest.java selected
```
keys: Ctrl+Shift+F10
```after
JUnit: FooTest and 1 more [temporary], kind Pattern, patterns p.FooTest, p.BarTest
```

### EXE-CTX-036 case 1: class and method candidates in the menu
settings: default
language: java
```before
--- p/FooTest.java
package p;
import org.junit.Test;
public class FooTest {
    @Test public void <caret>bar() { }
}
```
keys: open editor context menu
```after
Run 'FooTest.bar'
Debug 'FooTest.bar'
(more) Modify Run Configuration...
```

### EXE-CTX-040 case 1: existing configuration wins
settings: default
language: java
```before
--- p/Main.java
package p;
public class Main { public static void <caret>main(String[] a) { } }
Application: Start (main class p.Main, permanent)
```
keys: Ctrl+Shift+F10
```after
runs "Start"; no new configuration; combo shows Start
```

### EXE-CTX-041 case 1: several candidates give a popup
settings: default
language: java
```before
--- p/Dual.java
package p;
public class Dual {
    @org.junit.Test public void <caret>t() { }
    public static void main(String[] a) { }
}
```
keys: open Run menu item for the context (no existing configuration)
```after
group popup with children: JUnit entry "Dual.t" with type icon, "Dual.main()" Application entry
```

### EXE-CTX-050 case 1: markers of a class with main
settings: default
language: java
```before
--- p/Main.java
package p;
public class Main {
    public static void main(String[] a) { }
}
```
keys: look at the gutter
```after
line 2 (class Main identifier): run marker
line 3 (method main identifier): run marker
```

### EXE-CTX-050 case 2: no marker for a plain class
settings: default
language: java
```before
--- p/Util.java
package p;
public class Util { void f() { } }
```
keys: look at the gutter
```after
no run markers
```

### EXE-CTX-050 case 3: test class and methods
settings: default
language: java
```before
--- p/FooTest.java
package p;
import org.junit.Test;
public class FooTest {
    @Test public void a() { }
    @Test public void b() { }
    void helper() { }
}
```
keys: look at the gutter; hover line 3
```after
line 3 (class FooTest): run-all marker, tooltip: Run Test  Ctrl+Shift+F10
line 4 (a): run marker; line 5 (b): run marker; line 6 (helper): none
```

### EXE-CTX-053 case 1: marker popup
settings: default
language: java
```before
--- p/Main.java
package p;
public class Main {
    public static void main(String[] a) { }
}
```
keys: click the marker on line 3
```after
popup: Run 'Main.main()', Debug 'Main.main()', Run 'Main.main()' with Coverage, ---, Modify Run Configuration...
```

### EXE-CTX-060 case 1: toolbar states
settings: default
language: plain
```before
(a) no configuration; (b) Application: Main selected, not running; (c) Main running
```
keys: look at the main toolbar
```after
(a) selector text: Add Configuration...; Run, Debug enabled-less; Stop disabled
(b) selector: Main; Run enabled; Debug enabled; Stop disabled
(c) Run button shows Rerun icon; Stop enabled
```

### EXE-CTX-062 case 1: chooser popup
settings: default
language: plain
```before
Application: A, Application: B, JUnit: T [temporary]
```
keys: Alt+Shift+F10
```after
popup rows (with mnemonics): 1 A, 2 B, 3 T; (grouped by type); bottom ad: Hold Shift to Debug
```

### EXE-CTX-062 case 2: Shift switches the executor
settings: default
language: plain
```before
chooser popup (Run) open, item A highlighted
```
keys: hold Shift, press Enter
```after
A starts in Debug mode (Debug tool window opens)
```

### EXE-CTX-065 case 1: pin
settings: temporary.configurations.limit=1
language: plain
```before
JUnit: T1 [temporary] pinned, Application: T2 [temporary]
```
keys: run T3 (a new temporary)
```after
T1 kept (pinned), T3 kept, T2 removed
```

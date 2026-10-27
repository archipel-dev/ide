# Cases: debugger core (EXE-DBG)

Convention: source blocks mark breakpoints with `/*B*/` at the line start, the execution point with
`=>` in the `after` block, and variables as `name = value`.

### EXE-DBG-010 case 1: toggle with the keyboard
settings: default
language: java
```before
--- Main.java
public class Main {
    public static void main(String[] a) {
        int x = 1;<caret>
        System.out.println(x);
    }
}
```
keys: Ctrl+F8
```after
line 3 has a line breakpoint (red circle), enabled
```

### EXE-DBG-010 case 2: second toggle removes it
settings: default
language: java
```before
--- Main.java
public class Main {
    public static void main(String[] a) {
/*B*/    int x = 1;<caret>
    }
}
```
keys: Ctrl+F8
```after
no breakpoint on line 3
```

### EXE-DBG-010 case 3: line without code
settings: default
language: java
```before
--- Main.java
public class Main {
    <caret>
    public static void main(String[] a) { }
}
```
keys: Ctrl+F8
```after
no breakpoint is created on the empty line (the action does nothing)
```

### EXE-DBG-013 case 1: gutter tooltip
settings: default
language: java
```before
breakpoint at Main.java:3 with condition `x > 1`, log message on, suspend policy All
```
keys: hover the gutter icon
```after
Suspend: all
Condition: x > 1
Log message: yes
Right-click to edit
```

### EXE-DBG-014 case 1: quick edit popup
settings: default
language: java
```before
--- Main.java
public class Main {
    public static void main(String[] a) {
/*B*/    int x = 1;<caret>
    }
}
```
keys: Ctrl+Shift+F8
```after
popup with: Enabled (checked), Suspend: All (selected) / Thread, Condition: (empty field), link More, button Done
```

### EXE-DBG-015 case 1: condition true
settings: default
language: java
```before
--- Main.java
public class Main {
    public static void main(String[] a) {
        for (int i = 0; i < 5; i++) {
/*B cond: i == 3*/    System.out.println(i);
        }
    }
}
```
keys: Shift+F9
```after
=> line 4 paused with i = 3 (first stop); console shows 0 1 2 before the stop
```

### EXE-DBG-015 case 2: non-boolean condition
settings: default
language: java
```before
breakpoint at line 4 with condition `i + 1`
```
keys: debug
```after
dialog titled Breakpoint Condition Error; message: 'boolean' value expected in condition
the program is suspended at line 4
```

### EXE-DBG-015 case 3: condition error suspends
settings: default
language: java
```before
breakpoint with condition `missing > 0`
```
keys: debug
```after
dialog titled Breakpoint Condition Error; message: Cannot find local variable 'missing'
```

### EXE-DBG-016 case 1: logging breakpoint
settings: default
language: java
```before
--- Main.java
public class Main {
    public static void main(String[] a) {
/*B suspend none, log message on, log expression: a.length*/    System.out.println("x");
    }
}
```
keys: Shift+F9
```after
console contains (before the program's own output line):
Breakpoint reached at Main.main(Main.java:3)
0
x
no suspension
```

### EXE-DBG-019 case 1: dependent breakpoint
settings: default
language: java
```before
B1 at line 3 (master); B2 at line 6 depends on B1, After hit: Leave enabled
```
keys: debug, Resume
```after
first stop at B1 (line 3); after Resume the next stop is B2 (line 6)
if B1 were never hit B2 would not stop
```

### EXE-DBG-020 case 1: method breakpoint
settings: default
language: java
```before
--- Main.java
public class Main {
    static int twice(int v) { return v * 2; }
    public static void main(String[] a) { twice(4); }
}
method breakpoint on twice (entry and exit on)
```
keys: debug
```after
stop 1: status text: Method 'twice' entered at Main(Main:2)   (execution at line 2)
Resume: stop 2: Method breakpoint reached. Method 'twice' is about to exit at Main(Main:2)
```

### EXE-DBG-020 case 2: field watchpoint
settings: default
language: java
```before
class Counter { int n; void inc() { n++; } }
field watchpoint on n (modification on)
```
keys: debug a program calling inc()
```after
stop: {Counter@N}.n will be modified at Counter(Counter:1). Current value = '0'. New value = '1'.
```

### EXE-DBG-020 case 3: exception breakpoint
settings: default
language: java
```before
exception breakpoint on java.lang.IllegalStateException (caught and uncaught on)
program throws IllegalStateException inside try/catch
```
keys: debug
```after
stop at the throw statement (before the catch), execution line on the throw
frames: method of the throw is the top frame
```

### EXE-DBG-022 case 1: removal confirmation
settings: confirm breakpoint removal on
language: java
```before
breakpoint with condition `x > 0` at line 3
```
keys: click the gutter icon
```after
dialog title: Confirm Removal
text: Breakpoint contains condition: x > 0   (and a Remove choice)
```

### EXE-DBG-030 case 1: breakpoints dialog
settings: default
language: java
```before
line breakpoints at Main.java:3 and Util.java:10; method breakpoint at Util.twice; exception: Any exception (disabled)
```
keys: Ctrl+Shift+F8
```after
dialog title: Breakpoints
tree:
  Java Line Breakpoints
    [x] Main.java:3
    [x] Util.java:10
  Java Method Breakpoints
    [x] Util.twice()
  Java Exception Breakpoints
    [ ] Any exception
right pane when nothing selected: Select a breakpoint
```

### EXE-DBG-041 case 1: step over lines
settings: default
language: java
```before
--- Main.java
public class Main {
    static int f(int v) { return v + 1; }
    public static void main(String[] a) {
/*B*/    int x = f(1);
        int y = f(x);
        System.out.println(y);
    }
}
```
keys: debug, F8
```after
=> line 5 (the call on line 4 ran without stopping); x = 2
```

### EXE-DBG-041 case 2: step over at the end of a method returns to the caller
settings: default
language: java
```before
paused at line 2 `return v + 1;` inside f called from line 4
```
keys: F8
```after
=> back in main at line 4 (the assignment `x = ` still pending), frames: main
```

### EXE-DBG-041 case 3: breakpoint inside stepped-over call still stops
settings: default
language: java
```before
paused at line 4; breakpoint at line 2 inside f
```
keys: F8
```after
=> line 2 (stopped by the breakpoint, status text: Breakpoint reached at Main.f(Main.java:2))
```

### EXE-DBG-042 case 1: step into
settings: default
language: java
```before
paused at line 4 `int x = f(1);`
```
keys: F7
```after
=> line 2 in f; frames: f(int) , main; v = 1
```

### EXE-DBG-042 case 2: step into a filtered library call
settings: default
language: java
```before
paused at a line `System.out.println(y);`
```
keys: F7
```after
=> next line of main (println is in java.* so it is skipped)
```

### EXE-DBG-043 case 1: smart step into
settings: default
language: java
```before
--- Main.java
public class Main {
    static int f(int v) { return v; }
    static int g(int v) { return v; }
    public static void main(String[] a) {
/*B*/    int x = f(g(1)) + g(2);
    }
}
```
keys: debug, Shift+F7
```after
popup title: Method to Step Into
rows: g(int), f(int), g(int)
hint text: Choose a method to step into with arrow keys.
To exit the 'smart step into' mode, press Esc.
then Down, Enter on the second row: => line 2 in f (called with v = 1)
```

### EXE-DBG-044 case 1: step out
settings: default
language: java
```before
paused inside f (line 2) called from main line 5
```
keys: Shift+F8
```after
=> main line 5 after the call returned (value of the call available)
```

### EXE-DBG-045 case 1: run to cursor
settings: default
language: java
```before
paused at line 4; caret on line 6
```
keys: Alt+F9
```after
=> line 6; status: Stopped at cursor
```

### EXE-DBG-045 case 2: force run to cursor ignores breakpoints
settings: default
language: java
```before
paused at line 4; breakpoint at line 5; caret on line 6
```
keys: Ctrl+Alt+F9
```after
=> line 6 (line 5 breakpoint ignored)
```

### EXE-DBG-046 case 1: force step over
settings: default
language: java
```before
paused at line 4; breakpoint at line 2 inside f
```
keys: Alt+Shift+F8
```after
=> line 5 (the breakpoint inside f did not stop)
```

### EXE-DBG-047 case 1: filters list
settings: default
language: plain
```before
open Settings | Build, Execution, Deployment | Debugger | Stepping
```
keys: read the list
```after
Do not step into the classes (all checked): com.sun.*, java.*, javax.*, org.omg.*, sun.*, jdk.internal.*, junit.*, org.junit.*, com.intellij.rt.*, com.yourkit.runtime.*, com.springsource.loaded.*, org.springsource.loaded.*, javassist.*, org.apache.webbeans.*, com.ibm.ws.*, org.mockito.*, com.jetbrains.internal.IoOverNio*, com.azul.*
Skip synthetic methods [x]  Skip constructors [ ]  Skip class loaders [x]  Skip simple getters [ ]
```

### EXE-DBG-049 case 1: reset frame
settings: default
language: java
```before
paused inside f (line 2), called from main line 4 (x not yet assigned)
```
keys: action Debugger.PopFrame (Reset Frame)
```after
=> main line 4; frames: main; f's locals gone
```

### EXE-DBG-060 case 1: frames pane
settings: default
language: java
```before
paused in Main.f called from Main.main (also a call through java.util.ArrayList.forEach in between)
```
keys: look at Frames
```after
f(), Main            Main.java:2
accept(), Main$$Lambda   ...
3 hidden frames       (library frames collapsed)
main(), Main         Main.java:6
```

### EXE-DBG-062 case 1: running thread
settings: default
language: java
```before
session paused in thread main; thread worker-1 is running
```
keys: select worker-1 in the selector
```after
Frames pane text: Frames not available for unsuspended thread
```

### EXE-DBG-070 case 1: variables of a frame
settings: default
language: java
```before
--- Main.java
public class Main {
    public static void main(String[] a) {
        int x = 1;
        String s = "hi";
/*B*/    x++;
        int later = 3;
    }
}
```
keys: debug
```after
Variables:
  a = {String[0]@N} []
  x = 1
  s = "hi"
(no `later` yet)
```

### EXE-DBG-070 case 2: large list
settings: default
language: java
```before
local `list` is an ArrayList with 250 ints
```
keys: expand `list`
```after
children 0..99 shown, then the row: ... (150 more items. Double-click to see)
```

### EXE-DBG-075 case 1: inline values
settings: Show values inline = on
language: java
```before
--- Main.java
public class Main {
    public static void main(String[] a) {
        int x = 1;
        int y = x + 1;
/*B*/    System.out.println(y);
    }
}
```
keys: debug
```after
line 3: int x = 1;    x = 1
line 4: int y = x + 1;    y = 2   x = 1
line 5 (execution line): System.out.println(y);    y = 2
```

### EXE-DBG-076 case 1: watches
settings: default
language: java
```before
paused with x = 2; Watches empty
```
keys: click New Watch (Insert), type `x * 10`, Enter; press F8 (x becomes 3)
```after
Watches before the step: x * 10 = 20
Watches after the step: x * 10 = 30
empty text before adding: No watches
```

### EXE-DBG-090 case 1: evaluate expression
settings: default
language: java
```before
paused with a = 4, b = 5
```
keys: Alt+F8, type `a + b`, Enter
```after
dialog Evaluate Expression; Result: result = 9
```

### EXE-DBG-090 case 2: code fragment
settings: default
language: java
```before
paused with a = 4
```
keys: Alt+F8, press Code Fragment Mode, type `int t = a * 2; t + 1`, Evaluate
```after
dialog title: Evaluate Code Fragment; Result: result = 9
```

### EXE-DBG-092 case 1: errors
settings: default
language: java
```before
paused in main (no variable `nope`)
```
keys: Alt+F8, `nope + 1`, Enter; then `a +`, Enter
```after
first: Cannot find local variable 'nope'
second: Invalid expression : a +   (or the parser message of the language)
```

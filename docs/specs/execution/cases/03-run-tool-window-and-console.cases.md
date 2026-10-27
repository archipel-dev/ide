# Cases: Run tool window and console (EXE-CON)

### EXE-CON-003 case 1: tab reuse
settings: default
language: java
```before
Run tab "Main" finished (exit 0); Application: Main
```
keys: Shift+F10
```after
Run tool window tabs: Main (same tab, console cleared, new command line printed)
```

### EXE-CON-004 case 1: console toolbar
settings: default
language: java
```before
Run tab "Main" with a running process
```
keys: look at the left toolbar, then right-click in the console
```after
toolbar: Rerun, Stop, Dump Threads, Exit, Up the Stack Trace, Down the Stack Trace, Use Soft Wraps, Scroll to the end, Print, Clear All
context menu contains: Fold Lines Like This, Pause Output, Clear All, Copy
```

### EXE-CON-010 case 1: first lines and exit line
settings: default
language: java
```before
--- Main.java
public class Main { public static void main(String[] a) { System.out.println("hi"); } }
```
keys: run Main (exit 0)
```after
<java command line ending with  Main>
hi

Process finished with exit code 0
```

### EXE-CON-011 case 1: exit codes
settings: default
language: java
```before
program calls System.exit(3)
```
keys: run it
```after
...
Process finished with exit code 3
```

### EXE-CON-011 case 2: signal exit on Unix
settings: default
language: java
```before
process killed with SIGTERM
```
keys: Stop
```after
Process finished with exit code 143 (interrupted by signal 15:SIGTERM)
```

### EXE-CON-020 case 1: ANSI colors
settings: default
language: plain
```before
process prints: ESC[31mred ESC[0m plain ESC[1;32mbold green ESC[0m
```
keys: view the console
```after
visible text: "red  plain bold green" (no ESC characters)
"red" red; "plain" default; "bold green" bold green
```

### EXE-CON-030 case 1: stack frame link
settings: default
language: java
```before
--- p/Main.java
package p;
public class Main { public static void main(String[] a) { throw new IllegalStateException("x"); } }
console text:
Exception in thread "main" java.lang.IllegalStateException: x
	at p.Main.main(Main.java:2)
```
keys: Ctrl+click on `Main.java:2`
```after
editor opens p/Main.java with the caret at line 2
```

### EXE-CON-030 case 2: unlinkable frames
settings: default
language: java
```before
console text:
	at java.base/jdk.internal.misc.Unsafe.park(Native Method)
	at p.Gone.f(Unknown Source)
```
keys: hover each line
```after
no hyperlink on either line
```

### EXE-CON-040 case 1: 12 frames fold
settings: default
language: java
```before
console text:
java.lang.RuntimeException: boom
	at a.A.f1(A.java:1)
	at a.A.f2(A.java:2)
	at a.A.f3(A.java:3)
	at a.A.f4(A.java:4)
	at a.A.f5(A.java:5)
	at a.A.f6(A.java:6)
	at a.A.f7(A.java:7)
	at a.A.f8(A.java:8)
	at a.A.f9(A.java:9)
	at a.A.f10(A.java:10)
	at a.A.f11(A.java:11)
	at a.A.f12(A.java:12)
```
keys: view the console
```after
java.lang.RuntimeException: boom
	at a.A.f1(A.java:1)
	... (frames 2..8 visible)
	at a.A.f8(A.java:8)
	<4 folded frames>
```

### EXE-CON-040 case 2: exactly 8 frames do not fold
settings: default
language: java
```before
console text: an exception with 8 frame lines
```
keys: view the console
```after
all 8 frame lines visible, no placeholder
```

### EXE-CON-050 case 1: stdin round trip
settings: default
language: java
```before
program: reads a line with Scanner and prints "got: " + line
```
keys: run; type `abc`, Enter
```after
abc            (user input text)
got: abc
```

### EXE-CON-060 case 1: stick to end
settings: default
language: plain
```before
console receiving output continuously, view at the end
```
keys: scroll up two pages; wait for more output; press toolbar Scroll to the end
```after
while scrolled up: view stays; after the toggle: view jumps to the last line and follows again
```

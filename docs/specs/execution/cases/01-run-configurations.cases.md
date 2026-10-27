# Cases: run configurations (EXE-CONF)

State blocks list configurations as `<Type>: <name> [flags]` in display order. `keys:` lines are
user actions; `action X` names an action id.

### EXE-CONF-001 case 1: identity follows the name
settings: default
language: plain
```before
Application: Main
```
keys: open Edit Configurations (action editRunConfigurations), select `Main`, set Name to `App`, OK
```after
Application: App
lookup "Application.Main": none
lookup "Application.App": found
```

### EXE-CONF-002 case 1: free name kept
settings: default
language: plain
```before
Application: Main
```
keys: add new Application configuration named `Other`, OK
```after
Application: Main
Application: Other
```

### EXE-CONF-002 case 2: duplicate gets (1)
settings: default
language: plain
```before
Application: Main
```
keys: add new Application configuration, set Name to `Main`
```after
message under the form: Application with name 'Main' already exists
OK and Apply: disabled
```

### EXE-CONF-002 case 3: unique-name helper
settings: default
language: plain
```before
existing names of type Application: Main, Main (1)
```
keys: suggest unique name for `Main (1)`; suggest unique name for ``; suggest unique name for `Free`
```after
Main (2)
Unnamed
Free
```

### EXE-CONF-005 case 1: JUnit class and method names
settings: default
language: java
```before
--- src/p/FooTest.java
package p;
public class FooTest { @org.junit.Test public void bar() {} }
```
keys: JUnit configuration kind Class with class `p.FooTest`; then kind Method with method `bar`
```after
name (class kind): FooTest
name (method kind): FooTest.bar
```

### EXE-CONF-005 case 2: JUnit package, module and whole project
settings: default
language: java
```before
kind All in package, package `p.q`, scope In single module, module `app`
```
keys: read generated name; change scope to In whole project; read name; set package empty with module `app`, scope In single module
```after
p.q in app
Whole project
All in app
```

### EXE-CONF-005 case 3: pattern, category, tags, repeat
settings: default
language: plain
```before
kind Pattern with patterns [p.FooTest, p.BarTest, p.BazTest]
```
keys: read name; remove two patterns; clear patterns; set repeat mode N times with count 3
```after
FooTest and 2 more
FooTest
Temp suite
Temp suite [3]
```

### EXE-CONF-005 case 4: method with parameter, until failure
settings: default
language: plain
```before
kind Method, class p.FooTest, method bar, parameters `valueSource 1`
```
keys: read name; set repeat mode Until failure
```after
FooTest.bar with parameter "2"
FooTest.bar with parameter "2" [*]
```

### EXE-CONF-010 case 1: gutter run creates a temporary configuration
settings: default
language: java
```before
--- src/Main.java
public class Main { public static void main(String[] a) {} }
no configurations
```
keys: click the gutter run icon next to `Main`, choose `Run 'Main.main()'`
```after
toolbar popup (Recent): Application: Main [temporary]
Edit Configurations tree: Application > Main (dimmed, below permanent)
```

### EXE-CONF-011 case 1: temporary limit
settings: temporary.configurations.limit=2
language: java
```before
no configurations
```
keys: run temporary A; run temporary B; run temporary C
```after
Application: B [temporary]
Application: C [temporary]
```

### EXE-CONF-011 case 2: pinned temporaries survive
settings: temporary.configurations.limit=1
language: java
```before
Application: A [temporary, pinned]
```
keys: run temporary B; run temporary C
```after
Application: A [temporary, pinned]
Application: C [temporary]
```

### EXE-CONF-021 case 1: store as project file
settings: default
language: plain
```before
Application: Main [temporary]   (workspace)
```
keys: Edit Configurations, select `Main`, check `Store as project file`, OK
```after
Application: Main [permanent, shared]
file .idea/runConfigurations/Main.xml exists, root element: <component name="ProjectRunConfiguration" name="Main" type="Application" factoryName="Application">
workspace.xml no longer contains Main
```

### EXE-CONF-026 case 1: shared configuration including a non-shared one
settings: default
language: plain
```before
Compound: All [shared] includes Application: Server [workspace]
```
keys: load project
```after
Compound: All [workspace]
notification text: Compound configuration was made "not shared" since included Run Configuration 'Server' is not shared.
```

### EXE-CONF-043 case 1: dialog validation
settings: default
language: plain
```before
Application: Main
```
keys: Edit Configurations, copy `Main`, clear the Name field
```after
form error: shown (empty name)
OK, Apply: disabled
```

### EXE-CONF-051 case 1: Application checks
settings: default
language: java
```before
Application with empty Main class
```
keys: open the configuration in Edit Configurations
```after
error line: No main class specified
Run button in the toolbar: still enabled, pressing opens the dialog titled Edit Configuration
```

### EXE-CONF-051 case 2: class without main method
settings: default
language: java
```before
--- src/Foo.java
public class Foo { }
Application, Main class `Foo`
```
keys: open the configuration
```after
warning line: Main method not found in class Foo
Run: starts after confirmation, the process fails with the JVM message
```

### EXE-CONF-051 case 3: missing working directory
settings: default
language: java
```before
Application, Main class `Main`, Working directory `/nonexistent`
```
keys: press Run
```after
error dialog titled Run Configuration Error with text: Working directory '/nonexistent' doesn't exist
no process
```

### EXE-CONF-053 case 1: JUnit messages
settings: default
language: java
```before
JUnit kind Method, class `p.FooTest`, method empty
JUnit kind Class, class empty
JUnit kind All in package, package `nope`
```
keys: select each configuration in Edit Configurations
```after
Method name not specified
No test class specified
Package 'nope' does not exist
```

### EXE-CONF-070 case 1: default before-launch list of an Application
settings: default
language: java
```before
new Application configuration
```
keys: open Before launch section
```after
Before launch: Build
buttons: Add task; options: Show the run/debug configuration settings before start (off), Open run/debug tool window when started (on), Focus run/debug tool window when started (off)
```

### EXE-CONF-071 case 1: failing task stops the launch
settings: default
language: plain
```before
Application: Main, before launch: [Build (enabled), Run 'Prep' (enabled)]; Build fails with a compile error
```
keys: Run `Main`
```after
no process, Prep not started
Build tool window shows the error
```

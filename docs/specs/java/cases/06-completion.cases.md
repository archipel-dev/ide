# Java completion: acceptance cases

Format: `docs/specs/java/cases/README.md` (extends `docs/specs/editor/cases/README.md`). Each case belongs to
the requirement of the same ID in `docs/specs/java/06-completion.md`. Cases of this file that need
a library put it on the module with `settings: library=<name>`; JUnit and Hamcrest libraries are on the *test*
classpath of the module (production sources do not see them, JAVA-RES-029).

Keys of a completion case (the executable runner, `crates/archipel-ide-java/tests/java_cases.rs`):

* `keys: complete` presses Ctrl+Space once at the caret (invocation count 1); `complete [count N]` presses it N
  times. The `after` block then does not repeat the code but states the popup, one line per statement:
  `item: <name> (<package or outer class>)` (the class is offered; the tail is left out for classes of the file
  and of its package), `not item: <name or name (package)>` (it is not offered), `rank: <a> < <b>` (both are
  offered, `a` above `b`) and `no items`.
* `keys: complete, choose "<row>"` additionally picks the row with exactly that text; `after` is then the
  resulting file with `<caret>` behind the inserted name.

### JAVA-CMP-009 case 1: Annotation position offers annotation types of the test classpath
settings: library=junit5
language: java
```before
--- src/test/java/p/AppTest.java
package p;

class AppTest {
  @Te<caret>
  void greets() {}
}
```
keys: complete
```after
item: Test (org.junit.jupiter.api)
not item: TestInstance (org.junit.jupiter.api)
not item: String
```

### JAVA-CMP-019 case 1: Choosing an annotation type adds its import
settings: library=junit5
language: java
```before
--- src/test/java/p/AppTest.java
package p;

class AppTest {
  @Te<caret>
  void greets() {}
}
```
keys: complete, choose "Test (org.junit.jupiter.api)"
```after
package p;

import org.junit.jupiter.api.Test;

class AppTest {
  @Test<caret>
  void greets() {}
}
```

### JAVA-CMP-009 case 2: Production sources do not see test libraries
settings: library=junit5
language: java
```before
--- src/main/java/p/App.java
package p;

class App {
  @Te<caret>
  void greets() {}
}
```
keys: complete
```after
not item: Test (org.junit.jupiter.api)
```

### JAVA-CMP-009 case 3: A JUnit 4 project offers its own annotation type
settings: library=junit4
language: java
```before
--- src/test/java/p/AppTest.java
package p;

class AppTest {
  @Te<caret>
  void greets() {}
}
```
keys: complete
```after
item: Test (org.junit)
not item: Test (org.junit.jupiter.api)
```

### JAVA-CMP-029 case 1: Annotation types are filtered by the element they annotate
settings: library=junit5
language: java
```before
--- src/test/java/p/AppTest.java
package p;

class AppTest {
  @Te<caret>
  private int counter;
}
```
keys: complete
```after
not item: Test (org.junit.jupiter.api)
```

### JAVA-CMP-018 case 1: Class names after new come from the JDK index with a package hint
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  void run() {
    Object list = new ArrayLi<caret>
  }
}
```
keys: complete
```after
item: ArrayList (java.util)
not item: ArrayDeque (java.util)
not item: List (java.util)
```

### JAVA-CMP-019 case 2: Choosing ArrayList after new adds the import
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  void run() {
    Object list = new ArrayLi<caret>
  }
}
```
keys: complete, choose "ArrayList (java.util)"
```after
package p;

import java.util.ArrayList;

class Main {
  void run() {
    Object list = new ArrayList<caret>
  }
}
```

### JAVA-CMP-018 case 2: Camel-hump prefixes match class names
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  void run() {
    Object list = new ArLi<caret>
  }
}
```
keys: complete
```after
item: ArrayList (java.util)
```

### JAVA-CMP-009 case 4: Type position offers classes and interfaces, the discouraged ones last
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  private Lis<caret>
}
```
keys: complete
```after
item: List (java.util)
item: List (java.awt)
rank: List (java.util) < List (java.awt)
not item: Test (org.junit.jupiter.api)
```

### JAVA-CMP-009 case 5: Only interfaces after implements
settings: default
language: java
```before
--- src/main/java/p/Job.java
package p;

class Job implements Ru<caret> {
}
```
keys: complete
```after
item: Runnable (java.lang)
not item: Runtime (java.lang)
not item: RuntimeException (java.lang)
```

### JAVA-CMP-009 case 6: Only Throwable subtypes after throws
settings: default
language: java
```before
--- src/main/java/p/Reader.java
package p;

class Reader {
  void read() throws IOEx<caret> {
  }
}
```
keys: complete
```after
item: IOException (java.io)
not item: IOExceptionList
not item: String
```

### JAVA-CMP-009 case 7: Only Throwable subtypes in a catch parameter
settings: default
language: java
```before
--- src/main/java/p/Reader.java
package p;

class Reader {
  void read() {
    try {
    } catch (FileNotF<caret> e) {
    }
  }
}
```
keys: complete
```after
item: FileNotFoundException (java.io)
not item: FileNotFoundExceptionX
```

### JAVA-CMP-019 case 3: Choosing a Throwable type in a catch parameter adds the import
settings: default
language: java
```before
--- src/main/java/p/Reader.java
package p;

class Reader {
  void read() {
    try {
    } catch (FileNotF<caret> e) {
    }
  }
}
```
keys: complete, choose "FileNotFoundException (java.io)"
```after
package p;

import java.io.FileNotFoundException;

class Reader {
  void read() {
    try {
    } catch (FileNotFoundException<caret> e) {
    }
  }
}
```

### JAVA-CMP-020 case 1: A name clash inserts the fully qualified name
settings: default
language: java
```before
--- src/main/java/p/Clash.java
package p;

import java.util.Date;

class Clash {
  private Dat<caret>
}
```
keys: complete, choose "Date (java.sql)"
```after
package p;

import java.util.Date;

class Clash {
  private java.sql.Date<caret>
}
```

### JAVA-CMP-019 case 4: A class that is already imported inserts no second import
settings: default
language: java
```before
--- src/main/java/p/Reuse.java
package p;

import java.util.List;

class Reuse {
  private Lis<caret>
}
```
keys: complete, choose "List (java.util)"
```after
package p;

import java.util.List;

class Reuse {
  private List<caret>
}
```

### JAVA-CMP-018 case 3: Classes of the same package are offered without a package hint
settings: default
language: java
```before
--- src/main/java/p/Widget.java
package p;

public class Widget {
}
--- src/main/java/p/Use.java
package p;

class Use {
  private Wid<caret>
}
```
keys: complete, choose "Widget"
```after
package p;

class Use {
  private Widget<caret>
}
```

### JAVA-CMP-025 case 1: Platform-internal packages are not offered
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  private Unsa<caret>
}
```
keys: complete
```after
not item: Unsafe (sun.misc)
```

### JAVA-CMP-025 case 2: The third press includes library-internal classes
settings: default
language: java
```before
--- src/main/java/p/Main.java
package p;

class Main {
  private Unsa<caret>
}
```
keys: complete [count 3]
```after
item: Unsafe (sun.misc)
```

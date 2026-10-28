# Java imports: acceptance cases

Format: `docs/specs/java/cases/README.md`; requirements in `docs/specs/java/05-imports.md`. `keys: Alt+Enter,
choose "Import class"` presses Alt+Enter at the caret on an unresolved class reference and picks the fix; when
several classes qualify the chooser `Class to Import` opens and a second `choose "<fqn>"` picks its row (without it
the first row). `after` is the resulting file.

### JAVA-IMP-040 case 1: Import class on an unimported JUnit class (the owner's headline)
settings: library=junit5
language: java
```before
--- src/test/java/p/CalcTest.java
package p;

class CalcTest {
  void adds() {
    Ass<caret>ertions.assertEquals(1, 1);
  }
}
```
keys: Alt+Enter, choose "Import class"
```after
package p;

import org.junit.jupiter.api.Assertions;

class CalcTest {
  void adds() {
    Assertions.assertEquals(1, 1);
  }
}
```

### JAVA-IMP-040 case 2: The class is marked red before the fix
settings: library=junit5
language: java
```before
--- src/test/java/p/CalcTest.java
package p;

class CalcTest {
  void adds() {
    Assertions.assertEquals(1, 1);
  }
}
```
keys: inspect
```after
problem: error `Assertions` "Cannot resolve symbol 'Assertions'"
```

### JAVA-IMP-040 case 3: Import class on a type in a declaration
language: java
```before
class App {
  void run() {
    ArrayLis<caret>t<String> items;
  }
}
```
keys: Alt+Enter, choose "Import class"
```after
import java.util.ArrayList;

class App {
  void run() {
    ArrayList<String> items;
  }
}
```

### JAVA-IMP-040 case 4: Import class on a type that is still being typed (unterminated statement)
language: java
```before
import java.util.List;

class App {
  void run() {
    List<String> more = new ArrayList<caret>
  }
}
```
keys: Alt+Enter, choose "Import class"
```after
import java.util.ArrayList;
import java.util.List;

class App {
  void run() {
    List<String> more = new ArrayList
  }
}
```

### JAVA-IMP-041 case 1: Import class on an annotation name offers the annotation type of the test classpath
settings: library=junit5
language: java
```before
--- src/test/java/p/AppTest.java
package p;

class AppTest {
  @Te<caret>st
  void greets() {}
}
```
keys: Alt+Enter, choose "Import class"
```after
package p;

import org.junit.jupiter.api.Test;

class AppTest {
  @Test
  void greets() {}
}
```

### JAVA-IMP-042 case 1: `java.util.List` is ranked before `java.awt.List`
language: java
```before
class App {
  Li<caret>st<String> items;
}
```
keys: Alt+Enter, choose "Import class", choose "java.util.List"
```after
import java.util.List;

class App {
  List<String> items;
}
```

### JAVA-IMP-042 case 2: The first row of the chooser is the best ranked candidate
language: java
```before
class App {
  Li<caret>st<String> items;
}
```
keys: Alt+Enter, choose "Import class"
```after
import java.util.List;

class App {
  List<String> items;
}
```

### JAVA-IMP-040 case 5: A production file is not offered the class of a test library (JAVA-RES-029)
settings: library=junit5
language: java
```before
--- src/main/java/p/App.java
package p;

class App {
  void run() {
    Ass<caret>ertions.assertEquals(1, 1);
  }
}
```
keys: Alt+Enter
```after
no quick fix "Import class" in the popup
```

### JAVA-IMP-044 case 1: The import is added in its sorted position
language: java
```before
import java.util.List;
import java.util.Set;

class App {
  Li<caret>nkedList<String> a;
  List<String> b;
  Set<String> c;
}
```
keys: Alt+Enter, choose "Import class"
```after
import java.util.LinkedList;
import java.util.List;
import java.util.Set;

class App {
  LinkedList<String> a;
  List<String> b;
  Set<String> c;
}
```

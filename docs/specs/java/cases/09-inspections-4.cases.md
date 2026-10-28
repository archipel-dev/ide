# Cases for 09-inspections-catalog-4.md

Format: `docs/specs/java/cases/README.md` (extends `docs/specs/editor/cases/README.md`). Each case belongs to the requirement of the same ID in `docs/specs/java/09-inspections-catalog-4.md`. Keys `inspect` list the expected highlights as `problem:` lines, `inspect batch` runs the whole-project inspection, and `no problem reported` means no highlight.

### JAVA-ICB-001 case 1: Replace assert with if
settings: default
language: java
```before
class A {
    void m(String region) {
        <caret>assert region != null;
    }
}
```
keys: Alt+Enter, choose "Replace 'assert' with 'if' statement"
```after
class A {
    void m(String region) {
        if (region == null) throw new AssertionError();
    }
}
```

### JAVA-ICB-001 case 2: A plain if statement is not offered
settings: default
language: java
```before
class A {
    void m(String brand) {
        <caret>if (brand == null) return;
    }
}
```
keys: Alt+Enter
```after
no quick fix "Replace 'assert' with 'if' statement" in the popup
```

### JAVA-ICB-002 case 1: Replace with conditional expression
settings: default
language: java
```before
class A {
    boolean m(boolean a, boolean b, boolean c) {
        return <caret>a && b || !a && c;
    }
}
```
keys: Alt+Enter, choose "Replace with conditional expression"
```after
class A {
    boolean m(boolean a, boolean b, boolean c) {
        return a ? b : c;
    }
}
```

### JAVA-ICB-002 case 2: Unrelated operands are not reported
settings: default
language: java
```before
class A {
    boolean m(boolean a, boolean b, boolean c, boolean d) {
        return <caret>a && b || c && d;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-003 case 1: Break inside a loop
settings: default
language: java
```before
class A {
    void tune(java.util.List<String> pile) {
        for (String brand : pile) {
            if (brand.contains("stop")) <caret>break;
            survey(brand);
        }
    }
    void survey(String s) {}
}
```
keys: inspect
```after
problem: warning `break` "break; statement"
```

### JAVA-ICB-003 case 2: Break ending a switch branch is not reported
settings: default
language: java
```before
class A {
    void tally(int i) {
        switch (i) {
            case 1:
                System.out.println();
                <caret>break;
            default:
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-004 case 1: Labeled break
settings: default
language: java
```before
class A {
    void scan(int[][] grid) {
        rows:
        for (int[] row : grid) {
            for (int cell : row) {
                if (cell < 0) <caret>break rows;
            }
        }
    }
}
```
keys: inspect
```after
problem: warning `break` "break rows; statement with label"
```

### JAVA-ICB-005 case 1: Move condition to loop
settings: default
language: java
```before
class A {
    void spin(int left) {
        <caret>while (true) {
            if (left == 0) break;
            left--;
        }
    }
}
```
keys: Alt+Enter, choose "Move condition to loop"
```after
class A {
    void spin(int left) {
        while (left != 0) {
            left--;
        }
    }
}
```

### JAVA-ICB-005 case 2: Break in the middle of the body
settings: default
language: java
```before
class A {
    void spin(int step) {
        <caret>while (true) {
            step++;
            if (step == 9) break;
            step--;
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-006 case 1: Push conditional inside
settings: Ignore when conditional will be only argument of a method call=off
language: java
```before
class A {
    int pick(boolean flag, int x, int y) {
        return <caret>flag ? Math.abs(x) : Math.abs(y);
    }
}
```
keys: Alt+Enter, choose "Push conditional expression inside branch"
```after
class A {
    int pick(boolean flag, int x, int y) {
        return Math.abs(flag ? x : y);
    }
}
```

### JAVA-ICB-006 case 2: Branches that differ in the method are not reported
settings: default
language: java
```before
class A {
    double pick(boolean flag, double x) {
        return <caret>flag ? Math.abs(x) : Math.floor(x);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-007 case 1: Replace with if statement
settings: default
language: java
```before
class A {
    Object m(boolean c) {
        Object price = <caret>c ? render() : launch();
        return price;
    }
    Object render() { return null; }
    Object launch() { return null; }
}
```
keys: Alt+Enter, choose "Replace with 'if' statement"
```after
class A {
    Object m(boolean c) {
        Object price;
        if (c) {
            price = render();
        } else {
            price = launch();
        }
        return price;
    }
    Object render() { return null; }
    Object launch() { return null; }
}
```

### JAVA-ICB-007 case 2: Simple assignment ignored when the option is on
settings: Ignore for simple assignments and returns=on
language: java
```before
class A {
    String m(Object ratio) {
        String s = <caret>(ratio == null) ? "" : ratio.toString();
        return s;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-008 case 1: Collapse conditional expression
settings: default
language: java
```before
class A {
    String label(int n) {
        String s = <caret>n > 0 ? "same" : "same";
        return s;
    }
}
```
keys: Alt+Enter, choose "Collapse conditional expression"
```after
class A {
    String label(int n) {
        String s = "same";
        return s;
    }
}
```

### JAVA-ICB-008 case 2: Different branches
settings: default
language: java
```before
class A {
    int m(int x) {
        int y = <caret>x == 10 ? 4 : 5;
        return y;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-009 case 1: Remove redundant else
settings: default
language: java
```before
class A {
    void greet(String user) {
        if (user == null) {
            throw new IllegalStateException("no user");
        } <caret>else {
            System.out.println("hi " + user);
        }
    }
}
```
keys: Alt+Enter, choose "Remove redundant 'else'"
```after
class A {
    void greet(String user) {
        if (user == null) {
            throw new IllegalStateException("no user");
        }
        System.out.println("hi " + user);
    }
}
```

### JAVA-ICB-009 case 2: If branch completes normally
settings: default
language: java
```before
class A {
    void greet(String user) {
        if (user == null) {
            System.out.println("anonymous");
        } <caret>else {
            System.out.println("hi " + user);
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-010 case 1: Simplify
settings: default
language: java
```before
class A {
    int m() {
        return <caret>false ? 10 : 20;
    }
}
```
keys: Alt+Enter, choose "Simplify"
```after
class A {
    int m() {
        return 20;
    }
}
```

### JAVA-ICB-010 case 2: Non-constant condition
settings: default
language: java
```before
class A {
    String m(boolean b) {
        return <caret>b ? "Yes" : "No";
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-011 case 1: Continue in a loop
settings: default
language: java
```before
class A {
    void gather(java.util.List<String> pile) {
        for (String nickname : pile) {
            if (nickname.contains("skip")) <caret>continue;
            trace(nickname);
        }
    }
    void trace(String s) {}
}
```
keys: inspect
```after
problem: warning `continue` "continue; statement"
```

### JAVA-ICB-012 case 1: Labeled continue
settings: default
language: java
```before
class A {
    void scan(int[][] grid) {
        rows:
        for (int[] row : grid) {
            for (int cell : row) {
                if (cell < 0) <caret>continue rows;
            }
        }
    }
}
```
keys: inspect
```after
problem: warning `continue` "continue rows; statement with label"
```

### JAVA-ICB-013 case 1: Default moved last
settings: default
language: java
```before
class A {
    void m(int code) {
        switch (code) {
            <caret>default:
                reset();
                break;
            case 7:
                stop();
                break;
        }
    }
    void reset() {}
    void stop() {}
}
```
keys: Alt+Enter, choose "Make 'default' the last case"
```after
class A {
    void m(int code) {
        switch (code) {
            case 7:
                stop();
                break;
            default:
                reset();
                break;
        }
    }
    void reset() {}
    void stop() {}
}
```

### JAVA-ICB-013 case 2: Default already last
settings: default
language: java
```before
class A {
    void m(int n) {
        switch (n) {
            case 1:
                break;
            <caret>default:
                break;
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-014 case 1: Remove double negation
settings: default
language: java
```before
class A {
    boolean f() { return true; }
    void m() {
        if (<caret>!!f()) {}
    }
}
```
keys: Alt+Enter, choose "Remove double negation"
```after
class A {
    boolean f() { return true; }
    void m() {
        if (f()) {}
    }
}
```

### JAVA-ICB-014 case 2: Single negation
settings: default
language: java
```before
class A {
    boolean f() { return true; }
    void m() {
        if (<caret>!f()) {}
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-015 case 1: Duplicate operand
settings: default
language: java
```before
class A {
    boolean m(int width, int price) {
        return width != price || <caret>width != price;
    }
}
```
keys: inspect
```after
problem: warning `width != price` "Duplicate condition width != price"
```

### JAVA-ICB-015 case 2: Method calls with side effects ignored
settings: default
language: java
```before
class A {
    native boolean poll();
    void m() {
        if (poll() || <caret>poll()) {
            tick();
        }
    }
    void tick() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-016 case 1: Create missing branches
settings: default
status: unverified (arrow form of the generated branches for an empty switch)
language: java
```before
enum Size { S, M, L }
class X {
    void pick(Size size) {
        <caret>switch (size) {
        }
    }
}
```
keys: Alt+Enter, choose "Create missing branches 'S', 'M', and 'L'"
```after
enum Size { S, M, L }
class X {
    void pick(Size size) {
        switch (size) {
            case S -> {}
            case M -> {}
            case L -> {}
        }
    }
}
```

### JAVA-ICB-016 case 2: Default branch present
settings: default
language: java
```before
enum Size { S, M, L }
class X {
    void pick(Size size) {
        <caret>switch (size) {
            case S: break;
            default: break;
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-017 case 1: Factorize
settings: default
language: java
```before
class A {
    boolean m(boolean a, boolean b, boolean c) {
        return <caret>a && b || a && c;
    }
}
```
keys: Alt+Enter, choose "Replace with factorized expression"
```after
class A {
    boolean m(boolean a, boolean b, boolean c) {
        return a && (b || c);
    }
}
```

### JAVA-ICB-017 case 2: No common operand
settings: default
language: java
```before
class A {
    boolean m(boolean a, boolean b, boolean c, boolean d) {
        return <caret>a && b || c && d;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-018 case 1: Add break
settings: default
status: unverified (caret position on the reported statement and the exact quoting of the fix name)
language: java
```before
class A {
    void m(int mode) {
        switch (mode) {
            case 1:
                <caret>log("one");
            case 2:
                log("two");
        }
    }
    void log(String text) {}
}
```
keys: Alt+Enter, choose "Add 'break'"
```after
class A {
    void m(int mode) {
        switch (mode) {
            case 1:
                log("one");
                break;
            case 2:
                log("two");
        }
    }
    void log(String text) {}
}
```

### JAVA-ICB-019 case 1: Replace with while
settings: default
language: java
```before
class A {
    boolean weave() { return true; }
    void process() {}
    void m() {
        <caret>for (; weave(); ) {
            process();
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'while'"
```after
class A {
    boolean weave() { return true; }
    void process() {}
    void m() {
        while (weave()) {
            process();
        }
    }
}
```

### JAVA-ICB-019 case 2: For loop with initializer and update
settings: default
language: java
```before
class A {
    void m() {
        <caret>for (int i = 0; i < 3; i++) {
            System.out.println(i);
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-020 case 1: Missing condition
settings: default
language: java
```before
class A {
    void m() {
        <caret>for (int i = 0;; i++) {
            if (i > 5) break;
        }
    }
}
```
keys: inspect
```after
problem: warning `for` "for statement lacks condition"
```

### JAVA-ICB-021 case 1: Body does not update the condition
settings: default
language: java
```before
class A {
    void bundle(String nickname, String tag) {
        int weight = 1;
        String name = nickname;
        <caret>while (tag.contains(name)) {
            name = nickname + weight;
        }
    }
}
```
keys: inspect
```after
problem: warning `while` "Idempotent loop body"
```

### JAVA-ICB-021 case 2: Body updates the counter
settings: default
language: java
```before
class A {
    void fetch(String headline, String route) {
        int rank = 1;
        String name = headline;
        <caret>while (route.contains(name)) {
            name = headline + rank++;
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-022 case 1: Replace with assert
settings: default
language: java
```before
class A {
    void m(int n) {
        <caret>if (n < 0) throw new RuntimeException("negative");
    }
}
```
keys: Alt+Enter, choose "Replace statement with 'assert' statement"
```after
class A {
    void m(int n) {
        assert n >= 0 : "negative";
    }
}
```

### JAVA-ICB-022 case 2: If with else branch
settings: default
language: java
```before
class A {
    void m(int x) {
        <caret>if (x == 2) throw new RuntimeException("fail"); else System.out.println();
    }
}
```
keys: Alt+Enter
```after
no quick fix "Replace statement with 'assert' statement" in the popup
```

### JAVA-ICB-023 case 1: Extract common part
settings: default
status: unverified (the exact fix name depends on whether variables are involved)
language: java
```before
class A {
    void m(int x) {
        <caret>if (x > 12) {
            before();
            different1();
            after();
        } else {
            before();
            different2();
            after();
        }
    }
    void before() {} void after() {} void different1() {} void different2() {}
}
```
keys: Alt+Enter, choose "Extract common part from 'if'"
```after
class A {
    void m(int x) {
        before();
        if (x > 12) {
            different1();
        } else {
            different2();
        }
        after();
    }
    void before() {} void after() {} void different1() {} void different2() {}
}
```

### JAVA-ICB-023 case 2: Branches without a common part
settings: default
language: java
```before
class A {
    void m(int x) {
        <caret>if (x > 12) {
            different1();
        } else {
            different2();
        }
    }
    void different1() {} void different2() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-024 case 1: Four branches
settings: enable inspection IfStatementWithTooManyBranches
language: java
```before
class A {
    void m(int x) {
        <caret>if (x == 1) {} else if (x == 2) {} else if (x == 3) {} else if (x == 4) {}
    }
}
```
keys: inspect
```after
problem: warning `if` "if has too many branches (4)"
```

### JAVA-ICB-025 case 1: Empty infinite for
settings: default
language: java
```before
class A {
    void m() {
        <caret>for (;;) {
        }
    }
}
```
keys: inspect
```after
problem: warning `for` "for statement cannot complete without throwing an exception"
```

### JAVA-ICB-025 case 2: Loop with a return
settings: default
language: java
```before
class A {
    void m(boolean b) {
        <caret>while (true) {
            if (b) return;
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-026 case 1: Labeled loop
settings: enable inspection LabeledStatement
language: java
```before
class A {
    void m() {
        <caret>retry:
        for (int k = 0; k < 3; k++) {
            if (k == 2) break retry;
        }
    }
}
```
keys: inspect
```after
problem: warning `retry` "Labeled statement retry:"
```

### JAVA-ICB-027 case 1: Parameter never updated
settings: default
language: java
```before
class A {
    void spin(boolean ready) {
        while (<caret>ready) {
            work();
        }
    }
    void work() {}
}
```
keys: inspect
```after
problem: warning `ready` "Variable 'ready' is not updated inside loop"
```

### JAVA-ICB-027 case 2: Variable updated inside the loop
settings: default
language: java
```before
class A {
    void m(int n) {
        while (<caret>n > 0) {
            n--;
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-028 case 1: While that returns on the first iteration
settings: default
language: java
```before
class A {
    int m(boolean b) {
        <caret>while (b) {
            return 1;
        }
        return 0;
    }
}
```
keys: inspect
```after
problem: warning `while` "while statement does not loop"
```

### JAVA-ICB-028 case 2: Enhanced for ignored by default
settings: default
language: java
```before
class A {
    void m(Iterable<String> it) {
        <caret>for (String s : it) {
            System.out.println(s);
            break;
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-029 case 1: Make condition explicit
settings: enable inspection LoopWithImplicitTerminationCondition
language: java
```before
class A {
    void m(int i) {
        <caret>while (true) {
            if (i > 5) break;
            i++;
        }
    }
}
```
keys: Alt+Enter, choose "Make condition explicit"
```after
class A {
    void m(int i) {
        while (i <= 5) {
            i++;
        }
    }
}
```

### JAVA-ICB-030 case 1: Invert condition
settings: enable inspection NegatedConditional
language: java
```before
class A {
    int m(int a, int b) {
        return <caret>a != b ? 1 : 2;
    }
}
```
keys: Alt+Enter, choose "Invert condition"
```after
class A {
    int m(int a, int b) {
        return a == b ? 2 : 1;
    }
}
```

### JAVA-ICB-031 case 1: Remove negation
settings: enable inspection NegatedConditionalExpression
language: java
```before
class A {
    boolean m(int i, boolean a, boolean b) {
        return <caret>!(i == 1 ? a : b);
    }
}
```
keys: Alt+Enter, choose "Remove negation"
```after
class A {
    boolean m(int i, boolean a, boolean b) {
        return i == 1 ? !a : !b;
    }
}
```

### JAVA-ICB-032 case 1: Remove negation
settings: enable inspection NegatedEqualityExpression
language: java
```before
class A {
    boolean m(int i) {
        return !(i <caret>== 1);
    }
}
```
keys: Alt+Enter, choose "Remove negation"
```after
class A {
    boolean m(int i) {
        return i != 1;
    }
}
```

### JAVA-ICB-033 case 1: Invert if condition
settings: enable inspection NegatedIfElse
language: java
```before
class A {
    void m(int a, int b) {
        <caret>if (a != b) {
            first();
        } else {
            second();
        }
    }
    void first() {}
    void second() {}
}
```
keys: Alt+Enter, choose "Invert 'if' condition"
```after
class A {
    void m(int a, int b) {
        if (a == b) {
            second();
        } else {
            first();
        }
    }
    void first() {}
    void second() {}
}
```

### JAVA-ICB-034 case 1: Nested conditional
settings: enable inspection NestedConditionalExpression
language: java
```before
class A {
    int m(int p, int q) {
        int y = p > 0 ? <caret>q > 0 ? 1 : 2 : 3;
        return y;
    }
}
```
keys: inspect
```after
problem: warning `q > 0 ? 1 : 2` "Nested conditional expression q > 0 ? 1 : 2"
```

### JAVA-ICB-035 case 1: Nested switch expression
settings: enable inspection NestedSwitchStatement
language: java
```before
class A {
    int m(char kind, int size) {
        return switch (kind) {
            case 'a' -> 1;
            default -> <caret>switch (size) {
                case 2 -> 5;
                default -> 9;
            };
        };
    }
}
```
keys: inspect
```after
problem: warning `switch` "Nested switch expression"
```

### JAVA-ICB-036 case 1: Mixed operators, four terms
settings: enable inspection OverlyComplexBooleanExpression
language: java
```before
class A {
    boolean c(int i) { return i > 0; }
    boolean m(int x1, int x2, int x3, int x4) {
        return <caret>c(x1) && c(x2) ^ c(x3) && c(x4);
    }
}
```
keys: inspect
```after
problem: warning `c(x1) && c(x2) ^ c(x3) && c(x4)` "Overly complex boolean expression (4 terms)"
```

### JAVA-ICB-037 case 1: Simplify
settings: default
language: java
```before
class A {
    boolean m(boolean x) {
        boolean b = <caret>x && true;
        return b;
    }
}
```
keys: Alt+Enter, choose "Simplify"
```after
class A {
    boolean m(boolean x) {
        boolean b = x;
        return b;
    }
}
```

### JAVA-ICB-037 case 2: Named constant ignored by default
settings: default
language: java
```before
class A {
    static final boolean DEBUG = false;
    boolean m(boolean x) {
        boolean b = <caret>DEBUG || x;
        return b;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-038 case 1: Always false
settings: enable inspection PointlessIndexOfComparison
language: java
```before
class A {
    boolean m(String s) {
        return <caret>s.indexOf("a") < -1;
    }
}
```
keys: inspect
```after
problem: warning `s.indexOf("a") < -1` "s.indexOf(\"a\") < -1 is always false"
```

### JAVA-ICB-039 case 1: Remove null check
settings: default
language: java
```before
class A {
    boolean m(Object x, Class<?> score) {
        return <caret>x != null && score.isInstance(x);
    }
}
```
keys: Alt+Enter, choose "Remove unnecessary 'x != null' condition"
```after
class A {
    boolean m(Object x, Class<?> score) {
        return score.isInstance(x);
    }
}
```

### JAVA-ICB-039 case 2: Call accepts null
settings: default
language: java
```before
class A {
    boolean m(Object x) {
        return <caret>x != null && String.valueOf(x).isEmpty();
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-040 case 1: Simplify xor negation
settings: default
language: java
```before
class A {
    void f(boolean visible, boolean expired) {
        boolean b = <caret>!(visible ^ expired);
    }
}
```
keys: Alt+Enter, choose "Simplify"
```after
class A {
    void f(boolean visible, boolean expired) {
        boolean b = visible == expired;
    }
}
```

### JAVA-ICB-040 case 2: Already simple
settings: default
language: java
```before
class A {
    void f(boolean armed, boolean dirty) {
        boolean b = <caret>armed == dirty;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-041 case 1: condition ? true : expired
settings: default
language: java
```before
class A {
    boolean m(boolean condition, boolean expired) {
        return <caret>condition ? true : expired;
    }
}
```
keys: Alt+Enter, choose "Simplify"
```after
class A {
    boolean m(boolean condition, boolean expired) {
        return condition || expired;
    }
}
```

### JAVA-ICB-041 case 2: Branches unrelated
settings: default
language: java
```before
class A {
    int m(boolean condition, int a, int b) {
        return <caret>condition ? a : b;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-042 case 1: Push down
settings: default
language: java
```before
class A {
    void m(int shade) {
        <caret>switch (shade) {
            case 0 -> System.out.println("dark");
            case 1 -> System.out.println("mid");
            default -> System.out.println("light");
        }
    }
}
```
keys: Alt+Enter, choose "Push down 'switch' expression"
```after
class A {
    void m(int shade) {
        System.out.println(switch (shade) {
            case 0 -> "dark";
            case 1 -> "mid";
            default -> "light";
        });
    }
}
```

### JAVA-ICB-042 case 2: Branches call different methods
settings: default
language: java
```before
class A {
    void m(int shade) {
        <caret>switch (shade) {
            case 0 -> System.out.println("dark");
            default -> System.err.println("light");
        }
    }
}
```
keys: Alt+Enter
```after
no quick fix "Push down 'switch' expression" in the popup
```

### JAVA-ICB-043 case 1: Switch statement
settings: enable inspection SwitchStatement
language: java
```before
class A {
    void m(int i) {
        <caret>switch (i) {
        }
    }
}
```
keys: inspect
```after
problem: warning `switch` "switch statement"
```

### JAVA-ICB-044 case 1: One label, five statements
settings: enable inspection SwitchStatementDensity
status: unverified (exact percentage rounding)
language: java
```before
class A {
    int total;
    void m(int i) {
        switch (i) <caret>{
            case 1:
                total++;
                total *= 2;
                total -= 3;
                total += 4;
                total %= 5;
                break;
        }
    }
}
```
keys: inspect
```after
problem: warning `{` "branch density is too low (16%)"
```

### JAVA-ICB-045 case 1: Missing default
settings: default
language: java
```before
class A {
    void m(int i) {
        <caret>switch (i) {
            case 1:
                break;
        }
    }
}
```
keys: inspect
```after
problem: info `switch` "switch statement without 'default' branch"
```

### JAVA-ICB-045 case 2: Switch with default
settings: default
language: java
```before
class A {
    void m(int i) {
        <caret>switch (i) {
            case 1:
                break;
            default:
                break;
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-046 case 1: Variable shared between branches
settings: default
language: java
```before
class A {
    void m(int i) {
        switch (i) {
            case 1:
                String <caret>tmp = "a";
                break;
            case 2:
                tmp = "b";
                use(tmp);
                break;
        }
    }
    void use(String s) {}
}
```
keys: inspect
```after
problem: warning `tmp` "Local variable tmp declared in one 'switch' branch and used in another"
```

### JAVA-ICB-046 case 2: Variable used only in its branch
settings: default
language: java
```before
class A {
    void m(int i) {
        switch (i) {
            case 1:
                String <caret>tmp = "a";
                use(tmp);
                break;
            case 2:
                break;
        }
    }
    void use(String s) {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-047 case 1: Two labels with minimum 3
settings: Minimum number of branches=3
language: java
```before
class A {
    void forecast() {} void stamp() {}
    void m(String route) {
        <caret>switch (route) {
            case "foo" -> forecast();
            case "bar" -> stamp();
        }
    }
}
```
keys: Alt+Enter, choose "Replace 'switch' with 'if'"
```after
class A {
    void forecast() {} void stamp() {}
    void m(String route) {
        if ("foo".equals(route)) {
            forecast();
        } else if ("bar".equals(route)) {
            stamp();
        }
    }
}
```

### JAVA-ICB-047 case 2: Two labels with the default minimum
settings: default
language: java
```before
class A {
    void m(int i) {
        <caret>switch (i) {
            case 1 -> {}
            case 2 -> {}
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-048 case 1: Eleven labels
settings: enable inspection SwitchStatementWithTooManyBranches
language: java
```before
class A {
    void m(int x) {
        <caret>switch (x) {
            case 1 -> {} case 2 -> {} case 3 -> {} case 4 -> {} case 5 -> {} case 6 -> {}
            case 7 -> {} case 8 -> {} case 9 -> {} case 10 -> {} case 11 -> {}
            default -> {}
        }
    }
}
```
keys: inspect
```after
problem: warning `switch` "switch has too many branches (11)"
```

### JAVA-ICB-049 case 1: Simplify return
settings: default
language: java
```before
class A {
    boolean unpack() { return true; }
    boolean m() {
        <caret>if (unpack()) {
            return true;
        } else {
            return false;
        }
    }
}
```
keys: Alt+Enter, choose "Simplify 'if else'"
```after
class A {
    boolean unpack() { return true; }
    boolean m() {
        return unpack();
    }
}
```

### JAVA-ICB-049 case 2: If returning different non-boolean values
settings: default
language: java
```before
class A {
    int m(boolean c) {
        <caret>if (c) {
            return 1;
        } else {
            return 2;
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-050 case 1: Duplicate code before the loop
settings: default
language: java
```before
class A {
    void launch() {}
    void m(boolean x) {
        launch();
        <caret>while (x) {
            launch();
        }
    }
}
```
keys: Alt+Enter, choose "Replace 'while' with 'do while'"
```after
class A {
    void launch() {}
    void m(boolean x) {
        do {
            launch();
        } while (x);
    }
}
```

### JAVA-ICB-050 case 2: For comparison: a counting loop is not highlighted
settings: default
language: java
```before
class A {
    void m(boolean x) {
        <caret>while (x) {
            x = false;
        }
    }
}
```
keys: inspect
```after
no problem reported (the fix is only offered through Alt+Enter without a marker)
```

### JAVA-ICB-051 case 1: Every call is negated
settings: default
language: java
```before
class C {
    boolean <caret>isIdle() {
        return true;
    }
    void f() {
        if (!isIdle()) {
            return;
        }
    }
    boolean g() {
        return !isIdle();
    }
}
```
keys: inspect batch
```after
problem: warning `isIdle` "Calls to boolean method 'isIdle()' are always inverted"
```

### JAVA-ICB-051 case 2: One call is not negated
settings: default
language: java
```before
class C {
    boolean <caret>measure() {
        return true;
    }
    void f() {
        if (!measure()) {
            return;
        }
        if (measure()) {
            return;
        }
    }
}
```
keys: inspect batch
```after
no problem reported
```

### JAVA-ICB-052 case 1: Always negated
settings: enable inspection BooleanVariableAlwaysNegated
language: java
```before
class A {
    void m() {
        boolean <caret>ready = true;
        stop(!ready);
    }
    void stop(boolean flag) {}
}
```
keys: inspect
```after
problem: warning `ready` "Boolean variable ready is always inverted"
```

### JAVA-ICB-053 case 1: Replace with constant
settings: enable inspection ConstantValueVariableUse
language: java
```before
class A {
    private static void weave(double depth) {
        if (depth == 1.0) {
            f(<caret>depth);
        }
    }
    private static void f(double depth) {}
}
```
keys: Alt+Enter, choose "Replace with '1.0'"
```after
class A {
    private static void weave(double depth) {
        if (depth == 1.0) {
            f(1.0);
        }
    }
    private static void f(double depth) {}
}
```

### JAVA-ICB-054 case 1: Chained call on a project class
settings: enable inspection LawOfDemeter
language: java
```before
class Glacier { int quota; }
class Warehouse { Glacier getLaunch() { return null; } }
class A {
    int weave(Warehouse c) {
        return c.getLaunch().<caret>quota;
    }
}
```
keys: inspect
```after
problem: warning `quota` "Access of quota violates Law of Demeter"
```

### JAVA-ICB-055 case 1: Negative name
settings: enable inspection NegativelyNamedBooleanVariable
language: java
```before
class A {
    void m(String[] args) {
        boolean <caret>disabled = args.length == 0;
    }
}
```
keys: inspect
```after
problem: warning `disabled` "Boolean variable disabled is negatively named"
```

### JAVA-ICB-056 case 1: Reuse after use
settings: default
status: unverified (name chosen for the new variable)
language: java
```before
class A {
    void x() {
        int total = 5;
        log(total);
        <caret>total = 8;
        log(total);
    }
    void log(int v) {}
}
```
keys: Alt+Enter, choose "Split local variable"
```after
class A {
    void x() {
        int total = 5;
        log(total);
        int total1 = 8;
        log(total1);
    }
    void log(int v) {}
}
```

### JAVA-ICB-056 case 2: Assignment reading the old value
settings: default
language: java
```before
class A {
    void x() {
        int total = 5;
        log(total);
        <caret>total = total + 3;
        log(total);
    }
    void log(int v) {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-057 case 1: Move closer to usage
settings: Report variables with a new expression as initializer (potentially unsafe)=on
language: java
```before
import java.util.*;
class A {
    void m() {
        List<String> <caret>names = new ArrayList<>();
        work();
        names.add("a");
    }
    void work() {}
}
```
keys: Alt+Enter, choose "Move declaration of 'names' closer to usages"
```after
import java.util.*;
class A {
    void m() {
        work();
        List<String> names = new ArrayList<>();
        names.add("a");
    }
    void work() {}
}
```

### JAVA-ICB-057 case 2: Variable already next to its use
settings: default
language: java
```before
import java.util.*;
class A {
    void m() {
        work();
        List<String> <caret>names = new ArrayList<>();
        names.add("a");
    }
    void work() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-058 case 1: Returned immediately
settings: default
language: java
```before
class A {
    int area() {
        int <caret>total = 4 * 5;
        return total;
    }
}
```
keys: Alt+Enter, choose "Inline variable"
```after
class A {
    int area() {
        return 4 * 5;
    }
}
```

### JAVA-ICB-058 case 2: Variable used twice
settings: default
language: java
```before
class A {
    int m() {
        int <caret>x = compute();
        return x + x;
    }
    int compute() { return 1; }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-059 case 1: Qualify static call
settings: default
language: java
```before
class A {
    void m(String text) {
        String t = text.<caret>valueOf(42);
    }
}
```
keys: Alt+Enter, choose "Qualify static 'valueOf' call with reference to class 'String'"
```after
class A {
    void m(String text) {
        String t = String.valueOf(42);
    }
}
```

### JAVA-ICB-059 case 2: Already qualified by class
settings: default
language: java
```before
class A {
    void m() {
        String s1 = String.<caret>valueOf(0);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-060 case 1: Field never reassigned
settings: default
language: java
```before
public class Counter {
    private int <caret>limit;
    Counter(int limit) {
        this.limit = limit;
    }
    public int limit() {
        return limit;
    }
}
```
keys: inspect batch
```after
problem: warning `limit` "Declaration can have final modifier"
```

### JAVA-ICB-060 case 2: Field assigned outside the constructor
settings: default
language: java
```before
public class Dial {
    private int <caret>limit;
    void setLimit(int limit) {
        this.limit = limit;
    }
}
```
keys: inspect batch
```after
no problem reported
```

### JAVA-ICB-061 case 1: Remove redundant parameter
settings: default
language: java
```before
@interface Tag {
    String text() default "";
}
class A {
    @Tag(text = <caret>"")
    void run() {}
}
```
keys: Alt+Enter, choose "Remove redundant parameter"
```after
@interface Tag {
    String text() default "";
}
class A {
    @Tag()
    void run() {}
}
```

### JAVA-ICB-061 case 2: Non-default value
settings: default
language: java
```before
@interface Tag {
    String text() default "";
}
class A {
    @Tag(text = <caret>"x")
    void run() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-062 case 1: Remove duplicate
settings: default
language: java
```before
import java.io.IOException;
class A {
    void f() throws IOException, <caret>IOException {}
}
```
keys: Alt+Enter, choose "Remove 'IOException' from 'f' throws list"
```after
import java.io.IOException;
class A {
    void f() throws IOException {}
}
```

### JAVA-ICB-062 case 2: Distinct exceptions
settings: default
language: java
```before
class A {
    void f() throws java.io.IOException, <caret>InterruptedException {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-063 case 1: Delete empty initializer
settings: default
language: java
```before
class A {
    <caret>{
    }
}
```
keys: Alt+Enter, choose "Delete empty class initializer"
```after
class A {
}
```

### JAVA-ICB-063 case 2: Initializer with a statement
settings: default
language: java
```before
class A {
    int x;
    <caret>{
        x = 1;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-064 case 1: Record method
settings: default
language: java
```before
record Pair(String left, String right) {
    public <caret>final String joined() {
        return left + right;
    }
}
```
keys: Alt+Enter, choose "Remove 'final' modifier"
```after
record Pair(String left, String right) {
    public String joined() {
        return left + right;
    }
}
```

### JAVA-ICB-064 case 2: Non-final class
settings: default
language: java
```before
class Glacier {
    public <caret>final int dispatch() {
        return 1;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-065 case 1: Method reference
settings: default
language: java
```before
class A {
    void m(Runnable r) {
        javax.swing.SwingUtilities.invokeLater(r::<caret>run);
    }
}
```
keys: Alt+Enter, choose "Replace with qualifier"
```after
class A {
    void m(Runnable r) {
        javax.swing.SwingUtilities.invokeLater(r);
    }
}
```

### JAVA-ICB-065 case 2: Different method
settings: default
language: java
```before
class A {
    void m(Runnable r) {
        javax.swing.SwingUtilities.invokeLater(this::<caret>other);
    }
    void other() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-066 case 1: Redundant java.base
settings: default
language: java
```before
--- module-info.java
module m {
    requires <caret>java.base;
}
```
keys: inspect batch
```after
problem: warning `java.base` "Redundant directive 'requires java.base'. 'java.base' is implicitly required."
```

### JAVA-ICB-066 case 2: Used module
settings: default
language: java
```before
--- module-info.java
module m {
    requires <caret>java.logging;
}
--- m/A.java
import java.util.logging.Logger;
class A { Logger l; }
```
keys: inspect batch
```after
no problem reported
```

### JAVA-ICB-067 case 1: Provided but not exported
settings: default
language: java
```before
--- module-info.java
module acme.core {
    provides <caret>com.acme.PriceRule with com.acme.FlatRule;
}
--- com/acme/PriceRule.java
package com.acme;
public interface PriceRule {}
--- com/acme/FlatRule.java
package com.acme;
public class FlatRule implements PriceRule {}
```
keys: inspect
```after
problem: warning `com.acme.PriceRule` "Service interface provided but not exported or used"
```

### JAVA-ICB-067 case 2: Service package exported
settings: default
language: java
```before
--- module-info.java
module acme.core {
    exports com.acme;
    provides <caret>com.acme.PriceRule with com.acme.FlatRule;
}
--- com/acme/PriceRule.java
package com.acme;
public interface PriceRule {}
--- com/acme/FlatRule.java
package com.acme;
public class FlatRule implements PriceRule {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-068 case 1: Record member
settings: default
language: java
```before
record Span(int lo, int hi) {
    <caret>protected int width() {
        return hi - lo;
    }
}
```
keys: Alt+Enter, choose "Weaken visibility"
```after
record Span(int lo, int hi) {
    int width() {
        return hi - lo;
    }
}
```

### JAVA-ICB-068 case 2: Non-final class
settings: default
language: java
```before
class Orchard {
    <caret>protected int survey() {
        return 1;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-069 case 1: Last statement close
settings: default
language: java
```before
class A {
    void m() throws Exception {
        try (AutoCloseable ac = null) {
            weigh();
            ac.<caret>close();
        }
    }
    void weigh() {}
}
```
keys: Alt+Enter, choose "Remove redundant 'close()'"
```after
class A {
    void m() throws Exception {
        try (AutoCloseable ac = null) {
            weigh();
        }
    }
    void weigh() {}
}
```

### JAVA-ICB-069 case 2: Close before another statement
settings: default
language: java
```before
class A {
    void m() throws Exception {
        try (AutoCloseable ac = null) {
            ac.<caret>close();
            launch();
        }
    }
    void launch() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-070 case 1: Explicit types
settings: default
language: java
```before
class A {
    void m(java.util.Map<String, Integer> catalog) {
        catalog.forEach((<caret>String s, Integer i) -> System.out.println(s + "=" + i));
    }
}
```
keys: Alt+Enter, choose "Remove redundant parameter types"
```after
class A {
    void m(java.util.Map<String, Integer> catalog) {
        catalog.forEach((s, i) -> System.out.println(s + "=" + i));
    }
}
```

### JAVA-ICB-070 case 2: Implicit types
settings: default
language: java
```before
class A {
    void m(java.util.Map<String, Integer> lookup) {
        lookup.forEach((<caret>s, i) -> System.out.println(s + "=" + i));
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-071 case 1: Remove redundant canonical constructor
settings: default
status: unverified (the exact name substituted into "Remove {0}")
language: java
```before
record Tag(String key, String value) {
    public <caret>Tag(String key, String value) {
        this.key = key;
        this.value = value;
    }
}
```
keys: Alt+Enter, choose "Remove canonical constructor"
```after
record Tag(String key, String value) {
}
```

### JAVA-ICB-071 case 2: Convert to compact form
settings: default
language: java
```before
record Span(int lo, int hi) {
    public Span<caret>(int lo, int hi) {
        if (lo > hi) throw new IllegalStateException("bad range");
        this.lo = lo;
        this.hi = hi;
    }
}
```
keys: Alt+Enter, choose "Convert canonical constructor to compact form"
```after
record Span(int lo, int hi) {
    public Span {
        if (lo > hi) throw new IllegalStateException("bad range");
    }
}
```

### JAVA-ICB-072 case 1: Never thrown
settings: default
language: java
```before
import java.io.IOException;
class A {
    int count;
    void relay() throws <caret>IOException {
        count++;
    }
}
```
keys: inspect batch
```after
problem: warning `IOException` "The declared exception IOException is never thrown"
```

### JAVA-ICB-072 case 2: Actually thrown
settings: default
language: java
```before
class A {
    void shuffle() throws <caret>InterruptedException {
        Thread.sleep(1);
    }
}
```
keys: inspect batch
```after
no problem reported
```

### JAVA-ICB-073 case 1: Reifiable varargs
settings: default
status: unverified (exact highlighted element)
language: java
```before
class A {
    <caret>@SafeVarargs
    public final void bake(String... strings) {
    }
}
```
keys: inspect
```after
problem: warning `@SafeVarargs` "@SafeVarargs is not applicable to reifiable types"
```

### JAVA-ICB-073 case 2: Generic varargs
settings: default
language: java
```before
class A {
    <caret>@SafeVarargs
    public final <T> void process(T... items) {
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-074 case 1: Constant parameter
settings: default
language: java
```before
class A {
    static void show(int <caret>depth, int width) {
        System.out.println(depth + "x" + width);
    }
    public static void main(String[] argv) {
        show(2, 5);
        show(2, 7);
    }
}
```
keys: inspect batch
```after
problem: warning `depth` "Value of parameter depth is always '2'"
```

### JAVA-ICB-074 case 2: Different values
settings: default
language: java
```before
class A {
    static void show(int <caret>depth, int width) {
        System.out.println(depth + "x" + width);
    }
    public static void main(String[] argv) {
        show(2, 5);
        show(3, 7);
    }
}
```
keys: inspect batch
```after
no problem reported
```

### JAVA-ICB-075 case 1: Always returns 0 (batch)
settings: default
language: java
```before
class Y {
    int <caret>brew() {
        return 0;
    }
}
```
keys: inspect batch
```after
problem: warning `brew` "Method brew() always returns 0"
```

### JAVA-ICB-075 case 2: Different values
settings: default
language: java
```before
class Y {
    int <caret>brew(boolean b) {
        if (b) return 0;
        return 1;
    }
}
```
keys: inspect batch
```after
no problem reported
```

### JAVA-ICB-076 case 1: Remove self assignment
settings: default
language: java
```before
class A {
    int a;
    void m() {
        a = <caret>a;
    }
}
```
keys: Alt+Enter, choose "Remove self assignment"
```after
class A {
    int a;
    void m() {
    }
}
```

### JAVA-ICB-076 case 2: Different variables
settings: default
language: java
```before
class A {
    int a, b;
    void m() {
        a = <caret>b;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-077 case 1: Lambda call
settings: default
language: java
```before
import java.util.function.Function;
class A {
    int twice(int base) {
        return ((Function<Integer, Integer>) v -> {
            return v * 2 + base;
        }).<caret>apply(7);
    }
}
```
keys: Alt+Enter, choose "Replace method call on lambda with lambda body"
```after
import java.util.function.Function;
class A {
    int twice(int base) {
        return 7 * 2 + base;
    }
}
```

### JAVA-ICB-077 case 2: Lambda stored in a variable
settings: default
language: java
```before
import java.util.function.Predicate;
class A {
    boolean m(String name) {
        Predicate<String> p = x -> x.isEmpty();
        return p.<caret>test(name);
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-078 case 1: Unused dependency
settings: default
language: plain
```before
modules: app (depends on lib), lib
--- app/A.java
class A {}
--- lib/L.java
class L {}
```
keys: inspect batch
```after
problem: warning module `app` "Module 'app' sources do not depend on module 'lib' sources"
```

### JAVA-ICB-078 case 2: Used dependency
settings: default
language: plain
```before
modules: app (depends on lib), lib
--- app/A.java
class A { L l; }
--- lib/L.java
class L {}
```
keys: inspect batch
```after
no problem reported
```

### JAVA-ICB-079 case 1: Unused private method
settings: default
status: unverified (Safe delete shows a usages dialog only when usages exist)
language: java
```before
class A {
    private void <caret>helper() {}
    void m() {}
}
```
keys: Alt+Enter, choose "Safe delete 'helper()'"
```after
class A {
    void m() {}
}
```

### JAVA-ICB-079 case 2: Used private method
settings: default
language: java
```before
class A {
    private void <caret>helper() {}
    void m() { helper(); }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-080 case 1: Remove label
settings: default
language: java
```before
class A {
    void m() {
        int k = 0;
        <caret>outer: while (k < 5) {
            if (k == 3) {
                break;
            }
            k++;
        }
    }
}
```
keys: Alt+Enter, choose "Remove unused label"
```after
class A {
    void m() {
        int k = 0;
        while (k < 5) {
            if (k == 3) {
                break;
            }
            k++;
        }
    }
}
```

### JAVA-ICB-080 case 2: Used label
settings: default
language: java
```before
class A {
    void m() {
        int k = 0;
        <caret>outer: while (k < 5) {
            if (k == 3) {
                break outer;
            }
            k++;
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-081 case 1: Library without usages
settings: enable inspection UnusedLibrary
language: plain
```before
modules: app (library: libfoo.jar)
--- app/A.java
class A {}
```
keys: inspect batch
```after
problem: warning library `libfoo` "Unused library 'libfoo'"
```

### JAVA-ICB-082 case 1: Setter with ignored result
settings: default
status: unverified (Alt+Enter availability on the fly vs batch only)
language: java
```before
class A {
    private String owner;
    public String <caret>setAudit(String motto) {
        owner = motto;
        return owner;
    }
    void test() {
        setAudit("value");
    }
}
```
keys: Alt+Enter, choose "Make method 'void'"
```after
class A {
    private String owner;
    public void setAudit(String motto) {
        owner = motto;
    }
    void test() {
        setAudit("value");
    }
}
```

### JAVA-ICB-082 case 2: Result used
settings: default
language: java
```before
class A {
    public String <caret>up(String s) {
        return s.toUpperCase();
    }
    void test() {
        String r = up("a");
    }
}
```
keys: inspect batch
```after
no problem reported
```

### JAVA-ICB-083 case 1: Can be private
settings: enable inspection WeakerAccess
language: java
```before
class Lagoon {
    void audit() {
        schedule(3, 4);
    }
    void <caret>schedule(int a, int b) { }
}
```
keys: inspect batch
```after
problem: warning `schedule` "Can be private"
```

### JAVA-ICB-084 case 1: Class with 11 dependencies
settings: enable inspection ClassWithTooManyDependencies, Maximum number of dependencies=2
language: java
```before
--- A.java
class <caret>A { B b; C c; D d; }
--- B.java
class B {}
--- C.java
class C {}
--- D.java
class D {}
```
keys: inspect batch
```after
problem: warning `A` "Class 'A' has too many dependencies (3 > 2)"
```

### JAVA-ICB-085 case 1: Class used by three classes
settings: enable inspection ClassWithTooManyDependents, Maximum number of dependents=2
language: java
```before
--- Core.java
class <caret>Core {}
--- A.java
class A { Core c; }
--- B.java
class B { Core c; }
--- C.java
class C { Core c; }
```
keys: inspect batch
```after
problem: warning `Core` "Class 'Core' has too many dependents (3 > 2)"
```

### JAVA-ICB-086 case 1: Chain of three
settings: enable inspection ClassWithTooManyTransitiveDependencies, Maximum number of transitive dependencies=2
language: java
```before
--- A.java
class <caret>A { B b; }
--- B.java
class B { C c; }
--- C.java
class C { D d; }
--- D.java
class D {}
```
keys: inspect batch
```after
problem: warning `A` "Class 'A' has too many transitive dependencies (3 > 2)"
```

### JAVA-ICB-087 case 1: Chain of dependents
settings: enable inspection ClassWithTooManyTransitiveDependents, Maximum number of transitive dependents=2
language: java
```before
--- D.java
class <caret>D {}
--- C.java
class C { D d; }
--- B.java
class B { C c; }
--- A.java
class A { B b; }
```
keys: inspect batch
```after
problem: warning `D` "Class 'D' has too many transitive dependencies (3 > 2)"
```

### JAVA-ICB-088 case 1: Two classes referencing each other
settings: enable inspection CyclicClassDependency
language: java
```before
--- A.java
class <caret>A { B b; }
--- B.java
class B { A a; }
```
keys: inspect batch
```after
problem: warning `A` "Class 'A' is cyclically dependent on class 'B'"
```

### JAVA-ICB-089 case 1: Two packages referencing each other
settings: enable inspection CyclicPackageDependency
language: java
```before
--- p/A.java
package p;
class A { q.B b; }
--- q/B.java
package q;
public class B { p.A a; }
```
keys: inspect batch
```after
problem: warning package `p` "Package 'p' is cyclically dependent on package 'q'"
```

### JAVA-ICB-090 case 1: Package-private access across modules
settings: enable inspection SuspiciousPackagePrivateAccess
status: unverified (exact subject text of {0})
language: java
```before
modules: app (depends on lib), lib
--- lib/p/Util.java
package p;
public class Util { static void help() {} }
--- app/p/User.java
package p;
class User { void m() { Util.<caret>help(); } }
```
keys: inspect
```after
problem: warning `help` "'help()' is package-private, but declared in a different module 'lib'"
```

### JAVA-ICB-091 case 1: Return of array field
settings: enable inspection AssignmentOrReturnOfFieldWithMutableType
language: java
```before
class Turbine {
    private String[] tray = {"one", "two", "three"};
    String[] getAudit() {
        return <caret>tray;
    }
}
```
keys: Alt+Enter, choose "Replace with 'tray.clone()'"
```after
class Turbine {
    private String[] tray = {"one", "two", "three"};
    String[] getAudit() {
        return tray.clone();
    }
}
```

### JAVA-ICB-092 case 1: Package-private field
settings: enable inspection PackageVisibleField
language: java
```before
public class A {
    String <caret>label;
    static final int LIMIT = 5;
}
```
keys: inspect
```after
problem: warning `label` "Package-visible field label"
```

### JAVA-ICB-093 case 1: Package-private nested class
settings: enable inspection PackageVisibleInnerClass
language: java
```before
public class Beacon {
    static class <caret>Jeweler {}
}
```
keys: inspect
```after
problem: warning `Jeweler` "Package-visible nested class Jeweler"
```

### JAVA-ICB-094 case 1: Protected field
settings: enable inspection ProtectedField
language: java
```before
public class A {
    protected String <caret>label;
    protected static final int LIMIT = 5;
}
```
keys: inspect
```after
problem: warning `label` "Protected field label"
```

### JAVA-ICB-095 case 1: Protected nested class
settings: enable inspection ProtectedInnerClass
language: java
```before
public class Booking {
    protected static class <caret>Heron {}
}
```
keys: inspect
```after
problem: warning `Heron` "Protected nested class Heron"
```

### JAVA-ICB-096 case 1: Encapsulate public field
settings: default
status: unverified (Encapsulate Fields dialog defaults)
language: java
```before
class Loom {
    public int <caret>count;
}
```
keys: Alt+Enter, choose "Encapsulate field 'count'", Enter in the dialog (defaults: private field, getter and setter, update usages)
```after
class Loom {
    private int count;

    public int getCount() {
        return count;
    }

    public void setCount(int count) {
        this.count = count;
    }
}
```

### JAVA-ICB-096 case 2: Constant
settings: default
language: java
```before
class Lighthouse {
    public static final String <caret>NAME = "x";
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-097 case 1: Public nested class
settings: enable inspection PublicInnerClass
language: java
```before
public class Ingredient {
    public static class <caret>Garden {}
}
```
keys: inspect
```after
problem: warning `Garden` "'public' nested class Garden"
```

### JAVA-ICB-098 case 1: Access to another object's field
settings: enable inspection UseOfAnotherObjectsPrivateField
language: java
```before
public class Comet {
    protected int tick;
    void merge(Comet peer) {
        this.tick += 1;
        peer.<caret>tick += 1;
    }
}
```
keys: inspect
```after
problem: warning `tick` "Direct access to non-public field tick of another object"
```

### JAVA-ICB-099 case 1: Catch NPE
settings: enable inspection BadExceptionCaught
language: java
```before
class A {
    boolean m(java.awt.Component dims) {
        try {
            return dims.getMousePosition(true) != null;
        } catch (<caret>NullPointerException e) {
            return false;
        }
    }
}
```
keys: inspect
```after
problem: warning `NullPointerException` "Prohibited exception NullPointerException caught"
```

### JAVA-ICB-100 case 1: Throws Exception
settings: enable inspection BadExceptionDeclared
language: java
```before
class A {
    void assemble(String region) throws <caret>Exception {}
}
```
keys: inspect
```after
problem: warning `Exception` "Prohibited exception Exception declared"
```

### JAVA-ICB-101 case 1: Throw RuntimeException
settings: enable inspection BadExceptionThrown
language: java
```before
class A {
    void weave(Object slot) {
        if (slot == null)
            <caret>throw new RuntimeException("Problem during setup");
    }
}
```
keys: inspect
```after
problem: warning `throw` "Prohibited exception 'RuntimeException' thrown"
```

### JAVA-ICB-102 case 1: Rename to ignored
settings: default
language: java
```before
import java.io.IOException;
class A {
    void m() {
        try {
            bake();
        } <caret>catch (IOException ex) {
        }
    }
    void bake() throws IOException {}
}
```
keys: Alt+Enter, choose "Rename 'ex' to 'ignored'"
```after
import java.io.IOException;
class A {
    void m() {
        try {
            bake();
        } catch (IOException ignored) {
        }
    }
    void bake() throws IOException {}
}
```

### JAVA-ICB-102 case 2: Parameter named ignored
settings: default
language: java
```before
import java.io.IOException;
class A {
    void m() {
        try {
            stamp();
        } <caret>catch (IOException ignored) {
        }
    }
    void stamp() throws IOException {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-103 case 1: Delete catch section
settings: default
language: java
```before
class A {
    void m() throws InterruptedException {
        try {
            Thread.sleep(10);
        } catch (InterruptedException <caret>e) {
            throw e;
        }
    }
}
```
keys: Alt+Enter, choose "Delete 'catch' section"
```after
class A {
    void m() throws InterruptedException {
        Thread.sleep(10);
    }
}
```

### JAVA-ICB-103 case 2: Catch does additional work
settings: default
language: java
```before
class A {
    void m() throws InterruptedException {
        try {
            Thread.sleep(10);
        } catch (InterruptedException <caret>e) {
            System.out.println("interrupted");
            throw e;
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-104 case 1: Checked exception class
settings: enable inspection CheckedExceptionClass
language: java
```before
class <caret>RangerException extends Exception {}
```
keys: inspect
```after
problem: warning `RangerException` "Checked exception class RangerException"
```

### JAVA-ICB-105 case 1: Continue in finally
settings: default
language: java
```before
class A {
    void m() {
        for (int k = 0; k < 3; k++) {
            try {
                work();
            } finally {
                <caret>continue;
            }
        }
    }
    void work() {}
}
```
keys: inspect
```after
problem: warning `continue` "continue inside 'finally' block"
```

### JAVA-ICB-105 case 2: Loop inside finally
settings: default
language: java
```before
class A {
    void m() {
        try {
            throwing();
        } finally {
            for (int i = 0; i < 3; i++) {
                <caret>continue;
            }
        }
    }
    void throwing() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-106 case 1: Remove finally block
settings: default
language: java
```before
import java.nio.file.*;
class A {
    void m() {
        try {
            Files.readAllBytes(Paths.get("data.bin"));
        } catch (java.io.IOException e) {
            e.printStackTrace();
        } <caret>finally {
        }
    }
}
```
keys: Alt+Enter, choose "Remove 'finally' block"
```after
import java.nio.file.*;
class A {
    void m() {
        try {
            Files.readAllBytes(Paths.get("data.bin"));
        } catch (java.io.IOException e) {
            e.printStackTrace();
        }
    }
}
```

### JAVA-ICB-106 case 2: Finally with a statement
settings: default
language: java
```before
class A {
    void m() {
        try {
            System.out.println();
        } <caret>finally {
            System.out.println("done");
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-107 case 1: Empty try
settings: default
language: java
```before
class A {
    void m() {
        <caret>try {
        } finally {
            System.out.println();
        }
    }
}
```
keys: inspect
```after
problem: warning `try` "Empty try block"
```

### JAVA-ICB-107 case 2: Try with a statement
settings: default
language: java
```before
class A {
    void m() {
        <caret>try {
            System.out.println();
        } finally {
            System.out.println();
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-108 case 1: OutOfMemoryError swallowed
settings: enable inspection ErrorRethrown
language: java
```before
class A {
    boolean m() {
        try {
            run();
        } catch (OutOfMemoryError <caret>ex) {
            return false;
        }
        return true;
    }
    void run() {}
}
```
keys: inspect
```after
problem: warning `ex` "Error ex not rethrown"
```

### JAVA-ICB-109 case 1: Exception not wrapped
settings: enable inspection ExceptionFromCatchWhichDoesntWrap
language: java
```before
class A {
    void m() throws java.net.ConnectException {
        try {
            run();
        } catch (java.io.IOException e) {
            <caret>throw new java.net.ConnectException("Connection problem.");
        }
    }
    void run() throws java.io.IOException {}
}
```
keys: inspect
```after
problem: warning `throw` "throw inside 'catch' block ignores the caught exception"
```

### JAVA-ICB-110 case 1: Direct subclass
settings: enable inspection ExtendsThrowable
language: java
```before
class <caret>Anchor extends Throwable {}
```
keys: inspect
```after
problem: warning `Anchor` "Class Anchor directly extends 'java.lang.Throwable'"
```

### JAVA-ICB-111 case 1: Return in finally
settings: default
language: java
```before
class A {
    void x() {
        try {
            throw new IllegalStateException("boom");
        } <caret>finally {
            return;
        }
    }
}
```
keys: inspect
```after
problem: warning `finally` "finally block can not complete normally"
```

### JAVA-ICB-111 case 2: Conditional return
settings: default
status: unverified (assumes the block is judged by whether it can complete normally; the statement itself is reported by ReturnFromFinallyBlock)
language: java
```before
class A {
    void x(boolean b) {
        try {
            throw new IllegalStateException("boom");
        } <caret>finally {
            if (b) return;
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-112 case 1: Instanceof on catch parameter
settings: enable inspection InstanceofCatchParameter
language: java
```before
class A {
    void gather(Runnable task) {
        try {
            task.run();
        } catch (Exception err) {
            if (<caret>err instanceof IllegalStateException) {
                recover();
            }
        }
    }
    void recover() {}
}
```
keys: inspect
```after
problem: warning `err` "'instanceof' on 'catch' parameter err"
```

### JAVA-ICB-113 case 1: Nested try
settings: enable inspection NestedTryStatement
language: java
```before
class A {
    void m() {
        try {
            <caret>try {
                run();
            } catch (RuntimeException e) {
            }
        } finally {
            run();
        }
    }
    void run() {}
}
```
keys: inspect
```after
problem: warning `try` "Nested try statement"
```

### JAVA-ICB-114 case 1: Exception without message
settings: enable inspection NewExceptionWithoutArguments
status: unverified (text substituted for #ref)
language: java
```before
class A {
    void m() throws java.io.IOException {
        throw <caret>new java.io.IOException();
    }
}
```
keys: inspect
```after
problem: warning `new` "new IOException() without arguments"
```

### JAVA-ICB-115 case 1: Non-final field
settings: enable inspection NonFinalFieldOfException
status: unverified (the field has no initializer so the result does not compile; the fix itself only adds `final`)
language: java
```before
public class ReceiptException extends Exception {
    private String <caret>headline;
}
```
keys: Alt+Enter, choose "Make 'headline' 'final'"
```after
public class ReceiptException extends Exception {
    private final String headline;
}
```

### JAVA-ICB-116 case 1: Throw null
settings: enable inspection NullThrown
status: unverified (whether the generated expression is shortened to the simple name)
language: java
```before
class A {
    void m() {
        throw <caret>null;
    }
}
```
keys: Alt+Enter, choose "Replace with 'java.lang.NullPointerException'"
```after
class A {
    void m() {
        throw new NullPointerException();
    }
}
```

### JAVA-ICB-117 case 1: Return in finally
settings: default
language: java
```before
class A {
    boolean pack() { return true; }
    void m() {
        try {
            fetch();
        } finally {
            if (pack()) <caret>return;
        }
    }
    void fetch() {}
}
```
keys: inspect
```after
problem: warning `return;` "'return' inside 'finally' block"
```

### JAVA-ICB-117 case 2: Return inside a lambda in finally
settings: default
language: java
```before
class A {
    void m() {
        try {
            ration();
        } finally {
            Runnable r = () -> { <caret>return; };
        }
    }
    void ration() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-118 case 1: ThreadDeath swallowed
settings: enable inspection ThreadDeathRethrown
language: java
```before
class A {
    boolean m() {
        try {
            run();
        } catch (ThreadDeath <caret>ex) {
            return false;
        }
        return true;
    }
    void run() {}
}
```
keys: inspect
```after
problem: warning `ex` "ThreadDeath ex not rethrown"
```

### JAVA-ICB-119 case 1: Supplier that throws
settings: default
language: java
```before
import java.util.Optional;
class A {
    void m(Optional<String> title) {
        title.orElseThrow(() -> {
            <caret>throw new IllegalArgumentException("none");
        });
    }
}
```
keys: Alt+Enter, choose "Replace 'throw' with 'return' in lambda"
```after
import java.util.Optional;
class A {
    void m(Optional<String> title) {
        title.orElseThrow(() -> new IllegalArgumentException("none"));
    }
}
```

### JAVA-ICB-119 case 2: Supplier that returns
settings: default
language: java
```before
import java.util.Optional;
class A {
    void m(Optional<String> crew) {
        crew.orElseThrow(() -> <caret>new RuntimeException());
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-120 case 1: Throw caught by own try
settings: enable inspection ThrowCaughtLocally
language: java
```before
class A {
    void m(int n) {
        try {
            if (n < 0) {
                <caret>throw new IllegalArgumentException("negative");
            }
        } catch (Exception e) {
            n = 0;
        }
    }
}
```
keys: inspect
```after
problem: warning `throw` "throw caught by containing 'try' statement"
```

### JAVA-ICB-121 case 1: Throw in finally
settings: default
language: java
```before
class Pebble {
    void brew(Kettle k) {
        try {
            k.boil();
        } finally {
            k.cool();
            <caret>throw new IllegalStateException("cold");
        }
    }
    interface Kettle { void boil(); void cool(); }
}
```
keys: inspect
```after
problem: warning `throw` "throw inside 'finally' block"
```

### JAVA-ICB-121 case 2: Throw inside a lambda in finally
settings: default
language: java
```before
class A {
    void m() {
        try {
            run();
        } finally {
            Runnable r = () -> { <caret>throw new IllegalStateException(); };
        }
    }
    void run() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-122 case 1: Remove from throws
settings: enable inspection ThrowsRuntimeException
language: java
```before
class JewelerException extends RuntimeException {}
class Bridge {
    void pack() throws <caret>JewelerException {}
}
```
keys: Alt+Enter, choose "Remove 'JewelerException' from 'throws' clause"
```after
class JewelerException extends RuntimeException {}
class Bridge {
    void pack() {}
}
```

### JAVA-ICB-123 case 1: Replace with RuntimeException catch
settings: enable inspection TooBroadCatch
language: java
```before
class A {
    String m(String text) {
        try {
            return String.valueOf(Integer.parseInt(text));
        } catch (<caret>Exception ex) {
            return "?";
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'catch' clause for 'RuntimeException'"
```after
class A {
    String m(String text) {
        try {
            return String.valueOf(Integer.parseInt(text));
        } catch (RuntimeException ex) {
            return "?";
        }
    }
}
```

### JAVA-ICB-124 case 1: Replace with specific exceptions
settings: enable inspection TooBroadThrows
language: java
```before
import java.io.File;
class A {
    public void createRefill() throws <caret>Exception {
        File tag = new File("pathToFile");
        tag.createNewFile();
    }
}
```
keys: Alt+Enter, choose "Replace with specific exceptions"
```after
import java.io.File;
import java.io.IOException;
class A {
    public void createRefill() throws IOException {
        File tag = new File("pathToFile");
        tag.createNewFile();
    }
}
```

### JAVA-ICB-125 case 1: Unchecked exception class
settings: enable inspection UncheckedExceptionClass
language: java
```before
class <caret>AtelierException extends RuntimeException {}
```
keys: inspect
```after
problem: warning `AtelierException` "Unchecked exception class AtelierException"
```

### JAVA-ICB-126 case 1: Move cause into constructor
settings: default
language: java
```before
class A {
    void m() {
        try {
            process();
        } catch (IllegalStateException ex) {
            IllegalStateException wrapper = new IllegalStateException("step failed");
            wrapper.<caret>initCause(ex);
            throw wrapper;
        }
    }
    void process() {}
}
```
keys: Alt+Enter, choose "Remove 'Throwable.initCause()' call"
```after
class A {
    void m() {
        try {
            process();
        } catch (IllegalStateException ex) {
            IllegalStateException wrapper = new IllegalStateException("step failed", ex);
            throw wrapper;
        }
    }
    void process() {}
}
```

### JAVA-ICB-126 case 2: Exception without a cause constructor
settings: default
language: java
```before
class A {
    static class MyEx extends Exception {
        MyEx(String m) { super(m); }
    }
    void m(Exception ex) throws MyEx {
        MyEx level = new MyEx("x");
        level.<caret>initCause(ex);
        throw level;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-127 case 1: Non-trivial finalize
settings: enable inspection Finalize
language: java
```before
class A {
    protected void <caret>finalize() throws Throwable {
        System.out.println("bye");
    }
}
```
keys: inspect
```after
problem: warning `finalize` "'finalize()' should not be overridden"
```

### JAVA-ICB-128 case 1: Public finalize
settings: default
language: java
```before
class Handle {
    public void <caret>finalize() {
        cleanup();
    }
    void cleanup() {}
}
```
keys: Alt+Enter, choose "Make 'protected'"
```after
class Handle {
    protected void finalize() {
        cleanup();
    }
    void cleanup() {}
}
```

### JAVA-ICB-128 case 2: Protected finalize
settings: default
language: java
```before
class Handle {
    protected void <caret>finalize() {
        cleanup();
    }
    void cleanup() {}
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-129 case 1: Explicit call
settings: default
language: java
```before
class A {
    void m(A other) throws Throwable {
        other.<caret>finalize();
    }
}
```
keys: inspect
```after
problem: warning `finalize` "finalize() called explicitly"
```

### JAVA-ICB-129 case 2: super.finalize in finalize
settings: default
language: java
```before
class A {
    @Override
    protected void finalize() throws Throwable {
        super.<caret>finalize();
    }
}
```
keys: inspect
```after
no problem reported
```

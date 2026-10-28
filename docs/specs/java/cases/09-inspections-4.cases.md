# Cases for 09-inspections-catalog-4.md

Format: see docs/specs/editor/cases/README.md. Inspection cases: `keys: Alt+Enter, choose "<fix>"` applies the named quick fix at the caret; `keys: inspect` runs the inspection and `after` then lists `problem: <level> <range text> "<message>"` lines or `no problem reported`.

### JAVA-ICB-001 case 1: Replace assert with if
settings: default
language: java
```before
class A {
    void m(String param) {
        <caret>assert param != null;
    }
}
```
keys: Alt+Enter, choose "Replace 'assert' with 'if' statement"
```after
class A {
    void m(String param) {
        if (param == null) throw new AssertionError();
    }
}
```

### JAVA-ICB-001 case 2: A plain if statement is not offered
settings: default
language: java
```before
class A {
    void m(String param) {
        <caret>if (param == null) return;
    }
}
```
keys: Alt+Enter
```after
the intention list does not contain "Replace 'assert' with 'if' statement"; no problem reported
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
    void foo(java.util.List<String> strs) {
        for (String str : strs) {
            if (str.contains("stop")) <caret>break;
            handleStr(str);
        }
    }
    void handleStr(String s) {}
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
    void foo(int i) {
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
    void handle(java.util.List<String> strs) {
        outer:
        for (String s : strs) {
            for (char ch : s.toCharArray()) {
                if ('s' == ch) <caret>break outer;
            }
        }
    }
}
```
keys: inspect
```after
problem: warning `break` "break outer; statement with label"
```

### JAVA-ICB-005 case 1: Move condition to loop
settings: default
language: java
```before
class A {
    void m(int i) {
        <caret>while (true) {
            if (i == 23) break;
            i++;
        }
    }
}
```
keys: Alt+Enter, choose "Move condition to loop"
```after
class A {
    void m(int i) {
        while (i != 23) {
            i++;
        }
    }
}
```

### JAVA-ICB-005 case 2: Break in the middle of the body
settings: default
language: java
```before
class A {
    void m(int i) {
        <caret>while (true) {
            i++;
            if (i == 23) break;
            i--;
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
    double g(int a, int b) {
        return <caret>a == b ? Math.cos(0) : Math.cos(1);
    }
}
```
keys: Alt+Enter, choose "Push conditional expression inside branch"
```after
class A {
    double g(int a, int b) {
        return Math.cos(a == b ? 0 : 1);
    }
}
```

### JAVA-ICB-006 case 2: Branches that differ in the method are not reported
settings: default
language: java
```before
class A {
    double g(int a, int b) {
        return <caret>a == b ? Math.cos(0) : Math.sin(0);
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
        Object result = <caret>c ? foo() : bar();
        return result;
    }
    Object foo() { return null; }
    Object bar() { return null; }
}
```
keys: Alt+Enter, choose "Replace with 'if' statement"
```after
class A {
    Object m(boolean c) {
        Object result;
        if (c) {
            result = foo();
        } else {
            result = bar();
        }
        return result;
    }
    Object foo() { return null; }
    Object bar() { return null; }
}
```

### JAVA-ICB-007 case 2: Simple assignment ignored when the option is on
settings: Ignore for simple assignments and returns=on
language: java
```before
class A {
    String m(Object foo) {
        String s = <caret>(foo == null) ? "" : foo.toString();
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
    int m(int x) {
        int y = <caret>x == 10 ? 4 : 4;
        return y;
    }
}
```
keys: Alt+Enter, choose "Collapse conditional expression"
```after
class A {
    int m(int x) {
        int y = 4;
        return y;
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
    void m(String name) {
        if (name == null) {
            throw new IllegalArgumentException();
        } <caret>else {
            System.out.println(name);
        }
    }
}
```
keys: Alt+Enter, choose "Remove redundant 'else'"
```after
class A {
    void m(String name) {
        if (name == null) {
            throw new IllegalArgumentException();
        }
        System.out.println(name);
    }
}
```

### JAVA-ICB-009 case 2: If branch completes normally
settings: default
language: java
```before
class A {
    void m(String name) {
        if (name == null) {
            System.out.println("null");
        } <caret>else {
            System.out.println(name);
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
    String m() {
        return <caret>true ? "Yes" : "No";
    }
}
```
keys: Alt+Enter, choose "Simplify"
```after
class A {
    String m() {
        return "Yes";
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
    void foo(java.util.List<String> strs) {
        for (String str : strs) {
            if (str.contains("skip")) <caret>continue;
            handleStr(str);
        }
    }
    void handleStr(String s) {}
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
    void handle(java.util.List<String> strs) {
        outer:
        for (String s : strs) {
            for (char ch : s.toCharArray()) {
                if ('s' == ch) <caret>continue outer;
            }
        }
    }
}
```
keys: inspect
```after
problem: warning `continue` "continue outer; statement with label"
```

### JAVA-ICB-013 case 1: Default moved last
settings: default
language: java
```before
class A {
    void m(int n) {
        switch (n) {
            <caret>default:
                System.out.println();
                break;
            case 1:
                break;
        }
    }
}
```
keys: Alt+Enter, choose "Make 'default' the last case"
```after
class A {
    void m(int n) {
        switch (n) {
            case 1:
                break;
            default:
                System.out.println();
                break;
        }
    }
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
    boolean m(int digit1, int digit2) {
        return digit1 != digit2 || <caret>digit1 != digit2;
    }
}
```
keys: inspect
```after
problem: warning `digit1 != digit2` "Duplicate condition digit1 != digit2"
```

### JAVA-ICB-015 case 2: Method calls with side effects ignored
settings: default
language: java
```before
class A {
    native boolean unknownMethod();
    void m() {
        if (unknownMethod() || <caret>unknownMethod()) {
            System.out.println("Got it");
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-016 case 1: Create missing branches
settings: default
language: java
status: unverified (arrow form of the generated branches for an empty switch)
```before
enum E { A, B, C }
class X {
    void x(E e) {
        <caret>switch (e) {
        }
    }
}
```
keys: Alt+Enter, choose "Create missing branches 'A', 'B', and 'C'"
```after
enum E { A, B, C }
class X {
    void x(E e) {
        switch (e) {
            case A -> {}
            case B -> {}
            case C -> {}
        }
    }
}
```

### JAVA-ICB-016 case 2: Default branch present
settings: default
language: java
```before
enum E { A, B, C }
class X {
    void x(E e) {
        <caret>switch (e) {
            case A: break;
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
language: java
status: unverified (caret position on the reported statement and the exact quoting of the fix name)
```before
class A {
    void m(int x) {
        switch (x) {
            case 4:
                <caret>System.out.println("3");
            case 6:
                System.out.println("4");
        }
    }
}
```
keys: Alt+Enter, choose "Add 'break'"
```after
class A {
    void m(int x) {
        switch (x) {
            case 4:
                System.out.println("3");
                break;
            case 6:
                System.out.println("4");
        }
    }
}
```

### JAVA-ICB-019 case 1: Replace with while
settings: default
language: java
```before
class A {
    boolean exitCondition() { return true; }
    void process() {}
    void m() {
        <caret>for (; exitCondition(); ) {
            process();
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'while'"
```after
class A {
    boolean exitCondition() { return true; }
    void process() {}
    void m() {
        while (exitCondition()) {
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
    void foo(String baseName, String names) {
        int suffix = 1;
        String name = baseName;
        <caret>while (names.contains(name)) {
            name = baseName + suffix;
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
    void foo(String baseName, String names) {
        int suffix = 1;
        String name = baseName;
        <caret>while (names.contains(name)) {
            name = baseName + suffix++;
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
    void m(int x) {
        <caret>if (x == 2) throw new RuntimeException("fail");
    }
}
```
keys: Alt+Enter, choose "Replace statement with 'assert' statement"
```after
class A {
    void m(int x) {
        assert x != 2 : "fail";
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
the intention list does not contain "Replace statement with 'assert' statement"; no problem reported
```

### JAVA-ICB-023 case 1: Extract common part
settings: default
language: java
status: unverified (the exact fix name depends on whether variables are involved)
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
        <caret>label:
        while (true) {
            break label;
        }
    }
}
```
keys: inspect
```after
problem: warning `label` "Labeled statement label:"
```

### JAVA-ICB-027 case 1: Parameter never updated
settings: default
language: java
```before
class A {
    void loopDoesNotLoop(boolean b) {
        while (<caret>b) {
            System.out.println();
        }
    }
}
```
keys: inspect
```after
problem: warning `b` "Variable 'b' is not updated inside loop"
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
    void m(Object o1, Object o2) {
        <caret>if (o1 != o2) {
            System.out.println(1);
        } else {
            System.out.println(2);
        }
    }
}
```
keys: Alt+Enter, choose "Invert 'if' condition"
```after
class A {
    void m(Object o1, Object o2) {
        if (o1 == o2) {
            System.out.println(2);
        } else {
            System.out.println(1);
        }
    }
}
```

### JAVA-ICB-034 case 1: Nested conditional
settings: enable inspection NestedConditionalExpression
language: java
```before
class A {
    int m(int a, int b) {
        int y = a == 10 ? <caret>b == 20 ? 10 : a : b;
        return y;
    }
}
```
keys: inspect
```after
problem: warning `b == 20 ? 10 : a` "Nested conditional expression b == 20 ? 10 : a"
```

### JAVA-ICB-035 case 1: Nested switch expression
settings: enable inspection NestedSwitchStatement
language: java
```before
class A {
    int m(int i) {
        return switch (i) {
            case 0 -> 0;
            default -> <caret>switch (i) {
                case 100 -> 0;
                default -> i;
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
        boolean b = <caret>false || x;
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
    boolean m(Object x, Class<?> myClass) {
        return <caret>x != null && myClass.isInstance(x);
    }
}
```
keys: Alt+Enter, choose "Remove unnecessary 'x != null' condition"
```after
class A {
    boolean m(Object x, Class<?> myClass) {
        return myClass.isInstance(x);
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
    void f(boolean foo, boolean bar) {
        boolean b = <caret>!(foo ^ bar);
    }
}
```
keys: Alt+Enter, choose "Simplify"
```after
class A {
    void f(boolean foo, boolean bar) {
        boolean b = foo == bar;
    }
}
```

### JAVA-ICB-040 case 2: Already simple
settings: default
language: java
```before
class A {
    void f(boolean foo, boolean bar) {
        boolean b = <caret>foo == bar;
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-041 case 1: condition ? true : foo
settings: default
language: java
```before
class A {
    boolean m(boolean condition, boolean foo) {
        return <caret>condition ? true : foo;
    }
}
```
keys: Alt+Enter, choose "Simplify"
```after
class A {
    boolean m(boolean condition, boolean foo) {
        return condition || foo;
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
    void m(int value) {
        <caret>switch (value) {
            case 0 -> System.out.println("zero");
            case 1 -> System.out.println("one");
            default -> System.out.println("many");
        }
    }
}
```
keys: Alt+Enter, choose "Push down 'switch' expression"
```after
class A {
    void m(int value) {
        System.out.println(switch (value) {
            case 0 -> "zero";
            case 1 -> "one";
            default -> "many";
        });
    }
}
```

### JAVA-ICB-042 case 2: Branches call different methods
settings: default
language: java
```before
class A {
    void m(int value) {
        <caret>switch (value) {
            case 0 -> System.out.println("zero");
            default -> System.err.println("many");
        }
    }
}
```
keys: Alt+Enter
```after
the intention list does not contain "Push down 'switch' expression"; no problem reported
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
language: java
status: unverified (exact percentage rounding)
```before
class A {
    void m(int i) {
        switch (i) <caret>{
            case 1:
                System.out.println("1");
                System.out.println("2");
                System.out.println("3");
                System.out.println("4");
                System.out.println("5");
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
problem: information `switch` "switch statement without 'default' branch"
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
            case 2:
                int <caret>x = 0;
                break;
            case 3:
                x = 3;
                System.out.println(x);
                break;
        }
    }
}
```
keys: inspect
```after
problem: warning `x` "Local variable x declared in one 'switch' branch and used in another"
```

### JAVA-ICB-046 case 2: Variable used only in its branch
settings: default
language: java
```before
class A {
    void m(int i) {
        switch (i) {
            case 2:
                int <caret>x = 0;
                System.out.println(x);
                break;
            case 3:
                break;
        }
    }
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
    void foo() {} void bar() {}
    void m(String expression) {
        <caret>switch (expression) {
            case "foo" -> foo();
            case "bar" -> bar();
        }
    }
}
```
keys: Alt+Enter, choose "Replace 'switch' with 'if'"
```after
class A {
    void foo() {} void bar() {}
    void m(String expression) {
        if ("foo".equals(expression)) {
            foo();
        } else if ("bar".equals(expression)) {
            bar();
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
    boolean foo() { return true; }
    boolean m() {
        <caret>if (foo()) {
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
    boolean foo() { return true; }
    boolean m() {
        return foo();
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
    void foo() {}
    void m(boolean x) {
        foo();
        <caret>while (x) {
            foo();
        }
    }
}
```
keys: Alt+Enter, choose "Replace 'while' with 'do while'"
```after
class A {
    void foo() {}
    void m(boolean x) {
        do {
            foo();
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
    boolean <caret>alwaysTrue() {
        return true;
    }
    void f() {
        if (!alwaysTrue()) {
            return;
        }
    }
    boolean member = !alwaysTrue();
}
```
keys: inspect (batch, whole file)
```after
problem: warning `alwaysTrue` "Calls to boolean method 'alwaysTrue()' are always inverted"
```

### JAVA-ICB-051 case 2: One call is not negated
settings: default
language: java
```before
class C {
    boolean <caret>alwaysTrue() {
        return true;
    }
    void f() {
        if (!alwaysTrue()) {
            return;
        }
        if (alwaysTrue()) {
            return;
        }
    }
}
```
keys: inspect (batch, whole file)
```after
no problem reported
```

### JAVA-ICB-052 case 1: Always negated
settings: enable inspection BooleanVariableAlwaysNegated
language: java
```before
class A {
    void m() {
        boolean <caret>b = true;
        System.out.println(!b);
    }
}
```
keys: inspect
```after
problem: warning `b` "Boolean variable b is always inverted"
```

### JAVA-ICB-053 case 1: Replace with constant
settings: enable inspection ConstantValueVariableUse
language: java
```before
class A {
    private static void foo(double number) {
        if (number == 1.0) {
            f(<caret>number);
        }
    }
    private static void f(double number) {}
}
```
keys: Alt+Enter, choose "Replace with '1.0'"
```after
class A {
    private static void foo(double number) {
        if (number == 1.0) {
            f(1.0);
        }
    }
    private static void f(double number) {}
}
```

### JAVA-ICB-054 case 1: Chained call on a project class
settings: enable inspection LawOfDemeter
language: java
```before
class Wallet { int contents; }
class Customer { Wallet getWallet() { return null; } }
class A {
    int pay(Customer c) {
        return c.getWallet().<caret>contents;
    }
}
```
keys: inspect
```after
problem: warning `contents` "Access of contents violates Law of Demeter"
```

### JAVA-ICB-055 case 1: Negative name
settings: enable inspection NegativelyNamedBooleanVariable
language: java
```before
class A {
    void m() {
        boolean <caret>disabled = false;
    }
}
```
keys: inspect
```after
problem: warning `disabled` "Boolean variable disabled is negatively named"
```

### JAVA-ICB-056 case 1: Reuse after use
settings: default
language: java
status: unverified (name chosen for the new variable)
```before
class A {
    void x() {
        String s = "one";
        System.out.println("s = " + s);
        <caret>s = "two";
        System.out.println("s = " + s);
    }
}
```
keys: Alt+Enter, choose "Split local variable"
```after
class A {
    void x() {
        String s = "one";
        System.out.println("s = " + s);
        String s1 = "two";
        System.out.println("s = " + s1);
    }
}
```

### JAVA-ICB-056 case 2: Assignment reading the old value
settings: default
language: java
```before
class A {
    void x() {
        String s = "one";
        System.out.println("s = " + s);
        <caret>s = s + "two";
        System.out.println("s = " + s);
    }
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
class A {
    void m() {
        StringBuilder <caret>sb = new StringBuilder();
        System.out.println();
        sb.append(1);
    }
}
```
keys: Alt+Enter, choose "Move declaration of 'sb' closer to usages"
```after
class A {
    void m() {
        System.out.println();
        StringBuilder sb = new StringBuilder();
        sb.append(1);
    }
}
```

### JAVA-ICB-057 case 2: Variable already next to its use
settings: default
language: java
```before
class A {
    void m() {
        System.out.println();
        StringBuilder <caret>sb = new StringBuilder();
        sb.append(1);
    }
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
    boolean yes() {
        boolean <caret>b = true;
        return b;
    }
}
```
keys: Alt+Enter, choose "Inline variable"
```after
class A {
    boolean yes() {
        return true;
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
    void m(String s) {
        String s1 = s.<caret>valueOf(0);
    }
}
```
keys: Alt+Enter, choose "Qualify static 'valueOf' call with reference to class 'String'"
```after
class A {
    void m(String s) {
        String s1 = String.valueOf(0);
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
public class Person {
    private String <caret>name;
    Person(String name) {
        this.name = name;
    }
    public String getName() {
        return name;
    }
}
```
keys: inspect (batch, whole file)
```after
problem: warning `name` "Declaration can have final modifier"
```

### JAVA-ICB-060 case 2: Field assigned outside the constructor
settings: default
language: java
```before
public class Person {
    private String <caret>name;
    void setName(String name) {
        this.name = name;
    }
}
```
keys: inspect (batch, whole file)
```after
no problem reported
```

### JAVA-ICB-061 case 1: Remove redundant parameter
settings: default
language: java
```before
@interface Test {
    Class<?> expected() default Throwable.class;
}
class A {
    @Test(expected = <caret>Throwable.class)
    void testSmth() {}
}
```
keys: Alt+Enter, choose "Remove redundant parameter"
```after
@interface Test {
    Class<?> expected() default Throwable.class;
}
class A {
    @Test()
    void testSmth() {}
}
```

### JAVA-ICB-061 case 2: Non-default value
settings: default
language: java
```before
@interface Test {
    Class<?> expected() default Throwable.class;
}
class A {
    @Test(expected = <caret>Exception.class)
    void testSmth() {}
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
class A {
    void f() throws Exception, <caret>Exception {}
}
```
keys: Alt+Enter, choose "Remove 'Exception' from 'f' throws list"
```after
class A {
    void f() throws Exception {}
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
record Bar(int a, int b) {
    public <caret>final int sum() {
        return a + b;
    }
}
```
keys: Alt+Enter, choose "Remove 'final' modifier"
```after
record Bar(int a, int b) {
    public int sum() {
        return a + b;
    }
}
```

### JAVA-ICB-064 case 2: Non-final class
settings: default
language: java
```before
class Bar {
    public <caret>final int sum() {
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
keys: inspect (batch, whole project)
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
keys: inspect (batch, whole project)
```after
no problem reported
```

### JAVA-ICB-067 case 1: Provided but not exported
settings: default
language: java
```before
--- module-info.java
module myModule {
    provides <caret>com.example.MyService with com.example.MyServiceImpl;
}
--- com/example/MyService.java
package com.example;
public interface MyService {}
--- com/example/MyServiceImpl.java
package com.example;
public class MyServiceImpl implements MyService {}
```
keys: inspect
```after
problem: warning `com.example.MyService` "Service interface provided but not exported or used"
```

### JAVA-ICB-067 case 2: Service package exported
settings: default
language: java
```before
--- module-info.java
module myModule {
    exports com.example;
    provides <caret>com.example.MyService with com.example.MyServiceImpl;
}
--- com/example/MyService.java
package com.example;
public interface MyService {}
--- com/example/MyServiceImpl.java
package com.example;
public class MyServiceImpl implements MyService {}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-068 case 1: Record member
settings: default
language: java
```before
record Bar(int a, int b) {
    <caret>protected int sum() {
        return a + b;
    }
}
```
keys: Alt+Enter, choose "Weaken visibility"
```after
record Bar(int a, int b) {
    int sum() {
        return a + b;
    }
}
```

### JAVA-ICB-068 case 2: Non-final class
settings: default
language: java
```before
class Bar {
    <caret>protected int sum() {
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
            foo();
            ac.<caret>close();
        }
    }
    void foo() {}
}
```
keys: Alt+Enter, choose "Remove redundant 'close()'"
```after
class A {
    void m() throws Exception {
        try (AutoCloseable ac = null) {
            foo();
        }
    }
    void foo() {}
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
            foo();
        }
    }
    void foo() {}
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
    void m(java.util.Map<String, Integer> map) {
        map.forEach((<caret>String s, Integer i) -> System.out.println(s + "=" + i));
    }
}
```
keys: Alt+Enter, choose "Remove redundant parameter types"
```after
class A {
    void m(java.util.Map<String, Integer> map) {
        map.forEach((s, i) -> System.out.println(s + "=" + i));
    }
}
```

### JAVA-ICB-070 case 2: Implicit types
settings: default
language: java
```before
class A {
    void m(java.util.Map<String, Integer> map) {
        map.forEach((<caret>s, i) -> System.out.println(s + "=" + i));
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-071 case 1: Remove redundant canonical constructor
settings: default
language: java
status: unverified (the exact name substituted into "Remove {0}")
```before
record Point(int x, int y) {
    public <caret>Point(int x, int y) {
        this.x = x;
        this.y = y;
    }
}
```
keys: Alt+Enter, choose "Remove canonical constructor"
```after
record Point(int x, int y) {
}
```

### JAVA-ICB-071 case 2: Convert to compact form
settings: default
language: java
```before
record Range(int from, int to) {
    public Range<caret>(int from, int to) {
        if (from > to) throw new IllegalArgumentException();
        this.from = from;
        this.to = to;
    }
}
```
keys: Alt+Enter, choose "Convert canonical constructor to compact form"
```after
record Range(int from, int to) {
    public Range {
        if (from > to) throw new IllegalArgumentException();
    }
}
```

### JAVA-ICB-072 case 1: Never thrown
settings: default
language: java
```before
class A {
    void method() throws <caret>InterruptedException {
        System.out.println();
    }
}
```
keys: inspect (batch, whole file)
```after
problem: warning `InterruptedException` "The declared exception InterruptedException is never thrown"
```

### JAVA-ICB-072 case 2: Actually thrown
settings: default
language: java
```before
class A {
    void method() throws <caret>InterruptedException {
        Thread.sleep(1);
    }
}
```
keys: inspect (batch, whole file)
```after
no problem reported
```

### JAVA-ICB-073 case 1: Reifiable varargs
settings: default
language: java
status: unverified (exact highlighted element)
```before
class A {
    <caret>@SafeVarargs
    public final void processStrings(String... strings) {
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
    static void printPoint(int <caret>x, int y) {
        System.out.println(x + ", " + y);
    }
    public static void main(String[] args) {
        printPoint(0, 1);
        printPoint(0, 2);
    }
}
```
keys: inspect (batch, whole file)
```after
problem: warning `x` "Value of parameter x is always '0'"
```

### JAVA-ICB-074 case 2: Different values
settings: default
language: java
```before
class A {
    static void printPoint(int <caret>x, int y) {
        System.out.println(x + ", " + y);
    }
    public static void main(String[] args) {
        printPoint(0, 1);
        printPoint(3, 2);
    }
}
```
keys: inspect (batch, whole file)
```after
no problem reported
```

### JAVA-ICB-075 case 1: Always returns 0 (batch)
settings: default
language: java
```before
class Y {
    int <caret>yyy() {
        return 0;
    }
}
```
keys: inspect (batch, whole file)
```after
problem: warning `yyy` "Method yyy() always returns 0"
```

### JAVA-ICB-075 case 2: Different values
settings: default
language: java
```before
class Y {
    int <caret>yyy(boolean b) {
        if (b) return 0;
        return 1;
    }
}
```
keys: inspect (batch, whole file)
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
import java.util.List;
import java.util.function.Predicate;
class A {
    boolean contains(List<String> names, String name) {
        return ((Predicate<String>) x -> {
            return names.contains(x);
        }).<caret>test(name);
    }
}
```
keys: Alt+Enter, choose "Replace method call on lambda with lambda body"
```after
import java.util.List;
import java.util.function.Predicate;
class A {
    boolean contains(List<String> names, String name) {
        return names.contains(name);
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
keys: inspect (batch, whole project)
```after
problem: warning module app "Module 'app' sources do not depend on module 'lib' sources"
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
keys: inspect (batch, whole project)
```after
no problem reported
```

### JAVA-ICB-079 case 1: Unused private method
settings: default
language: java
status: unverified (Safe delete shows a usages dialog only when usages exist)
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
        <caret>label: for (int i = 0; i < 10; i++) {
            if (i == 3) {
                break;
            }
        }
    }
}
```
keys: Alt+Enter, choose "Remove unused label"
```after
class A {
    void m() {
        for (int i = 0; i < 10; i++) {
            if (i == 3) {
                break;
            }
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
        <caret>label: for (int i = 0; i < 10; i++) {
            if (i == 3) {
                break label;
            }
        }
    }
}
```
keys: inspect
```after
no problem reported
```

### JAVA-ICB-081 case 1: Library without usages
settings: default
language: plain
```before
modules: app (library: libfoo.jar)
--- app/A.java
class A {}
```
keys: inspect (batch, whole project, enable inspection UnusedLibrary)
```after
problem: warning library libfoo "Unused library 'libfoo'"
```

### JAVA-ICB-082 case 1: Setter with ignored result
settings: default
language: java
status: unverified (Alt+Enter availability on the fly vs batch only)
```before
class A {
    private String myStr;
    public String <caret>setStr(String str) {
        myStr = str;
        return myStr;
    }
    void test() {
        setStr("value");
    }
}
```
keys: Alt+Enter, choose "Make method 'void'"
```after
class A {
    private String myStr;
    public void setStr(String str) {
        myStr = str;
    }
    void test() {
        setStr("value");
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
keys: inspect (batch, whole file)
```after
no problem reported
```

### JAVA-ICB-083 case 1: Can be private
settings: enable inspection WeakerAccess
language: java
```before
class Sample {
    void foo() {
        bar("foo", "foo");
    }
    void <caret>bar(String x, String y) { }
}
```
keys: inspect (batch, whole file)
```after
problem: warning `bar` "Can be private"
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
keys: inspect (batch, whole project)
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
keys: inspect (batch, whole project)
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
keys: inspect (batch, whole project)
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
keys: inspect (batch, whole project)
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
keys: inspect (batch, whole project)
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
keys: inspect (batch, whole project)
```after
problem: warning package `p` "Package 'p' is cyclically dependent on package 'q'"
```

### JAVA-ICB-090 case 1: Package-private access across modules
settings: enable inspection SuspiciousPackagePrivateAccess
language: java
status: unverified (exact subject text of {0})
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
class Log {
    private String[] messages = {"one", "two", "three"};
    String[] getMessages() {
        return <caret>messages;
    }
}
```
keys: Alt+Enter, choose "Replace with 'messages.clone()'"
```after
class Log {
    private String[] messages = {"one", "two", "three"};
    String[] getMessages() {
        return messages.clone();
    }
}
```

### JAVA-ICB-092 case 1: Package-private field
settings: enable inspection PackageVisibleField
language: java
```before
public class A {
    Object <caret>object;
    final static int MODE = 0;
}
```
keys: inspect
```after
problem: warning `object` "Package-visible field object"
```

### JAVA-ICB-093 case 1: Package-private nested class
settings: enable inspection PackageVisibleInnerClass
language: java
```before
public class Outer {
    static class <caret>Nested {}
}
```
keys: inspect
```after
problem: warning `Nested` "Package-visible nested class Nested"
```

### JAVA-ICB-094 case 1: Protected field
settings: enable inspection ProtectedField
language: java
```before
public class A {
    protected Object <caret>object;
    protected final static int MODE = 0;
}
```
keys: inspect
```after
problem: warning `object` "Protected field object"
```

### JAVA-ICB-095 case 1: Protected nested class
settings: enable inspection ProtectedInnerClass
language: java
```before
public class Outer {
    protected static class <caret>Nested {}
}
```
keys: inspect
```after
problem: warning `Nested` "Protected nested class Nested"
```

### JAVA-ICB-096 case 1: Encapsulate public field
settings: default
language: java
status: unverified (Encapsulate Fields dialog defaults)
```before
class Main {
    public String <caret>name;
}
```
keys: Alt+Enter, choose "Encapsulate field 'name'", Enter in the dialog (defaults: private field, getter and setter, update usages)
```after
class Main {
    private String name;

    public String getName() {
        return name;
    }

    public void setName(String name) {
        this.name = name;
    }
}
```

### JAVA-ICB-096 case 2: Constant
settings: default
language: java
```before
class Main {
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
public class Outer {
    public static class <caret>Nested {}
}
```
keys: inspect
```after
problem: warning `Nested` "'public' nested class Nested"
```

### JAVA-ICB-098 case 1: Access to another object's field
settings: enable inspection UseOfAnotherObjectsPrivateField
language: java
```before
public class Base {
    protected int bar;
    void increment(Base base) {
        bar++;
        base.<caret>bar++;
    }
}
```
keys: inspect
```after
problem: warning `bar` "Direct access to non-public field bar of another object"
```

### JAVA-ICB-099 case 1: Catch NPE
settings: enable inspection BadExceptionCaught
language: java
```before
class A {
    boolean m(java.awt.Component component) {
        try {
            return component.getMousePosition(true) != null;
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
    void describeModule(String module) throws <caret>Exception {}
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
    void setup(Object mode) {
        if (mode == null)
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
            throwingMethod();
        } <caret>catch (IOException ex) {
        }
    }
    void throwingMethod() throws IOException {}
}
```
keys: Alt+Enter, choose "Rename 'ex' to 'ignored'"
```after
import java.io.IOException;
class A {
    void m() {
        try {
            throwingMethod();
        } catch (IOException ignored) {
        }
    }
    void throwingMethod() throws IOException {}
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
            throwingMethod();
        } <caret>catch (IOException ignored) {
        }
    }
    void throwingMethod() throws IOException {}
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
import java.io.*;
class A {
    void m() throws IOException {
        try {
            new FileInputStream("");
        } catch (FileNotFoundException <caret>e) {
            throw e;
        }
    }
}
```
keys: Alt+Enter, choose "Delete 'catch' section"
```after
import java.io.*;
class A {
    void m() throws IOException {
        new FileInputStream("");
    }
}
```

### JAVA-ICB-103 case 2: Catch does additional work
settings: default
language: java
```before
import java.io.*;
class A {
    void m() throws IOException {
        try {
            new FileInputStream("");
        } catch (FileNotFoundException <caret>e) {
            System.out.println("x");
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
class <caret>IllegalMoveException extends Exception {}
```
keys: inspect
```after
problem: warning `IllegalMoveException` "Checked exception class IllegalMoveException"
```

### JAVA-ICB-105 case 1: Continue in finally
settings: default
language: java
```before
class A {
    void m() {
        while (true) {
            try {
                throwing();
            } finally {
                <caret>continue;
            }
        }
    }
    void throwing() {}
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
            Files.readString(Paths.get("in.txt"));
        } catch (java.io.IOException e) {
            throw new RuntimeException(e);
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
            Files.readString(Paths.get("in.txt"));
        } catch (java.io.IOException e) {
            throw new RuntimeException(e);
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
class <caret>EnigmaThrowable extends Throwable {}
```
keys: inspect
```after
problem: warning `EnigmaThrowable` "Class EnigmaThrowable directly extends 'java.lang.Throwable'"
```

### JAVA-ICB-111 case 1: Return in finally
settings: default
language: java
```before
class A {
    void x() {
        try {
            throw new RuntimeException();
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
language: java
status: unverified (assumes the block is judged by whether it can complete normally; the statement itself is reported by ReturnFromFinallyBlock)
```before
class A {
    void x(boolean b) {
        try {
            throw new RuntimeException();
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
    void foo(Runnable runnable) {
        try {
            runnable.run();
        } catch (Throwable throwable) {
            if (<caret>throwable instanceof NoClassDefFoundError) {
                System.out.println("Class not found!");
            }
        }
    }
}
```
keys: inspect
```after
problem: warning `throwable` "'instanceof' on 'catch' parameter throwable"
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
language: java
status: unverified (text substituted for #ref)
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
language: java
status: unverified (the field has no initializer so the result does not compile; the fix itself only adds `final`)
```before
public class EditorException extends Exception {
    private String <caret>message;
}
```
keys: Alt+Enter, choose "Make 'message' 'final'"
```after
public class EditorException extends Exception {
    private final String message;
}
```

### JAVA-ICB-116 case 1: Throw null
settings: enable inspection NullThrown
language: java
status: unverified (whether the generated expression is shortened to the simple name)
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
    boolean bar() { return true; }
    void m() {
        try {
            foo();
        } finally {
            if (bar()) <caret>return;
        }
    }
    void foo() {}
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
            foo();
        } finally {
            Runnable r = () -> { <caret>return; };
        }
    }
    void foo() {}
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
    void m(Optional<String> optional) {
        optional.orElseThrow(() -> {
            <caret>throw new RuntimeException();
        });
    }
}
```
keys: Alt+Enter, choose "Replace 'throw' with 'return' in lambda"
```after
import java.util.Optional;
class A {
    void m(Optional<String> optional) {
        optional.orElseThrow(() -> new RuntimeException());
    }
}
```

### JAVA-ICB-119 case 2: Supplier that returns
settings: default
language: java
```before
import java.util.Optional;
class A {
    void m(Optional<String> optional) {
        optional.orElseThrow(() -> <caret>new RuntimeException());
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
    void m(boolean b) {
        try {
            if (!b) {
                <caret>throw new IllegalStateException("Directory not found.");
            }
        } catch (Exception e) {
            System.out.println("run failed");
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
class Fascinating {
    void doTask(Task t) {
        try {
            t.activity();
        } finally {
            if (!t.finish()) {
                <caret>throw new IllegalStateException();
            }
        }
    }
    interface Task { void activity(); boolean finish(); }
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
class InvalidDataException extends RuntimeException {}
class TextEditor {
    void readSettings() throws <caret>InvalidDataException {}
}
```
keys: Alt+Enter, choose "Remove 'InvalidDataException' from 'throws' clause"
```after
class InvalidDataException extends RuntimeException {}
class TextEditor {
    void readSettings() {}
}
```

### JAVA-ICB-123 case 1: Replace with RuntimeException catch
settings: enable inspection TooBroadCatch
language: java
```before
import java.io.File;
class A {
    String m(String pathToFile, String defaultFilePath) {
        try {
            File file = new File(pathToFile);
            return file.getAbsolutePath();
        } catch (<caret>Exception ex) {
            return defaultFilePath;
        }
    }
}
```
keys: Alt+Enter, choose "Replace with 'catch' clause for 'RuntimeException'"
```after
import java.io.File;
class A {
    String m(String pathToFile, String defaultFilePath) {
        try {
            File file = new File(pathToFile);
            return file.getAbsolutePath();
        } catch (RuntimeException ex) {
            return defaultFilePath;
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
    public void createFile() throws <caret>Exception {
        File file = new File("pathToFile");
        file.createNewFile();
    }
}
```
keys: Alt+Enter, choose "Replace with specific exceptions"
```after
import java.io.File;
import java.io.IOException;
class A {
    public void createFile() throws IOException {
        File file = new File("pathToFile");
        file.createNewFile();
    }
}
```

### JAVA-ICB-125 case 1: Unchecked exception class
settings: enable inspection UncheckedExceptionClass
language: java
```before
class <caret>EnigmaException extends RuntimeException {}
```
keys: inspect
```after
problem: warning `EnigmaException` "Unchecked exception class EnigmaException"
```

### JAVA-ICB-126 case 1: Move cause into constructor
settings: default
language: java
```before
class A {
    void m() {
        try {
            process();
        } catch (RuntimeException ex) {
            RuntimeException wrapper = new RuntimeException("Error while processing");
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
        } catch (RuntimeException ex) {
            RuntimeException wrapper = new RuntimeException("Error while processing", ex);
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
        MyEx wrapper = new MyEx("x");
        wrapper.<caret>initCause(ex);
        throw wrapper;
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
class X {
    public void <caret>finalize() {
        /* ... */
    }
}
```
keys: Alt+Enter, choose "Make 'protected'"
```after
class X {
    protected void finalize() {
        /* ... */
    }
}
```

### JAVA-ICB-128 case 2: Protected finalize
settings: default
language: java
```before
class X {
    protected void <caret>finalize() {
    }
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


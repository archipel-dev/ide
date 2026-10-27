# 07. The most used Java inspections (enabled by default)

Status: specification, clean-room. Written from the IntelliJ IDEA Community sources in our own words. Message
templates are quoted from the message bundles (short user-visible strings). Source paths are cited as
`ic: <path>`. No code is reproduced.

Primary sources: `ic: java/java-backend/resources/META-INF/Inspections.xml`, `GlobalInspections.xml`,
`InspectionGadgets.xml` (registrations with `enabledByDefault`, `level`, group), message bundles
`ic: java/java-analysis-impl/resources/messages/InspectionGadgetsBundle.properties`,
`ic: java/java-analysis-api/resources/messages/JavaAnalysisBundle.properties`,
`ic: java/openapi/resources/messages/JavaBundle.properties`,
`ic: java/java-psi-impl/resources/messages/JavaErrorBundle.properties`,
`ic: java/java-analysis-impl/.../UnusedImportsVisitor.java` and the highlighting visitors of
`ic: java/java-analysis-impl/src/com/intellij/codeInsight/daemon/impl/analysis/`.

The default profile enables about 440 Java inspections; the ones below are the 40 whose findings users meet
daily. The order is by how often a user meets them in ordinary code, not by group.

---

## 7.1 Inspection model

- An **inspection** has a *short name* (the id used in suppressions), a *display name*, a *group*, a *default
  severity* and an *enabled-by-default* flag. Severities: `ERROR`, `WARNING` (yellow wavy), `WEAK WARNING`
  (subtle, greyed underline), `INFO` (no marker in the editor; the fix appears as an Alt+Enter intention),
  `TEXT ATTRIBUTES` (only a text style, e.g. grey for unused). Profile settings can change severity per
  inspection and scope.
- **Descriptions** use a placeholder `#ref` meaning *the text of the highlighted element*; descriptions are
  rendered as HTML in tooltips with that substitution (`<code>` tags show monospace). In the message tables
  below `#ref` is kept literally.
- **Range**: the element named in each entry; "the name" = the name identifier; "the keyword" = the leading
  keyword token; "the call" = the method name for call expressions unless stated.
- **Quick fixes** appear under Alt+Enter on the highlight, each with a family name; most fixes have a
  *"Fix all in file"*/"in project" batch form and a preview (LATER).
- **Suppression**: `@SuppressWarnings("ShortName")` on a declaration, `//noinspection ShortName` on the
  statement, or by profile scope; `"all"` suppresses all.
- **Where each runs**: *on-the-fly* local inspections run in the daemon; *global* ones (unused declaration,
  redundant `throws`, unused return value, `can be final`, ...) need project-wide analysis and run in the
  batch inspection/code-cleanup flow, except for the on-the-fly subset described in 7.2.
- **Quality bar for the first release**: no false positives; the rules below are conservative where stated.

- **JAVA-INS-001** `MUST` Inspection registry with short name, severity, group, enabled flag, suppression by
  annotation and comment, severity override per profile.
- **JAVA-INS-002** `MUST` `#ref` substitution and HTML-free plain text for the problems list.
- **JAVA-INS-003** `SHOULD` Fix-all-in-file for each fix that is safe to batch.
- **JAVA-INS-004** `SHOULD` Per-inspection options (the options noted below with their defaults).

---

## 7.2 Highlighting that is part of the compiler pass (not separately togglable)

These come from the visitor with the errors (`04`) but have warning severity:

| ID | Pri | Name | Message | Severity | Range | Fixes |
|---|---|---|---|---|---|---|
| JAVA-INS-005 | MUST | Unused symbols (on the fly) | `Variable '{0}' is never used` (local, never read nor written); `Variable '{0}' is assigned but never accessed`; `Variable '{0}' is never assigned`; `Private field '{0}' is never used`; `Private field '{0}' is never assigned`; `{0} field '{1}' is assigned but never accessed`; `Parameter '{0}' is never used`; `Pattern variable '{0}' is never used`; `Private method '{0}' is never used`; `Private constructor '{0}' is never used`; `Private inner class '{0}' is never used` (and `interface`); `Type parameter '{0}' is never used`; `Local class '{0}' is never used` | WARNING, **grey text style** ("unused symbol"); not wavy | the name identifier | `Safe delete '{0}'`, `Remove unused variable '{0}'` (with the initializer side-effect choice), `Remove unused field`, `Remove parameter`, `Make '{0}' public/package-private` (for entry-point style) |
| JAVA-INS-006 | MUST | Unused declaration (non-private) | `Class '{0}' is never used`, `Interface '{0}' ...`, `Enum '{0}' ...`, `Record '{0}' ...`, `@Interface '{0}' ...`, `Method '{0}' is never used`, `Constructor '{0}' is never used`, `Field '{0}' is never used`, `Type parameter ...` | WARNING | the name | `Safe delete` |
| JAVA-INS-007 | MUST | Deprecated API usage | `'{0}' is deprecated`; `'{0}' is deprecated since version {1}`; `'{0}' is deprecated and marked for removal` (ERROR-styled when marked for removal at the default settings: warning with `forRemoval` shown stronger); `Overrides deprecated method in '{0}'`; `Default constructor in '{0}' is deprecated` | WARNING, **strikethrough style** | the reference name | replace with the suggested alternative (from `@deprecated` javadoc `{@link}`), `Inline method` when the deprecated method is a simple delegate |
| JAVA-INS-008 | MUST | Unchecked warnings | `Unchecked assignment: '{0}' to '{1}'`; `Unchecked cast: '{0}' to '{1}'`; `Unchecked call to '{0}' as a member of raw type '{1}'`; `Unchecked method '{0}' invocation`; `Unchecked overriding: return type requires unchecked conversion. Found '{0}', required '{1}'` | WARNING | the expression/name | `Add type arguments`, `Suppress with @SuppressWarnings("unchecked")` |
| JAVA-INS-009 | SHOULD | Numeric overflow in constant expression | `Numeric overflow in expression` | WARNING | the expression | none |
| JAVA-INS-010 | SHOULD | Static member accessed via instance | `Static member '{0}.{1}' accessed via instance reference` | WARNING | the member name | `Qualify static member access with class reference` |

Unused-symbol rules (7.2, JAVA-INS-005/006):
- A **local variable/parameter/pattern variable** is *used* if read; *"assigned but never accessed"* if only
  written (increments and compound assignments that only feed themselves count as writes). A parameter of a
  method that overrides/implements another or is part of an API (public method, main, test, framework callback)
  is not reported (hierarchy-aware option "Do not highlight parameters of inherited methods" on by default).
  A variable named `_`/starting with the unnamed-variable convention is exempt.
- A **private** member is *used* if referenced from inside its top-level class (excluding a recursive use inside
  itself); *Private field never assigned* if read but no write exists (and no reflection/framework
  annotation); methods referenced only by method reference or from another private method count as used.
- Members annotated with framework annotations (`@Test`, `@Autowired`, `@Inject`, `@PostConstruct`, `@Bean`,
  serialization hooks, `main`, `@Override`-ed methods, classes implementing `Serializable` hooks, JPA
  `@Entity` fields, Lombok-generated usage) are **entry points** and are not reported. Entry points are an
  extension point (implicit usage providers); the base set is: `public static void main`, JUnit/TestNG tests
  and lifecycle methods, serialization methods (`readObject`, `writeObject`, `readResolve`,
  `serialVersionUID`), `@Deprecated`-unaffected.
- **Non-private** unused symbols (7.2 row 6) need whole-project usage search, so they are *not* shown on the
  fly by default; they are produced by the batch run (`Analyze | Inspect Code`) and by the editor only for
  symbols in a *completely indexed* project when the "Unused declaration" inspection is enabled for the
  editor (default: members that are `public`/`protected`/package-private are reported on the fly only if no
  usage exists in the whole project scope and they are not entry points; library-exported API is excluded).
- **JAVA-INS-005 acceptance:** `void f(){ int x = 1; }` -> `Variable 'x' is never used` on `x` in grey;
  `int y; y = 2;` -> `Variable 'y' is assigned but never accessed`; `private int count;` never read ->
  `Private field 'count' is never used`.

---

## 7.3 The inspections table

Columns: ID, priority, short name, message template, default severity, range, trigger rule, fixes.

### Correctness (probable bugs)

| ID | Pri | Short name | Message | Sev | Range | Trigger | Fixes |
|---|---|---|---|---|---|---|---|
| JAVA-INS-011 | MUST | `DataFlowIssue` (Nullability and data flow problems) | `Method invocation '#ref' may produce 'NullPointerException'`; `Dereference of '#ref' may produce 'NullPointerException'`; `Array access '#ref' may produce 'NullPointerException'`; `Unboxing of '#ref' may produce 'NullPointerException'`; `Casting '{0}' to '#ref' may produce 'ClassCastException'`; `null is returned by the method declared as @NotNull` | WARNING | the dereferenced expression/method name | The data-flow engine proves or suspects a `null` (from `null` literals, `@Nullable` values, checked-against-null branches, `Map.get`, `Optional`-like contracts) reaches a dereference, unboxing or non-null context. Pessimistic on loops/calls: reports only when a specific nullable value is *possible*, never "unknown" | `Surround with 'if (x != null)'`, `Add 'assert x != null'`, `Replace with 'Objects.requireNonNull'`, `Annotate method as @Nullable` |
| JAVA-INS-012 | MUST | `ConstantValue` | `Condition '#ref' is always 'true'` / `'false'`; `Condition '#ref' is always 'true' when reached`; `Value '#ref' is always 'null'`; `Result of '#ref' is always '{0}'`; `Method reference result is always '{0}'` | WARNING | the condition/expression | Data flow proves the value constant (comparing a value already known equal, `x == null` after a null check, `i < 0` for an unsigned-looking range) | `Simplify '#ref' to true/false`, `Remove condition` |
| JAVA-INS-013 | MUST | `UnusedAssignment` | `The value {0} assigned to '#ref' is never used`; `Variable '#ref' initializer '{0}' is redundant`; `The value changed at '#ref' is never used`; `The value of pattern variable '#ref' is never used` | WARNING | the assigned variable/expression | Dead store: an assigned value is overwritten or the variable is not read after the assignment on any path | `Remove redundant assignment`, `Remove initializer` |
| JAVA-INS-014 | MUST | `EqualsBetweenInconvertibleTypes` | `'#ref' between objects of inconvertible types '{0}' and '{1}'` (variant `No class found which is a subtype of both '{0}' and '{1}'`) | WARNING | the `equals` call | `a.equals(b)` / `Objects.equals(a,b)` where the static types are provably unrelated (neither castable to the other: unrelated final classes, `String` vs `Integer`, unrelated parameterizations) | none |
| JAVA-INS-015 | MUST | `ArrayEquals` | `#ref() between arrays should probably be 'Arrays.equals()'` (display name `'equals()' called on array`; the array-comparison variant `Arrays comparison should probably be done using '{0}'`) | WARNING | the `equals` call | `.equals` on an expression of array type (identity comparison only) | `Replace with 'Arrays.equals()'` (also for nested arrays `Arrays.deepEquals`) |
| JAVA-INS-016 | SHOULD | `ArrayHashCode` | `#ref() called on array should probably be 'Arrays.hashCode()'` | WARNING | the call | `array.hashCode()` | `Replace with 'Arrays.hashCode()'` |
| JAVA-INS-017 | MUST | `StringEquality` | `String values are compared using '==', not 'equals()'` | WARNING | the operator `==`/`!=` | `==`/`!=` between two `String`-typed operands, except when one is `null`, a compile-time constant interned comparison, or `""` literal both sides | `Replace with 'equals()'` (`!equals`) |
| JAVA-INS-018 | SHOULD | `NumberEquality` | `Number objects are compared using '==', not 'equals()'` | WARNING | the operator | both operands are boxed `Number` types and neither is `null`/a cached constant context | `Replace with 'equals()'` |
| JAVA-INS-019 | SHOULD | `ObjectEquality` | `Object values are compared using '==', not 'equals()'` | INFO (no marker, intention) | the operator | `==` on two non-enum, non-`Class`, non-final-value reference operands whose class overrides `equals` | `Replace with 'equals()'` |
| JAVA-INS-020 | MUST | `OptionalGetWithoutIsPresent` | `'Optional.get()' without 'isPresent()' check` (the `#ref` form: `'#ref' without 'isPresent()' check`) | WARNING | the `get()`/`getAsInt`... call | `Optional.get`, `.orElseThrow()` is excluded; the call is reachable on a path where presence was not established (data flow of `isPresent`/`isEmpty`/`ifPresent` guards) | `Replace with 'orElseThrow()'`, `Surround with isPresent()` |
| JAVA-INS-021 | SHOULD | `OptionalUsedAsFieldOrParameterType` | `'Optional' used as type for field '{0}'` / `'Optional' used as type for parameter '{0}'` (templated with the `#ref` type) | WARNING | the type element | field or parameter type is `Optional`/`OptionalInt`/`OptionalLong`/`OptionalDouble` | none |
| JAVA-INS-022 | SHOULD | `OptionalIsPresent` | `Non functional style 'Optional.isPresent()' usage` | WARNING | the `isPresent` call | `if (o.isPresent()) { ... o.get() ...}` shaped code that can be written with `map`/`ifPresent`/`orElse` | `Replace with 'ifPresent'`/`map`/`orElse` forms |
| JAVA-INS-023 | MUST | `IgnoreResultOfCall` | `Result of '{0}.#ref()' is ignored` (object allocation: `Result of 'new #ref()' is ignored`) | WARNING | the call name | the return value of a method known to be pure-for-its-value (a configured list: `String` methods, `File.delete`/`mkdir`-style booleans when in the list, `BigDecimal`/`BigInteger` ops, `Optional`, stream terminal results, methods annotated `@CheckReturnValue`/`@CanIgnoreReturnValue` absent) is dropped | none (or `Assign to variable`) |
| JAVA-INS-024 | MUST | `InfiniteRecursion` | `Method '#ref()' recurses infinitely, and can only end by throwing an exception` | WARNING | the method name | all paths through the method body call itself unconditionally | none |
| JAVA-INS-025 | MUST | `InfiniteLoopStatement` | `'#ref' statement cannot complete without throwing an exception` | WARNING | the loop keyword | loop whose condition is constant true and body has no exit (`break`, `return`, `throw` reachable outside the loop, `System.exit`) and contains no wait on interruptions | none |
| JAVA-INS-026 | MUST | `MalformedFormatString` | `Format string '#ref' is malformed`; `Illegal format string specifier: {0}`; arguments mismatch variants (`Too few arguments for format string`/`Too many arguments`/`Argument type does not match specifier`) | WARNING | the format string literal | a `String.format`/`printf`/`formatted`/logger-style call with a format string that does not parse (`%q`), or whose specifier count/type does not match the arguments | none |
| JAVA-INS-027 | MUST | `ThrowableNotThrown` | `Throwable instance '#ref' is not thrown`, `Runtime exception instance 'new #ref()' is not thrown`, `Checked exception instance '#ref' is not thrown`, `Error instance '#ref' is not thrown` | WARNING | the creation/call | a `new XException(..)` (or a method returning a Throwable) used as an expression statement | `Add 'throw'` |
| JAVA-INS-028 | MUST | `SuspiciousMethodCalls` | `Suspicious call to '{0}'` (text includes the argument and the expected key/element type, e.g. `Suspicious call to 'Map.get': 'Integer' is not assignable to 'String'`) | WARNING | the argument | `Collection.contains/remove/indexOf`, `Map.get/containsKey/remove` called with an argument whose type is not convertible to the collection's element/key type | none |
| JAVA-INS-029 | SHOULD | `SillyAssignment` | `Variable '{0}' is assigned to itself` (array element: `Array element is assigned to itself`) | WARNING | the assignment | `x = x;` (also `this.x = this.x`) | `Remove the assignment` |
| JAVA-INS-030 | SHOULD | `UnreachableCatch` | `Unreachable section: exception '{0}' has already been caught` (plural: `exceptions '{0}' have already been caught`); the pre-fix hint `Catch section will become unreachable` | WARNING | the catch type | a `catch` for a checked/unchecked exception type that is a subclass/duplicate of an earlier one in a way javac accepts but that cannot be reached | `Delete catch` |
| JAVA-INS-031 | SHOULD | `CatchMayIgnoreException` | `Empty 'catch' block` (empty case) / `'catch' block may ignore exception` | WARNING | the `catch` keyword | catch block empty or whose parameter is never used and that does not contain a comment or `ignored`-named parameter | `Rename to 'ignored'`, `Generate 'catch' body (printStackTrace / throw)`, `Add comment` |
| JAVA-INS-032 | SHOULD | `EmptyStatementBody` | `#ref statement has empty body` (`#ref` = `if`, `while`, `for`, ...) | WARNING | the statement | `if (c);`, `while (c);`, `for(..);` with a lone semicolon | `Remove`, `Make loop body a block` |

### Redundant constructs (cleanup)

| ID | Pri | Short name | Message | Sev | Range | Trigger | Fixes |
|---|---|---|---|---|---|---|---|
| JAVA-INS-033 | MUST | `RedundantCast` | `Casting '{0}' to '#ref' is redundant` | WARNING | the cast type | the cast does not change overload resolution, inference, or static type needs (JLS 5.5 identity/widening cast whose removal keeps the program meaning) | `Remove redundant cast` |
| JAVA-INS-034 | MUST | `UnnecessaryBoxing` / `UnnecessaryUnboxing` | `Unnecessary boxing` / `Redundant boxing, '{0}.{1}()' call can be used instead` / `Unnecessary unboxing` | WARNING | the call (`Integer.valueOf(x)`, `new Integer(x)`, `.intValue()`) | explicit boxing/unboxing in a context that boxes/unboxes implicitly | `Remove boxing`, `Remove unboxing` |
| JAVA-INS-035 | MUST | `Convert2Diamond` | `Explicit type argument #ref can be replaced with <>` | WARNING | the type arguments | `new ArrayList<String>()` where the arguments can be inferred (diamond applicable, Java 7+; anonymous class: Java 9+) | `Replace with <>` |
| JAVA-INS-036 | SHOULD | `RedundantTypeArguments` | `Explicit type arguments can be inferred` / `Type arguments are redundant for the non-generic method call` | WARNING | the type argument list | explicit method type arguments `Collections.<String>emptyList()` inferable | `Remove explicit type arguments` |
| JAVA-INS-037 | MUST | `SizeReplaceableByIsEmpty` | `'size() == 0' can be replaced with 'isEmpty()'` (`#ref` forms: `'#ref' can be replaced with 'isEmpty()'`) | WARNING | the comparison expression | `c.size() == 0`, `c.size() > 0`, `s.length() == 0`, `!= 0` for types with `isEmpty()` (collections, maps, strings, `CharSequence` implementing it) | `Replace with 'isEmpty()'` (`!isEmpty()`) |
| JAVA-INS-038 | MUST | `ForCanBeForeach` | `#ref loop can be replaced with enhanced 'for'` | WARNING | the loop keyword | indexed `for (int i=0;i<a.length;i++)` / iterator `for (Iterator it = ...)` loops where the index/iterator is used only to read the element (option: also `List` indexed loops, default off) | `Replace with enhanced 'for' loop` |
| JAVA-INS-039 | MUST | `Convert2Lambda` / `Convert2MethodRef` / `CodeBlock2Expr` | `Anonymous #ref can be replaced with lambda`; `Lambda can be replaced with method reference`; `Statement lambda can be replaced with expression lambda`; `Anonymous #ref can be replaced with method reference` | WARNING (lambda/anonymous), INFO (code-block form) | the type / arrow | anonymous class of a functional interface with a single method and no `this`/state use; lambda that only calls a method with its parameters; block lambda with a single `return`/expression statement | `Replace with lambda`, `Replace with method reference`, `Replace with expression lambda` |
| JAVA-INS-040 | MUST | `TrivialIf` / `SimplifiableIfStatement` | `#ref statement can be simplified` (trivial: `if (c) return true; else return false;`); simplifiable: `If statement can be replaced with '{0}'` | WARNING / INFO | the `if` keyword | the if/else returns/assigns boolean constants that can be the condition itself | `Simplify 'if else'` |
| JAVA-INS-041 | MUST | `PointlessBooleanExpression` | `'#ref' can be simplified to '{0}'` | WARNING | the expression | `x && true`, `false || x`, `b == true`, `!(!b)` style constant-boolean operands (options: ignore named constants) | `Simplify` |
| JAVA-INS-042 | SHOULD | `ConstantConditionalExpression` / `DoubleNegation` | `'#ref' can be simplified to '{0}'`; `Double negation in '#ref'` | WARNING | the expression | `true ? a : b`; `!!x` | `Simplify`, `Remove double negation` |
| JAVA-INS-043 | MUST | `UnnecessaryReturn` | `'#ref' is unnecessary as the last statement in a 'void' method` (constructor variant) | WARNING | the statement | trailing `return;` | `Remove unnecessary 'return'` |
| JAVA-INS-044 | SHOULD | `UnnecessarySemicolon` | `Unnecessary semicolon '#ref'` | WARNING | the `;` | stray `;` after a class/method/statement, or after the last enum constant when nothing follows | `Remove unnecessary semicolon` |
| JAVA-INS-045 | SHOULD | `UnnecessaryModifier` / `ExtendsObject` | `Modifier '#ref' is redundant for interfaces` (and `for interface methods`, `inner interfaces`, `final` on a `private` method...); `Class '#ref' explicitly extends 'java.lang.Object'` | WARNING | the modifier / extends clause | implicit modifiers written out; `extends Object` | `Remove redundant modifier`, `Remove redundant 'extends Object'` |
| JAVA-INS-046 | SHOULD | `UnnecessaryToStringCall` | `Unnecessary '#ref()' call` / `Can be replaced with concatenation with empty string` | WARNING | the call | `"a" + x.toString()`, `sb.append(x.toString())`, `String.valueOf(string)` | `Remove unnecessary '#ref()' call` |
| JAVA-INS-047 | SHOULD | `DuplicateThrows` / `RedundantThrows` | `There is a more general exception, '{0}', in the throws list already.` (duplicate: display name `Duplicate throws`); `The declared exception '#ref' is never thrown in this method, nor in its overriding methods` (global) | WARNING | the type in `throws` | duplicate or subsumed entries | `Remove ... from 'throws'` |
| JAVA-INS-048 | SHOULD | `RedundantCompareCall`/`StringOperationCanBeSimplified`/`ObjectsEqualsCanBeSimplified` | per-rule messages (`Can be replaced with ...`) | WARNING/INFO | the call | simple API-pattern rewrites | replace |

### Design and style hints

| ID | Pri | Short name | Message | Sev | Range | Trigger | Fixes |
|---|---|---|---|---|---|---|---|
| JAVA-INS-049 | MUST | `MissingOverrideAnnotation` | `Missing '@Override' annotation on '#ref()'` | INFO (appears as intention/weak) | the method name | a method overrides/implements a superclass or interface method and lacks `@Override` (interface implementations included when the source level >= 6) | `Add '@Override' annotation` |
| JAVA-INS-050 | MUST | `FieldMayBeFinal` | `Field '#ref' may be 'final'` | WARNING | the field name | a non-final private (or any, in batch) field assigned only in initializers/constructors (once on every path), never reassigned | `Make '#ref' final` |
| JAVA-INS-051 | MUST | `InnerClassMayBeStatic` | `Inner class '#ref' may be 'static'` | WARNING | the class name | non-static member class that never uses the outer instance (no outer member access, `this` of outer, instance creation of outer-dependent inner classes) | `Make class static` |
| JAVA-INS-052 | MUST | `FieldCanBeLocal` / `ParameterCanBeLocal` | `Field can be converted to a local variable`; `Parameter '#ref' can be converted to a local variable` | WARNING | the name | a private field written before every read in each method that uses it (so each use could own a local); a parameter assigned before first read | `Convert field to local variable` |
| JAVA-INS-053 | SHOULD | `StringConcatenationInLoops` | `String concatenation '#ref' in loop` | WARNING | the `+`/`+=` | `s += x` or `s = s + x` where `s` is a `String` declared outside the loop | `Convert to StringBuilder` |
| JAVA-INS-054 | SHOULD | `ThrowablePrintStackTrace` | `Call to #ref() should probably be replaced with more robust logging` (display name `Call to 'printStackTrace()'`) | WARNING | the call | `e.printStackTrace()` with no arguments | `Replace with logger call` (LATER), `Rethrow` |
| JAVA-INS-055 | SHOULD | `RawUseOfParameterizedType` | `Raw use of parameterized class '#ref'` | WARNING | the type reference | a generic class used without arguments in a declaration, cast, `new` (options: ignore `instanceof`, `new`, `class` literals, `Class` objects) | `Infer generic type arguments` |
| JAVA-INS-056 | SHOULD | `AutoCloseableResource` | `AutoCloseable used without 'try'-with-resources` (display name; the resource-opened-not-closed variant reads `'{0}' should be opened in front of a 'try' block and closed in the corresponding 'finally' block`) | WARNING | the creation expression | a resource (`InputStream`, `Connection`, ... implementing `AutoCloseable`) created and not closed with try-with-resources or returned/stored | `Wrap with try-with-resources` |
| JAVA-INS-057 | SHOULD | `WrongPackageStatement` | `Package name '{0}' does not correspond to the file path '{1}'` (and `Missing package statement: '{0}'`) | ERROR | the package statement | `package` text differs from the directory path relative to the source root (accounting for source root package prefix) | `Set package name to '{expected}'`, `Move to package '{declared}'` |

- **JAVA-INS-058** `MUST` Every row above has the exact message template, range, trigger as stated, and
  severity; messages use `#ref` substitution of the highlighted text.
- **JAVA-INS-059** `MUST` False-positive discipline: an inspection only reports when its trigger is *certain*
  (resolved types, no unresolved references involved in the decision, no syntax errors in the element).
- **JAVA-INS-060** `SHOULD` Options named in the tables (defaults: ignore-named-constants off, report indexed
  `List` loops off, `Ignore when 'catch' block is not empty` off, parameter-hierarchy exclusion on).

---

## 7.4 Details for the highest-traffic rules

### 7.4.1 Redundant cast (JAVA-INS-033)

A cast `(T) e` is **redundant** when removing it leaves a program that has the same static type of the
expression *where it matters* and the same overload resolution for any enclosing call:
1. the operand type is already a subtype of `T` (upcast, identity), and
2. the cast is not required to select an overload (for `foo((Object) s)` with `foo(Object)`/`foo(String)`
   the cast is *needed* because removing it changes the chosen overload), not required for intersection
   types in lambdas, not required to resolve `?:` branch typing (`(Object) a : b` that avoids unboxing),
   not required for the left side of `+` string concatenation to avoid numeric addition, and not required
   by a generic method call inference where the cast guides inference.
Variants: casts of lambdas/method references to a functional interface that already is the target type are
redundant; casts to the same type in `equals(Object)`-style patterns are reported.

### 7.4.2 Unboxing/boxing and `isEmpty`

`UnnecessaryBoxing` triggers on `Integer.valueOf(i)`, `new Integer(i)` where the context is a
primitive-or-boxed target of the same kind; on `Integer.valueOf(s)` call with a `String` argument it does
*not* trigger (that is parsing). `SizeReplaceableByIsEmpty` triggers on `x.size() == 0`, `0 == x.size()`,
`x.size() < 1`, `x.size() != 0`, `x.size() > 0`, `x.length() == 0` for `String`/`StringBuilder` and
`Collection`/`Map` and any class with a public no-arg `isEmpty()` boolean method; it does not trigger inside
the `isEmpty()` method itself or when the class has no `isEmpty`.

### 7.4.3 Unused import

See `05-imports.md` 5.7 (message `Unused import statement`, WARNING, fix `Remove unused imports`; companion
`Missorted imports`, INFO).

### 7.4.4 Nullability annotations (support for JAVA-INS-011 and parameter checks)

Recognized through *annotation-package support* modules, one per family, each answering "which annotations
mean nullable / not-null / not-null-by-default in this scope": JetBrains annotations (`org.jetbrains.annotations`
`@Nullable`, `@NotNull`), JSpecify (`org.jspecify.annotations` `@Nullable`, `@NonNull`, `@NullMarked`,
`@NullUnmarked`), Checker Framework (`org.checkerframework.checker.nullness.qual.Nullable`/`NonNull`/
`MonotonicNonNull` and the compat/declaration variants, `@DefaultQualifier`), Eclipse (`org.eclipse.jdt.annotation`
`@Nullable`, `@NonNull`, `@NonNullByDefault`), SpotBugs/FindBugs (`edu.umd.cs.findbugs.annotations`
`@Nullable`, `@NonNull`, default-for-parameters annotations), Jakarta (`jakarta.annotation.Nullable`/`Nonnull`,
`jakarta.validation.constraints.NotNull`), JSR-305 (`javax.annotation.Nullable`/`Nonnull`/`CheckForNull` and
type-qualifier defaults/nicknames). Android and Spring families are provided by plugins, not Community. The
default pair used when the IDE generates annotations is the JetBrains pair. Contract annotations (`@Contract`
of JetBrains and Spring) and a fixed list of hard-coded contracts for popular libraries (JDK `Objects`,
`Optional`, Spring `Assert`, Apache `Validate`, JUnit/TestNG/Hamcrest assertions) drive the data flow.
(`ic: java/java-impl/src/com/intellij/codeInsight/annoPackages/`, `ic: java/java-analysis-impl/.../HardcodedContracts.java`.)
Extending to full data flow is an engine of its own: see Q-INS-1.

- **JAVA-INS-061** `SHOULD` Nullability annotation recognition and the contract list.
- **JAVA-INS-062** `LATER` Full data-flow engine (nullability, constants, optional presence, ranges).

---

## 7.5 Acceptance scenarios

**I1.**
```java
import java.util.List;                  // Unused import statement
class A {
    private int count;                  // Private field 'count' is never used
    void f(List<String> l, Object o) {
        String s = (String) "x";        // Casting '"x"' to 'String' is redundant  (message: Casting '"x"' to 'String' ...)
        if (l.size() == 0) { }          // 'size() == 0' can be replaced with 'isEmpty()'
        for (int i = 0; i < l.size(); i++) { System.out.println(l.get(i)); }
        // ^ 'for' loop can be replaced with enhanced 'for' only when the "indexed List loops" option is on (default off)
        Integer boxed = Integer.valueOf(3);   // Unnecessary boxing
        if (s == "x") { }                      // String values are compared using '==', not 'equals()'
        int[] a = {1}, b = {1};
        a.equals(b);                           // 'equals()' called on array, should probably be 'Arrays.equals()'
    }
    @Override public String toString() { return null; }
}
```
Expected: markers as annotated; `toString` has `@Override`, so no missing-override hint. The class has no
other warnings. The `(String) "x"` redundancy message text uses the operand text between the quotes of the
template: `Casting '"x"' to 'String' is redundant`.

**I2.** A class implementing `Runnable` with `public void run(){}` lacking `@Override`: INFO-level
`Missing '@Override' annotation on 'run()'`; fix `Add '@Override' annotation`.

**I3.** `Optional<String> o = ...; o.get();` without guard: WARNING `'Optional.get()' without 'isPresent()'
check` on `get`.

**I4.** `new IllegalStateException("x");` as a statement: `Runtime exception instance 'new IllegalStateException()'
is not thrown`; fix `Add 'throw'`.

---

## 7.6 Open questions

- **Q-INS-1** The data-flow engine (nullability, constants, `Optional` presence) is a large separate spec;
  recommend a follow-up file `08-dataflow.md` after Phase 4 conformance. JAVA-INS-011/012/013/020 are `MUST`
  in priority but depend on it; until then they may be provided by the language server fallback.
- **Q-INS-2** Exact wording of `ObjectEquality`, `ThrowablePrintStackTrace` and `RawUseOfParameterizedType` templates for the
  tooltip beyond the quoted forms: verify against a running IDE when conformance tests are written.
- **Q-INS-3** Whether to ship `INFO`-level rows (no editor marker) as intentions only: yes, recommended.

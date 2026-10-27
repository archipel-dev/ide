# 06. Java completion

Status: specification, clean-room. Written from the IntelliJ IDEA Community sources in our own words. Source
paths are cited as `ic: <path>`. No code is reproduced.

Primary sources: `ic: java/java-impl/src/com/intellij/codeInsight/completion/` (contributors, sorting, insert handlers),
`ic: java/java-backend/resources/META-INF/JavaPlugin.xml` (registration order), `ic: platform/analysis-impl` and
`ic: platform/lang-impl` completion service and weighers, `ic: java/java-impl/.../psi/util/proximity/` (the proximity chain),
`ic: java/java-impl/.../daemon/impl/...` and `05-imports.md` (shared import machinery).

Scope: what Java adds on top of the platform completion framework: which contributors run, what they offer by
syntactic position, how classes from libraries are offered and imported on insertion, static members, how items
are ranked, and chain completion (LATER).

---

## 6.1 Completion kinds and invocation counts

| Kind | Default shortcut | Offers |
|---|---|---|
| **Basic** | Ctrl+Space | everything visible at the caret that matches the typed prefix; first press: items that are in scope, plus classes that are *not yet imported* when the prefix may start a class name |
| **Basic, second press** | Ctrl+Space again | adds items hidden by first-press filters: members of non-imported classes beyond the scope filter, **static methods and fields of any class on the classpath** that match (unqualified identifier position, non-empty prefix), chained-call suggestions when nothing type-matching was found |
| **Smart** | Ctrl+Shift+Space | only items whose type fits the *expected type* at the caret (assignment target, argument position, return type, condition, `throw`, `new`), with expression chains one level deep on second press |
| **Auto-popup** | typing identifier characters | basic completion, with a delay and restrictions (not in comments, strings, after number literals) |

*Invocation count* is the number of consecutive presses on the same prefix; the same engine runs and the
count switches filters. Extended completion = count >= 2.

- **JAVA-CMP-001** `MUST` Basic completion with count 1 and count >= 2 as above. *Accept:* in `Str|` the
  first press shows `String`, `StringBuilder`, `StringBuffer`, ... from `java.lang` (already visible)
  and non-imported classes like `java.util.StringJoiner` shown with a package hint; the second press also
  offers static members.
- **JAVA-CMP-002** `SHOULD` Smart completion by expected type.
- **JAVA-CMP-003** `MUST` No completion in positions where nothing can be completed: after a number
  literal (`1.|`, `5|`), after a dot where the left side cannot have members, after an enum constant name
  in a constants list.
- **JAVA-CMP-004** `SHOULD` Auto-popup rules: not inside comments (except documentation references), not
  inside string literals unless the string is a class-name or file-name context.

---

## 6.2 Contributors and their order

Each contributor adds items and may stop the chain. The relevant Java registrations, with the ordering
constraints declared in the plugin descriptor (an item produced by an earlier contributor is shown the
same, but a contributor that "stops here" prevents later ones from running):

| # | Contributor | Runs for | Purpose |
|---|---|---|---|
| 1 | Method-chain contributor | Java, **first** | deep call chains (LATER, needs the compiler's reference index) |
| 2 | Refactoring contributor | Java, first | name suggestions during refactoring dialogs |
| 3 | Live-template contributor (platform) | any | template keys (`sout`, `psvm`, `fori`, ...) |
| 4 | **No-variants delegator** ("javaBasic2ClassName") | Java, before member-name and the main one, after live templates | runs the remaining contributors, then: if nothing (or only packages) matched, or on first press for class-name-like prefixes, adds **non-imported classes**; also collection utilities, tag calls, chain calls (below) |
| 5 | Method merger | any | merges overloads into one entry with a parameter popup |
| 6 | Qualifier-as-argument | Java | `foo.|` suggestions that pass the qualifier as an argument |
| 7 | Magic constants | Java | `@MagicConstant` values |
| 8 | Member-name contributor | Java, before override generation | names for variables, fields, parameters, methods, constants, type parameters, derived from types and context |
| 9 | **Main Java contributor** ("javaLegacy") | Java, last of the Java-specific ones | reference variants (`6.3`), keywords, modifiers, annotation attributes, `permits` lists, generated overrides, functional expressions, patterns; ends the chain for reference positions |
| 10 | Class-name contributor ("javaClassName") | any JVM language, last, before default | classes by name with import-on-insert (`6.4`), used for annotations and when the main one deferred it |
| 11 | Javadoc contributor | Java (doc comments) | `@param`, `@throws`, `{@link}` names and tags |
| 12 | Class-reference contributor | any | class names inside string literals of known "class-name" APIs (`Class.forName`, annotation `value = "..."` of class-name type) |
| 13 | Word completion (platform) | last | words from the file/project in comments and strings when no reference exists |
| 14 | Logger, JShell | Java | logger field generation, REPL |

The detail that matters for behavior (not the numbering): *the main contributor answers reference positions
and stops the chain; the no-variants delegator wraps it and adds non-imported classes after it*; so a
class name that is in scope is offered by the main contributor, and one that is not is offered as a
"will import on insertion" item.

- **JAVA-CMP-005** `MUST` Contributors 4, 5, 8, 9, 10, 11 (delegator, merger, member names, main, class
  names, javadoc) with the stop semantics above.
- **JAVA-CMP-006** `SHOULD` 3, 7, 12, 13 (templates, magic constants, string class references, words).
- **JAVA-CMP-007** `LATER` 1, 2, 6, 14.

---

## 6.3 What the main contributor offers, by position

Positions are classified from the syntax around the caret. For each, the *set* of items and the filters.

1. **Unqualified identifier in an expression**: local variables and parameters (in scope per
   `02-resolve.md` 2.3), fields (own, inherited, outer classes), methods (same), classes visible by the scope
   rules (explicitly named, same package, on-demand, `java.lang`), enum constants when the expected type is an
   enum (even unqualified), statically imported members, keywords valid here (`this`, `super`, `new`, `null`,
   `true`, `false`, `switch`, `instanceof` after an expression, lambda/ method reference forms), primitive
   types in cast positions, packages (as qualifier starts).
2. **After a dot `q.|`**: members of the *type of the qualifier* (after capture, 03 3.4): fields, methods
   (static ones only if `q` is a type; both if an expression; instance methods of an enclosing class are not
   offered via a type name), member types, `class`/`this`/`super` forms where valid; for a package: its classes
   and subpackages. Inherited members included. Members of `Object` are offered on interfaces. Inaccessible
   members are hidden on first press and offered (marked) on second press.
3. **Type position** (declaration types, `new`, casts, `instanceof`, generics arguments, `extends`/`implements`,
   `throws`, catch types, resource types, annotation names): only classes/interfaces/type parameters/packages
   suitable for the position (interfaces only in `implements`; classes only in `extends` of a class;
   `Throwable` subtypes in `throws`/`catch`; annotation types in annotations; no enums after `extends`;
   no final classes in `extends`), plus primitives where valid. After `new`: instantiable classes and, for the
   expected type, its inheritors with constructors (accessible constructors only; abstract classes only
   for anonymous bodies).
4. **Method call arguments**: expressions (as 1), plus *lambdas and method references* when the parameter
   type is a functional interface (one item showing the lambda signature and one per matching method reference).
5. **Declarations**: after type: variable/field/parameter name suggestions (member-name contributor);
   after modifiers: keywords valid for the context (modifier chooser: only modifiers legal for the element
   and not conflicting with already written ones); class-level: `class`, `interface`, `enum`, `record`,
   `@interface`, `sealed`, `permits`; overrides/generated members (type the name of a super method to
   generate an override with `@Override`; getter/setter generation).
6. **Annotations**: annotation types applicable to the target; attribute names (`@Foo(na|)`); attribute values
   of enum/constant/class-literal types; `value =` omission.
7. **Imports**: after `import`/`import static`: packages and classes; for static: members.
8. **Patterns and switch**: case labels: enum constants (unqualified), constant expressions of the selector
   type, type patterns with names, `default`, `null`, record deconstruction skeletons.
9. **Javadoc**: tags, parameter names, referenced names.
10. **Strings/comments**: no reference: word completion only.

Filters common to all: explicitly excluded classes/packages (05 5.8), inaccessible members on first press,
members whose use would be a compile error in this context are *not hidden* but ranked low (6.5), synthetic
members hidden, deprecated members shown with strikethrough and ranked after non-deprecated of same relevance.

- **JAVA-CMP-008** `MUST` Positions 1-3 with the sets and filters above. *Accept:* in `list.|` with
  `List<String> list`, items include `add`, `get`, `size`, `stream`, `forEach`, ... with parameter hints, no
  `Collections.*` statics; in `Collections.|` only static members and member types.
- **JAVA-CMP-009** `MUST` Type-position filtering (interface-only in `implements`, Throwable-only in
  `throws`/`catch`, annotation-only after `@`).
- **JAVA-CMP-010** `MUST` Argument-position lambdas and method references for functional-interface
  parameters.
- **JAVA-CMP-011** `SHOULD` Declaration positions: names, modifiers, override generation.
- **JAVA-CMP-012** `SHOULD` Annotation attributes and values.
- **JAVA-CMP-013** `SHOULD` Import positions.
- **JAVA-CMP-014** `SHOULD` Switch/pattern labels.
- **JAVA-CMP-015** `SHOULD` Javadoc.
- **JAVA-CMP-016** `MUST` Overloads of a method are one item (`6.8`).
- **JAVA-CMP-017** `MUST` Deprecated items struck through and ranked after non-deprecated equals.

---

## 6.4 Library classes in completion and auto-import on insertion

### 6.4.1 Which non-imported classes appear

The class-name search uses the **short-class-name index** (01 1.3) with the typed prefix, in the **resolve
scope of the file** (02 2.7; so test libraries appear only in test files):

1. Match: the prefix matcher is camel-hump and substring tolerant: `NPE` matches `NullPointerException`,
   `HM` matches `HashMap`, `sb` matches `StringBuilder`; typo tolerance only when nothing else matches
   (LATER).
2. Position filter: the class must be acceptable at the caret (position rules of 6.3.3).
3. Accessibility filter (first and second press): accessible classes only. In the third press (count >= 3)
   inaccessible and library-internal classes are included too (marked).
4. Exclusions: classes/packages excluded in settings (05 5.8) are not offered; classes in "discouraged"
   packages (`com.sun.*`, `sun.*`, `org.omg.*`, `java.awt.List`) are offered but ranked after the others.
5. **Already-visible classes** (resolvable by simple name) are produced by the main contributor and are not
   duplicated by the delegator.
6. Nested classes: offered as `Outer.Inner` (lookup string also accepts `Inner`); only top-level
   classes of other packages are offered directly when the prefix could start the *outer* name too.
7. Interplay with the count: with count 1 and a *single* match that is not imported, the item is shown but is
   not auto-inserted (completion does not auto-pick a non-imported class when it is the only candidate; the
   user must confirm) - this avoids silently importing a wrong class.
8. Presentation: class icon; the simple name; grey tail with the package in parentheses `(java.util)`;
   generic classes show type parameters `<E>`; deprecated strikeout; tail text `(package)`; for classes
   in the same file/package no package hint.

### 6.4.2 Insertion: import on completion

Choosing a class item:
1. Replace the typed prefix by the class's **simple name** (for `Outer.Inner` of an inner class: depending on
   "Insert imports for inner classes": the inner class is imported and its simple name inserted when on; else
   the `Outer.Inner` qualified form is inserted and `Outer` imported).
2. In the same write step, **add an import** for the class through the import machinery (`05` 5.6.3
   `addImport`): single-type import in sorted position, or on-demand according to the thresholds (5 classes;
   `java.awt`/`javax.swing` always). If the class is `java.lang`, in the same package, already imported, or
   already visible by simple name: no import line.
3. If adding the import is impossible (a *different* class with the same simple name is already visible,
   conflicting single import), insert the **fully qualified name** instead (`java.sql.Date d`).
4. If the caret is inside an `import` statement: insert the FQN text, no import edit (and for `import static`
   a trailing `.` is inserted).
5. Javadoc: in a doc comment, insert the FQN when the class is not imported and the code-style option for
   names in Javadoc says "fully qualify if not imported" (the default); otherwise the simple name and
   an import.
6. Tails by completion character: `(` for constructor items after `new` (with parameter info), `<` fills
   type arguments (`<>` or inferred), `.` re-opens completion, `;` at statement end; `!`/`?` after a type in
   a declaration add a nullability annotation.
7. After insertion, the usage statistic of the class is incremented (affects ranking, 6.5 stats and 5.3 key 3).
8. The edit is one undo step; the caret ends after the inserted name (and tail).

### 6.4.3 Static members of non-imported classes

On the **second press** in an unqualified identifier position with a non-empty prefix, the engine offers
**static methods and fields of any class** whose name matches, found through the *static member names index*
(01 1.3), restricted to the resolve scope, to public (or accessible) static members, and filtered by the
exclusion list. Each class contributes **once per method name** (overloads are merged). Items show
`Class.member` plus a package tail. On insertion:
- if the member's class is already statically imported on demand, or the member (or its class) is listed in
  *Include auto-import of static members in completion* (project and IDE lists; entries are class names,
  member names, or `-excluded` names; see 5.8), or the caret is inside that class: **a static import is
  added** (single, or on demand by the rules of 5.4.3) and only `member(` is inserted;
- otherwise the **qualified** text `Class.member(` is inserted and the class is imported normally
  (`6.4.2`), and an advertisement line says `To import a method statically, press Alt+Enter`;
- every such item has a lookup action `Import statically` (a toggle) that flips between the two insertion
  modes before confirming.
Members whose owner is in the *default package* are not offered to files outside the default package.

### 6.4.4 Requirements

- **JAVA-CMP-018** `MUST` Non-imported class items from the short-name index in the resolve scope, with the
  filters and presentation of 6.4.1.
- **JAVA-CMP-019** `MUST` Import-on-insert per 6.4.2 steps 1-4 and 8, using the import machinery of `05`.
  *Accept:* typing `ArrLi` + Ctrl+Space selecting `ArrayList` in a file with no imports inserts `ArrayList`
  and adds `import java.util.ArrayList;`.
- **JAVA-CMP-020** `MUST` Name-clash fallback to FQN. *Accept:* with `import java.util.Date;` present,
  selecting `java.sql.Date` inserts `java.sql.Date` (no import edit).
- **JAVA-CMP-021** `MUST` A lone non-imported candidate on the first press is not auto-inserted.
- **JAVA-CMP-022** `SHOULD` Static-member completion on the second press with the two insertion modes and the
  `Import statically` toggle. *Accept scenario C2.*
- **JAVA-CMP-023** `SHOULD` Javadoc and import-statement insertion forms (6.4.2 steps 4-5).
- **JAVA-CMP-024** `SHOULD` Inner-class import rule.
- **JAVA-CMP-025** `MUST` Excluded and discouraged classes handling.
- **JAVA-CMP-026** `LATER` Typo-tolerant matching; third-press inaccessible classes.

---

## 6.5 Ranking (relevance)

Items are ordered by a chain of weighers (earlier weigher = more significant; within one weigher a *smaller*
rank value or higher listed here is better as stated). The chain for Java, in order:

1. **Priority**: items explicitly prioritized by contributors (for example an exact lookup item the caller
   prioritized, templates with high priority).
2. *(Smart completion only)* default-type match before others.
3. **Prefix matching quality**: exact prefix (case-sensitive) beats case-insensitive prefix beats
   camel-hump/substring matches; shorter lookup strings are lifted (`liftShorter`: when one candidate is a
   prefix of another it is moved before it); "middle matching" (match not at the start) last.
4. **Statistics**: items the user chose before in the same context (per element and per
   *context* such as `expected type`/`qualifier type`), recency weighted.
5. **Kind** (the Java weigher), best first: annotation method in annotation attribute position; probable
   keyword; casting variants; variable of the expected type; lambda; likely method reference; method
   reference; **variable** (locals, parameters, instance fields, `this`); getter; chain qualified by field;
   chain qualified by getter; super-method parameter list; expected-type constant; expected-type argument;
   getter-qualified-by-method; accessible field getter; **normal** (everything else: methods, static members
   already visible); basic chain; `Collections` factory methods; expected-type method; very suitable class
   (e.g. a thrown exception class in `catch`); suitable class; non-initialized field (when completing in a
   constructor, fields not yet assigned); **class names and not-yet-imported static members**; introduced
   variable; unlikely item (comparison of a variable with itself, `String` after `new` in smart); improbable
   keyword; the rest. In *catch/throws* positions the "suitable class" tier is `Throwable` subtypes, with
   the exceptions the guarded block can throw ranked as "very suitable"; in `try (...)` positions
   `AutoCloseable` subtypes; in annotations: annotation types applicable to the target; in a type argument
   position: subtypes of the parameter's bound.
6. **Expected-type match**: items whose type is assignable to the expected type before those that are not
   (basic completion); for `instanceof` operands: convertible types first.
7. **Statics preference** (basic): after `Class.` prefer static members; elsewhere prefer instance
   members over statics by this order: keywords/primitives, then instance members, then statics, then classes.
8. **Recursion**: a method call to the enclosing method itself is ranked after others when the call would
   obviously be wrong/unexpected (same argument, wrong return).
9. **Similarly-ending names**: items whose name shares an ending with the expected type's name
   (`Person p = new |` suggests `Person`).
10. **Non-generic** over generic when an expected type exists; **simple** over complex.
11. **Explicit proximity** (extension point) then the **proximity chain** (`05` 5.3.1): same member, accessibility,
    explicitly imported, in editor, same directory, inheritance, same source root, same module, known JDK
    popularity, in resolve scope, directory type.
12. After the proximity chain: **most used** (global per-class usage statistics), **same words** (name
    contains words of the expected type or qualifier), **shorter**, **non-technical overload**.
13. **Grouping** (last): items of one class kept together where relevant.

Fine rules inside the kind weigher worth stating because users notice them:
- getters/setters, `equals`, `hashCode`, `toString`, `getClass`, `wait`, `notify`, `finalize` (members of
  `Object` and a few `String`/`Class` oddities) are ranked after domain members because the known-element
  weigher gives them negative popularity.
- `println` with arguments is slightly preferred over `println()`.
- In the second-press static list, members of popular classes (`Math`, `Collections`, `Objects`, `Arrays`,
  `Collectors`, `Stream`, `Optional`, `Assertions` in tests via the `junit/test` bonus) rank higher because of
  the JDK popularity and test-framework weights.
- Local variables of exactly the expected type are first among variables; a variable compared with itself is
  last.

- **JAVA-CMP-027** `MUST` Prefix quality, statistics (usage recency), kind order (5), expected-type match,
  statics preference, proximity chain (needed ones: accessibility, explicit import, same module, resolve
  scope, JDK popularity), shorter-first.
- **JAVA-CMP-028** `MUST` The order of tiers in the kind weigher as listed. *Accept:* in
  `String s = |` with locals `String name; int n; ` and field `String title`: items order is `name`,
  `title` (expected type variables), then `n`, ... then methods, then classes.
- **JAVA-CMP-029** `SHOULD` Catch/throws/resource/annotation/type-bound class suitability tiers.
- **JAVA-CMP-030** `SHOULD` Similarly-ending names, recursion, simple/non-generic rules.
- **JAVA-CMP-031** `LATER` ML-based re-ranking.

---

## 6.6 Expected type and smart completion

The *expected type* set at the caret is derived from the context: right side of assignment/initializer (the
variable's type), return statement (the method/lambda return type), argument position (the parameter
types of all candidate overloads, narrowed by the arguments already written), array index (`int`), condition
(`boolean`), `throw` (`Throwable`), `synchronized`, switch selector/case, cast operand, `instanceof` type,
`new` (the target type), operator operands. Each entry has a *kind* (exactly this type, this type or a
subtype, this type or a supertype) and a *default type* (the form to instantiate: for `List<String>` the
default is `ArrayList<String>` when one is chosen by the user's statistics).

Smart completion offers items from the same sources as basic but keeps only those whose type is assignable
(including after one conversion such as boxing/unboxing, `toArray`, stream conversions, casts for
`instanceof`-checked variables) and adds: constructors of inheritors of the expected type, static
methods/fields of the expected type's class returning it (`Duration.ZERO`), factory calls from `Collections`,
enum constants, `null`, lambda/method-reference items, array creation, class literals, and (second press)
one-level chains `a.getB()`.

- **JAVA-CMP-032** `SHOULD` Expected-type derivation for the contexts listed.
- **JAVA-CMP-033** `SHOULD` Smart completion type filter and its extra item kinds.
- **JAVA-CMP-034** `LATER` Chain completion (method chains from the compiler index, `a.b().c()` toward the
  expected type), stream conversions, `toArray` conversions.

---

## 6.7 Chain completion (LATER)

IntelliJ's "method chains" contributor proposes calls like `project.getBaseDir().getPath()` to reach the
expected type, using a *reference index produced by the compiler* (which methods return which types and
which methods are called together), searching up to four calls deep from locals/fields/classes, ranked by how
often the chain's steps are used in the project. Without that index, the one-level chain items of 6.6 are
the fallback. Deferred.

---

## 6.8 Item model and insertion of calls

- A **method item** is one entry per *name per class* when overloads exist (the merger collapses overloads):
  text `name(...)` with the parameter list of the first/most relevant overload as tail text; return type right
  aligned; choosing it inserts `name(`, `)` auto-closed, parameter info popup shown; if there is exactly one
  overload and the call has no arguments needed, `name()` with the caret after. Parameters with obvious
  matching local variables are pre-filled on Tab (smart insert).
- **Typed insertion**: methods with type parameters that cannot be inferred from arguments get explicit
  type arguments only at language levels below Java 8; at 8+ nothing is inserted.
- **Completion chars**: Enter and Tab replace the identifier; Tab replaces the whole current name (including
  the text after the caret); `.` accepts and continues; `(` accepts and opens the argument list; `;` accepts
  and terminates the statement; `<` accepts and fills type arguments for classes.
- **Smart semicolon**: when completing a statement-level call, a `;` is added if the statement is complete
  and the line has no terminator.

- **JAVA-CMP-035** `MUST` One item per name with overloads merged; insertion of `(`; parameter info.
- **JAVA-CMP-036** `SHOULD` Completion characters behavior (Enter/Tab/`.`/`(`/`;`/`<`).
- **JAVA-CMP-037** `SHOULD` Smart semicolon.

---

## 6.9 Scenarios

**C1: import on completion.** File with no imports, caret after `Arr`. Basic completion at count 1 lists
`Arrays`, `ArrayList`, `ArrayDeque`, `ArrayIndexOutOfBoundsException`, ... all `java.util`/`java.lang`; choose
`ArrayList` (type position `Arr|` in a declaration `Arr| names;`). Result: `ArrayList names;` and
`import java.util.ArrayList;` added at the top. In a file that already has `import java.util.List;` etc.:
the new import is placed in sorted position.

**C2: static member on second press.** Test file, caret in `assertTh|` (statement position). First press:
nothing matches except in-scope members (none). Second press: items `Assertions.assertThat(...)` (several
classes each once: `Assertions`, `AssertionsForClassTypes`, `AssertionsForInterfaceTypes`), `MatcherAssert.
assertThat(...)`, `Assert.assertThat(...)` (struck through if deprecated); ranking uses 6.5 (kind =
class-name-or-global-static tier for all; then explicit proximity/accessibility; deprecated last). Choose
the first with default settings: inserted text `Assertions.assertThat(` plus normal import of `Assertions`
(`import org.assertj.core.api.Assertions;`), and the advertisement `To import a method statically, press
Alt+Enter` was shown; toggling `Import statically` before choosing yields `import static
org.assertj.core.api.Assertions.assertThat;` and the text `assertThat(`. The same ordering and filters
as the import fix in `05` apply to *which* classes are listed (same index, same scope).

**C3: after dot.** `"abc".|` lists String instance methods, `chars`, `length`; static methods of `String`
(like `valueOf`) appear too because instance access may call statics (marked, ranked lower); `String.|`
lists statics and member types only.

**C4: type filter.** `catch (|` lists `Exception`, `IOException`, ... exceptions that the try block can
throw first (very suitable), other `Throwable` subclasses next, non-throwable classes last or hidden.

**C5: smart.** `List<String> l = |` smart: `new ArrayList<String>()`-style items for inheritors,
`Collections.emptyList()` (`collectionFactory` tier), `List.of()`, locals of type assignable, `null`.

---

## 6.10 Open questions

- **Q-CMP-1** Exact set of weigher values for `liftShorter`/`middleMatching` classifiers is platform
  behavior (not Java-specific); an Archipel platform decision should cover them for all languages.
- **Q-CMP-2** The usage-statistics store (`stats`) is shared with the import fix ordering (05 5.3 key 3);
  decide a persistent format once for both.

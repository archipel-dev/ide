# 05. Imports, auto-import and the `assertThat` case

Status: specification, clean-room. Written from the IntelliJ IDEA Community sources in our own words.
Source paths are cited as `ic: <path>` relative to the repository root of the Community checkout. No
code is reproduced. Message texts quoted below are short user-visible strings from the message
bundles and are facts.

Scope of this file:

1. The reported bug: `assertThat(x)` in a JUnit test, AssertJ on the test classpath, no import.
2. The static-import quick fixes (candidate search, filtering, ordering, popup, edit).
3. The class-import fix (`Import class`), the auto-import hint, silent auto-import.
4. Unused-import detection.
5. Optimize Imports (ordering, layout, on-demand thresholds, static imports, defaults).
6. Settings that influence all of the above.

Priority tags: `MUST` (the scenario cannot pass without it), `SHOULD` (visible quality gap),
`LATER` (valid, schedule after the Phase 4 gate).

Terminology. *Place* is the reference expression or reference element the fix is computed for.
*Resolve scope* of a file is defined in `02-resolve.md` section 2.7: the file's module with its
dependencies and libraries; test code and test-scope libraries are included only when the *file*
is in a test source root. *Member* is a method, field or class.

---

## 5.1 The reported bug, end to end

### 5.1.1 Situation

A module with a test source root `src/test/java`, and on the test classpath (Maven `test` scope or
Gradle `testImplementation`): JUnit 4.13 (or JUnit 5), AssertJ (`assertj-core`), Hamcrest. The user
types a call to `assertThat` with one argument and has no import for it.

Input file `src/test/java/demo/CalcTest.java`:

```java
package demo;

import org.junit.Test;

public class CalcTest {
    @Test
    public void adds() {
        String x = "a";
        assertThat(x).isEqualTo("a");
    }
}
```

### 5.1.2 What IntelliJ shows

**Markup.** One error on the method-name identifier `assertThat` only (not the arguments, not the
whole call, not `.isEqualTo`). It is an *unresolved reference* style highlight (red wavy underline,
the "cannot resolve" text attribute), not a plain error-range highlight. Message text exactly:

`Cannot resolve method 'assertThat(String)'`

The message is the bundle string `call.unresolved.name` with the name and the argument types, in
order, comma-separated with a single space, using presentable (short, simple) type names; an
argument whose type cannot be computed prints as `?`. (`ic: java/codeserver/highlighting/.../errors/JavaErrorKinds.java`
kinds `CALL_UNRESOLVED_NAME`, `JavaErrorFormatUtil.formatArgumentTypes`, bundle
`JavaCompilationErrorBundle.properties`.) `.isEqualTo("a")` gets no second error: its qualifier has
an unknown type, so nothing is reported for it.

**Why this error and not another.** The method call resolves to zero candidates (no method named
`assertThat` is visible: not a member of the class or its supertypes, not an outer class member, not
statically imported). With zero candidates the "cannot resolve method" variant is chosen. Other
variants exist (see `04-error-catalog.md` JAVA-ERR-007..010) for cases where candidates exist.

**Alt+Enter on the identifier (or anywhere inside the call).** The quick fixes of this error are
registered in this order (`ic: java/java-analysis-impl/.../HighlightFixUtil.java`
`registerMethodCallIntentions`, `registerStaticMethodQualifierFixes`):

| # | Item text | Notes |
|---|---|---|
| 1 | `Import static method...` | single candidate: `Import static method 'org.assertj.core.api.Assertions.assertThat'` |
| 2 | `Qualify static call...` | single candidate: `Qualify static call 'org.assertj.core.api.Assertions.assertThat'`; edit is `Assertions.assertThat(x)` plus a normal class import |
| 3 | `Add qualifier...` / `Add qualifier 'name' to method` | only when a variable or field in scope has a type with a method of this name |
| 4 | `Create method 'assertThat...'` | create-from-usage in the enclosing class (exact title: see Open question Q-IMP-1) |
| 5.. | other create-from-usage targets, then generic intentions | not part of this contract |

Items 1 and 2 are offered only when candidates exist (5.2). When the file is a production file
(`src/main/java`) and AssertJ is a test-scope library, there are zero candidates: items 1 and 2 are
absent and the user sees only the generic items. This is correct behavior, not a bug.

**Editor hint.** After highlighting finishes, a hint bubble appears over the *whole call
expression* `assertThat(x)` (not only the name): see 5.4. It never inserts anything by itself for
static members.

**After choosing the AssertJ entry.** Result file:

```java
package demo;

import org.junit.Test;

import static org.assertj.core.api.Assertions.assertThat;

public class CalcTest {
    @Test
    public void adds() {
        String x = "a";
        assertThat(x).isEqualTo("a");
    }
}
```

The call text is unchanged (still unqualified). A new single-static-import line is added in the
static block, which by default sits last, separated by one blank line from the preceding block
(5.6). The error and the hint disappear. If the file already had two single static imports from
`org.assertj.core.api.Assertions` the result would instead be one `import static
org.assertj.core.api.Assertions.*;` (rule JAVA-IMP-027).

### 5.1.3 Requirements for the bug

- **JAVA-IMP-001** `MUST` Unresolved unqualified method call with zero candidates reports exactly one
  error with message `Cannot resolve method '<name>(<argTypes>)'`, anchored on the name identifier,
  styled as unresolved reference. *Accept:* scenario 5.1.2, markup range equals the 10 characters of
  `assertThat`.
- **JAVA-IMP-002** `MUST` The message prints argument types as presentable short names; unknown type
  prints `?`; zero arguments prints `()`. *Accept:* `foo()` -> `Cannot resolve method 'foo()'`;
  `foo(1, "a", unknownVar)` -> `Cannot resolve method 'foo(int, String, ?)'`.
- **JAVA-IMP-003** `MUST` The static-import fix is registered on that error as the first quick fix
  (before qualify-static, add-qualifier, create-method). *Accept:* the Alt+Enter list of 5.1.2 starts
  with `Import static method...`.
- **JAVA-IMP-004** `MUST` In a test-source file the AssertJ `Assertions.assertThat` is found; in a
  production-source file with AssertJ test-scoped it is not. *Accept:* same snippet under
  `src/main/java` yields no import item.
- **JAVA-IMP-005** `MUST` Choosing a candidate adds `import static <class>.<method>;` in the static
  block and leaves the call text unchanged. *Accept:* file in 5.1.2.
- **JAVA-IMP-006** `MUST` The edit is a single undo step.
- **JAVA-IMP-007** `SHOULD` The fix title shows the member when exactly one class qualifies.

---

## 5.2 Static-import fix: candidate search

Fix family: "import static method" (call), "import static constant" (field reference). Both share one
algorithm; the method flavor is described and the field flavor lists the differences. Sources:
`ic: java/java-impl/src/com/intellij/codeInsight/daemon/impl/quickfix/StaticImportMemberFix.java`,
`StaticImportMethodFix.java`, `StaticImportConstantFix.java`, `StaticMembersProcessor.java`,
`MethodCallSignature.java`, `StaticImportMemberQuestionAction.java`.

### 5.2.1 Preconditions (when the fix exists at all)

The fix object is created on a background thread under a read lock and computes its candidates once
at creation. It is *not available* (zero candidates, never shown) when any of the following holds:

- the language level of the file does not support static imports (below Java 5), or the file is not a
  Java file;
- the reference is qualified (`Foo.assertThat(x)`): the fix is for simple names only;
- the file cannot be modified (read-only, library file);
- the call already resolves to a method (`resolveMethod()` not null);
- the reference has no name.

The fix also becomes unavailable as soon as the PSI changes after it was computed (stale fix): the
daemon recomputes on the next pass.

### 5.2.2 Candidate enumeration

1. Take the reference name `N` (`assertThat`).
2. Query the *method-name index* for all methods named `N` inside the **resolve scope of the place**
   (not the project scope, not the module scope). Library class-file stubs participate; both source
   and compiled copies may appear, see step 6. (`ic: java-indexing-impl/.../PsiShortNamesCacheImpl.java`,
   `JavaStubIndexKeys.METHODS`.)
3. For every hit apply the processor filters, in this order:
   a. **Excluded by settings**: the member's qualified name `pkg.Class.method` matches an entry of the
      exclude list (project-level list plus IDE-level "excluded packages"), see 5.7. Matching: the
      entry is a prefix of the name that ends at a dot boundary or equals the name; an entry that
      contains `*` is a mask matched from the start, again ending on a dot boundary.
   b. **Import filter extension points** may veto the containing class FQN ("use fully qualified
      name instead of importing"). Default: none.
   c. **Interface methods** declared in Java source or compiled Java: a method of an interface must
      carry the explicit `static` modifier, otherwise it is skipped. (A default or abstract interface
      method is never a candidate.)
   d. **Default package**: for the static-import flavor, a member declared in the default (empty)
      package is dropped. (For the qualify flavor it is kept only if the place is itself in the
      default package.)
   e. Group the survivors by containing class, preserving discovery order; stop collecting when 100
      distinct classes have been gathered.
4. For each class group (a class FQN is handled once; a second class object with the same FQN, e.g.
   source and class-file copies or two jar versions, is ignored) walk its members in discovery order:
   a. skip non-`static` members;
   b. the first static member of the class is remembered as the class's *fallback* entry (kind NONE);
   c. skip it for the "applicable" evaluation when it is **not accessible from the place** (visibility
      per JLS 6.6, evaluated at the place) or, for the static-import flavor, **not accessible from the
      file** (visibility evaluated as for an import in this compilation unit: public, or package-private
      in the same package, protected when permitted);
   d. otherwise compute its **applicability** (5.2.3); if it is APPLICABLE, the class contributes this
      member to the applicable list and the walk of this class stops; if only PARTLY, remember it as the
      class's best entry and continue looking for a better one in the same class.
5. Result lists:
   - `applicable`: one member per class, in class discovery order, then stably sorted (5.3);
     after them append the PARTLY members (each sorted by the same comparator) that are not already
     present;
   - `all`: one member per class (fallback entry upgraded to the best kind found).
6. The fix's final candidate list is `applicable` if non-empty, otherwise `all`. From it drop members
   that are *not valid*, and members that are not accessible from the file **and** whose file is not
   inside project content (this keeps inaccessible-but-project members: the user can then fix access
   afterwards; library members that are inaccessible are removed).

So: **applicable candidates hide non-applicable ones.** Only when no class has an applicable overload
does the fix list classes whose name matches but whose arguments do not.

### 5.2.3 Applicability of a candidate method

For a call place:

1. Infer a substitutor for the method's type parameters from the call's argument *types* (without a
   target type), in the standard loose-invocation manner (JLS 15.12.2.2/18.5.1; `03-types.md`).
2. The method is **applicable** when the argument list is applicable to it in the sense of JLS 15.12.2
   (arity, including varargs, and loose type conversion per argument). Arguments whose type is
   unknown (unresolved) are treated as convertible to anything.
3. If applicable, take the return type after substitution. If the return type is absent (constructor-like) or
   the place has no expected type (expression statement, qualifier of another call), the result is
   APPLICABLE. Otherwise compare with the **expected types** of the place (assignment target,
   argument position, return statement, condition): APPLICABLE if the return type is assignable to
   one of them; PARTLY if an expected type is a parameterized class type and its *erasure* accepts the
   return type; NONE otherwise. When there are no expected types the result is APPLICABLE.
4. Not applicable by arguments: NONE.

Field flavor: kind is decided only by the field type against the expected types (same rule as step 3);
there is no argument check.

### 5.2.4 Requirements

- **JAVA-IMP-010** `MUST` Candidate methods come from the method-name index limited to the place's
  resolve scope (test vs production per 02-resolve). *Accept:* production file never offers a
  test-scope library method; test file offers it.
- **JAVA-IMP-011** `MUST` Only `static` members; interface methods need an explicit `static`.
  *Accept:* `WithAssertions.assertThat` (an interface default method) is not offered.
- **JAVA-IMP-012** `MUST` Accessibility filter at the place and at the file for static imports.
  *Accept:* a package-private static method of another package is not offered; a private one never.
- **JAVA-IMP-013** `MUST` One member per class; a class FQN appears once even if indexed twice.
  *Accept:* a project having both `assertj-core` jar and its sources shows each class once.
- **JAVA-IMP-014** `MUST` Applicable members hide non-applicable ones. *Accept:* with
  Hamcrest, JUnit 4 and AssertJ present, `assertThat(x)` lists only AssertJ classes
  (Hamcrest/JUnit `assertThat` need two or three arguments); `assertThat(x, is(1))` lists Hamcrest
  `MatcherAssert` and JUnit 4 `Assert`, not AssertJ.
- **JAVA-IMP-015** `MUST` When nothing is applicable, list one member per class with a matching name.
  *Accept:* `assertThat()` with zero arguments lists all classes (fix title ends with `...`).
- **JAVA-IMP-016** `MUST` Excluded names are never offered (5.7).
- **JAVA-IMP-017** `MUST` Cap at 100 classes before sorting.
- **JAVA-IMP-018** `SHOULD` Unknown argument types do not make a candidate inapplicable.
  *Accept:* `assertThat(undefinedVar)` still offers the AssertJ classes.
- **JAVA-IMP-019** `SHOULD` Expected-type filtering (APPLICABLE/PARTLY/NONE) as in 5.2.3 step 3.
  *Accept:* `int n = max(a, b);` offers `Math.max(int,int)` before methods returning non-numeric types.
- **JAVA-IMP-019a** `MUST` Compiled (class-file) members and source members are treated the same;
  synthetic and bridge methods never appear.

---

## 5.3 Static-import fix: ordering

All lists (applicable, partly, fallback) are sorted with one comparator, applied to the *members*
(`ic: java/java-impl/.../CodeInsightUtil.java` `createSortIdenticalNamedMembersComparator`, and
`platform/lang-impl/.../psi/util/proximity/PsiProximityComparator.java`). Keys in order, first
difference decides:

1. **Not deprecated before deprecated.** "Effectively deprecated" = the member or its containing class
   is deprecated (annotation or javadoc tag), or marked for removal.
2. **Top-level before nested.** A member of a nested class sorts after a member of a top-level class
   (for class candidates: nested classes after top-level classes).
3. **Usage recency (learning).** If both members have a usage-statistics record (a record is written
   each time the user picks that member through an import fix or completion), the one used more recently
   first. Statistics are keyed by the member's qualified name and are per installation.
4. **Proximity weights** compared left to right, higher weight first (full table in 5.3.1).
5. **Class qualified name, case-insensitive, ascending** - applies only when both candidates are
   classes. For methods and fields this key does not exist: the two stay in the order the index
   returned them.

Result for ties: the order of step 4 can legitimately be "equal" for all library members (they are all
outside the module, all in the resolve scope, all not in any open editor, all in a library). In
IntelliJ the remaining order is the incidental iteration order of the index and is not guaranteed.

**Archipel rule (deliberate, deterministic).** After key 4, break ties by comparing the *containing
class qualified name*, case-insensitive, ascending; then by member signature text. This makes the
order reproducible; it deviates from IntelliJ only where IntelliJ is unspecified.

### 5.3.1 Proximity weighers, in chain order

Each weigher maps a candidate to a comparable value; the chain compares weigher 1 first. Higher is
better. Registered order (from the extension declarations, `ic: platform/lang-impl/resources/intellij.platform.lang.impl.xml`
and `java/java-backend/resources/META-INF/JavaPlugin.xml`; explicit `before/after` constraints,
topologically resolved):

| Rank | Weigher | Value (higher better) |
|---|---|---|
| 1 | reference-list kind (only classes in `extends`/`implements`/`throws` positions) | by-kind/by-name applicable > applicable... > unknown > inapplicable |
| 2 | same PSI member | 2: both inside the same method/field; 1: same class member; 0: otherwise |
| 3 | accessibility | normal(3) > discouraged(2; e.g. `com.sun.*`, `sun.*`, `org.omg.*`, `java.awt.List`, unstable-API annotated) > deprecated(1) > inaccessible(0) |
| 4 | explicitly imported | an ordered scale, best first: member already imported by its qualified name; declared in the same file; class declared top-level in the file's package; class imported by single import; member declared in the file's package; class in `java.lang`; class whose package is imported on demand (top-level); module-imported class; nested class reachable through an on-demand static import; nested class declared in the file's package; class from a package of which some other class is already imported; unknown |
| 5 | opened in editor | candidate file open in an editor |
| 6 | same directory | candidate lives in the file's directory |
| 7 | Java inheritance | class is a supertype/outer type of the place's class (excluding `java.lang.*`) |
| 8 | same source root | same source root as the place |
| 9 | same module | 2 same module; 1 any other project module; 0 library/SDK |
| 10 | known element | popular JDK classes (`String`, `Class`, `System`, `Runnable`, `Exception`, `Throwable`, `RuntimeException`, `ArrayList`, `HashMap`, `HashSet`) 8; `java.util` 7; `java.lang` 6; `Object` and other `java.lang.*` 5; `java.util.*` subpackages 4; `javax.swing` 3; other `java.*` 2; `javax.*` 1; others 0; names starting `com.` or `net.` -1. In test files, classes whose qualified name contains `junit` or `test` get 1 (classes only); `org.testng.internal` gets -1; excluded packages -1 |
| 11 | in resolve scope | candidate file is in the place's resolve scope |
| 12 | directory type | under production source root 3; test source root 2; resource roots 1; else 0 |
| 13 | SDK or library | non-JDK better than JDK? No: *true* (is JDK element) sorts higher - ties are rare; see note |

Notes. Rank 13 returns "is an SDK element" as a boolean and larger is better, so JDK members sort before
otherwise equal library members; it rarely decides anything because ranks 4, 9 and 10 differ first.
The declarations only constrain: reference-list before same-member before explicitly-imported;
accessibility before explicitly-imported; inheritance between explicitly-imported and same-source-root;
known-element between same-module and in-resolve-scope. The relative order of same-member versus
accessibility, and of inheritance versus editor-open/same-directory, is not constrained; the table shows
one valid order, and no known scenario depends on the unconstrained pairs.

Weighers that return "unknown" for member candidates (reference-list, inheritance for non-classes)
contribute equal values and so do not decide.

### 5.3.2 Why AssertJ, Hamcrest and JUnit are ordered as they are

- Applicability (5.2) already separates them by argument count: `assertThat(actual)` is AssertJ only.
- With two arguments, JUnit 4's `Assert.assertThat` has been deprecated since 4.13, so it goes after
  Hamcrest's `MatcherAssert.assertThat` by key 1. With an older JUnit that is not deprecated, key 1
  does not separate them; keys 2-4 tie; the Archipel tie-break gives `org.hamcrest.MatcherAssert`
  before `org.junit.Assert` (alphabetical).
- Among the AssertJ classes (`Assertions`, `AssertionsForClassTypes`, `AssertionsForInterfaceTypes`,
  and in old versions `Java6Assertions`, which is deprecated and therefore last) keys 1-4 tie except
  deprecation. The Archipel tie-break yields `Assertions`, `AssertionsForClassTypes`,
  `AssertionsForInterfaceTypes`, `Java6Assertions`. **Only the position of `Assertions` first is
  contractual** for conformance tests.
- If the file already statically imports another member of the same class (for example
  `Assertions.assertThatThrownBy`), rank 4 is unaffected for methods (it keys on the exact member
  name), but the "learned" recency (key 3) is: the user's previous pick wins.

### 5.3.3 Requirements

- **JAVA-IMP-020** `MUST` Comparator keys 1, 2, 5 and the Archipel tie-break as defined.
  *Accept:* AssertJ list order is `Assertions` first; deprecated `Java6Assertions` last.
- **JAVA-IMP-021** `SHOULD` Implement proximity weighers 2-4, 9-12 (the ones that can differ for
  library and project members). *Accept:* a project-local `TestUtils.assertThat(int)` in the same module
  outranks library methods when applicable.
- **JAVA-IMP-022** `SHOULD` Usage-recency learning (key 3).
- **JAVA-IMP-023** `LATER` Remaining weighers (editor-open, same directory, inheritance).

---

## 5.4 Hint, popup and the edit

### 5.4.1 Hint (the automatic bubble)

Conditions, all required (`ic: StaticImportMemberFix.showHint`, `platform/lang-impl/.../ShowAutoImportPass.java`):

- setting "Show import popup" is on (`SHOW_ADD_IMPORT_HINTS`, default on) and import hints are
  enabled for the file;
- setting "Add unambiguous imports on the fly" is **not** consulted for static members; and the
  static-members hint has its own switch: "Show import popup for: Static methods and fields"
  (`ADD_MEMBER_IMPORTS_ON_THE_FLY`, default on);
- the place is unqualified and its parent call still exists, PSI unchanged since the fix was computed;
- the hint candidate list is not empty. Hint candidates: the full candidate list when it contains an
  applicable member; otherwise (fallback list) only members whose **signature can match the call**:
  argument count compatible (a call whose closing parenthesis is missing counts as "at least this many
  arguments") and loosely compatible erased types per argument (primitive arguments box; type-variable
  parameters check convertibility; lambdas, method references and unknown types match anything);
- the user did not dismiss the hint for this element by pressing Escape;
- no other hint that is configured to be replaced is currently visible;
- not when the single candidate is declared in the same file (nothing to import).

Hint text (new UI): a link `Import static method...` followed by the member name and a question mark
and the shortcut for *Show Intention Actions* (Alt+Enter), e.g. `Import static method... Assertions.assertThat? Alt+Enter`
(exact member display name: qualified name for one candidate; for several candidates the first one with
the suffix `(multiple choices...)` in the classic UI). Range of the hint: the whole call expression.
Hints are produced for the highlights in the visible range, ordered by distance to the caret; the first
that shows stops the pass. The hint never auto-applies.

### 5.4.2 Choosing: single versus several candidates

- Exactly one candidate: invoking the fix (Alt+Enter item or hint click) applies it with no popup.
- Several: a list popup titled `Method to Import` (field flavor: `Field to Import`; class flavor:
  `Class to Import`). Rows: left `ShortClass.member` (e.g. `Assertions.assertThat`; for nested classes
  `Outer.Inner.member`), right grey `(package)` with the package of the containing class (omitted for
  the default package), class icon. Speed search is on (typing filters rows); the popup does **not**
  auto-select a lone remaining entry; Enter or click chooses; Right arrow opens a sub-popup with
  entries `Exclude '<prefix>' from auto-import` for the member's qualified name `a.b.c.Class.member`
  and each shorter prefix with at least two name segments (`a.b.c.Class`, `a.b.c`, `a.b`); choosing one
  opens Settings | Editor | General | Auto Import with that exclusion pre-added. Row order is the
  candidate order (5.3).
- Headless/test mode: first candidate is applied directly.

### 5.4.3 The edit (static import)

Performed in one write command named after the fix text. Steps:

1. For every reference in the file whose name equals `N`, remember what it currently resolves to.
2. If an equivalent import exists already - an on-demand static import of the same class, or a
   single static import of that class and name - do nothing more for the import list. Otherwise, if
   the chosen member is a class, import it as a class; otherwise add a static import:
   a. Count existing single static imports of the same class (`k`). If the code style says the class is
      always imported on demand (5.6), or `k + 1 >= NAMES_COUNT_TO_USE_IMPORT_ON_DEMAND` (default 3),
      and no conflicting on-demand import exists, then **delete those `k` single imports and add
      `import static <Class>.*;`**.
   b. Otherwise add `import static <Class>.<name>;`.
   The insertion position follows the layout rules of 5.6.
3. Re-bind other references named `N`: an unqualified reference that used to resolve to a different
   member now resolves to the import and would change meaning; it is rewritten to a qualified
   reference `Owner.N` for its previous target. A qualified reference whose qualifier is a class that
   is the imported class or a subtype of it has the qualifier removed. (This keeps behavior unchanged.)

The qualify flavor (`Qualify static call`) instead sets the call's qualifier to the containing class
and then shortens the class reference, which adds a class import if the code style asks for it.

### 5.4.4 Requirements

- **JAVA-IMP-030** `MUST` Popup title `Method to Import` (field `Field to Import`, class `Class to Import`).
- **JAVA-IMP-031** `MUST` One candidate: no popup. Several: popup with rows as described, speed search,
  no auto-select.
- **JAVA-IMP-032** `MUST` Fix title `Import static method...` for several candidates (with ellipsis), or
  `Import static method '<qualified member>'` for one. Constant flavor: `Import static constant...`.
- **JAVA-IMP-033** `MUST` Edit semantics of 5.4.3 including the on-demand collapse at the third name.
  *Accept scenario S3 below.*
- **JAVA-IMP-034** `MUST` Hint shown for static candidates only when `ADD_MEMBER_IMPORTS_ON_THE_FLY` is
  on; Escape dismisses it for that element.
- **JAVA-IMP-035** `SHOULD` Hint text and range as in 5.4.1.
- **JAVA-IMP-036** `SHOULD` Exclusion sub-popup entries.
- **JAVA-IMP-037** `MUST` The hint never edits the file.
- **JAVA-IMP-038** `SHOULD` Rebinding of other same-named references (5.4.3 step 3).
- **JAVA-IMP-039** `LATER` Preview of the import edit (diff) in the intention popup.

### 5.4.5 Acceptance scenarios

**S1: the bug.** Input 5.1.2. Expected: error `Cannot resolve method 'assertThat(String)'` on
`assertThat`; Alt+Enter first item `Import static method...`; popup `Method to Import` lists, in order:
`Assertions.assertThat (org.assertj.core.api)`, `AssertionsForClassTypes.assertThat (org.assertj.core.api)`,
`AssertionsForInterfaceTypes.assertThat (org.assertj.core.api)`; choose the first; file as in 5.1.2.

**S2: two arguments.** Input: `assertThat(x, is("a"));` with Hamcrest and JUnit 4.13. Popup:
`MatcherAssert.assertThat (org.hamcrest)` then `Assert.assertThat (org.junit)` (deprecated, struck through
in the real UI). No AssertJ rows.

**S3: collapse to on-demand.** Input has `import static org.assertj.core.api.Assertions.assertThat;`
and `import static org.assertj.core.api.Assertions.assertThatThrownBy;`; the user adds a call
`assertThatCode(...)` and imports it. Result: both single imports are removed and
`import static org.assertj.core.api.Assertions.*;` is the only import of that class.

**S4: production file.** Same input under `src/main/java`, AssertJ `testImplementation`. Error as S1;
Alt+Enter does not contain import items.

**S5: excluded.** Exclude list contains `org.assertj.core.api.AssertionsForClassTypes`. Popup in S1
has two rows. Excluding `org.assertj` removes the fix entirely (no candidates).

**S6: stale.** After the import is applied elsewhere in the same file, another unresolved `assertThat`
in a second method: the fix recomputes; since the first import now resolves it, no error remains.

---

## 5.5 `Import class` fix, auto-import and hint

Source: `ic: java/java-impl/.../quickfix/ImportClassFixBase.java`, `ImportClassFix.java`,
`actions/AddImportAction.java`, `daemon/impl/JavaReferenceImporter.java`,
`platform/lang-impl/.../ShowAutoImportPass.java`.

### 5.5.1 When it is offered

The reference is an unresolved class reference: error `Cannot resolve symbol 'Foo'` (JAVA-ERR-001)
on the identifier. The fix is registered with the highest priority (first in Alt+Enter), title
`Import class`.

### 5.5.2 Candidate search

1. Do nothing if the reference already resolves to a class (validly, or through an import that merely
   is not accessible); if it is part of an import statement; if it is the class name of a `new` whose
   expression has an outer-instance qualifier (`outer.new Inner()`); if it is an expression reference
   whose parent is neither a qualified reference nor an expression statement (so the unresolved
   identifier `Collections` in `Collections.emptyList()` qualifies, but `x = Collections;` does not);
   if the reference has a qualifier that is an expression.
2. Query the **short-class-name index** for `Foo` in the file's resolve scope.
3. Keep a class only if all hold: for an annotation position it is an annotation type; its qualified
   name is not null, contains a dot (not the default package), is a valid qualified name, is not in an
   excluded package (5.7), and is not vetoed by an import filter.
4. Remove an **inaccessible** class (visibility per JLS 6.6 at the place) when at least one candidate
   is accessible, or when the class cannot be modified, is in the file's own package, or is a member of
   an implicitly declared class. Accessible classes are never removed by this step. (Net effect: if
   nothing at all is accessible, inaccessible library classes in other packages are kept, so the user
   still gets an import and then an access error that explains the problem.)
5. If the parent is a qualified reference `Foo.bar` or a method reference `Foo::bar`, keep only classes
   that have a member `bar` (field or inner class or method; static when the form is `Foo.bar`,
   any when method reference). If no class has it, keep all (the member may be created later).
6. Context narrowing: for a type reference in a variable declaration with an initializer, keep classes
   that are supertypes-or-equal of the initializer type; for a parameter of an `@Override` method keep
   classes that appear as parameter types in the overridden methods' signatures. If narrowing yields
   nothing, keep the unnarrowed list.
7. If the reference is itself qualified with a package-like prefix (for example `util.List`) and that
   prefix is not implicitly imported, keep only classes whose package equals the prefix.
8. Drop classes whose simple name equals the simple name of an already **unresolved** single import in
   the file (the user already tried to import something and failed).
9. If the reference name matches the "do not auto-import" pattern, no candidates. Default pattern: one
   lowercase letter optionally followed by one more character (`[a-z].?`) - this prevents imports for
   names like `i`, `ab`.
10. If more than one candidate remains, remove candidates that violate a configured dependency rule
    (scope dependency rules) but never the last/first remaining one.
11. Sort with the comparator of 5.3 applied to classes (deprecated last, nested last, usage recency,
    proximity, then qualified name case-insensitive ascending). Concretely for `List`: `java.util.List`
    before `java.awt.List` (the latter is "discouraged" by accessibility rank 3 and has a lower
    known-element rank).

If the reference has type arguments `Foo<..>`, only classes with type parameters are offered by default;
the list is relaxed ("accept wrong number") when checking availability.

### 5.5.3 Applying

- One class: bind the reference to it, which adds the import through the code-style import machinery
  (5.6, `addImport`) or qualifies the name when importing is impossible (name conflict).
- Several: list popup titled `Class to Import`. Row text is the **fully qualified** name; icon is the
  class icon; right arrow opens the exclusion sub-popup (`Exclude '<prefix>' from auto-import`,
  prefixes as 5.4.2 for the class FQN). Speed search on, no auto-select. Choosing increments the usage
  statistic of that class.
- Inner classes: importing a nested class normally produces the import of the nested class only when
  code-style option "Insert imports for inner classes" is on; when it is off the reference is rewritten
  as `Outer.Inner` plus import of `Outer`, and the silent auto-import is not performed.

### 5.5.4 Hint and silent auto-import

- Hint (default on, setting "Show import popup"): computed for the unresolved references that carry
  an import fix, visible range only, nearest to the caret first, first one that shows wins. Not shown
  when the reference is qualified, when the caret is directly at the end of the reference name (the user
  is probably still typing), when an *unresolved* import in the file could be the intended import (it
  would be fixed instead), when the user dismissed it with Escape for that element, or in headless
  mode. Text: `<fqn>? Alt+Enter` for one candidate, `<first fqn>? (multiple choices...) Alt+Enter` for
  several (new UI: a link `Import class...` followed by the name and the shortcut). Range = the
  identifier. Clicking or pressing the shortcut runs the fix of 5.5.3.
- Silent auto-import ("Add unambiguous imports on the fly", default **off**): when on, an unresolved class
  reference with exactly one candidate is imported without asking, as an undo-transparent edit, if the
  file may be changed silently (in project content, writable, not vetoed by an extension) and the
  import would not insert unexpected characters (an inner-class import while "Insert imports for inner
  classes" is off). It applies in two situations: references on the caret line other than the one
  that contains the caret, and the reference whose hint is being computed when the caret is not at the
  end of the name. References with several candidates are never imported silently. Static members are
  never imported silently (only the hint).
- Optimize imports on the fly (default off) runs after such actions.

### 5.5.5 Requirements

- **JAVA-IMP-040** `MUST` `Import class` registered first on unresolved class references.
- **JAVA-IMP-041** `MUST` Candidate search steps 2-4 and 8-9.
- **JAVA-IMP-042** `MUST` Sorting per 5.3 on classes; `java.util.List` before `java.awt.List`.
  *Accept:* both on classpath of JDK; popup lists `java.util.List` first.
- **JAVA-IMP-043** `MUST` Popup `Class to Import` with FQN rows; one candidate applies directly.
- **JAVA-IMP-044** `MUST` The edit adds a single-type import in the sorted position (5.6) unless the
  class is in `java.lang` or the file's package (then no import line and the reference stays unqualified).
- **JAVA-IMP-045** `SHOULD` Step 5 and 6 narrowing.
- **JAVA-IMP-046** `SHOULD` Hint and Escape dismissal, caret-adjacent suppression.
- **JAVA-IMP-047** `SHOULD` Silent auto-import setting honoring one-candidate rule.
- **JAVA-IMP-048** `SHOULD` Inner-class import rule.
- **JAVA-IMP-049** `LATER` Narrowing by members used on the variable and by returned type.

---

## 5.6 Adding an import and Optimize Imports

Source: `ic: java/java-impl/.../psi/impl/source/codeStyle/ImportHelper.java`,
`java-frontback-impl/.../formatter/java/ImportHelperBase.java`, `JavaCodeStyleSettings.java`,
`platform/code-style-api/.../PackageEntry.java`, `PackageEntryTable.java`,
`java-impl/.../lang/java/JavaImportOptimizer.java`.

### 5.6.1 Default Java code-style import settings (exact)

| Setting | Default |
|---|---|
| Use single class imports | on |
| Class count to use import with `*` | 5 |
| Names count to use static import with `*` | 3 |
| Insert imports for inner classes | off |
| Layout static imports separately | on |
| Layout on-demand import from the same package first | on |
| Preserve module imports | on; Delete unused module imports: off |
| Keep blank lines between imports | off |
| Packages to use import with `*` | `java.awt` (exact package, no subpackages), `javax.swing` (exact) |
| Import layout (top to bottom) | `<all module imports>`, `<all other imports>`, blank line, `javax.*` (with subpackages), `java.*` (with subpackages), blank line, `<all other static imports>` |

So a default-laid-out file looks like: third-party and project imports first (sorted), then one blank
line, then `javax.` imports, then `java.` imports, then one blank line, then all static imports.
`import module` lines (Java 25) come before everything.

### 5.6.2 Layout and position algorithm

For an import with package (or, for a static import, class) name `P`:

1. Find its **layout entry**: the best-matching entry among the layout table. An entry matches when the
   package equals the entry's package, or the entry is "with subpackages" and `P` is below it. Static
   imports consider only static entries (`<all other static imports>` is the fallback for static); if
   "layout static imports separately" is off, statics use non-static entries. Best match = exact
   (non-subpackage) entry before subpackage entry; between two entries of the same kind the one with
   more dots in its package wins; `<all other ...>` loses to any specific entry. If no static entry exists
   at all, statics go into `<all other imports>`.
2. Entries are laid out in table order; within an entry, imports are sorted by their canonical text
   (qualified name; on-demand imports compare as `name.` so `a.b.*` sorts just before `a.b.C`),
   case-sensitive ordinal comparison of the text. (Optimize imports sorts by the name string the same way.)
3. Between two consecutive non-empty entries, the number of blank lines is the longest run of
   `<blank line>` entries between them in the table (default: 1 between the `all other`, `javax/java`,
   `static` groups). Two adjacent entries with no blank entry between them (`javax.*` and `java.*` by
   default) produce no blank line.
4. For a single import add (not full optimize): the new statement goes to the end of its entry's
   existing statements in sorted order, or if the entry is empty, before the first statement of any later
   entry.

### 5.6.3 Adding a class import (`addImport`)

Returns "not added" when a conflicting single import for the same short name exists (then the fully
qualified name must be used) or an import filter vetoes. Otherwise:

1. If the class is already visible under its short name (resolves to the same class) nothing to do.
2. An unused single import of a *different* class with the same short name is deleted first.
3. Decide on-demand: not if the package is already imported on demand; otherwise, if `Use single class
   imports` and `(number of existing single imports from that package) + 1 < Class count to use import
   with *` (5) and the package is not in "Packages to use import with *", use a single import. When an
   on-demand import is chosen and other single imports from that package exist, they are deleted and
   re-added if they would conflict.
4. If a class of the same short name exists in the current package or is visible otherwise (so an
   on-demand import would change which class a name refers to), force single import; if that cannot be
   made unambiguous, fail and use the fully qualified name.
5. Insert as in 5.6.2 step 4.

### 5.6.4 Optimize Imports (algorithm)

Entry points: Code | Optimize Imports (Ctrl+Alt+O), "Optimize imports on the fly" (off by default),
`Optimize imports` fix of the unused-import/missorted highlight. Only for files under a source root
(or scratch/light files). Steps (`prepareOptimizeImportsResult`):

1. **Collect used imports.** Walk all references of the file (code, annotations, javadoc, injected
   fragments; skip the import list). For each unqualified reference resolve it. Keep it iff it resolves
   through an import statement or implicit import: a class referenced through a single/on-demand import
   becomes `qualifiedName` (non-static); a member referenced through a static import (single or
   on-demand) becomes `Class.member` (static; for on-demand static, the member name is appended).
   Classes in the same package (and nested/own-file classes) are not imported. Unresolved *imports*
   already in the file are preserved as-is ("placeholders", on-demand ones as `x.*`) so the user does not
   lose them.
2. Sort names by plain string comparison, then group by layout entry (5.6.2) in table order; names that
   match no entry go last.
3. **Decide on-demand imports.** For every package (non-static) and for every class (static) count the
   names. On-demand is used when `Use single class imports` is off, or when the count `>=` the limit (class
   count 5 for packages, names count 3 for static class members), or the package/class is in the
   "Packages to use import with *" table. A static on-demand import is skipped if it would bring a static
   field whose name equals the short name of a single-type import of the file.
4. **Drop on-demand when it would be ambiguous.** For each static member short name imported from
   several classes: those classes are not imported on demand. Then find names that must stay single
   because (a) a class of the same short name exists in the file's package or in `java.lang`, (b) another
   on-demand package provides the same short name (two on-demand imports with a common class name makes
   both ambiguous) or an implicit/module import would shadow, (c) a static member of another on-demand
   class with that name is accessible. Those names are re-added as single imports.
5. If every name of an on-demand package ended up single, drop the on-demand import.
6. **Emit text**: for each name in layout order: for an on-demand package emit `import pkg.*;` (static:
   `import static Cls.*;`) the first time it is met (before or after its single names depending on "layout
   on-demand from same package first"); emit single imports for names forced single; implicit imports
   (`java.lang`, same package) are never emitted. Module imports are emitted first at the module entry
   position, sorted by module name.
7. Render the text, reformat with the file's code style (blank lines from the layout), keep comments that
   were attached to kept imports, and compare with the old list; no change = do nothing.
8. Notification text after running: `Rearranged imports` when nothing was removed; else `Removed N import`
   / `Removed N imports` optionally followed by `, added M import`/`imports` (counts of statements
   removed/added).

### 5.6.5 Requirements

- **JAVA-IMP-050** `MUST` Default settings of 5.6.1 exactly.
- **JAVA-IMP-051** `MUST` Layout algorithm 5.6.2 (entry matching, sorting, blank-line counting).
  *Accept scenario S7.*
- **JAVA-IMP-052** `MUST` Optimize Imports removes unused imports, keeps used, sorts per layout.
- **JAVA-IMP-053** `MUST` On-demand thresholds: classes 5 (`import java.util.*;` when 5 distinct
  `java.util` classes are used), static names 3, always-on-demand `java.awt.*`, `javax.swing.*`.
  *Accept scenario S8.*
- **JAVA-IMP-054** `MUST` Static imports are laid out in the static group at the end, separated by one
  blank line from the group above.
- **JAVA-IMP-055** `MUST` No import for `java.lang` or same-package classes (implicit).
- **JAVA-IMP-056** `SHOULD` Conflict handling of step 4 (name clash across on-demand imports forces
  single imports).
- **JAVA-IMP-057** `SHOULD` Unresolved imports are preserved.
- **JAVA-IMP-058** `SHOULD` Comments inside the import list preserved.
- **JAVA-IMP-059** `SHOULD` Notification strings of step 8.
- **JAVA-IMP-060** `LATER` Module imports (`import module java.base;`), template-language contexts, JSP.
- **JAVA-IMP-061** `SHOULD` Honor project code-style overrides of 5.6.1 (settings from the editor
  config / `.idea`-equivalent).

### Scenarios

**S7: layout.** Before (all six used):

```java
import java.util.List;
import static org.junit.Assert.assertEquals;
import org.junit.Test;
import javax.crypto.Cipher;
import java.io.File;
import com.acme.Util;
```

Optimize Imports result with default settings:

```java
import com.acme.Util;
import org.junit.Test;

import javax.crypto.Cipher;
import java.io.File;
import java.util.List;

import static org.junit.Assert.assertEquals;
```

Reading: the first group is `<all other imports>` sorted by text (`com.` before `org.`); one blank line
(the table has one blank entry before `javax`); `javax.*` and `java.*` are adjacent entries with no
blank entry between them, so no blank line; one blank line; the static group last.

**S8: on-demand.** Used only `javax.swing.JButton`: the line is `import javax.swing.*;` (the package is
in the always-on-demand table). Used five `java.util` classes (`List`, `Map`, `Set`, `Optional`,
`Objects`): `import java.util.*;`. Used four: four single imports. Used three static members of
`java.lang.Math`: `import static java.lang.Math.*;` (names count 3). Used two: two single static imports.

**S9: conflict.** Used: `java.util.Date`, `java.sql.Date`, and five other classes from `java.util` plus
five others from `java.sql`. Both packages reach the class threshold, but `Date` exists in both, so
neither on-demand import is kept as the only source of `Date`: both `Date` names are emitted as single
imports; the remaining names are then handled by the threshold rule again, and the on-demand imports
are kept for the other names only if that does not make any used simple name ambiguous. Acceptance
for implementers: after optimization every originally resolved simple name still resolves to the same
class (this is the invariant behind step 4).

---

## 5.7 Unused imports

Source: `ic: java/java-analysis-impl/.../UnusedImportsVisitor.java`, `LocalRefUseInfo.java`,
`RemoveAllUnusedImportsFix.java`, inspection `UNUSED_IMPORT` (JavaAnalysisBundle).

- Inspection "Unused import", default enabled, severity WARNING, rendered with the "unused" text
  attribute (grey, not wavy). Message exactly `Unused import statement` (the variant
  `Unused import (specified in template)` when the line comes from a file template).
- Range: the whole import statement (from `import` to the semicolon).
- An import is **used** if at least one reference in the file resolves through it. References counted:
  code, annotations, javadoc `{@link}`/`@see`/`@throws` references, injected language fragments. A
  reference in an incomplete project state with unresolved target counts as using any import that could
  provide the name (to avoid noise while the model loads). A method call that resolves to a local
  variable/field/parameter but whose name is imported statically counts the potential static import as
  used.
- An import is **redundant** (reported) when: not used; or it imports a class from the file's own
  package (single or on demand of the same package); or it is an on-demand module import not needed.
  Imports with syntax errors are skipped.
- Fixes (both on the highlight): `Remove unused imports` (removes all reported imports of the file in one
  edit) and `Enable 'Settings | Editor | General | Auto Import | Optimize imports on the fly'`.
- Companion inspection "Missorted imports", INFO severity, range = whole import list, appears when the
  used imports are not in layout order; fix `Optimize imports`. An unused-import highlight also schedules
  an automatic optimize run when "Optimize imports on the fly" is on.
- Also while reporting: the file shows an "imports can be optimized" hint only through those highlights.

- **JAVA-IMP-070** `MUST` Unused import detection with message `Unused import statement`, WARNING,
  range whole statement, unused text style. *Accept:* `import java.util.List;` with no use.
- **JAVA-IMP-071** `MUST` Fix `Remove unused imports` removes all unused imports of the file.
- **JAVA-IMP-072** `MUST` Same-package class import is reported.
- **JAVA-IMP-073** `MUST` Javadoc references count as use.
- **JAVA-IMP-074** `SHOULD` Static import used only as an on-demand member (`import static x.Y.*;`
  with one use) is used; with zero uses it is reported.
- **JAVA-IMP-075** `SHOULD` Missorted-imports companion inspection.
- **JAVA-IMP-076** `SHOULD` While the project model is incomplete (importing), unresolved references do
  not cause imports to be reported unused.
- **JAVA-IMP-077** `LATER` Unused import (template) variant.

---

## 5.8 Settings that influence this chain

Settings | Editor | General | Auto Import (Java part), data in `CodeInsightSettings`,
`DaemonCodeAnalyzerSettings`, `JavaProjectCodeInsightSettings` (`codeInsightSettings.xml`),
`JavaIdeCodeInsightSettings`:

| Setting | Default | Effect |
|---|---|---|
| Show import popup (classes) | on | class hint 5.5.4 |
| Show import popup (static methods and fields) | on | static hint 5.4.1 |
| Add unambiguous imports on the fly | off | silent class import |
| Optimize imports on the fly | off | optimize after edits and in fixes |
| Insert imports on paste | ask | (LATER) |
| Do not auto-import pattern | `[a-z].?` | class candidates suppressed |
| Exclude from auto-import and completion | empty | 5.2.2 step 3a, 5.5.2 step 3 |
| Include auto-import of static members in completion | empty (two lists: project, IDE) | 06-completion |

Exclusion entries are class names, member names (`java.util.Objects.requireNonNull`), package names, or
masks with `*`, e.g. `org.mockito.internal.*`. "Excluded packages" also apply to completion.

- **JAVA-IMP-080** `MUST` Honor the exclusion list in static-import, class-import and completion.
- **JAVA-IMP-081** `MUST` Honor `[a-z].?` no-auto-import pattern.
- **JAVA-IMP-082** `SHOULD` Settings page and persistence in project-level file plus IDE-level file.
- **JAVA-IMP-083** `SHOULD` Every default above available as a setting with the stated default.

---

## 5.9 Open questions

- **Q-IMP-1** Exact title of the create-method item (`Create method 'assertThat'` vs with parameter
  types) - bundle template is `Create method ''{0}''` where `{0}` is a signature string computed in
  the quick-fix; confirm against a running IntelliJ before locking.
- **Q-IMP-2** Index iteration order for ties is unspecified; the Archipel tie-break is a deliberate
  deviation (5.3).
- **Q-IMP-3** Whether ties on proximity should consider the *library dependency order* of the module (IntelliJ
  does not).

# 02. Kotlin name resolution as the IDE needs it, imports, auto-import, Optimize Imports

Status: specification, clean-room, our own words. Requirement prefixes `KT-RES` (resolution) and `KT-IMP`
(imports). Executable cases: [cases/02-resolve-and-imports.cases.md](cases/02-resolve-and-imports.cases.md).

ic: `plugins/kotlin/code-insight/fixes-k2/src/org/jetbrains/kotlin/idea/k2/codeinsight/fixes/imprt/*.kt`
(`ImportQuickFix`, `ImportLikeQuickFix`, `ImportQuickFixProvider`, `AbstractImportCandidatesProvider`,
`CallableImportCandidatesProvider`, `ClassifierImportCandidatesProvider`, `ImportPositionType`, `ExpressionImportWeigher`),
`plugins/kotlin/frontend-independent/src/org/jetbrains/kotlin/idea/quickfix/{ImportFixHelper,ImportPrioritizer,AutoImportVariant}.kt`,
`plugins/kotlin/base/util/src/org/jetbrains/kotlin/idea/base/util/ImportableFqNameClassifier.kt`,
`plugins/kotlin/code-insight/kotlin.code-insight.k2/src/org/jetbrains/kotlin/idea/k2/codeinsight/{K2OptimizeImportsFacility.kt,imports/OptimizedImportsBuilder.kt}`,
`plugins/kotlin/code-insight/inspections-k2/src/org/jetbrains/kotlin/idea/codeInsight/inspections/{KotlinUnusedImportInspection,KotlinOptimizeImportsQuickFix}.kt`,
`plugins/kotlin/base/analysis-api/analysis-api-utils/src/org/jetbrains/kotlin/idea/base/psi/imports/{ImportInsertion,KotlinImportPathComparator}.kt`,
`plugins/kotlin/formatter/minimal/src/org/jetbrains/kotlin/idea/core/formatter/{KotlinCodeStyleSettings.java,KotlinPackageEntry.kt,KotlinPackageEntryTable.kt}`,
`plugins/kotlin/code-insight/impl-base/src/org/jetbrains/kotlin/idea/imports/{ImportMapper,KotlinIdeDefaultImportProvider}.kt`,
`plugins/kotlin/code-insight/impl-base/src/org/jetbrains/kotlin/idea/codeInsight/KotlinCodeInsightSettings.kt`,
message bundle `plugins/kotlin/base/resources/resources-en/messages/KotlinBundle.properties`.
Language rules: Kotlin language specification, <https://kotlinlang.org/spec/packages-and-imports.html> and
<https://kotlinlang.org/spec/overload-resolution.html>, <https://kotlinlang.org/spec/scopes-and-identifiers.html>.

The framework side of auto-import (hint pass, silent-change predicate, hint lifecycle, chooser mechanics) is
`codeinsight/05-auto-import.md`; Optimize Imports on save/reformat is `codeinsight/08-optimize-imports-and-reformat.md`.
This file adds what is Kotlin-specific. Where the Java analogue exists see `java/05-imports.md` (differences are
called out).

**Citing the language specification.** Which declaration a name denotes is decided by the Kotlin language
specification (resolution order, overload resolution, most-specific, inference). The IDE must agree with the
Kotlin compiler (K2 frontend); it does not re-decide. This file therefore specifies only (a) the inputs the IDE
gives the resolver (scopes, default imports, resolve scope), (b) the IDE-only features built on it.

---

## 2.1 Inputs to resolution: scopes of a file

For a simple name in code, the Kotlin specification orders the *file-level* scopes, from strongest to weakest:

1. explicit imports (`import a.b.C`, `import a.b.C as D`; also explicit imports of members of objects, enum entries and
   Java statics);
2. declarations of the same package (from every file of the package, in the resolve scope of this file);
3. star imports (`import a.b.*`), all equal in rank;
4. default imports (2.2), all equal in rank, with a *low-priority* subset ranked below the others.

Inside the file, local and member scopes come first (innermost first: locals, parameters, class members with
implicit receivers, outer classes, then file scope). Member callables beat extensions of the same receiver; an extension
that is *shadowed* by a member is reported as a warning (03).

`KT-RES-001` MUST: file-level lookup uses exactly the order above, and a same-rank ambiguity is an ambiguity error
(03, `OVERLOAD_RESOLUTION_AMBIGUITY` / ambiguous reference), not "first wins". Acceptance: cases 02/1..3: an explicit import
beats a same-package class of the same name; a same-package class beats a star import; a star import beats a default
import; two star imports of the same simple name used by that name give an ambiguity error.

`KT-RES-002` MUST: an `import a.B as C` makes `B` **unavailable by its simple name through that directive** (only `C` is
introduced). Acceptance: after `import a.B as C`, a use of
`B` is unresolved unless some other directive brings it in; `C` resolves to `a.B`.

`KT-RES-003` MUST: `import a.Obj.member` imports one member of an `object` (or of a companion, as
`a.Cls.Companion.member`, or of a Java class if static); `import a.Obj.*` is **an error** for Kotlin objects
(`Cannot import on demand from object 'Obj'.`), but allowed for Java classes (imports the static members and nested
classes) and for Kotlin **classes** (imports nested classifiers only). Acceptance: cases 02/4.

## 2.2 Default imports

Every file has implicit star imports, depending on the *platform* of its module:

| Platform | Default imports (all star imports) |
|---|---|
| all | `kotlin.*`, `kotlin.annotation.*`, `kotlin.collections.*`, `kotlin.comparisons.*`, `kotlin.io.*`, `kotlin.ranges.*`, `kotlin.sequences.*`, `kotlin.text.*` |
| JVM (adds) | `java.lang.*`, `kotlin.jvm.*` |
| JS (adds) | `kotlin.js.*` |
| Native (adds) | nothing beyond "all" |
| scripts | the above plus the imports from the script definition |

A *common* (non-platform) source set receives only the "all" row (platform rows removed). A module can exclude
individual names from the default imports (a compiler option; `java.lang.Object`-like exclusions apply for
Kotlin/JVM so `Object`, ... still need an explicit import, see 2.9). `kotlin.*` classes that alias Java classes
(`kotlin.String`, `kotlin.Exception`, `kotlin.collections.ArrayList`, ...) shadow the same-named `java.lang`/`java.util`
classes because of the low-priority rule above: the `java.lang.*` default import is ranked **below** `kotlin.*`.

`KT-RES-010` MUST: default imports per row above, by module platform. Acceptance: in a JVM module `StringBuilder`,
`ArrayList` (= `kotlin.collections.ArrayList`), `Thread` (`java.lang`), `JvmStatic` (`kotlin.jvm`) resolve with no
import; in a common source set `Thread` does not; `String` is `kotlin.String` not `java.lang.String`.

`KT-RES-011` MUST: the *auto-import* machinery treats a candidate whose import is already implied by the default imports
as **already imported** (no import fix is offered, and Optimize Imports never writes it). Acceptance: no `Import` offered
for `println`; after use of `java.lang.Math.max` as `Math.max(...)` no import line is written.

## 2.3 Visibility and the resolve scope

A declaration is a candidate for a file only if it is in the file's *resolve scope* (module, its dependencies,
libraries; test code only for test files, see `java/02-resolve.md` 2.7) and **visible** from the use site. Kotlin
visibility: `public` (default), `internal` (same module, plus modules declared as *friends*: a test source set sees its
main source set's `internal`), `protected` (class and subclasses), `private` (class or file).

`KT-RES-020` MUST: import candidates, completion items and resolve results respect visibility; invisible declarations
are never offered by auto-import. An invisible-but-existing name is reported as `Cannot access 'x': it is private in
'Y'` (03) rather than unresolved. Acceptance: `private fun f()` in another file is not offered by `Import`; `internal`
declarations of the main source set are offered in test files of the same module.

## 2.4 The "Import" quick fix: when it exists

The fix is attached to these compiler diagnostics (03): unresolved reference (also in import directives and in
type, annotation, callable-reference and infix/operator positions), unresolved reference with wrong receiver, invisible
reference, too many arguments, no value passed for parameter, argument type mismatch, named parameter not found,
none applicable (overload), wrong number of type arguments, cannot infer parameter type, no `get`/`set` method,
missing `component<N>()`, missing/ambiguous `iterator()`, `hasNext()`, `next()`, missing delegate accessors
(`getValue`, `setValue`, `provideDelegate`), function expected (call of a non-function, for `invoke`), and
not-an-annotation-class.

So an extension or operator that exists but is not imported shows up through *whatever* error the call produces:
`list.foo()` with an unimported `fun List<Int>.foo()` yields `Unresolved reference 'foo'.`; `a + b` with an
unimported `operator fun A.plus(b: B)` yields the none-applicable/operator-missing error; `for (x in obj)` an
iterator error; `val (a, b) = p` a component error; `by lazyLike` a delegate error. Each has the Import fix when
a candidate exists.

`KT-IMP-001` MUST: the Import fix is registered for every diagnostic listed above whenever at least one candidate exists
(2.5), and is the **first** item of the Alt+Enter list (top priority) in the error group. Acceptance: cases 02/5..9.

## 2.5 Which declarations are candidates

The reference's *position type* decides what is searched; the unresolved *name* (for an operator, the convention
name such as `plus`; for a delegated property `getValue`; for a destructuring entry `component1`) is looked up in
the indexes of file `01`:

| Position | Searched |
|---|---|
| type reference (`val x: Foo`, `List<Foo>`, `as Foo`, `is Foo`) | classes, interfaces, objects, enums, annotation classes, type aliases |
| annotation (`@Foo`) | annotation classes and type aliases that expand to an annotation class |
| plain simple name / call `foo()`, `Foo()` | top-level and object-member functions/properties; classes and typealiases (constructor call or object); enum entries; Java static methods and fields; callables reachable via a *subclass object* (`SubclassObjectName`, 01); extensions applicable to an *implicit* receiver |
| dot call `x.foo()`, safe call `x?.foo()` | extension functions and properties applicable to the receiver type (receiver key + supertypes, then verified), callables from subclass objects applicable to the receiver; type aliases of **inner** classes; **no** plain top-level callables and **no** classes (a classifier is never imported for an explicit receiver) |
| `super.foo()` , other unknown | nothing |
| infix call `a foo b` | only **`infix` extension** functions (Kotlin only; Java methods never qualify) |
| operator or operator-like token (`a + b`, `a[i]`, `a()`) | only **`operator` extension** functions |
| callable reference `::foo`, `x::foo` | callables as for dot call/default call, plus classes that have a constructor (class kinds `class` only, not interface/enum/annotation/object) |
| import directive, package directive | nothing (the import fix is not offered inside imports; unresolved imports get the "add dependency" fix, 2.6) |
| KDoc link | nothing here (the KDoc inspection handles it) |

Eligible declarations, in addition to being in scope and visible (2.3) and **not already imported** (by an explicit
import without alias, or implied by default imports):

- **Kotlin class-likes**: not local; not `inner` unless the position is a type reference.
- **Kotlin callables**: top-level functions/properties; members of a **named non-literal `object`** (including
  `companion object`, imported as `Cls.Companion.member`); extensions if the receiver filter (1.5, 2.9) accepts;
  type aliases always.
- **Java classes**: have a qualified name; top-level, or static nested, or (type position) any nested class.
- **Java members**: `static` methods and fields only, of classes with a qualified name.
- **Enum entries**: for the default-call position; imported as `a.Color.RED`.

After filtering, candidates are de-duplicated by FQ name (the one with the best priority survives).

`KT-IMP-002` MUST: candidate sets per the two tables. Acceptance: cases 02/5..14: in type position only classifiers; in
`x.foo()` only extensions; an `infix fun` appears for `a foo b`, a non-infix extension does not; `private`, local and
already-imported declarations never appear.

`KT-IMP-003` MUST: a name that **matches the platform's "no auto-import" filter** (default pattern `[a-z].?`, a lowercase
letter plus at most one more character, `codeinsight/05-auto-import.md` CI-IMP-004) gets no candidates. Acceptance:
unresolved `ab` and `x` have no import fix; `Ab` does.

`KT-IMP-004` MUST: packages and classes listed in *Exclude from auto-import and completion* (platform setting) are
removed from the candidates and from the chooser. Acceptance: excluding `java.awt` removes `java.awt.List`.

`KT-IMP-005` MUST: a *class filter extension point* may veto classes (by FQ name, kind, modality, visibility, nestedness
and the context file); vetoed classes are not candidates. Acceptance: an extension returning false for `a.Hidden`
hides it from the fix, the chooser and completion.

## 2.6 Presentation: item text, hint, chooser

**Item text** (Alt+Enter row). Computed from the candidates *after* sorting (2.7). Let *kind* be the candidate's kind
(`class`, `object`, `type alias`, `function`, `property`, `extension function`, `extension property`, `infix function`,
`infix extension function`, `operator`, `enum entry`; plurals `classes`, `objects`, `type aliases`, `functions`,
`properties`, `extension functions`, `extension properties`, `infix functions`, `infix extension functions`, `operators`,
`enum entries`) and *name* its display name: the simple name for class-likes and enum entries, and for callables
`Owner.name`, where Owner is the simple name of the receiver type (extensions) or of the containing class/object (object
members, Java statics), and just `name` for top-level callables. Then:

1. Candidates of **more than one kind**: the text is `Import`.
2. One kind, and all distinct FQ names have the **same parent** (same package, or same containing class/object): let *k* be
   the number of distinct display names. `k = 2`: `Import <plural> '<n1>', '<n2>'` (alphabetical). Otherwise
   `Import <kind> '<best name>'` followed by ` and <k-1> more` when *k* > 1 (the kind is plural when *k* > 1).
3. One kind, different parents, kind is a class-like (`class`, `object`, `type alias`): `Import <kind> '<best name>'`.
4. One kind, different parents, callable kinds: group the distinct display names by the text before the first `.` (the owner;
   a name without `.` is its own group). If there are several groups: `Import <kind> '<name of the first group's first
   entry without its owner>'` followed by ` and 1 more` when that group has more than one entry. If there is one group of
   exactly two entries: `Import <plural> '<n1>', '<n2>'` (full names). Otherwise `Import <kind> '<first entry>'` followed by
   ` and 1 more` when the group has more than one entry (the number in this last phrase is always `1`).

*Best* = first in the order of 2.7.

**Hint** (the small tooltip above the reference, shown by the daemon pass of `codeinsight/05`): `Import <fqName of best
candidate>?` with the platform's `Alt+Enter` shortcut hint. Shown only for the nearest reference with candidates;
*silent import* (setting `Add unambiguous imports on the fly`, default **off**, app level) applies only when **exactly one
non-deprecated** candidate exists (or exactly one if all are deprecated) and that candidate is not "restricted"
(nested classes are never imported on the fly because the result would add qualification; callables are not imported on
the fly when an implicit receiver has an error type).

**Chooser** (several candidates): popup titled `Imports`; one row per distinct FQ name (best-ranked variant of each), in
the order of 2.7. Each row is drawn by the platform's PSI element renderer for the declaration (icon, name, container
text such as the package, file or module on the right). The *text used for speed search and accessibility* is a rendering of
the declaration with its visibility modifier and fully qualified names: a class `class a.b.Foo`; a callable
`public fun z.compute(n: kotlin.Int, s: kotlin.String): kotlin.Unit` followed, for top-level callables, by
` defined in <package>` and ` in file <File.kt>`. Speed search is on; the list does **not** auto-select a single item;
Enter chooses; **Right arrow** opens a one-step submenu with `Exclude '<x>' from auto-import` entries for the FQ name and
each of its parent prefixes while the prefix still has two or more segments (`java.util.List` -> `java.util.List`, `java.util`;
`a.b.c.D` -> `a.b.c.D`, `a.b.c`, `a.b`), choosing adds an exclusion (`KT-IMP-004`).

`KT-IMP-006` MUST: item text rules above (kinds, plural, "and n more", fallback `Import`). Acceptance: cases 02/5, 6, 10, 11.

`KT-IMP-007` MUST: hint text `Import <fq>?`, shown for the best candidate, silent import restricted as stated. Acceptance:
cases 02/12, 13.

`KT-IMP-008` MUST: chooser title `Imports`, rows, speed search, Right-arrow exclude submenu with the exact `Exclude '<x>' from
auto-import` strings. Acceptance: case 02/6.

## 2.7 Order of candidates

Candidates are ordered by a priority comparison applied in this order (first difference decides):

1. **deprecated last**;
2. **expression weight, higher first**: a base weight by origin (project source code **7**; `kotlin.*` **6**; `kotlinx.*` **5**;
   `java.*` **2**; any other library **0**) added to a *fit* weight that measures how well the candidate suits the call:
   for a function call, if the candidate is an extension, how well the actual receiver type(s) (explicit or implicit)
   is a subtype of the candidate's receiver (adds 12 if nullability agrees else 10, and 0 if no match); then +1 if the
   parameter count equals the argument count (or the candidate has a `vararg`), +0 if it has more parameters, and
   if it has **fewer** parameters than arguments no further weight; then for each argument left to right that is
   a subtype of the corresponding parameter (nullability-adjusted) the weight is multiplied by 100 and 10 or 12 added;
   stops at the first mismatch. A class candidate used as a constructor call is weighed by its best constructor. For an
   operator token the weight adds 8 when the candidate's name equals the operator's convention name, and receiver and
   right-operand fits as above. Other positions weigh 0 plus the base weight;
3. **classification, better first**: declared in the file's current package; top-level (root-package) names; imported
   by an explicit import already in the file; implied by default imports; imported through a star import of that
   package; **another name from the same package is already explicitly imported** (a "sibling"); not imported; Java
   classes that must not be used from Kotlin (such as `java.lang.Integer`) last;
4. **recently chosen first** (usage statistics of earlier choices, shared with completion);
5. **proximity, nearer first** (same module, same package/directory...; `platform/07-project-structure-and-scopes.md`);
6. **FQ name alphabetically**.

Different from Java (`java/05-imports.md`): there is no "one member per class" reduction; every distinct FQ name is a row.

`KT-IMP-010` MUST: the ordering above. Acceptance: cases 02/6, 10, 14.

## 2.8 What the fix does (the edit)

Choosing a candidate runs as one undoable command named `Add Import`.

- **Class-like candidates** (class, interface, object, enum, typealias, enum entry as class-like): the reference is
  **shortened** to its simple name and an import is added when needed; if the unresolved text was a partially qualified or
  nested name the result may keep the outer qualifier (an import for the outermost needed class is added, e.g.
  `Entry` -> `Map.Entry` unless the code-style option *Insert imports for nested classes* is on, then
  `import kotlin.collections.Map.Entry`). For declarations in the **root package** (one-segment name) no import is written;
  nothing can be imported from the root package. For callable references (`::foo`) the reference cannot be qualified,
  so an import is added.
- **Callables**: an import line `import a.b.name` is added (for members of an object: `import a.Obj.name`; for a
  companion `a.Cls.Companion.name`; for Java statics `a.b.Cls.name`).
- **Position of the new line** and format: see 2.11.
- **Usage statistics**: the choice is recorded for ordering (2.7 step 4).

`KT-IMP-011` MUST: the edit rules above; undo restores the file exactly. Acceptance: cases 02/5..9, 15.

## 2.9 Extension receiver applicability

An extension `fun R.f(...)` applies to a use `x.f(...)` when the static type of `x` is a subtype of `R` after
substituting type parameters and respecting nullability: `fun String.f()` does not apply to `String?` (error: "Only
safe (?.) or non-null asserted (!!.) calls are allowed on a nullable receiver of type 'String?'", see 03); `fun String?.f()`
applies to both. Unresolved receivers (error type) yield no extension
candidates for explicit-receiver positions.

`KT-RES-030` MUST: extension candidates are the indexed extensions for the receiver type and its supertypes (01 KT-STUB-022),
filtered by real applicability including nullability and generic bounds. Acceptance: case 02/9, 16.

## 2.10 Unused imports and Optimize Imports

### 2.10.1 The inspection

Inspection `KotlinUnusedImport`, display name `Unused import directive`, default WARNING, on by default, shown as
**gray text** (unused-symbol style, no wavy line) on the whole directive. Applies to project source files and scripts, not to
code fragments. Fixes in this order:

1. `Remove unused imports` (removes all unused imports of the file, no reordering);
2. `Optimize imports` (runs Optimize Imports on the file);
3. if the file's project does not have it on: `Enable optimize imports on the fly` (low priority; turns the project
   setting on and optimizes once).

An import is **used** if some reference in the file resolves to it, or the file contains an unresolved name that the
import could resolve (kept for safety, so imports are not removed while a name is unresolved and the project is
unresolved), with these rules: duplicates (same import twice) mark the second unused; a star import is used if any
declaration found through it is referenced or if there are unresolved names; an import is **unused** if every reference
that mentions its name resolves to something else with higher priority (for example a same-package class) and the import
adds nothing; an alias import is used iff the alias is used. (An explicit import of a name that default imports already provide
is reported as unused by Optimize Imports because step 5 of 2.10.2 drops it; see Q-IMP-4.)

`KT-IMP-020` MUST: the inspection, texts and fixes above. Acceptance: cases 02/17, 18.

### 2.10.2 Optimize Imports (Code | Optimize Imports; also run on save/reformat when configured)

Algorithm in prose:

1. Collect all references in the file and what each resolves to (declaration + the name it was written with, which can be an alias).
2. Start from the set of imports that resolve something that is **unresolved** (kept as-is) and duplicate-free.
3. Group the needed *importable names* (class-likes, top-level and object-member callables, Java statics, enum entries) by
   their parent package or class.
4. For each parent decide **explicit or star**:
   - never star for an alias; never star from the root package; never star from an `object`;
   - **star** if the number of distinct used names from that parent is at least the threshold
     (`5` for packages and top-level members; `3` for members of classes/objects/Java statics), **or** the parent package is in
     *Packages to Use Import with '*'* (defaults: `java.util` exactly, `kotlinx.android.synthetic` with subpackages,
     `io.ktor` with subpackages) and at least one name is used;
   - otherwise one explicit import per name.
5. Names that are implied by default imports are dropped (no line), also for names mapped from Java to Kotlin (`java.lang.Exception`
   -> `kotlin.Exception`, `java.util.ArrayList` -> `kotlin.collections.ArrayList`, `java.lang.StringBuilder` ->
   `kotlin.text.StringBuilder`, `java.lang.Error`, `RuntimeException`, `IllegalArgumentException`, `IllegalStateException`,
   `IndexOutOfBoundsException`, `UnsupportedOperationException`, `ArithmeticException`, `NumberFormatException`,
   `NullPointerException`, `ClassCastException`, `AssertionError`, ... , `java.util.concurrent.CancellationException`
   -> `kotlin.coroutines.cancellation`, `kotlin.jvm.Throws` -> `kotlin.Throws`, and similar).
6. After choosing, **verify** by re-resolving every reference with the new import set: if any reference would now resolve
   to a different declaration (a star import brought a same-named class that wins), **lock** the original explicit import
   (keep it) or forbid that star import, and repeat until stable.
7. **Sort** by the import layout (2.10.3): each import is put in the group of its best-matching layout entry; inside a group
   lexicographically by the import's text **ignoring backticks**; aliased imports follow the layout rule for aliases.
8. Write the list: existing import nodes are replaced one by one (new ones added, old ones deleted) so that fold state
   survives; if the new list equals the old list nothing changes.

Result notification (a short editor hint after the action): `Removed N imports` / `Removed N import` (N = 1), with
`, added M import(s)` when new lines were written, `Rearranged imports` when only the order changed, `Unused imports not
found` when nothing changed. (Count noun: `import` for fewer than 2, `imports` otherwise.)

`KT-IMP-021` MUST: steps 1 to 8. Acceptance: cases 02/19..27.

### 2.10.3 The layout

Setting `Settings | Editor | Code Style | Kotlin | Imports`: *Packages to Use Import with '*'* (list: `Package`,
`With Subpackages` columns, buttons `Add`, `Remove`), *Use import with '*' when at least* `5` *names used*, *Use import with '*'
when at least* `3` *members used* (shown as `Use single name import` / `Use import with '*'` radio plus count), checkbox *Insert
imports for nested classes* (default **off**), and *Import Layout* (list with `Add Package`, `Remove`, `Up`, `Down`; the two
special entries are shown as `all other imports` and `all alias imports`; checkbox `Import aliases separately`).

Defaults of the layout, in order: `<all other imports>`, `java.*`, `javax.*`, `kotlin.*`, `<all other alias imports>`.
With *Import aliases separately* on (default; the alias entry is present) alias imports sort last as one group; without that
entry they sort with the others. The matching of an import to a layout entry: the matching entry that is *not* "all other"
wins; between two matching entries a **without-subpackages** entry beats a with-subpackages one, then the entry with the
**more segments** wins; entries are compared by their index. There are **no blank lines** between groups by default.
Within the same group imports are sorted by plain string order of their text (with backticks removed), so `a.B` < `a.b`
(upper case before lower case, code point order).

`KT-IMP-022` MUST: defaults above; the matching rule; sorting by text without backticks. Acceptance: cases 02/19, 20.

### 2.10.4 On-the-fly optimize

Setting `Optimize imports on the fly` (project level, **off** by default; same flag as in Settings | Editor | General | Auto
Import): after each full daemon cycle, when the caret is **not** within the import block and its surrounding blank lines,
the file has **no errors outside the import block**, highlighting finished, the file is writable, no undo/redo is in
progress, and the "silently changeable" predicate (`codeinsight/05` section 3) holds, the optimized list is applied as an
undo-transparent change.

`KT-IMP-023` SHOULD: as stated. Acceptance: case 02/28.

## 2.11 Adding an import (the primitive used by fixes, completion, refactorings, paste)

Given an import path (fqName, `.*` flag, optional alias):

- If the file has **no imports** yet: insert after the package directive; ensure exactly **one blank line** between the package
  line and the import (so two line breaks); if the file has no package directive, insert at the top and ensure one blank line
  before the first declaration.
- Otherwise: compare with the existing imports using the layout comparator of 2.10.3; insert **after the last existing import
  that compares less than or equal**; if that import is the same path, do nothing (return the existing one); new line break
  before it. A new import therefore goes into the right group and the right alphabetical place even if the existing list is
  not sorted (it lands after the last element not greater than it).
- Adding **never** collapses to a star import and never removes imports; collapsing happens only in Optimize Imports.
- In a code fragment (debugger, etc.) the import text is attached to the fragment's own import list.

`KT-IMP-030` MUST: the rules above. Acceptance: cases 02/5, 15, 21.

## 2.12 Import on paste

When Kotlin code is pasted (copy and paste inside the IDE) the references of the pasted text are resolved in the source file;
those that do not resolve the same way in the target get imports added (setting `Insert imports on paste`: `Always`/`Ask`/
`Never`, default `Always`, `codeinsight/05` section 10). After `Always` a notification says `<n> import was added` /
`<n> imports were added` with link `Review Added Imports...` opening a dialog `Select Imports to Remove` ("The code fragment which
you have pasted uses elements that are not accessible in the new context. Necessary imports have been added. Select which of
the added imports you want to remove from the file.") with a checkbox list; confirming removes the checked imports.
`KT-IMP-040` SHOULD: as stated. Acceptance: case 02/29.

## 2.13 Settings and inventory items covered here

| Item | Where |
|---|---|
| Inspection `KotlinUnusedImport` | 2.10.1 (`KT-IMP-020`) |
| Action `ReplaceWithImportAlias` fix and inspection `ReplaceWithImportAlias` | 05 (inspections) and 06 (intention `IntroduceImportAlias`) |
| Intentions `ImportMemberIntention`, `ImportAllMembersIntention`, `IntroduceImportAliasIntention` | 06 |
| Completion's auto-import | 04 |
| Settings `Add unambiguous imports on the fly`, `Optimize imports on the fly` | 2.6, 2.10.4 |
| Settings page `Kotlin` (Code Style) imports panel | 2.10.3 |

## Open questions

- Q-IMP-1: the exact edit for nested classes with *Insert imports for nested classes* off (partially qualified vs. import of
  the outer class) for each nesting depth; implement as described and confirm in differential tests.
- Q-IMP-2: the `ExpressionImportWeigher` extension point (Compose-specific weighing) is not specified; Archipel ships no
  weigher.
- Q-IMP-4: whether the *inspection* (gray text) and Optimize Imports agree for explicit imports of default-imported names;
  implement Optimize Imports as specified and report such imports as unused.
- Q-IMP-3: ties in step 6 of 2.7 are alphabetical here; IntelliJ's proximity weights are platform-defined (see
  `platform/07`).

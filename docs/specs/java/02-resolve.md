# 02. Resolve

Status: specification, clean-room. Written from the IntelliJ IDEA Community sources in our own words, with
the Java Language Specification (JLS, SE 21 numbering) as the semantic authority. Source paths are cited as
`ic: <path>`.

Primary sources: `ic: java/java-psi-impl/src/com/intellij/psi/impl/source/PsiJavaFileBaseImpl.java` (file-level
scope), `.../psi/scope/util/PsiScopesUtil.java` (the upward walk), `.../psi/impl/source/resolve/ClassResolverProcessor.java`,
`VariableResolverProcessor.java`, `.../psi/scope/processor/MethodResolverProcessor.java`,
`.../psi/scope/conflictResolvers/JavaMethodsConflictResolver.java`, `ic: java/java-psi-api/.../PsiClassImplUtil.java`
(members of a class), `ic: java/java-psi-impl/.../impl/compiled/` (class-file stubs),
`ic: platform/analysis-impl/.../ResolveScopeManagerImpl.java` and
`ic: java/java-analysis-impl/.../JavaResolveScopeProvider.java` (resolve scope).

The Archipel project model (`docs/architecture/project-model.md`) provides modules, content and source
roots, libraries, SDKs, dependency scopes and `GlobalSearchScope`; this file uses those terms.

---

## 2.1 Concepts

- **Reference.** A syntax element that names a declaration: a class/package reference, a reference
  expression (`a.b`, `x`), a method call's method expression, a constructor call, a method reference
  (`A::m`), an import reference, a javadoc reference, an annotation attribute name.
- **Resolve result.** A target declaration (or nothing), a *substitutor* (type-variable bindings), whether
  the target is *accessible* from the place, whether the *static context* is correct, and the *file-level
  context* through which it was found (which import statement, or which enclosing class). Several results
  may be returned for an ambiguous reference ("multi-resolve").
- **Place.** The reference itself; visibility, static-ness, and scoping are all judged from the place.
- **Scope processor.** A visitor that is offered declarations one by one, with a name hint (only this name),
  a kind hint (class, field/variable, method, package), and a "keep going / stop" answer. Declarations are
  offered in *walk order* (2.3). Whether the walk stops at the first match or collects all candidates is
  decided by the processor kind.
- **Two entry points.** *Resolve* (given the reference, find the target(s)) and *variants* (enumerate all
  declarations visible at the place: completion, 06). The same walk serves both.

- **JAVA-RES-001** `MUST` A single scope-walk engine serves resolve and variants. *Accept:* completion
  of `Str` at a place offers exactly the classes that `Str...` resolution would find.
- **JAVA-RES-002** `MUST` Resolve results carry target, substitutor, accessibility, static-context
  correctness and file-level context (import statement or class scope).
- **JAVA-RES-003** `MUST` No panics if the project model is empty or the SDK is missing; unresolved is a
  valid result.

---

## 2.2 Classifying names (JLS 6.5)

A qualified name `a.b.c` is classified left to right:

1. The syntactic position decides the *kind* a reference may have: a type position (after `new`, in
   `extends`, in a cast, in a declaration type) expects a **type name**; an expression position expects
   an **expression name** whose leftmost part may also be a type or a package; the qualifier of a type name
   is a **package or type name**; a method name in a call is a **method name**.
2. For an ambiguous leftmost simple name `a` in an expression context: if a local variable, parameter or
   field named `a` is in scope it is an **expression name** (obscuring: variables obscure types, types
   obscure packages, JLS 6.4.2); else if a type named `a` is in scope it is a **type name**; else a
   **package name**.
3. The next segment is resolved relative to what the previous one resolved to: a member of the package
   (class or subpackage), a member of the type (static field, member type, static method), a member of the
   value's type (field, method). A segment that resolves to nothing makes the whole reference unresolved
   at that segment; only that segment gets the error.
4. A fully qualified reference to a class (`java.util.List`) resolves by package + class name lookup
   (2.6), not through scopes; it is *not* subject to shadowing, except that a variable named `java` would
   obscure the package in expression context.

- **JAVA-RES-004** `MUST` Name classification per steps 1-4. *Accept:* in
  `class A { int java; void f() { java.util.List x; } }` the declaration type `java.util.List` resolves
  (type position), while `java.util.List.of()` in an expression position inside `f` fails at `util`
  (variable `java` obscures the package, JLS 6.4.2).
- **JAVA-RES-005** `MUST` Errors are anchored on the first unresolved segment only (04-error-catalog 012).

---

## 2.3 The walk: order of scopes

The processor is offered declarations in this order. The walk starts at the reference and goes to the
parent repeatedly, each syntactic ancestor offering what it declares *as seen from the child through
which the walk arrived* (the "last parent" rule below). The walk ends when a processor says "found" or the
file's parent is passed.

### 2.3.1 Scopes inside a method or initializer (innermost first)

For **expression names (variables)** and **local classes**:

1. The block statements *before* the reference in each enclosing block, from nearest to farthest:
   local variable declarations (scope starts at the declarator's initializer end... precisely: JLS 6.3,
   the scope of a local starts at its declarator, so `int x = x + 1;` sees the new `x` in its own
   initializer), local class declarations, and *pattern variables* (flow scoping, 2.3.3).
2. `for` header variables (scope: header, condition, update, body), the enhanced-`for` variable (scope: body
   only), `try`-with-resources variables (scope: remaining resources and the try block), `catch`
   parameters (catch block), lambda parameters (lambda body), method and constructor parameters (whole
   body), local-class type parameters, method type parameters.
3. Named local classes and local records/enums/interfaces declared before the reference in the same or
   enclosing blocks.

A declaration is *not visible* above (before) its own position in a block; a later sibling statement's
declarations are never offered. A lambda body sees the lambda parameters and the enclosing scope; an
anonymous or local class body switches to class scope (2.3.2) and then continues to the enclosing method
scope for locals (those are captured).

### 2.3.2 Class scope

When the walk reaches a class (anonymous, local, member, or top-level), the class offers, for the requested
kind:

- **Fields**: the class's own fields first; then inherited fields, searching superclass chain and
  superinterfaces. A field declared in a class **hides** same-named fields of supertypes (JLS 8.3).
  A name inherited by two paths from different declarations is ambiguous (reference error 013).
- **Member types** (nested classes): own first, then inherited; own hides inherited. A class inherited
  through two supertypes with the same member-type name is ambiguous when referenced by simple name
  (JLS 8.5).
- **Type parameters** of the class (when the walk arrives from a non-static part, see static context).
- **Methods**: own and inherited *as a set of overloads*, with overriding applied (an overridden method is
  not a separate candidate). See 2.4 for the method-specific stop rule.
- The class's own **name** as a type is visible inside itself (and, via the enclosing scope, its siblings).

Inherited means "members of the superclass chain and all superinterfaces, transitively, each class visited
once, with type arguments substituted along the path" (substitutor per supertype). Private members of
supertypes are not inherited but may be found by the walk and rejected by accessibility, so the error is
"has private access" rather than "cannot resolve" (04: 014).

After the class, the walk continues to the **lexically enclosing** scope: for a member class its outer
class (which again offers fields, member types, methods incl. inherited ones), for a local/anonymous class
the enclosing method scope then the enclosing class, and so on to the file.

**Static context.** Entering a `static` method, `static` initializer, static field initializer, a
`static` nested class, or an explicit constructor-invocation argument list (`this(..)`, `super(..)`)
switches the walk into *static scope*: instance members found from there are flagged "static context
incorrect" (error 018) but remain in the result so the error is precise.

### 2.3.3 Pattern variables (JLS 6.3.1, flow scoping)

A pattern variable introduced by `x instanceof T v` is in scope exactly where JLS 6.3.1 says it is
definitely matched: the true branch of `if`, the right operand of `&&` (and `||` for the negated form),
the true/false arms of `? :`, the statements following an `if` whose then-branch cannot complete normally
when the negated pattern would introduce it, `while` conditions and bodies, switch case labels and guards
(`case T v when v.x > 0`), and nowhere else. The walk therefore has to consult the *condition expression
before the reference* for introduced bindings, with the polarity (true/false) of the position.

- **JAVA-RES-006** `MUST` Local/parameter walk order of 2.3.1 with visibility by position.
- **JAVA-RES-007** `MUST` Class scope order 2.3.2: own members hide inherited; inherited from
  superclass and superinterfaces with substitution; outer classes after.
- **JAVA-RES-008** `MUST` Static context flagging (018 instead of "cannot resolve").
- **JAVA-RES-009** `SHOULD` Pattern variable flow scoping.
- **JAVA-RES-010** `MUST` A member-type name found twice through different supertypes is ambiguous
  (013), a field/type declared in the class hides inherited ones.

### 2.3.4 Acceptance scenarios

```java
class P { int v = 1; }
class C extends P {
    int v = 2;                    // hides P.v
    void m(int v) {                // parameter shadows field
        int w = v;                 // -> parameter v
        { int z = this.v; }        // -> C.v (explicit qualifier)
        Runnable r = () -> { int v2 = v; }; // -> parameter v
    }
}
```
`super.v` resolves to `P.v`; an unqualified `v` in a method with no parameter named `v` resolves to `C.v`.

```java
class Outer {
    int a;
    class Inner extends Base {     // Base declares int a
        int f() { return a; }      // -> Base.a (inherited beats lexically enclosing: JLS 6.4.1)
    }
}
```
The inherited member found at the innermost class **wins over** the enclosing class member (the
member of the inner class scope is found first and the walk stops).

---

## 2.4 Methods: candidate collection and overload resolution

### 2.4.1 Which scope level provides candidates (JLS 15.12.1)

For an unqualified method call `m(args)` the walk visits scopes as in 2.3, and **stops at the first
(innermost) class scope that has at least one *accessible* member method named `m` whose static context is
correct** - candidates from that level only are considered; outer classes are not consulted even if their
`m` would be applicable. Static imports (single, then on-demand; 2.5) are consulted only if no class level
supplied such a candidate. For a qualified call `q.m(args)`: the members of the type of `q` (class: static
and instance by qualifier kind; expression: all) and its supertypes. For `super.m()`: members of the
superclass (and for `I.super.m()` the named superinterface). For constructors the candidate set is the
constructors of exactly the named class (never inherited).

If the level has candidates that are all inaccessible or in a wrong static context, they still are
reported (so that the error says "has private access" or "non-static ... static context") instead of
continuing outward, unless an outer level supplies a fully valid candidate; the exact preference is:
valid (accessible, static-correct, applicable) candidates win over invalid ones from outer levels only when
the inner level has no accessible static-correct candidate.

### 2.4.2 Overload resolution (JLS 15.12.2)

Given the candidate set `C` (after removing overridden methods, 2.4.3):

1. **Arity filter (potentially applicable, 15.12.2.1).** A method is potentially applicable if its arity
   matches (`n` parameters = `n` arguments), or it is variable-arity with `n-1 <= args`. Additionally, for
   lambda arguments (JLS 15.12.2.1) the target parameter type must be a functional interface (or a type
   variable of the method) with a compatible parameter count (and for block-bodied lambdas, void/value
   compatibility); for method-reference arguments the target must be a functional interface.
   If at least one method matches arity, methods that do not are dropped before applicability.
2. **Applicability phases (15.12.2.2-15.12.2.4).** Phase 1: applicable by *strict invocation* (no boxing, no
   varargs); Phase 2: *loose invocation* (boxing/unboxing allowed); Phase 3: *variable-arity invocation*.
   The first phase in which any method is applicable defines the set; later phases are not tried.
   IntelliJ computes, per candidate, an applicability *level* of {not applicable, varargs, fixed-arity} and
   keeps only the candidates with the highest level among `C` (fixed arity beats varargs); strict-versus-loose
   is resolved during the most-specific selection by preferring candidates that needed no boxing for the
   given arguments (same effect as JLS phases 1 and 2).
   Applicability for generic methods uses inference (JLS 18.5.1, `03-types.md`): the arguments *pertinent
   to applicability* (not implicitly-typed lambdas, inexact method references, and for generic methods
   not poly expressions with variable-dependent targets) take part; others are checked after selection.
3. **Most specific (15.12.2.5).** Among the applicable candidates, method `m1` is more specific than `m2`
   if `m2` is applicable to `m1`'s parameter types (for generic `m2` with inference, 18.5.4; for functional
   interface parameters with explicit lambdas, the extra rule on return types). Candidates that are
   more specific than all others win; if several are equally specific:
   - if all have *override-equivalent* signatures (same erasure/subsignature), prefer the concrete one,
     otherwise an abstract one chosen arbitrarily (JLS 15.12.2.5 last paragraph);
   - else the call is **ambiguous** (error 010).
4. **Static-ness and access are not part of applicability**: an inaccessible or wrong-context method is
   not discarded early; it participates, and if chosen produces 014-018. But accessible candidates are
   preferred over inaccessible ones **before** specificity, so an inaccessible overload never causes an
   ambiguity with accessible ones. Methods of an interface that are `static` and reached through a
   subtype or an instance are removed from the set when other candidates exist (they are not inherited,
   JLS 8.4.8) - and if nothing else exists the error is "Static method may only be called on its containing
   interface" (031).
5. After a winner, *invocation type inference* (JLS 18.5.2) computes the substitutor and return type,
   using the target type of the call when it is a poly expression.

### 2.4.3 Removing overridden and duplicate candidates

Before the above, candidates are reduced: a method overridden by another candidate in a subtype is dropped;
candidates with the same signature from different supertypes: interface method over `Object` member; a
method from an inner/outer class that is inaccessible loses to a method from an outer class with the same
signature; a super method with varargs does not beat a non-varargs override.

- **JAVA-RES-011** `MUST` Innermost-class-level rule for unqualified calls (2.4.1). *Accept:*
  ```java
  class O { void f(int a) {} class I { void f(String s) {} void g() { f(1); } } }
  ```
  reports an error in `g` (inner `f(String)` is the only level considered), not a call to `O.f`.
- **JAVA-RES-012** `MUST` Arity filter, then phases strict/loose/varargs as in 2.4.2.
  *Accept:* `void f(long a){}  void f(Integer a){}  f(1)` resolves to `f(long)` (strict beats boxing);
  `void g(Object o){} void g(int... a){} g(1)` resolves to `g(Object)`.
- **JAVA-RES-013** `MUST` Most-specific selection with ambiguity detection. *Accept:*
  `void h(Integer a, Object b){} void h(Object a, Integer b){} h(1, 1)` is ambiguous (010).
- **JAVA-RES-014** `MUST` Lambda/method-reference pertinence rules and arity-of-lambda filtering in the
  potentially-applicable step.
- **JAVA-RES-015** `MUST` Override removal and interface-over-Object preference.
- **JAVA-RES-016** `SHOULD` Accessible candidates beat inaccessible ones before specificity.
- **JAVA-RES-017** `SHOULD` Static interface method inheritance rule.

---

## 2.5 File-level scope: classes, imports, package, `java.lang`, modules

When the walk leaves the last class and reaches the compilation unit, declarations are offered in this
order. This order realizes the shadowing rules of JLS 6.4.1 and 7.5.

**Stage A - explicitly named, by name (shadow everything below):**
1. top-level classes declared in this file (by name; the implicit class of a compact source file is skipped);
2. single-type-import declarations (`import a.b.C;`);
3. single-static-import declarations (`import static a.b.C.m;`), where the imported name can denote a
   field, method or member type of `C` (all kinds of that name).
(Implicit static imports, if the language level adds any, precede the explicit ones.)

**Stage B - package and on-demand sources (only reached if stage A found nothing accessible):**
4. the current **package** (all classes of the file's package visible in the resolve scope; for a package
   that exists in several modules/roots all contribute);
5. **static on-demand** imports (`import static a.b.C.*;`): the static members of `C`, with the
   requirement that a member whose name is *also* explicitly declared in stage A (a single import or a
   file class) is not offered here;
6. **type on-demand** imports (`import a.b.*;`, including `import a.b.C.*;` that exposes member classes);
7. **module imports** (`import module m;`, Java 25): the exported packages of module `m` and its transitive
   `requires transitive` closure;
8. **implicit imports**: `java.lang` (always; filtered by language level so that classes added to
   `java.lang` after the file's level, such as `Record`, are not visible below that level) and, for compact
   source files, the implicitly imported classes.

**How the walk behaves.** As soon as an *accessible* class is found in stage A, in a class scope, or in the
current package (step 4), the walk **stops**: it is the answer. Only on-demand sources (steps 6, 7, 8 and
the type-on-demand part of 5) *keep collecting*, so that conflicts can be detected. Among the collected
candidates the following ranking applies, pairwise, and the loser is dropped: (1) the same FQN twice: one;
(2) a member class found through a more specific inheriting enclosing class shadows one from a less
specific class (JLS 8.5, except when both are inherited through unrelated interfaces: ambiguous);
(3) accessible beats inaccessible; (4) a class of the default (unnamed) package loses to any other;
(5) a single-type import beats an on-demand source; (6) an on-demand import (including the implicit
`java.lang` and the same-package pseudo-import) beats a module import (language levels with module
imports). What remains and is not the same declaration is **ambiguous** (error 013).

`java.lang` is therefore *an on-demand import* (JLS 7.5.2): a name supplied by `import foo.*;` and also
by `java.lang` is ambiguous, whereas a class in the current package or a single-type import hides it.
The same package hides every on-demand import because the walk stops at step 4.

Single-type imports are *shadowed* by a file class of the same name (error `'{0}' is already defined in
this compilation unit`), and two single-type imports of different classes with the same simple name conflict.

An import that fails to resolve still reserves its simple name: references to that name do not fall
through to later stages (this prevents wrong silent matches while the user is fixing the import).

Imports are resolved lazily and cached per file; a reference's *file-level context* records which import
statement supplied the target, which drives unused-import detection (05 section 5.7).

- **JAVA-RES-018** `MUST` Stage order A1,A2,A3 then B4..B8 exactly. *Accept:*
  ```java
  package p;                      // p contains class List
  import java.util.*;
  class T { List l; }             // -> p.List (package beats on-demand import)
  ```
  and with `import java.util.List;` added the same reference resolves to `java.util.List` (single import
  beats package).
- **JAVA-RES-019** `MUST` Ambiguity when two on-demand imports supply the same name: `import java.util.*;
  import java.awt.*; List x;` reports `Reference to 'List' is ambiguous, both 'java.awt.List' and
  'java.util.List' match`.
- **JAVA-RES-020** `MUST` `java.lang` is an implicit on-demand import: hidden by file classes, single imports and the same package; ambiguous with another on-demand import that supplies the same simple name. *Accept:* `import foo.*;` where `foo.Object` exists, then `Object o;` is ambiguous.
- **JAVA-RES-021** `MUST` Static single and static on-demand imports for fields, methods and member
  types; an explicitly named import hides on-demand static members of the same name.
- **JAVA-RES-022** `SHOULD` Language-level filter on `java.lang` classes.
- **JAVA-RES-023** `SHOULD` Unresolved single imports reserve the simple name.
- **JAVA-RES-024** `LATER` Module imports.
- **JAVA-RES-025** `MUST` Implicit classes of compact source files are not importable and not visible to
  other files.

---

## 2.6 Qualified names, packages and fully qualified lookup

- A fully qualified class name `p.q.C` is looked up by the **FQN index** (all classes whose FQN equals the
  name) within the resolve scope; among several results (the same FQN in a source file and a jar, or in two
  jars) the first in the scope's *search order* wins: module source roots, then the module's dependencies in
  declaration order (module then library entries), then SDK. A source copy beats a class-file copy of the
  same FQN in the same dependency.
- Nested classes: `p.C.D` is `D` nested in `p.C`; binary names with `$` appear only in class files.
- A package is "known" if some class/directory exists under it in the resolve scope; a package reference
  `a.b` whose parts are not classes resolves to the package even if empty (so that `a.b.C` can then resolve);
  an unknown package prefix yields `Cannot resolve symbol` at the first unknown segment (for imports
  the message is `Cannot resolve symbol 'x'` on that segment as well).
- Case: lookups are case-sensitive.

- **JAVA-RES-026** `MUST` FQN lookup by FQN index with search-order tie-break.
- **JAVA-RES-027** `MUST` Package existence derived from source directories (using package prefix of the
  source root) and library/SDK class packages.
- **JAVA-RES-028** `SHOULD` Multiple definitions of one FQN across the scope: first wins; a duplicate-class
  error is *not* reported across libraries.

---

## 2.7 Resolve scope of a file (which classes can be seen at all)

The resolve scope of a Java file decides which indexed declarations are candidates for every lookup in
2.5 and 2.6 and for the import fixes in 05:

1. A Java file **in a module's source root** (`ProjectFileIndex` says in source, in module M):
   *module M with its dependencies and libraries*, where **tests are included iff the file is a test source**:
   - production file: M's production sources, the production sources of modules M depends on (only
     exported/visible entries and only those with a scope that is on the compile classpath), libraries
     with scope Compile/Provided, and the SDK;
   - test file: additionally M's test sources, test sources of dependencies where the dependency entry
     allows it (the project-model rule: a dependency's *tests are never on someone else's classpath*, so
     test sources of other modules are visible only for the module's own test code through the module's own
     test source set), libraries with scope Test/Runtime.
   Plus the language-level / multi-release filter: entries of a multi-release jar's `META-INF/versions/N`
   are visible only if `N <=` the module's language level, and the highest such version wins.
   Plus the *module-system* filter: for a named Java module (`module-info.java` present) only packages the
   module reads and the target exports are visible; for the unnamed module everything on the classpath.
2. A Java file **under module content but outside any source root** (for example test data, build scripts
   treated as sources): the **file itself only** (so almost everything is unresolved) - a deliberate
   behavior to avoid false resolves.
3. A file **in a library** (class file or library source): the scope of that library (its own classes plus
   its dependencies, per library scope cache).
4. A **non-physical** file (scratch buffer, fragment, copy): all project scope plus the file.
5. Extension points can widen the scope (resolve-scope enlargers) or replace it (providers) per file.

- **JAVA-RES-029** `MUST` Resolve scope 1: module + dependencies + libraries + SDK, tests included only for
  test-source files. *Accept:* `org.junit.Assert` resolves in `src/test/java/..` and not in
  `src/main/java/..` when JUnit is `testImplementation`.
- **JAVA-RES-030** `MUST` Files outside source roots resolve only to themselves.
- **JAVA-RES-031** `SHOULD` Multi-release jar version filtering by language level.
- **JAVA-RES-032** `SHOULD` Java module system filtering (`requires`, `exports`) for named modules.
- **JAVA-RES-033** `MUST` The same scope is used for the import fixes and completion candidate searches.

---

## 2.8 Library classes: jars, class-file stubs, source attachment, the SDK

### 2.8.1 Class files as declarations

Libraries and the SDK are indexed from **class files** (no source needed): each class file becomes a
*stub tree* (see `01-psi-and-stubs.md`) built by reading the class-file structure: access flags, class
name and `InnerClasses` attributes (to derive the source-level name and nesting), generic signatures
(`Signature` attribute), supertypes, fields (with constant values), methods (parameters, throws, varargs
flag, default values of annotation methods), record components, permitted subclasses, annotations
(runtime-visible and invisible, including type annotations), deprecation, enclosing-method info for local
and anonymous classes. Rules:

- **Skipped**: synthetic members and classes (compiler-generated accessors, `$VALUES`, lambda bodies), bridge
  methods, the static initializer `<clinit>`; anonymous and local classes are not indexed as named
  classes (they have no source-level name); package-info and module-info are handled separately.
- **Nested classes** are attached to their outer class using the inner-class table; a class file whose
  outer class is missing is still indexed as a top-level-like entity by its binary name.
- **Constructors** appear as constructors named like the class; the synthetic enum/inner-class constructor
  parameters (outer instance, enum name/ordinal) are removed from the parameter list.
- **Parameter names**: taken from the `MethodParameters`/local-variable debug info when present, **from the
  attached source when one exists** (see below), otherwise generated from the parameter type (a
  lower-cased short type name such as `s`, `i`, `o`; a generated name is flagged so that UIs can mark it).
- **Modifiers** mirror the source level: interface methods get implicit `public abstract` as in source; the
  `default` modifier for default methods.
- **Deprecation**: the `Deprecated` attribute or `@Deprecated` annotation (with `forRemoval`).
- **Type annotations / nullability**: annotation text is kept as written (qualified names) so that external
  nullability annotations (`@Nullable`, `@NotNull`, JSpecify, Checker, Spring) can be recognized by name
  (`07-inspections.md`).

### 2.8.2 Source attachment

When a library entry has attached source roots (a `-sources.jar`, a directory, or the SDK's `src.zip`),
the class-file declaration is *mirrored* by the source declaration with the same FQN:
- navigation, documentation and parameter names use the source (the "navigation element");
- resolution itself continues to use the class-file declarations (stable, type-correct), so that resolve
  never depends on whether sources are attached;
- usage search and highlighting of library source files map references to the class-file declaration.
When no source exists, navigation opens a decompiled view (LATER) and documentation is limited to
annotations and signatures.

### 2.8.3 JDK classes from the SDK

The SDK contributes: for JDK 9+, the classes of the runtime image (`lib/modules` jimage, read as a
filesystem of module-scoped class files); for JDK 8 and earlier, `rt.jar` and friends from `jre/lib`; the
documented API surface follows the **module graph** (a module's non-exported packages are not visible to
modules that do not read it) and, where `--release` or a language-level restriction is in force, the
`ct.sym` signatures for older API levels (LATER). JDK sources come from `src.zip`/the `lib/src.zip`
and are an attached source root (2.8.2). The `java.lang` implicit import applies to the SDK's `java.lang`
package.

- **JAVA-RES-034** `MUST` Class-file stubs with the content and the skipping rules of 2.8.1, built without
  sources and without a running language server.
- **JAVA-RES-035** `MUST` Generic signatures decoded into type parameters and generic supertypes/members
  (so `List<String>.get(0)` has type `String`).
- **JAVA-RES-036** `MUST` JDK classes from the SDK image or `rt.jar`; `java.lang` implicit.
  *Accept:* in a project with a JDK 21 SDK, `String`, `Optional`, `List` and `ProcessHandle` resolve;
  `sun.misc.Unsafe` resolves with a discouraged marker only if the module graph exposes it.
- **JAVA-RES-037** `SHOULD` Source attachment mirrors: navigation/doc/parameter names from source,
  resolution from class files.
- **JAVA-RES-038** `SHOULD` Parameter-name generation for class files without debug info.
- **JAVA-RES-039** `SHOULD` Nested class derivation from the `InnerClasses` table, including classes whose
  outer class is not in the same jar.
- **JAVA-RES-040** `LATER` `--release` API-level filtering (`ct.sym`), decompiled view.
- **JAVA-RES-041** `MUST` Skip synthetic/bridge members and anonymous/local classes in the index.
- **JAVA-RES-042** `SHOULD` Library class lookup does not read the whole jar at startup: class files are
  parsed lazily on first need, and the index of names/FQNs is built incrementally per jar with
  content hashing so unchanged jars are not reparsed.

---

## 2.9 Accessibility (JLS 6.6)

Computed per resolve result from the member's declared modifier and the place:

- `public`: always (subject to module export for named modules).
- `private`: only inside the same *top-level* class body (including nested classes).
- package-private: only when the place's package (compilation unit's package) equals the member's package;
  members of an interface are implicitly public.
- `protected`: same package, **or** the place is inside a subclass of the declaring class **and** (for
  instance members) the qualifier's type is that subclass or its subtype (JLS 6.6.2.1; constructors
  JLS 6.6.2.2 - protected constructors are accessible by `super(...)` and anonymous subclass creation,
  not by plain `new C()` from another package).
- A class member is also inaccessible if its *containing class* is inaccessible.
- Accessibility never removes a result from the multi-resolve set by itself (2.4.2 step 4); it is a flag.

- **JAVA-RES-043** `MUST` Accessibility per above, evaluated at the place and for static imports at the
  file (05 5.2.2).

---

## 2.10 Constructors, `this`/`super`, anonymous classes, enum constants

- `new C(args)` resolves among `C`'s constructors with 2.4.2; if `C` has no declared constructor the
  implicit default constructor is the sole candidate (accessible iff the class is, JLS 8.8.9).
- `this(args)` / `super(args)`: constructors of the same / the direct superclass; an implicit `super()` is
  resolved at the class level (reports 022 on the constructor or class header).
- Anonymous class `new I() {..}`: resolves the superclass/interface constructor; interfaces use `Object()`.
- Enum constant `A(1) {..}`: constructors of the enum.
- Diamond `new C<>(..)`: constructor type inference where the class type arguments are inference variables
  (JLS 15.9.3, 18.5.2): see `03-types.md`.
- Record: canonical constructor implicitly declared; compact constructor.
- Method reference `C::new` is a constructor reference: resolution as a call with the functional
  interface's parameter types.

- **JAVA-RES-044** `MUST` Constructor resolution incl. implicit default, `this`/`super`, anonymous, enum.
- **JAVA-RES-045** `SHOULD` Diamond and record canonical constructors.

---

## 2.11 Performance and caching expectations

- A reference's resolution is cached per file *revision* and per project-model epoch; editing a method body
  must not invalidate resolution of other files.
- A scope walk touches only the files whose declarations are needed (lazy by name through indexes): never
  enumerate "all classes" to resolve one name.
- Resolution of one simple name against the file-level scope must be O(imports) plus index lookups.

- **JAVA-RES-046** `SHOULD` Per-file resolve cache invalidated only by changes that can affect it.
- **JAVA-RES-047** `SHOULD` Name lookups through indexes, never full scans.

---

## 2.12 Open questions

- **Q-RES-1** IntelliJ evaluates phase strictness (strict vs loose) inside most-specific selection, not as
  separate phases; we specify the JLS phases as the contract and require identical results on the
  conformance corpus. Differences, if found, are to be recorded here.
- **Q-RES-2** Behavior of multi-resolve when an outer scope has an applicable method and the inner level has
  only an inapplicable one (2.4.1): IntelliJ reports the inner candidate set; javac reports "cannot be
  applied". Confirm with differential tests.

# 01. Java PSI and stubs

Status: specification, clean-room. Written from the IntelliJ IDEA Community sources in our own words. Source
paths are cited as `ic: <path>`. No code is reproduced.

Primary sources: `ic: java/java-psi-api/` (element interfaces), `ic: java/java-psi-impl/src/com/intellij/psi/impl/java/stubs/`
and `.../stubs/impl/` (stub data), `ic: java/java-indexing-serializers/src/com/intellij/psi/impl/java/stubs/serializers/`
(what is persisted and what each stub contributes to the indexes), `ic: java/java-psi-api/.../stubs/index/JavaStubIndexKeys.java`,
`ic: java/java-indexing-impl/.../stubs/index/` (index definitions), `ic: java/java-psi-impl/.../impl/source/JavaLightStubBuilder.java`
(which source nodes are stubbed), `ic: java/java-psi-impl/.../impl/compiled/` (class-file stubs).

In Archipel the syntax layer is a tree-sitter based concrete syntax tree ("PSI" in the platform spec is a
`SyntaxSnapshot`); this file states what *facts* the Java layer must derive from a tree and keep per
file, independent of the tree technology. The **stub** is that per-file fact record: a small, persistable
subset of a file's declarations that is enough to answer global questions without parsing the file again.

---

## 1.1 Why stubs, and the contract

A *stub tree* of a Java file is the outline of its declarations: package, imports, classes (nested to any
depth), their type parameters, supertypes, fields, methods, parameters, annotations, and modifiers. It holds
**no method bodies and no expression trees** (except as noted in 1.4). It is built from source files and
from class files by two different builders that produce the same shape. Stubs are:

- **persisted** per file and **keyed by content hash**, so an unchanged file is never reparsed to answer a
  query;
- the input to **stub indexes**: maps from a key (a short name, an FQN, ...) to the declarations that match,
  so questions like "all methods named `assertThat`" or "classes whose `extends` clause mentions `Foo`"
  are answered by lookup;
- the way declarations of *other* files are read during resolve: a resolve that needs the supertypes of a
  class in another file reads that file's stub, not its syntax tree. Anything the stub does not record must
  be recovered by parsing the file on demand.

Everything stored is **text as written** (for example a type is its source text such as `List<String>` or
`java.util.Map.Entry<K, V>`; an annotation is its source text). Names inside stubs are *not resolved*;
resolution happens later in the context of the file that uses the stub (type text is resolved in the scope of
the declaring file).

- **JAVA-STUB-001** `MUST` A per-file stub is computed from source or from a class file, persisted, and keyed
  by file content (or class-file content) hash. *Accept:* touching a file without changing content does not
  reparse it; changing a method body leaves the stub equal.
- **JAVA-STUB-002** `MUST` Stubs hold unresolved source text for types, annotations and constant initializers.
- **JAVA-STUB-003** `MUST` Stub equality ignores bodies: an edit inside a method body (without adding a
  class, lambda, method reference or annotation inside it, see 1.4) leaves the stub **and every index entry
  unchanged**, so no other file is invalidated.
- **JAVA-STUB-004** `MUST` Class-file stubs and source stubs have the same shape (2.8.1 in `02-resolve.md`).

---

## 1.2 Stubbed element kinds and what each records

Each entry lists the stub's *own data* (fields) and its *children*. Flags are booleans (stored compactly).

### Compilation unit (file)

- Data: package name (empty for unnamed), *is compiled* (class file), language level (source) or class
  language level, and for module descriptors a module stub.
- Children, in order: package statement (stub), import list (stub), top-level classes, (module declaration).
  Declared order is preserved.

### Package statement

- Data: package name text. Annotations on the package statement appear as annotation children.

### Import list and import statements

- Import list: container only; it is the single child holding the statements.
- Import statement (one per `import ...;`): reference text (the qualified name exactly as written, without
  `import`, `static` or `;`, and without the trailing `.*`), flags **on-demand**, **static**, **module**
  (for `import module X;`).
- Children: none (the reference text is parsed lazily when needed).
- Import statements are *not* indexed.

### Class-like declarations

One stub for each of: class, interface, enum, annotation type (`@interface`), record, enum-constant body
(anonymous), anonymous class creation (in expressions inside bodies, see 1.4), local class, implicit class
of a compact source file, and type declaration of a "value class" (preview).
- Data:
  - simple **name** (absent for anonymous classes);
  - **qualified name** (FQN for top-level; outer FQN + `.` + name for member classes; absent for local and
    anonymous classes);
  - **flags**: interface, enum, record, annotation type, anonymous, enum-constant initializer, implicit
    (compact source file), value class, *anonymous inner* (anonymous class that lives inside another
    stubbed body), *local class inner*, "is in qualified new" (anonymous created with an outer-instance
    qualifier), has javadoc comment, deprecated (javadoc `@deprecated` tag), has a `@Deprecated`-looking
    annotation;
  - **source file name** (for classes read from class files: the `SourceFile` attribute, used to find a
    matching source file);
  - for anonymous classes: the **base class reference text** (the type after `new`).
- Children, in order: modifier list, type parameter list, extends list, implements list, permits list,
  record header (for records, containing record components), then members (fields, methods, initializers,
  nested classes, enum constants as fields) in declaration order.

### Modifier list

- Data: a bitmask of the explicitly written modifiers (`public`, `protected`, `private`, `static`, `abstract`,
  `final`, `native`, `synchronized`, `transient`, `volatile`, `strictfp`, `default`, `sealed`, `non-sealed`,
  and for class files the corresponding access flags). Implicit modifiers (for instance members of an
  interface being `public`) are **not** stored; they are computed from the owner's kind.
- Children: annotations.

### Type parameter list and type parameter

- Type parameter list: container. Type parameter: the **name**; children: annotations and the *extends list*
  (bounds, text).

### Reference lists (extends, implements, permits, throws)

- Data: the **role** (extends, implements, permits, throws, provides/with for modules) and the list of
  **referenced names as text**, exactly as written (including type arguments for the text form, and
  including a qualifier if written).
- Used for supertypes, for `throws` clauses (methods), and for the inheritors index.

### Field (including enum constants)

- Data: **name**, **type text**, **initializer text** (kept only when the initializer is a *constant
  expression candidate*: a literal, a possibly-negated literal, a reference to a constant, or a simple
  operation of those; any other initializer stores nothing), flags: enum constant, deprecated (javadoc),
  has a `@Deprecated`-looking annotation, has javadoc.
- Children: modifier list.
- Multiple declarators in one declaration (`int a, b;`) are separate field stubs sharing the type text.

### Method and constructor

- Data: **name** (for constructors the class name), **return type text** (absent for constructors),
  flags: constructor, varargs, annotation method (`@interface` element), deprecated (javadoc), has
  deprecated annotation, has javadoc; for annotation methods the **default value text**.
- Children, in order: modifier list, type parameter list, parameter list, throws list.
- Parameter list: container; parameter: **name**, **type text**, flags: varargs (ellipsis), name generated
  (class files without debug info); children: modifier list (annotations, `final`).
- Receiver parameters and lambda parameters are not stubbed as parameters of the enclosing method.

### Class initializer

- Marker stub only (static or instance initializer block), no data; kept so that class-level counts and
  ordering are stable.

### Record header and record components

- Record header: container. Record component: **name**, **type text**, flags: varargs, has deprecated
  annotation, deprecated, has javadoc; children: modifier list. Components are also indexed by name (1.3).

### Annotation

- Data: the **annotation text as written** (`@Foo(bar = 1)` without surrounding whitespace normalization)
  and children: the annotation parameter list.
- Annotation parameter list: container of name-value pairs; name-value pair: **attribute name** (may be
  absent for the single-value shorthand) and the **value text**; literal expression values are stubbed as a
  literal stub holding the literal text (only inside annotation values; literals elsewhere are not stubbed).
- Annotations are kept for: members and classes (so that `@Test`, `@Nullable`, `@Deprecated`,
  `@FunctionalInterface`, framework annotations can be recognized without parsing), parameters, type
  parameters, packages, modules. Annotations *inside method bodies* are only reached through 1.4.

### Modules (`module-info`)

- Module: name; children: requires (module name, flags transitive/static), exports/opens (package, target
  module list), uses (service type text), provides (service type text, implementation list); all as text.
  Indexed by module name (1.3).

### Functional-expression stubs

Lambda expressions and method references occurring inside bodies get a small stub (parameter count and an
approximate shape / the qualifier and name text) so that "find functional expressions of interface X" can
be answered by index. LATER (needed for Find Usages of functional interfaces, not for resolve).

- **JAVA-STUB-005** `MUST` Stubs for: file, package statement, import list and statements (with the three
  flags), classes of all kinds, modifier lists, type parameter lists and parameters, reference lists with
  roles, fields and enum constants, methods and constructors, parameter lists and parameters, class
  initializers, annotations (with parameter lists and name-value pairs), record headers and components.
- **JAVA-STUB-006** `MUST` The fields and flags listed above, stored as text where text is named.
- **JAVA-STUB-007** `MUST` Constant initializers kept only for constant candidates (so constant folding of
  library `static final` fields works without parsing the file).
- **JAVA-STUB-008** `SHOULD` Module stubs.
- **JAVA-STUB-009** `LATER` Functional-expression stubs.
- **JAVA-STUB-010** `MUST` Order of children equals source order; implicit members are *not* in the stub
  (see 1.5).

---

## 1.3 Stub indexes and their keys

An index maps a **key** to the set of stubs (declarations) that contributed it. A query takes a key and a
scope and returns the matching declarations in the scope, *reading each declaration from its stub*.

| Index (key name) | Key | Contributed by | Meaning / consumers |
|---|---|---|---|
| short class names | simple class name (`List`) | every named class that is not anonymous: top-level, member and local classes (local classes have a name but no FQN). Anonymous classes are excluded; implicit classes use their own key | class-name completion, `Import class` candidate search, Go to Class |
| class FQN | qualified name (`java.util.List`, nested `a.B.C`) | every class with a qualified name | fully-qualified lookup (`findClass`), import resolution |
| method names | simple method name | every method and constructor with a name | method-name lookup: `Import static method` candidates, Go to Symbol, usages search narrowing |
| field names | simple field name (enum constants included) | every field | field candidates, static constant import, Go to Symbol |
| record component names | component name | every record component | record support |
| annotations | **short name** of the annotation type as written (`Test` for `@org.junit.Test(...)`, qualifier and arguments removed) | every annotation stub | "all elements annotated with X" (test discovery, framework entry points, `@Nullable` scans). The key is *text*: the consumer must resolve to confirm |
| super classes | **short name** of every type in an `extends` or `implements` list as written (qualifier and type arguments removed); additionally the key `Enum` for enums and `Annotation` for annotation types (their implicit supertypes) | every class's extends/implements lists | direct-inheritor search by name: Go to Implementation, hierarchy, "is subclass" quick checks. Consumer resolves candidates to confirm |
| anonymous base reference | short name of the base type of an anonymous class | every anonymous class stub | anonymous inheritors |
| static member names | name of every **non-private static** method or field | methods and fields with the static flag and non-private access (members of interfaces count as static only when declared `static`) | static-member completion across classes (06), without scanning all classes |
| static member types | **short type text** of the member's type (return type for methods, field type for fields) | the same members | find static members producing a type (chain completion: LATER) |
| method parameter types | **short type text of each parameter** except primitives, primitive wrappers (`Integer`...), and parameters whose type is a type variable visible at the method | every method | "methods taking a parameter of type X" (refactoring support, data-flow hints) |
| implicit classes | name | compact-source-file classes | Java 25 compact files |
| module names | module name | module stubs | `requires` resolution |

Notes:
- Keys are plain strings; the class FQN key is a character sequence (to avoid copying).
- Which stubs index nothing: imports, modifier lists, type parameters, parameters, reference lists (they
  contribute to the "super classes" key as the *list*, see above), initializers, name-value pairs.
- Every key above is computed from stub data only, so it can be recomputed from a class file the same way as
  from source.
- Queries return declarations (not files): the engine reads the owning file's stub to materialize the
  declaration. Duplicates (same FQN from source and class file) are possible and are filtered by consumers.
- An index query may be restricted by a *scope filter* (project vs libraries vs sources vs class files);
  "source filter" scopes return class-file declarations only when the file is a library class file, and
  library sources only when requested (`02-resolve.md` 2.8.2).

- **JAVA-STUB-011** `MUST` Indexes: short class names, class FQN, method names, field names, annotations
  (short name), super classes (short names with the `Enum`/`Annotation` rule), anonymous base refs.
  *Accept:* a project with `class A extends pkg.Base<String> implements I, J` puts `A` under super-class keys
  `Base`, `I`, `J`; `enum E {}` under `Enum`.
- **JAVA-STUB-012** `MUST` The annotations key uses the unqualified name with arguments removed.
- **JAVA-STUB-013** `MUST` The method-names index also contains constructors, under the class name (their
  stub name is the class name); consumers that look for methods must filter them out. *Accept:* the
  `Import static method` search for `assertThat` never returns a constructor.
- **JAVA-STUB-014** `SHOULD` Static member names/types indexes (06 completion).
- **JAVA-STUB-015** `SHOULD` Method parameter types index with the exclusions above.
- **JAVA-STUB-016** `SHOULD` Record components, implicit classes, module names.
- **JAVA-STUB-017** `MUST` Index queries accept a scope (the file's resolve scope, 02 2.7) and filter
  result declarations by the scope's source/library kinds.
- **JAVA-STUB-018** `MUST` Index updates are incremental: for a changed file remove its old entries
  (forward map file -> keys) and add the new ones; an edit that does not change the stub changes no entry.

---

## 1.4 What source stubs descend into (bodies)

Method and initializer bodies are normally skipped when building a stub, so that edits inside them never
touch the stub. A body (code block) is **descended into** only when it contains, anywhere inside, at least
one of: a class (local class), an anonymous class creation, a lambda arrow `->`, a method-reference `::`,
or an annotation `@`. In that case the local classes, anonymous classes, lambdas, method references and
annotations inside get stubs (so inheritors by anonymous class, functional-expression usages and annotation
scans work). Doc comments are never part of stubs (only the *has doc comment* flag and the deprecated
flag are). Parameters in places other than a method parameter list (lambda, catch, for-each, resource)
get no stubs. Import statements have no children.

- **JAVA-STUB-019** `MUST` Bodies are skipped unless they contain a local/anonymous class, lambda,
  method reference or annotation.
- **JAVA-STUB-020** `SHOULD` Edits inside a skipped body do not change the stub; edits that add or remove
  one of the five triggers do.
- **JAVA-STUB-021** `MUST` The *has javadoc* and *deprecated by javadoc* flags are derived from the leading
  doc comment (a `@deprecated` tag), without storing the comment.

---

## 1.5 Elements that exist in the PSI model without a stub: synthetic and implicit members

The Java model adds members that do not appear in source. Resolve and completion must see them; a stub-based
declaration list does not contain them, so the layer above stubs (the class model) must synthesize them:

- **Default constructor**: a class with no declared constructor has one public-or-class-visibility
  no-argument constructor (visibility equals the class's for non-enum; private for enums), JLS 8.8.9.
- **Enum**: static `values()` returning `E[]` and static `valueOf(String)` returning `E`; superclass
  `java.lang.Enum<E>`; each enum constant is a public static final field of type `E`; constants with bodies
  are anonymous subclasses.
- **Record**: private final fields for components, public accessor methods (unless declared), the canonical
  constructor (unless declared; compact constructors expand to it), `toString`, `equals`, `hashCode`
  (members of `java.lang.Record`); implicit `final`/`static` for nested records.
- **Interface**: all fields are `public static final`; methods are `public abstract` unless `default`,
  `static` or `private`; nested types are `public static`. Annotation interfaces: methods are abstract
  elements.
- **Object members** on interfaces for member lookup: members of `java.lang.Object` are visible on interface
  types for call resolution (JLS 9.2).
- **Array type**: `length` field, `clone()` method; supertype `Object`, `Cloneable`, `Serializable`.
- **Anonymous classes**: constructor derived from the base type's constructor chosen at the creation site.
- **Implicit imports**: `java.lang` (and compact-source implicit imports).
- **Lombok-style augmentation** (generated members from annotation processors) is an extension: LATER, see
  `docs/analysis/editions-roadmap.md` for plugin packs.

- **JAVA-STUB-022** `MUST` Synthetic members of this section are part of every class model queried by
  resolve, completion and inspections, flagged *synthetic* so that UI can hide them (no "cannot navigate").
- **JAVA-STUB-023** `MUST` `values()` and `valueOf(String)` for enums; canonical constructor, accessors and
  `Record` members for records; default constructors.
- **JAVA-STUB-024** `SHOULD` Interface implicit modifiers.
- **JAVA-STUB-025** `LATER` Annotation-processor generated members.

---

## 1.6 Dumb mode, laziness and failure behavior

- While indexes are being built (initial project open, after dependency changes), queries that need an index
  are **unavailable**: callers either fall back to local information (the current file's syntax: local
  variables, same-file classes, explicit imports resolved by FQN through a minimal class-path scan) or show
  nothing. Features declare which: errors that depend on global resolve (unresolved symbol) are **not**
  reported in this state (the pending-reference style, `04` 4.1.3); completion lists local and keyword items
  only; quick fixes needing candidate search are unavailable.
- Reading a stub must not parse the file's body; reading a body (needed for inference of `var`, flow
  analysis of the *current* file) parses only that file.
- Missing/corrupt stubs are rebuilt on demand; a failure to build a stub for one file never fails the
  query for other files (the file is skipped and logged).

- **JAVA-STUB-026** `MUST` Defined behavior while indexes are not ready (above).
- **JAVA-STUB-027** `MUST` Corrupt/failed stub builds are isolated to the file.
- **JAVA-STUB-028** `SHOULD` Persisted stub format is versioned; a version change reindexes.

---

## 1.7 Acceptance scenarios

**ST1 outline and keys.** File `p/Sample.java`:

```java
package p;

import java.util.List;
import static org.junit.Assert.*;

@Deprecated
public abstract class Sample<T extends Comparable<T>> extends Base<T> implements Runnable, Comparable<Sample<T>> {
    public static final int MAX = 10;
    private List<String> names;
    @org.junit.Test
    public static <U> void helper(int x, List<U> list, String... rest) throws java.io.IOException { /* body */ }
    class Inner {}
}
```

Stub contents: file(package `p`); import list with two imports (`java.util.List` single non-static;
`org.junit.Assert` on-demand static); class `Sample` (qualified `p.Sample`, flags: has deprecated
annotation; modifiers public+abstract; one type parameter `T` with bound text `Comparable<T>`; extends list
`Base<T>`; implements list `Runnable`, `Comparable<Sample<T>>`); field `MAX` (type `int`, initializer `10`);
field `names` (type `List<String>`, no initializer text); method `helper` (return `void`, flags varargs;
type parameter `U`; parameters `x:int`, `list:List<U>`, `rest:String`+ellipsis; throws list `java.io.IOException`;
annotation `@org.junit.Test`); class `Inner` (qualified `p.Sample.Inner`).
Index contributions: short class names `Sample`, `Inner`; FQN `p.Sample`, `p.Sample.Inner`; method names
`helper`; field names `MAX`, `names`; annotations `Deprecated`, `Test`; super classes `Base`, `Runnable`,
`Comparable`; static member names `MAX`, `helper`; static member types `int`, `void`; method parameter types:
`List` (the `U` parameter's type text `List<U>` becomes `List`; `String` for the varargs; `int` is primitive,
excluded).

**ST2 body edit.** Change `/* body */` to `{ System.out.println(1); }`: stub and index entries unchanged.
Change it to include `Runnable r = () -> {};`: the stub gains a lambda stub; index entries for names
unchanged.

**ST3 class file.** `junit-4.13.jar!/org/junit/Assert.class` yields a stub with class `org.junit.Assert`,
public, methods including `assertEquals` overloads and `assertThat` (deprecated flag), static flags, generic
signatures decoded (`<T> void assertThat(T actual, Matcher<? super T> matcher)`), no private members of
interest but private ones are present (they are filtered by accessibility, not by the stub), parameter
names generated if the class has no debug info.

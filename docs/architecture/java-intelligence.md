# Native Java intelligence (Phase 4, first vertical slice)

Status: first slice of Phase 4 of [`platform-refactor-plan.md`](platform-refactor-plan.md). Built
from the specifications in `docs/specs/java/` (clean room: the code cites requirement IDs such as
`JAVA-IMP-014`) and the platform specs in `docs/architecture/`. Crate: `archipel-ide-java`
(no `egui`; depends on `archipel-ide-analysis`, `-index`, `-language`, `-projectmodel`).

What it does today:

1. **Stubs and indexes on the project model** for source files, library jars, class directories and
   the JDK.
2. **Resolve** of classes, variables, fields and calls through the scope walk of spec 02 (locals,
   class scopes with inheritance, file-level order, `java.lang`, qualified names), with the test and
   production resolve scope.
3. **Errors**: `Cannot resolve symbol '<name>'` and `Cannot resolve method '<name>(<argTypes>)'` as a
   daemon pass, `Not resolved until the project is fully loaded` while the model imports.
4. **Import fixes** in the one Alt+Enter list: `Import static method...`/`Import static constant...`
   with the full candidate search, ordering, the `Method to Import` chooser and the layout-aware
   edit; `Import class` with the same rigor.
5. **Unused import** inspection and **Optimize Imports** (library API, not yet bound to a command).

## Pipeline

```
ProjectModel (archipel-ide-externalsystem)        buffers, watcher
        |  JavaService::sync                              |  document_seen / paths_changed
        v                                                 v
  JavaIndex  --units-->  classfile.rs / jimage.rs / source.rs  -->  FileStub (stub.rs)
        |  entries_for                      persistent: RedbStore of archipel-ide-index
        v
  Java:Stub  Java:Class  Java:ShortName  Java:Method  Java:Field  Java:Annotation
  Java:SuperClass  Java:Package  Java:Names  Java:StaticMember   (all keyed by unresolved short text; Names: per package,
        |                                      with kind, flags, deprecated and annotation targets)
        |  ClasspathSnapshot (module + scope: units in classpath order, completeness) -> ResolveScope
        v
  env.rs (class lookup, hierarchy, file-level name walk, accessibility)
        v
  analyze.rs (scope walk, expression types, errors, import usage)
        v
  diagnostics.rs: JavaResolvePass (daemon) -> HighlightInfo + QuickFix (import_fix.rs, import_edit.rs)
```

### Units and persistence (JAVA-STUB-001/018, JAVA-RES-042)

An index *unit* is one `.java` file, one jar, one class directory or the JDK image. Units get a
persistent id in the durable store (`archipel-ide-index::DurableStore`, `redb` under
`<cache>/index/<project hash>/java/`, memory without a cache directory). The store keeps length,
mtime and content hash per unit, so a restart or a model refresh only reparses what changed:
unchanged stat means no read, unchanged hash means no reindex, and a body edit leaves the stub, and so
every index entry, byte-identical (`tests/source_stub.rs`, `tests/index.rs`). A complete persisted index is
`Ready` at once after a restart; the sync that follows only touches changed units. While the first build
runs, every query returns `NotReady` (dumb mode, JAVA-STUB-026): the daemon pass reports pending
references instead of errors and offers no import fix.

Open buffers are reindexed after 400 ms of quiet (`JavaService::document_seen`), saved changes come from
the watcher (`paths_changed`). The file being analyzed always uses its live text for its own classes
and its stale index entries are ignored.

### Class files and the JDK

`classfile.rs` reads the class-file format of the JVM specification (constant pool, signatures,
`InnerClasses`, `Record`, `PermittedSubclasses`, annotations, `ConstantValue`, `MethodParameters` and
`LocalVariableTable` for parameter names, generated names otherwise). Synthetic and bridge members,
`<clinit>`, anonymous and local classes are skipped; synthetic constructor parameters are removed;
nesting comes from `InnerClasses`. Private members of library classes are not kept (except constructors)
to keep the persisted size small: `x has private access` is still reported for private constructors but
not for private methods and fields of libraries.

**JDK choice:** the JDK is read from its runtime image `lib/modules` (`jimage.rs`, the container format
of JDK 9+) and only packages that their module exports without qualification are indexed. `ct.sym` is
not used (it only matters for `--release` filtering, JAVA-RES-040 `LATER`). A JDK 8 keeps its classes in
`jre/lib/rt.jar` (`Sdk::jdk_at` adds it, the index reads it as a jar, JAVA-RES-036); there the internal
`sun.*` packages sit next to the API, which is why completion filters them (JAVA-CMP-025). Compressed
jimage resources are skipped and counted. The stubs of a jar (and of the JDK image) are also kept in a
library cache shared by projects, keyed by the content hash of the jar (`<cache>/index/java-libs/`,
PLAT-IDX-034): a second project with the same JUnit jar reads the cached stubs instead of the class files.

### Names and packages (JAVA-STUB-011/017, JAVA-RES-027)

`Java:Names` stores one `NameRecord` (FQN, kind, flags, deprecated, `@Target` mask) per named class under
its package. The records of every unit are mirrored in memory per unit id (`names.rs`, loaded from the store
after a warm start) and `JavaIndex::names_matching(scope, pattern, accept, limit)` walks the units of a
resolve scope with the `NameMatcher` (prefix, case-insensitive prefix, camel-hump `ArLi`, later humps), one
hit per FQN, nearest classpath entry wins. `classes_in_package`, `subpackages` and `inheritor_closure`
(transitive inheritors by short name, the candidate set of `throws`/`catch`) need no stub load either.

### Classpath snapshot and the ad-hoc module (JAVA-RES-029/030, PLAT-SCOPE-054/062/064)

`ClasspathSnapshot` (`classpath.rs`) is the classpath of one module for production or test code: the units in
classpath order with their roles (own sources, module dependencies, library, SDK), the language level and a
`Completeness`: `Importing`, `ImportFailed(notes)` (the notes of the last import, for example Maven's
"dependencies were not resolved"), `MissingLibraries` (jars not on disk) or `NoLibraries` (a module imported
from a build tool without any library). The status bar shows "Java: classpath incomplete" with the reason on
hover (`LanguageActivity::notice`), the control socket shows it under `languages.java.notice`. A file outside
every module, and a folder without an imported model, get the **ad-hoc module** (`ad_hoc_model`: one module
over the folder and the detected JDK), so completion and errors work with the JDK and the folder's sources.

`ResolveScope` is a view over the snapshot: a value, unit id to classpath rank. A production file sees its module's production
sources, the production sources of the modules on its compile classpath, the libraries with a compile
scope and the SDK; a test file also sees its own module's test sources and the test-scope libraries. A
file under content but outside every source root resolves only to itself. Library *sources* are not
indexed and never part of a scope (resolution uses class files, PLAT-SCOPE-064).

## Types of expressions (the M5 slice completion needs, JAVA-TYP-001..003, 013, 025)

`ty::Ty` is a closed structural model: unknown, null, primitives, class types (FQN of the erasure, simple name,
type arguments), arrays, type variables and wildcards (`? extends T`, `? super T`, `?`, only as arguments).
The presentable form is derived from the structure, never stored, so substitution rebuilds types instead of
editing strings (`Ty::subst` for reads, `Ty::subst_partial` for signatures that keep unbound variables).
`analyze/types.rs` types an expression on demand, memoized per node: literals, `new` and array creation,
`this` (with its own type parameters as arguments), `Foo.class`, casts, operators, assignments, locals and
parameters from their declared type, `var` from its initializer, pattern variables from their pattern,
enhanced-`for` variables from the element type of the iterated expression, fields (through the enclosing class
that inherits them, so `class Sub extends Base<String>` sees `T` as `String`), field chains (`this.items`,
`Type.CONSTANT`, `a.b.c`) and calls. A call's return type is the declared one seen through the receiver's type
arguments (`List<String>.get(0)` is `String`); the candidates are the methods of that name of the right arity
(an overridden declaration hides the inherited one) that accept the known argument types, and they must agree
on the return type, otherwise it is unknown. Nothing is inferred (generic method type parameters, lambdas,
diamond arguments are unknown), and an unknown type never produces a member and never an error (JAVA-TYP-032):
M5 adds inference on the same model, subtyping and capture instead of replacing it.

`analyze/members.rs` enumerates the members of a receiver (`Receiver`: the classes whose members count, the
static type for substitution, type or value qualifier, arrays, `this`/`super`): fields, enum constants,
methods and member types of the class and every supertype, overridden declarations collapsed by name and erased
parameter types, static interface methods neither inherited nor callable on an instance (JLS 8.4.8, 15.12.3),
synthetic members (`values()`, `valueOf`, record accessors, array `length` and `clone()`), accessibility by
JLS 6.6 (a public member inherited from a package-private class stays reachable through its public subclass),
signatures with the receiver's type arguments substituted. `analyze/caret.rs` answers what completion asks at
the caret: the qualifier of a dot (a variable, a type or a package, in the order of JLS 6.5.2), the variables
in scope (blocks, parameters, patterns, `for`, `try` resources, lambdas), the members of every enclosing class,
the members reachable through static imports, the expected type (initializer, assignment, `return`, condition,
`throw`, the parameter of a call whose candidates agree) and the statement context (loop, switch, `else`, `catch`).

## Completion: member access, variants and keywords (M3, JAVA-CMP-001, 003, 008, 011, 016, 017, 022, 027, 028, 035)

The caret is classified from the tokens before it (`complete/classify.rs`: annotation, `new`, type positions,
**after a dot**, and the `Slot` of a declaration or expression: top level, class body, statement, expression,
parameter, class header, method header, plus the words of the declaration so far and the statement a closing
brace ended). Where the resolver is needed, `complete/probe.rs` replaces the typed prefix (and the rest of the
identifier under the caret) by a placeholder identifier, repairs the broken statement (`repair.rs`), parses it
and asks the resolver at the placeholder, so completion and errors share the scope walk and the typing. Then:

- `q.|` (`complete/members.rs`): `expr.|` lists the instance members of the static type with the inherited ones,
  `Type.|` the static members and member types (and `class`, `this` for an enclosing type), `pkg.|` the classes and
  subpackages, `this.|`/`super.|` the members of the class chain. One row per method name, overloads merged
  (the signature and the return type of the first declaration are the tails), deprecated rows struck through,
  inaccessible members left out (offered, marked and ranked low on the second press). The members of `Object`
  are listed after the domain members, also for interfaces. A receiver that cannot be typed (unresolved
  symbol, incomplete classpath, an inference) offers nothing natively and the language server's rows show; a
  primitive or a number literal before the dot offers nothing and says so (JAVA-CMP-003).
- an unqualified identifier (`complete/variants.rs`): locals and parameters, then the fields and methods of the
  enclosing classes nearest first (a local hides a field of the same name, a method of an inner class hides the
  outer overloads), then static imports; instance members in a static context stay, ranked low.
- keywords and modifiers (`complete/keywords.rs`): by slot and context, never conflicting with what is already
  written (`abstract` removes `final`, `static`, `private`; `default` only in interfaces), `break` and `continue`
  only in loops, `case`/`default` where a switch label starts, `else` after an `if`, `catch`/`finally` after a
  `try`, `extends`/`implements`/`permits` in a class header, `throws` after parameters, `package`/`import` at the
  top of a file, primitive types in declarations and after `new` (array creation).
- static members on the second press (`complete/statics.rs`, JAVA-CMP-022): the `Java:StaticMember` index
  (name of every non-private static method, field and enum constant, JAVA-STUB-014) gives the candidates of any
  class in the resolve scope; one row per class and name, `Class.member` with the package as its tail; choosing
  it writes `Class.member(` and imports the class, or just `member(` when the class is imported statically or
  the caret is inside it.
- ranking (`complete/weigh.rs`): a sort key per row in the order of the chain of spec 06 6.5 (kind
  tier, discouraged, deprecated, expected-type match, statics preference, proximity, shorter) under the
  platform's prefix quality and usage statistics. The expected type (`List<String> l = new Arr|`) puts the
  inheritors of the expected class first (from the inheritor closure of the index): `ArrayList` before
  `ArrayStoreException`.
- insertion: `name()` for a method without parameters, `name(` with the caret inside otherwise (a snippet with
  `$0`; an existing call keeps its parentheses), the simple name and the import for a class.

## Completion: class names (M3 slice, JAVA-CMP-009/018..021/025)

`complete/classify.rs` classifies the caret from the *tokens before it* (a small lexer with a brace and
parenthesis frame stack, so a half-typed `@Te` in a broken tree still classifies): annotation position (with
the element it annotates, from the text after it), after `new`, the type positions of `extends`,
`implements`, `throws`, `catch`, `instanceof`, type arguments and bounds, and the declaration and statement
positions where classes join other variants. `complete/class_names.rs` queries `names_matching` in the
resolve scope of the file and filters by position (annotation types whose `@Target` fits, instantiable
classes, interfaces only, `Throwable` subtypes), accessibility and the exclusion list; `sun.*`, `com.sun.*` and
`jdk.internal.*` appear on the third press only, `java.awt.List` and `org.omg.*` last. Choosing a row
(`complete::insertion`) writes the simple name and the import through `import_edit::add_class_import`, the
fully qualified name when the simple name belongs to another class, nothing for classes the file already
sees, as one undo step. The native source is registered on `COMPLETION_SOURCE`
(`archipel-ide-analysis/src/langext.rs`); the completion session (`archipel-ide-core/src/completion_native.rs`)
asks it before the language server, answers alone at class-name-only positions and merges ahead of the
server's items elsewhere (the server's rows with the same class name are dropped). Setting
`java.native.completion` (Settings | Editor | Inspections) turns it off.

## Errors, fixes and how they coexist with jdtls

The daemon pass `java-resolve` reports with the tool id `native:java-resolve`. The analysis is
conservative by design: nothing is reported while a supertype, an import or a Lombok-style generated
member could supply a name, and no error is reported on an expression whose type is unknown (so
`assertThat(x).isEqualTo("a")` yields exactly one error). Coexistence follows
[`docs/analysis/architecture.md`](../analysis/architecture.md): `FileMarkup::reconcile` deduplicates by
range and kind with jdtls entries, and the setting `java.native.analysis` (Settings | Editor |
Inspections) makes the native entry win (`Shared::native_wins`). See
[`docs/analysis/java-differential.md`](../analysis/java-differential.md) for the differential result
that decides the default (on, with the setting kept).

The Alt+Enter rows carry `Priority::Top` (`QuickFix::priority`), so the import fix of an error under the
caret is the first row. Several candidates open the titled chooser (`ModCommand::ChooseFrom`), shown by a
kit `ListPopup` in `archipel-ide-ui/src/import_chooser.rs`: rows `Class.member` with the package as the grey
column, speed search on, no auto-select; headless and tests apply the first row.

## Where the slice stops (open work, by spec ID)

Left out on purpose; each is a `SHOULD` or `LATER` in the spec unless stated:

- `JAVA-STUB-009/019` lambda, method-reference and local/anonymous class stubs: bodies are skipped
  entirely; local and anonymous classes are built on demand from the syntax tree of the current file.
- `JAVA-STUB-014` is the static member *name* index only (`Java:StaticMember`); the static member type index and
  `JAVA-STUB-015/016` (parameter-type, record-component and module indexes) are not built.
- Completion is native for class names, member access, reference variants, keywords and modifiers and static
  members on the second press; what stays with the language server: members of a receiver whose type needs
  inference (lambda parameters, generic method results such as `stream().map(...).collect(...)`, diamond
  arguments), smart completion and its extra items (JAVA-CMP-002, 033), lambda and method-reference items
  (JAVA-CMP-010), annotation attributes and values (012), import positions (013), `case` labels and
  patterns (014), javadoc (015), override and getter/setter generation (011 rest), enum constants for an
  expected enum type, `Type::|` method references, the `Import statically` toggle of a static row, member names
  (variable name suggestions).
- `JAVA-RES-009` pattern-variable flow scoping (bindings anywhere in the enclosing member count as in
  scope), `JAVA-RES-008` static-context errors (018), `JAVA-RES-024/031/032` module imports,
  multi-release version selection by language level (selected by `META-INF/versions/N <= language level`
  but not tested), module-system filtering of named modules.
- Spec 03 type inference: applicability is by erasure (assignability through the class hierarchy, boxing,
  widening, varargs); unknown and generic cases are permissive, so errors 008/009/010 (wrong arguments,
  ambiguity) are not reported; only the zero-candidate errors 007, 011 and 012.
- `JAVA-IMP-019` expected-type filtering of static candidates, `JAVA-IMP-021/023` proximity weighers
  beyond ranks 3, 4, 9-13, `JAVA-IMP-034..037` the editor hint bubble, `JAVA-IMP-036` the exclusion
  sub-popup, `JAVA-IMP-038` rebinding of other references, `JAVA-IMP-047` silent auto-import,
  `JAVA-IMP-061` code-style overrides, `JAVA-IMP-075` missorted imports. Optimize Imports exists as a
  function (`import_edit::optimize`); a command and a menu entry are not wired yet.
- Kotlin and Groovy sources, `--release` API filtering, decompiled views, source attachment mirrors.

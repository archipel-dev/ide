# 04. Kotlin code completion

Status: specification, clean-room, our own words. Requirement prefix `KT-CMP`. Executable cases:
[cases/04-completion.cases.md](cases/04-completion.cases.md).

ic: `plugins/kotlin/completion/impl-k2/src/org/jetbrains/kotlin/idea/completion/**` (K2 mode):
`KeywordCompletion.kt`, `KotlinCompletionCharFilter.kt`, `impl/k2/K2CompletionCharFilter.kt`, `impl/k2/contributors/*.kt`,
`impl/k2/contributors/keywords/*.kt`, `impl/k2/lookups/**` (`CompletionShortNamesRenderer`, `TailTextProvider`, `TypeTextProvider`,
`factories/*`), `impl/k2/handlers/*.kt`, `impl/k2/ImportStrategyDetector.kt`, `impl/k2/KotlinCompletionStatistician.kt`,
`confidence/EnableAutopopupInStringTemplate.kt`, `resources/intellij.kotlin.completion.impl.xml` (registry keys, weighers),
`resources/messages/KotlinCompletionImplK2Bundle.properties`.

The completion *framework* (pipeline, auto-popup rules, matching, sorting, lookup keyboard, chrome, invocation counts) is
`codeinsight/06-completion.md` and is assumed. This file specifies what Kotlin puts into it: which items, in which positions,
how they look, how they are inserted. The semantics of the offered declarations (scopes, visibility, receivers) are the Kotlin
language specification's and file `02`.

---

## 4.1 Contributors and positions

Completion first classifies the **position** of the caret (the *position context*) and then asks the contributors that apply to it.

| Position context | Meaning |
|---|---|
| expression name | an identifier in expression position, with or without an explicit receiver (`foo<caret>`, `x.foo<caret>`, `x?.foo<caret>`) |
| type name | an identifier in a type reference (`val a: Fo<caret>`, `List<Fo<caret>>`, `as Fo<caret>`, `is Fo<caret>`) |
| annotation | the name after `@` |
| call-argument | a name inside the argument list of a call or an array access (enables named-argument and multi-argument items) |
| callable reference | after `::` |
| infix | `a foo<caret>` right of an expression with a space |
| label | after `@` of `return@`, `break@`, `continue@`, `this@` |
| `super` | `super.x`, `super<`|
| import directive | inside `import a.b.<caret>` |
| package directive | inside `package a.<caret>` |
| value parameter name | `fun f(<caret>)`, `class C(val <caret>)` |
| class/object name, type parameter name | declaration names (no completion, see 4.9) |
| member declaration expected | inside a class body at the start of a member, after `override`, after modifiers |
| `when` condition | the entry condition of a `when` with subject, or branch start of one without |
| string template | after `$` in a string, or inside `${...}` |
| KDoc | inside a `/** */` comment (links, `@param` names) |
| `where` clause constraint name | `where <caret>` |
| unknown | anything else (keywords only) |

The contributors, with what they add (in descending importance; details in 4.2 to 4.9):

1. **callable and classifier contributors**: local variables, parameters, members of implicit receivers, top-level and
   imported callables, extensions, Java statics; classes, objects, type aliases, enum entries, packages;
2. **keyword contributor**: hard and soft keywords valid at the position, plus special items (`this@Label`, `return` forms, `super`, `override`/`actual` member lists);
3. **named-argument contributor** and **multiple-argument contributor** (call arguments);
4. **operator-name contributor** (`plus`, `minus`, ... after `operator fun` and in `infix`/`operator` positions);
5. **type-instantiation contributor** (anonymous object items);
6. **override/implement contributor** (names of overridable members after `override`);
7. **name-with-type contributor** (names for parameters and variables suggested from types and existing parameters);
8. **declaration-from-unresolved-name contributor** (declarations that would fix unresolved names in the file);
9. **when-subject contributor** (enum entries, sealed subclasses, `else`, `null`);
10. **same-as-file contributor** (the class name matching the file name at top level);
11. **chain-completion contributor** (calls chained through members of the result type; registry `kotlin.k2.chain.completion.enabled`, default on);
12. **plain-text symbol contributor** (names inside comments/strings that refer to symbols, for the "word" step);
13. in **Java files**: a contributor that adds Kotlin companion members and Kotlin extension functions as Java items (4.10).

`KT-CMP-001` MUST: position classification as above and that contributors run only for their positions. Acceptance: cases 04/1..4.

## 4.2 What expression-name completion offers

At an expression-name position the items are, from all of:

1. **locals and parameters** visible at the caret (including lambda parameters and `it`), loop variables, destructuring components,
   catch parameters; a local that is *declared at the caret* is not offered;
2. **members of every implicit receiver** (`this`, outer classes' `this`, extension-lambda receivers, context receivers), own and inherited,
   with their visibility respected; for Java classes, **synthetic properties** from getters/setters;
3. **top-level declarations** of the same package and of explicit/star/default imports (functions, properties, classes, type aliases, objects);
4. **extensions** applicable to an implicit receiver, from the same scopes;
5. **not-yet-imported declarations** (from indexes of file `01`): classes, top-level functions and properties, extensions applicable to the receiver, members of objects, Java statics; they are inserted with their import (4.6);
   they come *after* items that need no import (4.8);
6. **enum entries** of the expected enum type (also unqualified when the expected type is that enum);
7. **packages**: top-level packages when the registry `kotlin.k2.complete.top.level.packages` (default on) says so, and the next segment after `pkg.` in
   a qualified expression;
8. **keywords** valid at the position (4.7).

With an **explicit receiver** (`x.<caret>`, `x?.<caret>`): members of the receiver type (smart casts applied; a member that needs a smart cast to be available is
offered in a muted colour, Kotlin's "cast required" grey, and inserting it adds the cast), extensions applicable to the receiver type (already imported ones
first, then importable ones with import on insert), Java synthetic properties, `Companion`/nested classifiers when the receiver is a class name, enum entries
when the receiver is an enum class, static Java members when the receiver is a Java class. After `?.` a nullable receiver is treated as non-null for member lookup.
Operators declared as `get`/`set` members are additionally offered as the bracket form `[...]` (4.5).

Items are never offered when: the position is inside a comment (except KDoc references), a string literal *outside* a template, a numeric literal,
right after `..`/`..<` (only expressions), or the declaration would be invisible.

`KT-CMP-010` MUST: item sources 1 to 8 with the visibility, receiver and smart-cast rules above. Acceptance: cases 04/5..14.

`KT-CMP-011` MUST: items that require an import are offered on the first invocation, ranked after non-import items, and add the import on insertion
(4.6). Acceptance: cases 04/12, 13.

`KT-CMP-012` SHOULD: a Java getter/setter pair is offered as the synthetic property with the tail ` (from getX()/setX())` (read-only: ` (from getX())`)
and also matches when the user types the getter's or setter's name (`getNa` finds `name`). Acceptance: case 04/14.

## 4.3 Presentation of items

Each lookup row consists of: icon, **item text** (bold for keywords), **tail text** (grey, right after the item text), **type text** (right aligned, grey).

| Item | Item text | Tail text | Type text |
|---|---|---|---|
| function | `name` | `(` parameters `)` then container info (below); each parameter `name: Type`, `vararg name: Type`, ` = ...` when it has a default | the return type |
| function with a trailing-lambda variant | `name` (a second item, below) | ` { name: (T) -> R }` | return type |
| property / variable | `name` | container info | the property type |
| property of function type, invoked | `name` | ` {...} ` when the only parameter is a function type, then `(P1, P2)` of its function type, then container info | the function type's return type |
| class / interface / object / type alias | simple name | type parameters `<T, R>` if any, then ` (package)` | for a type alias the expanded type; none for classes |
| constructor item (classifier used as a call) | class name | `(` parameters `)` when the class has one constructor else `(...)`, then ` (package)` | none |
| SAM-constructor item | interface name | ` {...} ` (one parameter) or ` { a, b -> ... } ` | none |
| named argument | `name =` | ` Type` (the parameter type, or `...` when several overloads disagree) | none |
| named argument with value | `name = true` / `false` / `null` (only for `Boolean` and nullable parameters at the current position) | none | none |
| multiple arguments | `a, b, c` (several local names that match missing parameter names) | none | none |
| keyword | the keyword (compound: `data class`) | none | none |
| anonymous object | `object : Foo<...>(){...}` (`Foo` shows its type arguments if known, `()` omitted for interfaces) | ` (package)` | none |
| package | the segment | none | none |
| Kotlin extension shown in a Java file | `name` (Java form) | `[Kotlin Extension]` | return type |

**Container info** (appended to the tail text of a callable, ` (...)` or ` for ...`): a member of a class shows none; a top-level callable
shows ` (package)`, `(<root>)` for the root package; an **extension** shows ` for ReceiverType in Container`, where Container is the declaring
class/object simple name or the package; with an alias import the alias item shows the FQ name instead.

Rendering of types in these places is the short, user-readable form (as `03` 3.3.7); an abbreviated type shows its expansion in a comment form.

Style: keywords bold; **deprecated** items struck through; items needing a smart cast in grey; Java items with the Java icon, Kotlin items with
Kotlin icons (class, interface, object, enum, annotation, function, extension function, property, variable, parameter, package).

`KT-CMP-020` MUST: the presentation columns above for functions, properties, classes, constructors, extensions, named arguments, keywords. Acceptance:
cases 04/15..19.

## 4.4 Insertion: functions and properties

Choosing a function item (Enter/Tab) or typing one of its commit characters inserts the name, then:

1. **parentheses**: if the text after the name is not already `(` (spaces allowed) insert `()`; if there is a `<` type-argument list in the source it is
   kept; the caret goes **inside** the parentheses if the function has value parameters or the commit character was `(`, else **after** the closing parenthesis;
   when the function has any parameter, the **parameter info** popup is scheduled (`codeinsight` parameter info);
2. a commit with `.`, `,` or `=` never places the caret inside the parentheses (the character is then typed after);
3. **type arguments**: when the function has type parameters that cannot be inferred from its arguments or from the expected type and the commit character
   is a space or Enter, insert `<>` first with the caret between the angle brackets;
4. the **trailing-lambda variant** (a separate item `name { ... }`): offered when the last parameter has a function type (or a SAM interface) and all
   earlier parameters have defaults; inserting it writes ` {  }` (a space, braces, two spaces inside) with the caret inside, no parentheses; if the lambda
   has more than one parameter (or the matching SAM method does) a **live template** is started that lets the user choose the parameter names
   (suggested names from the types and from `@ParameterName`), joined with `, ` and ending with ` -> ` before the body; single-parameter lambdas use the implicit `it`;
   typing `(` instead of choosing the variant makes the normal parenthesised insertion;
5. a function item for a **variable of function type** inserts the call form;
6. for a **callable reference** position (`::name`) no parentheses are inserted;
7. **infix** functions in infix positions insert only the name followed by a space (completion is invoked again for the argument);
8. a function with the preferred registry `kotlin.auto.completion.prefer.vararg.to.noargs` (default on) lists the `vararg` overload before the no-argument one.

Choosing a property/variable inserts the name only; in a string template see 4.5; a property called through `invoke` with a function type is a function item (rule 5).

`KT-CMP-030` MUST: rules 1 to 3, 5 to 7. Acceptance: cases 04/20..27.

`KT-CMP-031` SHOULD: the trailing-lambda variant and its template (rule 4) and rule 8. Acceptance: cases 04/28, 29.

## 4.5 Insertion: classes, templates, brackets, named arguments

- **Class-like item**: the simple name is inserted and the class is **imported** if needed (4.6); no `()` and no type arguments are added by the class item (registry
  `kotlin.auto.completion.insert.constructor.parenthesis`, default off); the *constructor item* (separate row, expression positions) inserts `()` with the caret
  rules of 4.4 and, when type arguments are required and cannot be inferred, `<>` first (4.4 rule 3); objects and enum entries insert only the name.
- **Anonymous-object item** (`object : Foo()...`): inserts `object : Foo() {` newline, the caret in the body, `}`; for an abstract class or interface with
  abstract members the unimplemented members are generated (override stubs) as the *Implement members* action would.
- **String templates**: completing a name right after `$` inside a string inserts the name; if the following characters would be glued to it
  (`"$na<caret>me"` style: an identifier character follows) the entry is wrapped as `${name}`; a `${}` entry that needs no braces afterwards drops them. Typing
  `.` right after a simple template entry is not a commit character: it is added to the prefix inside strings (the item is kept open).
- **Bracket operators**: the items `get`/`set` of a member that is an `operator` can be inserted as `[]`; choosing deletes the preceding `.` and surrounding
  whitespace and puts the caret inside the brackets.
- **Named arguments**: in the argument list, at the position of the next unpassed parameter, items `name =` are offered for each parameter not yet passed
  by a matching candidate (only for functions with stable parameter names: Kotlin functions, not Java methods, except through annotation); insertion
  writes `name = ` and leaves the caret after it. For `Boolean` parameters items `name = true` and `name = false`, for nullable ones `name = null` are
  additionally offered when the parameter is the *next* one. A local variable with exactly the parameter's name additionally yields `name = name`.
  Not offered after the first named argument for positional completion, not for `private` primary constructors outside their class, not in smart completion.
- **Multiple arguments**: in the **last** argument position of a call or index access with no named arguments yet, if at least two local variables
  (in this file, no imports) have the names of at least two still-missing parameters in order, a single item `a, b` is offered which inserts all of them
  separated by `, ` (a pair of overlapping icons).

`KT-CMP-040` MUST: class insertion with import, `<>`, objects, string-template wrapping and dot handling. Acceptance: cases 04/30..34.

`KT-CMP-041` MUST: named-argument items with the conditions above. Acceptance: cases 04/35..38.

`KT-CMP-042` SHOULD: multiple-argument items, bracket-operator items, anonymous-object items. Acceptance: cases 04/39..41.

## 4.6 Imports on completion

Every item that is not already accessible carries an **import strategy** decided when the item is built:

- **class-like** item: *insert the FQ name and shorten*: the simple name stays, an import is added if there is no conflict (otherwise the name is
  partially qualified);
- **top-level function or property** (not an extension): *insert the FQ name and shorten*: the simple name stays and `import pkg.name` is added;
- **extension** callable: *add import* of the callable's FQ name (`import a.b.ext`);
- callable that is a member of an **enum, object, companion object** or a **Java static**: *insert the FQ name and shorten*, but callables of classes are
  shortened only if they are already imported: the item is inserted **qualified by its owner** (`Util.twice(2)`, `Color.RED`, `Math.abs(x)`) and only the
  *owner class* gets an import (`import lib.Util`) [verify];
- member of an ordinary class (no stable path): nothing.

An import is **not** added if the name is already available (explicit/star/default/same-package import) — checked against the import scopes of the file. Adding uses the
primitive of `02` 2.11 (no star collapse). The import is part of the same undo step as the insertion.

`KT-CMP-050` MUST: strategies above. Acceptance: cases 04/12, 13, 42.

## 4.7 Keyword completion

Keywords are offered by position: all hard keywords and soft keywords (`by`, `catch`, `constructor`, `finally`, `get`, `set`, `init`, `where`, `import`, `file`,
`field`, `property`, `receiver`, `param`, `setparam`, `delegate`, `dynamic`, `value`, `vararg`, `out`, `in`, `data`, `sealed`, `enum`, `annotation`, `inner`,
`open`, `abstract`, `final`, `override`, `lateinit`, `const`, `inline`, `noinline`, `crossinline`, `tailrec`, `operator`, `infix`, `external`, `suspend`, `expect`,
`actual`, `companion`, `internal`, `private`, `protected`, `public`, `reified`, `typealias`, `context`, `all`) whose use would still **parse** at the position:
the candidate keyword is tested by parsing a small synthetic context around the caret (for example `fun foo() { if (...)` for `if`); only keywords the parser accepts survive.

Rules on top of the parse test:

- never in comments, in string content, in a numeric literal, or right after `.` / `?.`;
- inside an annotation argument list only `if`, `else`, `true`, `false`, `when`;
- before an `import` list at file start only `import`, `package`, `as`, `file`;
- **compound keywords** are offered as one item when the second keyword is not yet typed after the caret: `data class` (and `data object` when the language level supports it), `enum class`, `annotation class`,
  `sealed class` / `sealed interface` (`sealed fun` is also valid but not suggested), `lateinit var`, `const val`, `suspend fun`, `companion object` (only in a class body that has no companion); when the next keyword is
  already present only the first is offered; `suspend` is not offered inside a type reference;
- `sealed` and `data` are not offered next to keywords that make them invalid (`annotation`, `enum`, `open`, `inner`, `abstract`, `data`, `sealed`);
- at the top level of a file whose name is a valid capitalised identifier and declares no class of that name, `object`, `interface`, `class`, `data class`
  additionally appear as items that include the file's name (`class Foo`, `data class Foo(...)`);
- **construct keywords** insert more than the word: `if`, `while`, `for` insert `(` `)` with the caret inside after a space; `try` a block; `catch` a `(...)`
  and block; `finally`, `do`, `init`, `constructor`, `context` their shapes [verify exact text, see cases];
- **no space after**: `this`, `super`, `null`, `true`, `false`, `break`, `continue`, `else`, `when`, `file`, `dynamic`, `get`, `set`, `companion object`;
  every other keyword gets one space after it; a use-site annotation target (`@file`, `@get`, ...) is shown with a trailing `:` and inserts `:` without space;
- `this` offers also `this@Outer` forms for each enclosing class/lambda label, `return` offers `return@label` forms and `return` with the expected value
  when the position is a return position (last statement, elvis right side), `break`/`continue` offer labels of enclosing loops, `super` offers
  `super<Type>` for each direct supertype when ambiguity exists, `override` offers member lists (4.9), `actual` offers actual stubs for `expect` members;
- `dynamic` is not offered on the JVM.

`KT-CMP-060` MUST: parse-based filtering and the rules above. Acceptance: cases 04/43..50.

## 4.8 Ordering

On top of the framework's classifier chain (`codeinsight/06` section 7) Kotlin contributes these relevance facts, in order of importance:

1. **expected-type match**: items whose type equals/fits the expected type of the position (assignment target, argument parameter type, return type,
   branch of `if`/`when`, elvis right side) are preferred; exact match above subtype match; for generic callables a possible match counts;
2. **kind priority** (enum `ItemPriority`, lower first): super-method-with-arguments, names from unresolved-name suggestions, bracket-operator items, **default**,
   then `implement`, `override`, static members from imports, other static members;
3. **locality**: locals and parameters before members before top-level before imported-on-insert;
4. **deprecated** last; **`componentN`** functions and (outside suspend contexts) **suspend** functions deprioritised;
5. **proximity** with one Kotlin rule: libraries under `kotlinx.*` are preferred over other libraries (inserted into the proximity chain after the "same module" weigher and before "known element");
6. **usage statistics** (framework) keyed by the item text; anonymous-object items are keyed by the FQ name of the class (they would otherwise all be `object`);
7. the **named-argument** items rank by the argument index of their first unpassed parameter.

`KT-CMP-070` SHOULD: these relevance rules. Acceptance: cases 04/51..55.

## 4.9 Declaration-position completion

- `override fun <caret>` / `override val <caret>` / `class C(override val <caret>)`: names of **overridable members** of the supertypes (not yet overridden, visible, not `final`),
  shown as `name(params)` items with return types; choosing generates the full override stub (`override fun foo(): Int { TODO("Not yet implemented") }`
  for abstract/open members, calling `super` where the member has a body) with the caret in the body; a property inserts `override val name: Type get()`... as the Override/Implement action
  would (`codeinsight/04`); the item order is implemented-before-overridden members.
- the keyword `override` itself lists, in a class body, every overridable member as a ready-made declaration (`override fun toString(): String`) so a member can be created by
  typing `ove` + Enter + choose.
- **parameter and variable names**: in a value-parameter position, a variable name position (`val <caret>`) or after a receiver type, names are suggested from (a) existing
  parameters in the *same file* with matching prefix, rendered `name: Type`, (b) classes in scope and in the indexes: the class name with its first letter lower-cased
  (`FooBar` -> `fooBar: FooBar`), plus abbreviations by camel-hump; the list is recomputed when the prefix changes to or from an upper-case letter.
- **unresolved names**: in an expression position, a name that is used elsewhere in the file but unresolved is offered as a *declaration suggestion*
  (rank `FROM_UNRESOLVED_NAME_SUGGESTION`), so one can create the missing declaration.
- **class/object/type-parameter/enum-entry declaration names, `fun name`, `val name`, property names in a class body**: no completion popup (the user is naming something new)
  except for names listed above; the popup is not auto-shown after the identifier-start of a declaration (`codeinsight/06` CI-CMP-031).
- `where T : <caret>`: type names; `where <caret>`: type parameter names of the declaration.

`KT-CMP-080` SHOULD: override/implement completion, name suggestions, unresolved-name suggestions. Acceptance: cases 04/56..60.

## 4.10 When-subject completion

In a `when (subject)` entry condition:
- subject of **enum** type: its entries not yet used, written qualified (`Color.RED`) unless the enum is imported member-wise, plus `else` and, for a nullable subject, `null`;
- **sealed** subject: its direct and nested subclasses not yet covered (`is Sub` for classes, plain `Sub` for objects), plus `else`, `null` when nullable;
- other subjects: types (as `is T`), constants and `else`;
- when the user has typed a prefix of `is` the prefix is cleared so both `is X` items and names remain;
- `else` is offered only if there is no `else` entry yet.
A used entry is not offered again. Index lookups are restricted to names that *start* with the prefix unless the invocation count is at least 2 or the prefix is longer than 3 characters.

`KT-CMP-090` SHOULD: as stated. Acceptance: cases 04/61..63.

## 4.11 Character filter (Kotlin part of `codeinsight/06` CI-CMP-071)

While the lookup is open (auto-popup or explicit), typing:
- an identifier character or `@` extends the prefix;
- `:` extends the prefix (for `::`), except for items marked hide-on-colon, which close the lookup;
- `.`: when the item is inside a string literal it extends the prefix; otherwise **selects the item and closes** (`foo.` is typed after insertion), except when the prefix
  is empty, the lookup was auto-popped and the char before the caret is also `.` (typing `..`): the lookup closes without selecting;
- `,`, space, `(`, `=`, `!`: select the current item and close (the character is then typed unless the insertion consumed it);
- `{`: selects only if the current item accepts a brace (functions: trailing-lambda forms), else closes;
- any other character closes the lookup without selecting;
- in the **beginning of a function literal** (where the first token may be a parameter name) the special characters never select: they close the lookup.
In a K2 mode, with registry `kotlin.k2.dot.add.enabled` (default on): typing `.` at an empty prefix right after another `.` extends the prefix instead.

`KT-CMP-100` MUST: the table above. Acceptance: cases 04/64..68.

## 4.12 Smart completion (expected-type completion)

`Ctrl+Shift+Space` (SmartTypeCompletion) is available in K2 (registry `kotlin.k2.smart.completion.enabled`, default on). It restricts the result to
items whose type fits the **expected type** of the position and extends them: for an expected type `T` it offers callables returning `T`, constructor calls and factory functions of `T` or subtypes,
members reached through **one level of chaining** (`holder.value`), lambdas for function types (`{ it -> ... }` skeleton), anonymous objects for interfaces, `this` (and `this@Outer`), `null` for nullable `T`,
`true`/`false` for `Boolean`, enum entries, objects, and items qualified by their owner for static members. It never offers named-argument items with no value. After insertion of a call
as an argument the caret handles the closing `,`/`)` by the smart-completion tail handler (the tail character is inserted or overwritten).

`KT-CMP-110` SHOULD: expected-type filtering and tail handling. Acceptance: cases 04/69..72.
`KT-CMP-111` LATER: the full catalogue of smart-completion item kinds (chains of length two, "Ctrl+Shift+Space twice" for more, `it` suggestions for lambdas, `emptyList()`/`listOf()` for collection types).

## 4.13 Completion in Java files, KDoc, dumb mode, commands

- **Java file**: Kotlin `companion object` members and Kotlin extension functions applicable to a Java expression are offered as Java items (registry
  `kotlin.java.completion.companion.members` and `kotlin.java.completion.extensions`, default on); an extension is shown with the tail `[Kotlin Extension]` and inserts as a static call
  `FooKt.ext(receiver, ...)` form (Java syntax) per 07.
- **KDoc**: after `[` and `@param`, `@property`, `@return` etc.: names of parameters, type parameters, properties and in-scope declarations; no import offers.
- **Dumb mode**: PSI-based completion is used while indexing (registry `kotlin.auto.completion.dumb.mode.use.psi.completion`, default on), see `01` 1.9.
- **Auto-popup in templates**: typing an identifier after `$` inside a string shows the popup even though other string contents do not (the confidence filter says "no skip" for a position in or right after a simple template entry unless whitespace intervenes); registry `kotlin.disable.auto.completion.inside.expression` (default off) can disable auto-popup inside expressions.
- **Command completion** (typing `..`/`.` then an action word to run refactorings and navigation from the popup; Kotlin registers providers for rename, optimize imports, generate, go to declaration, super method, implementation, format code, copy FQN, show usages, delete, change signature, create from usage, extract, inline, move, copy, type hierarchy, type info, documentation, surround with, safe delete, parameter info, live template insertion): `KT-CMP-120` LATER.

## 4.14 Settings of the Kotlin plugin that influence completion

None beyond the platform's Code Completion settings (`codeinsight/06` 2.2) and the registry keys quoted here. The Kotlin **code style** setting
`Insert imports for nested classes` and the import layout (`02` 2.10.3) apply to the imports completion adds.

`KT-CMP-130` MUST: registry-equivalent switches exist with these defaults: `kotlin.k2.dot.add.enabled=true`, `kotlin.auto.completion.prefer.vararg.to.noargs=true`,
`kotlin.auto.completion.insert.constructor.parenthesis=false`, `kotlin.auto.completion.dumb.mode.use.psi.completion=true`, `kotlin.k2.chain.completion.enabled=true`,
`kotlin.k2.smart.completion.enabled=true`, `kotlin.k2.complete.top.level.packages=true`, `kotlin.java.completion.companion.members=true`,
`kotlin.java.completion.extensions=true`, `kotlin.disable.auto.completion.inside.expression=false`. Acceptance: settings dump lists them.

## Open questions

- Q-CMP-1: exact inserted text of construct keywords (`if`, `while`, `for`, `try`, `catch`, `finally`, `do`, `init`, `constructor`, `context`) and caret placement: implement
  `keyword (<caret>)` for `if`/`while`/`for`, a `{ <caret> }` block for `try`/`finally`/`do`/`init`, then verify by differential tests.
- Q-CMP-2: precise numeric weights between the Kotlin-specific relevance factors (4.8); only their order is specified.
- Q-CMP-3: smart-completion catalogue (KT-CMP-111).

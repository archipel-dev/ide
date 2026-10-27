# 03. Kotlin diagnostics as the IDE presents them

Status: specification, clean-room, our own words. Requirement prefix `KT-DIAG`. Executable cases:
[cases/03-diagnostics.cases.md](cases/03-diagnostics.cases.md).

ic: `plugins/kotlin/highlighting/highlighting-k2/src/org/jetbrains/kotlin/idea/highlighting/visitor/KotlinDiagnosticHighlightVisitor.kt`,
`plugins/kotlin/highlighting/highlighting-k2/resources/messages/K2HighlightingBundle.properties`,
`plugins/kotlin/code-insight/fixes-k2/src/org/jetbrains/kotlin/idea/k2/codeinsight/fixes/KotlinK2QuickFixRegistrar.kt`
(diagnostic -> fix registration), `plugins/kotlin/code-insight/inspections-k2/src/org/jetbrains/kotlin/idea/codeInsight/inspections/diagnosticBased/*.kt`
(diagnostics shown as inspections), `plugins/kotlin/base/code-insight/resources/messages/KotlinBaseCodeInsightBundle.properties`
(suppress texts), `plugins/kotlin/base/resources/resources-en/messages/KotlinBundle.properties` (fix texts).
Compiler side (not in the Community checkout; Apache-2.0, Kotlin 2.4 sources `kotlin-compiler-2.4.21-sources.jar`):
`org/jetbrains/kotlin/fir/analysis/diagnostics/{FirErrors,FirErrorsDefaultMessages,FirDiagnosticRenderers}.kt`,
`org/jetbrains/kotlin/diagnostics/PositioningStrategies.kt` (which part of the source a diagnostic marks),
`org/jetbrains/kotlin/parsing/*.java` (parser messages). Semantics of every error: the Kotlin language specification,
<https://kotlinlang.org/spec/>.

Archipel implements its **own** Kotlin analysis. Which program is wrong and why is dictated by the language
specification and by the K2 compiler (the reference implementation); this file fixes **what the user sees**: the
message text, severity, the highlighted range and the quick fixes, for the diagnostics a developer meets daily.
Message templates use `{0}`, `{1}`, ... for arguments and `''` for a literal apostrophe, as in the compiler
(we give rendered examples).

---

## 3.1 Compiler diagnostics versus inspections

The IDE has two producers of problems for a Kotlin file:

1. **Compiler diagnostics** (the K2 front-end's checkers, 851 diagnostic kinds). They run in the daemon's *general
   highlighting* phase over the whole file at once (`codeinsight/01-daemon.md`); each diagnostic becomes one highlight
   per marked range; they cannot be disabled in an inspection profile (only suppressed, 3.4).
2. **Inspections** (`05-inspections.md`): IDE checks with a profile entry (name, level, on/off, options). A subset are
   **diagnostic-based inspections**: they are driven by one compiler diagnostic but are presented as an inspection with
   their *own* message and fix: `AssignedValueIsNeverRead`, `CanBeVal`, `EmptyRange`, `EscapingCapturedVariable`,
   `KotlinUnreachableCode`, `RedundantInterpolationPrefix`, `RedundantModalityModifier`, `RedundantUnitReturnType`,
   `RedundantVisibilityModifier`, `UnusedExpression`, `UnusedLambdaExpression`, `UnusedVariable`,
   `VariableInitializerIsRedundant`, `VariableNeverRead`. For these the compiler's own highlight is **dropped** when the
   IDE shows the inspection (`UnusedExpression`, `UnusedLambdaExpression` are filtered explicitly from the compiler
   highlighting) so a problem is never shown twice.

`KT-DIAG-001` MUST: every compiler diagnostic of this catalog is reported with the exact message, severity and range given,
independent of inspection profiles, and diagnostic-based inspections never produce duplicate marks. Acceptance:
cases 03/1..30.

### 3.1.1 Highlight type, tooltip and presentation

| Diagnostic family | Highlight type |
|---|---|
| unresolved reference (`UNRESOLVED_REFERENCE`, `UNRESOLVED_LABEL`, `UNRESOLVED_REFERENCE_WRONG_RECEIVER`, `UNRESOLVED_IMPORT`, `MISSING_STDLIB_CLASS`) | **wrong reference**: red wavy underline in the "cannot resolve" colour (a distinct text attribute from a plain error) |
| deprecation (`DEPRECATION`, `DEPRECATION_ERROR`) | **deprecated**: strikethrough of the referenced name, keeping the diagnostic's severity |
| `USELESS_CAST`, `USELESS_ELVIS`, `USELESS_IS_CHECK`, `REDUNDANT_OPEN_IN_INTERFACE` | **unused-symbol look**: grey text on the range (warning) |
| other error | error: red wavy underline (error stripe red) |
| other warning | warning: yellow background / underline per colour scheme |
| informational | information (no underline unless the scheme adds one) |

The highlight's *description* is the message; the *tooltip* is the message HTML-escaped with line feeds turned into
`<br>`; in the IDE's internal mode the factory name is appended. A diagnostic with several marked ranges produces one
highlight per range, with the same message and fixes.

**Pending state.** If the diagnostic is an error and the project model is still incomplete (indexing, import still
running) it is replaced by a *pending reference* mark with the tooltip `Not resolved until the project is fully loaded`
(KT-STUB-060); diagnostics that cannot be caused by a missing model are shown normally.

**Fixes.** Each highlight carries the quick fixes registered for its diagnostic kind (3.3, Fixes column) plus the lazily
computed fixes of `Unresolved reference`: the import fix (02), `Create ...` from-usage fixes, `Rename reference`, and an
*add dependency* fix when a library that contains the name can be proposed. **Every warning** additionally gets the
suppression options (3.4); a warning with no fix of its own still gets an intention so the lightbulb appears.

**Scope.** Not analysed: compiled (`.class`-derived) files, injected-language fragments that do not request full analysis, files
outside content roots in "essential highlighting only" mode. In *code fragments* (debugger expressions) visibility errors
(`INVISIBLE_REFERENCE`, `INVISIBLE_SETTER`) are suppressed.

`KT-DIAG-002` MUST: highlight types, tooltip/description rule, pending state, scope rules. Acceptance: cases 03/1, 2, 31.

## 3.2 Ranges ("positioning")

A diagnostic is attached to a source element, and a *positioning rule* chooses what part of it is marked. The rules used in
the catalog (column Range):

| Rule | Marked text |
|---|---|
| `name` (declaration name) | the name identifier; for a class/object it starts at the first of its modifiers/keyword (`data class Foo` marks `data class Foo`); for a function without a name the signature |
| `reference-name` | the identifier of the called/referenced name: for `a.foo(1)` the word `foo`; for `foo(1)` the word `foo`; for `x::foo` the word `foo`; for a type reference `List?` the inner `List`; for an operator expression the operator token |
| `reference` | like `reference-name` but unwraps nothing (the whole selector/callee expression, e.g. `foo<T>` including type arguments) |
| `dot` | the `.` or `?.` of the qualified expression (an assignment to `a.b` marks the dot in `a.b`) |
| `selector` | the selector part after the dot (`foo(1)` of `a.foo(1)`); in an assignment the left side; in an import its alias name or the imported reference |
| `operator` | the operation token (`!!`, `=`, `+`, `as`, `?:`) |
| `signature` | from the function/property keyword (modifiers excluded) to the end of the return type, else of the parameter list, else of the name |
| `whole` | the whole element |
| `default` | the whole element, except an object literal (`object :` .. to end of super type list), `object Foo` (keyword to name) and a constructor delegation call (the constructor signature) |
| `if`/`when`/`else` | the keyword `if` / `when` / `else` of that construct |
| `body-end` | the closing `}` of the function body (the function itself if it has no block) |
| `val-var` | the `val`/`var` keyword |
| `initializer` | for a property: the `=` sign, else the initializer; for a parameter default value: the `=`; (when the reported element is the initializer expression itself, that expression) |
| `from-operator` | from the operator token to the end of the whole expression (elvis, `as`) |
| `index` | the `[...]` brackets of an array access |
| `safe-access` | the `?.` token |
| `modifier` | the named modifier keyword (`override`, `open`, `abstract`, visibility, `lateinit`, `const`, ...), else the declaration name |
| `import-last` | the last name of the import path (aliased: the alias) |
| `label` | the label (`@name`) |
| `named-arg` | the name of a named argument |
| `unreachable` | the unreachable statements, each as its own range |

## 3.3 The catalog

Columns: `ID`; `Pri`; `Message` (compiler template; `{n}` rendered as described; `\n` = line feed); `Trigger` (kind,
severity, what causes it, compiler factory name); `Range`; `Fixes` (Alt+Enter row texts, in order; `{x}` placeholders are
filled with the actual names).

Severity: E = error, W = warning.

### 3.3.1 Resolution and imports

| ID | Pri | Message | Trigger | Range | Fixes |
|---|---|---|---|---|---|
| KT-DIAG-010 | MUST | `Unresolved reference 'foo'.` plus, when the unresolved name is an operator convention, ` for operator '+'` and, when the receiver type is known and error-free, ` on receiver of type 'A'` before the final period | E `UNRESOLVED_REFERENCE`: a simple name, selector, type name, annotation or operator does not resolve | reference-name | `Import ...` (02), `Create function 'foo'` / `Create property 'foo'` / `Create local variable 'foo'` / `Create parameter 'foo'` / `Create class 'Foo'` / `Create interface` / `Create enum`/`Create annotation` (06), `Rename reference`, add-dependency |
| KT-DIAG-011 | MUST | `Unresolved reference 'Foo'.` | E `UNRESOLVED_IMPORT`: an import path (last segment) that does not exist | import-last | add-dependency; no Import fix (02 KT-IMP-002) |
| KT-DIAG-012 | MUST | `Candidate 'fun String.foo(): Unit' is inapplicable because of a receiver type mismatch.` (one diagnostic per candidate) | E `UNRESOLVED_REFERENCE_WRONG_RECEIVER`: the name exists but only as an extension/member of another receiver | reference | `Import ...` for other candidates (02) |
| KT-DIAG-013 | SHOULD | `Unresolved label.` | E `UNRESOLVED_LABEL`: `return@x`, `break@x`, `this@x` with no such label | label | `Create label` ( `Create Label 'x'@` ) |
| KT-DIAG-014 | MUST | `Cannot access 'val x: Int': it is private in 'A'.` (`{0}` = the declaration head as in 3.3.7, `{1}` = `private`/`protected`/`internal`; `{2}` = `'a.B'` (quoted FQ name) for a member or `file` for a top-level declaration) | E `INVISIBLE_REFERENCE`: the declaration exists but is not visible here | reference | `Import ...` for other visible candidates; `Make 'foo' public` / `Make 'foo' internal` |
| KT-DIAG-015 | MUST | `Overload resolution ambiguity between candidates:` followed by one candidate per line, e.g. `fun foo(a: Int): Unit` | E `OVERLOAD_RESOLUTION_AMBIGUITY`: several equally specific candidates (Kotlin spec, overload resolution) | reference | none of its own |
| KT-DIAG-016 | MUST | `None of the following candidates is applicable:\n\n<one candidate per line>` | E `NONE_APPLICABLE`: candidates exist but none accepts the arguments | reference-name | `Import ...`, `Specify remaining required arguments by name`, `Change the signature of function 'f'` |
| KT-DIAG-017 | MUST | `Conflicting import: imported name 'Foo' is ambiguous.` | E `CONFLICTING_IMPORT`: two non-star imports bring the same simple name | import alias (else last name) | `Remove conflicting import {0}` |
| KT-DIAG-018 | MUST | `Cannot import on demand from object 'Obj'.` | E `CANNOT_ALL_UNDER_IMPORT_FROM_SINGLETON`: `import a.Obj.*` | import-last | none |
| KT-DIAG-019 | SHOULD | `Cannot access class 'X'. Check your module classpath for missing or conflicting dependencies.` | E `MISSING_DEPENDENCY_CLASS`: a used declaration mentions a class that is not on the classpath | reference-name | add-dependency |
| KT-DIAG-020 | MUST | `Expression 'x' of type 'Int' cannot be invoked as a function. Function 'invoke()' is not found.` | E `FUNCTION_EXPECTED`: `x()` where `x` has no `invoke` | reference-name | `Import ...` (an `invoke` extension), `Change to property access` / `Remove invocation` |
| KT-DIAG-021 | SHOULD | `Classifier 'Foo' does not have a companion object, so it cannot be used as an expression.` | E `NO_COMPANION_OBJECT` | selector | none |
| KT-DIAG-022 | SHOULD | `Names _, __, ___, ... can only be used in back-ticks (`_`, `__`, `___`, ...).` | E `UNDERSCORE_USAGE_WITHOUT_BACKTICKS` | name | `Rename ...` |
| KT-DIAG-023 | MUST | `Syntax error` rows: the **parser's own message** with no prefix, e.g. `Expecting ')'`, `Expecting an element`, `Expecting '}'`, `Expecting a name`, `Expecting an expression`, `Expecting member declaration`, `Expecting a top level declaration`, `Expecting a class body`, `Expecting property name or receiver type`, `Unexpected tokens (use ';' to separate expressions on the same line)`, `Name expected`, `Type expected`, `Unclosed comment`, `Missing '}`, `Expecting '->'` | E syntax error (parse error element) | the error element (an empty error is shown at the preceding position, one character wide) | none (structure-specific completion helpers such as auto-closing brackets are in the editor spec) |

### 3.3.2 Types and nullability

| ID | Pri | Message | Trigger | Range | Fixes |
|---|---|---|---|---|---|
| KT-DIAG-030 | MUST | `Type mismatch: inferred type is 'String', but 'Int' was expected.` ({1} inferred, {0} expected) | E `TYPE_MISMATCH` (remaining contexts such as `if`/`when` branches and lambda results) | default | `Change type of ... to ...`, `Cast expression 'x' to 'T'`, `Add 'toString()' call`, `Convert to ...`, `Wrap with collection literal call`, `Surround with lambda` |
| KT-DIAG-031 | MUST | `Argument type mismatch: actual type is 'String', but 'Int' was expected.` | E `ARGUMENT_TYPE_MISMATCH`: an argument of a resolved call | default (the argument) | `Change parameter 'a' type of function 'f' to 'String'`, `Cast expression ... to ...`, `Add 'toString()' call`, `Convert to Int` (number conversion: e.g. `Convert expression to 'Int' by inserting '.toInt()'`), `Wrap with '?.let { ... }' call` (for a nullable argument), `Round using roundToInt()`/... , `Convert string to character literal`, `Add 'arrayOf' wrapper`, `Change to use spread operator` |
| KT-DIAG-032 | MUST | `Initializer type mismatch: expected 'Int', actual 'String'.` | E `INITIALIZER_TYPE_MISMATCH`: `val x: Int = "a"` | initializer expression (when the reported element is the declaration, its `=`) | `Change type of 'x' to 'String'`, `Cast expression '"a"' to 'Int'`, `Convert expression to 'Int' by inserting '.toInt()'`, `Add 'toString()' call` |
| KT-DIAG-033 | MUST | `Return type mismatch: expected 'Int', actual 'String'.` | E `RETURN_TYPE_MISMATCH`: `return` expression or last expression of a block/lambda with a declared return type | whole returned expression | `Change return type of enclosing function 'f' to 'String'`, `Cast expression ...`, `Change to 'return@label'` |
| KT-DIAG-034 | MUST | `Assignment type mismatch: actual type is 'String', but 'Int' was expected.` | E `ASSIGNMENT_TYPE_MISMATCH`: `x = "a"` | operator (when the reported element is the assignment) else the right-hand side | `Change type of 'x' to 'String'`, `Cast expression ...` |
| KT-DIAG-035 | MUST | `Condition type mismatch: inferred type is 'Int' but 'Boolean' was expected.` | E `CONDITION_TYPE_MISMATCH`: `if (1)` | default (the condition) | `Cast expression ...`, `Simplify comparison` where applicable |
| KT-DIAG-036 | SHOULD | `Incompatible types 'A' and 'B'.` | E `INCOMPATIBLE_TYPES`: `==`/`when` branch of unrelated types | default | `Cast expression ...`, `Convert string to character literal` |
| KT-DIAG-037 | MUST | `Null cannot be a value of a non-null type 'String'.` | E `NULL_FOR_NONNULL_TYPE`: `val s: String = null` | default (the `null`) | `Change type of 's' to 'String?'` |
| KT-DIAG-038 | MUST | `Only safe (?.) or non-null asserted (!!.) calls are allowed on a nullable receiver of type 'String?'.` | E `UNSAFE_CALL`: `s.length` with `s: String?` | dot | `Replace with safe (?.) call`, `Wrap with '?.let { ... }' call`, `Add non-null asserted (s!!) call` (low priority, last) |
| KT-DIAG-039 | MUST | `Infix call is prohibited on a nullable receiver of type 'A?'. Use '?.'-qualified call instead.` and `Operator call is prohibited on a nullable receiver of type 'A?'. Use '?.'-qualified call instead.` | E `UNSAFE_INFIX_CALL`, `UNSAFE_OPERATOR_CALL`: `a foo b`, `a + b`, `a[i]` on a nullable | reference-name | `Replace with safe (?.) call`, `Add non-null asserted (a!!) call`, `Wrap with '?.let { ... }' call` |
| KT-DIAG-040 | SHOULD | `Reference has a nullable type 'F?'. Use explicit '?.invoke' to make a function-like call instead.` | E `UNSAFE_IMPLICIT_INVOKE_CALL`: `f()` with `f: (() -> Unit)?` | reference-name | `Replace with safe (?.) call` |
| KT-DIAG-041 | MUST | `Unnecessary safe call on a non-null receiver of type 'String'.` | W `UNNECESSARY_SAFE_CALL`: `s?.length` with `s: String` | safe-access (`?.`) | `Replace with dot call` |
| KT-DIAG-042 | MUST | `Unnecessary non-null assertion (!!) on a non-null receiver of type 'String'.` | W `UNNECESSARY_NOT_NULL_ASSERTION` | operator (`!!`) | `Remove unnecessary non-null assertion (!!)` |
| KT-DIAG-043 | MUST | `Elvis operator (?:) always returns the left operand of non-nullable type 'String'.` ; `Elvis operator (?:) is redundant if the right operand is always null.` ; `... if the left operand is always null.` | W `USELESS_ELVIS`, `USELESS_ELVIS_RIGHT_IS_NULL`, `USELESS_ELVIS_LEFT_IS_NULL`; grey look | from `?:` to end of the expression (left-is-null: from start to `?:`) | `Remove redundant elvis operator` |
| KT-DIAG-044 | MUST | `Cast is redundant.` | W `USELESS_CAST`; grey look | from `as` to end of expression | `Remove redundant cast` |
| KT-DIAG-045 | MUST | `Check for instance is always 'true'.` (`false` for the impossible form) | W `USELESS_IS_CHECK`; grey look | default | `Remove redundant 'is' check` |
| KT-DIAG-046 | MUST | `Unchecked cast of 'Any' to 'List<String>'.` | W `UNCHECKED_CAST` | from `as` to end of expression | none; suppress options (3.4) |
| KT-DIAG-047 | SHOULD | `This cast can never succeed.` | W `CAST_NEVER_SUCCEEDS` | operator | `Replace cast with '.toX()'` for numeric casts (message `This cast can never succeed. Use 'toInt()' to perform numeric conversion.`) |
| KT-DIAG-048 | MUST | `Smart cast to 'String' is impossible, because 'x' is a <reason>.` where reason is a phrase such as `mutable property that could have been changed by this time`, `property that has open or custom getter`, `local variable that is captured by a changing closure` | E `SMARTCAST_IMPOSSIBLE` | default | `Cast expression 'x' to 'String'`, `Introduce local variable` / `Replace 'if' expression with elvis expression`-style fixes where applicable |
| KT-DIAG-049 | SHOULD | `Condition is always 'true'.` / `'false'` | W `SENSELESS_COMPARISON`: `x == null` for non-null `x` | default | `Simplify comparison` |
| KT-DIAG-050 | SHOULD | `Division by zero.` | W `DIVISION_BY_ZERO` | default | none |
| KT-DIAG-051 | SHOULD | `Cannot infer type for type parameter 'T'. Specify it explicitly.` | E `CANNOT_INFER_PARAMETER_TYPE` | default | `Specify type arguments explicitly`, `Import ...` |
| KT-DIAG-052 | SHOULD | `No type arguments expected for class Foo` / `One type argument expected for class List<out E>` / `2 type arguments expected for class Map<K, V>` (`{0}` = count, then ` for '{1}'` with the declaration) | E `WRONG_NUMBER_OF_TYPE_ARGUMENTS` | type argument list or the type itself | `Add star projections`, `Change type arguments` |
| KT-DIAG-053 | SHOULD | `Type argument is not within its bounds: type parameter 'T' must be subtype of 'Number', but actual: 'String'.` | E `UPPER_BOUND_VIOLATED` | default | `Add 'String' as upper bound for T` where applicable |
| KT-DIAG-054 | SHOULD | `Cannot check for instance of erased type 'List<String>'.` | E `CANNOT_CHECK_FOR_ERASED` | default | `Change to star projection`, `Add star projections`, `Make T reified and f inline` |

### 3.3.3 Calls and arguments

| ID | Pri | Message | Trigger | Range | Fixes |
|---|---|---|---|---|---|
| KT-DIAG-060 | MUST | `No value passed for parameter 'b'.` | E `NO_VALUE_FOR_PARAMETER` | reference-name (callee) | `Add parameter name ...`, `Specify remaining required arguments by name`, `Add constructor parameter`, `Import ...` |
| KT-DIAG-061 | MUST | `Too many arguments for 'fun foo(a: Int): Unit'.` | E `TOO_MANY_ARGUMENTS` | default (the first extra argument) | `Remove argument`, `Change the signature of function 'foo'`, `Add 2nd parameter to function 'foo'`, `Import ...` |
| KT-DIAG-062 | MUST | `No parameter with name 'x' found.` | E `NAMED_PARAMETER_NOT_FOUND` | named-arg | `Rename ...` , `Import ...` |
| KT-DIAG-063 | SHOULD | `Argument already passed for this parameter.` | E `ARGUMENT_PASSED_TWICE` | named-arg | none |
| KT-DIAG-064 | SHOULD | `Mixing named and positional arguments is not allowed unless the order of the arguments matches the order of the parameters.` | E `MIXING_NAMED_AND_POSITIONAL_ARGUMENTS` | default | `Add name to argument...` |
| KT-DIAG-065 | MUST | `For-loop range must have an 'iterator()' method.` | E `ITERATOR_MISSING` | default | `Import ...` (an `iterator` extension) |
| KT-DIAG-066 | MUST | `Destructuring of type 'Foo' requires operator function 'component1()'.` | E `COMPONENT_FUNCTION_MISSING` | default | `Import ...`, `Make 'Foo' data class` / `Add 'data' modifier` |
| KT-DIAG-067 | MUST | `No 'get' operator method providing array access.` / `No 'set' operator method providing array access.` | E `NO_GET_METHOD`, `NO_SET_METHOD` | index (`[...]`) | `Import ...`, `Change type to mutable` |
| KT-DIAG-068 | SHOULD | `Type 'X' has no method 'getValue', so it cannot serve as a delegate.` (`{2}`: `delegate`/`property delegate`) | E `DELEGATE_SPECIAL_FUNCTION_MISSING` | `by` keyword (property delegate) | `Import ...` |
| KT-DIAG-069 | SHOULD | `Suspend function 'f' can only be called from a coroutine or another suspend function.` | E `ILLEGAL_SUSPEND_FUNCTION_CALL` | reference-name | `Make 'g' suspend` (`Make containing function suspend`) |
| KT-DIAG-070 | SHOULD | `'return' is prohibited here.` | E `RETURN_NOT_ALLOWED`: unlabeled/ wrong `return` in a lambda | return keyword (+ label) | `Change to 'return@label'`, `Remove 'return'` |
| KT-DIAG-071 | SHOULD | `'break' and 'continue' are only allowed inside loops.` | E `BREAK_OR_CONTINUE_OUTSIDE_A_LOOP` | default | none |
| KT-DIAG-072 | SHOULD | `Cannot inline 'f' here: it might contain non-local returns. Add 'crossinline' modifier to parameter declaration 'f'.` | E `NON_LOCAL_RETURN_NOT_ALLOWED` | reference | `Add 'crossinline' to parameter` |

### 3.3.4 Declarations, overriding, modifiers

| ID | Pri | Message | Trigger | Range | Fixes |
|---|---|---|---|---|---|
| KT-DIAG-080 | MUST | `Missing return statement.` | E `NO_RETURN_IN_FUNCTION_WITH_BLOCK_BODY`: a block-body function with a non-Unit return type whose end is reachable | body-end (the closing `}`) | `Add 'return' to last expression`, `Change return type to 'Unit'`... (`Remove explicitly specified return type`) |
| KT-DIAG-081 | MUST | `Variable 'x' must be initialized.` | E `UNINITIALIZED_VARIABLE`: read before assignment | default (the reference) | `Initialize variable` (`Add initializer`) |
| KT-DIAG-082 | MUST | `Property must be initialized.` ; `Property must be initialized or be abstract.` | E `MUST_BE_INITIALIZED`, `MUST_BE_INITIALIZED_OR_BE_ABSTRACT` | signature (without modifiers) | `Add initializer`, `Initialize with constructor parameter`, `Add 'lateinit' modifier`, `Make 'abstract'`, `Change to 'val'`... |
| KT-DIAG-083 | MUST | `'val' cannot be reassigned.` | E `VAL_REASSIGNMENT` | selector / left side | `Change to 'var'` (`Change to 'var'` plus ` and delete initializer` when it has none to keep), `Assign to property` |
| KT-DIAG-084 | MUST | `'when' expression must be exhaustive. Add the 'A', 'B' branches or an 'else' branch.` (one missing: `Add the 'A' branch or an 'else' branch.`; unknown subject: `Add an 'else' branch.`; at most 7 names are listed, then `...`) | E `NO_ELSE_IN_WHEN`: `when` used as an expression, or a statement over enum/sealed/Boolean | `when` keyword | `Add remaining branches`, `Add 'else' branch` (`Add else branch`), `Add remaining branches with * import` |
| KT-DIAG-085 | MUST | `'if' must have both main and 'else' branches when used as an expression.` | E `INVALID_IF_AS_EXPRESSION` | `if` keyword | `Add else branch` |
| KT-DIAG-086 | MUST | `Conflicting declarations:` + one declaration per line ; `Conflicting overloads:` + one per line ; `Redeclaration:` + one per line (classifiers) | E `REDECLARATION`, `CONFLICTING_OVERLOADS`, `CLASSIFIER_REDECLARATION`: same name (and same JVM-erased signature for functions) twice in a scope | name / signature (without modifiers) | `Show conflicting declarations` |
| KT-DIAG-087 | MUST | `Class 'C' is not abstract and does not implement abstract member:\n<member per line>` (plural: `members:`) | E `ABSTRACT_MEMBER_NOT_IMPLEMENTED` | name (class: from its first modifier to the name) | `Implement members`, `Make 'C' abstract` |
| KT-DIAG-088 | MUST | `'foo' overrides nothing.` followed, when the compiler knows near matches, by ` Potential signatures for overriding:` and one signature per line | E `NOTHING_TO_OVERRIDE` | `override` modifier | `Remove 'override' modifier`, `Change function signature...` |
| KT-DIAG-089 | MUST | `'foo' hides member of supertype 'A' and needs an 'override' modifier.` | E `VIRTUAL_MEMBER_HIDDEN` | name | `Add 'override' modifier` |
| KT-DIAG-090 | MUST | `'foo' in 'A' is final and cannot be overridden.` | E `OVERRIDING_FINAL_MEMBER` | `override` modifier | `Make 'foo' in A open` |
| KT-DIAG-091 | MUST | `This type has a constructor, so it must be initialized here.` | E `SUPERTYPE_NOT_INITIALIZED`: `class B : A` | default (the type) | `Change to constructor invocation` |
| KT-DIAG-092 | MUST | `This type is final, so it cannot be extended.` | E `FINAL_SUPERTYPE` | default | `Make 'A' open` |
| KT-DIAG-093 | SHOULD | `Return type of 'foo' is not a subtype of the return type of the overridden member 'fun foo(): Int'.` | E `RETURN_TYPE_MISMATCH_ON_OVERRIDE` | declaration return type | `Change return type of ... to ...` |
| KT-DIAG-094 | SHOULD | `'C' must override 'foo' because it inherits multiple implementations for it.` (interfaces variant: `... multiple interface methods for it.`) | E `MANY_IMPL_MEMBER_NOT_IMPLEMENTED`, `MANY_INTERFACES_MEMBER_NOT_IMPLEMENTED` | name | `Specify override for 'foo' explicitly` |
| KT-DIAG-095 | SHOULD | `Modifier 'x' is not applicable to 'y'.` / `Repeated 'x'.` / `Modifier 'a' is incompatible with 'b'.` / `Modifier 'x' is redundant in presence of 'y'.` (W) | E `WRONG_MODIFIER_TARGET`, `REPEATED_MODIFIER`, `INCOMPATIBLE_MODIFIERS`; W `REDUNDANT_MODIFIER` | default (the modifier) | `Remove 'x' modifier`, `Make 'a' not b` |
| KT-DIAG-096 | SHOULD | `Abstract function 'f' in non-abstract class 'C'.` / `Abstract property 'p' in non-abstract class 'C'.` | E `ABSTRACT_FUNCTION_IN_NON_ABSTRACT_CLASS`, `ABSTRACT_PROPERTY_IN_NON_ABSTRACT_CLASS` | modality modifier | `Make 'C' abstract`, `Remove 'abstract' modifier` |
| KT-DIAG-097 | SHOULD | `This property must have an explicit type, be initialized, or be delegated.` ; `This variable must either have an explicit type or be initialized.` | E `PROPERTY_WITH_NO_TYPE_NO_INITIALIZER`, `VARIABLE_WITH_NO_TYPE_NO_INITIALIZER` | signature / name | `Specify type explicitly`, `Add initializer` |
| KT-DIAG-098 | SHOULD | `Data class must have at least one primary constructor parameter.` ; `Primary constructor of data class must only have property ('val' / 'var') parameters.` | E `DATA_CLASS_WITHOUT_PARAMETERS`, `DATA_CLASS_NOT_PROPERTY_PARAMETER` | name / default | `Add 'val' or 'var' to parameter 'x'` |
| KT-DIAG-099 | SHOULD | `Explicit 'this' or 'super' call is required. There is no constructor in the superclass that can be called without arguments.` ; `Primary constructor call expected.` | E `EXPLICIT_DELEGATION_CALL_REQUIRED`, `PRIMARY_CONSTRUCTOR_DELEGATION_CALL_EXPECTED` | secondary constructor signature | `Insert 'super()' call` / `Insert 'this()' call` |
| KT-DIAG-100 | SHOULD | `'infix' modifier is required on 'fun foo(...)'.` ; `'operator' modifier is required on <member>.` | E `INFIX_MODIFIER_REQUIRED`, `OPERATOR_MODIFIER_REQUIRED` | default | `Add 'infix' modifier`, `Add 'operator' modifier` |
| KT-DIAG-101 | SHOULD | `Const 'val' initializer must be a constant value.` ; `Incorrect const expression.` | E `CONST_VAL_WITH_NON_CONST_INITIALIZER`, `ILLEGAL_CONST_EXPRESSION` | default | `Remove 'const' modifier` |
| KT-DIAG-102 | SHOULD | `'lateinit' modifier ...` (`is not allowed on properties of a type with nullable upper bound`, `... on primitive type properties`, `... on a property with initializer`, ...) | E `INAPPLICABLE_LATEINIT_MODIFIER` | `lateinit` modifier | `Remove 'lateinit' modifier`, `Change to 'var'` |
| KT-DIAG-103 | SHOULD | `This extension is shadowed by a member: public final fun foo(): Unit.` | W `EXTENSION_SHADOWED_BY_MEMBER` | name | none |
| KT-DIAG-104 | SHOULD | `Redundant visibility modifier.` | W `REDUNDANT_VISIBILITY_MODIFIER` (shown through the inspection `RedundantVisibilityModifier`) | modifier | `Remove redundant visibility modifier` |
| KT-DIAG-105 | SHOULD | `'else' entry must be the last one in a 'when' expression.` ; `'when' is exhaustive so 'else' is redundant here.` (W) ; `Duplicate branch condition in 'when'.` (W) ; `Expression under 'when' is never equal to null.` (W) | E `ELSE_MISPLACED_IN_WHEN`; W `REDUNDANT_ELSE_IN_WHEN`, `DUPLICATE_BRANCH_CONDITION_IN_WHEN`, `SENSELESS_NULL_IN_WHEN` | `else` keyword / default | `Move 'else' branch to the end`, `Remove branch` |
| KT-DIAG-106 | SHOULD | `'fun oldFun(): Unit' is deprecated. Use newFun instead.` (`{0}` = declaration head as in 3.3.7; `{1}` = the deprecation message trimmed, preceded by a space and with a final period added; omitted when empty) | W `DEPRECATION` (E for level ERROR: `DEPRECATION_ERROR`, same text); strikethrough | reference-name (a type reference: its selector) | `Replace usages of ... in whole project` / `Replace with ...` (ReplaceWith, LATER) |
| KT-DIAG-107 | SHOULD | `Opt-in` messages: the requirement's own message text (`This declaration needs opt-in. Its usage should be marked with '@X' or '@OptIn(X::class)'`) | W/E `OPT_IN_USAGE`, `OPT_IN_USAGE_ERROR` | reference | `Opt in for 'X' on 'foo'`, `Opt in for 'X' on containing class 'C'`, `Opt in for 'X' in containing file 'F.kt'`, `Opt in for 'X' in module 'm'`, `Propagate 'X' opt-in requirement to 'foo'`, ... (texts from `fix.opt_in.*`) |
| KT-DIAG-108 | SHOULD | `Unreachable code.` | W `UNREACHABLE_CODE`, shown through inspection `KotlinUnreachableCode` (05) | the unreachable statements | `Remove unreachable code` |
| KT-DIAG-109 | SHOULD | `This 'var' property is never written to, it can be declared as 'val'.` | W `CAN_BE_VAL`, shown through inspection `CanBeVal` with the inspection's text (05) | `val`/`var` keyword | `Change to 'val'` |

### 3.3.5 JVM-specific (Kotlin/JVM modules)

| ID | Pri | Message | Trigger | Range | Fixes |
|---|---|---|---|---|---|
| KT-DIAG-120 | SHOULD | `Only members in named objects and companion objects of classes can be annotated with '@JvmStatic'.` | E `JVM_STATIC_NOT_IN_OBJECT_OR_CLASS_COMPANION` | the annotation | `Remove '@JvmStatic' annotation` |
| KT-DIAG-121 | SHOULD | `'@JvmName' annotation is not applicable to this declaration.` ; `Illegal JVM name.` | E `INAPPLICABLE_JVM_NAME`, `ILLEGAL_JVM_NAME` | the annotation | `Remove '@JvmName' annotation` |
| KT-DIAG-122 | SHOULD | `'@JvmOverloads' annotation cannot be used on abstract methods.` / `... has no effect on private declarations.` / `... cannot be used on interface methods.` / `... cannot be used on local declarations.` | E `OVERLOADS_ABSTRACT`, W `OVERLOADS_PRIVATE`, E `OVERLOADS_INTERFACE`, `OVERLOADS_LOCAL` | the annotation | `Remove '@JvmOverloads' annotation` |
| KT-DIAG-123 | SHOULD | `'@JvmStatic' annotation is redundant for const or '@JvmField' properties.` | W `JVM_STATIC_ON_CONST_OR_JVM_FIELD` | the annotation | `Remove '@JvmStatic' annotation` |
| KT-DIAG-124 | SHOULD | `This class is not recommended for use in Kotlin. Use 'kotlin.collections.ArrayList' instead.` (message pattern; the class named is the Kotlin analogue) | W `PLATFORM_CLASS_MAPPED_TO_KOTLIN`: use of `java.util.ArrayList`-like types where a Kotlin type exists | reference-name | `Replace with 'kotlin.X'` |
| KT-DIAG-125 | LATER | `Platform declaration clash: The following declarations have the same JVM signature (foo()V):` followed by the declarations | E JVM backend diagnostic `CONFLICTING_JVM_DECLARATIONS` (reported by the IDE through light-class analysis) | name | `Rename`, `Add @JvmName` (see 07) |

### 3.3.6 Other diagnostics (not itemised)

The compiler has 851 error/warning kinds. Every kind not listed above is reported with the compiler's own message, severity and
range (`FirErrorsDefaultMessages`, `FirErrors`), with **no IDE fix** unless `KotlinK2QuickFixRegistrar` registers one. A complete
index (kind, severity, range rule, message) is machine-generated from those two sources (Q-DIAG-1).

`KT-DIAG-130` MUST: every compiler diagnostic that the reference compiler (Kotlin 2.4, K2) reports for a file is reported by the engine
with that message template, severity and marked range; kinds not itemised may be delivered later but never with a different text.
Acceptance: differential test over the compiler's own diagnostic test corpus.

`KT-DIAG-131` MUST: keep the compiler's rendering conventions: types shown in short, user-readable form (`List<String>`, `String?`,
`(Int) -> Unit`), declarations shown as Kotlin source heads (`fun foo(a: Int): Unit`, `val x: Int`), one declaration per line after
a `:`-terminated header (`Conflicting declarations:`), quoted names with single quotes. Acceptance: cases 03.

## 3.3.7 Rendering of declarations and types in messages

Arguments that are *symbols* (`{0}` of `Too many arguments for ...`, `Cannot access ...`, `... is deprecated`, candidate lists) are
rendered as the **declaration head in Kotlin syntax**, without body, annotations and (mostly) without visibility:
`fun foo(a: Int, b: String = ...): Unit`, `val x: Int`, `class Foo`, `constructor(a: Int)`, `fun String.ext(): Int`; a few modifiers that matter to the
reader (`abstract`, `open`, `override`, `operator`, `infix`, `suspend`, `inline`...) are kept; the "member of containing declaration" form adds
the owner (`public final fun foo(): Unit` inside `A`). A parameter default is printed as `...`. *Types* are rendered with simple names unless two
different types would print the same name (then FQ names), nullable as `T?`, function types as `(A, B) -> R`, extension function types as
`A.(B) -> R`, captured and flexible types in their user form (`String!` for a platform type). *Candidate lists*
put each candidate on its own line after the header line.

`KT-DIAG-132` SHOULD: this rendering; identical output for identical programs. Acceptance: cases 03/9, 10, 15.

## 3.4 Suppression and the "compiler warning" intention

A warning (never an error) can be suppressed with the standard `@Suppress("NAME")` annotation where NAME is the diagnostic
factory name in **upper-case** (`UNCHECKED_CAST`, `DEPRECATION`, `UNUSED_VARIABLE`, ...), or an inspection short name (`UnusedSymbol`); the
annotation may be on the statement, a declaration, a class or the file (`@file:Suppress(...)`). The Alt+Enter popup of a
warning offers a group of options `Suppress 'NAME' for statement` / `for fun foo` / `for class C` / `for file F.kt` (the kind word is one
of `statement`, `initializer`, `object`, `companion object`, `file`, `secondary constructor of`, `enum entry`, `type parameter`, `class`,
`interface`, `fun`, `val`, `var`, `parameter` followed by the declaration name) and the item `Compiler warning 'NAME' options`. Errors have no suppress
group (the factory name must not be in the non-suppressible list: syntax errors and a set of "must not be suppressed" errors).
Applying `Suppress ... for statement` writes `@Suppress("NAME")` before the statement (`@Suppress("UNCHECKED_CAST")` on a line of its own
or inline before the expression when the statement is a part of a larger expression).

`KT-DIAG-140` MUST: the suppress annotation honoured at statement, declaration, class and file level; the popup rows as described;
errors not suppressible. Acceptance: cases 03/32, 33.

## 3.5 Ordering of fixes

Fixes on one highlight are ordered by the platform's rule (`codeinsight/04-intentions-and-alt-enter.md`): priority first
(`TOP` for the import fix, normal, then `LOW` such as `Add non-null asserted (!!) call`), then registration order of the
registrar (which is the order of the table rows of 3.3 for one kind). Intentions that apply at the caret follow the error fixes.

## Open questions

- Q-DIAG-1: generating the full index of the 851 kinds into `docs/specs/kotlin/diagnostics-index.md` (script is not committed; the
  facts are the compiler's). Until then the implementation uses the compiler version it targets as the oracle.
- Q-DIAG-2: ranges for `INITIALIZER_TYPE_MISMATCH` / `ASSIGNMENT_TYPE_MISMATCH` when the reported element is the whole declaration or
  assignment (rule `initializer`/`operator`); the common case marks the right-hand side expression. Confirm with differential tests.
- Q-DIAG-3: exact `{2}` reason phrases of `SMARTCAST_IMPOSSIBLE` (computed by the compiler's smart-cast stability classification).

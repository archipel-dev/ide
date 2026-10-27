# 05. Kotlin inspections

Status: specification, clean-room, our own words. Requirement prefix `KT-INS`. Executable cases:
[cases/05-inspections.cases.md](cases/05-inspections.cases.md).

ic: `plugins/kotlin/code-insight/inspections-k2/src/org/jetbrains/kotlin/idea/codeInsight/inspections/**` (K2 implementations),
`plugins/kotlin/code-insight/inspections-k2/resources/intellij.kotlin.codeInsight.inspections.xml` (registration: short name, level,
default state, group), `plugins/kotlin/code-insight/descriptions/resources-en/inspectionDescriptions/*.html` (descriptions with examples),
`plugins/kotlin/base/resources/resources-en/messages/KotlinBundle.properties` (messages, fix texts, option labels),
`plugins/kotlin/highlighting/highlighting-k2/src/org/jetbrains/kotlin/idea/highlighting/{K2UnusedSymbolUtil,KotlinUnusedHighlightingProcessor}.kt`.
Inventory: `docs/specs/inventory/inspections-kotlin-1..4.md` (every inspection the Kotlin plugin registers: 325 rows), listed in Appendix A with a
priority.

The inspection *framework* (profiles, severities, options, suppression, batch mode, quick-fix application, "Fix all") is
`codeinsight/03-inspections.md`. Kotlin language semantics are the Kotlin specification's (<https://kotlinlang.org/spec/>) and the
standard library's documented contracts; this file specifies what is reported, with which text and where, and which fix is offered.

---

## 5.1 How Kotlin inspections behave

- Each inspection has a **short name** (the id used in profiles and in `@Suppress("ShortName")`), a **display name**, a **group**
  (the plugin's groups: `Redundant constructs`, `Style issues`, `Probable bugs`, `Naming conventions`, `Other problems`,
  `Java interop issues`, `Numeric issues`, `Logging`, `Migration`, `Code migration`, `Coroutine inspections`, ...), a **default level**
  and **enabled by default** flag (Appendix A).
- Levels: `ERROR` (red), `WARNING` (yellow), `WEAK WARNING` (subtle underline; shown in the editor like a hint on hover with the lightbulb), `INFORMATION`/`INFO`
  (no editor mark; the fix is available as a lightbulb item and in the Alt+Enter list when the caret is in the range). Levels are profile settings.
- Kind `local` = runs on the file as you type; `global` = also needs a project-wide search and so runs in batch ("Inspect Code")
  and, for the cases noted, in the editor through a dedicated highlighter (5.5); `+cleanup` = the fix may be applied by *Code | Code Cleanup*.
- Each problem has a **message** (tooltip), a **range**, and one or more **fixes**. A fix text is what the Alt+Enter list shows; fixes of a
  group are applicable to all problems of the file via `Fix all '<name>' problems in file` (framework).
- **Diagnostic-based inspections** (`03` 3.1) take the compiler's finding and give it the inspection's own message and fix.
- Suppression: `@Suppress("ShortName")` on the statement, declaration, class or file; the Alt+Enter option `Suppress for ...` is offered (`03` 3.4).
- Every inspection must be silent on code that does not compile in the way that makes its premise false (unresolved types, error
  expressions): it reports only when the types it needs are fully resolved.

`KT-INS-001` MUST: the descriptor of each inspection of Appendix A that is implemented: short name, display name, group, default level,
default enabled, kind. Acceptance: the inspection profile lists them with these values; switching a level or off takes effect on the next daemon cycle.

`KT-INS-002` MUST: silence on unresolved premises and on generated/incomplete code. Acceptance: for each case in `cases/05` the same text with one of its
referenced types replaced by an unresolved name produces no problem.

`KT-INS-003` MUST: `Fix all` and `Cleanup` apply a `local+cleanup` fix over the file in one undo step; a fix never changes behaviour except where
its text says `(may change semantics)`. Acceptance: case 05 `KT-INS-003`.

## 5.2 Option labels used by Kotlin inspections

| Inspection | Option label (exact) | Default |
|---|---|---|
| `UsePropertyAccessSyntax` | `Report non-trivial accessors` | off |
| `UnnecessaryVariable` | `Report immediately returned variables` | off |
| `JoinDeclarationAndAssignment` | `Report with complex initialization of member properties` | on |
| `BooleanLiteralArgument` | `Report also on call with single boolean literal argument` | off |
| `RedundantIf` | `Ignore chained 'if' statements` | on |
| `HasPlatformType` | `Report for types with platform arguments`; `Apply only to public or protected members` | off; on |
| `UnclearPrecedenceOfBinaryExpression` | `Report even obvious cases` | off |
| `TrailingComma` | `Report also a missing comma or a line break` | off |
| `MemberVisibilityCanBePrivate` | `Apply also to private members`/`Apply also to internal members` | off |
| `LiftReturnOrAssignment` | `Report only if each branch is a single statement` | on |
| `PackageDirectoryMismatch` | `Report packages that rely on the implicit package prefix of a source root` | off |
| `IntroduceWhenSubject` | `Do not highlight when guards are required` | on |
| `DestructuringDeclaration` | `Maximum number of unused components in destructuring declaration` | (number) |
| `KotlinConstantConditions` | `Warn when constant is stored in variable` | on |
| `ClassName`, `FunctionName`, `PropertyName`, ... (naming) | `Pattern:` regular expression | per inspection, 5.3 notes |
| `MapToForEach` | `Report also on statement` | off |

(Defaults marked `(number)` are not specified; others are shipped defaults.)

## 5.3 Catalog of the daily inspections

Columns: ID; priority; short name; the **message** shown (backticks; `<...>` filled with the actual names); the **range** marked;
the **fix**: the row text of the Alt+Enter list (alternatives separated by ` / `); `Trigger` points to the case in `cases/05` and gives notes.
Level and default state are in Appendix A. "(none)" = no automatic fix.

| ID | Pri | Short name | Message | Range | Fix | Trigger |
|---|---|---|---|---|---|---|
| KT-INS-010 | MUST | `UnusedSymbol` | `<Kind> '<name>' is never used` (Kind: Class, Interface, Object, Function, Property, Parameter, Constructor, Type alias, Enum entry, Type parameter) | name identifier; grey (unused-symbol look) | Safe delete '<name>' | case KT-INS-010 (non-private declarations only when no usage is found by search (5.5); entry points exempt) |
| KT-INS-011 | MUST | `UnusedVariable` | `Unused variable` | variable name | Remove variable '<name>' (simple initializer) / Remove variable '<name>' (may change semantics) | case KT-INS-011 (diagnostic-based (03 3.1)) |
| KT-INS-012 | MUST | `VariableNeverRead` | `Variable 'local' is never read` | variable name | (no automatic fix beyond the platform's suppress options) | case KT-INS-012 (a variable that is assigned but never read; diagnostic-based) |
| KT-INS-013 | MUST | `AssignedValueIsNeverRead` | `Assigned value is never read` | the assignment | Remove redundant assignment | case KT-INS-013 (if the right-hand side may have side effects it is kept as a statement; otherwise the assignment is deleted) |
| KT-INS-014 | MUST | `VariableInitializerIsRedundant` | `Variable initializer is redundant` | the initializer | Remove redundant initializer | case KT-INS-014 ([verify: explicit type added when it cannot be inferred any more]) |
| KT-INS-015 | MUST | `CanBeVal` | `Variable is never modified, so it can be declared using 'val'` | the `var` keyword | Change to 'val' | case KT-INS-015 (local variables only; diagnostic-based) |
| KT-INS-016 | MUST | `RedundantExplicitType` | `Explicitly given type is redundant here` | the type reference (and colon) | Remove explicit type specification | case KT-INS-016 (flagged only when the type equals the type of an obvious initializer (literal, constructor call, `true`/`false`, string template, class literal); not for `Long` literals needing the suffix) |
| KT-INS-017 | MUST | `RedundantVisibilityModifier` | `Redundant visibility modifier` | the modifier | Remove redundant visibility modifier | case KT-INS-017 (`public` on a top-level or member declaration; `protected` on an override of a protected member; diagnostic-based) |
| KT-INS-018 | MUST | `RedundantModalityModifier` | `Redundant modality modifier` | the modifier | Remove redundant modality modifier | case KT-INS-018 (`final` on a non-open member, `open` on an override member, `abstract` on interface members) |
| KT-INS-019 | MUST | `RedundantUnitReturnType` | `Redundant 'Unit' return type` | the return type | Remove redundant 'Unit' return type | case KT-INS-019 |
| KT-INS-020 | MUST | `RedundantUnitExpression` | `Redundant 'Unit'` | the `Unit` expression | Remove redundant 'Unit' | case KT-INS-020 |
| KT-INS-021 | MUST | `RedundantSemicolon` | `Redundant semicolon` | the semicolon | Remove redundant semicolon | case KT-INS-021 (semicolons that separate statements on one line or follow enum entries with members are not flagged) |
| KT-INS-022 | MUST | `RedundantLambdaArrow` | `Redundant lambda arrow` | the arrow | Remove arrow | case KT-INS-022 |
| KT-INS-023 | MUST | `RedundantSamConstructor` | `Redundant SAM constructor` | the SAM constructor name | Remove redundant SAM constructor | case KT-INS-023 (the lambda stays in parentheses; `MoveLambdaOutsideParentheses` may then fire) |
| KT-INS-024 | MUST | `RemoveRedundantQualifierName` | `Redundant qualifier name` | the redundant qualifier | Remove redundant qualifier name | case KT-INS-024 |
| KT-INS-025 | MUST | `RedundantCompanionReference` | `Redundant Companion reference` | the `Companion` segment | Remove redundant Companion reference | case KT-INS-025 |
| KT-INS-026 | MUST | `RemoveCurlyBracesFromTemplate` | `Redundant curly braces in string template` | the entry `${x}` | Remove curly braces | case KT-INS-026 (not flagged when the following character is an identifier character) |
| KT-INS-027 | MUST | `ConstantConditionIf` | `Condition of 'if' expression is constant` | the condition | Simplify expression | case KT-INS-027 |
| KT-INS-028 | MUST | `KotlinConstantConditions` | `Condition 'x != null' is always 'true'` / `Condition is always 'true'` / `Value is always null` / `'when' branch is never reachable` / `Cast will always fail` ... | the condition or value | Simplify expression / Remove branch (per case) | case KT-INS-028 (data-flow based; option `Warn when constant is stored in variable`) |
| KT-INS-029 | MUST | `KotlinUnreachableCode` | `Unreachable code` | each unreachable statement | Remove unreachable code | case KT-INS-029 (diagnostic-based) |
| KT-INS-030 | MUST | `UseExpressionBody` | `Use expression body instead of one-line 'return'` / `... of 'return when'` / `... of block body` | the `{` of the body or the `return` | Convert to expression body | case KT-INS-030 (severity INFORMATION (no underline, intention-like lightbulb)) |
| KT-INS-031 | MUST | `UsePropertyAccessSyntax` | `Use of getter method instead of property access syntax` / `Use of setter method instead of property access syntax` | the call's name | Use property access syntax | case KT-INS-031 (Java accessors only; option `Report non-trivial accessors` (off); excluded-methods list) |
| KT-INS-032 | MUST | `ConvertToStringTemplate` | `'String' concatenation can be converted to a template` | the whole concatenation | Convert concatenation to template | case KT-INS-032 |
| KT-INS-033 | MUST | `CascadeIf` | `Cascade 'if' should be replaced with 'when'` | the first `if` keyword | Replace 'if' with 'when' | case KT-INS-033 (at least three branches) |
| KT-INS-034 | MUST | `IfThenToElvis` | `If-Then foldable to '?:'` | the `if` keyword | Replace 'if' expression with elvis expression | case KT-INS-034 |
| KT-INS-035 | MUST | `IfThenToSafeAccess` | `If-Then foldable to '?.'` | the `if` keyword | Replace 'if' expression with safe access expression | case KT-INS-035 |
| KT-INS-036 | MUST | `FoldInitializerAndIfToElvis` | `If-Null return/break/... foldable to '?:'` | the `if` keyword | Replace 'if' with elvis operator | case KT-INS-036 |
| KT-INS-037 | MUST | `LiftReturnOrAssignment` | `'return' can be lifted out of 'when'` / `Assignment can be lifted out of 'if'` | the `when`/`if`/`try` keyword | Lift return out of 'when' | case KT-INS-037 (option: report only if each branch is a single statement (default on)) |
| KT-INS-038 | MUST | `JoinDeclarationAndAssignment` | `Join declaration and assignment` | the declaration name | Join with initializer | case KT-INS-038 |
| KT-INS-039 | MUST | `ReplaceSizeZeroCheckWithIsEmpty` | `Size zero check can be replaced with 'isEmpty()'` | the comparison | Replace size zero check with 'isEmpty' | case KT-INS-039 |
| KT-INS-040 | MUST | `ReplaceSizeCheckWithIsNotEmpty` | `Size check can be replaced with 'isNotEmpty()'` | the comparison | Replace size check with 'isNotEmpty' | case KT-INS-040 |
| KT-INS-041 | MUST | `MoveLambdaOutsideParentheses` | `Lambda argument inside parentheses` | the lambda | Move lambda argument out of parentheses | case KT-INS-041 |
| KT-INS-042 | MUST | `ReplaceGetOrSet` | `Explicit 'get' or 'set' call` | the call | Replace get or set call with indexing operator | case KT-INS-042 |
| KT-INS-043 | MUST | `ReplacePutWithAssignment` | `map.put() should be converted to assignment` | the call | Convert put to assignment | case KT-INS-043 |
| KT-INS-044 | MUST | `ClassName` | `Class name 'foo' should start with an uppercase letter` | the class name | (Rename) | case KT-INS-044 (pattern setting default `[A-Z][A-Za-z\d]*`; violation phrases: `should start with an uppercase letter`, `should not contain underscores`, `may contain only letters and digits`, `doesn't match regex '<pattern>'`) |
| KT-INS-045 | MUST | `FunctionName` | `Function name 'Foo' should start with a lowercase letter` | the function name | (Rename) | case KT-INS-045 (default pattern `[a-z][A-Za-z\d]*`; a function whose name equals a class's name and which returns that type (factory) is allowed; test functions in test source sets use the `TestFunctionName` rules) |
| KT-INS-046 | MUST | `PropertyName` | `Property name 'Foo' should start with a lowercase letter` | the property name | (Rename) | case KT-INS-046 (default pattern `[a-z][A-Za-z\d]*`; `const val` uses `[A-Z][_A-Z\d]*`; private properties allow a leading underscore) |
| KT-INS-047 | MUST | `LocalVariableName` | `Local variable name 'Foo' should start with a lowercase letter` | the name | (Rename) | case KT-INS-047 (default pattern `[a-z][A-Za-z\d]*`) |
| KT-INS-048 | MUST | `PackageDirectoryMismatch` | `Package directive does not match the file location` | the package directive | Move file to package-matching directory / Change file's package to match directory | case KT-INS-048 (option: report packages that rely on the implicit package prefix of a source root; fix texts `Move file to <dir>` and `Change file's package to <pkg>`) |
| KT-INS-049 | MUST | `UnusedReceiverParameter` | `Receiver parameter is never used` | the receiver type | Remove redundant receiver parameter | case KT-INS-049 (also applies to extension properties; usages are updated) |
| KT-INS-050 | MUST | `SelfAssignment` | `Variable 'bar' is assigned to itself` | the assignment | Remove self assignment | case KT-INS-050 |
| KT-INS-051 | MUST | `UselessCallOnNotNull` | `Redundant call on not-null type` | the call | Remove redundant call | case KT-INS-051 (`isNullOrEmpty()` on a non-null receiver is replaced by `isEmpty()` (fix `Change call to 'isEmpty'`)) |
| KT-INS-052 | SHOULD | `RedundantReturnKeyword` | `Redundant 'return' keyword` | the `return` keyword | Remove 'return' keyword | case KT-INS-052 |
| KT-INS-053 | SHOULD | `RemoveRedundantBackticks` | `Redundant backticks` | the backticked name | Remove redundant backticks | case KT-INS-053 (names that are keywords keep their backticks) |
| KT-INS-054 | SHOULD | `ReplaceToWithInfixForm` | `'to' call should be replaced with infix form` | the call | Replace 'to' with infix form | case KT-INS-054 |
| KT-INS-055 | SHOULD | `SimplifiableCallChain` | `Call chain on collection type can be simplified` | from the first to the last call of the chain | Merge call chain to 'count'  (text names the replacement call) | case KT-INS-055 (only pairs from the built-in table of the standard library) |
| KT-INS-056 | SHOULD | `ReplaceRangeToWithUntil` | `'rangeTo' or the '..' call should be replaced with 'until'` | the operator | Replace with 'until' | case KT-INS-056 (for languages that support the `..<` operator the sibling inspection `ReplaceRangeToWithRangeUntil` applies instead) |
| KT-INS-057 | SHOULD | `DestructuringDeclaration` | `Use destructuring declaration` | the declaration or lambda parameter | Use destructuring declaration | case KT-INS-057 (at least two component usages [verify threshold]) |
| KT-INS-058 | SHOULD | `ConvertTwoComparisonsToRangeCheck` | `Two comparisons should be converted to a range check` | the `&&` expression | Convert comparisons to 'in 1..12' | case KT-INS-058 |
| KT-INS-059 | SHOULD | `ReplaceWithOperatorAssignment` | `Replaceable with operator-assignment` | the assignment | Replace with operator-assignment | case KT-INS-059 |
| KT-INS-060 | SHOULD | `UnnecessaryVariable` | `Variable used only in following return and can be inlined` / `Variable is same as 'x' and can be inlined` | the variable name | Inline variable | case KT-INS-060 (the 'immediately returned' case is reported only with the option on (default off); copies of other variables are always reported) |
| KT-INS-061 | SHOULD | `NullableBooleanElvis` | `Equality check should be used instead of elvis for nullable boolean check` | the elvis expression | Replace with equality check | case KT-INS-061 |
| KT-INS-062 | SHOULD | `RemoveToStringInStringTemplate` | `Redundant 'toString()' call in string template` | the call | Remove 'toString()' call | case KT-INS-062 |
| KT-INS-063 | SHOULD | `UselessCallOnCollection` | `Redundant call on collection type` | the call | Remove redundant call | case KT-INS-063 (`mapNotNull` with a non-null result becomes `map`) |
| KT-INS-064 | SHOULD | `MayBeConstant` | `Might be 'const'` | the property name | Add 'const' modifier | case KT-INS-064 (top-level and object `val`s with a compile-time constant of primitive or String type) |
| KT-INS-065 | SHOULD | `ProtectedInFinal` | `'protected' visibility is effectively 'private' in a final class` | the modifier | Make private | case KT-INS-065 |
| KT-INS-066 | SHOULD | `AddOperatorModifier` | `Function should have 'operator' modifier` | the function name | Add 'operator' modifier | case KT-INS-066 |
| KT-INS-067 | SHOULD | `EqualsOrHashCode` | `'equals()' and 'hashCode()' not paired` | the name of the declared method | Generate 'hashCode()' / Generate 'equals()' | case KT-INS-067 (an object that overrides one of them gets `Delete equals()/hashCode()`) |
| KT-INS-068 | SHOULD | `StringReferentialEquality` | `String compared with referential equality` | the operator | Replace '===' with '==' | case KT-INS-068 |
| KT-INS-069 | SHOULD | `KotlinDoubleNegation` | `Redundant double negation` | the negation | Remove redundant negations | case KT-INS-069 |
| KT-INS-070 | SHOULD | `RedundantIf` | `Redundant 'if' statement` | the `if` keyword | Remove redundant 'if' statement | case KT-INS-070 |
| KT-INS-071 | SHOULD | `EmptyRange` | `This range is empty. Did you mean to use 'downTo'?` | the range | Replace with 'downTo'  (text per case) | case KT-INS-071 |
| KT-INS-072 | SHOULD | `SimplifyBooleanWithConstants` | `Boolean expression can be simplified` | the sub-expression | Simplify boolean expression | case KT-INS-072 |
| KT-INS-073 | SHOULD | `RedundantRequireNotNullCall` | `Redundant 'requireNotNull' or 'checkNotNull' call` | the call | Remove 'requireNotNull' call | case KT-INS-073 |
| KT-INS-074 | SHOULD | `UnusedExpression` | `Expression is unused` | the expression | Add 'return' to last expression | case KT-INS-074 (the fix is offered only if the type matches the function's return type; diagnostic-based, no compiler duplicate) |
| KT-INS-075 | SHOULD | `UnusedEquals` | `Unused equals expression` | the `==` expression | (none) | case KT-INS-075 |
| KT-INS-076 | SHOULD | `UnclearPrecedenceOfBinaryExpression` | `Multiple operators with different precedence` | the operator | Add parentheses to clarify precedence  (text per case) | case KT-INS-076 (option `Report even obvious cases` (off); level WARNING) |
| KT-INS-077 | SHOULD | `SortModifiers` | `Non-canonical modifiers order` | the modifier list | Sort modifiers | case KT-INS-077 |
| KT-INS-078 | SHOULD | `ReplaceNegatedIsEmptyWithIsNotEmpty` | `Negated call can be simplified` | the negation | Replace negated 'isEmpty' with 'isNotEmpty' | case KT-INS-078 |
| KT-INS-079 | SHOULD | `ReplaceWithIgnoreCaseEquals` | `Should be replaced with 'equals(..., ignoreCase = true)'` | the comparison | Replace with 'equals(..., ignoreCase = true)'  (text per case) | case KT-INS-079 (may change semantics for some locales) |
| KT-INS-080 | SHOULD | `MapGetWithNotNullAssertionOperator` | `'map.get()' with not-null assertion operator '(!!)'` | the `!!` | Replace with 'getValue' call | case KT-INS-080 (other offered fixes: `Replace with 'getOrElse' call`, `Replace with '?: error()'`) |
| KT-INS-081 | SHOULD | `BooleanLiteralArgument` | `Boolean literal argument without a parameter name` | the literal | Add names to call arguments | case KT-INS-081 (reported when at least two Boolean literals are passed (option: also a single one)) |
| KT-INS-082 | SHOULD | `CanBeParameter` | `Constructor parameter is never used as a property` | the `val`/`var` keyword | Remove 'val'  (text per case) | case KT-INS-082 |
| KT-INS-083 | SHOULD | `CanBePrimaryConstructorProperty` | `Property is explicitly assigned to parameter 'name', so it can be declared directly in the constructor` | the property | Move to constructor | case KT-INS-083 |
| KT-INS-084 | SHOULD | `ConvertSecondaryConstructorToPrimary` | `Convert to primary constructor` | the `constructor` keyword | Convert to primary constructor | case KT-INS-084 (severity INFORMATION) |
| KT-INS-085 | SHOULD | `ArrayInDataClass` | `Property with 'Array' type in a 'data class': it is recommended to override 'equals()' and 'hashCode()'` | the parameter | Generate equals() and hashCode() | case KT-INS-085 ([verify exact generated body; semantics: contentEquals / contentHashCode]) |
| KT-INS-086 | SHOULD | `AddVarianceModifier` | `Type parameter can have 'out' variance` / `'in' variance` | the type parameter | Add 'out' variance | case KT-INS-086 (severity INFORMATION) |
| KT-INS-087 | SHOULD | `SuspiciousCollectionReassignment` | `'+=' on a read-only List creates a new List under the hood` | the operator | Change type to mutable | case KT-INS-087 |
| KT-INS-088 | SHOULD | `RemoveRedundantSpreadOperator` | `Redundant spread operator` | the `*` | Remove redundant spread operator | case KT-INS-088 |
| KT-INS-089 | SHOULD | `ReplaceMapGetOrDefault` | `'map.getOrDefault()' can be replaced with indexing and elvis operator` | the call | Replace with indexing and elvis operator | case KT-INS-089 |
| KT-INS-090 | SHOULD | `ConvertTryFinallyToUseCall` | `Convert try / finally to use() call` | the `try` | Convert try / finally to use() call | case KT-INS-090 ([verify lambda parameter naming: `r.use { r -> ...}` in some versions]) |
| KT-INS-091 | SHOULD | `ReplaceWithEnumMap` | `'HashMap' can be replaced with 'EnumMap'` | the constructor call | Replace with 'EnumMap' | case KT-INS-091 ([verify; adds import java.util.EnumMap]) |
| KT-INS-092 | SHOULD | `TrailingComma` | `Missing trailing comma` / `Redundant trailing comma` (per the code-style setting `Use trailing comma`) | the last element | Add trailing comma / Remove trailing comma | case KT-INS-092 (default: only the redundant case is reported when the code-style setting is off; with the setting on the missing case is reported (option `Report also a missing comma or a line break`)) |
| KT-INS-093 | SHOULD | `MemberVisibilityCanBePrivate` | `Property 'url' could be private` / `Function 'f' could be private` | the name | Make private | case KT-INS-093 (OFF by default (search-based)) |
| KT-INS-094 | SHOULD | `ReplaceSubstringWithTake` | `'substring' call should be replaced with 'take' call` | the call | Replace 'substring' call with 'take' call | case KT-INS-094 |
| KT-INS-095 | SHOULD | `ConvertReferenceToLambda` | `Can be replaced with lambda` | the reference | Convert reference to lambda | case KT-INS-095 (severity INFORMATION) |
| KT-INS-096 | SHOULD | `ObjectLiteralToLambda` | `Object literal can be converted to lambda` | the object literal | Convert to lambda | case KT-INS-096 ([verify exact fix text]) |

`KT-INS-004` MUST: every row of 5.3 with priority MUST, with the exact message, range and fix. Acceptance: the cases of that row.

## 5.4 Naming inspections

The naming inspections check a declaration's name against a **regular expression** (setting `Pattern:`), and also against fixed rules; the message is
`<Kind> name '<name>' <violation>` where Kind is `Class`, `Function`, `Property`, `Local variable`, `Package`, `Enum entry`, `Object property`, `Const property`,
`Test function` and the violation is one of: `should start with an uppercase letter`, `should start with a lowercase letter`, `should not start with an uppercase letter`,
`should not start with an underscore`, `should not contain underscores`, `should not contain underscores in the middle or the end`,
`should not contain underscores when camel case is used`, `should not contain lowercase letters`, `may contain only letters and digits`,
`may contain only letters, digits or underscores`, `doesn't match regex '<pattern>'`. The fix is the platform *Rename* (inline rename with
usages), offered as `Rename ...`. Defaults (patterns): class `[A-Z][A-Za-z\d]*`; function `[a-z][A-Za-z\d]*` (a function named like a class and returning it is allowed as a factory);
property and local variable `[a-z][A-Za-z\d]*`; private property `_?[a-z][A-Za-z\d]*`; const property `[A-Z][_A-Z\d]*`; enum entry `[A-Z]([A-Za-z\d]*|[A-Z_\d]*)`;
package `[a-z_][a-zA-Z\d_]*(\.[a-z_][a-zA-Z\d_]*)*` [verify all defaults]. Level `WEAK WARNING`.

`KT-INS-005` MUST: ClassName, FunctionName, PropertyName, LocalVariableName as in 5.4; SHOULD for the other naming inspections of Appendix A. Acceptance: cases of `ClassName`, `FunctionName`, `PropertyName`, `LocalVariableName`.

## 5.5 Unused symbols

`UnusedSymbol` has two paths: **in the editor** it is computed by a dedicated highlighter that runs after resolve; in **batch** by the inspection itself.
Both report `<Kind> '<name>' is never used` (grey text, level WARNING; the exact `Kind` word is that of the element-description service) on the name identifier,
with the fix `Safe delete '<name>'` (variants: `Safe delete constructor`, `Safe delete primary constructor '<name>'`, `Safe delete secondary constructor '<name>'`,
`Safe delete parameter '<name>'`).

A declaration is a *candidate* unless: it is a companion object; it overrides something; it is a parameter of a functional-type parameter list, of an `external` declaration, of an `expect`/`actual`/abstract/
open/overridden/interface/operator function, of an anonymous function, or a property-parameter (`val`/`var`) of a data/value/inline class; a context parameter of something other than a function/property; a top-level
public declaration in a script; named `_` (explicitly ignored); `@Deprecated(level = HIDDEN)`; or in explicit API mode and part of the public API.

It is **used** if some reference to it exists in the project resolve scope (searched by name index then verified), with these extras: a constructor-declared property of a class that must have a non-empty primary constructor is
treated as used (component functions may use it); an `operator fun` (except `componentN`) is treated as used; a `finalize()`; fields/methods implicitly used by serialization; annotation classes listed as opt-in markers
in the compiler flags; classes with text usages (their name appears in a non-code text of the project such as XML); **entry points** (`main`, test methods/classes recognised by framework annotations, declarations annotated with
annotations from the *Entry points* setting; `kotlin.test.*` and `kotlin.js.JsExport` annotations count by default).
A **private** (or local-class-member, or non-property parameter) declaration is judged from the **file alone**; others use a project search, which is skipped when the search is not
cheap enough (a name used in too many places) and the declaration is then treated as used.

`KT-INS-006` MUST: candidates and usage rules above. Acceptance: cases of `UnusedSymbol` plus 05 `KT-INS-006` cases.

## Appendix A. Inventory of all Kotlin inspections

Priority: `MUST` (5.3, own row), `SHOULD` (implement after the MUST set; message = display name unless the table says otherwise; fix = the inspection's own
quick fix named in its bundle entry; behaviour as in the description page of the inspection), `LATER` (reason given). `kind`: local / global, `+cleanup`.
Default state `on`/`off` with the default level.

`KT-INS-090` SHOULD: every inspection marked SHOULD in Appendix A, with its default state and level and the shared contract of 5.1. Acceptance: profile dump equals Appendix A for the implemented ones; each has at least one positive and one negative test.

`KT-INS-091` LATER: every inspection marked LATER in Appendix A (reason in the table).

### Code

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `checkedExceptions` | Unchecked exceptions in Kotlin | on, WARNING | local | LATER | IntelliJ-platform / Fleet development inspection, not Kotlin language support |
| `CompanionObjectInExtension` | Companion object in extensions | on, WARNING | local | LATER | IntelliJ-platform / Fleet development inspection, not Kotlin language support |
| `ForbiddenInSuspectContextMethod` | Forbidden in suspend context method usage | on, WARNING | local | LATER | IntelliJ-platform / Fleet development inspection, not Kotlin language support |
| `KotlinApplicableInspectionBaseSimpleCallExpression` | Too broad highlighting range in a Kotlin inspection | on, WARNING | local | LATER | IntelliJ-platform / Fleet development inspection, not Kotlin language support |
| `KotlinObjectRegisteredAsExtension` | Kotlin object registered as extension | on, ERROR | local | LATER | IntelliJ-platform / Fleet development inspection, not Kotlin language support |
| `KotlinPolySymbolPropertyType` | PolySymbol property type mismatch | on, ERROR | local | LATER | IntelliJ-platform / Fleet development inspection, not Kotlin language support |
| `NonSerializableTypeInRpcInterface` | Non-serializable type used in RPC interface | on, ERROR | local | LATER | IntelliJ-platform / Fleet development inspection, not Kotlin language support |
| `PrivateExtensionClass` | Private modifier used with IntelliJ Platform extension class | on, WARNING | local | LATER | IntelliJ-platform / Fleet development inspection, not Kotlin language support |
| `PublicTestExtensionFunction` | Public extension function on Driver/Finder/IdeaFrameUI in intelij.driver.tests | on, WARNING | local | LATER | IntelliJ-platform / Fleet development inspection, not Kotlin language support |
| `RpcInterfaceDoesNotExtendRemoteApi` | @Rpc-annotated interface does not extend fleet.rpc.RemoteApi | on, ERROR | local | LATER | IntelliJ-platform / Fleet development inspection, not Kotlin language support |
| `UsePlatformProcessAwaitExit` | Use 'com.intellij.util.io.ProcessKt.awaitExit()' | on, ERROR | local | LATER | IntelliJ-platform / Fleet development inspection, not Kotlin language support |

### Compose

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `ComposableCallWithoutComposePlugin` | Composable function call without Compose plugin | on, ERROR | local | LATER | Compose compiler-plugin support |
| `ComposeResourcesImageResource` | Vector resource used with imageResource() | on, ERROR | local | LATER | Compose compiler-plugin support |

### Compose Multiplatform Preview

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `MultiplatformPreviewAnnotationInFunctionWithParameters` | Preview used on a Composable function with parameters | on, ERROR | local | LATER | Compose compiler-plugin support |
| `MultiplatformPreviewMultipleParameterProviders` | Multiple @PreviewParameter are not allowed | on, ERROR | local | LATER | Compose compiler-plugin support |

### Gradle

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `KotlinOptionsToCompilerOptions` | Use of deprecated 'kotlinOptions' DSL | on, WARNING | local | LATER | Gradle build-script support (Gradle spec set) |

### Kotlin

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `DifferentKotlinGradleVersion` | Kotlin Gradle and IDE plugins versions are different | on, WARNING | local (Groovy files) | LATER | build-system or IDE-development inspection (Gradle/Maven/kapt/bundles) |
| `DifferentKotlinMavenVersion` | Maven and IDE plugins versions are different | on, WARNING | local (XML files) | LATER | build-system or IDE-development inspection (Gradle/Maven/kapt/bundles) |
| `DifferentMavenStdlibVersion` | Library and maven plugin versions are different | on, WARNING | local (XML files) | LATER | build-system or IDE-development inspection (Gradle/Maven/kapt/bundles) |
| `DifferentStdlibGradleVersion` | Kotlin library and Gradle plugin versions are different | on, WARNING | local (Groovy files) | LATER | build-system or IDE-development inspection (Gradle/Maven/kapt/bundles) |
| `KaptKotlinCompilerPlugin` | Add Kotlin kapt compiler plugin to module build | on, WARNING | local | LATER | build-system or IDE-development inspection (Gradle/Maven/kapt/bundles) |
| `KotlinInvalidBundleOrProperty` | Invalid property key | on, ERROR | local | LATER | build-system or IDE-development inspection (Gradle/Maven/kapt/bundles) |
| `KotlinMavenPluginPhase` | Kotlin Maven Plugin misconfigured | on, WARNING | local (XML files) | LATER | build-system or IDE-development inspection (Gradle/Maven/kapt/bundles) |
| `KotlinTestJUnit` | kotlin-test-junit could be used | on, WARNING | local (XML files) | LATER | build-system or IDE-development inspection (Gradle/Maven/kapt/bundles) |

### Kotlin / Code migration

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `CanConvertToMultiDollarString` | Multi-dollar interpolation can be used in string literals | on, WEAK WARNING | local | LATER | multi-dollar interpolation (new language feature) |
| `ConvertFromMultiDollarToRegularString` | Removable multi-dollar interpolation prefix | on, INFORMATION | local | LATER | language-version migration tooling |
| `ConvertLongToDuration` | Long overload to Duration conversion | on, WARNING | local | LATER | language-version migration tooling |
| `CustomComponentDestructuringMigration` | Position-based destructuring should use positional syntax | on, WEAK WARNING | local | LATER | language-version migration tooling |
| `InfixCallToOrdinary` | Infix call may be dot call | on, INFORMATION | local | LATER | language-version migration tooling |

### Kotlin / Coroutine inspections

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `CoroutineContextWithJob` | 'CoroutineContext' can contain 'Job' element | on, WARNING | local | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |
| `DeferredResultUnused` | '@Deferred' result is unused | on, WARNING | local | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |
| `ForEachJoinOnCollectionOfJob` | 'forEach { it.join() }' call on 'Collection&lt;Job&gt;' instead of single 'joinAll()' | on, WARNING | local | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |
| `MapAwaitOnCollectionOfDeferred` | 'map { it.await() }' call on 'Collection&lt;Deferred&gt;' instead of single 'awaitAll()' | on, WARNING | local | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |
| `PreferCurrentCoroutineContextToCoroutineContext` | Potentially ambiguous 'kotlin.coroutine.coroutineContext' usage | on, WARNING | local | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |
| `RunBlocking` | RunBlocking in coroutine | on, WARNING | global | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |
| `RunBlockingInSuspendFunction` | 'runBlocking' inside suspend function | on, WARNING | local | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |
| `SimplifiableFlowCall` | Function call on 'Flow' type could be simplified | on, WEAK WARNING | local | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |
| `SimplifiableFlowCallChain` | Call chain on 'Flow' type can be simplified | on, WEAK WARNING | local | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |
| `SuppressedCancellationException` | Suppressed 'CancellationException' | on, WARNING | local | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |
| `SuspendCoroutineLacksCancellationGuarantees` | 'suspendCoroutine' lacks cancellation guarantees | on, WARNING | local | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |
| `SuspiciousImplicitCoroutineScopeReceiverAccess` | Suspicious implicit 'CoroutineScope' receiver access | on, WARNING | local | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |
| `SuspiciousMutableCollectionInStateFlow` | Suspicious mutable collection in 'MutableStateFlow' | on, WARNING | local | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |
| `UnusedFlow` | Unused flow | on, WARNING | local | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |
| `UselessCallOnFlow` | Redundant call on 'Flow' type | on, WARNING | local | LATER | needs the kotlinx.coroutines library model (suspend/Flow/Deferred semantics) |

### Kotlin / Java interop issues

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `HasPlatformType` | Function or property has platform type | on, WEAK WARNING | local | SHOULD |  |
| `JavaCollectionsStaticMethodOnImmutableList` | Call of Java mutator method on immutable Kotlin collection | on, WARNING | local | SHOULD |  |
| `JavaCollectionWithNullableTypeArgument` | Nullable type argument in Java collection that doesn't support nulls | on, WARNING | local | SHOULD |  |
| `JavaDefaultMethodsNotOverriddenByDelegation` | Java default methods are not overridden by delegation | on, WARNING | local | SHOULD |  |
| `PackageDirectoryMismatch` | Package name does not match containing directory | on, WEAK WARNING | local | MUST | KT-INS-048 |

### Kotlin / Logging

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `KotlinLoggerInitializedWithForeignClass` | Logger initialized with foreign class | off, WARNING | local | LATER | logging-framework model (off by default) |

### Kotlin / Migration

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `KotlinDeprecation` | Usage of redundant or deprecated syntax or deprecated symbols | on, WARNING | local+cleanup | LATER | cleanup/migration tool, not a daily inspection |

### Kotlin / Naming conventions

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `ClassName` | Class naming convention | on, WEAK WARNING | local | MUST | KT-INS-044 |
| `ConstPropertyName` | Const property naming convention | on, WEAK WARNING | local | SHOULD |  |
| `EnumEntryName` | Enum entry naming convention | on, WEAK WARNING | local | SHOULD |  |
| `FunctionName` | Function naming convention | on, WEAK WARNING | local | MUST | KT-INS-045 |
| `InconsistentCommentForJavaParameter` | Inconsistent comment for Java parameter | on, WARNING | local | SHOULD |  |
| `LocalVariableName` | Local variable naming convention | on, WEAK WARNING | local | MUST | KT-INS-047 |
| `ObjectPrivatePropertyName` | Object private property naming convention | on, WEAK WARNING | local | SHOULD |  |
| `ObjectPropertyName` | Object property naming convention | on, WEAK WARNING | local | SHOULD |  |
| `PackageName` | Package naming convention | on, WEAK WARNING | global | SHOULD |  |
| `PrivatePropertyName` | Private property naming convention | on, WEAK WARNING | local | SHOULD |  |
| `PropertyName` | Property naming convention | on, WEAK WARNING | local | MUST | KT-INS-046 |
| `TestFunctionName` | Test function naming convention | on, WEAK WARNING | local | SHOULD |  |

### Kotlin / Numeric issues

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `KotlinBigDecimalEquals` | 'equals()' called on 'BigDecimal' | off, WARNING | local+cleanup | LATER | off by default |

### Kotlin / Other problems

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `ConvertArgumentToSet` | Argument could be converted to 'Set' to improve performance | on, WEAK WARNING | local | SHOULD |  |
| `DeprecatedCallableAddReplaceWith` | @Deprecated annotation without 'replaceWith' argument | on, WEAK WARNING | local | SHOULD |  |
| `EnumValuesSoftDeprecate` | 'Enum.values()' is recommended to be replaced by 'Enum.entries' since 1.9 | on, WARNING | local | SHOULD |  |
| `EnumValuesTopLevelFunctionSoftDeprecate` | 'enumValues' is recommended to be replaced by 'enumEntries' | on, WARNING | local | SHOULD |  |
| `FloatingPointLiteralPrecision` | Floating-point literal exceeds the available precision | on, WEAK WARNING | local | SHOULD |  |
| `KDocMissingDocumentation` | Missing KDoc comments for public declarations | off, WARNING | local | LATER | off by default (KDoc coverage) |
| `KotlinAmbiguousActuals` | Ambiguous actuals | on, ERROR | local | LATER | Kotlin Multiplatform expect/actual |
| `KotlinNoActualForExpect` | No actual for expect declaration | on, ERROR | local | LATER | Kotlin Multiplatform expect/actual |
| `MigrateDiagnosticSuppression` | Diagnostic name should be replaced | on, WARNING | local+cleanup | LATER | migration of suppression names |
| `PublicApiImplicitType` | Public API declaration with implicit return type | off, WEAK WARNING | local | LATER | off by default (explicit API mode) |
| `ReplacePrintlnWithLogging` | Call to 'print()' or 'println()' | off, WARNING | local | LATER | off by default |
| `ReplaceWithEnumMap` | 'HashMap' can be replaced with 'EnumMap' | on, WARNING | local | SHOULD | KT-INS-091 |
| `ReplaceWithStringBuilderAppendRange` | 'StringBuilder.append(CharArray, offset, len)' call on the JVM | on, WARNING | local+cleanup | SHOULD |  |
| `ScriptShebangExecPermission` | Script is not executable. Missing execute permission. | on, WARNING | local | LATER | script files and file permissions |

### Kotlin / Probable bugs

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `AddExplicitTargetToParameterAnnotation` | Redundant interpolation prefix | on, WEAK WARNING | local | LATER | annotation-target migration |
| `AmbiguousNonLocalJump` | Ambiguous non-local 'break' or 'continue' | on, WEAK WARNING | local | SHOULD |  |
| `ArrayInDataClass` | Array property in data or value class | on, WARNING | local | SHOULD | KT-INS-085 |
| `AssignedValueIsNeverRead` | Assigned value is never read | on, WARNING | local | MUST | KT-INS-013 |
| `CanSealedSubClassBeObject` | Sealed subclass without state and overridden equals | on, WEAK WARNING | local | SHOULD |  |
| `ConflictingExtensionProperty` | Extension property conflicting with synthetic one | on, WARNING | local | SHOULD |  |
| `ConvertNameBasedDestructuringShortFormToFull` | Convert name-based destructuring short form to full | on, INFORMATION | local | LATER | name-based destructuring (experimental language feature) |
| `ConvertNaNEquality` | Convert equality check with 'NaN' to 'isNaN' call | on, WARNING | local | SHOULD |  |
| `DataClassPrivateConstructor` | Private data class constructor is exposed via the 'copy' method | on, WARNING | local | SHOULD |  |
| `DelegationToVarProperty` | Delegating to 'var' property | on, WARNING | local | SHOULD |  |
| `DestructingShortFormNameMismatch` | Declaration name mismatch in destructuring declaration | on, WARNING | local | LATER | name-based destructuring (experimental language feature) |
| `DuplicateArgumentsInSetOfAndMapOfFunctions` | Duplicate arguments in 'setOf' and 'mapOf' functions | on, WARNING | local | SHOULD |  |
| `EmptyRange` | Range with start greater than endInclusive is empty | on, WARNING | local | SHOULD | KT-INS-071 |
| `EqualsOrHashCode` | 'equals()' and 'hashCode()' not paired | on, WARNING | local | SHOULD | KT-INS-067 |
| `EscapingCapturedVariable` | Escaped captured variable | on, WARNING | local | SHOULD |  |
| `Fastjson2MissingKotlinReflect` | The kotlin-reflect library is required for fastjson2-kotlin | on, WARNING | local | LATER | third-party library check |
| `FilterIsInstanceResultIsAlwaysEmpty` | The result of 'filterIsInstance' call is always empty | on, WARNING | local | SHOULD |  |
| `ForEachParameterNotUsed` | Iterated elements are not used in forEach | on, WEAK WARNING | local | SHOULD |  |
| `ImplicitNullableNothingType` | Implicit 'Nothing?' type | on, WEAK WARNING | local | SHOULD |  |
| `IncompleteDestructuring` | Incomplete destructuring declaration | on, INFORMATION | local | LATER | name-based destructuring (experimental language feature) |
| `JavaIoSerializableObjectMustHaveReadResolve` | Serializable object must implement 'readResolve' | on, WARNING | local | SHOULD |  |
| `KDocUnresolvedReference` | Unresolved link in KDoc | on, WARNING | local | SHOULD |  |
| `KotlinArrayHashCode` | 'hashCode()' called on array | on, WARNING | local | SHOULD |  |
| `KotlinArrayToString` | 'toString()' called on array | on, WARNING | local | SHOULD |  |
| `KotlinConstantConditions` | Constant conditions | on, WARNING | local | MUST | KT-INS-028 |
| `KotlinCovariantEquals` | Covariant 'equals()' | on, WARNING | local | SHOULD |  |
| `KotlinEqualsBetweenInconvertibleTypes` | 'equals()' between objects of inconvertible types | on, WARNING | local | SHOULD |  |
| `KotlinMisorderedAssertEqualsArguments` | Misordered 'assertEquals()' arguments | on, WARNING | local | LATER | test-framework model (JUnit/kotlin.test) |
| `KotlinScriptTemplateNotRegistered` | Kotlin script definition is not set up | on, WARNING | local | LATER | Kotlin scripting definitions (01 1.8) |
| `KotlinThrowableNotThrown` | Throwable not thrown | on, WARNING | local | SHOULD |  |
| `KotlinUnreachableCode` | Unreachable code | on, WARNING | local | MUST | KT-INS-029 |
| `LateinitVarOverridesLateinitVar` | 'lateinit var' property overrides 'lateinit var' property | on, WARNING | local | SHOULD |  |
| `LazyWithoutDelegation` | 'lazy' can be used with delegation | on, WEAK WARNING | local | SHOULD |  |
| `MainFunctionReturnUnit` | Main function should return 'Unit' | on, WARNING | local+cleanup | SHOULD |  |
| `MainKtsAnnotationUsageInPlainKts` | Using .main.kts annotation in plain .kts script | on, WARNING | local | LATER | Kotlin scripting |
| `NonNullableBooleanPropertyInExternalInterface` | External interface contains non-nullable boolean property | off, WARNING | local | LATER | Kotlin/JS external declarations |
| `NonVarPropertyInExternalInterface` | External interface contains val property | on, WARNING | local | LATER | Kotlin/JS external declarations |
| `ObjectInheritsException` | Exception should not be an object | on, WARNING | local+cleanup | SHOULD |  |
| `RecursiveEqualsCall` | Recursive equals call | on, WARNING | local | SHOULD |  |
| `RecursivePropertyAccessor` | Recursive property accessor | on, WARNING | local | SHOULD |  |
| `RedundantDestructuringRename` | Redundant name-based destructuring rename | on, WARNING | local | LATER | name-based destructuring (experimental language feature) |
| `RedundantDestructuringUnderscore` | Redundant underscore entry in name-based destructuring | on, WARNING | local | LATER | name-based destructuring (experimental language feature) |
| `RedundantLabel` | Redundant label | on, WARNING | local | SHOULD |  |
| `SelfAssignment` | Redundant assignment | on, WARNING | local+cleanup | MUST | KT-INS-050 |
| `SelfReferenceConstructorParameter` | Constructor can never be complete | on, WARNING | local | SHOULD |  |
| `SetterBackingFieldAssignment` | Existing backing field without assignment | on, WARNING | local+cleanup | SHOULD |  |
| `StringReferentialEquality` | String compared with referential equality | on, WARNING | local | SHOULD | KT-INS-068 |
| `SuspiciousCallOnCollectionToAddOrRemovePath` | Suspicious call to add or remove Iterable argument | on, WARNING | local | SHOULD |  |
| `SuspiciousCascadingIf` | Suspicious cascading 'if' expression | on, WARNING | local | SHOULD |  |
| `SuspiciousCollectionReassignment` | Augmented assignment creates a new collection under the hood | on, WEAK WARNING | local | SHOULD | KT-INS-087 |
| `SuspiciousEqualsCombination` | Suspicious combination of == and === | on, WEAK WARNING | local | SHOULD |  |
| `SuspiciousGetterForMutableObject` | Suspicious getter: it returns a new mutable object on each access | on, WARNING | local | SHOULD |  |
| `SuspiciousJavaClassCallableReference` | Suspicious '::javaClass' callable reference | on, WARNING | local | SHOULD |  |
| `SuspiciousVarProperty` | Suspicious 'var' property: its setter does not influence its getter result | on, WARNING | local | SHOULD |  |
| `UnusedDataClassCopyResult` | Unused result of data class copy | on, WARNING | local | SHOULD |  |
| `UnusedEquals` | Unused equals expression | on, WARNING | local | SHOULD | KT-INS-075 |
| `UnusedExpression` | Unused expression | on, WARNING | local | SHOULD | KT-INS-074 |
| `UnusedLambdaExpression` | Unused lambda expression | on, WARNING | local | SHOULD |  |
| `UnusedLambdaExpressionBody` | Unused return value of a function with lambda expression body | on, WARNING | local | SHOULD |  |
| `UnusedUnaryOperator` | Unused unary operator | on, WARNING | local | SHOULD |  |
| `UselessCallOnCollection` | Redundant call on collection type | on, WARNING | local | SHOULD | KT-INS-063 |
| `UselessCallOnNotNull` | Redundant call on not-null type | on, WARNING | local | MUST | KT-INS-051 |
| `VariableNeverRead` | Variable is never read | on, WARNING | local | MUST | KT-INS-012 |
| `WrapUnaryOperator` | Ambiguous unary operator use with number constant | on, WEAK WARNING | local | SHOULD |  |

### Kotlin / Redundant constructs

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `CanBeParameter` | Constructor parameter is never used as a property | on, WARNING | local | SHOULD | KT-INS-082 |
| `CanBePrimaryConstructorProperty` | Property is explicitly assigned to constructor parameter | on, WARNING | local | SHOULD | KT-INS-083 |
| `CanUnescapeDollarLiteral` | Redundant escaped dollar characters in string literals | on, WEAK WARNING | local | SHOULD |  |
| `ComplexRedundantLet` | Redundant argument-based 'let' call | on, WEAK WARNING | local | SHOULD |  |
| `ConstantConditionIf` | Condition of 'if' expression is constant | on, WEAK WARNING | local | MUST | KT-INS-027 |
| `ControlFlowWithEmptyBody` | Control flow with empty body | on, WARNING | local | SHOULD |  |
| `ExplicitThis` | Redundant explicit 'this' | on, INFORMATION | local | SHOULD |  |
| `FunctionWithLambdaExpressionBody` | Function with '= { ... }' and inferred return type | on, WARNING | local | SHOULD |  |
| `IfExpressionWithIdenticalBranches` | 'if' expression with identical branches | on, WEAK WARNING | local | SHOULD |  |
| `KotlinDoubleNegation` | Redundant double negation | on, WEAK WARNING | local+cleanup | SHOULD | KT-INS-069 |
| `KotlinRedundantOverride` | Redundant overriding method | on, WEAK WARNING | local+cleanup | SHOULD |  |
| `KotlinUnusedImport` | Unused import directive | on, WARNING | local | SHOULD |  |
| `NullChecksToSafeCall` | Null-checks can be replaced with safe-calls | on, WEAK WARNING | local | SHOULD |  |
| `RedundantCompanionReference` | Redundant 'Companion' reference | on, WARNING | local+cleanup | MUST | KT-INS-025 |
| `RedundantConstructorKeyword` | Redundant 'constructor' keyword | on, WEAK WARNING | local+cleanup | SHOULD |  |
| `RedundantContextSensitiveResolutionQualifier` | Redundant qualifier removable via context-sensitive resolution | off, WEAK WARNING | local+cleanup | LATER | context-sensitive resolution (new language feature), off by default |
| `RedundantElvisReturnNull` | Redundant '?: return null' | on, WARNING | local | SHOULD |  |
| `RedundantEmptyInitializerBlock` | Redundant empty initializer block | on, WEAK WARNING | local | SHOULD |  |
| `RedundantEnumConstructorInvocation` | Redundant enum constructor invocation | on, WEAK WARNING | local | SHOULD |  |
| `RedundantExplicitType` | Obvious explicit type | on, WARNING | local | MUST | KT-INS-016 |
| `RedundantGetter` | Redundant property getter | on, WEAK WARNING | local+cleanup | SHOULD |  |
| `RedundantIf` | Redundant 'if' statement | on, WEAK WARNING | local+cleanup | SHOULD | KT-INS-070 |
| `RedundantInnerClassModifier` | Redundant 'inner' modifier | on, WARNING | local+cleanup | SHOULD |  |
| `RedundantInterpolationPrefix` | Redundant interpolation prefix | on, WEAK WARNING | local | SHOULD |  |
| `RedundantLabeledReturnOnLastExpressionInLambda` | Redundant labeled return on the last expression in a lambda | on, INFORMATION | local | SHOULD |  |
| `RedundantLambdaArrow` | Redundant lambda arrow | on, WARNING | local+cleanup | MUST | KT-INS-022 |
| `RedundantLambdaOrAnonymousFunction` | Redundant creation of lambda or anonymous function | on, WARNING | local | SHOULD |  |
| `RedundantModalityModifier` | Redundant modality modifier | on, WARNING | local+cleanup | MUST | KT-INS-018 |
| `RedundantNullableReturnType` | Redundant nullable return type | on, WARNING | local+cleanup | SHOULD |  |
| `RedundantRequireNotNullCall` | Redundant 'requireNotNull' or 'checkNotNull' call | on, WEAK WARNING | local | SHOULD | KT-INS-073 |
| `RedundantReturnKeyword` | Redundant 'return' keyword | on, WARNING | local | SHOULD | KT-INS-052 |
| `RedundantReturnLabel` | Redundant 'return' label | on, WEAK WARNING | local | SHOULD |  |
| `RedundantSamConstructor` | Redundant SAM constructor | on, WARNING | local | MUST | KT-INS-023 |
| `RedundantSemicolon` | Redundant semicolon | on, WARNING | local+cleanup | MUST | KT-INS-021 |
| `RedundantSetter` | Redundant property setter | on, WEAK WARNING | local+cleanup | SHOULD |  |
| `RedundantSuspendModifier` | Redundant 'suspend' modifier | on, WARNING | local | SHOULD |  |
| `RedundantUnitExpression` | Redundant 'Unit' | on, WARNING | local+cleanup | MUST | KT-INS-020 |
| `RedundantUnitReturnType` | Redundant 'Unit' return type | on, WARNING | local+cleanup | MUST | KT-INS-019 |
| `RedundantUpperBound` | Redundant upper bound 'Any?' | on, WARNING | local | SHOULD |  |
| `RedundantValueArgument` | Redundant value argument | off, WEAK WARNING | local+cleanup | LATER | off by default |
| `RedundantVisibilityModifier` | Redundant visibility modifier | on, WARNING | local+cleanup | MUST | KT-INS-017 |
| `RedundantWith` | Redundant 'with' call | on, WEAK WARNING | local | SHOULD |  |
| `RemoveCurlyBracesFromTemplate` | Redundant curly braces in string template | on, WEAK WARNING | local | MUST | KT-INS-026 |
| `RemoveEmptyClassBody` | Replace empty class body | on, WEAK WARNING | local+cleanup | SHOULD |  |
| `RemoveEmptyParenthesesFromLambdaCall` | Unnecessary parentheses in function call with lambda | on, WEAK WARNING | local+cleanup | SHOULD |  |
| `RemoveEmptyPrimaryConstructor` | Redundant empty primary constructor | on, WEAK WARNING | local+cleanup | SHOULD |  |
| `RemoveEmptySecondaryConstructorBody` | Redundant constructor body | on, WEAK WARNING | local+cleanup | SHOULD |  |
| `RemoveExplicitSuperQualifier` | Unnecessary supertype qualification | on, WEAK WARNING | local+cleanup | SHOULD |  |
| `RemoveExplicitTypeArguments` | Unnecessary type argument | on, WEAK WARNING | local | SHOULD |  |
| `RemoveRedundantBackticks` | Redundant backticks | on, WARNING | local | SHOULD | KT-INS-053 |
| `RemoveRedundantQualifierName` | Redundant qualifier name | on, WARNING | local+cleanup | MUST | KT-INS-024 |
| `RemoveRedundantSpreadOperator` | Redundant spread operator | on, WARNING | local | SHOULD | KT-INS-088 |
| `RemoveSetterParameterType` | Redundant setter parameter type | on, WARNING | local | SHOULD |  |
| `RemoveSingleExpressionStringTemplate` | Redundant string template | on, WARNING | local | SHOULD |  |
| `RemoveToStringInStringTemplate` | Redundant call to 'toString()' in string template | on, WEAK WARNING | local+cleanup | SHOULD | KT-INS-062 |
| `ScopeFunctionConversion` | Call can be replaced with another scope function | on, INFORMATION | local | SHOULD |  |
| `SimpleRedundantLet` | Redundant receiver-based 'let' call | on, WARNING | local | SHOULD |  |
| `SimplifyWhenWithBooleanConstantCondition` | Simplifiable 'when' | on, WEAK WARNING | local | SHOULD |  |
| `SuspiciousCallableReferenceInLambda` | Suspicious callable reference used as lambda result | on, WEAK WARNING | local | SHOULD |  |
| `UnnecessaryOptInAnnotation` | Unnecessary '@OptIn' annotation | on, WEAK WARNING | local | LATER | opt-in requirement model |
| `UnnecessaryVariable` | Unnecessary local variable | on, WEAK WARNING | local | SHOULD | KT-INS-060 |
| `UnusedContextParameterCall` | Unused context call arguments | on, WEAK WARNING | local | LATER | context parameters (new language feature) |
| `UnusedReceiverParameter` | Unused receiver parameter | on, WARNING | local | MUST | KT-INS-049 |
| `UnusedSymbol` | Unused symbol | on, WARNING | local | MUST | KT-INS-010 |
| `UnusedVariable` | Unused variable | on, WARNING | local | MUST | KT-INS-011 |
| `VariableInitializerIsRedundant` | Variable initializer is redundant | on, WARNING | local | MUST | KT-INS-014 |
| `WhenWithOnlyElse` | 'when' has only 'else' branch and can be simplified | off, WEAK WARNING | local | LATER | off by default |

### Kotlin / Style issues

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `AddOperatorModifier` | Function should have 'operator' modifier | on, INFORMATION | local | SHOULD | KT-INS-066 |
| `AddVarianceModifier` | Type parameter can have 'in' or 'out' variance | on, INFORMATION | local | SHOULD | KT-INS-086 |
| `BooleanLiteralArgument` | Boolean literal argument without parameter name | on, WEAK WARNING | local | SHOULD | KT-INS-081 |
| `CanBeVal` | Local 'var' is never modified and can be declared as 'val' | on, WARNING | local | MUST | KT-INS-015 |
| `CascadeIf` | Cascade 'if' can be replaced with 'when' | on, WEAK WARNING | local | MUST | KT-INS-033 |
| `CollapseCollectionLiteralChainCall` | Type conversion on collection literal can be removed | on, INFORMATION | local | LATER | collection literals (new language feature) |
| `CollectionConcatenationToBuildCollection` | Collection concatenation can be converted to collection builder | on, WEAK WARNING | local | SHOULD |  |
| `ConvertCallChainIntoSequence` | Call chain on collection could be converted into 'Sequence' to improve performance | on, WEAK WARNING | local | SHOULD |  |
| `ConvertCollectionLiteralToFunctionCall` | Collection literal can be replaced with a function call | on, INFORMATION | local | LATER | collection literals (new language feature) |
| `ConvertCollectionLiteralToTypeOfExpression` | ''…'' can be used | on, INFORMATION | local | LATER | collection literals (new language feature) |
| `ConvertDataClassToValueClass` | Data class can be converted to value class | on, INFORMATION | local | LATER | value-class migration advice (INFORMATION) |
| `ConvertExplicitContextArgumentToImplicit` | Explicit context argument can be implicit | on, INFORMATION | local | LATER | context parameters (new language feature) |
| `ConvertImplicitContextArgumentToExplicit` | Implicit context argument can be explicit | on, INFORMATION | local | LATER | context parameters (new language feature) |
| `ConvertPairConstructorToToFunction` | Convert Pair constructor to 'to' function | on, INFORMATION | local | SHOULD |  |
| `ConvertRangeCheckToTwoComparisons` | Convert to comparisons | on, INFORMATION | local | SHOULD |  |
| `ConvertReferenceToLambda` | Can be replaced with lambda | on, INFORMATION | local | SHOULD | KT-INS-095 |
| `ConvertSealedClassToSealedInterface` | Sealed class can be converted to sealed interface | on, INFORMATION | local | SHOULD |  |
| `ConvertSealedInterfaceToSealedClass` | Sealed interface can be converted to sealed class | on, INFORMATION | local | SHOULD |  |
| `ConvertSecondaryConstructorToPrimary` | Convert to primary constructor | on, INFORMATION | local | SHOULD | KT-INS-084 |
| `ConvertToCollectionLiterals` | Collection literal can be used | on, INFORMATION | local | LATER | collection literals (new language feature) |
| `ConvertToExplicitBackingFields` | Explicit backing fields can be used | on, WEAK WARNING | local | LATER | explicit backing fields (new language feature) |
| `ConvertToStringTemplate` | String concatenation that can be converted to string template | on, WEAK WARNING | local | MUST | KT-INS-032 |
| `ConvertTryFinallyToUseCall` | Convert try / finally to use() call | on, WEAK WARNING | local | SHOULD | KT-INS-090 |
| `ConvertTwoComparisonsToRangeCheck` | Two comparisons should be converted to a range check | on, WEAK WARNING | local | SHOULD | KT-INS-058 |
| `ConvertTypeOfToCollectionLiteral` | Collection literal can be used | on, INFORMATION | local | LATER | collection literals (new language feature) |
| `CopyWithoutNamedArguments` | 'copy' method of data class is called without named arguments | on, WEAK WARNING | local | SHOULD |  |
| `DestructuringDeclaration` | Use destructuring declaration | on, WEAK WARNING | local | SHOULD | KT-INS-057 |
| `DoubleBang` | Not-null assertion can be replaced with 'error()' | on, INFORMATION | local+cleanup | SHOULD |  |
| `FilterIsInstanceCallWithClassLiteralArgument` | 'filterIsInstance' call with a class literal argument | on, WEAK WARNING | local+cleanup | SHOULD |  |
| `FoldInitializerAndIfToElvis` | If-Null return/break/... foldable to '?:' | on, WEAK WARNING | local | MUST | KT-INS-036 |
| `IfThenToElvis` | If-Then foldable to '?:' | on, WEAK WARNING | local | MUST | KT-INS-034 |
| `IfThenToSafeAccess` | If-Then foldable to '?.' | on, WEAK WARNING | local | MUST | KT-INS-035 |
| `ImplicitThis` | Implicit 'this' | on, INFORMATION | local | SHOULD |  |
| `IntroduceWhenSubject` | 'when' that can be simplified by introducing an argument | on, WEAK WARNING | local | SHOULD |  |
| `JavaMapForEach` | Java Map.forEach method call should be replaced with Kotlin's forEach | on, WEAK WARNING | local | SHOULD |  |
| `JoinDeclarationAndAssignment` | Join declaration and assignment | on, WEAK WARNING | local | MUST | KT-INS-038 |
| `LiftReturnOrAssignment` | Return or assignment can be lifted out | on, WEAK WARNING | local | MUST | KT-INS-037 |
| `MapGetWithNotNullAssertionOperator` | 'map.get()' with not-null assertion operator (!!) | on, INFORMATION | local | SHOULD | KT-INS-080 |
| `MapToForEach` | 'map' call can be replaced with 'forEach' | on, WEAK WARNING | local | SHOULD |  |
| `MayBeConstant` | Might be 'const' | on, WEAK WARNING | local | SHOULD | KT-INS-064 |
| `MemberVisibilityCanBePrivate` | Class member can have 'private' visibility | off, WEAK WARNING | local | SHOULD | KT-INS-093 |
| `MoveLambdaOutsideParentheses` | Lambda argument inside parentheses | on, WEAK WARNING | local | MUST | KT-INS-041 |
| `MoveVariableDeclarationIntoWhen` | Variable declaration could be moved into 'when' | on, WEAK WARNING | local+cleanup | SHOULD |  |
| `NestedLambdaShadowedImplicitParameter` | Nested lambda has shadowed implicit parameter | on, WEAK WARNING | local | SHOULD |  |
| `NullableBooleanElvis` | Equality check can be used instead of elvis for nullable boolean check | on, INFORMATION | local+cleanup | SHOULD | KT-INS-061 |
| `NullableHashCode` | Nullable value 'hashCode()' can be simplified | on, WEAK WARNING | local+cleanup | SHOULD |  |
| `ObjectLiteralToLambda` | Object literal can be converted to lambda | on, WEAK WARNING | local | SHOULD | KT-INS-096 |
| `ProtectedInFinal` | 'protected' visibility is effectively 'private' in a final class | on, WEAK WARNING | local | SHOULD | KT-INS-065 |
| `RedundantAsSequence` | Redundant 'asSequence' call | on, WEAK WARNING | local | SHOULD |  |
| `RedundantElseInIf` | Redundant 'else' | on, INFORMATION | local | SHOULD |  |
| `RedundantObjectTypeCheck` | Non-idiomatic 'is' type check for an object | on, INFORMATION | local | SHOULD |  |
| `RedundantRunCatching` | Redundant 'runCatching' call | on, WEAK WARNING | local | SHOULD |  |
| `RemoveEmptyParenthesesFromAnnotationEntry` | Redundant parentheses in annotation | on, WEAK WARNING | local | SHOULD |  |
| `RemoveUnnecessaryParentheses` | Redundant parentheses | on, INFORMATION | local | SHOULD |  |
| `ReplaceAddAllWithMapTo` | 'addAll(map/filter {})' can be replaced with 'mapTo/filterTo' | on, INFORMATION | local | SHOULD |  |
| `ReplaceArrayOfWithLiteral` | 'arrayOf' call can be replaced with array literal [...] | on, WEAK WARNING | local | SHOULD |  |
| `ReplaceAssertBooleanWithAssertEquality` | Assert boolean could be replaced with assert equality | on, WEAK WARNING | local | SHOULD |  |
| `ReplaceAssociateFunction` | 'associate' can be replaced with 'associateBy' or 'associateWith' | on, WEAK WARNING | local | SHOULD |  |
| `ReplaceCallWithBinaryOperator` | Can be replaced with binary operator | on, WEAK WARNING | local | SHOULD |  |
| `ReplaceCollectionCountWithSize` | Collection count can be converted to size | off, WEAK WARNING | local | LATER | off by default |
| `ReplaceContains` | Replace 'contains' call with 'in' operator | on, INFORMATION | local+cleanup | SHOULD |  |
| `ReplaceGetOrSet` | Explicit 'get' or 'set' call | on, WEAK WARNING | local | MUST | KT-INS-042 |
| `ReplaceGuardClauseWithFunctionCall` | Guard clause can be replaced with Kotlin's function call | on, INFORMATION | local | SHOULD |  |
| `ReplaceIfExpressionWithFirstOrNull` | 'if' expression can be replaced with 'firstOrNull()' | on, WEAK WARNING | local | SHOULD |  |
| `ReplaceIsEmptyWithIfEmpty` | 'if' condition can be replaced with lambda call | on, WEAK WARNING | local | SHOULD |  |
| `ReplaceJavaStaticMethodWithKotlinAnalog` | Java methods should be replaced with Kotlin analog | on, WEAK WARNING | local | SHOULD |  |
| `ReplaceManualRangeWithIndicesCalls` | Range can be converted to indices or iteration | on, WEAK WARNING | local | SHOULD |  |
| `ReplaceMapGetOrDefault` | 'map.getOrDefault()' can be replaced with indexing and elvis operator | on, INFORMATION | local | SHOULD | KT-INS-089 |
| `ReplaceMapIndexedWithListGenerator` | Replace 'mapIndexed' with List generator | on, WEAK WARNING | local | SHOULD |  |
| `ReplaceMapKeysCallChainWithKeys` | Replace 'map { it.key }.toSet()' with 'keys' | on, INFORMATION | local+cleanup | SHOULD |  |
| `ReplaceNegatedIsEmptyWithIsNotEmpty` | Negated call can be simplified | on, WEAK WARNING | local | SHOULD | KT-INS-078 |
| `ReplaceNotNullAssertionWithElvisReturn` | Not-null assertion can be replaced with 'return' | off, WEAK WARNING | local | LATER | off by default |
| `ReplacePutWithAssignment` | 'map.put()' can be converted to assignment | on, WEAK WARNING | local | MUST | KT-INS-043 |
| `ReplaceRangeStartEndInclusiveWithFirstLast` | Boxed properties should be replaced with unboxed | on, WEAK WARNING | local | SHOULD |  |
| `ReplaceRangeToWithRangeUntil` | 'rangeTo' or the '..' call should be replaced with '..&lt;' | on, WEAK WARNING | local | SHOULD |  |
| `ReplaceRangeToWithUntil` | 'rangeTo' or the '..' call should be replaced with 'until' | on, WEAK WARNING | local | SHOULD | KT-INS-056 |
| `ReplaceReadLineWithReadln` | 'readLine' can be replaced with 'readln' or 'readlnOrNull' | on, WEAK WARNING | local+cleanup | SHOULD |  |
| `ReplaceSizeCheckWithIsNotEmpty` | Size check can be replaced with 'isNotEmpty()' | on, WEAK WARNING | local | MUST | KT-INS-040 |
| `ReplaceSizeZeroCheckWithIsEmpty` | Size zero check can be replaced with 'isEmpty()' | on, WEAK WARNING | local | MUST | KT-INS-039 |
| `ReplaceStringFormatWithLiteral` | 'String.format' call can be replaced with string templates | on, INFORMATION | local | SHOULD |  |
| `ReplaceSubstringWithDropLast` | 'substring' call should be replaced with 'dropLast' call | on, INFORMATION | local | SHOULD |  |
| `ReplaceSubstringWithIndexingOperation` | 'substring' call should be replaced with indexing operator | on, INFORMATION | local | SHOULD |  |
| `ReplaceSubstringWithSubstringAfter` | 'substring' call should be replaced with 'substringAfter' | on, INFORMATION | local | SHOULD |  |
| `ReplaceSubstringWithSubstringBefore` | 'substring' call should be replaced with 'substringBefore' | on, INFORMATION | local | SHOULD |  |
| `ReplaceSubstringWithTake` | 'substring' call should be replaced with 'take' call | on, INFORMATION | local | SHOULD | KT-INS-094 |
| `ReplaceToStringWithStringTemplate` | Call of 'toString' could be replaced with string template | on, INFORMATION | local | SHOULD |  |
| `ReplaceToWithInfixForm` | 'to' call should be replaced with infix form | on, WEAK WARNING | local | SHOULD | KT-INS-054 |
| `ReplaceUntilWithRangeUntil` | Replace 'until' with '..&lt;' operator | on, INFORMATION | local | SHOULD |  |
| `ReplaceWithCallWithContextCall` | Replace 'with' with 'context' | on, WEAK WARNING | local | LATER | context parameters (new language feature) |
| `ReplaceWithIgnoreCaseEquals` | Should be replaced with 'equals(..., ignoreCase = true)' | on, WEAK WARNING | local | SHOULD | KT-INS-079 |
| `ReplaceWithImportAlias` | Fully qualified name can be replaced with existing import alias | on, INFORMATION | local | SHOULD |  |
| `ReplaceWithOperatorAssignment` | Assignment can be replaced with operator assignment | on, WEAK WARNING | local | SHOULD | KT-INS-059 |
| `RevertExplicitBackingFields` | Replace explicit backing field with private property | on, INFORMATION | local | LATER | explicit backing fields (new language feature) |
| `SafeCastWithReturn` | Safe cast with 'return' should be replaced with 'if' type check | on, WEAK WARNING | local | SHOULD |  |
| `SimplifiableCall` | Library function call could be simplified | on, WEAK WARNING | local | SHOULD |  |
| `SimplifiableCallChain` | Call chain on collection type can be simplified | on, WEAK WARNING | local | SHOULD | KT-INS-055 |
| `SimplifyAssertNotNull` | 'assert' call can be replaced with '!!' or '?:' | on, INFORMATION | local | SHOULD |  |
| `SimplifyBooleanWithConstants` | Boolean expression can be simplified | on, WEAK WARNING | local | SHOULD | KT-INS-072 |
| `SimplifyNegatedBinaryExpression` | Negated boolean expression can be simplified | on, WEAK WARNING | local | SHOULD |  |
| `SimplifyNestedEachInScopeFunction` | Scope function with nested forEach can be simplified | on, WEAK WARNING | local | SHOULD |  |
| `SortModifiers` | Non-canonical modifier order | on, WEAK WARNING | local+cleanup | SHOULD | KT-INS-077 |
| `SuspiciousAsDynamic` | Suspicious 'asDynamic' member invocation | on, WEAK WARNING | local | LATER | Kotlin/JS dynamic |
| `TrailingComma` | Trailing comma recommendations | on, WEAK WARNING | local | SHOULD | KT-INS-092 |
| `UnclearPrecedenceOfBinaryExpression` | Multiple operators with different precedence | on, WARNING | local | SHOULD | KT-INS-076 |
| `UnlabeledReturnInsideLambda` | Unlabeled return inside lambda | on, INFORMATION | local | SHOULD |  |
| `UnsafeCastWithReturn` | Unsafe cast with 'return' should be replaced with the safe one | on, WARNING | local | SHOULD |  |
| `UseExpressionBody` | Expression body syntax is preferable here | on, INFORMATION | local | MUST | KT-INS-030 |
| `UsePropertyAccessSyntax` | Accessor call that can be replaced with property access syntax | on, WEAK WARNING | local+cleanup | MUST | KT-INS-031 |
| `UseWithIndex` | Manually incremented index variable can be replaced with use of 'withIndex()' | on, WEAK WARNING | local | SHOULD |  |
| `VerboseNullabilityAndEmptiness` | Verbose nullability and emptiness check | on, WEAK WARNING | local | SHOULD |  |

### Kotlin,React / Probable bugs

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `NonExternalClassifierExtendingStateOrProps` | Non-external classifier extending State or Props | on, WARNING | local | LATER | Kotlin/JS React support |

### Lombok / Kotlin

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `LombokKotlinCompilerPlugin` | Add Kotlin Lombok compiler plugin to module build | on, WARNING | local | LATER | Lombok compiler-plugin support |

### Workspace model

| Short name | Display name | Default | kind | Pri | Note |
|---|---|---|---|---|---|
| `WorkspaceEntityMutableField` | Unsupported 'var' field in an entity | on, WARNING | local | LATER | IntelliJ workspace-model development |
| `WorkspaceImplAbsent` | Generate implementation | on, WARNING | local | LATER | IntelliJ workspace-model development |
| `WorkspaceImplObsolete` | Obsolete version of entity implementation | on, WARNING | local | LATER | IntelliJ workspace-model development |
| `WorkspaceInheritance` | Workspace inheritance | on, WARNING | local | LATER | IntelliJ workspace-model development |


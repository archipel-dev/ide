# 06. Kotlin intentions and quick fixes

Status: specification, clean-room, our own words. Requirement prefixes `KT-INT` (intentions) and `KT-QF` (quick fixes and actions). Executable cases:
[cases/06-intentions-and-quickfixes.cases.md](cases/06-intentions-and-quickfixes.cases.md).

ic: `plugins/kotlin/code-insight/intentions-k2/src/**` (intentions), `plugins/kotlin/code-insight/fixes-k2/src/**` (quick fixes;
`KotlinK2QuickFixRegistrar.kt` maps diagnostics to fixes), `plugins/kotlin/code-insight/impl-base/src/**/createFromUsage/*`,
`plugins/kotlin/code-insight/descriptions/resources-en/intentionDescriptions/*` (descriptions), `KotlinBundle.properties` (texts),
`plugins/kotlin/j2k/**`. Inventory: `docs/specs/inventory/intentions-kotlin.md` (176 intentions), `docs/specs/inventory/kotlin.md` (actions,
settings, run configurations).

The Alt+Enter framework (list composition, priorities, preview, "Fix all") is `codeinsight/04-intentions-and-alt-enter.md`; this file lists Kotlin's contents.

---

## 6.1 Contract

- An **intention** is available when the caret is in its *range* (the element named in the table or inside it) and the code has the shape it needs; it
  appears in the Alt+Enter list under the lightbulb with its **row text**, below error fixes. Intentions never run unasked and never change behaviour unless the text says so.
- A **quick fix** belongs to a diagnostic or an inspection (`03` 3.3 Fixes column, `05` 5.3).
- Intentions that depend on a declared type require resolved types; on unresolved code they are absent.
- Every intention is applied in one undo step; the caret after application is at the start of the changed construct unless the case says otherwise.
- Most intentions offer an **intention preview** (the platform's diff popup, `codeinsight/04`).

`KT-INT-001` MUST: availability, row text, one undo step, preview for the intentions of 6.2. Acceptance: each case of 6.2.

## 6.2 Catalog of the common intentions

Row text = the exact Alt+Enter text. Cases show before/after.

| ID | Pri | Class | Row text | Trigger |
|---|---|---|---|---|
| KT-INT-010 | MUST | `ConvertToBlockBodyIntention` | Convert to block body | case KT-INT-010  |
| KT-INT-011 | MUST | `SpecifyTypeExplicitlyIntention` | Specify type explicitly | case KT-INT-011  |
| KT-INT-012 | MUST | `RemoveExplicitTypeIntention` | Remove explicit type specification | case KT-INT-012  |
| KT-INT-013 | MUST | `IfToWhenIntention` | Replace 'if' with 'when' | case KT-INT-013 [verify braces on branches] |
| KT-INT-014 | MUST | `WhenToIfIntention` | Replace 'when' with 'if' | case KT-INT-014 [verify layout] |
| KT-INT-015 | MUST | `AddBracesIntention` | Add braces to 'if' statement | case KT-INT-015  |
| KT-INT-016 | MUST | `RemoveBracesIntention` | Remove braces from 'if' statement | case KT-INT-016  |
| KT-INT-017 | MUST | `ConvertToStringTemplateIntention` | Convert concatenation to template | case KT-INT-017  |
| KT-INT-018 | MUST | `InvertIfConditionIntention` | Invert 'if' condition | case KT-INT-018  |
| KT-INT-019 | MUST | `SwapBinaryExpressionIntention` | Flip '+' | case KT-INT-019  |
| KT-INT-020 | MUST | `ConvertLambdaToReferenceIntention` | Convert lambda to reference | case KT-INT-020  |
| KT-INT-021 | MUST | `ReplaceItWithExplicitFunctionLiteralParamIntention` | Replace 'it' with explicit parameter | case KT-INT-021 [verify default name] |
| KT-INT-022 | MUST | `ReplaceExplicitFunctionLiteralParamWithItIntention` | Replace explicit parameter 'x' with 'it' | case KT-INT-022  |
| KT-INT-023 | MUST | `AddNamesToCallArgumentsIntention` | Add names to call arguments | case KT-INT-023  |
| KT-INT-024 | MUST | `RemoveSingleArgumentNameIntention` | Remove argument name | case KT-INT-024  |
| KT-INT-025 | MUST | `ImportMemberIntention` | Add import for 'invokeLater' | case KT-INT-025 all occurrences in the file are rewritten |
| KT-INT-026 | MUST | `ImportAllMembersIntention` | Import members from 'javax.swing.SwingUtilities' | case KT-INT-026 [verify row text] |
| KT-INT-027 | MUST | `IntroduceImportAliasIntention` | Introduce import alias | case KT-INT-027 inline rename of the alias name follows |
| KT-INT-028 | MUST | `AddValVarToConstructorParameterActionIntention` | Add 'val' or 'var' to parameter 'email' | case KT-INT-028 [verify chooser] |
| KT-INT-029 | MUST | `MovePropertyToConstructorIntention` | Move to constructor | case KT-INT-029  |
| KT-INT-030 | MUST | `MovePropertyToClassBodyIntention` | Move to class body | case KT-INT-030  |
| KT-INT-031 | MUST | `ConvertPropertyToFunctionIntention` | Convert property to function | case KT-INT-031 usages updated |
| KT-INT-032 | MUST | `ConvertFunctionToPropertyIntention` | Convert function to property | case KT-INT-032 usages updated |
| KT-INT-033 | MUST | `SplitPropertyDeclarationIntention` | Split property declaration | case KT-INT-033  |
| KT-INT-034 | MUST | `SplitIfIntention` | Split 'if' into two | case KT-INT-034 [verify indent] |
| KT-INT-035 | MUST | `MergeIfsIntention` | Merge 'if's | case KT-INT-035  |
| KT-INT-036 | MUST | `ToRawStringLiteralIntention` | Convert to raw string literal | case KT-INT-036 [verify] |
| KT-INT-037 | MUST | `OperatorToFunctionIntention` | Replace overloaded operator with function call | case KT-INT-037  |
| KT-INT-038 | MUST | `ToInfixCallIntention` | Replace with infix function call | case KT-INT-038  |
| KT-INT-039 | MUST | `MoveLambdaInsideParenthesesIntention` | Move lambda argument into parentheses | case KT-INT-039  |
| KT-INT-040 | MUST | `ConvertForEachToForLoopIntention` | Replace with a for loop | case KT-INT-040 [verify row text] |
| KT-INT-041 | MUST | `ConvertToForEachFunctionCallIntention` | Replace with a forEach function call | case KT-INT-041 [verify row text] |
| KT-INT-042 | MUST | `IterateExpressionIntention` | Iterate over collection | case KT-INT-042 [verify] |
| KT-INT-043 | MUST | `AddWhenRemainingBranchesIntention` | Add remaining branches | case KT-INT-043  |
| KT-INT-044 | MUST | `ImplementAbstractMemberIntention` | Implement abstract member | case KT-INT-044 [verify] |
| KT-INT-045 | MUST | `ConvertToApplyIntention` | Convert to apply | case KT-INT-045 [verify row text] |
| KT-INT-046 | MUST | `ConvertToWithIntention` | Convert to with | case KT-INT-046 [verify row text] |
| KT-INT-047 | MUST | `SafeAccessToIfThenIntention` | Replace safe access expression with 'if' expression | case KT-INT-047 [verify row text] |
| KT-INT-048 | MUST | `ElvisToIfThenIntention` | Replace elvis expression with 'if' expression | case KT-INT-048 [verify row text] |
| KT-INT-049 | MUST | `AddJvmStaticIntention` | Add '@JvmStatic' annotation | case KT-INT-049  |
| KT-INT-050 | MUST | `AddJvmOverloadsIntention` | Add '@JvmOverloads' annotation | case KT-INT-050  |

## 6.3 Create from usage

For an unresolved reference the lightbulb offers, depending on position (first row after `Import ...`, `02`): `Create function 'foo'`, `Create member function 'A.foo'`,
`Create extension function 'String.foo'`, `Create abstract function 'I.foo'`, `Create property 'foo'`, `Create local variable 'foo'`, `Create parameter 'foo'`,
`Create property 'foo' as constructor parameter`, `Create class 'Foo'` / `Create interface 'Foo'` / `Create enum 'Foo'` / `Create annotation 'Foo'` / `Create object 'Foo'`, `Create package 'x'`.
Callable fixes insert a stub with the inferred parameter and return types and start an editor template to confirm names and types; class fixes ask the target (a chooser titled
`Choose class container` when several containers fit) and create the declaration in place (nested/top-level) or in a new file named after the class.
Class fixes have normal priority only when the name starts with an upper-case letter, else low.

`KT-QF-001` SHOULD: create function/property/local/parameter/class fixes with the texts above. Acceptance: cases KT-QF-001.

## 6.4 Quick fixes of diagnostics

The mapping of diagnostics to fixes, with fix texts, is the Fixes column of `03` 3.3. Principal families (texts exact): null safety (`Replace with safe (?.) call`, `Add non-null asserted (x!!) call`,
`Wrap with '?.let { ... }' call`, `Remove unnecessary non-null assertion (!!)`, `Replace with dot call`), types (`Change type of 'x' to 'T'`, `Change return type of ...`, `Cast expression 'x' to 'T'`,
`Add 'toString()' call`, `Change parameter 'p' type of function 'f' to 'T'`), modifiers (`Add 'x' modifier`, `Remove 'x' modifier`, `Make 'x' open`, `Make 'C' abstract`, `Add 'override' modifier`),
`when` (`Add remaining branches`, `Add else branch`), initialisation (`Add initializer`, `Add 'lateinit' modifier`, `Initialize with constructor parameter`), arguments (`Add name to argument...`,
`Specify remaining required arguments by name`, `Remove argument`, `Change the signature of function 'f'`), members (`Implement members`, `Specify override for 'f' explicitly`),
opt-in (`Opt in for 'X' on ...`, `Propagate 'X' opt-in requirement to ...`).

`KT-QF-002` MUST: the fixes of 03 3.3 with MUST priority. Acceptance: the fix cases of `cases/03`.

## 6.5 Generate and other actions

| ID | Pri | Action (inventory id) | Text | Behaviour |
|---|---|---|---|---|
| KT-QF-010 | SHOULD | `KotlinGenerateToString` | `toString()` | Generate menu (Alt+Insert): dialog `Choose Properties To Be Included`, template chooser (`Single template`, `Single template with super call`, `Multiple templates with concatenation`, `Multiple templates with concatenation and super call`), checkbox `Generate call to super.toString()` |
| KT-QF-011 | SHOULD | `KotlinGenerateEqualsAndHashCode` | `equals() and hashCode()` | dialogs `Choose Properties To Be Included In 'equals()'` and `... 'hashCode()'`; replaces existing after confirmation `Functions ... are already defined for class ... Do you want to delete them and proceed?` |
| KT-QF-012 | SHOULD | `KotlinGenerateSecondaryConstructor` | `Secondary Constructor` | dialog `Choose Properties to Initialize by Constructor`; error `Constructor already exists` |
| KT-QF-013 | LATER | `KotlinGenerateDataMethod` | `Parameters Function` | generates `componentN`/`copy` helper |
| KT-QF-014 | LATER | `KotlinGenerateTestMethod`, `KotlinGenerateSetUpMethod`, `KotlinGenerateTearDownMethod` | `Test Function`, `SetUp Function`, `TearDown Function` | test-framework model |
| KT-QF-015 | SHOULD | `ConvertJavaToKotlin` | `Convert Java to Kotlin` (`Ctrl+Alt+Shift+K`) | converts selected Java files in place (rename `.java` to `.kt` via VCS-aware move); dialog `Some code in the rest of your project may require corrections after performing this conversion. Do you want to find such code and correct it too?` with `Investigate Errors` / `Proceed with Conversion`; clipboard paste of Java code asks `Clipboard content seems to be Java code. Do you want to convert it to Kotlin?`. LATER for Archipel's own converter; the behaviour contract is stated here |
| KT-QF-016 | SHOULD | `ExtractFunction`, `ExtractFunctionToScope`, `IntroduceProperty`, `IntroduceTypeAlias` | `Extract Function...`, `Extract Function to Scope...`, `Introduce Property...`, `Introduce Type Alias...` | refactorings: out of this spec (refactoring specs) |
| KT-QF-017 | LATER | `Kotlin.NewFile`, `Kotlin.NewScript` | `Kotlin Class/File`, `Kotlin Script` | New menu: dialog `New Kotlin Class/File` with kinds `Class`, `File`, `Data class`, `Sealed class`, `Annotation`, `Interface`, `Sealed interface`, `Enum class`, `Object`; errors `Name can't be empty`, `Name can't have empty parts`, `Name can't consist only of underscores` |
| KT-QF-018 | LATER | `ShowKotlinBytecode`, `DecompileKotlinToJava`, `Kotlin.RunScratch`, `Kotlin.ClearScratch`, `Kotlin.StopScratch`, `KotlinScripts.*`, `ConfigureKotlinInProject`, `KotlinCodeMigration*`, `Gradle.MigrateToDaemonJvmCriteria`, `LibraryToSourceDependencySupportToggleAction`, `Kotlin.Gradle.ShowDslLogs`, `Kotlin.XDebugger.ToggleKotlinVariableView`, `KtAddToExcludeListAction`, `CheckComponentsUsageSearchAction`, the Maven `KotlinGenerate*` actions | tooling, scripting, debugger and build integration |
| KT-QF-019 | LATER | internal and survey actions (`CreateIncrementalCompilationBackup`, `InspectBreakpointApplicability`, `InvalidateK2CachesInternalAction`, `KotlinCompilerReferenceIndexVerifierAction`, `KotlinCriDiagnostics`, `ReactivePostOpenProjectActionsAction`, `org.jetbrains.kotlin.onboarding.*`) | IDE-development or telemetry |

Settings pages: `preferences.language.Kotlin` (Kotlin), `project.kotlinCompiler` (Kotlin Compiler, project scope), `preferences.language.Kotlin.scripting` (Kotlin Scripting): `KT-QF-020` LATER
(compiler options UI, scripting). Run configuration types `KotlinRunConfigurationType`, `KotlinStandaloneScriptRunConfigurationType`: `KT-QF-021` LATER (run/debug specs). The Kotlin code-style page
(import layout, trailing commas, spaces) is specified in `02` 2.10.3 and the formatter spec.

## Appendix A. Inventory of all 176 intentions

Priority: `MUST` (6.2), `SHOULD` (after the MUST set; behaviour as in its description), `LATER` (reason). The description column is the first sentence of the intention's description page.

`KT-INT-200` SHOULD: every intention marked SHOULD, with row text from its bundle entry and the 6.1 contract. Acceptance: availability and result tests per intention.
`KT-INT-201` LATER: every intention marked LATER.

| Class | Pri | Description / reason |
|---|---|---|
| `AddAnnotationUseSiteTargetIntention` | SHOULD | Adds a use-site target to the selected annotation that is applicable to multiple annotation targets. |
| `AddBracesIntention` | MUST (KT-INT-015) | Adds braces to control flow statements. |
| `AddBracesToAllBranchesIntention` | SHOULD | Adds braces to all branches of if or when . |
| `AddForLoopIndicesIntention` | SHOULD | Adds an index variable to a for loop iterating over a collection. |
| `AddFullQualifierIntention` | SHOULD | Adds a full qualifier prefix to the selected reference. |
| `AddJvmOverloadsIntention` | MUST (KT-INT-050) | Adds a @JvmOverloads annotation to functions and constructors with default arguments. |
| `AddJvmStaticIntention` | MUST (KT-INT-049) | Adds a @JvmStatic annotation to the specified function or property of an object declaration. |
| `AddLabeledReturnInLambdaIntention` | SHOULD | Adds a labeled return to the last expression in a lambda. |
| `AddMissingClassKeywordIntention` | SHOULD | Adds a missing class keyword for special class kinds, such as enum . |
| `AddNameToArgumentIntention` | SHOULD | Adds a parameter name to the selected argument of a function. |
| `AddNamesInCommentToJavaCallArgumentsIntention` | SHOULD | Adds names in comments to all arguments of Java method calls or constructor calls. |
| `AddNamesToCallArgumentsIntention` | MUST (KT-INT-023) | Adds names to all positional arguments of a function call according to the named argument syntax. |
| `AddNamesToFollowingArgumentsIntention` | SHOULD | Adds names to all positional arguments of a function call according to the named argument syntax, starting wit… |
| `AddOpenModifierIntention` | SHOULD | Adds the open modifier to a non-private method or property in an open class. |
| `AddParameterNameAtCallsitesIntention` | SHOULD | Adds the parameter name to the given argument at all Kotlin callsites of the function. |
| `AddPropertyAccessorsIntention` | SHOULD | Adds explicit accessors for a property. |
| `AddPropertyGetterIntention` | SHOULD | Adds a property getter. |
| `AddPropertySetterIntention` | SHOULD | Adds a property setter. |
| `AddThrowsAnnotationIntention` | SHOULD | Adds a @Throws annotation for an exception at the caret. |
| `AddUnderscoresToNumericLiteralIntention` | LATER | type-argument underscore (new language feature) |
| `AddValVarToConstructorParameterActionIntention` | MUST (KT-INT-028) | Adds a val or var keyword to a primary constructor parameter, making it a property. |
| `AddWhenRemainingBranchesIntention` | MUST (KT-INT-043) | Adds remaining branches for the selected when expression. |
| `AnonymousFunctionToLambdaIntention` | SHOULD | Converts an anonymous function passed as an argument to a lambda expression. |
| `ChangePackageIntention` | SHOULD | Changes the qualified package name of the current file without moving the file itself. |
| `ChangeVisibilityModifierIntentionChooser` | SHOULD | Changes the declaration visibility. |
| `ChopArgumentListIntention` | LATER | formatting action |
| `ChopParameterListIntention` | LATER | formatting action |
| `ConvertAnyToAllAndViceVersaIntention` | SHOULD | Replaces an any function call with an all and vice versa. |
| `ConvertAnyToNoneAndViceVersaIntention` | SHOULD | Replaces an any function call with a none and vice versa. |
| `ConvertArrayParameterToVarargIntention` | SHOULD | Converts an array parameter to a vararg parameter. |
| `ConvertBinaryExpressionWithDemorgansLawIntention` | SHOULD | Converts a boolean expression to an equivalent expression using De Morgan's laws: |
| `ConvertBlockCommentToLineCommentIntention` | SHOULD | Replaces a block comment ( /* ... |
| `ConvertCallToOppositeIntention` | SHOULD | Replaces all/none/filter/filterNot/filterTo/filterNotTo/takeIf/takeUnless function calls with the opposite. |
| `ConvertCamelCaseTestFunctionToSpacedIntention` | LATER | test framework model |
| `ConvertCollectionConstructorToFunction` | LATER | collection literals / builders |
| `ConvertConcatenationToBuildStringIntention` | SHOULD | Converts a statement that builds String using + to one that builds String using a buildString call. |
| `ConvertContextParameterToReceiverIntention` | LATER | context parameters (new language feature) |
| `ConvertContextParameterToRegularParameterIntention` | LATER | context parameters (new language feature) |
| `ConvertEnumToSealedClassIntention` | LATER | sealed/enum conversion refactoring |
| `ConvertForEachToForLoopIntention` | MUST (KT-INT-040) | Converts a forEach function call into a for loop. |
| `ConvertFunctionToPropertyIntention` | MUST (KT-INT-032) | Converts a function to a property. |
| `ConvertFunctionTypeParameterToReceiverIntention` | SHOULD | Converts a given parameter of function type used as a parameter in another function to receiver. |
| `ConvertFunctionTypeReceiverToParameterIntention` | SHOULD | Converts a function type with receiver (e.g., String.() -&gt; Int ) into a function type with the receiver as the… |
| `ConvertLambdaToMultiLineIntention` | SHOULD | Converts a single-line lambda to a multi-line one. |
| `ConvertLambdaToReferenceIntention` | MUST (KT-INT-020) | Converts a lambda (function literal) expression that calls a single function to a function reference. |
| `ConvertLambdaToSingleLineIntention` | SHOULD | Converts a multi-line lambda to a single-line one. |
| `ConvertLateinitPropertyToNullableIntention` | LATER | property delegation conversion |
| `ConvertLazyPropertyToOrdinaryIntention` | LATER | property delegation conversion |
| `ConvertLineCommentToBlockCommentIntention` | SHOULD | Replaces one or more end-of-line comments ( // ... |
| `ConvertMemberToExtensionIntention` | SHOULD | Converts a member of a class or an interface into an extension to this class/interface. |
| `ConvertNullablePropertyToLateinitIntention` | LATER | property delegation conversion |
| `ConvertObjectLiteralToClassIntention` | SHOULD | Converts an object literal to a class and replaces the object literal expression with a constructor call. |
| `ConvertOptInMarkerToRequirementIntention` | LATER | opt-in requirements |
| `ConvertOrdinaryPropertyToLazyIntention` | LATER | property delegation conversion |
| `ConvertParameterToReceiverIntention` | SHOULD | Converts a function parameter to an extension receiver. |
| `ConvertPrimaryConstructorToSecondaryIntention` | SHOULD | Converts a primary constructor to a secondary one. |
| `ConvertPropertyGetterToInitializerIntention` | SHOULD | Converts a property getter to an initializer. |
| `ConvertPropertyInitializerToGetterIntention` | SHOULD | Converts a property initializer to a getter. |
| `ConvertPropertyToFunctionIntention` | MUST (KT-INT-031) | Converts a property to a function. |
| `ConvertReceiverParameterToContextParameterIntention` | LATER | context parameters (new language feature) |
| `ConvertReceiverToParameterIntention` | SHOULD | Converts the receiver of an extension function to its parameter. |
| `ConvertRegularParameterToContextParameterIntention` | LATER | context parameters (new language feature) |
| `ConvertRequirementToOptInIntention` | LATER | opt-in requirements |
| `ConvertSealedClassToEnumIntention` | LATER | sealed/enum conversion refactoring |
| `ConvertSnakeCaseTestFunctionToSpacedIntention` | LATER | test framework model |
| `ConvertStringTemplateToBuildStringIntention` | SHOULD | Converts a string template into a buildString call. |
| `ConvertToAlsoIntention` | SHOULD | Converts several calls with the same receiver to an also call. |
| `ConvertToApplyIntention` | MUST (KT-INT-045) | Converts several calls with the same receiver to an apply call. |
| `ConvertToBlockBodyIntention` | MUST (KT-INT-010) | Converts an expression body (a single expression after the = sign) to a block body with a return statement. |
| `ConvertToConcatenatedStringIntention` | SHOULD | Converts a String -building statement that uses a string template to one that uses string concatenation ( + ). |
| `ConvertToForEachFunctionCallIntention` | MUST (KT-INT-041) | Converts a for loop to a forEach function call. |
| `ConvertToIndexedFunctionCallIntention` | SHOULD | Converts a call of a collection function to a call of its *Indexed counterpart. |
| `ConvertToMultiDollarStringIntention` | LATER | multi-dollar interpolation (new language feature) |
| `ConvertToRawStringTemplateIntention` | SHOULD | Converts a String -building statement that uses concatenation ( + ) to one that uses a raw string. |
| `ConvertToRunIntention` | SHOULD | Converts several calls with the same receiver to a run call. |
| `ConvertToStringTemplateIntention` | MUST (KT-INT-017) | Converts a String -building statement that uses concatenation ( + ) to one that uses a string template. |
| `ConvertToWithIntention` | MUST (KT-INT-046) | Converts several calls with the same receiver to a with call. |
| `ConvertTrimIndentToTrimMarginIntention` | SHOULD | Converts a trimIndent() call to a trimMargin() call. |
| `ConvertTrimMarginToTrimIndentIntention` | SHOULD | Converts a trimMargin() call to a trimIndent() call. |
| `ConvertUnsafeCastCallToUnsafeCastIntention` | SHOULD | Converts an unsafeCast() call to an unsafe cast ( as ). |
| `ConvertVarargParameterToArrayIntention` | SHOULD | Converts a vararg parameter to an array parameter. |
| `ConvertVariableAssignmentToExpressionIntention` | SHOULD | Converts the assignment statement to an expression. |
| `CopyConcatenatedStringToClipboardIntention` | LATER | clipboard action |
| `CreateKotlinSubClassIntention` | SHOULD | Creates a default implementation for an interface, abstract class, open class, or sealed class. |
| `EliminateWhenSubjectIntention` | SHOULD | Inlines when argument into inner condition entries. |
| `ElvisToIfThenIntention` | MUST (KT-INT-048) | Converts an elvis expression ?: |
| `EvaluateCompileTimeExpressionIntention` | SHOULD | Replaces compile-time constant expression with its actual value. |
| `ExpandBooleanExpressionIntention` | SHOULD | Replaces the Boolean expression with an equivalent if-else expression. |
| `ExtractDeclarationFromCurrentFileIntention` | LATER | extract refactoring |
| `FlattenWhenIntention` | SHOULD | Flattens nested when expression into the enclosing when expression. |
| `FoldIfToFunctionCallIntention` | SHOULD | Lifts a function call out of an if expression. |
| `FoldIfToReturnAsymmetricallyIntention` | SHOULD | Converts a single-branch if expression immediately followed by return into a single return with an if expressi… |
| `FoldWhenToFunctionCallIntention` | SHOULD | Lifts a function call out of a when expression. |
| `GenerateCopyFunctionForFullValueClassIntention` | LATER | clipboard action |
| `IfToWhenIntention` | MUST (KT-INT-013) | Converts an if expression to an equivalent when expression. |
| `ImplementAbstractMemberAsConstructorParameterIntention` | SHOULD | Implements an abstract property as a constructor parameter in subclasses. |
| `ImplementAbstractMemberIntention` | MUST (KT-INT-044) | Implements an abstract member in subclasses. |
| `ImportAllMembersIntention` | MUST (KT-INT-026) | Adds the import of all members of selected reference and removes explicit usage of the reference in the file. |
| `ImportMemberIntention` | MUST (KT-INT-025) | Adds the import of selected member of the reference and removes explicit usage of the reference with the membe… |
| `IndentRawStringIntention` | SHOULD | Indents a raw string. |
| `InsertCurlyBracesToTemplateIntention` | SHOULD | Inserts curly braces around expressions in string templates. |
| `InsertExplicitTypeArgumentsIntention` | SHOULD | Inserts explicit type arguments in function calls. |
| `IntroduceBackingPropertyIntention` | SHOULD | Creates a backing property for the specified property. |
| `IntroduceImportAliasIntention` | MUST (KT-INT-027) | Creates an import alias for the import. |
| `IntroduceVariableIntention` | SHOULD | Extracts specified expression into a variable. |
| `InvertIfConditionIntention` | MUST (KT-INT-018) | Inverts if conditional expression. |
| `IterateExpressionIntention` | MUST (KT-INT-042) | Iterates over a given collection with a for loop. |
| `JoinArgumentListIntention` | LATER | formatting action |
| `JoinParameterListIntention` | LATER | formatting action |
| `KotlinCreateTestIntention` | LATER | test framework model |
| `LambdaToAnonymousFunctionIntention` | SHOULD | Converts a lambda expression into a call of an anonymous function. |
| `MavenPluginSourcesMoveToBuild` | LATER | Maven build support [XML files] |
| `MavenPluginSourcesMoveToExecutionIntention` | LATER | Maven build support [XML files] |
| `MergeElseIfIntention` | SHOULD | Merges the else branch of an if...else statement if this else branch only contains a nested if...else statemen… |
| `MergeIfsIntention` | MUST (KT-INT-035) | Merges two nested if statements without else branches into a single if . |
| `MoveFunctionToClassIntention` | LATER | move refactoring |
| `MoveLambdaInsideParenthesesIntention` | MUST (KT-INT-039) | Moves a lambda expression inside parentheses. |
| `MoveMemberOutOfCompanionObjectIntention` | LATER | move refactoring |
| `MoveMemberToCompanionBlockIntention` | LATER | move refactoring |
| `MoveMemberToCompanionExtensionIntention` | LATER | move refactoring |
| `MoveMemberToCompanionObjectIntention` | LATER | move refactoring |
| `MoveMemberToTopLevelIntention` | LATER | move refactoring |
| `MovePropertyToClassBodyIntention` | MUST (KT-INT-030) | Moves a property from the primary constructor to the class body. |
| `MovePropertyToConstructorIntention` | MUST (KT-INT-029) | Moves the specified property to the primary constructor. |
| `NullableBooleanEqualityCheckToElvisIntention` | SHOULD | Converts a nullable boolean equality check to an Elvis operator. |
| `OperatorToFunctionIntention` | MUST (KT-INT-037) | Replaces an overloaded operator with a function call. |
| `PutCallsOnSeparateLinesIntention` | LATER | formatting action |
| `PutExpressionsOnSeparateLinesIntention` | LATER | formatting action |
| `ReconstructTypeInCastOrIsIntention` | SHOULD | Replaces a shortened type on the right-hand side of as / as? |
| `RemoveAllArgumentNamesIntention` | SHOULD | Removes names from named arguments where possible, reordering arguments to match parameter order. |
| `RemoveBracesFromAllBranchesIntention` | SHOULD | Removes braces from all branches of if/when . |
| `RemoveBracesIntention` | MUST (KT-INT-016) | Removes braces from control flow statements with only a single statement in their code block. |
| `RemoveExplicitLambdaParameterTypesIntention` | SHOULD | Removes explicit parameter types in a lambda expression. |
| `RemoveExplicitTypeIntention` | MUST (KT-INT-012) | Removes an explicit type specification for local values, variables, properties, and functions. |
| `RemoveForLoopIndicesIntention` | SHOULD | Reports for loops iterating over a collection using the withIndex() function and not using the index variable. |
| `RemoveSingleArgumentNameIntention` | MUST (KT-INT-024) | Removes the name from a named argument to use positional argument syntax. |
| `RemoveUnderscoresFromNumericLiteralIntention` | LATER | type-argument underscore (new language feature) |
| `RenameClassToContainingFileNameIntention` | LATER | rename refactoring |
| `RenameFileToMatchClassIntention` | LATER | rename refactoring |
| `ReplaceAddWithPlusAssignIntention` | SHOULD | Replaces calls to add() and addAll() in a mutable collection with a += operator. |
| `ReplaceCallWithUnaryOperatorIntention` | SHOULD | Replaces prefix member calls ( plus , minus , not ) with their respective prefix operators. |
| `ReplaceExplicitFunctionLiteralParamWithItIntention` | MUST (KT-INT-022) | Replaces the explicit parameter of a single-parameter lambda expression with an implicit it parameter. |
| `ReplaceForEachWithRepeatIntention` | SHOULD | Converts forEach over a zero-based range to an equivalent repeat(n) { ... |
| `ReplaceForLoopWithRepeatIntention` | SHOULD | Converts for loop over a range starting from 0 to an equivalent repeat(n) { ... |
| `ReplaceInvokeIntention` | SHOULD | Replaces invoke member calls with a receiver call. |
| `ReplaceItWithExplicitFunctionLiteralParamIntention` | MUST (KT-INT-021) | Replaces the implicit it parameter in a lambda expression with an explicitly named parameter. |
| `ReplaceRepeatWithForLoopIntention` | SHOULD | Converts repeat(n) { ... |
| `ReplaceTypeArgumentWithUnderscoreIntention` | LATER | type-argument underscore (new language feature) |
| `ReplaceUnderscoreWithParameterNameIntention` | LATER | type-argument underscore (new language feature) |
| `ReplaceUnderscoreWithTypeArgumentIntention` | LATER | type-argument underscore (new language feature) |
| `ReplaceWithOrdinaryAssignmentIntention` | SHOULD | Modifies an expression with an augmented assignment to the expression that separates the assignment and the op… |
| `SafeAccessToIfThenIntention` | MUST (KT-INT-047) | Converts an expression that uses a safe-access operator ( ?. |
| `SamConversionToAnonymousObjectIntention` | SHOULD | Converts a SAM conversion to an anonymous object. |
| `SpecifyAllRemainingArgumentsByNameIntention` | SHOULD | Allows specifying all remaining arguments to a function/constructor call by name. |
| `SpecifyExplicitLambdaSignatureIntention` | SHOULD | Explicitly specifies types of parameters, the receiver, and the return type in a lambda expression. |
| `SpecifyRemainingRequiredArgumentsByNameIntention` | SHOULD | Allows specifying remaining required arguments to a function/constructor call by name. |
| `SpecifyTypeExplicitlyInDestructuringAssignmentIntention` | SHOULD | Specifies all types explicitly in a destructuring declaration. |
| `SpecifyTypeExplicitlyIntention` | MUST (KT-INT-011) | Specifies types explicitly for local values, variables, properties, and functions. |
| `SplitIfIntention` | MUST (KT-INT-034) | Converts an if expression containing a conjunction/disjunction operation in its condition into two nested if e… |
| `SplitPropertyDeclarationIntention` | MUST (KT-INT-033) | Splits a property declaration with an initializer and turns the initializer into an assignment. |
| `SwapBinaryExpressionIntention` | MUST (KT-INT-019) | Swaps the operands of a binary expression. |
| `SwapStringEqualsIgnoreCaseIntention` | SHOULD | Replaces string1.equals(string2, ignoreCase) with string2.equals(string1, ignoreCase) . |
| `ToInfixCallIntention` | MUST (KT-INT-038) | Converts a dot-qualified method call to an infix function call. |
| `ToOrdinaryStringLiteralIntention` | SHOULD | Converts a raw string literal (the one surrounded with triple-quote characters) to an ordinary one (surrounded… |
| `ToRawStringLiteralIntention` | MUST (KT-INT-036) | Converts an ordinary string literal to a raw string literal (the one surrounded with triple-quote characters). |
| `TrailingCommaIntention` | LATER | code-style setting |
| `UnfoldAssignmentToIfIntention` | SHOULD | Converts an assignment with an if right-hand side to an if expression where each branch is terminated with an… |
| `UnfoldAssignmentToWhenIntention` | SHOULD | Converts an assignment with a when right-hand side to a when expression where each branch is terminated with a… |
| `UnfoldFunctionCallToIfIntention` | SHOULD | Moves an if expression from a function argument outside the function call. |
| `UnfoldFunctionCallToWhenIntention` | SHOULD | Moves a when expression from a function argument outside the function call. |
| `UnfoldPropertyToIfIntention` | SHOULD | Converts a property with an if initializer to an uninitialized property followed by an if expression where eac… |
| `UnfoldPropertyToWhenIntention` | SHOULD | Converts a property with a when initializer to an uninitialized property followed by a when expression where e… |
| `UnfoldReturnToIfIntention` | SHOULD | Converts return with an if expression as a result to an if expression where each branch is terminated with a s… |
| `UnfoldReturnToWhenIntention` | SHOULD | Converts return with a when expression as a result to a when expression where each branch is terminated with a… |
| `ValToObjectIntention` | SHOULD | Converts a top-level property that is initialized with an object expression to an object declaration. |
| `WhenToIfIntention` | MUST (KT-INT-014) | Converts a when expression to one or more if expressions. |

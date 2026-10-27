# Intentions: kotlin

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Intentions (176)

Description = first sentence of the shipped description resource, if any.

| Class | Language | Category | Description | Spec |
|---|---|---|---|---|
| `AddAnnotationUseSiteTargetIntention` | kotlin | Kotlin | Adds a use-site target to the selected annotation that is applicable to multiple annotation targets. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddBracesIntention` | kotlin | Kotlin | Adds braces to control flow statements. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddBracesToAllBranchesIntention` | kotlin | Kotlin | Adds braces to all branches of if or when . | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddForLoopIndicesIntention` | kotlin | Kotlin | Adds an index variable to a for loop iterating over a collection. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddFullQualifierIntention` | kotlin | Kotlin | Adds a full qualifier prefix to the selected reference. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddJvmOverloadsIntention` | kotlin | Kotlin | Adds a @JvmOverloads annotation to functions and constructors with default arguments. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddJvmStaticIntention` | kotlin | Kotlin | Adds a @JvmStatic annotation to the specified function or property of an object declaration. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddLabeledReturnInLambdaIntention` | kotlin | Kotlin | Adds a labeled return to the last expression in a lambda. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddMissingClassKeywordIntention` | kotlin | Kotlin | Adds a missing class keyword for special class kinds, such as enum . | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddNameToArgumentIntention` | kotlin | Kotlin | Adds a parameter name to the selected argument of a function. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddNamesInCommentToJavaCallArgumentsIntention` | kotlin | Kotlin | Adds names in comments to all arguments of Java method calls or constructor calls. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddNamesToCallArgumentsIntention` | kotlin | Kotlin | Adds names to all positional arguments of a function call according to the named argument syntax. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddNamesToFollowingArgumentsIntention` | kotlin | Kotlin | Adds names to all positional arguments of a function call according to the named argument syntax, starting wit… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddOpenModifierIntention` | kotlin | Kotlin | Adds the open modifier to a non-private method or property in an open class. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddParameterNameAtCallsitesIntention` | kotlin | Kotlin | Adds the parameter name to the given argument at all Kotlin callsites of the function. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddPropertyAccessorsIntention` | kotlin | Kotlin | Adds explicit accessors for a property. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddPropertyGetterIntention` | kotlin | Kotlin | Adds a property getter. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddPropertySetterIntention` | kotlin | Kotlin | Adds a property setter. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddThrowsAnnotationIntention` | kotlin | Kotlin | Adds a @Throws annotation for an exception at the caret. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddUnderscoresToNumericLiteralIntention` | kotlin | Kotlin | Adds underscores to a numeric literal. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddValVarToConstructorParameterActionIntention` | kotlin | Kotlin | Adds a val or var keyword to a primary constructor parameter, making it a property. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddWhenRemainingBranchesIntention` | kotlin | Kotlin | Adds remaining branches for the selected when expression. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AnonymousFunctionToLambdaIntention` | kotlin | Kotlin | Converts an anonymous function passed as an argument to a lambda expression. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ChangePackageIntention` | kotlin | Kotlin | Changes the qualified package name of the current file without moving the file itself. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ChangeVisibilityModifierIntentionChooser` | kotlin | Kotlin | Changes the declaration visibility. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ChopArgumentListIntention` | kotlin | Kotlin | Formats a function call placing each argument on a separate line. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ChopParameterListIntention` | kotlin | Kotlin | Formats parameters of a declaration placing each parameter on a separate line. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertAnyToAllAndViceVersaIntention` | kotlin | Kotlin | Replaces an any function call with an all and vice versa. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertAnyToNoneAndViceVersaIntention` | kotlin | Kotlin | Replaces an any function call with a none and vice versa. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertArrayParameterToVarargIntention` | kotlin | Kotlin | Converts an array parameter to a vararg parameter. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertBinaryExpressionWithDemorgansLawIntention` | kotlin | Kotlin | Converts a boolean expression to an equivalent expression using De Morgan's laws: | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertBlockCommentToLineCommentIntention` | kotlin | Kotlin | Replaces a block comment ( /* ... | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertCallToOppositeIntention` | kotlin | Kotlin | Replaces all/none/filter/filterNot/filterTo/filterNotTo/takeIf/takeUnless function calls with the opposite. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertCamelCaseTestFunctionToSpacedIntention` | kotlin | Kotlin | Changes a camel case name of the given test function to a space-separated one. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertCollectionConstructorToFunction` | kotlin | Kotlin | Reports explicit calls of collection constructors which can be replaced by calls of stdlib builder functions,… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertConcatenationToBuildStringIntention` | kotlin | Kotlin | Converts a statement that builds String using + to one that builds String using a buildString call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertContextParameterToReceiverIntention` | kotlin | Kotlin | Converts a context parameter to receiver. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertContextParameterToRegularParameterIntention` | kotlin | Kotlin | Converts a context parameter to a regular function parameter. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertEnumToSealedClassIntention` | kotlin | Kotlin | Converts an enum class to a sealed class hierarchy turning enum entries into objects. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertForEachToForLoopIntention` | kotlin | Kotlin | Converts a forEach function call into a for loop. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertFunctionToPropertyIntention` | kotlin | Kotlin | Converts a function to a property. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertFunctionTypeParameterToReceiverIntention` | kotlin | Kotlin | Converts a given parameter of function type used as a parameter in another function to receiver. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertFunctionTypeReceiverToParameterIntention` | kotlin | Kotlin | Converts a function type with receiver (e.g., String.() -&gt; Int ) into a function type with the receiver as the… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertLambdaToMultiLineIntention` | kotlin | Kotlin | Converts a single-line lambda to a multi-line one. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertLambdaToReferenceIntention` | kotlin | Kotlin | Converts a lambda (function literal) expression that calls a single function to a function reference. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertLambdaToSingleLineIntention` | kotlin | Kotlin | Converts a multi-line lambda to a single-line one. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertLateinitPropertyToNullableIntention` | kotlin | Kotlin | Converts a lateinit var property or variable to a nullable var property or variable. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertLazyPropertyToOrdinaryIntention` | kotlin | Kotlin | Converts a lazy property or variable to an ordinary property or variable removing the lazy modifier. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertLineCommentToBlockCommentIntention` | kotlin | Kotlin | Replaces one or more end-of-line comments ( // ... | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertMemberToExtensionIntention` | kotlin | Kotlin | Converts a member of a class or an interface into an extension to this class/interface. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertNullablePropertyToLateinitIntention` | kotlin | Kotlin | Converts a nullable var property or variable to a lateinit var &gt; property or variable. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertObjectLiteralToClassIntention` | kotlin | Kotlin | Converts an object literal to a class and replaces the object literal expression with a constructor call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertOptInMarkerToRequirementIntention` | kotlin | Kotlin | Converts an opt-in marker into a propagated requirement. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertOrdinaryPropertyToLazyIntention` | kotlin | Kotlin | Converts an ordinary property or variable to a lazy property or variable. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertParameterToReceiverIntention` | kotlin | Kotlin | Converts a function parameter to an extension receiver. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertPrimaryConstructorToSecondaryIntention` | kotlin | Kotlin | Converts a primary constructor to a secondary one. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertPropertyGetterToInitializerIntention` | kotlin | Kotlin | Converts a property getter to an initializer. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertPropertyInitializerToGetterIntention` | kotlin | Kotlin | Converts a property initializer to a getter. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertPropertyToFunctionIntention` | kotlin | Kotlin | Converts a property to a function. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertReceiverParameterToContextParameterIntention` | kotlin | Kotlin | Converts a function receiver to a context parameter and updates usages. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertReceiverToParameterIntention` | kotlin | Kotlin | Converts the receiver of an extension function to its parameter. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertRegularParameterToContextParameterIntention` | kotlin | Kotlin | Converts a function value parameter to a context parameter and updates usages. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertRequirementToOptInIntention` | kotlin | Kotlin | Converts a propagated requirement into an @OptIn . | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertSealedClassToEnumIntention` | kotlin | Kotlin | Converts a sealed class to an enum class and replaces its derived objects with enum entries. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertSnakeCaseTestFunctionToSpacedIntention` | kotlin | Kotlin | Changes a snake case name of the given test function to a space-separated one. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertStringTemplateToBuildStringIntention` | kotlin | Kotlin | Converts a string template into a buildString call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertToAlsoIntention` | kotlin | Kotlin | Converts several calls with the same receiver to an also call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertToApplyIntention` | kotlin | Kotlin | Converts several calls with the same receiver to an apply call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertToBlockBodyIntention` | kotlin | Kotlin | Converts an expression body (a single expression after the = sign) to a block body with a return statement. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertToConcatenatedStringIntention` | kotlin | Kotlin | Converts a String -building statement that uses a string template to one that uses string concatenation ( + ). | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertToForEachFunctionCallIntention` | kotlin | Kotlin | Converts a for loop to a forEach function call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertToIndexedFunctionCallIntention` | kotlin | Kotlin | Converts a call of a collection function to a call of its *Indexed counterpart. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertToMultiDollarStringIntention` | kotlin | Kotlin | Adds an interpolation prefix ("$$" in most cases) to a string and updates all affected string content entries. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertToRawStringTemplateIntention` | kotlin | Kotlin | Converts a String -building statement that uses concatenation ( + ) to one that uses a raw string. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertToRunIntention` | kotlin | Kotlin | Converts several calls with the same receiver to a run call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertToStringTemplateIntention` | kotlin | Kotlin | Converts a String -building statement that uses concatenation ( + ) to one that uses a string template. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertToWithIntention` | kotlin | Kotlin | Converts several calls with the same receiver to a with call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertTrimIndentToTrimMarginIntention` | kotlin | Kotlin | Converts a trimIndent() call to a trimMargin() call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertTrimMarginToTrimIndentIntention` | kotlin | Kotlin | Converts a trimMargin() call to a trimIndent() call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertUnsafeCastCallToUnsafeCastIntention` | kotlin | Kotlin | Converts an unsafeCast() call to an unsafe cast ( as ). | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertVarargParameterToArrayIntention` | kotlin | Kotlin | Converts a vararg parameter to an array parameter. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertVariableAssignmentToExpressionIntention` | kotlin | Kotlin | Converts the assignment statement to an expression. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `CopyConcatenatedStringToClipboardIntention` | kotlin | Kotlin | Copies the text of a String concatenation to the system clipboard. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `CreateKotlinSubClassIntention` | kotlin | Kotlin | Creates a default implementation for an interface, abstract class, open class, or sealed class. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `EliminateWhenSubjectIntention` | kotlin | Kotlin | Inlines when argument into inner condition entries. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ElvisToIfThenIntention` | kotlin | Kotlin | Converts an elvis expression ?: | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `EvaluateCompileTimeExpressionIntention` | kotlin | Kotlin | Replaces compile-time constant expression with its actual value. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ExpandBooleanExpressionIntention` | kotlin | Kotlin | Replaces the Boolean expression with an equivalent if-else expression. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ExtractDeclarationFromCurrentFileIntention` | kotlin | Kotlin | Extracts a top-level class or object from current file and places it in a separate file in the same package an… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `FlattenWhenIntention` | kotlin | Kotlin | Flattens nested when expression into the enclosing when expression. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `FoldIfToFunctionCallIntention` | kotlin | Kotlin | Lifts a function call out of an if expression. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `FoldIfToReturnAsymmetricallyIntention` | kotlin | Kotlin | Converts a single-branch if expression immediately followed by return into a single return with an if expressi… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `FoldWhenToFunctionCallIntention` | kotlin | Kotlin | Lifts a function call out of a when expression. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `GenerateCopyFunctionForFullValueClassIntention` | kotlin | Kotlin | Adds a copy function to a full value class. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `IfToWhenIntention` | kotlin | Kotlin | Converts an if expression to an equivalent when expression. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ImplementAbstractMemberAsConstructorParameterIntention` | kotlin | Kotlin | Implements an abstract property as a constructor parameter in subclasses. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ImplementAbstractMemberIntention` | kotlin | Kotlin | Implements an abstract member in subclasses. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ImportAllMembersIntention` | kotlin | Kotlin | Adds the import of all members of selected reference and removes explicit usage of the reference in the file. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ImportMemberIntention` | kotlin | Kotlin | Adds the import of selected member of the reference and removes explicit usage of the reference with the membe… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `IndentRawStringIntention` | kotlin | Kotlin | Indents a raw string. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `InsertCurlyBracesToTemplateIntention` | kotlin | Kotlin | Inserts curly braces around expressions in string templates. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `InsertExplicitTypeArgumentsIntention` | kotlin | Kotlin | Inserts explicit type arguments in function calls. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `IntroduceBackingPropertyIntention` | kotlin | Kotlin | Creates a backing property for the specified property. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `IntroduceImportAliasIntention` | kotlin | Kotlin | Creates an import alias for the import. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `IntroduceVariableIntention` | kotlin | Kotlin | Extracts specified expression into a variable. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `InvertIfConditionIntention` | kotlin | Kotlin | Inverts if conditional expression. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `IterateExpressionIntention` | kotlin | Kotlin | Iterates over a given collection with a for loop. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `JoinArgumentListIntention` | kotlin | Kotlin | Formats a function call placing all arguments on a single line. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `JoinParameterListIntention` | kotlin | Kotlin | Formats parameters of a declaration placing all parameters on a single line. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `KotlinCreateTestIntention` | kotlin | Kotlin | Generates a test case for the specified class. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `LambdaToAnonymousFunctionIntention` | kotlin | Kotlin | Converts a lambda expression into a call of an anonymous function. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MavenPluginSourcesMoveToBuild` | XML | Kotlin | Moves the source directory from the execution's configuration to build the corresponding source directory tag. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MavenPluginSourcesMoveToExecutionIntention` | XML | Kotlin | Moves a source directory from build source directory tag to the corresponding kotlin-maven-plugin execution's… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MergeElseIfIntention` | kotlin | Kotlin | Merges the else branch of an if...else statement if this else branch only contains a nested if...else statemen… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MergeIfsIntention` | kotlin | Kotlin | Merges two nested if statements without else branches into a single if . | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MoveFunctionToClassIntention` | kotlin | Kotlin | Moves the function to a class. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MoveLambdaInsideParenthesesIntention` | kotlin | Kotlin | Moves a lambda expression inside parentheses. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MoveMemberOutOfCompanionObjectIntention` | kotlin | Kotlin | Moves a member of a companion object to the corresponding class. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MoveMemberToCompanionBlockIntention` | kotlin | Kotlin | Moves a class member to the corresponding companion block. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MoveMemberToCompanionExtensionIntention` | kotlin | Kotlin | Moves a class member to the corresponding companion block. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MoveMemberToCompanionObjectIntention` | kotlin | Kotlin | Moves a class member to the corresponding companion object. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MoveMemberToTopLevelIntention` | kotlin | Kotlin | Moves object members to the top level of the file. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MovePropertyToClassBodyIntention` | kotlin | Kotlin | Moves a property from the primary constructor to the class body. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MovePropertyToConstructorIntention` | kotlin | Kotlin | Moves the specified property to the primary constructor. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `NullableBooleanEqualityCheckToElvisIntention` | kotlin | Kotlin | Converts a nullable boolean equality check to an Elvis operator. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `OperatorToFunctionIntention` | kotlin | Kotlin | Replaces an overloaded operator with a function call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `PutCallsOnSeparateLinesIntention` | kotlin | Kotlin | Formats call chain by placing each call on a separate line. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `PutExpressionsOnSeparateLinesIntention` | kotlin | Kotlin | Formats expressions by placing each condition on a separate line. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReconstructTypeInCastOrIsIntention` | kotlin | Kotlin | Replaces a shortened type on the right-hand side of as / as? | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `RemoveAllArgumentNamesIntention` | kotlin | Kotlin | Removes names from named arguments where possible, reordering arguments to match parameter order. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `RemoveBracesFromAllBranchesIntention` | kotlin | Kotlin | Removes braces from all branches of if/when . | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `RemoveBracesIntention` | kotlin | Kotlin | Removes braces from control flow statements with only a single statement in their code block. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `RemoveExplicitLambdaParameterTypesIntention` | kotlin | Kotlin | Removes explicit parameter types in a lambda expression. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `RemoveExplicitTypeIntention` | kotlin | Kotlin | Removes an explicit type specification for local values, variables, properties, and functions. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `RemoveForLoopIndicesIntention` | kotlin | Kotlin | Reports for loops iterating over a collection using the withIndex() function and not using the index variable. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `RemoveSingleArgumentNameIntention` | kotlin | Kotlin | Removes the name from a named argument to use positional argument syntax. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `RemoveUnderscoresFromNumericLiteralIntention` | kotlin | Kotlin | Removes underscores from a numeric literal. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `RenameClassToContainingFileNameIntention` | kotlin | Kotlin | Changes the name of a class to the name of the containing file. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `RenameFileToMatchClassIntention` | kotlin | Kotlin | Changes the name of a file to the name of the top-level class. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReplaceAddWithPlusAssignIntention` | kotlin | Kotlin | Replaces calls to add() and addAll() in a mutable collection with a += operator. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReplaceCallWithUnaryOperatorIntention` | kotlin | Kotlin | Replaces prefix member calls ( plus , minus , not ) with their respective prefix operators. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReplaceExplicitFunctionLiteralParamWithItIntention` | kotlin | Kotlin | Replaces the explicit parameter of a single-parameter lambda expression with an implicit it parameter. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReplaceForEachWithRepeatIntention` | kotlin | Kotlin | Converts forEach over a zero-based range to an equivalent repeat(n) { ... | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReplaceForLoopWithRepeatIntention` | kotlin | Kotlin | Converts for loop over a range starting from 0 to an equivalent repeat(n) { ... | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReplaceInvokeIntention` | kotlin | Kotlin | Replaces invoke member calls with a receiver call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReplaceItWithExplicitFunctionLiteralParamIntention` | kotlin | Kotlin | Replaces the implicit it parameter in a lambda expression with an explicitly named parameter. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReplaceRepeatWithForLoopIntention` | kotlin | Kotlin | Converts repeat(n) { ... | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReplaceTypeArgumentWithUnderscoreIntention` | kotlin | Kotlin | Replaces the explicit type with an implicit _ in a type argument if possible. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReplaceUnderscoreWithParameterNameIntention` | kotlin | Kotlin | Gives a name to an anonymous parameter. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReplaceUnderscoreWithTypeArgumentIntention` | kotlin | Kotlin | Replaces the implicit _ with an explicit type in a type argument. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReplaceWithOrdinaryAssignmentIntention` | kotlin | Kotlin | Modifies an expression with an augmented assignment to the expression that separates the assignment and the op… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `SafeAccessToIfThenIntention` | kotlin | Kotlin | Converts an expression that uses a safe-access operator ( ?. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `SamConversionToAnonymousObjectIntention` | kotlin | Kotlin | Converts a SAM conversion to an anonymous object. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `SpecifyAllRemainingArgumentsByNameIntention` | kotlin | Kotlin | Allows specifying all remaining arguments to a function/constructor call by name. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `SpecifyExplicitLambdaSignatureIntention` | kotlin | Kotlin | Explicitly specifies types of parameters, the receiver, and the return type in a lambda expression. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `SpecifyRemainingRequiredArgumentsByNameIntention` | kotlin | Kotlin | Allows specifying remaining required arguments to a function/constructor call by name. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `SpecifyTypeExplicitlyInDestructuringAssignmentIntention` | kotlin | Kotlin | Specifies all types explicitly in a destructuring declaration. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `SpecifyTypeExplicitlyIntention` | kotlin | Kotlin | Specifies types explicitly for local values, variables, properties, and functions. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `SplitIfIntention` | kotlin | Kotlin | Converts an if expression containing a conjunction/disjunction operation in its condition into two nested if e… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `SplitPropertyDeclarationIntention` | kotlin | Kotlin | Splits a property declaration with an initializer and turns the initializer into an assignment. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `SwapBinaryExpressionIntention` | kotlin | Kotlin | Swaps the operands of a binary expression. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `SwapStringEqualsIgnoreCaseIntention` | kotlin | Kotlin | Replaces string1.equals(string2, ignoreCase) with string2.equals(string1, ignoreCase) . | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ToInfixCallIntention` | kotlin | Kotlin | Converts a dot-qualified method call to an infix function call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ToOrdinaryStringLiteralIntention` | kotlin | Kotlin | Converts a raw string literal (the one surrounded with triple-quote characters) to an ordinary one (surrounded… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ToRawStringLiteralIntention` | kotlin | Kotlin | Converts an ordinary string literal to a raw string literal (the one surrounded with triple-quote characters). | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `TrailingCommaIntention` | kotlin | Kotlin | Changes the Use trailing comma setting in Settings \| Editor \| Code Style \| Kotlin \| Other . | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `UnfoldAssignmentToIfIntention` | kotlin | Kotlin | Converts an assignment with an if right-hand side to an if expression where each branch is terminated with an… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `UnfoldAssignmentToWhenIntention` | kotlin | Kotlin | Converts an assignment with a when right-hand side to a when expression where each branch is terminated with a… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `UnfoldFunctionCallToIfIntention` | kotlin | Kotlin | Moves an if expression from a function argument outside the function call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `UnfoldFunctionCallToWhenIntention` | kotlin | Kotlin | Moves a when expression from a function argument outside the function call. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `UnfoldPropertyToIfIntention` | kotlin | Kotlin | Converts a property with an if initializer to an uninitialized property followed by an if expression where eac… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `UnfoldPropertyToWhenIntention` | kotlin | Kotlin | Converts a property with a when initializer to an uninitialized property followed by a when expression where e… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `UnfoldReturnToIfIntention` | kotlin | Kotlin | Converts return with an if expression as a result to an if expression where each branch is terminated with a s… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `UnfoldReturnToWhenIntention` | kotlin | Kotlin | Converts return with a when expression as a result to a when expression where each branch is terminated with a… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ValToObjectIntention` | kotlin | Kotlin | Converts a top-level property that is initialized with an object expression to an object declaration. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `WhenToIfIntention` | kotlin | Kotlin | Converts a when expression to one or more if expressions. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |

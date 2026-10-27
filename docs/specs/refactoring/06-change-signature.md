# 06. Change Signature

Status: specification. Requirement prefix `REF-SIG`. Java methods, constructors, records and
generic classes in full depth; Kotlin where its plugin differs. Framework rules are in
`01-framework.md`. Executable cases: `cases/06-change-signature.cases.md`.

ic: `platform/lang-impl/src/com/intellij/refactoring/changeSignature/ChangeSignatureDialogBase.java`,
`java/java-impl/src/com/intellij/refactoring/changeSignature/JavaChangeSignatureDialog.java`,
`java/java-impl-refactorings/src/com/intellij/refactoring/changeSignature/**`,
`java/java-impl-refactorings/src/com/intellij/refactoring/changeClassSignature/**`,
`platform/refactoring/resources/messages/RefactoringBundle.properties`,
`java/openapi/resources/messages/JavaRefactoringBundle.properties`,
`plugins/kotlin/base/resources/resources-en/messages/KotlinBundle.properties`,
`platform/lang-api/src/com/intellij/refactoring/suggested/**`.

## 1. Invocation

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| REF-SIG-001 | MUST | `Change Signature...` (`Ctrl+F6`) finds its target from the caret: inside a method's parameter list; on a method or constructor name; on a call (the resolved method, also through `new`, `this(...)`, `super(...)`, method references); in a record header (the canonical constructor); in a type-parameter list or type arguments (the class or method declaring them); on a class name that has type parameters (then Change Class Signature, REF-SIG-040). Annotation types and enums by name are not targets. Errors: `Cannot perform refactoring.\nCaret should be positioned at the name of method or class to be refactored`; implicit methods: `Cannot change the signature of implicit method '{0}()'`; a class without type parameters: `Class cannot have type parameters` (error hint for Change Class Signature). | Case REF-SIG-001 case 1,2. |
| REF-SIG-002 | MUST | A method that overrides or implements others first asks as in rename (REF-REN-061, texts with `to refactor`): dialog `Super Method Found` with `Base Method`/`Current Method`/`Cancel` (text `Do you want to refactor the base method instead?`), the in-place equivalent is a popup. Choosing the base method opens the dialog for the deepest base. Library methods and read-only files: `{0} is read-only` style errors (REF-FRM-012). | Case REF-SIG-002 case 1. |
| REF-SIG-003 | SHOULD | In-place (suggested refactoring) mode: editing a parameter list directly shows a gutter icon / intention `Update {usages|callers|overrides|implementations} to reflect signature change` (REF-FRM-061), with the full dialog available from the popup. | |

## 2. The dialog (Java)

Title `Change Signature`. Top to bottom:

1. `Name:` text field (method name; hidden for constructors), `Return type:` code field with
   completion and import support (hidden for constructors), visibility selector (radio buttons
   `public`, `protected`, `package local`, `private`) at the right of the parameter table.
2. `Parameters` table (tab/group title `Parameters`) with columns `Type`, `Name`, `Default Value`
   and a hidden-by-default `Any Var` flag column (shown as the checkbox `Use Any Var`); a
   toolbar with add (`Alt+Insert`), remove (`Alt+Delete`), move up, move down (`Alt+Up`/`Alt+Down`);
   cells are code editors with completion; selecting a row shows a detail editor with `Type:`, `Name:`
   and below it `Default value:` with the checkbox `Use Any Var`. The `Default Value` column is
   enabled only for *new* parameters. A varargs flag is expressed by writing `Type...`.
3. The `Exceptions` table (border title `Exceptions`) with a `Type` column
   plus a button `Propagate Exceptions...` (mnemonic x); `Propagate Parameters...` (`Alt+G`)
   for the parameter table.
4. When delegation is allowed (not for interface methods without default, not for the canonical
   record constructor): a panel `Method calls:` with radios `Modify` (default) and
   `Delegate via overloading method`.
5. `Signature Preview` text area showing the declaration (modifiers, return type, name, parameters,
   `throws`) updated live.
6. Buttons `Refactor` (default), `Preview`, `Cancel`.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| REF-SIG-010 | MUST | Initial state: the current visibility, name, return type, parameters (with their types and names), thrown exceptions. A newly added row has empty type and name and an empty default value. Edits update the signature preview immediately; the first column in a new row is focused. | Case REF-SIG-010 case 1. |
| REF-SIG-011 | MUST | Validation on `Refactor` (error dialog titled `Change Signature` unless noted): `Return type is not specified`, `Wrong return type: '{0}'`, `Specify {0} type for parameter '{1}'` ({0} is the word `parameter` or `return`), `Wrong type: '{0}' for parameter '{1}'`, `Vararg parameter should be the last in method signature`, `Wrong name: {0}` (invalid identifier), `Specify type for exception`, `Wrong type: '{0}' for exception`, `Wrong type '{0}' for exception, should extend java.lang.Throwable`, `Implicit last parameter should not be deleted`. Warning confirmations (Yes/No, title `Change Signature`): `Return type '{0}' cannot be resolved.\nContinue?`, `Type '{0}' for parameter '{1}' cannot be resolved.\nContinue?`. Inline validation text for a new parameter without default value while callers exist: `Default value is missing. Method calls will contain blanks instead of the new parameter value.` | Case REF-SIG-011 case 1-3. |
| REF-SIG-012 | MUST | Default value of a new parameter (the text written in `Default Value`) is inserted as the argument at every call site (parsed in the context of the call; qualified names are imported/shortened). Empty default value: a chooser dialog opens, titled `Default value for parameter "{name}" needed`, with the radio choices `Leave blank` (call sites get an empty argument slot to be filled in by the user), `Feel lucky` (`In calls to this method, if exactly one variable of the correct type is found, use it. Otherwise leave blank.`) and `Use selected value`. `Use Any Var` in the table is the same as `Feel lucky` for that parameter. | Case REF-SIG-012 case 1,2. |
| REF-SIG-013 | MUST | Result on the declaration: name, return type, visibility, parameter list (order, names, types, `final` modifiers kept, annotations kept), `throws` list are rewritten; the body is updated: references to renamed parameters follow; removed parameters that are still used in the body are a conflict (REF-SIG-020); the `@Override` annotation is kept; javadoc `@param`/`@throws`/`@return` tags follow (renamed, removed, added tags for new parameters with empty description). | Cases REF-SIG-013 case 1-4. |
| REF-SIG-014 | MUST | Result on callers: arguments are reordered, removed, added (default value); a call with a varargs argument list is rewritten correctly when the varargs position changes; an overriding or implementing method in the project is changed in the same way (parameter *names* of overriders follow the new names unless they differ); a method reference `A::f` is kept when the new signature still matches the functional interface, otherwise converted to a lambda with the warning `Method reference will be converted to a lambda expression` (conflict text `Method is used in one or more method references. Proceeding will convert these method references to lambda expressions.`); a lambda that implements a changed functional interface method gets its parameter list changed; javadoc references `{@link A#f(int)}` follow. The new return type adds casts at call sites when needed. | Cases REF-SIG-014 case 1-4. |
| REF-SIG-015 | MUST | `Delegate via overloading method`: the original signature is kept as an overload that calls the new method (`{returnType} f(oldParams) { [return] f(newArgs); }` with the default values for the new parameters), and the call sites are **not** modified. | Case REF-SIG-015 case 1. |
| REF-SIG-016 | MUST | `Propagate Parameters...`/`Propagate Exceptions...`: a chooser dialog titled `Select Methods To Propagate New Parameters` / `Select Methods To Propagate New Exceptions` shows a tree of caller methods (columns `Caller Method` and, in the preview pane, the caller text with the call highlighted: placeholders `Caller method text with highlighted callee call would be shown here` / `Callee method text would be shown here`; progress `Looking for Callers...`); selecting callers adds the same new parameter (or exception) to those callers and passes it through (recursively for selected callers); recursion: `Recursive propagation of parameter changes won't be performed` / `Recursive propagation of exception changes won't be performed`. | Case REF-SIG-016 case 1. |
| REF-SIG-017 | MUST | Command name `Changing signature of {method description}`; the usage view header for preview `References to be changed {n}` / `{0} to change signature`; conflict-checking progress `Validating...`, `Processing changes...`. | Undo list text. |
| REF-SIG-018 | MUST | Dumb mode and read-only: as REF-FRM-012; a method in a library: refused. | |

## 3. Conflicts (Java)

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| REF-SIG-020 | MUST | Conflicts reported in the framework dialog: removing a parameter that is still read in the body: `{0} is used in method body` ( `Record component '{0}' is used` for records); a new parameter whose name collides with a local variable or field used in the body: `There is already a {0} in the {1}. It will conflict with the new parameter.`; renaming a parameter onto an existing local: `There is already a {0} in the {1}. It will conflict with the renamed parameter.`; the new signature clashes with an existing method: `Method {0} is already defined in the {1}`; overriding methods that would no longer override: `{0} will no longer override {1}`; `{0} will become inaccessible from {1}` (visibility reduction); `Package-local class {0} will no longer be accessible from {1}`; a call passing `null` for a removed parameter: `{0} contains call with null argument for parameter {1}`; a record accessor: `{0} will no longer be {1} getter`; `Constructor will no longer be canonical`; `{0} already has a parameter named '{1}'.\nUse this name anyway?`; `Contract` annotation problems (`@Contract annotation cannot be updated automatically: {0}`, with details `Annotation contains mutation contract`, `Error in contract definition: {0}`, `Invalid contract clause '{0}'`, `Parameter '{0}' was deleted, but contract clause '{1}' depends on it`, `Parameter '{0}' was deleted, but contract clause '{1}' returns it`, `Invalid reference in return value: {0}`, `Annotation is inherited from base method`, `Automatic update of external annotation is not supported`). | Cases REF-SIG-020 case 1,2. |
| REF-SIG-021 | MUST | Deleting a parameter in a hierarchy: `{0} is a part of method hierarchy. Do you want to delete multiple parameters?` (Yes/No/Cancel) in Safe Delete; for Change Signature the hierarchy is always changed together. | |

## 4. Class signature (type parameters) and constructors

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| REF-SIG-040 | SHOULD | Change Class Signature dialog (`Change Class Signature`): label `Change signature of <code>{class}</code>.`, table `Parameters:` with columns for name, bound/`Extends` and default value for new type parameters (`Wrong {0} value: '{1}' for parameter '{2}'` on invalid input, `'{0}' already contains type parameter '{1}'`, `Type parameter can't be primitive`); usages of the class (`Foo<A>`) get the default type arguments. | LATER detail. |
| REF-SIG-041 | MUST | Constructors: Change Signature on a constructor updates `new X(...)`, `this(...)`, `super(...)` calls of subclasses and enum constants with arguments; the implicit default constructor can be targeted when the class has none (`Class {0} does not have implicit default constructor` for the opposite error). An anonymous class creation passes through. | Case REF-SIG-041 case 1. |
| REF-SIG-042 | MUST | Records: Change Signature on the record header changes the canonical constructor and the components: adding a component adds the accessor, removing one removes it; callers of `new R(..)` follow; deconstruction patterns `R(int a, int b)` are updated. | Case REF-SIG-042 case 1. |

## 5. Kotlin

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| REF-SIG-050 | MUST | Kotlin Change Signature dialog: `Name:`, `Return type:`, visibility, receiver type (`Receiver:` column in the parameter table), parameter table with column names `Name`, `Type:`, `Default Value`, `Default Parameter:`, `Val/Var` (for constructor properties), `Receiver:`, `Context Parameter:`; checkbox `Introduce default value` (adds the default to the *declaration* and leaves call sites), `Use Any Var`, `Leave blank`/`Feel lucky` default-value choices as for Java. Callers use named arguments when arguments are reordered past positional ones; removed parameters' arguments are removed; default-valued arguments are dropped when equal to the new default; lambda arguments outside parentheses are kept outside. | Case REF-SIG-050 case 1-3. |
| REF-SIG-051 | MUST | Kotlin conflicts and errors: `Can't change signature of {0} method`, `Change signature is not applicable to dynamically invoked functions`, `Can't refactor the function with variable arguments`, `Library declarations cannot be changed`, `Cannot modify '{0}' declaration from '{1}' file`, `The default Kotlin call value may lead to incorrect code in other languages`, `The default parameter may not be supported in other languages`, `A function in a companion block cannot have a receiver`, `A companion extension must have a receiver`, `There is already a parameter '{0}' in {1}. It will conflict with the new parameter.`, `There is already a variable '{0}' in {1}. It will conflict with the new parameter.`, callable-reference conflicts (`Callable reference '{0}' will no longer match the expected function type`, `... cannot be updated to a member with an extension receiver`, `... cannot pass an explicit argument to context parameter '{1}'`, `... cannot be updated automatically because parameters were converted to/from context parameters`, `... cannot pass its receiver value as a context argument`, `... cannot be updated: its qualifier cannot also supply the new receiver`, `... its receiver value cannot be turned into an argument`, `... its context argument cannot be used as an explicit receiver`). Overrides: `{declaration} overrides declarations in the following classes/interfaces: {1} Do you want {2} the base declarations?`. | Case REF-SIG-051 case 1. |
| REF-SIG-052 | SHOULD | Quick-fix style signature changes (`Change the signature of function '{0}'`, `Change function signature to '{0}'`, `Choose Signature` popup, `Remove parameter '{0}'`) reuse the same engine without a dialog; see the intention specs. | |

## 6. Open questions

1. Whether the `Extends` column shown for the Exceptions table exists; the table has the `Type`
   column; the bound column `Extends` is used for type parameters in Change Class Signature.
2. The accelerator keys of the parameter table toolbar (taken from the shared toolbar decorator).
3. Default of `Delegate via overloading method` is off (Modify selected) as in the source's
   initial state.

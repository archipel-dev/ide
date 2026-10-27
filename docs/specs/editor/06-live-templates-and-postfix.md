# Editor spec 06: live templates, surround with, postfix completion

Status: specification, language independent except the Java catalog (section 7). Requirement prefixes
`ED-TMPL` (live templates and surround) and `ED-POST` (postfix templates).
Priorities: `MUST` / `SHOULD` / `LATER`.

Live templates are abbreviations that expand into code with fill-in fields. This file specifies
the template text syntax, variables and predefined functions, how an abbreviation is found and
expanded (Tab by default), the editing session (moving between fields, finishing, cancelling),
Insert Live Template, Surround With, Surround with Live Template, postfix completion, the settings
pages and their defaults, the built-in Java catalog, and the undo behavior.

Executable acceptance cases are in `cases/06-live-templates-and-postfix.cases.md` (format
`cases/README.md`). In the cases, `template: <abbreviation> = <text>` defines a user template that
exists before the keys are pressed (context: everywhere); an `active field` after expansion is the
selection or the empty caret shown between a pair `<field>` markers only when the case needs to
show it. Cases are normative.

ic: `platform/analysis-impl/src/com/intellij/codeInsight/template/impl/{TemplateImpl,TemplateState,
TemplateStateProcessor}.java`, `platform/lang-impl/src/com/intellij/codeInsight/template/impl/
{TemplateManagerImpl,TemplateSettings,TemplateImplUtil,ListTemplatesHandler,InvokeTemplateAction,
SurroundWithTemplateHandler,LiveTemplateCompletionContributor}.java`,
`platform/lang-impl/src/com/intellij/codeInsight/template/macro/*`,
`platform/lang-impl/src/com/intellij/codeInsight/template/postfix/{templates,settings,completion}/*`,
`java/java-impl/resources/liveTemplates/Java.xml`,
`java/java-impl/src/com/intellij/codeInsight/template/{macro,postfix/templates}/*`,
`java/java-impl/src/com/intellij/codeInsight/generation/surroundWith/*`,
`platform/lang-api/resources/messages/CodeInsightBundle.properties`,
`platform/platform-resources-en/src/messages/ActionsBundle.properties`,
`platform/platform-resources/src/keymaps/$default.xml`.

## 1. Template text and variables

A template has: an **abbreviation** (key), a **description**, a **group** (for example `Java`), the
**template text**, a list of **variables**, **options**, and a set of **contexts** in which it applies.

| Element | Contract |
|---|---|
| Text | Plain text containing **variable references** `$NAME$`. A literal dollar sign is written `$$`. Line breaks in the text are `\n`. |
| Predefined names | `$END$`: where the caret goes when the template finishes (at most one is used; none means the end of the inserted text). `$SELECTION$`: the surrounded text (empty and ignored when the template is expanded by abbreviation, replaced by the selected text when used for Surround With). `$SELECTION_START$` and `$SELECTION_END$` mark a selection to be set after finishing. A reference to the same variable name several times creates **mirrors** that always show the same value. |
| Variable | **Name**; **Expression** (a *macro call*, section 3, evaluated to propose or compute the value); **Default value** (a text or macro call used when the expression gives nothing); **Skip if defined** (do not stop at the variable when its expression already gives a value; the stored flag `alwaysStopAt` is the inverse: *Skip if defined* off means always stop). |
| Options | `Reformat according to style` and `Shorten FQ names` (default **on** for the new templates from the dialog, per template for built-in ones); `Use static import if possible`; `Expand with` (`Default (Tab)`, `Tab`, `Enter`, `Space`, `Custom`, `None`). |
| Contexts | A set of context types (for example Java: statement, expression, declaration, comment, string; XML; Other). A template expands only where at least one of its contexts holds at the caret. |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TMPL-001 | MUST | Template text is parsed into literal parts and variable segments; `$$` is a literal `$`; a `$` that does not form `$IDENT$` is literal text. Variables referenced in the text but not defined get an empty expression (the user types the value). Unused variable definitions are ignored. | Cases ED-TMPL-001. |
| ED-TMPL-002 | MUST | Mirrors: all segments of one variable name show the same text; editing any of them updates the others live while the session runs, and values computed from an expression are recomputed after each change of a variable they depend on (a variable defined by an expression naming another variable). | Case. |
| ED-TMPL-003 | MUST | The user template store keeps, per template: abbreviation, description, group, text, variables (name, expression, default, skip-if-defined), options, contexts, `Expand with`. Templates are organized in groups and a group can be disabled as a whole; built-in templates are restorable. Templates and groups are application level (shared across projects) and can be exported and imported per group. | Reopen the IDE: a user template persists. |

## 2. Expansion by abbreviation

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TMPL-010 | MUST | Default **expand key: Tab** (action `Expand Live Template by Tab`; setting `Expand with` of the whole IDE: Tab, Enter or Space, default **Tab**). A template can override it per template. The expansion is tried when the key is pressed; when no template matches, the key does its normal job (Tab inserts indent, `ED-TYPE-080`). The Tab action is lower priority than tab-out of a bracket pair (`ED-TYPE-030`) and than an open lookup, but higher than inserting a tab. | Case with Tab. |
| ED-TMPL-011 | MUST | **Matching**: take the caret offset; try key lengths from the longest abbreviation in the store downward to 1; for each length the text before the caret of that length is the candidate key; if the key starts with an identifier-start character, the character before the key must not be an identifier part (word boundary); the first length that yields templates wins (so the longest abbreviation wins). Candidates are filtered by the expand-key setting and the contexts at the caret. Exactly one candidate expands; several candidates show a lookup popup (templates named by abbreviation and description) from which Enter or Tab expands. | `xab` with a template `ab`: no expansion; `x ab`: expansion. |
| ED-TMPL-012 | MUST | A template whose text contains the variable `$ARG$` takes an **argument**: the word before the abbreviation (separated by one blank or directly attached, as the key allows) is consumed and substituted for `$ARG$`. | Template `x` = `f($ARG$)`: `a x` Tab gives `f(a)`. |
| ED-TMPL-013 | MUST | Expansion replaces the typed abbreviation by the template text as one command (undo: `ED-UNDO-008`), computes variable expressions, then if `Reformat according to style` is on reformats the inserted range (indentation of continuation lines follows the code style; the first line keeps the caret's line indent), and if `Shorten FQ names` is on shortens qualified names to imports (language support). | `sout` Tab in Java gives `System.out.println();` with the caret between the parentheses. |
| ED-TMPL-014 | MUST | **Contexts** limit where an abbreviation expands (Java statement, expression, declaration, class level, comment, string...); `Other` means a file type with no specific context. The context is decided at the abbreviation start offset. A template with no context is never offered. | Java `sout` inside a comment: Tab is a normal Tab. |
| ED-TMPL-015 | MUST | **Insert Live Template** (Ctrl+J, `Insert Live Template...`): shows a popup of all templates applicable in the current context, filtered by the abbreviation prefix before the caret, each shown as `abbreviation  description`; choosing one expands it, replacing the prefix. A selection is used for `$SELECTION$` when present. | Ctrl+J lists `sout`. |
| ED-TMPL-016 | SHOULD | Applicable templates are also offered in basic completion (items with a template icon) with the abbreviation as lookup string, unless `Show live templates in completion` is off; choosing one expands it. | Type `sou` in Java: `sout` is listed. |
| ED-TMPL-017 | MUST | Template expansion can be invoked with the **custom** shortcut for templates whose `Expand with` is `Custom`; a template with `None` is reachable only through Insert Live Template. | Case. |

## 3. Variable expressions (predefined functions)

An expression is a call `name(arg, ...)`, where arguments are string literals in double quotes,
other variable names or nested calls. The functions:

| Group | Functions (exact names) |
|---|---|
| Text | `capitalize(String)`, `decapitalize(String)`, `capitalizeAndUnderscore(String)`, `camelCase(String)`, `snakeCase(String)`, `lowercaseAndDash(String)`, `spacesToUnderscores(String)`, `underscoresToSpaces(String)`, `underscoresToCamelCase(String)`, `firstWord(String)`, `escapeString(String)`, `concat(expressions...)`, `substringBefore(String, Delimiter)`, `regularExpression(String, Pattern, Replacement)` |
| Context | `date()`, `time()`, `user()`, `clipboard()`, `lineNumber()`, `lineCommentStart()`, `blockCommentStart()`, `blockCommentEnd()`, file path functions (names not confirmed from the sources read), `enum(...)` (a fixed list of choices), `complete()`, `completeSmart()`, `showParameterInfo()` |
| Java: names | `className()`, `qualifiedClassName()`, `currentPackage()`, `methodName()`, `methodParameters()`, `methodParameterTypes()`, `methodReturnType()`, `parameterInfo()`, `suggestVariableName()`, `suggestFirstVariableName(type)`, `suggestIndexName()` |
| Java: types and variables | `variableOfType(type)`, `arrayVariable()`, `iterableVariable()`, `typeOfVariable(var)`, `expectedType()`, `expressionType(expr)`, `rightSideType()`, `castToLeftSideType()`, `componentTypeOf(type)`, `iterableComponentType(expr)`, `guessElementType(expr)`, `typeParameterOf(var, index or name)`, `subtypes(type)`, `descendantClassesEnum(base)`, `annotated(...)`, `annotation(...)` |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TMPL-020 | MUST | A variable's **value** is computed in this order: a value the user typed; else the expression's result; else the default value; else empty. The expression is recomputed when the variables it names change. A function that needs semantic information (resolve) is evaluated after the file is committed and returns no result in dumb mode when it needs indexes (the variable is then a plain stop). | `$D$` with `date()`: the current date appears. |
| ED-TMPL-021 | MUST | A function may return a single result or a **list of lookup items**; when the variable is the current field and there are items, a completion-style popup shows them (selecting an item with Enter inserts it and goes to the next field when the item ends the field). | `iter` in Java lists collection variables in scope. |
| ED-TMPL-022 | MUST | Text functions are pure string functions: `capitalize("fooBar")` gives `FooBar`, `decapitalize` the inverse, `snakeCase("fooBar")` gives `foo_bar`, `lowercaseAndDash("fooBar")` gives `foo-bar`, `camelCase("foo_bar")` gives `fooBar`, `capitalizeAndUnderscore("fooBar")` gives `FOO_BAR`, `firstWord("foo bar")` gives `foo`, `spacesToUnderscores("a b")` gives `a_b`, `underscoresToSpaces("a_b")` gives `a b`, `escapeString` escapes for a string literal (`"` becomes `\"`, newline `\n`). | Unit cases of the pure functions. |
| ED-TMPL-023 | SHOULD | `clipboard()` returns the text of the current clipboard, `date()` and `time()` the local date and time in the locale's short format, `user()` the OS user name, `lineNumber()` the current line number (starting at 1). | Cases. |

## 4. The editing session

While a template is active, its fields are highlighted (boxed range of the current field, lighter
boxes for the other fields that may stop), and the following keys apply.

| Key | Action | Behavior |
|---|---|---|
| Tab, Enter | `NextTemplateVariable` (`Next Template Variable or Finish In-Place Refactoring`) | go to the next stop; after the last stop finish the template |
| Shift+Tab | `PreviousTemplateVariable` | go to the previous stop |
| Escape | `EditorEscape` | exit the session keeping the text as is, caret where it is |
| Alt+Shift+O | `SelectVirtualTemplateElement` | show the options popup of an in-place refactoring (not for plain templates) |
| any text | | typing in the current field replaces its default text (the field is selected when entered) and updates mirrors |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TMPL-030 | MUST | **Stops**: a variable is a tab stop when its definition has *Skip if defined* off, or its expression gives no result, or it offers more than one lookup item. Variables that already have a result and are not marked always-stop are filled in and skipped. Stops are visited in the order of the variable definitions, not of the text positions. The `$END$` position is the final stop. | `fori`: stops at `INDEX`, then `LIMIT`, then the end. |
| ED-TMPL-031 | MUST | Entering a field selects its current text (if non-empty) with the caret at its end; an empty field shows the caret. Moving on first **recalculates** all dependent values, reformats the template range, and then selects the next stop. After the last stop the caret goes to `$END$` (or to the end of the inserted text if there is none), the selection is removed and the template finishes. A selection template (used by Surround With) does not move the caret to the end when it has no `$END$` and sets the selection from `$SELECTION_START$` and `$SELECTION_END$` when present. | Cases. |
| ED-TMPL-032 | MUST | **Previous** moves back, recalculating first; at the first stop Shift+Tab does nothing (the session stays). | Shift+Tab on the first field: no change. |
| ED-TMPL-033 | MUST | **Breaking out**: editing text outside the template range, adding a caret, removing a caret, or any command that moves the caret outside the template range ends the session without further processing (the text stays). Escape ends it the same way. Undo during a session ends the session and then reverts (`ED-UNDO-041`). | Click outside the template: session ends. |
| ED-TMPL-034 | MUST | A session works with multiple carets: each caret gets its own expansion (interactive stops only for the first caret; the others mirror it), but stops are not navigable per caret. | Two carets expand `sout` independently. |
| ED-TMPL-035 | MUST | While a lookup is open in a field, Enter or Tab first chooses the lookup item; after the item is inserted the session advances to the next stop when the caret is at the end of the field (or inside or just before the next field), and finishes when the caret is beyond the field. | `iter`: Enter on a lookup item inserts it and moves on. |
| ED-TMPL-036 | SHOULD | Only one template session per editor: starting an expansion while one runs finishes the running one first (as if Escape). Nested expansion inside a field is allowed when the field is selected and the user types an abbreviation + Tab. | Case. |

## 5. Surround With

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TMPL-040 | MUST | **Surround With** (`Ctrl+Alt+T`, `Surround With...`): enabled when there is a selection (or, for languages that allow it, the caret in a surroundable element). Shows a chooser titled `Surround With` that lists the *surrounders* the language offers for the selected fragment (each with a one-line template description), numbered by mnemonics (digits then letters), followed by the live templates that contain `$SELECTION$` for the current context; choosing one applies it as one command. Surrounders are tried per kind of selection: statements, an expression, or text (languages differ). | Java: selecting two statements lists `if`, `if / else`, `while`, `do / while`, `for`, `try / catch`, `try / finally`, `try / catch / finally`, `synchronized`, `Runnable`, `{ }`. |
| ED-TMPL-041 | MUST | **Surround with Live Template** (`Ctrl+Alt+J`): shows only the live templates with `$SELECTION$` applicable at the selection; choosing one expands it with the selected text substituted and the selection removed (the originally selected text appears at `$SELECTION$` inside the template, indented as the template text). | `par` = `($SELECTION$)`: selecting `abc` gives `(abc)`. |
| ED-TMPL-042 | MUST | Java surrounder chooser texts: for statements `if`, `if / else`, `while`, `do / while`, `for`, `try / catch`, `try / finally`, `try / catch / finally`, `synchronized`, `Runnable`, `{ }`; for an expression `(expr)`, `((Type)expr)`, `!(expr)`, `if (expr) {...}`, `if / else`, null check, not-instanceof. Statement surrounders place the selected statements inside the construct (declarations used afterwards are moved out before it) and leave the caret right after the opening parenthesis of the condition when there is one. | Select `a();`, choose `if`: `if (\|) { a(); }` with the caret between the parentheses. |
| ED-TMPL-043 | SHOULD | Typing a quote or bracket with a selection also surrounds (`ED-TYPE-035`); the two features are independent. | Both work. |
| ED-TMPL-044 | MUST | `Code, Surround With, region...endregion` and the tag surround for XML (`Surround with <tag></tag>`, `Surround with CDATA section`) are live-template based and use `$SELECTION$`. | Surround two methods in a region (see `ED-FOLD-042`). |

## 6. Postfix completion

A postfix template transforms a **expression before a dot** into a statement or expression:
type an expression, a dot, a template key, then the expand key.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-POST-001 | MUST | Postfix templates are enabled by default (`Enable postfix completion`), expand with **Tab** by default (`Expand templates with`: Tab, Enter, Space), and are shown in completion (`Show postfix templates in completion autopopup` and `Show postfix completions as command completions` both default on). A template key is typed after a terminal symbol (Java: `.`, and `!` for the negation template). | Type `s.sout`: completion lists it; Tab expands. |
| ED-POST-002 | MUST | The expression is chosen by the template's **selector**: the expression ending at the caret before the dot (the maximal expression to the left, or when several candidates exist (for example `a.b.c`) the **innermost to outermost** list is offered in a chooser if more than one is applicable and the user has not selected text); with a selection, the selected expression is used. Applicability (type of the expression: boolean for `if`, `not`, `while`; array or Iterable for `for`; non-void for `var`, `return`, `sout`, ...) is checked before the template is offered. | `list.for` on a `List` works; `i.for` on an `int` is not offered. |
| ED-POST-003 | MUST | Expansion removes the dot, key and the expression text, inserts the template with the expression substituted, may insert a missing semicolon first, and is one undo command named `Expand Postfix Template` with the semicolon insert transparent. | Undo after expansion restores `s.sout`. |
| ED-POST-004 | MUST | The Java templates (keys; see section 7 for the produced code): `assert`, `synchronized`, `fori`, `forr`, `while`, `sout`, `iop`, `serr`, `souf`, `soutv`, `return`, `opt`, `iter`, `for`, `lambda`, `throw`, `format`, `requireNonNull`, `arg`, `cast`, `new`, `castvar`, `else`, `if`, `instanceof`, `inst`, `field`, `var`, `null`, `not`, `!`, `notnull`, `nn`, `par`, `switch`, `try`, `twr`, `stream`, and the conversions between collection and date types (`asList` to a list, `List.of` to a list, `new ArrayList` to `List`, `new HashSet` to `Set`, date conversions). Key names that are not given by an exact string in the sources are listed by class and must be confirmed. | Each key expands. |
| ED-POST-005 | SHOULD | User-defined postfix templates (key, applicable expression kinds, template text with `$EXPR$`) are editable under Settings, Editor, General, Postfix Completion; built-in templates can be switched off individually or per language; the list shows `Before` and `After` samples and a description. | Add `.log` template and expand it. |
| ED-POST-006 | MUST | A key typed with no applicable expression shows `Can't expand postfix template` (hint) instead of expanding. | `.sout` after nothing: hint. |

## 7. Built-in Java live templates (catalog)

Abbreviations with the code they produce (own wording; `|` is the final caret position,
`[field]` a stop). All reformat on expansion and shorten names, unless noted. Statement templates
apply in a Java statement context, declaration templates at class level.

| Abbreviation | Produces | Context |
|---|---|---|
| `sout` | `System.out.println(\|);` | statement |
| `serr` | `System.err.println(\|);` | statement |
| `souf` | `System.out.printf("\|");` | statement |
| `soutv` | `System.out.println("[expr] = " + [expr]);` where `[expr]` is a variable in scope (the quoted copy is the escaped expression text) | statement |
| `soutm` | prints `ClassName.methodName` | statement |
| `soutp` | prints the method parameters (`parameterInfo()`) | statement |
| `iop`, `ior` | `IO.println(\|);`, `IO.readln(\|);` (only in files where `IO` is available) | statement |
| `fori` | `for (int [i] = 0; [i] < [limit]; i++) { \| }` (index name suggested) | statement |
| `forr`, `ritar` | reverse loops | statement |
| `itar` | loop over an array variable: `for (int [i] = 0; i < [array].length; i++) { T v = array[i]; \| }` | statement |
| `iter` | enhanced for: `for ([T] [var] : [iterable]) { \| }` | statement |
| `itli`, `itco`, `itit`, `iten`, `itm` | index loop over a list; iterator loop over a collection; while loop over an iterator; over an enumeration; loop over map entries (`Map.Entry`, with `key` and `value` locals) | statement |
| `ittok` | loop over a `StringTokenizer` | statement |
| `inst` | `if (x instanceof T) { T v = (T) x; \| }` | statement |
| `inn`, `ifn` | `if (v != null) { \| }`, `if (v == null) { \| }` | statement |
| `lazy` | `if (v == null) { v = new T(\|); }` | statement |
| `else-if` | `else if ([cond]) { \| }` | after an if branch |
| `mn`, `mx` | `v = Math.min(v, \|);`, `v = Math.max(v, \|);` | statement |
| `lst` | `array[array.length - 1]` | expression |
| `toar` | `collection.toArray(new T[0])` | expression |
| `psvm`, `main` | `public static void main(String[] args) { \| }` | class level without an instance main |
| `geti` | `public static C getInstance() { return [value]; }` | declaration |
| `St` | `String ` | statement and declaration (no reformat) |
| `psf`, `prsf`, `psfi`, `psfs` | `public static final `, `private static final `, `... int `, `... String ` | declaration (no reformat) |
| `thr` | `throw new ` | statement (no reformat) |
| `soutc`, `serrc` | `System.out::println`, `System.err::println` | consumer |
| `C`, `RL`, `WL`, `I` | surround templates: `Callable` wrapper, read lock and write lock try/finally, a for-each over the selection | statement, with `$SELECTION$` |
| `sts` | `try (var scope = StructuredTaskScope.open()) { \| scope.join(); } catch ...` | statement (feature gated) |

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TMPL-050 | MUST | The built-in Java group exists with at least the templates above, their descriptions as in the product, and those contexts; the templates are user-editable and restorable. | Settings, Live Templates, Java lists them. |
| ED-TMPL-051 | MUST | `sout` and `psvm` are the minimum set for the first release; the other templates are SHOULD. | Cases. |

## 8. Settings

Settings, Editor, Live Templates (tree of groups; buttons add, copy, edit; right-hand editor with
`Abbreviation`, `Description`, `Template text`, `Edit Variables...`, `Options: Expand with, Reformat
according to style, Shorten FQ names`, and `Applicable in {contexts}` with a `Change` link that lists
context types; the message `No applicable contexts` and the error `Template key must be an
identifier` for postfix keys). The default `Expand with` is `Default (Tab)`. Postfix settings
are under Editor, General, Postfix Completion.

| ID | Pri | Requirement | Acceptance |
|---|---|---|---|
| ED-TMPL-060 | MUST | Defaults: expand key Tab; postfix enabled, in completion, expand with Tab; the built-in groups enabled. | Fresh profile. |
| ED-TMPL-061 | SHOULD | The settings page filters templates by abbreviation, supports reset to defaults per group, import and export (XML), and highlights templates not applicable in the current file type's contexts. | Export then import round-trips. |

## 9. Open questions

* The exact stop-character rules while a lookup is open in a field (ED-TMPL-035).
* Complete list of Java postfix keys whose string is in a class constant not read here (ED-POST-004).
* `lineNumber()` base (taken as 1) and the locale formats of `date()` and `time()`.
* Emmet (HTML/CSS abbreviation) expansion shares the Tab key; it is specified with the HTML area.
